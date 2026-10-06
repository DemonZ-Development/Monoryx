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
    state: &mut AppState,
    name: String,
    mc: String,
    loader: LoaderKind,
    loader_version: String,
) {
    match state.instances.create(name, mc, loader, loader_version) {
        Ok(cfg) => {
            state.refresh_instances();
            state.selected_instance = Some(cfg.id.clone());
            repair_instance(state, cfg.id);
            state.set_page(crate::app::events::Page::Downloads);
        }
        Err(error) => state.fail(error.user_message()),
    }
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
    state.sync_discord();
    let discord = state.discord.clone();
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
            &discord,
        )
        .await
        {
            Ok((code, log_file, launched_at)) => {
                crate::utils::system::show_window_for_current_process(true);
                let _ = tx.send(AppEvent::PlayFinished {
                    id: instance_id.clone(),
                    code,
                    log_file: Some(log_file),
                    launched_at: Some(launched_at),
                });
            }
            Err(e) => {
                crate::utils::system::show_window_for_current_process(true);
                let _ = tx.send(AppEvent::OperationFinished(operation_id, Err(e.clone())));
                let _ = tx.send(AppEvent::Error(e));
                let _ = tx.send(AppEvent::PlayFailed(instance_id.clone()));
            }
        }
        discord.game_finished(&instance_id);
        let _ = tx.send(AppEvent::AppCdsRecorded(instance_id.clone()));
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
    discord: &crate::discord::Presence,
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
    discord.game_started(
        instance_id,
        &cfg.name,
        &cfg.minecraft_version,
        cfg.loader.display_name(),
    );
    let mut account = config
        .active_account()
        .ok_or_else(|| "Configure an offline or Microsoft profile first.".to_string())?;

    if let crate::account::Account::Microsoft(ref ms) = account {
        let now = chrono::Utc::now().timestamp();
        if ms.expires_at <= now + 300 && !ms.refresh_token.is_empty() {
            phase("Refreshing Microsoft session...");
            match crate::account::microsoft::refresh_minecraft_token(
                dm.client(),
                &config.microsoft_client_id,
                &ms.refresh_token,
            )
            .await
            {
                Ok(new_profile) => {
                    let _ = tx.send(AppEvent::MicrosoftSessionRefreshed(new_profile.clone()));
                    account = crate::account::Account::Microsoft(new_profile);
                }
                Err(err) => {
                    return Err(format!(
                        "Microsoft session expired. Please sign in again. ({})",
                        err.user_message()
                    ));
                }
            }
        }
    }

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
    let problems = crate::minecraft::installer::validate_install(
        paths,
        &version,
        config.verify_level.as_verify(),
    );
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
    if let JavaMode::Custom(path) = &mode {
        if !runtimes.iter().any(|runtime| &runtime.path == path) {
            if let Some(probed) = crate::java::discovery::probe(path, "custom").await {
                runtimes.push(probed);
            }
        }
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
        if rt.major == 0 {
            return Err(format!(
                "Could not determine the Java version at {} (needs Java {req}). Pick another Java in this instance's settings.",
                rt.path.display()
            ));
        }
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
        username: account.username().to_string(),
        uuid: account.uuid(),
        access_token: account.access_token().to_string(),
        user_type: account.user_type().to_string(),
        memory_min_mb: cfg.memory_min_mb,
        memory_max_mb: crate::utils::system::game_memory_limit_mb(
            cfg.memory_max_mb,
            cfg.boost_mode.unwrap_or(config.boost_mode),
            crate::utils::system::default_max_memory_mb(),
            crate::utils::system::default_boost_max_memory_mb(),
        ),
        resolution: match (cfg.width, cfg.height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        },
        fullscreen: cfg.fullscreen,
        extra_jvm_args: if !cfg.jvm_args.is_empty() {
            cfg.jvm_args.clone()
        } else if config.jvm_preset != crate::config::JvmPreset::None {
            config.jvm_preset.flags().to_string()
        } else {
            config.default_jvm_args.clone()
        },
        appcds: {
            let archive = super::appcds::archive_path(instances, instance_id);
            if cfg.appcds_pending {
                crate::minecraft::launcher::AppCds::Record(archive)
            } else if config.appcds {
                crate::minecraft::launcher::AppCds::Use(archive)
            } else {
                crate::minecraft::launcher::AppCds::Off
            }
        },
        extra_game_args: if cfg.game_args.is_empty() {
            config.default_game_args.clone()
        } else {
            cfg.game_args.clone()
        },
        log_file: log_file.clone(),
        skins_restorer_compat: config.skins_restorer_compat,
    };
    let cp_entries = crate::minecraft::libraries::build_classpath(
        &version.libraries,
        &paths.libraries_dir(),
        &paths.versions_dir().join(&version_id).join(format!(
            "{}.jar",
            version.jar.as_deref().unwrap_or(&version_id)
        )),
        "https://libraries.minecraft.net",
    );
    let _ = tx.send(AppEvent::ClasspathResolved(
        instance_id.to_string(),
        cp_entries.clone(),
    ));

    let plan = crate::minecraft::launcher::build_launch_plan(&ctx).map_err(|e| e.user_message())?;
    if let Some(path) = match ctx.appcds {
        crate::minecraft::launcher::AppCds::Record(_) => None,
        _ => Some(crate::app::appcds::archive_path(instances, instance_id)),
    } {
        if path.is_file() {
            let rejected = std::fs::read_to_string(path.with_extension("cds.log"))
                .map(|t| {
                    t.contains("does not match")
                        || t.contains("wrong version")
                        || t.contains("Unable to use shared archive")
                        || t.contains("class paths mismatch")
                })
                .unwrap_or(false);
            if rejected {
                let _ = tx.send(AppEvent::Notice(
                    "Startup archive no longer matches this instance and was skipped. Use \
                     Re-record in Settings to rebuild it."
                        .to_string(),
                ));
            }
        }
        let _ = std::fs::remove_file(path.with_extension("cds.log"));
    }
    if let Some(note) = plan.appcds_note.clone() {
        let _ = tx.send(AppEvent::PlayLog(note.clone()));
        let _ = tx.send(AppEvent::Notice(note));
    }
    let _ = tx.send(AppEvent::PlayLog(format!(
        "Launching {} with {}",
        version_id,
        plan.java_exe.display()
    )));
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
    let _ = instances.mark_played(instance_id);
    let _ = tx.send(AppEvent::OperationFinished(operation_id.clone(), Ok(())));
    let _ = tx.send(AppEvent::PlaySpawned);
    if config.close_action == crate::config::CloseAction::Hide {
        crate::utils::system::show_window_for_current_process(false);
    }
    egui_ctx.request_repaint();
    let activity = proc.activity_handle();
    let mut game_window = crate::minecraft::game_window::GameWindow::new(proc.process_id());
    let wait = proc.wait();
    tokio::pin!(wait);
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(3));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut previous = None;
    let wait_res = loop {
        tokio::select! {
            result = &mut wait => break result,
            _ = ticker.tick() => {
                let current = activity.lock().map(|value| value.clone()).unwrap_or_default();
                let game = plan.cwd.clone();
                let observed_before = current.clone();
                let mut window = game_window.clone();
                let Ok((window, resolved)) = tokio::task::spawn_blocking(move || {
                    let resolved = window.update(&game, current);
                    (window, resolved)
                }).await else { continue; };
                game_window = window;
                if let Ok(mut observed) = activity.lock() {
                    if *observed == observed_before { *observed = resolved.clone(); }
                }
                if previous.as_ref() != Some(&resolved) {
                    discord.game_activity(instance_id, resolved.clone());
                    previous = Some(resolved.clone());
                    let _ = tx.send(AppEvent::GameActivity(instance_id.to_string(), resolved));
                    egui_ctx.request_repaint();
                }
            }
        }
    };
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
    state: &mut AppState,
    project_id: String,
    slug: String,
    title: String,
    version_id: Option<String>,
) {
    let kind = match state.discover_tab {
        crate::modrinth::search::DiscoverTab::ResourcePacks => ContentKind::Resourcepack,
        crate::modrinth::search::DiscoverTab::Shaders => ContentKind::Shader,
        _ => ContentKind::Mod,
    };
    install_content(state, project_id, slug, title, kind, version_id);
}

pub fn install_content(
    state: &mut AppState,
    project_id: String,
    slug: String,
    title: String,
    kind: ContentKind,
    version_id: Option<String>,
) {
    let Some(cfg) = state.selected() else {
        state
            .tx
            .send(AppEvent::Error("Select an instance first.".to_string()))
            .ok();
        return;
    };

    if let Some(reason) = state.install_blocker() {
        state.tx.send(AppEvent::Error(reason)).ok();
        return;
    }
    if kind == ContentKind::Mod && cfg.loader == LoaderKind::Vanilla {
        state
            .tx
            .send(AppEvent::Error(
                "Mods need a modded loader (Fabric, Quilt, Forge or NeoForge).".to_string(),
            ))
            .ok();
        return;
    }
    let op_id = format!("content:{}:{project_id}", cfg.id);
    let retry = crate::app::state::RetryAction::Content(
        cfg.id.clone(),
        project_id.clone(),
        slug.clone(),
        title.clone(),
        kind,
        version_id.clone(),
    );
    if !state.start_operation(
        &op_id,
        format!("Installing {title}"),
        Some(cfg.id.clone()),
        retry,
    ) {
        return;
    }
    state.begin_row_activity(&op_id, "Finding compatible files…");
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
    let cf_key = if state.config.curseforge.is_configured() {
        Some(state.config.curseforge.api_key.clone())
    } else {
        None
    };
    let cf_endpoint = if state.config.curseforge.is_configured() {
        Some(state.config.curseforge.custom_endpoint.clone())
    } else {
        None
    };
    let slots = state.install_slots.clone();
    state.runtime.spawn(async move {
        let Ok(_permit) = slots.acquire_owned().await else {
            return;
        };
        let _ = tx.send(AppEvent::InstallProgress(
            op_id.clone(),
            "Preparing files…".into(),
            0,
            0,
        ));
        let req = InstallRequest {
            project_id,
            project_slug: slug,
            project_title: title,
            kind,
            minecraft_version: cfg.minecraft_version.clone(),
            loader: loader_str,
            version_id,
            curseforge_api_key: cf_key,
            curseforge_endpoint: cf_endpoint,
        };
        let progress_tx = tx.clone();
        let progress_id = op_id.clone();
        let progress = Arc::new(move |phase, completed, total| {
            let _ = progress_tx.send(AppEvent::InstallProgress(
                progress_id.clone(),
                phase,
                completed,
                total,
            ));
        });
        let r = install_project(&dm, &mr, &instances, &cfg.id, req, Some(progress))
            .await
            .map(|outcome| {
                for warning in outcome.warnings {
                    tracing::warn!("Content installation: {warning}");
                }
                outcome.installed_files
            })
            .map_err(|e| e.user_message());
        let _ = tx.send(AppEvent::ContentInstallDone(op_id, cfg.id, r));
    });
}

pub fn check_updates(state: &mut AppState) {
    state.check_instance_updates(false);
}

pub fn scan_instance_updates(
    state: &AppState,
    cfg: crate::instance::InstanceConfig,
    generation: u64,
    target: crate::app::updates::UpdateTarget,
) -> tokio::task::AbortHandle {
    let mr = state.mr.clone();
    let instances = state.instances.clone();
    let tx = state.tx.clone();
    let http = state.http.clone();
    let slots = state.metadata_slots.clone();
    let ctx = state.egui_ctx.clone();
    let cf = crate::curseforge::CurseForgeClient::with_server(
        state.http.clone(),
        &state.config.curseforge.api_key,
        &state.config.curseforge.custom_endpoint,
    );
    let loader_str = match cfg.loader {
        LoaderKind::Fabric => "fabric",
        LoaderKind::Quilt => "quilt",
        LoaderKind::Forge => "forge",
        LoaderKind::Neoforge => "neoforge",
        LoaderKind::Vanilla => "vanilla",
    }
    .to_string();
    state
        .runtime
        .spawn(async move {
            let Ok(_permit) = slots.acquire_owned().await else {
                return;
            };
            let content = crate::modrinth::updates::check_updates(
                &mr,
                Some(&cf),
                &instances,
                &cfg.id,
                &cfg.minecraft_version,
                &loader_str,
            );
            let loader = compatible_loader_update(&http, &cfg);
            let (content, loader) = tokio::join!(content, loader);
            let report = crate::app::updates::InstanceUpdateReport {
                content: content.map_err(|error| error.user_message()),
                loader,
            };
            let _ = tx.send(AppEvent::InstanceUpdatesChecked(
                generation,
                target,
                Box::new(report),
            ));
            ctx.request_repaint();
        })
        .abort_handle()
}

pub fn update_one(state: &mut AppState, info: crate::modrinth::updates::UpdateInfo) {
    let Some(cfg) = state.selected() else { return };
    let op_id = format!("update:{}:{}", cfg.id, info.file_name);
    let retry = crate::app::state::RetryAction::Update(cfg.id.clone(), info.clone());
    if !state.start_operation(
        &op_id,
        format!("Updating {}", info.file_name),
        Some(cfg.id.clone()),
        retry,
    ) {
        return;
    }
    state.begin_row_activity(&info.file_name, "Downloading update…");
    let dm = state.dm.clone();
    let mr = state.mr.clone();
    let instances = state.instances.clone();
    let tx = state.tx.clone();
    let cf = crate::curseforge::CurseForgeClient::with_server(
        state.http.clone(),
        &state.config.curseforge.api_key,
        &state.config.curseforge.custom_endpoint,
    );
    let loader_str = match cfg.loader {
        LoaderKind::Fabric => "fabric",
        LoaderKind::Quilt => "quilt",
        LoaderKind::Forge => "forge",
        LoaderKind::Neoforge => "neoforge",
        LoaderKind::Vanilla => "vanilla",
    }
    .to_string();
    let slots = state.install_slots.clone();
    state.runtime.spawn(async move {
        let Ok(_permit) = slots.acquire_owned().await else {
            return;
        };
        let _ = tx.send(AppEvent::InstallProgress(
            op_id.clone(),
            "Preparing files…".into(),
            0,
            0,
        ));
        let r = crate::modrinth::updates::update_project(
            &dm,
            &mr,
            Some(&cf),
            &instances,
            &cfg.id,
            &info,
            &cfg.minecraft_version,
            &loader_str,
        )
        .await
        .map(|f| vec![f])
        .map_err(|e| e.user_message());
        let _ = tx.send(AppEvent::ContentInstallDone(op_id, cfg.id, r));
    });
}

pub fn update_all_mods(state: &mut AppState) {
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
            compatible_loader_update(&http, &cfg).await
        }
        .await;
        let _ = tx.send(AppEvent::LoaderUpdateChecked(instance_id, result));
    });
}

async fn compatible_loader_update(
    http: &reqwest::Client,
    cfg: &crate::instance::InstanceConfig,
) -> std::result::Result<Option<String>, String> {
    if cfg.loader == LoaderKind::Vanilla {
        return Ok(None);
    }
    let latest = crate::loaders::loader_for(cfg.loader)
        .latest_stable(http, &cfg.minecraft_version)
        .await
        .map_err(|error| error.user_message())?;
    Ok(crate::loaders::is_newer_version(&cfg.loader_version, &latest).then_some(latest))
}

pub fn install_loader_update(state: &mut AppState, instance_id: String, version: String) {
    let op_id = format!("loader:{instance_id}");
    if !state.start_operation(
        &op_id,
        format!("Updating loader to {version}"),
        Some(instance_id.clone()),
        crate::app::state::RetryAction::Loader(instance_id.clone(), version.clone()),
    ) {
        return;
    }
    state.loader_update_busy = Some(instance_id.clone());
    state.loader_update_error.clear();
    let instances = state.instances.clone();
    let dm = state.dm.clone();
    let paths = state.paths.clone();
    let tx = state.tx.clone();
    let slots = state.install_slots.clone();
    state.runtime.spawn(async move {
        let Ok(_permit) = slots.acquire_owned().await else {
            return;
        };
        let _ = tx.send(AppEvent::InstallProgress(
            op_id.clone(),
            "Preparing loader…".into(),
            0,
            0,
        ));
        let result = async {
            let mut cfg = instances.get(&instance_id).map_err(|e| e.user_message())?;
            if cfg.loader == LoaderKind::Vanilla {
                return Err("Vanilla has no separate loader to update".to_string());
            }
            let resolved = crate::loaders::loader_for(cfg.loader)
                .install(&dm, &paths, &cfg.minecraft_version, &version)
                .await
                .map_err(|e| e.user_message())?;
            let progress_tx = tx.clone();
            let progress_id = op_id.clone();
            let progress: crate::minecraft::installer::PhaseCallback =
                Arc::new(move |phase, completed, total| {
                    let _ = progress_tx.send(AppEvent::InstallProgress(
                        progress_id.clone(),
                        phase.label().into(),
                        completed,
                        total,
                    ));
                });
            crate::minecraft::installer::install_version(&dm, &paths, &resolved, Some(progress))
                .await
                .map_err(|e| e.user_message())?;
            cfg.loader_version = version.clone();
            cfg.resolved_version_id = resolved.id;
            instances.save(&cfg).map_err(|e| e.user_message())?;
            Ok(version)
        }
        .await;
        let _ = tx.send(AppEvent::OperationFinished(
            op_id,
            result.as_ref().map(|_| ()).map_err(Clone::clone),
        ));
        let _ = tx.send(AppEvent::LoaderUpdateDone(instance_id, result));
    });
}

pub fn repair_instance(state: &mut AppState, instance_id: String) {
    let op_id = format!("game:{instance_id}");
    let name = state
        .instances
        .get(&instance_id)
        .map_or_else(|_| instance_id.clone(), |cfg| cfg.name);
    if !state.start_operation(
        &op_id,
        format!("Preparing Minecraft for {name}"),
        Some(instance_id.clone()),
        crate::app::state::RetryAction::Game(instance_id.clone()),
    ) {
        return;
    }
    let dm = state.dm.clone();
    let paths = state.paths.clone();
    let instances = state.instances.clone();
    let http = state.http.clone();
    let tx = state.tx.clone();
    let ctx = state.egui_ctx.clone();
    let slots = state.install_slots.clone();
    state.runtime.spawn(async move {
        let Ok(_permit) = slots.acquire_owned().await else {
            return;
        };
        let _ = tx.send(AppEvent::InstallProgress(
            op_id.clone(),
            "Preparing files…".into(),
            0,
            0,
        ));
        let progress_tx = tx.clone();
        let progress_id = op_id.clone();
        let progress: crate::minecraft::installer::PhaseCallback =
            Arc::new(move |phase, completed, total| {
                let _ = progress_tx.send(AppEvent::InstallProgress(
                    progress_id.clone(),
                    phase.label().into(),
                    completed,
                    total,
                ));
            });
        let result =
            install_instance_inner(&dm, &http, &paths, &instances, &instance_id, Some(progress))
                .await;
        let _ = tx.send(AppEvent::OperationFinished(
            op_id,
            result.as_ref().map(|_| ()).map_err(Clone::clone),
        ));
        let _ = tx.send(AppEvent::InstallDone(instance_id, result));
        ctx.request_repaint();
    });
}

pub fn install_pack_file(state: &mut AppState, path: std::path::PathBuf) {
    let op_id = format!("pack-file:{}", path.display());
    if !state.start_operation(
        &op_id,
        "Installing modpack".into(),
        None,
        crate::app::state::RetryAction::PackFile(path.clone()),
    ) {
        return;
    }
    let dm = state.dm.clone();
    let paths = state.paths.clone();
    let instances = state.instances.clone();
    let tx = state.tx.clone();
    let slots = state.install_slots.clone();
    state.runtime.spawn(async move {
        let Ok(_permit) = slots.acquire_owned().await else {
            return;
        };
        let _ = tx.send(AppEvent::InstallProgress(
            op_id.clone(),
            "Preparing files…".into(),
            0,
            0,
        ));
        let r =
            crate::modrinth::modpack::install_mrpack(&dm, &paths, &instances, &path, None, None)
                .await
                .map(|rep| rep.instance_id)
                .map_err(|e| e.user_message());
        let _ = tx.send(AppEvent::PackInstallDone(op_id, r));
    });
}

pub fn install_modpack(state: &mut AppState, slug: String, title: String) {
    let op_id = format!("pack:{slug}");
    if !state.start_operation(
        &op_id,
        format!("Installing {title}"),
        None,
        crate::app::state::RetryAction::Pack(slug.clone(), title.clone()),
    ) {
        return;
    }
    let dm = state.dm.clone();
    let mr = state.mr.clone();
    let paths = state.paths.clone();
    let instances = state.instances.clone();
    let tx = state.tx.clone();
    let ctx = state.egui_ctx.clone();
    let slots = state.install_slots.clone();
    state.runtime.spawn(async move {
        let Ok(_permit) = slots.acquire_owned().await else {
            return;
        };
        let _ = tx.send(AppEvent::InstallProgress(
            op_id.clone(),
            "Preparing files…".into(),
            0,
            0,
        ));
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
            let temp_dir = paths.modpack_staging_dir();
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
        let _ = tx.send(AppEvent::PackInstallDone(op_id, res));
        ctx.request_repaint();
    });
}

pub fn open_project_page(state: &AppState, slug: String) {
    if crate::curseforge::is_curseforge_slug(&slug) {
        let Some(mod_id) = crate::curseforge::id_from_slug(&slug) else {
            let _ = state.tx.send(AppEvent::ProjectDetail(
                slug,
                Err("Invalid CurseForge project ID".to_string()),
            ));
            return;
        };
        let cf = crate::curseforge::CurseForgeClient::with_server(
            state.http.clone(),
            &state.config.curseforge.api_key,
            &state.config.curseforge.custom_endpoint,
        );
        let tx = state.tx.clone();
        let slots = state.metadata_slots.clone();
        let slug2 = slug.clone();
        state.runtime.spawn(async move {
            let Ok(permit) = slots.clone().acquire_owned().await else {
                return;
            };
            let p = match cf.project(mod_id).await {
                Ok(cf_mod) => {
                    let desc = cf.description(mod_id).await.ok();
                    Ok(crate::curseforge::to_project(&cf_mod, desc))
                }
                Err(e) => Err(e.user_message()),
            };
            drop(permit);
            let _ = tx.send(AppEvent::ProjectDetail(slug.clone(), p));
            let tx3 = tx;
            tokio::spawn(async move {
                let Ok(_permit) = slots.acquire_owned().await else {
                    return;
                };
                let v = cf
                    .files(mod_id, None, None)
                    .await
                    .map(|files| {
                        files
                            .iter()
                            .map(crate::curseforge::to_project_version)
                            .collect::<Vec<_>>()
                    })
                    .map_err(|e| e.user_message());
                let _ = tx3.send(AppEvent::ProjectVersions(slug2, v));
            });
        });
        return;
    }

    let mr = state.mr.clone();
    let tx = state.tx.clone();
    let slots = state.metadata_slots.clone();
    state.runtime.spawn(async move {
        let Ok(permit) = slots.clone().acquire_owned().await else {
            return;
        };
        let p = mr.project(&slug).await.map_err(|e| e.user_message());
        drop(permit);
        let slug2 = slug.clone();
        let mr2 = mr.clone();
        let tx2 = tx.clone();
        let _ = tx.send(AppEvent::ProjectDetail(slug.clone(), p));
        let tx3 = tx2;
        tokio::spawn(async move {
            let Ok(_permit) = slots.acquire_owned().await else {
                return;
            };
            let v = mr2
                .project_versions(&slug2, None, None)
                .await
                .map_err(|e| e.user_message());
            let _ = tx3.send(AppEvent::ProjectVersions(slug2, v));
        });
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

pub fn download_launcher_update(state: &AppState, info: crate::app::LauncherUpdateInfo) {
    let http = state.http.clone();
    let tx = state.tx.clone();
    let ctx = state.egui_ctx.clone();
    let root = state.paths.root().to_path_buf();
    state.runtime.spawn(async move {
        let result = async {
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (&http, &info, &root);
                Err("In-app installation is currently available on Windows only.".to_string())
            }
            #[cfg(target_os = "windows")]
            {
                use futures::StreamExt;
                use tokio::io::AsyncWriteExt;

                let asset = info
                    .download_asset
                    .ok_or("No update asset was published.")?;
                let url = info
                    .download_url
                    .ok_or("No update download URL was published.")?;
                let filename = crate::app::updater::validated_update_filename(
                    &asset.direct_url,
                    &info.latest_version,
                )?;
                if filename != asset.file_name {
                    return Err("Update filename does not match its release asset.".into());
                }
                let parsed_version = semver::Version::parse(&info.latest_version)
                    .map_err(|_| "Invalid update version.".to_string())?;
                if url != crate::app::updater::download_url("windows", &parsed_version) {
                    return Err("Update URL does not match the official download endpoint.".into());
                }
                let expected = match asset.sha256.as_deref() {
                    Some(hash) => crate::app::updater::parse_sha256(hash, "", true)
                        .ok_or("Invalid update SHA-256 checksum.")?,
                    None => {
                        crate::app::updater::fetch_expected_sha256(
                            &http,
                            &asset.direct_url,
                            &filename,
                        )
                        .await?
                    }
                };
                let dir = root.join("updates").join(parsed_version.to_string());
                tokio::fs::create_dir_all(&dir)
                    .await
                    .map_err(|e| format!("Could not prepare update folder: {e}"))?;
                let target = dir.join(&filename);
                let partial_guard = tempfile::NamedTempFile::new_in(&dir)
                    .map_err(|e| e.to_string())?
                    .into_temp_path();
                let partial = partial_guard.to_path_buf();
                let response = http
                    .get(&url)
                    .timeout(std::time::Duration::from_secs(600))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                if !response.status().is_success() {
                    return Err(format!(
                        "Update download failed: HTTP {}",
                        response.status()
                    ));
                }
                const MAX_SIZE: u64 = 512 * 1024 * 1024;
                let total = response.content_length();
                if total.is_some_and(|size| size > MAX_SIZE)
                    || asset.file_size.is_some_and(|size| size > MAX_SIZE)
                {
                    return Err("Update installer is unexpectedly large.".to_string());
                }
                if total
                    .zip(asset.file_size)
                    .is_some_and(|(received, expected)| received != expected)
                {
                    return Err("Update download size does not match the release metadata.".into());
                }
                let total = asset.file_size.or(total);
                let mut file = tokio::fs::File::create(&partial)
                    .await
                    .map_err(|e| format!("Could not save update installer: {e}"))?;
                let mut stream = response.bytes_stream();
                let mut received = 0_u64;
                let download_result: Result<(), String> = async {
                    while let Some(chunk) = stream.next().await {
                        let chunk =
                            chunk.map_err(|e| format!("Update download interrupted: {e}"))?;
                        received += chunk.len() as u64;
                        if received > MAX_SIZE {
                            return Err("Update installer is unexpectedly large.".to_string());
                        }
                        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
                        let progress = total
                            .filter(|size| *size > 0)
                            .map(|size| (received as f32 / size as f32).clamp(0.0, 1.0));
                        let _ = tx.send(AppEvent::LauncherUpdateDownloadProgress(progress));
                        ctx.request_repaint();
                    }
                    file.sync_all().await.map_err(|e| e.to_string())?;
                    if total.is_some_and(|size| size != received) {
                        return Err("Update download was incomplete.".to_string());
                    }
                    Ok(())
                }
                .await;
                drop(file);
                if let Err(error) = download_result {
                    let _ = tokio::fs::remove_file(&partial).await;
                    return Err(error);
                }
                use tokio::io::AsyncReadExt;
                let mut header_file = tokio::fs::File::open(&partial)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut header = [0_u8; 2];
                header_file
                    .read_exact(&mut header)
                    .await
                    .map_err(|e| e.to_string())?;
                if &header != b"MZ" {
                    let _ = tokio::fs::remove_file(&partial).await;
                    return Err("Downloaded update is not a Windows executable.".to_string());
                }
                drop(header_file);
                let hash_path = partial.clone();
                let actual = tokio::task::spawn_blocking(move || {
                    crate::utils::hash::sha256_file(&hash_path)
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| format!("Could not verify update checksum: {e}"))?;
                if !actual.eq_ignore_ascii_case(&expected) {
                    return Err(
                        "Downloaded update failed SHA-256 integrity verification.".to_string()
                    );
                }
                let persist_target = target.clone();
                tokio::task::spawn_blocking(move || {
                    partial_guard.persist(&persist_target).map_err(|e| e.error)
                })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| format!("Could not finalize update: {e}"))?;
                Ok(target)
            }
        }
        .await;
        let _ = tx.send(AppEvent::LauncherUpdateDownloaded(result));
        ctx.request_repaint();
    });
}

pub fn start_microsoft_login(
    state: &AppState,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let http = state.http.clone();
    let client_id = state.config.microsoft_client_id.clone();
    let tx = state.tx.clone();
    let ctx = state.egui_ctx.clone();

    state.runtime.spawn(async move {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let code_res = crate::account::microsoft::request_device_code(&http, &client_id)
            .await
            .map_err(|e| e.user_message());

        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        match code_res {
            Ok(code) => {
                let device_code = code.device_code.clone();
                let interval = code.interval.max(3);
                let expires_in = code.expires_in.min(900);
                let _ = tx.send(AppEvent::MicrosoftDeviceCode(Ok(code)));
                ctx.request_repaint();

                let start = std::time::Instant::now();
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }

                    if start.elapsed().as_secs() > expires_in {
                        let _ = tx.send(AppEvent::MicrosoftLoginDone(Err(
                            "Login session timed out. Please try again.".into(),
                        )));
                        ctx.request_repaint();
                        break;
                    }

                    match crate::account::microsoft::authenticate_with_device_code(
                        &http,
                        &client_id,
                        &device_code,
                    )
                    .await
                    {
                        Ok(Some(profile)) => {
                            if !cancel.load(std::sync::atomic::Ordering::Relaxed) {
                                let _ = tx.send(AppEvent::MicrosoftLoginDone(Ok(profile)));
                                ctx.request_repaint();
                            }
                            break;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            if !cancel.load(std::sync::atomic::Ordering::Relaxed) {
                                let _ =
                                    tx.send(AppEvent::MicrosoftLoginDone(Err(e.user_message())));
                                ctx.request_repaint();
                            }
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                if !cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = tx.send(AppEvent::MicrosoftDeviceCode(Err(err)));
                    ctx.request_repaint();
                }
            }
        }
    });
}
