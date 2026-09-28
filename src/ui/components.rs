use crate::ui::theme::{BORDER, DANGER, ELEVATED2, MUTED, OK, TEXT, TEXT2, WARNING};
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

pub fn card_frame(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let p = crate::ui::theme::palette(ui.ctx());
    let frame = egui::Frame::new()
        .fill(p.elevated)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });
    gloss_highlight(ui, frame.response.rect);
}

pub use crate::ui::theme::format_last_played;

pub fn hero_card_frame(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let p = crate::ui::theme::palette(ui.ctx());
    let frame = egui::Frame::new()
        .fill(p.elevated)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(12))
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
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).color(TEXT2));
        });
}

pub fn content_primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    styled_button(ui, text, ButtonTone::Primary, egui::vec2(0.0, 34.0))
}

pub fn content_secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    secondary_button(ui, text)
}

pub fn badge_accent(ui: &mut egui::Ui, text: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.accent))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).strong().color(TEXT));
        });
}

pub fn badge_boost(ui: &mut egui::Ui, text: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.accent))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).strong().color(p.accent));
        });
}

pub fn badge_ok(ui: &mut egui::Ui, text: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).strong().color(OK));
        });
}

pub fn badge_warning(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(225, 175, 70, 20))
        .stroke(Stroke::new(1.0_f32, WARNING))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).color(WARNING));
        });
}

pub fn stat(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(11.0).color(MUTED));
        ui.label(RichText::new(value).size(13.5).strong().color(TEXT));
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
        .corner_radius(CornerRadius::same(10))
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

#[derive(Clone, Copy)]
enum ButtonTone {
    Primary,
    Secondary,
    Danger,
}

fn styled_button(
    ui: &mut egui::Ui,
    text: &str,
    tone: ButtonTone,
    size: egui::Vec2,
) -> egui::Response {
    let p = crate::ui::theme::palette(ui.ctx());
    ui.scope(|ui| {
        let (normal, hover, pressed, foreground, border) = match tone {
            ButtonTone::Primary => (
                p.accent,
                p.accent_hover,
                p.accent.lerp_to_gamma(p.accent_text, 0.14),
                p.accent_text,
                p.accent,
            ),
            ButtonTone::Secondary => (
                p.elevated2,
                p.hover,
                p.hover.lerp_to_gamma(p.accent, 0.12),
                TEXT,
                p.border,
            ),
            ButtonTone::Danger => (
                p.elevated.lerp_to_gamma(DANGER, 0.12),
                p.elevated.lerp_to_gamma(DANGER, 0.24),
                p.elevated.lerp_to_gamma(DANGER, 0.32),
                DANGER,
                p.border.lerp_to_gamma(DANGER, 0.55),
            ),
        };
        let widgets = &mut ui.style_mut().visuals.widgets;
        for (widget, fill, stroke) in [
            (&mut widgets.inactive, normal, border),
            (
                &mut widgets.hovered,
                hover,
                border.lerp_to_gamma(foreground, 0.30),
            ),
            (&mut widgets.active, pressed, border),
        ] {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.fg_stroke = Stroke::new(1.0_f32, foreground);
            widget.bg_stroke = Stroke::new(1.0_f32, stroke);
            widget.corner_radius = CornerRadius::same(7);
            widget.expansion = 0.0;
        }
        ui.add(egui::Button::new(RichText::new(text).size(13.0)).min_size(size))
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}

pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    styled_button(ui, text, ButtonTone::Primary, egui::vec2(100.0, 36.0))
}

pub fn play_hero_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    styled_button(ui, text, ButtonTone::Primary, egui::vec2(150.0, 42.0))
}

pub fn boost_toggle_button(ui: &mut egui::Ui, active: bool) -> egui::Response {
    styled_button(
        ui,
        if active {
            "Eco Mode ON"
        } else {
            "Eco Mode OFF"
        },
        if active {
            ButtonTone::Primary
        } else {
            ButtonTone::Secondary
        },
        egui::vec2(136.0, 34.0),
    )
}

pub fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    styled_button(ui, text, ButtonTone::Secondary, egui::vec2(0.0, 34.0))
}

pub fn action_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    secondary_button(ui, text)
}

pub fn danger_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    styled_button(ui, text, ButtonTone::Danger, egui::vec2(0.0, 34.0))
}

pub fn sized_danger_button(ui: &mut egui::Ui, text: &str, width: f32) -> egui::Response {
    styled_button(ui, text, ButtonTone::Danger, egui::vec2(width, 36.0))
}

pub fn tab_button(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    styled_button(
        ui,
        label,
        if selected {
            ButtonTone::Primary
        } else {
            ButtonTone::Secondary
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
    styled_button(
        ui,
        label,
        if primary {
            ButtonTone::Primary
        } else {
            ButtonTone::Secondary
        },
        egui::vec2(width, 34.0),
    )
}

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
            9,
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
        ui.painter().rect_filled(icon, 7, p.hover);
        ui.painter().text(
            icon.center(),
            egui::Align2::CENTER_CENTER,
            title
                .chars()
                .next()
                .unwrap_or('M')
                .to_uppercase()
                .to_string(),
            egui::FontId::proportional(18.0),
            p.accent,
        );
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
            egui::FontId::proportional(11.5),
            TEXT2,
        );
        if selected {
            ui.painter().text(
                rect.right_center() - egui::vec2(15.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                "Selected",
                egui::FontId::proportional(11.5),
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
        ui.label(RichText::new(text).size(12.0).color(TEXT2));
    });
}

pub fn empty_state(ui: &mut egui::Ui, title: &str, hint: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(28.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(54.0, 54.0), egui::Sense::hover());
        let center = rect.center();
        ui.painter().circle_filled(center, 26.0_f32, ELEVATED2);
        ui.painter()
            .circle_stroke(center, 26.0_f32, Stroke::new(1.0_f32, BORDER));

        ui.painter().circle_stroke(
            center + egui::vec2(-3.0, -3.0),
            9.0_f32,
            Stroke::new(2.0_f32, TEXT2),
        );
        ui.painter().line_segment(
            [
                center + egui::vec2(4.0, 4.0),
                center + egui::vec2(11.0, 11.0),
            ],
            Stroke::new(2.5_f32, TEXT2),
        );
        ui.add_space(14.0);
        ui.label(RichText::new(title).size(19.0).strong().color(TEXT));
        ui.label(RichText::new(hint).size(12.5).color(TEXT2));
        ui.add_space(18.0);
    });
}

pub fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(12.0).color(TEXT2));
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
}
