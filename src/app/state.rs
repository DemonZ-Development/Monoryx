use crate::app::events::{AppEvent, DecodedImage, Page};
use crate::config::LauncherConfig;
use crate::content::{ContentKind, InstalledEntry};
use crate::downloads::job::JobState;
use crate::downloads::DownloadManager;
use crate::instance::config::{InstanceConfig, LoaderKind};
use crate::instance::manager::InstanceManager;
use crate::java::runtime::JavaRuntime;
use crate::minecraft::manifest::VersionManifest;
use crate::modrinth::api::ModrinthClient;
use crate::modrinth::models::{Project, ProjectVersion};
use crate::modrinth::search::{DiscoverTab, SearchFilters};
use crate::modrinth::updates::UpdateInfo;
use crate::storage::paths::MonoryxPaths;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct TrackedDownload {
    pub id: String,
    pub label: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed_bps: f64,
    pub state: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct RowActivity {
    pub label: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub finished: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct InstallOperation {
    pub label: String,
    pub phase: String,
    pub instance_id: Option<String>,
    pub completed: usize,
    pub total: usize,
}

impl InstallOperation {
    pub fn fraction(&self) -> Option<f32> {
        (self.total > 1).then(|| (self.completed as f32 / self.total as f32).clamp(0.0, 1.0))
    }
}

#[derive(Debug, Clone, Default)]
pub struct NewInstanceDraft {
    pub step: usize,
    pub name: String,
    pub version: String,
    pub loader: LoaderKind,
    pub loader_version: String,
    pub versions: Vec<String>,
    pub loader_versions: Vec<String>,
    pub loading_versions: bool,
    pub loading_loaders: bool,
    pub loader_fetch_key: String,
    pub dialog_seq: u64,
    pub error: String,
    pub version_query: String,
}

#[derive(Default)]
pub struct WorldsUiState {
    pub instance_id: Option<String>,
    pub selected_world: Option<String>,
    pub icons: HashMap<String, egui::TextureHandle>,
    pub snapshot: crate::instance::worlds::WorldSnapshot,
    pub entries: Vec<crate::instance::worlds::FileEntry>,
    pub relative: std::path::PathBuf,
    pub preview: Option<(std::path::PathBuf, String)>,
    pub selected_file: Option<std::path::PathBuf>,
    pub show_files: bool,
    pub loading: bool,
    pub files_loading: bool,
    pub busy: bool,
    pub error: String,
    pub generation: u64,
    pub files_generation: u64,
}

pub enum WorldActionKind {
    BackUp(String),
    Restore(String),
    ImportWorld(std::path::PathBuf),
    ImportGameFolder(std::path::PathBuf, Box<InstanceConfig>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiscoverSource {
    #[default]
    Modrinth,
    CurseForge,
}

impl DiscoverSource {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Modrinth => "Modrinth",
            Self::CurseForge => "CurseForge",
        }
    }
}

pub struct AppState {
    pub discord: crate::discord::Presence,
    pub game_activities: HashMap<String, (crate::minecraft::activity::GameActivity, i64)>,
    pub paths: MonoryxPaths,
    pub egui_ctx: egui::Context,
    pub config: LauncherConfig,
    pub http: reqwest::Client,
    pub metadata_slots: std::sync::Arc<tokio::sync::Semaphore>,
    pub dm: DownloadManager,
    pub mr: ModrinthClient,
    pub nexeu: crate::nexeu::Session,
    pub instances: InstanceManager,
    pub tx: Sender<AppEvent>,
    pub rx: Receiver<AppEvent>,
    pub runtime: tokio::runtime::Runtime,
    pub page: Page,
    pub instance_list: Vec<InstanceConfig>,
    pub mod_counts: HashMap<String, usize>,
    pub mod_counts_generation: u64,
    pub selected_instance: Option<String>,
    pub manifest: Option<VersionManifest>,
    pub manifest_loading: bool,
    pub patch_notes: HashMap<String, crate::minecraft::patch_notes::PatchNote>,
    pub patch_notes_loading: bool,
    pub patch_notes_loaded: bool,
    pub patch_notes_error: String,
    pub patch_note_full: Option<(String, String)>,
    pub patch_note_full_loading: Option<String>,
    pub versions_error: String,
    pub show_snapshots: bool,
    pub search: SearchFilters,
    pub search_results: Vec<crate::modrinth::models::SearchResult>,
    pub search_total: u32,
    pub search_loading: bool,
    pub search_generation: u64,
    pub search_error: String,
    pub search_debounce: Option<Instant>,
    pub discover_tab: DiscoverTab,
    pub discover_source: DiscoverSource,
    pub detail_slug: Option<String>,
    pub detail_project: Option<Project>,
    pub detail_versions: Vec<ProjectVersion>,
    pub detail_loading: bool,
    pub detail_versions_loading: bool,
    pub detail_error: String,
    pub detail_version_pick: String,
    pub markdown_blocks: Vec<crate::ui::markdown::Block>,
    pub library_entries: Vec<InstalledEntry>,
    pub library_filter: ContentKind,
    pub library_view_hierarchy: bool,
    pub cached_hierarchy: Option<crate::content::DependencyHierarchy>,
    hierarchy_generation: u64,
    hierarchy_pending: bool,
    pending_pack_exports: std::collections::HashSet<String>,
    pub pending_content_delete: Option<(String, ContentKind, String, String)>,
    pub updates: Vec<UpdateInfo>,
    pub updates_loading: bool,
    pub updates_checked: bool,
    pub updates_summary: String,
    pub updates_error: String,
    pub updates_instance: Option<String>,
    pub loader_update_checking: Option<String>,
    pub loader_update_candidate: Option<(String, String)>,
    pub loader_update_busy: Option<String>,
    pub loader_update_error: String,
    pub operations: std::collections::BTreeMap<String, InstallOperation>,
    pub downloads: HashMap<String, TrackedDownload>,
    pub downloads_history: Vec<TrackedDownload>,
    pub row_activity: HashMap<String, RowActivity>,
    pub java_list: Vec<JavaRuntime>,
    pub java_loading: bool,
    pub gpu_list: Vec<crate::utils::system::GpuInfo>,
    pub gpu_loading: bool,
    pub busy_install: HashMap<String, (String, usize, usize)>,
    pub playing: HashMap<String, bool>,
    pub last_exit: String,
    pub launcher_hidden: bool,
    pub launcher_minimized: bool,
    pub log_lines: Vec<String>,
    pub onboarding_step: u32,
    pub onboarding_user: String,
    pub onboarding_error: String,
    pub onboarding_mem_auto: bool,
    pub onboarding_mem_max: String,
    pub onboarding_use_microsoft: bool,
    pub notice: String,
    pub notice_at: Option<Instant>,
    pub error_dialog: String,
    pub confirm_delete: Option<String>,
    pub confirm_title: String,
    pub edit_instance: Option<InstanceConfig>,
    pub edit_tab: usize,
    pub edit_error: String,
    pub new_draft: NewInstanceDraft,
    pub show_new_instance: bool,
    pub thumbnails: HashMap<String, egui::TextureHandle>,
    pub image_slots: std::sync::Arc<tokio::sync::Semaphore>,
    pub pending_thumbs: HashMap<String, bool>,
    pub failed_thumbs: HashSet<String>,
    pub reset_discover_scroll: bool,
    pub project_image_url: Option<String>,
    pub project_image: Option<egui::TextureHandle>,
    pub project_image_loading: bool,
    pub project_image_error: String,
    pub screenshots: Vec<crate::app::screenshots::ScreenshotEntry>,
    pub screenshots_loading: bool,
    pub screenshots_generation: u64,
    pub screenshots_filter: Option<String>,
    pub screenshots_visible_count: usize,
    pub screenshot_thumbnails: HashMap<std::path::PathBuf, egui::TextureHandle>,
    pub pending_screenshot_thumbnails: HashSet<std::path::PathBuf>,
    pub screenshot_viewer: Option<std::path::PathBuf>,
    pub screenshot_full_image: Option<egui::TextureHandle>,
    pub screenshot_full_loading: bool,
    pub screenshot_full_error: String,
    pub worlds: WorldsUiState,
    pub classpath_preview: Option<Vec<std::path::PathBuf>>,
    pub global_status: String,
    pub global_frac: Option<f32>,
    pub settings_jvm: String,
    pub settings_game_args: String,
    pub settings_mem_min: String,
    pub settings_mem_max: String,
    pub settings_width: String,
    pub settings_height: String,
    pub launcher_update: Option<crate::app::updater::LauncherUpdateInfo>,
    pub launcher_update_loading: bool,
    pub launcher_update_error: Option<String>,
    pub launcher_update_download_loading: bool,
    pub launcher_update_download_progress: Option<f32>,
    pub launcher_update_downloaded: Option<std::path::PathBuf>,
    pub launcher_update_download_error: Option<String>,
    pub show_update_banner: bool,
    pub ms_device_code: Option<crate::account::microsoft::DeviceCodeResponse>,
    pub ms_login_loading: bool,
    pub ms_login_error: Option<String>,
    pub ms_login_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub crash_report: Option<crate::minecraft::crash::CrashInfo>,
    pub crash_share_generation: u64,
    pub crash_share_loading: bool,
    pub crash_share_url: Option<String>,
    pub crash_share_error: String,
    pub command_palette_open: bool,
    pub command_palette_query: String,
    pub home_instance_search: String,
    pub home_grid_view: bool,
    pub home_screenshot_index: usize,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let paths = MonoryxPaths::global();
        let _ = paths.ensure_all();
        Self::new_with_paths(cc, paths)
    }

    pub fn new_with_paths(_cc: &eframe::CreationContext<'_>, paths: MonoryxPaths) -> Self {
        Self::build(_cc, paths, true)
    }

    #[cfg(test)]
    pub(crate) fn new_for_preview(cc: &eframe::CreationContext<'_>, paths: MonoryxPaths) -> Self {
        Self::build(cc, paths, false)
    }

    fn build(_cc: &eframe::CreationContext<'_>, paths: MonoryxPaths, bootstrap: bool) -> Self {
        let _ = paths.ensure_all();
        let (config, config_error) = match LauncherConfig::load(&paths.config_file()) {
            Ok(config) => (config, String::new()),
            Err(error) => (LauncherConfig::default(), error.user_message()),
        };
        let http = crate::utils::net::create_client().expect("http client");
        let (tx, rx) = std::sync::mpsc::channel();
        let download_tx = tx.clone();
        let repaint = _cc.egui_ctx.clone();
        let dm = DownloadManager::new(http.clone(), config.parallel_downloads.max(1))
            .with_observer(std::sync::Arc::new(move |event| {
                let _ = download_tx.send(AppEvent::Download(event));
                repaint.request_repaint();
            }));
        let mr = ModrinthClient::new(http.clone());
        let instances = InstanceManager::new(paths.clone());
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("monoryx-bg")
            .build()
            .expect("tokio runtime");
        let page = if config.is_first_run() {
            Page::Onboarding
        } else {
            Page::from_page_str(&config.last_page)
        };
        let instance_list = instances.list().unwrap_or_default();
        let mut selected_instance = config.selected_instance.clone();
        if selected_instance
            .as_ref()
            .is_some_and(|id| instance_list.iter().all(|c| &c.id != id))
        {
            selected_instance = instance_list.first().map(|c| c.id.clone());
        }
        if selected_instance.is_none() {
            selected_instance = instance_list.first().map(|c| c.id.clone());
        }
        let show_snapshots = config.show_snapshots;
        let default_onboarding_user = config
            .profile
            .as_ref()
            .map(|p| p.username.clone())
            .unwrap_or_default();
        let default_onboarding_use_microsoft =
            config.use_microsoft_auth && config.microsoft_profile.is_some();
        let mod_counts = HashMap::new();
        let mut s = Self {
            discord: crate::discord::Presence::start(&runtime),
            game_activities: HashMap::new(),
            egui_ctx: _cc.egui_ctx.clone(),
            paths,
            config,
            http,
            metadata_slots: std::sync::Arc::new(tokio::sync::Semaphore::new(4)),
            dm,
            mr,
            nexeu: crate::nexeu::Session::default(),
            instances,
            tx,
            rx,
            runtime,
            page,
            instance_list,
            mod_counts,
            mod_counts_generation: 0,
            selected_instance,
            manifest: None,
            manifest_loading: false,
            patch_notes: HashMap::new(),
            patch_notes_loading: false,
            patch_notes_loaded: false,
            patch_notes_error: String::new(),
            patch_note_full: None,
            patch_note_full_loading: None,
            versions_error: String::new(),
            show_snapshots,
            search: SearchFilters::default(),
            search_results: Vec::new(),
            search_total: 0,
            search_loading: false,
            search_generation: 0,
            search_error: String::new(),
            search_debounce: None,
            discover_tab: DiscoverTab::Mods,
            discover_source: DiscoverSource::Modrinth,
            detail_slug: None,
            detail_project: None,
            detail_versions: Vec::new(),
            detail_loading: false,
            detail_versions_loading: false,
            detail_error: String::new(),
            detail_version_pick: String::new(),
            markdown_blocks: Vec::new(),
            library_entries: Vec::new(),
            library_filter: ContentKind::Mod,
            library_view_hierarchy: false,
            cached_hierarchy: None,
            hierarchy_generation: 0,
            hierarchy_pending: false,
            pending_pack_exports: std::collections::HashSet::new(),
            pending_content_delete: None,
            updates: Vec::new(),
            updates_loading: false,
            updates_checked: false,
            updates_summary: String::new(),
            updates_error: String::new(),
            updates_instance: None,
            loader_update_checking: None,
            loader_update_candidate: None,
            loader_update_busy: None,
            loader_update_error: String::new(),
            operations: std::collections::BTreeMap::new(),
            downloads: HashMap::new(),
            downloads_history: Vec::new(),
            java_list: Vec::new(),
            java_loading: false,
            gpu_list: Vec::new(),
            gpu_loading: false,
            busy_install: HashMap::new(),
            row_activity: HashMap::new(),
            playing: HashMap::new(),
            last_exit: String::new(),
            launcher_hidden: false,
            launcher_minimized: false,
            log_lines: Vec::new(),
            onboarding_step: 0,
            onboarding_user: default_onboarding_user,
            onboarding_error: String::new(),
            onboarding_mem_auto: true,
            onboarding_mem_max: crate::utils::system::default_max_memory_mb().to_string(),
            onboarding_use_microsoft: default_onboarding_use_microsoft,
            notice: String::new(),
            notice_at: None,
            error_dialog: config_error,
            confirm_delete: None,
            confirm_title: String::new(),
            edit_instance: None,
            edit_tab: 0,
            edit_error: String::new(),
            new_draft: NewInstanceDraft::default(),
            show_new_instance: false,
            thumbnails: HashMap::new(),
            image_slots: std::sync::Arc::new(tokio::sync::Semaphore::new(3)),
            pending_thumbs: HashMap::new(),
            failed_thumbs: HashSet::new(),
            reset_discover_scroll: false,
            project_image_url: None,
            project_image: None,
            project_image_loading: false,
            project_image_error: String::new(),
            screenshots: Vec::new(),
            screenshots_loading: false,
            screenshots_generation: 0,
            screenshots_filter: None,
            screenshots_visible_count: 48,
            screenshot_thumbnails: HashMap::new(),
            pending_screenshot_thumbnails: HashSet::new(),
            screenshot_viewer: None,
            screenshot_full_image: None,
            screenshot_full_loading: false,
            screenshot_full_error: String::new(),
            worlds: WorldsUiState::default(),
            classpath_preview: None,
            global_status: String::new(),
            global_frac: None,
            settings_jvm: String::new(),
            settings_game_args: String::new(),
            settings_mem_min: String::new(),
            settings_mem_max: String::new(),
            settings_width: String::new(),
            settings_height: String::new(),
            launcher_update: None,
            launcher_update_loading: false,
            launcher_update_error: None,
            launcher_update_download_loading: false,
            launcher_update_download_progress: None,
            launcher_update_downloaded: None,
            launcher_update_download_error: None,
            show_update_banner: true,
            ms_device_code: None,
            ms_login_loading: false,
            ms_login_error: None,
            ms_login_cancel: None,
            crash_report: None,
            crash_share_generation: 0,
            crash_share_loading: false,
            crash_share_url: None,
            crash_share_error: String::new(),
            command_palette_open: false,
            command_palette_query: String::new(),
            home_instance_search: String::new(),
            home_grid_view: false,
            home_screenshot_index: 0,
        };
        s.settings_jvm = s.config.default_jvm_args.clone();
        s.settings_game_args = s.config.default_game_args.clone();
        s.settings_mem_min = s.config.memory.min_mb.to_string();
        s.settings_mem_max = s.config.memory.max_mb.to_string();
        if bootstrap {
            s.refresh_instances();
            s.spawn_initial();
            if s.page == Page::Discover {
                s.sync_search_filters();
                s.run_search();
            }
        }
        s
    }

    pub fn save_config(&mut self) {
        self.config.last_page = self.page.as_str().to_string();
        self.config.selected_instance = self.selected_instance.clone();
        self.config.show_snapshots = self.show_snapshots;
        let _ = self.config.save(&self.paths.config_file());
    }

    pub fn discord_preview(&self) -> Option<serde_json::Value> {
        let active = self
            .game_activities
            .iter()
            .filter(|(id, _)| self.playing.get(*id).copied().unwrap_or(false))
            .max_by_key(|(_, (_, started))| *started);
        let session = active.and_then(|(id, (activity, started))| {
            let cfg = self.instance_list.iter().find(|cfg| &cfg.id == id)?;
            Some(crate::discord::Session {
                name: &cfg.name,
                version: &cfg.minecraft_version,
                loader: cfg.loader.display_name(),
                activity,
                started: *started,
            })
        });
        crate::discord::activity(&self.config.discord, self.page.label(), session)
    }

    pub fn sync_discord(&self) {
        self.discord
            .configure(&self.config.discord, self.page.label());
    }

    pub fn refresh_instances(&mut self) {
        self.instance_list = self.instances.list().unwrap_or_default();
        self.mod_counts_generation = self.mod_counts_generation.wrapping_add(1);
        let generation = self.mod_counts_generation;
        let manager = self.instances.clone();
        let ids: Vec<_> = self
            .instance_list
            .iter()
            .map(|instance| instance.id.clone())
            .collect();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.runtime.spawn(async move {
            let counts = tokio::task::spawn_blocking(move || {
                ids.into_iter()
                    .map(|id| {
                        let count = manager.mod_count(&id);
                        (id, count)
                    })
                    .collect()
            })
            .await
            .unwrap_or_default();
            let _ = tx.send(AppEvent::ModCounts(generation, counts));
            ctx.request_repaint();
        });
        if self
            .selected_instance
            .as_ref()
            .is_none_or(|id| self.instance_list.iter().all(|c| &c.id != id))
        {
            self.selected_instance = self.instance_list.first().map(|c| c.id.clone());
        }
        self.config.selected_instance = self.selected_instance.clone();
        self.refresh_library();
        self.refresh_screenshots();
        if self.page == Page::Worlds {
            self.refresh_worlds();
        }
    }

    pub fn selected(&self) -> Option<InstanceConfig> {
        self.selected_instance
            .as_ref()
            .and_then(|id| self.instance_list.iter().find(|c| &c.id == id).cloned())
    }

    pub fn instance_readiness(&self, cfg: &InstanceConfig) -> crate::instance::Readiness {
        crate::instance::readiness::readiness(
            &self.paths,
            cfg,
            self.busy_install.contains_key(&cfg.id),
        )
    }

    pub fn installable_instances(&self) -> Vec<InstanceConfig> {
        self.instance_list
            .iter()
            .filter(|cfg| self.instance_readiness(cfg).is_ready())
            .cloned()
            .collect()
    }

    pub fn installable_loaders(&self) -> Vec<crate::instance::LoaderKind> {
        crate::instance::readiness::installable_loaders(
            self.instance_list.iter(),
            &self.paths,
            &self.busy_install,
        )
    }

    pub fn install_blocker(&self) -> Option<String> {
        let Some(cfg) = self.selected() else {
            return Some("Choose an instance to install into.".to_string());
        };
        match self.instance_readiness(&cfg) {
            crate::instance::Readiness::Ready => None,
            crate::instance::Readiness::Installing => Some(format!(
                "{} is still downloading. Wait for it to finish, or repair it.",
                cfg.name
            )),
            crate::instance::Readiness::NotDownloaded => Some(format!(
                "{} has not been downloaded yet. Finish creating it, or repair its files, before installing mods.",
                cfg.name
            )),
        }
    }

    pub fn set_page(&mut self, page: Page) {
        self.page = page;
        self.save_config();
        if page != Page::Screenshots && page != Page::Home {
            self.screenshot_thumbnails.clear();
            self.screenshot_thumbnails.shrink_to_fit();
            self.pending_screenshot_thumbnails.clear();
            self.pending_screenshot_thumbnails.shrink_to_fit();
        }
        if page != Page::Discover {
            self.thumbnails.clear();
            self.thumbnails.shrink_to_fit();
            self.pending_thumbs.clear();
            self.pending_thumbs.shrink_to_fit();
            self.failed_thumbs.clear();
            self.failed_thumbs.shrink_to_fit();
        }
        if page != Page::Instances {
            self.patch_notes.clear();
            self.patch_notes.shrink_to_fit();
            self.patch_notes_loaded = false;
        }
        if page != Page::Worlds {
            self.worlds = WorldsUiState::default();
        }
        match page {
            Page::Library => self.refresh_library(),
            Page::Screenshots => self.refresh_screenshots(),
            Page::Worlds => self.refresh_worlds(),
            Page::Discover if self.search_results.is_empty() && !self.search_loading => {
                self.queue_search(true);
            }
            _ => {}
        }
    }

    pub fn load_patch_notes(&mut self) {
        if self.patch_notes_loading || self.patch_notes_loaded {
            return;
        }
        self.patch_notes_loading = true;
        let http = self.http.clone();
        let dir = self.paths.metadata_dir();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        let slots = self.metadata_slots.clone();
        self.runtime.spawn(async move {
            let Ok(_permit) = slots.acquire_owned().await else {
                return;
            };
            let cache = crate::storage::cache::DiskCache::new(
                dir,
                std::time::Duration::from_secs(24 * 3600),
            );
            let result = crate::minecraft::patch_notes::load_index(&http, &cache).await;
            let _ = tx.send(AppEvent::PatchNotesLoaded(result));
            ctx.request_repaint();
        });
    }

    pub fn load_full_patch_note(&mut self, version: &str) {
        if self
            .patch_note_full
            .as_ref()
            .is_some_and(|(stored, _)| stored == version)
            || self.patch_note_full_loading.as_deref() == Some(version)
        {
            return;
        }
        let Some(note) = self.patch_notes.get(version).cloned() else {
            return;
        };
        self.patch_note_full_loading = Some(version.to_string());
        self.patch_note_full = None;
        let http = self.http.clone();
        let dir = self.paths.metadata_dir();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        let version = version.to_string();
        let slots = self.metadata_slots.clone();
        self.runtime.spawn(async move {
            let Ok(_permit) = slots.acquire_owned().await else {
                return;
            };
            let cache = crate::storage::cache::DiskCache::new(
                dir,
                std::time::Duration::from_secs(7 * 24 * 3600),
            );
            let result = crate::minecraft::patch_notes::load_full(&http, &cache, &note).await;
            let _ = tx.send(AppEvent::PatchNoteFull(version, result));
            ctx.request_repaint();
        });
    }

    pub fn refresh_worlds(&mut self) {
        let Some(id) = self.selected_instance.clone() else {
            self.worlds = WorldsUiState::default();
            return;
        };
        if self.worlds.instance_id.as_deref() != Some(&id) {
            let generation = self.worlds.generation;
            let files_generation = self.worlds.files_generation;
            self.worlds = WorldsUiState {
                instance_id: Some(id.clone()),
                generation,
                files_generation,
                ..Default::default()
            };
        }
        self.worlds.generation = self.worlds.generation.wrapping_add(1);
        let generation = self.worlds.generation;
        self.worlds.loading = true;
        let manager = self.instances.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.runtime.spawn(async move {
            let scan_id = id.clone();
            let result = tokio::task::spawn_blocking(move || {
                crate::instance::worlds::scan(&manager, &scan_id)
                    .map_err(|error| error.user_message())
            })
            .await
            .unwrap_or_else(|error| Err(error.to_string()));
            let _ = tx.send(AppEvent::WorldsScanned(generation, id, result));
            ctx.request_repaint();
        });
        self.refresh_files();
    }

    pub fn refresh_files(&mut self) {
        let Some(id) = self.selected_instance.clone() else {
            return;
        };
        self.worlds.files_generation = self.worlds.files_generation.wrapping_add(1);
        let generation = self.worlds.files_generation;
        self.worlds.files_loading = true;
        let relative = self.worlds.relative.clone();
        let manager = self.instances.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.runtime.spawn(async move {
            let scan_id = id.clone();
            let scan_relative = relative.clone();
            let result = tokio::task::spawn_blocking(move || {
                crate::instance::worlds::list_files(&manager, &scan_id, &scan_relative)
                    .map_err(|error| error.user_message())
            })
            .await
            .unwrap_or_else(|error| Err(error.to_string()));
            let _ = tx.send(AppEvent::FilesScanned(generation, id, relative, result));
            ctx.request_repaint();
        });
    }

    pub fn preview_file(&mut self, relative: std::path::PathBuf) {
        let Some(id) = self.selected_instance.clone() else {
            return;
        };
        self.worlds.selected_file = Some(relative.clone());
        self.worlds.preview = None;
        let manager = self.instances.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.runtime.spawn(async move {
            let scan_id = id.clone();
            let scan_relative = relative.clone();
            let result = tokio::task::spawn_blocking(move || {
                crate::instance::worlds::preview_text(&manager, &scan_id, &scan_relative)
                    .map_err(|error| error.user_message())
            })
            .await
            .unwrap_or_else(|error| Err(error.to_string()));
            let _ = tx.send(AppEvent::FilePreview(id, relative, result));
            ctx.request_repaint();
        });
    }

    pub fn run_world_action(&mut self, action: WorldActionKind) {
        if self.worlds.busy {
            return;
        }
        let Some(id) = self.selected_instance.clone() else {
            return;
        };
        if self.playing.get(&id).copied().unwrap_or(false) {
            self.worlds.error =
                "Close Minecraft before importing, backing up, or restoring worlds.".to_string();
            return;
        }
        self.worlds.busy = true;
        self.worlds.error.clear();
        let manager = self.instances.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        let level = self.config.backup_compression;
        self.runtime.spawn(async move {
            let is_new_instance = matches!(action, WorldActionKind::ImportGameFolder(..));
            let result = tokio::task::spawn_blocking(move || {
                match action {
                    WorldActionKind::BackUp(name) => {
                        crate::instance::worlds::back_up(&manager, &id, &name, level)
                            .map(|file| format!("Backed up {name} to {file}"))
                    }
                    WorldActionKind::Restore(file) => {
                        crate::instance::worlds::restore_as_copy(&manager, &id, &file)
                            .map(|name| format!("Restored as {name}"))
                    }
                    WorldActionKind::ImportWorld(path) => {
                        crate::instance::worlds::import_world(&manager, &id, &path)
                            .map(|name| format!("Imported world {name}"))
                    }
                    WorldActionKind::ImportGameFolder(path, template) => {
                        crate::instance::worlds::import_game_folder(&manager, &template, &path)
                            .map(|config| config.id)
                    }
                }
                .map_err(|error| error.user_message())
            })
            .await
            .unwrap_or_else(|error| Err(error.to_string()));
            let _ = tx.send(AppEvent::WorldAction(is_new_instance, result));
            ctx.request_repaint();
        });
    }

    pub fn refresh_screenshots(&mut self) {
        self.screenshots_generation = self.screenshots_generation.wrapping_add(1);
        let generation = self.screenshots_generation;
        self.screenshots_loading = true;
        let instances = self.instance_list.clone();
        let manager = self.instances.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.runtime.spawn(async move {
            let entries = tokio::task::spawn_blocking(move || {
                crate::app::screenshots::scan(&instances, &manager)
            })
            .await
            .unwrap_or_default();
            let _ = tx.send(AppEvent::ScreenshotScan(generation, entries));
            ctx.request_repaint();
        });
    }

    pub fn ensure_screenshot_thumb(&mut self, path: &std::path::Path) {
        if !self.screenshots.iter().any(|entry| entry.path == path)
            || self.screenshot_thumbnails.contains_key(path)
            || !self
                .pending_screenshot_thumbnails
                .insert(path.to_path_buf())
        {
            return;
        }
        let path = path.to_path_buf();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        let slots = self.image_slots.clone();
        self.runtime.spawn(async move {
            let Ok(_permit) = slots.acquire_owned().await else {
                return;
            };
            let image_path = path.clone();
            let result =
                tokio::task::spawn_blocking(move || read_local_image(&image_path, 480, 270))
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result);
            let _ = tx.send(AppEvent::ScreenshotThumbnail(path, result));
            ctx.request_repaint();
        });
    }

    pub fn open_screenshot(&mut self, path: &std::path::Path) {
        if !self.screenshots.iter().any(|entry| entry.path == path) {
            return;
        }
        let path = path.to_path_buf();
        self.screenshot_viewer = Some(path.clone());
        self.screenshot_full_image = None;
        self.screenshot_full_loading = true;
        self.screenshot_full_error.clear();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        let slots = self.image_slots.clone();
        self.runtime.spawn(async move {
            let Ok(_permit) = slots.acquire_owned().await else {
                return;
            };
            let image_path = path.clone();
            let result =
                tokio::task::spawn_blocking(move || read_local_image(&image_path, 2560, 1440))
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result);
            let _ = tx.send(AppEvent::ScreenshotFull(path, result));
            ctx.request_repaint();
        });
    }

    pub fn notify(&mut self, msg: impl Into<String>) {
        self.notice = msg.into();
        self.notice_at = Some(Instant::now());
    }

    fn ctx_follow(ctx: egui::Context) {
        std::thread::spawn(move || {
            for _ in 0..600 {
                if ctx.input(|i| i.pointer.any_released()) {
                    break;
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(90));
                std::thread::sleep(std::time::Duration::from_millis(80));
            }
        });
    }

    pub fn begin_row_activity(&mut self, file_name: &str, label: &str) {
        self.row_activity.insert(
            file_name.to_string(),
            RowActivity {
                label: label.to_string(),
                downloaded: 0,
                total: None,
                finished: None,
            },
        );
        Self::ctx_follow(self.egui_ctx.clone());
        self.egui_ctx.request_repaint();
    }

    pub fn end_row_activity(&mut self, file_name: &str, ok: bool) {
        if let Some(entry) = self.row_activity.get_mut(file_name) {
            entry.finished = Some(ok);
        }
        self.egui_ctx.request_repaint();
    }

    #[must_use]
    pub fn row_is_busy(&self, file_name: &str) -> bool {
        self.row_activity
            .get(file_name)
            .is_some_and(|entry| entry.finished.is_none())
    }

    pub fn fail(&mut self, msg: impl Into<String>) {
        self.error_dialog = msg.into();
        tracing::error!("{}", self.error_dialog);
        self.log_lines.push(format!("ERROR: {}", self.error_dialog));
        if self.log_lines.len() > 500 {
            self.log_lines.drain(..self.log_lines.len() - 500);
        }
    }

    pub fn refresh_library(&mut self) {
        if self.updates_instance != self.selected_instance {
            self.updates.clear();
            self.updates_checked = false;
            self.updates_loading = false;
            self.updates_summary.clear();
            self.updates_error.clear();
            self.updates_instance = self.selected_instance.clone();
        }
        if let Some(id) = self.selected_instance.clone() {
            let store =
                crate::content::ContentStore::for_instance(&self.instances.instance_dir(&id));
            let mut entries = store.load().entries;
            let game_dir = self.instances.game_dir(&id);
            let disabled_dir = self.instances.disabled_dir(&id);
            entries.retain_mut(|entry| {
                if crate::utils::fs::safe_file_name(&entry.file_name).is_err() {
                    return false;
                }
                let active = game_dir.join(entry.kind.subdir()).join(&entry.file_name);
                let disabled = disabled_dir
                    .join(entry.kind.subdir())
                    .join(format!("{}.disabled", entry.file_name));
                let legacy = disabled_dir.join(format!("{}.disabled", entry.file_name));
                if active.is_file() {
                    entry.enabled = true;
                    true
                } else if disabled.is_file() || (entry.kind == ContentKind::Mod && legacy.is_file())
                {
                    entry.enabled = false;
                    true
                } else {
                    false
                }
            });
            for kind in [
                ContentKind::Mod,
                ContentKind::Resourcepack,
                ContentKind::Shader,
            ] {
                for (folder, enabled) in [
                    (game_dir.join(kind.subdir()), true),
                    (disabled_dir.join(kind.subdir()), false),
                ] {
                    if let Ok(files) = std::fs::read_dir(folder) {
                        for file in files.flatten() {
                            if !file.file_type().is_ok_and(|kind| kind.is_file()) {
                                continue;
                            }
                            let Some(raw) = file.file_name().to_str().map(str::to_string) else {
                                continue;
                            };
                            let name = if enabled {
                                raw.as_str()
                            } else {
                                raw.strip_suffix(".disabled").unwrap_or(&raw)
                            };
                            if !name.ends_with(".jar") && !name.ends_with(".zip") {
                                continue;
                            }
                            if entries
                                .iter()
                                .any(|entry| entry.kind == kind && entry.file_name == name)
                            {
                                continue;
                            }
                            entries.push(InstalledEntry {
                                file_name: name.to_string(),
                                kind,
                                project_id: None,
                                project_slug: None,
                                project_title: None,
                                version_id: None,
                                version_number: None,
                                file_hash_sha512: None,
                                file_hash_sha1: None,
                                size: file.metadata().map(|m| m.len()).unwrap_or(0),
                                enabled,
                                installed_at: String::new(),
                                loader: String::new(),
                                game_version: String::new(),
                            });
                        }
                    }
                }
            }
            entries.sort_by_cached_key(|e| {
                e.project_title
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
            });
            self.mod_counts.insert(
                id,
                entries
                    .iter()
                    .filter(|entry| entry.kind == ContentKind::Mod && entry.enabled)
                    .count(),
            );
            self.library_entries = entries;
            self.cached_hierarchy = None;
        } else {
            self.library_entries = Vec::new();
            self.cached_hierarchy = None;
        }
        self.hierarchy_generation = self.hierarchy_generation.wrapping_add(1);
        self.hierarchy_pending = false;
    }

    pub fn ensure_hierarchy(&mut self) {
        if self.cached_hierarchy.is_some() || self.hierarchy_pending {
            return;
        }
        let Some(id) = self.selected_instance.clone() else {
            return;
        };
        self.hierarchy_pending = true;
        let generation = self.hierarchy_generation;
        let mods_dir = self.instances.mods_dir(&id);
        let entries = self.library_entries.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                crate::content::build_hierarchy(&mods_dir, &entries)
            })
            .await
            .map_err(|error| error.to_string());
            let _ = tx.send(AppEvent::HierarchyBuilt(generation, id, result));
            ctx.request_repaint();
        });
    }

    pub fn export_pack(&mut self, config: crate::instance::config::InstanceConfig) {
        if !self.pending_pack_exports.insert(config.id.clone()) {
            return;
        }
        let instance_dir = self.instances.instance_dir(&config.id);
        let entries = self.library_entries.clone();
        let id = config.id.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                crate::instance::export::export_quick_zip(&instance_dir, &config, &entries)
            })
            .await
            .map_err(|error| error.to_string())
            .and_then(|result| result.map_err(|error| error.user_message()));
            let _ = tx.send(AppEvent::PackExported(id, result));
            ctx.request_repaint();
        });
    }

    pub fn poll_events(&mut self, ctx: &egui::Context) {
        let mut dirty = false;
        while let Ok(ev) = self.rx.try_recv() {
            dirty = true;
            self.handle_event(ev, ctx);
        }
        if dirty {
            ctx.request_repaint();
        }
        if self
            .search_debounce
            .is_some_and(|t| t.elapsed() > std::time::Duration::from_millis(450))
        {
            self.search_debounce = None;
            self.queue_search(false);
        }
    }

    fn handle_event(&mut self, ev: AppEvent, ctx: &egui::Context) {
        match ev {
            AppEvent::HierarchyBuilt(generation, id, result) => {
                if generation == self.hierarchy_generation
                    && self.selected_instance.as_deref() == Some(&id)
                {
                    self.hierarchy_pending = false;
                    match result {
                        Ok(hierarchy) => self.cached_hierarchy = Some(hierarchy),
                        Err(error) => {
                            self.cached_hierarchy =
                                Some(crate::content::DependencyHierarchy::default());
                            self.fail(error);
                        }
                    }
                }
            }
            AppEvent::PackExported(id, result) => {
                self.pending_pack_exports.remove(&id);
                match result {
                    Ok(path) => {
                        self.notify(format!(
                            "Exported pack: {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ));
                        let _ = open::that_detached(path.parent().unwrap_or(&path));
                    }
                    Err(error) => self.fail(error),
                }
            }
            AppEvent::Notice(m) => self.notify(m),
            AppEvent::Error(m) => self.fail(m),
            AppEvent::PatchNotesLoaded(result) => {
                self.patch_notes_loading = false;
                match result {
                    Ok(notes) => {
                        self.patch_notes = notes;
                        self.patch_notes_loaded = true;
                        self.patch_notes_error.clear();
                    }
                    Err(error) => self.patch_notes_error = error,
                }
            }
            AppEvent::PatchNoteFull(version, result) => {
                if self.patch_note_full_loading.as_deref() == Some(&version) {
                    self.patch_note_full_loading = None;
                    match result {
                        Ok(text) => self.patch_note_full = Some((version, text)),
                        Err(error) => self.patch_notes_error = error,
                    }
                }
            }
            AppEvent::ModCounts(generation, counts) => {
                if generation == self.mod_counts_generation {
                    self.mod_counts = counts;
                    if let Some(id) = &self.selected_instance {
                        self.mod_counts.insert(
                            id.clone(),
                            self.library_entries
                                .iter()
                                .filter(|entry| entry.kind == ContentKind::Mod && entry.enabled)
                                .count(),
                        );
                    }
                }
            }
            AppEvent::WorldsScanned(generation, id, result) => {
                if generation == self.worlds.generation
                    && self.selected_instance.as_deref() == Some(&id)
                {
                    self.worlds.loading = false;
                    match result {
                        Ok(mut snapshot) => {
                            self.worlds.icons.clear();
                            for world in &mut snapshot.worlds {
                                if let Some(image) = world.icon.take() {
                                    let pixels = egui::ColorImage::from_rgba_unmultiplied(
                                        [image.width, image.height],
                                        &image.pixels,
                                    );
                                    self.worlds.icons.insert(
                                        world.name.clone(),
                                        ctx.load_texture(
                                            format!("world:{}:{}", id, world.name),
                                            pixels,
                                            egui::TextureOptions::LINEAR,
                                        ),
                                    );
                                }
                            }
                            if !snapshot.worlds.iter().any(|world| {
                                Some(&world.name) == self.worlds.selected_world.as_ref()
                            }) && !snapshot.backups.iter().any(|backup| {
                                Some(&backup.world_name) == self.worlds.selected_world.as_ref()
                            }) {
                                self.worlds.selected_world = snapshot
                                    .worlds
                                    .first()
                                    .map(|world| world.name.clone())
                                    .or_else(|| {
                                        snapshot
                                            .backups
                                            .first()
                                            .map(|backup| backup.world_name.clone())
                                    });
                            }
                            self.worlds.snapshot = snapshot;
                            self.worlds.error.clear();
                        }
                        Err(error) => self.worlds.error = error,
                    }
                }
            }
            AppEvent::FilesScanned(generation, id, relative, result) => {
                if generation == self.worlds.files_generation
                    && self.selected_instance.as_deref() == Some(&id)
                    && self.worlds.relative == relative
                {
                    self.worlds.files_loading = false;
                    match result {
                        Ok(entries) => self.worlds.entries = entries,
                        Err(error) => self.worlds.error = error,
                    }
                }
            }
            AppEvent::FilePreview(id, relative, result) => {
                if self.selected_instance.as_deref() == Some(&id)
                    && self.worlds.selected_file.as_ref() == Some(&relative)
                {
                    self.worlds.preview = Some((relative, result.unwrap_or_else(|error| error)));
                }
            }
            AppEvent::WorldAction(new_instance, result) => {
                self.worlds.busy = false;
                match result {
                    Ok(message) if new_instance => {
                        self.selected_instance = Some(message);
                        self.worlds.relative = std::path::PathBuf::new();
                        self.save_config();
                        self.refresh_instances();
                        self.notify("Imported game folder as a new instance. Play it to install missing Minecraft files.");
                    }
                    Ok(message) => {
                        self.refresh_worlds();
                        self.notify(message);
                    }
                    Err(error) => self.worlds.error = error,
                }
            }
            AppEvent::CrashLogShared(generation, result)
                if generation == self.crash_share_generation =>
            {
                self.crash_share_loading = false;
                match result {
                    Ok(url) => {
                        self.crash_share_url = Some(url);
                        self.crash_share_error.clear();
                    }
                    Err(error) => self.crash_share_error = error,
                }
            }
            AppEvent::CrashLogShared(..) => {}
            AppEvent::NexeuOverview(generation, result) if generation == self.nexeu.generation => {
                self.nexeu.loading = false;
                match result {
                    Ok(overview) => {
                        if self.nexeu.selected_server.as_ref().is_some_and(|id| {
                            overview.servers.iter().all(|server| &server.uuid != id)
                        }) {
                            self.nexeu.selected_server = None;
                            self.nexeu.resources = None;
                            self.nexeu.logs = None;
                            self.nexeu.backups = None;
                        }
                        self.nexeu.overview = Some(overview);
                        self.nexeu.error.clear();
                    }
                    Err(error) => self.nexeu.error = error,
                }
            }
            AppEvent::NexeuResources(generation, id, result)
                if generation == self.nexeu.generation =>
            {
                if self.nexeu.selected_server.as_deref() == Some(&id) {
                    match result {
                        Ok(resources) => {
                            self.nexeu.resources = Some(resources);
                            self.nexeu.error.clear();
                        }
                        Err(error) => self.nexeu.error = error,
                    }
                }
            }
            AppEvent::NexeuLogs(generation, id, result) if generation == self.nexeu.generation => {
                if self.nexeu.selected_server.as_deref() == Some(&id) {
                    match result {
                        Ok(logs) => self.nexeu.logs = Some(logs),
                        Err(error) => self.nexeu.error = error,
                    }
                }
            }
            AppEvent::NexeuBackups(generation, id, result)
                if generation == self.nexeu.generation =>
            {
                if self.nexeu.selected_server.as_deref() == Some(&id) {
                    match result {
                        Ok(backups) => self.nexeu.backups = Some(backups),
                        Err(error) => self.nexeu.error = error,
                    }
                }
            }
            AppEvent::NexeuBackupCreated(generation, id, result)
                if generation == self.nexeu.generation =>
            {
                match result {
                    Ok(()) => {
                        self.notify("Nexeu backup started");
                        self.nexeu_load_backups(id);
                    }
                    Err(error) => self.nexeu.error = error,
                }
            }
            AppEvent::NexeuCommand(generation, result) if generation == self.nexeu.generation => {
                match result {
                    Ok(()) => {
                        self.nexeu.console_command.clear();
                        self.notify("Nexeu console command sent");
                    }
                    Err(error) => self.nexeu.error = error,
                }
            }
            AppEvent::NexeuPower(generation, result) if generation == self.nexeu.generation => {
                match result {
                    Ok(action) => {
                        self.notify(format!("Nexeu {action} request sent"));
                        self.nexeu_refresh();
                    }
                    Err(error) => self.nexeu.error = error,
                }
            }
            AppEvent::NexeuOverview(..)
            | AppEvent::NexeuResources(..)
            | AppEvent::NexeuLogs(..)
            | AppEvent::NexeuBackups(..)
            | AppEvent::NexeuBackupCreated(..)
            | AppEvent::NexeuCommand(..)
            | AppEvent::NexeuPower(..) => {}
            AppEvent::InstanceImported(result) => match result {
                Ok(id) => {
                    self.refresh_instances();
                    self.selected_instance = Some(id);
                    self.refresh_library();
                    self.notify("Instance imported. Play to install any missing Minecraft files.");
                }
                Err(error) => self.fail(error),
            },
            AppEvent::VersionsLoaded(r) => {
                self.manifest_loading = false;
                match r {
                    Ok(m) => {
                        self.manifest = Some(m);
                        self.versions_error.clear();
                        self.sync_search_filters();
                    }
                    Err(e) => {
                        self.versions_error = if self.manifest.is_some() {
                            "Offline: showing cached Minecraft versions. Downloads require internet.".to_string()
                        } else {
                            e
                        };
                    }
                }
            }
            AppEvent::LoaderVersions(_kind, key, r) => {
                if self.new_draft.loader_fetch_key != key {
                    return;
                }
                self.new_draft.loading_loaders = false;
                match r {
                    Ok(v) => {
                        self.new_draft.loader_versions = v.clone();
                        if self.new_draft.loader_version.is_empty() {
                            self.new_draft.loader_version = v.first().cloned().unwrap_or_default();
                        }
                        self.new_draft.error.clear();
                    }
                    Err(e) => self.new_draft.error = e,
                }
            }
            AppEvent::SearchDone(generation, r) => {
                if generation != self.search_generation {
                    return;
                }
                self.search_loading = false;
                match r {
                    Ok(resp) => {
                        self.search_results = resp.hits;
                        self.search_total = resp.total_hits;
                        self.search_error.clear();
                    }
                    Err(e) => self.search_error = e,
                }
            }
            AppEvent::ClasspathResolved(id, cp) => {
                if self.selected_instance.as_deref() == Some(id.as_str()) {
                    self.classpath_preview = Some(cp);
                }
            }
            AppEvent::AppCdsRecorded(id) => {
                if let Some(mut cfg) = self.instance_list.iter().find(|c| c.id == id).cloned() {
                    if cfg.appcds_pending {
                        cfg.appcds_pending = false;
                        let _ = self.instances.save(&cfg);
                        self.refresh_instances();
                        if crate::app::appcds::archive_exists(&self.instances, &id) {
                            if let Some(cp) = self.classpath_preview.clone() {
                                crate::app::appcds::write_stamp(&self.instances, &id, &cp);
                                self.notify(
                                    "Startup archive recorded. It applies from next launch.",
                                );
                            } else {
                                self.notify(
                                    "Startup archive recorded. Launch the game once so the \
                                     archive can be verified, then use it from the launch after.",
                                );
                            }
                            self.notify("Startup archive recorded. It applies from next launch.");
                        } else {
                            self.notify(
                                "Could not record a startup archive for this instance. It still \
                                 launches normally, just without the speed-up.",
                            );
                        }
                    }
                }
            }
            AppEvent::ProjectDetail(slug, r) => {
                if self.detail_slug.as_deref() == Some(slug.as_str()) {
                    self.detail_loading = false;
                    match r {
                        Ok(p) => {
                            self.markdown_blocks = p
                                .body
                                .as_deref()
                                .map_or_else(Vec::new, crate::ui::markdown::parse);
                            self.detail_project = Some(p);
                            self.detail_error.clear();
                        }
                        Err(e) => self.detail_error = e,
                    }
                }
            }
            AppEvent::ProjectVersions(slug, r) => {
                if self.detail_slug.as_deref() == Some(slug.as_str()) {
                    self.detail_versions_loading = false;
                    match r {
                        Ok(v) => self.detail_versions = v,
                        Err(e) => self.detail_error = e,
                    }
                }
            }
            AppEvent::Thumbnail(url, r) => {
                self.pending_thumbs.remove(&url);
                match r {
                    Ok(img) => {
                        let cimg = egui::ColorImage::from_rgba_unmultiplied(
                            [img.width, img.height],
                            &img.pixels,
                        );
                        let tex = ctx.load_texture(url.clone(), cimg, egui::TextureOptions::LINEAR);
                        if self.thumbnails.len() >= 96 {
                            if let Some(old) = self.thumbnails.keys().next().cloned() {
                                self.thumbnails.remove(&old);
                            }
                        }
                        self.thumbnails.insert(url, tex);
                    }
                    Err(_) => {
                        self.failed_thumbs.insert(url);
                    }
                }
            }
            AppEvent::ProjectImage(url, result) => {
                if self.project_image_url.as_deref() == Some(url.as_str()) {
                    self.project_image_loading = false;
                    match result {
                        Ok(img) => {
                            let cimg = egui::ColorImage::from_rgba_unmultiplied(
                                [img.width, img.height],
                                &img.pixels,
                            );
                            self.project_image =
                                Some(ctx.load_texture(url, cimg, egui::TextureOptions::LINEAR));
                            self.project_image_error.clear();
                        }
                        Err(error) => self.project_image_error = error,
                    }
                }
            }
            AppEvent::ScreenshotScan(generation, entries) => {
                if generation == self.screenshots_generation {
                    self.screenshots_loading = false;
                    let paths: HashSet<_> =
                        entries.iter().map(|entry| entry.path.clone()).collect();
                    self.screenshot_thumbnails
                        .retain(|path, _| paths.contains(path));
                    self.pending_screenshot_thumbnails
                        .retain(|path| paths.contains(path));
                    if self
                        .screenshot_viewer
                        .as_ref()
                        .is_some_and(|path| !paths.contains(path))
                    {
                        self.screenshot_viewer = None;
                        self.screenshot_full_image = None;
                    }
                    self.screenshots = entries;
                }
            }
            AppEvent::ScreenshotThumbnail(path, result) => {
                self.pending_screenshot_thumbnails.remove(&path);
                if self.screenshots.iter().any(|entry| entry.path == path) {
                    if let Ok(image) = result {
                        let pixels = egui::ColorImage::from_rgba_unmultiplied(
                            [image.width, image.height],
                            &image.pixels,
                        );
                        if self.screenshot_thumbnails.len() >= 24 {
                            if let Some(old) = self.screenshot_thumbnails.keys().next().cloned() {
                                self.screenshot_thumbnails.remove(&old);
                            }
                        }
                        let texture = ctx.load_texture(
                            format!("screenshot:{}", path.display()),
                            pixels,
                            egui::TextureOptions::LINEAR,
                        );
                        self.screenshot_thumbnails.insert(path, texture);
                    }
                }
            }
            AppEvent::ScreenshotFull(path, result) => {
                if self.screenshot_viewer.as_ref() == Some(&path) {
                    self.screenshot_full_loading = false;
                    match result {
                        Ok(image) => {
                            let pixels = egui::ColorImage::from_rgba_unmultiplied(
                                [image.width, image.height],
                                &image.pixels,
                            );
                            self.screenshot_full_image = Some(ctx.load_texture(
                                format!("screenshot-full:{}", path.display()),
                                pixels,
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                        Err(error) => self.screenshot_full_error = error,
                    }
                }
            }
            AppEvent::JavaList(list) => {
                self.java_list = list;
                self.java_loading = false;
            }
            AppEvent::GpuList(list) => {
                self.gpu_list = list;
                self.gpu_loading = false;
            }
            AppEvent::OperationStarted(id, label, instance_id) => {
                self.error_dialog.clear();
                self.operations.insert(
                    id,
                    InstallOperation {
                        label,
                        phase: "Preparing...".to_string(),
                        instance_id,
                        completed: 0,
                        total: 0,
                    },
                );
            }
            AppEvent::InstallProgress(id, phase, a, b) => {
                if let Some(operation) = self.operations.get_mut(&id) {
                    operation.phase = phase;
                    operation.completed = a;
                    operation.total = b;
                }
            }
            AppEvent::OperationFinished(id, result) => {
                if let Some(operation) = self.operations.remove(&id) {
                    self.downloads_history.insert(
                        0,
                        TrackedDownload {
                            id,
                            label: operation.label,
                            downloaded: 0,
                            total: None,
                            speed_bps: 0.0,
                            state: if result.is_ok() {
                                "completed"
                            } else {
                                "failed"
                            }
                            .to_string(),
                            message: result.err().unwrap_or_default(),
                        },
                    );
                    self.downloads_history.truncate(50);
                }
            }
            AppEvent::InstallDone(id, r) => {
                self.busy_install.remove(&id);
                self.global_status.clear();
                self.global_frac = None;
                match r {
                    Ok(v) => {
                        self.notify(format!("Instance ready ({v})"));
                        self.refresh_instances();
                    }
                    Err(e) => self.fail(e),
                }
            }
            AppEvent::ModInstallDone(r) => {
                let ok = r.is_ok();
                let message = r.as_ref().err().cloned().unwrap_or_default();
                self.handle_operation_done("mod-install", ok, message);
                match r {
                    Ok(files) => {
                        self.notify(format!("Installed {} file(s)", files.len()));
                        for file in &files {
                            self.end_row_activity(file, true);
                        }
                        self.refresh_library();
                        self.updates.retain(|u| !files.contains(&u.file_name));
                        if self.updates.is_empty() {
                            self.updates_summary.clear();
                            self.updates_checked = true;
                        } else {
                            self.updates_summary =
                                format!("{} update(s) still available.", self.updates.len());
                        }
                    }
                    Err(e) => {
                        for key in self.row_activity.keys().cloned().collect::<Vec<_>>() {
                            self.end_row_activity(&key, false);
                        }
                        self.fail(e);
                    }
                }
            }
            AppEvent::PackDone(r) => {
                let ok = r.is_ok();
                let message = r.as_ref().err().cloned().unwrap_or_default();
                self.handle_operation_done("modpack-install", ok, message);
                match r {
                    Ok(id) => {
                        self.refresh_instances();
                        self.selected_instance = Some(id);
                        self.notify("Modpack installed");
                    }
                    Err(e) => self.fail(e),
                }
            }
            AppEvent::UpdatesFound(id, result) => {
                if self.selected_instance.as_deref() != Some(&id) {
                    return;
                }
                self.updates_loading = false;
                self.updates_checked = true;
                match result {
                    Ok(scan) => {
                        self.updates = scan.updates;
                        let checked_sources =
                            if scan.curseforge_checked > 0 && scan.modrinth_checked > 0 {
                                format!(
                                    "Checked {} items ({} Modrinth, {} CurseForge).",
                                    scan.checked, scan.modrinth_checked, scan.curseforge_checked
                                )
                            } else if scan.curseforge_checked > 0 {
                                format!("Checked {} CurseForge item(s).", scan.checked)
                            } else if scan.modrinth_checked > 0 {
                                format!("Checked {} Modrinth item(s).", scan.checked)
                            } else {
                                format!("Checked {} item(s).", scan.checked)
                            };
                        self.updates_summary = format!(
                            "{} {} update(s) available.{}",
                            checked_sources,
                            self.updates.len(),
                            if scan.untracked > 0 {
                                format!(
                                    " {} manual item(s) could not be identified.",
                                    scan.untracked
                                )
                            } else {
                                String::new()
                            }
                        );
                        self.updates_error = if scan.errors.is_empty() {
                            String::new()
                        } else {
                            format!(
                                "Could not check {} item(s): {}",
                                scan.errors.len(),
                                scan.errors.join("; ")
                            )
                        };
                        self.refresh_library();
                    }
                    Err(error) => {
                        self.updates.clear();
                        self.updates_summary.clear();
                        self.updates_error = error;
                    }
                }
            }
            AppEvent::LoaderUpdateChecked(id, result) => {
                if self.loader_update_checking.as_deref() != Some(&id) {
                    return;
                }
                self.loader_update_checking = None;
                match result {
                    Ok(Some(version)) => {
                        self.loader_update_candidate = Some((id, version));
                        self.loader_update_error.clear();
                    }
                    Ok(None) => {
                        self.loader_update_candidate = None;
                        self.loader_update_error.clear();
                        self.notify("Loader is up to date");
                    }
                    Err(error) => self.loader_update_error = error,
                }
            }
            AppEvent::LoaderUpdateDone(id, result) => {
                if self.loader_update_busy.as_deref() != Some(&id) {
                    return;
                }
                self.loader_update_busy = None;
                match result {
                    Ok(version) => {
                        self.loader_update_candidate = None;
                        self.loader_update_error.clear();
                        if self.edit_instance.as_ref().is_some_and(|cfg| cfg.id == id) {
                            self.edit_instance = None;
                        }
                        self.refresh_instances();
                        self.notify(format!("Loader updated to {version}"));
                    }
                    Err(error) => self.loader_update_error = error,
                }
            }
            AppEvent::PlayStarted(id) => {
                self.game_activities.insert(
                    id.clone(),
                    (
                        crate::minecraft::activity::GameActivity::Loading,
                        chrono::Utc::now().timestamp(),
                    ),
                );
                self.playing.insert(id, true);
            }
            AppEvent::GameActivity(id, activity) => {
                if self.playing.get(&id).copied().unwrap_or(false) {
                    self.game_activities
                        .entry(id)
                        .or_insert((
                            crate::minecraft::activity::GameActivity::Loading,
                            chrono::Utc::now().timestamp(),
                        ))
                        .0 = activity;
                }
            }
            AppEvent::PlayFailed(id) => {
                self.game_activities.remove(&id);
                self.playing.insert(id, false);
                self.global_status.clear();
                self.global_frac = None;
                self.refresh_instances();
                self.launcher_hidden = false;
                self.launcher_minimized = false;
                crate::utils::system::show_window_for_current_process(true);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                if self.config.start_maximized {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                    egui::UserAttentionType::Critical,
                ));
                ctx.request_repaint();
            }
            AppEvent::PlaySpawned => {
                if self.config.close_action == crate::config::CloseAction::Hide {
                    self.launcher_hidden = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                } else if self.config.close_action == crate::config::CloseAction::Minimize {
                    self.launcher_minimized = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                self.thumbnails.clear();
                self.pending_thumbs.clear();
                self.global_status.clear();
                self.global_frac = None;
                self.notice.clear();
                ctx.request_repaint();
            }
            AppEvent::PlayFinished {
                id,
                code,
                log_file,
                launched_at,
            } => {
                self.playing.insert(id.clone(), false);
                self.game_activities.remove(&id);
                if let Some(launched) = launched_at {
                    if let Ok(duration) = std::time::SystemTime::now().duration_since(launched) {
                        let _ = self.instances.add_play_time(&id, duration.as_secs());
                    }
                }
                if code == 0 {
                    self.last_exit.clear();
                } else {
                    self.last_exit = format!("Minecraft exited with code {code}.");
                    self.handle_game_crash(&id, code, log_file.as_deref(), launched_at);
                }
                self.refresh_instances();
                if self.log_lines.len() > 300 {
                    let keep_from = self.log_lines.len() - 300;
                    self.log_lines.drain(..keep_from);
                }
                if !self.playing.values().any(|p| *p) {
                    self.launcher_hidden = false;
                    self.launcher_minimized = false;
                    crate::utils::system::show_window_for_current_process(true);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    if self.config.start_maximized {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                        egui::UserAttentionType::Critical,
                    ));
                    ctx.request_repaint();
                }
            }
            AppEvent::RestoreWindow => {
                self.error_dialog.clear();
                self.launcher_hidden = false;
                self.launcher_minimized = false;
                crate::utils::system::show_window_for_current_process(true);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                if self.config.start_maximized {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                    egui::UserAttentionType::Critical,
                ));
                ctx.request_repaint();
            }
            AppEvent::LauncherUpdate(res) => {
                self.launcher_update_loading = false;
                match res {
                    Ok(info) => {
                        if info.has_update {
                            self.show_update_banner = true;
                            self.notify(format!(
                                "MONORYX v{} update available!",
                                info.latest_version
                            ));
                        } else {
                            self.notify("You are running the latest version of MONORYX.");
                        }
                        self.launcher_update = Some(info);
                    }
                    Err(e) => {
                        self.notify(format!("Update check failed: {e}"));
                        self.launcher_update_error = Some(e);
                    }
                }
            }
            AppEvent::LauncherUpdateDownloadProgress(progress) => {
                self.launcher_update_download_progress = progress;
            }
            AppEvent::LauncherUpdateDownloaded(result) => {
                self.launcher_update_download_loading = false;
                match result {
                    Ok(path) => {
                        self.launcher_update_downloaded = Some(path);
                        self.notify("Update downloaded! Click 'Restart to update' to apply.");
                    }
                    Err(error) => self.launcher_update_download_error = Some(error),
                }
            }
            AppEvent::MicrosoftDeviceCode(result) => {
                if self.ms_login_cancel.is_none() {
                    return;
                }
                match result {
                    Ok(code) => {
                        self.ms_device_code = Some(code);
                        self.ms_login_loading = true;
                        self.ms_login_error = None;
                    }
                    Err(err) => {
                        self.ms_login_cancel = None;
                        self.ms_login_loading = false;
                        self.ms_login_error = Some(err);
                    }
                }
            }
            AppEvent::MicrosoftLoginDone(result) => {
                if self.ms_login_cancel.is_none() {
                    return;
                }
                self.ms_login_cancel = None;
                self.ms_login_loading = false;
                self.ms_device_code = None;
                match result {
                    Ok(profile) => {
                        self.notify(format!(
                            "Welcome, {}! Microsoft account linked.",
                            profile.username
                        ));
                        self.config.microsoft_profile = Some(profile);
                        if self.page == Page::Onboarding {
                            if self.onboarding_use_microsoft {
                                self.config.use_microsoft_auth = true;
                                if self.onboarding_step == 1 {
                                    self.onboarding_step = 2;
                                }
                            }
                        } else {
                            self.config.use_microsoft_auth = true;
                        }
                        self.save_config();
                    }
                    Err(err) => {
                        if self.page != Page::Onboarding || self.onboarding_use_microsoft {
                            self.ms_login_error = Some(err);
                        }
                    }
                }
            }
            AppEvent::MicrosoftSessionRefreshed(profile) => {
                if self
                    .config
                    .microsoft_profile
                    .as_ref()
                    .is_some_and(|current| current.uuid == profile.uuid)
                {
                    self.config.microsoft_profile = Some(profile);
                    self.save_config();
                }
            }
            AppEvent::PlayLog(line) => {
                self.log_lines.push(line);
                if self.log_lines.len() > 1000 {
                    let drain = self.log_lines.len() - 1000;
                    self.log_lines.drain(0..drain);
                }
            }
            AppEvent::Download(event) => {
                apply_download_event(&mut self.downloads, &mut self.downloads_history, event);
            }
            AppEvent::RepairDone(r) => {
                let ok = r.is_ok();
                let message = r.as_ref().err().cloned().unwrap_or_default();
                self.handle_operation_done("repair", ok, message);
                match r {
                    Ok(problems) => {
                        if problems.is_empty() {
                            self.notify("Repair finished: everything verified");
                        } else {
                            self.notify(format!(
                                "Repair finished: {} issue(s) fixed",
                                problems.len()
                            ));
                        }
                        self.refresh_instances();
                    }
                    Err(e) => self.fail(e),
                }
            }
        }
    }

    fn handle_operation_done(&mut self, id: &str, ok: bool, message: String) {
        let label = self.operations.get(id).map_or_else(
            || match id {
                "mod-install" => "Downloading mod and dependencies".to_string(),
                "modpack-install" => "Installing modpack".to_string(),
                "repair" => "Repairing instance".to_string(),
                _ => id.to_string(),
            },
            |op| op.label.clone(),
        );
        self.operations.remove(id);
        self.global_status.clear();
        self.global_frac = None;
        self.downloads_history.insert(
            0,
            TrackedDownload {
                id: id.to_string(),
                label,
                downloaded: 0,
                total: None,
                speed_bps: 0.0,
                state: if ok { "completed" } else { "failed" }.to_string(),
                message,
            },
        );
        self.downloads_history.truncate(50);
    }

    pub fn handle_game_crash(
        &mut self,
        instance_id: &str,
        code: i32,
        log_file: Option<&std::path::Path>,
        launched_at: Option<std::time::SystemTime>,
    ) {
        let instance_name = self
            .instances
            .get(instance_id)
            .map(|c| c.name)
            .unwrap_or_else(|_| instance_id.to_string());
        let game_dir = self.instances.game_dir(instance_id);
        let crash_info = crate::minecraft::crash::detect_crash(
            instance_id,
            &instance_name,
            &game_dir,
            code,
            log_file,
            launched_at,
        );
        self.crash_report = Some(crash_info);
        self.crash_share_generation = self.crash_share_generation.wrapping_add(1);
        self.crash_share_loading = false;
        self.crash_share_url = None;
        self.crash_share_error.clear();
    }

    pub fn share_crash_log(&mut self) {
        let Some(info) = self.crash_report.clone() else {
            return;
        };
        if self.crash_share_loading {
            return;
        }
        self.crash_share_loading = true;
        self.crash_share_url = None;
        self.crash_share_error.clear();
        let generation = self.crash_share_generation;
        let http = self.http.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.spawn(async move {
            let result = crate::minecraft::crash::share_on_mclogs(&http, &info).await;
            let _ = tx.send(AppEvent::CrashLogShared(generation, result));
            ctx.request_repaint();
        });
    }

    pub fn sync_search_filters(&mut self) {
        if let Some(cfg) = self.selected() {
            if self.search.game_version.is_empty() {
                self.search.game_version = cfg.minecraft_version.clone();
            }
            if (self.discover_tab == DiscoverTab::Mods
                || self.discover_tab == DiscoverTab::Modpacks)
                && self.search.loader.is_empty()
            {
                self.search.loader = match cfg.loader {
                    LoaderKind::Fabric => "fabric".to_string(),
                    LoaderKind::Quilt => "quilt".to_string(),
                    LoaderKind::Forge => "forge".to_string(),
                    LoaderKind::Neoforge => "neoforge".to_string(),
                    LoaderKind::Vanilla => String::new(),
                };
            }
        }
        self.search.project_type = self.discover_tab.project_type().to_string();
    }

    pub fn queue_search(&mut self, _immediate: bool) {
        self.run_search();
    }

    pub fn run_search(&mut self) {
        self.search_generation = self.search_generation.wrapping_add(1);
        let generation = self.search_generation;
        self.detail_slug = None;
        self.detail_project = None;
        self.detail_versions.clear();
        self.detail_loading = false;
        self.detail_versions_loading = false;
        self.detail_version_pick.clear();
        self.markdown_blocks.clear();
        self.search_loading = true;
        self.search_error.clear();
        let mr = self.mr.clone();
        let tx = self.tx.clone();
        let q = self.search.query.clone();
        let pt = self.search.project_type.clone();
        let gv = self.search.game_version.clone();

        let loader = if pt == "shader" || pt == "resourcepack" {
            String::new()
        } else {
            self.search.loader.clone()
        };
        let sort = self.search.sort.api_value().to_string();
        let offset = self.search.offset;
        let slots = self.metadata_slots.clone();
        if self.discover_source == DiscoverSource::CurseForge {
            let cf = crate::curseforge::CurseForgeClient::with_server(
                self.http.clone(),
                &self.config.curseforge.api_key,
                &self.config.curseforge.custom_endpoint,
            );
            let class_id =
                crate::curseforge::class_id_for(&pt).unwrap_or(crate::curseforge::class::MODS);
            let sort_field = match self.search.sort {
                crate::modrinth::search::SortOrder::Relevance => {
                    crate::curseforge::sort::POPULARITY
                }
                crate::modrinth::search::SortOrder::Downloads => {
                    crate::curseforge::sort::TOTAL_DOWNLOADS
                }
                crate::modrinth::search::SortOrder::Newest => crate::curseforge::sort::LAST_UPDATED,
                crate::modrinth::search::SortOrder::Updated => {
                    crate::curseforge::sort::LAST_UPDATED
                }
            };
            self.runtime.spawn(async move {
                let Ok(_permit) = slots.acquire_owned().await else {
                    return;
                };
                let r = cf
                    .search(
                        &q,
                        class_id,
                        if gv.is_empty() { None } else { Some(&gv) },
                        if loader.is_empty() {
                            None
                        } else {
                            Some(&loader)
                        },
                        sort_field,
                        offset,
                        24,
                    )
                    .await
                    .map(|page| crate::modrinth::models::SearchResponse {
                        hits: page
                            .data
                            .iter()
                            .map(crate::curseforge::to_search_result)
                            .collect(),
                        offset: page.pagination.index,
                        limit: page.pagination.page_size,
                        total_hits: page.pagination.total_count,
                    })
                    .map_err(|e| e.user_message());
                let _ = tx.send(AppEvent::SearchDone(generation, r));
            });
            return;
        }

        self.runtime.spawn(async move {
            let Ok(_permit) = slots.acquire_owned().await else {
                return;
            };
            let r = mr
                .search(
                    &q,
                    Some(&pt),
                    if gv.is_empty() { None } else { Some(&gv) },
                    if loader.is_empty() {
                        None
                    } else {
                        Some(&loader)
                    },
                    &sort,
                    24,
                    offset,
                )
                .await
                .map_err(|e| e.user_message());
            let _ = tx.send(AppEvent::SearchDone(generation, r));
        });
    }

    pub fn spawn_initial(&mut self) {
        if self.manifest.is_none() {
            let cache = crate::storage::cache::DiskCache::new(
                self.paths.manifests_dir(),
                std::time::Duration::from_secs(3600),
            );
            self.manifest = cache
                .get_stale("mojang-version-manifest-v2")
                .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        }
        self.manifest_loading = true;
        let http = self.http.clone();
        let tx = self.tx.clone();
        let paths = self.paths.clone();
        let slots = self.metadata_slots.clone();
        self.runtime.spawn(async move {
            let Ok(_permit) = slots.acquire_owned().await else {
                return;
            };
            let cache = crate::storage::cache::DiskCache::new(
                paths.manifests_dir(),
                std::time::Duration::from_secs(3600),
            );
            let r = crate::minecraft::manifest::fetch_manifest(&http, &cache)
                .await
                .map_err(|e| e.user_message());
            let _ = tx.send(AppEvent::VersionsLoaded(r));
        });
        self.refresh_java();
        if self.config.auto_check_updates {
            self.check_launcher_update();
        }
    }

    pub fn check_launcher_update(&mut self) {
        if self.launcher_update_loading || self.launcher_update_download_loading {
            return;
        }
        self.launcher_update_loading = true;
        self.launcher_update_error = None;
        self.launcher_update = None;
        self.launcher_update_downloaded = None;
        self.launcher_update_download_error = None;
        crate::app::tasks::check_launcher_update(self);
    }

    pub fn download_launcher_update(&mut self) {
        if self.launcher_update_download_loading {
            return;
        }
        let Some(info) = self.launcher_update.as_ref() else {
            return;
        };
        if info.download_url.is_none() || info.download_asset.is_none() {
            return;
        }
        let info = info.clone();
        self.launcher_update_download_loading = true;
        self.launcher_update_download_progress = None;
        self.launcher_update_downloaded = None;
        self.launcher_update_download_error = None;
        crate::app::tasks::download_launcher_update(self, info);
    }

    pub fn start_microsoft_login(&mut self) {
        self.cancel_microsoft_login();
        self.ms_login_loading = true;
        self.ms_login_error = None;
        self.ms_device_code = None;
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.ms_login_cancel = Some(cancel.clone());
        crate::app::tasks::start_microsoft_login(self, cancel);
    }

    pub fn cancel_microsoft_login(&mut self) {
        if let Some(c) = self.ms_login_cancel.take() {
            c.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.ms_login_loading = false;
        self.ms_device_code = None;
        self.ms_login_error = None;
    }

    pub fn replay_onboarding(&mut self) {
        self.cancel_microsoft_login();
        self.config.completed_onboarding = false;
        self.onboarding_step = 0;
        self.onboarding_error.clear();
        if self.onboarding_user.is_empty() {
            if let Some(p) = &self.config.profile {
                self.onboarding_user = p.username.clone();
            }
        }
        self.onboarding_use_microsoft =
            self.config.use_microsoft_auth && self.config.microsoft_profile.is_some();
        self.page = Page::Onboarding;
    }

    pub fn toggle_boost(&mut self) {
        if let Some(mut cfg) = self.selected() {
            let current = cfg.boost_mode.unwrap_or(self.config.boost_mode);
            let next = !current;
            cfg.boost_mode = Some(next);
            let _ = self.instances.save(&cfg);
            self.refresh_instances();
            if next {
                self.notify("Eco mode on: Minecraft will use a lower memory limit next launch.");
            } else {
                self.notify(
                    "Eco mode off: Minecraft will use its normal memory limit next launch.",
                );
            }
        } else {
            self.config.boost_mode = !self.config.boost_mode;
            self.save_config();
            if self.config.boost_mode {
                self.notify("Eco Mode Enabled by default!");
            } else {
                self.notify("Eco Mode Disabled by default.");
            }
        }
    }

    pub fn refresh_gpus(&mut self) {
        self.gpu_loading = true;
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            let list = crate::utils::system::detect_gpus().await;
            let _ = tx.send(AppEvent::GpuList(list));
        });
    }

    pub fn refresh_gpus_force(&mut self) {
        self.gpu_loading = true;
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            let list = crate::utils::system::detect_gpus_force().await;
            let _ = tx.send(AppEvent::GpuList(list));
        });
    }

    pub fn refresh_java(&mut self) {
        self.java_loading = true;
        let tx = self.tx.clone();
        let dir = self.paths.java_dir();
        self.runtime.spawn(async move {
            let list = crate::java::discovery::discover_all(&dir).await;
            let _ = tx.send(AppEvent::JavaList(list));
        });
    }

    pub fn ensure_thumb(&mut self, url: &str) {
        if url.is_empty()
            || !url.starts_with("https://")
            || self.thumbnails.contains_key(url)
            || self.pending_thumbs.contains_key(url)
            || self.failed_thumbs.contains(url)
        {
            return;
        }
        self.pending_thumbs.insert(url.to_string(), true);
        let http = self.http.clone();
        let tx = self.tx.clone();
        let url_owned = url.to_string();
        let img_dir = self.paths.images_dir();
        let slots = self.image_slots.clone();
        self.runtime.spawn(async move {
            let Ok(_permit) = slots.acquire_owned().await else {
                return;
            };
            let cache = crate::storage::cache::DiskCache::new(
                img_dir,
                std::time::Duration::from_secs(7 * 24 * 3600),
            );
            let key = format!(
                "thumb-{}",
                crate::utils::hash::sha1_bytes(url_owned.as_bytes())
            );
            let result = match cache.get(&key) {
                Some(bytes) => Ok(bytes),
                None => fetch_image_bytes(&http, &url_owned, 8_000_000)
                    .await
                    .inspect(|bytes| {
                        let _ = cache.put(&key, bytes);
                    }),
            };
            let decoded = match result {
                Ok(bytes) => tokio::task::spawn_blocking(move || decode_image(bytes, 128, 128))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r),
                Err(error) => Err(error),
            };
            let _ = tx.send(AppEvent::Thumbnail(url_owned, decoded));
        });
    }

    pub fn open_project_image(&mut self, url: &str) {
        if !url.starts_with("https://") {
            self.project_image_error = "This image URL is not supported.".to_string();
            return;
        }
        if self.project_image_url.as_deref() == Some(url)
            && (self.project_image_loading || self.project_image.is_some())
        {
            return;
        }
        self.project_image_url = Some(url.to_string());
        self.project_image = None;
        self.project_image_loading = true;
        self.project_image_error.clear();
        let http = self.http.clone();
        let tx = self.tx.clone();
        let url_owned = url.to_string();
        let img_dir = self.paths.images_dir();
        let slots = self.image_slots.clone();
        self.runtime.spawn(async move {
            let Ok(_permit) = slots.acquire_owned().await else {
                return;
            };
            let cache = crate::storage::cache::DiskCache::new(
                img_dir,
                std::time::Duration::from_secs(7 * 24 * 3600),
            );
            let key = format!(
                "gallery-{}",
                crate::utils::hash::sha1_bytes(url_owned.as_bytes())
            );
            let result = match cache.get(&key) {
                Some(bytes) => Ok(bytes),
                None => fetch_image_bytes(&http, &url_owned, 12_000_000)
                    .await
                    .inspect(|bytes| {
                        let _ = cache.put(&key, bytes);
                    }),
            };
            let decoded = match result {
                Ok(bytes) => tokio::task::spawn_blocking(move || decode_image(bytes, 1600, 1000))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r),
                Err(error) => Err(error),
            };
            let _ = tx.send(AppEvent::ProjectImage(url_owned, decoded));
        });
    }

    pub fn selected_version_list(&self) -> Vec<String> {
        if let Some(m) = &self.manifest {
            let mut v: Vec<String> = m
                .versions
                .iter()
                .filter(|e| {
                    if e.kind == "release" {
                        return true;
                    }
                    if e.kind == "snapshot" {
                        return self.show_snapshots;
                    }
                    self.show_snapshots
                })
                .map(|e| e.id.clone())
                .collect();
            if v.is_empty() {
                v = m.versions.iter().map(|e| e.id.clone()).collect();
            }
            v
        } else {
            Vec::new()
        }
    }

    pub fn nexeu_refresh(&mut self) {
        if self.nexeu.api_key.trim().is_empty() {
            self.nexeu.error = "Enter a Nexeu game-panel API key first.".to_string();
            return;
        }
        self.nexeu.loading = true;
        self.nexeu.generation = self.nexeu.generation.wrapping_add(1);
        let generation = self.nexeu.generation;
        self.nexeu.overview = None;
        self.nexeu.selected_server = None;
        self.nexeu.resources = None;
        self.nexeu.logs = None;
        self.nexeu.backups = None;
        self.nexeu.console_command.clear();
        self.nexeu.error.clear();
        let http = self.http.clone();
        let key = self.nexeu.api_key.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.spawn(async move {
            let result = crate::nexeu::overview(&http, &key).await;
            let _ = tx.send(AppEvent::NexeuOverview(generation, result));
            ctx.request_repaint();
        });
    }

    pub fn nexeu_select_server(&mut self, id: String) {
        let generation = self.nexeu.generation;
        if self.nexeu.selected_server.as_deref() != Some(&id) {
            self.nexeu.console_command.clear();
        }
        self.nexeu.selected_server = Some(id.clone());
        self.nexeu.resources = None;
        self.nexeu.logs = None;
        self.nexeu.backups = None;
        let http = self.http.clone();
        let key = self.nexeu.api_key.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.spawn(async move {
            let result = crate::nexeu::resources(&http, &key, &id).await;
            let _ = tx.send(AppEvent::NexeuResources(generation, id, result));
            ctx.request_repaint();
        });
        if let Some(id) = self.nexeu.selected_server.clone() {
            self.nexeu_load_backups(id.clone());
            self.nexeu_load_logs(id);
        }
    }

    pub fn nexeu_load_logs(&self, id: String) {
        let generation = self.nexeu.generation;
        let http = self.http.clone();
        let key = self.nexeu.api_key.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.spawn(async move {
            let result = crate::nexeu::logs(&http, &key, &id).await;
            let _ = tx.send(AppEvent::NexeuLogs(generation, id, result));
            ctx.request_repaint();
        });
    }

    pub fn nexeu_power(&self, id: String, action: &'static str) {
        let generation = self.nexeu.generation;
        let http = self.http.clone();
        let key = self.nexeu.api_key.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.spawn(async move {
            let result = crate::nexeu::power(&http, &key, &id, action)
                .await
                .map(|()| action.to_string());
            let _ = tx.send(AppEvent::NexeuPower(generation, result));
            ctx.request_repaint();
        });
    }

    pub fn nexeu_load_backups(&self, id: String) {
        let generation = self.nexeu.generation;
        let http = self.http.clone();
        let key = self.nexeu.api_key.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.spawn(async move {
            let result = crate::nexeu::backups(&http, &key, &id).await;
            let _ = tx.send(AppEvent::NexeuBackups(generation, id, result));
            ctx.request_repaint();
        });
    }

    pub fn nexeu_create_backup(&self, id: String) {
        let generation = self.nexeu.generation;
        let http = self.http.clone();
        let key = self.nexeu.api_key.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.spawn(async move {
            let result = crate::nexeu::create_backup(&http, &key, &id).await;
            let _ = tx.send(AppEvent::NexeuBackupCreated(generation, id, result));
            ctx.request_repaint();
        });
    }

    pub fn nexeu_send_command(&self, id: String) {
        let generation = self.nexeu.generation;
        let http = self.http.clone();
        let key = self.nexeu.api_key.clone();
        let command = self.nexeu.console_command.clone();
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.spawn(async move {
            let result = crate::nexeu::command(&http, &key, &id, &command).await;
            let _ = tx.send(AppEvent::NexeuCommand(generation, result));
            ctx.request_repaint();
        });
    }

    pub fn spawn<Fut>(&self, fut: Fut)
    where
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.runtime.spawn(fut);
    }
}

fn apply_download_event(
    downloads: &mut HashMap<String, TrackedDownload>,
    history: &mut Vec<TrackedDownload>,
    event: crate::downloads::job::DownloadEvent,
) {
    let crate::downloads::job::DownloadEvent {
        id,
        label,
        state,
        progress,
        message,
    } = event;
    let terminal = matches!(
        state,
        JobState::Completed | JobState::Failed | JobState::Cancelled
    );
    if !terminal {
        downloads.insert(
            id,
            TrackedDownload {
                id: String::new(),
                label,
                downloaded: progress.downloaded,
                total: progress.total,
                speed_bps: progress.speed_bps,
                state: format!("{state:?}").to_lowercase(),
                message,
            },
        );
        return;
    }
    let tracked = downloads.remove(&id).map_or_else(
        || TrackedDownload {
            id: id.clone(),
            label,
            downloaded: progress.downloaded,
            total: progress.total,
            speed_bps: 0.0,
            state: format!("{state:?}").to_lowercase(),
            message: message.clone(),
        },
        |mut d| {
            d.state = format!("{state:?}").to_lowercase();
            d.message = message.clone();
            d
        },
    );
    history.insert(0, tracked);
    history.truncate(50);
}

async fn fetch_image_bytes(
    http: &reqwest::Client,
    url: &str,
    limit: usize,
) -> std::result::Result<Vec<u8>, String> {
    let mut response = http
        .get(url)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("Image is too large to display.".to_string());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if bytes.len() + chunk.len() > limit {
            return Err("Image is too large to display.".to_string());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn decode_image(
    bytes: Vec<u8>,
    max_width: u32,
    max_height: u32,
) -> std::result::Result<DecodedImage, String> {
    let reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| error.to_string())?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 50_000_000 {
        return Err("Image dimensions are too large to display.".to_string());
    }
    let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
    let rgba = image.thumbnail(max_width, max_height).to_rgba8();
    Ok(DecodedImage {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        pixels: rgba.into_raw(),
    })
}

fn read_local_image(
    path: &std::path::Path,
    max_width: u32,
    max_height: u32,
) -> Result<DecodedImage, String> {
    let size = std::fs::metadata(path)
        .map_err(|error| error.to_string())?
        .len();
    if size > 50_000_000 {
        return Err("Image file is too large to display.".to_string());
    }
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    decode_image(bytes, max_width, max_height)
}

#[cfg(test)]
mod background_tests {
    use super::*;

    #[test]
    fn hierarchy_results_cannot_overwrite_a_newer_instance_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, MonoryxPaths::new(dir.path().into()));
        state.selected_instance = Some("active".into());
        state.hierarchy_generation = 2;
        state.hierarchy_pending = true;
        state.handle_event(
            AppEvent::HierarchyBuilt(
                1,
                "active".into(),
                Ok(crate::content::DependencyHierarchy::default()),
            ),
            &ctx,
        );
        state.handle_event(
            AppEvent::HierarchyBuilt(
                2,
                "previous".into(),
                Ok(crate::content::DependencyHierarchy::default()),
            ),
            &ctx,
        );
        assert!(state.cached_hierarchy.is_none() && state.hierarchy_pending);
        state.handle_event(
            AppEvent::HierarchyBuilt(
                2,
                "active".into(),
                Ok(crate::content::DependencyHierarchy::default()),
            ),
            &ctx,
        );
        assert!(state.cached_hierarchy.is_some() && !state.hierarchy_pending);
    }

    #[test]
    fn background_session_refresh_keeps_current_settings_and_ignores_another_account() {
        let dir = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(dir.path().into());
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths.clone());
        let mut profile = crate::account::microsoft::MicrosoftProfile {
            username: "Player".into(),
            uuid: uuid::Uuid::new_v4(),
            access_token: "original".into(),
            refresh_token: "refresh".into(),
            expires_at: 100,
        };
        state.config.microsoft_profile = Some(profile.clone());
        state.config.last_page = "library".into();
        state.page = Page::Library;
        let mut other = profile.clone();
        other.uuid = uuid::Uuid::new_v4();
        state.handle_event(AppEvent::MicrosoftSessionRefreshed(other), &ctx);
        assert_eq!(
            state.config.microsoft_profile.as_ref().unwrap().uuid,
            profile.uuid
        );
        profile.access_token = "renewed".into();
        state.handle_event(AppEvent::MicrosoftSessionRefreshed(profile), &ctx);
        let loaded = LauncherConfig::load(&paths.config_file()).unwrap();
        assert_eq!(loaded.last_page, "library");
        assert_eq!(loaded.microsoft_profile.unwrap().access_token, "renewed");
    }
}
