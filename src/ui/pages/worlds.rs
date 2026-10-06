use crate::app::state::{AppState, WorldActionKind};
use crate::ui::components::{
    card_frame, content_primary_button, page_header, secondary_button, tab_button,
};
use crate::ui::theme::{format_bytes, MUTED, TEXT, TEXT2};
use egui::{RichText, Stroke};
use std::path::PathBuf;

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Worlds & Files",
        "Your saves, backups and game folders.",
    );
    ui.horizontal_wrapped(|ui| {
        let mut selection = state.selected_instance.clone();
        let selected = state.selected().map(|instance| format!("{} · {} · {}", instance.name, instance.minecraft_version, instance.loader.display_name())).unwrap_or_else(|| "Choose an instance".to_string());
        egui::ComboBox::from_id_salt("worlds-instance").width(270.0).selected_text(selected).show_ui(ui, |ui| {
            for instance in &state.instance_list {
                ui.selectable_value(&mut selection, Some(instance.id.clone()), format!("{} · {} · {}", instance.name, instance.minecraft_version, instance.loader.display_name()));
            }
        });
        if selection != state.selected_instance {
            state.selected_instance = selection;
            state.worlds.relative = PathBuf::new();
            state.worlds.selected_file = None;
            state.worlds.preview = None;
            state.worlds.snapshot = Default::default();
            state.save_config();
            state.refresh_library();
            state.refresh_worlds();
        }
        if secondary_button(ui, "Refresh").clicked() { state.refresh_worlds(); }
        ui.add_enabled_ui(!state.worlds.busy, |ui| {
            if secondary_button(ui, "Import instance").on_hover_text("Copy an existing Minecraft game folder into a new instance using this version and loader.").clicked() {
                if let (Some(path), Some(template)) = (rfd::FileDialog::new().pick_folder(), state.selected()) {
                    state.run_world_action(WorldActionKind::ImportGameFolder(path, Box::new(template)));
                }
            }
        });
        if state.worlds.loading || state.worlds.files_loading || state.worlds.busy {
            ui.spinner();
            ui.label(RichText::new(if state.worlds.busy { "Working..." } else { "Refreshing..." }).color(TEXT2));
        }
    });
    if state.selected().is_none() {
        card_frame(ui, |ui| {
            ui.label("Choose an instance to see its worlds and files.");
        });
        return;
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if tab_button(ui, "Worlds & backups", !state.worlds.show_files).clicked() {
            state.worlds.show_files = false;
        }
        if tab_button(ui, "Files", state.worlds.show_files).clicked() {
            state.worlds.show_files = true;
        }
    });
    if !state.worlds.error.is_empty() {
        ui.colored_label(crate::ui::theme::DANGER, &state.worlds.error);
    }
    ui.add_space(8.0);
    if state.worlds.show_files {
        show_files(state, ui);
    } else {
        show_worlds(state, ui);
    }
}

fn import_world(state: &mut AppState) {
    if let Some(path) = rfd::FileDialog::new().pick_folder() {
        state.run_world_action(WorldActionKind::ImportWorld(path));
    }
}

fn panel_heading(ui: &mut egui::Ui, title: &str, count: usize) {
    ui.label(RichText::new(title).size(17.0).strong().color(TEXT));
    ui.label(RichText::new(count.to_string()).size(12.0).color(MUTED));
}

fn empty_panel(ui: &mut egui::Ui, backup: bool, title: &str, detail: &str) {
    let p = crate::ui::theme::palette(ui.ctx());
    ui.add_space(16.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 58.0), egui::Sense::hover());
    let icon = egui::Rect::from_center_size(rect.center(), egui::vec2(50.0, 38.0));
    if backup {
        for shift in [8.0, 4.0, 0.0] {
            let r = icon.translate(egui::vec2(shift, -shift));
            ui.painter().rect(
                r,
                5,
                p.elevated2,
                Stroke::new(1.0_f32, p.border),
                egui::StrokeKind::Inside,
            );
        }
        ui.painter().line_segment(
            [
                icon.center() + egui::vec2(-9.0, 0.0),
                icon.center() + egui::vec2(9.0, 0.0),
            ],
            Stroke::new(3.0_f32, p.accent),
        );
    } else {
        ui.painter().rect(
            icon,
            5,
            p.elevated2,
            Stroke::new(1.0_f32, p.border),
            egui::StrokeKind::Inside,
        );
        let center = icon.center();
        ui.painter().line_segment(
            [
                center + egui::vec2(-16.0, 9.0),
                center + egui::vec2(-3.0, -6.0),
            ],
            Stroke::new(2.0_f32, p.accent),
        );
        ui.painter().line_segment(
            [
                center + egui::vec2(-3.0, -6.0),
                center + egui::vec2(14.0, 9.0),
            ],
            Stroke::new(2.0_f32, p.accent),
        );
        ui.painter()
            .circle_filled(center + egui::vec2(12.0, -9.0), 3.0, p.accent);
    }
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(title).size(15.0).strong().color(TEXT));
        ui.add(egui::Label::new(RichText::new(detail).size(12.0).color(TEXT2)).wrap());
    });
    ui.add_space(18.0);
}

fn show_worlds(state: &mut AppState, ui: &mut egui::Ui) {
    if ui.available_width() >= 760.0 {
        ui.columns(2, |columns| {
            world_panel(state, &mut columns[0]);
            backup_panel(state, &mut columns[1]);
        });
    } else {
        world_panel(state, ui);
        ui.add_space(8.0);
        backup_panel(state, ui);
    }
    ui.add_space(12.0);
    card_frame(ui, |ui| {
        ui.label(
            RichText::new("Instance folders")
                .size(15.0)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new("Browse the files used by this instance.")
                .size(12.0)
                .color(TEXT2),
        );
        ui.add_space(6.0);
        let id = state.selected_instance.clone().unwrap_or_default();
        let mods = state.mod_counts.get(&id).copied().unwrap_or(0);
        let folders = [
            (
                "saves",
                "Saves",
                format!("{} worlds", state.worlds.snapshot.worlds.len()),
            ),
            ("mods", "Mods", format!("{mods} enabled")),
            ("config", "Config", "Game and mod settings".to_string()),
        ];
        let columns = if ui.available_width() >= 700.0 { 4 } else { 2 };
        for chunk in folders.chunks(columns) {
            ui.columns(columns, |uis| {
                for (index, (folder, label, description)) in chunk.iter().enumerate() {
                    let ui = &mut uis[index];
                    if secondary_button(ui, label).clicked() {
                        state.worlds.show_files = true;
                        navigate_folder(state, PathBuf::from(folder));
                    }
                    ui.label(RichText::new(description).size(11.5).color(TEXT2));
                }
            });
        }
    });
}

fn world_panel(state: &mut AppState, ui: &mut egui::Ui) {
    card_frame(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            panel_heading(ui, "Your worlds", state.worlds.snapshot.worlds.len());
            ui.add_enabled_ui(!state.worlds.busy && !world_is_running(state), |ui| {
                if secondary_button(ui, "Import world").clicked() {
                    import_world(state);
                }
            });
        });
        ui.label(
            RichText::new("Choose a world to see its details and saved copies.")
                .size(12.0)
                .color(TEXT2),
        );
        ui.add_space(6.0);
        if state.worlds.snapshot.worlds.is_empty() && state.worlds.snapshot.backups.is_empty() {
            empty_panel(
                ui,
                false,
                if state.worlds.loading {
                    "Looking for your worlds…"
                } else {
                    "Your next adventure starts here"
                },
                "Create a world in Minecraft, or import one you already play.",
            );
        }
        let worlds = state.worlds.snapshot.worlds.clone();
        egui::ScrollArea::vertical()
            .id_salt("saved-worlds-list")
            .max_height(470.0)
            .show(ui, |ui| {
                for world in worlds {
                    let subtitle = [
                        world.mode.clone(),
                        world.version.as_ref().map(|v| format!("Minecraft {v}")),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" · ");
                    let last = world.last_played.unwrap_or(world.modified);
                    let date: chrono::DateTime<chrono::Local> = last.into();
                    let detail = format!(
                        "{} {}",
                        if world.last_played.is_some() {
                            "Played"
                        } else {
                            "Saved"
                        },
                        date.format("%b %d · %I:%M %p")
                    );
                    let selected = state.worlds.selected_world.as_deref() == Some(&world.name);
                    if world_row(
                        ui,
                        &world.name,
                        &world.display_name,
                        &subtitle,
                        &detail,
                        state.worlds.icons.get(&world.name),
                        selected,
                    )
                    .clicked()
                    {
                        state.worlds.selected_world = Some(world.name);
                    }
                }
                let mut archived = std::collections::BTreeSet::new();
                for backup in &state.worlds.snapshot.backups {
                    if !state
                        .worlds
                        .snapshot
                        .worlds
                        .iter()
                        .any(|world| world.name == backup.world_name)
                    {
                        archived.insert(backup.world_name.clone());
                    }
                }
                for name in archived {
                    let selected = state.worlds.selected_world.as_ref() == Some(&name);
                    if world_row(
                        ui,
                        &name,
                        &name,
                        "Saved copies available",
                        "Original world is no longer in this instance",
                        None,
                        selected,
                    )
                    .clicked()
                    {
                        state.worlds.selected_world = Some(name);
                    }
                }
            });
    });
}

fn world_is_running(state: &AppState) -> bool {
    state
        .selected_instance
        .as_ref()
        .is_some_and(|id| state.playing.get(id).copied().unwrap_or(false))
}

fn world_row(
    ui: &mut egui::Ui,
    id: &str,
    name: &str,
    subtitle: &str,
    detail: &str,
    icon: Option<&egui::TextureHandle>,
    selected: bool,
) -> egui::Response {
    ui.push_id(id, |ui| {
        let p = crate::ui::theme::palette(ui.ctx());
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 92.0), egui::Sense::click());
        ui.painter().rect(
            rect,
            8,
            if selected {
                p.elevated2
            } else if response.hovered() {
                p.hover
            } else {
                p.elevated
            },
            Stroke::new(1.0_f32, p.border),
            egui::StrokeKind::Inside,
        );
        let image_rect = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(12.0, 18.0),
            egui::vec2(56.0, 56.0),
        );
        if let Some(texture) = icon {
            ui.painter().image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            ui.painter().rect_filled(image_rect, 7, p.hover);
            ui.painter().text(
                image_rect.center(),
                egui::Align2::CENTER_CENTER,
                name.chars()
                    .next()
                    .unwrap_or('W')
                    .to_uppercase()
                    .to_string(),
                egui::FontId::proportional(23.0),
                p.accent,
            );
        }
        let painter = ui.painter().with_clip_rect(egui::Rect::from_min_max(
            rect.left_top() + egui::vec2(80.0, 8.0),
            rect.right_bottom() - egui::vec2(10.0, 8.0),
        ));
        for (y, text, size, color) in [
            (15.0, name, 15.0, TEXT),
            (40.0, subtitle, 11.5, TEXT2),
            (63.0, detail, 11.0, MUTED),
        ] {
            painter.text(
                rect.left_top() + egui::vec2(80.0, y),
                egui::Align2::LEFT_TOP,
                text,
                egui::FontId::proportional(size),
                color,
            );
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                ui.is_enabled(),
                selected,
                name,
            )
        });
        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(name)
    })
    .inner
}

fn backup_panel(state: &mut AppState, ui: &mut egui::Ui) {
    card_frame(ui, |ui| {
        let Some(selected) = state.worlds.selected_world.clone() else {
            empty_panel(ui, true, "A little peace of mind", "Select a world to make a backup. You can restore it as a separate save whenever you need it.");
            return;
        };
        let world = state
            .worlds
            .snapshot
            .worlds
            .iter()
            .find(|world| world.name == selected)
            .cloned();
        let backups: Vec<_> = state
            .worlds
            .snapshot
            .backups
            .iter()
            .filter(|backup| backup.world_name == selected)
            .cloned()
            .collect();
        ui.label(
            RichText::new(
                world
                    .as_ref()
                    .map_or(selected.as_str(), |world| world.display_name.as_str()),
            )
            .size(21.0)
            .strong()
            .color(TEXT),
        );
        if let Some(world) = &world {
            ui.horizontal_wrapped(|ui| {
                if let Some(mode) = &world.mode {
                    crate::ui::components::badge(ui, mode);
                }
                if let Some(version) = &world.version {
                    crate::ui::components::badge(ui, &format!("Minecraft {version}"));
                }
                ui.label(
                    RichText::new(
                        world
                            .bytes
                            .map(format_bytes)
                            .unwrap_or_else(|| "Size unavailable".into()),
                    )
                    .color(TEXT2),
                );
            });
            let date: chrono::DateTime<chrono::Local> =
                world.last_played.unwrap_or(world.modified).into();
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    if world.last_played.is_some() {
                        "Last played"
                    } else {
                        "Last saved"
                    },
                    date.format("%b %d, %Y at %I:%M %p")
                ))
                .color(TEXT2),
            );
            ui.label(
                RichText::new(format!("Folder: {}", world.name))
                    .size(11.5)
                    .color(MUTED),
            );
        }
        if let Some(backup) = backups.first() {
            let date: chrono::DateTime<chrono::Local> = backup.modified.into();
            ui.label(
                RichText::new(format!(
                    "Latest backup · {}",
                    date.format("%b %d at %I:%M %p")
                ))
                .color(crate::ui::theme::palette(ui.ctx()).accent),
            );
        } else {
            ui.label(RichText::new("This world hasn't been backed up yet.").color(TEXT2));
        }
        ui.add_space(6.0);
        let running = world_is_running(state);
        if running {
            ui.label(
                RichText::new("Close Minecraft first so your backup includes the latest save.")
                    .color(crate::ui::theme::WARNING),
            );
        }
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(world.is_some() && !state.worlds.busy && !running, |ui| {
                if content_primary_button(ui, "Back up now").clicked() {
                    state.run_world_action(WorldActionKind::BackUp(selected.clone()));
                }
            });
            if world.is_some() && secondary_button(ui, "Open folder").clicked() {
                if let Some(id) = &state.selected_instance {
                    let _ = open::that(state.instances.game_dir(id).join("saves").join(&selected));
                }
            }
        });
        ui.add_space(12.0);
        ui.separator();
        ui.horizontal(|ui| {
            panel_heading(ui, "Saved copies", backups.len());
        });
        ui.label(
            RichText::new("Restore creates another world. Your current save stays as it is.")
                .size(12.0)
                .color(TEXT2),
        );
        if backups.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new("Your first backup will appear here.").color(MUTED));
        }
        egui::ScrollArea::vertical()
            .id_salt("world-backups-list")
            .max_height(270.0)
            .show(ui, |ui| {
                for backup in backups {
                    let date: chrono::DateTime<chrono::Local> = backup.modified.into();
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(date.format("%b %d, %Y · %I:%M %p").to_string())
                            .strong()
                            .color(TEXT),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(format_bytes(backup.bytes)).color(TEXT2));
                        ui.add_enabled_ui(!state.worlds.busy && !running, |ui| {
                            if secondary_button(ui, "Restore as a new world").clicked() {
                                state.run_world_action(WorldActionKind::Restore(backup.file_name));
                            }
                        });
                    });
                    ui.separator();
                }
            });
    });
}
fn navigate_folder(state: &mut AppState, relative: PathBuf) {
    state.worlds.relative = relative;
    state.worlds.preview = None;
    state.worlds.selected_file = None;
    state.worlds.error.clear();
    state.refresh_files();
}

fn show_files(state: &mut AppState, ui: &mut egui::Ui) {
    let Some(id) = state.selected_instance.clone() else {
        return;
    };
    ui.horizontal_wrapped(|ui| {
        if secondary_button(ui, "Game folder").clicked() {
            navigate_folder(state, PathBuf::new());
        }
        ui.add_enabled_ui(!state.worlds.relative.as_os_str().is_empty(), |ui| {
            if secondary_button(ui, "Up one folder").clicked() {
                let mut parent = state.worlds.relative.clone();
                parent.pop();
                navigate_folder(state, parent);
            }
        });
        ui.label(
            RichText::new(state.worlds.relative.display().to_string())
                .monospace()
                .color(TEXT2),
        );
        if secondary_button(ui, "Open in Explorer").clicked() {
            let _ = open::that(state.instances.game_dir(&id).join(&state.worlds.relative));
        }
    });
    ui.add_space(8.0);
    if ui.available_width() >= 760.0 {
        ui.columns(2, |columns| {
            file_list(state, &mut columns[0]);
            file_preview(state, &mut columns[1]);
        });
    } else {
        file_list(state, ui);
        ui.add_space(8.0);
        file_preview(state, ui);
    }
}

fn file_list(state: &mut AppState, ui: &mut egui::Ui) {
    card_frame(ui, |ui| {
        ui.label(
            RichText::new("Folders and files")
                .size(15.0)
                .strong()
                .color(TEXT),
        );
        ui.separator();
        let entries = state.worlds.entries.clone();
        if entries.is_empty() && !state.worlds.files_loading {
            ui.label(RichText::new("This folder is empty.").color(TEXT2));
        }
        egui::ScrollArea::vertical()
            .id_salt("instance-file-list")
            .max_height(400.0)
            .show(ui, |ui| {
                for entry in entries {
                    let label = if entry.is_dir {
                        format!("▸  {}", entry.name)
                    } else {
                        format!("{}  ·  {}", entry.name, format_bytes(entry.bytes))
                    };
                    let selected = state.worlds.selected_file.as_ref() == Some(&entry.relative);
                    let response = ui.add_sized(
                        [ui.available_width(), 34.0],
                        egui::Button::new(label).selected(selected),
                    );
                    if response.clicked() {
                        if entry.is_dir {
                            navigate_folder(state, entry.relative);
                        } else {
                            state.preview_file(entry.relative);
                        }
                    }
                }
            });
    });
}

fn file_preview(state: &mut AppState, ui: &mut egui::Ui) {
    card_frame(ui, |ui| {
        ui.label(RichText::new("Preview").size(15.0).strong().color(TEXT));
        ui.separator();
        if let Some((path, content)) = &state.worlds.preview {
            ui.label(
                RichText::new(path.file_name().unwrap_or_default().to_string_lossy())
                    .strong()
                    .color(TEXT),
            );
            egui::ScrollArea::both()
                .id_salt("instance-text-preview")
                .max_height(400.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(RichText::new(content).monospace().size(12.0))
                            .selectable(true),
                    );
                });
        } else if state.worlds.selected_file.is_some() {
            ui.spinner();
            ui.label("Loading preview...");
        } else {
            empty_panel(
                ui,
                true,
                "Select a text file",
                "Read configs, logs and other text files here.",
            );
        }
    });
}
