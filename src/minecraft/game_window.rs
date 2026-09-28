use super::activity::{self, GameActivity};
use std::path::Path;

#[derive(Clone)]
pub struct GameWindow {
    process_id: Option<u32>,
    renamed: Option<(usize, String)>,
    session_title_seen: bool,
}

impl GameWindow {
    pub fn new(process_id: Option<u32>) -> Self {
        Self {
            process_id,
            renamed: None,
            session_title_seen: false,
        }
    }

    pub fn update(&mut self, game: &Path, mut activity: GameActivity) -> GameActivity {
        let window = self.process_id.and_then(find_window);
        if let Some(window) = &window {
            let ours = self
                .renamed
                .as_ref()
                .is_some_and(|(handle, title)| *handle == window.handle && title == &window.title);
            if !ours {
                observe_title(&window.title, &mut activity, self.session_title_seen);
                if window.title.contains(" - Multiplayer")
                    || window.title.contains(" - Singleplayer")
                {
                    self.session_title_seen = true;
                } else if is_game_title(&window.title) && !window.title.contains(" - ") {
                    self.session_title_seen = false;
                }
                self.renamed = None;
            }
        }
        let activity = activity::resolve(game, activity);
        if let (
            Some(window),
            GameActivity::Server {
                name: Some(name), ..
            },
        ) = (window, &activity)
        {
            if let Some(title) = named_server_title(&window.title, name) {
                if set_title(&window, &title) {
                    self.renamed = Some((window.handle, title));
                }
            }
        }
        activity
    }
}

fn observe_title(title: &str, activity: &mut GameActivity, session_title_seen: bool) {
    if title.contains(" - Multiplayer") {
        if let GameActivity::Connecting(address) = activity {
            *activity = GameActivity::Server {
                address: address.clone(),
                name: None,
            };
        }
    } else if title.contains(" - Singleplayer") {
        if !matches!(activity, GameActivity::Singleplayer(_)) {
            *activity = GameActivity::Singleplayer(None);
        }
    } else if session_title_seen
        && is_game_title(title)
        && !title.contains(" - ")
        && matches!(
            activity,
            GameActivity::Server { .. } | GameActivity::Singleplayer(_)
        )
    {
        *activity = GameActivity::Menu;
    }
}

fn is_game_title(title: &str) -> bool {
    title == "Minecraft" || title.starts_with("Minecraft ") || title.starts_with("Minecraft* ")
}

fn named_server_title(title: &str, name: &str) -> Option<String> {
    let marker = [
        "3rd-party Server",
        "Third-party Server",
        "third-party server",
    ]
    .into_iter()
    .find(|marker| title.contains(marker))?;
    if !is_game_title(title) || !title.contains(" - Multiplayer") {
        return None;
    }
    let mut clean = String::new();
    let mut formatting = false;
    for ch in name.chars().take(256) {
        if ch == '§' {
            formatting = true;
            continue;
        }
        if formatting {
            formatting = false;
            continue;
        }
        if !ch.is_control() {
            clean.push(ch);
        }
    }
    let clean = clean.trim();
    if clean.is_empty() || clean == marker {
        return None;
    }
    Some(title.replacen(marker, clean, 1))
}

struct Window {
    handle: usize,
    title: String,
    process_id: u32,
}

#[cfg(target_os = "windows")]
fn find_window(process_id: u32) -> Option<Window> {
    use windows_sys::Win32::{
        Foundation::{BOOL, HWND, LPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindow, GetWindowTextW, GetWindowThreadProcessId, GW_OWNER,
        },
    };
    struct Search {
        process_id: u32,
        found: Option<Window>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, param: LPARAM) -> BOOL {
        let search = &mut *(param as *mut Search);
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == search.process_id && GetWindow(hwnd, GW_OWNER).is_null() {
            let mut buffer = [0u16; 1024];
            let len = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
            let title = String::from_utf16_lossy(&buffer[..len.max(0) as usize]);
            if is_game_title(&title) {
                search.found = Some(Window {
                    handle: hwnd as usize,
                    title,
                    process_id: pid,
                });
                return 0;
            }
        }
        1
    }
    let mut search = Search {
        process_id,
        found: None,
    };
    unsafe {
        EnumWindows(Some(visit), &mut search as *mut Search as LPARAM);
    }
    search.found
}

#[cfg(target_os = "windows")]
fn set_title(window: &Window, title: &str) -> bool {
    use windows_sys::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            GetWindowThreadProcessId, SendMessageTimeoutW, SMTO_ABORTIFHUNG, SMTO_BLOCK, WM_SETTEXT,
        },
    };
    let hwnd = window.handle as HWND;
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
    }
    if pid != window.process_id {
        return false;
    }
    let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    let mut result = 0;
    unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_SETTEXT,
            0,
            title.as_ptr() as isize,
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            100,
            &mut result,
        ) != 0
            && result != 0
    }
}

#[cfg(not(target_os = "windows"))]
fn find_window(_process_id: u32) -> Option<Window> {
    None
}

#[cfg(not(target_os = "windows"))]
fn set_title(window: &Window, _title: &str) -> bool {
    let _ = window.process_id;
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn updates_only_the_supervised_window_and_tracks_its_return_to_menu() {
        use windows_sys::Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_OVERLAPPEDWINDOW},
        };
        struct Fixture(HWND);
        impl Drop for Fixture {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        let class: Vec<u16> = "STATIC".encode_utf16().chain(Some(0)).collect();
        let title: Vec<u16> = "Minecraft* 26.3 - Multiplayer (3rd-party Server)"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let fixture = Fixture(unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                100,
                100,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        });
        assert!(!fixture.0.is_null());
        assert!(find_window(u32::MAX).is_none());
        let dir = tempfile::tempdir().unwrap();
        let data = b"\x0a\x00\x00\x09\x00\x07servers\x0a\x00\x00\x00\x01\x08\x00\x04name\x00\x07Friends\x08\x00\x02ip\x00\x0cmc.test.host\x00\x00";
        std::fs::write(dir.path().join("servers.dat"), data).unwrap();
        let pid = std::process::id();
        let mut tracker = GameWindow::new(Some(pid));
        let playing = tracker.update(
            dir.path(),
            GameActivity::Connecting("mc.test.host:25565".into()),
        );
        assert!(
            matches!(&playing, GameActivity::Server { name: Some(name), .. } if name == "Friends")
        );
        let window = find_window(pid).unwrap();
        assert_eq!(window.handle, fixture.0 as usize);
        assert_eq!(window.title, "Minecraft* 26.3 - Multiplayer (Friends)");
        assert_eq!(tracker.update(dir.path(), playing.clone()), playing);
        let wrong_pid = Window {
            process_id: u32::MAX,
            handle: window.handle,
            title: window.title.clone(),
        };
        assert!(!set_title(&wrong_pid, "Wrong process"));
        assert!(set_title(&window, "Minecraft* 26.3"));
        assert_eq!(tracker.update(dir.path(), playing), GameActivity::Menu);
    }

    #[test]
    fn replaces_only_the_generic_server_label_with_the_saved_name() {
        assert_eq!(
            named_server_title(
                "Minecraft* 26.3 - Multiplayer (3rd-party Server)",
                "§aFriends 🌍\n"
            ),
            Some("Minecraft* 26.3 - Multiplayer (Friends 🌍)".into())
        );
        assert_eq!(
            named_server_title("Minecraft 26.3 - Multiplayer (Realms)", "Friends"),
            None
        );
        assert_eq!(
            named_server_title("Another app - Multiplayer (3rd-party Server)", "Friends"),
            None
        );
        assert_eq!(
            named_server_title("Minecraft 26.3 - Multiplayer (3rd-party Server)", "§a\n"),
            None
        );
    }

    #[test]
    fn native_title_confirms_play_and_detects_menu_without_renderer_logs() {
        let mut state = GameActivity::Connecting("mc.example:25565".into());
        observe_title("Minecraft* 26.3", &mut state, false);
        assert!(matches!(state, GameActivity::Connecting(_)));
        observe_title(
            "Minecraft* 26.3 - Multiplayer (3rd-party Server)",
            &mut state,
            false,
        );
        assert!(matches!(state, GameActivity::Server { .. }));
        observe_title("Minecraft* 26.3", &mut state, false);
        assert!(
            matches!(state, GameActivity::Server { .. }),
            "A game that never supplied a session title must not lose its log-detected activity"
        );
        observe_title("Minecraft* 26.3", &mut state, true);
        assert_eq!(state, GameActivity::Menu);
    }
}
