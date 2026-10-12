use crate::downloads::job::{DownloadEvent, DownloadJob, DownloadProgress, JobState};
use crate::error::{MonoryxError, Result};
use futures::StreamExt as _;
use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, OnceLock, Weak};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OwnedMutexGuard, Semaphore};

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

struct DownloadLease {
    _caller_guard: Option<Arc<dyn Send + Sync>>,
    _destination_guard: OwnedMutexGuard<()>,
}

impl DownloadLease {
    async fn run<T, F>(self: &Arc<Self>, operation: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
    {
        let lease = self.clone();
        tokio::task::spawn_blocking(move || {
            let result = operation();
            drop(lease);
            result
        })
        .await
        .map_err(|error| MonoryxError::Download(error.to_string()))?
    }
}

struct DownloadFile {
    file: std::sync::Mutex<std::fs::File>,
    lease: Arc<DownloadLease>,
}

impl DownloadFile {
    async fn open(path: PathBuf, resumed: bool, lease: &Arc<DownloadLease>) -> Result<Arc<Self>> {
        let file_lease = lease.clone();
        lease
            .run(move || {
                let file = if resumed {
                    std::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(path)?
                } else {
                    std::fs::File::create(path)?
                };
                Ok(Arc::new(Self {
                    file: std::sync::Mutex::new(file),
                    lease: file_lease,
                }))
            })
            .await
    }

    async fn write(self: &Arc<Self>, chunk: impl AsRef<[u8]> + Send + 'static) -> Result<()> {
        let file = self.clone();
        self.lease
            .run(move || {
                file.file
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .write_all(chunk.as_ref())?;
                Ok(())
            })
            .await
    }

    async fn finish(self: Arc<Self>) -> Result<u64> {
        let file = self.clone();
        self.lease
            .run(move || {
                let mut handle = file.file.lock().unwrap_or_else(|error| error.into_inner());
                handle.flush()?;
                handle.sync_all()?;
                Ok(handle.metadata()?.len())
            })
            .await
    }
}

struct TransferRate {
    reported_at: Instant,
    reported_bytes: u64,
}

impl TransferRate {
    fn new(downloaded: u64, now: Instant) -> Self {
        Self {
            reported_at: now,
            reported_bytes: downloaded,
        }
    }

    fn sample(&mut self, downloaded: u64, now: Instant) -> Option<f64> {
        let elapsed = now.saturating_duration_since(self.reported_at);
        if elapsed < Duration::from_millis(100) {
            return None;
        }
        let speed = downloaded.saturating_sub(self.reported_bytes) as f64 / elapsed.as_secs_f64();
        self.reported_at = now;
        self.reported_bytes = downloaded;
        Some(speed)
    }
}

fn destination_key(dest: &Path) -> Result<PathBuf> {
    let name = dest.file_name().ok_or_else(|| {
        MonoryxError::Download("Download destination must name a file".to_string())
    })?;
    let parent = dest
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut ancestor = std::path::absolute(parent)?;
    let mut suffix = Vec::new();
    let mut key = loop {
        match ancestor.canonicalize() {
            Ok(canonical) => break canonical,
            Err(error) => {
                let component = ancestor.components().next_back();
                match component {
                    Some(Component::Normal(name)) => suffix.push(name.to_os_string()),
                    Some(Component::ParentDir) => suffix.push("..".into()),
                    Some(Component::CurDir) => suffix.push(".".into()),
                    _ => return Err(error.into()),
                }
                if !ancestor.pop() {
                    return Err(error.into());
                }
            }
        }
    };
    for component in suffix.into_iter().rev() {
        if component == ".." {
            key.pop();
        } else if component != "." {
            key.push(component);
        }
    }
    key.push(name);
    #[cfg(windows)]
    let key = PathBuf::from(key.to_string_lossy().to_lowercase());
    Ok(key)
}

async fn destination_lock(
    dest: &Path,
    caller_guard: Option<Arc<dyn Send + Sync>>,
) -> Result<Arc<Mutex<()>>> {
    static LOCKS: OnceLock<std::sync::Mutex<HashMap<PathBuf, Weak<Mutex<()>>>>> = OnceLock::new();
    let dest = dest.to_path_buf();
    let key = tokio::task::spawn_blocking(move || {
        let result = destination_key(&dest);
        drop(caller_guard);
        result
    })
    .await
    .map_err(|error| MonoryxError::Download(error.to_string()))??;
    let mut locks = LOCKS
        .get_or_init(std::sync::Mutex::default)
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(&key).and_then(Weak::upgrade) {
        return Ok(lock);
    }
    let lock = Arc::new(Mutex::new(()));
    locks.insert(key, Arc::downgrade(&lock));
    Ok(lock)
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
        if !meta.is_file() {
            return false;
        }
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
        self.download_guarded(job, progress, None).await
    }

    pub async fn download_with_guard(
        &self,
        job: &DownloadJob,
        progress: Option<ProgressCallback>,
        caller_guard: Arc<dyn Send + Sync>,
    ) -> Result<()> {
        self.download_guarded(job, progress, Some(caller_guard))
            .await
    }

    async fn download_guarded(
        &self,
        job: &DownloadJob,
        progress: Option<ProgressCallback>,
        caller_guard: Option<Arc<dyn Send + Sync>>,
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
            let destination = destination_lock(&job.dest, caller_guard.clone()).await?;
            let lease = Arc::new(DownloadLease {
                _caller_guard: caller_guard,
                _destination_guard: destination.lock_owned().await,
            });
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
            self.download_inner(job, Some(callback), &lease).await
        }
        .await;
        if let Ok(size) = &result {
            lifecycle.event.progress.downloaded = *size;
            lifecycle.event.progress.total = Some(*size);
        }
        let result = result.map(|_| ());
        lifecycle.finish(&result);
        result
    }

    async fn download_inner(
        &self,
        job: &DownloadJob,
        progress: Option<ProgressCallback>,
        lease: &Arc<DownloadLease>,
    ) -> Result<u64> {
        let parent = job
            .dest
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        lease
            .run(move || {
                std::fs::create_dir_all(parent)?;
                Ok(())
            })
            .await?;
        let manager = self.clone();
        let existing_job = job.clone();
        let existing = lease
            .run(move || {
                Ok(if manager.is_valid_existing(&existing_job) {
                    std::fs::metadata(existing_job.dest)
                        .ok()
                        .map(|metadata| metadata.len())
                } else {
                    None
                })
            })
            .await?;
        if let Some(size) = existing {
            if let Some(cb) = &progress {
                cb(DownloadProgress {
                    downloaded: size,
                    total: Some(size),
                    speed_bps: 0.0,
                });
            }
            return Ok(size);
        }
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match self.download_once(job, progress.clone(), lease).await {
                Ok(size) => return Ok(size),
                Err(e) => {
                    let permanent =
                        matches!(&e, MonoryxError::HashMismatch { .. }) || is_permanent_http(&e);
                    if permanent || attempt > self.max_retries {
                        let part = part_path(&job.dest);
                        let _ = lease
                            .run(move || {
                                std::fs::remove_file(part)?;
                                Ok(())
                            })
                            .await;
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
        lease: &Arc<DownloadLease>,
    ) -> Result<u64> {
        let part = part_path(&job.dest);

        let resume_part = part.clone();
        let resume_from = lease
            .run(move || {
                Ok(std::fs::metadata(resume_part)
                    .map(|metadata| metadata.len())
                    .unwrap_or(0))
            })
            .await?;

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

        let file = DownloadFile::open(part.clone(), resumed, lease).await?;

        let mut downloaded: u64 = if resumed { resume_from } else { 0 };
        let mut rate = TransferRate::new(downloaded, Instant::now());
        let ceiling = job
            .expected_size
            .zip(total)
            .map(|(expected, total)| expected.max(total))
            .map_or_else(
                || MAX_DOWNLOAD_BYTES,
                |known| known.saturating_add(SIZE_SLACK_BYTES),
            );
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded = downloaded.saturating_add(chunk.len() as u64);
            if downloaded > ceiling {
                return Err(MonoryxError::Download(format!(
                    "{} exceeded its expected size of {ceiling} bytes",
                    job.label
                )));
            }
            file.write(chunk).await?;
            if let Some(cb) = &progress {
                if let Some(speed_bps) = rate.sample(downloaded, Instant::now()) {
                    cb(DownloadProgress {
                        downloaded,
                        total,
                        speed_bps,
                    });
                }
            }
        }
        let actual_size = file.finish().await?;

        for expected in [job.expected_sha1.clone(), job.expected_sha512.clone()]
            .into_iter()
            .flatten()
        {
            let is_sha512 = expected.len() == 128;
            let part_for_hash = part.clone();
            let actual = lease
                .run(move || {
                    if is_sha512 {
                        crate::utils::hash::sha512_file(&part_for_hash)
                    } else {
                        crate::utils::hash::sha1_file(&part_for_hash)
                    }
                })
                .await?;
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

        let dest = job.dest.clone();
        lease
            .run(move || {
                std::fs::rename(part, dest)?;
                Ok(())
            })
            .await?;
        if let Some(cb) = &progress {
            cb(DownloadProgress {
                downloaded: actual_size,
                total: Some(actual_size),
                speed_bps: 0.0,
            });
        }
        Ok(actual_size)
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

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

    #[test]
    fn transfer_rate_uses_each_interval_and_excludes_resumed_bytes() {
        let start = Instant::now();
        let mut rate = TransferRate::new(5_000, start);
        assert_eq!(rate.sample(10_000, start + Duration::from_millis(50)), None);
        assert_eq!(
            rate.sample(15_000, start + Duration::from_millis(100)),
            Some(100_000.0)
        );
        assert_eq!(
            rate.sample(25_000, start + Duration::from_millis(200)),
            Some(100_000.0)
        );
        assert_eq!(
            rate.sample(30_000, start + Duration::from_millis(400)),
            Some(25_000.0)
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn destination_locks_match_windows_case_aliases() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("Libraries").join("Example.jar");
        let alias = PathBuf::from(dest.to_string_lossy().to_uppercase());
        let first = destination_lock(&dest, None).await.unwrap();
        let second = destination_lock(&alias, None).await.unwrap();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[tokio::test]
    async fn destination_keys_stay_stable_when_parent_directories_are_created() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("new").join("libraries");
        let dest = parent.join("artifact.jar");
        let first = destination_lock(&dest, None).await.unwrap();
        assert!(!parent.exists());
        std::fs::create_dir_all(&parent).unwrap();
        let second = destination_lock(&dest, None).await.unwrap();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[tokio::test]
    async fn cancelled_filesystem_work_retains_destination_lock_and_rollback_guard() {
        struct RollbackGuard {
            root: PathBuf,
            completed: Option<tokio::sync::oneshot::Sender<()>>,
        }

        impl Drop for RollbackGuard {
            fn drop(&mut self) {
                std::fs::remove_dir_all(&self.root).unwrap();
                self.completed.take().unwrap().send(()).unwrap();
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("instance");
        std::fs::create_dir_all(&root).unwrap();
        let dest = root.join("pending.bin");
        let continuation = dir.path().join("continued.bin");
        let continuation_for_task = continuation.clone();
        let lock = destination_lock(&dest, None).await.unwrap();
        let (completed_tx, mut completed_rx) = tokio::sync::oneshot::channel();
        let caller_guard = Arc::new(RollbackGuard {
            root: root.clone(),
            completed: Some(completed_tx),
        });
        let lease = Arc::new(DownloadLease {
            _caller_guard: Some(caller_guard),
            _destination_guard: lock.clone().lock_owned().await,
        });
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let task = tokio::spawn(async move {
            lease
                .run(move || {
                    let mut file = std::fs::File::create(dest)?;
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    file.write_all(b"pending")?;
                    Ok(())
                })
                .await?;
            std::fs::write(continuation_for_task, b"continued")?;
            Ok::<(), MonoryxError>(())
        });
        tokio::time::timeout(Duration::from_secs(5), started_rx)
            .await
            .unwrap()
            .unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(lock.try_lock().is_err());
        assert!(root.exists());
        assert!(matches!(
            completed_rx.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        release_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), completed_rx)
            .await
            .unwrap()
            .unwrap();
        let _guard = tokio::time::timeout(Duration::from_secs(5), lock.lock())
            .await
            .unwrap();
        assert!(!root.exists());
        assert!(!continuation.exists());
    }

    async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
        let mut buffer = [0; 4096];
        let mut received = 0;
        loop {
            assert!(received < buffer.len());
            let count = stream.read(&mut buffer[received..]).await.unwrap();
            assert!(count > 0);
            received += count;
            if buffer[..received]
                .windows(4)
                .any(|bytes| bytes == b"\r\n\r\n")
            {
                return String::from_utf8(buffer[..received].to_vec()).unwrap();
            }
        }
    }

    #[tokio::test]
    async fn cancelling_writer_releases_destination_and_preserves_resumable_bytes() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let payload = b"download resumed after cancellation";
        let prefix_length = 10;
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            read_request(&mut stream).await;
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            stream.write_all(headers.as_bytes()).await.unwrap();
            stream.write_all(&payload[..prefix_length]).await.unwrap();
            started_tx.send(()).unwrap();
            let mut buffer = [0; 1];
            match stream.read(&mut buffer).await {
                Ok(0) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                    ) => {}
                result => panic!("cancelled download left its connection open: {result:?}"),
            }
            drop(stream);
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_request(&mut stream).await.to_lowercase();
            assert!(request.contains(&format!("range: bytes={prefix_length}-\r\n")));
            let headers = format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
                payload.len() - prefix_length,
                prefix_length,
                payload.len() - 1,
                payload.len()
            );
            stream.write_all(headers.as_bytes()).await.unwrap();
            stream.write_all(&payload[prefix_length..]).await.unwrap();
        });
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("resumed.bin");
        let job = DownloadJob::new("resumed", &url, dest.clone())
            .with_size(payload.len() as u64)
            .with_sha1(crate::utils::hash::sha1_bytes(payload));
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let manager = DownloadManager::new(client.clone(), 1);
        let first_job = job.clone();
        let first = tokio::spawn(async move { manager.download(&first_job, None).await });
        tokio::time::timeout(Duration::from_secs(5), async {
            started_rx.await.unwrap();
            loop {
                if tokio::fs::metadata(part_path(&dest))
                    .await
                    .is_ok_and(|metadata| metadata.len() == prefix_length as u64)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());
        let lock = destination_lock(&dest, None).await.unwrap();
        let guard = tokio::time::timeout(Duration::from_secs(5), lock.lock())
            .await
            .unwrap();
        drop(guard);
        let manager = DownloadManager::new(client, 1);
        tokio::time::timeout(Duration::from_secs(5), manager.download(&job, None))
            .await
            .unwrap()
            .unwrap();
        server.await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), payload);
        assert!(!part_path(&dest).exists());
    }

    #[tokio::test]
    async fn shared_destinations_are_serialized_across_managers_and_path_aliases() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let payload = b"shared verified download";
        let requests = Arc::new(AtomicUsize::new(0));
        let server_requests = requests.clone();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let mut started = Some(started_tx);
            let mut release = Some(release_rx);
            let mut responses = Vec::new();
            loop {
                let (mut stream, _) = tokio::select! {
                    accepted = listener.accept() => accepted.unwrap(),
                    _ = &mut shutdown_rx => break,
                };
                server_requests.fetch_add(1, Ordering::SeqCst);
                let started = started.take();
                let release = release.take();
                responses.push(tokio::spawn(async move {
                    read_request(&mut stream).await;
                    let headers = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        payload.len()
                    );
                    stream.write_all(headers.as_bytes()).await.unwrap();
                    if let Some(started) = started {
                        started.send(()).unwrap();
                    }
                    if let Some(release) = release {
                        release.await.unwrap();
                    }
                    stream.write_all(payload).await.unwrap();
                }));
            }
            for response in responses {
                response.await.unwrap();
            }
        });
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("downloads").join("shared.bin");
        let job = DownloadJob::new("shared", &url, dest.clone())
            .with_size(payload.len() as u64)
            .with_sha1(crate::utils::hash::sha1_bytes(payload));
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let manager = DownloadManager::new(client.clone(), 2);
        let first_job = job.clone();
        let first = tokio::spawn(async move { manager.download(&first_job, None).await });
        tokio::time::timeout(Duration::from_secs(5), started_rx)
            .await
            .unwrap()
            .unwrap();
        let mut second_job = job;
        second_job.dest = dir
            .path()
            .join("downloads")
            .join("..")
            .join("downloads")
            .join("shared.bin");
        let lock = destination_lock(&second_job.dest, None).await.unwrap();
        assert!(lock.try_lock().is_err());
        let (queued_tx, mut queued_rx) = tokio::sync::mpsc::unbounded_channel();
        let manager = DownloadManager::new(client, 2).with_observer(Arc::new(move |event| {
            if event.state == JobState::Queued {
                queued_tx.send(()).unwrap();
            }
        }));
        let second = tokio::spawn(async move { manager.download(&second_job, None).await });
        queued_rx.recv().await.unwrap();
        release_tx.send(()).unwrap();
        let (first_result, second_result) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(first, second)
        })
        .await
        .unwrap();
        first_result.unwrap().unwrap();
        second_result.unwrap().unwrap();
        shutdown_tx.send(()).unwrap();
        server.await.unwrap();
        assert_eq!(requests.load(Ordering::SeqCst), 1);
        assert_eq!(std::fs::read(&dest).unwrap(), payload);
        assert!(!part_path(&dest).exists());
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
