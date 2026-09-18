pub mod hash;
pub mod job;
pub mod manager;

pub use job::{DownloadJob, DownloadProgress, JobState};
pub use manager::DownloadManager;
