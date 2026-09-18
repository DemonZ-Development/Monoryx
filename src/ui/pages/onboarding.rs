use crate::app::state::AppState;
use crate::ui::components::{card_frame, field_label, page_header, primary_button};
use crate::ui::theme::{TEXT, TEXT2};
use egui::RichText;
pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(RichText::new("MONORYX").size(34.0).strong().color(TEXT));
        ui.label(
            RichText::new("Minecraft, without the clutter.")
                .size(14.0)
                .color(TEXT2),
        );
        ui.add_space(24.0);
    });
    if state.onboarding_step == 0 {
        ui.vertical_centered(|ui| {
            card_frame(ui, |ui| {
                ui.set_width(420.0);
                page_header(
                    ui,
                    "Clean. Fast. Yours.",
                    "Isolated instances, offline profiles, one-click mods.",
                );
                ui.add_space(8.0);
                if primary_button(ui, "Continue").clicked() {
                    state.onboarding_step = 1;
                }
            });
        });
        return;
    }
    if state.onboarding_step == 1 {
        ui.vertical_centered(|ui| {
            card_frame(ui, |ui| {
                ui.set_width(420.0);
                page_header(
                    ui,
                    "Choose an offline username",
                    "Offline profiles work for singleplayer and offline-mode servers.",
                );
                field_label(ui, "Username");
                ui.text_edit_singleline(&mut state.onboarding_user);
                if !state.onboarding_error.is_empty() {
                    ui.label(
                        RichText::new(&state.onboarding_error)
                            .color(egui::Color32::from_rgb(0xE0, 0x5A, 0x5A)),
                    );
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Offline").size(11.0).color(TEXT2));
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        state.onboarding_step = 0;
                    }
                    if primary_button(ui, "Continue").clicked() {
                        match crate::account::offline::OfflineProfile::new(
                            state.onboarding_user.trim(),
                        ) {
                            Ok(p) => {
                                state.config.profile = Some(p);
                                state.onboarding_error.clear();
                                state.onboarding_step = 2;
                            }
                            Err(e) => state.onboarding_error = e.user_message(),
                        }
                    }
                });
            });
        });
        return;
    }
    ui.vertical_centered(|ui| {
        card_frame(ui, |ui| {
            ui.set_width(460.0);
            page_header(
                ui,
                "Launcher defaults",
                "You can change these later in Settings.",
            );
            ui.checkbox(&mut state.onboarding_mem_auto, "Automatic memory");
            if !state.onboarding_mem_auto {
                field_label(ui, "Max memory (MB)");
                ui.text_edit_singleline(&mut state.onboarding_mem_max);
            }
            field_label(ui, "Java");
            ui.label(
                RichText::new("Automatic - MONORYX picks a compatible runtime.")
                    .size(12.0)
                    .color(TEXT2),
            );
            super::settings::gpu_preference_selector(ui, &mut state.config.gpu_preference);
            if !state.onboarding_error.is_empty() {
                ui.label(
                    RichText::new(&state.onboarding_error)
                        .color(egui::Color32::from_rgb(0xE0, 0x5A, 0x5A)),
                );
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Back").clicked() {
                    state.onboarding_step = 1;
                }
                if primary_button(ui, "Open MONORYX").clicked() {
                    if !state.onboarding_mem_auto {
                        match state.onboarding_mem_max.trim().parse::<u64>() {
                            Ok(v) if v >= 512 => state.config.memory.max_mb = v,
                            _ => {
                                state.onboarding_error =
                                    "Max memory must be a number >= 512.".to_string();
                                return;
                            }
                        }
                    }
                    state.config.completed_onboarding = true;
                    state.save_config();
                    state.page = crate::app::events::Page::Home;
                    state.notify("Welcome to MONORYX");
                }
            });
        });
    });
}
