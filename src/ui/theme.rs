use crate::config::ThemeKind;
use egui::{Color32, CornerRadius, RichText, Stroke, Visuals};

pub mod metrics {

    pub const BUTTON_H: f32 = 36.0;
    pub const BUTTON_W: f32 = 100.0;
    pub const SIDEBAR_ITEM_H: f32 = 34.0;
    pub const WIZARD_CARD_W: f32 = 460.0;
    pub const ONBOARDING_WINDOW: [f32; 2] = [880.0, 660.0];
    pub const SIDEBAR_MIN: f32 = 200.0;
    pub const SIDEBAR_MAX: f32 = 236.0;

    pub const CARD_RADIUS: u8 = 10;
    pub const CONTROL_RADIUS: u8 = 8;
    pub const PILL_RADIUS: u8 = 6;
    pub const CARD_MARGIN: i8 = 16;
    pub const PAGE_MARGIN: i8 = 26;
}

pub mod type_scale {
    pub const DISPLAY: f32 = 26.0;
    pub const TITLE: f32 = 20.0;
    pub const HEADING: f32 = 19.0;
    pub const BODY: f32 = 13.5;
    pub const LABEL: f32 = 12.0;
    pub const CAPTION: f32 = 11.5;
    pub const MICRO: f32 = 10.5;
}

#[must_use]
pub fn display(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .size(type_scale::DISPLAY)
        .strong()
        .color(TEXT)
}

#[must_use]
pub fn title(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .size(type_scale::TITLE)
        .strong()
        .color(TEXT)
}

#[must_use]
pub fn heading(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .size(type_scale::HEADING)
        .strong()
        .color(TEXT)
}

#[must_use]
pub fn body(text: impl Into<String>) -> RichText {
    RichText::new(text).size(type_scale::BODY).color(TEXT)
}

#[must_use]
pub fn body_secondary(text: impl Into<String>) -> RichText {
    RichText::new(text).size(type_scale::BODY).color(TEXT2)
}

#[must_use]
pub fn label(text: impl Into<String>) -> RichText {
    RichText::new(text).size(type_scale::LABEL).color(TEXT2)
}

#[must_use]
pub fn caption(text: impl Into<String>) -> RichText {
    RichText::new(text).size(type_scale::CAPTION).color(MUTED)
}

#[must_use]
pub fn section_label(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .size(type_scale::MICRO)
        .strong()
        .color(MUTED)
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub elevated: Color32,
    pub elevated2: Color32,
    pub border: Color32,
    pub hover: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_text: Color32,
}

pub fn palette_for(theme: ThemeKind) -> Palette {
    match theme {
        ThemeKind::Monochrome => Palette {
            bg: BG,
            elevated: ELEVATED,
            elevated2: ELEVATED2,
            border: BORDER,
            hover: HOVER,
            accent: ACCENT,
            accent_hover: ACCENT_HOVER,
            accent_text: SELECTED_FG,
        },
        ThemeKind::Gloss => Palette {
            bg: Color32::from_rgb(10, 15, 24),
            elevated: Color32::from_rgb(22, 30, 44),
            elevated2: Color32::from_rgb(31, 42, 59),
            border: Color32::from_rgb(71, 95, 125),
            hover: Color32::from_rgb(42, 58, 79),
            accent: Color32::from_rgb(179, 223, 255),
            accent_hover: Color32::from_rgb(215, 240, 255),
            accent_text: Color32::from_rgb(10, 23, 40),
        },
        ThemeKind::SoftPink => Palette {
            bg: Color32::from_rgb(30, 20, 30),
            elevated: Color32::from_rgb(44, 30, 44),
            elevated2: Color32::from_rgb(57, 38, 57),
            border: Color32::from_rgb(94, 63, 88),
            hover: Color32::from_rgb(70, 45, 68),
            accent: Color32::from_rgb(246, 177, 210),
            accent_hover: Color32::from_rgb(255, 210, 231),
            accent_text: Color32::from_rgb(47, 22, 39),
        },
        ThemeKind::SoftBrown => Palette {
            bg: Color32::from_rgb(29, 23, 19),
            elevated: Color32::from_rgb(43, 33, 27),
            elevated2: Color32::from_rgb(57, 43, 34),
            border: Color32::from_rgb(95, 73, 56),
            hover: Color32::from_rgb(70, 52, 40),
            accent: Color32::from_rgb(221, 183, 143),
            accent_hover: Color32::from_rgb(242, 208, 169),
            accent_text: Color32::from_rgb(45, 29, 20),
        },
    }
}

pub fn palette(ctx: &egui::Context) -> Palette {
    palette_for(current_theme(ctx))
}

pub fn current_theme(ctx: &egui::Context) -> ThemeKind {
    ctx.data(|data| data.get_temp::<ThemeKind>(egui::Id::new("active-theme")))
        .unwrap_or_default()
}

pub const BG: Color32 = Color32::from_rgb(11, 12, 14);
pub const ELEVATED: Color32 = Color32::from_rgb(18, 19, 22);
pub const ELEVATED2: Color32 = Color32::from_rgb(32, 34, 40);
pub const BORDER: Color32 = Color32::from_rgb(53, 56, 65);
pub const BORDER_ACCENT: Color32 = Color32::from_rgb(60, 63, 72);
pub const HOVER: Color32 = Color32::from_rgb(44, 47, 55);

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

pub const TEXT2: Color32 = Color32::from_rgb(170, 176, 188);

pub const MUTED: Color32 = Color32::from_rgb(122, 129, 144);

pub const TEXT_DISABLED: Color32 = Color32::from_rgb(96, 102, 115);

pub const SELECTED: Color32 = ACCENT;
pub const SELECTED_FG: Color32 = Color32::from_rgb(0, 0, 0);

pub const DANGER: Color32 = Color32::from_rgb(224, 90, 90);
pub const OK: Color32 = Color32::from_rgb(175, 180, 190);
pub const WARNING: Color32 = Color32::from_rgb(225, 175, 70);
pub const INFO: Color32 = Color32::from_rgb(205, 210, 218);

pub fn apply_theme(ctx: &egui::Context) {
    apply_selected_theme(ctx, ThemeKind::Monochrome);
}

pub fn apply_selected_theme(ctx: &egui::Context, theme: ThemeKind) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("active-theme"), theme));
    let p = palette_for(theme);
    let mut visuals = Visuals::dark();
    visuals.dark_mode = true;
    visuals.panel_fill = p.bg;
    visuals.window_fill = p.elevated;
    visuals.extreme_bg_color = p.bg;
    visuals.code_bg_color = p.elevated2;
    visuals.faint_bg_color = p.elevated;

    visuals.widgets.noninteractive.bg_fill = p.elevated;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT2);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);

    visuals.widgets.inactive.bg_fill = p.elevated2;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);

    visuals.widgets.hovered.bg_fill = p.hover;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.bg_stroke =
        Stroke::new(1.0_f32, p.border.lerp_to_gamma(p.accent, 0.35));

    visuals.widgets.active.bg_fill = p.hover.lerp_to_gamma(p.accent, 0.12);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, TEXT);

    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, p.accent);

    visuals.selection.bg_fill = p.elevated2.lerp_to_gamma(p.accent, 0.18);
    visuals.selection.stroke = Stroke::new(1.0_f32, p.accent);

    visuals.widgets.inactive.weak_bg_fill = p.elevated2;
    visuals.widgets.hovered.weak_bg_fill = p.hover;
    visuals.widgets.active.weak_bg_fill = visuals.widgets.active.bg_fill;
    visuals.widgets.hovered.expansion = 0.0;
    visuals.widgets.active.expansion = 0.0;

    visuals.widgets.open.bg_fill = p.elevated2;
    visuals.widgets.open.weak_bg_fill = p.elevated2;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, p.border);

    visuals.window_stroke = Stroke::new(1.0_f32, p.border);
    visuals.window_corner_radius = CornerRadius::same(12);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(metrics::CONTROL_RADIUS);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(metrics::CONTROL_RADIUS);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(metrics::CONTROL_RADIUS);
    visuals.widgets.active.corner_radius = CornerRadius::same(metrics::CONTROL_RADIUS);
    visuals.widgets.open.corner_radius = CornerRadius::same(metrics::CONTROL_RADIUS);

    ctx.set_visuals(visuals);

    install_system_fallback_fonts(ctx);

    let mut style = (*ctx.style()).clone();
    style.animation_time = 0.12;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.interact_size = egui::vec2(38.0, metrics::BUTTON_H);
    style.spacing.text_edit_width = 260.0;
    style.spacing.combo_height = 260.0;

    for (kind, size) in [
        (egui::TextStyle::Body, type_scale::BODY),
        (egui::TextStyle::Button, type_scale::BODY),
        (egui::TextStyle::Small, type_scale::CAPTION),
    ] {
        style
            .text_styles
            .insert(kind, egui::FontId::proportional(size));
    }
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(type_scale::TITLE, egui::FontFamily::Proportional),
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

#[must_use]
pub fn sidebar_width(ctx: &egui::Context) -> f32 {
    let fraction = ctx.available_rect().width() * 0.155;
    fraction.clamp(metrics::SIDEBAR_MIN, metrics::SIDEBAR_MAX)
}

#[must_use]
pub fn format_eta(downloaded: u64, total: u64, speed_bps: f64) -> String {
    if speed_bps <= 1.0 || total == 0 || downloaded >= total {
        return String::new();
    }
    let seconds = (total - downloaded) as f64 / speed_bps;
    if !seconds.is_finite() || seconds > 86_400.0 {
        return String::new();
    }
    let seconds = seconds as u64;
    if seconds < 60 {
        format!("{seconds}s left")
    } else if seconds < 3600 {
        format!("{}m left", seconds / 60)
    } else {
        format!("{}h {}m left", seconds / 3600, (seconds % 3600) / 60)
    }
}

#[must_use]
pub fn format_percent(fraction: f32) -> String {
    format!("{:.0}%", (fraction.clamp(0.0, 1.0) * 100.0).round())
}

fn install_system_fallback_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut loaded_fonts = Vec::new();

    #[cfg(target_os = "windows")]
    let candidates = [
        r"C:\Windows\Fonts\malgun.ttf",
        r"C:\Windows\Fonts\malgunsl.ttf",
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msgothic.ttc",
    ];

    #[cfg(target_os = "macos")]
    let candidates = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/AppleSDGothicNeo.ttc",
    ];

    #[cfg(target_os = "linux")]
    let candidates = [
        "Noto Sans CJK KR",
        "Noto Sans CJK JP",
        "Noto Sans CJK SC",
        "Noto Sans",
    ];

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    for path in candidates {
        let path = std::path::Path::new(path);

        if !path.exists() {
            continue;
        }

        let Ok(data) = std::fs::read(path) else {
            continue;
        };

        let name = format!("system_fallback_{}", loaded_fonts.len());

        fonts.font_data.insert(name.clone(), egui::FontData::from_owned(data).into());

        loaded_fonts.push(name);
    }

    #[cfg(target_os = "linux")]
    for family in candidates {
        let Ok(output) = std::process::Command::new("fc-match")
            .args(["-f", "%{file}", family])
            .output()
        else {
            continue;
        };

        if !output.status.success() {
            continue;
        }

        let path = String::from_utf8_lossy(&output.stdout);

        if path.is_empty() {
            continue;
        }

        let path = std::path::Path::new(path.trim());

        if !path.exists() {
            continue;
        }

        let Ok(data) = std::fs::read(path) else {
            continue;
        };

        let name = format!("system_fallback_{}", loaded_fonts.len());

        fonts.font_data.insert(name.clone(), egui::FontData::from_owned(data).into());

        loaded_fonts.push(name);
    }

    if loaded_fonts.is_empty() {
        return;
    }

    for font_name in &loaded_fonts {
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .push(font_name.clone());

        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .push(font_name.clone());
    }

    ctx.set_fonts(fonts);
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

    #[test]
    fn format_eta_scales_with_speed() {
        assert_eq!(format_eta(0, 1_000, 100.0), "10s left");
        assert_eq!(format_eta(0, 6_000, 100.0), "1m left");
        assert_eq!(format_eta(0, 36_000, 100.0), "6m left");
        assert_eq!(format_eta(0, 540_000, 100.0), "1h 30m left");
    }

    #[test]
    fn format_eta_is_blank_when_useless() {
        assert_eq!(format_eta(0, 1_000, 0.0), "");
        assert_eq!(format_eta(0, 0, 500.0), "");
        assert_eq!(format_eta(1_000, 1_000, 500.0), "");
        assert_eq!(format_eta(0, u64::MAX, 1.0), "");
    }

    #[test]
    fn format_percent_clamps() {
        assert_eq!(format_percent(0.0), "0%");
        assert_eq!(format_percent(0.5), "50%");
        assert_eq!(format_percent(1.0), "100%");
        assert_eq!(format_percent(-1.0), "0%");
        assert_eq!(format_percent(4.0), "100%");
    }

    #[test]
    fn secondary_and_tertiary_text_are_distinguishable() {
        let gap = (TEXT2.r() as i32 - TEXT_DISABLED.r() as i32).abs()
            + (TEXT2.g() as i32 - TEXT_DISABLED.g() as i32).abs()
            + (TEXT2.b() as i32 - TEXT_DISABLED.b() as i32).abs();
        assert!(
            gap >= 120,
            "TEXT2 and TEXT_DISABLED must not read as the same tone"
        );
        let tertiary = (MUTED.r() as i32 - TEXT_DISABLED.r() as i32).abs()
            + (MUTED.g() as i32 - TEXT_DISABLED.g() as i32).abs()
            + (MUTED.b() as i32 - TEXT_DISABLED.b() as i32).abs();
        assert!(tertiary >= 30);
    }

    #[test]
    fn sidebar_width_stays_inside_bounds() {
        let ctx = egui::Context::default();
        for width in [850.0_f32, 1280.0, 1920.0, 2560.0] {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 800.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    let w = sidebar_width(ctx);
                    assert!(
                        (metrics::SIDEBAR_MIN..=metrics::SIDEBAR_MAX).contains(&w),
                        "sidebar {w} out of range at window width {width}"
                    );
                },
            );
        }
    }
}
