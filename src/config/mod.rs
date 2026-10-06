use crate::account::offline::OfflineProfile;
use crate::error::{MonoryxError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
mod secrets;

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
    Monochrome,
    Gloss,
    #[default]
    Halloween,
    SoftPink,
    SoftBrown,
}

impl ThemeKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Monochrome => "Monochrome",
            Self::Gloss => "Gloss",
            Self::Halloween => "Halloween",
            Self::SoftPink => "Soft pink",
            Self::SoftBrown => "Soft brown",
        }
    }

    pub const fn all() -> [Self; 5] {
        [
            Self::Monochrome,
            Self::Gloss,
            Self::Halloween,
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

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct CurseForgeSettings {
    #[serde(default, skip_serializing)]
    pub api_key: String,
    #[serde(default)]
    pub custom_endpoint: String,
}

impl std::fmt::Debug for CurseForgeSettings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CurseForgeSettings")
            .field("custom_endpoint", &self.custom_endpoint)
            .finish_non_exhaustive()
    }
}

impl CurseForgeSettings {
    #[must_use]
    pub fn has_custom_key(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    #[must_use]
    pub fn is_configured(&self) -> bool {
        true
    }

    pub fn clear_custom(&mut self) {
        self.api_key.clear();
        self.custom_endpoint.clear();
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VerifyLevel {
    #[default]
    Quick,
    Full,
}

impl VerifyLevel {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Quick => "Fast check (size only)",
            Self::Full => "Full check (hash every file)",
        }
    }

    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::Quick => {
                "Recommended. Repair game files still does a complete hash check whenever you ask for it."
            }
            Self::Full => "Safest, but adds seconds to every launch on a heavily modded instance.",
        }
    }

    pub const fn as_verify(self) -> crate::minecraft::installer::Verify {
        match self {
            Self::Quick => crate::minecraft::installer::Verify::Quick,
            Self::Full => crate::minecraft::installer::Verify::Full,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum JvmPreset {
    None,
    #[default]
    Aikar,
    Shenandoah,
    GenerationalZgc,
    LowMemory,
    HighThroughput,
}

impl JvmPreset {
    #[must_use]
    pub const fn all() -> [Self; 6] {
        [
            Self::Aikar,
            Self::Shenandoah,
            Self::GenerationalZgc,
            Self::HighThroughput,
            Self::LowMemory,
            Self::None,
        ]
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "Vanilla (no extra flags)",
            Self::Aikar => "Aikar's G1GC (recommended)",
            Self::Shenandoah => "Shenandoah GC (ultra-low pause)",
            Self::GenerationalZgc => "Generational ZGC (Java 21+)",
            Self::HighThroughput => "High Throughput G1GC (modpacks 8GB+)",
            Self::LowMemory => "Low Memory / Budget (<4GB RAM)",
        }
    }

    #[must_use]
    pub const fn short_name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Aikar => "Aikar",
            Self::Shenandoah => "Shenandoah",
            Self::GenerationalZgc => "ZGC",
            Self::HighThroughput => "High Throughput",
            Self::LowMemory => "Low RAM",
        }
    }

    #[must_use]
    pub const fn flags(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Aikar => concat!(
                "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 ",
                "-XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC ",
                "-XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 -XX:G1HeapRegionSize=8M ",
                "-XX:G1ReservePercent=20 -XX:G1HeapWastePercent=5 -XX:G1MixedGCCountTarget=4 ",
                "-XX:InitiatingHeapOccupancyPercent=15 -XX:G1MixedGCLiveThresholdPercent=90 ",
                "-XX:G1RSetUpdatingPauseTimePercent=5 -XX:SurvivorRatio=32 ",
                "-XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1 ",
                "-Dusing.aikars.flags=https://mcflags.emc.gs -Daikarsnewflags=true"
            ),
            Self::Shenandoah => concat!(
                "-XX:+UseShenandoahGC -XX:+UnlockExperimentalVMOptions ",
                "-XX:ShenandoahGCMode=satb -XX:ShenandoahGCHeuristics=adaptive ",
                "-XX:+AlwaysPreTouch -XX:+DisableExplicitGC"
            ),
            Self::GenerationalZgc => concat!(
                "-XX:+UseZGC -XX:+ZGenerational -XX:+UnlockExperimentalVMOptions ",
                "-XX:+AlwaysPreTouch -XX:+DisableExplicitGC"
            ),
            Self::HighThroughput => concat!(
                "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=130 ",
                "-XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch ",
                "-XX:G1NewSizePercent=28 -XX:G1MaxNewSizePercent=38 -XX:G1ReservePercent=15 ",
                "-XX:G1HeapRegionSize=16M -XX:InitiatingHeapOccupancyPercent=20"
            ),
            Self::LowMemory => "-XX:+UseSerialGC -XX:+DisableExplicitGC -XX:+AlwaysPreTouch",
        }
    }

    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::None => "Vanilla launcher behavior. No extra garbage collector flags injected.",
            Self::Aikar => {
                "The community standard for Minecraft. Optimizes G1GC to prevent garbage collection lag spikes during long sessions."
            }
            Self::Shenandoah => {
                "Reduces GC pause times down to single-digit milliseconds. Excellent for Java 11, 17, and 21 on multi-core CPUs."
            }
            Self::GenerationalZgc => {
                "Next-generation ultra-low latency collector for Java 21+. Keeps GC pause times under 1ms even with high memory heaps."
            }
            Self::HighThroughput => {
                "Aggressively tuned for heavy 200+ modpacks with 8GB or more allocated RAM to maximize FPS stability."
            }
            Self::LowMemory => {
                "Uses lightweight Serial GC to minimize memory overhead on budget PCs or laptops with less than 4GB allocated."
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials_id: Option<String>,
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
    #[serde(default = "default_true")]
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
    pub microsoft_profile: Option<crate::account::microsoft::MicrosoftProfile>,
    #[serde(default)]
    pub microsoft_client_id: String,
    #[serde(default)]
    pub use_microsoft_auth: bool,
    #[serde(default)]
    pub backup_compression: BackupCompression,
    #[serde(default)]
    pub verify_level: VerifyLevel,
    #[serde(default)]
    pub jvm_preset: JvmPreset,
    #[serde(default)]
    pub appcds: bool,
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
            credentials_id: None,
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
            start_maximized: true,
            completed_onboarding: false,
            parallel_downloads: 6,
            show_snapshots: false,
            boost_mode: false,
            auto_check_updates: true,
            skins_restorer_compat: true,
            curseforge: CurseForgeSettings {
                api_key: String::new(),
                custom_endpoint: String::new(),
            },
            microsoft_profile: None,
            microsoft_client_id: String::new(),
            use_microsoft_auth: false,
            backup_compression: BackupCompression::default(),
            verify_level: VerifyLevel::default(),
            jvm_preset: JvmPreset::default(),
            appcds: false,
        }
    }
}

impl LauncherConfig {
    #[must_use]
    pub fn active_account(&self) -> Option<crate::account::Account> {
        if self.use_microsoft_auth {
            if let Some(ms) = &self.microsoft_profile {
                return Some(crate::account::Account::Microsoft(ms.clone()));
            }
        }
        self.profile
            .as_ref()
            .map(|p| crate::account::Account::Offline(p.clone()))
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let mut config: Self = match toml::from_str(&text) {
            Ok(config) => config,
            Err(error) => {
                let backup = path.with_extension("toml.bak");
                let backup_text = std::fs::read_to_string(&backup).map_err(|_| {
                    MonoryxError::TomlDe(format!(
                        "{error}. Configuration was preserved at {}",
                        path.display()
                    ))
                })?;
                let recovered: Self = toml::from_str(&backup_text).map_err(|_| {
                    MonoryxError::TomlDe(format!(
                        "{error}. Configuration and its backup were preserved."
                    ))
                })?;
                let preserved =
                    path.with_extension(format!("corrupt-{}.toml", uuid::Uuid::new_v4()));
                std::fs::copy(path, &preserved)?;
                crate::utils::fs::atomic_write_str(path, &backup_text)?;
                tracing::warn!("Recovered launcher settings from backup; damaged configuration preserved at {}", preserved.display());
                recovered
            }
        };
        let legacy_secrets = secrets::Secrets::from_config(&config);
        if !legacy_secrets.is_empty() {
            config.save(path)?;
            config.credentials_id = toml::from_str::<Self>(&std::fs::read_to_string(path)?)
                .map_err(|e| MonoryxError::TomlDe(e.to_string()))?
                .credentials_id;
        } else if let Some(id) = &config.credentials_id {
            secrets::Secrets::read(id)?.apply(&mut config);
        }
        if text.contains("keep_open") || text.contains("keepopen") || text.contains("keep-open") {
            config.save(path)?;
        }
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let lock = crate::utils::fs::path_lock(parent);
        let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        let previous = match std::fs::read_to_string(path) {
            Ok(text) => Some(toml::from_str::<Self>(&text).map_err(|_| MonoryxError::InvalidConfig(format!("Refusing to overwrite damaged settings at {}. Restore the file or its backup first.", path.display())))?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let old_id = previous
            .as_ref()
            .and_then(|config| config.credentials_id.clone());
        let old_secrets = if let Some(id) = &old_id {
            secrets::Secrets::read(id)?
        } else {
            previous
                .as_ref()
                .map(secrets::Secrets::from_config)
                .unwrap_or_default()
        };
        let new_secrets = secrets::Secrets::from_config(self);
        let mut persisted = self.clone();
        persisted.credentials_id = if new_secrets.is_empty() {
            None
        } else if new_secrets == old_secrets && old_id.is_some() {
            old_id.clone()
        } else {
            Some(uuid::Uuid::new_v4().to_string())
        };
        let text =
            toml::to_string_pretty(&persisted).map_err(|e| MonoryxError::TomlSer(e.to_string()))?;
        let changed = old_id != persisted.credentials_id;
        if changed {
            if let Some(id) = &persisted.credentials_id {
                if let Err(error) = new_secrets.write(id) {
                    secrets::Secrets::delete(id);
                    return Err(error);
                }
            }
        }
        let write_result = (|| {
            if let Some(mut backup) = previous {
                backup.credentials_id = None;
                backup.microsoft_profile = None;
                backup.use_microsoft_auth = false;
                let backup_text = toml::to_string_pretty(&backup)
                    .map_err(|e| MonoryxError::TomlSer(e.to_string()))?;
                crate::utils::fs::atomic_write_str(&path.with_extension("toml.bak"), &backup_text)?;
            }
            crate::storage::atomic::atomic_write_str(path, &text)
        })();
        if let Err(error) = write_result {
            if changed {
                if let Some(id) = &persisted.credentials_id {
                    secrets::Secrets::delete(id);
                }
            }
            return Err(error);
        }
        if let Some(id) = old_id.filter(|id| persisted.credentials_id.as_ref() != Some(id)) {
            secrets::Secrets::delete(&id);
        }
        Ok(())
    }

    #[must_use]
    pub fn is_first_run(&self) -> bool {
        (self.profile.is_none() && self.microsoft_profile.is_none()) || !self.completed_onboarding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed_in_config() -> LauncherConfig {
        LauncherConfig {
            microsoft_profile: Some(crate::account::microsoft::MicrosoftProfile {
                username: "Player".into(),
                uuid: uuid::Uuid::new_v4(),
                access_token: "private-access-token".into(),
                refresh_token: "private-refresh-token".into(),
                expires_at: 100,
            }),
            curseforge: CurseForgeSettings {
                api_key: "private-api-key".into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn credentials_roundtrip_without_plaintext_in_settings_or_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = signed_in_config();
        config.save(&path).unwrap();
        config.save(&path).unwrap();
        for file in [&path, &path.with_extension("toml.bak")] {
            let text = std::fs::read_to_string(file).unwrap();
            assert!(!text.contains("private-"));
            assert!(!text.contains("access_token"));
            assert!(!text.contains("refresh_token"));
            assert!(!text.contains("api_key"));
        }
        let loaded = LauncherConfig::load(&path).unwrap();
        assert_eq!(loaded.microsoft_profile, config.microsoft_profile);
        assert_eq!(loaded.curseforge.api_key, config.curseforge.api_key);
    }

    #[test]
    fn legacy_plaintext_credentials_migrate_only_after_vault_accepts_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let text = format!("[microsoft_profile]\nusername = 'Player'\nuuid = '{}'\naccess_token = 'private-access-token'\nrefresh_token = 'private-refresh-token'\nexpires_at = 100\n[curseforge]\napi_key = 'private-api-key'\n", uuid::Uuid::new_v4());
        std::fs::write(&path, &text).unwrap();
        secrets::backend::FAIL.set(true);
        let failed = LauncherConfig::load(&path);
        secrets::backend::FAIL.set(false);
        assert!(failed.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        let loaded = LauncherConfig::load(&path).unwrap();
        assert_eq!(loaded.curseforge.api_key, "private-api-key");
        assert_eq!(
            loaded.microsoft_profile.unwrap().refresh_token,
            "private-refresh-token"
        );
        assert!(!std::fs::read_to_string(&path).unwrap().contains("private-"));
        assert!(!std::fs::read_to_string(path.with_extension("toml.bak"))
            .unwrap()
            .contains("private-"));
    }

    #[test]
    fn unavailable_vault_preserves_saved_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        signed_in_config().save(&path).unwrap();
        let before = std::fs::read(&path).unwrap();
        secrets::backend::FAIL.set(true);
        let loaded = LauncherConfig::load(&path);
        let saved = LauncherConfig::default().save(&path);
        secrets::backend::FAIL.set(false);
        assert!(loaded.is_err() && saved.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn damaged_config_is_preserved_and_recovers_last_saved_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut config = LauncherConfig {
            last_page: "library".into(),
            ..Default::default()
        };
        config.save(&path).unwrap();
        config.last_page = "settings".into();
        config.save(&path).unwrap();
        std::fs::write(&path, b"broken = [").unwrap();
        assert!(LauncherConfig::default().save(&path).is_err());
        let recovered = LauncherConfig::load(&path).unwrap();
        assert_eq!(recovered.last_page, "library");
        assert!(std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .any(
                |file| file.file_name().to_string_lossy().contains("corrupt-")
                    && std::fs::read(file.path()).unwrap() == b"broken = ["
            ));
    }

    #[test]
    fn sign_out_removes_saved_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        signed_in_config().save(&path).unwrap();
        let id = LauncherConfig::load(&path).unwrap().credentials_id.unwrap();
        LauncherConfig::default().save(&path).unwrap();
        assert!(secrets::Secrets::read(&id).is_err());
        assert!(LauncherConfig::load(&path)
            .unwrap()
            .credentials_id
            .is_none());
    }

    #[test]
    fn changed_credentials_use_a_new_reference_after_successful_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut config = signed_in_config();
        config.save(&path).unwrap();
        let original_id = LauncherConfig::load(&path).unwrap().credentials_id.unwrap();
        config.microsoft_profile.as_mut().unwrap().refresh_token =
            "new-private-refresh-token".into();
        config.save(&path).unwrap();
        let loaded = LauncherConfig::load(&path).unwrap();
        assert_ne!(loaded.credentials_id.as_deref(), Some(original_id.as_str()));
        assert_eq!(
            loaded.microsoft_profile.unwrap().refresh_token,
            "new-private-refresh-token"
        );
        assert!(secrets::Secrets::read(&original_id).is_err());
    }

    #[test]
    fn failed_settings_write_keeps_the_previous_credential_reference_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut config = signed_in_config();
        config.save(&path).unwrap();
        let before = std::fs::read(&path).unwrap();
        let original_id = LauncherConfig::load(&path).unwrap().credentials_id.unwrap();
        std::fs::create_dir(path.with_extension("toml.bak")).unwrap();
        config.microsoft_profile.as_mut().unwrap().refresh_token =
            "new-private-refresh-token".into();
        assert!(config.save(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let loaded = LauncherConfig::load(&path).unwrap();
        assert_eq!(loaded.credentials_id.as_deref(), Some(original_id.as_str()));
        assert_eq!(
            loaded.microsoft_profile.unwrap().refresh_token,
            "private-refresh-token"
        );
    }

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
        assert!(LauncherConfig::default().start_maximized);
        assert!(
            toml::from_str::<LauncherConfig>("")
                .unwrap()
                .start_maximized
        );
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

    #[test]
    fn jvm_presets_flags() {
        assert_eq!(JvmPreset::None.flags(), "");
        assert!(JvmPreset::Aikar.flags().contains("UseG1GC"));
        assert!(JvmPreset::Shenandoah.flags().contains("UseShenandoahGC"));
        assert!(JvmPreset::GenerationalZgc.flags().contains("UseZGC"));
        assert!(JvmPreset::HighThroughput.flags().contains("UseG1GC"));
        assert!(JvmPreset::HighThroughput
            .flags()
            .contains("G1HeapRegionSize=16M"));
        assert!(JvmPreset::LowMemory.flags().contains("UseSerialGC"));

        assert_eq!(JvmPreset::all().len(), 6);
        for preset in JvmPreset::all() {
            assert!(!preset.label().is_empty());
            assert!(!preset.hint().is_empty());
        }
    }
}
