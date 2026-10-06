use crate::app::state::AppState;
use crate::content::{ContentKind, InstalledEntry};
use crate::ui::components::{
    badge, badge_accent, badge_ok, badge_warning, card_frame, compact_action_button_with_feedback,
    compact_danger_button, compact_hover_card_frame, compact_secondary_button,
    content_primary_button, content_secondary_button, empty_state, page_header,
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
            if crate::ui::components::pill_tab_button(ui, label, state.library_filter == k)
                .clicked()
            {
                state.library_filter = k;
            }
        }

        if state.library_filter == ContentKind::Mod {
            ui.separator();
            if crate::ui::components::pill_tab_button(ui, "☰ List", !state.library_view_hierarchy)
                .clicked()
            {
                state.library_view_hierarchy = false;
            }
            if crate::ui::components::pill_tab_button(
                ui,
                "🌲 Hierarchy",
                state.library_view_hierarchy,
            )
            .clicked()
            {
                state.library_view_hierarchy = true;
                state.ensure_hierarchy();
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

        if content_secondary_button(ui, "📦 Export Pack (.zip)")
            .on_hover_text("One-click export full instance pack to exports/ folder")
            .clicked()
        {
            state.export_pack(cfg.clone());
        }

        ui.menu_button("📋 Export List", |ui| {
            if ui.button("Copy Markdown Table").clicked() {
                let md =
                    crate::instance::export::export_mod_list_markdown(&cfg, &state.library_entries);
                ui.ctx().copy_text(md);
                state.notify("Copied Markdown mod list to clipboard");
                ui.close();
            }
            if ui.button("Copy Plain Text").clicked() {
                let txt =
                    crate::instance::export::export_mod_list_text(&cfg, &state.library_entries);
                ui.ctx().copy_text(txt);
                state.notify("Copied plain text mod list to clipboard");
                ui.close();
            }
            if ui.button("Save modlist.txt to exports/").clicked() {
                let exports_dir = crate::storage::paths::data_root().join("exports");
                let _ = crate::utils::fs::ensure_dir(&exports_dir);
                let safe_name = cfg
                    .name
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect::<String>();
                let file = exports_dir.join(format!("{safe_name}-mods.txt"));
                let txt =
                    crate::instance::export::export_mod_list_text(&cfg, &state.library_entries);
                if std::fs::write(&file, txt).is_ok() {
                    state.notify(format!(
                        "Saved {}",
                        file.file_name().unwrap_or_default().to_string_lossy()
                    ));
                    let _ = open::that_detached(&file);
                }
                ui.close();
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
    if state.library_view_hierarchy && state.library_filter == ContentKind::Mod {
        render_hierarchy_view(state, ui, &cfg);
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
        compact_hover_card_frame(ui, format!("lib_entry_{}", e.file_name), |ui| {
            let has_update = state
                .updates
                .iter()
                .any(|update| update.file_name == e.file_name && update.kind == e.kind);
            let width = ui.available_width();
            if width >= 960.0 {
                ui.horizontal(|ui| {
                    let id_width = (width * 0.28).clamp(200.0, 320.0);
                    let details_width = (width * 0.22).clamp(180.0, 240.0);
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
                    let id_width = (width * 0.52).max(180.0);
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
                ui.add_space(4.0);
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
        ui.add_space(3.0);
    }
}

fn entry_identity(ui: &mut egui::Ui, entry: &InstalledEntry) {
    ui.spacing_mut().item_spacing.y = 2.0;
    let title = entry.project_title.as_deref().unwrap_or(&entry.file_name);
    ui.add(egui::Label::new(RichText::new(title).size(14.0).strong().color(TEXT)).truncate());
    let sub = match (title != entry.file_name, entry.size > 0) {
        (true, true) => Some(format!(
            "{} · {}",
            entry.file_name,
            format_bytes(entry.size)
        )),
        (true, false) => Some(entry.file_name.clone()),
        (false, true) => Some(format_bytes(entry.size)),
        (false, false) => None,
    };
    if let Some(text) = sub {
        ui.add(egui::Label::new(RichText::new(text).size(11.0).color(TEXT2)).truncate());
    }
}

fn entry_details(ui: &mut egui::Ui, entry: &InstalledEntry, has_update: bool) {
    ui.spacing_mut().item_spacing.y = 2.0;
    let source = if let Some(pid) = entry.project_id.as_deref() {
        if crate::curseforge::is_curseforge_slug(pid) {
            "CURSEFORGE"
        } else {
            "MODRINTH"
        }
    } else {
        "MANUAL"
    };
    let (status, color) = if has_update {
        ("Update available", WARNING)
    } else if entry.enabled {
        ("Enabled", TEXT2)
    } else {
        ("Disabled", MUTED)
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(RichText::new(source).size(10.5).strong().color(MUTED));
        ui.label(RichText::new("·").size(10.5).color(MUTED));
        ui.label(RichText::new(status).size(11.0).color(color));
    });
    let mut parts = Vec::new();
    if let Some(version) = entry.version_number.as_deref() {
        parts.push(format!("v{version}"));
    }
    if !entry.game_version.is_empty() {
        let loader = if entry.loader.is_empty() {
            String::new()
        } else {
            format!(" {}", entry.loader)
        };
        parts.push(format!("MC {}{loader}", entry.game_version));
    }
    if !parts.is_empty() {
        ui.add(
            egui::Label::new(RichText::new(parts.join(" · ")).size(11.0).color(TEXT2)).truncate(),
        );
    }
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
                if compact_action_button_with_feedback(
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
                    if compact_secondary_button(ui, "Reinstall").clicked() {
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
            if compact_secondary_button(ui, if entry.enabled { "Disable" } else { "Enable" })
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
            if compact_secondary_button(ui, "Open folder").clicked() {
                let dir = match entry.kind {
                    ContentKind::Mod => state.instances.mods_dir(instance_id),
                    ContentKind::Resourcepack => state.instances.resourcepacks_dir(instance_id),
                    ContentKind::Shader => state.instances.shaderpacks_dir(instance_id),
                };
                let _ = open::that(dir);
            }
        }
        EntryAction::Remove => {
            if compact_danger_button(ui, "Remove").clicked() {
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

fn render_hierarchy_view(
    state: &mut AppState,
    ui: &mut egui::Ui,
    cfg: &crate::instance::config::InstanceConfig,
) {
    state.ensure_hierarchy();
    let hierarchy = match &state.cached_hierarchy {
        Some(h) => h.clone(),
        None => return,
    };

    if !hierarchy.all_missing.is_empty() {
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("⚠ Missing Dependencies Detected")
                        .size(13.0)
                        .strong()
                        .color(WARNING),
                );
                badge_warning(ui, &format!("{} Missing", hierarchy.all_missing.len()));
            });
            ui.add_space(4.0);
            for item in &hierarchy.all_missing {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("• {} requires", item.mod_name)).color(TEXT2));
                    ui.label(RichText::new(&item.display_dep).strong().color(WARNING));
                    let dep_query = item.search_query.clone();
                    if ui.small_button("Search on Discover").clicked() {
                        state.search.query = dep_query;
                        state.set_page(crate::app::Page::Discover);
                        state.run_search();
                    }
                });
            }
        });
        ui.add_space(8.0);
    }

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("PRIMARY MODS")
                .size(11.0)
                .strong()
                .color(MUTED),
        );
        badge(ui, &hierarchy.root_mods.len().to_string());
    });
    ui.add_space(4.0);

    if hierarchy.root_mods.is_empty() {
        card_frame(ui, |ui| {
            ui.label(
                RichText::new("No standalone mods detected.")
                    .size(12.0)
                    .color(MUTED),
            );
        });
    }

    for node in &hierarchy.root_mods {
        let e = &node.entry;
        compact_hover_card_frame(ui, format!("h_root_{}", e.file_name), |ui| {
            let has_update = state
                .updates
                .iter()
                .any(|u| u.file_name == e.file_name && u.kind == e.kind);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&node.display_name)
                                .size(14.0)
                                .strong()
                                .color(TEXT),
                        );
                        if let Some(ver) = &e.version_number {
                            badge(ui, ver);
                        }
                    });
                    if node.display_name != e.file_name {
                        ui.label(RichText::new(&e.file_name).size(11.0).color(MUTED));
                    }
                    if !node.dependencies.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("↳ Depends on:").size(11.0).color(TEXT2));
                            for dep in &node.dependencies {
                                badge_ok(ui, dep);
                            }
                        });
                    }
                    if !node.missing_dependencies.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("↳ Missing:").size(11.0).color(WARNING));
                            for m in &node.missing_dependencies {
                                badge_warning(ui, m);
                            }
                        });
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    entry_actions(ui, state, &cfg.id, e, true);
                    if has_update {
                        entry_details(ui, e, true);
                    }
                });
            });
        });
        ui.add_space(4.0);
    }

    if !hierarchy.shared_libraries.is_empty() {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("SHARED LIBRARIES & APIS")
                    .size(11.0)
                    .strong()
                    .color(MUTED),
            );
            badge(ui, &hierarchy.shared_libraries.len().to_string());
        });
        ui.add_space(4.0);

        for node in &hierarchy.shared_libraries {
            let e = &node.entry;
            compact_hover_card_frame(ui, format!("h_lib_{}", e.file_name), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&node.display_name)
                                    .size(14.0)
                                    .strong()
                                    .color(TEXT),
                            );
                            if let Some(ver) = &e.version_number {
                                badge(ui, ver);
                            }
                            if !node.required_by.is_empty() {
                                badge_accent(
                                    ui,
                                    &format!("Used by {} mods", node.required_by.len()),
                                );
                            } else {
                                badge(ui, "Library");
                            }
                        });
                        if !node.required_by.is_empty() {
                            let used_summary = node.required_by.join(", ");
                            ui.label(
                                RichText::new(format!("Required by: {used_summary}"))
                                    .size(11.0)
                                    .color(TEXT2),
                            );
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        entry_actions(ui, state, &cfg.id, e, true);
                    });
                });
            });
            ui.add_space(4.0);
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
        state.instance_list = vec![inst];
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
