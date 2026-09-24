use crate::app::events::AppEvent;
use crate::app::state::AppState;
use crate::content::ContentKind;
use crate::downloads::{DownloadJob, DownloadManager};
use crate::instance::config::LoaderKind;
use crate::modrinth::install::{install_project, InstallRequest};
use std::sync::Arc;

pub fn fetch_loader_versions(state: &AppState, loader: LoaderKind, mc: String, force: bool) {
    let key = format!("{mc}|{}", loader.as_str());
    if loader == LoaderKind::Vanilla || mc.is_empty() {
        let _ = state.tx.send(AppEvent::LoaderVersions(
            loader,
            key,
            Ok(vec![String::new()]),
        ));
        return;
    }
    let http = state.http.clone();
    let metadata_dir = state.paths.metadata_dir();
    let tx = state.tx.clone();
    state.runtime.spawn(async move {
        let cache = crate::storage::cache::DiskCache::new(
            metadata_dir,
            std::time::Duration::from_secs(900),
        );
        let cache_key = format!("loader-versions-{key}");
        let l = crate::loaders::loader_for(loader);
        let r = if let Some(cached) = (!force)
            .then(|| cache.get(&cache_key))
            .flatten()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        {
            Ok(cached)
        } else {
            match l.available_versions(&http, &mc).await {
                Ok(versions) => {
                    if let Ok(bytes) = serde_json::to_vec(&versions) {
                        let _ = cache.put(&cache_key, &bytes);
                    }
                    Ok(versions)
                }
                Err(error) if !force => cache
                    .get_stale(&cache_key)
                    .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                    .ok_or_else(|| error.user_message()),
                Err(error) => Err(error.user_message()),
            }
        };
        let _ = tx.send(AppEvent::LoaderVersions(loader, key, r));
    });
}

pub fn create_and_install(
    state: &AppState,
    name: String,
    mc: String,
    loader: LoaderKind,
    loader_version: String,
) {
    let tx = state.tx.clone();
    let paths = state.paths.clone();
    let dm = state.dm.clone();
    let http = state.http.clone();
    let instances = state.instances.clone();
    let op_id = uuid::Uuid::new_v4().to_string();
    let _ = tx.send(AppEvent::OperationStarted(
        op_id.clone(),
        format!("Installing {name}..."),
        None,
    ));
    state.runtime.spawn(async move {
        let cfg = match instances.create(name, mc.clone(), loader, loader_version.clone()) {
            Ok(c) => c,
            Err(e) => {
                let msg = e.user_message();
                let _ = tx.send(AppEvent::OperationFinished(op_id.clone(), Err(msg.clone())));
                let _ = tx.send(AppEvent::InstallDone(String::new(), Err(msg)));
                return;
            }
        };
        let id = cfg.id.clone();
        let _ = instances.ensure_game_dirs(&id);
        let op_tx = tx.clone();
        let op_id_for_prog = op_id.clone();
        let phase_cb: Arc<
            dyn Fn(crate::minecraft::installer::InstallPhase, usize, usize) + Send + Sync,
        > = Arc::new(move |phase, a, b| {
            let _ = op_tx.send(AppEvent::InstallProgress(
                op_id_for_prog.clone(),
                phase.label().to_string(),
                a,
                b.max(1),
            ));
        });
        let result =
            install_instance_inner(&dm, &http, &paths, &instances, &id, Some(phase_cb)).await;
        match result {
            Ok(version_id) => {
                let _ = tx.send(AppEvent::OperationFinished(op_id.clone(), Ok(())));
                let _ = tx.send(AppEvent::InstallDone(id, Ok(version_id)));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::OperationFinished(op_id, Err(e.clone())));
                let _ = tx.send(AppEvent::InstallDone(id, Err(e)));
            }
        }
    });
}

pub async fn install_instance_inner(
    dm: &DownloadManager,
    http: &reqwest::Client,
    paths: &crate::storage::paths::MonoryxPaths,
    instances: &crate::instance::manager::InstanceManager,
    instance_id: &str,
    phase_cb: Option<crate::minecraft::installer::PhaseCallback>,
) -> std::result::Result<String, String> {
    let mut cfg = instances.get(instance_id).map_err(|e| e.user_message())?;
    instances
        .ensure_game_dirs(instance_id)
        .map_err(|e| e.user_message())?;
    let loader = crate::loaders::loader_for(cfg.loader);
    let resolved = loader
        .install(dm, paths, &cfg.minecraft_version, &cfg.loader_version)
        .await
        .map_err(|e| e.user_message())?;
    cfg.resolved_version_id = resolved.id.clone();
    if cfg.loader != LoaderKind::Vanilla && cfg.loader_version.is_empty() {
        cfg.loader_version = guess_loader_version(cfg.loader, &cfg.minecraft_version, &resolved.id)
            .unwrap_or_default();
    }
    instances.save(&cfg).map_err(|e| e.user_message())?;
    let report = crate::minecraft::installer::install_version(dm, paths, &resolved, phase_cb)
        .await
        .map_err(|e| e.user_message())?;
    let _ = http;
    Ok(report.version_id)
}

fn guess_loader_version(loader: LoaderKind, mc: &str, resolved_id: &str) -> Option<String> {
    let version = match loader {
        LoaderKind::Fabric => resolved_id
            .strip_prefix("fabric-loader-")?
            .strip_suffix(&format!("-{mc}"))?,
        LoaderKind::Quilt => resolved_id
            .strip_prefix("quilt-loader-")?
            .strip_suffix(&format!("-{mc}"))?,
        LoaderKind::Forge => resolved_id.strip_prefix(&format!("{mc}-forge-"))?,
        LoaderKind::Neoforge => resolved_id
            .strip_prefix("neoforge-")
            .or_else(|| resolved_id.strip_prefix(&format!("{mc}-neoforge-")))?,
        LoaderKind::Vanilla => return None,
    };
    (!version.is_empty()).then(|| version.to_string())
}

pub fn play_instance(state: &mut AppState, instance_id: String) {
    if state.playing.values().any(|playing| *playing)
        || state
            .operations
            .values()
            .any(|op| op.instance_id.as_ref() == Some(&instance_id))
    {
        return;
    }
    state.last_exit.clear();
    state.error_dialog.clear();
    state.playing.insert(instance_id.clone(), true);
    state.notify("Preparing Minecraft. See Downloads for progress.");
    let operation_id = format!("launch-{instance_id}");
    state.operations.insert(
        operation_id.clone(),
        crate::app::state::InstallOperation {
            label: "Starting Minecraft".to_string(),
            phase: "Checking installation...".to_string(),
            instance_id: Some(instance_id.clone()),
            completed: 0,
            total: 0,
        },
    );
    let tx = state.tx.clone();
    let paths = state.paths.clone();
    let dm = state.dm.clone();
    let http = state.http.clone();
    let instances = state.instances.clone();
    let config = state.config.clone();
    let java_list = state.java_list.clone();
    let egui_ctx = state.egui_ctx.clone();
    state.runtime.spawn(async move {
        let _ = tx.send(AppEvent::PlayStarted(instance_id.clone()));
        egui_ctx.request_repaint();
        match play_inner(
            &dm,
            &http,
            &paths,
            &instances,
            &config,
            &java_list,
            &instance_id,
            &tx,
            &egui_ctx,
        )
        .await
        {
            Ok((code, log_file, launched_at)) => {
                crate::utils::system::show_window_for_current_process(true);
                let _ = tx.send(AppEvent::PlayFinished {
                    id: instance_id,
                    code,
                    log_file: Some(log_file),
                    launched_at: Some(launched_at),
                });
            }
            Err(e) => {
                crate::utils::system::show_window_for_current_process(true);
                let _ = tx.send(AppEvent::OperationFinished(operation_id, Err(e.clone())));
                let _ = tx.send(AppEvent::Error(e));
                let _ = tx.send(AppEvent::PlayFailed(instance_id));
            }
        }
        crate::utils::system::show_window_for_current_process(true);
        egui_ctx.request_repaint();
    });
}

#[allow(clippy::too_many_arguments)]
async fn play_inner(
    dm: &DownloadManager,
    http: &reqwest::Client,
    paths: &crate::storage::paths::MonoryxPaths,
    instances: &crate::instance::manager::InstanceManager,
    config: &crate::config::LauncherConfig,
    java_list: &[crate::java::runtime::JavaRuntime],
    instance_id: &str,
    tx: &std::sync::mpsc::Sender<AppEvent>,
    egui_ctx: &egui::Context,
) -> std::result::Result<(i32, std::path::PathBuf, std::time::SystemTime), String> {
    use crate::java::runtime::{required_major_for_version, select_runtime, JavaMode};
    let operation_id = format!("launch-{instance_id}");
    let phase = |label: &str| {
        let _ = tx.send(AppEvent::InstallProgress(
            operation_id.clone(),
            label.to_string(),
            0,
            0,
        ));
        let _ = tx.send(AppEvent::PlayLog(label.to_string()));
    };
    phase("Checking installation...");
    let mut cfg = instances.get(instance_id).map_err(|e| e.user_message())?;
    let profile = config
        .profile
        .clone()
        .ok_or_else(|| "Create an offline profile first.".to_string())?;
    instances
        .ensure_game_dirs(instance_id)
        .map_err(|e| e.user_message())?;
    let version_id = if cfg.resolved_version_id.is_empty() {
        let loader = crate::loaders::loader_for(cfg.loader);
        let resolved = loader
            .install(dm, paths, &cfg.minecraft_version, &cfg.loader_version)
            .await
            .map_err(|e| e.user_message())?;
        cfg.resolved_version_id = resolved.id.clone();
        instances.save(&cfg).map_err(|e| e.user_message())?;
        let report = crate::minecraft::installer::install_version(dm, paths, &resolved, None)
            .await
            .map_err(|e| e.user_message())?;
        report.version_id
    } else {
        cfg.resolved_version_id.clone()
    };
    let version_path = paths
        .versions_dir()
        .join(&version_id)
        .join(format!("{version_id}.json"));
    let version_text = std::fs::read_to_string(&version_path)
        .map_err(|e| format!("Version metadata missing, repair the instance. ({e})"))?;
    let version: crate::minecraft::version::VersionJson = serde_json::from_str(&version_text)
        .map_err(|e| format!("Corrupt version metadata: {e}"))?;
    let problems = crate::minecraft::installer::validate_install(paths, &version);
    if !problems.is_empty() {
        crate::minecraft::installer::install_version(dm, paths, &version, None)
            .await
            .map_err(|e| e.user_message())?;
    }
    phase("Finding a compatible Java runtime...");
    let required = required_major_for_version(
        &cfg.minecraft_version,
        version.java_version.as_ref().and_then(|j| j.major_version),
    );
    let mode = match cfg.java_mode {
        crate::instance::config::JavaMode::Automatic => {
            JavaMode::from_str_fallback(&config.java.mode, &config.java.custom_path)
        }
        crate::instance::config::JavaMode::System => JavaMode::SystemDefault,
        crate::instance::config::JavaMode::Custom => {
            if cfg.java_path.is_empty() {
                JavaMode::from_str_fallback(&config.java.mode, &config.java.custom_path)
            } else {
                JavaMode::Custom(std::path::PathBuf::from(&cfg.java_path))
            }
        }
    };
    let mut runtimes = java_list.to_vec();
    if runtimes.is_empty() {
        runtimes = crate::java::discovery::discover_all(&paths.java_dir()).await;
    }
    let mut selected = select_runtime(&runtimes, required, &mode);
    if selected.is_none() {
        if let Some(req) = required {
            if mode == JavaMode::Automatic {
                phase(&format!("Installing Java {req}..."));
                let exe = crate::java::managed::install_managed(dm, &paths.java_dir(), req, None)
                    .await
                    .map_err(|e| e.user_message())?;
                selected = Some(crate::java::runtime::JavaRuntime {
                    path: exe,
                    major: req,
                    version_string: format!("Temurin {req}"),
                    source: "managed".to_string(),
                });
            }
        }
    }
    let rt = selected.ok_or_else(|| {
        format!(
            "No compatible Java found (needs Java {}). Install it and retry.",
            required
                .map(|v| v.to_string())
                .unwrap_or_else(|| "8+".to_string())
        )
    })?;
    if let Some(req) = required {
        if rt.major < req {
            return Err(format!(
                "Found Java {} but Minecraft {} needs Java {req}.",
                rt.major, cfg.minecraft_version
            ));
        }
    }
    let java_exe = crate::java::discovery::prefer_javaw(&rt.path);
    let game_dir = instances.game_dir(instance_id);
    let ts = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
    let log_file = instances
        .instance_dir(instance_id)
        .join("game")
        .join("logs")
        .join(format!("monoryx-{ts}.log"));
    let ctx = crate::minecraft::launcher::LaunchContext {
        version: version.clone(),
        version_id: version_id.clone(),
        game_dir: game_dir.clone(),
        assets_dir: paths.assets_dir(),
        libraries_dir: paths.libraries_dir(),
        client_jar: paths.versions_dir().join(&version_id).join(format!(
            "{}.jar",
            version.jar.as_deref().unwrap_or(&version_id)
        )),
        natives_dir: paths.versions_dir().join(&version_id).join("natives"),
        java_exe,
        username: profile.username.clone(),
        uuid: profile.uuid,
        access_token: "0".to_string(),
        memory_min_mb: cfg.memory_min_mb,
        memory_max_mb: {
            let is_boost = cfg.boost_mode.unwrap_or(config.boost_mode);
            if is_boost && cfg.memory_max_mb == crate::utils::system::default_max_memory_mb() {
                crate::utils::system::default_boost_max_memory_mb()
            } else {
                cfg.memory_max_mb
            }
        },
        resolution: match (cfg.width, cfg.height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        },
        fullscreen: cfg.fullscreen,
        extra_jvm_args: if cfg.jvm_args.is_empty() {
            config.default_jvm_args.clone()
        } else {
            cfg.jvm_args.clone()
        },
        extra_game_args: if cfg.game_args.is_empty() {
            config.default_game_args.clone()
        } else {
            cfg.game_args.clone()
        },
        log_file: log_file.clone(),
        boost_mode: cfg.boost_mode.unwrap_or(config.boost_mode),
        skins_restorer_compat: config.skins_restorer_compat,
    };
    let plan = crate::minecraft::launcher::build_launch_plan(&ctx).map_err(|e| e.user_message())?;
    let _ = tx.send(AppEvent::PlayLog(format!(
        "Launching {} with {}",
        version_id,
        plan.java_exe.display()
    )));
    let _ = instances.mark_played(instance_id);
    if let Err(e) =
        crate::minecraft::launcher::apply_gpu_preference(&plan.java_exe, config.gpu_preference)
            .await
    {
        let _ = tx.send(AppEvent::PlayLog(format!(
            "GPU preference not applied: {}",
            e.user_message()
        )));
    }
    let start_time = std::time::SystemTime::now();
    let mut proc = crate::minecraft::launcher::SupervisedProcess::spawn(&plan, &log_file)
        .await
        .map_err(|e| e.user_message())?;
    let _ = tx.send(AppEvent::OperationFinished(operation_id.clone(), Ok(())));
    let _ = tx.send(AppEvent::PlaySpawned);
    if config.close_action == crate::config::CloseAction::Hide {
        crate::utils::system::show_window_for_current_process(false);
    }
    egui_ctx.request_repaint();
    let wait_res = proc.wait().await;
    crate::utils::system::show_window_for_current_process(true);
    let code = wait_res.map_err(|e| e.user_message())?;
    if code != 0 {
        let _ = tx.send(AppEvent::PlayLog(format!(
            "Minecraft exited with code {code}. Log: {}",
            log_file.display()
        )));
    }
    let _ = http;
    Ok((code, log_file, start_time))
}

pub fn install_mod(
    state: &AppState,
    project_id: String,
    slug: String,
    title: String,
    version_id: Option<String>,
) {
    let Some(cfg) = state.selected() else {
        state
            .tx
            .send(AppEvent::Error("Select an instance first.".to_string()))
            .ok();
        return;
    };
    let kind = match state.discover_tab {
        crate::modrinth::search::DiscoverTab::ResourcePacks => ContentKind::Resourcepack,
        crate::modrinth::search::DiscoverTab::Shaders => ContentKind::Shader,
        _ => ContentKind::Mod,
    };
    if kind == ContentKind::Mod && cfg.loader == LoaderKind::Vanilla {
        state
            .tx
            .send(AppEvent::Error(
                "Mods need a modded loader (Fabric, Quilt, Forge or NeoForge).".to_string(),
            ))
            .ok();
        return;
    }
    let dm = state.dm.clone();
    let mr = state.mr.clone();
    let instances = state.instances.clone();
    let tx = state.tx.clone();
    let loader_str = match cfg.loader {
        LoaderKind::Fabric => "fabric",
        LoaderKind::Quilt => "quilt",
        LoaderKind::Forge => "forge",
        LoaderKind::Neoforge => "neoforge",
        LoaderKind::Vanilla => "vanilla",
    }
    .to_string();
    state.runtime.spawn(async move {
        let req = InstallRequest {
            project_id,
            project_slug: slug,
            project_title: title,
            kind,
            minecraft_version: cfg.minecraft_version.clone(),
            loader: loader_str,
            version_id,
        };
        let r = install_project(&dm, &mr, &instances, &cfg.id, req, None)
            .await
            .map(|o| o.installed_files)
            .map_err(|e| e.user_message());
        let _ = tx.send(AppEvent::ModInstallDone(r));
    });
}

pub fn check_updates(state: &AppState) {
    let Some(cfg) = state.selected() else { return };
    let instance_id = cfg.id.clone();
    let mr = state.mr.clone();
    let instances = state.instances.clone();
    let tx = state.tx.clone();
    let loader_str = match cfg.loader {
        LoaderKind::Fabric => "fabric",
        LoaderKind::Quilt => "quilt",
        LoaderKind::Forge => "forge",
        LoaderKind::Neoforge => "neoforge",
        LoaderKind::Vanilla => "vanilla",
    }
    .to_string();
    state.runtime.spawn(async move {
        let r = crate::modrinth::updates::check_updates(
            &mr,
            &instances,
            &cfg.id,
            &cfg.minecraft_version,
            &loader_str,
        )
        .await
        .map_err(|error| error.user_message());
        let _ = tx.send(AppEvent::UpdatesFound(instance_id, r));
    });
}

pub fn update_one(state: &AppState, info: crate::modrinth::updates::UpdateInfo) {
    let Some(cfg) = state.selected() else { return };
    let dm = state.dm.clone();
    let mr = state.mr.clone();
    let instances = state.instances.clone();
    let tx = state.tx.clone();
    let loader_str = match cfg.loader {
        LoaderKind::Fabric => "fabric",
        LoaderKind::Quilt => "quilt",
        LoaderKind::Forge => "forge",
        LoaderKind::Neoforge => "neoforge",
        LoaderKind::Vanilla => "vanilla",
    }
    .to_string();
    state.runtime.spawn(async move {
        let r = crate::modrinth::updates::update_project(
            &dm,
            &mr,
            &instances,
            &cfg.id,
            &info,
            &cfg.minecraft_version,
            &loader_str,
        )
        .await
        .map(|f| vec![f])
        .map_err(|e| e.user_message());
        let _ = tx.send(AppEvent::ModInstallDone(r));
    });
}

pub fn update_all_mods(state: &AppState) {
    let infos = state.updates.clone();
    for info in infos {
        update_one(state, info);
    }
}

pub fn check_loader_update(state: &AppState, instance_id: String) {
    let instances = state.instances.clone();
    let http = state.http.clone();
    let tx = state.tx.clone();
    state.runtime.spawn(async move {
        let result = async {
            let cfg = instances.get(&instance_id).map_err(|e| e.user_message())?;
            if cfg.loader == LoaderKind::Vanilla {
                return Ok(None);
            }
            let latest = crate::loaders::loader_for(cfg.loader)
                .latest_stable(&http, &cfg.minecraft_version)
                .await
                .map_err(|e| e.user_message())?;
            Ok((latest != cfg.loader_version).then_some(latest))
        }
        .await;
        let _ = tx.send(AppEvent::LoaderUpdateChecked(instance_id, result));
    });
}

pub fn install_loader_update(state: &AppState, instance_id: String, version: String) {
    let instances = state.instances.clone();
    let dm = state.dm.clone();
    let paths = state.paths.clone();
    let tx = state.tx.clone();
    state.runtime.spawn(async move {
        let result = async {
            let mut cfg = instances.get(&instance_id).map_err(|e| e.user_message())?;
            if cfg.loader == LoaderKind::Vanilla {
                return Err("Vanilla has no separate loader to update".to_string());
            }
            let resolved = crate::loaders::loader_for(cfg.loader)
                .install(&dm, &paths, &cfg.minecraft_version, &version)
                .await
                .map_err(|e| e.user_message())?;
            crate::minecraft::installer::install_version(&dm, &paths, &resolved, None)
                .await
                .map_err(|e| e.user_message())?;
            cfg.loader_version = version.clone();
            cfg.resolved_version_id = resolved.id;
            instances.save(&cfg).map_err(|e| e.user_message())?;
            Ok(version)
        }
        .await;
        let _ = tx.send(AppEvent::LoaderUpdateDone(instance_id, result));
    });
}

pub fn repair_instance(state: &AppState, instance_id: String) {
    let dm = state.dm.clone();
    let paths = state.paths.clone();
    let instances = state.instances.clone();
    let http = state.http.clone();
    let tx = state.tx.clone();
    state.runtime.spawn(async move {
        let r: std::result::Result<Vec<String>, String> = async {
            let mut cfg = instances.get(&instance_id).map_err(|e| e.user_message())?;
            let loader = crate::loaders::loader_for(cfg.loader);
            let resolved = loader
                .install(&dm, &paths, &cfg.minecraft_version, &cfg.loader_version)
                .await
                .map_err(|e| e.user_message())?;
            cfg.resolved_version_id = resolved.id.clone();
            instances.save(&cfg).map_err(|e| e.user_message())?;
            let problems = crate::minecraft::installer::validate_install(&paths, &resolved);
            crate::minecraft::installer::install_version(&dm, &paths, &resolved, None)
                .await
                .map_err(|e| e.user_message())?;
            let _ = http;
            Ok(problems)
        }
        .await;
        let _ = tx.send(AppEvent::RepairDone(r));
    });
}

pub fn install_pack_file(state: &AppState, path: std::path::PathBuf) {
    let dm = state.dm.clone();
    let paths = state.paths.clone();
    let instances = state.instances.clone();
    let tx = state.tx.clone();
    state.runtime.spawn(async move {
        let r =
            crate::modrinth::modpack::install_mrpack(&dm, &paths, &instances, &path, None, None)
                .await
                .map(|rep| rep.instance_id)
                .map_err(|e| e.user_message());
        let _ = tx.send(AppEvent::PackDone(r));
    });
}

pub fn install_modpack(state: &AppState, slug: String, title: String) {
    let dm = state.dm.clone();
    let mr = state.mr.clone();
    let paths = state.paths.clone();
    let instances = state.instances.clone();
    let tx = state.tx.clone();
    let ctx = state.egui_ctx.clone();
    state.runtime.spawn(async move {
        let res: std::result::Result<String, String> = async {
            let versions = mr
                .project_versions(&slug, None, None)
                .await
                .map_err(|e| e.user_message())?;
            let latest = versions
                .into_iter()
                .next()
                .ok_or_else(|| "No versions found for this modpack.".to_string())?;
            let pack_file = latest
                .files
                .into_iter()
                .find(|f| f.filename.ends_with(".mrpack"))
                .ok_or_else(|| "No .mrpack file found in latest modpack release.".to_string())?;
            let temp_dir = paths.cache_dir().join("modpacks");
            let _ = tokio::fs::create_dir_all(&temp_dir).await;
            let dest = temp_dir.join(
                crate::utils::fs::safe_file_name(&pack_file.filename)
                    .map_err(|e| e.user_message())?,
            );
            let sha1 = pack_file.hashes.get("sha1").cloned();
            let mut job =
                DownloadJob::new(format!("Modpack: {title}"), pack_file.url, dest.clone())
                    .with_size(pack_file.size);
            if let Some(h) = sha1 {
                job = job.with_sha1(h);
            }
            dm.download(&job, None)
                .await
                .map_err(|e| e.user_message())?;
            let rep = crate::modrinth::modpack::install_mrpack(
                &dm,
                &paths,
                &instances,
                &dest,
                Some(title),
                None,
            )
            .await
            .map_err(|e| e.user_message())?;
            let _ = tokio::fs::remove_file(&dest).await;
            Ok(rep.instance_id)
        }
        .await;
        let _ = tx.send(AppEvent::PackDone(res));
        ctx.request_repaint();
    });
}

pub fn open_project_page(state: &AppState, slug: String) {
    let mr = state.mr.clone();
    let tx = state.tx.clone();
    state.runtime.spawn(async move {
        let p = mr.project(&slug).await.map_err(|e| e.user_message());
        let slug2 = slug.clone();
        let mr2 = mr.clone();
        let tx2 = tx.clone();
        let _ = tx.send(AppEvent::ProjectDetail(p));
        let tx3 = tx2;
        tokio::spawn(async move {
            let v = mr2
                .project_versions(&slug2, None, None)
                .await
                .map_err(|e| e.user_message());
            let _ = tx3.send(AppEvent::ProjectVersions(v));
        });
    });
}

pub fn download_with_tracking(state: &AppState, job: DownloadJob) {
    let dm = state.dm.clone();
    state.runtime.spawn(async move {
        let _ = dm.download(&job, None).await;
    });
}

pub fn check_launcher_update(state: &AppState) {
    let http = state.http.clone();
    let tx = state.tx.clone();
    let ctx = state.egui_ctx.clone();
    state.runtime.spawn(async move {
        let r = crate::app::updater::check_launcher_update(&http).await;
        let _ = tx.send(AppEvent::LauncherUpdate(r));
        ctx.request_repaint();
    });
}
