use crate::app::state::AppState;
use crate::ui::components::{
    badge, card_frame, empty_state, page_header, primary_button, stat, thin_progress,
};
use crate::ui::theme::{TEXT, TEXT2};
use egui::RichText;

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Home", "Play. Modify. Nothing else.");
    if !state.last_exit.is_empty() {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&state.last_exit).color(egui::Color32::from_rgb(0xE0, 0x5A, 0x5A)),
            );
            if ui.button("View Log").clicked() {
                state.set_page(crate::app::events::Page::Logs);
            }
        });
        ui.add_space(6.0);
    }
    let Some(cfg) = state.selected() else {
        card_frame(ui, |ui| {
            empty_state(
                ui,
                "No instances yet",
                "Create your first isolated instance to start playing.",
            );
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                if primary_button(ui, "New Instance").clicked() {
                    crate::ui::pages::instances::open_new_dialog(state);
                    state.set_page(crate::app::events::Page::Instances);
                }
            });
        });
        return;
    };
    let running = state.playing.get(&cfg.id).copied().unwrap_or(false);
    let installing = state.busy_install.contains_key(&cfg.id);
    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(&cfg.name).size(20.0).strong().color(TEXT));
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Minecraft {}", cfg.minecraft_version))
                            .size(12.5)
                            .color(TEXT2),
                    );
                    badge(ui, cfg.loader.display_name());
                    if !cfg.loader_version.is_empty() {
                        badge(ui, &cfg.loader_version);
                    }
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if running {
                    ui.add_enabled(false, egui::Button::new(RichText::new("RUNNING").strong()));
                } else if installing {
                    ui.add_enabled(false, egui::Button::new("WORKING..."));
                } else if primary_button(ui, "PLAY").clicked() {
                    crate::app::tasks::play_instance(state, cfg.id.clone());
                }
            });
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            stat(ui, "Mods", &state.instances.mod_count(&cfg.id).to_string());
            ui.add_space(18.0);
            stat(ui, "Memory", &format!("{} MB", cfg.memory_max_mb));
            ui.add_space(18.0);
            stat(
                ui,
                "Last played",
                cfg.last_played_at.as_deref().unwrap_or("Never"),
            );
        });
        if let Some((_, a, b)) = state.busy_install.get(&cfg.id) {
            ui.add_space(8.0);
            thin_progress(ui, Some(*a as f32 / (*b).max(1) as f32));
            ui.label(
                RichText::new(format!("Working... {a}/{b}"))
                    .size(11.0)
                    .color(TEXT2),
            );
        }
    });
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.button("Open folder").clicked() {
            let p = state.instances.game_dir(&cfg.id);
            let _ = open::that(p);
        }
        if ui.button("Edit instance").clicked() {
            state.edit_instance = Some(cfg.clone());
        }
        if ui.button("View logs").clicked() {
            state.set_page(crate::app::events::Page::Logs);
        }
        if ui.button("Repair").clicked() {
            state.global_status = "Repairing...".to_string();
            crate::app::tasks::repair_instance(state, cfg.id.clone());
        }
    });
    if state.instance_list.len() > 1 {
        ui.add_space(10.0);
        ui.label(RichText::new("Instances").size(13.0).strong().color(TEXT));
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for inst in state.instance_list.clone() {
                    let sel = Some(inst.id.clone()) == state.selected_instance;
                    let label = if sel {
                        format!("> {}", inst.name)
                    } else {
                        inst.name.clone()
                    };
                    if ui.selectable_label(sel, label).clicked() {
                        state.selected_instance = Some(inst.id.clone());
                        state.save_config();
                        state.refresh_library();
                    }
                }
            });
        });
    }
}
