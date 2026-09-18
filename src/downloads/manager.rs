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
        if let Ok(meta) = std::fs::metadata(&job.dest) {
            if let Some(expected) = job.expected_size {
                if meta.len() != expected {
                    return false;
                }
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
                    let backoff = Duration::from_millis(400 * 2u64.pow(attempt.min(4)));
                    tokio::time::sleep(backoff).await;
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
                "GET {} returned HTTP 429",
                job.url
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
        let mut stream = resp.bytes_stream();
        let mut last_report = Instant::now();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await.map_err(MonoryxError::Io)?;
            downloaded += chunk.len() as u64;
            if let Some(cb) = &progress {
                if last_report.elapsed() >= Duration::from_millis(100) {
                    let elapsed = start.elapsed().as_secs_f64().max(0.001);
                    cb(DownloadProgress {
                        downloaded,
                        total,
                        speed_bps: downloaded as f64 / elapsed,
                    });
                    last_report = Instant::now();
                }
            }
        }
        file.flush().await.map_err(MonoryxError::Io)?;
        drop(file);

        if let Some(expected) = job.expected_size.or(total) {
            if let Some(t) = total {
                let actual = tokio::fs::metadata(&part)
                    .await
                    .map_err(MonoryxError::Io)?
                    .len();
                if actual != t && job.expected_size.is_some() && actual != expected {
                    return Err(MonoryxError::Download(format!(
                        "size mismatch for {}: expected {expected}, got {actual}",
                        job.label
                    )));
                }
            }
        }

        if let Some(sha1) = &job.expected_sha1 {
            let actual = tokio::task::spawn_blocking({
                let part = part.clone();
                move || crate::utils::hash::sha1_file(&part)
            })
            .await
            .map_err(|e| MonoryxError::Download(e.to_string()))??;
            if actual.to_lowercase() != sha1.to_lowercase() {
                return Err(MonoryxError::HashMismatch {
                    file: job.label.clone(),
                    expected: sha1.clone(),
                    actual,
                });
            }
        }
        if let Some(sha512) = &job.expected_sha512 {
            let actual = tokio::task::spawn_blocking({
                let part = part.clone();
                move || crate::utils::hash::sha512_file(&part)
            })
            .await
            .map_err(|e| MonoryxError::Download(e.to_string()))??;
            if actual.to_lowercase() != sha512.to_lowercase() {
                return Err(MonoryxError::HashMismatch {
                    file: job.label.clone(),
                    expected: sha512.clone(),
                    actual,
                });
            }
        }

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
        let mut handles = Vec::new();
        #[allow(clippy::unnecessary_to_owned)]
        for job in jobs.to_vec() {
            let this = self.clone();
            let cb = on_each.clone();
            handles.push(tokio::spawn(async move {
                let job_for_cb = job.clone();
                let inner: ProgressCallback = Arc::new(move |p| {
                    if let Some(f) = &cb {
                        f(&job_for_cb, p);
                    }
                });
                this.download(&job, Some(inner)).await
            }));
        }
        for h in handles {
            h.await
                .map_err(|e| MonoryxError::Download(e.to_string()))??;
        }
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
        if msg.contains("HTTP 404")
            || msg.contains("HTTP 400")
            || msg.contains("HTTP 401")
            || msg.contains("HTTP 403")
            || msg.contains("HTTP 422")
        {
            return true;
        }
    }
    false
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
