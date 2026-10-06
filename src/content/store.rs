use crate::error::{MonoryxError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    #[default]
    Mod,
    Resourcepack,
    Shader,
}

impl ContentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Resourcepack => "resourcepack",
            Self::Shader => "shader",
        }
    }
    pub const fn subdir(self) -> &'static str {
        match self {
            Self::Mod => "mods",
            Self::Resourcepack => "resourcepacks",
            Self::Shader => "shaderpacks",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledEntry {
    pub file_name: String,
    pub kind: ContentKind,
    pub project_id: Option<String>,
    pub project_slug: Option<String>,
    pub project_title: Option<String>,
    pub version_id: Option<String>,
    pub version_number: Option<String>,
    pub file_hash_sha512: Option<String>,
    pub file_hash_sha1: Option<String>,
    pub size: u64,
    pub enabled: bool,
    pub installed_at: String,
    pub loader: String,
    pub game_version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstalledContent {
    pub entries: Vec<InstalledEntry>,
}

#[derive(Debug, Clone)]
pub struct ContentStore {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl ContentStore {
    pub fn for_instance(instance_dir: &Path) -> Self {
        let lock = crate::utils::fs::path_lock(instance_dir);
        Self {
            path: instance_dir.join("content.json"),
            lock,
        }
    }

    pub fn load(&self) -> InstalledContent {
        self.load_result().unwrap_or_default()
    }

    pub fn load_result(&self) -> Result<InstalledContent> {
        if !self.path.exists() {
            return Ok(InstalledContent::default());
        }
        let text = std::fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save(&self, content: &InstalledContent) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        self.save_unlocked(content)
    }

    fn save_unlocked(&self, content: &InstalledContent) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(content)?;
        crate::utils::fs::atomic_write(&self.path, text.as_bytes())
    }

    pub fn upsert(&self, entry: InstalledEntry) -> Result<()> {
        self.replace(None, entry)
    }

    pub fn replace(&self, previous_file: Option<&str>, entry: InstalledEntry) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut content = self.load_result()?;
        content.entries.retain(|e| {
            e.kind != entry.kind
                || (e.file_name != entry.file_name && Some(e.file_name.as_str()) != previous_file)
        });
        content.entries.push(entry);
        content.entries.sort_by(|a, b| {
            a.project_title
                .clone()
                .unwrap_or_default()
                .to_lowercase()
                .cmp(&b.project_title.clone().unwrap_or_default().to_lowercase())
        });
        self.save_unlocked(&content)
    }

    pub fn install_file(
        &self,
        staged: &Path,
        entry: InstalledEntry,
        game_dir: &Path,
        disabled_dir: &Path,
        preserve_enabled: bool,
    ) -> Result<()> {
        self.install_file_with_commit(
            staged,
            entry,
            game_dir,
            disabled_dir,
            preserve_enabled,
            |content| self.save_unlocked(content),
        )
    }

    fn install_file_with_commit(
        &self,
        staged: &Path,
        mut entry: InstalledEntry,
        game_dir: &Path,
        disabled_dir: &Path,
        preserve_enabled: bool,
        commit: impl FnOnce(&InstalledContent) -> Result<()>,
    ) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        crate::utils::fs::safe_file_name(&entry.file_name)?;
        let mut content = self.load_result()?;
        let previous: Vec<_> = content
            .entries
            .iter()
            .filter(|old| {
                old.kind == entry.kind
                    && (old.file_name == entry.file_name
                        || (entry.project_id.is_some() && old.project_id == entry.project_id))
            })
            .cloned()
            .collect();
        if previous.iter().any(|old| {
            old.file_name == entry.file_name
                && old.project_id.is_some()
                && old.project_id != entry.project_id
        }) {
            return Err(MonoryxError::Instance(format!(
                "{} already belongs to another installed project",
                entry.file_name
            )));
        }
        if let Some(old) = previous.first().filter(|_| preserve_enabled) {
            entry.enabled = old.enabled;
        }
        let folder = if entry.enabled {
            game_dir.join(entry.kind.subdir())
        } else {
            disabled_dir.join(entry.kind.subdir())
        };
        std::fs::create_dir_all(&folder)?;
        let instance_dir = self
            .path
            .parent()
            .ok_or_else(|| MonoryxError::InvalidConfig("Missing instance directory".into()))?;
        if !crate::utils::fs::is_within_root(instance_dir, &folder) {
            return Err(MonoryxError::UnsafePath(folder.display().to_string()));
        }
        let target = folder.join(if entry.enabled {
            entry.file_name.clone()
        } else {
            format!("{}.disabled", entry.file_name)
        });
        if target.exists() && previous.is_empty() {
            return Err(MonoryxError::Instance(format!(
                "Refusing to overwrite untracked file {}",
                target.display()
            )));
        }
        let backup = tempfile::tempdir_in(instance_dir)?;
        let mut moved = Vec::new();
        let mut placed = false;
        let result = (|| {
            let mut candidates = vec![target.clone()];
            for old in &previous {
                crate::utils::fs::safe_file_name(&old.file_name)?;
                candidates.extend([
                    game_dir.join(old.kind.subdir()).join(&old.file_name),
                    game_dir
                        .join(old.kind.subdir())
                        .join(format!("{}.disabled", old.file_name)),
                    disabled_dir
                        .join(old.kind.subdir())
                        .join(format!("{}.disabled", old.file_name)),
                ]);
                if old.kind == ContentKind::Mod {
                    candidates.push(disabled_dir.join(format!("{}.disabled", old.file_name)));
                }
            }
            candidates.sort();
            candidates.dedup();
            for original in candidates.into_iter().filter(|path| path.exists()) {
                if !crate::utils::fs::is_within_root(instance_dir, &original) {
                    return Err(MonoryxError::UnsafePath(original.display().to_string()));
                }
                let saved = backup.path().join(moved.len().to_string());
                std::fs::rename(&original, &saved)?;
                moved.push((original, saved));
            }
            std::fs::rename(staged, &target)?;
            placed = true;
            content.entries.retain(|old| {
                !previous
                    .iter()
                    .any(|prior| prior.kind == old.kind && prior.file_name == old.file_name)
            });
            content.entries.push(entry);
            content.entries.sort_by_cached_key(|entry| {
                entry
                    .project_title
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
            });
            commit(&content)
        })();
        if let Err(error) = result {
            if placed {
                if let Err(cleanup) = std::fs::remove_file(&target) {
                    let preserved = backup.keep();
                    return Err(MonoryxError::Instance(format!("{error}; could not remove incomplete content: {cleanup}. Original files are preserved at {}", preserved.display())));
                }
            }
            for (original, saved) in &moved {
                if let Err(restore) = std::fs::rename(saved, original) {
                    let preserved = backup.keep();
                    return Err(MonoryxError::Instance(format!("{error}; could not restore content: {restore}. Original files are preserved at {}", preserved.display())));
                }
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn remove(&self, kind: ContentKind, file_name: &str) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut content = self.load_result()?;
        content
            .entries
            .retain(|e| !(e.kind == kind && e.file_name == file_name));
        self.save_unlocked(&content)
    }

    pub fn set_enabled(&self, kind: ContentKind, file_name: &str, enabled: bool) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut content = self.load_result()?;
        if let Some(entry) = content
            .entries
            .iter_mut()
            .find(|e| e.kind == kind && e.file_name == file_name)
        {
            entry.enabled = enabled;
            self.save_unlocked(&content)?;
        }
        Ok(())
    }

    pub fn project_ids(&self) -> Vec<String> {
        self.load()
            .entries
            .into_iter()
            .filter_map(|e| e.project_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn id_to_name(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        for e in self.load().entries {
            if let (Some(id), Some(title)) = (e.project_id, e.project_title) {
                m.insert(id, title);
            }
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(file_name: String) -> InstalledEntry {
        InstalledEntry {
            file_name,
            kind: ContentKind::Mod,
            project_id: None,
            project_slug: None,
            project_title: None,
            version_id: None,
            version_number: None,
            file_hash_sha512: None,
            file_hash_sha1: None,
            size: 0,
            enabled: true,
            installed_at: String::new(),
            loader: String::new(),
            game_version: String::new(),
        }
    }

    #[test]
    fn concurrent_independent_stores_retain_every_entry() {
        let dir = tempfile::tempdir().unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(8));
        std::thread::scope(|scope| {
            for worker in 0..8 {
                let path = dir.path();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    let store = ContentStore::for_instance(path);
                    barrier.wait();
                    for index in 0..20 {
                        store
                            .upsert(entry(format!("{worker}-{index}.jar")))
                            .unwrap();
                    }
                });
            }
        });
        assert_eq!(
            ContentStore::for_instance(dir.path())
                .load_result()
                .unwrap()
                .entries
                .len(),
            160
        );
    }

    #[test]
    fn replacement_preserves_unrelated_entries() {
        let dir = tempfile::tempdir().unwrap();
        let store = ContentStore::for_instance(dir.path());
        store.upsert(entry("old.jar".into())).unwrap();
        store.upsert(entry("other.jar".into())).unwrap();
        store
            .replace(Some("old.jar"), entry("new.jar".into()))
            .unwrap();
        let names: Vec<_> = store
            .load()
            .entries
            .into_iter()
            .map(|e| e.file_name)
            .collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"new.jar".into()) && names.contains(&"other.jar".into()));
    }

    #[test]
    fn staged_install_preserves_disabled_state_and_replaces_old_version() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let disabled = game.join(".monoryx-disabled");
        std::fs::create_dir_all(disabled.join("mods")).unwrap();
        let store = ContentStore::for_instance(dir.path());
        let mut old = entry("old.jar".into());
        old.project_id = Some("project".into());
        old.enabled = false;
        store.upsert(old).unwrap();
        std::fs::write(disabled.join("mods/old.jar.disabled"), b"old").unwrap();
        let staged = dir.path().join("download");
        std::fs::write(&staged, b"new").unwrap();
        let mut new = entry("new.jar".into());
        new.project_id = Some("project".into());
        store
            .install_file(&staged, new, &game, &disabled, true)
            .unwrap();
        assert_eq!(
            std::fs::read(disabled.join("mods/new.jar.disabled")).unwrap(),
            b"new"
        );
        assert!(!disabled.join("mods/old.jar.disabled").exists());
        assert!(!game.join("mods/new.jar").exists());
        assert_eq!(store.load().entries.len(), 1);
        assert!(!store.load().entries[0].enabled);
    }

    #[test]
    fn failed_staged_install_restores_previous_file_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(game.join("mods")).unwrap();
        let store = ContentStore::for_instance(dir.path());
        store.upsert(entry("same.jar".into())).unwrap();
        std::fs::write(game.join("mods/same.jar"), b"original").unwrap();
        let metadata = std::fs::read(&store.path).unwrap();
        assert!(store
            .install_file(
                &dir.path().join("missing"),
                entry("same.jar".into()),
                &game,
                &game.join(".monoryx-disabled"),
                true
            )
            .is_err());
        assert_eq!(
            std::fs::read(game.join("mods/same.jar")).unwrap(),
            b"original"
        );
        assert_eq!(std::fs::read(&store.path).unwrap(), metadata);
    }

    #[test]
    fn metadata_commit_failure_rolls_back_a_placed_download() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(game.join("mods")).unwrap();
        let store = ContentStore::for_instance(dir.path());
        store.upsert(entry("same.jar".into())).unwrap();
        std::fs::write(game.join("mods/same.jar"), b"original").unwrap();
        let metadata = std::fs::read(&store.path).unwrap();
        let staged = dir.path().join("download");
        std::fs::write(&staged, b"replacement").unwrap();
        let result = store.install_file_with_commit(
            &staged,
            entry("same.jar".into()),
            &game,
            &game.join(".monoryx-disabled"),
            true,
            |_| {
                assert_eq!(
                    std::fs::read(game.join("mods/same.jar")).unwrap(),
                    b"replacement"
                );
                Err(std::io::Error::other("metadata disk write failed").into())
            },
        );
        assert!(result.is_err());
        assert_eq!(
            std::fs::read(game.join("mods/same.jar")).unwrap(),
            b"original"
        );
        assert_eq!(std::fs::read(&store.path).unwrap(), metadata);
    }

    #[test]
    fn corrupt_content_metadata_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let store = ContentStore::for_instance(dir.path());
        std::fs::write(&store.path, b"not json").unwrap();
        assert!(store.load_result().is_err());
        assert!(store.remove(ContentKind::Mod, "any.jar").is_err());
        assert_eq!(std::fs::read(&store.path).unwrap(), b"not json");
    }
}
