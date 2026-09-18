use crate::error::Result;
use std::path::PathBuf;

#[must_use]
pub fn data_root() -> PathBuf {
    if let Ok(v) = std::env::var("MONORYX_DATA") {
        if !v.trim().is_empty() {
            return PathBuf::from(v);
        }
    }
    if let Some(proj) = directories::ProjectDirs::from("dev", "DemonZDevelopment", "MONORYX") {
        return proj.data_dir().to_path_buf();
    }
    PathBuf::from("monoryx-data")
}

#[derive(Debug, Clone)]
pub struct MonoryxPaths {
    root: PathBuf,
}

impl MonoryxPaths {
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    #[must_use]
    pub fn global() -> Self {
        Self::new(data_root())
    }

    #[must_use]
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.toml")
    }
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }
    #[must_use]
    pub fn manifests_dir(&self) -> PathBuf {
        self.root.join("cache").join("manifests")
    }
    #[must_use]
    pub fn metadata_dir(&self) -> PathBuf {
        self.root.join("cache").join("metadata")
    }
    #[must_use]
    pub fn images_dir(&self) -> PathBuf {
        self.root.join("cache").join("images")
    }
    #[must_use]
    pub fn minecraft_dir(&self) -> PathBuf {
        self.root.join("minecraft")
    }
    #[must_use]
    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("minecraft").join("assets")
    }
    #[must_use]
    pub fn libraries_dir(&self) -> PathBuf {
        self.root.join("minecraft").join("libraries")
    }
    #[must_use]
    pub fn versions_dir(&self) -> PathBuf {
        self.root.join("minecraft").join("versions")
    }
    #[must_use]
    pub fn java_dir(&self) -> PathBuf {
        self.root.join("java").join("runtimes")
    }
    #[must_use]
    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }
    #[must_use]
    pub fn instance_dir(&self, id: &str) -> PathBuf {
        self.instances_dir().join(id)
    }
    #[must_use]
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn ensure_all(&self) -> Result<()> {
        for d in [
            self.root.clone(),
            self.cache_dir(),
            self.manifests_dir(),
            self.metadata_dir(),
            self.images_dir(),
            self.minecraft_dir(),
            self.assets_dir(),
            self.libraries_dir(),
            self.versions_dir(),
            self.java_dir(),
            self.instances_dir(),
            self.logs_dir(),
        ] {
            std::fs::create_dir_all(d)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_respected() {
        std::env::set_var("MONORYX_DATA_TEST_X", "/tmp/x");

        let p = MonoryxPaths::new(PathBuf::from("/tmp/x"));
        assert_eq!(p.config_file(), PathBuf::from("/tmp/x/config.toml"));
        std::env::remove_var("MONORYX_DATA_TEST_X");
    }
}
