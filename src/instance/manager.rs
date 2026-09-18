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
        out.sort_by_key(|a| a.name.to_lowercase());
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
        std::fs::create_dir_all(dir.join("game"))?;
        for sub in GAME_SUBDIRS {
            std::fs::create_dir_all(dir.join("game").join(sub))?;
        }
        self.save(&cfg)?;
        Ok(cfg)
    }

    pub fn save(&self, cfg: &InstanceConfig) -> Result<()> {
        cfg.validate()?;
        let text = toml::to_string_pretty(cfg).map_err(|e| MonoryxError::TomlSer(e.to_string()))?;
        crate::storage::atomic::atomic_write_str(&self.config_path(&cfg.id), &text)
    }

    pub fn rename(&self, id: &str, new_name: String) -> Result<InstanceConfig> {
        let mut cfg = self.get(id)?;
        crate::utils::validation::validate_instance_name(&new_name)?;
        cfg.name = new_name.trim().to_string();
        self.save(&cfg)?;
        Ok(cfg)
    }

    pub fn duplicate(&self, id: &str, new_name: String) -> Result<InstanceConfig> {
        let src = self.get(id)?;
        crate::utils::validation::validate_instance_name(&new_name)?;
        let mut copy = src.clone();
        copy.id = uuid::Uuid::new_v4().to_string();
        copy.name = new_name.trim().to_string();
        copy.created_at = chrono::Utc::now().to_rfc3339();
        copy.last_played_at = None;
        copy.total_plays = 0;
        copy.play_time_secs = 0;
        let dst_dir = self.instance_dir(&copy.id);
        std::fs::create_dir_all(&dst_dir)?;

        let src_game = self.game_dir(id);
        if src_game.exists() {
            crate::utils::fs::copy_dir_recursive(&src_game, &self.game_dir(&copy.id))?;
        }
        self.save(&copy)?;
        Ok(copy)
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
        if file_name.contains(['/', '\\', '.'])
            && (file_name.contains('/') || file_name.contains('\\'))
        {
            return Err(MonoryxError::Instance("invalid mod file name".to_string()));
        }
        let mods = self.mods_dir(id);
        let disabled = self.disabled_dir(id);
        std::fs::create_dir_all(&disabled)?;

        let base = file_name.trim_end_matches(".disabled");
        let enabled_path = mods.join(base);
        let disabled_path = disabled.join(format!("{base}.disabled"));
        if enabled {
            if disabled_path.exists() {
                std::fs::rename(&disabled_path, &enabled_path)?;
                return Ok(enabled_path);
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

fn load_config(path: &Path) -> Result<InstanceConfig> {
    let text = std::fs::read_to_string(path)?;
    toml::from_str(&text).map_err(|e| MonoryxError::TomlDe(e.to_string()))
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
}
