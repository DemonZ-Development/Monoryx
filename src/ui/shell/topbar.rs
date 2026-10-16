use crate::app::events::Page;
use crate::app::state::AppState;
use crate::ui::theme::{type_scale, Palette, MUTED, TEXT, TEXT2};
use egui::{Color32, CornerRadius, RichText, Stroke};

pub(super) fn show(state: &mut AppState, ctx: &egui::Context, theme: Palette) {
    egui::TopBottomPanel::top("topbar")
        .exact_height(58.0)
        .frame(
            egui::Frame::new()
                .fill(theme.elevated)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .inner_margin(egui::Margin::symmetric(14, 0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                brand(state, ctx, ui, theme);
                ui.add_space(12.0);
                play_button(state, ctx, ui, theme);
                ui.add_space(12.0);
                nav_tabs(state, ctx, ui, theme);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    account_chip(state, ctx, ui, theme);
                    ui.add_space(4.0);
                    activity_bell(state, ui, theme);
                    ui.add_space(4.0);
                    if ctx.available_rect().width() >= 1180.0 {
                        search_pill(state, ui, theme);
                    }
                });
            });
        });
    status_strip(state, ctx, theme);
}

fn brand(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui, theme: Palette) {
    if crate::ui::theme::current_theme(ctx) == crate::config::ThemeKind::Halloween {
        let (bat_rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 18.0), egui::Sense::hover());
        crate::ui::components::draw_glowing_bat(ui.painter(), bat_rect.center(), theme.accent);
        ui.add_space(2.0);
    }
    let (rect, response) = ui.allocate_exact_size(egui::vec2(96.0, 24.0), egui::Sense::click());
    ui.painter().text(
        rect.left_center(),
        egui::Align2::LEFT_CENTER,
        "MONORYX",
        egui::FontId::new(17.0, egui::FontFamily::Proportional),
        TEXT,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    crate::ui::components::focus_ring(ui, &response, "Home");
    if response.on_hover_text("Go home").clicked() {
        state.set_page(Page::Home);
    }
}

fn play_button(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui, theme: Palette) {
    let Some(cfg) = state.selected() else {
        if crate::ui::components::primary_button(ui, "Create instance").clicked() {
            crate::ui::pages::instances::open_new_dialog(state);
            state.set_page(Page::Instances);
        }
        return;
    };
    let running = state.playing.get(&cfg.id).copied().unwrap_or(false);
    let installing = state.busy_install.contains_key(&cfg.id);
    let height = 34.0;
    let main_w = 82.0;
    let chevron_w = 26.0;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(main_w + chevron_w, height), egui::Sense::hover());
    let main_rect = egui::Rect::from_min_size(rect.min, egui::vec2(main_w, height));
    let chevron_rect = egui::Rect::from_min_max(egui::pos2(main_rect.max.x, rect.min.y), rect.max);
    let main_response = ui.interact(main_rect, ui.id().with("topbar-play"), egui::Sense::click());
    let chevron_response = ui.interact(
        chevron_rect,
        ui.id().with("topbar-play-more"),
        egui::Sense::click(),
    );
    let hovered = main_response.hovered() || chevron_response.hovered();
    let fade = ctx.animate_bool_with_time(ui.id().with("topbar-play-hover"), hovered, 0.12);
    if fade > 0.001 && fade < 0.999 {
        ctx.request_repaint();
    }
    let fill = if running || installing {
        theme.elevated2
    } else {
        theme.accent.lerp_to_gamma(theme.accent_hover, fade)
    };
    let chevron_fill = if running || installing {
        fill
    } else {
        fill.lerp_to_gamma(theme.bg, 0.22)
    };
    let text_color = if running || installing {
        TEXT2
    } else {
        theme.accent_text
    };
    ui.painter().rect_filled(
        main_rect,
        CornerRadius {
            nw: 8,
            ne: 0,
            sw: 8,
            se: 0,
        },
        fill,
    );
    ui.painter().rect_filled(
        chevron_rect,
        CornerRadius {
            nw: 0,
            ne: 8,
            sw: 0,
            se: 8,
        },
        chevron_fill,
    );
    let label = if running {
        "Running"
    } else if installing {
        "Installing"
    } else {
        "Play"
    };
    let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, text_color);
    let triangle_w = if running || installing { 0.0 } else { 12.0 };
    let start_x = main_rect.center().x - (galley.size().x + triangle_w) / 2.0;
    if triangle_w > 0.0 {
        let cy = main_rect.center().y;
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(start_x, cy - 5.0),
                egui::pos2(start_x, cy + 5.0),
                egui::pos2(start_x + 9.0, cy),
            ],
            text_color,
            Stroke::NONE,
        ));
    }
    ui.painter().galley(
        egui::pos2(
            start_x + triangle_w,
            main_rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        text_color,
    );
    ui.painter().text(
        chevron_rect.center(),
        egui::Align2::CENTER_CENTER,
        "⏷",
        egui::FontId::proportional(12.5),
        text_color,
    );
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    crate::ui::components::focus_ring(ui, &main_response, "Play");
    let hover_text = if running {
        "Minecraft is already running".to_string()
    } else if installing {
        "Preparing game files".to_string()
    } else {
        format!("Play {}", cfg.name)
    };
    if main_response.on_hover_text(hover_text).clicked() && !running && !installing {
        crate::app::tasks::play_instance(state, cfg.id.clone());
    }
    egui::Popup::menu(&chevron_response).show(|ui| {
        ui.set_min_width(240.0);
        ui.label(crate::ui::theme::section_label("PLAY AN INSTANCE"));
        let instances = state.instance_list.clone();
        for instance in instances.iter().take(8) {
            let selected = state.selected_instance.as_deref() == Some(instance.id.as_str());
            let label = format!("{} · MC {}", instance.name, instance.minecraft_version);
            if ui.selectable_label(selected, label).clicked() {
                state.selected_instance = Some(instance.id.clone());
                state.save_config();
                state.refresh_library();
                ui.close();
            }
        }
        if instances.is_empty() {
            ui.label(RichText::new("No instances yet").color(TEXT2));
        }
        ui.separator();
        if ui.button("Manage instances").clicked() {
            state.set_page(Page::Instances);
            ui.close();
        }
        if ui.button("New instance").clicked() {
            crate::ui::pages::instances::open_new_dialog(state);
            state.set_page(Page::Instances);
            ui.close();
        }
    });
}

fn nav_tabs(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui, theme: Palette) {
    let window_width = ctx.available_rect().width();
    let search_width = if window_width >= 1180.0 { 216.0 } else { 0.0 };
    let account_width = if window_width >= 1020.0 { 150.0 } else { 34.0 };
    let right_cluster = search_width + account_width + 30.0 + 64.0;
    let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
    let more_width = ui
        .painter()
        .layout_no_wrap("More ⏷".to_string(), font.clone(), TEXT2)
        .size()
        .x
        + 30.0;
    let spacing = ui.spacing().item_spacing.x;
    let mut budget = ui.available_width() - right_cluster - more_width - spacing;
    let mut hidden: Vec<Page> = Vec::new();
    for page in Page::all() {
        if !hidden.is_empty() {
            hidden.push(page);
            continue;
        }
        let tab_width = ui
            .painter()
            .layout_no_wrap(page.label().to_string(), font.clone(), TEXT)
            .size()
            .x
            + 22.0
            + spacing;
        if tab_width > budget {
            hidden.push(page);
            continue;
        }
        budget -= tab_width;
        if nav_tab(state, ctx, ui, page, theme).clicked() {
            state.set_page(page);
        }
    }
    if !hidden.is_empty() {
        more_menu(state, ui, theme, &hidden);
    }
}

fn nav_tab(
    state: &AppState,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    page: Page,
    theme: Palette,
) -> egui::Response {
    let label = page.label();
    let selected = state.page == page;
    let id = ui.make_persistent_id(("topbar-tab", page.as_str()));
    let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), TEXT);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(galley.size().x + 22.0, 30.0),
        egui::Sense::click(),
    );
    let fade = ctx.animate_bool_with_time(id.with("hover"), response.hovered(), 0.15);
    if fade > 0.001 && fade < 0.999 {
        ctx.request_repaint();
    }
    let fill = if selected {
        theme.accent
    } else if fade > 0.01 {
        theme.elevated.lerp_to_gamma(theme.hover, fade)
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
    }
    let text_color = if selected {
        theme.accent_text
    } else {
        TEXT2.lerp_to_gamma(TEXT, fade)
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font,
        text_color,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    crate::ui::components::focus_ring(ui, &response, label);
    response.on_hover_text(super::page_tooltip(page))
}

fn more_menu(state: &mut AppState, ui: &mut egui::Ui, theme: Palette, hidden: &[Page]) {
    let on_hidden = hidden.contains(&state.page);
    let title_color = if on_hidden { theme.accent } else { TEXT2 };
    ui.menu_button(
        RichText::new("More ⏷").size(13.0).color(title_color),
        |ui| {
            ui.set_min_width(216.0);
            for &page in hidden {
                let selected = state.page == page;
                let label = format!("{}  {}", super::page_icon(page), page.label());
                if ui.selectable_label(selected, label).clicked() {
                    state.set_page(page);
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Switch to sidebar layout").clicked() {
                state.config.ui_layout = crate::config::UiLayout::Sidebar;
                state.save_config();
                ui.close();
            }
        },
    );
}

fn activity_bell(state: &mut AppState, ui: &mut egui::Ui, theme: Palette) {
    let active = state.operations.len() + state.downloads.len() + state.busy_install.len();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), theme.hover);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "🔔",
        egui::FontId::proportional(13.0),
        TEXT2,
    );
    if active > 0 {
        ui.painter().circle_filled(
            egui::pos2(rect.right() - 7.0, rect.top() + 7.0),
            3.5,
            theme.accent,
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    crate::ui::components::focus_ring(ui, &response, "Downloads");
    let hint = if active > 0 {
        format!("{active} active task(s) — open Downloads")
    } else {
        "Downloads".to_string()
    };
    if response.on_hover_text(hint).clicked() {
        state.set_page(Page::Downloads);
    }
}

fn account_chip(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui, theme: Palette) {
    let account = super::AccountView::from_state(state);
    let show_name = ctx.available_rect().width() >= 1020.0;
    let width = if show_name { 150.0 } else { 34.0 };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), theme.hover);
    }
    let Some(account) = account else {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "👤",
            egui::FontId::proportional(13.0),
            TEXT2,
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        crate::ui::components::focus_ring(ui, &response, "Accounts");
        if response.on_hover_text("Configure account").clicked() {
            state.set_page(Page::Accounts);
        }
        return;
    };
    let avatar_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 17.0, rect.center().y),
        egui::vec2(22.0, 22.0),
    );
    if account.offline {
        crate::ui::components::draw_cute_avatar(
            ui.painter(),
            avatar_rect,
            &account.username,
            false,
        );
    } else {
        crate::ui::components::draw_avatar(
            ui.painter(),
            avatar_rect,
            &account.username,
            &account.uuid.to_string(),
        );
    }
    if show_name {
        ui.painter().text(
            egui::pos2(rect.left() + 34.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            crate::ui::components::elide(&account.username, 12),
            egui::FontId::new(type_scale::LABEL, egui::FontFamily::Proportional),
            TEXT,
        );
        ui.painter().text(
            egui::pos2(rect.right() - 8.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "⏷",
            egui::FontId::proportional(type_scale::MICRO),
            MUTED,
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    crate::ui::components::focus_ring(ui, &response, "Manage account");
    if response
        .on_hover_text(format!(
            "Active account: {} ({})",
            account.username,
            account.kind_label()
        ))
        .clicked()
    {
        state.set_page(Page::Accounts);
    }
}

fn search_pill(state: &mut AppState, ui: &mut egui::Ui, theme: Palette) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(216.0, 30.0), egui::Sense::click());
    let hovered = response.hovered();
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        if hovered {
            theme.hover
        } else {
            theme.elevated2
        },
        Stroke::new(1.0_f32, theme.border),
        egui::StrokeKind::Inside,
    );
    crate::ui::components::magnifier(
        ui.painter(),
        egui::pos2(rect.left() + 16.0, rect.center().y),
        6.0,
        TEXT2,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 30.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "Search or jump to…",
        egui::FontId::proportional(type_scale::LABEL),
        TEXT2,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 10.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        "Ctrl+K",
        egui::FontId::proportional(type_scale::MICRO),
        MUTED,
    );
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    crate::ui::components::focus_ring(ui, &response, "Command palette");
    if response
        .on_hover_text("Search pages, instances, and actions (Ctrl+K)")
        .clicked()
    {
        state.command_palette_open = true;
    }
}

fn status_strip(state: &mut AppState, ctx: &egui::Context, theme: Palette) {
    let playing = state
        .playing
        .iter()
        .find(|(_, playing)| **playing)
        .and_then(|(id, _)| {
            state
                .instance_list
                .iter()
                .find(|instance| instance.id == *id)
                .map(|instance| instance.name.clone())
        });
    let operation = state
        .operations
        .values()
        .next()
        .map(|operation| (operation.phase.clone(), operation.fraction()));
    let text = match (&playing, &operation) {
        (Some(name), _) => format!("Playing {name}"),
        (None, Some((phase, _))) => phase.clone(),
        (None, None) => state.global_status.clone(),
    };
    let text = if !text.is_empty() {
        text
    } else if state.downloads.len() == 1 {
        "1 download in progress".to_string()
    } else if !state.downloads.is_empty() {
        format!("{} downloads in progress", state.downloads.len())
    } else {
        String::new()
    };
    let fraction = operation
        .as_ref()
        .and_then(|(_, fraction)| *fraction)
        .or(state.global_frac);
    if text.is_empty() && fraction.is_none() {
        return;
    }
    egui::TopBottomPanel::bottom("topbar-status")
        .exact_height(30.0)
        .frame(
            egui::Frame::new()
                .fill(theme.elevated)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .inner_margin(egui::Margin::symmetric(14, 0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let (dot_rect, _) =
                    ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter()
                    .circle_filled(dot_rect.center(), 4.0, theme.accent);
                ui.label(RichText::new(&text).size(type_scale::LABEL).color(TEXT));
                if fraction.is_some() || !state.operations.is_empty() {
                    ui.allocate_ui_with_layout(
                        egui::vec2(180.0, 8.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            crate::ui::components::thin_progress(ui, fraction);
                        },
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::ui::components::button(
                        ui,
                        "Downloads",
                        crate::ui::components::Tone::Ghost,
                    )
                    .clicked()
                    {
                        state.set_page(Page::Downloads);
                    }
                });
            });
        });
}
