use crate::app::state::AppState;
use crate::config::{CloseAction, GpuPreference, ThemeKind};
use crate::ui::components::{
    badge, badge_accent, badge_boost, card_frame, field_label, page_header, tab_button,
};
use crate::ui::theme::{DANGER, INFO, MUTED, OK, TEXT, TEXT2, WARNING};
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

    let before = preferences_signature(state);
    let tab_id = egui::Id::new("settings-tab");
    let mut tab = ctx.data_mut(|data| {
        data.get_temp::<SettingsTab>(tab_id)
            .unwrap_or(SettingsTab::Launcher)
    });
    let tabs = [
        (SettingsTab::Launcher, "Launcher"),
        (SettingsTab::Appearance, "Appearance"),
        (SettingsTab::Minecraft, "Minecraft"),
        (SettingsTab::Runtime, "Java & GPU"),
        (SettingsTab::Discord, "Discord"),
        (SettingsTab::About, "About"),
    ];
    let columns = ((ui.available_width() + 8.0) / 118.0).floor().max(1.0) as usize;
    for row in tabs.chunks(columns) {
        ui.horizontal(|ui| {
            for &(value, label) in row {
                if tab_button(ui, label, tab == value).clicked() {
                    tab = value;
                }
            }
        });
    }
    ctx.data_mut(|data| data.insert_temp(tab_id, tab));
    ui.add_space(12.0);

    match tab {
        SettingsTab::Launcher => launcher_settings(state, ctx, ui),
        SettingsTab::Appearance => appearance_settings(state, ctx, ui),
        SettingsTab::Minecraft => minecraft_settings(state, ui),
        SettingsTab::Runtime => runtime_settings(state, ctx, ui),
        SettingsTab::Discord => discord_settings(state, ui),
        SettingsTab::About => about_settings(state, ui),
    }
    if preferences_signature(state) != before {
        state.save_config();
    }
    let pending = state.settings_mem_min != state.config.memory.min_mb.to_string()
        || state.settings_mem_max != state.config.memory.max_mb.to_string()
        || state.settings_jvm != state.config.default_jvm_args
        || state.settings_game_args != state.config.default_game_args
        || ctx.data(|data| {
            data.get_temp::<crate::config::CurseForgeSettings>(egui::Id::new("curseforge-draft"))
                .is_some_and(|draft| {
                    draft.api_key != state.config.curseforge.api_key
                        || draft.custom_endpoint != state.config.curseforge.custom_endpoint
                })
        });
    ui.add_space(8.0);
    ui.label(
        RichText::new(if pending {
            "Some text edits have not been applied yet."
        } else {
            "Preferences save automatically. Text fields with Apply buttons save when you apply them."
        })
        .size(11.0)
        .color(MUTED),
    );
}

fn launcher_settings(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Launcher Updates")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );
            badge(ui, &format!("v{}", env!("CARGO_PKG_VERSION")));
        });

        ui.label(
            RichText::new("Check for a newer version of MONORYX.")
                .size(12.0)
                .color(TEXT2),
        );

        ui.add_space(6.0);
        if ui
            .checkbox(
                &mut state.config.auto_check_updates,
                "Automatically check for launcher updates on startup",
            )
            .changed()
        {
            save_settings(state, "Update preference saved".to_string());
        }
        if ui
            .checkbox(
                &mut state.config.auto_check_instance_updates,
                "Automatically check packages and loaders for updates",
            )
            .changed()
        {
            save_settings(state, "Instance update preference saved".into());
        }
        ui.add(egui::Label::new(
            RichText::new("Checks the selected instance on startup and every 30 minutes. Only updates for its Minecraft version and loader are offered; you choose when to install.")
                .size(11.0).color(MUTED),
        ).wrap());

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
                let banner_corner = CornerRadius::same(8);
                egui::Frame::new()
                    .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                    .stroke(Stroke::new(
                        1.0_f32,
                        crate::ui::theme::palette(ui.ctx()).border,
                    ))
                    .corner_radius(banner_corner)
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
                                    RichText::new(&update.release_notes).size(11.5).color(TEXT2),
                                );
                            });

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            #[cfg(target_os = "windows")]
                            if let Some(path) = state.launcher_update_downloaded.clone() {
                                if crate::ui::components::primary_button(
                                    ui,
                                    "Restart to apply update",
                                )
                                .clicked()
                                {
                                    if let Err(error) =
                                        crate::app::updater::apply_update_and_restart(&path)
                                    {
                                        state.fail(format!("Could not apply update: {error}"));
                                    }
                                }
                            } else if let Some(_dl) = &update.download_url {
                                let label = if state.launcher_update_download_loading {
                                    "Downloading..."
                                } else {
                                    "Download Update"
                                };
                                if crate::ui::components::primary_button(ui, label).clicked()
                                    && !state.launcher_update_download_loading
                                {
                                    state.download_launcher_update();
                                }
                            }
                            #[cfg(not(target_os = "windows"))]
                            if let Some(_dl) = &update.download_url {
                                if crate::ui::components::primary_button(ui, "Download Update")
                                    .clicked()
                                {
                                    let _ = open::that(_dl);
                                }
                            }
                            if crate::ui::components::secondary_button(ui, "Official Website")
                                .clicked()
                            {
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
                ui.label(RichText::new("You are running the latest version of MONORYX.").color(OK));
            }
        }
    });

    ui.add_space(10.0);

    card_frame(ui, |ui| {
        ui.label(
            RichText::new("Launch Behavior & Window")
                .size(16.0)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new("Control launcher window visibility and behavior when Minecraft runs.")
                .size(12.0)
                .color(TEXT2),
        );
        ui.add_space(10.0);

        egui::Grid::new("launch-behavior-grid")
            .num_columns(2)
            .spacing([24.0, 12.0])
            .min_col_width(130.0)
            .max_col_width((ui.available_width() - 170.0).max(170.0))
            .show(ui, |ui| {
                ui.label(RichText::new("When game starts").color(TEXT2));
                ui.vertical(|ui| {
                    egui::ComboBox::from_id_salt("close-action")
                        .selected_text(state.config.close_action.as_str())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut state.config.close_action,
                                CloseAction::Hide,
                                "Hide until Minecraft exits",
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
                    ui.add(
                        egui::Label::new(
                            RichText::new("Hide restores the launcher when Minecraft exits.")
                                .size(11.0)
                                .color(MUTED),
                        )
                        .wrap(),
                    );
                });
                ui.end_row();

                if !state.config.start_maximized {
                    ui.label(RichText::new("Windowed size").color(TEXT2));
                    ui.horizontal_wrapped(|ui| {
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
                        if crate::ui::components::secondary_button(ui, "Apply size").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                                state.config.window_width.clamp(850.0, 2560.0),
                                state.config.window_height.clamp(560.0, 1440.0),
                            )));
                            #[cfg(target_os = "windows")]
                            crate::utils::system::ensure_window_positioned(false, true);
                            save_settings(state, "Window size applied".to_string());
                        }
                        if crate::ui::components::secondary_button(ui, "Center window").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                            #[cfg(target_os = "windows")]
                            crate::utils::system::ensure_window_positioned(false, true);
                        }
                    });
                    ui.end_row();
                }

                ui.label(RichText::new("Parallel downloads").color(TEXT2));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut state.config.parallel_downloads)
                            .range(1..=16)
                            .speed(1),
                    );
                    ui.label(RichText::new("threads").size(12.0).color(MUTED));
                });
                ui.end_row();
            });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut state.config.start_maximized,
                    "Start launcher maximized",
                )
                .changed()
            {
                if state.config.start_maximized {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                    #[cfg(target_os = "windows")]
                    crate::utils::system::ensure_window_positioned(true, false);
                }
                save_settings(state, "Startup window preference saved".to_string());
            }
            if !state.config.start_maximized
                && crate::ui::components::secondary_button(ui, "Maximize now").clicked()
            {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                #[cfg(target_os = "windows")]
                crate::utils::system::ensure_window_positioned(true, false);
            }
        });
        ui.checkbox(
            &mut state.config.remember_instance,
            "Remember last selected instance",
        );
        ui.checkbox(
            &mut state.show_snapshots,
            "Show Minecraft snapshots and experimental releases",
        );

        ui.add_space(10.0);
    });

    ui.add_space(10.0);

    ui.collapsing("Advanced cache & storage", |ui| {
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Cache & storage")
                        .size(16.0)
                        .strong()
                        .color(TEXT),
                );
                badge_accent(ui, "Image cache");
            });

            ui.label(
                RichText::new(
                    "Thumbnail caches are limited and cleared when you leave their pages.",
                )
                .size(12.0)
                .color(TEXT2),
            );

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Cached thumbnail textures: {}",
                        state.thumbnails.len()
                    ))
                    .color(TEXT),
                );
                if crate::ui::components::secondary_button(ui, "Clear image cache").clicked() {
                    let count = state.thumbnails.len();
                    state.thumbnails.clear();
                    state.notify(format!("Purged {count} cached textures; memory freed"));
                }
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new("Hide the launcher while Minecraft runs to reduce rendering work.")
                    .size(11.5)
                    .color(MUTED),
            );
        });
    });
    ui.add_space(10.0);

    ui.collapsing("Advanced CurseForge connection", |ui| {
    let draft_id = egui::Id::new("curseforge-draft");
    let mut connection = ctx.data_mut(|data| {
        data.get_temp::<crate::config::CurseForgeSettings>(draft_id)
            .unwrap_or_else(|| state.config.curseforge.clone())
    });
    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("CurseForge Integration")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );
            if state.config.curseforge.has_custom_key() {
                badge_accent(ui, "Custom Key");
            } else if !state.config.curseforge.custom_endpoint.trim().is_empty() {
                badge_accent(ui, "Custom Proxy");
            } else {
                badge_accent(ui, "Official Server");
            }
        });

        ui.label(
            RichText::new(
                "Search, browse, and install mods, resource packs, and shaders directly from CurseForge.",
            )
            .size(12.0)
            .color(TEXT2),
        );

        ui.add_space(8.0);
        field_label(ui, "Custom API Key (Optional)");
        ui.label(RichText::new("Apply connection changes below to save them.").size(11.0).color(MUTED));
        let mut key = connection.api_key.clone();
        let resp = ui.add(
            egui::TextEdit::singleline(&mut key)
                .margin(egui::vec2(10.0, 8.0))
                .min_size(egui::vec2(0.0, 34.0))
                .password(true)
                .hint_text("Leave blank to use official server")
                .desired_width(ui.available_width().min(480.0)),
        );
        if resp.changed() {
            connection.api_key = key.trim().to_string();
        }
        ui.add(
            egui::Label::new(
                RichText::new("By default, MONORYX connects through our official server. You can optionally enter your own developer key.")
                    .size(11.0)
                    .color(MUTED),
            )
            .wrap(),
        );

        ui.add_space(6.0);
        ui.collapsing("Advanced Server Configuration (Optional)", |ui| {
            field_label(ui, "Custom Proxy Endpoint (Optional)");
            let mut endpoint = connection.custom_endpoint.clone();
            let resp = ui.add(
                egui::TextEdit::singleline(&mut endpoint)
                .margin(egui::vec2(10.0, 8.0))
                .min_size(egui::vec2(0.0, 34.0))
                    .hint_text("https://services.demonz.org/curseforge/v1")
                    .desired_width(ui.available_width().min(480.0)),
            );
            if resp.changed() {
                connection.custom_endpoint = endpoint.trim().to_string();
            }
            ui.add(
                egui::Label::new(
                    RichText::new("If you host your own Cloudflare Worker proxy, enter its URL here. Leave empty to use the default official server. HTTPS is required.")
                        .size(11.0)
                        .color(MUTED),
                )
                .wrap(),
            );
            let endpoint_text = connection.custom_endpoint.trim();
            if !endpoint_text.is_empty()
                && !connection.api_key.trim().is_empty()
                && !crate::curseforge::is_official_endpoint(endpoint_text)
            {
                ui.add(
                    egui::Label::new(
                        RichText::new("Your custom API key will not be sent to this third-party endpoint, so it will be ignored. Clear the key field to remove it from MONORYX's credential store.")
                            .size(11.5)
                            .color(WARNING),
                    )
                    .wrap(),
                );
            }
        });

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if crate::ui::components::primary_button(ui, "Apply CurseForge settings").clicked() {
                state.config.curseforge = connection.clone();
                save_settings(state, "CurseForge settings saved".to_string());
            }

            if (state.config.curseforge.has_custom_key()
                || !state.config.curseforge.custom_endpoint.trim().is_empty())
                && crate::ui::components::secondary_button(ui, "Use Official Server").clicked()
            {
                state.config.curseforge.clear_custom();
                connection = state.config.curseforge.clone();
                save_settings(state, "Switched to official server".to_string());
            }

            ui.add_space(4.0);
            if ui
                .link("Get an API key")
                .on_hover_text("Open CurseForge for Studios console")
                .clicked()
            {
                let _ = open::that("https://console.curseforge.com/");
            }
        });
    });
    ctx.data_mut(|data| data.insert_temp(draft_id, connection));
    });
    ui.add_space(10.0);

    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Setup & Onboarding")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );
            badge_accent(ui, "Wizard");
        });

        ui.label(
            RichText::new(
                "Review the initial setup screens, configure offline or Microsoft accounts, and reset defaults.",
            )
            .size(12.0)
            .color(TEXT2),
        );

        ui.add_space(8.0);
        if crate::ui::components::secondary_button(ui, "Replay Onboarding Tour").clicked() {
            state.replay_onboarding();
            crate::ui::pages::onboarding::reset_background(ctx);
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            #[cfg(target_os = "windows")]
            crate::utils::system::ensure_window_positioned(true, false);
        }
    });
}

fn appearance_settings(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    let previous = state.config.theme;
    card_frame(ui, |ui| {
        ui.label(RichText::new("Theme").size(16.0).strong().color(TEXT));
        ui.label(RichText::new("Choose a launcher color palette.").color(TEXT2));
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            let btn_corner = CornerRadius::same(8);
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
                        .corner_radius(btn_corner)
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

fn minecraft_settings(state: &mut AppState, ui: &mut egui::Ui) {
    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Memory and Eco mode")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );
            if state.config.boost_mode {
                badge_boost(ui, "ACTIVE");
            }
        });

        ui.label(
            RichText::new(
                "Eco mode uses a lower game memory limit. It may lower FPS in large worlds or modded games. Turn it off when you want the smoothest play. This does not affect the launcher's RAM use.",
            )
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

        ui.add_space(10.0);
        ui.label(RichText::new("Game memory").size(12.0).strong().color(TEXT));
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for (label, min, max, hint) in [
                (
                    "2 GB",
                    "512",
                    "2048",
                    "Minimal RAM consumption for vanilla and light play",
                ),
                (
                    "3 GB",
                    "512",
                    "3072",
                    "Great balance for modded 1.20+ with Fabric",
                ),
                ("4 GB", "1024", "4096", "For modpacks with 80-150 mods"),
                ("6 GB", "1024", "6144", "For large 200+ modpacks"),
                (
                    "8 GB",
                    "2048",
                    "8192",
                    "For heavy modpacks with high-res shaders",
                ),
            ] {
                if crate::ui::components::secondary_button(ui, label)
                    .on_hover_text(hint)
                    .clicked()
                {
                    state.settings_mem_min = min.to_string();
                    state.settings_mem_max = max.to_string();
                    state.config.memory.min_mb = min.parse().unwrap_or(512);
                    state.config.memory.max_mb = max.parse().unwrap_or(2048);
                    save_settings(state, format!("{label} preset applied ({max} MB)"));
                }
            }
        });

        ui.add_space(12.0);
        let mut memory_gb = state.config.memory.max_mb as f64 / 1024.0;
        if ui
            .add(
                egui::Slider::new(&mut memory_gb, 0.5..=32.0)
                    .step_by(0.25)
                    .suffix(" GB"),
            )
            .changed()
        {
            state.config.memory.max_mb = (memory_gb * 1024.0).round() as u64;
            state.config.memory.min_mb = state.config.memory.min_mb.min(state.config.memory.max_mb);
            state.settings_mem_min = state.config.memory.min_mb.to_string();
            state.settings_mem_max = state.config.memory.max_mb.to_string();
        }
        ui.collapsing("Advanced memory values (MB)", |ui| {
            ui.label(
                RichText::new("Edit both values, then choose Apply memory values.")
                    .size(11.0)
                    .color(MUTED),
            );
            egui::Grid::new("minecraft-memory-grid")
                .num_columns(2)
                .spacing([24.0, 12.0])
                .min_col_width(130.0)
                .max_col_width((ui.available_width() - 170.0).max(170.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("Default min memory").color(TEXT2));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.settings_mem_min)
                                .margin(egui::vec2(10.0, 8.0))
                                .min_size(egui::vec2(0.0, 34.0))
                                .id(egui::Id::new("settings-mem-min"))
                                .desired_width(110.0)
                                .char_limit(7)
                                .hint_text("512"),
                        );
                        ui.label(RichText::new("MB").size(12.0).color(MUTED));
                    });
                    ui.end_row();

                    ui.label(RichText::new("Default max memory").color(TEXT2));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.settings_mem_max)
                                .margin(egui::vec2(10.0, 8.0))
                                .min_size(egui::vec2(0.0, 34.0))
                                .id(egui::Id::new("settings-mem-max"))
                                .desired_width(110.0)
                                .char_limit(7)
                                .hint_text("3072"),
                        );
                        ui.label(RichText::new("MB").size(12.0).color(MUTED));
                    });
                    ui.end_row();
                });

            ui.add_space(10.0);
            if crate::ui::components::primary_button(ui, "Apply memory values").clicked() {
                save_memory_and_defaults(state);
            }
        });
    });

    ui.add_space(10.0);

    ui.collapsing("Advanced launch & backups", |ui| {
    card_frame(ui, |ui| {
        ui.label(
            RichText::new("Startup & Optimization")
                .size(16.0)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new(
                "Configure launch verification, JVM tuning, class-data sharing, and world backups.",
            )
            .size(12.0)
            .color(TEXT2),
        );

        ui.add_space(10.0);
        egui::Grid::new("minecraft-startup-grid")
            .num_columns(2)
            .spacing([24.0, 12.0])
            .min_col_width(130.0)
            .max_col_width((ui.available_width() - 170.0).max(170.0))
            .show(ui, |ui| {
                ui.label(RichText::new("Before launching, verify").color(TEXT2));
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        for option in [
                            crate::config::VerifyLevel::Quick,
                            crate::config::VerifyLevel::Full,
                        ] {
                            if ui
                                .selectable_value(
                                    &mut state.config.verify_level,
                                    option,
                                    option.label(),
                                )
                                .changed()
                            {
                                save_settings(state, "Launch verification updated.".to_string());
                            }
                        }
                    });
                    ui.add(
                        egui::Label::new(
                            RichText::new(state.config.verify_level.hint())
                                .size(11.0)
                                .color(MUTED),
                        )
                        .wrap(),
                    );
                });
                ui.end_row();

                ui.label(RichText::new("JVM tuning preset").color(TEXT2));
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        for option in crate::config::JvmPreset::all() {
                            if ui
                                .selectable_value(
                                    &mut state.config.jvm_preset,
                                    option,
                                    option.label(),
                                )
                                .changed()
                            {
                                save_settings(state, "JVM preset updated.".to_string());
                            }
                        }
                    });
                    ui.add(
                        egui::Label::new(
                            RichText::new(state.config.jvm_preset.hint())
                                .size(11.0)
                                .color(MUTED),
                        )
                        .wrap(),
                    );
                });
                ui.end_row();

                ui.label(RichText::new("Backup compression").color(TEXT2));
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        for option in [
                            crate::config::BackupCompression::Fast,
                            crate::config::BackupCompression::Maximum,
                            crate::config::BackupCompression::Zstd,
                        ] {
                            if ui
                                .selectable_value(
                                    &mut state.config.backup_compression,
                                    option,
                                    option.label(),
                                )
                                .changed()
                            {
                                save_settings(state, "Backup compression updated.".to_string());
                            }
                        }
                    });
                    ui.add(
                        egui::Label::new(
                            RichText::new(state.config.backup_compression.hint())
                                .size(11.0)
                                .color(MUTED),
                        )
                        .wrap(),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "A world's region files are already compressed by Minecraft, so raising this helps most on a save with lots of loose files, datapacks or resource packs in it.",
                            )
                            .size(11.0)
                            .color(MUTED),
                        )
                        .wrap(),
                    );
                });
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut state.config.appcds,
                    RichText::new("Use a startup archive (AppCDS)").size(13.0),
                )
                .changed()
            {
                save_settings(state, "Startup archive updated.".to_string());
            }
        });
        ui.add(
            egui::Label::new(
                RichText::new(
                    "Lets the JVM reuse class files it has already loaded, which removes a chunk of class parsing from every launch. It only applies to instances that already have an archive, so use Record below once per instance, then launch normally. The next launch after that is the faster one.",
                )
                .size(11.0)
                .color(MUTED),
            )
            .wrap(),
        );
        if let Some(id) = state.selected_instance.clone() {
            let recorded = crate::app::appcds::archive_exists(&state.instances, &id);
            let cp = state.classpath_preview.clone().unwrap_or_default();
            let state_kind = crate::app::appcds::archive_state(&state.instances, &id, &cp);
            let current = state_kind == crate::app::appcds::ArchiveState::Current;
            let size = crate::app::appcds::archive_size(&state.instances, &id);
            let pending = state.selected().is_some_and(|c| c.appcds_pending);

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let label = if pending {
                    "Re-record on next launch"
                } else if !recorded {
                    "Record startup archive"
                } else if current {
                    "Archive up to date"
                } else {
                    "Re-record startup archive"
                };
                let can_record = !recorded || !current;
                let mut clicked = false;
                ui.add_enabled_ui(can_record, |ui| {
                    if crate::ui::components::secondary_button(ui, label).clicked() {
                        clicked = true;
                    }
                });
                if clicked {
                    crate::app::appcds::request_archive(state, &id);
                }
                if let Some(bytes) = size {
                    ui.label(
                        RichText::new(format!("{} KB", bytes / 1024))
                            .size(11.0)
                            .color(MUTED),
                    );
                }
            });
            let (status, tone) = if pending {
                (
                    "Queued. The next launch records it and the launch after that is the faster one. Recording happens as the game closes, so quit normally.",
                    INFO,
                )
            } else if !recorded {
                ("Not recorded yet for this instance.", MUTED)
            } else if current {
                ("In use on this instance's next launch.", MUTED)
            } else {
                (
                    "Out of date: your mods or config changed since this was recorded, so the JVM quietly ignores it. Re-record to get the speed-up back.",
                    WARNING,
                )
            };
            ui.add(egui::Label::new(RichText::new(status).size(11.0).color(tone)).wrap());
        } else {
            ui.label(
                RichText::new("Select an instance to record its startup archive.")
                    .size(11.0)
                    .color(MUTED),
            );
        }

        ui.add_space(8.0);
        ui.collapsing("Advanced launch arguments", |ui| {
            field_label(ui, "Default JVM arguments");
            ui.add(
                egui::TextEdit::singleline(&mut state.settings_jvm)
                .margin(egui::vec2(10.0, 8.0))
                .min_size(egui::vec2(0.0, 34.0))
                    .id(egui::Id::new("settings-jvm-args"))
                    .hint_text("-Xmx4G")
                    .desired_width(ui.available_width().min(480.0)),
            );
            ui.add(
                egui::Label::new(
                    RichText::new("Applied to new instances. Existing ones keep their own value.")
                        .size(11.0)
                        .color(MUTED),
                )
                .wrap(),
            );
            ui.add_space(4.0);
            field_label(ui, "Default game arguments");
            ui.add(
                egui::TextEdit::singleline(&mut state.settings_game_args)
                .margin(egui::vec2(10.0, 8.0))
                .min_size(egui::vec2(0.0, 34.0))
                    .id(egui::Id::new("settings-game-args"))
                    .hint_text("--username Steve")
                    .desired_width(ui.available_width().min(480.0)),
            );
            ui.add(
                egui::Label::new(
                    RichText::new("Appended to the game command line.")
                        .size(11.0)
                        .color(MUTED),
                )
                .wrap(),
            );
            ui.add_space(8.0);
            if crate::ui::components::primary_button(ui, "Apply launch arguments").clicked() {
                state.config.default_jvm_args = state.settings_jvm.clone();
                state.config.default_game_args = state.settings_game_args.clone();
                save_settings(state, "Launch arguments saved".into());
            }
        });
    });
    });
}

fn save_memory_and_defaults(state: &mut AppState) {
    match (
        state.settings_mem_min.trim().parse::<u64>(),
        state.settings_mem_max.trim().parse::<u64>(),
    ) {
        (Ok(min), Ok(max)) => match crate::utils::system::validate_memory(min, max) {
            Ok(warn) => {
                state.config.memory.min_mb = min;
                state.config.memory.max_mb = max;
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

fn runtime_settings(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    card_frame(ui, |ui| {
        ui.label(
            RichText::new("Java Runtime & GPU")
                .size(16.0)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new(
                "Manage Java installations and graphics processor preference for Minecraft.",
            )
            .size(12.0)
            .color(TEXT2),
        );

        ui.add_space(6.0);
        if state.java_loading {
            ui.label("Detecting Java...");
        }
        if state.java_list.is_empty() && !state.java_loading {
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "No Java detected. Automatic mode downloads a compatible managed runtime when you launch a game.",
                    )
                    .color(TEXT2),
                )
                .wrap(),
            );
        }
        ui.horizontal(|ui| {
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
            if crate::ui::components::secondary_button(ui, "Refresh Java").clicked() {
                state.refresh_java();
            }
        });

        ui.add_space(8.0);
        egui::Grid::new("runtime-settings-grid")
            .num_columns(2)
            .spacing([24.0, 12.0])
            .min_col_width(130.0)
            .max_col_width((ui.available_width() - 170.0).max(170.0))
            .show(ui, |ui| {
                ui.label(RichText::new("Java Mode").color(TEXT2));
                ui.vertical(|ui| {
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
                                ui.selectable_value(
                                    &mut state.config.java.mode,
                                    value.to_string(),
                                    label,
                                );
                            }
                        });
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Automatic selects a compatible Java runtime and downloads a managed Temurin runtime if needed.",
                            )
                            .size(11.0)
                            .color(MUTED),
                        )
                        .wrap(),
                    );
                });
                ui.end_row();

                if state.config.java.mode == "custom" {
                    ui.label(RichText::new("Custom Java path").color(TEXT2));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.config.java.custom_path)
                .margin(egui::vec2(10.0, 8.0))
                .min_size(egui::vec2(0.0, 34.0))
                                .id(egui::Id::new("settings-java-path"))
                                .desired_width((ui.available_width() - 85.0).clamp(100.0, 380.0))
                                .hint_text("C:\\Program Files\\Java\\bin\\javaw.exe"),
                        );
                        if crate::ui::components::secondary_button(ui, "Browse").clicked() {
                            if let Some(p) = rfd::FileDialog::new().pick_file() {
                                state.config.java.custom_path = p.display().to_string();
                            }
                        }
                    });
                    ui.end_row();
                }

                ui.label(RichText::new("GPU preference").color(TEXT2));
                ui.vertical(|ui| {
                    ui.add_enabled_ui(cfg!(target_os = "windows"), |ui| {
                        egui::ComboBox::from_id_salt("gpu-preference")
                            .selected_text(state.config.gpu_preference.as_str())
                            .show_ui(ui, |ui| {
                                for value in [
                                    GpuPreference::System,
                                    GpuPreference::HighPerformance,
                                    GpuPreference::PowerSaving,
                                ] {
                                    ui.selectable_value(
                                        &mut state.config.gpu_preference,
                                        value,
                                        value.as_str(),
                                    );
                                }
                            });
                    });
                    ui.add(
                        egui::Label::new(
                            RichText::new("Windows only. High Performance targets dedicated GPUs.")
                                .size(11.0)
                                .color(MUTED),
                        )
                        .wrap(),
                    );
                });
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.collapsing("Advanced Java tuning", |ui| {
            ui.label(
                RichText::new("Java & GC Performance Presets")
                    .size(14.0)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new(
                    "Choose Java flags for your game. The default works for most instances.",
                )
                .size(11.5)
                .color(TEXT2),
            );
            ui.add_space(6.0);
            egui::ComboBox::from_id_salt("runtime-jvm-preset")
                .selected_text(state.config.jvm_preset.label())
                .show_ui(ui, |ui| {
                    for preset in crate::config::JvmPreset::all() {
                        ui.selectable_value(&mut state.config.jvm_preset, preset, preset.label());
                    }
                });
            ui.add_space(4.0);
            ui.label(
                RichText::new(state.config.jvm_preset.hint())
                    .size(11.0)
                    .color(MUTED),
            );
            if state.config.jvm_preset != crate::config::JvmPreset::None {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if crate::ui::components::secondary_button(ui, "Copy flags to defaults")
                        .clicked()
                    {
                        state.config.default_jvm_args = state.config.jvm_preset.flags().to_string();
                        state.settings_jvm = state.config.default_jvm_args.clone();
                        state.notify("Copied preset flags to default JVM args");
                    }
                });
            }
        });

        ui.add_space(8.0);
        gpu_status(ui, ctx, state);

        ui.add_space(10.0);
    });
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
        ui.label(RichText::new("Detected GPUs").strong().color(TEXT2));
        if crate::ui::components::secondary_button(ui, "Refresh GPUs").clicked() {
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
        ui.add_space(4.0);
        ui.add(
            egui::Label::new(
                RichText::new(
                    "Dedicated GPU found. High performance asks Windows to run Minecraft on it.",
                )
                .size(11.5)
                .color(TEXT2),
            )
            .wrap(),
        );
        if crate::ui::components::secondary_button(ui, "Use High performance").clicked() {
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
                .size(16.0)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new(
                "Show your Minecraft activity on your Discord profile while the desktop app is open.",
            )
            .color(TEXT2),
        );
        ui.add_space(4.0);
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
        ui.label(
            RichText::new("What others can see")
                .size(13.5)
                .strong()
                .color(TEXT),
        );
        ui.add_space(2.0);
        ui.add_enabled_ui(state.config.discord.enabled, |ui| {
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
        });
        ui.add_space(2.0);
        ui.add(
            egui::Label::new(
                RichText::new(
                    "World names are detected from active saves on Windows. Server names come from your saved server list. If a game version doesn't expose a detail, the activity stays general.",
                )
                .size(11.5)
                .color(MUTED),
            )
            .wrap(),
        );
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
                        .size(11.5)
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
                    .size(11.5)
                    .color(MUTED),
            );
        }
    });
    if changed {
        state.save_config();
        state.sync_discord();
    }
}

fn about_settings(state: &mut AppState, ui: &mut egui::Ui) {
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
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if crate::ui::components::secondary_button(ui, "Open Data Folder").clicked() {
                let _ = open::that(state.paths.root());
            }
            if crate::ui::components::secondary_button(ui, "Open Logs Folder").clicked() {
                let _ = open::that(state.paths.logs_dir());
            }
        });
        ui.add_space(10.0);
        ui.label(
            RichText::new("Where your data lives")
                .size(14.0)
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
                    "Everything lives here: your instances, mods, worlds, world backups, screenshots, settings, plus the Minecraft files, Java runtimes and assets MONORYX downloaded. Uninstalling asks whether to delete this folder, and you can always say no and delete it yourself here instead.",
                )
                .size(11.0)
                .color(MUTED),
            )
            .wrap(),
        );
    });
}

pub(crate) fn show_appearance(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("settings-tab"), SettingsTab::Appearance));
}

fn preferences_signature(state: &AppState) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    (
        state.config.close_action as u8,
        state.config.gpu_preference as u8,
        state.config.start_maximized,
        state.config.remember_instance,
        state.show_snapshots,
        state.config.parallel_downloads,
        state.config.window_width.to_bits(),
        state.config.window_height.to_bits(),
        state.config.memory.min_mb,
        state.config.memory.max_mb,
    )
        .hash(&mut hash);
    state.config.java.mode.hash(&mut hash);
    state.config.java.custom_path.hash(&mut hash);
    state.config.default_jvm_args.hash(&mut hash);
    state.config.default_game_args.hash(&mut hash);
    state.config.jvm_preset.label().hash(&mut hash);
    hash.finish()
}

#[cfg(test)]
pub(crate) fn select_review_tab(ctx: &egui::Context, scene: &str) {
    let tab = match scene {
        "settings-appearance" => SettingsTab::Appearance,
        "settings-minecraft" => SettingsTab::Minecraft,
        "settings-runtime" => SettingsTab::Runtime,
        _ => SettingsTab::Launcher,
    };
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("settings-tab"), tab));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::paths::MonoryxPaths;

    #[test]
    fn settings_renders_all_tabs_wide_and_narrow() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths);

        let tabs = [
            SettingsTab::Launcher,
            SettingsTab::Appearance,
            SettingsTab::Minecraft,
            SettingsTab::Runtime,
            SettingsTab::Discord,
            SettingsTab::About,
        ];

        for tab in tabs {
            ctx.data_mut(|data| data.insert_temp(egui::Id::new("settings-tab"), tab));
            for width in [1536.0, 1024.0, 850.0, 720.0] {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 900.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                show(&mut state, ctx, ui);
                            });
                        });
                    },
                );
            }
        }
    }

    #[test]
    fn replay_onboarding_resets_state_and_navigates() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx);
        let mut state = AppState::new_for_preview(&cc, paths);
        state.config.completed_onboarding = true;
        state.page = crate::app::events::Page::Settings;
        state.config.profile = Some(crate::account::offline::OfflineProfile::new("Steve").unwrap());
        state.config.use_microsoft_auth = false;

        state.replay_onboarding();

        assert_eq!(state.page, crate::app::events::Page::Onboarding);
        assert!(!state.config.completed_onboarding);
        assert_eq!(state.onboarding_step, 0);
        assert_eq!(state.onboarding_user, "Steve");
        assert!(!state.onboarding_use_microsoft);
    }

    #[test]
    fn curseforge_settings_switching_official_clears_keys() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx);
        let mut state = AppState::new_for_preview(&cc, paths);

        state.config.curseforge.api_key = "custom_key".to_string();
        state.config.curseforge.custom_endpoint = "https://custom.proxy/v1".to_string();
        assert!(state.config.curseforge.has_custom_key());

        state.config.curseforge.clear_custom();
        assert!(!state.config.curseforge.has_custom_key());
        assert!(state.config.curseforge.custom_endpoint.is_empty());
    }
}
