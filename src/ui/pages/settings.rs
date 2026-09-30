use crate::app::state::AppState;
use crate::config::{CloseAction, GpuPreference, ThemeKind};
use crate::ui::components::{badge_accent, badge_boost, card_frame, field_label, page_header};
use crate::ui::theme::{DANGER, MUTED, OK, TEXT, TEXT2};
use egui::{CornerRadius, RichText, Stroke};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Launcher,
    Appearance,
    Minecraft,
    Runtime,
    Discord,
    About,
}

#[cfg(test)]
pub(crate) fn select_discord_for_preview(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("settings-tab"), SettingsTab::Discord));
}

pub fn show(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Settings", "Make MONORYX feel right for you.");

    let tab_id = egui::Id::new("settings-tab");
    let mut tab = ctx.data_mut(|data| {
        data.get_temp::<SettingsTab>(tab_id)
            .unwrap_or(SettingsTab::Launcher)
    });
    ui.horizontal_wrapped(|ui| {
        for (value, label) in [
            (SettingsTab::Launcher, "Launcher"),
            (SettingsTab::Appearance, "Appearance"),
            (SettingsTab::Minecraft, "Minecraft"),
            (SettingsTab::Runtime, "Java & GPU"),
            (SettingsTab::Discord, "Discord"),
            (SettingsTab::About, "About"),
        ] {
            ui.selectable_value(&mut tab, value, label);
        }
    });
    ctx.data_mut(|data| data.insert_temp(tab_id, tab));
    ui.add_space(12.0);

    if tab == SettingsTab::Discord {
        discord_settings(state, ui);
    }

    if tab == SettingsTab::Launcher {
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Launcher Updates")
                        .size(16.0)
                        .strong()
                        .color(TEXT),
                );
                badge_accent(ui, &format!("v{}", env!("CARGO_PKG_VERSION")));
            });

            ui.label(
                RichText::new("Check for a newer version of MONORYX.")
                    .size(12.0)
                    .color(TEXT2),
            );

            ui.add_space(4.0);
            if ui
                .checkbox(
                    &mut state.config.auto_check_updates,
                    "Automatically check for launcher updates on startup",
                )
                .changed()
            {
                save_settings(state, "Update preference saved".to_string());
            }

            ui.add_space(8.0);
            let is_checking = state.launcher_update_loading;
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!is_checking, |ui| {
                    if crate::ui::components::primary_button(
                        ui,
                        if is_checking {
                            "Checking…"
                        } else {
                            "Check for updates"
                        },
                    )
                    .clicked()
                    {
                        state.check_launcher_update();
                    }
                });
                if is_checking {
                    ui.spinner();
                    ui.label(
                        RichText::new("Checking for updates...")
                            .size(12.5)
                            .color(TEXT2),
                    );
                }
            });

            if let Some(err) = &state.launcher_update_error {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("Update check error: {err}"))
                        .size(12.5)
                        .color(DANGER),
                );
            }

            if let Some(update) = state.launcher_update.clone() {
                ui.add_space(8.0);
                if update.has_update {
                    egui::Frame::new()
                        .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                        .stroke(Stroke::new(
                            1.0_f32,
                            crate::ui::theme::palette(ui.ctx()).border,
                        ))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "New Version v{} Available!",
                                        update.latest_version
                                    ))
                                    .strong()
                                    .size(14.0)
                                    .color(crate::ui::theme::INFO),
                                );
                                if let Some(pub_date) = &update.published_at {
                                    ui.label(RichText::new(pub_date).size(11.0).color(MUTED));
                                }
                            });

                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Release Notes:")
                                    .size(11.5)
                                    .strong()
                                    .color(TEXT),
                            );
                            egui::ScrollArea::vertical()
                                .max_height(120.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&update.release_notes)
                                            .size(11.5)
                                            .color(TEXT2),
                                    );
                                });

                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                #[cfg(target_os = "windows")]
                                if let Some(path) = state.launcher_update_downloaded.clone() {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Run installer").strong().color(
                                                    crate::ui::theme::palette(ui.ctx()).accent_text,
                                                ),
                                            )
                                            .fill(crate::ui::theme::palette(ui.ctx()).accent),
                                        )
                                        .clicked()
                                    {
                                        match std::process::Command::new(path).spawn() {
                                            Ok(_) => state.notify("Installer launched"),
                                            Err(error) => state.fail(format!(
                                                "Could not launch installer: {error}"
                                            )),
                                        }
                                    }
                                } else if let Some(_dl) = &update.download_url {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(
                                                    if state.launcher_update_download_loading {
                                                        "Downloading..."
                                                    } else {
                                                        "Download Update"
                                                    },
                                                )
                                                .strong()
                                                .color(
                                                    crate::ui::theme::palette(ui.ctx()).accent_text,
                                                ),
                                            )
                                            .fill(crate::ui::theme::palette(ui.ctx()).accent),
                                        )
                                        .clicked()
                                        && !state.launcher_update_download_loading
                                    {
                                        state.download_launcher_update();
                                    }
                                }
                                #[cfg(not(target_os = "windows"))]
                                if let Some(_dl) = &update.download_url {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Download Update").strong().color(
                                                    crate::ui::theme::palette(ui.ctx()).accent_text,
                                                ),
                                            )
                                            .fill(crate::ui::theme::palette(ui.ctx()).accent),
                                        )
                                        .clicked()
                                    {
                                        let _ = open::that(_dl);
                                    }
                                }
                                if ui.button("View on GitHub").clicked() {
                                    let _ = open::that(&update.html_url);
                                }
                            });
                            if state.launcher_update_download_loading {
                                ui.add_space(6.0);
                                crate::ui::components::thin_progress(
                                    ui,
                                    state.launcher_update_download_progress,
                                );
                            }
                            if let Some(error) = &state.launcher_update_download_error {
                                ui.colored_label(DANGER, error);
                            }
                        });
                } else if !state.launcher_update_loading && state.launcher_update_error.is_none() {
                    ui.label(
                        RichText::new("You are running the latest version of MONORYX.").color(OK),
                    );
                }
            }
        });

        ui.add_space(8.0);

        card_frame(ui, |ui| {
            ui.label(
                RichText::new("Launch Behavior & Window")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new(
                    "Control launcher window visibility and behavior when Minecraft runs.",
                )
                .size(12.0)
                .color(TEXT2),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("When game starts:").color(TEXT2));
                egui::ComboBox::from_id_salt("close-action")
                    .selected_text(state.config.close_action.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.config.close_action,
                            CloseAction::Hide,
                            "Close / Hide (restore after game exits) (Recommended)",
                        );
                        ui.selectable_value(
                            &mut state.config.close_action,
                            CloseAction::Minimize,
                            "Minimize",
                        );
                        ui.selectable_value(
                            &mut state.config.close_action,
                            CloseAction::Never,
                            "Keep launcher open",
                        );
                    });
            });
            ui.label(
                RichText::new("Hide restores the launcher when Minecraft exits.")
                    .size(11.5)
                    .color(MUTED),
            );

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut state.config.start_maximized, "Start maximized")
                    .changed()
                {
                    if state.config.start_maximized {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                        #[cfg(target_os = "windows")]
                        crate::utils::system::ensure_window_positioned(true, false);
                    }
                    save_settings(state, "Startup window preference saved".to_string());
                }
                let max_btn =
                    egui::Button::new(RichText::new("Maximize now").size(12.0).color(TEXT))
                        .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                        .stroke(Stroke::new(
                            1.0_f32,
                            crate::ui::theme::palette(ui.ctx()).border,
                        ))
                        .corner_radius(CornerRadius::same(6));
                if ui.add(max_btn).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                    #[cfg(target_os = "windows")]
                    crate::utils::system::ensure_window_positioned(true, false);
                }
            });
            if !state.config.start_maximized {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Windowed size:").color(TEXT2));
                    ui.add(
                        egui::DragValue::new(&mut state.config.window_width)
                            .range(850.0..=2560.0)
                            .speed(10.0)
                            .suffix(" px wide"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut state.config.window_height)
                            .range(560.0..=1440.0)
                            .speed(10.0)
                            .suffix(" px high"),
                    );
                    let apply_btn =
                        egui::Button::new(RichText::new("Apply size").size(12.0).color(TEXT))
                            .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                            .stroke(Stroke::new(
                                1.0_f32,
                                crate::ui::theme::palette(ui.ctx()).border,
                            ))
                            .corner_radius(CornerRadius::same(6));
                    if ui.add(apply_btn).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                            state.config.window_width.clamp(850.0, 2560.0),
                            state.config.window_height.clamp(560.0, 1440.0),
                        )));
                        #[cfg(target_os = "windows")]
                        crate::utils::system::ensure_window_positioned(false, true);
                        save_settings(state, "Window size applied".to_string());
                    }
                    let center_btn =
                        egui::Button::new(RichText::new("Center window").size(12.0).color(TEXT))
                            .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                            .stroke(Stroke::new(
                                1.0_f32,
                                crate::ui::theme::palette(ui.ctx()).border,
                            ))
                            .corner_radius(CornerRadius::same(6));
                    if ui.add(center_btn).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                        #[cfg(target_os = "windows")]
                        crate::utils::system::ensure_window_positioned(false, true);
                    }
                });
            }
            ui.add_space(4.0);
            ui.checkbox(
                &mut state.config.remember_instance,
                "Remember last selected instance",
            );
            ui.checkbox(
                &mut state.show_snapshots,
                "Show Minecraft snapshots and experimental releases",
            );

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Parallel download threads:").color(TEXT2));
                ui.add(
                    egui::DragValue::new(&mut state.config.parallel_downloads)
                        .range(1..=16)
                        .speed(1),
                );
            });

            ui.add_space(8.0);
            let save_btn = egui::Button::new(
                RichText::new("Save Window & Launch Settings")
                    .size(12.5)
                    .strong()
                    .color(crate::ui::theme::palette(ui.ctx()).accent_text),
            )
            .fill(crate::ui::theme::palette(ui.ctx()).accent)
            .corner_radius(CornerRadius::same(6));
            if ui.add(save_btn).clicked() {
                save_settings(state, "Launch settings saved".to_string());
            }
        });

        ui.add_space(8.0);

        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Launcher Performance & Memory")
                        .size(16.0)
                        .strong()
                        .color(TEXT),
                );
                badge_accent(ui, "Low Footprint");
            });

            ui.label(
                RichText::new(
                    "MONORYX is built in native Rust without Electron or Chromium overhead, keeping idle memory light.",
                )
                .size(12.0)
                .color(TEXT2),
            );

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Cached thumbnail textures: {}",
                        state.thumbnails.len()
                    ))
                    .color(TEXT),
                );
                if ui.button("Purge Image Cache (Free RAM)").clicked() {
                    let count = state.thumbnails.len();
                    state.thumbnails.clear();
                    state.notify(format!("Purged {count} cached textures; memory freed"));
                }
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Tip: Setting 'When game starts' to 'Close / Hide' fully suspends window rendering and releases GPU resources while Minecraft runs.",
                )
                .size(11.5)
                .color(MUTED),
            );
        });
    }

    ui.add_space(8.0);

    if tab == SettingsTab::Appearance {
        let previous = state.config.theme;
        card_frame(ui, |ui| {
            ui.label(RichText::new("Theme").size(16.0).strong().color(TEXT));
            ui.label(RichText::new("Choose a launcher color palette.").color(TEXT2));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for theme in ThemeKind::all() {
                    let p = crate::ui::theme::palette_for(theme);
                    let selected = state.config.theme == theme;
                    let button =
                        egui::Button::new(RichText::new(theme.label()).strong().color(p.accent))
                            .fill(p.elevated)
                            .stroke(Stroke::new(
                                if selected { 2.0_f32 } else { 1.0_f32 },
                                p.accent,
                            ))
                            .min_size(egui::vec2(120.0, 46.0));
                    if ui.add(button).clicked() {
                        state.config.theme = theme;
                    }
                }
            });
        });
        if state.config.theme != previous {
            crate::ui::theme::apply_selected_theme(ctx, state.config.theme);
            save_settings(
                state,
                format!("{} theme selected", state.config.theme.label()),
            );
        }
    }

    if tab == SettingsTab::Minecraft {
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Memory and Eco mode")
                        .size(16.0)
                        .strong()
                        .color(TEXT),
                );
                let boost_on = state.config.boost_mode;
                if boost_on {
                    badge_boost(ui, "ACTIVE");
                }
            });

            ui.label(
            RichText::new("Eco mode uses a lower game memory limit. It may lower FPS in large worlds or modded games. Turn it off when you want the smoothest play. This does not affect the launcher's RAM use.")
                .size(12.0)
                .color(TEXT2),
        );

            ui.add_space(6.0);
            if ui
                .checkbox(&mut state.config.boost_mode, "Use Eco mode by default")
                .changed()
            {
                save_settings(
                    state,
                    if state.config.boost_mode {
                        "Eco Mode enabled by default".to_string()
                    } else {
                        "Eco Mode disabled by default".to_string()
                    },
                );
            }

            ui.add_space(8.0);
            ui.label(
                RichText::new("Quick Memory Presets:")
                    .size(12.0)
                    .strong()
                    .color(TEXT),
            );
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button("2 GB (Low RAM / Vanilla)")
                    .on_hover_text(
                        "Recommended: Minimal RAM consumption for vanilla and light play",
                    )
                    .clicked()
                {
                    state.settings_mem_min = "512".to_string();
                    state.settings_mem_max = "2048".to_string();
                    state.config.memory.min_mb = 512;
                    state.config.memory.max_mb = 2048;
                    save_settings(state, "Low RAM preset applied (2048 MB)".to_string());
                }
                if ui
                    .button("3 GB (Balanced / Light Mods)")
                    .on_hover_text("Recommended: Great balance for modded 1.20+ with Fabric")
                    .clicked()
                {
                    state.settings_mem_min = "512".to_string();
                    state.settings_mem_max = "3072".to_string();
                    state.config.memory.min_mb = 512;
                    state.config.memory.max_mb = 3072;
                    save_settings(state, "Balanced preset applied (3072 MB)".to_string());
                }
                if ui
                    .button("4 GB (Modpacks)")
                    .on_hover_text("For modpacks with 80-150 mods")
                    .clicked()
                {
                    state.settings_mem_min = "1024".to_string();
                    state.settings_mem_max = "4096".to_string();
                    state.config.memory.min_mb = 1024;
                    state.config.memory.max_mb = 4096;
                    save_settings(state, "Modpack preset applied (4096 MB)".to_string());
                }
                if ui
                    .button("6 GB (Heavy Modpacks)")
                    .on_hover_text("For large 200+ modpacks")
                    .clicked()
                {
                    state.settings_mem_min = "1024".to_string();
                    state.settings_mem_max = "6144".to_string();
                    state.config.memory.min_mb = 1024;
                    state.config.memory.max_mb = 6144;
                    save_settings(state, "Heavy modpack preset applied (6144 MB)".to_string());
                }
                if ui
                    .button("8 GB (Extreme Shaders)")
                    .on_hover_text("For heavy modpacks with high-res shaders")
                    .clicked()
                {
                    state.settings_mem_min = "2048".to_string();
                    state.settings_mem_max = "8192".to_string();
                    state.config.memory.min_mb = 2048;
                    state.config.memory.max_mb = 8192;
                    save_settings(state, "Extreme preset applied (8192 MB)".to_string());
                }
            });
        });
    }

    ui.add_space(8.0);

    if tab == SettingsTab::Runtime {
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Java Runtime & GPU")
                        .size(16.0)
                        .strong()
                        .color(TEXT),
                );
                if ui.button("Refresh Java").clicked() {
                    state.refresh_java();
                }
            });
            if state.java_loading {
                ui.label("Detecting Java...");
            }
            if state.java_list.is_empty() && !state.java_loading {
                ui.label(
                    RichText::new(
                        "No Java detected. Automatic mode downloads a compatible managed runtime when you launch a game.",
                    )
                    .color(TEXT2),
                );
            }
            ui.collapsing(
                format!("Detected Java runtimes ({})", state.java_list.len()),
                |ui| {
                    for j in &state.java_list {
                        ui.label(
                            RichText::new(format!("Java {} · {}", j.major, j.path.display()))
                                .size(12.0)
                                .color(TEXT),
                        );
                    }
                },
            );
            ui.add_space(4.0);
            field_label(ui, "Mode");
            egui::ComboBox::from_id_salt("java-mode")
                .selected_text(match state.config.java.mode.as_str() {
                    "system" => "System",
                    "custom" => "Custom",
                    _ => "Automatic",
                })
                .show_ui(ui, |ui| {
                    for (value, label) in [
                        ("automatic", "Automatic"),
                        ("system", "System"),
                        ("custom", "Custom"),
                    ] {
                        ui.selectable_value(&mut state.config.java.mode, value.to_string(), label);
                    }
                });
            ui.label(
                RichText::new(
                    "Automatic selects a compatible Java runtime and downloads a managed Temurin runtime if needed.",
                )
                .color(MUTED),
            );
            if state.config.java.mode == "custom" {
                field_label(ui, "Custom Java path");
                crate::ui::components::limited_text_edit(
                    ui,
                    "settings-java-path",
                    &mut state.config.java.custom_path,
                    crate::ui::components::limits::PATH,
                    "C:\\Program Files\\Java\\bin\\javaw.exe",
                );
                if ui.button("Browse").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_file() {
                        state.config.java.custom_path = p.display().to_string();
                    }
                }
            }
            ui.add_space(4.0);
            gpu_preference_selector(ui, &mut state.config.gpu_preference);
            gpu_status(ui, ctx, state);
            if ui.button("Save Java & GPU settings").clicked() {
                save_settings(state, "Java & GPU settings saved".to_string());
            }
        });
    }

    ui.add_space(8.0);

    if tab == SettingsTab::Minecraft {
        card_frame(ui, |ui| {
            ui.label(
                RichText::new("Minecraft Defaults")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );
            field_label(ui, "Default min memory (MB)");
            crate::ui::components::limited_text_edit(
                ui,
                "settings-mem-min",
                &mut state.settings_mem_min,
                7,
                "512",
            );
            field_label(ui, "Default max memory (MB)");
            crate::ui::components::limited_text_edit(
                ui,
                "settings-mem-max",
                &mut state.settings_mem_max,
                7,
                "3072",
            );
            field_label(ui, "Backup compression");
            for option in [
                crate::config::BackupCompression::Fast,
                crate::config::BackupCompression::Maximum,
                crate::config::BackupCompression::Zstd,
            ] {
                ui.selectable_value(&mut state.config.backup_compression, option, option.label());
            }
            ui.label(
                RichText::new(state.config.backup_compression.hint())
                    .size(11.0)
                    .color(MUTED),
            );
            ui.label(
                RichText::new(
                    "A world's region files are already compressed by Minecraft, so raising \
                     this helps most on a save with lots of loose files, datapacks or \
                     resource packs in it.",
                )
                .size(11.0)
                .color(MUTED),
            );
            ui.collapsing("Advanced launch arguments", |ui| {
                field_label(ui, "Default JVM arguments");
                crate::ui::components::limited_text_edit_with_hint(
                    ui,
                    "settings-jvm-args",
                    &mut state.settings_jvm,
                    crate::ui::components::limits::JVM_ARGS,
                    "-Xmx4G",
                    "Applied to new instances. Existing ones keep their own value.",
                );
                field_label(ui, "Default game arguments");
                crate::ui::components::limited_text_edit_with_hint(
                    ui,
                    "settings-game-args",
                    &mut state.settings_game_args,
                    crate::ui::components::limits::GAME_ARGS,
                    "--username Steve",
                    "Appended to the game command line.",
                );
            });
            if ui.button("Save Minecraft defaults").clicked() {
                match (
                    state.settings_mem_min.trim().parse::<u64>(),
                    state.settings_mem_max.trim().parse::<u64>(),
                ) {
                    (Ok(min), Ok(max)) => match crate::utils::system::validate_memory(min, max) {
                        Ok(warn) => {
                            state.config.memory.min_mb = min;
                            state.config.memory.max_mb = max;
                            state.config.default_jvm_args = state.settings_jvm.clone();
                            state.config.default_game_args = state.settings_game_args.clone();
                            save_settings(
                                state,
                                warn.unwrap_or_else(|| "Minecraft defaults saved".to_string()),
                            );
                        }
                        Err(e) => state.fail(e),
                    },
                    _ => state.fail("Memory values must be numbers."),
                }
            }
        });
    }

    ui.add_space(8.0);

    if tab == SettingsTab::About {
        card_frame(ui, |ui| {
            ui.label(
                RichText::new("About MONORYX")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new(format!(
                    "Version {} - High-Performance Native Launcher",
                    env!("CARGO_PKG_VERSION")
                ))
                .color(TEXT),
            );
            ui.label(
                RichText::new("Minecraft, without the clutter. Apache-2.0. By DemonZDevelopment.")
                    .size(12.0)
                    .color(TEXT2),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Open Data Folder").clicked() {
                    let _ = open::that(state.paths.root());
                }
                if ui.button("Open Logs Folder").clicked() {
                    let _ = open::that(state.paths.logs_dir());
                }
            });
            ui.add_space(8.0);
            ui.label(
                RichText::new("Where your data lives")
                    .size(13.5)
                    .strong()
                    .color(TEXT),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(state.paths.root().display().to_string())
                        .monospace()
                        .size(11.5)
                        .color(TEXT2),
                )
                .wrap(),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Everything lives here: your instances, mods, worlds, world backups, \
                         screenshots, settings, plus the Minecraft files, Java runtimes and \
                         assets MONORYX downloaded. Uninstalling asks whether to delete this \
                         folder, and you can always say no and delete it yourself here instead.",
                    )
                    .size(11.0)
                    .color(MUTED),
                )
                .wrap(),
            );
        });
    }
}

pub(super) fn gpu_preference_selector(ui: &mut egui::Ui, preference: &mut GpuPreference) {
    field_label(ui, "GPU preference");
    ui.add_enabled_ui(cfg!(target_os = "windows"), |ui| {
        egui::ComboBox::from_id_salt("gpu-preference")
            .selected_text(preference.as_str())
            .show_ui(ui, |ui| {
                for value in [
                    GpuPreference::System,
                    GpuPreference::HighPerformance,
                    GpuPreference::PowerSaving,
                ] {
                    ui.selectable_value(preference, value, value.as_str());
                }
            });
    });
    ui.label(
        RichText::new(
            "Windows only. Applies on launch to the selected Java executable. High Performance targets dedicated GPUs.",
        )
        .size(11.5)
        .color(MUTED),
    );
}

fn gpu_status(ui: &mut egui::Ui, ctx: &egui::Context, state: &mut AppState) {
    if !cfg!(target_os = "windows") {
        return;
    }
    let auto_id = egui::Id::new("gpu-autodetect-once");
    if !ctx.data_mut(|data| data.get_temp::<bool>(auto_id).unwrap_or(false)) {
        ctx.data_mut(|data| data.insert_temp(auto_id, true));
        state.refresh_gpus();
    }
    ui.horizontal(|ui| {
        field_label(ui, "Detected GPUs");
        if ui.button("Refresh").clicked() {
            state.refresh_gpus_force();
        }
    });
    if state.gpu_loading {
        ui.label(RichText::new("Detecting GPUs...").color(TEXT2));
    } else if state.gpu_list.is_empty() {
        ui.label(RichText::new("No GPUs detected yet.").color(TEXT2));
    }
    for g in state.gpu_list.clone() {
        let tag = if g.dedicated {
            "Dedicated"
        } else {
            "Integrated"
        };
        ui.label(
            RichText::new(format!("{} ({tag})", g.name))
                .size(12.0)
                .color(TEXT),
        );
    }
    if state.gpu_list.iter().any(|g| g.dedicated)
        && state.config.gpu_preference == GpuPreference::System
    {
        ui.add_space(2.0);
        ui.label(
            RichText::new(
                "Dedicated GPU found. High performance asks Windows to run Minecraft on it.",
            )
            .size(11.5)
            .color(TEXT2),
        );
        if ui.button("Use High performance").clicked() {
            state.config.gpu_preference = GpuPreference::HighPerformance;
            save_settings(state, "GPU preference set to High performance".to_string());
        }
    }
}

fn save_settings(state: &mut AppState, msg: String) {
    state.config.last_page = state.page.as_str().to_string();
    state.config.selected_instance = state.selected_instance.clone();
    state.config.show_snapshots = state.show_snapshots;
    match state.config.save(&state.paths.config_file()) {
        Ok(()) => state.notify(msg),
        Err(e) => state.fail(e.user_message()),
    }
}

fn discord_settings(state: &mut AppState, ui: &mut egui::Ui) {
    use crate::ui::components::{primary_button, secondary_button};
    let mut changed = false;
    card_frame(ui, |ui| {
        ui.label(
            RichText::new("Share what you're playing")
                .size(20.0)
                .strong()
                .color(TEXT),
        );
        ui.label(RichText::new("Show your Minecraft activity on your Discord profile while the desktop app is open.").color(TEXT2));
        changed |= ui
            .checkbox(
                &mut state.config.discord.enabled,
                "Enable Discord Rich Presence",
            )
            .changed();
        ui.label(
            RichText::new(state.discord.status())
                .size(12.0)
                .color(crate::ui::theme::palette(ui.ctx()).accent),
        );
        ui.add_space(8.0);
        ui.label(RichText::new("What others can see").strong().color(TEXT));
        changed |= ui
            .checkbox(
                &mut state.config.discord.show_launcher,
                "Show activity while browsing the launcher",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut state.config.discord.show_instance,
                "Show the instance name",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut state.config.discord.show_world,
                "Show the singleplayer world name when available",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut state.config.discord.show_server,
                "Show the server name (or address if it isn't saved)",
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut state.config.discord.show_elapsed,
                "Show time spent playing",
            )
            .changed();
        ui.label(RichText::new("World names are detected from active saves on Windows. Server names come from your saved server list. If a game version doesn't expose a detail, the activity stays general.").size(12.0).color(MUTED));
    });
    ui.add_space(10.0);
    card_frame(ui, |ui| {
        ui.label(
            RichText::new("Profile buttons")
                .size(16.0)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new("Your activity includes links to MONORYX and DemonZ Development.")
                .color(TEXT2),
        );
    });
    ui.add_space(10.0);
    card_frame(ui, |ui| {
        ui.label(
            RichText::new("Activity preview")
                .size(16.0)
                .strong()
                .color(TEXT),
        );
        if let Some(preview) = state.discord_preview() {
            ui.label(RichText::new("MONORYX").strong().color(TEXT));
            ui.label(RichText::new(preview["details"].as_str().unwrap_or_default()).color(TEXT));
            ui.label(RichText::new(preview["state"].as_str().unwrap_or_default()).color(TEXT2));
            if preview.get("timestamps").is_some() {
                ui.label(
                    RichText::new("Elapsed time appears below your activity.")
                        .size(12.0)
                        .color(MUTED),
                );
            }
            if let Some(buttons) = preview["buttons"].as_array() {
                ui.horizontal_wrapped(|ui| {
                    for (index, button) in buttons.iter().enumerate() {
                        let label = button["label"].as_str().unwrap_or_default();
                        let clicked = if index == 0 {
                            primary_button(ui, label).clicked()
                        } else {
                            secondary_button(ui, label).clicked()
                        };
                        if clicked {
                            if let Some(url) = button["url"].as_str() {
                                let _ = open::that(url);
                            }
                        }
                    }
                });
            }
        } else {
            ui.label(
                RichText::new("Your activity is hidden until you start Minecraft.").color(TEXT2),
            );
        }
        if !state.config.discord.enabled {
            ui.label(
                RichText::new("Preview only. Turn on Rich Presence above to share it.")
                    .size(12.0)
                    .color(MUTED),
            );
        }
    });
    if changed {
        state.save_config();
        state.sync_discord();
    }
}
