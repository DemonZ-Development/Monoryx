use crate::app::state::AppState;
use crate::ui::components::{card_frame, page_header};
use crate::ui::theme::{TEXT, TEXT2};
use egui::RichText;

pub fn show(state: &mut AppState, _ctx: &egui::Context, ui: &mut egui::Ui) {
    page_header(ui, "Nexeu Servers", "Your hosting space.");
    ui.horizontal_wrapped(|ui| {
        ui.hyperlink_to("Game panel", "https://game.nexeu.zip/");
        ui.hyperlink_to("Billing and new servers", "https://client.nexeu.zip/");
        ui.hyperlink_to("Support on Discord", "https://discord.gg/hSKbA8haZh");
    });
    ui.add_space(10.0);
    card_frame(ui, |ui| {
        ui.label(
            RichText::new("Nexeu sign in · Coming soon")
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new(
                "Launcher sign in is coming soon. Manage your account on Nexeu's website for now.",
            )
            .color(TEXT2),
        );
        ui.add_space(8.0);
        ui.label(RichText::new("Game panel API · Beta").strong().color(TEXT));
        egui::CollapsingHeader::new("Connect with a panel API key (Beta)")
            .id_salt("nexeu-token-connect")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(
                        "Use the full API key shown when you created it in the game panel. It stays in memory until you disconnect or close the launcher.",
                    )
                    .color(TEXT2),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.nexeu.api_key)
                            .password(true)
                            .hint_text("Full game panel API key")
                            .desired_width(260.0),
                    );
                    if ui
                        .add_enabled(!state.nexeu.loading, egui::Button::new("Connect"))
                        .clicked()
                    {
                        state.nexeu_refresh();
                    }
                    if !state.nexeu.api_key.is_empty() && ui.button("Disconnect").clicked() {
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
            });
        if !state.nexeu.error.is_empty() {
            ui.colored_label(crate::ui::theme::DANGER, &state.nexeu.error);
        }
    });

    let Some(overview) = state.nexeu.overview.clone() else {
        ui.add_space(8.0);
        ui.label("Your local instances are available while Nexeu is offline.");
        return;
    };
    ui.add_space(10.0);
    let account = overview
        .account
        .get("user")
        .or_else(|| overview.account.get("account"))
        .unwrap_or(&overview.account);
    let name = account
        .get("username")
        .or_else(|| account.get("name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Account connected");
    ui.label(
        RichText::new(format!("Signed in as {name}"))
            .strong()
            .color(TEXT),
    );

    for announcement in overview.announcements.iter().take(3) {
        card_frame(ui, |ui| {
            ui.label(RichText::new(&announcement.title).strong());
            if let Some(content) = &announcement.content {
                ui.label(content);
            }
        });
        ui.add_space(4.0);
    }

    ui.heading(format!("Servers ({})", overview.servers.len()));
    if overview.servers.is_empty() {
        ui.label("No servers are linked to this game panel account.");
    }
    for server in overview.servers {
        let selected = state.nexeu.selected_server.as_deref() == Some(&server.uuid);
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&server.name).strong().color(TEXT));
                if let Some(status) = &server.status {
                    ui.label(status);
                }
                if let Some(allocation) = &server.allocation {
                    let host = allocation
                        .ip_alias
                        .as_deref()
                        .or(allocation.ip.as_deref())
                        .unwrap_or("");
                    if !host.is_empty() {
                        ui.label(format!("{}:{}", host, allocation.port.unwrap_or(25565)));
                    }
                }
            });
            if let Some(description) = &server.description {
                if !description.is_empty() {
                    ui.label(description);
                }
            }
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(if selected { "Refresh usage" } else { "Details" })
                    .clicked()
                {
                    state.nexeu_select_server(server.uuid.clone());
                }
                if selected {
                    ui.menu_button("Power", |ui| {
                        for (label, action) in
                            [("Start", "start"), ("Stop", "stop"), ("Restart", "restart")]
                        {
                            if ui.button(label).clicked() {
                                state.nexeu_power(server.uuid.clone(), action);
                                ui.close();
                            }
                        }
                    });
                    if ui.button("Logs").clicked() {
                        state.nexeu_load_logs(server.uuid.clone());
                    }
                }
                ui.hyperlink_to("Open panel", "https://game.nexeu.zip/");
            });
            if selected {
                if let Some(resources) = &state.nexeu.resources {
                    let memory = resources
                        .get("memory_bytes")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let disk = resources
                        .get("disk_bytes")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let cpu = resources
                        .get("cpu_absolute")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0);
                    let running = resources
                        .get("state")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown");
                    ui.label(format!(
                        "{running}  •  RAM {:.1} GiB  •  Disk {:.1} GiB  •  CPU {:.1}%",
                        memory as f64 / 1_073_741_824.0,
                        disk as f64 / 1_073_741_824.0,
                        cpu
                    ));
                }
                if let Some(logs) = &state.nexeu.logs {
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            ui.monospace(logs);
                        });
                }
                ui.separator();
                ui.label(RichText::new("Console command").strong());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.nexeu.console_command)
                            .hint_text("e.g. say Hello players")
                            .desired_width(300.0),
                    );
                    if ui.button("Send").clicked() {
                        state.nexeu_send_command(server.uuid.clone());
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Backups").strong());
                    if ui.button("Refresh").clicked() {
                        state.nexeu_load_backups(server.uuid.clone());
                    }
                    if ui.button("Create backup").clicked() {
                        state.nexeu_create_backup(server.uuid.clone());
                    }
                });
                if let Some(backups) = state.nexeu.backups.clone() {
                    if backups.is_empty() {
                        ui.label("No backups yet.");
                    }
                    for backup in backups {
                        let status = if backup.is_successful == Some(true) {
                            "Ready"
                        } else {
                            "Processing"
                        };
                        let size = backup
                            .bytes
                            .map(crate::ui::theme::format_bytes)
                            .unwrap_or_default();
                        ui.label(format!(
                            "{} · {} · {} · {}",
                            backup.name,
                            status,
                            size,
                            backup.created.as_deref().unwrap_or("")
                        ));
                    }
                }
            }
        });
        ui.add_space(6.0);
    }
}
