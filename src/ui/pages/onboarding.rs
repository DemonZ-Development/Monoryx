use crate::app::state::AppState;
use crate::ui::components::{button_row, provider_card, step_rail, wizard_frame};
use crate::ui::theme::{type_scale, DANGER, MUTED, TEXT, TEXT2};
use egui::{CornerRadius, RichText, Stroke};

const STEPS: usize = 3;

const INTRO_FEATURES: [(&str, &str); 3] = [
    (
        "Isolated instances",
        "Each profile keeps its own mods, worlds and settings. Nothing leaks between them.",
    ),
    (
        "One-click mods",
        "Browse Modrinth and packs install with their dependencies already resolved.",
    ),
    (
        "Offline ready",
        "Play singleplayer and offline-mode servers without a Microsoft account.",
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
    let pixels: Vec<egui::Color32> = decoded
        .pixels()
        .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    let handle = ctx.load_texture(
        "monoryx-onboarding-bg",
        egui::ColorImage::new(size, pixels),
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
    let Some(texture) = background_texture(&ctx) else {
        return;
    };
    let native = texture.size_vec2();

    let scale = (rect.width() / native.x).max(rect.height() / native.y);
    let shown = egui::Rect::from_center_size(rect.center(), native * scale);
    let p = crate::ui::theme::palette(&ctx);

    let painter = ui.painter().with_clip_rect(rect);
    painter.image(
        texture.id(),
        shown,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    painter.rect_filled(
        rect,
        0,
        egui::Color32::from_rgba_unmultiplied(p.bg.r(), p.bg.g(), p.bg.b(), 224),
    );
}

pub fn show(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    let step = (state.onboarding_step as usize).min(STEPS - 1);
    let panel_height = ui.available_height();
    let enter = ctx.animate_bool_with_time(egui::Id::new("onboarding-enter"), true, 0.18);
    if enter < 0.999 {
        ctx.request_repaint();
    }

    paint_background(ui, ui.max_rect());

    wordmark(ui);
    step_rail(ui, STEPS, step);

    wizard_frame(ui, step, panel_height, |ui| match step {
        0 => intro(state, ui),
        1 => username(state, ui),
        _ => defaults(state, ui),
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
    ui.vertical_centered(|ui| {
        ui.add_space(28.0);
        ui.label(
            RichText::new("MONORYX")
                .size(type_scale::TITLE)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new("Play. Modify. Nothing else.")
                .size(type_scale::LABEL)
                .color(MUTED),
        );
        ui.add_space(18.0);
    });
}

fn heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .size(type_scale::TITLE)
            .strong()
            .color(TEXT),
    );
    ui.label(RichText::new(subtitle).size(type_scale::LABEL).color(TEXT2));
    ui.add_space(18.0);
}

fn intro(state: &mut AppState, ui: &mut egui::Ui) {
    heading(ui, "Clean. Fast. Yours.", "What you get on day one.");
    for (title, detail) in INTRO_FEATURES {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(format!("\u{2022}  {title}"))
                    .size(type_scale::BODY)
                    .strong()
                    .color(TEXT),
            );
            ui.add(
                egui::Label::new(RichText::new(detail).size(type_scale::CAPTION).color(TEXT2))
                    .wrap(),
            );
        });
        ui.add_space(10.0);
    }
    ui.add_space(6.0);

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let next =
            crate::ui::components::button(ui, "Continue", crate::ui::components::Tone::Primary)
                .on_hover_text("Press Enter to continue");
        if next.clicked() {
            state.onboarding_step = 1;
        }
    });
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
        if ui.selectable_label(offline, "Play Offline").clicked() && state.onboarding_use_microsoft
        {
            state.onboarding_use_microsoft = false;
            state.cancel_microsoft_login();
            state.onboarding_error.clear();
        }
        if ui
            .selectable_label(state.onboarding_use_microsoft, "Sign in with Microsoft")
            .clicked()
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
        ui.horizontal(|ui| {
            let (back, next) =
                button_row(ui, "Back", "Continue", crate::ui::components::Tone::Primary);
            if back.clicked() {
                state.cancel_microsoft_login();
                state.onboarding_step = 0;
            }
            if next.clicked() {
                state.config.use_microsoft_auth = true;
                state.onboarding_step = 2;
            }
        });
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
    ui.horizontal(|ui| {
        let (back, next) = button_row(ui, "Back", "Continue", crate::ui::components::Tone::Primary);
        if back.clicked() {
            state.cancel_microsoft_login();
            state.onboarding_step = 0;
        }
        if next.clicked() {
            if state.config.microsoft_profile.is_some() {
                state.config.use_microsoft_auth = true;
                state.onboarding_step = 2;
            } else {
                state.onboarding_error =
                    "Please complete Microsoft sign-in to continue.".to_string();
            }
        }
    });
}

fn offline_flow(state: &mut AppState, ui: &mut egui::Ui) {
    crate::ui::components::field_label(ui, "Username");
    let response = crate::ui::components::limited_text_edit(
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
    ui.horizontal(|ui| {
        let (back, next) = button_row(ui, "Back", "Continue", crate::ui::components::Tone::Primary);
        if back.clicked() {
            state.cancel_microsoft_login();
            state.onboarding_step = 0;
        }
        let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if next.clicked() || enter {
            match crate::account::offline::OfflineProfile::new(state.onboarding_user.trim()) {
                Ok(profile) => {
                    state.config.profile = Some(profile);
                    state.config.use_microsoft_auth = false;
                    state.onboarding_error.clear();
                    state.onboarding_step = 2;
                }
                Err(error) => state.onboarding_error = error.user_message(),
            }
        }
    });
}

pub fn complete_onboarding(state: &mut AppState, ctx: &egui::Context) -> bool {
    if !state.onboarding_mem_auto {
        match state.onboarding_mem_max.trim().parse::<u64>() {
            Ok(value) if value >= 512 => state.config.memory.max_mb = value,
            _ => {
                state.onboarding_error = "Max memory must be a number of at least 512.".to_string();
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

    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
        state.config.window_width.clamp(850.0, 2560.0),
        state.config.window_height.clamp(560.0, 1440.0),
    )));
    state.notify("Welcome to MONORYX");
    true
}

fn defaults(state: &mut AppState, ui: &mut egui::Ui) {
    heading(
        ui,
        "Launcher defaults",
        "Everything here can be changed later in Settings.",
    );
    ui.checkbox(&mut state.onboarding_mem_auto, "Automatic memory");
    if state.onboarding_mem_auto {
        ui.label(
            RichText::new(format!(
                "MONORYX will cap this at {} MB based on your system.",
                crate::utils::system::default_max_memory_mb()
            ))
            .size(type_scale::CAPTION)
            .color(TEXT2),
        );
    } else {
        crate::ui::components::field_label(ui, "Max memory (MB)");
        crate::ui::components::limited_text_edit(
            ui,
            "onboarding-mem-max",
            &mut state.onboarding_mem_max,
            7,
            "3072",
        );
    }
    ui.add_space(12.0);
    crate::ui::components::field_label(ui, "Java");
    ui.label(
        RichText::new("Automatic — MONORYX downloads a matching Temurin runtime.")
            .size(type_scale::CAPTION)
            .color(TEXT2),
    );
    ui.add_space(12.0);
    crate::ui::components::field_label(ui, "GPU");
    super::settings::gpu_preference_selector(ui, &mut state.config.gpu_preference);
    if !state.onboarding_error.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new(&state.onboarding_error).color(DANGER));
    }
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        let (back, next) = button_row(
            ui,
            "Back",
            "Open MONORYX",
            crate::ui::components::Tone::Primary,
        );
        if back.clicked() {
            state.onboarding_step = 1;
        }
        if next.clicked() {
            let _ = complete_onboarding(state, ui.ctx());
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
            "Please complete Microsoft sign-in to continue."
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
        h.click("Open MONORYX");
        assert!(!h.state.config.completed_onboarding);
        assert!(h.state.onboarding_error.contains("at least 512"));

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
