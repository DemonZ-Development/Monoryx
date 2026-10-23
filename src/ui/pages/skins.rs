use crate::app::events::Page;
use crate::app::state::AppState;
use crate::ui::components::{
    badge, badge_accent, badge_ok, badge_warning, button_sized, card_frame, compact_danger_button,
    compact_secondary_button, content_secondary_button, draw_avatar, draw_cute_avatar, elide,
    page_header, Tone,
};
use crate::ui::skins::{
    apply_cape, build_skinsrestorer_command, default_alex_skin, default_skin_for_player,
    default_steve_skin, has_offline_skin, instance_has_skin_mod, load_capes, load_local_skin_file,
    load_skin, remove_offline_skin, save_offline_preset, save_offline_skin, upload_offline_skin,
    upload_skin, viewer::SkinViewer, Cape, LoadedSkin,
};
use crate::ui::theme::{metrics, palette, type_scale, DANGER, MUTED, TEXT, TEXT2};
use egui::{Color32, CornerRadius, RichText, Stroke, TextureHandle, TextureOptions};
use std::future::Future;
use std::sync::{mpsc, Arc, Mutex};

const PREVIEW_HEIGHT: f32 = 420.0;
const WIDE_LAYOUT_MIN: f32 = 940.0;
const COLUMN_GAP: f32 = 14.0;
const SIDEBAR_MIN: f32 = 320.0;
const SIDEBAR_MAX: f32 = 388.0;

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
    account_username: String,
    account_offline: bool,
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
    status_error: bool,
    mod_scan: Option<(f64, Vec<String>)>,
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
            account_username: String::new(),
            account_offline: false,
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
            status_error: false,
            mod_scan: None,
            tx,
            rx,
        }
    }
}

pub fn show(state: &mut AppState, ctx: &egui::Context, ui: &mut egui::Ui) {
    if state.config.active_account_ref().is_none() {
        state.set_page(Page::Home);
        return;
    }
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
            page.account_username = active.username().to_string();
            page.account_offline = active.is_offline();
            page.active_username = active.username().to_string();
            page.is_offline = active.is_offline();
            page.load(state, ctx);
        }
    } else if let Some(active) = state.config.active_account_ref() {
        if active.username() != page.account_username || active.is_offline() != page.account_offline
        {
            page.account_username = active.username().to_string();
            page.account_offline = active.is_offline();
            page.active_username = active.username().to_string();
            page.is_offline = active.is_offline();
            page.uploaded_skin_url = None;
            page.capes.clear();
            page.selected_cape = None;
            page.load(state, ctx);
        }
    }

    if page.uploaded_skin_url != state.config.last_uploaded_skin_url {
        state.config.last_uploaded_skin_url = page.uploaded_skin_url.clone();
        state.save_config();
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
        "Customize your look and carry it across servers and modpacks.",
    );

    ui.add_space(2.0);

    if ui.available_width() >= WIDE_LAYOUT_MIN {
        let total_width = ui.available_width();
        let sidebar_width = (total_width * 0.35).clamp(SIDEBAR_MIN, SIDEBAR_MAX);
        let stage_width = (total_width - sidebar_width - COLUMN_GAP).max(360.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(stage_width, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_width(stage_width);
                    preview_card(&mut *page, &mut *state, ctx, ui);
                },
            );
            ui.add_space(COLUMN_GAP);
            ui.allocate_ui_with_layout(
                egui::vec2(sidebar_width, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_width(sidebar_width);
                    appearance_card(&mut *page, &mut *state, ctx, ui);
                },
            );
        });
    } else {
        preview_card(&mut *page, &mut *state, ctx, ui);
        ui.add_space(12.0);
        appearance_card(&mut *page, &mut *state, ctx, ui);
    }

    ui.add_space(14.0);

    if page.is_offline {
        delivery_card(&mut *page, &mut *state, ctx, ui);
    } else {
        official_card(&mut *page, &mut *state, ctx, ui);
    }
}

fn preview_card(
    page: &mut SkinsPage,
    state: &mut AppState,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Character preview")
                        .size(15.0)
                        .strong()
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("Drag to rotate · scroll to zoom")
                        .size(type_scale::CAPTION)
                        .color(TEXT2),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled_ui(!page.loading, |ui| {
                    if compact_secondary_button(ui, "Refresh").clicked() {
                        page.load(state, ctx);
                    }
                });
                if compact_secondary_button(ui, "↺ Reset view")
                    .on_hover_text("Recenter the camera and restore the default zoom.")
                    .clicked()
                {
                    page.viewer.reset();
                }
            });
        });

        ui.add_space(10.0);

        let p = palette(ctx);
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), PREVIEW_HEIGHT),
            egui::Sense::drag(),
        );

        paint_stage(ui, rect, page.viewer.zoom);

        if let Some(skin) = &page.skin {
            page.viewer
                .render(ui, rect, &response, skin, page.cape.as_ref());
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }

        if page.loading {
            paint_loading_overlay(ui, rect);
        }

        let model = if page.viewer.slim {
            "Slim model · 3px arms"
        } else {
            "Classic model · 4px arms"
        };
        ui.painter().text(
            rect.left_bottom() + egui::vec2(14.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            model,
            egui::FontId::proportional(type_scale::MICRO),
            MUTED,
        );

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if page.loading {
                let status = if page.status.is_empty() {
                    "Loading appearance…"
                } else {
                    page.status.as_str()
                };
                crate::ui::components::activity_indicator(ui, status);
            } else {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                let dot_color = if page.status_error {
                    DANGER
                } else if page.status.is_empty() {
                    MUTED
                } else {
                    p.accent
                };
                ui.painter().circle_filled(dot.center(), 2.5, dot_color);
                let label = if page.status.is_empty() {
                    "Appearance ready"
                } else {
                    page.status.as_str()
                };
                ui.label(
                    RichText::new(label)
                        .size(type_scale::CAPTION)
                        .color(if page.status_error { DANGER } else { TEXT2 }),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(url) = &page.cape_url {
                    if compact_secondary_button(ui, "Cape texture ↗").clicked() {
                        let _ = open::that(url);
                    }
                }
                if let Some(url) = &page.skin_url {
                    if compact_secondary_button(ui, "Skin texture ↗").clicked() {
                        let _ = open::that(url);
                    }
                }
            });
        });
    });
}

fn paint_stage(ui: &egui::Ui, rect: egui::Rect, zoom: f32) {
    let p = palette(ui.ctx());
    let painter = ui.painter().with_clip_rect(rect.shrink(1.0));
    let stage_fill = p.bg.lerp_to_gamma(p.elevated2, 0.6);
    let corner = CornerRadius::same(10);
    painter.rect_filled(rect, corner, stage_fill);

    let focus = egui::pos2(rect.center().x, rect.top() + rect.height() * 0.38);
    for (radius, alpha) in [(0.54_f32, 5_u8), (0.40, 6), (0.26, 7)] {
        painter.circle_filled(
            focus,
            rect.height() * radius,
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), alpha),
        );
    }

    let horizon = rect.top() + rect.height() * 0.72;
    let floor = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 1.0, horizon),
        egui::pos2(rect.right() - 1.0, rect.bottom() - 1.0),
    );
    painter.rect_filled(
        floor,
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: 10,
            se: 10,
        },
        Color32::from_black_alpha(32),
    );

    let grid = Stroke::new(
        1.0_f32,
        Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 12),
    );
    for offset in -8..=8 {
        let x = rect.center().x + offset as f32 * (rect.width() / 14.0);
        painter.line_segment(
            [
                egui::pos2(rect.center().x, horizon),
                egui::pos2(x, rect.bottom()),
            ],
            grid,
        );
    }
    for step in 1..=6 {
        let t = step as f32 / 6.0;
        let y = horizon + (rect.bottom() - horizon) * t.powf(1.8);
        painter.line_segment(
            [
                egui::pos2(rect.left() + 6.0, y),
                egui::pos2(rect.right() - 6.0, y),
            ],
            grid,
        );
    }
    painter.line_segment(
        [
            egui::pos2(rect.left() + 6.0, horizon),
            egui::pos2(rect.right() - 6.0, horizon),
        ],
        Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 26),
        ),
    );

    let scale = rect.height() / 42.0 * zoom;
    let feet = egui::pos2(rect.center().x, rect.center().y + 16.0 * scale);
    if feet.y < rect.bottom() - 2.0 {
        paint_ellipse(
            &painter,
            feet,
            3.4 * scale,
            0.82 * scale,
            Color32::from_black_alpha(28),
        );
        paint_ellipse(
            &painter,
            feet,
            2.4 * scale,
            0.58 * scale,
            Color32::from_black_alpha(34),
        );
    }

    painter.rect_stroke(
        rect,
        corner,
        Stroke::new(1.0_f32, p.border),
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            egui::pos2(rect.left() + 14.0, rect.top() + 1.0),
            egui::pos2(rect.right() - 14.0, rect.top() + 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 8)),
    );
}

fn paint_ellipse(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius_x: f32,
    radius_y: f32,
    color: Color32,
) {
    let points = (0..32)
        .map(|index| {
            let angle = index as f32 / 32.0 * std::f32::consts::TAU;
            center + egui::vec2(angle.cos() * radius_x, angle.sin() * radius_y)
        })
        .collect();
    painter.add(egui::epaint::PathShape::convex_polygon(
        points,
        color,
        Stroke::NONE,
    ));
}

fn paint_loading_overlay(ui: &egui::Ui, rect: egui::Rect) {
    let p = palette(ui.ctx());
    let center = rect.center();
    ui.painter()
        .rect_filled(rect, 10, Color32::from_black_alpha(52));

    let angle_offset = ui.input(|input| input.time) as f32 * 4.0;
    for index in 0..8 {
        let angle = angle_offset + index as f32 * std::f32::consts::TAU / 8.0;
        let position = center - egui::vec2(0.0, 24.0) + egui::vec2(angle.cos(), angle.sin()) * 7.0;
        let alpha = 0.2 + 0.8 * (index as f32 / 8.0);
        ui.painter()
            .circle_filled(position, 1.8, p.accent.gamma_multiply(alpha));
    }

    ui.painter().text(
        center + egui::vec2(0.0, 2.0),
        egui::Align2::CENTER_CENTER,
        "Loading appearance…",
        egui::FontId::proportional(type_scale::LABEL),
        p.accent,
    );
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(50));
}

fn appearance_card(
    page: &mut SkinsPage,
    state: &mut AppState,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    card_frame(ui, |ui| {
        identity_block(&mut *page, &mut *state, ctx, ui);
        divider(ui);

        section_label(ui, "MODEL");
        ui.add_space(8.0);
        model_picker(ui, &mut page.viewer.slim);

        ui.add_space(14.0);
        section_label(ui, "LAYERS");
        ui.add_space(6.0);
        let online = !page.is_offline;
        layer_toggle(
            ui,
            "outer-layer",
            "Outer layer",
            "Hat, jacket and sleeves",
            true,
            &mut page.viewer.overlay,
        );
        layer_toggle(
            ui,
            "cape-layer",
            "Cape",
            "Show your equipped cape",
            online,
            &mut page.viewer.show_cape,
        );
        layer_toggle(
            ui,
            "elytra-layer",
            "Elytra",
            "Render the cape as elytra",
            online,
            &mut page.viewer.elytra,
        );
        if !online {
            ui.add_space(2.0);
            ui.label(
                RichText::new("Capes and elytra need a Microsoft profile.")
                    .size(type_scale::MICRO)
                    .color(MUTED),
            );
        }

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            section_label(ui, "PRESETS");
            ui.add_space(4.0);
            badge(ui, "Preview only");
        });
        ui.add_space(8.0);
        preset_row(&mut *page, ctx, ui);
        ui.add_space(6.0);

        if page.is_offline {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Set as my skin")
                        .size(type_scale::MICRO)
                        .color(MUTED),
                );
                if compact_secondary_button(ui, "Steve")
                    .on_hover_text("Replace your skin file with the default Steve skin.")
                    .clicked()
                {
                    apply_preset(&mut *page, &mut *state, ctx, false);
                }
                if compact_secondary_button(ui, "Alex")
                    .on_hover_text("Replace your skin file with the default Alex skin.")
                    .clicked()
                {
                    apply_preset(&mut *page, &mut *state, ctx, true);
                }
            });
        } else {
            ui.label(
                RichText::new("Presets are preview-only here. Use the official skin upload below.")
                    .size(type_scale::MICRO)
                    .color(MUTED),
            );
        }

        if page.is_offline {
            divider(ui);
            file_actions(&mut *page, &mut *state, ctx, ui);
        } else {
            divider(ui);
            ui.label(
                RichText::new("Official skin and cape controls are available below.")
                    .size(type_scale::CAPTION)
                    .color(TEXT2),
            );
        }
    });
}

fn identity_block(
    page: &mut SkinsPage,
    state: &mut AppState,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    ui.horizontal(|ui| {
        let (avatar, _) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::hover());
        if page.is_offline {
            draw_cute_avatar(ui.painter(), avatar, &page.active_username, false);
        } else if let Some(profile) = state
            .config
            .microsoft_profile
            .as_ref()
            .filter(|profile| profile.username == page.active_username)
        {
            draw_avatar(
                ui.painter(),
                avatar,
                &page.active_username,
                &profile.uuid.to_string(),
            );
        } else {
            draw_cute_avatar(ui.painter(), avatar, &page.active_username, true);
        }

        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(&page.active_username)
                        .size(17.0)
                        .strong()
                        .color(TEXT),
                )
                .truncate(),
            );
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if page.is_offline {
                    badge(ui, "Offline");
                    if page.has_custom_skin {
                        badge_accent(ui, "Custom skin");
                    }
                } else {
                    badge_ok(ui, "Microsoft");
                }
                ui.label(
                    RichText::new(if page.viewer.slim {
                        "Slim model"
                    } else {
                        "Classic model"
                    })
                    .size(type_scale::MICRO)
                    .color(MUTED),
                );
            });
        });
    });

    if state.config.profile.is_some() && state.config.microsoft_profile.is_some() {
        ui.add_space(12.0);
        section_label(ui, "PREVIEW PROFILE");
        ui.add_space(5.0);

        let selected = usize::from(!page.is_offline);
        if let Some(choice) = profile_switch(ui, selected) {
            match (choice, page.is_offline) {
                (0, false) => {
                    if let Some(profile) = &state.config.profile {
                        page.active_username = profile.username.clone();
                        page.is_offline = true;
                        page.load(state, ctx);
                    }
                }
                (1, true) => {
                    if let Some(profile) = &state.config.microsoft_profile {
                        page.active_username = profile.username.clone();
                        page.is_offline = false;
                        page.load(state, ctx);
                    }
                }
                _ => {}
            }
        }
    }
}

fn divider(ui: &mut egui::Ui) {
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(type_scale::MICRO)
            .strong()
            .color(MUTED),
    );
}

fn profile_switch(ui: &mut egui::Ui, selected: usize) -> Option<usize> {
    let p = palette(ui.ctx());
    let gap = 4.0;
    let width = ((ui.available_width() - gap) / 2.0).max(80.0);
    let mut picked = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (index, label) in ["Offline", "Microsoft"].iter().enumerate() {
            let active = selected == index;
            let fill = if active { p.accent } else { p.elevated2 };
            let text_color = if active { p.accent_text } else { TEXT2 };
            let stroke = if active {
                Stroke::NONE
            } else {
                Stroke::new(1.0_f32, p.border)
            };
            let response = ui
                .add_sized(
                    [width, 30.0],
                    egui::Button::new(
                        RichText::new(*label)
                            .size(type_scale::CAPTION)
                            .strong()
                            .color(text_color),
                    )
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(metrics::PILL_RADIUS)),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() {
                picked = Some(index);
            }
        }
    });

    picked
}

fn model_picker(ui: &mut egui::Ui, slim: &mut bool) {
    let gap = 6.0;
    let width = ((ui.available_width() - gap) / 2.0).max(80.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        if model_tile(ui, width, "Classic", "4 px arms", !*slim).clicked() {
            *slim = false;
        }
        if model_tile(ui, width, "Slim", "3 px arms", *slim).clicked() {
            *slim = true;
        }
    });
}

fn model_tile(
    ui: &mut egui::Ui,
    width: f32,
    title: &str,
    subtitle: &str,
    selected: bool,
) -> egui::Response {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 50.0), egui::Sense::click());

    let fill = if selected {
        p.accent
    } else if response.hovered() {
        p.hover
    } else {
        p.elevated2
    };
    let stroke = if selected {
        Stroke::NONE
    } else {
        Stroke::new(1.0_f32, p.border)
    };
    let title_color = if selected { p.accent_text } else { TEXT };
    let subtitle_color = if selected {
        Color32::from_rgba_unmultiplied(
            p.accent_text.r(),
            p.accent_text.g(),
            p.accent_text.b(),
            175,
        )
    } else {
        MUTED
    };

    ui.painter()
        .rect(rect, 8, fill, stroke, egui::StrokeKind::Inside);
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, 10.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(type_scale::LABEL),
        title_color,
    );
    ui.painter().text(
        rect.left_top() + egui::vec2(12.0, 27.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        egui::FontId::proportional(type_scale::MICRO),
        subtitle_color,
    );

    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            title,
        )
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn layer_toggle(
    ui: &mut egui::Ui,
    id_salt: &str,
    title: &str,
    hint: &str,
    enabled: bool,
    value: &mut bool,
) {
    let p = palette(ui.ctx());
    let id = ui.make_persistent_id(("skin-layer", id_salt));
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.0), sense);

    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, 6, p.hover);
    }

    let disabled_dim =
        |color: Color32| Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 120);
    let title_color = if enabled { TEXT } else { disabled_dim(TEXT) };
    let hint_color = if enabled { MUTED } else { disabled_dim(MUTED) };

    ui.painter().text(
        rect.left_center() + egui::vec2(0.0, -7.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(type_scale::LABEL),
        title_color,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(0.0, 8.0),
        egui::Align2::LEFT_CENTER,
        hint,
        egui::FontId::proportional(type_scale::MICRO),
        hint_color,
    );

    let switch = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 21.0, rect.center().y),
        egui::vec2(36.0, 20.0),
    );
    let t = ui
        .ctx()
        .animate_bool_with_time(id.with("on"), enabled && *value, 0.12);
    let track = if enabled {
        p.elevated2.lerp_to_gamma(p.accent, t)
    } else {
        p.elevated2
    };
    let track_stroke = if enabled {
        p.border.lerp_to_gamma(p.accent, t * 0.7)
    } else {
        p.border
    };
    ui.painter().rect_filled(switch, 10, track);
    ui.painter().rect_stroke(
        switch,
        10,
        Stroke::new(1.0_f32, track_stroke),
        egui::StrokeKind::Inside,
    );

    let knob_x = egui::lerp((switch.left() + 10.0)..=(switch.right() - 10.0), t);
    let knob_color = if !enabled {
        disabled_dim(TEXT2)
    } else if t > 0.5 {
        p.accent_text
    } else {
        TEXT2
    };
    ui.painter()
        .circle_filled(egui::pos2(knob_x, switch.center().y), 6.5, knob_color);

    if enabled {
        if response.clicked() {
            *value = !*value;
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            enabled,
            enabled && *value,
            title,
        )
    });

    if !enabled {
        response.on_hover_text("Requires a Microsoft profile.");
    }
}

fn preset_row(page: &mut SkinsPage, ctx: &egui::Context, ui: &mut egui::Ui) {
    let gap = 6.0;
    let width = ((ui.available_width() - 2.0 * gap) / 3.0).max(72.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        if preset_tile(
            ui,
            width,
            "Steve",
            "Classic",
            SkinPreset::Steve,
            page.active_preset == SkinPreset::Steve,
        )
        .clicked()
        {
            preview_preset(page, ctx, SkinPreset::Steve);
        }
        if preset_tile(
            ui,
            width,
            "Alex",
            "Slim",
            SkinPreset::Alex,
            page.active_preset == SkinPreset::Alex,
        )
        .clicked()
        {
            preview_preset(page, ctx, SkinPreset::Alex);
        }
        if preset_tile(ui, width, "Shuffle", "Flip preset", SkinPreset::None, false).clicked() {
            let next = if page.active_preset == SkinPreset::Steve {
                SkinPreset::Alex
            } else {
                SkinPreset::Steve
            };
            preview_preset(page, ctx, next);
        }
    });
}

fn preset_tile(
    ui: &mut egui::Ui,
    width: f32,
    title: &str,
    subtitle: &str,
    kind: SkinPreset,
    selected: bool,
) -> egui::Response {
    let p = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 78.0), egui::Sense::click());

    let fill = if selected {
        p.elevated2.lerp_to_gamma(p.accent, 0.14)
    } else if response.hovered() {
        p.hover
    } else {
        p.elevated2
    };
    let stroke = if selected {
        Stroke::new(1.5_f32, p.accent)
    } else {
        Stroke::new(1.0_f32, p.border)
    };
    ui.painter()
        .rect(rect, 8, fill, stroke, egui::StrokeKind::Inside);

    let art = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 25.0),
        egui::vec2(24.0, 24.0),
    );
    draw_preset_art(ui.painter(), art, kind);

    ui.painter().text(
        egui::pos2(rect.center().x, rect.top() + 47.0),
        egui::Align2::CENTER_TOP,
        title,
        egui::FontId::proportional(type_scale::CAPTION),
        if selected { p.accent } else { TEXT },
    );
    ui.painter().text(
        egui::pos2(rect.center().x, rect.top() + 62.0),
        egui::Align2::CENTER_TOP,
        subtitle,
        egui::FontId::proportional(type_scale::MICRO),
        MUTED,
    );

    if selected {
        ui.painter().circle_filled(
            egui::pos2(rect.right() - 10.0, rect.top() + 10.0),
            3.0,
            p.accent,
        );
    }

    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            title,
        )
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn draw_preset_art(painter: &egui::Painter, rect: egui::Rect, kind: SkinPreset) {
    let p = palette(painter.ctx());
    match kind {
        SkinPreset::Steve | SkinPreset::Alex => {
            let (hair, skin, pupil, mouth) = match kind {
                SkinPreset::Steve => (
                    Color32::from_rgb(59, 42, 26),
                    Color32::from_rgb(198, 138, 106),
                    Color32::from_rgb(74, 95, 165),
                    Color32::from_rgb(138, 90, 68),
                ),
                _ => (
                    Color32::from_rgb(193, 120, 55),
                    Color32::from_rgb(241, 201, 165),
                    Color32::from_rgb(91, 158, 90),
                    Color32::from_rgb(181, 122, 99),
                ),
            };
            let pattern = [
                "HHHHHHHH", "HHHHHHHH", "HSSSSSSH", "SSSSSSSS", "SWBSSBWS", "SSSSSSSS", "SSSMMSSS",
                "SSMMMMSS",
            ];
            let unit = rect.width() / 8.0;
            for (row, line) in pattern.iter().enumerate() {
                for (column, symbol) in line.chars().enumerate() {
                    let color = match symbol {
                        'H' => hair,
                        'W' => Color32::from_rgb(238, 238, 238),
                        'B' => pupil,
                        'M' => mouth,
                        _ => skin,
                    };
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(
                                rect.left() + column as f32 * unit,
                                rect.top() + row as f32 * unit,
                            ),
                            egui::vec2(unit + 0.3, unit + 0.3),
                        ),
                        0,
                        color,
                    );
                }
            }
            painter.rect_stroke(
                rect,
                2,
                Stroke::new(1.0_f32, Color32::from_black_alpha(70)),
                egui::StrokeKind::Inside,
            );
        }
        SkinPreset::None => {
            let die = egui::Rect::from_center_size(
                rect.center(),
                egui::vec2(rect.width() * 0.82, rect.height() * 0.82),
            );
            painter.rect_filled(die, 5, p.elevated);
            painter.rect_stroke(
                die,
                5,
                Stroke::new(1.2_f32, p.accent),
                egui::StrokeKind::Inside,
            );
            for (fx, fy) in [(0.28_f32, 0.28_f32), (0.5, 0.5), (0.72, 0.72)] {
                painter.circle_filled(
                    egui::pos2(die.left() + die.width() * fx, die.top() + die.height() * fy),
                    1.7,
                    p.accent,
                );
            }
        }
    }
}

fn preview_preset(page: &mut SkinsPage, ctx: &egui::Context, preset: SkinPreset) {
    let (image, slim, label) = match preset {
        SkinPreset::Steve => (default_steve_skin(), false, "Steve"),
        SkinPreset::Alex => (default_alex_skin(), true, "Alex"),
        SkinPreset::None => return,
    };

    page.skin = Some(ctx.load_texture("skins-skin", image, TextureOptions::NEAREST));
    page.cape = None;
    page.skin_url = None;
    page.cape_url = None;
    page.viewer.slim = slim;
    page.active_preset = preset;
    page.has_custom_skin = false;
    page.status = format!("Previewing {label} preset.");
    page.status_error = false;
}

fn apply_preset(page: &mut SkinsPage, state: &mut AppState, ctx: &egui::Context, slim: bool) {
    let skins_dir = state.paths.root().join("skins");
    let instances = state.instances.clone();
    let username = page.active_username.clone();

    page.viewer.slim = slim;
    page.active_preset = if slim {
        SkinPreset::Alex
    } else {
        SkinPreset::Steve
    };
    page.uploaded_skin_url = None;

    let status = if slim {
        "Saving Alex preset…"
    } else {
        "Saving Steve preset…"
    };
    page.spawn(state, ctx, status, async move {
        match save_offline_preset(&skins_dir, &username, slim) {
            Ok(image) => {
                let _ = instances.sync_offline_skin_all(&skins_dir, &username);
                Message::LocalSkin(image, true)
            }
            Err(error) => Message::Error(error),
        }
    });
}

fn file_actions(
    page: &mut SkinsPage,
    state: &mut AppState,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    ui.horizontal(|ui| {
        section_label(ui, "SKIN FILE");
        if page.has_custom_skin {
            badge_accent(ui, "Custom");
        } else {
            badge(ui, "Default");
        }
    });

    ui.add_space(7.0);

    if button_sized(
        ui,
        "Import custom skin (.png)",
        Tone::Primary,
        egui::vec2(ui.available_width(), metrics::BUTTON_H),
    )
    .clicked()
    {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Minecraft skin", &["png"])
            .pick_file()
        {
            let skins_dir = state.paths.root().join("skins");
            let instances = state.instances.clone();
            let username = page.active_username.clone();

            page.spawn(state, ctx, "Importing custom skin…", async move {
                match save_offline_skin(&skins_dir, &username, &path) {
                    Ok(image) => {
                        let _ = instances.sync_offline_skin_all(&skins_dir, &username);
                        Message::LocalSkin(image, true)
                    }
                    Err(error) => Message::Error(error),
                }
            });
        }
    }

    ui.add_space(6.0);

    ui.horizontal(|ui| {
        if content_secondary_button(ui, "Upload to server").clicked() {
            page.trigger_upload(state, ctx);
        }
        if content_secondary_button(ui, "Sync to mods")
            .on_hover_text("Copy your skin into every instance's CustomSkinLoader folder.")
            .clicked()
        {
            page.trigger_sync_mods(state);
        }
    });

    if page.has_custom_skin {
        ui.add_space(5.0);
        if compact_danger_button(ui, "Reset to default skin").clicked() {
            reset_custom_skin(page, state, ctx);
        }
    }
}

fn delivery_card(
    page: &mut SkinsPage,
    state: &mut AppState,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Skin delivery")
                        .size(15.0)
                        .strong()
                        .color(TEXT),
                );
                ui.label(
                    RichText::new(
                        "Keep the same look on plugin servers and in every modded instance.",
                    )
                    .size(type_scale::CAPTION)
                    .color(TEXT2),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                badge(ui, "CustomSkinLoader");
                badge(ui, "SkinsRestorer");
            });
        });

        ui.add_space(12.0);

        let inner_width = ui.available_width();
        if inner_width >= 760.0 {
            let column_width = (inner_width - 24.0) / 2.0;
            ui.horizontal_top(|ui| {
                column(ui, column_width, |ui| {
                    skinsrestorer_column(&mut *page, &mut *state, ctx, ui);
                });
                ui.add_space(24.0);
                column(ui, column_width, |ui| {
                    mod_loader_column(&mut *page, &mut *state, ui);
                });
            });
        } else {
            skinsrestorer_column(&mut *page, &mut *state, ctx, ui);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);
            mod_loader_column(&mut *page, &mut *state, ui);
        }
    });
}

fn column(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            ui.set_width(width);
            add(ui);
        },
    );
}

fn skinsrestorer_column(
    page: &mut SkinsPage,
    state: &mut AppState,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("SkinsRestorer")
                .size(type_scale::BODY)
                .strong()
                .color(TEXT),
        );
        badge(ui, "Server plugin");
    });
    ui.label(
        RichText::new(
            "Upload your skin once, then apply it with a direct URL command. Every player on a SkinsRestorer server can see it.",
        )
        .size(type_scale::MICRO)
        .color(TEXT2),
    );

    ui.add_space(10.0);

    if let Some(url) = page.uploaded_skin_url.clone() {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("HOSTED TEXTURE")
                    .size(type_scale::MICRO)
                    .strong()
                    .color(MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if compact_secondary_button(ui, "Open ↗").clicked() {
                    let _ = open::that(&url);
                }
            });
        });
        ui.add_space(3.0);
        ui.add(egui::Label::new(
            RichText::new(elide(&url, 54))
                .monospace()
                .size(type_scale::MICRO)
                .color(TEXT2),
        ))
        .on_hover_text(&url);

        ui.add_space(7.0);
        let command = build_skinsrestorer_command(&url, page.viewer.slim);
        command_box(
            ui,
            state,
            "DIRECT URL COMMAND · RECOMMENDED",
            &command,
            "Copied /skin url command",
        );

        ui.add_space(5.0);
        if compact_secondary_button(ui, "Clear hosted URL").clicked() {
            page.uploaded_skin_url = None;
            state.config.last_uploaded_skin_url = None;
            state.save_config();
            page.status = "Hosted skin URL cleared.".into();
            page.status_error = false;
        }
    } else {
        subtle_panel(ui, |ui| {
            ui.label(
                RichText::new("No hosted URL yet")
                    .size(type_scale::CAPTION)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new(
                    "Upload once to get the direct texture URL used by the /skin url command.",
                )
                .size(type_scale::MICRO)
                .color(MUTED),
            );
            ui.add_space(7.0);
            if content_secondary_button(ui, "Upload now").clicked() {
                page.trigger_upload(state, ctx);
            }
        });
    }

    ui.add_space(10.0);
    egui::CollapsingHeader::new(
        RichText::new("Username lookup · Mojang profiles only")
            .size(type_scale::MICRO)
            .color(TEXT2),
    )
    .id_salt("skins-name-lookup")
    .default_open(false)
    .show(ui, |ui| {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Only resolves if this exact name is registered on Mojang with a skin. For offline profiles, use the direct URL command above instead.",
            )
            .size(type_scale::MICRO)
            .color(MUTED),
        );
        ui.add_space(5.0);
        command_box(
            ui,
            state,
            "LOOKUP BY NAME",
            &format!("/skin {}", page.active_username),
            "Copied /skin command",
        );
    });

    ui.add_space(8.0);
    endpoint_section(page, state, ui);
}

fn endpoint_section(page: &mut SkinsPage, state: &mut AppState, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(
        RichText::new("Advanced · custom upload endpoint")
            .size(type_scale::MICRO)
            .color(TEXT2),
    )
    .id_salt("skins-upload-endpoint")
    .default_open(false)
    .show(ui, |ui| {
        ui.add_space(4.0);
        let hint = state.config.effective_skin_server_url().to_string();
        let response = ui.add(
            egui::TextEdit::singleline(&mut page.custom_server_input)
                .hint_text(&hint)
                .desired_width(ui.available_width()),
        );
        ui.add_space(5.0);

        let mut save = response.lost_focus()
            && page.custom_server_input.trim() != state.config.skin_server_url.trim();
        let mut reset = false;
        ui.horizontal(|ui| {
            if compact_secondary_button(ui, "Save").clicked() {
                save = true;
            }
            if compact_secondary_button(ui, "Use default").clicked() {
                reset = true;
            }
        });

        if reset {
            page.custom_server_input.clear();
            state.config.skin_server_url.clear();
            state.save_config();
            state.notify("Default skin server restored");
        } else if save {
            let endpoint = page.custom_server_input.trim().to_string();
            page.custom_server_input = endpoint.clone();
            state.config.skin_server_url = endpoint;
            state.save_config();
            state.notify("Skin upload endpoint saved");
        }

        ui.label(
            RichText::new("Leave blank to use the built-in skin server.")
                .size(type_scale::MICRO)
                .color(MUTED),
        );
    });
}

fn command_box(ui: &mut egui::Ui, state: &mut AppState, label: &str, command: &str, note: &str) {
    ui.label(
        RichText::new(label)
            .size(type_scale::MICRO)
            .strong()
            .color(MUTED),
    );
    ui.add_space(3.0);

    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let copy_width = 56.0;
                let text_width = (ui.available_width() - copy_width - 8.0).max(80.0);
                ui.add_sized(
                    [text_width, 22.0],
                    egui::Label::new(
                        RichText::new(command)
                            .monospace()
                            .size(type_scale::MICRO)
                            .color(p.accent),
                    )
                    .truncate(),
                )
                .on_hover_text(command);
                if compact_secondary_button(ui, "Copy").clicked() {
                    ui.ctx().copy_text(command.to_string());
                    state.notify(note);
                }
            });
        });
}

fn subtle_panel(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

fn code_path(ui: &mut egui::Ui, label: &str, path: &str) {
    ui.label(
        RichText::new(label)
            .size(type_scale::MICRO)
            .strong()
            .color(MUTED),
    );
    ui.add_space(3.0);
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.elevated2)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(path)
                        .monospace()
                        .size(type_scale::MICRO)
                        .color(TEXT2),
                )
                .truncate(),
            )
            .on_hover_text(path);
        });
}

fn mod_loader_column(page: &mut SkinsPage, state: &mut AppState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("CustomSkinLoader")
                .size(type_scale::BODY)
                .strong()
                .color(TEXT),
        );
        badge(ui, "Mod loader");
    });
    ui.label(
        RichText::new("Copies your skin into every instance and writes the loader config.")
            .size(type_scale::MICRO)
            .color(TEXT2),
    );

    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        badge(ui, &format!("{} instances", state.instance_list.len()));
        badge(ui, "Fabric / Forge / NeoForge / Quilt");
    });

    let total_instances = state.instance_list.len();
    let now = ui.input(|input| input.time);
    let scan_stale = page
        .mod_scan
        .as_ref()
        .is_none_or(|(scanned_at, _)| now - *scanned_at > 1.5);
    if scan_stale {
        let missing = state
            .instance_list
            .iter()
            .filter(|instance| !instance_has_skin_mod(&state.instances.game_dir(&instance.id)))
            .map(|instance| instance.name.clone())
            .collect();
        page.mod_scan = Some((now, missing));
    }
    let missing_mods = page
        .mod_scan
        .as_ref()
        .map(|(_, missing)| missing.clone())
        .unwrap_or_default();

    if total_instances > 0 {
        ui.add_space(6.0);
        if missing_mods.is_empty() {
            badge_ok(ui, "CustomSkinLoader detected in all instances");
        } else {
            ui.horizontal_wrapped(|ui| {
                badge_warning(
                    ui,
                    &format!("Skin mod missing in {} instance(s)", missing_mods.len()),
                );
                if compact_secondary_button(ui, "Get CustomSkinLoader ↗").clicked() {
                    let _ = open::that("https://modrinth.com/mod/customskinloader");
                }
            });
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(
                    RichText::new(format!(
                        "The PNG is still copied, but Minecraft only renders it where CustomSkinLoader or OfflineSkins is installed. Missing in: {}",
                        elide(&missing_mods.join(", "), 52)
                    ))
                    .size(type_scale::MICRO)
                    .color(MUTED),
                )
                .wrap(),
            )
            .on_hover_text(missing_mods.join("\n"));
        }
    }

    ui.add_space(7.0);
    code_path(
        ui,
        "TARGET PATH",
        &format!(
            "CustomSkinLoader/LocalSkin/skins/{}.png",
            page.active_username
        ),
    );

    ui.add_space(8.0);
    let has_instances = !state.instance_list.is_empty();
    ui.add_enabled_ui(has_instances, |ui| {
        if content_secondary_button(ui, "Sync all instances").clicked() {
            page.trigger_sync_mods(state);
        }
    });
    if !has_instances {
        ui.label(
            RichText::new("Create an instance first to sync skins.")
                .size(type_scale::MICRO)
                .color(MUTED),
        );
    } else {
        ui.label(
            RichText::new("Runs automatically after importing a skin or applying a preset.")
                .size(type_scale::MICRO)
                .color(MUTED),
        );
    }
}

fn official_card(
    page: &mut SkinsPage,
    state: &mut AppState,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Official skin & capes")
                        .size(15.0)
                        .strong()
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("Managed by your Microsoft account.")
                        .size(type_scale::CAPTION)
                        .color(TEXT2),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                badge_ok(ui, "Microsoft");
            });
        });

        ui.add_space(12.0);

        let token = state
            .config
            .microsoft_profile
            .as_ref()
            .map(|profile| profile.access_token.clone())
            .filter(|token| !token.is_empty());

        let Some(token) = token else {
            subtle_panel(ui, |ui| {
                ui.label(
                    RichText::new(
                        "Sign in with Microsoft in Accounts to manage your official skin and capes.",
                    )
                    .size(type_scale::CAPTION)
                    .color(TEXT2),
                );
                ui.add_space(7.0);
                if content_secondary_button(ui, "Go to Accounts").clicked() {
                    state.set_page(Page::Accounts);
                }
            });
            return;
        };

        let inner_width = ui.available_width();
        if inner_width >= 720.0 {
            let column_width = (inner_width - 24.0) / 2.0;
            ui.horizontal_top(|ui| {
                column(ui, column_width, |ui| {
                    official_skin_column(&mut *page, &mut *state, ctx, ui, &token);
                });
                ui.add_space(24.0);
                column(ui, column_width, |ui| {
                    cape_column(&mut *page, &mut *state, ctx, ui, &token);
                });
            });
        } else {
            official_skin_column(&mut *page, &mut *state, ctx, ui, &token);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);
            cape_column(&mut *page, &mut *state, ctx, ui, &token);
        }
    });
}

fn official_skin_column(
    page: &mut SkinsPage,
    state: &mut AppState,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    token: &str,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Mojang skin")
                .size(type_scale::BODY)
                .strong()
                .color(TEXT),
        );
        badge(ui, if page.viewer.slim { "Slim" } else { "Classic" });
    });
    ui.label(
        RichText::new(
            "Upload a 64×64 PNG to your Microsoft profile. The selected model is applied with it.",
        )
        .size(type_scale::MICRO)
        .color(TEXT2),
    );

    ui.add_space(10.0);
    if button_sized(
        ui,
        "Upload skin (.png)",
        Tone::Primary,
        egui::vec2(ui.available_width(), metrics::BUTTON_H),
    )
    .clicked()
    {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Minecraft skin", &["png"])
            .pick_file()
        {
            let http = state.http.clone();
            let token = token.to_string();
            let slim = page.viewer.slim;

            page.spawn(state, ctx, "Uploading skin…", async move {
                let result = async {
                    let bytes = tokio::fs::read(path)
                        .await
                        .map_err(|error| error.to_string())?;
                    upload_skin(&http, &token, bytes, slim).await
                }
                .await;

                match result {
                    Ok(()) => Message::Applied("Skin uploaded to Mojang profile.".into()),
                    Err(error) => Message::Error(error),
                }
            });
        }
    }

    ui.add_space(6.0);
    ui.label(
        RichText::new("Official skin changes appear the next time the game loads your profile.")
            .size(type_scale::MICRO)
            .color(MUTED),
    );
}

fn cape_column(
    page: &mut SkinsPage,
    state: &mut AppState,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    token: &str,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Official capes")
                .size(type_scale::BODY)
                .strong()
                .color(TEXT),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_enabled_ui(!page.loading, |ui| {
                if content_secondary_button(ui, "Load capes").clicked() {
                    let http = state.http.clone();
                    let token = token.to_string();
                    page.spawn(state, ctx, "Loading capes…", async move {
                        match load_capes(&http, &token).await {
                            Ok(capes) => Message::Capes(capes),
                            Err(error) => Message::Error(error),
                        }
                    });
                }
            });
        });
    });
    ui.label(
        RichText::new("Load the capes owned by your Microsoft account, then apply one.")
            .size(type_scale::MICRO)
            .color(TEXT2),
    );

    ui.add_space(10.0);

    if page.capes.is_empty() {
        subtle_panel(ui, |ui| {
            ui.label(
                RichText::new("No capes loaded yet")
                    .size(type_scale::CAPTION)
                    .strong()
                    .color(TEXT),
            );
            ui.label(
                RichText::new("Select “Load capes” to fetch the capes on your account.")
                    .size(type_scale::MICRO)
                    .color(MUTED),
            );
        });
        return;
    }

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
                ui.selectable_value(&mut page.selected_cape, Some(cape.id.clone()), label(cape));
            }
        });

    ui.add_space(7.0);
    let can_apply = !page.loading && page.selected_cape.is_some();
    let mut apply = false;
    ui.add_enabled_ui(can_apply, |ui| {
        if button_sized(
            ui,
            "Apply cape",
            Tone::Primary,
            egui::vec2(ui.available_width(), metrics::BUTTON_H),
        )
        .clicked()
        {
            apply = true;
        }
    });

    if apply {
        if let Some(cape_id) = page.selected_cape.clone() {
            let http = state.http.clone();
            let token = token.to_string();

            page.spawn(state, ctx, "Applying cape…", async move {
                match apply_cape(&http, &token, &cape_id).await {
                    Ok(()) => Message::Applied("Cape applied.".into()),
                    Err(error) => Message::Error(error),
                }
            });
        }
    }
}

fn reset_custom_skin(page: &mut SkinsPage, state: &mut AppState, ctx: &egui::Context) {
    let skins_dir = state.paths.root().join("skins");
    remove_offline_skin(&skins_dir, &page.active_username);
    let _ = state
        .instances
        .sync_offline_skin_all(&skins_dir, &page.active_username);

    page.has_custom_skin = false;
    page.active_preset = SkinPreset::None;
    page.uploaded_skin_url = None;
    page.status = "Custom skin removed. Default appearance restored.".into();
    page.status_error = false;
    state.config.last_uploaded_skin_url = None;
    state.save_config();
    state.notify("Custom skin removed");
    page.load(state, ctx);
}

impl SkinsPage {
    fn spawn<F>(&mut self, state: &AppState, ctx: &egui::Context, status: &str, task: F)
    where
        F: Future<Output = Message> + Send + 'static,
    {
        self.loading = true;
        self.status = status.to_string();
        self.status_error = false;

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
                    self.status_error = false;
                }
                Message::LocalSkin(image, is_custom) => {
                    let options = TextureOptions::NEAREST;

                    self.skin = Some(ctx.load_texture("skins-skin", image, options));
                    self.cape = None;
                    self.skin_url = None;
                    self.cape_url = None;
                    self.has_custom_skin = is_custom;
                    self.uploaded_skin_url = None;
                    self.active_preset = SkinPreset::None;
                    self.status = if is_custom {
                        "Custom local skin active.".into()
                    } else {
                        "Local skin loaded.".into()
                    };
                    self.status_error = false;
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
                    self.status_error = false;
                }
                Message::Capes(capes) => {
                    self.selected_cape = capes
                        .iter()
                        .find(|cape| cape.state == "ACTIVE")
                        .map(|cape| cape.id.clone());

                    self.capes = capes;
                    self.status = "Capes loaded.".into();
                    self.status_error = false;
                }
                Message::UploadedSkinUrl(url) => {
                    self.uploaded_skin_url = Some(url);
                    self.status = "Skin uploaded to skin server. /skin command ready!".into();
                    self.status_error = false;
                }
                Message::Applied(message) => {
                    self.status = message;
                    self.status_error = false;
                }
                Message::Error(error) => {
                    self.status = error;
                    self.status_error = true;
                }
            }
        }
    }

    fn trigger_upload(&mut self, state: &AppState, ctx: &egui::Context) {
        let skins_dir = state.paths.root().join("skins");
        let path = skins_dir.join(format!("{}.png", self.active_username));
        let bytes_res = if path.is_file() {
            std::fs::read(&path).map_err(|e| e.to_string())
        } else {
            let (_, is_alex) = default_skin_for_player(&self.active_username);
            let _ = save_offline_preset(&skins_dir, &self.active_username, is_alex);
            std::fs::read(&path).map_err(|e| e.to_string())
        };

        match bytes_res {
            Ok(bytes) => {
                let http = state.http.clone();
                let endpoint = if self.custom_server_input.trim().is_empty() {
                    state.config.effective_skin_server_url().to_string()
                } else {
                    self.custom_server_input.trim().to_string()
                };
                let username = self.active_username.clone();
                let slim = self.viewer.slim;

                self.spawn(state, ctx, "Uploading skin to skin server...", async move {
                    match upload_offline_skin(&http, &endpoint, &username, bytes, slim).await {
                        Ok(url) => Message::UploadedSkinUrl(url),
                        Err(err) => Message::Error(err),
                    }
                });
            }
            Err(err) => {
                self.status = format!("Could not prepare skin for upload: {err}");
                self.status_error = true;
            }
        }
    }

    fn trigger_sync_mods(&mut self, state: &mut AppState) {
        let skins_dir = state.paths.root().join("skins");
        let path = skins_dir.join(format!("{}.png", self.active_username));
        if !path.is_file() {
            let (_, is_alex) = default_skin_for_player(&self.active_username);
            let _ = save_offline_preset(&skins_dir, &self.active_username, is_alex);
        }
        let username = self.active_username.clone();
        match state.instances.sync_offline_skin_all(&skins_dir, &username) {
            Ok(count) => {
                self.status = format!("Synchronized skin to {count} instance(s) for mod loaders.");
                self.status_error = false;
                state.notify(format!("Synced skin to {count} instance(s)"));
            }
            Err(err) => {
                self.status = format!("Mod loader sync failed: {err}");
                self.status_error = true;
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
                    self.status_error = false;
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
        state
            .instance_list
            .push(crate::instance::config::InstanceConfig::new(
                "Test".into(),
                "1.21.1".into(),
                crate::instance::config::LoaderKind::Fabric,
                "0.16.0".into(),
            ));

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
    fn skins_page_renders_narrow_layout_without_panicking() {
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
                    egui::vec2(640.0, 900.0),
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
    fn skins_page_renders_for_microsoft_profile_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let paths = crate::storage::paths::MonoryxPaths::new(dir.path().to_path_buf());
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths);
        state.config.use_microsoft_auth = true;
        state.config.microsoft_profile = Some(crate::account::microsoft::MicrosoftProfile {
            username: "PlayerMS".into(),
            uuid: uuid::Uuid::new_v4(),
            access_token: "test-token".into(),
            refresh_token: "test-refresh".into(),
            expires_at: 0,
        });
        state.page = Page::Skins;

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

        assert_eq!(state.page, Page::Skins);
    }

    #[test]
    fn skins_page_redirects_without_active_account() {
        let dir = tempfile::tempdir().unwrap();
        let paths = crate::storage::paths::MonoryxPaths::new(dir.path().to_path_buf());
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut state = AppState::new_for_preview(&cc, paths);
        state.config.use_microsoft_auth = true;
        state.page = Page::Skins;

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut state, ctx, ui);
            });
        });

        assert_eq!(state.page, Page::Home);
    }
}
