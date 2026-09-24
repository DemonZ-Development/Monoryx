use crate::app::state::AppState;
use crate::ui::components::{card_frame, page_header};
use crate::ui::theme::{DANGER, TEXT, TEXT2};
use egui::RichText;

pub fn show(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Logs", "Launcher activity and the last game output.");
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !state.log_lines.is_empty(),
                egui::Button::new("Copy visible log"),
            )
            .clicked()
        {
            ctx.copy_text(state.log_lines.join("\n"));
            state.notify("Visible log copied to clipboard");
        }
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
                for line in &state.log_lines {
                    ui.add(
                        egui::Label::new(RichText::new(line).size(11.0).color(TEXT).monospace())
                            .selectable(true),
                    );
                }
                if !state.last_exit.is_empty() {
                    ui.label(RichText::new(&state.last_exit).color(DANGER));
                }
            });
    });
    for (name, file) in [
        ("Forge installer", "forge-installer.log"),
        ("NeoForge installer", "neoforge-installer.log"),
    ] {
        let path = state.paths.logs_dir().join(file);
        if !path.exists() {
            continue;
        }
        ui.collapsing(name, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open file").clicked() {
                    let _ = open::that(&path);
                }
                if ui.button("Copy log").clicked() {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        ctx.copy_text(text);
                        state.notify(format!("{name} log copied to clipboard"));
                    }
                }
            });
            if let Ok(text) = std::fs::read_to_string(&path) {
                for line in text
                    .lines()
                    .rev()
                    .take(80)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                {
                    ui.add(
                        egui::Label::new(RichText::new(line).size(11.0).monospace())
                            .selectable(true),
                    );
                }
            }
        });
    }
}
