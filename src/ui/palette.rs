use crate::app::events::Page;
use crate::app::state::AppState;
use crate::ui::theme::{metrics, type_scale, MUTED, TEXT, TEXT2};
use egui::RichText;

struct Command {
    label: String,
    hint: String,
    slash: &'static str,
    group: &'static str,
    run: Box<dyn Fn(&mut AppState)>,
}

fn go(page: Page) -> Box<dyn Fn(&mut AppState)> {
    Box::new(move |state: &mut AppState| state.set_page(page))
}

fn act(f: impl Fn(&mut AppState) + 'static) -> Box<dyn Fn(&mut AppState)> {
    Box::new(f)
}

fn commands(state: &AppState) -> Vec<Command> {
    let mut list: Vec<Command> = [
        (Page::Home, "Go to Home", "Ctrl+1", "/home"),
        (Page::Instances, "Go to Instances", "Ctrl+2", "/instances"),
        (Page::Worlds, "Go to Worlds & Files", "Ctrl+3", "/worlds"),
        (Page::Discover, "Go to Discover", "Ctrl+4", "/discover"),
        (Page::Library, "Go to Library", "Ctrl+5", "/library"),
        (Page::Screenshots, "Go to Screenshots", "F2", "/screenshots"),
        (Page::Downloads, "Go to Downloads", "Ctrl+7", "/downloads"),
        (Page::Accounts, "Go to Accounts", "Ctrl+8", "/accounts"),
        (Page::Settings, "Go to Settings", "Ctrl+9", "/settings"),
        (Page::Logs, "Go to Logs", "", "/logs"),
    ]
    .into_iter()
    .map(|(page, label, hint, slash)| Command {
        label: label.to_string(),
        hint: hint.to_string(),
        slash,
        group: "Navigate",
        run: go(page),
    })
    .collect();

    let has_instance = state.selected().is_some();
    list.push(Command {
        label: "New instance".into(),
        hint: String::new(),
        slash: "/new",
        group: "Actions",
        run: act(|state: &mut AppState| {
            crate::ui::pages::instances::open_new_dialog(state);
            state.set_page(Page::Instances);
        }),
    });
    list.push(Command {
        label: "Open screenshot gallery".into(),
        hint: "F2".into(),
        slash: "/gallery",
        group: "Actions",
        run: go(Page::Screenshots),
    });
    list.push(Command {
        label: "Toggle Eco mode (Boost)".into(),
        hint: "/boost".into(),
        slash: "/boost",
        group: "Actions",
        run: act(|state: &mut AppState| state.toggle_boost()),
    });
    list.push(Command {
        label: "Boost mode: toggle".into(),
        hint: "/boost".into(),
        slash: "/boost",
        group: "Actions",
        run: act(|state: &mut AppState| state.toggle_boost()),
    });
    for theme in crate::config::ThemeKind::all() {
        let (hint, slash): (&'static str, &'static str) = match theme {
            crate::config::ThemeKind::Monochrome => ("", "/monochrome"),
            crate::config::ThemeKind::Gloss => ("", "/gloss"),
            crate::config::ThemeKind::Halloween => ("🎃", "/halloween"),
            crate::config::ThemeKind::SoftPink => ("", "/softpink"),
            crate::config::ThemeKind::SoftBrown => ("", "/softbrown"),
        };
        list.push(Command {
            label: format!("Theme: {}", theme.label()),
            hint: hint.to_string(),
            slash,
            group: "Themes",
            run: act(move |state: &mut AppState| {
                state.config.theme = theme;
                state.save_config();
            }),
        });
    }
    if has_instance {
        list.push(Command {
            label: "Repair game files".into(),
            hint: String::new(),
            slash: "/repair",
            group: "Actions",
            run: act(|state: &mut AppState| {
                if let Some(cfg) = state.selected() {
                    crate::app::tasks::repair_instance(state, cfg.id);
                }
            }),
        });
        list.push(Command {
            label: "Edit selected instance".into(),
            hint: String::new(),
            slash: "/edit",
            group: "Actions",
            run: act(|state: &mut AppState| {
                state.edit_instance = state.selected();
            }),
        });
        list.push(Command {
            label: "Open game folder".into(),
            hint: String::new(),
            slash: "/folder",
            group: "Actions",
            run: act(|state: &mut AppState| {
                if let Some(cfg) = state.selected() {
                    let _ = open::that(state.instances.game_dir(&cfg.id));
                }
            }),
        });
    }

    for cfg in state.instance_list.iter().take(40) {
        let id = cfg.id.clone();
        list.push(Command {
            label: format!("Select instance: {}", cfg.name),
            hint: cfg.minecraft_version.clone(),
            slash: "",
            group: "Instances",
            run: act(move |state: &mut AppState| {
                state.selected_instance = Some(id.clone());
                state.save_config();
                state.refresh_library();
                state.set_page(Page::Home);
            }),
        });
    }
    list
}

#[must_use]
pub fn fuzzy_score(query: &str, haystack: &str) -> Option<usize> {
    let needle: Vec<char> = query
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if needle.is_empty() {
        return Some(0);
    }
    let hay: Vec<char> = haystack.to_ascii_lowercase().chars().collect();
    let mut index = 0usize;
    let mut score = 0usize;
    let mut previous_match = false;
    for (position, ch) in hay.iter().enumerate() {
        if index >= needle.len() {
            break;
        }
        if *ch == needle[index] {
            index += 1;

            if position == 0 || hay[position - 1] == ' ' || hay[position - 1] == ':' {
                score += 6;
            } else if previous_match {
                score += 4;
            } else {
                score += 1;
            }
            previous_match = true;
        } else {
            previous_match = false;
        }
    }
    if index == needle.len() {
        Some(score)
    } else {
        None
    }
}

#[must_use]
pub fn rank(query: &str, label: &str) -> Option<usize> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Some(0);
    }
    let base = fuzzy_score(trimmed, label)?;
    let lower_label = label.to_ascii_lowercase();
    let lower_query = trimmed.to_ascii_lowercase();
    if lower_label.starts_with(&lower_query) {
        Some(base + 20)
    } else {
        Some(base)
    }
}

#[must_use]
pub fn rank_command(query: &str, label: &str, slash: &str) -> Option<usize> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Some(0);
    }
    if let Some(sub) = trimmed.strip_prefix('/') {
        if sub.is_empty() {
            return if !slash.is_empty() { Some(30) } else { None };
        }
        let lower_query = trimmed.to_ascii_lowercase();
        if !slash.is_empty() {
            let lower_slash = slash.to_ascii_lowercase();
            if lower_slash == lower_query {
                return Some(120);
            }
            if lower_slash.starts_with(&lower_query) {
                return Some(
                    90 + (10usize.saturating_sub(slash.len().saturating_sub(trimmed.len()))),
                );
            }
        }
        return rank(sub, label);
    }

    let mut score = rank(trimmed, label);
    if !slash.is_empty() {
        let slash_stem = slash.trim_start_matches('/');
        if slash_stem
            .to_ascii_lowercase()
            .starts_with(&trimmed.to_ascii_lowercase())
        {
            let slash_score = 75;
            score = Some(score.map_or(slash_score, |s| s.max(slash_score)));
        }
    }
    score
}

pub fn handle_shortcuts(state: &mut AppState, ctx: &egui::Context) {
    if state.page == Page::Onboarding {
        return;
    }
    let ctrl = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);

    if ctrl && ctx.input(|i| i.key_pressed(egui::Key::K)) {
        state.command_palette_open = !state.command_palette_open;
        if !state.command_palette_open {
            state.command_palette_query.clear();
        }
    }
    if !ctrl {
        if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Slash)) {
            state.command_palette_open = true;
            state.command_palette_query = "/".to_string();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F2)) {
            state.set_page(Page::Screenshots);
        }
        return;
    }
    let page = match ctx.input(|i| i.key_pressed(egui::Key::Num1)) {
        true => Some(Page::Home),
        false => match ctx.input(|i| i.key_pressed(egui::Key::Num2)) {
            true => Some(Page::Instances),
            false => match ctx.input(|i| i.key_pressed(egui::Key::Num3)) {
                true => Some(Page::Worlds),
                false => match ctx.input(|i| i.key_pressed(egui::Key::Num4)) {
                    true => Some(Page::Discover),
                    false => match ctx.input(|i| i.key_pressed(egui::Key::Num5)) {
                        true => Some(Page::Library),
                        false => match ctx.input(|i| i.key_pressed(egui::Key::Num6)) {
                            true => Some(Page::Screenshots),
                            false => match ctx.input(|i| i.key_pressed(egui::Key::Num7)) {
                                true => Some(Page::Downloads),
                                false => match ctx.input(|i| i.key_pressed(egui::Key::Num8)) {
                                    true => Some(Page::Accounts),
                                    false => match ctx.input(|i| i.key_pressed(egui::Key::Num9)) {
                                        true => Some(Page::Settings),
                                        false => None,
                                    },
                                },
                            },
                        },
                    },
                },
            },
        },
    };
    if let Some(page) = page {
        state.command_palette_open = false;
        state.command_palette_query.clear();
        state.set_page(page);
    }
}

pub fn show(state: &mut AppState, ctx: &egui::Context) {
    if !state.command_palette_open {
        return;
    }
    let screen = ctx.available_rect();
    let width = 520.0_f32.min(screen.width() - 48.0).max(280.0);
    let mut submit = false;
    let mut chosen: Option<usize> = None;
    let mut dismiss = false;

    let all = commands(state);
    let mut ranked: Vec<(usize, Command)> = all
        .into_iter()
        .filter_map(|command| {
            let score = if state.command_palette_query.trim().is_empty() {
                Some(0)
            } else {
                rank_command(&state.command_palette_query, &command.label, command.slash)
            };
            score.map(|score| (score, command))
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.label.cmp(&b.1.label)));
    let visible = ranked.len().min(9);

    let full_screen = ctx.screen_rect();
    let backdrop_clicked = egui::Area::new(egui::Id::new("command-palette-backdrop"))
        .order(egui::Order::Middle)
        .fixed_pos(full_screen.min)
        .interactable(true)
        .show(ctx, |ui| {
            ui.allocate_rect(full_screen, egui::Sense::click())
                .clicked()
        })
        .inner;
    if backdrop_clicked {
        dismiss = true;
    }

    egui::Window::new("command_palette")
        .id(egui::Id::new("command-palette-window"))
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 96.0))
        .fixed_size(egui::vec2(width, 0.0))
        .show(ctx, |ui| {
            let (_resp, _) = crate::ui::components::search_field(
                ui,
                "command-palette-input",
                &mut state.command_palette_query,
                128,
                "Type a command…",
            );
            if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit = true;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                dismiss = true;
            }
            ui.add_space(6.0);
            let mut last_group = "";
            for (index, (_, command)) in ranked.iter().take(visible).enumerate() {
                if command.group != last_group {
                    if index > 0 {
                        ui.add_space(6.0);
                    }
                    ui.label(crate::ui::theme::section_label(command.group));
                    last_group = command.group;
                }
                let p = crate::ui::theme::palette(ui.ctx());
                let response = ui
                    .scope(|ui| {
                        ui.set_width(ui.available_width());
                        let rect = ui.available_rect_before_wrap();
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(rect.width(), 32.0),
                            egui::Sense::click(),
                        );
                        if response.hovered() {
                            ui.painter()
                                .rect_filled(rect, metrics::CONTROL_RADIUS, p.hover);
                        }
                        let clip = egui::Rect::from_min_max(
                            rect.min + egui::vec2(10.0, 0.0),
                            rect.max - egui::vec2(96.0, 0.0),
                        );
                        ui.painter().with_clip_rect(clip).text(
                            rect.left_center() + egui::vec2(10.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            &command.label,
                            egui::FontId::proportional(type_scale::BODY),
                            TEXT,
                        );
                        if !command.hint.is_empty() {
                            ui.painter().text(
                                rect.right_center() - egui::vec2(10.0, 0.0),
                                egui::Align2::RIGHT_CENTER,
                                &command.hint,
                                egui::FontId::proportional(type_scale::CAPTION),
                                MUTED,
                            );
                        }
                        response
                    })
                    .inner;
                if response.clicked() {
                    chosen = Some(index);
                }
            }
            if ranked.is_empty() {
                ui.label(RichText::new("No matching commands").color(TEXT2));
            }
        });

    if dismiss {
        state.command_palette_open = false;
        state.command_palette_query.clear();
    } else if submit {
        if let Some(command) = ranked.first() {
            (command.1.run)(state);
            if state.config.theme != crate::ui::theme::current_theme(ctx) {
                crate::ui::theme::apply_selected_theme(ctx, state.config.theme);
            }
        }
        state.command_palette_open = false;
        state.command_palette_query.clear();
    } else if let Some(index) = chosen {
        if let Some(command) = ranked.get(index) {
            (command.1.run)(state);
            if state.config.theme != crate::ui::theme::current_theme(ctx) {
                crate::ui::theme::apply_selected_theme(ctx, state.config.theme);
            }
        }
        state.command_palette_open = false;
        state.command_palette_query.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        assert_eq!(rank("", "Go to Home"), Some(0));
        assert_eq!(rank("   ", "Go to Home"), Some(0));
    }

    #[test]
    fn prefix_outranks_scattered_match() {
        let prefix = rank("new", "New instance").expect("prefix matches");
        let scattered = rank("new", "Run: enable widgets").expect("scattered matches");
        assert!(prefix > scattered);
    }

    #[test]
    fn non_subsequence_does_not_match() {
        assert_eq!(rank("zzz", "Go to Home"), None);
        assert_eq!(rank("hz", "Go to Home"), None, "'z' never follows an 'h'");
        assert_eq!(rank("mg", "Go to Home"), None);
    }

    #[test]
    fn out_of_order_subsequence_does_not_match() {
        assert!(rank("ho", "Go to Home").is_some());
        assert_eq!(rank("mo", "Go to Home"), None);
    }

    #[test]
    fn acronym_style_queries_work() {
        assert!(rank("gtl", "Go to Library").is_some());
        assert!(rank("gth", "Go to Home").is_some());
    }

    #[test]
    fn hints_and_labels_are_not_confused_for_matching() {
        assert!(rank("ctrl", "Go to Home").is_none());
    }

    #[test]
    fn slash_boost_command_matches() {
        assert!(rank_command("/boost", "Toggle Eco mode (Boost)", "/boost").is_some());
        assert!(rank_command("/boost", "Boost mode: toggle", "/boost").is_some());
        assert!(rank_command("boost", "Toggle Eco mode (Boost)", "/boost").is_some());
        assert!(rank_command("/", "Toggle Eco mode (Boost)", "/boost").is_some());
        assert!(rank_command("/halloween", "Theme: Halloween", "/halloween").is_some());
    }
}
