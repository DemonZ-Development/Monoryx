use crate::account::offline::OfflineProfile;
use crate::app::state::AppState;
use crate::ui::components::{badge_accent, badge_ok, card_frame, field_label, page_header};
use crate::ui::theme::{DANGER, MUTED, TEXT, TEXT2, WARNING};
use egui::{CornerRadius, RichText, Stroke};

pub fn show(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Accounts",
        "Manage player identities, offline profiles, and Microsoft account.",
    );

    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Active Player Identity")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );
            if state.config.active_account().is_some() {
                badge_ok(ui, "Ready to Play");
            } else {
                badge_accent(ui, "No Profile Set");
            }
        });

        ui.add_space(6.0);

        if let Some(account) = state.config.active_account() {
            ui.horizontal(|ui| {
                let (avatar_rect, _) =
                    ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::hover());
                if !account.is_offline() {
                    crate::ui::components::draw_avatar(
                        ui.painter(),
                        avatar_rect,
                        account.username(),
                        &account.uuid().to_string(),
                    );
                } else {
                    crate::ui::components::draw_cute_avatar(
                        ui.painter(),
                        avatar_rect,
                        account.username(),
                        false,
                    );
                }

                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(account.username())
                                .size(18.0)
                                .strong()
                                .color(TEXT),
                        );
                        if account.is_offline() {
                            badge_accent(ui, "Offline Profile");
                        } else {
                            badge_ok(ui, "Microsoft Account");
                        }
                    });
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Player UUID:").size(11.5).color(MUTED));
                        ui.label(
                            RichText::new(account.uuid().to_string())
                                .size(11.5)
                                .monospace()
                                .color(TEXT2),
                        );
                    });
                });

                if state.config.microsoft_profile.is_some() && state.config.profile.is_some() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if state.config.use_microsoft_auth {
                            if ui.button("Switch to Offline").clicked() {
                                state.config.use_microsoft_auth = false;
                                state.save_config();
                                state.notify("Switched active profile to Offline.");
                            }
                        } else {
                            if ui.button("Switch to Microsoft").clicked() {
                                state.config.use_microsoft_auth = true;
                                state.save_config();
                                state.notify("Switched active profile to Microsoft.");
                            }
                        }
                    });
                }
            });
        } else {
            ui.label(
                RichText::new(
                    "No player account is currently configured. Configure an offline username or sign in with Microsoft below.",
                )
                .size(12.5)
                .color(TEXT2),
            );
        }
    });

    ui.add_space(8.0);

    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Offline Account")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );
            badge_accent(ui, "SkinsRestorer Compatible");
        });

        ui.label(
                RichText::new(
                    "Offline profiles allow you to play singleplayer and join offline/community servers without an internet login.",
                )
                .size(12.0)
                .color(TEXT2),
            );

        let current_username = state
            .config
            .profile
            .as_ref()
            .map(|p| p.username.as_str())
            .unwrap_or_default();
        let draft_id = egui::Id::new(("accounts-offline-username", current_username));
        let mut draft = ctx.data_mut(|data| {
            data.get_temp::<String>(draft_id)
                .unwrap_or_else(|| current_username.to_string())
        });

        let is_playing = state.playing.values().any(|playing| *playing);
        if is_playing {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Exit all running game instances before modifying your account.")
                    .color(WARNING),
            );
        }

        ui.add_space(4.0);
        ui.add_enabled_ui(!is_playing, |ui| {
            field_label(ui, "Username (3-16 letters, numbers or underscores)");
            let edit_resp = crate::ui::components::limited_text_edit(
                ui,
                "accounts-username",
                &mut draft,
                crate::ui::components::limits::USERNAME,
                "Steve",
            );
            let enter_pressed =
                edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Save Offline Account").clicked() || enter_pressed {
                    state.config.last_page = state.page.as_str().to_string();
                    state.config.selected_instance = state.selected_instance.clone();
                    state.config.show_snapshots = state.show_snapshots;
                    match save_offline_profile(
                        &mut state.config,
                        &state.paths.config_file(),
                        &draft,
                    ) {
                        Ok(()) => {
                            if let Some(prof) = &state.config.profile {
                                draft = prof.username.clone();
                            }
                            state.notify("Offline account saved successfully");
                        }
                        Err(e) => state.fail(e.user_message()),
                    }
                }
                if state.config.microsoft_profile.is_some()
                    && state.config.use_microsoft_auth
                    && state.config.profile.is_some()
                    && ui.button("Set as Active Profile").clicked()
                {
                    state.config.use_microsoft_auth = false;
                    state.save_config();
                    state.notify("Switched active profile to Offline.");
                }
            });
        });
        ctx.data_mut(|data| data.insert_temp(draft_id, draft));

        ui.add_space(10.0);

        egui::Frame::new()
                .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                .stroke(Stroke::new(1.0_f32, crate::ui::theme::palette(ui.ctx()).border))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(14, 12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Skin System & Server Compatibility")
                                .size(12.5)
                                .strong()
                                .color(TEXT),
                        );
                        badge_ok(ui, "Active");
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Offline profiles are deterministically generated using RFC 4122 UUID v3 (derived from OfflinePlayer:<username> via MD5) and standard Mojang profile structures. This provides native compatibility with SkinsRestorer, SkinSystem, and CustomSkinLoader across servers.",
                        )
                        .size(11.5)
                        .color(TEXT2),
                    );
                    if let Some(prof) = &state.config.profile {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Derived UUID:").size(11.0).color(MUTED));
                            ui.label(RichText::new(prof.uuid.to_string()).size(11.0).monospace().color(TEXT2));
                        });
                    }
                    ui.add_space(4.0);
                    if ui
                        .checkbox(
                            &mut state.config.skins_restorer_compat,
                            "Enable server-wide skin restoration compatibility flags",
                        )
                        .changed()
                    {
                        save_account_config(state, "Skin compatibility preference saved".to_string());
                    }
                    ui.label(
                        RichText::new(
                            "Tip: On servers running SkinsRestorer, type '/skin <skin_name>' in game chat to change your skin anytime.",
                        )
                        .size(11.0)
                        .color(MUTED),
                    );
                });
    });

    ui.add_space(8.0);

    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Microsoft Account")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );

            if state.config.microsoft_profile.is_some() {
                badge_ok(ui, "Connected");
            } else {
                badge_accent(ui, "Official Mojang");
            }
        });

        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Sign in with your official Microsoft & Xbox Live account to join online multiplayer servers, access Minecraft Realms, and automatically sync your official Mojang capes and skins.",
            )
            .size(12.0)
            .color(TEXT2),
        );

        if let Some(err) = &state.ms_login_error {
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("Error: {err}"))
                    .size(12.0)
                    .color(DANGER),
            );
        }

        ui.add_space(8.0);

        if let Some(ms) = state.config.microsoft_profile.clone() {
            egui::Frame::new()
                .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                .stroke(Stroke::new(
                    1.0_f32,
                    crate::ui::theme::palette(ui.ctx()).border,
                ))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (avatar_rect, _) =
                            ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::hover());
                        crate::ui::components::draw_avatar(
                            ui.painter(),
                            avatar_rect,
                            &ms.username,
                            &ms.uuid.to_string(),
                        );
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&ms.username).size(15.0).strong().color(TEXT),
                                );
                                if state.config.use_microsoft_auth {
                                    badge_ok(ui, "Active");
                                }
                            });
                            ui.label(
                                RichText::new(ms.uuid.to_string())
                                    .size(11.0)
                                    .monospace()
                                    .color(TEXT2),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if crate::ui::components::danger_button(ui, "Sign Out").clicked() {
                                state.config.microsoft_profile = None;
                                state.config.use_microsoft_auth = false;
                                state.save_config();
                                state.notify("Signed out from Microsoft account.");
                            }
                            if !state.config.use_microsoft_auth
                                && ui.button("Set as Active").clicked()
                            {
                                state.config.use_microsoft_auth = true;
                                state.save_config();
                                state.notify("Microsoft account set as active.");
                            }
                        });
                    });
                });
        } else if state.ms_login_loading {
            if let Some(code) = state.ms_device_code.clone() {
                egui::Frame::new()
                    .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                    .stroke(Stroke::new(
                        1.0_f32,
                        crate::ui::theme::palette(ui.ctx()).accent,
                    ))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("Microsoft Device Login Flow")
                                .size(14.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("1. Open in browser:").size(12.5).color(TEXT2));
                            let uri = if code.verification_uri.trim().is_empty() {
                                "https://www.microsoft.com/link"
                            } else {
                                code.verification_uri.as_str()
                            };
                            if ui.button("microsoft.com/link").on_hover_text(uri).clicked() {
                                let _ = open::that(uri);
                            }
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("2. Enter Code:").size(12.5).color(TEXT2));
                            ui.label(
                                RichText::new(&code.user_code)
                                    .size(16.0)
                                    .strong()
                                    .monospace()
                                    .color(crate::ui::theme::INFO),
                            );
                            if ui.button("Copy Code").clicked() {
                                ui.ctx().copy_text(code.user_code.clone());
                                state.notify("Code copied to clipboard!");
                            }
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                RichText::new("Waiting for sign-in approval in browser...")
                                    .size(12.0)
                                    .color(MUTED),
                            );
                            if ui.button("Cancel").clicked() {
                                state.cancel_microsoft_login();
                            }
                        });
                    });
            } else {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new("Requesting Microsoft login code...")
                            .size(12.5)
                            .color(TEXT2),
                    );
                    if ui.button("Cancel").clicked() {
                        state.cancel_microsoft_login();
                    }
                });
            }
        } else {
            ui.horizontal(|ui| {
                if crate::ui::components::primary_button(ui, "Sign in with Microsoft").clicked() {
                    state.start_microsoft_login();
                }
            });

            ui.add_space(8.0);
            ui.collapsing("Advanced Client Settings (Optional)", |ui| {
                ui.label(
                    RichText::new(
                        "MONORYX uses its built-in client configuration by default. If you prefer to use your own Azure App registration, enter its Client ID below.",
                    )
                    .size(11.5)
                    .color(TEXT2),
                );
                ui.add_space(4.0);
                field_label(ui, "Custom Client ID (Optional)");
                let mut client_id = state.config.microsoft_client_id.clone();
                let resp = ui.text_edit_singleline(&mut client_id);
                if resp.changed() {
                    state.config.microsoft_client_id = client_id;
                    state.save_config();
                }
            });
        }
    });
}

fn save_account_config(state: &mut AppState, msg: String) {
    state.config.last_page = state.page.as_str().to_string();
    state.config.selected_instance = state.selected_instance.clone();
    state.config.show_snapshots = state.show_snapshots;
    match state.config.save(&state.paths.config_file()) {
        Ok(()) => state.notify(msg),
        Err(e) => state.fail(e.user_message()),
    }
}

pub fn save_offline_profile(
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
