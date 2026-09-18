use egui::{Color32, CornerRadius, Stroke, Visuals};
pub const BG: Color32 = Color32::from_rgb(17, 18, 21);
pub const ELEVATED: Color32 = Color32::from_rgb(24, 26, 30);
pub const ELEVATED2: Color32 = Color32::from_rgb(32, 35, 40);
pub const TEXT: Color32 = Color32::from_rgb(0xDE, 0xDE, 0xDE);
pub const TEXT2: Color32 = Color32::from_rgb(0xA0, 0xA0, 0xA0);
pub const MUTED: Color32 = Color32::from_rgb(0x70, 0x70, 0x70);
pub const BORDER: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
pub const HOVER: Color32 = Color32::from_rgb(0x1C, 0x1C, 0x1C);
pub const SELECTED: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);
pub const SELECTED_FG: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0A);
pub const DANGER: Color32 = Color32::from_rgb(0xE0, 0x5A, 0x5A);
pub const OK: Color32 = Color32::from_rgb(0x9A, 0x9A, 0x9A);
pub fn apply_monochrome(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.dark_mode = true;
    visuals.panel_fill = BG;
    visuals.window_fill = ELEVATED;
    visuals.extreme_bg_color = BG;
    visuals.code_bg_color = ELEVATED2;
    visuals.faint_bg_color = ELEVATED;
    visuals.widgets.noninteractive.bg_fill = ELEVATED;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT2);
    visuals.widgets.inactive.bg_fill = ELEVATED2;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.hovered.bg_fill = HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x3A, 0x3A, 0x3A));
    visuals.widgets.active.bg_fill = SELECTED;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, SELECTED_FG);
    visuals.selection.bg_fill = SELECTED;
    visuals.selection.stroke = Stroke::new(1.0_f32, SELECTED_FG);
    visuals.widgets.inactive.weak_bg_fill = ELEVATED2;
    visuals.widgets.hovered.weak_bg_fill = HOVER;
    visuals.widgets.active.weak_bg_fill = SELECTED;
    visuals.widgets.open.bg_fill = ELEVATED2;
    visuals.widgets.open.weak_bg_fill = ELEVATED2;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    ctx.set_visuals(visuals);
    let mut style = (*ctx.style()).clone();
    style.animation_time = 0.18;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(16.0, 10.0);
    style.spacing.interact_size = egui::vec2(40.0, 36.0);
    style.spacing.text_edit_width = 260.0;
    style.spacing.combo_height = 260.0;
    for (kind, size) in [
        (egui::TextStyle::Body, 14.0),
        (egui::TextStyle::Button, 14.0),
        (egui::TextStyle::Small, 12.0),
    ] {
        style
            .text_styles
            .insert(kind, egui::FontId::proportional(size));
    }
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(22.0, egui::FontFamily::Proportional),
    );
    ctx.set_style(style);
}
pub fn wordmark(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("MONORYX")
                .size(20.0)
                .strong()
                .color(TEXT),
        );
    });
    ui.label(
        egui::RichText::new("Minecraft, without the clutter.")
            .size(12.0)
            .color(TEXT2),
    );
}
pub fn format_bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u + 1 < UNITS.len() {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{} {}", n, UNITS[u])
    } else {
        format!("{:.1} {}", v, UNITS[u])
    }
}
pub fn format_speed(bps: f64) -> String {
    format!("{}/s", format_bytes(bps as u64))
}
