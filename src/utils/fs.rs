use crate::error::{MonoryxError, Result};
use std::path::{Component, Path, PathBuf};

pub fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let tmp = path.with_extension("tmp-monoryx-part");
    std::fs::write(&tmp, bytes)?;

    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn atomic_write_str(path: &Path, content: &str) -> Result<()> {
    atomic_write(path, content.as_bytes())
}

pub fn safe_join(root: &Path, untrusted: &str) -> Result<PathBuf> {
    let normalized = untrusted.replace('\\', "/");
    let p = Path::new(&normalized);
    if p.is_absolute() {
        return Err(MonoryxError::UnsafePath(untrusted.to_string()));
    }
    let mut out = PathBuf::from(root);
    for comp in p.components() {
        match comp {
            Component::Normal(c) => out.push(c),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(MonoryxError::UnsafePath(untrusted.to_string()));
            }
        }
    }
    Ok(out)
}

pub fn safe_file_name(name: &str) -> Result<&str> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', ':', '\0'])
        || name.chars().any(char::is_control)
        || Path::new(name)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(MonoryxError::UnsafePath(name.to_string()));
    }
    Ok(name)
}

pub fn is_within_root(root: &Path, candidate: &Path) -> bool {
    if let (Ok(r), Ok(c)) = (root.canonicalize(), candidate.canonicalize()) {
        return c.starts_with(r);
    }
    let abs_root = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(root)
    };
    let abs_cand = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(candidate)
    };

    let mut r = abs_root.components();
    let mut c = abs_cand.components();
    loop {
        match (r.next(), c.next()) {
            (Some(a), Some(b)) if a == b => continue,
            (None, _) => return true,
            _ => return false,
        }
    }
}

pub fn remove_dir_inside_root(root: &Path, target: &Path) -> Result<()> {
    let root_c = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let target_c = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    if target_c == root_c || !target_c.starts_with(&root_c) {
        return Err(MonoryxError::Instance(
            "refusing to delete a directory outside the MONORYX data root".to_string(),
        ));
    }
    if target_c.exists() {
        std::fs::remove_dir_all(&target_c)?;
    }
    Ok(())
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    ensure_dir(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else if ty.is_file() {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_accepts_normal() {
        let root = Path::new("/data/instances/x");
        let p = safe_join(root, "mods/sodium.jar").unwrap();
        assert!(p.ends_with("mods/sodium.jar"));
    }

    #[test]
    fn safe_join_rejects_traversal() {
        let root = Path::new("/data");
        assert!(safe_join(root, "../../evil.exe").is_err());
        assert!(safe_join(root, "/absolute/path").is_err());
        assert!(safe_join(root, "a/../../b").is_err());
        assert!(safe_join(root, "..\\evil.exe").is_err());
    }

    #[test]
    fn remote_filename_must_be_one_file() {
        for name in [
            "../config.toml",
            "..\\config.toml",
            "/tmp/file",
            "C:evil",
            "",
            "..",
        ] {
            assert!(safe_file_name(name).is_err(), "{name}");
        }
        assert_eq!(
            safe_file_name("sodium-1.2.3.jar").unwrap(),
            "sodium-1.2.3.jar"
        );
    }

    #[test]
    fn remove_outside_root_refused() {
        let root = Path::new("/tmp/monoryx-root-test");

        let outside = Path::new("/tmp/monoryx-other-test/evil");
        assert!(remove_dir_inside_root(root, outside).is_err());
    }
}
