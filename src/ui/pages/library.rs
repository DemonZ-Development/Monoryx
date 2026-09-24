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
                state.updates_checked = false;
                state.updates_summary.clear();
                state.updates_error.clear();
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
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Checking installed content...");
        });
    }
    if state.updates_checked && !state.updates_summary.is_empty() {
        ui.label(RichText::new(&state.updates_summary).color(TEXT2));
    }
    if !state.updates_error.is_empty() {
        ui.colored_label(crate::ui::theme::DANGER, &state.updates_error);
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
                        if state
                            .updates
                            .iter()
                            .any(|u| u.file_name == e.file_name && u.kind == e.kind)
                        {
                            badge(ui, "Update available");
                        }
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if let Some(update) = state
                                .updates
                                .iter()
                                .find(|u| u.file_name == e.file_name && u.kind == e.kind)
                                .cloned()
                            {
                                if ui
                                    .small_button(format!("Update to {}", update.new_version))
                                    .clicked()
                                {
                                    crate::app::tasks::update_one(state, update);
                                }
                            }
                            let label = if e.enabled { "Disable" } else { "Enable" };
                            if ui.small_button(label).clicked() {
                                let id = cfg.id.clone();
                                let name = e.file_name.clone();
                                let target = !e.enabled;
                                match state
                                    .instances
                                    .set_content_enabled(&id, &name, e.kind, target)
                                {
                                    Ok(_) => {
                                        let store = crate::content::ContentStore::for_instance(
                                            &state.instances.instance_dir(&id),
                                        );
                                        if let Err(err) = store.set_enabled(e.kind, &name, target) {
                                            state.fail(err.user_message());
                                        }
                                        state.refresh_library();
                                    }
                                    Err(err) => state.fail(err.user_message()),
                                }
                            }
                            ui.menu_button("More", |ui| {
                                if ui.button("Open folder").clicked() {
                                    let dir = match e.kind {
                                        ContentKind::Mod => state.instances.mods_dir(&cfg.id),
                                        ContentKind::Resourcepack => {
                                            state.instances.resourcepacks_dir(&cfg.id)
                                        }
                                        ContentKind::Shader => {
                                            state.instances.shaderpacks_dir(&cfg.id)
                                        }
                                    };
                                    let _ = open::that(dir);
                                    ui.close();
                                }
                                if ui.button("Remove").clicked() {
                                    match crate::modrinth::install::remove_project_blocking(
                                        &state.instances,
                                        &cfg.id,
                                        e.kind,
                                        &e.file_name,
                                    ) {
                                        Ok(()) => state.refresh_library(),
                                        Err(err) => state.fail(err),
                                    }
                                    ui.close();
                                }
                            });
                        });
                    });
                });
            });
        });
        ui.add_space(4.0);
    }
}
