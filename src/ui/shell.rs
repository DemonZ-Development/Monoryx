use crate::app::events::Page;
use crate::app::state::AppState;
use crate::ui::theme::{
    ACCENT, BG, BORDER, DANGER, ELEVATED, ELEVATED2, HOVER, INFO, MUTED, SELECTED, SELECTED_FG,
    TEXT, TEXT2,
};
use egui::{Color32, CornerRadius, RichText, Stroke};

pub fn app_update(state: &mut AppState, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let startup_window_id = egui::Id::new("startup-window-size-applied");
    let startup_passes = ctx.data_mut(|data| data.get_temp::<u8>(startup_window_id).unwrap_or(0));
    if startup_passes < 5 {
        ctx.data_mut(|data| data.insert_temp(startup_window_id, startup_passes + 1));
        if state.config.start_maximized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            #[cfg(target_os = "windows")]
            crate::utils::system::ensure_window_positioned(true, false);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                state.config.window_width.clamp(850.0, 2560.0),
                state.config.window_height.clamp(560.0, 1440.0),
            )));
            #[cfg(target_os = "windows")]
            crate::utils::system::ensure_window_positioned(false, startup_passes == 0);
        }
    }
    let any_playing = state.playing.values().any(|p| *p);
    if any_playing {
        ctx.request_repaint_after(std::time::Duration::from_millis(300));
    } else {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }

    state.poll_events(ctx);

    if state.launcher_hidden {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        return;
    }

    crate::ui::theme::apply_theme(ctx);

    if state.page == Page::Onboarding {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
            });
            crate::ui::pages::onboarding::show(state, ctx, ui);
        });
        handle_overlays(state, ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        return;
    }

    if let Some(update) = state.launcher_update.clone() {
        if update.has_update && state.show_update_banner {
            egui::TopBottomPanel::top("launcher_update_banner")
                .frame(
                    egui::Frame::new()
                        .fill(ELEVATED2)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .inner_margin(egui::Margin::symmetric(16, 8)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Update Available: MONORYX v{} is out! (Current: v{})",
                                update.latest_version, update.current_version
                            ))
                            .strong()
                            .color(crate::ui::theme::INFO),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Dismiss").clicked() {
                                state.show_update_banner = false;
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("View Update").color(SELECTED_FG).strong(),
                                    )
                                    .fill(ACCENT)
                                    .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                state.set_page(Page::Settings);
                            }
                            if let Some(dl) = &update.download_url {
                                if ui.small_button("Download").clicked() {
                                    let _ = open::that(dl);
                                }
                            }
                        });
                    });
                });
        }
    }

    egui::SidePanel::left("sidebar")
        .exact_width(225.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(ELEVATED)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("MONORYX")
                        .size(20.0)
                        .strong()
                        .color(TEXT),
                );
            });
            ui.label(
                RichText::new("Play. Modify. Nothing else.")
                    .size(11.0)
                    .color(MUTED),
            );

            ui.add_space(24.0);
            ui.label(RichText::new("WORKSPACE").size(10.0).strong().color(MUTED));
            ui.add_space(4.0);

            for page in Page::all() {
                if page == Page::Accounts {
                    ui.add_space(16.0);
                    ui.label(RichText::new("MANAGE").size(10.0).strong().color(MUTED));
                    ui.add_space(4.0);
                }
                let sel = state.page == page;
                let has_update_badge = page == Page::Settings
                    && state
                        .launcher_update
                        .as_ref()
                        .is_some_and(|u| u.has_update);

                let resp = sidebar_item(ui, ctx, page.label(), sel, has_update_badge);
                if resp.clicked() {
                    state.set_page(page);
                }
            }

            ui.add_space(12.0);
            let op_phase = state.operations.values().next().map(|o| o.phase.clone());
            if !state.global_status.is_empty() || state.global_frac.is_some() || op_phase.is_some()
            {
                egui::Frame::new()
                    .fill(ELEVATED2)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        if let Some(phase) = &op_phase {
                            ui.label(RichText::new(phase).size(11.0).color(TEXT));
                        } else {
                            ui.label(RichText::new(&state.global_status).size(11.0).color(TEXT));
                        }
                        crate::ui::components::thin_progress(ui, state.global_frac);
                        if ui.small_button("Downloads").clicked() {
                            state.set_page(Page::Downloads);
                        }
                    });
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("v1.1.0 Beta")
                            .size(10.5)
                            .color(MUTED),
                    );
                    if let Some(u) = &state.launcher_update {
                        if u.has_update
                            && ui
                                .button(RichText::new("Update").size(10.5).color(INFO))
                                .clicked()
                        {
                            state.set_page(Page::Settings);
                        }
                    }
                });

                ui.add_space(6.0);

                if let Some(p) = state.config.profile.clone() {
                    let card_id = ui.make_persistent_id("bottom_profile_card");
                    let hovered = ui.ctx().data(|d| d.get_temp::<bool>(card_id).unwrap_or(false));
                    let fade = ui.ctx().animate_bool_with_time(card_id.with("hover"), hovered, 0.15);
                    if fade > 0.001 && fade < 0.999 {
                        ctx.request_repaint();
                    }
                    let fill = ELEVATED2.lerp_to_gamma(HOVER, fade);
                    let border_color = BORDER.lerp_to_gamma(Color32::from_rgb(55, 58, 67), fade);

                    let frame_resp = egui::Frame::new()
                        .fill(fill)
                        .stroke(Stroke::new(1.0_f32, border_color))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let (avatar_rect, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                                ui.painter().circle_filled(avatar_rect.center(), 11.0, Color32::from_rgb(30, 32, 38));
                                ui.painter().circle_stroke(avatar_rect.center(), 11.0, Stroke::new(1.0_f32, BORDER));
                                let initial = p.username.chars().next().unwrap_or('?').to_uppercase().to_string();
                                ui.painter().text(
                                    avatar_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    initial,
                                    egui::FontId::proportional(11.5),
                                    TEXT,
                                );

                                ui.add_space(4.0);
                                let uname_display = if p.username.len() > 11 {
                                    format!("{}…", &p.username[..10])
                                } else {
                                    p.username.clone()
                                };
                                ui.label(RichText::new(uname_display).size(12.5).strong().color(TEXT));

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    egui::Frame::new()
                                        .fill(ELEVATED)
                                        .stroke(Stroke::new(1.0_f32, BORDER))
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(egui::Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new("Offline").size(10.0).color(TEXT2));
                                        });
                                });
                            });
                        });

                    let resp = ui.interact(frame_resp.response.rect, card_id, egui::Sense::click());
                    ui.ctx().data_mut(|d| d.insert_temp(card_id, resp.hovered()));
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if resp.on_hover_text(format!("Offline account: {}\nSkinsRestorer active (Click to manage in Accounts)", p.username)).clicked() {
                        state.set_page(Page::Accounts);
                    }
                } else {
                    let card_id = ui.make_persistent_id("bottom_no_profile_card");
                    let hovered = ui.ctx().data(|d| d.get_temp::<bool>(card_id).unwrap_or(false));
                    let fade = ui.ctx().animate_bool_with_time(card_id.with("hover"), hovered, 0.15);
                    if fade > 0.001 && fade < 0.999 {
                        ctx.request_repaint();
                    }
                    let fill = ELEVATED2.lerp_to_gamma(HOVER, fade);
                    let border_color = BORDER.lerp_to_gamma(Color32::from_rgb(55, 58, 67), fade);

                    let frame_resp = egui::Frame::new()
                        .fill(fill)
                        .stroke(Stroke::new(1.0_f32, border_color))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Configure Account").size(11.5).strong().color(TEXT));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(RichText::new("Set").size(11.0).color(TEXT2));
                                });
                            });
                        });
                    let resp = ui.interact(frame_resp.response.rect, card_id, egui::Sense::click());
                    ui.ctx().data_mut(|d| d.insert_temp(card_id, resp.hovered()));
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if resp.on_hover_text("Click to configure an account").clicked() {
                        state.set_page(Page::Accounts);
                    }
                }
            });
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(BG)
                .inner_margin(egui::Margin::same(26)),
        )
        .show(ctx, |ui| {
            let page_id = egui::Id::new("page-transition");
            let changed = ctx.data_mut(|data| {
                let previous = data.get_temp::<Page>(page_id);
                data.insert_temp(page_id, state.page);
                previous != Some(state.page)
            });
            if changed {
                ctx.animate_bool_with_time(page_id.with("enter"), false, 0.0);
            }
            let enter = ctx.animate_bool_with_time(page_id.with("enter"), true, 0.22);
            let t = enter * enter * (3.0 - 2.0 * enter);
            ui.set_opacity(0.35 + t * 0.65);
            ui.add_space((1.0 - t) * 8.0);
            if enter > 0.001 && enter < 0.999 {
                ctx.request_repaint();
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match state.page {
                    Page::Home => crate::ui::pages::home::show(state, ctx, ui),
                    Page::Instances => crate::ui::pages::instances::show(state, ctx, ui),
                    Page::Discover => crate::ui::pages::discover::show(state, ctx, ui),
                    Page::Library => crate::ui::pages::library::show(state, ctx, ui),
                    Page::Downloads => crate::ui::pages::downloads::show(state, ctx, ui),
                    Page::Nexeu => crate::ui::pages::nexeu::show(state, ctx, ui),
                    Page::Accounts => crate::ui::pages::accounts::show(state, ctx, ui),
                    Page::Settings => crate::ui::pages::settings::show(state, ctx, ui),
                    Page::Logs => crate::ui::pages::logs::show(state, ctx, ui),
                    Page::Onboarding => {}
                });
        });

    handle_overlays(state, ctx);

    if !state.downloads.is_empty()
        || state.search_loading
        || !state.busy_install.is_empty()
        || !state.operations.is_empty()
        || state.java_loading
        || state.launcher_update_loading
    {
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
    } else if state.notice_at.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(400));
    }
}

fn handle_overlays(state: &mut AppState, ctx: &egui::Context) {
    if !state.notice.is_empty() {
        let fresh = state
            .notice_at
            .map(|t| t.elapsed() < std::time::Duration::from_secs(4))
            .unwrap_or(false);
        if fresh {
            egui::TopBottomPanel::bottom("notice")
                .frame(
                    egui::Frame::new()
                        .fill(ELEVATED)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .inner_margin(egui::Margin::same(10)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&state.notice).color(TEXT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Dismiss").clicked() {
                                state.notice.clear();
                            }
                        });
                    });
                });
        } else {
            state.notice.clear();
        }
    }

    if !state.error_dialog.is_empty() {
        let msg = state.error_dialog.clone();
        egui::Window::new("Something went wrong")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add(egui::Label::new(RichText::new(&msg).color(TEXT)).selectable(true));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Copy error").clicked() {
                        ctx.copy_text(msg.clone());
                        state.notify("Error message copied to clipboard");
                    }
                    if ui.button("Close").clicked() {
                        state.error_dialog.clear();
                    }
                    if ui.button("View Logs").clicked() {
                        state.error_dialog.clear();
                        state.set_page(Page::Logs);
                    }
                });
            });
    }

    if state.edit_instance.is_some() {
        egui::Window::new("Edit Instance")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(440.0)
            .show(ctx, |ui| {
                show_edit_dialog(state, ui);
            });
    }

    if state.crash_report.is_some() {
        show_crash_dialog(state, ctx);
    }

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.error_dialog.clear();
        state.show_new_instance = false;
        state.detail_project = None;
        state.crash_report = None;
    }
}

fn show_edit_dialog(state: &mut AppState, ui: &mut egui::Ui) {
    let Some(mut cfg) = state.edit_instance.clone() else {
        return;
    };

    ui.label(RichText::new("Name").size(11.0).color(TEXT2));
    ui.text_edit_singleline(&mut cfg.name);

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Minecraft {}", cfg.minecraft_version)).color(TEXT));
        ui.label(RichText::new(cfg.loader.display_name()).color(TEXT2));
        if !cfg.loader_version.is_empty() {
            ui.label(RichText::new(&cfg.loader_version).color(TEXT2));
        }
    });

    if cfg.loader != crate::instance::config::LoaderKind::Vanilla {
        ui.horizontal(|ui| {
            let checking = state.loader_update_checking.as_deref() == Some(&cfg.id);
            let installing = state.loader_update_busy.as_deref() == Some(&cfg.id);
            if ui
                .add_enabled(
                    !checking && !installing,
                    egui::Button::new("Check loader update"),
                )
                .clicked()
            {
                state.loader_update_checking = Some(cfg.id.clone());
                state.loader_update_candidate = None;
                state.loader_update_error.clear();
                crate::app::tasks::check_loader_update(state, cfg.id.clone());
            }
            if checking || installing {
                ui.spinner();
                ui.label(if checking {
                    "Checking..."
                } else {
                    "Installing..."
                });
            }
            if let Some((id, version)) = state.loader_update_candidate.clone() {
                if id == cfg.id
                    && ui
                        .add_enabled(!installing, egui::Button::new(format!("Install {version}")))
                        .clicked()
                {
                    state.loader_update_busy = Some(cfg.id.clone());
                    state.loader_update_error.clear();
                    crate::app::tasks::install_loader_update(state, cfg.id.clone(), version);
                }
            }
        });
        if !state.loader_update_error.is_empty() {
            ui.colored_label(DANGER, &state.loader_update_error);
        }
    }

    ui.add_space(4.0);

    let mut boost_val = cfg.boost_mode.unwrap_or(state.config.boost_mode);
    ui.horizontal(|ui| {
        if ui
            .checkbox(&mut boost_val, "Eco Mode (Low RAM & Optimized GC)")
            .changed()
        {
            cfg.boost_mode = Some(boost_val);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Min MB");
        ui.add(
            egui::DragValue::new(&mut cfg.memory_min_mb)
                .range(256..=131_072)
                .speed(128),
        );
        ui.label("Max MB");
        ui.add(
            egui::DragValue::new(&mut cfg.memory_max_mb)
                .range(256..=131_072)
                .speed(128),
        );
    });

    ui.label(RichText::new("JVM arguments").size(11.0).color(TEXT2));
    ui.text_edit_singleline(&mut cfg.jvm_args);

    ui.label(RichText::new("Game arguments").size(11.0).color(TEXT2));
    ui.text_edit_singleline(&mut cfg.game_args);

    ui.horizontal(|ui| {
        let mut w = cfg.width.map(|v| v.to_string()).unwrap_or_default();
        let mut h = cfg.height.map(|v| v.to_string()).unwrap_or_default();
        ui.label("W");
        if ui.text_edit_singleline(&mut w).changed() {
            cfg.width = w.parse().ok();
        }
        ui.label("H");
        if ui.text_edit_singleline(&mut h).changed() {
            cfg.height = h.parse().ok();
        }
        ui.checkbox(&mut cfg.fullscreen, "Fullscreen");
    });

    ui.label(RichText::new("Java runtime").size(11.0).color(TEXT2));
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("edit-instance-java-mode")
            .selected_text(match cfg.java_mode {
                crate::instance::config::JavaMode::Automatic => "Automatic",
                crate::instance::config::JavaMode::System => "System",
                crate::instance::config::JavaMode::Custom => "Custom",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut cfg.java_mode,
                    crate::instance::config::JavaMode::Automatic,
                    "Automatic",
                );
                ui.selectable_value(
                    &mut cfg.java_mode,
                    crate::instance::config::JavaMode::System,
                    "System",
                );
                ui.selectable_value(
                    &mut cfg.java_mode,
                    crate::instance::config::JavaMode::Custom,
                    "Custom",
                );
            });
        if cfg.java_mode == crate::instance::config::JavaMode::Custom {
            ui.text_edit_singleline(&mut cfg.java_path);
            if ui.small_button("Browse").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_file() {
                    cfg.java_path = p.display().to_string();
                }
            }
        }
    });

    if !state.edit_error.is_empty() {
        ui.label(RichText::new(&state.edit_error).color(DANGER));
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("Cancel").clicked() {
            state.edit_instance = None;
            state.edit_error.clear();
        }
        if ui.button("Save").clicked() {
            match cfg.validate() {
                Ok(()) => {
                    if let Err(e) = state.instances.save(&cfg) {
                        state.edit_error = e.user_message();
                    } else {
                        state.edit_instance = None;
                        state.edit_error.clear();
                        state.refresh_instances();
                        state.notify("Instance saved");
                    }
                }
                Err(e) => state.edit_error = e.user_message(),
            }
        }
        if ui.button("Export").clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .set_file_name(format!("{}-export.zip", cfg.name))
                .save_file()
            {
                let dir = state.instances.instance_dir(&cfg.id);
                let meta: std::collections::HashMap<String, (Option<String>, Option<String>)> =
                    crate::content::ContentStore::for_instance(&dir)
                        .load()
                        .entries
                        .into_iter()
                        .map(|entry| (entry.file_name, (entry.project_id, entry.version_id)))
                        .collect();
                match crate::instance::export::export_instance(&dir, &cfg, &p, true, &meta) {
                    Ok(()) => state.notify("Instance exported"),
                    Err(e) => state.fail(e.user_message()),
                }
            }
        }
        if ui.button("Manage content").clicked() {
            state.selected_instance = Some(cfg.id.clone());
            state.edit_instance = None;
            state.refresh_library();
            state.set_page(Page::Library);
        }
    });

    if state.edit_instance.is_some() {
        state.edit_instance = Some(cfg);
    }
}

fn sidebar_item(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    label: &str,
    selected: bool,
    has_badge: bool,
) -> egui::Response {
    let id = ui.make_persistent_id(format!("sidebar_item_{label}"));
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 38.0), egui::Sense::click());

    let fade = ctx.animate_bool_with_time(id.with("hover"), response.hovered(), 0.15);
    if fade > 0.001 && fade < 0.999 {
        ctx.request_repaint();
    }
    let fill = if selected {
        SELECTED
    } else if fade > 0.01 {
        Color32::from_rgba_unmultiplied(28, 29, 34, (fade * 255.0) as u8)
    } else {
        Color32::TRANSPARENT
    };

    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
    }

    let text_color = if selected {
        SELECTED_FG
    } else {
        TEXT2.lerp_to_gamma(TEXT, fade)
    };

    let text_pos = egui::pos2(rect.min.x + 14.0, rect.center().y);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        label,
        if selected {
            egui::FontId::new(13.5, egui::FontFamily::Proportional)
        } else {
            egui::FontId::proportional(13.5)
        },
        text_color,
    );

    if has_badge && !selected {
        let badge_pos = egui::pos2(rect.max.x - 12.0, rect.center().y);
        ui.painter().text(
            badge_pos,
            egui::Align2::RIGHT_CENTER,
            "•",
            egui::FontId::proportional(14.0),
            ACCENT,
        );
    }

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    response
}

fn show_crash_dialog(state: &mut AppState, ctx: &egui::Context) {
    let Some(info) = state.crash_report.clone() else {
        return;
    };

    let mut is_open = true;
    let mut close_dialog = false;
    let mut navigate_logs = false;

    let screen_rect = ctx.input(|i| i.screen_rect());
    egui::Area::new(egui::Id::new("crash_modal_backdrop"))
        .fixed_pos(egui::pos2(0.0, 0.0))
        .interactable(true)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.allocate_rect(screen_rect, egui::Sense::click());
            ui.painter().rect_filled(
                screen_rect,
                CornerRadius::ZERO,
                Color32::from_black_alpha(150),
            );
        });

    let dialog_width = (screen_rect.width() - 32.0).clamp(320.0, 820.0);
    let dialog_height = (screen_rect.height() - 48.0).clamp(240.0, 700.0);
    let log_height = (screen_rect.height() - 580.0).clamp(80.0, 300.0);
    egui::Window::new("Game Crash Detected")
        .open(&mut is_open)
        .order(egui::Order::Middle)
        .collapsible(false)
        .fixed_size(egui::vec2(dialog_width, dialog_height))
        .vscroll(true)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .constrain_to(screen_rect.shrink(12.0))
        .show(ctx, |ui| {
            ui.add_space(2.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("GAME CRASH DETECTED")
                        .size(17.0)
                        .strong()
                        .color(DANGER),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    crate::ui::components::badge(
                        ui,
                        &format!(
                            "Exit: {}",
                            crate::minecraft::crash::format_exit_code(info.exit_code)
                        ),
                    );
                });
            });

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Instance: {}", info.instance_name))
                        .strong()
                        .color(TEXT),
                );
            });

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Copy crash report").strong().color(SELECTED_FG))
                            .fill(ACCENT),
                    )
                    .clicked()
                {
                    ctx.copy_text(format!(
                        "MONORYX Game Crash Report\n-------------------------\nInstance: {}\nExit Code: {}\nSummary:\n{}\nSource: {}\n\nDetails:\n{}",
                        info.instance_name,
                        crate::minecraft::crash::format_exit_code(info.exit_code),
                        info.summary,
                        info.source_label,
                        info.details
                    ));
                    state.notify("Crash report copied to clipboard");
                }
                if ui
                    .add_enabled(
                        !state.crash_share_loading,
                        egui::Button::new("Share on mclo.gs"),
                    )
                    .clicked()
                {
                    state.share_crash_log();
                }
                if state.crash_share_loading {
                    ui.spinner();
                }
                if ui.button("View full logs").clicked() {
                    navigate_logs = true;
                }
                if ui.button("Reports folder").clicked() {
                    let _ = std::fs::create_dir_all(&info.crash_reports_dir);
                    let _ = open::that(&info.crash_reports_dir);
                }
                if ui.button("Dismiss").clicked() {
                    close_dialog = true;
                }
            });

            if let Some(url) = state.crash_share_url.clone() {
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("mclo.gs URL:").size(12.0).strong().color(crate::ui::theme::INFO));
                    ui.hyperlink_to(&url, &url);
                    if ui.button("Copy link").clicked() {
                        ctx.copy_text(url.clone());
                        state.notify("mclo.gs link copied");
                    }
                });
            }
            if !state.crash_share_error.is_empty() {
                ui.colored_label(DANGER, &state.crash_share_error);
            }

            ui.add_space(6.0);

            egui::Frame::new()
                .fill(Color32::from_rgb(34, 18, 22))
                .stroke(Stroke::new(1.0_f32, DANGER))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new("Error Summary")
                            .size(11.5)
                            .strong()
                            .color(DANGER),
                    );
                    ui.add_space(3.0);
                    ui.label(RichText::new(&info.summary).size(12.5).color(TEXT));
                });

            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Crash Log / Trace")
                        .size(12.0)
                        .strong()
                        .color(TEXT2),
                );
                ui.label(
                    RichText::new(format!("({})", info.source_label))
                        .size(11.0)
                        .color(MUTED),
                );
            });
            ui.add_space(3.0);

            egui::Frame::new()
                .fill(ELEVATED2)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    egui::ScrollArea::both()
                        .max_height(log_height)
                        .max_width((dialog_width - 36.0).max(250.0))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&info.details)
                                        .monospace()
                                        .size(11.0)
                                        .color(TEXT),
                                )
                                .selectable(true),
                            );
                        });
                });

            ui.add_space(4.0);
            ui.label(
                RichText::new("Note: 'Copy crash report' copies summary and details to clipboard. 'Share on mclo.gs' uploads the log publicly.")
                    .size(11.0)
                    .color(MUTED),
            );
        });

    if !is_open || close_dialog {
        state.crash_report = None;
    } else if navigate_logs {
        state.crash_report = None;
        state.selected_instance = Some(info.instance_id);
        state.set_page(Page::Logs);
    }
}
