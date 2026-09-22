use crate::app::state::AppState;
use crate::ui::components::{
    action_button, badge, badge_ok, card_frame, empty_state, format_last_played,
    hero_card_frame, page_header, play_hero_button, primary_button, stat, thin_progress,
};
use crate::ui::theme::{
    ACCENT, BOOST, BOOST_BG, BORDER, DANGER, ELEVATED, ELEVATED2, HOVER, MUTED, OK, TEXT, TEXT2, WARNING,
};
use egui::{Color32, CornerRadius, RichText, Stroke};

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Home", "Play. Modify. Nothing else.");

    if !state.last_exit.is_empty() {
        egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(240, 98, 98, 25))
            .stroke(Stroke::new(1.0_f32, DANGER))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&state.last_exit).strong().color(DANGER));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Clear").clicked() {
                            state.last_exit.clear();
                        }
                        if ui.small_button("View Log").clicked() {
                            state.set_page(crate::app::events::Page::Logs);
                        }
                    });
                });
            });
        ui.add_space(8.0);
    }

    let Some(cfg) = state.selected() else {
        card_frame(ui, |ui| {
            empty_state(
                ui,
                "No instances created yet",
                "Create your first isolated Minecraft instance to start playing.",
            );
            ui.vertical_centered(|ui| {
                if primary_button(ui, "+ Create New Instance").clicked() {
                    crate::ui::pages::instances::open_new_dialog(state);
                    state.set_page(crate::app::events::Page::Instances);
                }
            });
            ui.add_space(14.0);
        });
        return;
    };

    let running = state.playing.get(&cfg.id).copied().unwrap_or(false);
    let installing = state.busy_install.contains_key(&cfg.id);
    let boost_on = cfg.boost_mode.unwrap_or(state.config.boost_mode);

    hero_card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&cfg.name).size(22.0).strong().color(TEXT));
                    if running {
                        badge_ok(ui, "RUNNING");
                    }
                    let boost_text = if boost_on { "Eco Mode: ON" } else { "Eco Mode: OFF" };
                    let boost_btn = egui::Button::new(
                        RichText::new(boost_text)
                            .size(11.5)
                            .strong()
                            .color(if boost_on { BOOST } else { MUTED }),
                    )
                    .fill(if boost_on { BOOST_BG } else { ELEVATED2 })
                    .stroke(Stroke::new(1.0_f32, if boost_on { BOOST } else { BORDER }))
                    .corner_radius(CornerRadius::same(6));
                    if ui
                        .add(boost_btn)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(if boost_on {
                            "Eco Mode is ACTIVE (Low RAM & Optimized GC). Click to turn OFF."
                        } else {
                            "Eco Mode is OFF. Click to turn ON."
                        })
                        .clicked()
                    {
                        state.toggle_boost();
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Minecraft {}", cfg.minecraft_version))
                            .size(12.5)
                            .color(TEXT2),
                    );
                    badge(ui, cfg.loader.display_name());
                    if !cfg.loader_version.is_empty() {
                        badge(ui, &cfg.loader_version);
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if running {
                    let btn = egui::Button::new(
                        RichText::new("GAME RUNNING")
                            .strong()
                            .size(13.5)
                            .color(OK),
                    )
                    .fill(ELEVATED2)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(CornerRadius::same(8));
                    ui.add_sized(egui::vec2(150.0, 42.0), btn);
                } else if installing {
                    let btn = egui::Button::new(
                        RichText::new("INSTALLING...")
                            .strong()
                            .size(13.5)
                            .color(WARNING),
                    )
                    .fill(ELEVATED2)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(CornerRadius::same(8));
                    ui.add_sized(egui::vec2(150.0, 42.0), btn);
                } else if play_hero_button(ui, "PLAY").clicked() {
                    crate::app::tasks::play_instance(state, cfg.id.clone());
                }
            });
        });

        ui.add_space(14.0);

        let effective_ram = if boost_on && cfg.memory_max_mb == crate::utils::system::default_max_memory_mb() {
            crate::utils::system::default_boost_max_memory_mb()
        } else {
            cfg.memory_max_mb
        };
        let ram_label = if boost_on {
            format!("{effective_ram} MB (Eco)")
        } else {
            format!("{} MB", cfg.memory_max_mb)
        };
        let last_played_label = format_last_played(cfg.last_played_at.as_deref());
        let total_launches = cfg.total_plays.to_string();

        ui.horizontal(|ui| {
            stat(ui, "Mods", &state.instances.mod_count(&cfg.id).to_string());
            ui.add_space(28.0);
            stat(ui, "Memory", &ram_label);
            ui.add_space(28.0);
            stat(ui, "Last played", &last_played_label);
            if cfg.total_plays > 0 {
                ui.add_space(28.0);
                stat(ui, "Total Launches", &total_launches);
            }
        });

        if let Some((_, a, b)) = state.busy_install.get(&cfg.id) {
            ui.add_space(10.0);
            thin_progress(ui, Some(*a as f32 / (*b).max(1) as f32));
            ui.label(
                RichText::new(format!("Downloading & verifying assets... {a}/{b}"))
                    .size(11.5)
                    .color(TEXT2),
            );
        }
    });

    ui.add_space(12.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
        if action_button(ui, "Open folder").clicked() {
            let p = state.instances.game_dir(&cfg.id);
            let _ = open::that(p);
        }
        if action_button(ui, "Edit instance").clicked() {
            state.edit_instance = Some(cfg.clone());
        }
        if action_button(ui, "Manage mods").clicked() {
            state.set_page(crate::app::events::Page::Library);
        }
        if action_button(ui, "View logs").clicked() {
            state.set_page(crate::app::events::Page::Logs);
        }
        if action_button(ui, "Repair").clicked() {
            state.global_status = "Repairing instance...".to_string();
            crate::app::tasks::repair_instance(state, cfg.id.clone());
        }
    });

    if state.instance_list.len() > 1 {
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Your Instances").size(15.0).strong().color(TEXT));
            ui.label(
                RichText::new(format!("({} total)", state.instance_list.len()))
                    .size(12.0)
                    .color(MUTED),
            );
        });
        ui.add_space(6.0);

        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for inst in state.instance_list.clone() {
                        let is_selected = Some(inst.id.clone()) == state.selected_instance;
                        let card_id = ui.make_persistent_id(format!("home_inst_card_{}", inst.id));
                        let hovered = ui.ctx().data(|d| d.get_temp::<bool>(card_id).unwrap_or(false));
                        let fade = ui.ctx().animate_bool_with_time(card_id.with("hover"), hovered, 0.15);
                        if fade > 0.001 && fade < 0.999 {
                            ui.ctx().request_repaint();
                        }
                        let base_fill = if is_selected { ELEVATED2 } else { ELEVATED };
                        let card_fill = base_fill.lerp_to_gamma(HOVER, fade);
                        let card_stroke = if is_selected {
                            Stroke::new(1.5_f32, ACCENT)
                        } else {
                            Stroke::new(1.0_f32, BORDER.lerp_to_gamma(Color32::from_rgb(52, 55, 65), fade))
                        };

                        let frame_resp = egui::Frame::new()
                            .fill(card_fill)
                            .stroke(card_stroke)
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(14, 12))
                            .show(ui, |ui| {
                                ui.set_min_width(170.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&inst.name)
                                            .size(14.0)
                                            .strong()
                                            .color(if is_selected { ACCENT } else { TEXT }),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "MC {} - {}",
                                            inst.minecraft_version,
                                            inst.loader.display_name()
                                        ))
                                        .size(11.0)
                                        .color(TEXT2),
                                    );
                                    ui.add_space(6.0);
                                    if !is_selected {
                                        ui.label(RichText::new("Click to switch").size(11.0).color(MUTED));
                                    } else {
                                        ui.label(RichText::new("Active").size(11.0).strong().color(ACCENT));
                                    }
                                });
                            });

                        let resp = ui.interact(frame_resp.response.rect, card_id, egui::Sense::click());
                        ui.ctx().data_mut(|d| d.insert_temp(card_id, resp.hovered()));
                        if resp.hovered() && !is_selected {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() && !is_selected {
                            state.selected_instance = Some(inst.id.clone());
                            state.save_config();
                            state.refresh_library();
                        }
                    }
                });
            });
    }
}
