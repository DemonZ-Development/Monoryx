use crate::app::state::AppState;
use crate::ui::components::{card_frame, page_header};
use crate::ui::skins::{
    apply_cape, load_capes, load_skin, upload_skin, viewer::SkinViewer, Cape, LoadedSkin,
};
use crate::ui::theme::{MUTED, TEXT, TEXT2};
use egui::{RichText, TextureHandle, TextureOptions};
use std::future::Future;
use std::sync::{mpsc, Arc, Mutex};

const PREVIEW_HEIGHT: f32 = 390.0;

struct SkinsPage {
    initialized: bool,
    username: String,
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
    Capes(Vec<Cape>),
    Applied(String),
    Error(String),
}

impl Default for SkinsPage {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            initialized: false,
            username: String::new(),
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
        if let Some(profile) = state.config.microsoft_profile.as_ref() {
            let username = profile.username.clone();
            page.initialized = true;
            page.username = username.clone();

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

        let mut load_requested = false;

        ui.horizontal(|ui| {
            let input = ui.add(
                egui::TextEdit::singleline(&mut page.username)
                    .hint_text("Minecraft username")
                    .desired_width(230.0),
            );

            let submitted = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let clicked = ui
                .add_enabled(!page.loading, egui::Button::new("Load"))
                .clicked();

            load_requested = submitted || clicked;

            let own_name = ui
                .button("My Skin")
                .clicked()
                .then(|| {
                    state
                        .config
                        .microsoft_profile
                        .as_ref()
                        .map(|profile| profile.username.clone())
                })
                .flatten();

            if let Some(name) = own_name {
                page.username = name;
                load_requested = true;
            }
        });

        let username = page.username.trim().to_string();

        if load_requested && !page.loading && !username.is_empty() {
            page.load(state, ctx);
        }

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.checkbox(&mut page.viewer.slim, "Slim");
            ui.checkbox(&mut page.viewer.overlay, "Outer Layer");
            ui.checkbox(&mut page.viewer.show_cape, "Cape");
            // ui.checkbox(&mut page.viewer.elytra, "Elytra");

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
                "Load a skin to preview",
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
        ui.label(
            RichText::new("Skin Applier")
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
            ui.label(RichText::new("Sign in with Microsoft to change your skin.").color(MUTED));
            return;
        };

        let (upload, refresh) = ui
            .horizontal(|ui| {
                let upload = ui
                    .add_enabled(!page.loading, egui::Button::new("Upload PNG"))
                    .clicked();
                let refresh = ui
                    .add_enabled(!page.loading, egui::Button::new("Load My Capes"))
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
                    Ok(()) => Message::Applied("Skin uploaded.".into()),
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

        if page.capes.is_empty() {
            return;
        }

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
                    self.status = "Skin loaded.".into();
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
        if self.loading || self.username.trim().is_empty() {
            return;
        }

        let http = state.http.clone();
        let username = self.username.trim().to_string();

        self.spawn(state, ctx, "Loading skin...", async move {
            match load_skin(&http, &username).await {
                Ok(loaded) => Message::Loaded(loaded),
                Err(error) => Message::Error(error),
            }
        });
    }
}
