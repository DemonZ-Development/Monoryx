use crate::app::events::{AppEvent, Page};
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
use crate::modrinth::search::{SearchFilters, SortOrder};
use crate::modrinth::updates::UpdateInfo;
use crate::storage::paths::MonoryxPaths;
use std::collections::HashMap;
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
}

pub struct AppState {
    pub paths: MonoryxPaths,
    pub egui_ctx: egui::Context,
    pub config: LauncherConfig,
    pub http: reqwest::Client,
    pub dm: DownloadManager,
    pub mr: ModrinthClient,
    pub instances: InstanceManager,
    pub tx: Sender<AppEvent>,
    pub rx: Receiver<AppEvent>,
    pub runtime: tokio::runtime::Runtime,
    pub page: Page,
    pub instance_list: Vec<InstanceConfig>,
    pub selected_instance: Option<String>,
    pub manifest: Option<VersionManifest>,
    pub manifest_loading: bool,
    pub versions_error: String,
    pub show_snapshots: bool,
    pub search: SearchFilters,
    pub search_results: Vec<crate::modrinth::models::SearchResult>,
    pub search_total: u32,
    pub search_loading: bool,
    pub search_error: String,
    pub search_debounce: Option<Instant>,
    pub discover_tab: ContentKind,
    pub detail_project: Option<Project>,
    pub detail_versions: Vec<ProjectVersion>,
    pub detail_loading: bool,
    pub detail_error: String,
    pub detail_version_pick: String,
    pub library_entries: Vec<InstalledEntry>,
    pub library_filter: ContentKind,
    pub updates: Vec<UpdateInfo>,
    pub updates_loading: bool,
    pub operations: std::collections::BTreeMap<String, InstallOperation>,
    pub downloads: HashMap<String, TrackedDownload>,
    pub downloads_history: Vec<TrackedDownload>,
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
    pub notice: String,
    pub notice_at: Option<Instant>,
    pub error_dialog: String,
    pub confirm_delete: Option<String>,
    pub confirm_title: String,
    pub edit_instance: Option<InstanceConfig>,
    pub edit_error: String,
    pub new_draft: NewInstanceDraft,
    pub show_new_instance: bool,
    pub thumbnails: HashMap<String, egui::TextureHandle>,
    pub pending_thumbs: HashMap<String, bool>,
    pub global_status: String,
    pub global_frac: Option<f32>,
    pub settings_jvm: String,
    pub settings_game_args: String,
    pub settings_mem_min: String,
    pub settings_mem_max: String,
    pub settings_width: String,
    pub settings_height: String,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let paths = MonoryxPaths::global();
        let _ = paths.ensure_all();
        Self::new_with_paths(cc, paths)
    }

    pub fn new_with_paths(_cc: &eframe::CreationContext<'_>, paths: MonoryxPaths) -> Self {
        let _ = paths.ensure_all();
        let config = LauncherConfig::load(&paths.config_file()).unwrap_or_default();
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
        let mut s = Self {
            egui_ctx: _cc.egui_ctx.clone(),
            paths,
            config,
            http,
            dm,
            mr,
            instances,
            tx,
            rx,
            runtime,
            page,
            instance_list,
            selected_instance,
            manifest: None,
            manifest_loading: false,
            versions_error: String::new(),
            show_snapshots,
            search: SearchFilters::default(),
            search_results: Vec::new(),
            search_total: 0,
            search_loading: false,
            search_error: String::new(),
            search_debounce: None,
            discover_tab: ContentKind::Mod,
            detail_project: None,
            detail_versions: Vec::new(),
            detail_loading: false,
            detail_error: String::new(),
            detail_version_pick: String::new(),
            library_entries: Vec::new(),
            library_filter: ContentKind::Mod,
            updates: Vec::new(),
            updates_loading: false,
            operations: std::collections::BTreeMap::new(),
            downloads: HashMap::new(),
            downloads_history: Vec::new(),
            java_list: Vec::new(),
            java_loading: false,
            gpu_list: Vec::new(),
            gpu_loading: false,
            busy_install: HashMap::new(),
            playing: HashMap::new(),
            last_exit: String::new(),
            launcher_hidden: false,
            launcher_minimized: false,
            log_lines: Vec::new(),
            onboarding_step: 0,
            onboarding_user: String::new(),
            onboarding_error: String::new(),
            onboarding_mem_auto: true,
            onboarding_mem_max: crate::utils::system::default_max_memory_mb().to_string(),
            notice: String::new(),
            notice_at: None,
            error_dialog: String::new(),
            confirm_delete: None,
            confirm_title: String::new(),
            edit_instance: None,
            edit_error: String::new(),
            new_draft: NewInstanceDraft::default(),
            show_new_instance: false,
            thumbnails: HashMap::new(),
            pending_thumbs: HashMap::new(),
            global_status: String::new(),
            global_frac: None,
            settings_jvm: String::new(),
            settings_game_args: String::new(),
            settings_mem_min: String::new(),
            settings_mem_max: String::new(),
            settings_width: String::new(),
            settings_height: String::new(),
        };
        s.settings_jvm = s.config.default_jvm_args.clone();
        s.settings_game_args = s.config.default_game_args.clone();
        s.settings_mem_min = s.config.memory.min_mb.to_string();
        s.settings_mem_max = s.config.memory.max_mb.to_string();
        s.refresh_instances();
        s.spawn_initial();
        s
    }

    pub fn save_config(&mut self) {
        self.config.last_page = self.page.as_str().to_string();
        self.config.selected_instance = self.selected_instance.clone();
        self.config.show_snapshots = self.show_snapshots;
        let _ = self.config.save(&self.paths.config_file());
    }

    pub fn refresh_instances(&mut self) {
        self.instance_list = self.instances.list().unwrap_or_default();
        if self
            .selected_instance
            .as_ref()
            .is_none_or(|id| self.instance_list.iter().all(|c| &c.id != id))
        {
            self.selected_instance = self.instance_list.first().map(|c| c.id.clone());
        }
        self.config.selected_instance = self.selected_instance.clone();
        self.refresh_library();
    }

    pub fn selected(&self) -> Option<InstanceConfig> {
        self.selected_instance
            .as_ref()
            .and_then(|id| self.instance_list.iter().find(|c| &c.id == id).cloned())
    }

    pub fn set_page(&mut self, page: Page) {
        self.page = page;
        self.save_config();
        match page {
            Page::Library => self.refresh_library(),
            Page::Discover if self.search_results.is_empty() && !self.search_loading => {
                self.queue_search(true);
            }
            _ => {}
        }
    }

    pub fn notify(&mut self, msg: impl Into<String>) {
        self.notice = msg.into();
        self.notice_at = Some(Instant::now());
    }

    pub fn fail(&mut self, msg: impl Into<String>) {
        self.error_dialog = msg.into();
    }

    pub fn refresh_library(&mut self) {
        if let Some(id) = self.selected_instance.clone() {
            let store =
                crate::content::ContentStore::for_instance(&self.instances.instance_dir(&id));
            let mut entries = store.load().entries;
            entries.sort_by(|a, b| {
                a.project_title
                    .clone()
                    .unwrap_or_default()
                    .to_lowercase()
                    .cmp(&b.project_title.clone().unwrap_or_default().to_lowercase())
            });
            self.library_entries = entries;
        } else {
            self.library_entries = Vec::new();
        }
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
            AppEvent::Notice(m) => self.notify(m),
            AppEvent::Error(m) => self.fail(m),
            AppEvent::VersionsLoaded(r) => {
                self.manifest_loading = false;
                match r {
                    Ok(m) => {
                        self.manifest = Some(m);
                        self.versions_error.clear();
                        self.sync_search_filters();
                    }
                    Err(e) => self.versions_error = e,
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
            AppEvent::SearchDone(r) => {
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
            AppEvent::ProjectDetail(r) => {
                self.detail_loading = false;
                match r {
                    Ok(p) => {
                        self.detail_project = Some(p);
                        self.detail_error.clear();
                    }
                    Err(e) => self.detail_error = e,
                }
            }
            AppEvent::ProjectVersions(r) => match r {
                Ok(v) => self.detail_versions = v,
                Err(e) => self.detail_error = e,
            },
            AppEvent::Thumbnail(url, r) => {
                self.pending_thumbs.remove(&url);
                if let Ok(bytes) = r {
                    if let Ok(img) = image::load_from_memory(&bytes) {
                        let rgba = img.to_rgba8();
                        let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                        let pixels = rgba.into_raw();
                        let cimg = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
                        let tex = ctx.load_texture(url.clone(), cimg, egui::TextureOptions::LINEAR);
                        self.thumbnails.insert(url, tex);
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
                let ok = r.is_ok();
                let message = r.as_ref().err().cloned().unwrap_or_default();
                self.handle_operation_done(&id, ok, message);
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
                        self.refresh_library();
                    }
                    Err(e) => self.fail(e),
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
            AppEvent::UpdatesFound(u) => {
                self.updates = u;
                self.updates_loading = false;
            }
            AppEvent::PlayStarted(id) => {
                self.playing.insert(id, true);
            }
            AppEvent::PlaySpawned => {
                if self.config.close_action == crate::config::CloseAction::Hide {
                    self.save_config();
                    self.launcher_hidden = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                } else if self.config.close_action == crate::config::CloseAction::Minimize {
                    self.launcher_minimized = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                self.global_status.clear();
                self.global_frac = None;
                self.notice.clear();
            }
            AppEvent::PlayFinished(id, code) => {
                self.playing.insert(id, false);
                if code == 0 {
                    self.last_exit.clear();
                } else {
                    self.last_exit = format!("Minecraft exited with code {code}.");
                }
                self.refresh_instances();
                if !self.playing.values().any(|p| *p) {
                    if self.launcher_hidden {
                        self.launcher_hidden = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    } else if self.launcher_minimized {
                        self.launcher_minimized = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                }
            }
            AppEvent::PlayLog(line) => {
                self.log_lines.push(line);
                if self.log_lines.len() > 2000 {
                    let drain = self.log_lines.len() - 2000;
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
        let label = self
            .operations
            .get(id)
            .map_or_else(|| id.to_string(), |op| op.label.clone());
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

    pub fn sync_search_filters(&mut self) {
        if let Some(cfg) = self.selected() {
            if self.search.game_version.is_empty() {
                self.search.game_version = cfg.minecraft_version.clone();
            }
            if self.search.loader.is_empty() {
                self.search.loader = match cfg.loader {
                    LoaderKind::Fabric => "fabric".to_string(),
                    LoaderKind::Quilt => "quilt".to_string(),
                    LoaderKind::Forge => "forge".to_string(),
                    LoaderKind::Neoforge => "neoforge".to_string(),
                    LoaderKind::Vanilla => "minecraft".to_string(),
                };
            }
        }
        self.search.project_type = match self.discover_tab {
            ContentKind::Mod => "mod".to_string(),
            ContentKind::Resourcepack => "resourcepack".to_string(),
            ContentKind::Shader => "shader".to_string(),
        };
    }

    pub fn queue_search(&mut self, immediate: bool) {
        if immediate {
            self.run_search();
        } else {
            self.search.loading_placeholder();
        }
    }

    pub fn run_search(&mut self) {
        self.sync_search_filters();
        self.search_loading = true;
        self.search_error.clear();
        let mr = self.mr.clone();
        let tx = self.tx.clone();
        let q = self.search.query.clone();
        let pt = self.search.project_type.clone();
        let gv = self.search.game_version.clone();
        let loader = self.search.loader.clone();
        let sort = self.search.sort.api_value().to_string();
        self.runtime.spawn(async move {
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
                    0,
                )
                .await
                .map_err(|e| e.user_message());
            let _ = tx.send(AppEvent::SearchDone(r));
        });
    }

    pub fn spawn_initial(&mut self) {
        self.manifest_loading = true;
        let http = self.http.clone();
        let tx = self.tx.clone();
        let paths = self.paths.clone();
        self.runtime.spawn(async move {
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
    }

    pub fn refresh_gpus(&mut self) {
        self.gpu_loading = true;
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            let list = crate::utils::system::detect_gpus().await;
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
            || self.thumbnails.contains_key(url)
            || self.pending_thumbs.contains_key(url)
        {
            return;
        }
        self.pending_thumbs.insert(url.to_string(), true);
        let http = self.http.clone();
        let tx = self.tx.clone();
        let url_owned = url.to_string();
        let img_dir = self.paths.images_dir();
        self.runtime.spawn(async move {
            let cache = crate::storage::cache::DiskCache::new(
                img_dir,
                std::time::Duration::from_secs(7 * 24 * 3600),
            );
            let key = format!(
                "thumb-{}",
                crate::utils::hash::sha1_bytes(url_owned.as_bytes())
            );
            if let Some(bytes) = cache.get(&key) {
                let _ = tx.send(AppEvent::Thumbnail(url_owned, Ok(bytes)));
                return;
            }
            let r = http
                .get(&url_owned)
                .timeout(std::time::Duration::from_secs(20))
                .send()
                .await
                .map_err(|e| e.to_string())
                .and_then(|resp| {
                    if !resp.status().is_success() {
                        return Err(format!("HTTP {}", resp.status()));
                    }
                    Ok(resp)
                });
            match r {
                Err(e) => {
                    let _ = tx.send(AppEvent::Thumbnail(url_owned, Err(e)));
                }
                Ok(resp) => {
                    let bytes = resp
                        .bytes()
                        .await
                        .map(|b| b.to_vec())
                        .map_err(|e| e.to_string());
                    if let Ok(ref b) = bytes {
                        if b.len() < 8_000_000 {
                            let _ = cache.put(&key, b);
                        }
                    }
                    let _ = tx.send(AppEvent::Thumbnail(url_owned, bytes));
                }
            }
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

trait SearchExt {
    fn loading_placeholder(&mut self);
}

impl SearchExt for SearchFilters {
    fn loading_placeholder(&mut self) {
        let _ = &self.query;
    }
}

pub fn sort_options() -> [SortOrder; 4] {
    SortOrder::all()
}
