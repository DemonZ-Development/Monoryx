use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadJob {
    pub id: String,
    pub label: String,
    pub url: String,
    pub dest: std::path::PathBuf,
    pub expected_sha1: Option<String>,
    pub expected_sha512: Option<String>,
    pub expected_size: Option<u64>,
}

impl DownloadJob {
    #[must_use]
    pub fn new(label: impl Into<String>, url: impl Into<String>, dest: std::path::PathBuf) -> Self {
        let label = label.into();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            label,
            url: url.into(),
            dest,
            expected_sha1: None,
            expected_sha512: None,
            expected_size: None,
        }
    }

    #[must_use]
    pub fn with_sha1(mut self, sha1: impl Into<String>) -> Self {
        self.expected_sha1 = Some(sha1.into());
        self
    }

    #[must_use]
    pub fn with_sha512(mut self, sha512: impl Into<String>) -> Self {
        self.expected_sha512 = Some(sha512.into());
        self
    }

    #[must_use]
    pub fn with_size(mut self, size: u64) -> Self {
        self.expected_size = Some(size);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    #[default]
    Queued,
    Active,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct DownloadEvent {
    pub id: String,
    pub label: String,
    pub state: JobState,
    pub progress: DownloadProgress,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed_bps: f64,
}

impl DownloadProgress {
    #[must_use]
    pub fn fraction(&self) -> Option<f32> {
        match self.total {
            Some(t) if t > 0 => Some((self.downloaded as f32 / t as f32).clamp(0.0, 1.0)),
            _ => None,
        }
    }
}
