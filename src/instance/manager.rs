use crate::error::{MonoryxError, Result};
use crate::instance::config::{InstanceConfig, LoaderKind};
use crate::storage::paths::MonoryxPaths;
use std::path::{Path, PathBuf};

pub const GAME_SUBDIRS: &[&str] = &[
    "saves",
    "mods",
    ".monoryx-disabled",
    "config",
    "resourcepacks",
    "shaderpacks",
    "screenshots",
    "logs",
    "crash-reports",
];

#[derive(Debug, Clone)]
pub struct InstanceManager {
    paths: MonoryxPaths,
}

impl InstanceManager {
    #[must_use]
    pub fn new(paths: MonoryxPaths) -> Self {
        Self { paths }
    }

    #[must_use]
    pub fn paths(&self) -> &MonoryxPaths {
        &self.paths
    }

    pub fn list(&self) -> Result<Vec<InstanceConfig>> {
        let dir = self.paths.instances_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let cfg_path = entry.path().join("instance.toml");
            if !cfg_path.exists() {
                continue;
            }
            match load_config(&cfg_path) {
                Ok(c) => out.push(c),
                Err(e) => {
                    tracing::warn!("skipping broken instance {}: {e}", entry.path().display())
                }
            }
        }
        out.sort_by(|a, b| natural_name_cmp(&a.name, &b.name));
        Ok(out)
    }

    pub fn get(&self, id: &str) -> Result<InstanceConfig> {
        load_config(&self.config_path(id))
    }

    pub fn create(
        &self,
        name: String,
        minecraft_version: String,
        loader: LoaderKind,
        loader_version: String,
    ) -> Result<InstanceConfig> {
        crate::utils::validation::validate_instance_name(&name)?;
        let cfg = InstanceConfig::new(name, minecraft_version, loader, loader_version);
        cfg.validate()?;
        let dir = self.instance_dir(&cfg.id);
        let result = (|| {
            self.ensure_game_dirs(&cfg.id)?;
            self.save(&cfg)
        })();
        if let Err(error) = result {
            return Err(self.rollback_creation(&dir, error));
        }
        Ok(cfg)
    }

    pub fn save(&self, cfg: &InstanceConfig) -> Result<()> {
        cfg.validate()?;
        let text = toml::to_string_pretty(cfg).map_err(|e| MonoryxError::TomlSer(e.to_string()))?;
        let path = self.config_path(&cfg.id);
        if path.exists() {
            let previous = std::fs::read_to_string(&path)?;
            if toml::from_str::<InstanceConfig>(&previous).is_ok() {
                crate::storage::atomic::atomic_write_str(&backup_path(&path), &previous)?;
            }
        }
        crate::storage::atomic::atomic_write_str(&path, &text)
    }

    pub fn rename(&self, id: &str, new_name: String) -> Result<InstanceConfig> {
        let mut cfg = self.get(id)?;
        crate::utils::validation::validate_instance_name(&new_name)?;
        cfg.name = new_name.trim().to_string();
        self.save(&cfg)?;
        Ok(cfg)
    }

    pub fn duplicate(&self, id: &str, new_name: String) -> Result<InstanceConfig> {
        let mut copy = self.get(id)?;
        crate::utils::validation::validate_instance_name(&new_name)?;
        copy.id = uuid::Uuid::new_v4().to_string();
        copy.name = new_name.trim().to_string();
        copy.created_at = chrono::Utc::now().to_rfc3339();
        copy.last_played_at = None;
        copy.total_plays = 0;
        copy.play_time_secs = 0;
        copy.validate()?;
        let dst_dir = self.instance_dir(&copy.id);
        let result = (|| {
            self.ensure_game_dirs(&copy.id)?;
            let src_game = self.game_dir(id);
            if src_game.exists() {
                crate::utils::fs::copy_dir_recursive(&src_game, &self.game_dir(&copy.id))?;
            }
            let src_content = self.instance_dir(id).join("content.json");
            if src_content.exists() {
                std::fs::copy(src_content, dst_dir.join("content.json"))?;
            }
            self.save(&copy)
        })();
        if let Err(error) = result {
            return Err(self.rollback_creation(&dst_dir, error));
        }
        Ok(copy)
    }

    fn rollback_creation(&self, dir: &Path, error: MonoryxError) -> MonoryxError {
        match crate::utils::fs::remove_dir_inside_root(self.paths.root(), dir) {
            Ok(()) => error,
            Err(cleanup) => MonoryxError::Instance(format!(
                "{error}; could not remove incomplete instance at {}: {cleanup}",
                dir.display()
            )),
        }
    }

    pub fn delete(&self, id: &str, delete_files: bool) -> Result<()> {
        let dir = self.instance_dir(id);
        if delete_files {
            crate::utils::fs::remove_dir_inside_root(self.paths.root(), &dir)?;
        } else {
            let cfg = self.config_path(id);
            if cfg.exists() {
                std::fs::remove_file(cfg)?;
            }
        }
        Ok(())
    }

    pub fn mark_played(&self, id: &str) -> Result<()> {
        let mut cfg = self.get(id)?;
        cfg.last_played_at = Some(chrono::Utc::now().to_rfc3339());
        cfg.total_plays += 1;
        self.save(&cfg)
    }

    pub fn add_play_time(&self, id: &str, secs: u64) -> Result<()> {
        let mut cfg = self.get(id)?;
        cfg.play_time_secs = cfg.play_time_secs.saturating_add(secs);
        self.save(&cfg)
    }

    #[must_use]
    pub fn instance_dir(&self, id: &str) -> PathBuf {
        self.paths.instance_dir(id)
    }

    #[must_use]
    pub fn config_path(&self, id: &str) -> PathBuf {
        self.instance_dir(id).join("instance.toml")
    }

    #[must_use]
    pub fn game_dir(&self, id: &str) -> PathBuf {
        self.instance_dir(id).join("game")
    }

    #[must_use]
    pub fn mods_dir(&self, id: &str) -> PathBuf {
        self.game_dir(id).join("mods")
    }

    #[must_use]
    pub fn disabled_dir(&self, id: &str) -> PathBuf {
        self.game_dir(id).join(".monoryx-disabled")
    }

    #[must_use]
    pub fn resourcepacks_dir(&self, id: &str) -> PathBuf {
        self.game_dir(id).join("resourcepacks")
    }

    #[must_use]
    pub fn shaderpacks_dir(&self, id: &str) -> PathBuf {
        self.game_dir(id).join("shaderpacks")
    }

    pub fn ensure_game_dirs(&self, id: &str) -> Result<PathBuf> {
        let game = self.game_dir(id);
        std::fs::create_dir_all(&game)?;
        for sub in GAME_SUBDIRS {
            std::fs::create_dir_all(game.join(sub))?;
        }
        Ok(game)
    }

    pub fn sync_offline_skin(&self, skins_dir: &Path, username: &str, id: &str) -> Result<()> {
        let game = self.game_dir(id);
        crate::ui::skins::sync_instance_skins(skins_dir, &game, username)
            .map_err(MonoryxError::Instance)
    }

    pub fn sync_offline_skin_all(&self, skins_dir: &Path, username: &str) -> Result<usize> {
        let instances = self.list()?;
        let mut count = 0;
        for instance in &instances {
            let game = self.game_dir(&instance.id);
            if crate::ui::skins::sync_instance_skins(skins_dir, &game, username).is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    #[must_use]
    pub fn mod_count(&self, id: &str) -> usize {
        let mods = self.mods_dir(id);
        std::fs::read_dir(mods)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("jar"))
                    .count()
            })
            .unwrap_or(0)
    }

    pub fn set_mod_enabled(&self, id: &str, file_name: &str, enabled: bool) -> Result<PathBuf> {
        self.set_content_enabled(id, file_name, crate::content::ContentKind::Mod, enabled)
    }

    pub fn set_content_enabled(
        &self,
        id: &str,
        file_name: &str,
        kind: crate::content::ContentKind,
        enabled: bool,
    ) -> Result<PathBuf> {
        crate::utils::fs::safe_file_name(file_name)?;
        let mods = self.game_dir(id).join(kind.subdir());
        let disabled = self.disabled_dir(id).join(kind.subdir());
        let old_disabled = self.disabled_dir(id);
        std::fs::create_dir_all(&disabled)?;

        let base = file_name.trim_end_matches(".disabled");
        let enabled_path = mods.join(base);
        let disabled_path = disabled.join(format!("{base}.disabled"));
        if enabled {
            if disabled_path.exists() {
                std::fs::rename(&disabled_path, &enabled_path)?;
                return Ok(enabled_path);
            }

            if kind == crate::content::ContentKind::Mod {
                let legacy = old_disabled.join(format!("{base}.disabled"));
                if legacy.exists() {
                    std::fs::rename(&legacy, &enabled_path)?;
                    return Ok(enabled_path);
                }
            }

            let in_place = mods.join(format!("{base}.disabled"));
            if in_place.exists() {
                std::fs::rename(&in_place, &enabled_path)?;
                return Ok(enabled_path);
            }
            if enabled_path.exists() {
                return Ok(enabled_path);
            }
            return Err(MonoryxError::Instance(format!(
                "mod not found: {file_name}"
            )));
        }
        if enabled_path.exists() {
            std::fs::rename(&enabled_path, &disabled_path)?;
            return Ok(disabled_path);
        }
        let in_place = mods.join(format!("{base}.disabled"));
        if in_place.exists() {
            std::fs::rename(&in_place, &disabled_path)?;
            return Ok(disabled_path);
        }
        if disabled_path.exists() {
            return Ok(disabled_path);
        }
        Err(MonoryxError::Instance(format!(
            "mod not found: {file_name}"
        )))
    }
}

fn backup_path(path: &Path) -> std::path::PathBuf {
    path.with_extension("toml.bak")
}

fn recover_config(path: &Path, damaged: &str, error: &toml::de::Error) -> Result<InstanceConfig> {
    let backup = backup_path(path);
    let Ok(backup_text) = std::fs::read_to_string(&backup) else {
        return Err(MonoryxError::TomlDe(format!(
            "{error}. Instance settings were preserved at {}",
            path.display()
        )));
    };
    let Ok(recovered) = toml::from_str::<InstanceConfig>(&backup_text) else {
        return Err(MonoryxError::TomlDe(format!(
            "{error}. Instance settings and its backup were preserved at {}",
            path.display()
        )));
    };
    let preserved = path.with_extension(format!("corrupt-{}.toml", uuid::Uuid::new_v4()));
    std::fs::write(&preserved, damaged)?;
    crate::storage::atomic::atomic_write_str(path, &backup_text)?;
    tracing::warn!(
        "Damaged instance settings preserved at {} and recovered from backup",
        preserved.display()
    );
    Ok(recovered)
}

fn load_config(path: &Path) -> Result<InstanceConfig> {
    let text = std::fs::read_to_string(path)?;
    match toml::from_str::<InstanceConfig>(&text) {
        Ok(cfg) => Ok(cfg),
        Err(error) => recover_config(path, &text, &error),
    }
}

fn natural_name_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let left = left.to_lowercase();
    let right = right.to_lowercase();
    let mut a = left.as_bytes().iter().copied().peekable();
    let mut b = right.as_bytes().iter().copied().peekable();
    loop {
        match (a.peek(), b.peek()) {
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut a_digits = Vec::new();
                let mut b_digits = Vec::new();
                while a.peek().is_some_and(u8::is_ascii_digit) {
                    a_digits.push(a.next().unwrap());
                }
                while b.peek().is_some_and(u8::is_ascii_digit) {
                    b_digits.push(b.next().unwrap());
                }
                let a_number = a_digits
                    .iter()
                    .skip_while(|&&c| c == b'0')
                    .copied()
                    .collect::<Vec<_>>();
                let b_number = b_digits
                    .iter()
                    .skip_while(|&&c| c == b'0')
                    .copied()
                    .collect::<Vec<_>>();
                let cmp = a_number
                    .len()
                    .cmp(&b_number.len())
                    .then_with(|| a_number.cmp(&b_number));
                if cmp != Ordering::Equal {
                    return cmp;
                }
                let cmp = a_digits.len().cmp(&b_digits.len());
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            (Some(_), Some(_)) => {
                let cmp = a.next().cmp(&b.next());
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> (tempfile::TempDir, InstanceManager) {
        let d = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(d.path().to_path_buf());
        paths.ensure_all().unwrap();
        (d, InstanceManager::new(paths))
    }

    #[test]
    fn create_list_rename_roundtrip() {
        let (_d, m) = manager();
        let c = m
            .create(
                "Perf".into(),
                "1.21".into(),
                LoaderKind::Fabric,
                "0.16.9".into(),
            )
            .unwrap();
        assert_eq!(m.list().unwrap().len(), 1);
        let r = m.rename(&c.id, "Fast".into()).unwrap();
        assert_eq!(r.name, "Fast");
    }

    #[test]
    fn damaged_instance_settings_recover_from_backup_instead_of_vanishing() {
        let (d, m) = manager();
        let mut cfg = m
            .create(
                "Keep Me".into(),
                "1.21".into(),
                LoaderKind::Fabric,
                "0.16.9".into(),
            )
            .unwrap();
        cfg.memory_max_mb = 6144;
        m.save(&cfg).unwrap();
        m.save(&cfg).unwrap();
        let path = m.config_path(&cfg.id);
        std::fs::write(&path, "name = \"Keep Me\"\nthis is not = = valid toml").unwrap();

        let listed = m.list().unwrap();
        assert_eq!(listed.len(), 1, "instance must not disappear");
        assert_eq!(listed[0].id, cfg.id);
        assert_eq!(listed[0].memory_max_mb, 6144);
        assert_eq!(listed[0].name, "Keep Me");

        let salvaged: Vec<_> = std::fs::read_dir(d.path().join("instances").join(&cfg.id))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("instance.corrupt-"))
            .collect();
        assert_eq!(salvaged.len(), 1, "damaged file must be preserved");
    }

    #[test]
    fn damaged_instance_without_backup_is_reported_not_deleted() {
        let (d, m) = manager();
        let cfg = m
            .create(
                "Only Copy".into(),
                "1.21".into(),
                LoaderKind::Vanilla,
                String::new(),
            )
            .unwrap();
        let path = m.config_path(&cfg.id);
        let _ = std::fs::remove_file(backup_path(&path));
        std::fs::write(&path, "totally broken = = =").unwrap();
        assert!(m.list().unwrap().is_empty());
        assert!(path.exists(), "damaged file must stay on disk");
        assert!(d.path().join("instances").join(&cfg.id).exists());
    }

    #[test]
    fn duplicate_gets_new_id() {
        let (_d, m) = manager();
        let c = m
            .create(
                "A".into(),
                "1.21".into(),
                LoaderKind::Vanilla,
                String::new(),
            )
            .unwrap();
        let d2 = m.duplicate(&c.id, "B".into()).unwrap();
        assert_ne!(c.id, d2.id);
        assert_eq!(m.list().unwrap().len(), 2);
    }

    #[test]
    fn duplicate_preserves_game_files_and_installed_content_metadata() {
        let (_dir, manager) = manager();
        let mut original = manager
            .create(
                "Original".into(),
                "1.21.1".into(),
                LoaderKind::Fabric,
                "0.16.9".into(),
            )
            .unwrap();
        original.total_plays = 5;
        original.play_time_secs = 600;
        original.last_played_at = Some(chrono::Utc::now().to_rfc3339());
        manager.save(&original).unwrap();
        let original_content = manager.instance_dir(&original.id).join("content.json");
        let metadata = r#"{"entries":[{"file_name":"fixture.jar","kind":"mod","project_id":"fixture","project_slug":"fixture-mod","project_title":"Fixture Mod","version_id":"version","version_number":"1.0","file_hash_sha512":null,"file_hash_sha1":null,"size":4,"enabled":false,"installed_at":"2026-10-09T00:00:00Z","loader":"fabric","game_version":"1.21.1"}]}"#;
        std::fs::write(&original_content, metadata).unwrap();
        let disabled = manager.disabled_dir(&original.id).join("mods");
        std::fs::create_dir_all(&disabled).unwrap();
        std::fs::write(disabled.join("fixture.jar.disabled"), b"test").unwrap();
        let duplicate = manager.duplicate(&original.id, "Duplicate".into()).unwrap();
        assert_eq!(
            std::fs::read_to_string(manager.instance_dir(&duplicate.id).join("content.json"))
                .unwrap(),
            metadata
        );
        assert_eq!(
            std::fs::read(
                manager
                    .disabled_dir(&duplicate.id)
                    .join("mods/fixture.jar.disabled")
            )
            .unwrap(),
            b"test"
        );
        assert_eq!(duplicate.total_plays, 0);
        assert_eq!(duplicate.play_time_secs, 0);
        assert_eq!(duplicate.last_played_at, None);
    }

    #[test]
    fn failed_duplicate_removes_its_partial_directory() {
        let (_dir, manager) = manager();
        let original = manager
            .create(
                "Original".into(),
                "1.21.1".into(),
                LoaderKind::Vanilla,
                String::new(),
            )
            .unwrap();
        std::fs::write(
            manager.game_dir(&original.id).join("options.txt"),
            b"settings",
        )
        .unwrap();
        std::fs::create_dir(manager.instance_dir(&original.id).join("content.json")).unwrap();
        assert!(manager.duplicate(&original.id, "Duplicate".into()).is_err());
        assert_eq!(
            std::fs::read_dir(manager.paths.instances_dir())
                .unwrap()
                .count(),
            1
        );
        assert_eq!(manager.list().unwrap().len(), 1);
        assert!(manager.game_dir(&original.id).join("options.txt").exists());
    }

    #[test]
    fn delete_refuses_outside_root() {
        let (_d, m) = manager();

        let c = m
            .create(
                "X".into(),
                "1.21".into(),
                LoaderKind::Vanilla,
                String::new(),
            )
            .unwrap();
        m.delete(&c.id, true).unwrap();
        assert!(m.list().unwrap().is_empty());

        assert!(m.delete("../../evil", true).is_err() || m.list().unwrap().is_empty());
    }

    #[test]
    fn enable_disable_moves_files() {
        let (_d, m) = manager();
        let c = m
            .create("M".into(), "1.21".into(), LoaderKind::Fabric, String::new())
            .unwrap();
        m.ensure_game_dirs(&c.id).unwrap();
        std::fs::write(m.mods_dir(&c.id).join("sodium.jar"), b"fake").unwrap();
        m.set_mod_enabled(&c.id, "sodium.jar", false).unwrap();
        assert!(!m.mods_dir(&c.id).join("sodium.jar").exists());
        m.set_mod_enabled(&c.id, "sodium.jar", true).unwrap();
        assert!(m.mods_dir(&c.id).join("sodium.jar").exists());
    }

    #[test]
    fn names_sort_numbers_naturally() {
        let mut names = [
            "Placeholder 10",
            "Placeholder 2",
            "placeholder 1",
            "Placeholder 9",
        ];
        names.sort_by(|a, b| natural_name_cmp(a, b));
        assert_eq!(
            names,
            [
                "placeholder 1",
                "Placeholder 2",
                "Placeholder 9",
                "Placeholder 10"
            ]
        );
    }

    #[test]
    fn sync_offline_skins_across_instances() {
        let (d, m) = manager();
        let inst1 = m
            .create(
                "Inst1".into(),
                "1.21".into(),
                LoaderKind::Fabric,
                String::new(),
            )
            .unwrap();
        let inst2 = m
            .create(
                "Inst2".into(),
                "1.20".into(),
                LoaderKind::Forge,
                String::new(),
            )
            .unwrap();

        let skins_dir = d.path().join("skins");
        std::fs::create_dir_all(&skins_dir).unwrap();
        std::fs::write(skins_dir.join("AlexPlayer.png"), b"alex_data").unwrap();

        let count = m.sync_offline_skin_all(&skins_dir, "AlexPlayer").unwrap();
        assert_eq!(count, 2);

        let csl1 = m
            .game_dir(&inst1.id)
            .join("CustomSkinLoader")
            .join("LocalSkin")
            .join("skins")
            .join("AlexPlayer.png");
        assert!(csl1.is_file());
        assert_eq!(std::fs::read(csl1).unwrap(), b"alex_data");

        let csl2 = m
            .game_dir(&inst2.id)
            .join("CustomSkinLoader")
            .join("LocalSkin")
            .join("skins")
            .join("AlexPlayer.png");
        assert!(csl2.is_file());
        assert_eq!(std::fs::read(csl2).unwrap(), b"alex_data");
    }
}
