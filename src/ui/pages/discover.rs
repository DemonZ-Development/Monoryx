use crate::app::state::AppState;
use crate::content::ContentKind;
use crate::modrinth::search::SortOrder;
use crate::ui::components::{badge, card_frame, empty_state, page_header, thin_progress};
use crate::ui::theme::{BORDER, ELEVATED, TEXT, TEXT2};
use egui::{CornerRadius, RichText, Stroke};
pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Discover",
        "Mods, modpacks, resource packs and shaders from Modrinth.",
    );
    ui.horizontal(|ui| {
        for (kind, label) in [
            (ContentKind::Mod, "Mods"),
            (ContentKind::Resourcepack, "Resource Packs"),
            (ContentKind::Shader, "Shaders"),
        ] {
            let sel = state.discover_tab == kind && state.search.project_type != "modpack";
            if ui.selectable_label(sel, label).clicked() {
                state.discover_tab = kind;
                state.search.project_type = kind.as_str().to_string();
                state.search.offset = 0;
                state.run_search();
            }
        }
        if ui
            .selectable_label(state.search.project_type == "modpack", "Modpacks")
            .clicked()
        {
            state.search.project_type = "modpack".to_string();
            state.search.offset = 0;
            state.run_search();
        }
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let resp = ui.text_edit_singleline(&mut state.search.query);
        if resp.changed() {
            state.search_debounce = Some(std::time::Instant::now());
        }
        if ui.button("Search").clicked() {
            state.search_debounce = None;
            state.run_search();
        }
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            state.search_debounce = None;
            state.run_search();
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("MC").size(11.0).color(TEXT2));
        ui.text_edit_singleline(&mut state.search.game_version);
        ui.label(RichText::new("Loader").size(11.0).color(TEXT2));
        ui.text_edit_singleline(&mut state.search.loader);
        egui::ComboBox::from_id_salt("sort")
            .selected_text(state.search.sort.label())
            .show_ui(ui, |ui| {
                for s in SortOrder::all() {
                    ui.selectable_value(&mut state.search.sort, s, s.label());
                }
            });
        if ui.small_button("Apply").clicked() {
            state.run_search();
        }
        if ui.small_button("Use instance").clicked() {
            state.search.game_version.clear();
            state.search.loader.clear();
            state.sync_search_filters();
            state.run_search();
        }
    });
    ui.add_space(4.0);
    if state.search_loading {
        ui.label("Searching...");
        thin_progress(ui, None);
    }
    if !state.search_error.is_empty() {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&state.search_error).color(egui::Color32::from_rgb(0xE0, 0x5A, 0x5A)),
            );
            if ui.button("Retry").clicked() {
                state.run_search();
            }
        });
    }
    if state.detail_project.is_some() {
        show_detail(state, ui);
        ui.add_space(8.0);
    }
    if state.search_results.is_empty() && !state.search_loading && state.search_error.is_empty() {
        card_frame(ui, |ui| {
            empty_state(ui, "No results", "Try another query or clear the filters.");
        });
        return;
    }
    ui.label(
        RichText::new(format!("{} results", state.search_total))
            .size(11.0)
            .color(TEXT2),
    );
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for hit in state.search_results.clone() {
                egui::Frame::new()
                    .fill(ELEVATED)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            show_thumb(state, ui, hit.icon_url.as_deref().unwrap_or(""), 44.0);
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&hit.title).size(15.0).strong().color(TEXT),
                                    );
                                });
                                ui.label(
                                    RichText::new(format!(
                                        "by {} - {} downloads",
                                        hit.author, hit.downloads
                                    ))
                                    .size(11.0)
                                    .color(TEXT2),
                                );
                                ui.label(RichText::new(&hit.description).size(12.0).color(TEXT));
                                ui.horizontal(|ui| {
                                    badge(ui, &hit.project_type);
                                    if let Some(lv) = hit.latest_version.as_deref() {
                                        if !lv.is_empty() {
                                            badge(ui, lv);
                                        }
                                    }
                                });
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.vertical(|ui| {
                                        if ui.button("Open").clicked() {
                                            state.detail_loading = true;
                                            state.detail_project = None;
                                            state.detail_versions.clear();
                                            crate::app::tasks::open_project_page(
                                                state,
                                                hit.slug.clone(),
                                            );
                                        }
                                        if state.search.project_type != "modpack"
                                            && ui.button("Install").clicked()
                                        {
                                            crate::app::tasks::install_mod(
                                                state,
                                                hit.slug.clone(),
                                                hit.slug.clone(),
                                                hit.title.clone(),
                                                None,
                                            );
                                            state.global_status =
                                                format!("Installing {}...", hit.title);
                                        }
                                        if state.search.project_type == "modpack" {
                                            ui.label(
                                                RichText::new("Use Open for packs")
                                                    .size(10.0)
                                                    .color(TEXT2),
                                            );
                                        }
                                    });
                                },
                            );
                        });
                    });
                ui.add_space(6.0);
            }
        });
}
fn show_detail(state: &mut AppState, ui: &mut egui::Ui) {
    let Some(p) = state.detail_project.clone() else {
        return;
    };
    egui::Frame::new()
        .fill(crate::ui::theme::ELEVATED2)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                show_thumb(state, ui, p.icon_url.as_deref().unwrap_or(""), 64.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(&p.title).size(18.0).strong().color(TEXT));
                    ui.label(
                        RichText::new(format!("{} downloads", p.downloads))
                            .size(11.5)
                            .color(TEXT2),
                    );
                    if let Some(b) = &p.body {
                        let short: String = b.chars().take(400).collect();
                        ui.label(RichText::new(short).size(12.0).color(TEXT));
                    } else {
                        ui.label(RichText::new(&p.description).size(12.0).color(TEXT));
                    }
                    ui.horizontal(|ui| {
                        if let Some(lic) = &p.license {
                            badge(ui, &lic.name);
                        }
                        badge(ui, &p.project_type);
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button("Close").clicked() {
                        state.detail_project = None;
                        state.detail_versions.clear();
                    }
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if state.search.project_type == "modpack" {
                    if ui.button("Install Pack (new instance)").clicked() {
                        ui.label("Pick a version below, then Install.");
                    }
                } else if ui.button(RichText::new("Install").strong()).clicked() {
                    let pick = if state.detail_version_pick.is_empty() {
                        None
                    } else {
                        Some(state.detail_version_pick.clone())
                    };
                    crate::app::tasks::install_mod(
                        state,
                        p.slug.clone(),
                        p.slug.clone(),
                        p.title.clone(),
                        pick,
                    );
                    state.global_status = format!("Installing {}...", p.title);
                }
                if ui.button("Open in browser").clicked() {
                    let _ = open::that(format!("https://modrinth.com/project/{}", p.slug));
                }
            });
            if !state.detail_versions.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("Versions").strong().color(TEXT));
                egui::ComboBox::from_id_salt("detail-ver")
                    .selected_text(if state.detail_version_pick.is_empty() {
                        "(newest compatible)".to_string()
                    } else {
                        state.detail_version_pick.clone()
                    })
                    .show_ui(ui, |ui| {
                        for v in state.detail_versions.clone().into_iter().take(40) {
                            let label =
                                format!("{} ({})", v.version_number, v.game_versions.join(", "));
                            ui.selectable_value(
                                &mut state.detail_version_pick,
                                v.id.clone(),
                                label,
                            );
                        }
                    });
                if ui.small_button("Clear choice").clicked() {
                    state.detail_version_pick.clear();
                }
            }
            if state.detail_loading {
                ui.label("Loading details...");
            }
            if !state.detail_error.is_empty() {
                ui.label(
                    RichText::new(&state.detail_error)
                        .color(egui::Color32::from_rgb(0xE0, 0x5A, 0x5A)),
                );
            }
        });
}
fn show_thumb(state: &mut AppState, ui: &mut egui::Ui, url: &str, size: f32) {
    if url.is_empty() {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        ui.painter().rect_filled(rect, 6, ELEVATED);
        return;
    }
    state.ensure_thumb(url);
    if let Some(tex) = state.thumbnails.get(url) {
        ui.add(egui::Image::new((tex.id(), egui::vec2(size, size))).corner_radius(6));
    } else {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        ui.painter().rect_filled(rect, 6, ELEVATED);
    }
}
