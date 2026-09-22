use crate::app::state::AppState;
use crate::ui::components::{card_frame, empty_state, page_header};
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
            if let Some(frac) = op.fraction() {
                crate::ui::components::thin_progress(ui, Some(frac));
            } else {
                crate::ui::components::thin_progress(ui, None);
            }
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
            let state_icon = match d.state.as_str() {
                "queued" => " (queued)",
                "cancelled" => " (cancelled)",
                _ => "",
            };
            crate::ui::components::thin_progress(ui, frac);
            let total_str = d
                .total
                .map(crate::ui::theme::format_bytes)
                .unwrap_or_else(|| "?".to_string());
            ui.label(
                RichText::new(format!(
                    "{} / {} - {}{}",
                    crate::ui::theme::format_bytes(d.downloaded),
                    total_str,
                    crate::ui::theme::format_speed(d.speed_bps),
                    state_icon,
                ))
                .size(11.0)
                .color(TEXT2),
            );
        });
        ui.add_space(4.0);
    }
    if !state.downloads_history.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new("History").strong().color(TEXT));
        for h in state.downloads_history.clone().into_iter().take(20) {
            ui.horizontal(|ui| {
                if h.state == "completed" {
                    crate::ui::components::badge_ok(ui, "Completed");
                } else {
                    crate::ui::components::badge(ui, "Failed");
                }
                ui.label(
                    RichText::new(&h.label)
                        .size(12.0)
                        .color(TEXT),
                );
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
