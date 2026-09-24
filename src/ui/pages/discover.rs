use crate::app::state::AppState;
use crate::modrinth::search::{DiscoverTab, SortOrder};
use crate::ui::components::{
    badge, card_frame, empty_state, hover_card_frame, page_header, thin_progress,
};
use crate::ui::theme::{
    ACCENT, BORDER, DANGER, ELEVATED, ELEVATED2, MUTED, SELECTED_FG, TEXT, TEXT2,
};
use egui::{Color32, CornerRadius, RichText, Stroke};

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Discover",
        "Explore mods, modpacks, resource packs and shaders from Modrinth.",
    );

    if state.discover_tab != DiscoverTab::Modpacks {
        let current = state.selected();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Install into:").strong().color(TEXT));
            let mut selection = state.selected_instance.clone();
            egui::ComboBox::from_id_salt("discover-target-instance")
                .selected_text(
                    current
                        .as_ref()
                        .map_or("Choose an instance", |cfg| cfg.name.as_str()),
                )
                .show_ui(ui, |ui| {
                    for instance in &state.instance_list {
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
                state.search.game_version.clear();
                state.search.loader.clear();
                state.sync_search_filters();
                state.search.offset = 0;
                state.run_search();
            }
        });
        ui.add_space(8.0);
    }

    ui.horizontal(|ui| {
        for tab in DiscoverTab::all() {
            let sel = state.discover_tab == tab;
            let label = tab.label();
            let (fill, stroke, text_color) = if sel {
                (crate::ui::theme::SELECTED, Stroke::NONE, SELECTED_FG)
            } else {
                (ELEVATED, Stroke::new(1.0_f32, BORDER), TEXT2)
            };

            let btn = egui::Button::new(RichText::new(label).strong().size(12.5).color(text_color))
                .fill(fill)
                .stroke(stroke)
                .corner_radius(CornerRadius::same(8));

            if ui.add_sized(egui::vec2(120.0, 32.0), btn).clicked() {
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

    egui::Frame::new()
        .fill(ELEVATED)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut state.search.query)
                        .hint_text("Search Modrinth")
                        .desired_width((ui.available_width() - 170.0).max(150.0)),
                );
                if resp.changed() {
                    state.search_debounce = Some(std::time::Instant::now());
                }
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    state.search_debounce = None;
                    state.run_search();
                }

                if !state.search.query.is_empty()
                    && ui
                        .small_button("Clear")
                        .on_hover_text("Clear query")
                        .clicked()
                {
                    state.search.query.clear();
                    state.search.offset = 0;
                    state.run_search();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let search_btn = egui::Button::new(
                        RichText::new("Search")
                            .strong()
                            .size(13.0)
                            .color(SELECTED_FG),
                    )
                    .fill(ACCENT)
                    .corner_radius(CornerRadius::same(6));

                    if ui.add_sized(egui::vec2(80.0, 28.0), search_btn).clicked() {
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
                                    .hint_text("All"),
                            );
                            if !state.search.game_version.is_empty()
                                && ui.small_button("Clear").clicked()
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
                            if !targeted_mods && ui.small_button("Clear Filters").clicked() {
                                state.search.game_version.clear();
                                state.search.loader.clear();
                                state.search.offset = 0;
                                state.run_search();
                            }
                            if ui
                                .small_button("Use selected instance")
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
                        if ui.small_button("Retry").clicked() {
                            state.run_search();
                        }
                    });
                });
            });
        ui.add_space(8.0);
    }

    if state.detail_project.is_some() {
        show_detail(state, ui);
        ui.add_space(10.0);
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
                                        .color(SELECTED_FG),
                                )
                                .fill(ACCENT)
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
                            .fill(ELEVATED2)
                            .stroke(Stroke::new(1.0_f32, BORDER))
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

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for hit in state.search_results.clone() {
                hover_card_frame(ui, format!("discover_hit_{}", hit.slug), |ui| {
                    ui.horizontal(|ui| {
                        show_thumb(state, ui, hit.icon_url.as_deref().unwrap_or(""), 52.0);
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&hit.title).size(15.5).strong().color(TEXT));
                            });
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("by {}", hit.author))
                                        .size(11.5)
                                        .color(TEXT2),
                                );
                                ui.label(RichText::new("-").size(10.0).color(MUTED));
                                ui.label(
                                    RichText::new(format!(
                                        "{} downloads",
                                        format_downloads(hit.downloads)
                                    ))
                                    .size(11.5)
                                    .color(TEXT2),
                                );
                            });
                            ui.label(
                                RichText::new(&hit.description)
                                    .size(12.0)
                                    .color(TEXT)
                                    .line_height(Some(16.0)),
                            );
                            ui.horizontal_wrapped(|ui| {
                                badge(ui, &hit.project_type);
                                if let Some(lv) = hit.latest_version.as_deref() {
                                    if !lv.is_empty() {
                                        badge(ui, lv);
                                    }
                                }
                                for cat in hit.categories.iter().take(3) {
                                    badge(ui, cat);
                                }
                            });
                        });
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.small_button("Details").clicked() {
                            state.detail_loading = true;
                            state.detail_project = None;
                            state.detail_versions.clear();
                            crate::app::tasks::open_project_page(state, hit.slug.clone());
                        }
                        if state.search.project_type != "modpack" {
                            let installed = state
                                .library_entries
                                .iter()
                                .find(|entry| {
                                    entry.project_slug.as_deref() == Some(&hit.slug)
                                        && entry.kind.as_str() == hit.project_type
                                })
                                .cloned();
                            let btn = egui::Button::new(
                                RichText::new(if installed.is_some() {
                                    "Reinstall"
                                } else {
                                    "Install"
                                })
                                .strong()
                                .color(SELECTED_FG),
                            )
                            .fill(ACCENT)
                            .corner_radius(CornerRadius::same(6));
                            if ui.add(btn).clicked() {
                                crate::app::tasks::install_mod(
                                    state,
                                    hit.slug.clone(),
                                    hit.slug.clone(),
                                    hit.title.clone(),
                                    None,
                                );
                                state.global_status = format!("Installing {}...", hit.title);
                            }
                            if let Some(installed) = installed {
                                if ui.small_button("Remove").clicked() {
                                    if let Some(id) = state.selected_instance.clone() {
                                        match crate::modrinth::install::remove_project_blocking(
                                            &state.instances,
                                            &id,
                                            installed.kind,
                                            &installed.file_name,
                                        ) {
                                            Ok(()) => {
                                                state.refresh_library();
                                                state.notify("Content removed");
                                            }
                                            Err(error) => state.fail(error),
                                        }
                                    }
                                }
                            }
                        } else {
                            let btn = egui::Button::new(
                                RichText::new("Install Pack").strong().color(SELECTED_FG),
                            )
                            .fill(ACCENT)
                            .corner_radius(CornerRadius::same(6));
                            if ui.add(btn).clicked() {
                                state.global_status =
                                    format!("Installing modpack {}...", hit.title);
                                crate::app::tasks::install_modpack(
                                    state,
                                    hit.slug.clone(),
                                    hit.title.clone(),
                                );
                            }
                        }
                    });
                });
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
                                .fill(ELEVATED2)
                                .stroke(Stroke::new(1.0_f32, BORDER))
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
                                .fill(ELEVATED2)
                                .stroke(Stroke::new(1.0_f32, BORDER))
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
        });
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

fn show_detail(state: &mut AppState, ui: &mut egui::Ui) {
    let Some(p) = state.detail_project.clone() else {
        return;
    };
    egui::Frame::new()
        .fill(ELEVATED2)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                show_thumb(state, ui, p.icon_url.as_deref().unwrap_or(""), 64.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(&p.title).size(18.0).strong().color(TEXT));
                    ui.label(
                        RichText::new(format!("{} downloads", format_downloads(p.downloads)))
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
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if state.search.project_type == "modpack" {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Install Modpack as New Instance")
                                    .strong()
                                    .color(SELECTED_FG),
                            )
                            .fill(ACCENT)
                            .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                    {
                        state.global_status = format!("Installing modpack {}...", p.title);
                        crate::app::tasks::install_modpack(state, p.slug.clone(), p.title.clone());
                    }
                    if ui.button("Open on Modrinth").clicked() {
                        let _ = open::that(format!("https://modrinth.com/modpack/{}", p.slug));
                    }
                } else if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Install into active instance")
                                .strong()
                                .color(SELECTED_FG),
                        )
                        .fill(ACCENT)
                        .corner_radius(CornerRadius::same(6)),
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
                if ui.button("Open in browser").clicked() {
                    let _ = open::that(format!("https://modrinth.com/project/{}", p.slug));
                }
            });
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
                        .cloned()
                        .collect();
                    if !matching.is_empty() {
                        matching
                    } else {
                        state.detail_versions.clone()
                    }
                } else {
                    state.detail_versions.clone()
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

                egui::ComboBox::from_id_salt("detail-ver")
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
                if ui.small_button("Reset version selection").clicked() {
                    state.detail_version_pick.clear();
                }
            }
            if state.detail_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading project details...");
                });
            }
            if !state.detail_error.is_empty() {
                ui.label(RichText::new(&state.detail_error).color(DANGER));
            }
        });
}

fn show_thumb(state: &mut AppState, ui: &mut egui::Ui, url: &str, size: f32) {
    if url.is_empty() {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        ui.painter().rect_filled(rect, 8, ELEVATED2);
        return;
    }
    state.ensure_thumb(url);
    if let Some(tex) = state.thumbnails.get(url) {
        ui.add(egui::Image::new((tex.id(), egui::vec2(size, size))).corner_radius(8));
    } else {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        ui.painter().rect_filled(rect, 8, ELEVATED2);
    }
}
