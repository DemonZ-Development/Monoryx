use crate::app::events::Page;
use crate::app::state::AppState;
use crate::modrinth::search::{DiscoverTab, SortOrder};
use crate::ui::components::{
    badge, card_frame, content_primary_button, content_secondary_button, empty_state,
    hover_card_frame, page_header, thin_progress,
};
use crate::ui::theme::{DANGER, MUTED, TEXT, TEXT2};
use egui::{Color32, CornerRadius, RichText, Stroke};

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Discover",
        "Explore mods, modpacks, resource packs and shaders from Modrinth.",
    );

    if state.discover_tab != DiscoverTab::Modpacks {
        let current = state.selected();
        let installable = state.installable_instances();
        let blocked = state.install_blocker();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Install into:").strong().color(TEXT));
            if installable.is_empty() {
                let label = if state.instance_list.is_empty() {
                    "No instances yet"
                } else {
                    "No downloaded instances"
                };
                ui.add_enabled(
                    false,
                    egui::Button::new(RichText::new(label).size(12.0).color(MUTED))
                        .min_size(egui::vec2(220.0, 28.0)),
                );
                if state.instance_list.is_empty() {
                    if ui.link("Create an instance").clicked() {
                        crate::ui::pages::instances::open_new_dialog(state);
                        state.set_page(Page::Instances);
                    }
                } else if ui.link("Repair files").clicked() {
                    if let Some(cfg) = current.clone() {
                        crate::app::tasks::repair_instance(state, cfg.id);
                    }
                }
            } else {
                let mut selection = state.selected_instance.clone();
                egui::ComboBox::from_id_salt("discover-target-instance")
                    .selected_text(
                        current
                            .as_ref()
                            .map_or("Choose an instance", |cfg| cfg.name.as_str()),
                    )
                    .show_ui(ui, |ui| {
                        for instance in &installable {
                            ui.selectable_value(
                                &mut selection,
                                Some(instance.id.clone()),
                                format!(
                                    "{} · MC {} · {}",
                                    instance.name,
                                    instance.minecraft_version,
                                    instance.loader.display_name()
                                ),
                            );
                        }
                    });
                if selection != state.selected_instance {
                    state.selected_instance = selection;
                    state.save_config();
                    state.refresh_library();

                    let allowed = state.installable_loaders();
                    if !allowed
                        .iter()
                        .any(|kind| kind.as_str() == state.search.loader)
                    {
                        state.search.loader = allowed
                            .first()
                            .map_or(String::new(), |kind| kind.as_str().to_string());
                    }
                    state.search.game_version.clear();
                    state.sync_search_filters();
                    state.search.offset = 0;
                    state.run_search();
                }
            }
        });
        if let Some(reason) = &blocked {
            ui.label(
                RichText::new(format!(
                    "{reason} You can still browse and queue downloads."
                ))
                .size(11.0)
                .color(MUTED),
            );
        }
        ui.add_space(8.0);
    }

    ui.horizontal(|ui| {
        for tab in DiscoverTab::all() {
            let sel = state.discover_tab == tab;
            let label = tab.label();
            if crate::ui::components::tab_button(ui, label, sel).clicked() {
                state.discover_tab = tab;
                state.search.project_type = tab.project_type().to_string();
                if tab == DiscoverTab::Mods {
                    state.search.game_version.clear();
                    state.search.loader.clear();
                    state.sync_search_filters();
                }
                state.search.offset = 0;
                state.run_search();
            }
        }
    });

    ui.add_space(10.0);

    if state.discover_tab == DiscoverTab::Mods || state.discover_tab == DiscoverTab::Modpacks {
        let available = state.installable_loaders();
        if !available.is_empty() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Mod Loader:").size(12.0).color(TEXT2));
                if let Some(loader) =
                    crate::ui::components::loader_pills(ui, &state.search.loader, &available)
                {
                    if loader != state.search.loader {
                        state.search.loader = loader;
                        state.sync_search_filters();
                        state.search.offset = 0;
                        state.run_search();
                    }
                }
            });
            ui.add_space(8.0);
        }
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new("Source:").size(12.0).color(TEXT2));
        ui.label(RichText::new("Modrinth").size(12.0).color(TEXT));
        badge(ui, "CurseForge coming soon");
        if ui
            .link("Learn more")
            .on_hover_text("CurseForge browsing is on the way")
            .clicked()
        {
            let _ = open::that("https://www.curseforge.com/minecraft");
        }
    });
    ui.add_space(8.0);
    egui::Frame::new()
        .fill(crate::ui::theme::palette(ui.ctx()).elevated)
        .stroke(Stroke::new(
            1.0_f32,
            crate::ui::theme::palette(ui.ctx()).border,
        ))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let (_resp, enter) = crate::ui::components::search_field(
                    ui,
                    "discover-search",
                    &mut state.search.query,
                    crate::ui::components::limits::SEARCH,
                    "Search Modrinth for mods, modpacks, resource packs…",
                );
                if _resp.changed() {
                    state.search_debounce = Some(std::time::Instant::now());
                }
                if enter {
                    state.search_debounce = None;
                    state.run_search();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if content_primary_button(ui, "Search").clicked() {
                        state.search_debounce = None;
                        state.search.offset = 0;
                        state.run_search();
                    }
                });
            });

            ui.add_space(4.0);
            egui::CollapsingHeader::new("Filters")
                .id_salt("discover-filters")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let targeted_mods =
                            state.discover_tab == DiscoverTab::Mods && state.selected().is_some();
                        if targeted_mods {
                            if let Some(instance) = state.selected() {
                                ui.label(
                                    RichText::new(format!(
                                        "Compatible with {} · Minecraft {} · {}",
                                        instance.name,
                                        instance.minecraft_version,
                                        instance.loader.display_name()
                                    ))
                                    .size(12.0)
                                    .color(TEXT2),
                                );
                            }
                        } else {
                            ui.label(RichText::new("MC Version:").size(12.0).color(TEXT2));
                            ui.add(
                                egui::TextEdit::singleline(&mut state.search.game_version)
                                    .desired_width(75.0)
                                    .hint_text("All")
                                    .char_limit(crate::ui::components::limits::ADDRESS),
                            );
                            if !state.search.game_version.is_empty() && ui.button("Clear").clicked()
                            {
                                state.search.game_version.clear();
                                state.search.offset = 0;
                                state.run_search();
                            }
                        }

                        ui.add_space(8.0);

                        let is_loader_applicable = state.discover_tab == DiscoverTab::Mods
                            || state.discover_tab == DiscoverTab::Modpacks;

                        if is_loader_applicable && !targeted_mods {
                            ui.label(
                                RichText::new("Supported loader filter:")
                                    .size(12.0)
                                    .color(TEXT2),
                            );
                            let current_loader_label = if state.search.loader.is_empty() {
                                "All Loaders"
                            } else {
                                match state.search.loader.as_str() {
                                    "fabric" => "Fabric",
                                    "forge" => "Forge",
                                    "neoforge" => "NeoForge",
                                    "quilt" => "Quilt",
                                    other => other,
                                }
                            };
                            egui::ComboBox::from_id_salt("discover-loader")
                                .selected_text(current_loader_label)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_value(
                                            &mut state.search.loader,
                                            String::new(),
                                            "Any supported loader",
                                        )
                                        .clicked()
                                    {
                                        state.search.offset = 0;
                                        state.run_search();
                                    }
                                    for (id, label) in [
                                        ("fabric", "Fabric"),
                                        ("forge", "Forge"),
                                        ("neoforge", "NeoForge"),
                                        ("quilt", "Quilt"),
                                    ] {
                                        if ui
                                            .selectable_value(
                                                &mut state.search.loader,
                                                id.to_string(),
                                                label,
                                            )
                                            .clicked()
                                        {
                                            state.search.offset = 0;
                                            state.run_search();
                                        }
                                    }
                                });
                            ui.label(
                                RichText::new(
                                    "Search choices; these are not installed in your instance.",
                                )
                                .size(11.0)
                                .color(MUTED),
                            );
                        }

                        ui.add_space(8.0);
                        ui.label(RichText::new("Sort:").size(12.0).color(TEXT2));
                        let prev_sort = state.search.sort;
                        egui::ComboBox::from_id_salt("sort")
                            .selected_text(state.search.sort.label())
                            .show_ui(ui, |ui| {
                                for s in SortOrder::all() {
                                    ui.selectable_value(&mut state.search.sort, s, s.label());
                                }
                            });
                        if state.search.sort != prev_sort {
                            state.search.offset = 0;
                            state.run_search();
                        }

                        ui.horizontal(|ui| {
                            if !targeted_mods && ui.button("Clear Filters").clicked() {
                                state.search.game_version.clear();
                                state.search.loader.clear();
                                state.search.offset = 0;
                                state.run_search();
                            }
                            if ui
                                .button("Use selected instance")
                                .on_hover_text(
                                    "Search for your selected instance's version and loader",
                                )
                                .clicked()
                            {
                                state.search.game_version.clear();
                                state.search.loader.clear();
                                state.sync_search_filters();
                                state.search.offset = 0;
                                state.run_search();
                            }
                        });
                    });
                });
        });

    ui.add_space(8.0);

    if state.search_loading {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                RichText::new("Searching Modrinth...")
                    .size(12.5)
                    .color(TEXT2),
            );
        });
        thin_progress(ui, None);
        ui.add_space(6.0);
    }

    if !state.search_error.is_empty() {
        egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(224, 90, 90, 22))
            .stroke(Stroke::new(1.0_f32, DANGER))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(14, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&state.search_error).color(DANGER));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Retry").clicked() {
                            state.run_search();
                        }
                    });
                });
            });
        ui.add_space(8.0);
    }

    if state.search_results.is_empty() && !state.search_loading && state.search_error.is_empty() {
        card_frame(ui, |ui| {
            let filter_hint = if !state.search.game_version.is_empty() {
                format!(
                    "No results found for Minecraft {}. Try clearing the version filter to see all available releases.",
                    state.search.game_version
                )
            } else {
                "Try another search term, or clear any filters to view all available content."
                    .to_string()
            };
            empty_state(ui, "No results found", &filter_hint);
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    if !state.search.game_version.is_empty()
                        && ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Clear Version Filter")
                                        .strong()
                                        .color(crate::ui::theme::palette(ui.ctx()).accent_text),
                                )
                                .fill(crate::ui::theme::palette(ui.ctx()).accent)
                                .corner_radius(CornerRadius::same(8)),
                            )
                            .clicked()
                    {
                        state.search.game_version.clear();
                        state.search.offset = 0;
                        state.run_search();
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Clear All Filters & Show All")
                                    .size(12.5)
                                    .color(TEXT),
                            )
                            .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                            .stroke(Stroke::new(
                                1.0_f32,
                                crate::ui::theme::palette(ui.ctx()).border,
                            ))
                            .corner_radius(CornerRadius::same(8)),
                        )
                        .clicked()
                    {
                        state.search.query.clear();
                        state.search.game_version.clear();
                        state.search.loader.clear();
                        state.search.offset = 0;
                        state.run_search();
                    }
                });
            });
            ui.add_space(16.0);
        });
        return;
    }

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} projects found", state.search_total))
                .size(12.0)
                .color(TEXT2),
        );
        if !state.search.game_version.is_empty() {
            badge(ui, &format!("MC: {}", state.search.game_version));
        }
        if !state.search.loader.is_empty() {
            badge(ui, &format!("Loader: {}", state.search.loader));
        }
    });

    ui.add_space(4.0);

    {
        let hits = std::mem::take(&mut state.search_results);
        let two_columns = ui.available_width() >= 1120.0;
        let mut index = 0;
        while index < hits.len() {
            let expanded = state.detail_slug.as_deref() == Some(hits[index].slug.as_str());
            if !two_columns || expanded {
                show_hit_card(state, ui, &hits[index], expanded);
                index += 1;
            } else {
                let second = hits
                    .get(index + 1)
                    .filter(|next| state.detail_slug.as_deref() != Some(next.slug.as_str()));
                if let Some(hit) = second {
                    ui.columns(2, |columns| {
                        show_hit_card(state, &mut columns[0], &hits[index], false);
                        show_hit_card(state, &mut columns[1], hit, false);
                    });
                    index += 2;
                } else {
                    show_hit_card(state, ui, &hits[index], false);
                    index += 1;
                }
            }
            ui.add_space(8.0);
        }

        if state.search_total > 24 {
            ui.add_space(6.0);
            let current_page = (state.search.offset / 24) + 1;
            let total_pages = ((state.search_total as f32) / 24.0).ceil() as u32;
            ui.horizontal(|ui| {
                let has_prev = state.search.offset > 0;
                if ui
                    .add_enabled(
                        has_prev,
                        egui::Button::new("Previous Page")
                            .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                            .stroke(Stroke::new(
                                1.0_f32,
                                crate::ui::theme::palette(ui.ctx()).border,
                            ))
                            .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    state.search.offset = state.search.offset.saturating_sub(24);
                    state.run_search();
                }

                ui.label(
                    RichText::new(format!(
                        "Page {current_page} of {total_pages} ({} total)",
                        state.search_total
                    ))
                    .size(12.0)
                    .color(TEXT2),
                );

                let has_next = state.search.offset + 24 < state.search_total;
                if ui
                    .add_enabled(
                        has_next,
                        egui::Button::new("Next Page")
                            .fill(crate::ui::theme::palette(ui.ctx()).elevated2)
                            .stroke(Stroke::new(
                                1.0_f32,
                                crate::ui::theme::palette(ui.ctx()).border,
                            ))
                            .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    state.search.offset += 24;
                    state.run_search();
                }
            });
            ui.add_space(10.0);
        }
        state.search_results = hits;
    }
}

fn format_downloads(d: u64) -> String {
    if d >= 1_000_000 {
        format!("{:.1}M", d as f64 / 1_000_000.0)
    } else if d >= 1_000 {
        format!("{:.1}K", d as f64 / 1_000.0)
    } else {
        d.to_string()
    }
}

fn show_hit_card(
    state: &mut AppState,
    ui: &mut egui::Ui,
    hit: &crate::modrinth::models::SearchResult,
    expanded: bool,
) {
    hover_card_frame(ui, format!("discover_hit_{}", hit.slug), |ui| {
        if expanded {
            if state
                .detail_project
                .as_ref()
                .is_some_and(|project| project.slug == hit.slug)
            {
                show_detail(state, ui);
            } else {
                ui.horizontal(|ui| {
                    show_thumb(state, ui, hit.icon_url.as_deref().unwrap_or(""), 52.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&hit.title).size(17.0).strong().color(TEXT));
                        ui.label(RichText::new("Project details").size(11.5).color(TEXT2));
                    });
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 52.0),
                        egui::Layout::right_to_left(egui::Align::Min),
                        |ui| {
                            if content_secondary_button(ui, "Close").clicked() {
                                close_detail(state);
                            }
                        },
                    );
                });
                if state.detail_loading {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Loading details...").color(TEXT2));
                    });
                }
                if !state.detail_error.is_empty() {
                    ui.colored_label(DANGER, &state.detail_error);
                    if content_secondary_button(ui, "Retry").clicked() {
                        state.detail_loading = true;
                        state.detail_versions_loading = true;
                        state.detail_error.clear();
                        crate::app::tasks::open_project_page(state, hit.slug.clone());
                    }
                }
            }
        } else {
            show_hit_summary(state, ui, hit);
        }
    });
}

fn show_hit_summary(
    state: &mut AppState,
    ui: &mut egui::Ui,
    hit: &crate::modrinth::models::SearchResult,
) {
    ui.horizontal(|ui| {
        show_thumb(state, ui, hit.icon_url.as_deref().unwrap_or(""), 52.0);
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.label(RichText::new(&hit.title).size(15.5).strong().color(TEXT));
            ui.label(
                RichText::new(format!(
                    "by {}  ·  {} downloads",
                    hit.author,
                    format_downloads(hit.downloads)
                ))
                .size(11.5)
                .color(TEXT2),
            );
            let description: String = hit.description.chars().take(180).collect();
            let suffix = if hit.description.chars().count() > 180 {
                "…"
            } else {
                ""
            };
            ui.label(
                RichText::new(format!("{description}{suffix}"))
                    .size(12.0)
                    .color(TEXT),
            );
        });
    });
    ui.horizontal_wrapped(|ui| {
        badge(ui, &hit.project_type);
        for cat in hit.categories.iter().take(3) {
            badge(ui, cat);
        }
    });
    ui.horizontal_wrapped(|ui| {
        if content_secondary_button(ui, "Details").clicked() {
            open_hit_details(state, hit);
        }
        if hit.project_type == "modpack" {
            if content_primary_button(ui, "Install Pack").clicked() {
                state.global_status = format!("Installing modpack {}...", hit.title);
                crate::app::tasks::install_modpack(state, hit.slug.clone(), hit.title.clone());
            }
        } else {
            let installed = state
                .library_entries
                .iter()
                .find(|entry| {
                    entry.project_slug.as_deref() == Some(&hit.slug)
                        && entry.kind.as_str() == hit.project_type
                })
                .cloned();

            let blocker = state.install_blocker();
            let install = content_primary_button(
                ui,
                if installed.is_some() {
                    "Reinstall"
                } else {
                    "Install"
                },
            );
            let install = if let Some(reason) = &blocker {
                install.on_hover_text(format!(
                    "{reason}\nBrowse the project anyway to see versions and details."
                ))
            } else {
                install
            };
            if install.clicked() {
                open_hit_details(state, hit);
            }
            if let Some(installed) = installed {
                if crate::ui::components::danger_button(ui, "Remove").clicked() {
                    if let Some(id) = state.selected_instance.clone() {
                        state.pending_content_delete =
                            Some((id, installed.kind, installed.file_name, hit.title.clone()));
                    }
                }
            }
        }
    });
}

fn open_hit_details(state: &mut AppState, hit: &crate::modrinth::models::SearchResult) {
    state.project_image_url = None;
    state.project_image = None;
    state.detail_slug = Some(hit.slug.clone());
    state.detail_loading = true;
    state.detail_versions_loading = true;
    state.detail_project = None;
    state.markdown_blocks.clear();
    state.detail_versions.clear();
    state.detail_version_pick.clear();
    state.detail_error.clear();
    crate::app::tasks::open_project_page(state, hit.slug.clone());
}

fn close_detail(state: &mut AppState) {
    state.project_image_url = None;
    state.project_image = None;
    state.detail_slug = None;
    state.detail_project = None;
    state.markdown_blocks.clear();
    state.detail_versions.clear();
    state.detail_loading = false;
    state.detail_versions_loading = false;
    state.detail_version_pick.clear();
    state.detail_error.clear();
}

fn show_detail(state: &mut AppState, ui: &mut egui::Ui) {
    let Some(p) = state.detail_project.take() else {
        return;
    };
    ui.vertical(|ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            show_thumb(state, ui, p.icon_url.as_deref().unwrap_or(""), 64.0);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&p.title).size(18.0).strong().color(TEXT));
                ui.label(
                    RichText::new(format!(
                        "{}  ·  {} downloads",
                        p.author.as_deref().unwrap_or("Modrinth"),
                        format_downloads(p.downloads)
                    ))
                    .size(11.5)
                    .color(TEXT2),
                );
            });
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 64.0),
                egui::Layout::right_to_left(egui::Align::Min),
                |ui| {
                    if content_secondary_button(ui, "Close").clicked() {
                        close_detail(state);
                    }
                },
            );
        });
        ui.add_space(8.0);
        ui.label(RichText::new(&p.description).size(12.0).color(TEXT));
        ui.horizontal_wrapped(|ui| {
            if let Some(license) = &p.license {
                badge(ui, &license.name);
            }
            badge(ui, &p.project_type);
            for category in p.categories.iter().take(4) {
                badge(ui, category);
            }
        });
        if state.detail_versions_loading && state.search.project_type != "modpack" {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Loading versions...").color(TEXT2));
            });
        }
        if !state.detail_versions.is_empty() {
            ui.add_space(8.0);
            let selected_inst = state.selected();
            let inst_loader = selected_inst.as_ref().and_then(|i| {
                if i.loader == crate::instance::config::LoaderKind::Vanilla {
                    None
                } else {
                    Some(i.loader.as_str().to_lowercase())
                }
            });

            let mut filtered_versions: Vec<_> = if let Some(loader) = &inst_loader {
                let matching: Vec<_> = state
                    .detail_versions
                    .iter()
                    .filter(|v| {
                        v.loaders.iter().any(|l| {
                            let l_lower = l.to_lowercase();
                            l_lower == *loader
                                || (loader == "quilt" && l_lower == "fabric")
                                || (loader == "fabric" && l_lower == "quilt")
                        })
                    })
                    .collect();
                if !matching.is_empty() {
                    matching
                } else {
                    state.detail_versions.iter().collect()
                }
            } else {
                state.detail_versions.iter().collect()
            };

            if let Some(inst) = &selected_inst {
                let mc = &inst.minecraft_version;
                filtered_versions.sort_by_key(|v| {
                    if v.game_versions.iter().any(|gv| gv == mc) {
                        0
                    } else {
                        1
                    }
                });
            }

            let heading = if let Some(inst) = &selected_inst {
                if inst.loader != crate::instance::config::LoaderKind::Vanilla {
                    format!("Compatible Versions ({})", inst.loader.display_name())
                } else {
                    "Compatible Versions".to_string()
                }
            } else {
                "Compatible Versions".to_string()
            };
            ui.label(RichText::new(heading).strong().color(TEXT));

            let selected_label = if state.detail_version_pick.is_empty() {
                "(newest compatible)".to_string()
            } else if let Some(v) = filtered_versions
                .iter()
                .find(|v| v.id == state.detail_version_pick)
            {
                let loader_str = if v.loaders.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", v.loaders.join("/"))
                };
                format!(
                    "{}{loader_str} ({})",
                    v.version_number,
                    v.game_versions.join(", ")
                )
            } else {
                state.detail_version_pick.clone()
            };

            let control_width = ((ui.available_width() - 8.0) / 2.0).clamp(150.0, 250.0);
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt(("detail-ver", &p.slug))
                    .width(control_width)
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        for v in filtered_versions.into_iter().take(40) {
                            let loader_str = if v.loaders.is_empty() {
                                String::new()
                            } else {
                                format!(" · {}", v.loaders.join("/"))
                            };
                            let label = format!(
                                "{}{loader_str} ({})",
                                v.version_number,
                                v.game_versions.join(", ")
                            );
                            ui.selectable_value(
                                &mut state.detail_version_pick,
                                v.id.clone(),
                                label,
                            );
                        }
                    });
                if detail_button(ui, "Reset version selection", false, true, control_width)
                    .clicked()
                {
                    state.detail_version_pick.clear();
                }
            });
        }
        ui.add_space(8.0);
        let installed = state.library_entries.iter().any(|entry| {
            entry.project_slug.as_deref() == Some(p.slug.as_str())
                && entry.kind.as_str() == p.project_type
        });
        let install_label = match (installed, state.detail_version_pick.is_empty()) {
            (true, true) => "Reinstall newest compatible",
            (true, false) => "Reinstall selected version",
            (false, true) => "Install newest compatible",
            (false, false) => "Install selected version",
        };
        let compatibility_issue = state.selected().and_then(|instance| {
            if state.detail_versions_loading {
                return None;
            }
            let versions: Vec<_> = state
                .detail_versions
                .iter()
                .filter(|version| {
                    state.detail_version_pick.is_empty() || version.id == state.detail_version_pick
                })
                .collect();
            if versions.is_empty() {
                return Some("No project versions are available.".to_string());
            }
            let mc = &instance.minecraft_version;
            if !versions.iter().any(|version| {
                version.game_versions.is_empty()
                    || version.game_versions.iter().any(|game| game == mc)
            }) {
                return Some(format!(
                    "No build for Minecraft {mc}. Choose a different version or instance."
                ));
            }
            if p.project_type == "mod" {
                let loader = instance.loader.as_str();
                if !versions
                    .iter()
                    .any(|version| crate::modrinth::models::is_compatible(version, mc, loader))
                {
                    return Some(format!(
                        "No {} build for Minecraft {mc}. Choose a different loader or instance.",
                        instance.loader.display_name()
                    ));
                }
            }
            None
        });
        if let Some(reason) = &compatibility_issue {
            ui.label(RichText::new(reason).color(TEXT2));
        }

        let readiness_issue = state.install_blocker();
        let can_install = compatibility_issue.is_none() && readiness_issue.is_none();
        if let Some(reason) = &readiness_issue {
            ui.label(RichText::new(reason).color(MUTED));
        }
        let control_width = ((ui.available_width() - 8.0) / 2.0).clamp(150.0, 250.0);
        ui.horizontal(|ui| {
            if p.project_type == "modpack" {
                if detail_button(ui, "Install as new instance", true, true, control_width).clicked()
                {
                    state.global_status = format!("Installing modpack {}...", p.title);
                    crate::app::tasks::install_modpack(state, p.slug.clone(), p.title.clone());
                }
            } else if detail_button(
                ui,
                install_label,
                true,
                !state.detail_versions_loading && can_install,
                control_width,
            )
            .clicked()
            {
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
            if detail_button(ui, "Open in browser", false, true, control_width).clicked() {
                let _ = open::that(format!("https://modrinth.com/project/{}", p.slug));
            }
        });
        if !p.gallery.is_empty() {
            ui.add_space(14.0);
            ui.label(RichText::new("GALLERY").size(10.5).strong().color(MUTED));
            ui.horizontal_wrapped(|ui| {
                for image in p.gallery.iter().take(8) {
                    if image.url.is_empty() {
                        continue;
                    }
                    state.ensure_thumb(&image.url);
                    let response = if let Some(texture) = state.thumbnails.get(&image.url) {
                        ui.add(
                            egui::Image::new((texture.id(), egui::vec2(120.0, 76.0)))
                                .fit_to_exact_size(egui::vec2(120.0, 76.0))
                                .corner_radius(6)
                                .sense(egui::Sense::click()),
                        )
                    } else {
                        ui.add_sized([120.0, 76.0], egui::Button::new("View image"))
                    };
                    if response.clicked() {
                        state.open_project_image(&image.url);
                    }
                    response.on_hover_text(image.title.as_deref().unwrap_or("View image"));
                }
            });
        }
        if let Some(body) = &p.body {
            if !body.trim().is_empty() && body.trim() != p.description.trim() {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("ABOUT THIS PROJECT")
                        .size(10.5)
                        .strong()
                        .color(MUTED),
                );
                if let Some(url) = crate::ui::markdown::show(ui, &state.markdown_blocks) {
                    state.open_project_image(&url);
                }
            }
        }
        if !state.detail_error.is_empty() {
            ui.label(RichText::new(&state.detail_error).color(DANGER));
        }
    });
    if state.detail_slug.as_deref() == Some(p.slug.as_str()) {
        state.detail_project = Some(p);
    }
}

fn detail_button(
    ui: &mut egui::Ui,
    label: &str,
    primary: bool,
    enabled: bool,
    width: f32,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        crate::ui::components::sized_action_button(ui, label, primary, width)
    })
    .inner
}

fn show_thumb(state: &mut AppState, ui: &mut egui::Ui, url: &str, size: f32) {
    if url.is_empty() {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 8, crate::ui::theme::palette(ui.ctx()).elevated2);
        return;
    }
    state.ensure_thumb(url);
    if let Some(tex) = state.thumbnails.get(url) {
        ui.add(egui::Image::new((tex.id(), egui::vec2(size, size))).corner_radius(8));
    } else {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 8, crate::ui::theme::palette(ui.ctx()).elevated2);
    }
}
