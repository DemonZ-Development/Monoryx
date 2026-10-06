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

    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries().len()
    }

    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.entries().into_iter().filter_map(|(_, len)| len).sum()
    }

    fn entries(&self) -> Vec<(PathBuf, Option<u64>)> {
        let mut out = Vec::new();
        let Ok(buckets) = std::fs::read_dir(&self.dir) else {
            return out;
        };
        for bucket in buckets.flatten() {
            let Ok(files) = std::fs::read_dir(bucket.path()) else {
                continue;
            };
            for file in files.flatten() {
                let len = file.metadata().ok().map(|meta| meta.len());
                out.push((file.path(), len));
            }
        }
        out
    }

    pub fn prune_expired(&self, max_age: Duration) -> Result<usize> {
        let now = SystemTime::now();
        let mut removed = 0usize;
        for (path, _) in self.entries() {
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            let age = now.duration_since(modified).unwrap_or(Duration::ZERO);
            if age > max_age && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn evict_to_budget(&self, max_entries: usize, max_bytes: u64) -> Result<usize> {
        let mut entries = self.entries();
        if entries.len() <= max_entries
            && entries.iter().filter_map(|(_, len)| *len).sum::<u64>() <= max_bytes
        {
            return Ok(0);
        }
        let now = SystemTime::now();
        let mut stamped: Vec<(Duration, PathBuf, u64)> = entries
            .drain(..)
            .map(|(path, len)| {
                let age = std::fs::metadata(&path)
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|modified| now.duration_since(modified).ok())
                    .unwrap_or(Duration::ZERO);
                (age, path, len.unwrap_or(0))
            })
            .collect();
        stamped.sort_by_key(|(age, _, _)| *age);

        let mut count = stamped.len();
        let mut bytes: u64 = stamped.iter().map(|(_, _, len)| *len).sum();
        let mut removed = 0usize;
        while count > max_entries || bytes > max_bytes {
            let Some((_, path, len)) = stamped.pop() else {
                break;
            };
            if std::fs::remove_file(&path).is_ok() {
                count -= 1;
                bytes = bytes.saturating_sub(len);
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn path(&self, key: &str) -> PathBuf {
        self.path_for(key)
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

    #[test]
    fn expired_entries_are_reclaimed_instead_of_growing_forever() {
        let d = tempfile::tempdir().unwrap();
        let c = DiskCache::new(d.path().to_path_buf(), Duration::from_secs(3600));
        for index in 0..40 {
            c.put(&format!("key-{index}"), b"payload").unwrap();
        }
        assert_eq!(c.entry_count(), 40);
        c.prune_expired(Duration::from_nanos(1)).unwrap();
        assert_eq!(c.entry_count(), 0);
    }

    #[test]
    fn eviction_trims_to_the_oldest_entries_under_budget() {
        let d = tempfile::tempdir().unwrap();
        let c = DiskCache::new(d.path().to_path_buf(), Duration::from_secs(3600));
        for index in 0..10 {
            c.put(&format!("k{index}"), &[b'x'; 64]).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(c.entry_count(), 10);
        assert_eq!(c.total_bytes(), 640);

        let mut by_age: Vec<(std::time::SystemTime, String)> = (0..10)
            .map(|index| {
                let key = format!("k{index}");
                let modified = std::fs::metadata(c.path(&key)).unwrap().modified().unwrap();
                (modified, key)
            })
            .collect();
        by_age.sort_by_key(|(modified, _)| *modified);
        let expected: Vec<String> = by_age
            .iter()
            .rev()
            .take(4)
            .map(|(_, k)| k.clone())
            .collect();

        c.evict_to_budget(4, 640).unwrap();
        assert_eq!(c.entry_count(), 4);
        assert_eq!(c.total_bytes(), 256);
        for key in &expected {
            assert!(c.get(key).is_some(), "{key} should have survived eviction");
        }
        for (modified, key) in &by_age {
            if !expected.contains(key) {
                assert!(
                    c.get(key).is_none(),
                    "{key} is older than every survivor and should be evicted first"
                );
            }
            let _ = modified;
        }
    }

    #[test]
    fn eviction_is_a_no_op_when_within_budget() {
        let d = tempfile::tempdir().unwrap();
        let c = DiskCache::new(d.path().to_path_buf(), Duration::from_secs(3600));
        c.put("a", b"1").unwrap();
        c.put("b", b"2").unwrap();
        assert_eq!(c.evict_to_budget(10, 10_000).unwrap(), 0);
        assert_eq!(c.entry_count(), 2);
    }
}
