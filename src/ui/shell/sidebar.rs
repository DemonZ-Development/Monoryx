use crate::app::events::Page;
use crate::app::state::AppState;
use crate::ui::theme::{metrics, type_scale, Palette, ACCENT, MUTED, TEXT, TEXT2};
use egui::{Color32, CornerRadius, RichText, Stroke};

pub(super) fn show(state: &mut AppState, ctx: &egui::Context, theme: Palette) {
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
}

fn sidebar_item(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    page: Page,
    selected: bool,
    has_badge: bool,
) -> egui::Response {
    let label = page.label();
    let icon = super::page_icon(page);
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
    response.on_hover_text(super::page_tooltip(page))
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

fn sidebar_footer(state: &mut AppState, ui: &mut egui::Ui) {
    let account = super::AccountView::from_state(state);
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
                            format!("{} ⏷", acc.kind_label()),
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
                    acc.kind_label()
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
                crate::config::ThemeKind::Aurora => ("🌌", "Aurora"),
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
