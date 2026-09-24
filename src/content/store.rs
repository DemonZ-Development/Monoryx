use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
}

impl ContentStore {
    pub fn for_instance(instance_dir: &Path) -> Self {
        Self {
            path: instance_dir.join("content.json"),
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
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(content)?;
        crate::utils::fs::atomic_write(&self.path, text.as_bytes())
    }

    pub fn upsert(&self, entry: InstalledEntry) -> Result<()> {
        let mut content = self.load_result()?;
        content
            .entries
            .retain(|e| e.file_name != entry.file_name || e.kind != entry.kind);
        content.entries.push(entry);
        content.entries.sort_by(|a, b| {
            a.project_title
                .clone()
                .unwrap_or_default()
                .to_lowercase()
                .cmp(&b.project_title.clone().unwrap_or_default().to_lowercase())
        });
        self.save(&content)
    }

    pub fn remove(&self, kind: ContentKind, file_name: &str) -> Result<()> {
        let mut content = self.load_result()?;
        content
            .entries
            .retain(|e| !(e.kind == kind && e.file_name == file_name));
        self.save(&content)
    }

    pub fn set_enabled(&self, kind: ContentKind, file_name: &str, enabled: bool) -> Result<()> {
        let mut content = self.load_result()?;
        if let Some(entry) = content
            .entries
            .iter_mut()
            .find(|e| e.kind == kind && e.file_name == file_name)
        {
            entry.enabled = enabled;
            self.save(&content)?;
        }
        Ok(())
    }

    pub fn by_project(&self, project_id: &str) -> Option<InstalledEntry> {
        self.load()
            .entries
            .into_iter()
            .find(|e| e.project_id.as_deref() == Some(project_id))
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
