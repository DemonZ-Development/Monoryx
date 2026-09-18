use crate::account::offline::OfflineProfile;
use crate::error::{MonoryxError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CloseAction {
    Never,
    Minimize,
    #[default]
    #[serde(alias = "close")]
    Hide,
}

impl CloseAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Never => "Never",
            Self::Minimize => "Minimize",
            Self::Hide => "Hide and restore after game exits",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GpuPreference {
    #[default]
    System,
    HighPerformance,
    PowerSaving,
}

impl GpuPreference {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::HighPerformance => "High performance",
            Self::PowerSaving => "Power saving",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDefaults {
    #[serde(default = "crate::utils::system::default_min_memory_mb")]
    pub min_mb: u64,
    #[serde(default = "crate::utils::system::default_max_memory_mb")]
    pub max_mb: u64,
}

impl Default for MemoryDefaults {
    fn default() -> Self {
        Self {
            min_mb: crate::utils::system::default_min_memory_mb(),
            max_mb: crate::utils::system::default_max_memory_mb(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JavaDefaults {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub custom_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default)]
    pub profile: Option<OfflineProfile>,
    #[serde(default)]
    pub close_action: CloseAction,
    #[serde(default = "default_true")]
    pub remember_instance: bool,
    #[serde(default)]
    pub selected_instance: Option<String>,
    #[serde(default = "default_last_page")]
    pub last_page: String,
    #[serde(default)]
    pub memory: MemoryDefaults,
    #[serde(default)]
    pub java: JavaDefaults,
    #[serde(default)]
    pub gpu_preference: GpuPreference,
    #[serde(default)]
    pub default_jvm_args: String,
    #[serde(default)]
    pub default_game_args: String,
    #[serde(default = "default_width")]
    pub window_width: f32,
    #[serde(default = "default_height")]
    pub window_height: f32,
    #[serde(default = "default_true")]
    pub completed_onboarding: bool,
    #[serde(default = "default_parallel")]
    pub parallel_downloads: usize,
    #[serde(default)]
    pub show_snapshots: bool,
}

fn default_true() -> bool {
    true
}
fn default_last_page() -> String {
    "home".to_string()
}
fn default_width() -> f32 {
    1100.0
}
fn default_height() -> f32 {
    700.0
}
fn default_parallel() -> usize {
    6
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            profile: None,
            close_action: CloseAction::Hide,
            remember_instance: true,
            selected_instance: None,
            last_page: "home".to_string(),
            memory: MemoryDefaults::default(),
            java: JavaDefaults {
                mode: "automatic".to_string(),
                custom_path: String::new(),
            },
            gpu_preference: GpuPreference::default(),
            default_jvm_args: String::new(),
            default_game_args: String::new(),
            window_width: 1100.0,
            window_height: 700.0,
            completed_onboarding: false,
            parallel_downloads: 6,
            show_snapshots: false,
        }
    }
}

impl LauncherConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        toml::from_str(&text).map_err(|e| MonoryxError::TomlDe(e.to_string()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text =
            toml::to_string_pretty(self).map_err(|e| MonoryxError::TomlSer(e.to_string()))?;
        crate::storage::atomic::atomic_write_str(path, &text)
    }

    #[must_use]
    pub fn is_first_run(&self) -> bool {
        self.profile.is_none() || !self.completed_onboarding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.toml");
        let c = LauncherConfig {
            last_page: "library".to_string(),
            ..Default::default()
        };
        c.save(&p).unwrap();
        let back = LauncherConfig::load(&p).unwrap();
        assert_eq!(back.last_page, "library");
    }

    #[test]
    fn missing_file_gives_default() {
        let d = tempfile::tempdir().unwrap();
        let c = LauncherConfig::load(&d.path().join("nope.toml")).unwrap();
        assert!(c.is_first_run());
    }

    #[test]
    fn gpu_preference_defaults_to_system() {
        assert_eq!(GpuPreference::default(), GpuPreference::System);
        assert_eq!(
            toml::from_str::<LauncherConfig>("").unwrap().gpu_preference,
            GpuPreference::System
        );
    }

    #[test]
    fn gpu_preference_serialization_roundtrip() {
        for (text, value) in [
            ("system", GpuPreference::System),
            ("high_performance", GpuPreference::HighPerformance),
            ("power_saving", GpuPreference::PowerSaving),
        ] {
            let json = serde_json::to_string(&value).unwrap();
            assert_eq!(json, format!("\"{text}\""));
            assert_eq!(serde_json::from_str::<GpuPreference>(&json).unwrap(), value);
        }
    }

    #[test]
    fn gpu_preference_persists_in_config() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.toml");
        let c = LauncherConfig {
            gpu_preference: GpuPreference::HighPerformance,
            ..Default::default()
        };
        c.save(&p).unwrap();
        let back = LauncherConfig::load(&p).unwrap();
        assert_eq!(back.gpu_preference, GpuPreference::HighPerformance);
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("gpu_preference = \"high_performance\""));
    }
}
