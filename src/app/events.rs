use crate::instance::config::LoaderKind;
use crate::java::runtime::JavaRuntime;
use crate::minecraft::manifest::VersionManifest;
use crate::modrinth::models::{Project, ProjectVersion, SearchResponse};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    Onboarding,
    #[default]
    Home,
    Instances,
    Worlds,
    Discover,
    Library,
    Screenshots,
    Downloads,
    Nexeu,
    Accounts,
    Settings,
    Logs,
}

impl Page {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Onboarding => "Welcome",
            Self::Home => "Home",
            Self::Instances => "Instances",
            Self::Worlds => "Worlds & Files",
            Self::Discover => "Discover",
            Self::Library => "Library",
            Self::Screenshots => "Screenshots",
            Self::Downloads => "Downloads",
            Self::Nexeu => "Nexeu Servers",
            Self::Accounts => "Accounts",
            Self::Settings => "Settings",
            Self::Logs => "Logs",
        }
    }
    pub const fn all() -> [Self; 11] {
        [
            Self::Home,
            Self::Instances,
            Self::Worlds,
            Self::Discover,
            Self::Library,
            Self::Screenshots,
            Self::Downloads,
            Self::Nexeu,
            Self::Accounts,
            Self::Settings,
            Self::Logs,
        ]
    }

    pub const fn group_header(self) -> Option<&'static str> {
        match self {
            Self::Onboarding | Self::Home => None,
            Self::Accounts => Some("MANAGE"),
            _ => None,
        }
    }

    pub const fn shortcut(self) -> Option<&'static str> {
        match self {
            Self::Home => Some("Ctrl+1"),
            Self::Instances => Some("Ctrl+2"),
            Self::Worlds => Some("Ctrl+3"),
            Self::Discover => Some("Ctrl+4"),
            Self::Library => Some("Ctrl+5"),
            Self::Screenshots => Some("F2"),
            Self::Downloads => Some("Ctrl+7"),
            Self::Accounts => Some("Ctrl+8"),
            Self::Settings => Some("Ctrl+9"),
            Self::Nexeu | Self::Logs | Self::Onboarding => None,
        }
    }
    pub fn from_page_str(s: &str) -> Self {
        match s {
            "instances" => Self::Instances,
            "worlds" => Self::Worlds,
            "discover" => Self::Discover,
            "library" => Self::Library,
            "screenshots" => Self::Screenshots,
            "downloads" => Self::Downloads,
            "nexeu" => Self::Nexeu,
            "accounts" => Self::Accounts,
            "settings" => Self::Settings,
            "logs" => Self::Logs,
            _ => Self::Home,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Onboarding => "onboarding",
            Self::Home => "home",
            Self::Instances => "instances",
            Self::Worlds => "worlds",
            Self::Discover => "discover",
            Self::Library => "library",
            Self::Screenshots => "screenshots",
            Self::Downloads => "downloads",
            Self::Nexeu => "nexeu",
            Self::Accounts => "accounts",
            Self::Settings => "settings",
            Self::Logs => "logs",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_roundtrip_and_accounts_page() {
        for page in Page::all() {
            let s = page.as_str();
            let parsed = Page::from_page_str(s);
            assert_eq!(page, parsed);
            assert!(!page.label().is_empty());
        }
        assert_eq!(Page::from_page_str("accounts"), Page::Accounts);
        assert_eq!(Page::Accounts.label(), "Accounts");
        assert_eq!(Page::Accounts.as_str(), "accounts");
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum AppEvent {
    Notice(String),
    Error(String),
    VersionsLoaded(std::result::Result<VersionManifest, String>),
    PatchNotesLoaded(
        std::result::Result<
            std::collections::HashMap<String, crate::minecraft::patch_notes::PatchNote>,
            String,
        >,
    ),
    PatchNoteFull(String, std::result::Result<String, String>),
    LoaderVersions(LoaderKind, String, std::result::Result<Vec<String>, String>),
    SearchDone(u64, std::result::Result<SearchResponse, String>),
    AppCdsRecorded(String),
    ClasspathResolved(String, Vec<std::path::PathBuf>),
    ProjectDetail(String, std::result::Result<Project, String>),
    ProjectVersions(String, std::result::Result<Vec<ProjectVersion>, String>),
    Thumbnail(String, std::result::Result<DecodedImage, String>),
    ProjectImage(String, std::result::Result<DecodedImage, String>),
    ScreenshotThumbnail(
        std::path::PathBuf,
        std::result::Result<DecodedImage, String>,
    ),
    ScreenshotFull(
        std::path::PathBuf,
        std::result::Result<DecodedImage, String>,
    ),
    ScreenshotScan(u64, Vec<crate::app::screenshots::ScreenshotEntry>),
    WorldsScanned(
        u64,
        String,
        std::result::Result<crate::instance::worlds::WorldSnapshot, String>,
    ),
    FilesScanned(
        u64,
        String,
        std::path::PathBuf,
        std::result::Result<Vec<crate::instance::worlds::FileEntry>, String>,
    ),
    FilePreview(
        String,
        std::path::PathBuf,
        std::result::Result<String, String>,
    ),
    WorldAction(bool, std::result::Result<String, String>),
    ModCounts(u64, std::collections::HashMap<String, usize>),
    JavaList(Vec<JavaRuntime>),
    GpuList(Vec<crate::utils::system::GpuInfo>),
    OperationStarted(String, String, Option<String>),
    InstallProgress(String, String, usize, usize),
    OperationFinished(String, std::result::Result<(), String>),
    InstallDone(String, std::result::Result<String, String>),
    ModInstallDone(std::result::Result<Vec<String>, String>),
    PackDone(std::result::Result<String, String>),
    InstanceImported(std::result::Result<String, String>),
    UpdatesFound(
        String,
        std::result::Result<crate::modrinth::updates::UpdateScan, String>,
    ),
    LoaderUpdateChecked(String, std::result::Result<Option<String>, String>),
    LoaderUpdateDone(String, std::result::Result<String, String>),
    PlayStarted(String),
    GameActivity(String, crate::minecraft::activity::GameActivity),
    PlayFailed(String),
    PlayFinished {
        id: String,
        code: i32,
        log_file: Option<std::path::PathBuf>,
        launched_at: Option<std::time::SystemTime>,
    },
    PlaySpawned,
    PlayLog(String),
    CrashLogShared(u64, std::result::Result<String, String>),
    Download(crate::downloads::job::DownloadEvent),
    RepairDone(std::result::Result<Vec<String>, String>),
    LauncherUpdate(std::result::Result<crate::app::updater::LauncherUpdateInfo, String>),
    LauncherUpdateDownloadProgress(Option<f32>),
    LauncherUpdateDownloaded(std::result::Result<std::path::PathBuf, String>),
    RestoreWindow,
    NexeuOverview(u64, std::result::Result<crate::nexeu::Overview, String>),
    NexeuResources(u64, String, std::result::Result<serde_json::Value, String>),
    NexeuLogs(u64, String, std::result::Result<String, String>),
    NexeuBackups(
        u64,
        String,
        std::result::Result<Vec<crate::nexeu::Backup>, String>,
    ),
    NexeuBackupCreated(u64, String, std::result::Result<(), String>),
    NexeuCommand(u64, std::result::Result<(), String>),
    NexeuPower(u64, std::result::Result<String, String>),
    MicrosoftDeviceCode(std::result::Result<crate::account::microsoft::DeviceCodeResponse, String>),
    MicrosoftLoginDone(std::result::Result<crate::account::microsoft::MicrosoftProfile, String>),
    MicrosoftSessionRefreshed(crate::account::microsoft::MicrosoftProfile),
}

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}
