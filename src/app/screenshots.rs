use crate::instance::config::InstanceConfig;
use crate::instance::manager::InstanceManager;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ScreenshotEntry {
    pub path: PathBuf,
    pub instance_id: String,
    pub instance_name: String,
    pub modified: SystemTime,
    pub bytes: u64,
}

pub fn scan(instances: &[InstanceConfig], manager: &InstanceManager) -> Vec<ScreenshotEntry> {
    let mut found = Vec::new();
    for instance in instances {
        let folder = manager.game_dir(&instance.id).join("screenshots");
        let Ok(files) = std::fs::read_dir(folder) else {
            continue;
        };
        for file in files.flatten() {
            if !file.file_type().is_ok_and(|kind| kind.is_file()) {
                continue;
            }
            let path = file.path();
            let supported = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "webp"
                    )
                });
            if !supported {
                continue;
            }
            let Ok(metadata) = file.metadata() else {
                continue;
            };
            found.push(ScreenshotEntry {
                path,
                instance_id: instance.id.clone(),
                instance_name: instance.name.clone(),
                modified: metadata.modified().unwrap_or(UNIX_EPOCH),
                bytes: metadata.len(),
            });
        }
    }
    found.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::config::LoaderKind;
    use crate::storage::paths::MonoryxPaths;

    #[test]
    fn scans_supported_images_across_instances() {
        let dir = tempfile::tempdir().unwrap();
        let manager = InstanceManager::new(MonoryxPaths::new(dir.path().to_path_buf()));
        let first = manager
            .create(
                "First".to_string(),
                "1.21.1".to_string(),
                LoaderKind::Vanilla,
                String::new(),
            )
            .unwrap();
        let second = manager
            .create(
                "Second".to_string(),
                "1.21.1".to_string(),
                LoaderKind::Vanilla,
                String::new(),
            )
            .unwrap();
        let first_dir = manager.game_dir(&first.id).join("screenshots");
        let second_dir = manager.game_dir(&second.id).join("screenshots");
        std::fs::write(first_dir.join("one.PNG"), b"test").unwrap();
        std::fs::write(first_dir.join("ignore.txt"), b"test").unwrap();
        std::fs::write(second_dir.join("two.jpeg"), b"test").unwrap();
        let entries = scan(&[first, second], &manager);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|entry| entry.instance_name == "First"));
        assert!(entries.iter().any(|entry| entry.instance_name == "Second"));
    }
}
