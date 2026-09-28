use crate::app::state::AppState;
use crate::content::{ContentKind, InstalledEntry};
use crate::ui::components::{
    card_frame, content_primary_button, content_secondary_button, empty_state, hover_card_frame,
    page_header,
};
use crate::ui::theme::{format_bytes, MUTED, TEXT, TEXT2, WARNING};
use egui::RichText;

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Library", "Content installed in the selected instance.");
    ui.horizontal(|ui| {
        ui.label(RichText::new("Instance:").strong().color(TEXT));
        let mut selection = state.selected_instance.clone();
        egui::ComboBox::from_id_salt("library-target-instance")
            .selected_text(
                state
                    .selected()
                    .as_ref()
                    .map_or("Choose an instance", |instance| instance.name.as_str()),
            )
            .show_ui(ui, |ui| {
                for instance in &state.instance_list {
                    ui.selectable_value(
                        &mut selection,
                        Some(instance.id.clone()),
                        format!(
                            "{} · MC {} · {}",
                            instance.name,
                            instance.minecraft_version,
                            instance.loader.display_name()
                        ),
                    );
                }
            });
        if selection != state.selected_instance {
            state.selected_instance = selection;
            state.save_config();
            state.refresh_library();
        }
    });
    ui.add_space(8.0);
    let Some(cfg) = state.selected() else {
        card_frame(ui, |ui| {
            empty_state(ui, "No instance selected", "Create an instance first.");
        });
        return;
    };
    ui.horizontal_wrapped(|ui| {
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
    });
    ui.horizontal(|ui| {
        if ui
            .add_enabled_ui(!state.updates_loading, |ui| {
                content_secondary_button(ui, "Check updates")
            })
            .inner
            .clicked()
        {
            state.updates_loading = true;
            state.updates_checked = false;
            state.updates_summary.clear();
            state.updates_error.clear();
            crate::app::tasks::check_updates(state);
        }
        if !state.updates.is_empty()
            && content_primary_button(ui, &format!("Update All ({})", state.updates.len()))
                .clicked()
        {
            crate::app::tasks::update_all_mods(state);
        }
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
        .iter()
        .filter(|e| e.kind == state.library_filter)
        .cloned()
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
    let enabled_count = entries.iter().filter(|entry| entry.enabled).count();
    let modrinth_count = entries
        .iter()
        .filter(|entry| entry.project_id.is_some())
        .count();
    let kind_label = match state.library_filter {
        ContentKind::Mod => "mods",
        ContentKind::Resourcepack => "resource packs",
        ContentKind::Shader => "shaders",
    };
    ui.label(
        RichText::new(format!(
            "{} {}  ·  {} enabled  ·  {} from Modrinth  ·  {} manual",
            entries.len(),
            kind_label,
            enabled_count,
            modrinth_count,
            entries.len() - modrinth_count
        ))
        .size(11.5)
        .color(TEXT2),
    );
    ui.add_space(2.0);
    for e in entries {
        hover_card_frame(ui, format!("lib_entry_{}", e.file_name), |ui| {
            let has_update = state
                .updates
                .iter()
                .any(|update| update.file_name == e.file_name && update.kind == e.kind);
            let width = ui.available_width();
            if width >= 950.0 {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(width * 0.35, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| entry_identity(ui, &e),
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(width * 0.20, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| entry_details(ui, &e, has_update),
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(width * 0.45 - 24.0, 0.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| entry_actions(ui, state, &cfg.id, &e, true),
                    );
                });
            } else {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(width * 0.60, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| entry_identity(ui, &e),
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(width * 0.40 - 10.0, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| entry_details(ui, &e, has_update),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| entry_actions(ui, state, &cfg.id, &e, false));
            }
        });
        ui.add_space(4.0);
    }
}

fn entry_identity(ui: &mut egui::Ui, entry: &InstalledEntry) {
    let title = entry.project_title.as_deref().unwrap_or(&entry.file_name);
    ui.label(RichText::new(title).size(15.0).strong().color(TEXT));
    if title != entry.file_name {
        ui.label(RichText::new(&entry.file_name).size(11.5).color(TEXT2));
    }
    if entry.size > 0 {
        ui.label(
            RichText::new(format_bytes(entry.size))
                .size(11.0)
                .color(MUTED),
        );
    }
}

fn entry_details(ui: &mut egui::Ui, entry: &InstalledEntry, has_update: bool) {
    let source = if entry.project_id.is_some() {
        "MODRINTH"
    } else {
        "MANUAL FILE"
    };
    ui.label(RichText::new(source).size(10.5).strong().color(MUTED));
    if let Some(version) = entry.version_number.as_deref() {
        ui.label(
            RichText::new(format!("Version {version}"))
                .size(11.5)
                .color(TEXT2),
        );
    }
    if !entry.game_version.is_empty() {
        let loader = if entry.loader.is_empty() {
            String::new()
        } else {
            format!(" · {}", entry.loader)
        };
        ui.label(
            RichText::new(format!("MC {}{loader}", entry.game_version))
                .size(11.5)
                .color(TEXT2),
        );
    }
    let (status, color) = if has_update {
        ("Update available", WARNING)
    } else if entry.enabled {
        ("Enabled", TEXT2)
    } else {
        ("Disabled", MUTED)
    };
    ui.label(RichText::new(status).size(11.5).color(color));
}

fn entry_actions(
    ui: &mut egui::Ui,
    state: &mut AppState,
    instance_id: &str,
    entry: &InstalledEntry,
    right_aligned: bool,
) {
    #[derive(Clone, Copy)]
    enum Action {
        Update,
        Reinstall,
        Toggle,
        OpenFolder,
        Remove,
    }
    let update = state
        .updates
        .iter()
        .find(|update| update.file_name == entry.file_name && update.kind == entry.kind)
        .cloned();
    let mut actions = Vec::with_capacity(5);
    if update.is_some() {
        actions.push(Action::Update);
    }
    if entry.project_id.is_some() {
        actions.push(Action::Reinstall);
    }
    actions.extend([Action::Toggle, Action::OpenFolder, Action::Remove]);
    if right_aligned {
        actions.reverse();
    }
    for action in actions {
        match action {
            Action::Update => {
                if let Some(update) = &update {
                    if content_primary_button(ui, &format!("Update to {}", update.new_version))
                        .clicked()
                    {
                        crate::app::tasks::update_one(state, update.clone());
                    }
                }
            }
            Action::Reinstall => {
                if let Some(project_id) = entry.project_id.clone() {
                    let response = if update.is_some() {
                        content_secondary_button(ui, "Reinstall")
                    } else {
                        content_primary_button(ui, "Reinstall")
                    };
                    if response.clicked() {
                        crate::app::tasks::install_content(
                            state,
                            project_id.clone(),
                            entry.project_slug.clone().unwrap_or(project_id),
                            entry
                                .project_title
                                .clone()
                                .unwrap_or_else(|| entry.file_name.clone()),
                            entry.kind,
                            None,
                        );
                    }
                }
            }
            Action::Toggle => {
                if content_secondary_button(ui, if entry.enabled { "Disable" } else { "Enable" })
                    .clicked()
                {
                    let target = !entry.enabled;
                    match state.instances.set_content_enabled(
                        instance_id,
                        &entry.file_name,
                        entry.kind,
                        target,
                    ) {
                        Ok(_) => {
                            let store = crate::content::ContentStore::for_instance(
                                &state.instances.instance_dir(instance_id),
                            );
                            if let Err(err) =
                                store.set_enabled(entry.kind, &entry.file_name, target)
                            {
                                state.fail(err.user_message());
                            }
                            state.refresh_library();
                        }
                        Err(err) => state.fail(err.user_message()),
                    }
                }
            }
            Action::OpenFolder => {
                if content_secondary_button(ui, "Open folder").clicked() {
                    let dir = match entry.kind {
                        ContentKind::Mod => state.instances.mods_dir(instance_id),
                        ContentKind::Resourcepack => state.instances.resourcepacks_dir(instance_id),
                        ContentKind::Shader => state.instances.shaderpacks_dir(instance_id),
                    };
                    let _ = open::that(dir);
                }
            }
            Action::Remove => {
                if crate::ui::components::danger_button(ui, "Remove").clicked() {
                    state.pending_content_delete = Some((
                        instance_id.to_string(),
                        entry.kind,
                        entry.file_name.clone(),
                        entry
                            .project_title
                            .clone()
                            .unwrap_or_else(|| entry.file_name.clone()),
                    ));
                }
            }
        }
    }
}
