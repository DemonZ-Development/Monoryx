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
    (total / 4).clamp(1024, 4096)
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

pub async fn detect_gpus() -> Vec<GpuInfo> {
    #[cfg(target_os = "windows")]
    {
        match query_windows_gpus().await {
            Ok(names) => names
                .into_iter()
                .map(|name| {
                    let dedicated = is_dedicated_gpu_name(&name);
                    GpuInfo { name, dedicated }
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

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
}
