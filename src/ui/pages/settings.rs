use crate::account::offline::OfflineProfile;
use crate::app::state::AppState;
use crate::config::{CloseAction, GpuPreference};
use crate::ui::components::{card_frame, field_label, page_header};
use crate::ui::theme::{TEXT, TEXT2};
use egui::RichText;

pub fn show(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Settings",
        "Launcher, Minecraft, Java and download preferences.",
    );
    egui::ScrollArea::vertical().show(ui, |ui| {
        card_frame(ui, |ui| {
            ui.label(RichText::new("Offline account").strong().color(TEXT));
            let username = state.config.profile.as_ref().map(|p| p.username.as_str()).unwrap_or_default();
            let draft_id = egui::Id::new(("settings-offline-username", username));
            let mut draft = ctx.data_mut(|data| {
                data.get_temp::<String>(draft_id).unwrap_or_else(|| username.to_string())
            });
            let playing = state.playing.values().any(|playing| *playing);
            ui.label(RichText::new("Changing your username changes your offline UUID and player identity. Existing worlds and servers may treat you as a different player, with a separate inventory and progress.").color(TEXT2));
            if playing {
                ui.label(RichText::new("Exit all running games before editing your account.").color(TEXT2));
            }
            ui.add_enabled_ui(!playing, |ui| {
                field_label(ui, "Username (3-16 letters, numbers or underscores)");
                ui.text_edit_singleline(&mut draft);
                if ui.button("Save account").clicked() {
                    match save_offline_profile(&mut state.config, &state.paths.config_file(), &draft) {
                        Ok(()) => {
                            draft = state.config.profile.as_ref().unwrap().username.clone();
                            state.notify("Offline account saved");
                        }
                        Err(e) => state.fail(e.user_message()),
                    }
                }
            });
            ctx.data_mut(|data| data.insert_temp(draft_id, draft));
        });
        ui.add_space(6.0);
        card_frame(ui, |ui| {
            ui.label(RichText::new("General").strong().color(TEXT));
            ui.horizontal(|ui| {
                ui.label(RichText::new("When game starts:").color(TEXT2));
                egui::ComboBox::from_id_salt("close-action")
                    .selected_text(state.config.close_action.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.config.close_action, CloseAction::Never, "Never");
                        ui.selectable_value(&mut state.config.close_action, CloseAction::Minimize, "Minimize");
                        ui.selectable_value(&mut state.config.close_action, CloseAction::Hide, "Hide and restore after game exits");
                    });
            });
            ui.label(RichText::new("Hide keeps the launcher supervisor running in the background and restores the window after the game exits.").color(TEXT2));
            ui.checkbox(&mut state.config.remember_instance, "Remember selected instance");
            ui.checkbox(&mut state.show_snapshots, "Show snapshots and old versions");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Parallel downloads:").color(TEXT2));
                let mut v = state.config.parallel_downloads.to_string();
                if ui.text_edit_singleline(&mut v).changed() {
                    if let Ok(n) = v.parse::<usize>() {
                        state.config.parallel_downloads = n.clamp(1, 16);
                    }
                }
            });
            if ui.button("Save").clicked() {
                save_settings(state, "Settings saved".to_string());
            }
        });
        ui.add_space(6.0);
        card_frame(ui, |ui| {
            ui.label(RichText::new("Minecraft defaults").strong().color(TEXT));
            field_label(ui, "Default min memory (MB)");
            ui.text_edit_singleline(&mut state.settings_mem_min);
            field_label(ui, "Default max memory (MB)");
            ui.text_edit_singleline(&mut state.settings_mem_max);
            field_label(ui, "Default JVM arguments");
            ui.text_edit_singleline(&mut state.settings_jvm);
            field_label(ui, "Default game arguments");
            ui.text_edit_singleline(&mut state.settings_game_args);
            if ui.button("Save Minecraft defaults").clicked() {
                match (state.settings_mem_min.trim().parse::<u64>(), state.settings_mem_max.trim().parse::<u64>()) {
                    (Ok(min), Ok(max)) => match crate::utils::system::validate_memory(min, max) {
                        Ok(warn) => {
                            state.config.memory.min_mb = min;
                            state.config.memory.max_mb = max;
                            state.config.default_jvm_args = state.settings_jvm.clone();
                            state.config.default_game_args = state.settings_game_args.clone();
                            save_settings(state, warn.unwrap_or_else(|| "Minecraft defaults saved".to_string()));
                        }
                        Err(e) => state.fail(e),
                    },
                    _ => state.fail("Memory values must be numbers."),
                }
            }
        });
        ui.add_space(6.0);
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Java").strong().color(TEXT));
                if ui.small_button("Refresh").clicked() {
                    state.refresh_java();
                }
            });
            if state.java_loading {
                ui.label("Detecting Java...");
            }
            if state.java_list.is_empty() && !state.java_loading {
                ui.label(RichText::new("No Java detected. Automatic mode downloads a compatible managed runtime when you launch a game.").color(TEXT2));
            }
            for j in state.java_list.clone() {
                ui.label(RichText::new(format!("Java {} - {}", j.major, j.path.display())).size(12.0).color(TEXT));
                ui.label(RichText::new(format!("{} - {}", j.source, j.version_string)).size(11.0).color(TEXT2));
            }
            ui.add_space(4.0);
            field_label(ui, "Mode");
            egui::ComboBox::from_id_salt("java-mode")
                .selected_text(match state.config.java.mode.as_str() {
                    "system" => "System",
                    "custom" => "Custom",
                    _ => "Automatic",
                })
                .show_ui(ui, |ui| {
                    for (value, label) in [("automatic", "Automatic"), ("system", "System"), ("custom", "Custom")] {
                        ui.selectable_value(&mut state.config.java.mode, value.to_string(), label);
                    }
                });
            ui.label(RichText::new("Automatic selects a compatible Java runtime and downloads a managed Temurin runtime if needed. System uses a detected system installation; Custom uses the path below. System and Custom do not download Java automatically.").color(TEXT2));
            field_label(ui, "Custom Java path");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut state.config.java.custom_path);
                if ui.small_button("Browse").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_file() {
                        state.config.java.custom_path = p.display().to_string();
                    }
                }
            });
            gpu_preference_selector(ui, &mut state.config.gpu_preference);
            gpu_status(ui, ctx, state);
            if ui.button("Save Java settings").clicked() {
                save_settings(state, "Java settings saved".to_string());
            }
        });
        ui.add_space(6.0);
        card_frame(ui, |ui| {
            ui.label(RichText::new("Appearance").strong().color(TEXT));
            ui.label(RichText::new("Dark monochrome is the MONORYX identity.").color(TEXT2));
        });
        ui.add_space(6.0);
        card_frame(ui, |ui| {
            ui.label(RichText::new("About").strong().color(TEXT));
            ui.label(RichText::new(format!("MONORYX {}", env!("CARGO_PKG_VERSION"))).color(TEXT));
            ui.label(RichText::new("Native Rust launcher. Apache-2.0. By DemonZDevelopment.").size(12.0).color(TEXT2));
            ui.label(RichText::new("MONORYX is an independent project and is not affiliated with Mojang Studios or Microsoft.").size(11.0).color(TEXT2));
            ui.horizontal(|ui| {
                if ui.small_button("Data folder").clicked() {
                    let _ = open::that(state.paths.root());
                }
                if ui.small_button("Logs folder").clicked() {
                    let _ = open::that(state.paths.logs_dir());
                }
                if ui.small_button("Check for Updates").clicked() {
                    let _ = open::that("https://github.com/DemonZDevelopment/monoryx/releases");
                }
            });
            ui.label(RichText::new(format!("Data: {}", state.paths.root().display())).size(11.0).color(TEXT2));
        });
    });
}

pub(super) fn gpu_preference_selector(ui: &mut egui::Ui, preference: &mut GpuPreference) {
    field_label(ui, "GPU preference");
    ui.add_enabled_ui(cfg!(target_os = "windows"), |ui| {
        egui::ComboBox::from_id_salt("gpu-preference")
            .selected_text(preference.as_str())
            .show_ui(ui, |ui| {
                for value in [
                    GpuPreference::System,
                    GpuPreference::HighPerformance,
                    GpuPreference::PowerSaving,
                ] {
                    ui.selectable_value(preference, value, value.as_str());
                }
            });
    });
    ui.label(RichText::new("Windows only. Applies on the next launch to the selected Java executable, including other apps using it. Hardware and drivers determine the GPU; your choice is not guaranteed. System restores OS selection only when a preference already exists.").color(TEXT2));
}

fn gpu_status(ui: &mut egui::Ui, ctx: &egui::Context, state: &mut AppState) {
    if !cfg!(target_os = "windows") {
        return;
    }
    let auto_id = egui::Id::new("gpu-autodetect-once");
    if !ctx.data_mut(|data| data.get_temp::<bool>(auto_id).unwrap_or(false)) {
        ctx.data_mut(|data| data.insert_temp(auto_id, true));
        state.refresh_gpus();
    }
    ui.horizontal(|ui| {
        field_label(ui, "Detected GPUs");
        if ui.small_button("Refresh").clicked() {
            state.refresh_gpus();
        }
    });
    if state.gpu_loading {
        ui.label(RichText::new("Detecting GPUs...").color(TEXT2));
    } else if state.gpu_list.is_empty() {
        ui.label(RichText::new("No GPUs detected yet.").color(TEXT2));
    }
    for g in state.gpu_list.clone() {
        let tag = if g.dedicated {
            "Dedicated"
        } else {
            "Integrated"
        };
        ui.label(
            RichText::new(format!("{} ({tag})", g.name))
                .size(12.0)
                .color(TEXT),
        );
    }
    if state.gpu_list.iter().any(|g| g.dedicated)
        && state.config.gpu_preference == GpuPreference::System
    {
        ui.label(
            RichText::new(
                "Dedicated GPU found. High performance asks Windows to run Minecraft on it.",
            )
            .color(TEXT2),
        );
        if ui.small_button("Use High performance").clicked() {
            state.config.gpu_preference = GpuPreference::HighPerformance;
            save_settings(state, "GPU preference set to High performance".to_string());
        }
    }
}

fn save_settings(state: &mut AppState, message: String) {
    state.config.last_page = state.page.as_str().to_string();
    state.config.selected_instance = state.selected_instance.clone();
    state.config.show_snapshots = state.show_snapshots;
    match state.config.save(&state.paths.config_file()) {
        Ok(()) => state.notify(message),
        Err(e) => state.fail(e.user_message()),
    }
}

fn save_offline_profile(
    config: &mut crate::config::LauncherConfig,
    path: &std::path::Path,
    username: &str,
) -> crate::error::Result<()> {
    let mut profile = OfflineProfile::new(username)?;
    if let Some(previous) = &config.profile {
        profile.created_at = previous.created_at.clone();
    }
    let mut updated = config.clone();
    updated.profile = Some(profile);
    updated.save(path)?;
    *config = updated;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LauncherConfig;

    #[test]
    fn account_rename_preserves_creation_and_persists_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let previous = OfflineProfile::new("Steve").unwrap();
        let mut config = LauncherConfig {
            profile: Some(previous.clone()),
            ..Default::default()
        };
        save_offline_profile(&mut config, &path, " Alex ").unwrap();
        let profile = config.profile.as_ref().unwrap();
        assert_eq!(profile.username, "Alex");
        assert_eq!(profile.created_at, previous.created_at);
        assert_ne!(profile.uuid, previous.uuid);
        assert_eq!(profile.uuid, OfflineProfile::new("Alex").unwrap().uuid);
        assert_eq!(LauncherConfig::load(&path).unwrap().profile, config.profile);
    }

    #[test]
    fn invalid_account_leaves_memory_and_disk_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let previous = OfflineProfile::new("Steve").unwrap();
        let mut config = LauncherConfig {
            profile: Some(previous.clone()),
            ..Default::default()
        };
        config.save(&path).unwrap();
        assert!(save_offline_profile(&mut config, &path, "bad name!").is_err());
        assert_eq!(config.profile, Some(previous));
        assert_eq!(LauncherConfig::load(&path).unwrap().profile, config.profile);
    }

    #[test]
    fn failed_account_save_preserves_original_profile() {
        let dir = tempfile::tempdir().unwrap();
        let previous = OfflineProfile::new("Steve").unwrap();
        let mut config = LauncherConfig {
            profile: Some(previous.clone()),
            ..Default::default()
        };
        assert!(save_offline_profile(&mut config, dir.path(), "Alex").is_err());
        assert_eq!(config.profile, Some(previous));
    }
}
