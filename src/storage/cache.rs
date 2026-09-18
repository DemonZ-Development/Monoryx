use crate::error::Result;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct DiskCache {
    dir: PathBuf,
    default_ttl: Duration,
}

impl DiskCache {
    #[must_use]
    pub fn new(dir: PathBuf, default_ttl: Duration) -> Self {
        Self { dir, default_ttl }
    }

    fn path_for(&self, key: &str) -> PathBuf {
        let safe: String = key
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        let h = crate::utils::hash::sha1_bytes(key.as_bytes());
        self.dir.join(&h[0..2]).join(format!("{safe}-{h}.cache"))
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.get_with_ttl(key, self.default_ttl)
    }

    pub fn get_with_ttl(&self, key: &str, ttl: Duration) -> Option<Vec<u8>> {
        let p = self.path_for(key);
        let meta = std::fs::metadata(&p).ok()?;
        let modified = meta.modified().ok()?;
        if SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::MAX)
            > ttl
        {
            return None;
        }
        std::fs::read(p).ok()
    }

    pub fn get_stale(&self, key: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path_for(key)).ok()
    }

    pub fn put(&self, key: &str, bytes: &[u8]) -> Result<()> {
        let p = self.path_for(key);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::utils::fs::atomic_write(&p, bytes)
    }

    pub fn last_modified(&self, key: &str) -> Option<SystemTime> {
        std::fs::metadata(self.path_for(key)).ok()?.modified().ok()
    }

    pub fn path(&self, key: &str) -> PathBuf {
        self.path_for(key)
    }

    #[must_use]
    pub fn is_fresh(&self, key: &str, ttl: Duration) -> bool {
        self.get_with_ttl(key, ttl).is_some()
    }

    pub fn clear(&self) -> Result<()> {
        if self.dir.exists() {
            std::fs::remove_dir_all(&self.dir)?;
            std::fs::create_dir_all(&self.dir)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let c = DiskCache::new(d.path().to_path_buf(), Duration::from_secs(60));
        c.put("hello", b"world").unwrap();
        assert_eq!(c.get("hello").unwrap(), b"world");
        assert_eq!(c.get_stale("hello").unwrap(), b"world");
    }

    #[test]
    fn stale_ttl_miss() {
        let d = tempfile::tempdir().unwrap();
        let c = DiskCache::new(d.path().to_path_buf(), Duration::from_millis(1));
        c.put("k", b"v").unwrap();
        std::thread::sleep(Duration::from_millis(5));
        assert!(c.get("k").is_none());
        assert!(c.get_stale("k").is_some());
    }
}
