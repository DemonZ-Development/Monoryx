use crate::account::offline::OfflineProfile;
use crate::app::state::AppState;
use crate::ui::components::{badge_accent, badge_ok, card_frame, field_label, page_header};
use crate::ui::theme::{BORDER, ELEVATED2, MUTED, TEXT, TEXT2, WARNING};
use egui::{Color32, CornerRadius, RichText, Stroke};

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
            if state.config.profile.is_some() {
                badge_ok(ui, "Ready to Play");
            } else {
                badge_accent(ui, "No Profile Set");
            }
        });

        ui.add_space(4.0);

        if let Some(profile) = &state.config.profile {
            ui.horizontal(|ui| {
                let (avatar_rect, _) =
                    ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                let center = avatar_rect.center();
                ui.painter()
                    .circle_filled(center, 22.0, Color32::from_rgb(26, 28, 34));
                ui.painter()
                    .circle_stroke(center, 22.0, Stroke::new(1.5_f32, BORDER));
                let initial = profile
                    .username
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_uppercase()
                    .to_string();
                ui.painter().text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    initial,
                    egui::FontId::proportional(20.0),
                    TEXT,
                );

                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&profile.username)
                                .size(18.0)
                                .strong()
                                .color(TEXT),
                        );
                        badge_accent(ui, "Offline Profile");
                    });
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Player UUID:").size(11.5).color(MUTED));
                        ui.label(
                            RichText::new(profile.uuid.to_string())
                                .size(11.5)
                                .monospace()
                                .color(TEXT2),
                        );
                    });
                });
            });
        } else {
            ui.label(
                    RichText::new("No player account is currently configured. Configure an offline username below to launch Minecraft.")
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
            let edit_resp = ui.text_edit_singleline(&mut draft);
            let enter_pressed =
                edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            ui.add_space(4.0);
            if ui.button("Save Offline Account").clicked() || enter_pressed {
                state.config.last_page = state.page.as_str().to_string();
                state.config.selected_instance = state.selected_instance.clone();
                state.config.show_snapshots = state.show_snapshots;
                match save_offline_profile(&mut state.config, &state.paths.config_file(), &draft) {
                    Ok(()) => {
                        if let Some(prof) = &state.config.profile {
                            draft = prof.username.clone();
                        }
                        state.notify("Offline account saved successfully");
                    }
                    Err(e) => state.fail(e.user_message()),
                }
            }
        });
        ctx.data_mut(|data| data.insert_temp(draft_id, draft));

        ui.add_space(10.0);

        egui::Frame::new()
                .fill(ELEVATED2)
                .stroke(Stroke::new(1.0_f32, BORDER))
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

            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(160, 160, 180, 22))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(110, 115, 130)))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 3))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Coming Soon")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(215, 220, 235)),
                    );
                });
        });

        ui.add_space(2.0);
        ui.label(
                RichText::new(
                    "Sign in with your official Microsoft & Xbox Live account to join online-mode multiplayer servers, access Minecraft Realms, and automatically sync your official Mojang capes and skins.",
                )
                .size(12.0)
                .color(TEXT2),
            );

        ui.add_space(10.0);

        let btn_id = ui.make_persistent_id("btn_ms_signin");
        let is_hovered = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(btn_id).unwrap_or(false));
        let hover_fade = ui
            .ctx()
            .animate_bool_with_time(btn_id.with("hover"), is_hovered, 0.15);
        if hover_fade > 0.001 && hover_fade < 0.999 {
            ui.ctx().request_repaint();
        }
        let btn_fill = ELEVATED2.lerp_to_gamma(Color32::from_rgb(32, 34, 42), hover_fade);
        let btn_border = BORDER.lerp_to_gamma(Color32::from_rgb(70, 75, 90), hover_fade);

        let ms_button_frame = egui::Frame::new()
            .fill(btn_fill)
            .stroke(Stroke::new(1.0_f32, btn_border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(14, 9))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (icon_rect, _) =
                        ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                    let half = 6.0_f32;
                    let gap = 2.0_f32;
                    let p = icon_rect.min;
                    let light = Color32::from_rgb(220, 220, 225);
                    let dim = Color32::from_rgb(140, 140, 150);
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(p, egui::vec2(half, half)),
                        1,
                        light,
                    );
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            p + egui::vec2(half + gap, 0.0),
                            egui::vec2(half, half),
                        ),
                        1,
                        dim,
                    );
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            p + egui::vec2(0.0, half + gap),
                            egui::vec2(half, half),
                        ),
                        1,
                        dim,
                    );
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            p + egui::vec2(half + gap, half + gap),
                            egui::vec2(half, half),
                        ),
                        1,
                        light,
                    );

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Sign in with Microsoft")
                            .size(13.0)
                            .strong()
                            .color(TEXT),
                    );
                });
            });

        let btn_resp = ui.interact(ms_button_frame.response.rect, btn_id, egui::Sense::click());
        ui.ctx()
            .data_mut(|d| d.insert_temp(btn_id, btn_resp.hovered()));
        if btn_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if btn_resp
                .on_hover_text("Microsoft authentication is under active development and will arrive in an upcoming MONORYX update.")
                .clicked()
            {
                state.notify("Microsoft Account sign-in is coming soon in an upcoming MONORYX release!");
            }

        ui.add_space(10.0);

        egui::Frame::new()
                .fill(ELEVATED2)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.label(RichText::new("Upcoming Microsoft Features").size(12.5).strong().color(TEXT));
                    ui.add_space(6.0);

                    let features = [
                        ("Official Servers & Realms", "Authenticate with Xbox Live to play on Hypixel, Mineplex, and official Realms"),
                        ("Cloud Skin Synchronization", "Load your official skin directly from Mojang profile servers"),
                        ("Secure Token Refresh", "Industry-standard OAuth 2.0 PKCE flow with encrypted local token storage"),
                    ];

                    for (title, desc) in features {
                        ui.horizontal(|ui| {
                            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot_rect.center(), 2.5_f32, Color32::from_rgb(130, 135, 150));
                            ui.add_space(4.0);
                            ui.label(RichText::new(title).size(12.0).strong().color(TEXT));
                            ui.label(RichText::new(format!("— {desc}")).size(11.5).color(TEXT2));
                        });
                        ui.add_space(3.0);
                    }
                });
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
