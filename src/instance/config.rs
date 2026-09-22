use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    #[default]
    Vanilla,
    Fabric,
    Quilt,
    Neoforge,
    Forge,
}

impl LoaderKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vanilla => "vanilla",
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
            Self::Neoforge => "neoforge",
            Self::Forge => "forge",
        }
    }

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Fabric => "Fabric",
            Self::Quilt => "Quilt",
            Self::Neoforge => "NeoForge",
            Self::Forge => "Forge",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fabric" => Self::Fabric,
            "quilt" => Self::Quilt,
            "neoforge" | "neo-forge" => Self::Neoforge,
            "forge" => Self::Forge,
            _ => Self::Vanilla,
        }
    }

    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Vanilla,
            Self::Fabric,
            Self::Quilt,
            Self::Neoforge,
            Self::Forge,
        ]
    }

    #[must_use]
    pub const fn mods_dir_name(self) -> &'static str {
        "mods"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum JavaMode {
    #[default]
    Automatic,
    System,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub minecraft_version: String,
    pub loader: LoaderKind,
    #[serde(default)]
    pub loader_version: String,

    #[serde(default)]
    pub resolved_version_id: String,
    #[serde(default = "default_min")]
    pub memory_min_mb: u64,
    #[serde(default = "default_max")]
    pub memory_max_mb: u64,
    #[serde(default)]
    pub java_mode: JavaMode,
    #[serde(default)]
    pub java_path: String,
    #[serde(default)]
    pub jvm_args: String,
    #[serde(default)]
    pub game_args: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub last_played_at: Option<String>,
    #[serde(default)]
    pub total_plays: u64,
    #[serde(default)]
    pub play_time_secs: u64,
    #[serde(default)]
    pub boost_mode: Option<bool>,
}

fn default_min() -> u64 {
    512
}
fn default_max() -> u64 {
    crate::utils::system::default_max_memory_mb()
}

impl InstanceConfig {
    #[must_use]
    pub fn new(
        name: String,
        minecraft_version: String,
        loader: LoaderKind,
        loader_version: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            icon: "cube".to_string(),
            minecraft_version,
            loader,
            loader_version,
            resolved_version_id: String::new(),
            memory_min_mb: default_min(),
            memory_max_mb: crate::utils::system::default_max_memory_mb(),
            java_mode: JavaMode::Automatic,
            java_path: String::new(),
            jvm_args: String::new(),
            game_args: String::new(),
            width: None,
            height: None,
            fullscreen: false,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_played_at: None,
            total_plays: 0,
            play_time_secs: 0,
            boost_mode: None,
        }
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        crate::utils::validation::validate_instance_name(&self.name)?;
        crate::utils::system::validate_memory(self.memory_min_mb, self.memory_max_mb)
            .map_err(crate::error::MonoryxError::InvalidConfig)?;
        if self.minecraft_version.trim().is_empty() {
            return Err(crate::error::MonoryxError::InvalidConfig(
                "Minecraft version must be set".to_string(),
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn game_dir_name(&self) -> &'static str {
        "game"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip_toml() {
        let c = InstanceConfig::new(
            "Perf".into(),
            "1.21".into(),
            LoaderKind::Fabric,
            "0.16.9".into(),
        );
        let s = toml::to_string_pretty(&c).unwrap();
        let back: InstanceConfig = toml::from_str(&s).unwrap();
        assert_eq!(back.id, c.id);
        assert_eq!(back.loader, LoaderKind::Fabric);
    }

    #[test]
    fn loader_parse() {
        assert_eq!(LoaderKind::parse("FABRIC"), LoaderKind::Fabric);
        assert_eq!(LoaderKind::parse("neo-forge"), LoaderKind::Neoforge);
        assert_eq!(LoaderKind::parse("unknown"), LoaderKind::Vanilla);
    }

    #[test]
    fn instance_boost_mode_roundtrip() {
        let mut c = InstanceConfig::new(
            "Test".into(),
            "1.21".into(),
            LoaderKind::Vanilla,
            String::new(),
        );
        assert_eq!(c.boost_mode, None);
        c.boost_mode = Some(true);
        let s = toml::to_string_pretty(&c).unwrap();
        let back: InstanceConfig = toml::from_str(&s).unwrap();
        assert_eq!(back.boost_mode, Some(true));
    }
}
