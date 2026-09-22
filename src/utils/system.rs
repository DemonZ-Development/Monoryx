#[must_use]
pub fn total_memory_mb() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.total_memory() / 1024 / 1024
}

#[must_use]
pub fn available_memory_mb() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.available_memory() / 1024 / 1024
}

#[must_use]
pub fn default_max_memory_mb() -> u64 {
    let total = total_memory_mb();
    if total == 0 {
        return 2048;
    }

    (total / 4).clamp(1024, 3072)
}

#[must_use]
pub fn default_boost_max_memory_mb() -> u64 {
    let total = total_memory_mb();
    if total == 0 {
        return 2048;
    }

    (total / 5).clamp(1024, 2560)
}

#[must_use]
pub const fn default_min_memory_mb() -> u64 {
    512
}

pub fn validate_memory(min_mb: u64, max_mb: u64) -> Result<Option<String>, String> {
    if min_mb < 256 {
        return Err("Minimum memory must be at least 256 MB.".to_string());
    }
    if max_mb < 512 {
        return Err("Maximum memory must be at least 512 MB.".to_string());
    }
    if max_mb < min_mb {
        return Err("Maximum memory must be >= minimum memory.".to_string());
    }
    let total = total_memory_mb();
    if total > 0 && max_mb > total {
        return Ok(Some(format!(
            "Maximum ({max_mb} MB) exceeds total system memory ({total} MB)."
        )));
    }
    if total > 0 && max_mb * 2 > total * 3 / 2 {
        return Ok(Some(
            "Allocating most of your system RAM can cause instability.".to_string(),
        ));
    }
    Ok(None)
}

#[must_use]
pub fn mojang_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

#[must_use]
pub fn mojang_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuInfo {
    pub name: String,
    pub dedicated: bool,
}

#[must_use]
pub fn is_dedicated_gpu_name(name: &str) -> bool {
    let n = name.to_lowercase();
    if n.contains("basic display")
        || n.contains("basicdisplay")
        || n.contains("vmware")
        || n.contains("virtualbox")
        || n.contains("hyper-v")
        || n.contains("parallels")
        || n.contains("virtio")
        || n.contains("remote")
    {
        return false;
    }
    if n.contains("nvidia") || n.contains("arc") {
        return true;
    }
    if n.contains("radeon") {
        return n.contains(" rx")
            || n.contains("rx ")
            || n.contains("rx-")
            || n.contains(" pro")
            || n.contains("vii")
            || n.contains("w6")
            || n.contains("w7");
    }
    false
}

static GPU_CACHE: std::sync::RwLock<Option<Vec<GpuInfo>>> = std::sync::RwLock::new(None);

pub async fn detect_gpus() -> Vec<GpuInfo> {
    if let Ok(guard) = GPU_CACHE.read() {
        if let Some(list) = guard.as_ref() {
            return list.clone();
        }
    }
    detect_gpus_force().await
}

pub async fn detect_gpus_force() -> Vec<GpuInfo> {
    #[cfg(target_os = "windows")]
    {
        let list = match query_windows_gpus().await {
            Ok(names) => names
                .into_iter()
                .map(|name| {
                    let dedicated = is_dedicated_gpu_name(&name);
                    GpuInfo { name, dedicated }
                })
                .collect(),
            Err(_) => Vec::new(),
        };
        if let Ok(mut guard) = GPU_CACHE.write() {
            *guard = Some(list.clone());
        }
        list
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

pub const SINGLE_INSTANCE_PORT: u16 = 41928;
pub const SINGLE_INSTANCE_PORT_FALLBACK: u16 = 41929;

pub enum SingleInstanceStatus {
    Primary(std::net::TcpListener),
    AlreadyRunning,
    Standalone,
}

pub fn try_acquire_single_instance() -> SingleInstanceStatus {
    match std::net::TcpListener::bind(("127.0.0.1", SINGLE_INSTANCE_PORT)) {
        Ok(listener) => {
            let _ = listener.set_nonblocking(true);
            SingleInstanceStatus::Primary(listener)
        }
        Err(_) => {

            if let Ok(mut stream) = std::net::TcpStream::connect_timeout(
                &std::net::SocketAddr::from(([127, 0, 0, 1], SINGLE_INSTANCE_PORT)),
                std::time::Duration::from_millis(600),
            ) {
                use std::io::{Read, Write};
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(600)));
                let _ = stream.write_all(b"MONORYX_RESTORE\n");
                let _ = stream.flush();
                let mut buf = [0u8; 32];
                if let Ok(n) = stream.read(&mut buf) {
                    if let Ok(resp) = std::str::from_utf8(&buf[..n]) {
                        if resp.contains("MONORYX_ACK") {
                            #[cfg(target_os = "windows")]
                            restore_any_running_monoryx_window();
                            return SingleInstanceStatus::AlreadyRunning;
                        }
                    }
                }
            }

            if let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", SINGLE_INSTANCE_PORT_FALLBACK)) {
                let _ = listener.set_nonblocking(true);
                return SingleInstanceStatus::Primary(listener);
            }

            SingleInstanceStatus::Standalone
        }
    }
}

#[cfg(target_os = "windows")]
pub fn find_monoryx_windows(target_pid: Option<u32>) -> Vec<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowLongW, GetWindowTextW, GetWindowThreadProcessId,
        GWL_STYLE, GW_OWNER, WS_CAPTION,
    };

    struct SearchCtx {
        target_pid: Option<u32>,
        found: Vec<HWND>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam as *mut SearchCtx);
        let mut proc_id = 0u32;
        GetWindowThreadProcessId(hwnd, &mut proc_id);
        if let Some(target) = ctx.target_pid {
            if proc_id != target {
                return 1;
            }
        }
        let owner = GetWindow(hwnd, GW_OWNER);
        if owner.is_null() {
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            if (style & WS_CAPTION) == WS_CAPTION {
                let mut title = [0u16; 64];
                let len = GetWindowTextW(hwnd, title.as_mut_ptr(), 64);
                if len > 0 {
                    let title_str = String::from_utf16_lossy(&title[..len as usize]);
                    if title_str.starts_with("MONORYX") {
                        ctx.found.push(hwnd);
                    }
                }
            }
        }
        1
    }

    let mut ctx = SearchCtx {
        target_pid,
        found: Vec::new(),
    };
    unsafe {
        EnumWindows(Some(enum_proc), &mut ctx as *mut _ as LPARAM);
    }
    ctx.found
}

#[cfg(target_os = "windows")]
pub fn show_window_for_current_process(show: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetForegroundWindow, ShowWindow, SW_HIDE, SW_RESTORE, SW_SHOW,
    };
    let hwnds = find_monoryx_windows(Some(std::process::id()));
    for hwnd in hwnds {
        unsafe {
            if show {
                ShowWindow(hwnd, SW_SHOW);
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            } else {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn restore_any_running_monoryx_window() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };
    let hwnds = find_monoryx_windows(None);
    for hwnd in hwnds {
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn show_window_for_current_process(_show: bool) {}

#[cfg(not(target_os = "windows"))]
pub fn restore_any_running_monoryx_window() {}

#[cfg(target_os = "windows")]
async fn query_windows_gpus() -> Result<Vec<String>, String> {
    let mut cmd = tokio::process::Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
    ])
    .stdin(std::process::Stdio::null())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::null())
    .kill_on_drop(true);
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
    let child = cmd.spawn().map_err(|e| e.to_string())?;
    let output = tokio::time::timeout(std::time::Duration::from_secs(15), child.wait_with_output())
        .await
        .map_err(|_| "GPU detection timed out".to_string())?
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("GPU detection failed".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedicated_gpu_names() {
        assert!(is_dedicated_gpu_name("NVIDIA GeForce RTX 4060"));
        assert!(is_dedicated_gpu_name("NVIDIA Quadro P1000"));
        assert!(is_dedicated_gpu_name("Intel Arc A770 Graphics"));
        assert!(is_dedicated_gpu_name("AMD Radeon RX 7600"));
        assert!(is_dedicated_gpu_name("AMD Radeon Pro W6800"));
    }

    #[test]
    fn integrated_gpu_names() {
        assert!(!is_dedicated_gpu_name("Intel(R) UHD Graphics 630"));
        assert!(!is_dedicated_gpu_name("Intel(R) Iris(R) Xe Graphics"));
        assert!(!is_dedicated_gpu_name("AMD Radeon(TM) Graphics"));
        assert!(!is_dedicated_gpu_name("AMD Radeon Vega 8 Graphics"));
        assert!(!is_dedicated_gpu_name("Microsoft Basic Display Adapter"));
    }

    #[test]
    fn memory_validation() {
        assert!(validate_memory(512, 2048).is_ok());
        assert!(validate_memory(4096, 512).is_err());
        assert!(validate_memory(64, 512).is_err());
    }

    #[test]
    fn default_max_in_range() {
        let v = default_max_memory_mb();
        assert!((1024..=8192).contains(&v) || v == 2048);
    }

    #[test]
    fn default_boost_max_in_range() {
        let v = default_boost_max_memory_mb();
        assert!((1024..=2560).contains(&v) || v == 2048);
    }
}
