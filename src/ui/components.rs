use crate::ui::theme::{
    ACCENT, ACCENT_HOVER, BORDER, BORDER_ACCENT, BOOST, BOOST_BG, BOOST_HOVER, DANGER, ELEVATED,
    ELEVATED2, HOVER, MUTED, OK, SELECTED_FG, TEXT, TEXT2, WARNING,
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

pub fn card_frame(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(ELEVATED)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });
}

pub use crate::ui::theme::format_last_played;

pub fn hero_card_frame(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(ELEVATED)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });
}

pub fn badge(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(ELEVATED2)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).color(TEXT2));
        });
}

pub fn badge_accent(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(ELEVATED2)
        .stroke(Stroke::new(1.0_f32, BORDER_ACCENT))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).strong().color(TEXT));
        });
}

pub fn badge_boost(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(BOOST_BG)
        .stroke(Stroke::new(1.0_f32, BOOST))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).strong().color(BOOST));
        });
}

pub fn badge_ok(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(ELEVATED2)
        .stroke(Stroke::new(1.0_f32, BORDER))
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
    let fade = ui.ctx().animate_bool_with_time(id.with("hover"), hovered, 0.15);
    let fill = ELEVATED.lerp_to_gamma(ELEVATED2, fade);
    let border_color = BORDER.lerp_to_gamma(Color32::from_rgb(52, 55, 65), fade);

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

pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let id = ui.next_auto_id();
    let hovered = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    let fade = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), hovered, 0.15);
    if fade > 0.001 && fade < 0.999 {
        ui.ctx().request_repaint();
    }
    let fill = ACCENT.lerp_to_gamma(ACCENT_HOVER, fade);
    let btn = egui::Button::new(RichText::new(text).strong().size(13.5).color(SELECTED_FG))
        .fill(fill)
        .corner_radius(CornerRadius::same(8))
        .stroke(Stroke::new(1.0_f32, fill));
    let response = ui.add_sized(egui::vec2(150.0, 38.0), btn);
    ui.ctx()
        .data_mut(|data| data.insert_temp(id, response.hovered()));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn play_hero_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let id = ui.next_auto_id();
    let hovered = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    let fade = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), hovered, 0.15);
    if fade > 0.001 && fade < 0.999 {
        ui.ctx().request_repaint();
    }
    let fill = ACCENT.lerp_to_gamma(ACCENT_HOVER, fade);
    let btn = egui::Button::new(
        RichText::new(text)
            .strong()
            .size(14.0)
            .color(SELECTED_FG),
    )
    .fill(fill)
    .corner_radius(CornerRadius::same(8))
    .stroke(Stroke::NONE);
    let response = ui.add_sized(egui::vec2(150.0, 42.0), btn);
    ui.ctx()
        .data_mut(|data| data.insert_temp(id, response.hovered()));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn boost_toggle_button(ui: &mut egui::Ui, active: bool) -> egui::Response {
    let (fill, stroke_color, text_color, label) = if active {
        (BOOST_BG, BOOST, BOOST_HOVER, "Eco Mode ON")
    } else {
        (ELEVATED2, BORDER, MUTED, "Eco Mode OFF")
    };
    let btn = egui::Button::new(RichText::new(label).strong().size(12.0).color(text_color))
        .fill(fill)
        .corner_radius(CornerRadius::same(8))
        .stroke(Stroke::new(1.0_f32, stroke_color));
    let resp = ui.add_sized(egui::vec2(136.0, 34.0), btn);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

pub fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let id = ui.next_auto_id();
    let hovered = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    let fade = ui.ctx().animate_bool_with_time(id.with("hover"), hovered, 0.12);
    if fade > 0.001 && fade < 0.999 {
        ui.ctx().request_repaint();
    }
    let fill = ELEVATED2.lerp_to_gamma(HOVER, fade);
    let border_color = BORDER.lerp_to_gamma(Color32::from_rgb(55, 58, 67), fade);
    let text_color = TEXT2.lerp_to_gamma(TEXT, fade);
    let btn = egui::Button::new(RichText::new(text).size(13.0).color(text_color))
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border_color))
        .corner_radius(CornerRadius::same(8));
    let resp = ui.add(btn);
    ui.ctx().data_mut(|d| d.insert_temp(id, resp.hovered()));
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

pub fn action_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let id = ui.next_auto_id();
    let hovered = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    let fade = ui.ctx().animate_bool_with_time(id.with("hover"), hovered, 0.12);
    if fade > 0.001 && fade < 0.999 {
        ui.ctx().request_repaint();
    }
    let fill = ELEVATED2.lerp_to_gamma(HOVER, fade);
    let border_color = BORDER.lerp_to_gamma(Color32::from_rgb(55, 58, 67), fade);
    let text_color = TEXT2.lerp_to_gamma(TEXT, fade);
    let btn = egui::Button::new(RichText::new(text).size(12.5).color(text_color))
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border_color))
        .corner_radius(CornerRadius::same(6));
    let resp = ui.add(btn);
    ui.ctx().data_mut(|d| d.insert_temp(id, resp.hovered()));
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

pub fn danger_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let id = ui.next_auto_id();
    let hovered = ui.ctx().data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    let fade = ui.ctx().animate_bool_with_time(id.with("hover"), hovered, 0.12);
    if fade > 0.001 && fade < 0.999 {
        ui.ctx().request_repaint();
    }
    let fill = Color32::from_rgba_unmultiplied(224, 90, 90, if hovered { 40 } else { 22 });
    let stroke_color = DANGER.lerp_to_gamma(Color32::from_rgb(255, 120, 120), fade);
    let btn = egui::Button::new(RichText::new(text).size(13.0).color(DANGER))
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, stroke_color))
        .corner_radius(CornerRadius::same(8));
    let resp = ui.add(btn);
    ui.ctx().data_mut(|d| d.insert_temp(id, resp.hovered()));
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

pub fn danger_label(text: &str) -> RichText {
    RichText::new(text).color(DANGER)
}

pub fn thin_progress(ui: &mut egui::Ui, frac: Option<f32>) {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3, ELEVATED2);
    match frac {
        Some(f) => {
            let target = f.clamp(0.0, 1.0);
            let id = resp.id.with("thin_progress_val");
            let val = ui.ctx().animate_value_with_time(id, target, 0.12);
            if val > 0.001 {
                let w = rect.width() * val;
                let r = egui::Rect::from_min_size(rect.min, egui::vec2(w, rect.height()));
                ui.painter().rect_filled(r, 3, ACCENT);
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
            ui.painter().rect_filled(r, 3, ACCENT);
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
        ui.painter().circle_stroke(center, 26.0_f32, Stroke::new(1.0_f32, BORDER));

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
