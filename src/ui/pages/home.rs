use crate::app::{events::Page, state::AppState};
use crate::instance::InstanceConfig;
use crate::ui::components::{
    action_button, badge, card_frame, format_last_played, hero_card_frame, page_header,
    play_hero_button, primary_button, stat, thin_progress,
};
use crate::ui::theme::{MUTED, TEXT, TEXT2};
use egui::RichText;

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Home", "Pick an instance and jump back in.");
    if !state.last_exit.is_empty() {
        card_frame(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("The last game session ended unexpectedly.")
                        .color(crate::ui::theme::WARNING),
                );
                if action_button(ui, "View logs").clicked() {
                    state.set_page(Page::Logs);
                }
                if action_button(ui, "Dismiss").clicked() {
                    state.last_exit.clear();
                }
            });
        });
        ui.add_space(10.0);
    }
    let Some(cfg) = state.selected() else {
        hero_card_frame(ui, |ui| {
            ui.label(
                RichText::new("Let's set up your first game")
                    .size(22.0)
                    .color(TEXT),
            );
            ui.label(
                RichText::new(
                    "An instance keeps a Minecraft version, its worlds, and its mods together.",
                )
                .color(TEXT2),
            );
            ui.add_space(12.0);
            if primary_button(ui, "Create an instance").clicked() {
                crate::ui::pages::instances::open_new_dialog(state);
                state.set_page(Page::Instances);
            }
        });
        return;
    };
    if ui.available_width() >= 800.0 {
        ui.columns(2, |columns| {
            selected_instance(state, &cfg, &mut columns[0]);
            screenshots(state, &cfg, &mut columns[1]);
        });
    } else {
        selected_instance(state, &cfg, ui);
        ui.add_space(12.0);
        screenshots(state, &cfg, ui);
    }
    ui.add_space(18.0);
    instance_list(state, ui);
}

fn selected_instance(state: &mut AppState, cfg: &InstanceConfig, ui: &mut egui::Ui) {
    let running = state.playing.get(&cfg.id).copied().unwrap_or(false);
    let installing = state.busy_install.contains_key(&cfg.id);
    let eco = cfg.boost_mode.unwrap_or(state.config.boost_mode);
    hero_card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            crate::ui::components::render_instance_thumbnail(
                ui,
                44.0,
                &cfg.name,
                cfg.loader.display_name(),
                false,
            );
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("SELECTED INSTANCE").size(10.5).color(MUTED));
                ui.add(
                    egui::Label::new(RichText::new(&cfg.name).size(24.0).strong().color(TEXT))
                        .wrap(),
                );
            });
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            badge(ui, &format!("Minecraft {}", cfg.minecraft_version));
            badge(ui, cfg.loader.display_name());
            if eco {
                badge(ui, "Eco mode");
            }
        });
        ui.add_space(12.0);
        let default_ram = ui.ctx().data_mut(|data| {
            *data.get_temp_mut_or_insert_with(egui::Id::new("home-default-memory"), || {
                (
                    crate::utils::system::default_max_memory_mb(),
                    crate::utils::system::default_boost_max_memory_mb(),
                )
            })
        });
        let ram = crate::utils::system::game_memory_limit_mb(
            cfg.memory_max_mb,
            eco,
            default_ram.0,
            default_ram.1,
        );
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 24.0;
            stat(
                ui,
                "Mods",
                &state
                    .mod_counts
                    .get(&cfg.id)
                    .copied()
                    .unwrap_or(0)
                    .to_string(),
            );
            stat(ui, "Memory", &format!("{:.1} GB", ram as f64 / 1024.0));
            stat(
                ui,
                "Last played",
                &format_last_played(cfg.last_played_at.as_deref()),
            );
        });
        if crate::utils::system::eco_memory_limited(cfg.memory_max_mb, eco, default_ram.1) {
            ui.label(
                RichText::new(format!(
                    "Eco mode is capping this launch to {:.1} GB, below the {} set on the \
                     instance.",
                    ram as f64 / 1024.0,
                    cfg.memory_max_mb as f64 / 1024.0
                ))
                .size(11.0)
                .color(crate::ui::theme::MUTED),
            );
        }
        ui.add_space(14.0);
        ui.add_enabled_ui(!running && !installing, |ui| {
            if play_hero_button(
                ui,
                if running {
                    "Game is running"
                } else if installing {
                    "Getting things ready…"
                } else {
                    "Play Minecraft"
                },
            )
            .clicked()
            {
                crate::app::tasks::play_instance(state, cfg.id.clone());
            }
        });
        if let Some((_, completed, total)) = state.busy_install.get(&cfg.id) {
            thin_progress(ui, Some(*completed as f32 / (*total).max(1) as f32));
            ui.label(
                RichText::new(format!("Preparing game files · {completed}/{total}"))
                    .size(12.0)
                    .color(TEXT2),
            );
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if action_button(ui, "Mods & packs").clicked() { state.set_page(Page::Library); }
            if action_button(ui, "Worlds & backups").clicked() { state.set_page(Page::Worlds); }
            ui.menu_button("Instance tools", |ui| {
                if action_button(ui, "Edit instance").clicked() { state.edit_instance = Some(cfg.clone()); ui.close(); }
                if action_button(ui, "Open game folder").clicked() { let _ = open::that(state.instances.game_dir(&cfg.id)); ui.close(); }
                if action_button(ui, "View logs").clicked() { state.set_page(Page::Logs); ui.close(); }
                ui.separator();
                ui.add_enabled_ui(!running && !installing, |ui| {
                    if action_button(ui, "Repair game files").on_hover_text("Check and re-download missing or damaged game files.").clicked() {
                        crate::app::tasks::repair_instance(state, cfg.id.clone()); ui.close();
                    }
                });
                if action_button(ui, if eco { "Turn off Eco mode" } else { "Turn on Eco mode" }).on_hover_text("Eco mode uses less memory and may lower FPS in demanding worlds. Your custom memory limit is preserved.").clicked() { state.toggle_boost(); ui.close(); }
            });
        });
    });
}

fn instance_list(state: &mut AppState, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("Your instances")
                .size(17.0)
                .strong()
                .color(TEXT),
        );
        ui.label(RichText::new(state.instance_list.len().to_string()).color(MUTED));
        if action_button(ui, "Manage instances").clicked() {
            state.set_page(Page::Instances);
        }
    });
    ui.add_space(6.0);
    let instances = state.instance_list.clone();
    let count = if ui.available_width() >= 1150.0 {
        3
    } else if ui.available_width() >= 800.0 {
        2
    } else {
        1
    };
    egui::ScrollArea::vertical()
        .id_salt("home-instance-list")
        .max_height(260.0)
        .show(ui, |ui| {
            for chunk in instances.chunks(count) {
                ui.columns(count, |columns| {
                    for (instance, ui) in chunk.iter().zip(columns.iter_mut()) {
                        let selected = state.selected_instance.as_deref() == Some(&instance.id);
                        if crate::ui::components::instance_choice(
                            ui,
                            &instance.id,
                            &instance.name,
                            &format!(
                                "Minecraft {} · {}",
                                instance.minecraft_version,
                                instance.loader.display_name()
                            ),
                            selected,
                        )
                        .clicked()
                            && !selected
                        {
                            state.selected_instance = Some(instance.id.clone());
                            state.save_config();
                            state.refresh_library();
                        }
                    }
                });
            }
        });
}

fn screenshots(state: &mut AppState, cfg: &InstanceConfig, ui: &mut egui::Ui) {
    card_frame(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Latest screenshot")
                    .size(17.0)
                    .strong()
                    .color(TEXT),
            );
            if action_button(ui, "Open gallery").clicked() {
                state.set_page(Page::Screenshots);
            }
        });
        if let Some(latest) = state
            .screenshots
            .iter()
            .find(|shot| shot.instance_id == cfg.id)
            .cloned()
        {
            state.ensure_screenshot_thumb(&latest.path);
            if let Some(texture) = state.screenshot_thumbnails.get(&latest.path) {
                let native = texture.size_vec2();
                let texture_id = texture.id();
                let scale = (ui.available_width() / native.x).min(220.0 / native.y);
                ui.vertical_centered(|ui| {
                    if ui
                        .add(egui::ImageButton::new((texture_id, native * scale)).frame(false))
                        .on_hover_text("Open full image")
                        .clicked()
                    {
                        state.open_screenshot(&latest.path);
                    }
                });
            } else {
                ui.spinner();
            }
            let date: chrono::DateTime<chrono::Local> = latest.modified.into();
            ui.label(
                RichText::new(format!(
                    "{} · {}",
                    cfg.name,
                    date.format("%b %d at %I:%M %p")
                ))
                .size(12.0)
                .color(TEXT2),
            );
        } else {
            ui.add_space(28.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("Keep a moment from your world")
                        .size(18.0)
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("Press F2 while playing to take a screenshot.").color(TEXT2),
                );
                ui.label(
                    RichText::new("You'll find it here when you come back.")
                        .size(12.0)
                        .color(MUTED),
                );
            });
            ui.add_space(40.0);
        }
    });
}
