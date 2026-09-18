use crate::instance::config::LoaderKind;
use crate::java::runtime::JavaRuntime;
use crate::minecraft::manifest::VersionManifest;
use crate::modrinth::models::{Project, ProjectVersion, SearchResponse};
use crate::modrinth::updates::UpdateInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    Onboarding,
    #[default]
    Home,
    Instances,
    Discover,
    Library,
    Downloads,
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
            Self::Settings => "Settings",
            Self::Logs => "Logs",
        }
    }
    pub const fn all() -> [Self; 7] {
        [
            Self::Home,
            Self::Instances,
            Self::Discover,
            Self::Library,
            Self::Downloads,
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
            Self::Settings => "settings",
            Self::Logs => "logs",
        }
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
    UpdatesFound(Vec<UpdateInfo>),
    PlayStarted(String),
    PlayFinished(String, i32),
    PlaySpawned,
    PlayLog(String),
    Download(crate::downloads::job::DownloadEvent),
    RepairDone(std::result::Result<Vec<String>, String>),
}
