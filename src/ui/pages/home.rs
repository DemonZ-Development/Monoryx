use crate::app::{events::Page, state::AppState};
use crate::instance::InstanceConfig;
use crate::ui::components::{
    action_button, badge, button, card_frame, compact_search_field, draw_instance_banner_fallback,
    format_last_played, hero_card_frame, page_header, primary_button, render_instance_thumbnail,
    stat, thin_progress, Tone,
};
use crate::ui::theme::{metrics, type_scale, MUTED, TEXT, TEXT2};
use egui::{Color32, CornerRadius, RichText, Stroke};

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Home", "Pick an instance and jump back in.");
    if !state.last_exit.is_empty() {
        card_frame(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("The last game session ended unexpectedly.")
                        .color(crate::ui::theme::WARNING),
                );
                if action_button(ui, "View logs").clicked() {
                    state.set_page(Page::Logs);
                }
                if action_button(ui, "Dismiss").clicked() {
                    state.last_exit.clear();
                }
            });
        });
        ui.add_space(10.0);
    }
    let Some(cfg) = state.selected() else {
        hero_card_frame(ui, |ui| {
            ui.label(
                RichText::new("Let's set up your first game")
                    .size(22.0)
                    .color(TEXT),
            );
            ui.label(
                RichText::new(
                    "An instance keeps a Minecraft version, its worlds, and its mods together.",
                )
                .color(TEXT2),
            );
            ui.add_space(12.0);
            if primary_button(ui, "Create an instance").clicked() {
                crate::ui::pages::instances::open_new_dialog(state);
                state.set_page(Page::Instances);
            }
        });
        return;
    };
    if ui.available_width() >= 900.0 {
        let total_w = ui.available_width();
        let spacing = 12.0_f32;
        let left_w = ((total_w - spacing) * 0.54).clamp(380.0, 520.0);
        let right_w = total_w - left_w - spacing;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_max_width(left_w);
                    selected_instance(state, &cfg, ui);
                },
            );
            ui.add_space(spacing);
            ui.allocate_ui_with_layout(
                egui::vec2(right_w, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_max_width(right_w);
                    screenshots(state, &cfg, ui);
                },
            );
        });
    } else {
        selected_instance(state, &cfg, ui);
        ui.add_space(12.0);
        instance_list(state, ui);
        ui.add_space(18.0);
        screenshots(state, &cfg, ui);
        return;
    }
    ui.add_space(18.0);
    instance_list(state, ui);
}

fn selected_instance(state: &mut AppState, cfg: &InstanceConfig, ui: &mut egui::Ui) {
    let running = state.playing.get(&cfg.id).copied().unwrap_or(false);
    let installing = state.busy_install.contains_key(&cfg.id);
    let eco = cfg.boost_mode.unwrap_or(state.config.boost_mode);
    hero_card_frame(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.horizontal(|ui| {
            render_instance_thumbnail(ui, 46.0, &cfg.name, cfg.loader.display_name(), false);
            ui.add_space(8.0);
            let identity_width = (ui.available_width() - 86.0).max(120.0);
            ui.allocate_ui_with_layout(
                egui::vec2(identity_width, 46.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_width(identity_width);
                    ui.add(
                        egui::Label::new(RichText::new(&cfg.name).size(20.0).strong().color(TEXT))
                            .truncate(),
                    )
                    .on_hover_text(&cfg.name);
                    ui.label(
                        RichText::new(format!(
                            "Minecraft {} · {}",
                            cfg.minecraft_version,
                            cfg.loader.display_name()
                        ))
                        .size(11.5)
                        .color(TEXT2),
                    );
                },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let menu_resp = ui.menu_button("More", |ui| {
                    if action_button(ui, "Open game folder").clicked() {
                        let _ = open::that(state.instances.game_dir(&cfg.id));
                        ui.close();
                    }
                    if action_button(ui, "View logs").clicked() {
                        state.set_page(Page::Logs);
                        ui.close();
                    }
                    ui.separator();
                    ui.add_enabled_ui(!running && !installing, |ui| {
                        if action_button(ui, "Repair game files")
                            .on_hover_text("Check and re-download missing or damaged game files.")
                            .clicked()
                        {
                            crate::app::tasks::repair_instance(state, cfg.id.clone());
                            ui.close();
                        }
                    });
                    if action_button(
                        ui,
                        if eco {
                            "Turn off Eco mode"
                        } else {
                            "Turn on Eco mode"
                        },
                    )
                    .on_hover_text(
                        "Eco mode uses less memory and may lower FPS in demanding worlds.",
                    )
                    .clicked()
                    {
                        state.toggle_boost();
                        ui.close();
                    }
                });
                menu_resp.response.on_hover_text("More instance options");
                if running || installing {
                    ui.add_space(6.0);
                    let (status_text, dot_color) = if running {
                        ("• Running", Color32::from_rgb(34, 197, 94))
                    } else {
                        ("• Preparing", Color32::from_rgb(251, 146, 60))
                    };
                    ui.label(RichText::new(status_text).size(12.0).color(dot_color));
                }
            });
        });
        ui.add_space(6.0);
        let available_updates = state.instance_update_count();
        if state.updates_loading && state.updates_instance.as_deref() == Some(&cfg.id) {
            crate::ui::components::activity_indicator(ui, "Checking compatible updates…");
        }
        if available_updates > 0 {
            ui.horizontal_wrapped(|ui| {
                crate::ui::components::badge_accent(
                    ui,
                    &format!("{available_updates} compatible update(s)"),
                );
                if action_button(ui, "Review updates").clicked() {
                    state.set_page(Page::Library);
                }
            });
        }
        let default_ram = ui.ctx().data_mut(|data| {
            *data.get_temp_mut_or_insert_with(egui::Id::new("home-default-memory"), || {
                (
                    crate::utils::system::default_max_memory_mb(),
                    crate::utils::system::default_boost_max_memory_mb(),
                )
            })
        });
        let ram = crate::utils::system::game_memory_limit_mb(
            cfg.memory_max_mb,
            eco,
            default_ram.0,
            default_ram.1,
        );
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 20.0;
            stat(
                ui,
                "Mods",
                &state
                    .mod_counts
                    .get(&cfg.id)
                    .copied()
                    .unwrap_or(0)
                    .to_string(),
            );
            stat(ui, "Memory", &format!("{:.1} GB", ram as f64 / 1024.0));
            stat(
                ui,
                "Last played",
                &format_last_played(cfg.last_played_at.as_deref()),
            );
            stat(
                ui,
                "Playtime",
                &crate::ui::theme::format_playtime(cfg.play_time_secs),
            );
        });
        if crate::utils::system::eco_memory_limited(cfg.memory_max_mb, eco, default_ram.1) {
            ui.label(
                RichText::new(format!(
                    "Eco mode is capping this launch to {:.1} GB, below the {} set on the \
                     instance.",
                    ram as f64 / 1024.0,
                    cfg.memory_max_mb as f64 / 1024.0
                ))
                .size(11.0)
                .color(crate::ui::theme::MUTED),
            );
        }
        ui.add_space(8.0);
        ui.add_enabled_ui(!running && !installing, |ui| {
            let play_text = if running {
                "Game is running"
            } else if installing {
                "Getting things ready…"
            } else {
                "▶ Play Minecraft"
            };
            let play_btn = primary_button(ui, play_text);
            if play_btn
                .on_hover_text(if running {
                    "Game is currently running"
                } else {
                    "Play Minecraft"
                })
                .clicked()
            {
                crate::app::tasks::play_instance(state, cfg.id.clone());
            }
        });
        if let Some((phase, completed, total)) = state.busy_install.get(&cfg.id) {
            thin_progress(ui, (*total > 1).then(|| *completed as f32 / *total as f32));
            ui.label(
                RichText::new(if *total > 1 {
                    format!("{phase} · {completed}/{total}")
                } else {
                    phase.clone()
                })
                .size(12.0)
                .color(TEXT2),
            );
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let spacing = ui.spacing().item_spacing.x;
            let item_w = ((ui.available_width() - spacing * 2.0) / 3.0).max(40.0);
            let btn_size = egui::vec2(item_w, metrics::BUTTON_H);
            if action_card_button(ui, "Mods & packs", btn_size)
                .on_hover_text("Manage mods, resource packs, and shaders")
                .clicked()
            {
                state.set_page(Page::Library);
            }
            if action_card_button(ui, "Worlds & backups", btn_size)
                .on_hover_text("View and back up world saves")
                .clicked()
            {
                state.set_page(Page::Worlds);
            }
            if action_card_button(ui, "Instance settings", btn_size)
                .on_hover_text("Configure instance settings and Java memory")
                .clicked()
            {
                state.edit_instance = Some(cfg.clone());
            }
        });
    });
}

fn action_card_button(ui: &mut egui::Ui, text: &str, size: egui::Vec2) -> egui::Response {
    let p = crate::ui::theme::palette(ui.ctx());
    let corner = CornerRadius::same(metrics::CONTROL_RADIUS);
    ui.scope(|ui| {
        let widgets = &mut ui.style_mut().visuals.widgets;
        widgets.inactive.bg_fill = p.elevated2;
        widgets.hovered.bg_fill = p.hover;
        widgets.active.bg_fill = p.hover;
        widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);
        widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, Color32::WHITE);
        widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);
        widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, p.border);
        widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, p.border.lerp_to_gamma(TEXT, 0.3));
        widgets.inactive.corner_radius = corner;
        widgets.hovered.corner_radius = corner;
        ui.style_mut().spacing.button_padding = egui::vec2(6.0, 4.0);
        ui.add(
            egui::Button::new(RichText::new(text).size(type_scale::CAPTION))
                .min_size(size)
                .fill(p.elevated2)
                .corner_radius(corner),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}

fn instance_list(state: &mut AppState, ui: &mut egui::Ui) {
    let search_q = state.home_instance_search.trim().to_lowercase();
    let filtered: Vec<_> = state
        .instance_list
        .iter()
        .filter(|inst| {
            if search_q.is_empty() {
                true
            } else {
                inst.name.to_lowercase().contains(&search_q)
                    || inst.minecraft_version.to_lowercase().contains(&search_q)
                    || inst
                        .loader
                        .display_name()
                        .to_lowercase()
                        .contains(&search_q)
            }
        })
        .cloned()
        .collect();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Your instances")
                .size(17.0)
                .strong()
                .color(TEXT),
        );
        badge(ui, &filtered.len().to_string());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if button(ui, "+ New instance", Tone::Primary).clicked() {
                crate::ui::pages::instances::open_new_dialog(state);
                state.set_page(Page::Instances);
            }
        });
    });
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2((ui.available_width() - 88.0).min(280.0), 32.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                compact_search_field(
                    ui,
                    "home-instances-search",
                    &mut state.home_instance_search,
                    48,
                    "Search instances…",
                );
            },
        );
        if view_toggle_btn(ui, !state.home_grid_view, "☰", "List view").clicked() {
            state.home_grid_view = false;
        }
        if view_toggle_btn(ui, state.home_grid_view, "⊞", "Grid view").clicked() {
            state.home_grid_view = true;
        }
    });
    ui.add_space(8.0);

    if filtered.is_empty() {
        ui.add_space(16.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(format!(
                    "No instances match \"{}\"",
                    state.home_instance_search
                ))
                .color(TEXT2),
            );
            if action_button(ui, "Clear search")
                .on_hover_text("Clear search query")
                .clicked()
            {
                state.home_instance_search.clear();
            }
        });
        return;
    }

    if state.home_grid_view {
        let count = if ui.available_width() >= 1150.0 {
            3
        } else if ui.available_width() >= 750.0 {
            2
        } else {
            1
        };
        egui::ScrollArea::vertical()
            .id_salt("home-instances-grid")
            .max_height(280.0)
            .show(ui, |ui| {
                for chunk in filtered.chunks(count) {
                    ui.columns(count, |columns| {
                        for (instance, col_ui) in chunk.iter().zip(columns.iter_mut()) {
                            let is_selected =
                                state.selected_instance.as_deref() == Some(&instance.id);
                            render_instance_banner_card(state, col_ui, instance, is_selected);
                        }
                    });
                    ui.add_space(8.0);
                }
            });
    } else {
        egui::ScrollArea::vertical()
            .id_salt("home-instance-list")
            .max_height(260.0)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                for instance in &filtered {
                    let is_selected = state.selected_instance.as_deref() == Some(&instance.id);
                    render_instance_list_row(state, ui, instance, is_selected);
                }
            });
    }
}

fn render_instance_banner_card(
    state: &mut AppState,
    ui: &mut egui::Ui,
    instance: &InstanceConfig,
    is_selected: bool,
) {
    let p = crate::ui::theme::palette(ui.ctx());
    let card_height = 96.0_f32;
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), card_height),
        egui::Sense::click(),
    );

    let theme = crate::ui::theme::current_theme(ui.ctx());
    let fill = if resp.hovered() {
        p.elevated.lerp_to_gamma(p.hover, 0.5)
    } else {
        p.elevated
    };
    let border_stroke = Stroke::new(
        if is_selected { 1.5_f32 } else { 1.0_f32 },
        if is_selected { p.accent } else { p.border },
    );

    let card_corner = CornerRadius::same(metrics::CARD_RADIUS);
    let pill_corner = CornerRadius::same(metrics::PILL_RADIUS);
    let btn_corner = CornerRadius::same(metrics::CONTROL_RADIUS);

    ui.painter().rect_filled(rect, card_corner, fill);

    let banner_width = (rect.width() * 0.32).clamp(90.0, 150.0);
    let banner_rect = egui::Rect::from_min_max(
        rect.left_top(),
        egui::pos2(rect.left() + banner_width, rect.bottom()),
    );

    let banner_painter = ui.painter().with_clip_rect(rect);
    let shot_path = state
        .screenshots
        .iter()
        .find(|s| s.instance_id == instance.id)
        .map(|s| s.path.clone());
    let mut banner_drawn = false;
    if let Some(path) = shot_path {
        state.ensure_screenshot_thumb(&path);
        if let Some(tex) = state.screenshot_thumbnails.get(&path) {
            banner_painter.image(
                tex.id(),
                banner_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            banner_drawn = true;
        }
    }
    if !banner_drawn {
        draw_instance_banner_fallback(
            &banner_painter,
            banner_rect,
            &instance.name,
            instance.loader.display_name(),
        );
    }
    banner_painter.line_segment(
        [banner_rect.right_top(), banner_rect.right_bottom()],
        Stroke::new(1.0_f32, p.border),
    );

    ui.painter()
        .rect_stroke(rect, card_corner, border_stroke, egui::StrokeKind::Inside);

    if is_selected {
        let badge_text = if theme == crate::config::ThemeKind::Halloween {
            "✓ Selected 🎃"
        } else {
            "✓ Selected"
        };
        let badge_w = if theme == crate::config::ThemeKind::Halloween {
            96.0
        } else {
            76.0
        };
        let badge_rect = egui::Rect::from_min_size(
            banner_rect.left_top() + egui::vec2(6.0, 6.0),
            egui::vec2(badge_w, 20.0),
        );
        ui.painter()
            .rect_filled(badge_rect, pill_corner, Color32::from_black_alpha(200));
        ui.painter().text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            badge_text,
            egui::FontId::proportional(type_scale::MICRO),
            p.accent,
        );
    }

    let play_btn_size = 32.0_f32;
    let play_btn_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 24.0, rect.center().y),
        egui::vec2(play_btn_size, play_btn_size),
    );
    let play_id = ui.make_persistent_id(format!("home_card_play_{}", instance.id));
    let play_resp = ui
        .interact(play_btn_rect, play_id, egui::Sense::click())
        .on_hover_text(format!("Launch {}", instance.name));
    let play_hovered = play_resp.hovered();

    ui.painter().rect_filled(
        play_btn_rect,
        btn_corner,
        if play_hovered {
            p.accent_hover
        } else {
            p.accent
        },
    );
    ui.painter().text(
        play_btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "▶",
        egui::FontId::proportional(14.0),
        p.accent_text,
    );

    let content_left = banner_rect.right() + 10.0;
    let content_right = play_btn_rect.left() - 8.0;
    let clip_rect = egui::Rect::from_min_max(
        egui::pos2(content_left, rect.top()),
        egui::pos2(content_right.max(content_left + 40.0), rect.bottom()),
    );
    let text_painter = ui.painter().with_clip_rect(clip_rect);

    let text_y = rect.top() + 14.0;
    text_painter.text(
        egui::pos2(content_left, text_y),
        egui::Align2::LEFT_TOP,
        &instance.name,
        egui::FontId::proportional(15.0),
        TEXT,
    );

    let badges_y = text_y + 22.0;
    let eco = instance.boost_mode.unwrap_or(state.config.boost_mode);
    let badges_text = format!(
        "Minecraft {} · {}{}",
        instance.minecraft_version,
        instance.loader.display_name(),
        if eco { " · Eco" } else { "" }
    );
    text_painter.text(
        egui::pos2(content_left, badges_y),
        egui::Align2::LEFT_TOP,
        badges_text,
        egui::FontId::proportional(type_scale::CAPTION),
        TEXT2,
    );

    let stats_y = badges_y + 18.0;
    let mods = state.mod_counts.get(&instance.id).copied().unwrap_or(0);
    let stats_text = format!(
        "📦 {} mods · ⏱ {}",
        mods,
        crate::ui::theme::format_playtime(instance.play_time_secs)
    );
    text_painter.text(
        egui::pos2(content_left, stats_y),
        egui::Align2::LEFT_TOP,
        stats_text,
        egui::FontId::proportional(type_scale::MICRO),
        MUTED,
    );

    crate::ui::components::focus_ring(ui, &resp, &format!("Select {}", instance.name));
    crate::ui::components::focus_ring(ui, &play_resp, &format!("Play {}", instance.name));
    if play_resp.clicked() {
        state.home_screenshot_index = 0;
        state.selected_instance = Some(instance.id.clone());
        state.save_config();
        crate::app::tasks::play_instance(state, instance.id.clone());
    } else if resp.clicked() && !is_selected {
        state.home_screenshot_index = 0;
        state.selected_instance = Some(instance.id.clone());
        state.save_config();
        state.refresh_library();
    }

    if !play_hovered {
        resp.on_hover_text(if is_selected {
            format!("{} (currently selected)", instance.name)
        } else {
            format!("Click to select {}", instance.name)
        });
    }
}

fn view_toggle_btn(ui: &mut egui::Ui, active: bool, icon: &str, tooltip: &str) -> egui::Response {
    let p = crate::ui::theme::palette(ui.ctx());
    let theme = crate::ui::theme::current_theme(ui.ctx());
    let size = egui::vec2(32.0, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let (fill, stroke, text_color) = if active {
        if theme == crate::config::ThemeKind::Halloween {
            (
                Color32::from_rgb(52, 28, 14),
                Stroke::new(1.0_f32, p.border),
                Color32::from_rgb(255, 140, 30),
            )
        } else {
            (
                p.elevated2.lerp_to_gamma(p.accent, 0.2),
                Stroke::new(1.0_f32, p.border),
                p.accent,
            )
        }
    } else {
        let bg = if resp.hovered() { p.hover } else { p.elevated2 };
        (bg, Stroke::new(1.0_f32, p.border), TEXT2)
    };
    let corner = CornerRadius::same(6);
    ui.painter()
        .rect(rect, corner, fill, stroke, egui::StrokeKind::Inside);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(15.0),
        text_color,
    );
    crate::ui::components::focus_ring(ui, &resp, tooltip);
    resp.on_hover_text(tooltip)
}

fn render_instance_list_row(
    state: &mut AppState,
    ui: &mut egui::Ui,
    instance: &InstanceConfig,
    is_selected: bool,
) {
    let p = crate::ui::theme::palette(ui.ctx());
    let row_height = 68.0_f32;
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_height),
        egui::Sense::click(),
    );
    let fill = if resp.hovered() {
        p.elevated.lerp_to_gamma(p.hover, 0.5)
    } else {
        p.elevated
    };
    let border_stroke = Stroke::new(1.0_f32, p.border);
    let card_corner = CornerRadius::same(metrics::CARD_RADIUS);
    let thumb_corner = CornerRadius::same(6);
    let btn_corner = CornerRadius::same(metrics::CONTROL_RADIUS);
    ui.painter().rect(
        rect,
        card_corner,
        fill,
        border_stroke,
        egui::StrokeKind::Inside,
    );

    let thumb_size = egui::vec2(96.0, 52.0);
    let thumb_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 10.0 + thumb_size.x * 0.5, rect.center().y),
        thumb_size,
    );
    let thumb_painter = ui.painter().with_clip_rect(thumb_rect);
    let shot_path = state
        .screenshots
        .iter()
        .find(|s| s.instance_id == instance.id)
        .map(|s| s.path.clone());
    let mut thumb_drawn = false;
    if let Some(path) = shot_path {
        state.ensure_screenshot_thumb(&path);
        if let Some(tex) = state.screenshot_thumbnails.get(&path) {
            let tex_size = tex.size_vec2();
            let s = (thumb_rect.width() / tex_size.x).min(thumb_rect.height() / tex_size.y);
            let sub_draw = egui::Rect::from_center_size(thumb_rect.center(), tex_size * s);
            thumb_painter.image(
                tex.id(),
                sub_draw,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            thumb_drawn = true;
        }
    }
    if !thumb_drawn {
        draw_instance_banner_fallback(
            &thumb_painter,
            thumb_rect,
            &instance.name,
            instance.loader.display_name(),
        );
    }
    ui.painter().rect_stroke(
        thumb_rect,
        thumb_corner,
        Stroke::new(1.0_f32, p.border),
        egui::StrokeKind::Inside,
    );

    let menu_btn_size = 32.0_f32;
    let menu_btn_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 24.0, rect.center().y),
        egui::vec2(menu_btn_size, menu_btn_size),
    );
    let running = state.playing.get(&instance.id).copied().unwrap_or(false);
    let installing = state.busy_install.contains_key(&instance.id);
    let eco = instance.boost_mode.unwrap_or(state.config.boost_mode);

    ui.scope_builder(egui::UiBuilder::new().max_rect(menu_btn_rect), |ui| {
        let p = crate::ui::theme::palette(ui.ctx());
        let widgets = &mut ui.style_mut().visuals.widgets;
        widgets.inactive.bg_fill = p.elevated2;
        widgets.hovered.bg_fill = p.hover;
        widgets.active.bg_fill = p.hover;
        widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);
        widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, Color32::WHITE);
        widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);
        widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, p.border);
        widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, p.border.lerp_to_gamma(TEXT, 0.3));
        widgets.inactive.corner_radius = btn_corner;
        widgets.hovered.corner_radius = btn_corner;
        ui.style_mut().spacing.button_padding = egui::vec2(4.0, 4.0);
        let menu_resp = ui.menu_button("•••", |ui| {
            if action_button(ui, "Edit instance").clicked() {
                state.edit_instance = Some(instance.clone());
                ui.close();
            }
            if action_button(ui, "Open game folder").clicked() {
                let _ = open::that(state.instances.game_dir(&instance.id));
                ui.close();
            }
            if action_button(ui, "View logs").clicked() {
                state.set_page(Page::Logs);
                ui.close();
            }
            ui.separator();
            ui.add_enabled_ui(!running && !installing, |ui| {
                if action_button(ui, "Repair game files")
                    .on_hover_text("Check and re-download missing or damaged game files.")
                    .clicked()
                {
                    crate::app::tasks::repair_instance(state, instance.id.clone());
                    ui.close();
                }
            });
            if action_button(
                ui,
                if eco {
                    "Turn off Eco mode"
                } else {
                    "Turn on Eco mode"
                },
            )
            .on_hover_text("Eco mode uses less memory and may lower FPS in demanding worlds.")
            .clicked()
            {
                state.toggle_boost();
                ui.close();
            }
        });
        menu_resp.response.on_hover_text("Instance options");
    });

    let play_btn_size = 32.0_f32;
    let play_btn_rect = egui::Rect::from_center_size(
        egui::pos2(menu_btn_rect.left() - 22.0, rect.center().y),
        egui::vec2(play_btn_size, play_btn_size),
    );
    let play_id = ui.make_persistent_id(format!("home_row_play_{}", instance.id));
    let play_resp = ui
        .interact(play_btn_rect, play_id, egui::Sense::click())
        .on_hover_text(format!("Launch {}", instance.name));
    let play_hovered = play_resp.hovered();

    ui.painter().rect_filled(
        play_btn_rect,
        btn_corner,
        if play_hovered {
            p.accent_hover
        } else {
            p.accent
        },
    );
    ui.painter().text(
        play_btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "▶",
        egui::FontId::proportional(13.0),
        p.accent_text,
    );

    if running || installing {
        let (status_str, status_color) = if running {
            ("• Running", Color32::from_rgb(34, 197, 94))
        } else {
            ("• Preparing", Color32::from_rgb(251, 146, 60))
        };
        ui.painter().text(
            egui::pos2(play_btn_rect.left() - 14.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            status_str,
            egui::FontId::proportional(type_scale::CAPTION),
            status_color,
        );
    }

    let text_start_x = thumb_rect.right() + 14.0;
    let text_max_x = if running || installing {
        play_btn_rect.left() - 110.0
    } else {
        play_btn_rect.left() - 14.0
    };
    let clip_rect = egui::Rect::from_min_max(
        egui::pos2(text_start_x, rect.top()),
        egui::pos2(text_max_x.max(text_start_x + 60.0), rect.bottom()),
    );
    let text_painter = ui.painter().with_clip_rect(clip_rect);

    text_painter.text(
        egui::pos2(text_start_x, rect.center().y - 17.0),
        egui::Align2::LEFT_CENTER,
        &instance.name,
        egui::FontId::proportional(15.0),
        TEXT,
    );

    let sub_text = format!(
        "Minecraft {} · {}",
        instance.minecraft_version,
        instance.loader.display_name(),
    );
    text_painter.text(
        egui::pos2(text_start_x, rect.center().y + 1.0),
        egui::Align2::LEFT_CENTER,
        sub_text,
        egui::FontId::proportional(type_scale::CAPTION),
        TEXT2,
    );

    let mods = state.mod_counts.get(&instance.id).copied().unwrap_or(0);
    let stats_text = format!(
        "📦 {} mods  ⏱ {}",
        mods,
        crate::ui::theme::format_playtime(instance.play_time_secs)
    );
    text_painter.text(
        egui::pos2(text_start_x, rect.center().y + 18.0),
        egui::Align2::LEFT_CENTER,
        stats_text,
        egui::FontId::proportional(type_scale::MICRO),
        MUTED,
    );

    crate::ui::components::focus_ring(ui, &resp, &format!("Select {}", instance.name));
    crate::ui::components::focus_ring(ui, &play_resp, &format!("Play {}", instance.name));
    if play_resp.clicked() {
        state.home_screenshot_index = 0;
        state.selected_instance = Some(instance.id.clone());
        state.save_config();
        crate::app::tasks::play_instance(state, instance.id.clone());
    } else if resp.clicked() && !is_selected {
        state.home_screenshot_index = 0;
        state.selected_instance = Some(instance.id.clone());
        state.save_config();
        state.refresh_library();
    }

    let menu_hovered =
        menu_btn_rect.contains(ui.input(|i| i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO)));
    if !play_hovered && !menu_hovered {
        resp.on_hover_text(if is_selected {
            format!("{} (currently selected)", instance.name)
        } else {
            format!("Click to select {}", instance.name)
        });
    }
}

fn screenshots(state: &mut AppState, cfg: &InstanceConfig, ui: &mut egui::Ui) {
    card_frame(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Latest screenshot")
                    .size(17.0)
                    .strong()
                    .color(TEXT),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(first_shot) = state
                    .screenshots
                    .iter()
                    .filter(|s| s.instance_id == cfg.id)
                    .nth(state.home_screenshot_index)
                    .cloned()
                {
                    if ui
                        .add(egui::Button::new(RichText::new("⛶").size(14.0)).frame(false))
                        .on_hover_text("Fullscreen viewer")
                        .clicked()
                    {
                        state.open_screenshot(&first_shot.path);
                    }
                }
                if open_gallery_button(ui)
                    .on_hover_text("Open screenshots gallery")
                    .clicked()
                {
                    state.set_page(Page::Screenshots);
                }
            });
        });

        let instance_shots: Vec<_> = state
            .screenshots
            .iter()
            .filter(|shot| shot.instance_id == cfg.id)
            .cloned()
            .collect();
        let available_shots = instance_shots;

        if !available_shots.is_empty() {
            let idx = state
                .home_screenshot_index
                .min(available_shots.len().saturating_sub(1));
            let current = &available_shots[idx];
            state.ensure_screenshot_thumb(&current.path);

            ui.add_space(6.0);
            let max_w = ui.available_width().max(120.0);
            if let Some(texture) = state.screenshot_thumbnails.get(&current.path) {
                let native = texture.size_vec2();
                let aspect = if native.y > 0.0 {
                    native.x / native.y
                } else {
                    16.0 / 9.0
                };
                let target_h = (max_w / aspect).round().clamp(120.0, 220.0);
                let (img_rect, img_resp) =
                    ui.allocate_exact_size(egui::vec2(max_w, target_h), egui::Sense::click());
                let p = crate::ui::theme::palette(ui.ctx());
                let shot_corner = CornerRadius::same(6);
                let scale = (img_rect.width() / native.x).min(img_rect.height() / native.y);
                let draw_rect = egui::Rect::from_center_size(img_rect.center(), native * scale);
                ui.painter().rect_filled(img_rect, shot_corner, p.elevated2);
                ui.painter().with_clip_rect(img_rect).image(
                    texture.id(),
                    draw_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                ui.painter().rect_stroke(
                    img_rect,
                    shot_corner,
                    Stroke::new(1.0_f32, p.border),
                    egui::StrokeKind::Inside,
                );
                if img_resp
                    .on_hover_text("Click to view full screenshot")
                    .clicked()
                {
                    state.open_screenshot(&current.path);
                }
                if available_shots.len() > 1 {
                    let prev_center = egui::pos2(img_rect.left() + 20.0, img_rect.center().y);
                    let prev_rect =
                        egui::Rect::from_center_size(prev_center, egui::vec2(28.0, 28.0));
                    let prev_id = ui.make_persistent_id("home_shot_prev");
                    let prev_resp = ui.interact(prev_rect, prev_id, egui::Sense::click());
                    ui.painter().circle_filled(
                        prev_center,
                        13.0,
                        if prev_resp.hovered() {
                            Color32::from_black_alpha(220)
                        } else {
                            Color32::from_black_alpha(150)
                        },
                    );
                    ui.painter().text(
                        prev_center,
                        egui::Align2::CENTER_CENTER,
                        "⏴",
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                    if prev_resp.on_hover_text("Previous screenshot").clicked() {
                        state.home_screenshot_index =
                            (idx + available_shots.len() - 1) % available_shots.len();
                    }

                    let next_center = egui::pos2(img_rect.right() - 20.0, img_rect.center().y);
                    let next_rect =
                        egui::Rect::from_center_size(next_center, egui::vec2(28.0, 28.0));
                    let next_id = ui.make_persistent_id("home_shot_next");
                    let next_resp = ui.interact(next_rect, next_id, egui::Sense::click());
                    ui.painter().circle_filled(
                        next_center,
                        13.0,
                        if next_resp.hovered() {
                            Color32::from_black_alpha(220)
                        } else {
                            Color32::from_black_alpha(150)
                        },
                    );
                    ui.painter().text(
                        next_center,
                        egui::Align2::CENTER_CENTER,
                        "⏵",
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                    if next_resp.on_hover_text("Next screenshot").clicked() {
                        state.home_screenshot_index = (idx + 1) % available_shots.len();
                    }
                }
            } else {
                ui.spinner();
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let p = crate::ui::theme::palette(ui.ctx());
                let thumb_corner = CornerRadius::same(4);
                for (i, entry) in available_shots.iter().enumerate().take(5) {
                    state.ensure_screenshot_thumb(&entry.path);
                    let is_cur = i == idx;
                    let thumb_size = egui::vec2(52.0, 32.0);
                    let (rect, resp) = ui.allocate_exact_size(thumb_size, egui::Sense::click());
                    ui.painter().rect_filled(rect, thumb_corner, p.elevated2);
                    if let Some(tex) = state.screenshot_thumbnails.get(&entry.path) {
                        let tex_size = tex.size_vec2();
                        let s = (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
                        let sub_draw = egui::Rect::from_center_size(rect.center(), tex_size * s);
                        ui.painter().with_clip_rect(rect).image(
                            tex.id(),
                            sub_draw,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    if is_cur {
                        ui.painter().rect_stroke(
                            rect,
                            thumb_corner,
                            Stroke::new(1.5_f32, p.border),
                            egui::StrokeKind::Outside,
                        );
                    } else if resp.hovered() {
                        ui.painter().rect_stroke(
                            rect,
                            thumb_corner,
                            Stroke::new(1.0_f32, p.hover),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if resp
                        .on_hover_text(format!(
                            "View screenshot {} of {}",
                            i + 1,
                            available_shots.len()
                        ))
                        .clicked()
                    {
                        state.home_screenshot_index = i;
                    }
                }
                if available_shots.len() > 5 {
                    let overflow = available_shots.len() - 5;
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(40.0, 32.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, thumb_corner, p.elevated2);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("+{overflow}"),
                        egui::FontId::proportional(12.0),
                        TEXT2,
                    );
                    if resp
                        .on_hover_text(format!("+{overflow} more screenshots · Open gallery"))
                        .clicked()
                    {
                        state.set_page(Page::Screenshots);
                    }
                }
            });

            let date: chrono::DateTime<chrono::Local> = current.modified.into();
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "{} · {}",
                    cfg.name,
                    date.format("%b %d at %I:%M %p")
                ))
                .size(12.0)
                .color(TEXT2),
            );
        } else {
            ui.add_space(28.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Keep a moment from your world")
                        .size(18.0)
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("Press F2 while playing to take a screenshot.").color(TEXT2),
                );
                ui.label(
                    RichText::new("You'll find it here when you come back.")
                        .size(12.0)
                        .color(MUTED),
                );
            });
            ui.add_space(40.0);
        }
    });
}

fn open_gallery_button(ui: &mut egui::Ui) -> egui::Response {
    let p = crate::ui::theme::palette(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(116.0, 30.0), egui::Sense::click());
    let (bg, border, fg) = if resp.is_pointer_button_down_on() {
        (p.hover.lerp_to_gamma(p.accent, 0.12), p.border, TEXT)
    } else if resp.hovered() {
        (p.hover, p.border.lerp_to_gamma(TEXT, 0.30), Color32::WHITE)
    } else {
        (p.elevated2, p.border, TEXT)
    };
    let corner = CornerRadius::same(metrics::CONTROL_RADIUS);
    ui.painter().rect(
        rect,
        corner,
        bg,
        Stroke::new(1.0_f32, border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "Open gallery",
        egui::FontId::proportional(type_scale::CAPTION),
        fg,
    );
    crate::ui::components::draw_external_link_icon(
        ui.painter(),
        egui::pos2(rect.right() - 14.0, rect.center().y),
        9.0,
        fg,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}
