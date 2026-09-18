use crate::error::{MonoryxError, Result};
use std::collections::HashSet;
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

fn extract_one_jar(jar: &Path, excludes: &[String], out_dir: &Path) -> Result<usize> {
    let file = std::fs::File::open(jar)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| MonoryxError::Archive(e.to_string()))?;
    let exclude_set: HashSet<&str> = excludes.iter().map(String::as_str).collect();
    let mut count = 0usize;
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
        let dest = crate::utils::fs::safe_join(out_dir, &name)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut out)?;
        count += 1;
    }
    Ok(count)
}

#[must_use]
pub fn is_safe_entry_path(name: &str) -> bool {
    if name.starts_with('/') || name.starts_with('\\') {
        return false;
    }
    if name.contains("..") {
        return false;
    }

    if name.len() >= 3 && name.as_bytes()[1] == b':' {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_entries() {
        assert!(!is_safe_entry_path("/abs/path.so"));
        assert!(!is_safe_entry_path("../../evil.so"));
        assert!(!is_safe_entry_path("a/../../b.so"));
        assert!(!is_safe_entry_path("C:/evil.dll"));
        assert!(is_safe_entry_path("META-INF/MANIFEST.MF"));
        assert!(is_safe_entry_path("windows/x64/lwjgl.dll"));
    }
}
