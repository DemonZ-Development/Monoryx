use crate::ui::theme::{
    metrics, type_scale, BORDER, DANGER, ELEVATED2, MUTED, OK, TEXT, TEXT2, WARNING,
};
use base64::Engine;
use egui::{Color32, CornerRadius, RichText, Stroke};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

pub fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(26.0).strong().color(TEXT));
    });
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).size(12.5).color(TEXT2));
    }
    ui.add_space(14.0);
}

pub fn card_frame(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let p = crate::ui::theme::palette(ui.ctx());
    let corner = CornerRadius::same(metrics::CARD_RADIUS);
    let frame = egui::Frame::new()
        .fill(p.elevated)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(corner)
        .inner_margin(egui::Margin::same(metrics::CARD_MARGIN))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    gloss_highlight(ui, frame.response.rect);
}

pub use crate::ui::theme::format_last_played;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StepState {
    Done,
    Active,
    Todo,
}

#[must_use]
pub fn step_state(index: usize, active: usize) -> StepState {
    if index < active {
        StepState::Done
    } else if index == active {
        StepState::Active
    } else {
        StepState::Todo
    }
}

pub fn step_rail(ui: &mut egui::Ui, total: usize, active: usize) {
    let total = total.max(1);
    let active = active.min(total - 1);
    let p = crate::ui::theme::palette(ui.ctx());
    let spacing = 14.0_f32;
    let dot = 10.0_f32;
    let connector = 30.0_f32;
    let total_width = dot * total as f32 + connector * (total.saturating_sub(1)) as f32;

    ui.vertical_centered(|ui| {
        let (rail, response) =
            ui.allocate_exact_size(egui::vec2(total_width, dot), egui::Sense::hover());
        for index in 0..total {
            let center = egui::pos2(
                rail.min.x + dot / 2.0 + (dot + connector) * index as f32,
                rail.center().y,
            );
            if index + 1 < total {
                let line = egui::Rect::from_min_max(
                    egui::pos2(center.x + dot / 2.0, center.y - 1.0),
                    egui::pos2(
                        rail.min.x + dot / 2.0 + (dot + connector) * (index + 1) as f32 - dot / 2.0,
                        center.y + 1.0,
                    ),
                );
                ui.painter().rect_filled(
                    line,
                    1,
                    if step_state(index, active) == StepState::Done {
                        p.accent
                    } else {
                        p.border
                    },
                );
            }
            let fill = match step_state(index, active) {
                StepState::Done | StepState::Active => p.accent,
                StepState::Todo => p.elevated2,
            };
            ui.painter().circle_filled(center, dot / 2.0, fill);
            if step_state(index, active) == StepState::Todo {
                ui.painter()
                    .circle_stroke(center, dot / 2.0, Stroke::new(1.0_f32, p.border));
            }
        }
        response.on_hover_text(format!("Step {} of {total}", active + 1));
        ui.add_space(spacing);
        ui.label(crate::ui::theme::caption(format!(
            "Step {} of {total}",
            active + 1
        )));
    });
}

pub fn wizard_frame(
    ui: &mut egui::Ui,
    step: usize,
    _panel_height: f32,
    content_key: u64,
    mut add: impl FnMut(&mut egui::Ui, bool),
) {
    let p = crate::ui::theme::palette(ui.ctx());

    let cache_id = egui::Id::new(("wizard-height", step));
    let body_cache = egui::Id::new(("wizard-body-height", step, content_key));

    ui.add_space(14.0);
    ui.vertical_centered(|ui| {
        let corner = CornerRadius::same(16);
        let mut measured = 0.0_f32;
        let frame = egui::Frame::new()
            .fill(p.elevated)
            .stroke(Stroke::new(1.0_f32, p.border))
            .corner_radius(corner)
            .inner_margin(egui::Margin::same(22))
            .show(ui, |ui| {
                ui.set_width(metrics::WIZARD_CARD_W.min(ui.available_width()));
                let cap = (ui.available_height() - 62.0).max(80.0);
                let reserved = ui
                    .ctx()
                    .data(|d| d.get_temp::<f32>(body_cache).unwrap_or(cap))
                    .clamp(0.0, cap);
                egui::ScrollArea::vertical()
                    .id_salt(("wizard-body", step))
                    .max_height(reserved)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            add(ui, false);
                        });
                        measured = ui.min_rect().height();
                    });
                ui.add_space(12.0);
                add(ui, true);
            });
        if measured > 0.0 {
            ui.ctx().data_mut(|d| d.insert_temp(body_cache, measured));
        }
        if crate::ui::theme::current_theme(ui.ctx()) == crate::config::ThemeKind::Halloween {
            draw_spiderweb(ui.painter(), frame.response.rect);
        }
        let height = frame.response.rect.height();
        ui.ctx().data_mut(|d| d.insert_temp(cache_id, height));
    });
}

pub fn provider_card(ui: &mut egui::Ui, name: &str, detail: &str, status: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    let corner = CornerRadius::same(8);
    let tile_corner = CornerRadius::same(6);
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(corner)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (tile, _) =
                    ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                ui.painter().rect_filled(tile, tile_corner, p.hover);
                ui.painter().text(
                    tile.center(),
                    egui::Align2::CENTER_CENTER,
                    name.chars()
                        .next()
                        .unwrap_or('?')
                        .to_uppercase()
                        .to_string(),
                    egui::FontId::proportional(14.0),
                    p.accent,
                );
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(name)
                            .size(type_scale::BODY)
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(RichText::new(detail).size(type_scale::CAPTION).color(TEXT2));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(status).size(type_scale::MICRO).color(MUTED));
                });
            });
        });
}
pub fn hero_card_frame(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let p = crate::ui::theme::palette(ui.ctx());
    let corner = CornerRadius::same(14);
    let frame = egui::Frame::new()
        .fill(p.elevated)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(corner)
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    gloss_highlight(ui, frame.response.rect);
    halloween_accent(ui, frame.response.rect);
}

fn gloss_highlight(ui: &egui::Ui, rect: egui::Rect) {
    if crate::ui::theme::current_theme(ui.ctx()) == crate::config::ThemeKind::Gloss {
        ui.painter().line_segment(
            [
                rect.left_top() + egui::vec2(14.0, 1.0),
                rect.right_top() + egui::vec2(-14.0, 1.0),
            ],
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(190, 225, 255, 65)),
        );
    }
}

fn halloween_accent(ui: &egui::Ui, rect: egui::Rect) {
    if crate::ui::theme::current_theme(ui.ctx()) == crate::config::ThemeKind::Halloween {
        draw_spiderweb(ui.painter(), rect);
        draw_small_pumpkin(
            ui.painter(),
            rect.left_top() + egui::vec2(16.0, 4.0),
            22.0,
            -0.20,
            true,
        );
        draw_small_pumpkin(
            ui.painter(),
            rect.right_bottom() + egui::vec2(-12.0, -8.0),
            20.0,
            0.18,
            true,
        );
        draw_small_pumpkin(
            ui.painter(),
            rect.right_bottom() + egui::vec2(6.0, -1.0),
            15.0,
            -0.14,
            true,
        );
    }
}

pub fn draw_spiderweb(painter: &egui::Painter, rect: egui::Rect) {
    if rect.width() < 120.0 || rect.height() < 80.0 {
        return;
    }
    let origin = rect.right_top();
    let web_color = Color32::from_rgba_unmultiplied(255, 145, 50, 42);
    let stroke = Stroke::new(1.0_f32, web_color);

    let angles: [f32; 5] = [0.0, 0.38, 0.785, 1.18, 1.57];
    let lengths: [f32; 5] = [54.0, 50.0, 56.0, 50.0, 54.0];
    let mut spokes = Vec::with_capacity(5);
    for (angle, len) in angles.iter().zip(lengths.iter()) {
        let pt = origin + egui::vec2(-len * angle.cos(), len * angle.sin());
        painter.line_segment([origin, pt], stroke);
        spokes.push(pt);
    }

    for frac in [0.35_f32, 0.65_f32, 0.95_f32] {
        for i in 0..(spokes.len() - 1) {
            let p1 = origin.lerp(spokes[i], frac);
            let p2 = origin.lerp(spokes[i + 1], frac);
            painter.line_segment([p1, p2], stroke);
        }
    }
}

pub fn draw_bat_scaled(painter: &egui::Painter, center: egui::Pos2, scale: f32, color: Color32) {
    let s = scale.max(0.15);
    let left_wing = vec![
        center,
        center + egui::vec2(-4.0 * s, -2.5 * s),
        center + egui::vec2(-10.0 * s, -3.5 * s),
        center + egui::vec2(-15.0 * s, -2.5 * s),
        center + egui::vec2(-11.0 * s, 1.0 * s),
        center + egui::vec2(-6.0 * s, 2.0 * s),
    ];
    let right_wing = vec![
        center,
        center + egui::vec2(4.0 * s, -2.5 * s),
        center + egui::vec2(10.0 * s, -3.5 * s),
        center + egui::vec2(15.0 * s, -2.5 * s),
        center + egui::vec2(11.0 * s, 1.0 * s),
        center + egui::vec2(6.0 * s, 2.0 * s),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(
        left_wing,
        color,
        Stroke::NONE,
    ));
    painter.add(egui::epaint::PathShape::convex_polygon(
        right_wing,
        color,
        Stroke::NONE,
    ));
    painter.circle_filled(center + egui::vec2(-2.0 * s, -3.2 * s), 1.3 * s, color);
    painter.circle_filled(center + egui::vec2(2.0 * s, -3.2 * s), 1.3 * s, color);
    painter.circle_filled(center, 3.2 * s, color);
}

pub fn draw_glowing_bat(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    draw_bat_scaled(painter, center, 1.0, color);
}

pub fn draw_pine_tree(
    painter: &egui::Painter,
    base: egui::Pos2,
    height: f32,
    width: f32,
    color: Color32,
) {
    let half_w = width * 0.5;
    let trunk_w = (width * 0.18).clamp(1.5, 4.0);
    let trunk_top_y = base.y - height * 0.22;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(base.x - trunk_w * 0.5, trunk_top_y),
            egui::pos2(base.x + trunk_w * 0.5, base.y),
        ),
        CornerRadius::ZERO,
        color,
    );

    let tier1 = vec![
        egui::pos2(base.x - half_w, base.y - height * 0.18),
        egui::pos2(base.x + half_w, base.y - height * 0.18),
        egui::pos2(base.x, base.y - height * 0.55),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(
        tier1,
        color,
        Stroke::NONE,
    ));

    let tier2 = vec![
        egui::pos2(base.x - half_w * 0.78, base.y - height * 0.44),
        egui::pos2(base.x + half_w * 0.78, base.y - height * 0.44),
        egui::pos2(base.x, base.y - height * 0.8),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(
        tier2,
        color,
        Stroke::NONE,
    ));

    let tier3 = vec![
        egui::pos2(base.x - half_w * 0.55, base.y - height * 0.68),
        egui::pos2(base.x + half_w * 0.55, base.y - height * 0.68),
        egui::pos2(base.x, base.y - height),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(
        tier3,
        color,
        Stroke::NONE,
    ));

    painter.line_segment(
        [
            egui::pos2(base.x, base.y - height),
            egui::pos2(base.x, base.y - height - 3.0),
        ],
        Stroke::new(1.0_f32, color),
    );
}

pub fn draw_dead_tree(painter: &egui::Painter, base: egui::Pos2, height: f32, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let thin_stroke = Stroke::new(1.0_f32, color);
    let top = base + egui::vec2(2.0, -height);
    let mid = base + egui::vec2(-2.0, -height * 0.55);
    painter.line_segment([base, mid], Stroke::new(2.2_f32, color));
    painter.line_segment([mid, top], stroke);

    let b1 = mid + egui::vec2(-8.0, -height * 0.2);
    painter.line_segment([mid, b1], stroke);
    painter.line_segment([b1, b1 + egui::vec2(-4.0, -6.0)], thin_stroke);
    painter.line_segment([b1, b1 + egui::vec2(-1.0, -8.0)], thin_stroke);

    let b2 = base + egui::vec2(5.0, -height * 0.42);
    painter.line_segment([base + egui::vec2(0.0, -height * 0.35), b2], stroke);
    painter.line_segment([b2, b2 + egui::vec2(6.0, -7.0)], thin_stroke);
    painter.line_segment([b2, b2 + egui::vec2(3.0, -10.0)], thin_stroke);

    painter.line_segment([top, top + egui::vec2(-3.0, -6.0)], thin_stroke);
    painter.line_segment([top, top + egui::vec2(4.0, -5.0)], thin_stroke);
}

pub fn draw_external_link_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    size: f32,
    color: Color32,
) {
    let half = size * 0.5;
    let left = center.x - half;
    let right = center.x + half;
    let top = center.y - half;
    let bot = center.y + half;
    let stroke = Stroke::new(1.2_f32, color);

    let box_pts = [
        egui::pos2(center.x - 0.5, top),
        egui::pos2(left, top),
        egui::pos2(left, bot),
        egui::pos2(right, bot),
        egui::pos2(right, center.y + 0.5),
    ];
    for w in box_pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }

    let arrow_start = egui::pos2(center.x - 1.0, center.y + 1.0);
    let arrow_end = egui::pos2(right, top);
    painter.line_segment([arrow_start, arrow_end], stroke);
    painter.line_segment([egui::pos2(right - 3.5, top), arrow_end], stroke);
    painter.line_segment([egui::pos2(right, top + 3.5), arrow_end], stroke);
}

pub fn draw_small_pumpkin(
    painter: &egui::Painter,
    center: egui::Pos2,
    size: f32,
    angle: f32,
    has_face: bool,
) {
    let rot = |v: egui::Vec2| -> egui::Vec2 {
        let cos = angle.cos();
        let sin = angle.sin();
        egui::vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
    };

    let half_w = size * 0.5;
    let half_h = size * 0.42;
    let stem_h = (size * 0.35).clamp(2.5, 6.0);
    let stem_w = (size * 0.18).clamp(1.4, 3.0);
    let stem_top = center + rot(egui::vec2(0.8, -half_h - stem_h));
    let stem_base = center + rot(egui::vec2(-0.4, -half_h + 0.6));

    painter.line_segment(
        [stem_base, stem_top],
        Stroke::new(stem_w, Color32::from_rgb(62, 85, 38)),
    );

    let side_color = Color32::from_rgb(215, 92, 18);
    let mid_color = Color32::from_rgb(248, 124, 26);

    let left_center = center + rot(egui::vec2(-half_w * 0.42, 0.0));
    let right_center = center + rot(egui::vec2(half_w * 0.42, 0.0));

    painter.circle_filled(left_center, half_h * 0.90, side_color);
    painter.circle_filled(right_center, half_h * 0.90, side_color);
    painter.circle_filled(center, half_h * 0.98, mid_color);

    let rib_stroke = Stroke::new(0.8_f32, Color32::from_rgb(180, 72, 12));
    painter.line_segment(
        [
            center + rot(egui::vec2(-half_w * 0.22, -half_h * 0.70)),
            center + rot(egui::vec2(-half_w * 0.22, half_h * 0.70)),
        ],
        rib_stroke,
    );
    painter.line_segment(
        [
            center + rot(egui::vec2(half_w * 0.22, -half_h * 0.70)),
            center + rot(egui::vec2(half_w * 0.22, half_h * 0.70)),
        ],
        rib_stroke,
    );

    if has_face {
        let eye_color = Color32::from_rgb(28, 14, 10);
        let eye_size = (size * 0.16).clamp(1.2, 3.0);
        let eye_y = -half_h * 0.16;

        let left_eye = vec![
            center + rot(egui::vec2(-half_w * 0.32, eye_y)),
            center + rot(egui::vec2(-half_w * 0.12, eye_y)),
            center + rot(egui::vec2(-half_w * 0.22, eye_y - eye_size)),
        ];
        painter.add(egui::epaint::PathShape::convex_polygon(
            left_eye,
            eye_color,
            Stroke::NONE,
        ));

        let right_eye = vec![
            center + rot(egui::vec2(half_w * 0.12, eye_y)),
            center + rot(egui::vec2(half_w * 0.32, eye_y)),
            center + rot(egui::vec2(half_w * 0.22, eye_y - eye_size)),
        ];
        painter.add(egui::epaint::PathShape::convex_polygon(
            right_eye,
            eye_color,
            Stroke::NONE,
        ));

        let mouth_y = half_h * 0.32;
        let mouth_pts = [
            center + rot(egui::vec2(-half_w * 0.30, mouth_y)),
            center + rot(egui::vec2(-half_w * 0.10, mouth_y + eye_size * 0.6)),
            center + rot(egui::vec2(half_w * 0.10, mouth_y + eye_size * 0.6)),
            center + rot(egui::vec2(half_w * 0.30, mouth_y)),
        ];
        for w in mouth_pts.windows(2) {
            painter.line_segment([w[0], w[1]], Stroke::new(eye_size * 0.9, eye_color));
        }
    }
}

pub fn draw_halloween_shell_artwork(painter: &egui::Painter, rect: egui::Rect) {
    if rect.width() < 300.0 || rect.height() < 200.0 {
        return;
    }
    let anchor_x = rect.right();
    let anchor_y = rect.top();

    let mountain_pts = vec![
        egui::pos2(anchor_x - 440.0, anchor_y + 92.0),
        egui::pos2(anchor_x - 360.0, anchor_y + 66.0),
        egui::pos2(anchor_x - 290.0, anchor_y + 56.0),
        egui::pos2(anchor_x - 230.0, anchor_y + 64.0),
        egui::pos2(anchor_x - 170.0, anchor_y + 58.0),
        egui::pos2(anchor_x - 110.0, anchor_y + 74.0),
        egui::pos2(anchor_x - 110.0, anchor_y + 110.0),
        egui::pos2(anchor_x - 440.0, anchor_y + 110.0),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(
        mountain_pts,
        Color32::from_rgb(20, 15, 24),
        Stroke::NONE,
    ));

    let mountain_trees: &[(f32, f32, f32, f32)] = &[
        (-380.0, 72.0, 16.0, 7.5),
        (-350.0, 68.0, 18.0, 8.0),
        (-320.0, 62.0, 20.0, 8.5),
        (-290.0, 58.0, 22.0, 9.0),
        (-265.0, 61.0, 19.0, 8.0),
        (-240.0, 63.0, 21.0, 9.0),
        (-215.0, 64.0, 18.0, 8.0),
        (-190.0, 62.0, 20.0, 8.5),
        (-165.0, 64.0, 17.0, 7.5),
    ];
    for &(dx, dy, h, w) in mountain_trees {
        draw_pine_tree(
            painter,
            egui::pos2(anchor_x + dx, anchor_y + dy),
            h,
            w,
            Color32::from_rgb(16, 12, 18),
        );
    }

    let moon_center = egui::pos2(anchor_x - 105.0, anchor_y + 36.0);
    painter.circle_filled(moon_center, 23.0, Color32::from_rgb(224, 106, 26));

    painter.circle_filled(
        moon_center + egui::vec2(-6.5, -3.5),
        4.5,
        Color32::from_rgba_unmultiplied(190, 82, 18, 85),
    );
    painter.circle_filled(
        moon_center + egui::vec2(-2.0, 5.5),
        5.5,
        Color32::from_rgba_unmultiplied(190, 82, 18, 75),
    );
    painter.circle_filled(
        moon_center + egui::vec2(6.5, -4.5),
        3.5,
        Color32::from_rgba_unmultiplied(190, 82, 18, 80),
    );

    let hill_pts = vec![
        egui::pos2(anchor_x - 260.0, anchor_y + 98.0),
        egui::pos2(anchor_x - 190.0, anchor_y + 90.0),
        egui::pos2(anchor_x - 120.0, anchor_y + 82.0),
        egui::pos2(anchor_x - 45.0, anchor_y + 78.0),
        egui::pos2(anchor_x, anchor_y + 84.0),
        egui::pos2(anchor_x, anchor_y + 150.0),
        egui::pos2(anchor_x - 260.0, anchor_y + 150.0),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(
        hill_pts,
        Color32::from_rgb(12, 10, 14),
        Stroke::NONE,
    ));

    let hill_trees: &[(f32, f32, f32, f32)] = &[
        (-240.0, 94.0, 18.0, 8.0),
        (-215.0, 90.0, 21.0, 9.0),
        (-190.0, 87.0, 19.0, 8.5),
        (-165.0, 84.0, 23.0, 9.5),
        (-140.0, 81.0, 20.0, 8.5),
    ];
    for &(dx, dy, h, w) in hill_trees {
        draw_pine_tree(
            painter,
            egui::pos2(anchor_x + dx, anchor_y + dy),
            h,
            w,
            Color32::from_rgb(14, 11, 16),
        );
    }

    let castle_color = Color32::from_rgb(9, 7, 11);
    let window_color = Color32::from_rgb(255, 155, 30);

    let tower_center_x = anchor_x - 52.0;
    let tower_base_y = anchor_y + 78.0;
    let tower_roof_base_y = anchor_y + 32.0;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(tower_center_x - 10.0, tower_roof_base_y),
            egui::pos2(tower_center_x + 10.0, tower_base_y),
        ),
        CornerRadius::ZERO,
        castle_color,
    );
    let spire_pts = vec![
        egui::pos2(tower_center_x - 11.5, tower_roof_base_y),
        egui::pos2(tower_center_x + 11.5, tower_roof_base_y),
        egui::pos2(tower_center_x, anchor_y + 8.0),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(
        spire_pts,
        castle_color,
        Stroke::NONE,
    ));
    painter.line_segment(
        [
            egui::pos2(tower_center_x, anchor_y + 8.0),
            egui::pos2(tower_center_x, anchor_y + 2.0),
        ],
        Stroke::new(1.0_f32, castle_color),
    );
    painter.line_segment(
        [
            egui::pos2(tower_center_x - 2.5, anchor_y + 5.0),
            egui::pos2(tower_center_x + 2.5, anchor_y + 5.0),
        ],
        Stroke::new(1.0_f32, castle_color),
    );

    let win1_center = egui::pos2(tower_center_x, anchor_y + 42.0);
    let win1_rect = egui::Rect::from_center_size(win1_center, egui::vec2(4.8, 7.5));
    painter.rect_filled(win1_rect, CornerRadius::same(1), window_color);
    painter.line_segment(
        [
            egui::pos2(win1_rect.left(), win1_rect.center().y),
            egui::pos2(win1_rect.right(), win1_rect.center().y),
        ],
        Stroke::new(0.8_f32, castle_color),
    );
    painter.line_segment(
        [
            egui::pos2(win1_rect.center().x, win1_rect.top()),
            egui::pos2(win1_rect.center().x, win1_rect.bottom()),
        ],
        Stroke::new(0.8_f32, castle_color),
    );

    let win2_center = egui::pos2(tower_center_x, anchor_y + 58.0);
    let win2_rect = egui::Rect::from_center_size(win2_center, egui::vec2(4.5, 6.5));
    painter.rect_filled(win2_rect, CornerRadius::same(1), window_color);

    let left_tower_x = anchor_x - 76.0;
    let left_roof_base_y = anchor_y + 42.0;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(left_tower_x - 8.0, left_roof_base_y),
            egui::pos2(left_tower_x + 8.0, tower_base_y),
        ),
        CornerRadius::ZERO,
        castle_color,
    );
    let left_spire = vec![
        egui::pos2(left_tower_x - 9.5, left_roof_base_y),
        egui::pos2(left_tower_x + 9.5, left_roof_base_y),
        egui::pos2(left_tower_x, anchor_y + 18.0),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(
        left_spire,
        castle_color,
        Stroke::NONE,
    ));
    painter.line_segment(
        [
            egui::pos2(left_tower_x, anchor_y + 18.0),
            egui::pos2(left_tower_x, anchor_y + 12.0),
        ],
        Stroke::new(1.0_f32, castle_color),
    );
    let win_left_center = egui::pos2(left_tower_x, anchor_y + 50.0);
    painter.rect_filled(
        egui::Rect::from_center_size(win_left_center, egui::vec2(4.0, 6.5)),
        CornerRadius::same(1),
        window_color,
    );

    let far_left_x = anchor_x - 95.0;
    let far_left_roof_y = anchor_y + 52.0;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(far_left_x - 6.0, far_left_roof_y),
            egui::pos2(far_left_x + 6.0, tower_base_y),
        ),
        CornerRadius::ZERO,
        castle_color,
    );
    let far_left_spire = vec![
        egui::pos2(far_left_x - 7.5, far_left_roof_y),
        egui::pos2(far_left_x + 7.5, far_left_roof_y),
        egui::pos2(far_left_x, anchor_y + 32.0),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(
        far_left_spire,
        castle_color,
        Stroke::NONE,
    ));

    let wall_y = anchor_y + 56.0;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(left_tower_x + 8.0, wall_y),
            egui::pos2(tower_center_x - 10.0, tower_base_y),
        ),
        CornerRadius::ZERO,
        castle_color,
    );
    for cx in [left_tower_x + 10.0, left_tower_x + 15.0] {
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(cx, wall_y - 3.0), egui::vec2(2.5, 3.0)),
            CornerRadius::ZERO,
            castle_color,
        );
    }

    let right_wing_x = anchor_x - 34.0;
    let right_wall_y = anchor_y + 58.0;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(tower_center_x + 10.0, right_wall_y),
            egui::pos2(right_wing_x + 6.0, tower_base_y),
        ),
        CornerRadius::ZERO,
        castle_color,
    );
    for cx in [tower_center_x + 12.0, tower_center_x + 17.0] {
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(cx, right_wall_y - 3.0), egui::vec2(2.5, 3.0)),
            CornerRadius::ZERO,
            castle_color,
        );
    }
    let win_right_center = egui::pos2(right_wing_x, anchor_y + 66.0);
    painter.rect_filled(
        egui::Rect::from_center_size(win_right_center, egui::vec2(3.5, 5.5)),
        CornerRadius::same(1),
        window_color,
    );

    draw_dead_tree(
        painter,
        egui::pos2(anchor_x - 20.0, anchor_y + 78.0),
        54.0,
        castle_color,
    );

    let bat_color = Color32::from_rgb(16, 12, 18);
    draw_bat_scaled(
        painter,
        egui::pos2(anchor_x - 148.0, anchor_y + 26.0),
        0.72,
        bat_color,
    );
    draw_bat_scaled(
        painter,
        egui::pos2(anchor_x - 124.0, anchor_y + 44.0),
        0.52,
        bat_color,
    );
    draw_bat_scaled(
        painter,
        egui::pos2(anchor_x - 180.0, anchor_y + 36.0),
        0.42,
        bat_color,
    );
    draw_bat_scaled(
        painter,
        egui::pos2(anchor_x - 245.0, anchor_y + 50.0),
        0.36,
        bat_color,
    );
}

pub fn draw_halloween_sidebar_artwork(painter: &egui::Painter, rect: egui::Rect) {
    if rect.height() < 240.0 {
        return;
    }
    let bot = rect.bottom();
    let left = rect.left();
    let right = rect.right();
    let w = rect.width();

    let back_tree_color = Color32::from_rgb(18, 14, 22);
    let back_trees: &[(f32, f32, f32, f32)] = &[
        (0.88, 130.0, 180.0, 32.0),
        (0.72, 110.0, 150.0, 28.0),
        (0.50, 100.0, 120.0, 24.0),
        (0.30, 95.0, 100.0, 22.0),
        (0.12, 90.0, 80.0, 18.0),
    ];
    for &(fx, dy, h, tw) in back_trees {
        draw_pine_tree(
            painter,
            egui::pos2(left + w * fx, bot - dy),
            h,
            tw,
            back_tree_color,
        );
    }

    let mist_color1 = Color32::from_rgba_unmultiplied(42, 26, 44, 28);
    painter.rect_filled(
        egui::Rect::from_min_max(egui::pos2(left, bot - 110.0), egui::pos2(right, bot - 86.0)),
        CornerRadius::same(6),
        mist_color1,
    );

    let house_color = Color32::from_rgb(10, 8, 12);
    let roof_color = Color32::from_rgb(8, 6, 10);
    let house_center_x = right - 44.0;
    let house_base_y = bot - 98.0;
    let house_roof_y = bot - 128.0;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(house_center_x - 22.0, house_roof_y),
            egui::pos2(house_center_x + 22.0, house_base_y),
        ),
        CornerRadius::ZERO,
        house_color,
    );
    let roof_pts = vec![
        egui::pos2(house_center_x - 26.0, house_roof_y),
        egui::pos2(house_center_x + 26.0, house_roof_y),
        egui::pos2(house_center_x, bot - 158.0),
    ];
    painter.add(egui::epaint::PathShape::convex_polygon(
        roof_pts,
        roof_color,
        Stroke::NONE,
    ));
    painter.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(house_center_x + 12.0, bot - 154.0),
            egui::vec2(4.0, 10.0),
        ),
        CornerRadius::ZERO,
        roof_color,
    );
    painter.line_segment(
        [
            egui::pos2(house_center_x, bot - 158.0),
            egui::pos2(house_center_x, bot - 164.0),
        ],
        Stroke::new(1.0_f32, roof_color),
    );

    let win_fill = Color32::from_rgb(255, 150, 30);

    let win_upper_center = egui::pos2(house_center_x + 13.0, bot - 132.0);
    let win_upper = egui::Rect::from_center_size(win_upper_center, egui::vec2(6.0, 10.0));
    painter.rect_filled(win_upper, CornerRadius::same(1), win_fill);
    painter.line_segment(
        [
            egui::pos2(win_upper.left(), win_upper.center().y),
            egui::pos2(win_upper.right(), win_upper.center().y),
        ],
        Stroke::new(0.8_f32, house_color),
    );
    painter.line_segment(
        [
            egui::pos2(win_upper.center().x, win_upper.top()),
            egui::pos2(win_upper.center().x, win_upper.bottom()),
        ],
        Stroke::new(0.8_f32, house_color),
    );

    let win_lower_center = egui::pos2(house_center_x - 12.0, bot - 110.0);
    let win_lower = egui::Rect::from_center_size(win_lower_center, egui::vec2(7.0, 11.0));
    painter.rect_filled(win_lower, CornerRadius::same(1), win_fill);
    painter.line_segment(
        [
            egui::pos2(win_lower.left(), win_lower.center().y),
            egui::pos2(win_lower.right(), win_lower.center().y),
        ],
        Stroke::new(0.8_f32, house_color),
    );
    painter.line_segment(
        [
            egui::pos2(win_lower.center().x, win_lower.top()),
            egui::pos2(win_lower.center().x, win_lower.bottom()),
        ],
        Stroke::new(0.8_f32, house_color),
    );

    let front_tree_color = Color32::from_rgb(13, 10, 15);
    let front_trees: &[(f32, f32, f32, f32)] = &[
        (0.92, 94.0, 160.0, 30.0),
        (0.66, 94.0, 130.0, 24.0),
        (0.44, 94.0, 95.0, 20.0),
        (0.24, 94.0, 72.0, 17.0),
        (0.08, 94.0, 52.0, 15.0),
    ];
    for &(fx, dy, h, tw) in front_trees {
        draw_pine_tree(
            painter,
            egui::pos2(left + w * fx, bot - dy),
            h,
            tw,
            front_tree_color,
        );
    }

    let bat_color = Color32::from_rgb(18, 14, 22);
    draw_bat_scaled(
        painter,
        egui::pos2(left + w * 0.62, bot - 225.0),
        0.55,
        bat_color,
    );
    draw_bat_scaled(
        painter,
        egui::pos2(left + w * 0.44, bot - 270.0),
        0.45,
        bat_color,
    );

    draw_small_pumpkin(
        painter,
        egui::pos2(house_center_x - 16.0, house_base_y + 1.0),
        9.0,
        -0.10,
        true,
    );
    draw_small_pumpkin(
        painter,
        egui::pos2(house_center_x + 15.0, house_base_y + 2.0),
        8.0,
        0.08,
        false,
    );
}

pub fn play_hero_split_button(
    ui: &mut egui::Ui,
    text: &str,
    menu: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let total_w = ui.available_width();
    let split_w = 40.0_f32;
    let main_w = (total_w - split_w - 4.0).max(120.0);
    let p = crate::ui::theme::palette(ui.ctx());
    let mut play_resp = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let resp = button_sized(ui, text, Tone::Primary, egui::vec2(main_w, 44.0));
        play_resp = Some(resp);
        ui.scope(|ui| {
            let widgets = &mut ui.style_mut().visuals.widgets;
            widgets.inactive.bg_fill = p.accent;
            widgets.inactive.weak_bg_fill = p.accent;
            widgets.hovered.bg_fill = p.accent_hover;
            widgets.hovered.weak_bg_fill = p.accent_hover;
            widgets.active.bg_fill = p.accent_hover;
            widgets.active.weak_bg_fill = p.accent_hover;
            widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, p.accent_text);
            widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, p.accent_text);
            widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, p.accent_text);
            widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, p.accent);
            widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, p.accent_hover);
            let launch_corner = egui::CornerRadius::same(crate::ui::theme::metrics::CONTROL_RADIUS);
            widgets.inactive.corner_radius = launch_corner;
            widgets.hovered.corner_radius = launch_corner;
            widgets.active.corner_radius = launch_corner;
            ui.style_mut().spacing.button_padding = egui::vec2(12.0, 10.0);
            ui.style_mut().spacing.interact_size = egui::vec2(split_w, 44.0);
            let menu_resp = ui.menu_button("", menu);
            let r = menu_resp.response.rect;
            let c = r.center();
            let stroke = egui::Stroke::new(1.8_f32, p.accent_text);
            let p1 = c + egui::vec2(-5.5, -3.0);
            let p2 = c + egui::vec2(0.0, 3.5);
            let p3 = c + egui::vec2(5.5, -3.0);
            ui.painter().line_segment([p1, p2], stroke);
            ui.painter().line_segment([p2, p3], stroke);
            ui.painter().circle_filled(p1, 0.9, p.accent_text);
            ui.painter().circle_filled(p2, 0.9, p.accent_text);
            ui.painter().circle_filled(p3, 0.9, p.accent_text);
            menu_resp.response.on_hover_text("Launch options");
        });
    });
    play_resp.unwrap()
}

pub fn draw_instance_banner_fallback(
    painter: &egui::Painter,
    rect: egui::Rect,
    name: &str,
    loader: &str,
) {
    let p_theme = Color32::from_rgb(22, 24, 32);
    painter.rect_filled(rect, CornerRadius::ZERO, p_theme);
    let icon_size = (rect.height() * 0.60).clamp(24.0, 42.0);
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(icon_size, icon_size));
    draw_instance_thumbnail(painter, icon_rect, name, loader, false);
}

pub fn badge(ui: &mut egui::Ui, text: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    let corner = CornerRadius::same(metrics::PILL_RADIUS);
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(corner)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(type_scale::CAPTION).color(TEXT2));
        });
}

pub fn content_primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    button(ui, text, Tone::Primary)
}

pub fn action_button_with_feedback(ui: &mut egui::Ui, text: &str, busy: bool) -> egui::Response {
    if !busy {
        return button(ui, text, Tone::Primary);
    }
    let p = crate::ui::theme::palette(ui.ctx());
    ui.add_enabled(
        false,
        egui::Button::new(RichText::new(text).size(type_scale::BODY))
            .min_size(egui::vec2(220.0, metrics::BUTTON_H))
            .fill(p.accent),
    )
    .on_hover_cursor(egui::CursorIcon::Wait)
}

pub fn compact_action_button_with_feedback(
    ui: &mut egui::Ui,
    text: &str,
    busy: bool,
) -> egui::Response {
    if !busy {
        return compact_button(ui, text, Tone::Primary);
    }
    let p = crate::ui::theme::palette(ui.ctx());
    ui.add_enabled(
        false,
        egui::Button::new(RichText::new(text).size(type_scale::BODY))
            .min_size(egui::vec2(160.0, 28.0))
            .fill(p.accent),
    )
    .on_hover_cursor(egui::CursorIcon::Wait)
}

pub fn spinner(ui: &mut egui::Ui, size: f32) {
    let p = crate::ui::theme::palette(ui.ctx());
    let origin = ui.cursor().min;
    let rect = egui::Rect::from_min_size(origin, egui::vec2(size, size));
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = size / 2.0 - 1.2;
    for i in 0..8_u32 {
        let angle = std::f32::consts::FRAC_PI_2 * (i as f32) / 4.0;
        let alpha = 0.15 + 0.85 * ((i as f32) / 8.0).min(1.0);
        let dir = egui::pos2(angle.cos(), angle.sin());
        painter.line_segment(
            [
                center + dir.to_vec2() * (radius - 2.2),
                center + dir.to_vec2() * radius,
            ],
            egui::Stroke::new(1.8_f32, p.accent.gamma_multiply(alpha)),
        );
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(90));
}

pub fn content_secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    secondary_button(ui, text)
}

pub fn badge_accent(ui: &mut egui::Ui, text: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    let corner = CornerRadius::same(metrics::PILL_RADIUS);
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(corner)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(type_scale::CAPTION)
                    .strong()
                    .color(p.accent),
            );
        });
}

pub fn badge_boost(ui: &mut egui::Ui, text: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    let corner = CornerRadius::same(metrics::PILL_RADIUS);
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(corner)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(type_scale::CAPTION)
                    .strong()
                    .color(p.accent),
            );
        });
}

pub fn badge_ok(ui: &mut egui::Ui, text: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    let corner = CornerRadius::same(metrics::PILL_RADIUS);
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(corner)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(type_scale::CAPTION)
                    .strong()
                    .color(OK),
            );
        });
}

pub fn badge_warning(ui: &mut egui::Ui, text: &str) {
    let corner = CornerRadius::same(metrics::PILL_RADIUS);
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(225, 175, 70, 20))
        .stroke(Stroke::new(1.0_f32, WARNING))
        .corner_radius(corner)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(type_scale::CAPTION).color(WARNING));
        });
}

pub fn stat(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(crate::ui::theme::caption(label));
        ui.label(
            RichText::new(value)
                .size(type_scale::BODY)
                .strong()
                .color(TEXT),
        );
    });
}

pub fn hover_card_frame(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let id = ui.make_persistent_id(id_salt);
    let hovered = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    let fade = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), hovered, 0.15);
    let p = crate::ui::theme::palette(ui.ctx());
    let fill = p.elevated.lerp_to_gamma(p.elevated2, fade);
    let border_color = p.border;

    let corner = CornerRadius::same(metrics::CARD_RADIUS);
    let frame_resp = egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border_color))
        .corner_radius(corner)
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    gloss_highlight(ui, frame_resp.response.rect);

    let resp = ui.interact(frame_resp.response.rect, id, egui::Sense::hover());
    ui.ctx().data_mut(|d| d.insert_temp(id, resp.hovered()));
    if fade > 0.001 && fade < 0.999 {
        ui.ctx().request_repaint();
    }
    resp
}

pub fn compact_hover_card_frame(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let id = ui.make_persistent_id(id_salt);
    let hovered = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    let fade = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), hovered, 0.15);
    let p = crate::ui::theme::palette(ui.ctx());
    let fill = p.elevated.lerp_to_gamma(p.elevated2, fade);
    let border_color = p.border;

    let corner = CornerRadius::same(metrics::CARD_RADIUS);
    let frame_resp = egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border_color))
        .corner_radius(corner)
        .inner_margin(egui::Margin::symmetric(14, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    gloss_highlight(ui, frame_resp.response.rect);

    let resp = ui.interact(frame_resp.response.rect, id, egui::Sense::hover());
    ui.ctx().data_mut(|d| d.insert_temp(id, resp.hovered()));
    if fade > 0.001 && fade < 0.999 {
        ui.ctx().request_repaint();
    }
    resp
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

pub fn button(ui: &mut egui::Ui, text: &str, tone: Tone) -> egui::Response {
    button_sized(ui, text, tone, egui::vec2(0.0, metrics::BUTTON_H))
}

pub fn compact_button(ui: &mut egui::Ui, text: &str, tone: Tone) -> egui::Response {
    button_sized(ui, text, tone, egui::vec2(0.0, 28.0))
}

pub fn compact_secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    compact_button(ui, text, Tone::Secondary)
}

pub fn compact_danger_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    compact_button(ui, text, Tone::Danger)
}

pub fn button_sized(ui: &mut egui::Ui, text: &str, tone: Tone, size: egui::Vec2) -> egui::Response {
    let p = crate::ui::theme::palette(ui.ctx());
    let (normal, hover, pressed, foreground, border) = match tone {
        Tone::Primary => (
            p.accent,
            p.accent_hover,
            p.accent.lerp_to_gamma(p.accent_text, 0.14),
            p.accent_text,
            p.accent,
        ),
        Tone::Secondary => (
            p.elevated2,
            p.hover,
            p.hover.lerp_to_gamma(p.accent, 0.12),
            TEXT,
            p.border,
        ),
        Tone::Ghost => (
            Color32::TRANSPARENT,
            p.elevated2,
            p.elevated2,
            TEXT2,
            Color32::TRANSPARENT,
        ),
        Tone::Danger => (
            p.elevated.lerp_to_gamma(DANGER, 0.12),
            p.elevated.lerp_to_gamma(DANGER, 0.24),
            p.elevated.lerp_to_gamma(DANGER, 0.32),
            DANGER,
            p.border.lerp_to_gamma(DANGER, 0.55),
        ),
    };
    let corner = CornerRadius::same(metrics::CONTROL_RADIUS);
    ui.scope(|ui| {
        let widgets = &mut ui.style_mut().visuals.widgets;
        for (widget, fill, stroke) in [
            (&mut widgets.inactive, normal, border),
            (
                &mut widgets.hovered,
                hover,
                if border == Color32::TRANSPARENT {
                    p.border
                } else {
                    border.lerp_to_gamma(foreground, 0.30)
                },
            ),
            (&mut widgets.active, pressed, border),
        ] {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.fg_stroke = Stroke::new(1.0_f32, foreground);
            widget.bg_stroke = Stroke::new(1.0_f32, stroke);
            widget.corner_radius = corner;
            widget.expansion = 0.0;
        }
        ui.add(
            egui::Button::new(RichText::new(text).size(type_scale::BODY))
                .wrap_mode(egui::TextWrapMode::Extend)
                .min_size(size)
                .fill(normal)
                .corner_radius(corner),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}

pub fn button_row(
    ui: &mut egui::Ui,
    cancel: &str,
    confirm: &str,
    confirm_tone: Tone,
) -> (egui::Response, egui::Response) {
    let width = ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).max(96.0);
    let left = button_sized(
        ui,
        cancel,
        Tone::Secondary,
        egui::vec2(width, metrics::BUTTON_H),
    );
    let right = button_sized(
        ui,
        confirm,
        confirm_tone,
        egui::vec2(width, metrics::BUTTON_H),
    );
    (left, right)
}

pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    button_sized(
        ui,
        text,
        Tone::Primary,
        egui::vec2(metrics::BUTTON_W, metrics::BUTTON_H),
    )
}

pub fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    button(ui, text, Tone::Secondary)
}

pub fn action_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    secondary_button(ui, text)
}

pub fn danger_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    button(ui, text, Tone::Danger)
}

pub fn sized_danger_button(ui: &mut egui::Ui, text: &str, width: f32) -> egui::Response {
    button_sized(ui, text, Tone::Danger, egui::vec2(width, metrics::BUTTON_H))
}

pub fn tab_button(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    button_sized(
        ui,
        label,
        if selected {
            Tone::Primary
        } else {
            Tone::Secondary
        },
        egui::vec2(110.0, 34.0),
    )
}

pub fn pill_tab_button(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let p = crate::ui::theme::palette(ui.ctx());
    let corner = CornerRadius::same(metrics::PILL_RADIUS);
    let fill = if selected { p.accent } else { p.elevated2 };
    let text_color = if selected { p.accent_text } else { TEXT2 };
    let stroke = if selected {
        Stroke::NONE
    } else {
        Stroke::new(1.0_f32, p.border)
    };

    ui.scope(|ui| {
        let widgets = &mut ui.style_mut().visuals.widgets;
        widgets.inactive.bg_fill = fill;
        widgets.inactive.weak_bg_fill = fill;
        widgets.inactive.bg_stroke = stroke;
        widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text_color);
        widgets.inactive.corner_radius = corner;

        widgets.hovered.bg_fill = if selected { p.accent_hover } else { p.hover };
        widgets.hovered.weak_bg_fill = widgets.hovered.bg_fill;
        widgets.hovered.bg_stroke = if selected {
            Stroke::NONE
        } else {
            Stroke::new(1.0_f32, p.border.lerp_to_gamma(Color32::WHITE, 0.2))
        };
        widgets.hovered.fg_stroke = Stroke::new(
            1.0_f32,
            if selected {
                p.accent_text
            } else {
                Color32::WHITE
            },
        );
        widgets.hovered.corner_radius = corner;

        widgets.active.bg_fill = fill;
        widgets.active.weak_bg_fill = fill;
        widgets.active.bg_stroke = stroke;
        widgets.active.fg_stroke = Stroke::new(1.0_f32, text_color);
        widgets.active.corner_radius = corner;

        ui.style_mut().spacing.button_padding = egui::vec2(12.0, 5.0);
        let btn_text = if selected {
            RichText::new(label)
                .size(type_scale::CAPTION)
                .strong()
                .color(text_color)
        } else {
            RichText::new(label)
                .size(type_scale::CAPTION)
                .color(text_color)
        };
        ui.add(
            egui::Button::new(btn_text)
                .wrap_mode(egui::TextWrapMode::Extend)
                .fill(fill)
                .stroke(stroke)
                .corner_radius(corner)
                .min_size(egui::vec2(0.0, 26.0)),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}

pub fn sized_action_button(
    ui: &mut egui::Ui,
    label: &str,
    primary: bool,
    width: f32,
) -> egui::Response {
    button_sized(
        ui,
        label,
        if primary {
            Tone::Primary
        } else {
            Tone::Secondary
        },
        egui::vec2(width, metrics::BUTTON_H),
    )
}

pub fn draw_cute_avatar(
    painter: &egui::Painter,
    rect: egui::Rect,
    username: &str,
    is_microsoft: bool,
) {
    let center = rect.center();
    let size = rect.width().min(rect.height());
    let r = size * 0.44;

    let hash = username
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let base_colors = [
        Color32::from_rgb(180, 205, 237),
        Color32::from_rgb(255, 195, 205),
        Color32::from_rgb(200, 230, 201),
        Color32::from_rgb(225, 190, 231),
        Color32::from_rgb(255, 224, 178),
        Color32::from_rgb(178, 235, 242),
    ];
    let bg_color = base_colors[(hash as usize) % base_colors.len()];

    let ear_dx = r * 0.55;
    let ear_dy = r * 0.62;
    let ear_r = r * 0.32;
    painter.circle_filled(center + egui::vec2(-ear_dx, -ear_dy), ear_r, bg_color);
    painter.circle_filled(
        center + egui::vec2(-ear_dx, -ear_dy),
        ear_r * 0.52,
        Color32::from_rgba_unmultiplied(255, 140, 160, 200),
    );
    painter.circle_filled(center + egui::vec2(ear_dx, -ear_dy), ear_r, bg_color);
    painter.circle_filled(
        center + egui::vec2(ear_dx, -ear_dy),
        ear_r * 0.52,
        Color32::from_rgba_unmultiplied(255, 140, 160, 200),
    );

    painter.circle_filled(center, r, bg_color);
    painter.circle_stroke(
        center,
        r,
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 45)),
    );

    let eye_dx = r * 0.34;
    let eye_y = center.y - r * 0.04;
    let eye_r = (r * 0.16).max(1.8);
    let eye_color = Color32::from_rgb(28, 30, 42);

    let left_eye = egui::pos2(center.x - eye_dx, eye_y);
    painter.circle_filled(left_eye, eye_r, eye_color);
    painter.circle_filled(
        left_eye + egui::vec2(eye_r * 0.3, -eye_r * 0.3),
        (eye_r * 0.36).max(1.0),
        Color32::WHITE,
    );

    let right_eye = egui::pos2(center.x + eye_dx, eye_y);
    painter.circle_filled(right_eye, eye_r, eye_color);
    painter.circle_filled(
        right_eye + egui::vec2(eye_r * 0.3, -eye_r * 0.3),
        (eye_r * 0.36).max(1.0),
        Color32::WHITE,
    );

    let blush_r = (r * 0.17).max(1.5);
    let blush_y = eye_y + eye_r * 1.45;
    painter.circle_filled(
        egui::pos2(center.x - eye_dx * 1.25, blush_y),
        blush_r,
        Color32::from_rgba_unmultiplied(255, 120, 150, 160),
    );
    painter.circle_filled(
        egui::pos2(center.x + eye_dx * 1.25, blush_y),
        blush_r,
        Color32::from_rgba_unmultiplied(255, 120, 150, 160),
    );

    let nose_pos = egui::pos2(center.x, center.y + r * 0.13);
    painter.circle_filled(
        nose_pos,
        (r * 0.08).max(1.0),
        Color32::from_rgb(240, 100, 130),
    );

    let mouth_y = center.y + r * 0.26;
    let mouth_w = (r * 0.16).max(1.5);
    painter.line_segment(
        [
            egui::pos2(center.x - mouth_w, mouth_y),
            egui::pos2(center.x, mouth_y + (r * 0.08).max(1.0)),
        ],
        Stroke::new(1.2_f32, Color32::from_rgb(60, 45, 55)),
    );
    painter.line_segment(
        [
            egui::pos2(center.x, mouth_y + (r * 0.08).max(1.0)),
            egui::pos2(center.x + mouth_w, mouth_y),
        ],
        Stroke::new(1.2_f32, Color32::from_rgb(60, 45, 55)),
    );

    if is_microsoft {
        let badge_r = (r * 0.30).clamp(2.5, 7.0);
        let badge_pos = center + egui::vec2(r * 0.65, -r * 0.65);
        painter.circle_filled(badge_pos, badge_r, Color32::from_rgb(255, 204, 0));
        painter.circle_stroke(
            badge_pos,
            badge_r,
            Stroke::new(1.0_f32, Color32::from_rgb(180, 140, 0)),
        );
        painter.text(
            badge_pos,
            egui::Align2::CENTER_CENTER,
            "★",
            egui::FontId::proportional((badge_r * 1.3).max(4.0)),
            Color32::from_rgb(60, 45, 0),
        );
    }
}

static AVATAR_DECODED: Mutex<Option<HashMap<String, egui::ColorImage>>> = Mutex::new(None);
static AVATAR_PENDING: Mutex<Option<std::collections::HashSet<String>>> = Mutex::new(None);

pub fn draw_avatar(painter: &egui::Painter, rect: egui::Rect, username: &str, uuid: &str) {
    let ctx = painter.ctx();

    if let Ok(mut lock) = AVATAR_DECODED.lock() {
        if let Some(map) = lock.as_mut() {
            if let Some(cimg) = map.remove(uuid) {
                let texture = ctx.load_texture(
                    format!("avatar_{uuid}"),
                    cimg,
                    egui::TextureOptions::NEAREST,
                );
                ctx.data_mut(|data| {
                    let mut textures = data
                        .get_persisted::<HashMap<String, egui::TextureHandle>>(egui::Id::new(
                            "avatar_textures",
                        ))
                        .unwrap_or_default();
                    textures.insert(uuid.to_string(), texture);
                    data.insert_persisted(egui::Id::new("avatar_textures"), textures);
                });
            }
        }
    }

    let texture = ctx.data_mut(|data| {
        data.get_persisted::<HashMap<String, egui::TextureHandle>>(egui::Id::new("avatar_textures"))
            .and_then(|textures| textures.get(uuid).cloned())
    });

    if let Some(texture) = texture {
        painter.image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        return;
    }

    draw_cute_avatar(painter, rect, username, true);

    let clean_uuid = uuid.replace('-', "");
    if clean_uuid.len() != 32 {
        return;
    }

    let should_fetch = if let Ok(mut lock) = AVATAR_PENDING.lock() {
        let set = lock.get_or_insert_with(std::collections::HashSet::new);
        set.insert(uuid.to_string())
    } else {
        false
    };

    if should_fetch {
        let ctx = ctx.clone();
        let target_uuid = uuid.to_string();
        tokio::spawn(async move {
            let result = fetch_player_skin_head(&clean_uuid).await;
            if let Some(cimg) = result {
                if let Ok(mut lock) = AVATAR_DECODED.lock() {
                    let map = lock.get_or_insert_with(HashMap::new);
                    map.insert(target_uuid, cimg);
                }
                ctx.request_repaint();
            }
        });
    }
}

async fn fetch_player_skin_head(clean_uuid: &str) -> Option<egui::ColorImage> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .ok()?;

    let url = format!("https://sessionserver.mojang.com/session/minecraft/profile/{clean_uuid}");
    let response = client.get(&url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let profile: Value = response.json().await.ok()?;
    let value = profile["properties"]
        .as_array()?
        .iter()
        .find(|property| property["name"].as_str() == Some("textures"))?
        .get("value")?
        .as_str()?;

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value)
        .ok()?;
    let textures: Value = serde_json::from_slice(&decoded).ok()?;
    let skin_url = textures["textures"]["SKIN"]["url"].as_str()?;

    let response = client.get(skin_url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let bytes = response.bytes().await.ok()?;
    let skin = image::load_from_memory(&bytes).ok()?.to_rgba8();

    if skin.width() < 48 || skin.height() < 16 {
        return None;
    }

    let mut face = image::RgbaImage::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            face.put_pixel(x, y, *skin.get_pixel(8 + x, 8 + y));
        }
    }
    for y in 0..8 {
        for x in 0..8 {
            let pixel = skin.get_pixel(40 + x, 8 + y);
            if pixel.0[3] > 0 {
                face.put_pixel(x, y, *pixel);
            }
        }
    }

    let raw = face.into_raw();
    Some(egui::ColorImage::from_rgba_unmultiplied([8, 8], &raw))
}

pub fn draw_instance_thumbnail(
    painter: &egui::Painter,
    rect: egui::Rect,
    _name: &str,
    loader: &str,
    is_selected: bool,
) {
    let p = rect.min;
    let w = rect.width();
    let h = rect.height();

    let block_rect = rect.shrink(1.0);
    painter.rect(
        block_rect,
        CornerRadius::ZERO,
        Color32::from_rgb(32, 35, 44),
        Stroke::new(
            if is_selected { 1.5_f32 } else { 1.0_f32 },
            if is_selected {
                Color32::from_rgb(140, 175, 255)
            } else {
                Color32::from_rgb(60, 65, 80)
            },
        ),
        egui::StrokeKind::Inside,
    );

    let loader_lower = loader.to_ascii_lowercase();
    let (top_color, side_color, symbol) = if loader_lower.contains("fabric") {
        (
            Color32::from_rgb(215, 195, 160),
            Color32::from_rgb(70, 130, 180),
            "F",
        )
    } else if loader_lower.contains("neoforge") {
        (
            Color32::from_rgb(255, 140, 60),
            Color32::from_rgb(180, 70, 20),
            "N",
        )
    } else if loader_lower.contains("forge") {
        (
            Color32::from_rgb(220, 110, 50),
            Color32::from_rgb(140, 55, 25),
            "⚙",
        )
    } else if loader_lower.contains("quilt") {
        (
            Color32::from_rgb(180, 140, 220),
            Color32::from_rgb(110, 60, 160),
            "Q",
        )
    } else {
        (
            Color32::from_rgb(92, 160, 54),
            Color32::from_rgb(134, 96, 67),
            "⛏",
        )
    };

    let pad = 2.0_f32;
    let inner_w = (w - pad * 2.0).max(1.0);
    let inner_h = (h - pad * 2.0).max(1.0);
    let top_h = inner_h * 0.38;

    let top_rect = egui::Rect::from_min_size(p + egui::vec2(pad, pad), egui::vec2(inner_w, top_h));
    painter.rect_filled(top_rect, CornerRadius::ZERO, top_color);

    let fringe_y = p.y + pad + top_h;
    let bot_rect = egui::Rect::from_min_size(
        egui::pos2(p.x + pad, fringe_y),
        egui::vec2(inner_w, (inner_h - top_h).max(1.0)),
    );
    painter.rect_filled(bot_rect, CornerRadius::ZERO, side_color);

    let step = inner_w / 4.0;
    for i in 0..4 {
        let fx = p.x + pad + (i as f32) * step;
        let drop = if i % 2 == 0 { 3.0 } else { 1.5 };
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(fx, fringe_y), egui::vec2(step, drop)),
            0,
            top_color,
        );
    }

    painter.text(
        rect.center() + egui::vec2(0.0, 1.0),
        egui::Align2::CENTER_CENTER,
        symbol,
        egui::FontId::proportional((h * 0.36).max(10.0)),
        Color32::from_rgba_unmultiplied(255, 255, 255, 230),
    );
}

pub fn render_instance_thumbnail(
    ui: &mut egui::Ui,
    size: f32,
    name: &str,
    loader: &str,
    is_selected: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    draw_instance_thumbnail(ui.painter(), rect, name, loader, is_selected);
    resp
}

#[allow(clippy::too_many_arguments)]
pub fn instance_choice(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    title: &str,
    subtitle: &str,
    selected: bool,
) -> egui::Response {
    ui.push_id(id_salt, |ui| {
        let p = crate::ui::theme::palette(ui.ctx());
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 72.0), egui::Sense::click());
        let fill = if response.is_pointer_button_down_on() {
            p.hover
        } else if selected {
            p.elevated2
        } else if response.hovered() {
            p.elevated.lerp_to_gamma(p.hover, 0.65)
        } else {
            p.elevated
        };
        ui.painter().rect(
            rect,
            metrics::CARD_RADIUS,
            fill,
            Stroke::new(
                if selected { 1.5_f32 } else { 1.0_f32 },
                if selected { p.accent } else { p.border },
            ),
            egui::StrokeKind::Inside,
        );
        if selected {
            let mark = egui::Rect::from_min_size(
                rect.left_top() + egui::vec2(0.0, 19.0),
                egui::vec2(3.0, 34.0),
            );
            ui.painter().rect_filled(mark, 2, p.accent);
        }
        let icon = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(13.0, 17.0),
            egui::vec2(38.0, 38.0),
        );
        draw_instance_thumbnail(ui.painter(), icon, title, subtitle, selected);
        let text_rect = egui::Rect::from_min_max(
            rect.left_top() + egui::vec2(63.0, 10.0),
            rect.right_bottom() - egui::vec2(88.0, 8.0),
        );
        let painter = ui.painter().with_clip_rect(text_rect);
        painter.text(
            rect.left_top() + egui::vec2(63.0, 16.0),
            egui::Align2::LEFT_TOP,
            title,
            egui::FontId::proportional(14.0),
            TEXT,
        );
        painter.text(
            rect.left_top() + egui::vec2(63.0, 40.0),
            egui::Align2::LEFT_TOP,
            subtitle,
            egui::FontId::proportional(type_scale::CAPTION),
            TEXT2,
        );
        if selected {
            let selected_text = if crate::ui::theme::current_theme(ui.ctx())
                == crate::config::ThemeKind::Halloween
            {
                "Selected 🎃"
            } else {
                "Selected"
            };
            ui.painter().text(
                rect.right_center() - egui::vec2(15.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                selected_text,
                egui::FontId::proportional(type_scale::CAPTION),
                p.accent,
            );
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                ui.is_enabled(),
                selected,
                title,
            )
        });
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}

#[must_use]
pub fn elide(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

pub fn thin_progress(ui: &mut egui::Ui, frac: Option<f32>) {
    let p = crate::ui::theme::palette(ui.ctx());
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    ui.painter().rect_filled(rect, 3, p.elevated2);
    match frac {
        Some(f) => {
            let target = f.clamp(0.0, 1.0);
            let id = resp.id.with("thin_progress_val");
            let val = ui.ctx().animate_value_with_time(id, target, 0.12);
            if val > 0.001 {
                let w = rect.width() * val;
                let r = egui::Rect::from_min_size(rect.min, egui::vec2(w, rect.height()));
                ui.painter().rect_filled(r, 3, p.accent);
            }
            if (val - target).abs() > 0.002 {
                ui.ctx().request_repaint();
            }
        }
        None => {
            let t = ui.ctx().input(|i| i.time);
            let w = (rect.width() * 0.35).max(28.0);
            let span = (rect.width() - w).max(0.0);
            let x = rect.min.x + (0.5 - 0.5 * (t * 2.2).cos()) as f32 * span;
            let r =
                egui::Rect::from_min_size(egui::pos2(x, rect.min.y), egui::vec2(w, rect.height()));
            ui.painter().rect_filled(r, 3, p.accent);
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
    }
}

pub fn progress_row(ui: &mut egui::Ui, frac: Option<f32>, detail: Option<ProgressDetail<'_>>) {
    thin_progress(ui, frac);
    let Some(detail) = detail else {
        return;
    };
    let left = match (frac, detail.completed, detail.total) {
        (Some(f), _, _) => crate::ui::theme::format_percent(f),
        (None, Some(c), Some(t)) if t > 0 => format!("{c}/{t}"),
        _ => String::new(),
    };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(left)
                .size(type_scale::CAPTION)
                .strong()
                .color(TEXT2),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(eta) = detail.eta {
                if !eta.is_empty() {
                    ui.label(
                        RichText::new(eta.to_string())
                            .size(type_scale::CAPTION)
                            .color(MUTED),
                    );
                }
            }
            if let Some(speed) = detail.speed_bps {
                if speed > 0.0 {
                    ui.label(
                        RichText::new(crate::ui::theme::format_speed(speed))
                            .size(type_scale::CAPTION)
                            .color(MUTED),
                    );
                }
            }
            if let (Some(downloaded), Some(total)) = (detail.downloaded, detail.total) {
                ui.label(
                    RichText::new(format!(
                        "{} / {}",
                        crate::ui::theme::format_bytes(downloaded),
                        crate::ui::theme::format_bytes(total)
                    ))
                    .size(type_scale::CAPTION)
                    .color(TEXT2),
                );
            }
        });
    });
}

#[derive(Clone, Copy, Default)]
pub struct ProgressDetail<'a> {
    pub downloaded: Option<u64>,
    pub total: Option<u64>,
    pub speed_bps: Option<f64>,
    pub completed: Option<usize>,
    pub eta: Option<&'a str>,
}

pub fn empty_state(ui: &mut egui::Ui, title: &str, hint: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(28.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(54.0, 54.0), egui::Sense::hover());
        let center = rect.center();
        ui.painter().circle_filled(center, 26.0_f32, ELEVATED2);
        ui.painter()
            .circle_stroke(center, 26.0_f32, Stroke::new(1.0_f32, BORDER));

        ui.add_space(14.0);
        magnifier(ui.painter(), center + egui::vec2(3.0, 3.0), 9.0, TEXT2);
        ui.add_space(14.0);
        ui.label(RichText::new(title).size(19.0).strong().color(TEXT));
        ui.label(RichText::new(hint).size(12.5).color(TEXT2));
        ui.add_space(18.0);
    });
}

pub mod limits {

    pub const USERNAME: usize = 16;

    pub const INSTANCE_NAME: usize = 48;

    pub const JVM_ARGS: usize = 512;

    pub const GAME_ARGS: usize = 512;

    pub const SEARCH: usize = 128;

    pub const ADDRESS: usize = 255;

    pub const PATH: usize = 512;

    pub const WORLD_NAME: usize = 64;

    pub const CHECKSUM: usize = 128;
}

pub fn magnifier(painter: &egui::Painter, center: egui::Pos2, radius: f32, color: Color32) {
    let stroke = Stroke::new(radius * 0.22, color);
    painter.circle_stroke(
        center - egui::vec2(radius * 0.25, radius * 0.25),
        radius,
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(radius * 0.55, radius * 0.55),
            center + egui::vec2(radius * 1.15, radius * 1.15),
        ],
        stroke,
    );
}

pub fn search_field(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut String,
    max: usize,
    hint: &str,
) -> (egui::Response, bool) {
    let p = crate::ui::theme::palette(ui.ctx());
    let corner = CornerRadius::same(10);
    let response = egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(corner)
        .inner_margin(egui::Margin::symmetric(12, 0))
        .show(ui, |ui| {
            ui.set_height(44.0);
            ui.horizontal_centered(|ui| {
                let (glass, _) =
                    ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                magnifier(ui.painter(), glass.center(), 5.0, MUTED);
                ui.add_space(10.0);
                let field = ui.add(
                    egui::TextEdit::singleline(value)
                        .id(egui::Id::new(id))
                        .hint_text(hint)
                        .char_limit(max)
                        .font(egui::TextStyle::Heading)
                        .frame(false)
                        .desired_width((ui.available_width() - 24.0).max(80.0)),
                );
                if !value.is_empty() {
                    let (clear, _) =
                        ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
                    if ui
                        .interact(clear, egui::Id::new((id, "clear")), egui::Sense::click())
                        .on_hover_text("Clear")
                        .clicked()
                    {
                        value.clear();
                    }
                    ui.painter().text(
                        clear.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{2715}",
                        egui::FontId::proportional(11.0),
                        MUTED,
                    );
                }
                field
            })
        })
        .response;
    let enter = response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    (response, enter)
}

pub fn compact_search_field(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut String,
    max: usize,
    hint: &str,
) -> (egui::Response, bool) {
    let p = crate::ui::theme::palette(ui.ctx());
    let field_corner = CornerRadius::same(metrics::CONTROL_RADIUS);
    let response = egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(field_corner)
        .inner_margin(egui::Margin::symmetric(10, 0))
        .show(ui, |ui| {
            ui.set_height(32.0);
            ui.horizontal_centered(|ui| {
                let (glass, _) =
                    ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                magnifier(ui.painter(), glass.center(), 4.0, MUTED);
                ui.add_space(6.0);
                let field = ui.add(
                    egui::TextEdit::singleline(value)
                        .id(egui::Id::new(id))
                        .hint_text(hint)
                        .char_limit(max)
                        .font(egui::TextStyle::Body)
                        .frame(false)
                        .desired_width((ui.available_width() - 20.0).max(60.0)),
                );
                if !value.is_empty() {
                    let (clear, _) =
                        ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
                    if ui
                        .interact(
                            clear,
                            egui::Id::new((id, "compact_clear")),
                            egui::Sense::click(),
                        )
                        .on_hover_text("Clear")
                        .clicked()
                    {
                        value.clear();
                    }
                    ui.painter().text(
                        clear.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{2715}",
                        egui::FontId::proportional(10.0),
                        MUTED,
                    );
                }
                field
            })
        })
        .response;
    let enter = response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    (response, enter)
}

pub fn loader_pills(
    ui: &mut egui::Ui,
    selected: &str,
    available: &[crate::instance::LoaderKind],
) -> Option<String> {
    if available.is_empty() {
        return None;
    }
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        for kind in available.iter().copied() {
            let id = kind.as_str();
            let label = kind.display_name();
            let active = selected == id;
            let width = 40.0 + label.len() as f32 * 8.5;
            let p = crate::ui::theme::palette(ui.ctx());
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(width, 36.0), egui::Sense::click());
            if active {
                ui.painter().rect_filled(rect, 18, p.accent);
            } else if response.hovered() {
                ui.painter().rect_stroke(
                    rect,
                    18,
                    Stroke::new(1.0_f32, p.border),
                    egui::StrokeKind::Inside,
                );
            }
            let color = if active { p.accent_text } else { TEXT2 };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(type_scale::BODY),
                color,
            );
            focus_ring(ui, &response, label);
            if response.clicked() {
                picked = Some(id.to_string());
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
    });
    picked
}
pub fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(12.0).color(TEXT2));
}

#[must_use]
pub fn dialog_backdrop(ctx: &egui::Context) -> bool {
    let mut clicked = false;
    ctx.memory_mut(|memory| {
        memory.set_modal_layer(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("dialog-backdrop"),
        ))
    });
    egui::Area::new(egui::Id::new("dialog-backdrop"))
        .order(egui::Order::Middle)
        .fixed_pos(egui::Pos2::ZERO)
        .interactable(true)
        .show(ctx, |ui| {
            let rect = ui.ctx().screen_rect();
            crate::ui::backdrop::paint(ctx, ui.painter(), true);
            let fade = ctx.animate_bool_with_time(egui::Id::new("dialog-shade-enter"), true, 0.14);
            ui.painter()
                .rect_filled(rect, 0, Color32::from_black_alpha((110.0 * fade) as u8));
            let response = ui.interact(rect, ui.id().with("hit"), egui::Sense::click());
            if response.clicked() {
                clicked = true;
            }
        });
    clicked
}

pub fn limited_text_edit(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut String,
    max: usize,
    hint: &str,
) -> egui::Response {
    let used = value.chars().count();
    let response = ui.add(
        egui::TextEdit::singleline(value)
            .id(egui::Id::new(id))
            .hint_text(hint)
            .char_limit(max)
            .desired_width(ui.available_width())
            .margin(egui::vec2(10.0, 8.0))
            .min_size(egui::vec2(0.0, 34.0)),
    );
    if used >= max.saturating_mul(4) / 5 {
        let near_limit = used as f64 / max as f64 > 0.9;
        let colour = if used >= max {
            DANGER
        } else if near_limit {
            WARNING
        } else {
            MUTED
        };
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.label(
                RichText::new(format!("{used}/{max}"))
                    .size(11.0)
                    .color(colour),
            );
        });
    }
    response
}

pub fn limited_text_edit_with_hint(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut String,
    max: usize,
    hint: &str,
    below: &str,
) -> egui::Response {
    let response = limited_text_edit(ui, id, value, max, hint);
    ui.label(RichText::new(below).size(11.0).color(MUTED));
    response
}

pub fn focus_ring(ui: &egui::Ui, response: &egui::Response, label: &str) {
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.shrink(3.0),
            metrics::CONTROL_RADIUS,
            Stroke::new(2.0_f32, crate::ui::theme::palette(ui.ctx()).bg),
            egui::StrokeKind::Inside,
        );
        ui.painter().rect_stroke(
            response.rect.shrink(1.0),
            metrics::CONTROL_RADIUS,
            Stroke::new(2.0_f32, crate::ui::theme::palette(ui.ctx()).accent),
            egui::StrokeKind::Inside,
        );
    }
}

pub fn activity_indicator(ui: &mut egui::Ui, label: &str) {
    ui.horizontal_wrapped(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            let angle = ui.input(|input| input.time) as f32 * 4.0;
            let color = crate::ui::theme::palette(ui.ctx()).accent;
            for index in 0..8 {
                let theta = angle + index as f32 * std::f32::consts::TAU / 8.0;
                let position = rect.center() + egui::vec2(theta.cos(), theta.sin()) * 6.0;
                ui.painter().circle_filled(
                    position,
                    1.6,
                    color.gamma_multiply((index + 1) as f32 / 8.0),
                );
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
        ui.label(RichText::new(label).strong().color(TEXT));
    });
}

pub fn dialog_content(ui: &mut egui::Ui) {
    let enter = ui
        .ctx()
        .animate_bool_with_time(egui::Id::new("dialog-content-enter"), true, 0.14);
    ui.set_opacity(0.65 + enter * 0.35);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hovering_another_instance_keeps_selection_and_geometry() {
        let ctx = egui::Context::default();
        crate::ui::theme::apply_theme(&ctx);
        let draw = |selected: usize, pointer: Option<egui::Pos2>| {
            let mut rects = [egui::Rect::NOTHING; 2];
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 400.0),
                )),
                events: pointer
                    .map(|pos| vec![egui::Event::PointerMoved(pos)])
                    .unwrap_or_default(),
                ..Default::default()
            };
            let output = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    for (index, rect) in rects.iter_mut().enumerate() {
                        *rect = instance_choice(
                            ui,
                            index,
                            "Test instance",
                            "Minecraft 1.21 · Fabric",
                            selected == index,
                        )
                        .rect;
                    }
                });
            });
            let strokes: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::Shape::Rect(shape) = &shape.shape {
                        if rects.contains(&shape.rect) {
                            return Some(shape.stroke.color);
                        }
                    }
                    None
                })
                .collect();
            (rects, strokes)
        };
        let (initial, _) = draw(0, None);
        let (hovered, strokes) = draw(0, Some(initial[1].center()));
        assert_eq!(initial, hovered);
        assert_eq!(strokes[0], crate::ui::theme::palette(&ctx).accent);
        assert_ne!(strokes[1], crate::ui::theme::palette(&ctx).accent);
        let (switched, _) = draw(1, Some(initial[1].center()));
        assert_eq!(initial, switched);
    }

    #[test]
    fn button_tones_render_distinct_fills() {
        let ctx = egui::Context::default();
        crate::ui::theme::apply_theme(&ctx);
        let fills = |tone: Tone| {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 200.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        button(ui, "Go", tone);
                    });
                },
            );
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(r) if r.fill.is_opaque() => Some(r.fill),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let palette = crate::ui::theme::palette(&ctx);
        let primary = fills(Tone::Primary);
        let secondary = fills(Tone::Secondary);
        let danger = fills(Tone::Danger);
        let ghost = fills(Tone::Ghost);

        assert_eq!(
            primary.last().copied(),
            Some(palette.accent),
            "primary must use the accent"
        );
        assert_eq!(
            secondary.last().copied(),
            Some(palette.elevated2),
            "secondary must use the raised surface"
        );
        assert_ne!(
            danger.last().copied(),
            Some(palette.accent),
            "danger must not read as the primary accent"
        );
        assert_eq!(ghost.len() + 1, secondary.len(), "ghost paints no fill");
    }

    #[test]
    fn button_row_gives_both_buttons_the_same_width() {
        let ctx = egui::Context::default();
        crate::ui::theme::apply_theme(&ctx);
        let mut widths = Vec::new();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 200.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(320.0);
                    let (back, next) = button_row(ui, "Back", "Continue", Tone::Primary);
                    widths.push((back.rect.width(), next.rect.width()));
                });
            },
        );
        let (back, next) = widths[0];
        assert_eq!(back, next, "Back and Continue must be the same width");
        assert!(next > 96.0);
    }

    #[test]
    fn step_state_tracks_progress() {
        assert_eq!(step_state(0, 0), StepState::Active);
        assert_eq!(step_state(1, 0), StepState::Todo);
        assert_eq!(step_state(0, 2), StepState::Done);
        assert_eq!(step_state(2, 2), StepState::Active);
        assert_eq!(step_state(1, 2), StepState::Done);
    }

    #[test]
    fn step_rail_renders_one_dot_per_step() {
        let ctx = egui::Context::default();
        crate::ui::theme::apply_theme(&ctx);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 300.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    step_rail(ui, 3, 1);
                });
            },
        );
        let dots = output
            .shapes
            .iter()
            .filter(|s| match &s.shape {
                egui::Shape::Circle(circle) => circle.stroke == egui::Stroke::NONE,
                _ => false,
            })
            .count();
        assert_eq!(dots, 3, "one filled dot per step");
    }

    #[test]
    fn wizard_frame_caches_a_usable_card_height() {
        let ctx = egui::Context::default();
        crate::ui::theme::apply_theme(&ctx);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 700.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let panel_height = ui.available_height();
                    wizard_frame(ui, 0, panel_height, 0, |ui, _| {
                        ui.label("content");
                    });
                });
            },
        );
        let cached = ctx.data(|d| d.get_temp::<f32>(egui::Id::new(("wizard-height", 0usize))));
        assert!(
            cached.is_some_and(|h| h > 0.0),
            "wizard_frame must cache its card height so later frames can centre it"
        );
    }

    #[test]
    fn progress_row_renders_bar_and_status() {
        let ctx = egui::Context::default();
        crate::ui::theme::apply_theme(&ctx);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 300.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(360.0);
                    progress_row(
                        ui,
                        Some(0.5),
                        Some(ProgressDetail {
                            downloaded: Some(50 * 1024),
                            total: Some(100 * 1024),
                            speed_bps: Some(1024.0),
                            completed: None,
                            eta: Some("50s left"),
                        }),
                    );
                });
            },
        );

        let rects = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(r) => Some(r.rect),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            rects.len() >= 3,
            "bar track, bar fill and status rows expected"
        );
        assert!(rects.iter().any(|r| r.width() > 10.0));
    }

    #[test]
    fn elide_respects_char_boundaries() {
        assert_eq!(elide("short", 12), "short");
        assert_eq!(elide("exactly-12!", 12), "exactly-12!");
        assert_eq!(elide("abcdefgh", 4), "abc\u{2026}");

        assert_eq!(elide("\u{65e5}\u{672c}\u{8a9e}", 2), "\u{65e5}\u{2026}");
        assert_eq!(elide("\u{65e5}\u{672c}\u{8a9e}", 1), "\u{2026}");
    }

    fn run_through_limit(value: &str, max: usize) -> String {
        let ctx = egui::Context::default();
        crate::ui::theme::apply_theme(&ctx);
        let mut stored = String::new();
        let mut focus_at = egui::Pos2::ZERO;

        let draw = |ctx: &egui::Context, stored: &mut String, focus_at: &mut egui::Pos2| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(400.0);
                let response = limited_text_edit(ui, "limit-probe", stored, max, "");
                focus_at.x = response.rect.center().x;
                focus_at.y = response.rect.center().y;
            });
        };

        let screen = |events| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 300.0),
            )),
            events,
            ..Default::default()
        };

        let _ = ctx.run(screen(Vec::new()), |ctx| {
            draw(ctx, &mut stored, &mut focus_at)
        });
        for pressed in [true, false] {
            let _ = ctx.run(
                screen(vec![
                    egui::Event::PointerMoved(focus_at),
                    egui::Event::PointerButton {
                        pos: focus_at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::default(),
                    },
                ]),
                |ctx| draw(ctx, &mut stored, &mut focus_at),
            );
        }
        let _ = ctx.run(screen(vec![egui::Event::Text(value.to_string())]), |ctx| {
            draw(ctx, &mut stored, &mut focus_at)
        });
        stored
    }

    #[test]
    fn limited_fields_cap_overlong_pastes() {
        assert_eq!(
            run_through_limit("Not_Subaka", limits::USERNAME),
            "Not_Subaka"
        );
        let long = "a".repeat(200);
        assert_eq!(
            run_through_limit(&long, limits::USERNAME).chars().count(),
            limits::USERNAME
        );
    }

    #[test]
    fn limits_are_ordered_by_purpose() {
        let checks: Vec<(&str, usize, usize)> = vec![
            ("USERNAME", limits::USERNAME, 16),
            ("INSTANCE_NAME", limits::INSTANCE_NAME, 48),
            ("JVM_ARGS", limits::JVM_ARGS, 1024),
            ("GAME_ARGS", limits::GAME_ARGS, 1024),
            ("SEARCH", limits::SEARCH, 256),
            ("ADDRESS", limits::ADDRESS, 512),
            ("PATH", limits::PATH, 1024),
            ("WORLD_NAME", limits::WORLD_NAME, 128),
            ("CHECKSUM", limits::CHECKSUM, 128),
        ];
        for (name, value, ceiling) in checks {
            assert!(value > 0, "{name} must allow at least one character");
            assert!(
                value <= ceiling,
                "{name} ({value}) exceeds its ceiling {ceiling}"
            );
        }

        let floors: Vec<(&str, usize, usize)> = vec![
            ("USERNAME", limits::USERNAME, 16),
            ("CHECKSUM", limits::CHECKSUM, 128),
        ];
        for (name, value, floor) in floors {
            assert!(
                value >= floor,
                "{name} ({value}) is below the required {floor}"
            );
        }
        let ordered: Vec<(&str, usize)> =
            vec![("SEARCH", limits::SEARCH), ("ADDRESS", limits::ADDRESS)];
        assert!(
            ordered[0].1 < ordered[1].1,
            "a search query must be shorter than an address"
        );
    }

    #[test]
    fn search_field_is_taller_than_a_plain_text_edit() {
        let ctx = egui::Context::default();
        crate::ui::theme::apply_theme(&ctx);
        let measure = |big: bool| {
            let mut out = 0.0_f32;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 300.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.set_width(400.0);
                        let mut value = String::new();
                        if big {
                            let (response, _) = search_field(ui, "big", &mut value, 128, "Search");
                            out = response.rect.height();
                        } else {
                            out = ui.add(egui::TextEdit::singleline(&mut value)).rect.height();
                        }
                    });
                },
            );
            out
        };
        let (big, plain) = (measure(true), measure(false));
        assert!(
            big > plain,
            "the search field ({big}) must read larger than a standard input ({plain})"
        );
    }

    #[test]
    fn pumpkins_draw_without_panicking() {
        let ctx = egui::Context::default();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_halloween_shell_artwork(ui.painter(), ui.max_rect());
                    draw_halloween_sidebar_artwork(ui.painter(), ui.max_rect());
                    draw_small_pumpkin(ui.painter(), egui::pos2(100.0, 100.0), 10.0, 0.0, true);
                    draw_small_pumpkin(ui.painter(), egui::pos2(120.0, 100.0), 8.0, 0.15, false);
                });
            },
        );
    }

    #[test]
    fn compact_hover_card_frame_is_denser_than_standard() {
        let ctx = egui::Context::default();
        let measure = |compact: bool| -> f32 {
            let mut h = 0.0;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 600.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let resp = if compact {
                            compact_hover_card_frame(ui, "card", |ui| {
                                ui.label("content");
                            })
                        } else {
                            hover_card_frame(ui, "card", |ui| {
                                ui.label("content");
                            })
                        };
                        h = resp.rect.height();
                    });
                },
            );
            h
        };
        let compact_h = measure(true);
        let standard_h = measure(false);
        assert!(
            compact_h < standard_h,
            "compact {compact_h} must be less than standard {standard_h}"
        );
    }
}
