use crate::app::state::AppState;
use crate::ui::components::{badge, badge_ok, card_frame, page_header};
use crate::ui::skins::{
    apply_cape, has_offline_skin, load_capes, load_local_skin_file, load_skin, remove_offline_skin,
    save_offline_skin, upload_skin, viewer::SkinViewer, Cape, LoadedSkin,
};
use crate::ui::theme::{MUTED, TEXT, TEXT2};
use egui::{RichText, TextureHandle, TextureOptions};
use std::future::Future;
use std::sync::{mpsc, Arc, Mutex};

const PREVIEW_HEIGHT: f32 = 390.0;

struct SkinsPage {
    initialized: bool,
    active_username: String,
    is_offline: bool,
    has_custom_skin: bool,
    viewer: SkinViewer,
    skin: Option<TextureHandle>,
    cape: Option<TextureHandle>,
    skin_url: Option<String>,
    cape_url: Option<String>,
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
            viewer: SkinViewer::default(),
            skin: None,
            cape: None,
            skin_url: None,
            cape_url: None,
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
        if let Some(active) = state.config.active_account_ref() {
            page.active_username = active.username().to_string();
            page.is_offline = active.is_offline();
            page.load(state, ctx);
        }
    }

    page_header(
        ui,
        "Skins",
        "Preview Minecraft skins and manage your appearance.",
    );

    ui.add_space(8.0);

    card_frame(ui, |ui| {
        ui.label(RichText::new("Skin Viewer").size(16.0).strong().color(TEXT));

        ui.add_space(6.0);

        let active = state.config.active_account_ref();
        match active {
            Some(account) => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Profile:").color(TEXT2));
                    ui.label(
                        RichText::new(page.active_username.as_str())
                            .strong()
                            .color(TEXT),
                    );
                    if page.is_offline {
                        badge(ui, "Offline");
                        if page.has_custom_skin {
                            badge_ok(ui, "Custom File");
                        }
                    } else {
                        badge_ok(ui, "Microsoft");
                    }

                    ui.add_space(8.0);

                    if ui
                        .add_enabled(!page.loading, egui::Button::new("Refresh"))
                        .clicked()
                    {
                        page.active_username = account.username().to_string();
                        page.is_offline = account.is_offline();
                        page.load(state, ctx);
                    }
                });

                if state.config.profile.is_some() && state.config.microsoft_profile.is_some() {
                    let current_is_ms = !page.is_offline;
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("View Account:").size(11.5).color(TEXT2));
                        if ui
                            .selectable_label(!current_is_ms, "Offline Profile")
                            .clicked()
                            && current_is_ms
                        {
                            if let Some(prof) = &state.config.profile {
                                page.active_username = prof.username.clone();
                                page.is_offline = true;
                                page.load(state, ctx);
                            }
                        }
                        if ui
                            .selectable_label(current_is_ms, "Microsoft Profile")
                            .clicked()
                            && !current_is_ms
                        {
                            if let Some(prof) = &state.config.microsoft_profile {
                                page.active_username = prof.username.clone();
                                page.is_offline = false;
                                page.load(state, ctx);
                            }
                        }
                    });
                }
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

        ui.horizontal(|ui| {
            ui.checkbox(&mut page.viewer.slim, "Slim");
            ui.checkbox(&mut page.viewer.overlay, "Outer Layer");
            ui.checkbox(&mut page.viewer.show_cape, "Cape");
            ui.checkbox(&mut page.viewer.elytra, "Elytra");

            if ui.button("Reset View").clicked() {
                page.viewer.reset();
            }
        });

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), PREVIEW_HEIGHT),
            egui::Sense::drag(),
        );

        ui.painter()
            .rect_filled(rect, 10.0, ui.visuals().extreme_bg_color);

        if let Some(skin) = &page.skin {
            page.viewer
                .render(ui, rect, &response, skin, page.cape.as_ref());
        } else {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if page.loading {
                    "Loading appearance..."
                } else {
                    "No skin loaded"
                },
                egui::FontId::proportional(14.0),
                MUTED,
            );
        }

        ui.label(
            RichText::new("Drag to rotate · Scroll to zoom")
                .size(11.0)
                .color(MUTED),
        );

        if let Some(url) = &page.skin_url {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Skin texture").color(TEXT2));

                if ui.button("Open").clicked() {
                    let _ = open::that(url);
                }
            });
        }

        if let Some(url) = &page.cape_url {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Cape texture").color(TEXT2));

                if ui.button("Open").clicked() {
                    let _ = open::that(url);
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

            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Import a custom PNG for your offline profile or rely on SkinsRestorer on multiplayer servers.",
                )
                .size(11.5)
                .color(TEXT2),
            );

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let import_clicked = ui
                    .add_enabled(!page.loading, egui::Button::new("Import Custom Skin (PNG)"))
                    .clicked();

                let picked = import_clicked
                    .then(|| {
                        rfd::FileDialog::new()
                            .add_filter("Minecraft skin", &["png"])
                            .pick_file()
                    })
                    .flatten();

                if let Some(path) = picked {
                    let skins_dir = state.paths.root().join("skins");
                    let username = page.active_username.clone();

                    page.spawn(state, ctx, "Importing custom skin...", async move {
                        match save_offline_skin(&skins_dir, &username, &path) {
                            Ok(image) => Message::LocalSkin(image, true),
                            Err(err) => Message::Error(err),
                        }
                    });
                }

                if page.has_custom_skin
                    && ui
                        .add_enabled(!page.loading, egui::Button::new("Reset to Mojang Skin"))
                        .clicked()
                {
                    let skins_dir = state.paths.root().join("skins");
                    remove_offline_skin(&skins_dir, &page.active_username);
                    page.has_custom_skin = false;
                    page.load(state, ctx);
                }
            });

            if !page.active_username.is_empty() {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("SkinsRestorer Command:")
                            .size(11.5)
                            .color(TEXT2),
                    );
                    let cmd = format!("/skin {}", page.active_username);
                    ui.label(RichText::new(&cmd).size(11.5).monospace().color(TEXT));
                    if ui.button("Copy").clicked() {
                        ui.ctx().copy_text(cmd);
                        state.notify("SkinsRestorer command copied to clipboard");
                    }
                });
            }
        } else {
            ui.label(
                RichText::new("Microsoft Skin Management")
                    .size(16.0)
                    .strong()
                    .color(TEXT),
            );

            ui.add_space(4.0);

            let Some(token) = state
                .config
                .microsoft_profile
                .as_ref()
                .map(|profile| profile.access_token.clone())
                .filter(|token| !token.is_empty())
            else {
                ui.label(
                    RichText::new("Sign in with Microsoft to upload official skins.").color(MUTED),
                );
                return;
            };

            let (upload, refresh) = ui
                .horizontal(|ui| {
                    let upload = ui
                        .add_enabled(!page.loading, egui::Button::new("Upload Skin (PNG)"))
                        .clicked();
                    let refresh = ui
                        .add_enabled(!page.loading, egui::Button::new("Load Capes"))
                        .clicked();

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
                ui.add_space(8.0);

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
                    self.status = "Skin loaded.".into();
                }
                Message::LocalSkin(image, is_custom) => {
                    let options = TextureOptions::NEAREST;

                    self.skin = Some(ctx.load_texture("skins-skin", image, options));
                    self.cape = None;
                    self.skin_url = None;
                    self.cape_url = None;
                    self.has_custom_skin = is_custom;
                    self.status = if is_custom {
                        "Custom local skin active.".into()
                    } else {
                        "Local skin loaded.".into()
                    };
                }
                Message::Capes(capes) => {
                    self.selected_cape = capes
                        .iter()
                        .find(|cape| cape.state == "ACTIVE")
                        .map(|cape| cape.id.clone());

                    self.capes = capes;
                    self.status = "Capes loaded.".into();
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
                    self.status = "Custom local skin active.".into();
                    return;
                }
                Err(_) => {
                    remove_offline_skin(&skins_dir, &username);
                }
            }
        }

        let http = state.http.clone();

        self.spawn(state, ctx, "Loading skin...", async move {
            match load_skin(&http, &username).await {
                Ok(loaded) => Message::Loaded(loaded),
                Err(error) => Message::Error(error),
            }
        });
    }
}
