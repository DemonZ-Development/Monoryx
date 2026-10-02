#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
pub mod account;
pub mod app;
pub mod config;
pub mod content;

#[allow(dead_code)]
pub mod curseforge;
pub mod discord;
pub mod downloads;
pub mod error;
pub mod instance;
pub mod java;
pub mod loaders;
pub mod minecraft;
pub mod modrinth;
pub mod nexeu;
pub mod storage;
pub mod ui;
pub mod utils;

use crate::app::state::AppState;

fn monoryx_icon() -> egui::IconData {
    let icon = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .expect("bundled MONORYX icon is valid")
        .into_rgba8();
    let (width, height) = icon.dimensions();
    egui::IconData {
        rgba: icon.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result<()> {
    let raw_args: Vec<String> = std::env::args().collect();
    if raw_args
        .iter()
        .any(|a| a == "--update-source" || a == "--source")
    {
        let mut source = None;
        let mut target = None;
        let mut wait_pid: Option<u32> = None;
        let mut relaunch = false;
        let mut iter = raw_args.into_iter().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--update-source" | "--source" => {
                    source = iter.next().map(std::path::PathBuf::from)
                }
                "--target-dest" | "--target" => target = iter.next().map(std::path::PathBuf::from),
                "--wait-pid" | "--pid" => wait_pid = iter.next().and_then(|v| v.parse().ok()),
                "--relaunch" => relaunch = true,
                _ => {}
            }
        }
        if let (Some(s), Some(t)) = (source, target) {
            if let Some(pid) = wait_pid {
                let start = std::time::Instant::now();
                while start.elapsed() < std::time::Duration::from_secs(25) {
                    #[cfg(target_os = "windows")]
                    let alive = {
                        unsafe {
                            let handle = windows_sys::Win32::System::Threading::OpenProcess(
                                0x00100000, 0, pid,
                            );
                            if handle.is_null() {
                                false
                            } else {
                                let wait =
                                    windows_sys::Win32::System::Threading::WaitForSingleObject(
                                        handle, 150,
                                    );
                                windows_sys::Win32::Foundation::CloseHandle(handle);
                                wait == 0x00000102
                            }
                        }
                    };
                    #[cfg(not(target_os = "windows"))]
                    let alive = std::path::Path::new(&format!("/proc/{pid}")).exists();
                    if !alive {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
            let old_backup = t.with_file_name(format!(
                "{}.old",
                t.file_name().unwrap_or_default().to_string_lossy()
            ));
            let _ = std::fs::remove_file(&old_backup);
            if t.exists() {
                let _ = std::fs::rename(&t, &old_backup);
            }
            if std::fs::copy(&s, &t).is_ok() {
                let _ = std::fs::remove_file(&old_backup);
                let _ = std::fs::remove_file(&s);
                if relaunch {
                    let _ = std::process::Command::new(&t).spawn();
                }
            }
            std::process::exit(0);
        }
    }

    init_logging();
    tracing::info!("MONORYX {} starting", env!("CARGO_PKG_VERSION"));

    let single_instance = match crate::utils::system::try_acquire_single_instance() {
        crate::utils::system::SingleInstanceStatus::Primary(l) => Some(l),
        crate::utils::system::SingleInstanceStatus::AlreadyRunning => {
            #[cfg(target_os = "windows")]
            crate::utils::system::restore_any_running_monoryx_window();
            tracing::info!("MONORYX is already running. Signal sent to restore primary window; exiting duplicate.");
            return Ok(());
        }
        crate::utils::system::SingleInstanceStatus::Standalone => {
            tracing::warn!("Could not acquire single-instance lock; running standalone.");
            None
        }
    };

    let startup_config = crate::config::LauncherConfig::load(
        &crate::storage::paths::MonoryxPaths::global().config_file(),
    )
    .unwrap_or_default();
    let app_title = format!("MONORYX v{}", env!("CARGO_PKG_VERSION"));

    let first_run = startup_config.is_first_run();
    let mut viewport = egui::ViewportBuilder::default()
        .with_min_inner_size([850.0, 560.0])
        .with_maximized(startup_config.start_maximized && !first_run)
        .with_title(app_title.clone())
        .with_icon(monoryx_icon());
    if !startup_config.start_maximized {
        let (width, height) = if first_run {
            (
                crate::ui::theme::metrics::ONBOARDING_WINDOW[0],
                crate::ui::theme::metrics::ONBOARDING_WINDOW[1],
            )
        } else {
            (
                startup_config.window_width.clamp(850.0, 2560.0),
                startup_config.window_height.clamp(560.0, 1440.0),
            )
        };
        viewport = viewport.with_inner_size([width, height]);
    }
    let options = eframe::NativeOptions {
        viewport,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        &app_title,
        options,
        Box::new(|cc| {
            crate::ui::theme::apply_theme(&cc.egui_ctx);
            let state = AppState::new(cc);

            #[cfg(target_os = "windows")]
            crate::utils::system::ensure_window_positioned(
                startup_config.start_maximized && !first_run,
                !startup_config.start_maximized,
            );

            if let Some(listener) = single_instance {
                let tx = state.tx.clone();
                let ctx = cc.egui_ctx.clone();
                std::thread::Builder::new()
                    .name("monoryx-single-instance".to_string())
                    .spawn(move || {
                        let _ = listener.set_nonblocking(false);
                        while let Ok((mut stream, _)) = listener.accept() {
                            use std::io::{Read, Write};
                            let mut buf = [0u8; 64];
                            if let Ok(n) = stream.read(&mut buf) {
                                if let Ok(msg) = std::str::from_utf8(&buf[..n]) {
                                    if msg.contains("RESTORE") {
                                        let _ = stream.write_all(b"MONORYX_ACK\n");
                                        let _ = stream.flush();
                                        crate::utils::system::show_window_for_current_process(true);
                                        let _ =
                                            tx.send(crate::app::events::AppEvent::RestoreWindow);
                                        ctx.request_repaint();
                                    }
                                }
                            }
                        }
                    })
                    .ok();
            }

            Ok(Box::new(MonoryxApp { state }))
        }),
    )
}

struct MonoryxApp {
    state: AppState,
}

impl eframe::App for MonoryxApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        crate::ui::shell::app_update(&mut self.state, ctx, frame);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.state.save_config();
    }
}

fn init_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    let data_root = crate::storage::paths::data_root();
    let logs_dir = data_root.join("logs");
    let file_layer = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("monoryx.log")
        .build(&logs_dir)
        .map(|appender| {
            let (file_writer, guard) = tracing_appender::non_blocking(appender);
            std::mem::forget(guard);
            fmt::layer().with_writer(file_writer).with_ansi(false)
        })
        .map_err(|error| eprintln!("MONORYX: file logging unavailable: {error}"))
        .ok();
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("monoryx=info,eframe=warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(file_layer)
        .try_init()
        .ok();
}
