use crate::app::state::AppState;
use crate::ui::components::{card_frame, page_header};
use crate::ui::theme::{format_bytes, palette, MUTED, TEXT, TEXT2};
use egui::{CornerRadius, RichText, Stroke};

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Screenshots", "Images captured in Minecraft with F2.");
    ui.horizontal(|ui| {
        let name = state
            .screenshots_filter
            .as_ref()
            .and_then(|id| {
                state
                    .instance_list
                    .iter()
                    .find(|instance| &instance.id == id)
            })
            .map_or("All instances", |instance| instance.name.as_str());
        let mut choice = state.screenshots_filter.clone();
        egui::ComboBox::from_id_salt("screenshot-instance-filter")
            .selected_text(name)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut choice, None, "All instances");
                for instance in &state.instance_list {
                    ui.selectable_value(
                        &mut choice,
                        Some(instance.id.clone()),
                        format!("{} · {}", instance.name, instance.minecraft_version),
                    );
                }
            });
        if choice != state.screenshots_filter {
            state.screenshots_filter = choice;
            state.screenshots_visible_count = 48;
        }
        if ui.button("Refresh").clicked() {
            state.refresh_screenshots();
        }
        if state.screenshots_loading {
            ui.spinner();
        }
    });
    ui.add_space(8.0);
    let entries: Vec<_> = state
        .screenshots
        .iter()
        .filter(|entry| {
            state
                .screenshots_filter
                .as_ref()
                .is_none_or(|id| id == &entry.instance_id)
        })
        .cloned()
        .collect();
    ui.label(RichText::new(format!("{} images", entries.len())).color(TEXT2));
    ui.add_space(8.0);
    if entries.is_empty() {
        card_frame(ui, |ui| {
            ui.label(
                RichText::new("No screenshots yet")
                    .size(18.0)
                    .strong()
                    .color(TEXT),
            );
            ui.label(RichText::new("Press F2 while playing. Images saved by your MONORYX instances will appear here.").color(TEXT2));
            if let Some(instance) = state.selected() {
                if ui.button("Open screenshot folder").clicked() {
                    let _ = open::that(state.instances.game_dir(&instance.id).join("screenshots"));
                }
            }
        });
        return;
    }

    let width = ui.available_width();
    let columns = ((width / 260.0).floor() as usize).clamp(1, 4);
    let gap = 12.0;
    let cell_width =
        ((width - gap * (columns.saturating_sub(1)) as f32) / columns as f32).max(150.0);
    let visible = entries.len().min(state.screenshots_visible_count);
    for chunk in entries[..visible].chunks(columns) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for entry in chunk {
                let p = palette(ui.ctx());
                ui.allocate_ui_with_layout(
                    egui::vec2(cell_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let response = egui::Frame::new()
                            .fill(p.elevated)
                            .stroke(Stroke::new(1.0_f32, p.border))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                let preview = egui::vec2(
                                    (cell_width - 20.0).max(100.0),
                                    (cell_width * 0.55).max(90.0),
                                );
                                if let Some(texture) = state.screenshot_thumbnails.get(&entry.path)
                                {
                                    ui.add(
                                        egui::Image::new((texture.id(), preview))
                                            .fit_to_exact_size(preview)
                                            .corner_radius(6),
                                    );
                                } else {
                                    let prospective = egui::Rect::from_min_size(
                                        ui.next_widget_position(),
                                        preview,
                                    );
                                    if ui.clip_rect().intersects(prospective) {
                                        state.ensure_screenshot_thumb(&entry.path);
                                    }
                                    let (rect, _) =
                                        ui.allocate_exact_size(preview, egui::Sense::hover());
                                    ui.painter().rect_filled(rect, 6, p.elevated2);
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "Loading preview",
                                        egui::FontId::proportional(12.0),
                                        MUTED,
                                    );
                                }
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(
                                        entry
                                            .path
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy(),
                                    )
                                    .strong()
                                    .color(TEXT),
                                )
                                .on_hover_text(entry.path.display().to_string());
                                ui.label(
                                    RichText::new(format!(
                                        "{} · {}",
                                        entry.instance_name,
                                        format_bytes(entry.bytes)
                                    ))
                                    .size(11.0)
                                    .color(TEXT2),
                                );
                                let captured: chrono::DateTime<chrono::Local> =
                                    entry.modified.into();
                                ui.label(
                                    RichText::new(
                                        captured.format("%b %-d, %Y · %I:%M %p").to_string(),
                                    )
                                    .size(11.0)
                                    .color(TEXT2),
                                );
                            });
                        let click = ui.interact(
                            response.response.rect,
                            ui.make_persistent_id(&entry.path),
                            egui::Sense::click(),
                        );
                        if click.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if click.clicked() {
                            state.open_screenshot(&entry.path);
                        }
                    },
                );
            }
        });
        ui.add_space(gap);
    }
    if visible < entries.len()
        && ui
            .button(format!("Show more ({})", entries.len() - visible))
            .clicked()
    {
        state.screenshots_visible_count += 48;
    }
}
