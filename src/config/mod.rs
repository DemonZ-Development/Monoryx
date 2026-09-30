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
    #[serde(
        alias = "close",
        alias = "Close",
        alias = "Hide",
        alias = "hide",
        alias = "keep_open",
        alias = "keepopen",
        alias = "keep-open"
    )]
    Hide,
}

impl CloseAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Never => "Keep launcher open",
            Self::Minimize => "Minimize",
            Self::Hide => "Close / Hide (restore after game exits)",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeKind {
    #[default]
    Monochrome,
    Gloss,
    SoftPink,
    SoftBrown,
}

impl ThemeKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Monochrome => "Monochrome",
            Self::Gloss => "Gloss",
            Self::SoftPink => "Soft pink",
            Self::SoftBrown => "Soft brown",
        }
    }

    pub const fn all() -> [Self; 4] {
        [
            Self::Monochrome,
            Self::Gloss,
            Self::SoftPink,
            Self::SoftBrown,
        ]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CurseForgeSettings {
    #[serde(default)]
    pub api_key: String,
}

impl CurseForgeSettings {
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !self.api_key.trim().is_empty()
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackupCompression {
    #[default]
    Fast,

    Maximum,

    Zstd,
}

impl BackupCompression {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fast => "Fast (default deflate)",
            Self::Maximum => "Maximum (deflate 9)",
            Self::Zstd => "Smallest (zstd)",
        }
    }

    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::Fast => "Quickest to write. Choose this if you back up often.",
            Self::Maximum => "Smaller files, slower to write and read.",
            Self::Zstd => {
                "Smallest files and still fast. MONORYX restores these fine, but Windows \
                 Explorer cannot open them."
            }
        }
    }

    #[must_use]
    pub const fn deflate_level(self) -> Option<i64> {
        match self {
            Self::Fast => None,
            Self::Maximum | Self::Zstd => Some(9),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default)]
    pub discord: crate::discord::Settings,
    #[serde(default)]
    pub theme: ThemeKind,
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
    #[serde(default)]
    pub start_maximized: bool,
    #[serde(default = "default_true")]
    pub completed_onboarding: bool,
    #[serde(default = "default_parallel")]
    pub parallel_downloads: usize,
    #[serde(default)]
    pub show_snapshots: bool,
    #[serde(default)]
    pub boost_mode: bool,
    #[serde(default = "default_true")]
    pub auto_check_updates: bool,
    #[serde(default = "default_true")]
    pub skins_restorer_compat: bool,
    #[serde(default)]
    pub curseforge: CurseForgeSettings,
    #[serde(default)]
    pub backup_compression: BackupCompression,
}

fn default_last_page() -> String {
    "home".to_string()
}
fn default_width() -> f32 {
    1280.0
}
fn default_height() -> f32 {
    720.0
}
fn default_parallel() -> usize {
    6
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            theme: ThemeKind::default(),
            discord: crate::discord::Settings::default(),
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
            window_width: default_width(),
            window_height: default_height(),
            start_maximized: false,
            completed_onboarding: false,
            parallel_downloads: 6,
            show_snapshots: false,
            boost_mode: false,
            auto_check_updates: true,
            skins_restorer_compat: true,
            curseforge: CurseForgeSettings {
                api_key: String::new(),
            },
            backup_compression: BackupCompression::default(),
        }
    }
}

impl LauncherConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let config: Self =
            toml::from_str(&text).map_err(|e| MonoryxError::TomlDe(e.to_string()))?;
        if text.contains("keep_open") || text.contains("keepopen") || text.contains("keep-open") {
            let _ = config.save(path);
        }
        Ok(config)
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

    #[test]
    fn theme_roundtrip_and_window_defaults() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("config.toml");
        let config = LauncherConfig {
            theme: ThemeKind::SoftPink,
            ..Default::default()
        };
        config.save(&path).unwrap();
        let loaded = LauncherConfig::load(&path).unwrap();
        assert_eq!(loaded.theme, ThemeKind::SoftPink);
        assert!(!LauncherConfig::default().start_maximized);
        assert_eq!(LauncherConfig::default().window_width, 1280.0);
    }

    #[test]
    fn boost_and_update_defaults() {
        let def = LauncherConfig::default();
        assert!(!def.boost_mode);
        assert!(def.auto_check_updates);
        assert!(def.skins_restorer_compat);

        let parsed: LauncherConfig = toml::from_str("").unwrap();
        assert!(!parsed.boost_mode);
        assert!(parsed.auto_check_updates);
        assert!(parsed.skins_restorer_compat);
    }

    #[test]
    fn close_action_defaults_to_hide() {
        assert_eq!(CloseAction::default(), CloseAction::Hide);
        assert_eq!(LauncherConfig::default().close_action, CloseAction::Hide);

        let parsed: LauncherConfig = toml::from_str("").unwrap();
        assert_eq!(parsed.close_action, CloseAction::Hide);

        let parsed_empty_table: LauncherConfig = toml::from_str("[memory]\n").unwrap();
        assert_eq!(parsed_empty_table.close_action, CloseAction::Hide);

        let parsed_legacy_close: LauncherConfig =
            toml::from_str("close_action = \"close\"\n").unwrap();
        assert_eq!(parsed_legacy_close.close_action, CloseAction::Hide);

        let parsed_legacy_close_cap: LauncherConfig =
            toml::from_str("close_action = \"Close\"\n").unwrap();
        assert_eq!(parsed_legacy_close_cap.close_action, CloseAction::Hide);

        let parsed_hide: LauncherConfig = toml::from_str("close_action = \"hide\"\n").unwrap();
        assert_eq!(parsed_hide.close_action, CloseAction::Hide);

        let parsed_hide_cap: LauncherConfig = toml::from_str("close_action = \"Hide\"\n").unwrap();
        assert_eq!(parsed_hide_cap.close_action, CloseAction::Hide);

        let parsed_never: LauncherConfig = toml::from_str("close_action = \"never\"\n").unwrap();
        assert_eq!(parsed_never.close_action, CloseAction::Never);

        let parsed_minimize: LauncherConfig =
            toml::from_str("close_action = \"minimize\"\n").unwrap();
        assert_eq!(parsed_minimize.close_action, CloseAction::Minimize);

        let parsed_keep_open: LauncherConfig =
            toml::from_str("close_action = \"keep_open\"\n").unwrap();
        assert_eq!(parsed_keep_open.close_action, CloseAction::Hide);

        let parsed_keepopen: LauncherConfig =
            toml::from_str("close_action = \"keepopen\"\n").unwrap();
        assert_eq!(parsed_keepopen.close_action, CloseAction::Hide);

        let parsed_keep_dash_open: LauncherConfig =
            toml::from_str("close_action = \"keep-open\"\n").unwrap();
        assert_eq!(parsed_keep_dash_open.close_action, CloseAction::Hide);
    }

    #[test]
    fn keep_open_migrates_on_load() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.toml");
        std::fs::write(&p, "close_action = \"keep_open\"\n").unwrap();
        let loaded = LauncherConfig::load(&p).unwrap();
        assert_eq!(loaded.close_action, CloseAction::Hide);
        let migrated_text = std::fs::read_to_string(&p).unwrap();
        assert!(migrated_text.contains("close_action = \"hide\""));
    }
}
