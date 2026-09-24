#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
pub mod account;
pub mod app;
pub mod config;
pub mod content;
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
    const S: i32 = 64;
    const T: i32 = 4;
    let mut rgba = vec![0u8; (S * S * 4) as usize];
    for y in 0..S {
        for x in 0..S {
            let i = ((y * S + x) * 4) as usize;
            rgba[i] = 0x09;
            rgba[i + 1] = 0x09;
            rgba[i + 2] = 0x09;
            rgba[i + 3] = 0xFF;
        }
    }
    let mut dot = |x: i32, y: i32| {
        if x >= 0 && y >= 0 && x < S && y < S {
            let i = ((y * S + x) * 4) as usize;
            rgba[i] = 0xF3;
            rgba[i + 1] = 0xF3;
            rgba[i + 2] = 0xF3;
            rgba[i + 3] = 0xFF;
        }
    };
    let mut bar = |x0: i32, y0: i32, x1: i32, y1: i32| {
        let steps = ((x1 - x0).abs().max((y1 - y0).abs()) * 2).max(1);
        for s in 0..=steps {
            let x = x0 + (x1 - x0) * s / steps;
            let y = y0 + (y1 - y0) * s / steps;
            for dy in -T..=T {
                for dx in -T..=T {
                    dot(x + dx, y + dy);
                }
            }
        }
    };
    bar(17, 14, 17, 50);
    bar(47, 14, 47, 50);
    bar(17, 14, 32, 38);
    bar(47, 14, 32, 38);
    egui::IconData {
        rgba,
        width: S as u32,
        height: S as u32,
    }
}

fn main() -> eframe::Result<()> {
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
    let mut viewport = egui::ViewportBuilder::default()
        .with_min_inner_size([850.0, 560.0])
        .with_maximized(startup_config.start_maximized)
        .with_title("MONORYX v1.1.0 Beta")
        .with_icon(monoryx_icon());
    if !startup_config.start_maximized {
        viewport = viewport.with_inner_size([
            startup_config.window_width.clamp(850.0, 2560.0),
            startup_config.window_height.clamp(560.0, 1440.0),
        ]);
    }
    let options = eframe::NativeOptions {
        viewport,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "MONORYX v1.1.0 Beta",
        options,
        Box::new(|cc| {
            crate::ui::theme::apply_theme(&cc.egui_ctx);
            let state = AppState::new(cc);

            #[cfg(target_os = "windows")]
            crate::utils::system::ensure_window_positioned(
                startup_config.start_maximized,
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
