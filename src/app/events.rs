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
    Discover,
    Library,
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
            Self::Discover => "Discover",
            Self::Library => "Library",
            Self::Downloads => "Downloads",
            Self::Nexeu => "Nexeu Servers",
            Self::Accounts => "Accounts",
            Self::Settings => "Settings",
            Self::Logs => "Logs",
        }
    }
    pub const fn all() -> [Self; 9] {
        [
            Self::Home,
            Self::Instances,
            Self::Discover,
            Self::Library,
            Self::Downloads,
            Self::Nexeu,
            Self::Accounts,
            Self::Settings,
            Self::Logs,
        ]
    }
    pub fn from_page_str(s: &str) -> Self {
        match s {
            "instances" => Self::Instances,
            "discover" => Self::Discover,
            "library" => Self::Library,
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
            Self::Discover => "discover",
            Self::Library => "library",
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
    LoaderVersions(LoaderKind, String, std::result::Result<Vec<String>, String>),
    SearchDone(std::result::Result<SearchResponse, String>),
    ProjectDetail(std::result::Result<Project, String>),
    ProjectVersions(std::result::Result<Vec<ProjectVersion>, String>),
    Thumbnail(String, std::result::Result<Vec<u8>, String>),
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
}
