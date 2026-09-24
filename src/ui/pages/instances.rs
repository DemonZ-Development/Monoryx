use crate::app::state::AppState;
use crate::instance::config::LoaderKind;
use crate::ui::components::{
    badge, card_frame, empty_state, field_label, hover_card_frame, page_header, primary_button,
};
use crate::ui::theme::{BORDER, DANGER, ELEVATED2, SELECTED, SELECTED_FG, TEXT, TEXT2};
use egui::{CornerRadius, RichText, Stroke};

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Instances",
        "Isolated game directories. Nothing is shared.",
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
                                .map(|config| config.id)
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
                        badge(ui, &format!("{} mods", state.instances.mod_count(&inst.id)));
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
                        .fill(ELEVATED2)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(CornerRadius::same(8));
                        ui.add_enabled(false, btn);
                    } else if ui
                        .add(
                            egui::Button::new(RichText::new("Play").strong().color(SELECTED_FG))
                                .fill(crate::ui::theme::ACCENT)
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
            ui.horizontal(|ui| {
                if ui.small_button("Select").clicked() {
                    state.selected_instance = Some(inst.id.clone());
                    state.save_config();
                    state.refresh_library();
                }
                if ui.small_button("Edit").clicked() {
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
                    if ui.button(RichText::new("Delete").color(DANGER)).clicked() {
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
            .backdrop_color(egui::Color32::from_black_alpha(180))
            .frame(
                egui::Frame::popup(&_ctx.style())
                    .inner_margin(24)
                    .corner_radius(16),
            )
            .show(_ctx, |ui| {
                ui.set_width((_ctx.screen_rect().width() - 80.0).clamp(280.0, 480.0));
                egui::ScrollArea::vertical()
                    .max_height((_ctx.screen_rect().height() - 100.0).max(200.0))
                    .show(ui, |ui| show_new_dialog(state, ui));
            });
        if response.should_close() {
            state.show_new_instance = false;
        }
    }
    if let Some(id) = state.confirm_delete.clone() {
        let title = state.confirm_title.clone();
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(_ctx, |ui| {
                ui.label("This removes the instance and its game files. This cannot be undone.");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.confirm_delete = None;
                    }
                    if ui.button(RichText::new("Delete").color(DANGER)).clicked() {
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
    state.new_draft.loader = LoaderKind::Fabric;
    state.new_draft.dialog_seq = next_dialog_seq();
    state.show_new_instance = true;
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
    let fill = ELEVATED2.lerp_to_gamma(SELECTED, fade);
    let text = TEXT.lerp_to_gamma(SELECTED_FG, fade);
    ui.add(
        egui::Button::new(RichText::new(label).size(12.5).strong().color(text))
            .fill(fill)
            .stroke(Stroke::new(1.0_f32, BORDER))
            .corner_radius(CornerRadius::same(16)),
    )
}

fn show_new_dialog(state: &mut AppState, ui: &mut egui::Ui) {
    let enter = ui.ctx().animate_bool_with_time(
        egui::Id::new(("new-instance-dialog", state.new_draft.dialog_seq)),
        true,
        0.18,
    );
    ui.add_space((1.0 - enter) * 10.0);
    page_header(
        ui,
        "New instance",
        "Your world, your mods. Keep everything separate.",
    );
    if state.new_draft.versions.is_empty() {
        state.new_draft.versions = state.selected_version_list();
    }
    field_label(ui, "Name");
    ui.add(
        egui::TextEdit::singleline(&mut state.new_draft.name)
            .hint_text("e.g. Performance")
            .desired_width(f32::INFINITY),
    );
    ui.add_space(10.0);
    field_label(ui, "Minecraft Version");
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
    egui::ComboBox::from_id_salt("new-mc")
        .selected_text(if current.is_empty() {
            "Loading...".to_string()
        } else {
            current.clone()
        })
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for v in state.new_draft.versions.clone() {
                ui.selectable_value(&mut state.new_draft.version, v.clone(), v);
            }
        });
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
    if current != state.new_draft.version {
        state.new_draft.loader_version.clear();
        state.new_draft.loader_versions.clear();
        state.new_draft.loader_fetch_key.clear();
        state.new_draft.loading_loaders = false;
        state.new_draft.error.clear();
    }
    ui.add_space(10.0);
    field_label(ui, "Loader");
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
        field_label(ui, "Loader Version");
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
                ui.label(RichText::new("Loading...").size(11.0).color(TEXT2));
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
                    "(newest stable)".to_string()
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
            RichText::new("Retry with Refresh, or choose another Minecraft version or loader.")
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
        ui.label(
            RichText::new(
                "No compatible loader found. Choose another Minecraft version or loader.",
            )
            .color(TEXT2),
        );
    }
    ui.add_space(12.0);
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("Cancel").clicked() {
            state.show_new_instance = false;
        }
        let ready = loader_ready
            && !state.new_draft.name.trim().is_empty()
            && state.new_draft.versions.contains(&state.new_draft.version);
        ui.add_enabled_ui(ready, |ui| {
            if primary_button(ui, "Create & install").clicked() {
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
        });
    });
}
