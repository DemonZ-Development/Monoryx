use crate::app::state::AppState;
use crate::content::ContentKind;
use crate::ui::components::{badge, card_frame, empty_state, hover_card_frame, page_header};
use crate::ui::theme::{TEXT, TEXT2};
use egui::RichText;

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Library", "Content installed in the selected instance.");
    let Some(cfg) = state.selected() else {
        card_frame(ui, |ui| {
            empty_state(ui, "No instance selected", "Create an instance first.");
        });
        return;
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(&cfg.name).strong().color(TEXT));
        for (k, label) in [
            (ContentKind::Mod, "Mods"),
            (ContentKind::Resourcepack, "Resource Packs"),
            (ContentKind::Shader, "Shaders"),
        ] {
            if ui
                .selectable_label(state.library_filter == k, label)
                .clicked()
            {
                state.library_filter = k;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Check updates").clicked() {
                state.updates_loading = true;
                crate::app::tasks::check_updates(state);
            }
            if !state.updates.is_empty()
                && ui
                    .button(format!("Update All ({})", state.updates.len()))
                    .clicked()
            {
                crate::app::tasks::update_all_mods(state);
            }
        });
    });
    if state.updates_loading {
        ui.label("Checking for updates...");
    }
    for u in state.updates.clone() {
        if entry_kind(state, &u.file_name) != Some(state.library_filter)
            && !matches_kind(u.kind, state.library_filter)
        {
            continue;
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{}: {} -> {}",
                    u.title, u.current_version, u.new_version
                ))
                .color(TEXT),
            );
            if ui.small_button("Update").clicked() {
                crate::app::tasks::update_one(state, u);
            }
        });
    }
    ui.add_space(6.0);
    let entries: Vec<_> = state
        .library_entries
        .clone()
        .into_iter()
        .filter(|e| e.kind == state.library_filter)
        .collect();
    if entries.is_empty() {
        card_frame(ui, |ui| {
            empty_state(
                ui,
                "Nothing here yet",
                "Install content from Discover or copy files manually.",
            );
        });
        let mods_dir = match state.library_filter {
            ContentKind::Mod => state.instances.mods_dir(&cfg.id),
            ContentKind::Resourcepack => state.instances.resourcepacks_dir(&cfg.id),
            ContentKind::Shader => state.instances.shaderpacks_dir(&cfg.id),
        };
        scan_manual(state, &cfg.id, &mods_dir);
        return;
    }
    for e in entries {
        hover_card_frame(ui, format!("lib_entry_{}", e.file_name), |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(
                            e.project_title
                                .clone()
                                .unwrap_or_else(|| e.file_name.clone()),
                        )
                        .size(14.0)
                        .strong()
                        .color(TEXT),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            e.file_name,
                            e.version_number.clone().unwrap_or_default()
                        ))
                        .size(11.0)
                        .color(TEXT2),
                    );
                    ui.horizontal(|ui| {
                        if e.project_id.is_some() {
                            badge(ui, "Modrinth");
                        } else {
                            badge(ui, "Manual");
                        }
                        if !e.enabled {
                            badge(ui, "Disabled");
                        }
                        if state.updates.iter().any(|u| u.file_name == e.file_name) {
                            badge(ui, "Update available");
                        }
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let label = if e.enabled { "Disable" } else { "Enable" };
                            if ui.small_button(label).clicked() {
                                let id = cfg.id.clone();
                                let name = e.file_name.clone();
                                let target = !e.enabled;
                                match state.instances.set_mod_enabled(&id, &name, target) {
                                    Ok(_) => state.refresh_library(),
                                    Err(err) => state.fail(err.user_message()),
                                }
                            }
                            if ui.small_button("Remove").clicked() {
                                match crate::modrinth::install::remove_project_blocking(
                                    &state.instances,
                                    &cfg.id,
                                    e.kind,
                                    &e.file_name,
                                ) {
                                    Ok(()) => state.refresh_library(),
                                    Err(err) => state.fail(err),
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            if e.project_id.is_some() && ui.small_button("Update").clicked() {
                                if let Some(u) = state
                                    .updates
                                    .iter()
                                    .find(|u| u.file_name == e.file_name)
                                    .cloned()
                                {
                                    crate::app::tasks::update_one(state, u);
                                } else {
                                    state.updates_loading = true;
                                    crate::app::tasks::check_updates(state);
                                }
                            }
                            if ui.small_button("Folder").clicked() {
                                let dir = match e.kind {
                                    ContentKind::Mod => state.instances.mods_dir(&cfg.id),
                                    ContentKind::Resourcepack => {
                                        state.instances.resourcepacks_dir(&cfg.id)
                                    }
                                    ContentKind::Shader => state.instances.shaderpacks_dir(&cfg.id),
                                };
                                let _ = open::that(dir.join(&e.file_name));
                            }
                        });
                    });
                });
            });
        });
        ui.add_space(4.0);
    }
}

fn matches_kind(a: ContentKind, b: ContentKind) -> bool {
    a == b
}

fn entry_kind(_state: &AppState, _file: &str) -> Option<ContentKind> {
    None
}

fn scan_manual(state: &mut AppState, _instance_id: &str, dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut found: Vec<String> = Vec::new();
    for e in entries.flatten() {
        if e.file_type().map(|t| t.is_file()).unwrap_or(false) {
            if let Some(name) = e.file_name().to_str().map(str::to_string) {
                if name.ends_with(".jar") || name.ends_with(".zip") {
                    found.push(name);
                }
            }
        }
    }
    if found.is_empty() {
        return;
    }
    state
        .log_lines
        .push(format!("{} manual file(s) present", found.len()));
}
