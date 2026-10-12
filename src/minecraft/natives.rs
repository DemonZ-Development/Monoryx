use crate::error::{MonoryxError, Result};
use std::collections::HashSet;
use std::io::Read as _;
use std::path::Path;

pub fn extract_natives(
    native_jars: &[(std::path::PathBuf, Vec<String>)],
    natives_dir: &Path,
) -> Result<usize> {
    if natives_dir.exists() {
        std::fs::remove_dir_all(natives_dir)?;
    }
    std::fs::create_dir_all(natives_dir)?;
    let mut extracted = 0usize;
    for (jar, excludes) in native_jars {
        extracted += extract_one_jar(jar, excludes, natives_dir)?;
    }
    Ok(extracted)
}

pub const MAX_NATIVE_FILES: usize = 20_000;
pub const MAX_NATIVE_BYTES: u64 = 512 * 1024 * 1024;

fn extract_one_jar(jar: &Path, excludes: &[String], out_dir: &Path) -> Result<usize> {
    let file = std::fs::File::open(jar)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| MonoryxError::Archive(e.to_string()))?;
    let exclude_set: HashSet<&str> = excludes.iter().map(String::as_str).collect();
    let mut count = 0usize;
    let mut written: u64 = 0;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| MonoryxError::Archive(e.to_string()))?;
        let name = entry.name().to_string();
        if entry.is_dir() {
            continue;
        }

        if exclude_set.iter().any(|ex| name.starts_with(ex)) {
            continue;
        }

        if entry.is_symlink() {
            tracing::warn!("skipping symlink in natives jar: {name}");
            continue;
        }
        if count >= MAX_NATIVE_FILES || written > MAX_NATIVE_BYTES {
            return Err(MonoryxError::Archive(format!(
                "natives archive {} exceeds the extraction budget",
                jar.display()
            )));
        }
        let dest = crate::utils::fs::safe_join(out_dir, &name)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let declared = entry.size();
        let mut out = std::fs::File::create(&dest)?;
        let copied = std::io::copy(
            &mut entry.by_ref().take(declared.saturating_add(1)),
            &mut out,
        )?;
        written = written.saturating_add(copied);
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_entries() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("natives.jar");
        let out = dir.path().join("out");
        let file = std::fs::File::create(&jar).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("../escape.dll", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut archive, b"native").unwrap();
        archive.finish().unwrap();
        assert!(matches!(
            extract_natives(&[(jar, Vec::new())], &out),
            Err(MonoryxError::UnsafePath(_))
        ));
        assert!(!dir.path().join("escape.dll").exists());
    }
}
