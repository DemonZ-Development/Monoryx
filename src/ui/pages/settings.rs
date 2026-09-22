use crate::app::state::AppState;
use crate::config::{CloseAction, GpuPreference};
use crate::ui::components::{badge_accent, badge_boost, card_frame, field_label, page_header};
use crate::ui::theme::{
    ACCENT, BORDER, BOOST, DANGER, ELEVATED2, MUTED, OK, SELECTED_FG, TEXT, TEXT2,
};
use egui::{CornerRadius, RichText, Stroke};

pub fn show(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Settings",
        "Launcher updates, window behavior, performance, and runtime preferences.",
    );

        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Launcher Updates").size(16.0).strong().color(TEXT));
                badge_accent(ui, "v1.0.0 Beta");
            });

            ui.label(
                RichText::new(
                    "Keep MONORYX up-to-date with the latest performance improvements, security patches, and features.",
                )
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

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Check for Updates").strong()).clicked() {
                    state.check_launcher_update();
                }

                if state.launcher_update_loading {
                    ui.spinner();
                    ui.label(RichText::new("Checking for updates...").color(TEXT2));
                }
            });

            if let Some(err) = &state.launcher_update_error {
                ui.add_space(4.0);
                ui.label(RichText::new(format!("Update check error: {err}")).color(DANGER));
            }

            if let Some(update) = state.launcher_update.clone() {
                ui.add_space(8.0);
                if update.has_update {
                    egui::Frame::new()
                        .fill(ELEVATED2)
                        .stroke(Stroke::new(1.0_f32, BORDER))
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
                            egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                                ui.label(
                                    RichText::new(&update.release_notes).size(11.5).color(TEXT2),
                                );
                            });

                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if let Some(dl) = &update.download_url {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Download Update")
                                                    .strong()
                                                    .color(SELECTED_FG),
                                            )
                                            .fill(ACCENT),
                                        )
                                        .clicked()
                                    {
                                        let _ = open::that(dl);
                                    }
                                }
                                if ui.button("View on GitHub").clicked() {
                                    let _ = open::that(&update.html_url);
                                }
                            });
                        });
                } else if !state.launcher_update_loading && state.launcher_update_error.is_none() {
                    ui.label(RichText::new("You are running the latest version of MONORYX.").color(OK));
                }
            }
        });

        ui.add_space(8.0);

        card_frame(ui, |ui| {
            ui.label(RichText::new("Launch Behavior & Window").size(16.0).strong().color(TEXT));
            ui.label(
                RichText::new("Control launcher window visibility and behavior when Minecraft runs.")
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
                            "Never",
                        );
                    });
            });
            ui.label(
                RichText::new(
                    "When set to Close / Hide, MONORYX closes its window when Minecraft launches to save system resources, and automatically restores and refocuses itself when the game exits.",
                )
                .size(11.5)
                .color(MUTED),
            );

            ui.add_space(4.0);
            ui.checkbox(&mut state.config.remember_instance, "Remember last selected instance");
            ui.checkbox(&mut state.show_snapshots, "Show Minecraft snapshots and experimental releases");

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Parallel download threads:").color(TEXT2));
                let mut v = state.config.parallel_downloads.to_string();
                if ui.text_edit_singleline(&mut v).changed() {
                    if let Ok(n) = v.parse::<usize>() {
                        state.config.parallel_downloads = n.clamp(1, 16);
                    }
                }
                ui.label(RichText::new("(1-16)").size(11.0).color(MUTED));
            });

            ui.add_space(4.0);
            if ui.button("Save Window & Launch Settings").clicked() {
                save_settings(state, "Launch settings saved".to_string());
            }
        });

        ui.add_space(8.0);

        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Eco Mode & Memory Presets")
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
                RichText::new(
                    "Tuned for smooth gameplay and minimal physical RAM footprint. Returns unused heap memory back to Windows, deduplicates string memory, and optimizes Java garbage collection.",
                )
                .size(12.0)
                .color(TEXT2),
            );

            ui.add_space(6.0);
            if ui
                .checkbox(
                    &mut state.config.boost_mode,
                    "Enable Eco Mode by default for new instances",
                )
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
            ui.label(RichText::new("Quick Memory Presets:").size(12.0).strong().color(TEXT));
            ui.horizontal(|ui| {
                if ui
                    .button("2 GB (Low RAM / Vanilla)")
                    .on_hover_text("Recommended: Minimal RAM consumption for vanilla and light play")
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
                    .button("4 GB (Heavy Modpacks)")
                    .on_hover_text("For large modpacks with 100+ mods")
                    .clicked()
                {
                    state.settings_mem_min = "1024".to_string();
                    state.settings_mem_max = "4096".to_string();
                    state.config.memory.min_mb = 1024;
                    state.config.memory.max_mb = 4096;
                    save_settings(state, "Heavy modpack preset applied (4096 MB)".to_string());
                }
            });

            ui.add_space(10.0);
            egui::Frame::new()
                .fill(ELEVATED2)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.label(RichText::new("Active Optimizations").size(13.0).strong().color(TEXT));
                    ui.add_space(8.0);

                    let optimizations = [
                        ("RAM Reclaim", "Periodically returns unused heap memory back to Windows while playing"),
                        ("String Deduplication", "Eliminates duplicate strings in memory across game and mod assets"),
                        ("Smooth Frame Pacing", "Caps Java garbage collector pause times to 50ms to eliminate micro-stutters"),
                        ("Compact 32-bit Pointers", "Enables compressed object references to save 20-30% heap space"),
                    ];

                    for (title, desc) in optimizations {
                        ui.horizontal(|ui| {
                            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot_rect.center(), 2.5_f32, BOOST);
                            ui.add_space(4.0);
                            ui.label(RichText::new(title).size(12.0).strong().color(TEXT));
                            ui.label(RichText::new(format!("- {desc}")).size(11.5).color(TEXT2));
                        });
                        ui.add_space(4.0);
                    }
                });
        });

        ui.add_space(8.0);

        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Java Runtime & GPU").size(16.0).strong().color(TEXT));
                if ui.small_button("Refresh Java").clicked() {
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
            for j in state.java_list.clone() {
                ui.label(
                    RichText::new(format!("Java {} - {}", j.major, j.path.display()))
                        .size(12.0)
                        .color(TEXT),
                );
                ui.label(
                    RichText::new(format!("{} - {}", j.source, j.version_string))
                        .size(11.0)
                        .color(TEXT2),
                );
            }
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
            field_label(ui, "Custom Java path");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut state.config.java.custom_path);
                if ui.small_button("Browse").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_file() {
                        state.config.java.custom_path = p.display().to_string();
                    }
                }
            });
            ui.add_space(4.0);
            gpu_preference_selector(ui, &mut state.config.gpu_preference);
            gpu_status(ui, ctx, state);
            if ui.button("Save Java & GPU settings").clicked() {
                save_settings(state, "Java & GPU settings saved".to_string());
            }
        });

        ui.add_space(8.0);

        card_frame(ui, |ui| {
            ui.label(RichText::new("Minecraft Defaults").size(16.0).strong().color(TEXT));
            field_label(ui, "Default min memory (MB)");
            ui.text_edit_singleline(&mut state.settings_mem_min);
            field_label(ui, "Default max memory (MB)");
            ui.text_edit_singleline(&mut state.settings_mem_max);
            field_label(ui, "Default JVM arguments");
            ui.text_edit_singleline(&mut state.settings_jvm);
            field_label(ui, "Default game arguments");
            ui.text_edit_singleline(&mut state.settings_game_args);
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

        ui.add_space(8.0);

        card_frame(ui, |ui| {
            ui.label(RichText::new("About MONORYX").size(16.0).strong().color(TEXT));
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
        });
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
    if !ctx
        .data_mut(|data| data.get_temp::<bool>(auto_id).unwrap_or(false))
    {
        ctx.data_mut(|data| data.insert_temp(auto_id, true));
        state.refresh_gpus();
    }
    ui.horizontal(|ui| {
        field_label(ui, "Detected GPUs");
        if ui.small_button("Refresh").clicked() {
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
        if ui.small_button("Use High performance").clicked() {
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
