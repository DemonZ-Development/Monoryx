use crate::app::state::AppState;
use crate::ui::components::{card_frame, empty_state, page_header, progress_row, ProgressDetail};
use crate::ui::theme::{TEXT, TEXT2};
use egui::RichText;

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Downloads",
        "Active, queued, completed and failed transfers.",
    );
    for op in state.operations.clone().into_values() {
        card_frame(ui, |ui| {
            ui.label(RichText::new(&op.label).strong().color(TEXT));
            ui.label(RichText::new(&op.phase).size(11.0).color(TEXT2));
            progress_row(ui, op.fraction(), Some(ProgressDetail::default()));
        });
        ui.add_space(4.0);
    }
    if state.downloads.is_empty() && state.operations.is_empty() {
        card_frame(ui, |ui| {
            empty_state(
                ui,
                "No active downloads",
                "Install Minecraft, mods or modpacks to see progress here.",
            );
        });
    }
    for d in state.downloads.clone().into_values() {
        card_frame(ui, |ui| {
            ui.label(RichText::new(&d.label).strong().color(TEXT));
            let frac = d.total.map(|t| {
                if t > 0 {
                    (d.downloaded as f32 / t as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            });
            let eta = crate::ui::theme::format_eta(d.downloaded, d.total.unwrap_or(0), d.speed_bps);
            progress_row(
                ui,
                frac,
                Some(ProgressDetail {
                    downloaded: Some(d.downloaded),
                    total: d.total,
                    speed_bps: Some(d.speed_bps),
                    completed: None,
                    eta: Some(&eta),
                }),
            );
            let state_icon = match d.state.as_str() {
                "queued" => "Queued — waiting for a free download slot.",
                "cancelled" => "Cancelled.",
                _ => "",
            };
            if !state_icon.is_empty() {
                ui.label(RichText::new(state_icon).size(11.0).color(TEXT2));
            }
        });
        ui.add_space(4.0);
    }
    let history: Vec<_> = state
        .downloads_history
        .iter()
        .filter(|h| {
            h.state != "completed"
                || h.label.starts_with("Installing ")
                || h.id == "mod-install"
                || h.id == "modpack-install"
        })
        .take(20)
        .cloned()
        .collect();
    if !history.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new("History").strong().color(TEXT));
        for h in history {
            card_frame(ui, |ui| {
                ui.horizontal(|ui| {
                    if h.state == "completed" {
                        crate::ui::components::badge_ok(ui, "Completed");
                    } else {
                        crate::ui::components::badge(ui, "Failed");
                    }
                    ui.label(RichText::new(&h.label).size(12.0).color(TEXT));
                });
                if !h.message.is_empty() {
                    ui.label(RichText::new(&h.message).size(11.0).color(TEXT2));
                }
            });
        }
        ui.add_space(4.0);
        if ui.button("Clear history").clicked() {
            state.downloads_history.clear();
        }
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Open data folder").clicked() {
            let _ = open::that(state.paths.root());
        }
        if ui.button("Clear cache").clicked() {
            let cache = crate::storage::cache::DiskCache::new(
                state.paths.cache_dir(),
                std::time::Duration::from_secs(3600),
            );
            match cache.clear() {
                Ok(()) => state.notify("Cache cleared"),
                Err(e) => state.fail(e.user_message()),
            }
        }
    });
}
