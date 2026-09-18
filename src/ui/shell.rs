use crate::app::events::Page;
use crate::app::state::AppState;
use crate::ui::theme::{BG, BORDER, ELEVATED, MUTED, SELECTED, SELECTED_FG, TEXT, TEXT2};
use egui::{CornerRadius, RichText, Stroke};
pub fn app_update(state: &mut AppState, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
    state.poll_events(ctx);
    crate::ui::theme::apply_monochrome(ctx);
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
    egui::SidePanel::left("sidebar")
        .exact_width(220.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(ELEVATED)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .inner_margin(egui::Margin::same(18)),
        )
        .show(ctx, |ui| {
            ui.add_space(14.0);
            ui.label(RichText::new("MONORYX").size(23.0).strong().color(TEXT));
            ui.label(
                RichText::new("Play. Modify. Nothing else.")
                    .size(10.5)
                    .color(MUTED),
            );
            ui.add_space(28.0);
            ui.label(RichText::new("WORKSPACE").size(10.0).strong().color(MUTED));
            ui.add_space(4.0);
            for page in Page::all() {
                if page == Page::Settings {
                    ui.add_space(20.0);
                    ui.label(RichText::new("MANAGE").size(10.0).strong().color(MUTED));
                }
                let sel = state.page == page;
                let fade = ctx.animate_bool_with_time(
                    egui::Id::new(("sidebar-sel", page.as_str())),
                    sel,
                    0.15,
                );
                let fill = ELEVATED.lerp_to_gamma(SELECTED, fade);
                let text = TEXT2.lerp_to_gamma(SELECTED_FG, fade);
                let resp = ui.add_sized(
                    egui::vec2(ui.available_width(), 42.0),
                    egui::Button::new(RichText::new(page.label()).size(13.5).color(text).strong())
                        .fill(fill)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(8)),
                );
                if resp.clicked() {
                    state.set_page(page);
                }
            }
            ui.add_space(10.0);
            let op_phase = state.operations.values().next().map(|o| o.phase.clone());
            if !state.global_status.is_empty() || state.global_frac.is_some() || op_phase.is_some()
            {
                egui::Frame::new()
                    .fill(ELEVATED)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::same(8))
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
                ui.label(
                    RichText::new(format!("v{} • Alpha", env!("CARGO_PKG_VERSION")))
                        .size(10.5)
                        .color(MUTED),
                );
                if let Some(p) = state.config.profile.clone() {
                    ui.horizontal(|ui| {
                        if ui
                            .button(RichText::new(&p.username).size(12.5).strong())
                            .on_hover_text("Edit offline account in Settings")
                            .clicked()
                        {
                            state.set_page(Page::Settings);
                        }
                        egui::Frame::new()
                            .fill(ELEVATED)
                            .stroke(Stroke::new(1.0_f32, BORDER))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(egui::Margin::symmetric(5, 1))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Offline").size(10.0).color(TEXT2));
                            });
                    });
                } else {
                    ui.label(RichText::new("No profile").size(11.0).color(TEXT2));
                }
            });
        });
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(BG)
                .inner_margin(egui::Margin::same(28)),
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
            ui.set_opacity(0.4 + enter * 0.6);
            ui.add_space((1.0 - enter) * 8.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match state.page {
                    Page::Home => crate::ui::pages::home::show(state, ctx, ui),
                    Page::Instances => crate::ui::pages::instances::show(state, ctx, ui),
                    Page::Discover => crate::ui::pages::discover::show(state, ctx, ui),
                    Page::Library => crate::ui::pages::library::show(state, ctx, ui),
                    Page::Downloads => crate::ui::pages::downloads::show(state, ctx, ui),
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
                        .inner_margin(egui::Margin::same(8)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&state.notice).color(TEXT));
                        if ui.small_button("Dismiss").clicked() {
                            state.notice.clear();
                        }
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
            .show(ctx, |ui| {
                ui.label(RichText::new(&msg).color(TEXT));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
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
            .min_width(420.0)
            .show(ctx, |ui| {
                show_edit_dialog(state, ui);
            });
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.error_dialog.clear();
        state.show_new_instance = false;
        state.detail_project = None;
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
    });
    let mut min_s = cfg.memory_min_mb.to_string();
    let mut max_s = cfg.memory_max_mb.to_string();
    ui.horizontal(|ui| {
        ui.label("Min MB");
        if ui.text_edit_singleline(&mut min_s).changed() {
            if let Ok(v) = min_s.parse::<u64>() {
                cfg.memory_min_mb = v;
            }
        }
        ui.label("Max MB");
        if ui.text_edit_singleline(&mut max_s).changed() {
            if let Ok(v) = max_s.parse::<u64>() {
                cfg.memory_max_mb = v;
            }
        }
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
    ui.label(
        RichText::new("Java (automatic / system / custom)")
            .size(11.0)
            .color(TEXT2),
    );
    ui.horizontal(|ui| {
        let mut mode_s = match cfg.java_mode {
            crate::instance::config::JavaMode::Automatic => "automatic".to_string(),
            crate::instance::config::JavaMode::System => "system".to_string(),
            crate::instance::config::JavaMode::Custom => "custom".to_string(),
        };
        ui.text_edit_singleline(&mut mode_s);
        cfg.java_mode = match mode_s.as_str() {
            "system" => crate::instance::config::JavaMode::System,
            "custom" => crate::instance::config::JavaMode::Custom,
            _ => crate::instance::config::JavaMode::Automatic,
        };
        ui.text_edit_singleline(&mut cfg.java_path);
        if ui.small_button("Browse").clicked() {
            if let Some(p) = rfd::FileDialog::new().pick_file() {
                cfg.java_path = p.display().to_string();
                cfg.java_mode = crate::instance::config::JavaMode::Custom;
            }
        }
    });
    if !state.edit_error.is_empty() {
        ui.label(RichText::new(&state.edit_error).color(egui::Color32::from_rgb(0xE0, 0x5A, 0x5A)));
    }
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
                    Default::default();
                match crate::instance::export::export_instance(&dir, &cfg, &p, true, &meta) {
                    Ok(()) => state.notify("Instance exported"),
                    Err(e) => state.fail(e.user_message()),
                }
            }
        }
    });
    if state.edit_instance.is_some() {
        state.edit_instance = Some(cfg);
    }
}
