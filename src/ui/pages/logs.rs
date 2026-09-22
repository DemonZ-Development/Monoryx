use crate::app::state::AppState;
use crate::ui::components::{card_frame, page_header};
use crate::ui::theme::{DANGER, TEXT, TEXT2};
use egui::RichText;

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Logs", "Launcher activity and the last game output.");
    ui.horizontal(|ui| {
        if ui.button("Clear view").clicked() {
            state.log_lines.clear();
        }
        if ui.button("Open logs folder").clicked() {
            let _ = open::that(state.paths.logs_dir());
        }
        if let Some(cfg) = state.selected() {
            if ui.button("Open instance logs").clicked() {
                let _ = open::that(state.instances.game_dir(&cfg.id).join("logs"));
            }
        }
    });
    ui.add_space(6.0);
    card_frame(ui, |ui| {
        egui::ScrollArea::vertical()
            .max_height(420.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if state.log_lines.is_empty() {
                    ui.label(
                        RichText::new("No log lines yet. Launch the game or install content.")
                            .color(TEXT2),
                    );
                }
                for line in state.log_lines.clone() {
                    ui.label(RichText::new(line).size(11.0).color(TEXT).monospace());
                }
                if !state.last_exit.is_empty() {
                    ui.label(
                        RichText::new(&state.last_exit)
                            .color(DANGER),
                    );
                }
            });
    });
}
