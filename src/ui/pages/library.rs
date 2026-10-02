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
            state.updates_instance = state.selected_instance.clone();
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
    let curseforge_count = entries
        .iter()
        .filter(|entry| {
            entry
                .project_id
                .as_deref()
                .map(crate::curseforge::is_curseforge_slug)
                .unwrap_or(false)
        })
        .count();
    let modrinth_count = entries
        .iter()
        .filter(|entry| {
            entry
                .project_id
                .as_deref()
                .map(|id| !crate::curseforge::is_curseforge_slug(id))
                .unwrap_or(false)
        })
        .count();
    let manual_count = entries
        .len()
        .saturating_sub(curseforge_count + modrinth_count);
    let kind_label = match state.library_filter {
        ContentKind::Mod => "mods",
        ContentKind::Resourcepack => "resource packs",
        ContentKind::Shader => "shaders",
    };
    let source_summary = if curseforge_count > 0 {
        format!(
            "{} from Modrinth  ·  {} from CurseForge  ·  {} manual",
            modrinth_count, curseforge_count, manual_count
        )
    } else {
        format!(
            "{} from Modrinth  ·  {} manual",
            modrinth_count, manual_count
        )
    };
    ui.label(
        RichText::new(format!(
            "{} {}  ·  {} enabled  ·  {}",
            entries.len(),
            kind_label,
            enabled_count,
            source_summary
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
            if width >= 960.0 {
                ui.horizontal(|ui| {
                    let id_width = (width * 0.28).clamp(200.0, 320.0);
                    let details_width = (width * 0.18).clamp(160.0, 200.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(id_width, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.set_min_width(id_width);
                            entry_identity(ui, &e);
                        },
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(details_width, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.set_min_width(details_width);
                            entry_details(ui, &e, has_update);
                        },
                    );
                    let actions_width = ui.available_width();
                    ui.allocate_ui_with_layout(
                        egui::vec2(actions_width, 0.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            entry_actions(ui, state, &cfg.id, &e, true);
                        },
                    );
                });
            } else {
                ui.horizontal(|ui| {
                    let id_width = (width * 0.55).max(180.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(id_width, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.set_min_width(id_width);
                            entry_identity(ui, &e);
                        },
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| entry_details(ui, &e, has_update),
                    );
                });
                ui.add_space(6.0);
                if width >= 520.0 {
                    let actions_width = ui.available_width();
                    ui.allocate_ui_with_layout(
                        egui::vec2(actions_width, 0.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            entry_actions(ui, state, &cfg.id, &e, true);
                        },
                    );
                } else {
                    ui.horizontal_wrapped(|ui| entry_actions(ui, state, &cfg.id, &e, false));
                }
            }
        });
        ui.add_space(4.0);
    }
}

fn entry_identity(ui: &mut egui::Ui, entry: &InstalledEntry) {
    let title = entry.project_title.as_deref().unwrap_or(&entry.file_name);
    ui.add(egui::Label::new(RichText::new(title).size(15.0).strong().color(TEXT)).truncate());
    if title != entry.file_name {
        ui.add(
            egui::Label::new(RichText::new(&entry.file_name).size(11.5).color(TEXT2)).truncate(),
        );
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
    let source = if let Some(pid) = entry.project_id.as_deref() {
        if crate::curseforge::is_curseforge_slug(pid) {
            "CURSEFORGE"
        } else {
            "MODRINTH"
        }
    } else {
        "MANUAL FILE"
    };
    ui.label(RichText::new(source).size(10.5).strong().color(MUTED));
    if let Some(version) = entry.version_number.as_deref() {
        ui.add(
            egui::Label::new(
                RichText::new(format!("Version {version}"))
                    .size(11.5)
                    .color(TEXT2),
            )
            .truncate(),
        );
    }
    if !entry.game_version.is_empty() {
        let loader = if entry.loader.is_empty() {
            String::new()
        } else {
            format!(" · {}", entry.loader)
        };
        ui.add(
            egui::Label::new(
                RichText::new(format!("MC {}{loader}", entry.game_version))
                    .size(11.5)
                    .color(TEXT2),
            )
            .truncate(),
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryAction {
    Update,
    Reinstall,
    Toggle,
    OpenFolder,
    Remove,
}

fn entry_actions(
    ui: &mut egui::Ui,
    state: &mut AppState,
    instance_id: &str,
    entry: &InstalledEntry,
    right_aligned: bool,
) {
    let update = state
        .updates
        .iter()
        .find(|update| update.file_name == entry.file_name && update.kind == entry.kind)
        .cloned();
    let mut actions = Vec::with_capacity(5);
    if update.is_some() {
        actions.push(EntryAction::Update);
    }
    if entry.project_id.is_some() {
        actions.push(EntryAction::Reinstall);
    }
    actions.extend([
        EntryAction::Toggle,
        EntryAction::OpenFolder,
        EntryAction::Remove,
    ]);

    if !right_aligned {
        for action in actions {
            entry_action(ui, state, instance_id, entry, action, &update);
        }
        return;
    }

    ui.spacing_mut().item_spacing.x = 6.0;
    for action in actions.into_iter().rev() {
        entry_action(ui, state, instance_id, entry, action, &update);
    }
}

fn entry_action(
    ui: &mut egui::Ui,
    state: &mut AppState,
    instance_id: &str,
    entry: &InstalledEntry,
    action: EntryAction,
    update: &Option<crate::modrinth::updates::UpdateInfo>,
) {
    match action {
        EntryAction::Update => {
            if let Some(update) = &update {
                let busy = state.row_is_busy(&entry.file_name);
                if crate::ui::components::action_button_with_feedback(
                    ui,
                    &format!("Update to {}", update.new_version),
                    busy,
                )
                .clicked()
                {
                    state.begin_row_activity(&entry.file_name, "Updating");
                    crate::app::tasks::update_one(state, update.clone());
                }
            }
        }
        EntryAction::Reinstall => {
            if let Some(project_id) = entry.project_id.clone() {
                let busy = state.row_is_busy(&entry.file_name);
                let mut clicked = false;
                ui.add_enabled_ui(!busy, |ui| {
                    if content_secondary_button(ui, "Reinstall").clicked() {
                        clicked = true;
                    }
                });
                if clicked {
                    state.begin_row_activity(&entry.file_name, "Installing");
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
        EntryAction::Toggle => {
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
                        if let Err(err) = store.set_enabled(entry.kind, &entry.file_name, target) {
                            state.fail(err.user_message());
                        }
                        state.refresh_library();
                    }
                    Err(err) => state.fail(err.user_message()),
                }
            }
        }
        EntryAction::OpenFolder => {
            if content_secondary_button(ui, "Open folder").clicked() {
                let dir = match entry.kind {
                    ContentKind::Mod => state.instances.mods_dir(instance_id),
                    ContentKind::Resourcepack => state.instances.resourcepacks_dir(instance_id),
                    ContentKind::Shader => state.instances.shaderpacks_dir(instance_id),
                };
                let _ = open::that(dir);
            }
        }
        EntryAction::Remove => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ContentKind, InstalledEntry};
    use crate::storage::paths::MonoryxPaths;

    #[test]
    fn library_renders_without_panicking_wide_and_narrow() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let manager = crate::instance::InstanceManager::new(paths.clone());
        let inst = manager
            .create(
                "Test Instance".into(),
                "1.21.1".into(),
                crate::instance::LoaderKind::Fabric,
                "0.16.14".into(),
            )
            .unwrap();

        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths);
        state.selected_instance = Some(inst.id.clone());
        state.instance_list = vec![inst.clone()];
        state.library_entries = vec![
            InstalledEntry {
                file_name: "test-mod-1.0.jar".to_string(),
                kind: ContentKind::Mod,
                enabled: true,
                project_id: Some("test-mod".to_string()),
                project_slug: Some("test-mod".to_string()),
                project_title: Some("Test Mod".to_string()),
                version_id: Some("v1".to_string()),
                version_number: Some("1.0.0".to_string()),
                file_hash_sha512: None,
                file_hash_sha1: None,
                installed_at: "2026-01-01".to_string(),
                game_version: "1.21.1".to_string(),
                loader: "fabric".to_string(),
                size: 1024 * 1024,
            },
            InstalledEntry {
                file_name: "manual-addon.jar".to_string(),
                kind: ContentKind::Mod,
                enabled: false,
                project_id: None,
                project_slug: None,
                project_title: None,
                version_id: None,
                version_number: None,
                file_hash_sha512: None,
                file_hash_sha1: None,
                installed_at: "2026-01-01".to_string(),
                game_version: "".to_string(),
                loader: "".to_string(),
                size: 2048,
            },
        ];

        state.updates = vec![crate::modrinth::updates::UpdateInfo {
            file_name: "test-mod-1.0.jar".to_string(),
            kind: ContentKind::Mod,
            project_id: "test-mod".to_string(),
            title: "Test Mod".to_string(),
            current_version: "1.0.0".to_string(),
            new_version: "1.1.0+fabric".to_string(),
            new_version_id: "v2".to_string(),
        }];

        for width in [1536.0, 960.0, 860.0, 800.0, 480.0] {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 800.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            show(&mut state, ctx, ui);
                        });
                    });
                },
            );
        }
    }
}
