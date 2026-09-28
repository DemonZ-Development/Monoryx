use std::{io::Read, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GameActivity {
    #[default]
    Loading,
    Menu,
    Singleplayer(Option<String>),
    Connecting(String),
    Server {
        address: String,
        name: Option<String>,
    },
}

impl GameActivity {
    pub fn observe(&mut self, raw: &str) {
        if raw.contains("[CHAT]") || raw.len() > 16_384 {
            return;
        }
        let line = if let Some((_, message)) = raw.split_once("<![CDATA[") {
            message.split("]]>").next().unwrap_or(message)
        } else {
            raw
        };
        let lower = line.to_lowercase();
        if lower.contains("stopping server")
            || lower.contains("disconnecting from")
            || lower.contains("lost connection:")
            || lower.contains("couldn't connect to server")
            || lower.contains("failed to connect to the server")
            || lower.trim() == "stopping!"
            || lower.ends_with(": stopping!")
        {
            *self = Self::Menu;
        } else if let Some((_, address)) = line.split_once("Connecting to ") {
            let address = address.trim();
            if let Some((host, port)) = address.rsplit_once(", ") {
                if port.parse::<u16>().is_ok() {
                    *self = Self::Connecting(format!("{host}:{port}"));
                }
            }
        } else if lower.contains("starting integrated minecraft server")
            || lower.contains("starting integrated server")
        {
            *self = Self::Singleplayer(None);
        } else if lower.contains("loaded ") && lower.trim_end().ends_with(" advancements") {
            if let Self::Connecting(address) = self {
                *self = Self::Server {
                    address: address.clone(),
                    name: None,
                };
            }
        } else if lower.contains("sound engine started") && matches!(self, Self::Loading) {
            *self = Self::Menu;
        }
    }
}

pub fn resolve(game: &Path, activity: GameActivity) -> GameActivity {
    match activity {
        GameActivity::Singleplayer(_) => GameActivity::Singleplayer(active_world_name(game)),
        GameActivity::Server { address, .. } => {
            let name = server_name(game, &address);
            GameActivity::Server { address, name }
        }
        other => other,
    }
}

fn server_name(game: &Path, address: &str) -> Option<String> {
    let data = super::nbt::read_file(&game.join("servers.dat"), false)?;
    let super::nbt::Value::List(servers) = data.get("servers")? else {
        return None;
    };
    servers
        .iter()
        .find(|server| {
            server
                .get("ip")
                .and_then(super::nbt::Value::text)
                .is_some_and(|ip| normalize_address(ip) == normalize_address(address))
        })
        .and_then(|server| server.get("name"))
        .and_then(super::nbt::Value::text)
        .map(str::to_string)
}

fn normalize_address(address: &str) -> String {
    address
        .trim()
        .to_lowercase()
        .trim_end_matches(":25565")
        .to_string()
}

fn active_world_name(game: &Path) -> Option<String> {
    if !cfg!(windows) {
        return None;
    }
    for entry in std::fs::read_dir(game.join("saves"))
        .ok()?
        .flatten()
        .take(2000)
    {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let root = entry.path();
        let locked = std::fs::File::open(root.join("session.lock"))
            .and_then(|mut file| file.read_exact(&mut [0_u8; 1]))
            .is_err_and(|error| error.raw_os_error() == Some(33));
        if locked {
            let name = super::nbt::read_file(&root.join("level.dat"), true).and_then(|data| {
                data.get("Data")?
                    .get("LevelName")?
                    .text()
                    .map(str::to_string)
            });
            return name.or_else(|| Some(entry.file_name().to_string_lossy().to_string()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fabric_xml_render_reloads_do_not_leave_the_server() {
        let mut activity = GameActivity::Loading;
        for message in [
            "Sound engine started",
            "Connecting to mc.example.net, 30012",
            "Loaded 945 advancements",
            "Stopping worker threads",
            "Started 10 worker threads",
            "Sound engine started",
            "Stopping worker threads",
        ] {
            activity.observe(&format!(
                "[STDOUT] <log4j:Message><![CDATA[{message}]]></log4j:Message>"
            ));
        }
        assert_eq!(
            activity,
            GameActivity::Server {
                address: "mc.example.net:30012".into(),
                name: None
            }
        );
        activity.observe("[STDOUT] <log4j:Message><![CDATA[Stopping!]]></log4j:Message>");
        assert_eq!(activity, GameActivity::Menu);
    }

    #[test]
    #[ignore = "Read-only replay of a local session selected via MONORYX_ACTIVITY_REPLAY"]
    fn replay_local_session_and_saved_server_name() {
        let log = std::path::PathBuf::from(
            std::env::var_os("MONORYX_ACTIVITY_REPLAY").expect("set a session log path"),
        );
        let game = log.parent().unwrap().parent().unwrap();
        let mut observed = GameActivity::Loading;
        let mut saw_named_server = false;
        let mut retained_server_during_reload = false;
        for line in std::fs::read_to_string(&log).unwrap().lines() {
            let was_playing = matches!(observed, GameActivity::Server { .. });
            observed.observe(line);
            if was_playing && line.contains("Stopping worker threads") {
                assert!(matches!(observed, GameActivity::Server { .. }));
                retained_server_during_reload = true;
            }
            if matches!(observed, GameActivity::Server { .. }) {
                let resolved = resolve(game, observed.clone());
                assert!(
                    matches!(resolved, GameActivity::Server { name: Some(ref name), .. } if !name.trim().is_empty()),
                    "Connected server must match its saved server-list name"
                );
                saw_named_server = true;
            }
        }
        assert!(saw_named_server && retained_server_during_reload);
        assert_eq!(observed, GameActivity::Menu);
    }

    #[cfg(windows)]
    #[test]
    fn identifies_the_locked_world_without_selecting_a_recent_but_inactive_save() {
        use std::os::windows::io::AsRawHandle;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn LockFile(
                handle: *mut std::ffi::c_void,
                low: u32,
                high: u32,
                bytes_low: u32,
                bytes_high: u32,
            ) -> i32;
        }
        let dir = tempfile::tempdir().unwrap();
        for name in ["Inactive", "Playing now"] {
            std::fs::create_dir_all(dir.path().join("saves").join(name)).unwrap();
            std::fs::write(
                dir.path().join("saves").join(name).join("session.lock"),
                b"lock",
            )
            .unwrap();
        }
        assert_eq!(active_world_name(dir.path()), None);
        let path = dir.path().join("saves/Playing now/session.lock");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap();
        assert_ne!(unsafe { LockFile(file.as_raw_handle(), 0, 0, 1, 0) }, 0);
        assert_eq!(
            active_world_name(dir.path()).as_deref(),
            Some("Playing now")
        );
        drop(file);
        assert_eq!(active_world_name(dir.path()), None);
    }

    #[test]
    fn resolves_saved_server_name_with_the_default_port() {
        let dir = tempfile::tempdir().unwrap();
        let data = b"\x0a\x00\x00\x09\x00\x07servers\x0a\x00\x00\x00\x01\x08\x00\x04name\x00\x07Friends\x08\x00\x02ip\x00\x0cmc.test.host\x00\x00";
        std::fs::write(dir.path().join("servers.dat"), data).unwrap();
        assert_eq!(
            server_name(dir.path(), "mc.test.host:25565").as_deref(),
            Some("Friends")
        );
        assert_eq!(server_name(dir.path(), "another.host"), None);
    }
    #[test]
    fn tracks_server_connection_confirmation_and_disconnect() {
        let mut activity = GameActivity::Menu;
        activity.observe("[Render thread/INFO]: Connecting to mc.example.com, 25565");
        assert_eq!(
            activity,
            GameActivity::Connecting("mc.example.com:25565".into())
        );
        activity.observe("[Render thread/INFO]: Loaded 64 advancements");
        assert!(matches!(activity, GameActivity::Server { .. }));
        activity.observe("[Render thread/INFO]: Stopping worker threads");
        activity.observe("[Render thread/INFO]: Started 10 worker threads");
        activity.observe("[Render thread/INFO]: Sound engine started");
        assert!(matches!(activity, GameActivity::Server { .. }));
        activity.observe("[Render thread/INFO]: Disconnecting from server");
        assert_eq!(activity, GameActivity::Menu);
        activity.observe("[CHAT] Connecting to secret.example, 25565");
        assert_eq!(activity, GameActivity::Menu);
    }
    #[test]
    fn handles_xml_logs_and_failed_connect_without_claiming_play() {
        let mut activity = GameActivity::Loading;
        activity.observe("<log4j:Message><![CDATA[Starting integrated minecraft server version 1.21.1]]></log4j:Message>");
        assert_eq!(activity, GameActivity::Singleplayer(None));
        activity.observe("Stopping server");
        activity.observe("Connecting to example.net, 25565");
        activity.observe("Couldn't connect to server");
        assert_eq!(activity, GameActivity::Menu);
    }
}
