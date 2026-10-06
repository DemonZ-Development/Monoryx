use crate::downloads::job::{DownloadEvent, DownloadJob, DownloadProgress, JobState};
use crate::error::{MonoryxError, Result};
use futures::StreamExt as _;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt as _;
use tokio::sync::Semaphore;

pub type ProgressCallback = Arc<dyn Fn(DownloadProgress) + Send + Sync>;
pub type EachCallback = Option<Arc<dyn Fn(&DownloadJob, DownloadProgress) + Send + Sync>>;

pub type LifecycleCallback = Arc<dyn Fn(DownloadEvent) + Send + Sync>;

#[derive(Clone)]
pub struct DownloadManager {
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
    max_retries: u32,
    observer: Option<LifecycleCallback>,
}

struct DownloadLifecycle {
    observer: Option<LifecycleCallback>,
    event: DownloadEvent,
    finished: bool,
}

impl DownloadLifecycle {
    fn emit(&self) {
        if let Some(cb) = &self.observer {
            cb(self.event.clone());
        }
    }

    fn finish(&mut self, result: &Result<()>) {
        self.finished = true;
        self.event.state = if result.is_ok() {
            JobState::Completed
        } else {
            JobState::Failed
        };
        self.event.message = result
            .as_ref()
            .err()
            .map(MonoryxError::user_message)
            .unwrap_or_default();
        self.emit();
    }
}

impl Drop for DownloadLifecycle {
    fn drop(&mut self) {
        if !self.finished {
            self.event.state = JobState::Cancelled;
            self.event.message = "Download cancelled".to_string();
            self.emit();
        }
    }
}

impl DownloadManager {
    #[must_use]
    pub fn new(client: reqwest::Client, max_concurrent: usize) -> Self {
        let max_concurrent = max_concurrent.clamp(1, 16);
        Self {
            client,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
            max_retries: 3,
            observer: None,
        }
    }

    pub fn with_observer(mut self, observer: LifecycleCallback) -> Self {
        self.observer = Some(observer);
        self
    }

    #[must_use]
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub fn is_valid_existing(&self, job: &DownloadJob) -> bool {
        if !job.dest.exists() {
            return false;
        }
        let Ok(meta) = std::fs::metadata(&job.dest) else {
            return false;
        };
        if let Some(expected) = job.expected_size {
            if meta.len() != expected {
                return false;
            }
        }
        if let Some(sha1) = &job.expected_sha1 {
            let actual = crate::utils::hash::sha1_file(&job.dest).unwrap_or_default();
            if actual.to_lowercase() != sha1.to_lowercase() {
                return false;
            }
        }
        if let Some(sha512) = &job.expected_sha512 {
            let actual = crate::utils::hash::sha512_file(&job.dest).unwrap_or_default();
            if actual.to_lowercase() != sha512.to_lowercase() {
                return false;
            }
        }
        true
    }

    pub async fn download(
        &self,
        job: &DownloadJob,
        progress: Option<ProgressCallback>,
    ) -> Result<()> {
        let mut lifecycle = DownloadLifecycle {
            observer: self.observer.clone(),
            event: DownloadEvent {
                id: job.id.clone(),
                label: job.label.clone(),
                state: JobState::Queued,
                progress: DownloadProgress {
                    total: job.expected_size,
                    ..Default::default()
                },
                message: String::new(),
            },
            finished: false,
        };
        lifecycle.emit();
        let result = async {
            let _permit = self
                .semaphore
                .acquire()
                .await
                .map_err(|e| MonoryxError::Download(e.to_string()))?;
            lifecycle.event.state = JobState::Active;
            lifecycle.emit();
            let observer = self.observer.clone();
            let event = lifecycle.event.clone();
            let callback: ProgressCallback = Arc::new(move |p| {
                if let Some(cb) = &observer {
                    let mut tick = event.clone();
                    tick.progress = p.clone();
                    cb(tick);
                }
                if let Some(cb) = &progress {
                    cb(p);
                }
            });
            self.download_inner(job, Some(callback)).await
        }
        .await;
        if result.is_ok() {
            let size = std::fs::metadata(&job.dest).ok().map(|m| m.len());
            lifecycle.event.progress.downloaded = size.unwrap_or(0);
            lifecycle.event.progress.total = size;
        }
        lifecycle.finish(&result);
        result
    }

    async fn download_inner(
        &self,
        job: &DownloadJob,
        progress: Option<ProgressCallback>,
    ) -> Result<()> {
        if self.is_valid_existing(job) {
            if let Some(cb) = &progress {
                let total = job
                    .expected_size
                    .or_else(|| std::fs::metadata(&job.dest).ok().map(|m| m.len()));
                cb(DownloadProgress {
                    downloaded: total.unwrap_or(0),
                    total,
                    speed_bps: 0.0,
                });
            }
            return Ok(());
        }
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match self.download_once(job, progress.clone()).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let permanent =
                        matches!(&e, MonoryxError::HashMismatch { .. }) || is_permanent_http(&e);
                    if permanent || attempt > self.max_retries {
                        let part = part_path(&job.dest);
                        let _ = tokio::fs::remove_file(&part).await;
                        return Err(e);
                    }
                    let backoff = match retry_after_hint(&e) {
                        Some(delay) => delay,
                        None => Duration::from_millis(400 * 2u64.pow(attempt.min(4))),
                    };
                    tokio::time::sleep(backoff.min(Duration::from_secs(30))).await;
                }
            }
        }
    }

    async fn download_once(
        &self,
        job: &DownloadJob,
        progress: Option<ProgressCallback>,
    ) -> Result<()> {
        if let Some(parent) = job.dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(MonoryxError::Io)?;
        }
        let part = part_path(&job.dest);

        let resume_from: u64 = tokio::fs::metadata(&part)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        let mut req = self.client.get(&job.url);
        if resume_from > 0 {
            req = req.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
        }
        let resp = req.send().await?;
        let status = resp.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(MonoryxError::Download(format!(
                "GET {} returned HTTP 429, retry after {}s",
                job.url,
                resp.headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.trim().parse::<u64>().ok())
                    .unwrap_or(5)
            )));
        }
        if status.is_client_error() {
            return Err(MonoryxError::Download(format!(
                "GET {} returned HTTP {status}",
                job.url
            )));
        }
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(MonoryxError::Download(format!(
                "GET {} returned HTTP {status}",
                job.url
            )));
        }

        let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
        let content_len: Option<u64> = resp.content_length();
        let total = match (resumed, content_len) {
            (true, Some(c)) => Some(resume_from + c),
            (true, None) => job.expected_size,
            (false, c) => c.or(job.expected_size),
        };

        let mut file = if resumed {
            tokio::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&part)
                .await
                .map_err(MonoryxError::Io)?
        } else {
            tokio::fs::File::create(&part)
                .await
                .map_err(MonoryxError::Io)?
        };

        let mut downloaded: u64 = if resumed { resume_from } else { 0 };
        let start = Instant::now();
        let mut reported_bytes: u64 = 0;
        let ceiling = job
            .expected_size
            .zip(total)
            .map(|(expected, total)| expected.max(total))
            .map_or_else(
                || MAX_DOWNLOAD_BYTES,
                |known| known.saturating_add(SIZE_SLACK_BYTES),
            );
        let mut stream = resp.bytes_stream();
        let mut last_report_at = Instant::now();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded = downloaded.saturating_add(chunk.len() as u64);
            if downloaded > ceiling {
                return Err(MonoryxError::Download(format!(
                    "{} exceeded its expected size of {ceiling} bytes",
                    job.label
                )));
            }
            file.write_all(&chunk).await.map_err(MonoryxError::Io)?;
            if let Some(cb) = &progress {
                if last_report_at.elapsed() >= Duration::from_millis(100) {
                    let elapsed = last_report_at
                        .duration_since(start)
                        .as_secs_f64()
                        .max(0.001);
                    cb(DownloadProgress {
                        downloaded,
                        total,
                        speed_bps: (downloaded - reported_bytes) as f64 / elapsed,
                    });
                    reported_bytes = downloaded;
                    last_report_at = Instant::now();
                }
            }
        }
        file.flush().await.map_err(MonoryxError::Io)?;
        file.sync_all().await.map_err(MonoryxError::Io)?;
        drop(file);

        let actual_size = tokio::fs::metadata(&part)
            .await
            .map_err(MonoryxError::Io)?
            .len();

        for expected in [job.expected_sha1.clone(), job.expected_sha512.clone()]
            .into_iter()
            .flatten()
        {
            let is_sha512 = expected.len() == 128;
            let part_for_hash = part.clone();
            let actual = tokio::task::spawn_blocking(move || {
                if is_sha512 {
                    crate::utils::hash::sha512_file(&part_for_hash)
                } else {
                    crate::utils::hash::sha1_file(&part_for_hash)
                }
            })
            .await
            .map_err(|e| MonoryxError::Download(e.to_string()))??;
            if actual.to_lowercase() != expected.to_lowercase() {
                return Err(MonoryxError::HashMismatch {
                    file: job.label.clone(),
                    expected,
                    actual,
                });
            }
        }

        let sizes = [job.expected_size, total]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let has_verified_hash = job.expected_sha1.is_some() || job.expected_sha512.is_some();
        size_verdict(&job.label, &sizes, actual_size, has_verified_hash)?;

        tokio::fs::rename(&part, &job.dest)
            .await
            .map_err(MonoryxError::Io)?;
        if let Some(cb) = &progress {
            cb(DownloadProgress {
                downloaded: total.unwrap_or(downloaded),
                total,
                speed_bps: 0.0,
            });
        }
        Ok(())
    }

    pub async fn download_all(&self, jobs: &[DownloadJob], on_each: EachCallback) -> Result<()> {
        futures::stream::iter(jobs.iter().cloned())
            .map(|job| {
                let this = self.clone();
                let cb = on_each.clone();
                async move {
                    let job_for_cb = job.clone();
                    let inner: ProgressCallback = Arc::new(move |p| {
                        if let Some(f) = &cb {
                            f(&job_for_cb, p);
                        }
                    });
                    this.download(&job, Some(inner)).await
                }
            })
            .buffer_unordered(self.max_concurrent)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        Ok(())
    }
}

fn part_path(dest: &Path) -> std::path::PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    std::path::PathBuf::from(s)
}

fn is_permanent_http(e: &MonoryxError) -> bool {
    if let MonoryxError::Download(msg) = e {
        if let Some(code) = http_status_in(msg) {
            return matches!(code, 400 | 401 | 403 | 404 | 410 | 422);
        }
        if msg.contains("exceeded its expected size") {
            return true;
        }
    }
    false
}

fn http_status_in(msg: &str) -> Option<u16> {
    let marker = msg.find("HTTP ")?;
    let digits: String = msg[marker + 5..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

fn retry_after_hint(e: &MonoryxError) -> Option<Duration> {
    let MonoryxError::Download(msg) = e else {
        return None;
    };
    let marker = msg.find("retry after ")?;
    let digits: String = msg[marker + "retry after ".len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits
        .parse()
        .ok()
        .map(Duration::from_secs)
        .map(|delay| delay + Duration::from_millis(250))
}

const MAX_DOWNLOAD_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const SIZE_SLACK_BYTES: u64 = 1024 * 1024;

const SIZE_TOLERANCE_BYTES: u64 = 4096;

fn size_verdict(
    label: &str,
    expected: &[u64],
    actual: u64,
    has_verified_hash: bool,
) -> Result<(), MonoryxError> {
    let Some(first) = expected.first().copied() else {
        return Ok(());
    };
    let worst = expected
        .iter()
        .map(|e| e.abs_diff(actual))
        .max()
        .unwrap_or(0);
    if worst == 0 {
        return Ok(());
    }
    if has_verified_hash && worst <= SIZE_TOLERANCE_BYTES {
        tracing::warn!(
            "{label}: size differs from metadata ({actual} vs {first}) but the hash matched"
        );
        return Ok(());
    }
    if worst > SIZE_TOLERANCE_BYTES {
        return Err(MonoryxError::Download(format!(
            "size mismatch for {label}: expected {first}, got {actual}"
        )));
    }
    if !has_verified_hash {
        return Err(MonoryxError::Download(format!(
            "size mismatch for {label}: expected {first}, got {actual}, and the server gave \
             no hash to confirm the file"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn part_path_appends() {
        assert_eq!(part_path(Path::new("/a/b.jar")), Path::new("/a/b.jar.part"));
    }

    #[test]
    fn permanent_detection() {
        assert!(is_permanent_http(&MonoryxError::Download(
            "GET x returned HTTP 404".into()
        )));
        assert!(!is_permanent_http(&MonoryxError::Download(
            "GET x returned HTTP 500".into()
        )));
    }

    #[tokio::test]
    async fn lifecycle_emits_queued_active_completed() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.bin");
        std::fs::write(&dest, b"cached").unwrap();
        let job =
            DownloadJob::new("existing file", "http://localhost/none", dest.clone()).with_size(6);
        let events: Arc<Mutex<Vec<DownloadEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let observer: LifecycleCallback = Arc::new(move |e| sink.lock().unwrap().push(e));
        let manager = DownloadManager::new(reqwest::Client::new(), 1).with_observer(observer);
        let result = manager.download(&job, None).await;
        assert!(result.is_ok());
        let got = events.lock().unwrap().clone();
        let states: Vec<JobState> = got.iter().map(|e| e.state).collect();
        assert_eq!(states.first(), Some(&JobState::Queued));
        assert!(states.contains(&JobState::Active));
        assert_eq!(states.last(), Some(&JobState::Completed));
        assert!(got.iter().all(|e| e.id == job.id));
        assert_eq!(got.last().unwrap().progress.downloaded, 6);
    }
}

#[cfg(test)]
mod size_verdict_tests {
    use super::*;

    #[test]
    fn permanent_statuses_are_read_from_the_typed_code_not_the_message() {
        for code in [400u16, 401, 403, 404, 410, 422] {
            let err = MonoryxError::Download(format!("GET https://x/y returned HTTP {code}"));
            assert!(is_permanent_http(&err), "HTTP {code} must not be retried");
        }
        for code in [408u16, 429, 500, 502, 503, 504] {
            let err = MonoryxError::Download(format!("GET https://x/y returned HTTP {code}"));
            assert!(!is_permanent_http(&err), "HTTP {code} should be retried");
        }
        let misleading =
            MonoryxError::Download("GET https://x/?note=HTTP%20404 returned HTTP 500".to_string());
        assert!(!is_permanent_http(&misleading));
    }

    #[test]
    fn rate_limit_backoff_is_taken_from_the_retry_after_header() {
        let err = MonoryxError::Download(
            "GET https://x/y returned HTTP 429, retry after 17s".to_string(),
        );
        assert_eq!(
            retry_after_hint(&err),
            Some(Duration::from_secs(17) + Duration::from_millis(250))
        );
        let none = MonoryxError::Download("GET https://x/y returned HTTP 500".to_string());
        assert_eq!(retry_after_hint(&none), None);
        assert!(!is_permanent_http(&err), "429 must stay retryable");
    }

    #[test]
    fn oversized_payloads_are_rejected_before_they_fill_the_disk() {
        assert!(is_permanent_http(&MonoryxError::Download(
            "iris exceeded its expected size of 10 bytes".to_string()
        )));
    }

    #[test]
    fn a_large_drift_with_a_matching_hash_is_still_rejected() {
        assert!(
            size_verdict("iris", &[2_850_938], 2_982_010, true).is_err(),
            "a 131 KB drift is a different file, so this must be rejected"
        );
    }

    #[test]
    fn small_drift_with_a_matching_hash_is_accepted() {
        assert!(size_verdict("a.jar", &[1000], 1002, true).is_ok());
    }

    #[test]
    fn drift_with_no_hash_is_rejected() {
        assert!(size_verdict("a.jar", &[1000], 1002, false).is_err());
        assert!(size_verdict("a.jar", &[1000], 500_000, false).is_err());
    }

    #[test]
    fn exact_match_always_passes() {
        assert!(size_verdict("a.jar", &[1000], 1000, false).is_ok());
        assert!(size_verdict("a.jar", &[1000], 1000, true).is_ok());
    }

    #[test]
    fn no_expected_size_passes() {
        assert!(size_verdict("a.jar", &[], 12345, false).is_ok());
    }

    #[test]
    fn one_of_two_sizes_matching_is_enough_when_hash_verified() {
        assert!(size_verdict("a.jar", &[1000, 1001], 1000, true).is_ok());
    }

    #[test]
    fn wildly_off_size_is_rejected_even_with_a_hash() {
        assert!(size_verdict("a.jar", &[1000], 900_000, true).is_err());
    }
}
