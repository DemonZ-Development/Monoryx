use crate::app::state::AppState;
use crate::ui::components::{
    badge, badge_accent, badge_ok, card_frame, compact_secondary_button, content_primary_button,
    content_secondary_button, page_header, pill_tab_button,
};
use crate::ui::skins::{
    apply_cape, build_skinsrestorer_command, default_alex_skin, default_skin_for_player,
    default_steve_skin, has_offline_skin, load_capes, load_local_skin_file, load_skin,
    remove_offline_skin, save_offline_preset, save_offline_skin, upload_offline_skin, upload_skin,
    viewer::SkinViewer, Cape, LoadedSkin,
};
use crate::ui::theme::{MUTED, TEXT, TEXT2};
use egui::{RichText, TextureHandle, TextureOptions};
use std::future::Future;
use std::sync::{mpsc, Arc, Mutex};

const PREVIEW_HEIGHT: f32 = 410.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SkinPreset {
    None,
    Steve,
    Alex,
}

struct SkinsPage {
    initialized: bool,
    active_username: String,
    is_offline: bool,
    has_custom_skin: bool,
    active_preset: SkinPreset,
    viewer: SkinViewer,
    skin: Option<TextureHandle>,
    cape: Option<TextureHandle>,
    skin_url: Option<String>,
    cape_url: Option<String>,
    uploaded_skin_url: Option<String>,
    custom_server_input: String,
    capes: Vec<Cape>,
    selected_cape: Option<String>,
    loading: bool,
    status: String,
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
}

enum Message {
    Loaded(LoadedSkin),
    LocalSkin(egui::ColorImage, bool),
    DefaultSkin(egui::ColorImage, bool, SkinPreset),
    UploadedSkinUrl(String),
    Capes(Vec<Cape>),
    Applied(String),
    Error(String),
}

impl Default for SkinsPage {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            initialized: false,
            active_username: String::new(),
            is_offline: false,
            has_custom_skin: false,
            active_preset: SkinPreset::None,
            viewer: SkinViewer::default(),
            skin: None,
            cape: None,
            skin_url: None,
            cape_url: None,
            uploaded_skin_url: None,
            custom_server_input: String::new(),
            capes: Vec::new(),
            selected_cape: None,
            loading: false,
            status: String::new(),
            tx,
            rx,
        }
    }
}

pub fn show(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    let id = egui::Id::new("monoryx-skins-page");

    let shared = ctx.data_mut(|data| {
        data.get_temp_mut_or_insert_with(id, || Arc::new(Mutex::new(SkinsPage::default())))
            .clone()
    });

    let mut guard = shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let page = &mut *guard;

    page.poll(ctx);

    if !page.initialized {
        page.initialized = true;
        if page.uploaded_skin_url.is_none() {
            page.uploaded_skin_url = state.config.last_uploaded_skin_url.clone();
        }
        if page.custom_server_input.is_empty() {
            page.custom_server_input = state.config.skin_server_url.clone();
        }
        if let Some(active) = state.config.active_account_ref() {
            page.active_username = active.username().to_string();
            page.is_offline = active.is_offline();
            page.load(state, ctx);
        }
    }

    if page.skin.is_none() && !page.loading {
        let (default_img, slim) = default_skin_for_player(&page.active_username);
        let options = TextureOptions::NEAREST;
        page.skin = Some(ctx.load_texture("skins-skin", default_img, options));
        page.viewer.slim = slim;
        page.active_preset = if slim {
            SkinPreset::Alex
        } else {
            SkinPreset::Steve
        };
    }

    page_header(
        ui,
        "Skins",
        "Preview Minecraft skins and manage your appearance.",
    );

    ui.add_space(8.0);

    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Skin Viewer").size(16.0).strong().color(TEXT));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if compact_secondary_button(ui, "↺ Reset Angle").clicked() {
                    page.viewer.reset();
                }

                if ui
                    .add_enabled(!page.loading, egui::Button::new("Refresh"))
                    .clicked()
                {
                    if let Some(account) = state.config.active_account_ref() {
                        page.active_username = account.username().to_string();
                        page.is_offline = account.is_offline();
                        page.load(state, ctx);
                    }
                }
            });
        });

        ui.add_space(4.0);

        let active = state.config.active_account_ref();
        match active {
            Some(_) => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Profile:").size(12.0).color(TEXT2));
                    ui.label(
                        RichText::new(page.active_username.as_str())
                            .size(13.0)
                            .strong()
                            .color(TEXT),
                    );
                    if page.is_offline {
                        badge(ui, "Offline");
                        if page.has_custom_skin {
                            badge_ok(ui, "Custom File");
                        } else {
                            badge_accent(ui, "Default Preset");
                        }
                    } else {
                        badge_ok(ui, "Microsoft");
                    }

                    if state.config.profile.is_some() && state.config.microsoft_profile.is_some() {
                        ui.add_space(12.0);
                        let current_is_ms = !page.is_offline;
                        if pill_tab_button(ui, "Offline Profile", !current_is_ms).clicked()
                            && current_is_ms
                        {
                            if let Some(prof) = &state.config.profile {
                                page.active_username = prof.username.clone();
                                page.is_offline = true;
                                page.load(state, ctx);
                            }
                        }
                        if pill_tab_button(ui, "Microsoft Profile", current_is_ms).clicked()
                            && !current_is_ms
                        {
                            if let Some(prof) = &state.config.microsoft_profile {
                                page.active_username = prof.username.clone();
                                page.is_offline = false;
                                page.load(state, ctx);
                            }
                        }
                    }
                });
            }
            None => {
                ui.label(
                    RichText::new(
                        "No active account selected. Go to Accounts to configure a profile.",
                    )
                    .color(MUTED),
                );
            }
        }

        ui.add_space(10.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

            ui.label(RichText::new("Model:").size(11.5).strong().color(TEXT2));
            if pill_tab_button(ui, "Classic (4px)", !page.viewer.slim).clicked() {
                page.viewer.slim = false;
            }
            if pill_tab_button(ui, "Slim (3px)", page.viewer.slim).clicked() {
                page.viewer.slim = true;
            }

            ui.add_space(6.0);
            ui.label(RichText::new("Layers:").size(11.5).strong().color(TEXT2));
            if pill_tab_button(ui, "Outer Layer", page.viewer.overlay).clicked() {
                page.viewer.overlay = !page.viewer.overlay;
            }
            if pill_tab_button(ui, "Cape", page.viewer.show_cape).clicked() {
                page.viewer.show_cape = !page.viewer.show_cape;
            }
            if pill_tab_button(ui, "Elytra", page.viewer.elytra).clicked() {
                page.viewer.elytra = !page.viewer.elytra;
            }

            ui.add_space(6.0);
            ui.label(RichText::new("Presets:").size(11.5).strong().color(TEXT2));
            if pill_tab_button(ui, "Steve", page.active_preset == SkinPreset::Steve).clicked() {
                let img = default_steve_skin();
                page.skin = Some(ctx.load_texture("skins-skin", img, TextureOptions::NEAREST));
                page.viewer.slim = false;
                page.active_preset = SkinPreset::Steve;
                page.has_custom_skin = false;
                page.status = "Preset Steve selected.".into();
            }
            if pill_tab_button(ui, "Alex", page.active_preset == SkinPreset::Alex).clicked() {
                let img = default_alex_skin();
                page.skin = Some(ctx.load_texture("skins-skin", img, TextureOptions::NEAREST));
                page.viewer.slim = true;
                page.active_preset = SkinPreset::Alex;
                page.has_custom_skin = false;
                page.status = "Preset Alex selected.".into();
            }
            if compact_secondary_button(ui, "🎲 Random").clicked() {
                if page.active_preset == SkinPreset::Steve {
                    let img = default_alex_skin();
                    page.skin = Some(ctx.load_texture("skins-skin", img, TextureOptions::NEAREST));
                    page.viewer.slim = true;
                    page.active_preset = SkinPreset::Alex;
                    page.has_custom_skin = false;
                    page.status = "Random preset: Alex".into();
                } else {
                    let img = default_steve_skin();
                    page.skin = Some(ctx.load_texture("skins-skin", img, TextureOptions::NEAREST));
                    page.viewer.slim = false;
                    page.active_preset = SkinPreset::Steve;
                    page.has_custom_skin = false;
                    page.status = "Random preset: Steve".into();
                }
            }
        });

        ui.add_space(10.0);

        let palette = crate::ui::theme::palette(ctx);
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), PREVIEW_HEIGHT),
            egui::Sense::drag(),
        );

        ui.painter().rect_filled(rect, 10.0, palette.elevated2);
        ui.painter().rect_stroke(
            rect,
            10.0,
            egui::Stroke::new(1.0_f32, palette.border),
            egui::StrokeKind::Inside,
        );

        if let Some(skin) = &page.skin {
            page.viewer
                .render(ui, rect, &response, skin, page.cape.as_ref());
        }

        if page.loading {
            let center = rect.center();
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                "Loading appearance...",
                egui::FontId::proportional(14.0),
                palette.accent,
            );
        }

        let hint_pos = egui::pos2(rect.right() - 12.0, rect.bottom() - 10.0);
        ui.painter().text(
            hint_pos,
            egui::Align2::RIGHT_BOTTOM,
            "Drag to rotate · Scroll to zoom",
            egui::FontId::proportional(11.0),
            MUTED,
        );

        if page.skin_url.is_some() || page.cape_url.is_some() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if let Some(url) = &page.skin_url {
                    if compact_secondary_button(ui, "↗ Open Skin Texture").clicked() {
                        let _ = open::that(url);
                    }
                }
                if let Some(url) = &page.cape_url {
                    if compact_secondary_button(ui, "↗ Open Cape Texture").clicked() {
                        let _ = open::that(url);
                    }
                }
            });
        }
    });

    ui.add_space(10.0);

    card_frame(ui, |ui| {
        if page.is_offline {
            ui.label(
                RichText::new("Offline Skin Management")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );

            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    "Import a custom 64×64 PNG for local play or configure SkinsRestorer on multiplayer servers.",
                )
                .size(12.0)
                .color(TEXT2),
            );

            ui.add_space(10.0);

            let (import_clicked, upload_clicked, sync_mods_clicked, reset_clicked, steve_clicked, alex_clicked) = ui
                .horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                    let import = content_primary_button(ui, "Import Custom Skin (.png)").clicked();
                    let upload = content_secondary_button(ui, "Upload to DemonZ Skin Server").clicked();
                    let sync = compact_secondary_button(ui, "Sync to Mod Loaders").clicked();
                    let reset = page.has_custom_skin
                        && content_secondary_button(ui, "Reset to Default Skin").clicked();
                    let steve = compact_secondary_button(ui, "Save Steve Preset").clicked();
                    let alex = compact_secondary_button(ui, "Save Alex Preset").clicked();
                    (import, upload, sync, reset, steve, alex)
                })
                .inner;

            let picked = import_clicked
                .then(|| {
                    rfd::FileDialog::new()
                        .add_filter("Minecraft skin", &["png"])
                        .pick_file()
                })
                .flatten();

            if let Some(path) = picked {
                let skins_dir = state.paths.root().join("skins");
                let instances = state.instances.clone();
                let username = page.active_username.clone();

                page.spawn(state, ctx, "Importing custom skin...", async move {
                    match save_offline_skin(&skins_dir, &username, &path) {
                        Ok(image) => {
                            let _ = instances.sync_offline_skin_all(&skins_dir, &username);
                            Message::LocalSkin(image, true)
                        }
                        Err(err) => Message::Error(err),
                    }
                });
            }

            if upload_clicked {
                let skins_dir = state.paths.root().join("skins");
                let path = skins_dir.join(format!("{}.png", page.active_username));
                let bytes_res = if path.is_file() {
                    std::fs::read(&path).map_err(|e| e.to_string())
                } else {
                    let (_, is_alex) = default_skin_for_player(&page.active_username);
                    let _ = save_offline_preset(&skins_dir, &page.active_username, is_alex);
                    std::fs::read(&path).map_err(|e| e.to_string())
                };

                match bytes_res {
                    Ok(bytes) => {
                        let http = state.http.clone();
                        let endpoint = if page.custom_server_input.trim().is_empty() {
                            state.config.effective_skin_server_url().to_string()
                        } else {
                            page.custom_server_input.trim().to_string()
                        };
                        let username = page.active_username.clone();
                        let slim = page.viewer.slim;

                        page.spawn(state, ctx, "Uploading skin to DemonZ skin server...", async move {
                            match upload_offline_skin(&http, &endpoint, &username, bytes, slim).await {
                                Ok(url) => Message::UploadedSkinUrl(url),
                                Err(err) => Message::Error(err),
                            }
                        });
                    }
                    Err(err) => {
                        page.status = format!("Could not prepare skin for upload: {err}");
                    }
                }
            }

            if sync_mods_clicked {
                let skins_dir = state.paths.root().join("skins");
                let username = page.active_username.clone();
                match state.instances.sync_offline_skin_all(&skins_dir, &username) {
                    Ok(count) => {
                        page.status = format!("Synchronized skin to {count} instance(s) for mod loaders.");
                        state.notify(format!("Synced skin to {count} instance(s)"));
                    }
                    Err(err) => {
                        page.status = format!("Sync failed: {err}");
                    }
                }
            }

            if reset_clicked {
                let skins_dir = state.paths.root().join("skins");
                remove_offline_skin(&skins_dir, &page.active_username);
                let _ = state.instances.sync_offline_skin_all(&skins_dir, &page.active_username);
                page.has_custom_skin = false;
                page.load(state, ctx);
            }

            if steve_clicked {
                let skins_dir = state.paths.root().join("skins");
                let instances = state.instances.clone();
                let username = page.active_username.clone();
                page.spawn(state, ctx, "Saving Steve preset...", async move {
                    match save_offline_preset(&skins_dir, &username, false) {
                        Ok(image) => {
                            let _ = instances.sync_offline_skin_all(&skins_dir, &username);
                            Message::LocalSkin(image, true)
                        }
                        Err(err) => Message::Error(err),
                    }
                });
            }

            if alex_clicked {
                let skins_dir = state.paths.root().join("skins");
                let instances = state.instances.clone();
                let username = page.active_username.clone();
                page.spawn(state, ctx, "Saving Alex preset...", async move {
                    match save_offline_preset(&skins_dir, &username, true) {
                        Ok(image) => {
                            let _ = instances.sync_offline_skin_all(&skins_dir, &username);
                            Message::LocalSkin(image, true)
                        }
                        Err(err) => Message::Error(err),
                    }
                });
            }

            if !page.active_username.is_empty() {
                ui.add_space(14.0);
                let p = crate::ui::theme::palette(ctx);
                let info_frame = egui::Frame::new()
                    .fill(p.elevated2)
                    .stroke(egui::Stroke::new(1.0_f32, p.border))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(12));

                info_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("SkinsRestorer Server Integration")
                                .size(13.0)
                                .strong()
                                .color(TEXT),
                        );
                        badge(ui, "Multiplayer Plugin");
                        if page.uploaded_skin_url.is_some() {
                            badge_ok(ui, "DemonZ Skin Hosted");
                        }
                    });

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "On servers equipped with SkinsRestorer, skins are fetched by username from Mojang or by direct URL via Mineskin and DemonZ:",
                        )
                        .size(11.5)
                        .color(TEXT2),
                    );

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("1. Mojang Skin:").size(11.5).strong().color(TEXT2));
                        let cmd = format!("/skin {}", page.active_username);
                        ui.label(RichText::new(&cmd).monospace().size(11.5).color(p.accent));
                        if compact_secondary_button(ui, "Copy").clicked() {
                            ui.ctx().copy_text(cmd);
                            state.notify("Copied '/skin <name>' command");
                        }
                    });

                    ui.add_space(6.0);
                    if let Some(url) = &page.uploaded_skin_url {
                        let cmd = build_skinsrestorer_command(url, page.viewer.slim);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("2. Hosted URL:").size(11.5).strong().color(TEXT2));
                            ui.label(RichText::new(url).monospace().size(11.0).color(TEXT));
                            if compact_secondary_button(ui, "↗ Open").clicked() {
                                let _ = open::that(url);
                            }
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("3. Multiplayer Command:").size(11.5).strong().color(TEXT2));
                            ui.label(RichText::new(&cmd).monospace().size(11.5).color(p.accent));
                            if compact_secondary_button(ui, "Copy Command").clicked() {
                                ui.ctx().copy_text(cmd);
                                state.notify("Copied SkinsRestorer '/skin url' command to clipboard");
                            }
                            if compact_secondary_button(ui, "Clear").clicked() {
                                page.uploaded_skin_url = None;
                                state.config.last_uploaded_skin_url = None;
                                state.save_config();
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("2. Custom Web Skin:").size(11.5).strong().color(TEXT2));
                            let url_cmd = "/skin url <png_url> [classic|slim]";
                            ui.label(RichText::new(url_cmd).monospace().size(11.5).color(TEXT));
                            if compact_secondary_button(ui, "Upload to DemonZ").clicked() {
                                let skins_dir = state.paths.root().join("skins");
                                let path = skins_dir.join(format!("{}.png", page.active_username));
                                let bytes_res = if path.is_file() {
                                    std::fs::read(&path).map_err(|e| e.to_string())
                                } else {
                                    let (_, is_alex) = default_skin_for_player(&page.active_username);
                                    let _ = save_offline_preset(&skins_dir, &page.active_username, is_alex);
                                    std::fs::read(&path).map_err(|e| e.to_string())
                                };

                                match bytes_res {
                                    Ok(bytes) => {
                                        let http = state.http.clone();
                                        let endpoint = if page.custom_server_input.trim().is_empty() {
                                            state.config.effective_skin_server_url().to_string()
                                        } else {
                                            page.custom_server_input.trim().to_string()
                                        };
                                        let username = page.active_username.clone();
                                        let slim = page.viewer.slim;

                                        page.spawn(state, ctx, "Uploading skin to DemonZ skin server...", async move {
                                            match upload_offline_skin(&http, &endpoint, &username, bytes, slim).await {
                                                Ok(url) => Message::UploadedSkinUrl(url),
                                                Err(err) => Message::Error(err),
                                            }
                                        });
                                    }
                                    Err(err) => {
                                        page.status = format!("Could not prepare skin for upload: {err}");
                                    }
                                }
                            }
                        });
                    }

                    ui.add_space(8.0);
                    ui.collapsing("Advanced: Custom Skin Server Endpoint", |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Upload Endpoint:").size(11.0).color(TEXT2));
                            let edit = ui.add(
                                egui::TextEdit::singleline(&mut page.custom_server_input)
                                    .hint_text(state.config.effective_skin_server_url())
                                    .desired_width(260.0),
                            );
                            if edit.lost_focus() && page.custom_server_input != state.config.skin_server_url {
                                state.config.skin_server_url = page.custom_server_input.trim().to_string();
                                state.save_config();
                            }
                            if compact_secondary_button(ui, "Reset").clicked() {
                                page.custom_server_input.clear();
                                state.config.skin_server_url.clear();
                                state.save_config();
                            }
                        });
                    });
                });

                ui.add_space(10.0);
                let mod_loader_frame = egui::Frame::new()
                    .fill(p.elevated2)
                    .stroke(egui::Stroke::new(1.0_f32, p.border))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(12));

                mod_loader_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Mod Loader Skin Integration")
                                .size(13.0)
                                .strong()
                                .color(TEXT),
                        );
                        badge(ui, "Fabric / Forge / NeoForge / Quilt");
                        badge_accent(ui, "CustomSkinLoader");
                    });

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Offline custom skins are automatically mapped to instance directories (CustomSkinLoader/LocalSkin/skins/{username}.png) with auto-generated CustomSkinLoader.json configuration. Client skin mods display your custom appearance in both singleplayer and multiplayer.",
                        )
                        .size(11.5)
                        .color(TEXT2),
                    );

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if compact_secondary_button(ui, "Sync All Instances Now").clicked() {
                            let skins_dir = state.paths.root().join("skins");
                            match state.instances.sync_offline_skin_all(&skins_dir, &page.active_username) {
                                Ok(count) => {
                                    state.notify(format!("Synchronized skin to {count} instance(s)"));
                                    page.status = format!("Synchronized offline skin to {count} instance(s) for mod loaders.");
                                }
                                Err(err) => {
                                    page.status = format!("Mod loader sync failed: {err}");
                                }
                            }
                        }
                    });
                });
            }
        } else {
            ui.label(
                RichText::new("Microsoft Skin Management")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );

            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    "Upload custom appearances directly to official Mojang servers and manage your active capes.",
                )
                .size(12.0)
                .color(TEXT2),
            );

            ui.add_space(10.0);

            let Some(token) = state
                .config
                .microsoft_profile
                .as_ref()
                .map(|profile| profile.access_token.clone())
                .filter(|token| !token.is_empty())
            else {
                ui.label(
                    RichText::new("Sign in with Microsoft in Accounts to manage official skins.")
                        .color(MUTED),
                );
                return;
            };

            let (upload, refresh) = ui
                .horizontal(|ui| {
                    let upload =
                        content_primary_button(ui, "Upload Skin to Mojang (.png)").clicked();
                    let refresh = content_secondary_button(ui, "Load Official Capes").clicked();
                    (upload, refresh)
                })
                .inner;

            let picked = upload
                .then(|| {
                    rfd::FileDialog::new()
                        .add_filter("Minecraft skin", &["png"])
                        .pick_file()
                })
                .flatten();

            if let Some(path) = picked {
                let http = state.http.clone();
                let token = token.clone();
                let slim = page.viewer.slim;

                page.spawn(state, ctx, "Uploading skin...", async move {
                    let result = async {
                        let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
                        upload_skin(&http, &token, bytes, slim).await
                    }
                    .await;

                    match result {
                        Ok(()) => Message::Applied("Skin uploaded to Mojang profile.".into()),
                        Err(error) => Message::Error(error),
                    }
                });
            }

            if refresh {
                let http = state.http.clone();
                let token = token.clone();

                page.spawn(state, ctx, "Loading capes...", async move {
                    match load_capes(&http, &token).await {
                        Ok(capes) => Message::Capes(capes),
                        Err(error) => Message::Error(error),
                    }
                });
            }

            if !page.capes.is_empty() {
                ui.add_space(10.0);

                let label = |cape: &Cape| {
                    if cape.alias.is_empty() {
                        cape.id.clone()
                    } else {
                        cape.alias.clone()
                    }
                };

                let selected_text = page
                    .capes
                    .iter()
                    .find(|cape| page.selected_cape.as_deref() == Some(cape.id.as_str()))
                    .map(label)
                    .unwrap_or_else(|| "Select cape".to_string());

                egui::ComboBox::from_id_salt("skins-cape-selector")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        for cape in &page.capes {
                            ui.selectable_value(
                                &mut page.selected_cape,
                                Some(cape.id.clone()),
                                label(cape),
                            );
                        }
                    });

                let can_apply = !page.loading && page.selected_cape.is_some();

                let apply = ui
                    .add_enabled(can_apply, egui::Button::new("Apply Cape"))
                    .clicked()
                    .then(|| page.selected_cape.clone())
                    .flatten();

                if let Some(cape_id) = apply {
                    let http = state.http.clone();
                    let token = token.clone();

                    page.spawn(state, ctx, "Applying cape...", async move {
                        match apply_cape(&http, &token, &cape_id).await {
                            Ok(()) => Message::Applied("Cape applied.".into()),
                            Err(error) => Message::Error(error),
                        }
                    });
                }
            }
        }
    });

    if page.loading || !page.status.is_empty() {
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if page.loading {
                ui.spinner();
            }

            ui.label(RichText::new(&page.status).color(TEXT2));
        });
    }
}

impl SkinsPage {
    fn spawn<F>(&mut self, state: &AppState, ctx: &egui::Context, status: &str, task: F)
    where
        F: Future<Output = Message> + Send + 'static,
    {
        self.loading = true;
        self.status = status.to_string();

        let tx = self.tx.clone();
        let repaint = ctx.clone();

        state.runtime.spawn(async move {
            let message = task.await;

            let _ = tx.send(message);
            repaint.request_repaint();
        });
    }

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(message) = self.rx.try_recv() {
            self.loading = false;

            match message {
                Message::Loaded(loaded) => {
                    let options = TextureOptions::NEAREST;

                    self.skin = Some(ctx.load_texture("skins-skin", loaded.skin, options));
                    self.cape = loaded
                        .cape
                        .map(|image| ctx.load_texture("skins-cape", image, options));
                    self.skin_url = Some(loaded.skin_url);
                    self.cape_url = loaded.cape_url;
                    self.viewer.slim = loaded.slim;
                    self.has_custom_skin = false;
                    self.active_preset = SkinPreset::None;
                    self.status = "Skin loaded.".into();
                }
                Message::LocalSkin(image, is_custom) => {
                    let options = TextureOptions::NEAREST;

                    self.skin = Some(ctx.load_texture("skins-skin", image, options));
                    self.cape = None;
                    self.skin_url = None;
                    self.cape_url = None;
                    self.has_custom_skin = is_custom;
                    self.active_preset = SkinPreset::None;
                    self.status = if is_custom {
                        "Custom local skin active.".into()
                    } else {
                        "Local skin loaded.".into()
                    };
                }
                Message::DefaultSkin(image, slim, preset) => {
                    let options = TextureOptions::NEAREST;

                    self.skin = Some(ctx.load_texture("skins-skin", image, options));
                    self.cape = None;
                    self.skin_url = None;
                    self.cape_url = None;
                    self.viewer.slim = slim;
                    self.has_custom_skin = false;
                    self.active_preset = preset;
                    self.status = "Default Minecraft skin loaded.".into();
                }
                Message::Capes(capes) => {
                    self.selected_cape = capes
                        .iter()
                        .find(|cape| cape.state == "ACTIVE")
                        .map(|cape| cape.id.clone());

                    self.capes = capes;
                    self.status = "Capes loaded.".into();
                }
                Message::UploadedSkinUrl(url) => {
                    self.uploaded_skin_url = Some(url);
                    self.status = "Skin uploaded to DemonZ skin server. /skin command ready!".into();
                }
                Message::Applied(message) => {
                    self.status = message;
                }
                Message::Error(error) => {
                    self.status = error;
                }
            }
        }
    }

    fn load(&mut self, state: &AppState, ctx: &egui::Context) {
        let username = self.active_username.trim().to_string();
        if self.loading || username.is_empty() {
            return;
        }

        let skins_dir = state.paths.root().join("skins");
        if self.is_offline && has_offline_skin(&skins_dir, &username) {
            let path = skins_dir.join(format!("{username}.png"));
            match load_local_skin_file(&path) {
                Ok(image) => {
                    let options = TextureOptions::NEAREST;
                    self.skin = Some(ctx.load_texture("skins-skin", image, options));
                    self.cape = None;
                    self.skin_url = None;
                    self.cape_url = None;
                    self.has_custom_skin = true;
                    self.active_preset = SkinPreset::None;
                    self.status = "Custom local skin active.".into();
                    return;
                }
                Err(_) => {
                    remove_offline_skin(&skins_dir, &username);
                }
            }
        }

        let http = state.http.clone();
        let fallback_username = username.clone();
        let is_offline = self.is_offline;

        self.spawn(state, ctx, "Loading skin...", async move {
            match load_skin(&http, &username).await {
                Ok(loaded) => Message::Loaded(loaded),
                Err(error) => {
                    if is_offline {
                        let (default_img, slim) = default_skin_for_player(&fallback_username);
                        let preset = if slim {
                            SkinPreset::Alex
                        } else {
                            SkinPreset::Steve
                        };
                        Message::DefaultSkin(default_img, slim, preset)
                    } else {
                        Message::Error(error)
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::offline::OfflineProfile;

    #[test]
    fn skins_page_renders_offline_profile_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let paths = crate::storage::paths::MonoryxPaths::new(dir.path().to_path_buf());
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths);
        state.config.profile = Some(OfflineProfile::new("Cyrus").unwrap());

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
    fn skins_page_renders_with_uploaded_skin_url_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let paths = crate::storage::paths::MonoryxPaths::new(dir.path().to_path_buf());
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths);
        state.config.profile = Some(OfflineProfile::new("Cyrus").unwrap());
        state.config.last_uploaded_skin_url =
            Some("https://demonz.org/api/skins/download/cyrus.png".to_string());

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
}
