use crate::app::state::AppState;
use crate::ui::components::{card_frame, content_primary_button, page_header};
use crate::ui::theme::{format_bytes, palette, DANGER, MUTED, TEXT, TEXT2};
use egui::{CornerRadius, RichText, Stroke};

const MAX_CONSOLE_LINES: usize = 400;

fn console_tail(logs: &str, limit: usize) -> Vec<&str> {
    let mut lines: Vec<&str> = logs.lines().collect();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    lines
}

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(
        ui,
        "Nexeu Servers",
        "Server controls, console and backups in one place.",
    );
    if state.nexeu.overview.is_none() {
        card_frame(ui, |ui| {
            ui.label(
                RichText::new("Connect your game panel")
                    .size(18.0)
                    .strong()
                    .color(TEXT),
            );
            ui.label(RichText::new("Enter a client API key from the Nexeu game panel. It stays in memory until you disconnect or close MONORYX.").color(TEXT2));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.nexeu.api_key)
                        .password(true)
                        .hint_text("Full client API key")
                        .desired_width(380.0),
                );
                if ui
                    .add_enabled(!state.nexeu.loading, egui::Button::new("Connect"))
                    .clicked()
                {
                    state.nexeu_refresh();
                }
                if state.nexeu.loading {
                    ui.spinner();
                }
            });
            ui.hyperlink_to("Manage API keys", "https://game.nexeu.zip/");
        });
        if !state.nexeu.error.is_empty() {
            ui.colored_label(DANGER, &state.nexeu.error);
        }
        return;
    }
    let overview = state.nexeu.overview.clone().unwrap();
    let account = overview
        .account
        .get("user")
        .or_else(|| overview.account.get("account"))
        .unwrap_or(&overview.account);
    let name = account
        .get("username")
        .or_else(|| account.get("name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Connected account");
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Connected as {name}")).color(TEXT2));
        if ui.button("Refresh").clicked() {
            state.nexeu_refresh();
        }
        if ui.button("Disconnect").clicked() {
            let generation = state.nexeu.generation.wrapping_add(1);
            state.nexeu = crate::nexeu::Session {
                generation,
                ..Default::default()
            };
        }
        if state.nexeu.loading {
            ui.spinner();
        }
    });
    if !state.nexeu.error.is_empty() {
        ui.colored_label(DANGER, &state.nexeu.error);
    }
    ui.add_space(10.0);
    if overview.servers.is_empty() {
        card_frame(ui, |ui| {
            ui.label("No servers are linked to this account.");
        });
        return;
    }
    let selected_id = state
        .nexeu
        .selected_server
        .clone()
        .unwrap_or_else(|| overview.servers[0].uuid.clone());
    if state.nexeu.selected_server.is_none() {
        state.nexeu_select_server(selected_id.clone());
    }
    let server = overview
        .servers
        .iter()
        .find(|server| server.uuid == selected_id)
        .unwrap_or(&overview.servers[0]);
    let sidebar_width = (ui.available_width() * 0.25).clamp(215.0, 310.0);
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(sidebar_width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.label(
                    RichText::new(format!("SERVERS  {}", overview.servers.len()))
                        .size(11.0)
                        .strong()
                        .color(MUTED),
                );
                ui.add_space(5.0);
                let p = palette(ui.ctx());
                for item in &overview.servers {
                    let selected = item.uuid == selected_id;
                    let response = egui::Frame::new()
                        .fill(if selected { p.elevated2 } else { p.elevated })
                        .stroke(Stroke::new(
                            if selected { 1.5_f32 } else { 1.0_f32 },
                            if selected { p.accent } else { p.border },
                        ))
                        .corner_radius(CornerRadius::same(9))
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.set_min_width(sidebar_width - 24.0);
                            ui.label(RichText::new(&item.name).strong().color(TEXT));
                            ui.label(
                                RichText::new(
                                    item.status.as_deref().unwrap_or("Status unavailable"),
                                )
                                .size(11.0)
                                .color(TEXT2),
                            );
                        });
                    if ui
                        .interact(
                            response.response.rect,
                            ui.make_persistent_id(&item.uuid),
                            egui::Sense::click(),
                        )
                        .clicked()
                    {
                        state.nexeu_select_server(item.uuid.clone());
                    }
                    ui.add_space(6.0);
                }
            },
        );
        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.set_min_width((ui.available_width() - 8.0).max(300.0));
            card_frame(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&server.name).size(21.0).strong().color(TEXT));
                    ui.label(
                        RichText::new(server.status.as_deref().unwrap_or("Unknown")).color(TEXT2),
                    );
                });
                if let Some(description) = &server.description {
                    if !description.is_empty() {
                        ui.label(RichText::new(description).color(TEXT2));
                    }
                }
                if let Some(allocation) = &server.allocation {
                    let host = allocation
                        .ip_alias
                        .as_deref()
                        .or(allocation.ip.as_deref())
                        .unwrap_or("");
                    if !host.is_empty() {
                        ui.label(
                            RichText::new(format!("{}:{}", host, allocation.port.unwrap_or(25565)))
                                .monospace()
                                .color(TEXT2),
                        );
                    }
                }
                ui.add_space(9.0);
                ui.horizontal_wrapped(|ui| {
                    if content_primary_button(ui, "Start").clicked() {
                        state.nexeu_power(server.uuid.clone(), "start");
                    }
                    if ui.button("Restart").clicked() {
                        state.nexeu.pending_power = Some((server.uuid.clone(), "restart"));
                    }
                    if ui.button("Stop").clicked() {
                        state.nexeu.pending_power = Some((server.uuid.clone(), "stop"));
                    }
                    if ui.button("Refresh usage").clicked() {
                        state.nexeu_select_server(server.uuid.clone());
                    }
                });
            });
            ui.add_space(10.0);
            let resources = state.nexeu.resources.as_ref();
            let state_label = resources
                .and_then(|value| value.get("state"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Unknown");
            let cpu = resources
                .and_then(|value| value.get("cpu_absolute"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let memory = resources
                .and_then(|value| value.get("memory_bytes"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let disk = resources
                .and_then(|value| value.get("disk_bytes"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            ui.columns(4, |columns| {
                metric(&mut columns[0], "STATUS", state_label);
                metric(&mut columns[1], "CPU", &format!("{cpu:.1}%"));
                metric(&mut columns[2], "MEMORY", &format_bytes(memory));
                metric(&mut columns[3], "DISK", &format_bytes(disk));
            });
            ui.add_space(10.0);
            card_frame(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Console").size(16.0).strong().color(TEXT));
                    if ui.button("Refresh log").clicked() {
                        state.nexeu_load_logs(server.uuid.clone());
                    }
                });
                let p = palette(ui.ctx());
                egui::Frame::new()
                    .fill(p.bg)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(240.0)
                            .min_scrolled_height(170.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                match state.nexeu.logs.as_deref() {
                                    Some(logs) => {
                                        let visible = console_tail(logs, MAX_CONSOLE_LINES);
                                        for line in visible {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(line)
                                                        .monospace()
                                                        .size(11.5)
                                                        .color(TEXT2),
                                                )
                                                .wrap(),
                                            );
                                        }
                                    }
                                    None => {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(
                                                    "Refresh the log to view server output.",
                                                )
                                                .color(TEXT2),
                                            )
                                            .wrap(),
                                        );
                                    }
                                }
                            });
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut state.nexeu.console_command)
                            .hint_text("Send a server command")
                            .desired_width((ui.available_width() - 90.0).max(160.0)),
                    );
                    if (ui.button("Send").clicked()
                        || (edit.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter))))
                        && !state.nexeu.console_command.trim().is_empty()
                    {
                        state.nexeu_send_command(server.uuid.clone());
                    }
                });
            });
            ui.add_space(10.0);
            card_frame(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Backups").size(16.0).strong().color(TEXT));
                    if ui.button("Refresh").clicked() {
                        state.nexeu_load_backups(server.uuid.clone());
                    }
                    if ui.button("Create backup").clicked() {
                        state.nexeu_create_backup(server.uuid.clone());
                    }
                });
                if let Some(backups) = &state.nexeu.backups {
                    if backups.is_empty() {
                        ui.label(RichText::new("No backups yet.").color(TEXT2));
                    }
                    for backup in backups {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&backup.name).strong().color(TEXT));
                            ui.label(
                                RichText::new(if backup.is_successful == Some(true) {
                                    "Ready"
                                } else {
                                    "Processing"
                                })
                                .color(TEXT2),
                            );
                            if let Some(bytes) = backup.bytes {
                                ui.label(RichText::new(format_bytes(bytes)).color(TEXT2));
                            }
                        });
                    }
                } else {
                    ui.label(RichText::new("Loading backups...").color(TEXT2));
                }
            });
        });
    });
    for announcement in overview.announcements.iter().take(2) {
        ui.add_space(8.0);
        card_frame(ui, |ui| {
            ui.label(RichText::new(&announcement.title).strong());
            if let Some(content) = &announcement.content {
                ui.label(content);
            }
        });
    }
    power_confirmation(state, _ctx);
}

fn power_confirmation(state: &mut AppState, ctx: &egui::Context) {
    let Some((id, action)) = state.nexeu.pending_power.clone() else {
        return;
    };
    let label = match action {
        "restart" => "Restart server?",
        "stop" => "Stop server?",
        _ => "Power action?",
    };
    let warning = match action {
        "restart" => "Players on this server will be disconnected.",
        "stop" => "This shuts the server down and players will be disconnected.",
        _ => "",
    };
    let mut finished = false;
    egui::Window::new(label)
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(RichText::new(warning).color(DANGER));
            ui.horizontal(|ui| {
                if crate::ui::components::danger_button(
                    ui,
                    if action == "restart" {
                        "Restart"
                    } else {
                        "Stop"
                    },
                )
                .clicked()
                {
                    state.nexeu_power(id, action);
                    finished = true;
                }
                if ui.button("Cancel").clicked() {
                    finished = true;
                }
            });
        });
    if finished {
        state.nexeu.pending_power = None;
    }
}

fn metric(ui: &mut egui::Ui, label: &str, value: &str) {
    card_frame(ui, |ui| {
        ui.label(RichText::new(label).size(10.5).color(MUTED));
        ui.label(RichText::new(value).size(15.0).strong().color(TEXT));
    });
}
