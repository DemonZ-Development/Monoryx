use crate::ui::theme::{
    metrics, type_scale, BORDER, DANGER, ELEVATED2, MUTED, OK, TEXT, TEXT2, WARNING,
};
use egui::{Color32, CornerRadius, RichText, Stroke};

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

pub fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).size(19.0).strong().color(TEXT));
}

pub fn card_frame(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let p = crate::ui::theme::palette(ui.ctx());
    let frame = egui::Frame::new()
        .fill(p.elevated)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(metrics::CARD_RADIUS))
        .inner_margin(egui::Margin::same(metrics::CARD_MARGIN))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
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
    panel_height: f32,
    add: impl FnOnce(&mut egui::Ui),
) {
    let p = crate::ui::theme::palette(ui.ctx());

    let cache_id = egui::Id::new(("wizard-height", step));
    let known = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(cache_id).unwrap_or(0.0));

    let consumed = panel_height - ui.available_height();
    let block = consumed + known;
    if known > 0.0 && block < panel_height {
        ui.add_space(((panel_height - block) / 2.0).max(0.0));
    }
    ui.vertical_centered(|ui| {
        let frame = egui::Frame::new()
            .fill(p.elevated)
            .stroke(Stroke::new(1.0_f32, p.border))
            .corner_radius(CornerRadius::same(16))
            .inner_margin(egui::Margin::same(28))
            .show(ui, |ui| {
                ui.set_width(metrics::WIZARD_CARD_W.min(ui.available_width()));
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        add,
                    );
                });
            });
        let height = frame.response.rect.height();
        ui.ctx().data_mut(|d| d.insert_temp(cache_id, height));
    });
}

pub fn provider_card(ui: &mut egui::Ui, name: &str, detail: &str, status: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let (tile, _) =
                    ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                ui.painter().rect_filled(tile, 6, p.hover);
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
    let frame = egui::Frame::new()
        .fill(p.elevated)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });
    gloss_highlight(ui, frame.response.rect);
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

pub fn badge(ui: &mut egui::Ui, text: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(metrics::PILL_RADIUS))
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
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.accent))
        .corner_radius(CornerRadius::same(metrics::PILL_RADIUS))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(type_scale::CAPTION)
                    .strong()
                    .color(TEXT),
            );
        });
}

pub fn badge_boost(ui: &mut egui::Ui, text: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.accent))
        .corner_radius(CornerRadius::same(metrics::PILL_RADIUS))
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
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(metrics::PILL_RADIUS))
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
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(225, 175, 70, 20))
        .stroke(Stroke::new(1.0_f32, WARNING))
        .corner_radius(CornerRadius::same(metrics::PILL_RADIUS))
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
    let border_color = p.border.lerp_to_gamma(p.accent, fade * 0.4);

    let frame_resp = egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border_color))
        .corner_radius(CornerRadius::same(metrics::CARD_RADIUS))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });

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
            widget.corner_radius = CornerRadius::same(metrics::CONTROL_RADIUS);
            widget.expansion = 0.0;
        }
        ui.add(
            egui::Button::new(RichText::new(text).size(type_scale::BODY))
                .min_size(size)
                .fill(normal),
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

pub fn play_hero_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    button_sized(ui, text, Tone::Primary, egui::vec2(168.0, 44.0))
}

pub fn boost_toggle_button(ui: &mut egui::Ui, active: bool) -> egui::Response {
    button_sized(
        ui,
        if active {
            "Eco Mode ON"
        } else {
            "Eco Mode OFF"
        },
        if active {
            Tone::Primary
        } else {
            Tone::Secondary
        },
        egui::vec2(136.0, metrics::BUTTON_H),
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
        6.0,
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
    painter.rect_filled(
        top_rect,
        CornerRadius {
            nw: 5,
            ne: 5,
            sw: 0,
            se: 0,
        },
        top_color,
    );

    let fringe_y = p.y + pad + top_h;
    let bot_rect = egui::Rect::from_min_size(
        egui::pos2(p.x + pad, fringe_y),
        egui::vec2(inner_w, (inner_h - top_h).max(1.0)),
    );
    painter.rect_filled(
        bot_rect,
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: 5,
            se: 5,
        },
        side_color,
    );

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
                if selected {
                    p.accent
                } else if response.hovered() {
                    p.border.lerp_to_gamma(p.accent, 0.25)
                } else {
                    p.border
                },
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
            ui.painter().text(
                rect.right_center() - egui::vec2(15.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                "Selected",
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

pub fn danger_label(text: &str) -> RichText {
    RichText::new(text).color(DANGER)
}

pub fn thin_progress(ui: &mut egui::Ui, frac: Option<f32>) {
    let p = crate::ui::theme::palette(ui.ctx());
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
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
            ui.ctx().request_repaint();
        }
    }
}

pub fn loading_row(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(RichText::new(text).size(type_scale::LABEL).color(TEXT2));
    });
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
    let response = egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(
            1.5_f32,
            if value.is_empty() { p.border } else { p.accent },
        ))
        .corner_radius(CornerRadius::same(10))
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
    egui::Area::new(egui::Id::new("dialog-backdrop"))
        .order(egui::Order::Background)
        .fixed_pos(egui::Pos2::ZERO)
        .interactable(true)
        .show(ctx, |ui| {
            let rect = ui.ctx().screen_rect();
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
            .desired_width(ui.available_width()),
    );
    if used > 0 {
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
                    wizard_frame(ui, 0, panel_height, |ui| {
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
}
