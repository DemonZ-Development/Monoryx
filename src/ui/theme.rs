use egui::{Color32, CornerRadius, Stroke, Visuals};

pub const BG: Color32 = Color32::from_rgb(11, 12, 14);
pub const ELEVATED: Color32 = Color32::from_rgb(18, 19, 22);
pub const ELEVATED2: Color32 = Color32::from_rgb(23, 24, 29);
pub const BORDER: Color32 = Color32::from_rgb(35, 37, 43);
pub const BORDER_ACCENT: Color32 = Color32::from_rgb(60, 63, 72);
pub const HOVER: Color32 = Color32::from_rgb(28, 29, 34);

pub const ACCENT: Color32 = Color32::from_rgb(255, 255, 255);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(238, 238, 242);
pub const ACCENT_MUTED: Color32 = Color32::from_rgb(45, 47, 54);

pub const ECO_MODE: Color32 = Color32::from_rgb(205, 210, 218);
pub const ECO_MODE_HOVER: Color32 = Color32::from_rgb(235, 238, 245);
pub const ECO_MODE_BG: Color32 = Color32::from_rgb(23, 24, 29);

pub const BOOST: Color32 = ECO_MODE;
pub const BOOST_HOVER: Color32 = ECO_MODE_HOVER;
pub const BOOST_BG: Color32 = ECO_MODE_BG;

pub const TEXT: Color32 = Color32::from_rgb(245, 245, 247);
pub const TEXT2: Color32 = Color32::from_rgb(136, 142, 155);
pub const MUTED: Color32 = Color32::from_rgb(98, 103, 114);

pub const SELECTED: Color32 = ACCENT;
pub const SELECTED_FG: Color32 = Color32::from_rgb(0, 0, 0);

pub const DANGER: Color32 = Color32::from_rgb(224, 90, 90);
pub const OK: Color32 = Color32::from_rgb(175, 180, 190);
pub const WARNING: Color32 = Color32::from_rgb(225, 175, 70);
pub const INFO: Color32 = Color32::from_rgb(205, 210, 218);

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.dark_mode = true;
    visuals.panel_fill = BG;
    visuals.window_fill = ELEVATED;
    visuals.extreme_bg_color = BG;
    visuals.code_bg_color = ELEVATED2;
    visuals.faint_bg_color = ELEVATED;

    visuals.widgets.noninteractive.bg_fill = ELEVATED;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT2);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);

    visuals.widgets.inactive.bg_fill = ELEVATED2;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);

    visuals.widgets.hovered.bg_fill = HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(50, 53, 62));

    visuals.widgets.active.bg_fill = SELECTED;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, SELECTED_FG);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, SELECTED);

    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 30);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);

    visuals.widgets.inactive.weak_bg_fill = ELEVATED2;
    visuals.widgets.hovered.weak_bg_fill = HOVER;
    visuals.widgets.active.weak_bg_fill = SELECTED;

    visuals.widgets.open.bg_fill = ELEVATED2;
    visuals.widgets.open.weak_bg_fill = ELEVATED2;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, BORDER);

    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(8);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.animation_time = 0.18;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.interact_size = egui::vec2(38.0, 34.0);
    style.spacing.text_edit_width = 260.0;
    style.spacing.combo_height = 260.0;

    for (kind, size) in [
        (egui::TextStyle::Body, 13.5),
        (egui::TextStyle::Button, 13.5),
        (egui::TextStyle::Small, 11.5),
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

pub fn apply_monochrome(ctx: &egui::Context) {
    apply_theme(ctx);
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
        egui::RichText::new("Play. Modify. Nothing else.")
            .size(11.0)
            .color(MUTED),
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

pub fn format_last_played(raw: Option<&str>) -> String {
    let Some(raw) = raw else {
        return "Never".to_string();
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("never") {
        return "Never".to_string();
    }

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        use chrono::Datelike;
        let local_dt = dt.with_timezone(&chrono::Local);
        let now = chrono::Local::now();
        let duration = now.signed_duration_since(local_dt);

        if duration.num_seconds().abs() < 60 {
            "Just now".to_string()
        } else if duration.num_minutes() > 0 && duration.num_minutes() < 60 {
            format!("{}m ago", duration.num_minutes())
        } else if now.date_naive() == local_dt.date_naive() {
            format!("Today at {}", local_dt.format("%I:%M %p"))
        } else if (now.date_naive() - chrono::Duration::days(1)) == local_dt.date_naive() {
            format!("Yesterday at {}", local_dt.format("%I:%M %p"))
        } else if duration.num_days() > 0 && duration.num_days() < 7 {
            local_dt.format("%a at %I:%M %p").to_string()
        } else if now.year() == local_dt.year() {
            local_dt.format("%b %d, %I:%M %p").to_string()
        } else {
            local_dt.format("%b %d, %Y").to_string()
        }
    } else if trimmed.len() >= 10
        && trimmed.chars().nth(4) == Some('-')
        && trimmed.chars().nth(7) == Some('-')
    {
        trimmed[..10].to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_last_played_empty_or_none() {
        assert_eq!(format_last_played(None), "Never");
        assert_eq!(format_last_played(Some("")), "Never");
        assert_eq!(format_last_played(Some("   ")), "Never");
        assert_eq!(format_last_played(Some("Never")), "Never");
        assert_eq!(format_last_played(Some("never")), "Never");
    }

    #[test]
    fn format_last_played_past_date() {
        let past = "2020-05-15T14:30:00.000000000+00:00";
        let formatted = format_last_played(Some(past));
        assert!(formatted.contains("2020"));
        assert!(formatted.contains("May"));
    }

    #[test]
    fn format_last_played_raw_nanosecond_utc_collapsing() {
        let raw = "2026-09-22T10:41:13.902901900+00:00";
        let formatted = format_last_played(Some(raw));
        assert!(!formatted.contains("902901900"));
        assert!(!formatted.contains("+00:00"));
        assert!(formatted.len() < 24);
    }
}
