use crate::app::state::AppState;
use crate::ui::components::{button_row, pill_tab_button, provider_card, step_rail, wizard_frame};
use crate::ui::theme::{type_scale, DANGER, MUTED, TEXT, TEXT2};
use egui::{Color32, CornerRadius, RichText, Stroke};

const STEPS: usize = 3;

const INTRO_FEATURES: [(&str, &str); 3] = [
    (
        "Isolated instances",
        "Keep each Minecraft version, its mods, worlds and settings together.",
    ),
    (
        "One-click mods",
        "Find mods on Modrinth or CurseForge. Required dependencies are installed with them.",
    ),
    (
        "Offline ready",
        "Play local singleplayer worlds and compatible offline servers without an active Microsoft sign-in.",
    ),
];

const INTRO_BACKGROUND: &[u8] = include_bytes!("../../../assets/onboarding-bg.jpg");

fn background_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = background_id();
    if let Some(handle) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(handle);
    }
    let decoded = image::load_from_memory_with_format(INTRO_BACKGROUND, image::ImageFormat::Jpeg)
        .ok()?
        .into_rgba8();
    let size = [decoded.width() as usize, decoded.height() as usize];
    let handle = ctx.load_texture(
        "monoryx-onboarding-bg",
        egui::ColorImage::from_rgba_unmultiplied(size, decoded.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(id, handle.clone()));
    Some(handle)
}

pub fn background_id() -> egui::Id {
    egui::Id::new("onboarding-background")
}

pub fn release_background(ctx: &egui::Context) {
    let released_id = egui::Id::new("onboarding-background-released");
    if ctx.data(|d| d.get_temp::<bool>(released_id).unwrap_or(false)) {
        return;
    }
    let id = background_id();
    if !ctx.data(|d| d.get_temp::<egui::TextureHandle>(id).is_some()) {
        return;
    }
    let empty = ctx.load_texture(
        "monoryx-onboarding-bg-released",
        egui::ColorImage::new([1, 1], vec![egui::Color32::TRANSPARENT]),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| {
        d.insert_temp(id, empty);
        d.insert_temp(released_id, true);
    });
}

pub fn reset_background(ctx: &egui::Context) {
    let released_id = egui::Id::new("onboarding-background-released");
    let id = background_id();
    ctx.data_mut(|d| {
        d.remove::<egui::TextureHandle>(id);
        d.remove::<bool>(released_id);
    });
}

fn paint_background(ui: &mut egui::Ui, rect: egui::Rect) {
    let ctx = ui.ctx().clone();
    let p = crate::ui::theme::palette(&ctx);
    let is_halloween = crate::ui::theme::current_theme(&ctx) == crate::config::ThemeKind::Halloween;

    let painter = ui.painter().with_clip_rect(rect);

    if is_halloween {
        painter.rect_filled(rect, 0, p.bg);

        let moon_x = (rect.right() - 170.0).max(rect.left() + 200.0);
        let moon_y = rect.top() + 130.0;
        let moon_pos = egui::pos2(moon_x, moon_y);
        painter.circle_filled(moon_pos, 25.0, egui::Color32::from_rgb(250, 235, 208));
        painter.circle_filled(
            moon_pos + egui::vec2(-6.0, 5.0),
            6.0,
            egui::Color32::from_rgba_unmultiplied(220, 200, 170, 50),
        );
        painter.circle_filled(
            moon_pos + egui::vec2(8.0, -4.0),
            4.0,
            egui::Color32::from_rgba_unmultiplied(220, 200, 170, 40),
        );

        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(moon_x - 340.0, moon_y + 20.0),
                egui::vec2(220.0, 10.0),
            ),
            CornerRadius::same(5),
            egui::Color32::from_rgba_unmultiplied(120, 100, 140, 20),
        );
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(moon_x - 200.0, moon_y + 44.0),
                egui::vec2(170.0, 8.0),
            ),
            CornerRadius::same(4),
            egui::Color32::from_rgba_unmultiplied(120, 100, 140, 16),
        );
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(moon_x - 70.0, moon_y - 6.0),
                egui::vec2(95.0, 6.0),
            ),
            CornerRadius::same(3),
            egui::Color32::from_rgba_unmultiplied(230, 210, 180, 25),
        );

        let left_hill = vec![
            egui::pos2(rect.center().x - 180.0, moon_y + 170.0),
            egui::pos2(rect.center().x - 60.0, moon_y + 122.0),
            egui::pos2(moon_x - 180.0, moon_y + 96.0),
            egui::pos2(moon_x - 70.0, moon_y + 112.0),
            egui::pos2(moon_x + 60.0, moon_y + 170.0),
        ];
        painter.add(egui::epaint::PathShape::convex_polygon(
            left_hill,
            egui::Color32::from_rgb(22, 24, 33),
            egui::Stroke::NONE,
        ));

        let moon_hill = vec![
            egui::pos2(moon_x - 260.0, moon_y + 170.0),
            egui::pos2(moon_x - 140.0, moon_y + 82.0),
            egui::pos2(moon_x - 50.0, moon_y + 68.0),
            egui::pos2(moon_x + 30.0, moon_y + 74.0),
            egui::pos2(moon_x + 130.0, moon_y + 64.0),
            egui::pos2(moon_x + 240.0, moon_y + 100.0),
            egui::pos2(moon_x + 240.0, moon_y + 170.0),
        ];
        painter.add(egui::epaint::PathShape::convex_polygon(
            moon_hill,
            egui::Color32::from_rgb(26, 28, 38),
            egui::Stroke::NONE,
        ));

        crate::ui::components::draw_bat_scaled(
            &painter,
            moon_pos + egui::vec2(-40.0, -32.0),
            1.15,
            egui::Color32::from_rgb(14, 15, 20),
        );
        crate::ui::components::draw_bat_scaled(
            &painter,
            moon_pos + egui::vec2(-85.0, 18.0),
            0.85,
            egui::Color32::from_rgb(16, 17, 24),
        );
        crate::ui::components::draw_bat_scaled(
            &painter,
            moon_pos + egui::vec2(55.0, -38.0),
            0.65,
            egui::Color32::from_rgb(18, 19, 27),
        );
        crate::ui::components::draw_bat_scaled(
            &painter,
            moon_pos + egui::vec2(-150.0, -12.0),
            0.95,
            egui::Color32::from_rgb(15, 16, 22),
        );

        let bot_y = rect.bottom();
        let step_x = 42.0_f32;
        let mut cur_x = rect.left() + 18.0;
        let mut idx = 0_usize;
        while cur_x < rect.right() - 10.0 {
            let tree_h = 55.0 + ((idx * 37) % 55) as f32;
            let tree_w = 20.0 + ((idx * 19) % 16) as f32;
            let tree_color = if idx.is_multiple_of(2) {
                egui::Color32::from_rgb(32, 34, 46)
            } else {
                egui::Color32::from_rgb(24, 25, 35)
            };
            if idx.is_multiple_of(3) {
                crate::ui::components::draw_dead_tree(
                    &painter,
                    egui::pos2(cur_x, bot_y),
                    tree_h * 0.9,
                    tree_color,
                );
            } else {
                crate::ui::components::draw_pine_tree(
                    &painter,
                    egui::pos2(cur_x, bot_y),
                    tree_h,
                    tree_w,
                    tree_color,
                );
            }
            cur_x += step_x;
            idx += 1;
        }

        let fog_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), bot_y - 35.0),
            egui::pos2(rect.right(), bot_y),
        );
        painter.rect_filled(
            fog_rect,
            0,
            egui::Color32::from_rgba_unmultiplied(28, 25, 38, 90),
        );
    } else if let Some(texture) = background_texture(&ctx) {
        let native = texture.size_vec2();
        let scale = (rect.width() / native.x).max(rect.height() / native.y);
        let shown = egui::Rect::from_center_size(rect.center(), native * scale);

        painter.image(
            texture.id(),
            shown,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        let overlay_alpha = if is_halloween { 236 } else { 224 };
        painter.rect_filled(
            rect,
            0,
            egui::Color32::from_rgba_unmultiplied(p.bg.r(), p.bg.g(), p.bg.b(), overlay_alpha),
        );
    } else {
        painter.rect_filled(rect, 0, p.bg);
    }
}

pub fn show(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    let step = (state.onboarding_step as usize).min(STEPS - 1);
    let panel_height = ui.available_height();
    let enter = ctx.animate_bool_with_time(egui::Id::new("onboarding-enter"), true, 0.18);
    if enter < 0.999 {
        ctx.request_repaint();
    }

    paint_background(ui, ui.max_rect());

    let is_halloween =
        crate::ui::theme::current_theme(ui.ctx()) == crate::config::ThemeKind::Halloween;
    let cache_id = egui::Id::new(("wizard-height", step));
    let default_h = match step {
        0 => 270.0,
        1 => 330.0,
        _ => 380.0,
    };
    let known_card_height = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(cache_id).unwrap_or(default_h));
    let header_height = if is_halloween { 135.0 } else { 115.0 };
    let total_block = header_height + known_card_height + 24.0;
    let available_space = panel_height - total_block;
    if available_space > 30.0 {
        ui.add_space(((available_space / 2.0) - 34.0).clamp(0.0, 160.0));
    }

    wordmark(ui);
    step_rail(ui, STEPS, step);

    let content_key = onboarding_content_key(state, step);
    wizard_frame(ui, step, panel_height, content_key, |ui, footer| {
        if footer {
            onboarding_footer(state, ui, step);
        } else {
            match step {
                0 => intro(ui),
                1 => username(state, ui),
                _ => defaults(state, ui),
            }
        }
    });

    if step == 0 && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        state.onboarding_step = 1;
    }
    if step == 1
        && state.onboarding_use_microsoft
        && state.config.microsoft_profile.is_some()
        && ctx.input(|i| i.key_pressed(egui::Key::Enter))
    {
        state.config.use_microsoft_auth = true;
        state.onboarding_step = 2;
    }
    if step == 2 && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        let _ = complete_onboarding(state, ctx);
    }
    if step > 0 && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        if step == 1 {
            state.cancel_microsoft_login();
        }
        state.onboarding_step = (step - 1) as u32;
    }
}

fn wordmark(ui: &mut egui::Ui) {
    let is_halloween =
        crate::ui::theme::current_theme(ui.ctx()) == crate::config::ThemeKind::Halloween;
    let p = crate::ui::theme::palette(ui.ctx());

    ui.vertical_centered(|ui| {
        if is_halloween {
            let (bat_rect, _) =
                ui.allocate_exact_size(egui::vec2(28.0, 16.0), egui::Sense::hover());
            crate::ui::components::draw_glowing_bat(ui.painter(), bat_rect.center(), p.accent);
            ui.add_space(4.0);
        }
        ui.label(
            RichText::new("MONORYX")
                .size(type_scale::TITLE)
                .strong()
                .color(if is_halloween {
                    egui::Color32::from_rgb(255, 235, 215)
                } else {
                    TEXT
                }),
        );
        ui.label(
            RichText::new("Play. Modify. Nothing else.")
                .size(type_scale::LABEL)
                .color(if is_halloween {
                    egui::Color32::from_rgb(175, 155, 140)
                } else {
                    MUTED
                }),
        );
        ui.add_space(10.0);
    });
}

fn heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    let is_halloween =
        crate::ui::theme::current_theme(ui.ctx()) == crate::config::ThemeKind::Halloween;
    ui.label(
        RichText::new(title)
            .size(type_scale::TITLE)
            .strong()
            .color(if is_halloween {
                egui::Color32::from_rgb(255, 240, 225)
            } else {
                TEXT
            }),
    );
    ui.label(
        RichText::new(subtitle)
            .size(type_scale::LABEL)
            .color(if is_halloween {
                egui::Color32::from_rgb(195, 180, 170)
            } else {
                egui::Color32::from_rgb(182, 189, 202)
            }),
    );
    ui.add_space(12.0);
}

fn intro(ui: &mut egui::Ui) {
    heading(
        ui,
        "Welcome to MONORYX",
        "Set up your account and game defaults.",
    );
    for (title, detail) in INTRO_FEATURES {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(format!("\u{2022}  {title}"))
                    .size(type_scale::BODY)
                    .strong()
                    .color(TEXT),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(detail)
                        .size(type_scale::CAPTION)
                        .color(egui::Color32::from_rgb(182, 189, 202)),
                )
                .wrap(),
            );
        });
        ui.add_space(10.0);
    }
}

fn username(state: &mut AppState, ui: &mut egui::Ui) {
    if state.onboarding_use_microsoft {
        heading(
            ui,
            "Sign in with Microsoft",
            "Use your official Microsoft account to join multiplayer servers and sync skins.",
        );
    } else {
        heading(
            ui,
            "Choose an offline username",
            "Works for singleplayer and offline-mode servers. You can add a Microsoft account later.",
        );
    }

    ui.horizontal(|ui| {
        let offline = !state.onboarding_use_microsoft;
        if pill_tab_button(ui, "Play Offline", offline).clicked() && state.onboarding_use_microsoft
        {
            state.onboarding_use_microsoft = false;
            state.cancel_microsoft_login();
            state.onboarding_error.clear();
        }
        if pill_tab_button(ui, "Sign in with Microsoft", state.onboarding_use_microsoft).clicked()
            && !state.onboarding_use_microsoft
        {
            state.onboarding_use_microsoft = true;
            state.onboarding_error.clear();
            if !state.ms_login_loading
                && state.ms_device_code.is_none()
                && state.config.microsoft_profile.is_none()
            {
                state.start_microsoft_login();
            }
        }
    });

    ui.add_space(14.0);

    if state.onboarding_use_microsoft {
        microsoft_flow(state, ui);
    } else {
        offline_flow(state, ui);
    }
}

fn microsoft_flow(state: &mut AppState, ui: &mut egui::Ui) {
    if let Some(profile) = &state.config.microsoft_profile {
        provider_card(ui, &profile.username, "Microsoft Account", "Connected");
        ui.add_space(8.0);
        if ui.button("Sign into different account").clicked() {
            state.config.microsoft_profile = None;
            state.config.use_microsoft_auth = false;
            state.start_microsoft_login();
            return;
        }
        ui.add_space(10.0);

        return;
    }

    if let Some(code) = state.ms_device_code.clone() {
        egui::Frame::new()
            .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
            .stroke(Stroke::new(
                1.0_f32,
                crate::ui::theme::palette(ui.ctx()).accent,
            ))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Microsoft Device Login Flow")
                        .size(14.0)
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("1. Open in browser:").size(12.5).color(TEXT2));
                    let uri = if code.verification_uri.trim().is_empty() {
                        "https://www.microsoft.com/link"
                    } else {
                        code.verification_uri.as_str()
                    };
                    if ui.button("microsoft.com/link").on_hover_text(uri).clicked() {
                        let _ = open::that(uri);
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("2. Enter Code:").size(12.5).color(TEXT2));
                    ui.label(
                        RichText::new(&code.user_code)
                            .size(16.0)
                            .strong()
                            .monospace()
                            .color(crate::ui::theme::INFO),
                    );
                    if ui.button("Copy Code").clicked() {
                        ui.ctx().copy_text(code.user_code.clone());
                        state.notify("Code copied to clipboard!");
                    }
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new("Waiting for sign-in approval in browser...")
                            .size(12.0)
                            .color(MUTED),
                    );
                    if ui.button("Cancel").clicked() {
                        state.cancel_microsoft_login();
                    }
                });
            });
    } else if state.ms_login_loading {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                RichText::new("Requesting Microsoft login code...")
                    .size(12.5)
                    .color(TEXT2),
            );
            if ui.button("Cancel").clicked() {
                state.cancel_microsoft_login();
            }
        });
    } else {
        ui.horizontal(|ui| {
            if crate::ui::components::primary_button(ui, "Sign in with Microsoft").clicked() {
                state.start_microsoft_login();
            }
        });
    }

    if let Some(err) = &state.ms_login_error {
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("Error: {err}"))
                .size(12.0)
                .color(DANGER),
        );
        ui.add_space(4.0);
        if ui.button("Try Again").clicked() {
            state.start_microsoft_login();
        }
    }

    if !state.onboarding_error.is_empty() {
        ui.add_space(8.0);
        ui.label(
            RichText::new(&state.onboarding_error)
                .size(12.0)
                .color(DANGER),
        );
    }

    ui.add_space(18.0);
}

fn onboarding_content_key(state: &AppState, step: usize) -> u64 {
    let mut key = (step as u64) << 8;
    if state.onboarding_use_microsoft {
        key |= 1 << 6;
        if state.ms_login_loading {
            key |= 1 << 5;
        }
        if state.ms_device_code.is_some() {
            key |= 1 << 4;
        }
        if state.ms_login_error.is_some() {
            key |= 1 << 3;
        }
    }
    if !state.onboarding_error.is_empty() {
        key |= 1 << 2;
    }
    key
}

fn offline_flow(state: &mut AppState, ui: &mut egui::Ui) {
    crate::ui::components::field_label(ui, "Username");
    crate::ui::components::limited_text_edit(
        ui,
        "onboarding-username",
        &mut state.onboarding_user,
        crate::ui::components::limits::USERNAME,
        "3 to 16 letters, numbers or _",
    );
    if let Some(name) = &state.config.profile {
        provider_card(ui, &name.username, "No account required", "Offline");
    }
    if !state.onboarding_error.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new(&state.onboarding_error).color(DANGER));
    }
    ui.add_space(18.0);
}

pub fn complete_onboarding(state: &mut AppState, ctx: &egui::Context) -> bool {
    if !state.onboarding_mem_auto {
        match state.onboarding_mem_max.trim().parse::<u64>() {
            Ok(value) if value >= 512 => {
                if let Err(error) = crate::utils::system::validate_memory(
                    state.config.memory.min_mb.min(value),
                    value,
                ) {
                    state.onboarding_error = error;
                    return false;
                }
                state.config.memory.min_mb = state.config.memory.min_mb.min(value);
                state.config.memory.max_mb = value;
            }
            _ => {
                state.onboarding_error = "Choose at least 0.5 GB of game memory.".to_string();
                return false;
            }
        }
    } else {
        state.config.memory.max_mb = crate::utils::system::default_max_memory_mb();
    }
    if state.config.profile.is_none() {
        if let Some(ms) = &state.config.microsoft_profile {
            if let Ok(off) = crate::account::offline::OfflineProfile::new(&ms.username) {
                state.config.profile = Some(off);
            }
        }
    }
    state.onboarding_error.clear();
    state.config.completed_onboarding = true;
    state.save_config();
    state.page = crate::app::events::Page::Home;

    if state.config.start_maximized {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
    } else {
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            state.config.window_width.clamp(850.0, 2560.0),
            state.config.window_height.clamp(560.0, 1440.0),
        )));
    }
    state.notify("Welcome to MONORYX");
    true
}

fn defaults(state: &mut AppState, ui: &mut egui::Ui) {
    heading(
        ui,
        "Launcher defaults",
        "Everything here can be changed later in Settings.",
    );

    ui.checkbox(
        &mut state.onboarding_mem_auto,
        RichText::new("Automatic memory").strong().color(TEXT),
    );
    if state.onboarding_mem_auto {
        ui.label(
            RichText::new(format!(
                "{:.1} GB recommended for this system",
                crate::utils::system::default_max_memory_mb() as f64 / 1024.0
            ))
            .size(type_scale::CAPTION)
            .color(Color32::from_rgb(182, 189, 202)),
        );
    } else {
        ui.add_space(4.0);
        crate::ui::components::field_label(ui, "Game memory (GB)");
        let mut memory = state.onboarding_mem_max.parse::<u64>().unwrap_or(3072) as f64 / 1024.0;
        if ui
            .add(
                egui::DragValue::new(&mut memory)
                    .range(0.5..=64.0)
                    .speed(0.25)
                    .suffix(" GB"),
            )
            .changed()
        {
            state.onboarding_mem_max = ((memory * 1024.0).round() as u64).to_string();
        }
    }

    ui.add_space(16.0);

    ui.label(RichText::new("Java runtime").strong().color(TEXT));
    ui.label(
        RichText::new("Automatic (Temurin)")
            .size(type_scale::CAPTION)
            .color(Color32::from_rgb(182, 189, 202)),
    );

    ui.add_space(16.0);

    ui.label(RichText::new("GPU preference").strong().color(TEXT));
    ui.add_space(2.0);
    ui.add_enabled_ui(cfg!(target_os = "windows"), |ui| {
        egui::ComboBox::from_id_salt("onboarding-gpu-preference")
            .selected_text(state.config.gpu_preference.as_str())
            .show_ui(ui, |ui| {
                for value in [
                    crate::config::GpuPreference::System,
                    crate::config::GpuPreference::HighPerformance,
                    crate::config::GpuPreference::PowerSaving,
                ] {
                    ui.selectable_value(&mut state.config.gpu_preference, value, value.as_str());
                }
            });
    });
    ui.label(
        RichText::new("Windows only. System default lets Windows choose the GPU.")
            .size(type_scale::CAPTION)
            .color(Color32::from_rgb(182, 189, 202)),
    );

    if !state.onboarding_error.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new(&state.onboarding_error).color(DANGER));
    }
}

fn onboarding_footer(state: &mut AppState, ui: &mut egui::Ui, step: usize) {
    if step == 0 {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::ui::components::primary_button(ui, "Continue").clicked() {
                    state.onboarding_step = 1;
                }
            });
        });
        return;
    }
    ui.horizontal(|ui| {
        let (back, next) = button_row(
            ui,
            "Back",
            if step == 2 {
                "Open MONORYX"
            } else {
                "Continue"
            },
            crate::ui::components::Tone::Primary,
        );
        if back.clicked() {
            state.cancel_microsoft_login();
            state.onboarding_step = (step - 1) as u32;
        }
        if next.clicked() {
            match step {
                0 => state.onboarding_step = 1,
                1 if state.onboarding_use_microsoft => {
                    if state.config.microsoft_profile.is_some() {
                        state.config.use_microsoft_auth = true;
                        state.onboarding_step = 2;
                    } else {
                        state.onboarding_error =
                            "Finish signing in, or choose Play Offline.".into();
                    }
                }
                1 => {
                    match crate::account::offline::OfflineProfile::new(state.onboarding_user.trim())
                    {
                        Ok(profile) => {
                            state.config.profile = Some(profile);
                            state.config.use_microsoft_auth = false;
                            state.onboarding_error.clear();
                            state.onboarding_step = 2;
                        }
                        Err(error) => state.onboarding_error = error.user_message(),
                    }
                }
                _ => {
                    let _ = complete_onboarding(state, ui.ctx());
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_label_point(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text.as_str() == label => {
                    Some(text.pos + egui::vec2(3.0, 9.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no button labelled {label:?} was rendered"))
    }

    struct Harness {
        state: AppState,
        ctx: egui::Context,
    }

    impl Harness {
        fn new(step: u32, username: &str) -> Self {
            let ctx = egui::Context::default();
            crate::ui::theme::apply_theme(&ctx);
            let temp = tempfile::tempdir().unwrap();
            let paths = crate::storage::paths::MonoryxPaths::new(temp.path().into());
            let cc = eframe::CreationContext::_new_kittest(ctx.clone());
            let mut state = AppState::new_for_preview(&cc, paths);
            state.onboarding_step = step;
            state.onboarding_user = username.to_string();
            Self { state, ctx }
        }

        fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
            self.ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(880.0, 660.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(&mut self.state, ctx, ui);
                    });
                },
            )
        }

        fn click(&mut self, label: &str) -> u32 {
            let output = self.frame(Vec::new());
            let at = find_label_point(&output, label);
            self.frame(vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
            ]);
            self.frame(vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            }]);
            self.state.onboarding_step
        }
    }

    #[test]
    fn intro_continue_button_advances_to_the_username_step() {
        let mut h = Harness::new(0, "");
        assert_eq!(h.click("Continue"), 1);
        assert_eq!(h.state.page, crate::app::events::Page::Onboarding);
    }

    #[test]
    fn username_step_advance_requires_a_valid_name() {
        let mut h = Harness::new(1, "ab");
        assert_eq!(
            h.click("Continue"),
            1,
            "a 2-character name must not advance"
        );
        assert!(!h.state.onboarding_error.is_empty());

        let mut h = Harness::new(1, "Not_Subaka");
        assert_eq!(h.click("Continue"), 2);
        assert_eq!(
            h.state.config.profile.as_ref().map(|p| p.username.as_str()),
            Some("Not_Subaka")
        );
    }

    #[test]
    fn back_returns_to_the_previous_step() {
        let mut h = Harness::new(1, "Not_Subaka");
        assert_eq!(h.click("Back"), 0);
        let mut h = Harness::new(2, "Not_Subaka");
        assert_eq!(h.click("Back"), 1);
    }

    #[test]
    fn finishing_the_wizard_lands_on_home() {
        let mut h = Harness::new(2, "Not_Subaka");
        assert_eq!(
            h.click("Open MONORYX"),
            2,
            "step stays put, it completes instead"
        );
        assert!(h.state.config.completed_onboarding);
        assert_eq!(h.state.page, crate::app::events::Page::Home);
    }

    #[test]
    fn switching_to_microsoft_and_completing_login() {
        let mut h = Harness::new(1, "");
        assert!(!h.state.onboarding_use_microsoft);
        h.click("Sign in with Microsoft");
        assert!(h.state.onboarding_use_microsoft);

        h.state.config.microsoft_profile = Some(crate::account::microsoft::MicrosoftProfile {
            username: "SteveOnline".into(),
            uuid: uuid::Uuid::nil(),
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: 0,
        });
        h.state.config.use_microsoft_auth = true;
        assert_eq!(h.click("Continue"), 2);
    }

    #[test]
    fn switching_back_to_offline_from_microsoft() {
        let mut h = Harness::new(1, "Steve");
        h.state.config.use_microsoft_auth = true;
        h.click("Sign in with Microsoft");
        assert!(h.state.onboarding_use_microsoft);
        h.click("Play Offline");
        assert!(!h.state.onboarding_use_microsoft);
        assert_eq!(h.click("Continue"), 2);
        assert!(!h.state.config.use_microsoft_auth);
        assert_eq!(
            h.state.config.profile.as_ref().map(|p| p.username.as_str()),
            Some("Steve")
        );
    }

    #[test]
    fn microsoft_step_continue_without_login_shows_error() {
        let mut h = Harness::new(1, "");
        h.click("Sign in with Microsoft");
        assert!(h.state.onboarding_use_microsoft);
        assert_eq!(h.click("Continue"), 1);
        assert_eq!(
            h.state.onboarding_error,
            "Finish signing in, or choose Play Offline."
        );
    }

    #[test]
    fn microsoft_device_code_cancel_clears_state() {
        let mut h = Harness::new(1, "");
        h.click("Sign in with Microsoft");
        h.state.ms_login_loading = true;
        h.state.ms_device_code = Some(crate::account::microsoft::DeviceCodeResponse {
            device_code: "dev123".into(),
            user_code: "ABCD-EFGH".into(),
            verification_uri: "https://microsoft.com/link".into(),
            expires_in: 900,
            interval: 5,
            message: None,
        });
        h.click("Cancel");
        assert!(!h.state.ms_login_loading);
        assert!(h.state.ms_device_code.is_none());
    }

    #[test]
    fn defaults_step_custom_memory_validation() {
        let mut h = Harness::new(2, "Steve");
        h.state.onboarding_mem_auto = false;
        h.state.onboarding_mem_max = "256".to_string();
        assert!(!complete_onboarding(&mut h.state, &h.ctx));
        assert!(!h.state.config.completed_onboarding);
        assert!(h.state.onboarding_error.contains("0.5 GB"));

        h.state.onboarding_mem_max = "4096".to_string();
        h.click("Open MONORYX");
        assert!(h.state.config.completed_onboarding);
        assert_eq!(h.state.config.memory.max_mb, 4096);
    }

    #[test]
    fn defaults_step_auto_memory_resets_custom_mb() {
        let mut h = Harness::new(2, "Steve");
        h.state.config.memory.max_mb = 9999;
        h.state.onboarding_mem_auto = true;
        h.click("Open MONORYX");
        assert!(h.state.config.completed_onboarding);
        assert_eq!(
            h.state.config.memory.max_mb,
            crate::utils::system::default_max_memory_mb()
        );
    }

    #[test]
    fn reset_background_clears_texture_cache() {
        let ctx = egui::Context::default();
        let released_id = egui::Id::new("onboarding-background-released");
        let id = background_id();
        ctx.data_mut(|d| {
            d.insert_temp(released_id, true);
        });
        assert!(ctx.data(|d| d.get_temp::<bool>(released_id).unwrap_or(false)));
        reset_background(&ctx);
        assert!(!ctx.data(|d| d.get_temp::<bool>(released_id).unwrap_or(false)));
        assert!(ctx.data(|d| d.get_temp::<egui::TextureHandle>(id).is_none()));
    }
}
