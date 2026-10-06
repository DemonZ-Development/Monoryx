use crate::app::state::AppState;
use crate::instance::config::LoaderKind;
use crate::ui::components::{
    badge, badge_warning, card_frame, empty_state, field_label, hover_card_frame, page_header,
    primary_button, step_rail,
};
use crate::ui::theme::{DANGER, TEXT, TEXT2};
use egui::{CornerRadius, RichText, Stroke};

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Instances",
        "Keep each Minecraft version, its worlds, and its mods together.",
    );
    ui.horizontal(|ui| {
        if primary_button(ui, "+ New instance").clicked() {
            open_new_dialog(state);
        }
        ui.menu_button("Import", |ui| {
            if ui.button("Modrinth pack (.mrpack)").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Modrinth Modpack", &["mrpack"])
                    .pick_file()
                {
                    state.global_status = "Importing modpack...".to_string();
                    crate::app::tasks::install_pack_file(state, path);
                }
                ui.close();
            }
            if ui.button("MONORYX archive (.zip)").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("MONORYX export", &["zip"])
                    .pick_file()
                {
                    let manager = state.instances.clone();
                    let tx = state.tx.clone();
                    let ctx = state.egui_ctx.clone();
                    state.notify("Importing local instance archive...");
                    state.spawn(async move {
                        let result = tokio::task::spawn_blocking(move || {
                            crate::instance::export::import_instance_export(&manager, &path)
                                .map(|report| crate::app::events::ImportedInstance {
                                    id: report.config.id,
                                    stripped_args: report.stripped_args,
                                })
                                .map_err(|error| error.user_message())
                        })
                        .await
                        .unwrap_or_else(|error| Err(error.to_string()));
                        let _ = tx.send(crate::app::events::AppEvent::InstanceImported(result));
                        ctx.request_repaint();
                    });
                }
                ui.close();
            }
        });
        if ui
            .checkbox(&mut state.show_snapshots, "Show snapshots")
            .changed()
        {
            state.new_draft.versions = state.selected_version_list();
            state.save_config();
        }
    });
    ui.add_space(6.0);
    if state.instance_list.is_empty() {
        card_frame(ui, |ui| {
            empty_state(ui, "No instances", "Create one to install Minecraft.");
        });
    }
    for inst in state.instance_list.clone() {
        let running = state.playing.get(&inst.id).copied().unwrap_or(false);
        let is_selected = state.selected_instance.as_deref() == Some(&inst.id);
        let boost_on = inst.boost_mode.unwrap_or(state.config.boost_mode);

        hover_card_frame(ui, &inst.id, |ui| {
            ui.horizontal(|ui| {
                crate::ui::components::render_instance_thumbnail(
                    ui,
                    36.0,
                    &inst.name,
                    inst.loader.display_name(),
                    is_selected,
                );
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&inst.name).size(17.0).strong().color(TEXT));
                        if is_selected {
                            crate::ui::components::badge_accent(ui, "Active");
                        }
                        if boost_on {
                            crate::ui::components::badge_boost(ui, "Eco Mode");
                        }
                    });
                    ui.horizontal(|ui| {
                        badge(ui, &format!("MC {}", inst.minecraft_version));
                        badge(ui, inst.loader.display_name());
                        if !inst.loader_version.is_empty() {
                            badge(ui, &inst.loader_version);
                        }

                        match state.instance_readiness(&inst) {
                            crate::instance::Readiness::Ready => {}
                            crate::instance::Readiness::Installing => {
                                badge(ui, "Installing");
                            }
                            crate::instance::Readiness::NotDownloaded => {
                                badge_warning(ui, "Not downloaded");
                            }
                        }
                        badge(
                            ui,
                            &format!(
                                "{} mods",
                                state.mod_counts.get(&inst.id).copied().unwrap_or(0)
                            ),
                        );
                        let ram_text = if boost_on
                            && inst.memory_max_mb == crate::utils::system::default_max_memory_mb()
                        {
                            format!(
                                "{} MB (Eco Mode)",
                                crate::utils::system::default_boost_max_memory_mb()
                            )
                        } else {
                            format!("{} MB RAM", inst.memory_max_mb)
                        };
                        badge(ui, &ram_text);
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if running {
                        let btn = egui::Button::new(
                            RichText::new("RUNNING")
                                .strong()
                                .color(crate::ui::theme::OK),
                        )
                        .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                        .stroke(Stroke::new(
                            1.0_f32,
                            crate::ui::theme::palette(ui.ctx()).border,
                        ))
                        .corner_radius(CornerRadius::same(8));
                        ui.add_enabled(false, btn);
                    } else if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Play")
                                    .strong()
                                    .color(crate::ui::theme::palette(ui.ctx()).accent_text),
                            )
                            .fill(crate::ui::theme::palette(ui.ctx()).accent)
                            .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        state.selected_instance = Some(inst.id.clone());
                        state.save_config();
                        crate::app::tasks::play_instance(state, inst.id.clone());
                    }
                });
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button("Select").clicked() {
                    state.selected_instance = Some(inst.id.clone());
                    state.save_config();
                    state.refresh_library();
                }
                if ui.button("Edit").clicked() {
                    state.edit_instance = Some(inst.clone());
                }
                ui.menu_button("More", |ui| {
                    if ui.button("Duplicate").clicked() {
                        match state
                            .instances
                            .duplicate(&inst.id, format!("{} Copy", inst.name))
                        {
                            Ok(_) => {
                                state.refresh_instances();
                                state.notify("Instance duplicated");
                            }
                            Err(e) => state.fail(e.user_message()),
                        }
                    }
                    if ui.button("Open folder").clicked() {
                        let _ = open::that(state.instances.game_dir(&inst.id));
                    }
                    if ui.button("Repair files").clicked() {
                        state.global_status = "Repairing...".to_string();
                        crate::app::tasks::repair_instance(state, inst.id.clone());
                    }
                    if crate::ui::components::danger_button(ui, "Delete instance").clicked() {
                        state.confirm_delete = Some(inst.id.clone());
                        state.confirm_title = format!("Delete \"{}\"?", inst.name);
                    }
                });
            });
        });
        ui.add_space(6.0);
    }
    if state.show_new_instance {
        let response = egui::Modal::new(egui::Id::new("new-instance"))
            .backdrop_color(egui::Color32::TRANSPARENT)
            .frame(
                egui::Frame::popup(&_ctx.style())
                    .inner_margin(24)
                    .corner_radius(16),
            )
            .show(_ctx, |ui| {
                crate::ui::components::dialog_content(ui);
                ui.set_width((_ctx.screen_rect().width() - 48.0).clamp(320.0, 540.0));
                show_new_dialog(state, ui);
            });
        if response.should_close() {
            state.show_new_instance = false;
        }
    }
    if let Some(id) = state.confirm_delete.clone() {
        let title = state.confirm_title.clone();
        egui::Window::new(title)
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(_ctx, |ui| {
                ui.label("This removes the instance and its game files. This cannot be undone.");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.confirm_delete = None;
                    }
                    if crate::ui::components::danger_button(ui, "Delete instance").clicked() {
                        match state.instances.delete(&id, true) {
                            Ok(()) => {
                                state.confirm_delete = None;
                                state.refresh_instances();
                                state.notify("Instance deleted");
                            }
                            Err(e) => state.fail(e.user_message()),
                        }
                    }
                });
            });
    }
}

pub fn open_new_dialog(state: &mut AppState) {
    state.new_draft = Default::default();
    state.new_draft.loader = LoaderKind::Vanilla;
    state.new_draft.version = state
        .manifest
        .as_ref()
        .map(|manifest| manifest.latest.release.clone())
        .unwrap_or_default();
    state.new_draft.dialog_seq = next_dialog_seq();
    state.show_new_instance = true;
    state.load_patch_notes();
    if state.manifest.is_none() && !state.manifest_loading {
        state.spawn_initial();
    }
}

fn next_dialog_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

fn loader_pill(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let fade = ui
        .ctx()
        .animate_bool_with_time(ui.id().with(("loader", label)), selected, 0.18);
    let p = crate::ui::theme::palette(ui.ctx());
    let fill = p.elevated2.lerp_to_gamma(p.accent, fade);
    let text = TEXT.lerp_to_gamma(p.accent_text, fade);
    ui.add(
        egui::Button::new(RichText::new(label).size(12.5).strong().color(text))
            .fill(fill)
            .stroke(Stroke::new(
                1.0_f32,
                crate::ui::theme::palette(ui.ctx()).border,
            ))
            .corner_radius(CornerRadius::same(16)),
    )
}

fn show_version_changelog(state: &mut AppState, ui: &mut egui::Ui) {
    let version = state.new_draft.version.clone();
    if version.is_empty() {
        return;
    }
    let entry = state
        .manifest
        .as_ref()
        .and_then(|manifest| manifest.versions.iter().find(|entry| entry.id == version))
        .cloned();
    let note = state.patch_notes.get(&version).cloned();
    egui::CollapsingHeader::new("Version details and changelog")
        .default_open(false)
        .show(ui, |ui| {
            if let Some(entry) = entry {
                ui.label(
                    RichText::new(format!(
                        "{} · Released {}",
                        entry.kind,
                        entry.release_time.get(..10).unwrap_or(&entry.release_time)
                    ))
                    .color(TEXT2),
                );
            }
            if let Some(note) = note {
                ui.label(RichText::new(&note.title).strong());
                if !note.short_text.is_empty() {
                    ui.label(&note.short_text);
                }
                if state.patch_note_full_loading.as_deref() == Some(&version) {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading changelog...");
                    });
                } else if state
                    .patch_note_full
                    .as_ref()
                    .is_some_and(|(stored, _)| stored == &version)
                {
                    if let Some((_, full)) = &state.patch_note_full {
                        egui::ScrollArea::vertical()
                            .max_height(230.0)
                            .show(ui, |ui| {
                                ui.add(egui::Label::new(full).wrap());
                            });
                    }
                } else if ui.button("Read full changelog").clicked() {
                    state.load_full_patch_note(&version);
                }
            } else if state.patch_notes_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Looking up official changelog...");
                });
            } else {
                ui.label(
                    RichText::new("No changelog entry in Mojang's launcher feed for this version.")
                        .color(TEXT2),
                );
            }
            if !state.patch_notes_error.is_empty() {
                ui.label(RichText::new(&state.patch_notes_error).color(TEXT2));
            }
            ui.hyperlink_to(
                "Find official release notes",
                crate::minecraft::patch_notes::official_search_url(&version),
            );
        });
}

fn show_new_dialog(state: &mut AppState, ui: &mut egui::Ui) {
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if state.new_draft.step == 1 {
            state.new_draft.step = 0;
        } else {
            state.show_new_instance = false;
        }
    }

    let step = state.new_draft.step;
    page_header(
        ui,
        "New instance",
        if step == 0 {
            "Your world, your mods. Keep everything separate."
        } else {
            "Choose your mod loader and runtime."
        },
    );
    step_rail(ui, 2, step);
    ui.add_space(8.0);

    let max_body_h = (ui.ctx().screen_rect().height() - 240.0).clamp(160.0, 420.0);
    egui::ScrollArea::vertical()
        .max_height(max_body_h)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            if step == 0 {
                show_step_basic(state, ui);
            } else {
                show_step_loader(state, ui);
            }
        });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(8.0);

    if step == 0 {
        show_footer_basic(state, ui);
    } else {
        show_footer_loader(state, ui);
    }
}

fn show_step_basic(state: &mut AppState, ui: &mut egui::Ui) {
    if state.new_draft.versions.is_empty() {
        state.new_draft.versions = state.selected_version_list();
    }
    field_label(ui, "1  INSTANCE NAME");
    crate::ui::components::limited_text_edit(
        ui,
        "new-instance-name",
        &mut state.new_draft.name,
        crate::ui::components::limits::INSTANCE_NAME,
        "e.g. Performance",
    );
    ui.add_space(3.0);
    ui.label(
        RichText::new("This name appears in Home and Library.")
            .size(11.5)
            .color(TEXT2),
    );
    ui.add_space(10.0);

    field_label(ui, "2  MINECRAFT VERSION");
    let current = if state.new_draft.version.is_empty() {
        state
            .manifest
            .as_ref()
            .map(|m| m.latest.release.clone())
            .unwrap_or_default()
    } else {
        state.new_draft.version.clone()
    };
    if state.new_draft.version.is_empty() {
        state.new_draft.version = current.clone();
    }
    let query = state.new_draft.version_query.trim().to_ascii_lowercase();
    let filtered: Vec<String> = state
        .new_draft
        .versions
        .iter()
        .filter(|version| query.is_empty() || version.to_ascii_lowercase().contains(&query))
        .take(80)
        .cloned()
        .collect();

    ui.horizontal(|ui| {
        let filter_width = (ui.available_width() * 0.38).clamp(110.0, 180.0);
        ui.add(
            egui::TextEdit::singleline(&mut state.new_draft.version_query)
                .margin(egui::vec2(10.0, 8.0))
                .min_size(egui::vec2(0.0, 34.0))
                .hint_text("Filter...")
                .desired_width(filter_width),
        );
        egui::ComboBox::from_id_salt("new-mc")
            .selected_text(if current.is_empty() {
                "Select version...".to_string()
            } else {
                current.clone()
            })
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for v in filtered {
                    ui.selectable_value(&mut state.new_draft.version, v.clone(), v);
                }
            });
    });
    if !query.is_empty()
        && !state
            .new_draft
            .versions
            .iter()
            .any(|version| version.to_ascii_lowercase().contains(&query))
    {
        ui.label(
            RichText::new("No matching versions found.")
                .size(11.0)
                .color(TEXT2),
        );
    }
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        if ui
            .checkbox(
                &mut state.show_snapshots,
                RichText::new("Show snapshots and older versions")
                    .size(11.5)
                    .color(TEXT2),
            )
            .changed()
        {
            state.new_draft.versions = state.selected_version_list();
            state.save_config();
        }
    });

    if current != state.new_draft.version {
        state.new_draft.loader_version.clear();
        state.new_draft.loader_versions.clear();
        state.new_draft.loader_fetch_key.clear();
        state.new_draft.loading_loaders = false;
        state.new_draft.error.clear();
    }

    if state.manifest_loading {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new("Loading versions...").size(11.0).color(TEXT2));
        });
    }
    if !state.versions_error.is_empty() {
        ui.label(
            RichText::new(&state.versions_error)
                .size(11.0)
                .color(DANGER),
        );
    }

    ui.add_space(4.0);
    show_version_changelog(state, ui);
}

fn show_footer_basic(state: &mut AppState, ui: &mut egui::Ui) {
    let name_valid = !state.new_draft.name.trim().is_empty();
    let version_valid = !state.new_draft.version.is_empty()
        && state.new_draft.versions.contains(&state.new_draft.version);
    let can_proceed = name_valid && version_valid;
    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
    let mut do_advance = false;

    ui.horizontal(|ui| {
        if ui.button("Cancel").clicked() {
            state.show_new_instance = false;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_enabled_ui(can_proceed, |ui| {
                if primary_button(ui, "Next: Choose loader →").clicked() {
                    do_advance = true;
                }
            });
            if !can_proceed {
                let hint = if !name_valid {
                    "Enter a name first"
                } else {
                    "Choose a Minecraft version"
                };
                ui.label(RichText::new(hint).size(11.5).color(TEXT2));
            }
        });
    });

    if (enter_pressed || do_advance) && can_proceed {
        state.new_draft.step = 1;
    }
}

fn show_step_loader(state: &mut AppState, ui: &mut egui::Ui) {
    let p = crate::ui::theme::palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Minecraft").size(11.5).color(TEXT2));
                ui.label(
                    RichText::new(&state.new_draft.version)
                        .size(12.0)
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("Name").size(11.5).color(TEXT2));
                ui.label(
                    RichText::new(state.new_draft.name.trim())
                        .size(12.0)
                        .strong()
                        .color(TEXT),
                );
            });
        });
    ui.add_space(10.0);

    field_label(ui, "3  MOD LOADER");
    ui.label(
        RichText::new(
            "Vanilla installs Minecraft only. Choose a loader if this instance will use mods.",
        )
        .size(11.5)
        .color(TEXT2),
    );
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for l in LoaderKind::all() {
            let sel = state.new_draft.loader == l;
            if loader_pill(ui, sel, l.display_name()).clicked() && !sel {
                state.new_draft.loader = l;
                state.new_draft.loader_version.clear();
                state.new_draft.loader_versions.clear();
                state.new_draft.loader_fetch_key.clear();
                state.new_draft.loading_loaders = false;
                state.new_draft.error.clear();
            }
        }
    });

    if state.new_draft.loader != LoaderKind::Vanilla {
        ui.add_space(10.0);
        field_label(ui, "LOADER VERSION");
        ui.label(
            RichText::new(
                "The newest compatible loader is selected automatically. Change it only if needed.",
            )
            .size(11.5)
            .color(TEXT2),
        );
        ui.add_space(4.0);
        let key = format!(
            "{}|{}",
            state.new_draft.version,
            state.new_draft.loader.as_str()
        );
        if state.new_draft.loader_fetch_key != key && !state.new_draft.loading_loaders {
            state.new_draft.loader_versions.clear();
            state.new_draft.loader_version.clear();
        }
        let mut refresh = false;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !state.new_draft.loading_loaders,
                    egui::Button::new("Refresh"),
                )
                .clicked()
            {
                refresh = true;
            }
            if state.new_draft.loading_loaders {
                ui.spinner();
                ui.label(
                    RichText::new("Loading loader versions...")
                        .size(11.0)
                        .color(TEXT2),
                );
            } else if !state.new_draft.loader_versions.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "{} available",
                        state.new_draft.loader_versions.len()
                    ))
                    .size(11.0)
                    .color(TEXT2),
                );
            }
        });
        if refresh
            || (state.new_draft.loader_fetch_key != key
                && !state.new_draft.loading_loaders
                && !state.new_draft.version.is_empty())
        {
            state.new_draft.loading_loaders = true;
            state.new_draft.loader_fetch_key = key;
            state.new_draft.error.clear();
            crate::app::tasks::fetch_loader_versions(
                state,
                state.new_draft.loader,
                state.new_draft.version.clone(),
                refresh,
            );
        }
        if !state.new_draft.loading_loaders && !state.new_draft.loader_versions.is_empty() {
            egui::ComboBox::from_id_salt("new-loader-ver")
                .selected_text(if state.new_draft.loader_version.is_empty() {
                    "Newest compatible".to_string()
                } else {
                    state.new_draft.loader_version.clone()
                })
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for v in state.new_draft.loader_versions.clone() {
                        ui.selectable_value(&mut state.new_draft.loader_version, v.clone(), v);
                    }
                });
        }
    }

    if !state.new_draft.error.is_empty() {
        ui.add_space(6.0);
        ui.label(
            RichText::new(&state.new_draft.error)
                .size(11.0)
                .color(DANGER),
        );
        ui.label(
            RichText::new("Retry with Refresh, or choose another loader.")
                .size(11.0)
                .color(TEXT2),
        );
    }

    let loader_ready = state.new_draft.loader == LoaderKind::Vanilla
        || (!state.new_draft.loading_loaders
            && state
                .new_draft
                .loader_versions
                .contains(&state.new_draft.loader_version));
    if !loader_ready && !state.new_draft.loading_loaders && state.new_draft.error.is_empty() {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "No compatible loader found. Choose another Minecraft version or loader.",
            )
            .size(11.5)
            .color(TEXT2),
        );
    }

    ui.add_space(10.0);
    let version_label = if state.new_draft.version.is_empty() {
        "No version"
    } else {
        state.new_draft.version.as_str()
    };
    ui.label(
        RichText::new(format!(
            "Install: Minecraft {version_label} · {}{}",
            state.new_draft.loader.display_name(),
            if state.new_draft.loader == LoaderKind::Vanilla
                || state.new_draft.loader_version.is_empty()
            {
                "".to_string()
            } else {
                format!(" {}", state.new_draft.loader_version)
            }
        ))
        .size(11.5)
        .color(TEXT2),
    );
}

fn show_footer_loader(state: &mut AppState, ui: &mut egui::Ui) {
    let loader_ready = state.new_draft.loader == LoaderKind::Vanilla
        || (!state.new_draft.loading_loaders
            && state
                .new_draft
                .loader_versions
                .contains(&state.new_draft.loader_version));
    let ready = loader_ready
        && !state.new_draft.name.trim().is_empty()
        && state.new_draft.versions.contains(&state.new_draft.version);

    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
    let mut do_create = false;

    ui.horizontal(|ui| {
        if ui.button("← Back").clicked() {
            state.new_draft.step = 0;
        }
        if ui.button("Cancel").clicked() {
            state.show_new_instance = false;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_enabled_ui(ready, |ui| {
                if primary_button(ui, "Create and install").clicked() {
                    do_create = true;
                }
            });
            if !ready {
                let reason = if state.new_draft.name.trim().is_empty() {
                    "Enter a name first"
                } else if !state.new_draft.versions.contains(&state.new_draft.version) {
                    "Choose a Minecraft version"
                } else if state.new_draft.loading_loaders {
                    "Loading loader versions..."
                } else {
                    "Waiting for a compatible loader"
                };
                ui.label(RichText::new(reason).size(11.5).color(TEXT2));
            }
        });
    });

    if (enter_pressed || do_create) && ready {
        let name = state.new_draft.name.trim().to_string();
        if name.is_empty() {
            state.new_draft.error = "Give the instance a name.".to_string();
            return;
        }
        if state.new_draft.version.is_empty() {
            state.new_draft.error = "Pick a Minecraft version.".to_string();
            return;
        }
        let loader = state.new_draft.loader;
        let lv = state.new_draft.loader_version.clone();
        state.show_new_instance = false;
        state.global_status = format!("Installing {name}...");
        crate::app::tasks::create_and_install(
            state,
            name,
            state.new_draft.version.clone(),
            loader,
            lv,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::config::InstanceConfig;
    use crate::storage::paths::MonoryxPaths;

    #[test]
    fn instances_page_renders_with_active_instance() {
        let dir = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(dir.path().to_path_buf());
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths);

        let inst = InstanceConfig::new(
            "THE BEST LAUNCHERRR".to_string(),
            "1.21.1".to_string(),
            LoaderKind::Fabric,
            "0.16.0".to_string(),
        );
        state.selected_instance = Some(inst.id.clone());
        state.instance_list = vec![inst];

        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 700.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(&mut state, ctx, ui);
                });
            },
        );
    }

    #[test]
    fn new_instance_dialog_step_advancement_and_validation() {
        let dir = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(dir.path().to_path_buf());
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths);

        open_new_dialog(&mut state);
        assert!(state.show_new_instance);
        assert_eq!(state.new_draft.step, 0);

        state.new_draft.versions = vec!["1.21.1".to_string(), "1.20.1".to_string()];
        state.new_draft.version = "1.21.1".to_string();

        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 700.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(&mut state, ctx, ui);
                });
            },
        );

        state.new_draft.name = "Test Instance".to_string();
        state.new_draft.step = 1;

        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 700.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(&mut state, ctx, ui);
                });
            },
        );

        assert_eq!(state.new_draft.step, 1);
    }
}
