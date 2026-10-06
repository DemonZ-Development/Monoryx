use crate::minecraft::activity::GameActivity;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const APPLICATION_ID: &str = "1553781209744674916";
const LAUNCHER_URL: &str = crate::utils::links::PROJECT_URL;
const DEVELOPER_URL: &str = "https://demonz.org/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub show_launcher: bool,
    pub show_instance: bool,
    pub show_world: bool,
    pub show_server: bool,
    pub show_elapsed: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_launcher: true,
            show_instance: true,
            show_world: true,
            show_server: true,
            show_elapsed: true,
        }
    }
}

pub struct Session<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub loader: &'a str,
    pub activity: &'a GameActivity,
    pub started: i64,
}

fn text(value: &str) -> String {
    let mut result = String::new();
    let mut formatting = false;
    for ch in value.chars() {
        if ch == '§' {
            formatting = true;
            continue;
        }
        if formatting {
            formatting = false;
            continue;
        }
        if ch.is_control() {
            continue;
        }
        if result.len() + ch.len_utf8() > 120 {
            break;
        }
        result.push(ch);
    }
    result.trim().to_string()
}

pub fn activity(settings: &Settings, page: &str, session: Option<Session<'_>>) -> Option<Value> {
    if session.is_none() && !settings.show_launcher {
        return None;
    }
    let mut result = if let Some(session) = session {
        let details = match session.activity {
            GameActivity::Loading => "Starting Minecraft".to_string(),
            GameActivity::Menu => "In the Minecraft menus".to_string(),
            GameActivity::Singleplayer(Some(name)) if settings.show_world => {
                format!("Playing in {}", text(name))
            }
            GameActivity::Singleplayer(_) => "Playing singleplayer".to_string(),
            GameActivity::Connecting(_) => "Connecting to a server".to_string(),
            GameActivity::Server { name, address } if settings.show_server => {
                format!("Playing on {}", text(name.as_deref().unwrap_or(address)))
            }
            GameActivity::Server { .. } => "Playing multiplayer".to_string(),
        };
        let state = if settings.show_instance {
            format!(
                "{} · {} · {}",
                session.name, session.version, session.loader
            )
        } else {
            format!("Minecraft {} · {}", session.version, session.loader)
        };
        let mut activity = json!({ "type": 0, "details": text(&details), "state": text(&state), "instance": false });
        if settings.show_elapsed {
            activity["timestamps"] = json!({ "start": session.started });
        }
        activity
    } else {
        json!({ "type": 0, "details": "Getting ready to play", "state": text(&format!("MONORYX · {page}")), "instance": false })
    };
    result["buttons"] = json!([
        { "label": "Get MONORYX", "url": LAUNCHER_URL },
        { "label": "DemonZ Development", "url": DEVELOPER_URL },
    ]);
    Some(result)
}

#[derive(Clone)]
pub struct Presence {
    sender: tokio::sync::watch::Sender<Option<Value>>,
    status: Arc<Mutex<String>>,
    model: Arc<Mutex<PresenceModel>>,
}

#[derive(Default)]
struct PresenceModel {
    settings: Settings,
    page: String,
    games: std::collections::HashMap<String, OwnedSession>,
}

struct OwnedSession {
    name: String,
    version: String,
    loader: String,
    activity: GameActivity,
    started: i64,
}

impl PresenceModel {
    fn activity(&self) -> Option<Value> {
        if !self.settings.enabled {
            return None;
        }
        let session = self
            .games
            .values()
            .max_by_key(|game| game.started)
            .map(|game| Session {
                name: &game.name,
                version: &game.version,
                loader: &game.loader,
                activity: &game.activity,
                started: game.started,
            });
        activity(&self.settings, &self.page, session)
    }
}
impl Presence {
    pub fn start(runtime: &tokio::runtime::Runtime) -> Self {
        let (sender, receiver) = tokio::sync::watch::channel(None);
        let status = Arc::new(Mutex::new("Off".to_string()));
        runtime.spawn(run(receiver, status.clone()));
        Self {
            sender,
            status,
            model: Arc::new(Mutex::new(PresenceModel::default())),
        }
    }
    fn change(&self, change: impl FnOnce(&mut PresenceModel) -> bool) {
        let Ok(mut model) = self.model.lock() else {
            return;
        };
        if !change(&mut model) {
            return;
        }
        let activity = model.activity();
        self.sender.send_if_modified(|current| {
            if *current == activity {
                false
            } else {
                *current = activity;
                true
            }
        });
    }

    pub fn configure(&self, settings: &Settings, page: &str) {
        self.change(|model| {
            if model.settings == *settings && model.page == page {
                return false;
            }
            model.settings.clone_from(settings);
            if model.page != page {
                model.page = page.to_string();
            }
            true
        });
    }

    pub fn game_started(&self, id: &str, name: &str, version: &str, loader: &str) {
        self.change(|model| {
            model.games.insert(
                id.to_string(),
                OwnedSession {
                    name: name.to_string(),
                    version: version.to_string(),
                    loader: loader.to_string(),
                    activity: GameActivity::Loading,
                    started: chrono::Utc::now().timestamp(),
                },
            );
            true
        });
    }

    pub fn game_activity(&self, id: &str, activity: GameActivity) {
        self.change(|model| {
            if let Some(game) = model.games.get_mut(id) {
                if game.activity != activity {
                    game.activity = activity;
                    return true;
                }
            }
            false
        });
    }

    pub fn game_finished(&self, id: &str) {
        self.change(|model| model.games.remove(id).is_some());
    }
    pub fn status(&self) -> String {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| "Unavailable".into())
    }
}

trait Ipc: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Ipc for T {}
type Connection = Box<dyn Ipc>;
type IoResult<T> = std::io::Result<T>;

async fn connect() -> IoResult<Connection> {
    #[cfg(windows)]
    for index in 0..10 {
        if let Ok(pipe) = tokio::net::windows::named_pipe::ClientOptions::new()
            .open(format!(r"\\.\pipe\discord-ipc-{index}"))
        {
            return Ok(Box::new(pipe));
        }
    }
    #[cfg(unix)]
    {
        let mut dirs: Vec<_> = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"]
            .iter()
            .filter_map(std::env::var_os)
            .map(std::path::PathBuf::from)
            .collect();
        dirs.push(std::path::PathBuf::from("/tmp"));
        for dir in dirs {
            for index in 0..10 {
                if let Ok(socket) =
                    tokio::net::UnixStream::connect(dir.join(format!("discord-ipc-{index}"))).await
                {
                    return Ok(Box::new(socket));
                }
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "Open Discord on this computer to show your activity.",
    ))
}

async fn write_frame(
    writer: &mut (impl AsyncWrite + Unpin),
    opcode: u32,
    data: &Value,
) -> IoResult<()> {
    let bytes = serde_json::to_vec(data)?;
    let mut frame = Vec::with_capacity(bytes.len() + 8);
    frame.extend_from_slice(&opcode.to_le_bytes());
    frame.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    frame.extend_from_slice(&bytes);
    tokio::time::timeout(std::time::Duration::from_secs(3), writer.write_all(&frame)).await??;
    Ok(())
}

async fn read_frame(reader: &mut (impl AsyncRead + Unpin)) -> IoResult<(u32, Value)> {
    let mut header = [0_u8; 8];
    reader.read_exact(&mut header).await?;
    let opcode = u32::from_le_bytes(header[..4].try_into().unwrap());
    let len = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
    if len > 64 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Discord frame too large",
        ));
    }
    let mut payload = vec![0; len];
    reader.read_exact(&mut payload).await?;
    Ok((opcode, serde_json::from_slice(&payload)?))
}

fn set_status(status: &Mutex<String>, message: &str) {
    if let Ok(mut status) = status.lock() {
        *status = message.into();
    }
}

async fn run(
    mut receiver: tokio::sync::watch::Receiver<Option<Value>>,
    status: Arc<Mutex<String>>,
) {
    loop {
        if receiver.has_changed().is_err() {
            return;
        }
        if receiver.borrow().is_none() {
            set_status(&status, "Off");
            if receiver.changed().await.is_err() {
                return;
            }
            continue;
        }
        match connected(&mut receiver, &status).await {
            Ok(()) => continue,
            Err(error) => set_status(&status, &error.to_string()),
        }
        tokio::select! {
            changed = receiver.changed() => if changed.is_err() { return; },
            _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {},
        }
    }
}

async fn connected(
    receiver: &mut tokio::sync::watch::Receiver<Option<Value>>,
    status: &Mutex<String>,
) -> IoResult<()> {
    let pipe = connect().await?;
    connected_with(pipe, receiver, status).await
}

async fn connected_with(
    mut pipe: Connection,
    receiver: &mut tokio::sync::watch::Receiver<Option<Value>>,
    status: &Mutex<String>,
) -> IoResult<()> {
    set_status(status, "Connecting to Discord…");
    write_frame(
        &mut pipe,
        0,
        &json!({ "v": 1, "client_id": APPLICATION_ID }),
    )
    .await?;
    let (_, ready) =
        tokio::time::timeout(std::time::Duration::from_secs(5), read_frame(&mut pipe)).await??;
    if ready["evt"] != "READY" {
        return Err(std::io::Error::other(
            "Discord didn't accept the application. Check the Application ID.",
        ));
    }
    let (mut reader, mut writer) = tokio::io::split(pipe);
    let (tx, mut frames) = tokio::sync::mpsc::channel(8);
    let reader_task = tokio::spawn(async move {
        loop {
            let frame = read_frame(&mut reader).await;
            let ended = frame.is_err();
            if tx.send(frame).await.is_err() || ended {
                break;
            }
        }
    });
    let outcome = async {
        let mut dirty = true;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            if receiver.borrow().is_none() {
                write_frame(&mut writer, 1, &command(None)).await?;
                return Ok(());
            }
            tokio::select! {
                changed = receiver.changed() => {
                    if changed.is_err() { let _ = write_frame(&mut writer, 1, &command(None)).await; return Ok(()); }
                    dirty = true;
                }
                _ = interval.tick() => if dirty {
                    let value = receiver.borrow().clone();
                    write_frame(&mut writer, 1, &command(value)).await?;
                    dirty = false;
                },
                frame = frames.recv() => {
                    let (opcode, payload) = frame.ok_or_else(|| std::io::Error::other("Discord disconnected"))??;
                    match opcode {
                        1 if payload["evt"] == "ERROR" => return Err(std::io::Error::other(format!("Discord: {}", payload["data"]["message"].as_str().unwrap_or("Activity was rejected")))),
                        1 if payload["cmd"] == "SET_ACTIVITY" => set_status(status, "Connected · your activity is visible"),
                        2 => return Err(std::io::Error::other("Discord disconnected. Reconnecting…")),
                        3 => write_frame(&mut writer, 4, &payload).await?,
                        _ => {},
                    }
                }
            }
        }
    }.await;
    reader_task.abort();
    outcome
}

fn command(activity: Option<Value>) -> Value {
    json!({ "cmd": "SET_ACTIVITY", "args": { "pid": std::process::id(), "activity": activity }, "nonce": uuid::Uuid::new_v4().to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_updates_publish_without_ui_frames_and_respect_current_privacy() {
        let (sender, receiver) = tokio::sync::watch::channel(None);
        let presence = Presence {
            sender,
            status: Arc::new(Mutex::new(String::new())),
            model: Arc::new(Mutex::new(PresenceModel::default())),
        };
        let mut settings = Settings {
            enabled: true,
            ..Default::default()
        };
        presence.configure(&settings, "Home");
        presence.game_started("test", "My instance", "26.3", "Fabric");
        assert_eq!(
            receiver.borrow().as_ref().unwrap()["details"],
            "Starting Minecraft"
        );
        let game_worker = presence.clone();
        std::thread::spawn(move || {
            game_worker.game_activity(
                "test",
                GameActivity::Server {
                    address: "private.example:25565".into(),
                    name: Some("Friends' server".into()),
                },
            )
        })
        .join()
        .unwrap();
        assert_eq!(
            receiver.borrow().as_ref().unwrap()["details"],
            "Playing on Friends' server"
        );
        presence.configure(&settings, "Home");
        assert_eq!(
            receiver.borrow().as_ref().unwrap()["details"],
            "Playing on Friends' server"
        );
        settings.show_server = false;
        presence.configure(&settings, "Settings");
        assert_eq!(
            receiver.borrow().as_ref().unwrap()["details"],
            "Playing multiplayer"
        );
        settings.enabled = false;
        presence.configure(&settings, "Settings");
        presence.game_activity(
            "test",
            GameActivity::Singleplayer(Some("Private world".into())),
        );
        assert!(receiver.borrow().is_none());
        presence.game_finished("test");
        settings.enabled = true;
        presence.configure(&settings, "Home");
        assert_eq!(
            receiver.borrow().as_ref().unwrap()["details"],
            "Getting ready to play"
        );
    }

    #[tokio::test]
    async fn handshake_ping_and_disabling_presence_use_the_discord_protocol() {
        tokio::time::timeout(std::time::Duration::from_secs(4), async {
            let (client, mut server) = tokio::io::duplex(8192);
            let (settings, mut receiver) =
                tokio::sync::watch::channel(Some(json!({"details":"Playing singleplayer"})));
            let task = tokio::spawn(async move {
                connected_with(Box::new(client), &mut receiver, &Mutex::new(String::new())).await
            });
            let (opcode, handshake) = read_frame(&mut server).await.unwrap();
            assert_eq!(opcode, 0);
            assert_eq!(handshake["client_id"], APPLICATION_ID);
            write_frame(&mut server, 1, &json!({"evt":"READY"}))
                .await
                .unwrap();
            let (_, presence) = read_frame(&mut server).await.unwrap();
            assert_eq!(presence["cmd"], "SET_ACTIVITY");
            assert_eq!(
                presence["args"]["activity"]["details"],
                "Playing singleplayer"
            );
            write_frame(&mut server, 3, &json!({"ping":42}))
                .await
                .unwrap();
            assert_eq!(
                read_frame(&mut server).await.unwrap(),
                (4, json!({"ping":42}))
            );
            settings.send(None).unwrap();
            let (_, clear) = read_frame(&mut server).await.unwrap();
            assert!(clear["args"]["activity"].is_null());
            task.await.unwrap().unwrap();
        })
        .await
        .unwrap();
    }
    #[test]
    fn privacy_switches_remove_names_and_timestamps_everywhere() {
        let settings = Settings {
            show_world: false,
            show_server: false,
            show_instance: false,
            show_elapsed: false,
            ..Default::default()
        };
        for activity_state in [
            GameActivity::Singleplayer(Some("Private world".into())),
            GameActivity::Server {
                address: "secret.example".into(),
                name: Some("Private server".into()),
            },
        ] {
            let result = activity(
                &settings,
                "Home",
                Some(Session {
                    name: "Private pack",
                    version: "1.21.1",
                    loader: "Fabric",
                    activity: &activity_state,
                    started: 100,
                }),
            )
            .unwrap()
            .to_string();
            assert!(!result.contains("Private"));
            assert!(!result.contains("secret"));
            assert!(!result.contains("timestamps"));
        }
    }
    #[test]
    fn profile_buttons_use_the_project_and_developer_urls() {
        let settings = Settings::default();
        let buttons = activity(&settings, "Home", None).unwrap()["buttons"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(buttons.len(), 2);
        assert_eq!(buttons[0]["url"], LAUNCHER_URL);
        assert_eq!(buttons[1]["url"], DEVELOPER_URL);
        assert!(activity(
            &Settings {
                show_launcher: false,
                ..settings
            },
            "Home",
            None
        )
        .is_none());
    }

    #[test]
    fn settings_default_on_and_old_button_fields_are_ignored() {
        let migrated: crate::config::LauncherConfig = toml::from_str("last_page = 'home'").unwrap();
        assert!(migrated.discord.enabled);
        let old: Settings = toml::from_str("enabled = true\nlauncher_button = false\nbutton_label = 'Old button'\nbutton_url = 'https://example.com'").unwrap();
        assert!(old.enabled);
        let settings = Settings {
            enabled: true,
            show_world: false,
            ..Default::default()
        };
        let roundtrip: Settings = toml::from_str(&toml::to_string(&settings).unwrap()).unwrap();
        assert_eq!(settings, roundtrip);
    }
    #[tokio::test]
    async fn ipc_frames_roundtrip_and_reject_oversized_payloads() {
        let (mut client, mut server) = tokio::io::duplex(2048);
        let task = tokio::spawn(async move {
            write_frame(&mut client, 1, &command(None)).await.unwrap();
        });
        let (opcode, message) = read_frame(&mut server).await.unwrap();
        assert_eq!(opcode, 1);
        assert!(message["args"]["activity"].is_null());
        task.await.unwrap();
        let (mut client, mut server) = tokio::io::duplex(8);
        client
            .write_all(&[1, 0, 0, 0, 255, 255, 255, 127])
            .await
            .unwrap();
        assert!(read_frame(&mut server).await.is_err());
    }
}
