use crate::java::runtime::JavaRuntime;
use std::path::{Path, PathBuf};

pub async fn discover_all(managed_dir: &Path) -> Vec<JavaRuntime> {
    let mut out: Vec<JavaRuntime> = Vec::new();
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    let mut push = |rt: JavaRuntime| {
        let key = rt.path.canonicalize().unwrap_or_else(|_| rt.path.clone());
        if seen.insert(key) {
            out.push(rt);
        }
    };

    if let Ok(home) = std::env::var("JAVA_HOME") {
        for cand in candidate_exes(&PathBuf::from(home)) {
            if let Some(rt) = probe(&cand, "JAVA_HOME").await {
                push(rt);
            }
        }
    }

    for exe_name in ["java", "javaw"] {
        if let Some(p) = resolve_on_path(exe_name) {
            if let Some(rt) = probe(&p, "PATH").await {
                push(rt);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        for base in windows_search_roots() {
            for cand in scan_java_homes(&base, 2) {
                if let Some(rt) = probe(&cand, "system").await {
                    push(rt);
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        for base in [
            "/Library/Java/JavaVirtualMachines",
            "/opt/homebrew/opt/openjdk",
            "/usr/local/opt/openjdk",
        ] {
            for cand in scan_java_homes(&PathBuf::from(base), 3) {
                if let Some(rt) = probe(&cand, "system").await {
                    push(rt);
                }
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let user_jvm = PathBuf::from(home).join("Library/Java/JavaVirtualMachines");
            for cand in scan_java_homes(&user_jvm, 3) {
                if let Some(rt) = probe(&cand, "system").await {
                    push(rt);
                }
            }
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        for base in ["/usr/lib/jvm", "/opt/java", "/opt/jdk", "/usr/java"] {
            for cand in scan_java_homes(&PathBuf::from(base), 2) {
                if let Some(rt) = probe(&cand, "system").await {
                    push(rt);
                }
            }
        }
    }

    if managed_dir.exists() {
        for cand in scan_java_homes(managed_dir, 3) {
            if let Some(rt) = probe(&cand, "managed").await {
                push(rt);
            }
        }
    }

    out.sort_by_key(|r| r.major);
    out
}

pub async fn probe(exe: &Path, source: &str) -> Option<JavaRuntime> {
    if !exe.is_file() {
        return None;
    }
    let console = exe.with_file_name("java.exe");
    let probe_exe = if cfg!(target_os = "windows")
        && exe
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("javaw.exe"))
        && console.is_file()
    {
        &console
    } else {
        exe
    };
    let mut cmd = tokio::process::Command::new(probe_exe);
    cmd.arg("-version")
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output())
        .await
        .ok()?
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    let major = parse_java_major(&combined)?;
    Some(JavaRuntime {
        path: exe.to_path_buf(),
        major,
        version_string: combined.lines().next().unwrap_or("").trim().to_string(),
        source: source.to_string(),
    })
}

pub fn parse_java_major(output: &str) -> Option<u32> {
    let start = output.find('"')?;
    let rest = &output[start + 1..];
    let end = rest.find('"')?;
    let ver = &rest[..end];
    parse_version_token(ver)
}

fn parse_version_token(ver: &str) -> Option<u32> {
    let core = ver.split(['-', '+', '_']).next().unwrap_or(ver);
    let parts: Vec<&str> = core.split('.').collect();
    if parts.first() == Some(&"1") && parts.len() >= 2 {
        return parts[1].parse().ok();
    }
    parts.first()?.parse().ok()
}

fn candidate_exes(home: &Path) -> Vec<PathBuf> {
    let bin = home.join("bin");
    let mut v = Vec::new();
    if cfg!(target_os = "windows") {
        for name in ["javaw.exe", "java.exe"] {
            v.push(bin.join(name));
        }

        for name in ["javaw.exe", "java.exe"] {
            v.push(home.join(name));
        }
    } else {
        v.push(bin.join("java"));
        v.push(home.join("java"));
        v.push(home.join("Contents/Home/bin/java"));
    }
    v.into_iter().filter(|p| p.exists()).collect()
}

fn resolve_on_path(exe: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        if cfg!(target_os = "windows") {
            for ext in ["exe", "bat", "cmd"] {
                let p = dir.join(format!("{exe}.{ext}"));
                if p.is_file() {
                    return Some(p);
                }
            }
            let p = dir.join(exe);
            if p.is_file() {
                return Some(p);
            }
        } else {
            let p = dir.join(exe);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn windows_search_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from(r"C:\Program Files\Java"),
        PathBuf::from(r"C:\Program Files\Eclipse Adoptium"),
        PathBuf::from(r"C:\Program Files\Microsoft"),
        PathBuf::from(r"C:\Program Files\Amazon Corretto"),
        PathBuf::from(r"C:\Program Files\Zulu"),
    ];
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        roots.push(PathBuf::from(pf86).join("Java"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join(r"Programs\Eclipse Adoptium"));
    }
    roots.into_iter().filter(|p| p.exists()).collect()
}

fn scan_java_homes(base: &Path, depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    scan_inner(base, depth, &mut out);
    out
}

fn scan_inner(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    for cand in candidate_exes(dir) {
        out.push(cand);
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if out.len() > 64 {
                return;
            }
            scan_inner(&entry.path(), depth - 1, out);
        }
    }
}

#[must_use]
pub fn prefer_javaw(path: &Path) -> PathBuf {
    if !cfg!(target_os = "windows") {
        return path.to_path_buf();
    }
    if path.file_name().and_then(|n| n.to_str()) == Some("java.exe") {
        let javaw = path.with_file_name("javaw.exe");
        if javaw.exists() {
            return javaw;
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern() {
        let s = "openjdk version \"21.0.2\" 2024-01-16\nOpenJDK Runtime Environment";
        assert_eq!(parse_java_major(s), Some(21));
    }

    #[test]
    fn parses_legacy_8() {
        let s = "java version \"1.8.0_391\"\nJava(TM) SE Runtime Environment";
        assert_eq!(parse_java_major(s), Some(8));
    }

    #[test]
    fn parses_ea() {
        assert_eq!(
            parse_java_major("openjdk version \"17-ea\" 2021-09-16"),
            Some(17)
        );
    }

    #[test]
    fn garbage_returns_none() {
        assert_eq!(parse_java_major("no version here"), None);
    }
}
