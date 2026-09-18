use crate::ui::theme::{BORDER, ELEVATED, ELEVATED2, MUTED, SELECTED, SELECTED_FG, TEXT, TEXT2};
use egui::{Color32, CornerRadius, RichText, Stroke};
pub fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).size(30.0).strong().color(TEXT));
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).size(12.5).color(TEXT2));
    }
    ui.add_space(18.0);
}
pub fn card_frame(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(ELEVATED)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(22))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });
}
pub fn badge(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(ELEVATED2)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(TEXT2));
        });
}
pub fn stat(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(11.0).color(MUTED));
        ui.label(RichText::new(value).size(14.0).color(TEXT));
    });
}
pub fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let id = ui.next_auto_id();
    let hovered = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    let fade = ui
        .ctx()
        .animate_bool_with_time(id.with("hover"), hovered, 0.14);
    let fill = SELECTED.lerp_to_gamma(Color32::WHITE, fade);
    let btn = egui::Button::new(RichText::new(text).strong().color(SELECTED_FG))
        .fill(fill)
        .corner_radius(CornerRadius::same(10))
        .stroke(Stroke::new(1.0_f32, fill));
    let response = ui.add_sized(egui::vec2(166.0, 42.0), btn);
    ui.ctx()
        .data_mut(|data| data.insert_temp(id, response.hovered()));
    response
}
pub fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.button(RichText::new(text).color(TEXT))
}
pub fn danger_label(text: &str) -> RichText {
    RichText::new(text).color(Color32::from_rgb(0xE0, 0x5A, 0x5A))
}
pub fn thin_progress(ui: &mut egui::Ui, frac: Option<f32>) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3, ELEVATED2);
    match frac {
        Some(f) => {
            let f = f.clamp(0.0, 1.0);
            if f > 0.0 {
                let w = rect.width() * f;
                let r = egui::Rect::from_min_size(rect.min, egui::vec2(w, rect.height()));
                ui.painter().rect_filled(r, 3, TEXT);
            }
        }
        None => {
            let t = ui.ctx().input(|i| i.time);
            let w = (rect.width() * 0.35).max(24.0);
            let span = rect.width() - w;
            let x = rect.min.x + (0.5 - 0.5 * (t * 2.2).cos()) as f32 * span;
            let r =
                egui::Rect::from_min_size(egui::pos2(x, rect.min.y), egui::vec2(w, rect.height()));
            ui.painter().rect_filled(r, 3, TEXT);
            ui.ctx().request_repaint();
        }
    }
}
pub fn loading_row(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(RichText::new(text).size(11.0).color(TEXT2));
    });
}
pub fn empty_state(ui: &mut egui::Ui, title: &str, hint: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(34.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 16, ELEVATED2);
        let center = rect.center();
        for offset in [-8.0_f32, 8.0] {
            ui.painter().line_segment(
                [
                    center + egui::vec2(-16.0, offset),
                    center + egui::vec2(16.0, offset),
                ],
                Stroke::new(2.0_f32, TEXT2),
            );
            ui.painter().line_segment(
                [
                    center + egui::vec2(offset, -16.0),
                    center + egui::vec2(offset, 16.0),
                ],
                Stroke::new(2.0_f32, TEXT2),
            );
        }
        ui.add_space(14.0);
        ui.label(RichText::new(title).size(22.0).strong().color(TEXT));
        ui.label(RichText::new(hint).size(13.0).color(TEXT2));
        ui.add_space(24.0);
    });
}
pub fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(12.0).color(TEXT2));
}
