use thiserror::Error;

#[derive(Debug, Error)]
pub enum MonoryxError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(String),

    #[error("TOML serialization error: {0}")]
    TomlSer(String),

    #[error("Invalid username: {0}")]
    InvalidUsername(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Loader unavailable: {0}")]
    LoaderUnavailable(String),

    #[error("Java runtime not found: {0}")]
    JavaNotFound(String),

    #[error("Incompatible Java version: {0}")]
    JavaIncompatible(String),

    #[error("Download failed: {0}")]
    Download(String),

    #[error("Hash mismatch for {file}: expected {expected}, got {actual}")]
    HashMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("Archive error: {0}")]
    Archive(String),

    #[error("Unsafe archive path rejected: {0}")]
    UnsafePath(String),

    #[error("Instance error: {0}")]
    Instance(String),

    #[error("Modrinth API error: {0}")]
    Modrinth(String),

    #[error("Dependency conflict: {0}")]
    DependencyConflict(String),

    #[error("Launch failed: {0}")]
    Launch(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl MonoryxError {
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Http(e) => {
                if let Some(status) = e.status() {
                    match status.as_u16() {
                        404 => "The requested file was not found (HTTP 404).".to_string(),
                        429 => "Rate limited (HTTP 429). Please wait and retry.".to_string(),
                        500..=599 => format!("The server returned HTTP {status}. Please retry."),
                        _ => format!("Network request failed: HTTP {status}."),
                    }
                } else if e.is_timeout() {
                    "The request timed out. Check your connection and retry.".to_string()
                } else if e.is_connect() {
                    "Couldn't connect. Check your internet connection.".to_string()
                } else {
                    format!("Network error: {e}")
                }
            }
            Self::HashMismatch { file, .. } => {
                format!("File {file} failed integrity verification and was discarded.")
            }
            Self::JavaNotFound(msg) => format!("No compatible Java installation found. {msg}"),
            Self::InvalidUsername(msg) => format!("Invalid username: {msg}"),
            other => other.to_string(),
        }
    }
}

pub type Result<T, E = MonoryxError> = std::result::Result<T, E>;
