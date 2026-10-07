use crate::app::events::Page;
use crate::app::state::AppState;
use crate::ui::theme::{metrics, type_scale, ACCENT, DANGER, MUTED, SELECTED_FG, TEXT, TEXT2};
use egui::{Color32, CornerRadius, RichText, Stroke};

pub fn app_update(state: &mut AppState, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let startup_window_id = egui::Id::new("startup-window-size-applied");
    let startup_passes = ctx.data_mut(|data| data.get_temp::<u8>(startup_window_id).unwrap_or(0));
    if startup_passes < 5 {
        ctx.data_mut(|data| data.insert_temp(startup_window_id, startup_passes + 1));
        if state.page == Page::Onboarding || state.config.start_maximized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            #[cfg(target_os = "windows")]
            crate::utils::system::ensure_window_positioned(true, false);
        } else {
            let (w, h) = (
                state.config.window_width.clamp(850.0, 2560.0),
                state.config.window_height.clamp(560.0, 1440.0),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
            #[cfg(target_os = "windows")]
            crate::utils::system::ensure_window_positioned(false, startup_passes == 0);
        }
    }
    let any_playing = state.playing.values().any(|p| *p);
    if any_playing {
        ctx.request_repaint_after(std::time::Duration::from_millis(300));
    } else {
        ctx.request_repaint_after(std::time::Duration::from_millis(1500));
    }

    state.poll_events(ctx);
    state.poll_instance_updates();
    state.sync_discord();

    if state.launcher_hidden {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        return;
    }

    if crate::ui::theme::current_theme(ctx) != state.config.theme {
        crate::ui::theme::apply_selected_theme(ctx, state.config.theme);
    }
    let theme = crate::ui::theme::palette(ctx);

    crate::ui::palette::handle_shortcuts(state, ctx);

    if state.page != Page::Onboarding {
        crate::ui::pages::onboarding::release_background(ctx);
    }

    if state.page == Page::Onboarding {
        egui::CentralPanel::default().show(ctx, |ui| {
            crate::ui::pages::onboarding::show(state, ctx, ui);
        });
        handle_overlays(state, ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        return;
    }

    crate::ui::pages::onboarding::release_background(ctx);

    if let Some(update) = state.launcher_update.clone() {
        if update.has_update && state.show_update_banner {
            egui::TopBottomPanel::top("launcher_update_banner")
                .frame(
                    egui::Frame::new()
                        .fill(theme.elevated2)
                        .stroke(Stroke::new(1.0_f32, theme.border))
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
                            if ui.button("Dismiss").clicked() {
                                state.show_update_banner = false;
                            }
                            let banner_btn_corner = CornerRadius::same(6);
                            if let Some(path) = state.launcher_update_downloaded.clone() {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Restart to Update")
                                                .color(SELECTED_FG)
                                                .strong(),
                                        )
                                        .fill(ACCENT)
                                        .corner_radius(banner_btn_corner),
                                    )
                                    .clicked()
                                {
                                    let _ = crate::app::updater::apply_update_and_restart(&path);
                                }
                            } else {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("View Update")
                                                .color(SELECTED_FG)
                                                .strong(),
                                        )
                                        .fill(ACCENT)
                                        .corner_radius(banner_btn_corner),
                                    )
                                    .clicked()
                                {
                                    state.set_page(Page::Settings);
                                }
                                if let Some(_dl) = &update.download_url {
                                    if crate::ui::components::secondary_button(ui, "Download")
                                        .clicked()
                                    {
                                        #[cfg(target_os = "windows")]
                                        {
                                            state.download_launcher_update();
                                            state.set_page(Page::Settings);
                                        }
                                        #[cfg(not(target_os = "windows"))]
                                        let _ = open::that(_dl);
                                    }
                                }
                            }
                        });
                    });
                });
        }
    }

    egui::SidePanel::left("sidebar")
        .exact_width(crate::ui::theme::sidebar_width(ctx))
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(theme.elevated)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ctx, |ui| {
            if crate::ui::theme::current_theme(ctx) == crate::config::ThemeKind::Halloween {
                let mut artwork = ui.painter().clone();
                artwork.set_opacity(0.45);
                crate::ui::components::draw_halloween_sidebar_artwork(&artwork, ui.max_rect());
            }
            ui.add_space(6.0);
            if crate::ui::theme::current_theme(ctx) == crate::config::ThemeKind::Halloween {
                let (bat_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 16.0),
                    egui::Sense::hover(),
                );
                let bat_center = egui::pos2(bat_rect.left() + 53.0, bat_rect.center().y);
                crate::ui::components::draw_glowing_bat(ui.painter(), bat_center, theme.accent);
                ui.add_space(2.0);
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("MONORYX").size(20.0).strong().color(TEXT));
            });
            ui.label(
                RichText::new("Play. Modify. Nothing else.")
                    .size(11.0)
                    .color(MUTED),
            );

            ui.add_space(24.0);
            egui::ScrollArea::vertical()
                .id_salt("sidebar-navigation")
                .max_height((ui.available_height() - 192.0).max(80.0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.label(RichText::new("WORKSPACE").size(10.0).strong().color(MUTED));
                    ui.add_space(4.0);

                    for page in Page::all()
                        .into_iter()
                        .filter(|page| !matches!(page, Page::Accounts | Page::Settings))
                    {
                        let sel = state.page == page;
                        let has_update_badge =
                            page == Page::Library && state.instance_update_count() > 0;

                        let resp = sidebar_item(ui, ctx, page, sel, has_update_badge);
                        if resp.clicked() {
                            state.set_page(page);
                        }
                    }

                    ui.add_space(12.0);
                    status_card(state, ui, theme);
                });
            ui.add_space(8.0);
            for page in [Page::Accounts, Page::Settings] {
                let badge = page == Page::Settings
                    && state.launcher_update.as_ref().is_some_and(|u| u.has_update);
                if sidebar_item(ui, ctx, page, state.page == page, badge).clicked() {
                    state.set_page(page);
                }
            }
            sidebar_footer(state, ui);
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(theme.bg)
                .inner_margin(egui::Margin::same(metrics::PAGE_MARGIN)),
        )
        .show(ctx, |ui| {
            if crate::ui::theme::current_theme(ctx) == crate::config::ThemeKind::Halloween {
                crate::ui::components::draw_halloween_shell_artwork(ui.painter(), ui.max_rect());
            }
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
            if enter > 0.001 && enter < 0.999 {
                ctx.request_repaint();
            }

            let mut scroll_area = egui::ScrollArea::vertical()
                .id_salt(("page-scroll", state.page.as_str()))
                .auto_shrink([false, false]);
            if state.page == Page::Discover && state.reset_discover_scroll {
                scroll_area = scroll_area.vertical_scroll_offset(0.0);
                state.reset_discover_scroll = false;
            }

            let content_width = ui.available_width();
            scroll_area.show(ui, |ui| {
                ui.set_width(content_width);
                match state.page {
                    Page::Home => crate::ui::pages::home::show(state, ctx, ui),
                    Page::Instances => crate::ui::pages::instances::show(state, ctx, ui),
                    Page::Worlds => crate::ui::pages::worlds::show(state, ctx, ui),
                    Page::Discover => crate::ui::pages::discover::show(state, ctx, ui),
                    Page::Library => crate::ui::pages::library::show(state, ctx, ui),
                    Page::Screenshots => crate::ui::pages::screenshots::show(state, ctx, ui),
                    Page::Downloads => crate::ui::pages::downloads::show(state, ctx, ui),
                    Page::Nexeu => crate::ui::pages::nexeu::show(state, ctx, ui),
                    Page::Accounts => crate::ui::pages::accounts::show(state, ctx, ui),
                    Page::Settings => crate::ui::pages::settings::show(state, ctx, ui),
                    Page::Logs => crate::ui::pages::logs::show(state, ctx, ui),
                    Page::Onboarding => {}
                }
            });
        });

    crate::ui::palette::show(state, ctx);

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
    let dismissible = state.edit_instance.is_some()
        || !state.error_dialog.is_empty()
        || state.pending_content_delete.is_some()
        || state.crash_report.is_some()
        || state.screenshot_viewer.is_some()
        || state.project_image_url.is_some()
        || state.detail_slug.is_some()
        || state.show_new_instance
        || state.confirm_sign_out
        || state.confirm_delete.is_some();
    if dismissible && crate::ui::components::dialog_backdrop(ctx) {
        state.edit_instance = None;
        state.error_dialog.clear();
        state.pending_content_delete = None;
        state.crash_report = None;
        state.screenshot_viewer = None;
        state.confirm_sign_out = false;
        state.screenshot_full_image = None;
        state.show_new_instance = false;
        state.confirm_delete = None;
        crate::ui::pages::discover::close_detail(state);
        state.project_image_url = None;
        state.project_image = None;
        state.project_image_loading = false;
        state.project_image_error.clear();
    }

    if !dismissible {
        ctx.animate_bool_with_time(egui::Id::new("dialog-shade-enter"), false, 0.0);
        ctx.animate_bool_with_time(egui::Id::new("dialog-content-enter"), false, 0.0);
        crate::ui::backdrop::paint(
            ctx,
            &ctx.layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                egui::Id::new("dialog-backdrop"),
            )),
            false,
        );
    }

    if let Some(current) = state.screenshot_viewer.clone() {
        let index = state
            .screenshots
            .iter()
            .position(|entry| entry.path == current);
        let mut close = ctx.input(|input| input.key_pressed(egui::Key::Escape));
        let mut next = None;
        egui::Window::new("Screenshot")
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(520.0)
            .max_width(ctx.available_rect().width() * 0.95)
            .show(ctx, |ui| {
                crate::ui::components::dialog_content(ui);
                ui.horizontal(|ui| {
                    if let Some(index) = index {
                        ui.label(format!("{} of {}", index + 1, state.screenshots.len()));
                    }
                    ui.label(
                        RichText::new(current.file_name().unwrap_or_default().to_string_lossy())
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(8.0);
                if state.screenshot_full_loading {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading image...");
                    });
                } else if let Some(texture) = &state.screenshot_full_image {
                    let size = texture.size_vec2();
                    let max = egui::vec2(
                        ctx.available_rect().width() * 0.84,
                        ctx.available_rect().height() * 0.76,
                    );
                    let scale = (max.x / size.x).min(max.y / size.y).min(1.0);
                    ui.image((texture.id(), size * scale));
                } else if !state.screenshot_full_error.is_empty() {
                    ui.colored_label(DANGER, &state.screenshot_full_error);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if let Some(index) = index.filter(|_| state.screenshots.len() > 1) {
                        if ui.button("Previous").clicked() {
                            next = Some(
                                state.screenshots[(index + state.screenshots.len() - 1)
                                    % state.screenshots.len()]
                                .path
                                .clone(),
                            );
                        }
                        if ui.button("Next").clicked() {
                            next = Some(
                                state.screenshots[(index + 1) % state.screenshots.len()]
                                    .path
                                    .clone(),
                            );
                        }
                    }
                    if ui.button("Open file").clicked() {
                        let _ = open::that(&current);
                    }
                    if ui.button("Open folder").clicked() {
                        if let Some(parent) = current.parent() {
                            let _ = open::that(parent);
                        }
                    }
                });
            });
        if close {
            state.screenshot_viewer = None;
            state.screenshot_full_image = None;
            state.screenshot_full_image = None;
        } else if let Some(path) = next {
            state.open_screenshot(&path);
        }
    }
    if let Some(current_url) = state.project_image_url.clone() {
        let gallery: Vec<String> = state
            .detail_project
            .as_ref()
            .map_or_else(Vec::new, |project| {
                project
                    .gallery
                    .iter()
                    .map(|image| image.url.clone())
                    .collect()
            });
        let current_index = gallery.iter().position(|url| url == &current_url);
        let mut next_image = None;
        let mut close = ctx.input(|input| input.key_pressed(egui::Key::Escape));
        egui::Window::new("Project images")
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(480.0)
            .max_width(ctx.available_rect().width() * 0.9)
            .show(ctx, |ui| {
                crate::ui::components::dialog_content(ui);
                ui.horizontal(|ui| {
                    if let Some(index) = current_index {
                        ui.label(format!("{} of {}", index + 1, gallery.len()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(8.0);
                if state.project_image_loading {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading image...");
                    });
                } else if let Some(texture) = &state.project_image {
                    let size = texture.size_vec2();
                    let max_width = (ctx.available_rect().width() * 0.8).min(1200.0);
                    let max_height = (ctx.available_rect().height() * 0.72).min(800.0);
                    let scale = (max_width / size.x).min(max_height / size.y).min(1.0);
                    ui.image((texture.id(), size * scale));
                } else if !state.project_image_error.is_empty() {
                    ui.colored_label(DANGER, &state.project_image_error);
                }
                if let Some(current_index) = current_index.filter(|_| gallery.len() > 1) {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Previous").clicked() {
                            next_image = Some(
                                gallery[(current_index + gallery.len() - 1) % gallery.len()]
                                    .clone(),
                            );
                        }
                        if ui.button("Next").clicked() {
                            next_image = Some(gallery[(current_index + 1) % gallery.len()].clone());
                        }
                    });
                }
            });
        if close {
            state.project_image_url = None;
            state.project_image = None;
            state.project_image_loading = false;
            state.project_image_error.clear();
        } else if let Some(url) = next_image {
            state.open_project_image(&url);
        }
    }
    if let Some((instance_id, kind, file_name, title)) = state.pending_content_delete.clone() {
        egui::Window::new("Remove installed content?")
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(420.0)
            .max_width(420.0)
            .show(ctx, |ui| {
                crate::ui::components::dialog_content(ui);
                ui.add_space(6.0);
                ui.label(RichText::new(format!("Remove {title} from this instance?")).color(TEXT));
                ui.label(RichText::new(&file_name).size(12.0).color(TEXT2));
                ui.add_space(14.0);
                ui.columns(2, |columns| {
                    let width = columns[1].available_width();
                    if columns[0]
                        .add_sized(
                            [columns[0].available_width(), 36.0],
                            egui::Button::new("Cancel"),
                        )
                        .clicked()
                    {
                        state.pending_content_delete = None;
                    }
                    if crate::ui::components::sized_danger_button(&mut columns[1], "Remove", width)
                        .clicked()
                    {
                        state.pending_content_delete = None;
                        match crate::modrinth::install::remove_project_blocking(
                            &state.instances,
                            &instance_id,
                            kind,
                            &file_name,
                        ) {
                            Ok(()) => {
                                state.refresh_library();
                                state.notify("Content removed");
                            }
                            Err(error) => state.fail(error),
                        }
                    }
                });
                ui.add_space(4.0);
            });
    }
    if !state.notice.is_empty() {
        let fresh = state
            .notice_at
            .map(|t| t.elapsed() < std::time::Duration::from_secs(4))
            .unwrap_or(false);
        if fresh {
            egui::TopBottomPanel::bottom("notice")
                .frame(
                    egui::Frame::new()
                        .fill(crate::ui::theme::palette(ctx).elevated)
                        .stroke(Stroke::new(1.0_f32, crate::ui::theme::palette(ctx).border))
                        .inner_margin(egui::Margin::same(10)),
                )
                .show(ctx, |ui| {
                    crate::ui::components::dialog_content(ui);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&state.notice).color(TEXT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Dismiss").clicked() {
                                state.notice.clear();
                                state.notice_at = None;
                            }
                        });
                    });
                });
        } else {
            state.notice.clear();
            state.notice_at = None;
        }
    }

    if !state.error_dialog.is_empty() {
        let msg = state.error_dialog.clone();
        egui::Window::new("Something went wrong")
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                crate::ui::components::dialog_content(ui);
                ui.add(
                    egui::Label::new(RichText::new(&msg).color(TEXT))
                        .selectable(true)
                        .wrap(),
                );
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if crate::ui::components::secondary_button(ui, "Copy error").clicked() {
                        ctx.copy_text(msg.clone());
                        state.notify("Error message copied to clipboard");
                    }
                    if crate::ui::components::primary_button(ui, "Close").clicked() {
                        state.error_dialog.clear();
                    }
                    if state.downloads_history.iter().any(|history| {
                        history.state == "failed" && state.retry_actions.contains_key(&history.id)
                    }) && crate::ui::components::secondary_button(ui, "Downloads & retry")
                        .clicked()
                    {
                        state.error_dialog.clear();
                        state.set_page(Page::Downloads);
                    }
                    if crate::ui::components::secondary_button(ui, "View Logs").clicked() {
                        state.error_dialog.clear();
                        state.set_page(Page::Logs);
                    }
                });
            });
    }

    if state.edit_instance.is_some() {
        let max_w = (ctx.screen_rect().width() - 40.0).clamp(360.0, 520.0);
        egui::Window::new("Edit Instance")
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(max_w)
            .min_width(max_w)
            .max_width(max_w)
            .show(ctx, |ui| {
                crate::ui::components::dialog_content(ui);
                ui.set_width(max_w);
                show_edit_dialog(state, ui);
            });
    }

    if state.crash_report.is_some() {
        show_crash_dialog(state, ctx);
    }

    if !egui::Popup::is_any_open(ctx) && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.error_dialog.clear();
        state.show_new_instance = false;
        crate::ui::pages::discover::close_detail(state);
        state.crash_report = None;
        state.edit_instance = None;
        state.pending_content_delete = None;
        state.screenshot_viewer = None;
        state.screenshot_full_image = None;
        state.project_image_url = None;
        state.project_image = None;
        state.project_image_loading = false;
        state.project_image_error.clear();
    }
}

fn show_edit_dialog(state: &mut AppState, ui: &mut egui::Ui) {
    let Some(mut cfg) = state.edit_instance.clone() else {
        return;
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
        for (tab_idx, tab_name) in [(0, "General"), (1, "Performance"), (2, "Launch options")] {
            let is_selected = state.edit_tab == tab_idx;
            if crate::ui::components::pill_tab_button(ui, tab_name, is_selected).clicked() {
                state.edit_tab = tab_idx;
            }
        }
    });

    ui.add_space(12.0);

    let max_body_h = (ui.ctx().screen_rect().height() - 200.0).clamp(160.0, 420.0);
    egui::ScrollArea::vertical()
        .max_height(max_body_h)
        .auto_shrink([false, true])
        .show(ui, |ui| match state.edit_tab {
            0 => show_edit_general(state, ui, &mut cfg),
            1 => show_edit_performance(state, ui, &mut cfg),
            _ => show_edit_launch(ui, &mut cfg),
        });

    ui.add_space(14.0);
    ui.separator();
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if crate::ui::components::secondary_button(ui, "Cancel").clicked() {
            state.edit_instance = None;
            state.edit_error.clear();
        }
        if crate::ui::components::secondary_button(ui, "Manage content").clicked() {
            state.selected_instance = Some(cfg.id.clone());
            state.edit_instance = None;
            state.refresh_library();
            state.set_page(Page::Library);
        }
        if crate::ui::components::secondary_button(ui, "Export").clicked() {
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
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if crate::ui::components::primary_button(ui, "Save changes").clicked() {
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
            if !state.edit_error.is_empty() {
                ui.label(RichText::new(&state.edit_error).size(11.5).color(DANGER));
            }
        });
    });

    if state.edit_instance.is_some() {
        state.edit_instance = Some(cfg);
    }
}

fn show_edit_general(
    state: &mut AppState,
    ui: &mut egui::Ui,
    cfg: &mut crate::instance::config::InstanceConfig,
) {
    crate::ui::components::field_label(ui, "INSTANCE NAME");
    crate::ui::components::limited_text_edit(
        ui,
        "edit-instance-name",
        &mut cfg.name,
        crate::ui::components::limits::INSTANCE_NAME,
        "Instance name",
    );
    ui.add_space(3.0);
    ui.label(
        RichText::new("This name appears in Home and Library.")
            .size(11.5)
            .color(TEXT2),
    );
    ui.add_space(14.0);

    crate::ui::components::field_label(ui, "VERSION & RUNTIME");
    let p = crate::ui::theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                crate::ui::components::badge(ui, &format!("Minecraft {}", cfg.minecraft_version));
                let loader_text = if cfg.loader_version.is_empty() {
                    cfg.loader.display_name().to_string()
                } else {
                    format!("{} {}", cfg.loader.display_name(), cfg.loader_version)
                };
                crate::ui::components::badge(ui, &loader_text);
            });

            if cfg.loader != crate::instance::config::LoaderKind::Vanilla {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let checking = state.loader_update_checking.as_deref() == Some(&cfg.id);
                    let installing = state.loader_update_busy.as_deref() == Some(&cfg.id);
                    let mut check_clicked = false;
                    ui.add_enabled_ui(!checking && !installing, |ui| {
                        if crate::ui::components::secondary_button(ui, "Check for loader updates")
                            .clicked()
                        {
                            check_clicked = true;
                        }
                    });
                    if check_clicked {
                        state.loader_update_checking = Some(cfg.id.clone());
                        state.loader_update_candidate = None;
                        state.loader_update_error.clear();
                        crate::app::tasks::check_loader_update(state, cfg.id.clone());
                    }
                    if checking || installing {
                        ui.spinner();
                        ui.label(
                            RichText::new(if checking {
                                "Checking..."
                            } else {
                                "Installing..."
                            })
                            .size(11.5)
                            .color(TEXT2),
                        );
                    }
                    if let Some((id, version)) = state.loader_update_candidate.clone() {
                        if id == cfg.id {
                            let mut install_clicked = false;
                            ui.add_enabled_ui(!installing, |ui| {
                                if crate::ui::components::primary_button(
                                    ui,
                                    &format!("Install {version}"),
                                )
                                .clicked()
                                {
                                    install_clicked = true;
                                }
                            });
                            if install_clicked {
                                state.loader_update_busy = Some(cfg.id.clone());
                                state.loader_update_error.clear();
                                crate::app::tasks::install_loader_update(
                                    state,
                                    cfg.id.clone(),
                                    version,
                                );
                            }
                        }
                    }
                });
                if !state.loader_update_error.is_empty() {
                    ui.add_space(4.0);
                    ui.colored_label(DANGER, &state.loader_update_error);
                }
            }
        });

    ui.add_space(14.0);
    crate::ui::components::field_label(ui, "QUICK ACTIONS");
    ui.horizontal(|ui| {
        if crate::ui::components::secondary_button(ui, "Open game folder").clicked() {
            let _ = open::that(state.instances.game_dir(&cfg.id));
        }
        if crate::ui::components::secondary_button(ui, "Manage mods & files").clicked() {
            state.selected_instance = Some(cfg.id.clone());
            state.edit_instance = None;
            state.refresh_library();
            state.set_page(Page::Library);
        }
    });
}

fn show_edit_performance(
    state: &mut AppState,
    ui: &mut egui::Ui,
    cfg: &mut crate::instance::config::InstanceConfig,
) {
    crate::ui::components::field_label(ui, "MEMORY ALLOCATION");
    let mut boost_val = cfg.boost_mode.unwrap_or(state.config.boost_mode);
    if ui
        .checkbox(&mut boost_val, "Eco mode (lower game memory limit)")
        .changed()
    {
        cfg.boost_mode = Some(boost_val);
    }
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for (label, value) in [
            ("Minimum", &mut cfg.memory_min_mb),
            ("Maximum", &mut cfg.memory_max_mb),
        ] {
            ui.label(RichText::new(label).size(12.0).color(TEXT2));
            let mut gb = *value as f64 / 1024.0;
            if ui
                .add(
                    egui::DragValue::new(&mut gb)
                        .range(0.5..=128.0)
                        .speed(0.25)
                        .suffix(" GB"),
                )
                .changed()
            {
                *value = (gb * 1024.0).round() as u64;
            }
        }
    });
    ui.add_space(3.0);
    ui.label(
        RichText::new("Allocated memory dynamically scales up to the maximum during gameplay.")
            .size(11.0)
            .color(TEXT2),
    );

    ui.add_space(14.0);
    crate::ui::components::field_label(ui, "JAVA RUNTIME");
    ui.label(
        RichText::new("Choose the Java installation used to launch Minecraft.")
            .size(11.5)
            .color(TEXT2),
    );
    ui.add_space(4.0);
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
            crate::ui::components::limited_text_edit(
                ui,
                "edit-instance-java-path",
                &mut cfg.java_path,
                crate::ui::components::limits::PATH,
                "C:\\Program Files\\Java\\bin\\javaw.exe",
            );
            if crate::ui::components::secondary_button(ui, "Browse").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_file() {
                    cfg.java_path = p.display().to_string();
                }
            }
        }
    });

    ui.add_space(14.0);
    crate::ui::components::field_label(ui, "JVM PRESETS");
    ui.label(
        RichText::new("Apply garbage collector optimizations suited for your hardware.")
            .size(11.5)
            .color(TEXT2),
    );
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for preset in [
            crate::config::JvmPreset::Aikar,
            crate::config::JvmPreset::Shenandoah,
            crate::config::JvmPreset::GenerationalZgc,
            crate::config::JvmPreset::HighThroughput,
            crate::config::JvmPreset::LowMemory,
        ] {
            let is_active = cfg.jvm_args == preset.flags();
            if crate::ui::components::pill_tab_button(ui, preset.short_name(), is_active)
                .on_hover_text(preset.label())
                .clicked()
            {
                cfg.jvm_args = preset.flags().to_string();
            }
        }
        if !cfg.jvm_args.is_empty() && ui.button("Clear").clicked() {
            cfg.jvm_args.clear();
        }
    });

    ui.add_space(6.0);
    crate::ui::components::limited_text_edit_with_hint(
        ui,
        "edit-instance-jvm",
        &mut cfg.jvm_args,
        crate::ui::components::limits::JVM_ARGS,
        "-Xmx4G",
        "Passed to the JVM before the game class.",
    );
}

fn show_edit_launch(ui: &mut egui::Ui, cfg: &mut crate::instance::config::InstanceConfig) {
    crate::ui::components::field_label(ui, "WINDOW SIZE");
    ui.label(
        RichText::new("Initial window resolution when Minecraft starts.")
            .size(11.5)
            .color(TEXT2),
    );
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let mut w = cfg.width.map(|v| v.to_string()).unwrap_or_default();
        let mut h = cfg.height.map(|v| v.to_string()).unwrap_or_default();
        ui.label(RichText::new("Width:").size(12.0).color(TEXT2));
        if ui
            .add(
                egui::TextEdit::singleline(&mut w)
                    .margin(egui::vec2(10.0, 8.0))
                    .min_size(egui::vec2(0.0, 34.0))
                    .desired_width(75.0)
                    .hint_text("Default"),
            )
            .changed()
        {
            cfg.width = w.parse().ok();
        }
        ui.add_space(14.0);
        ui.label(RichText::new("Height:").size(12.0).color(TEXT2));
        if ui
            .add(
                egui::TextEdit::singleline(&mut h)
                    .margin(egui::vec2(10.0, 8.0))
                    .min_size(egui::vec2(0.0, 34.0))
                    .desired_width(75.0)
                    .hint_text("Default"),
            )
            .changed()
        {
            cfg.height = h.parse().ok();
        }
    });
    ui.add_space(6.0);
    ui.checkbox(&mut cfg.fullscreen, "Start in fullscreen mode");

    ui.add_space(14.0);
    crate::ui::components::field_label(ui, "GAME ARGUMENTS");
    ui.label(
        RichText::new("Custom command line arguments passed directly to Minecraft.")
            .size(11.5)
            .color(TEXT2),
    );
    ui.add_space(4.0);
    crate::ui::components::limited_text_edit_with_hint(
        ui,
        "edit-instance-game-args",
        &mut cfg.game_args,
        crate::ui::components::limits::GAME_ARGS,
        "--username Steve",
        "Appended to the game command line.",
    );
}

fn page_icon(page: Page) -> &'static str {
    match page {
        Page::Onboarding => "✨",
        Page::Home => "🏠",
        Page::Instances => "📦",
        Page::Worlds => "📁",
        Page::Discover => "🧭",
        Page::Library => "🔖",
        Page::Screenshots => "🖼",
        Page::Downloads => "📥",
        Page::Nexeu => "🔗",
        Page::Accounts => "👤",
        Page::Settings => "⚙",
        Page::Logs => "📄",
    }
}

fn sidebar_item(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    page: Page,
    selected: bool,
    has_badge: bool,
) -> egui::Response {
    let label = page.label();
    let icon = page_icon(page);
    let id = ui.make_persistent_id(format!("sidebar_item_{label}"));
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::click());

    let fade = ctx.animate_bool_with_time(id.with("hover"), response.hovered(), 0.15);
    if fade > 0.001 && fade < 0.999 {
        ctx.request_repaint();
    }
    let p = crate::ui::theme::palette(ctx);
    let fill = if selected {
        p.accent
    } else if fade > 0.01 {
        p.elevated.lerp_to_gamma(p.hover, fade)
    } else {
        Color32::TRANSPARENT
    };

    if fill != Color32::TRANSPARENT {
        let item_corner = CornerRadius::same(8);
        ui.painter().rect_filled(rect, item_corner, fill);
    }

    let text_color = if selected {
        p.accent_text
    } else {
        TEXT2.lerp_to_gamma(TEXT, fade)
    };

    ui.painter().text(
        egui::pos2(rect.min.x + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(12.5),
        text_color,
    );

    ui.painter().text(
        egui::pos2(rect.min.x + 32.0, rect.center().y),
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
        ui.painter().text(
            egui::pos2(rect.max.x - 12.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "\u{2022}",
            egui::FontId::proportional(14.0),
            ACCENT,
        );
    }

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    crate::ui::components::focus_ring(ui, &response, label);
    response.on_hover_text(page_tooltip(page))
}

fn page_tooltip(page: Page) -> &'static str {
    match page {
        Page::Home => "Home · Your selected instance, quick actions, and recent activity",
        Page::Instances => "Instances · Create, configure, and manage Minecraft installations",
        Page::Worlds => "Worlds & Files · Browse world saves, resource packs, and instance files",
        Page::Discover => "Discover · Search and install mods, modpacks, and shaders",
        Page::Library => "Library · Installed mods and content across your instances",
        Page::Screenshots => "Screenshots · View, organize, and export in-game screenshots",
        Page::Downloads => "Downloads · Active game and mod downloads",
        Page::Nexeu => "Nexeu Servers · Partner Minecraft community servers",
        Page::Accounts => "Accounts · Switch player accounts or manage offline mode",
        Page::Settings => "Settings · Launcher preferences, Java runtimes, and themes",
        Page::Logs => "Logs · Launcher and game crash diagnostic logs",
        Page::Onboarding => "Setup",
    }
}
fn status_card(state: &mut AppState, ui: &mut egui::Ui, theme: crate::ui::theme::Palette) {
    let op_phase = state.operations.values().next().map(|o| o.phase.clone());
    if state.global_status.is_empty() && state.global_frac.is_none() && op_phase.is_none() {
        return;
    }
    let text = op_phase.unwrap_or_else(|| state.global_status.clone());
    let frac = state.global_frac;
    let card_corner = CornerRadius::same(metrics::CARD_RADIUS);
    egui::Frame::new()
        .fill(theme.elevated2)
        .stroke(Stroke::new(1.0_f32, theme.border))
        .corner_radius(card_corner)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(&text).size(type_scale::CAPTION).color(TEXT));
            crate::ui::components::progress_row(
                ui,
                frac,
                Some(crate::ui::components::ProgressDetail::default()),
            );
            if crate::ui::components::button(ui, "Downloads", crate::ui::components::Tone::Ghost)
                .clicked()
            {
                state.set_page(Page::Downloads);
            }
        });
}

struct SidebarAccount {
    username: String,
    uuid: uuid::Uuid,
    offline: bool,
}

fn sidebar_footer(state: &mut AppState, ui: &mut egui::Ui) {
    let account = state
        .config
        .active_account_ref()
        .map(|account| SidebarAccount {
            username: account.username().to_string(),
            uuid: account.uuid(),
            offline: account.is_offline(),
        });
    let has_update = state.launcher_update.as_ref().is_some_and(|u| u.has_update);
    let version = env!("CARGO_PKG_VERSION").to_string();
    let theme = crate::ui::theme::palette(ui.ctx());

    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        if let Some(acc) = account {
            let card_id = ui.make_persistent_id("bottom_profile_card");
            let hovered = ui
                .ctx()
                .data(|d| d.get_temp::<bool>(card_id).unwrap_or(false));
            let fade = ui
                .ctx()
                .animate_bool_with_time(card_id.with("hover"), hovered, 0.15);
            if fade > 0.001 && fade < 0.999 {
                ui.ctx().request_repaint();
            }
            let fill = theme.elevated2.lerp_to_gamma(theme.hover, fade);
            let border_color = theme.border;
            let card_corner = CornerRadius::same(metrics::CARD_RADIUS);

            let frame_resp = egui::Frame::new()
                .fill(fill)
                .stroke(Stroke::new(1.0_f32, border_color))
                .corner_radius(card_corner)
                .inner_margin(egui::Margin::symmetric(10, 7))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (avatar, _) =
                            ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                        if !acc.offline {
                            crate::ui::components::draw_avatar(
                                ui.painter(),
                                avatar,
                                acc.username.as_str(),
                                &acc.uuid.to_string(),
                            );
                        } else {
                            crate::ui::components::draw_cute_avatar(
                                ui.painter(),
                                avatar,
                                acc.username.as_str(),
                                false,
                            );
                        }
                        ui.add_space(6.0);

                        let badge_width = 50.0_f32;
                        let name_width = (ui.available_width() - badge_width).max(24.0);
                        let (name_rect, _) = ui.allocate_exact_size(
                            egui::vec2(name_width, 22.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().with_clip_rect(name_rect).text(
                            name_rect.left_center(),
                            egui::Align2::LEFT_CENTER,
                            crate::ui::components::elide(acc.username.as_str(), 11),
                            egui::FontId::new(type_scale::LABEL, egui::FontFamily::Proportional),
                            TEXT,
                        );
                        let (badge, _) = ui.allocate_exact_size(
                            egui::vec2(badge_width, 22.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().text(
                            badge.right_center(),
                            egui::Align2::RIGHT_CENTER,
                            if acc.offline {
                                "Offline ▾"
                            } else {
                                "Microsoft ▾"
                            },
                            egui::FontId::proportional(type_scale::MICRO),
                            if acc.offline {
                                MUTED
                            } else {
                                Color32::from_rgb(100, 200, 255)
                            },
                        );
                    });
                });

            let resp = ui.interact(frame_resp.response.rect, card_id, egui::Sense::click());
            ui.ctx()
                .data_mut(|d| d.insert_temp(card_id, resp.hovered()));
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            crate::ui::components::focus_ring(ui, &resp, "Manage active account");
            if resp
                .on_hover_text(format!(
                    "Active account: {} ({})\nClick to manage in Accounts",
                    acc.username.as_str(),
                    if acc.offline { "Offline" } else { "Microsoft" }
                ))
                .clicked()
            {
                state.set_page(Page::Accounts);
            }
        } else if crate::ui::components::button(
            ui,
            "Configure account",
            crate::ui::components::Tone::Secondary,
        )
        .on_hover_text("Set an offline username to get started")
        .clicked()
        {
            state.set_page(Page::Accounts);
        }

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("v{version}"))
                    .size(type_scale::MICRO)
                    .color(MUTED),
            );
            let (theme_icon, theme_label) = match state.config.theme {
                crate::config::ThemeKind::Monochrome => ("◐", "Mono"),
                crate::config::ThemeKind::Gloss => ("✨", "Gloss"),
                crate::config::ThemeKind::Halloween => ("🎃", "Spooky"),
                crate::config::ThemeKind::SoftPink => ("🌸", "Pink"),
                crate::config::ThemeKind::SoftBrown => ("🍂", "Brown"),
            };
            let icon_color = if state.config.theme == crate::config::ThemeKind::Halloween {
                Color32::from_rgb(255, 125, 20)
            } else {
                TEXT2
            };
            let theme_btn_size = egui::vec2(66.0, 20.0);
            let (theme_rect, theme_click) =
                ui.allocate_exact_size(theme_btn_size, egui::Sense::click());
            let theme_hovered = theme_click.hovered();
            if theme_hovered {
                let toggle_corner = CornerRadius::same(4);
                ui.painter()
                    .rect_filled(theme_rect, toggle_corner, theme.hover);
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            ui.painter().text(
                egui::pos2(theme_rect.left() + 4.0, theme_rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{theme_icon} {theme_label}"),
                egui::FontId::proportional(type_scale::MICRO),
                if theme_hovered {
                    Color32::WHITE
                } else {
                    icon_color
                },
            );
            crate::ui::components::focus_ring(ui, &theme_click, "Choose theme");
            if theme_click
                .on_hover_text(format!(
                    "Theme: {}. Open appearance settings.",
                    state.config.theme.label()
                ))
                .clicked()
            {
                crate::ui::pages::settings::show_appearance(ui.ctx());
                state.set_page(Page::Settings);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if has_update
                    && crate::ui::components::button(
                        ui,
                        "Update",
                        crate::ui::components::Tone::Ghost,
                    )
                    .on_hover_text("A newer MONORYX is available.")
                    .clicked()
                {
                    state.set_page(Page::Settings);
                }
                let search_btn_size = egui::vec2(54.0, 20.0);
                let (search_rect, search_click) =
                    ui.allocate_exact_size(search_btn_size, egui::Sense::click());
                let search_hovered = search_click.hovered();
                let pill_corner = CornerRadius::same(metrics::CONTROL_RADIUS);
                let search_bg = if search_hovered {
                    theme.hover
                } else {
                    theme.elevated2
                };
                ui.painter().rect(
                    search_rect,
                    pill_corner,
                    search_bg,
                    Stroke::new(
                        1.0_f32,
                        if search_hovered {
                            theme.border.lerp_to_gamma(TEXT, 0.25)
                        } else {
                            theme.border
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                if search_hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                ui.painter().text(
                    search_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Ctrl+K",
                    egui::FontId::proportional(type_scale::MICRO),
                    if search_hovered {
                        Color32::WHITE
                    } else {
                        TEXT2
                    },
                );
                crate::ui::components::focus_ring(ui, &search_click, "Command palette");
                if search_click
                    .on_hover_text("Open command palette (Ctrl+K)")
                    .clicked()
                {
                    state.command_palette_open = true;
                }
            });
        });
    });
}

fn show_crash_dialog(state: &mut AppState, ctx: &egui::Context) {
    use crate::minecraft::diagnostics::NextStep;
    use crate::ui::components::{card_frame, primary_button, secondary_button};
    let Some(info) = state.crash_report.clone() else {
        return;
    };
    let mut open = true;
    let mut close = false;
    let mut next = None;
    let screen = ctx.input(|input| input.screen_rect());
    egui::Area::new(egui::Id::new("crash_modal_backdrop"))
        .fixed_pos(screen.min)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            crate::ui::components::dialog_content(ui);
            ui.allocate_rect(screen, egui::Sense::click());
            ui.painter()
                .rect_filled(screen, 0, Color32::from_black_alpha(150));
        });
    egui::Window::new("Let's get you back in game")
        .order(egui::Order::Foreground)
        .id(egui::Id::new("friendly-crash-report"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width((screen.width() * 0.8).clamp(600.0, 1020.0))
        .max_size(screen.size() - egui::vec2(28.0, 28.0))
        .vscroll(true)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            crate::ui::components::dialog_content(ui);
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&info.instance_name).strong().color(TEXT2));
                crate::ui::components::badge(
                    ui,
                    &format!(
                        "Exit {}",
                        crate::minecraft::crash::format_exit_code(info.exit_code)
                    ),
                );
            });
            ui.label(
                RichText::new(&info.advice.title)
                    .size(24.0)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new(&info.advice.explanation)
                    .size(14.0)
                    .color(TEXT2),
            );
            ui.add_space(12.0);
            card_frame(ui, |ui| {
                ui.label(RichText::new("What to try").size(16.0).strong().color(TEXT));
                for (index, step) in info.advice.steps.iter().enumerate() {
                    ui.label(
                        RichText::new(format!("{}. {step}", index + 1))
                            .size(13.0)
                            .color(TEXT),
                    );
                }
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    let label = match info.advice.next {
                        NextStep::Mods => "Review mods",
                        NextStep::Settings => "Open instance settings",
                        NextStep::Repair => "Repair game files",
                        NextStep::Logs => "View full logs",
                    };
                    if primary_button(ui, label).clicked() {
                        next = Some(info.advice.next);
                    }
                    if secondary_button(ui, "Copy report")
                        .on_hover_text("Copy the explanation and log to your clipboard.")
                        .clicked()
                    {
                        ctx.copy_text(format!(
                            "MONORYX crash report\nInstance: {}\nExit: {}\n{}\n{}\n\n{}\n\n{}",
                            info.instance_name,
                            info.exit_code,
                            info.advice.title,
                            info.advice.explanation,
                            info.summary,
                            info.details
                        ));
                        state.notify("Report copied. You can paste it when asking for help.");
                    }
                    if secondary_button(ui, "Close").clicked() {
                        close = true;
                    }
                });
            });
            if !info.advice.evidence.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("From the game log")
                        .size(12.0)
                        .strong()
                        .color(TEXT2),
                );
                for evidence in &info.advice.evidence {
                    ui.add(
                        egui::Label::new(RichText::new(evidence).size(12.0).color(TEXT2))
                            .wrap()
                            .selectable(true),
                    );
                }
            }
            ui.add_space(10.0);
            egui::CollapsingHeader::new("Technical details")
                .id_salt("crash-technical")
                .show(ui, |ui| {
                    ui.label(RichText::new(&info.source_label).size(11.5).color(MUTED));
                    ui.label(RichText::new(&info.summary).color(TEXT2));
                    egui::ScrollArea::both()
                        .id_salt("crash-log-text")
                        .max_height((screen.height() * 0.36).max(160.0))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&info.details)
                                        .monospace()
                                        .size(12.0)
                                        .color(TEXT),
                                )
                                .selectable(true),
                            );
                        });
                    ui.horizontal_wrapped(|ui| {
                        if secondary_button(ui, "View full logs").clicked() {
                            next = Some(NextStep::Logs);
                        }
                        if secondary_button(ui, "Open reports folder").clicked() {
                            let _ = std::fs::create_dir_all(&info.crash_reports_dir);
                            let _ = open::that(&info.crash_reports_dir);
                        }
                    });
                });
            egui::CollapsingHeader::new("Share a link for help")
                .id_salt("crash-sharing")
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(
                            "This uploads the log to mclo.gs. Anyone with the link can read it.",
                        )
                        .size(12.0)
                        .color(TEXT2),
                    );
                    ui.add_enabled_ui(!state.crash_share_loading, |ui| {
                        if secondary_button(ui, "Upload to mclo.gs").clicked() {
                            state.share_crash_log();
                        }
                    });
                    if state.crash_share_loading {
                        ui.spinner();
                    }
                    if let Some(url) = state.crash_share_url.clone() {
                        ui.hyperlink_to(&url, &url);
                        if secondary_button(ui, "Copy link").clicked() {
                            ctx.copy_text(url);
                            state.notify("Link copied");
                        }
                    }
                    if !state.crash_share_error.is_empty() {
                        ui.colored_label(DANGER, &state.crash_share_error);
                    }
                });
            ui.add_space(6.0);
        });
    if let Some(next) = next {
        state.selected_instance = Some(info.instance_id.clone());
        state.save_config();
        state.refresh_library();
        match next {
            NextStep::Mods => state.set_page(Page::Library),
            NextStep::Settings => {
                state.edit_instance = state.selected();
            }
            NextStep::Logs => state.set_page(Page::Logs),
            NextStep::Repair => crate::app::tasks::repair_instance(state, info.instance_id),
        }
        close = true;
    }
    if !open || close || ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        state.crash_report = None;
    }
}
