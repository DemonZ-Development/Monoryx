use crate::config::GpuPreference;
use crate::error::{MonoryxError, Result};
use crate::minecraft::arguments::{
    expand_args, expand_legacy_args, split_user_args, LaunchSubstitutions,
};
use crate::minecraft::libraries::build_classpath;
use crate::minecraft::rules::RuleFeatures;
use crate::minecraft::version::VersionJson;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt as _, BufReader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Idle,
    Starting,
    Running,
    Stopped,
    Crashed(i32),
}

#[derive(Debug, Clone)]
pub struct LaunchContext {
    pub version: VersionJson,
    pub version_id: String,
    pub game_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub libraries_dir: PathBuf,
    pub client_jar: PathBuf,
    pub natives_dir: PathBuf,
    pub java_exe: PathBuf,
    pub username: String,
    pub uuid: uuid::Uuid,

    pub access_token: String,
    pub memory_min_mb: u64,
    pub memory_max_mb: u64,
    pub resolution: Option<(u32, u32)>,
    pub fullscreen: bool,
    pub extra_jvm_args: String,
    pub extra_game_args: String,
    pub log_file: PathBuf,
    pub boost_mode: bool,
    pub skins_restorer_compat: bool,
}

#[derive(Debug, Clone)]
pub struct LaunchPlan {
    pub java_exe: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

#[must_use]
pub fn describe_plan(plan: &LaunchPlan) -> String {
    let mut parts = vec![plan.java_exe.display().to_string()];
    let mut prev = String::new();
    for a in &plan.args {
        if prev == "--accessToken" {
            parts.push("<redacted>".to_string());
        } else {
            parts.push(a.clone());
        }
        prev = a.clone();
    }
    parts.join(" ")
}

pub fn build_launch_plan(ctx: &LaunchContext) -> Result<LaunchPlan> {
    let sep = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };
    let cp_entries = build_classpath(
        &ctx.version.libraries,
        &ctx.libraries_dir,
        &ctx.client_jar,
        "https://libraries.minecraft.net",
    );
    let classpath = cp_entries
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(sep);

    let assets_index = ctx
        .version
        .asset_index
        .as_ref()
        .map(|a| a.id.clone())
        .or_else(|| ctx.version.assets.clone())
        .unwrap_or_else(|| "legacy".to_string());

    let features = RuleFeatures {
        has_custom_resolution: ctx.resolution.is_some(),
        ..RuleFeatures::default()
    };

    let subs = LaunchSubstitutions {
        auth_player_name: ctx.username.clone(),
        version_name: ctx.version_id.clone(),
        game_directory: ctx.game_dir.display().to_string(),
        assets_root: ctx.assets_dir.display().to_string(),
        assets_index_name: assets_index,
        auth_uuid: ctx.uuid.hyphenated().to_string(),
        auth_uuid_undashed: ctx.uuid.as_simple().to_string(),
        auth_access_token: ctx.access_token.clone(),
        user_type: if ctx.skins_restorer_compat {
            "mojang".to_string()
        } else {
            "legacy".to_string()
        },
        version_type: ctx
            .version
            .kind
            .clone()
            .unwrap_or_else(|| "release".to_string()),
        natives_directory: ctx.natives_dir.display().to_string(),
        launcher_name: "MONORYX".to_string(),
        launcher_version: env!("CARGO_PKG_VERSION").to_string(),
        classpath: classpath.clone(),
        classpath_separator: sep.to_string(),
        primary_jar: ctx.client_jar.display().to_string(),
        resolution_width: ctx.resolution.map(|(w, _)| w.to_string()),
        resolution_height: ctx.resolution.map(|(_, h)| h.to_string()),
        quick_play_path: None,
        quick_play_singleplayer: None,
        quick_play_multiplayer: None,
        quick_play_realms: None,
    };
    let map = subs.as_map();

    let main_class = ctx.version.main_class.clone().ok_or_else(|| {
        MonoryxError::Launch("version metadata is missing the main class".to_string())
    })?;

    let max_mem = ctx.memory_max_mb.max(256);
    let min_mem = if ctx.boost_mode {
        ctx.memory_min_mb.clamp(256, 512).min(max_mem)
    } else {
        ctx.memory_min_mb.clamp(128, max_mem)
    };

    let mut jvm: Vec<String> = vec![
        format!("-Xms{min_mem}M"),
        format!("-Xmx{max_mem}M"),
    ];

    let modern = ctx.version.arguments.is_some();
    if modern {
        if let Some(args) = &ctx.version.arguments {
            jvm.extend(expand_args(&args.jvm, &features, &map));
        }
    } else {
        jvm.push(format!("-Djava.library.path={}", subs.natives_directory));
        jvm.push("-cp".to_string());
        jvm.push(classpath);
    }

    if let Some(logging) = &ctx.version.logging {
        if let Some(arg) = logging
            .get("client")
            .and_then(|c| c.get("argument"))
            .and_then(|a| a.as_str())
        {
            jvm.push(substitute_logging_arg(
                arg,
                &ctx.game_dir,
                &logging_path(ctx),
            ));
        }
    }

    let user_jvm_args = split_user_args(&ctx.extra_jvm_args);

    if ctx.boost_mode {
        let custom_gc = has_custom_gc_flag(&jvm) || has_custom_gc_flag(&user_jvm_args);
        let boost_args = [
            "-XX:+IgnoreUnrecognizedVMOptions",
            "-XX:+UnlockExperimentalVMOptions",
            "-XX:+UseG1GC",
            "-XX:+ParallelRefProcEnabled",
            "-XX:MaxGCPauseMillis=50",
            "-XX:+DisableExplicitGC",
            "-XX:G1NewSizePercent=20",
            "-XX:G1MaxNewSizePercent=30",
            "-XX:G1ReservePercent=10",
            "-XX:G1HeapWastePercent=5",
            "-XX:G1MixedGCCountTarget=4",
            "-XX:InitiatingHeapOccupancyPercent=15",
            "-XX:G1MixedGCLiveThresholdPercent=90",
            "-XX:G1RSetUpdatingPauseTimePercent=5",
            "-XX:SurvivorRatio=32",
            "-XX:+PerfDisableSharedMem",
            "-XX:MaxTenuringThreshold=1",
            "-XX:+UseStringDeduplication",
            "-XX:+OptimizeStringConcat",
            "-XX:+UseCompressedOops",
            "-XX:+UseCompressedClassPointers",
            "-XX:G1PeriodicGCInterval=10000",
            "-XX:G1PeriodicGCSystemLoadThreshold=0",
            "-XX:MinHeapFreeRatio=10",
            "-XX:MaxHeapFreeRatio=20",
            "-XX:-ShrinkHeapInSteps",
        ];
        for arg in boost_args {
            let is_g1_specific = arg.contains("G1") || arg == "-XX:+UseG1GC";
            if custom_gc && is_g1_specific {
                continue;
            }
            if let Some(key) = vm_option_key(arg) {
                if !has_matching_vm_option(&jvm, key) && !has_matching_vm_option(&user_jvm_args, key) {
                    jvm.push(arg.to_string());
                }
            } else if !jvm.iter().any(|a| a == arg) && !user_jvm_args.iter().any(|a| a == arg) {
                jvm.push(arg.to_string());
            }
        }
    }

    jvm.extend(user_jvm_args);

    if ctx.skins_restorer_compat {
        let skin_flag = "-Dskinsrestorer.compat=true".to_string();
        if !jvm.contains(&skin_flag) {
            jvm.push(skin_flag);
        }
    }

    let mut game: Vec<String> = Vec::new();
    if let Some(args) = &ctx.version.arguments {
        game.extend(expand_args(&args.game, &features, &map));
    } else if let Some(legacy) = &ctx.version.minecraft_arguments {
        game.extend(expand_legacy_args(legacy, &map));
    } else {
        return Err(MonoryxError::Launch(
            "version metadata has neither arguments nor minecraftArguments".to_string(),
        ));
    }
    game.extend(split_user_args(&ctx.extra_game_args));

    if let Some((w, h)) = ctx.resolution {
        if !modern {
            game.push("--width".to_string());
            game.push(w.to_string());
            game.push("--height".to_string());
            game.push(h.to_string());
        }
    }
    if ctx.fullscreen && !game.iter().any(|a| a == "--fullscreen") {
        game.push("--fullscreen".to_string());
    }

    let mut args = jvm;
    args.push(main_class);
    args.extend(game);

    Ok(LaunchPlan {
        java_exe: ctx.java_exe.clone(),
        args,
        cwd: ctx.game_dir.clone(),
    })
}

fn logging_path(ctx: &LaunchContext) -> PathBuf {
    if let Some(logging) = &ctx.version.logging {
        if let Some(id) = logging
            .get("client")
            .and_then(|c| c.get("file"))
            .and_then(|f| f.get("id"))
            .and_then(|v| v.as_str())
        {
            return ctx.client_jar.parent().unwrap_or(Path::new(".")).join(id);
        }
    }
    ctx.client_jar
        .parent()
        .unwrap_or(Path::new("."))
        .join("client.xml")
}

fn substitute_logging_arg(template: &str, _game_dir: &Path, path: &Path) -> String {
    template.replace("${path}", &path.display().to_string())
}

fn vm_option_key(arg: &str) -> Option<&str> {
    if let Some(rest) = arg.strip_prefix("-XX:+") {
        Some(rest.split('=').next().unwrap_or(rest))
    } else if let Some(rest) = arg.strip_prefix("-XX:-") {
        Some(rest.split('=').next().unwrap_or(rest))
    } else if let Some(rest) = arg.strip_prefix("-XX:") {
        Some(rest.split('=').next().unwrap_or(rest))
    } else if let Some(rest) = arg.strip_prefix("-D") {
        Some(rest.split('=').next().unwrap_or(rest))
    } else if arg.starts_with("-Xms") {
        Some("-Xms")
    } else if arg.starts_with("-Xmx") {
        Some("-Xmx")
    } else {
        None
    }
}

fn has_matching_vm_option(args: &[String], key: &str) -> bool {
    args.iter().any(|a| vm_option_key(a) == Some(key))
}

fn has_custom_gc_flag(args: &[String]) -> bool {
    args.iter().any(|a| {
        a.contains("UseZGC")
            || a.contains("UseParallelGC")
            || a.contains("UseSerialGC")
            || a.contains("UseShenandoahGC")
            || a.contains("UseEpsilonGC")
            || a.contains("UseConcMarkSweepGC")
    })
}

fn gpu_preference_code(preference: GpuPreference) -> &'static str {
    match preference {
        GpuPreference::System => "0",
        GpuPreference::HighPerformance => "1",
        GpuPreference::PowerSaving => "2",
    }
}

fn gpu_reg_args(java_exe: &Path, code: &str) -> Vec<String> {
    vec![
        "add".to_string(),
        r"HKCU\Software\Microsoft\DirectX\UserGpuPreferences".to_string(),
        "/v".to_string(),
        java_exe.display().to_string(),
        "/t".to_string(),
        "REG_SZ".to_string(),
        "/d".to_string(),
        format!("GpuPreference={code}"),
        "/f".to_string(),
    ]
}

fn system_reg_exe() -> PathBuf {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    PathBuf::from(system_root).join("System32").join("reg.exe")
}

fn windows_registry_error(context: &str, status: Option<i32>) -> MonoryxError {
    MonoryxError::Launch(format!(
        "{context} failed{}",
        status.map_or_else(String::new, |c| format!(" (exit code {c})"))
    ))
}

#[cfg(target_os = "windows")]
async fn query_gpu_preference(java_exe: &Path) -> Result<Option<String>> {
    let reg = system_reg_exe();
    let mut cmd = tokio::process::Command::new(&reg);
    cmd.args([
        "query",
        r"HKCU\Software\Microsoft\DirectX\UserGpuPreferences",
        "/v",
        &java_exe.display().to_string(),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .creation_flags(CREATE_NO_WINDOW)
    .kill_on_drop(true);
    let child = cmd
        .spawn()
        .map_err(|e| MonoryxError::Launch(format!("couldn't query GPU preference: {e}")))?;
    let output = match tokio::time::timeout(REG_QUERY_TIMEOUT, child.wait_with_output()).await {
        Ok(r) => {
            r.map_err(|e| MonoryxError::Launch(format!("couldn't query GPU preference: {e}")))?
        }
        Err(_) => {
            return Err(MonoryxError::Launch(
                "querying GPU preference timed out".to_string(),
            ));
        }
    };
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_reg_query_value(
        &text,
        &java_exe.display().to_string(),
    ))
}

fn parse_reg_query_value(text: &str, value_name: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix(value_name) else {
            continue;
        };
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let mut parts = rest.split_whitespace();
        if parts.next().is_some() {
            if let Some(data) = parts.next() {
                return Some(data.to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
async fn write_gpu_preference(java_exe: &Path, code: &str) -> Result<()> {
    let reg = system_reg_exe();
    let mut cmd = tokio::process::Command::new(&reg);
    cmd.args(gpu_reg_args(java_exe, code))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .kill_on_drop(true);
    let child = cmd
        .spawn()
        .map_err(|e| MonoryxError::Launch(format!("couldn't update GPU preference: {e}")))?;
    let output = match tokio::time::timeout(REG_QUERY_TIMEOUT, child.wait_with_output()).await {
        Ok(r) => {
            r.map_err(|e| MonoryxError::Launch(format!("couldn't update GPU preference: {e}")))?
        }
        Err(_) => {
            return Err(MonoryxError::Launch(
                "updating GPU preference timed out".to_string(),
            ));
        }
    };
    if !output.status.success() {
        return Err(windows_registry_error(
            "updating GPU preference",
            output.status.code(),
        ));
    }
    Ok(())
}

pub async fn apply_gpu_preference(
    java_exe: &Path,
    preference: crate::config::GpuPreference,
) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        apply_gpu_preference_windows(java_exe, preference).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = java_exe;
        match preference {
            crate::config::GpuPreference::System => Ok(()),
            _ => Err(MonoryxError::Launch(
                "GPU preference is only supported on Windows".to_string(),
            )),
        }
    }
}

#[cfg(target_os = "windows")]
async fn apply_gpu_preference_windows(java_exe: &Path, preference: GpuPreference) -> Result<()> {
    let code = gpu_preference_code(preference);
    if code == "0" && query_gpu_preference(java_exe).await?.is_none() {
        tracing::debug!(
            "no existing GPU preference for {}; nothing to do",
            java_exe.display()
        );
        return Ok(());
    }
    write_gpu_preference(java_exe, code).await
}

const REG_QUERY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug)]
pub struct SupervisedProcess {
    state: Arc<Mutex<ProcessState>>,
    child: Option<tokio::process::Child>,
    log_path: PathBuf,
}

impl SupervisedProcess {
    pub async fn spawn(plan: &LaunchPlan, log_file: &Path) -> Result<Self> {
        if let Some(parent) = log_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let state = Arc::new(Mutex::new(ProcessState::Starting));
        let mut cmd = tokio::process::Command::new(&plan.java_exe);
        cmd.args(&plan.args)
            .current_dir(&plan.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd.spawn().map_err(|e| {
            MonoryxError::Launch(format!(
                "couldn't start Java ({}): {e}",
                plan.java_exe.display()
            ))
        })?;
        *state.lock().unwrap() = ProcessState::Running;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let log_path = log_file.to_path_buf();
        let state_clone = state.clone();
        tokio::spawn(async move {
            let _ = pump_logs(stdout, stderr, &log_path, state_clone).await;
        });

        Ok(Self {
            state,
            child: Some(child),
            log_path: log_file.to_path_buf(),
        })
    }

    #[must_use]
    pub fn state(&self) -> ProcessState {
        *self.state.lock().unwrap()
    }

    #[must_use]
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub async fn wait(&mut self) -> Result<i32> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| MonoryxError::Launch("game process already reaped".to_string()))?;
        let status = child
            .wait()
            .await
            .map_err(|e| MonoryxError::Launch(format!("error waiting for game process: {e}")))?;
        let code = status.code().unwrap_or(-1);
        *self.state.lock().unwrap() = if status.success() {
            ProcessState::Stopped
        } else {
            ProcessState::Crashed(code)
        };
        Ok(code)
    }

    pub async fn kill(&mut self) -> Result<()> {
        if let Some(child) = self.child.as_mut() {
            child
                .kill()
                .await
                .map_err(|e| MonoryxError::Launch(format!("couldn't stop game process: {e}")))?;
            *self.state.lock().unwrap() = ProcessState::Stopped;
        }
        Ok(())
    }
}

async fn pump_logs(
    stdout: Option<tokio::process::ChildStdout>,
    stderr: Option<tokio::process::ChildStderr>,
    log_path: &Path,
    _state: Arc<Mutex<ProcessState>>,
) -> Result<()> {
    let file = tokio::fs::File::create(log_path)
        .await
        .map_err(MonoryxError::Io)?;
    let writer = Arc::new(tokio::sync::Mutex::new(tokio::io::BufWriter::new(file)));

    use tokio::io::AsyncWriteExt as _;

    let stdout_fut = {
        let writer = writer.clone();
        async move {
            if let Some(stdout) = stdout {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let mut guard = writer.lock().await;
                    let _ = guard
                        .write_all(format!("[STDOUT] {line}\n").as_bytes())
                        .await;
                }
            }
        }
    };

    let stderr_fut = {
        let writer = writer.clone();
        async move {
            if let Some(stderr) = stderr {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let mut guard = writer.lock().await;
                    let _ = guard
                        .write_all(format!("[STDERR] {line}\n").as_bytes())
                        .await;
                }
            }
        }
    };

    tokio::join!(stdout_fut, stderr_fut);

    let mut guard = writer.lock().await;
    guard.flush().await.map_err(MonoryxError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft::version::VersionJson;

    fn minimal_version() -> VersionJson {
        serde_json::from_value(serde_json::json!({
            "id": "1.21",
            "type": "release",
            "mainClass": "net.minecraft.client.main.Main",
            "minecraftArguments": "--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory} --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --userType ${user_type}",
            "libraries": [],
            "assets": "1.21"
        }))
        .unwrap()
    }

    fn ctx() -> LaunchContext {
        LaunchContext {
            version: minimal_version(),
            version_id: "1.21".into(),
            game_dir: PathBuf::from("/tmp/game"),
            assets_dir: PathBuf::from("/tmp/assets"),
            libraries_dir: PathBuf::from("/tmp/libraries"),
            client_jar: PathBuf::from("/tmp/client.jar"),
            natives_dir: PathBuf::from("/tmp/natives"),
            java_exe: PathBuf::from("java"),
            username: "Steve".into(),
            uuid: uuid::Uuid::nil(),
            access_token: "token".into(),
            memory_min_mb: 512,
            memory_max_mb: 2048,
            resolution: None,
            fullscreen: false,
            extra_jvm_args: String::new(),
            extra_game_args: String::new(),
            log_file: PathBuf::from("/tmp/game.log"),
            boost_mode: false,
            skins_restorer_compat: true,
        }
    }

    #[test]
    fn skins_restorer_compat_plan_flags() {
        let mut c = ctx();
        c.skins_restorer_compat = true;
        let plan = build_launch_plan(&c).unwrap();
        assert!(plan.args.contains(&"-Dskinsrestorer.compat=true".to_string()));

        let mut c_legacy = ctx();
        c_legacy.skins_restorer_compat = false;
        let plan_legacy = build_launch_plan(&c_legacy).unwrap();
        assert!(!plan_legacy.args.contains(&"-Dskinsrestorer.compat=true".to_string()));
    }

    #[test]
    fn boost_mode_injects_ram_and_perf_flags() {
        let mut c = ctx();
        c.boost_mode = true;
        let plan = build_launch_plan(&c).unwrap();
        assert!(plan.args.contains(&"-XX:+IgnoreUnrecognizedVMOptions".to_string()));
        assert!(plan.args.contains(&"-XX:+UseStringDeduplication".to_string()));
        assert!(plan.args.contains(&"-XX:+UseG1GC".to_string()));
        assert!(plan.args.contains(&"-XX:+UseCompressedOops".to_string()));
        assert!(plan.args.contains(&"-XX:G1PeriodicGCInterval=10000".to_string()));
        assert!(plan.args.contains(&"-XX:MinHeapFreeRatio=10".to_string()));
        assert!(plan.args.contains(&"-XX:MaxHeapFreeRatio=20".to_string()));
        assert!(plan.args.contains(&"-XX:-ShrinkHeapInSteps".to_string()));

        assert!(!plan.args.contains(&"-XX:+AlwaysPreTouch".to_string()));

        assert!(plan.args.contains(&"-Xms512M".to_string()));
    }

    #[test]
    fn boost_mode_respects_custom_gc_and_avoids_collision() {
        let mut c = ctx();
        c.boost_mode = true;
        c.extra_jvm_args = "-XX:+UseZGC -XX:MaxGCPauseMillis=100".to_string();
        let plan = build_launch_plan(&c).unwrap();

        assert!(!plan.args.contains(&"-XX:+UseG1GC".to_string()));
        assert!(!plan.args.contains(&"-XX:G1PeriodicGCInterval=10000".to_string()));
        assert!(plan.args.contains(&"-XX:+UseZGC".to_string()));

        assert!(plan.args.contains(&"-XX:MaxGCPauseMillis=100".to_string()));
        assert!(!plan.args.contains(&"-XX:MaxGCPauseMillis=50".to_string()));
    }

    #[test]
    fn inverted_heap_sizes_clamped_safely() {
        let mut c = ctx();
        c.memory_min_mb = 4096;
        c.memory_max_mb = 2048;
        let plan = build_launch_plan(&c).unwrap();
        assert!(plan.args.contains(&"-Xms2048M".to_string()));
        assert!(plan.args.contains(&"-Xmx2048M".to_string()));
    }

    #[test]
    fn legacy_plan_substitutes() {
        let plan = build_launch_plan(&ctx()).unwrap();
        let joined = plan.args.join(" ");
        assert!(joined.contains("--username Steve"));
        assert!(joined.contains("--version 1.21"));
        assert!(joined.contains("-Xmx2048M"));
        assert!(joined.contains("net.minecraft.client.main.Main"));
    }

    #[test]
    fn user_args_appended() {
        let mut c = ctx();
        c.extra_game_args = "--demo".into();
        c.extra_jvm_args = "-Dfoo=bar".into();
        let plan = build_launch_plan(&c).unwrap();
        assert!(plan.args.contains(&"-Dfoo=bar".to_string()));
        assert!(plan.args.contains(&"--demo".to_string()));
    }

    #[test]
    fn gpu_reg_args_use_explicit_value_data() {
        let java_exe = Path::new(r"C:\Program Files\Java\bin\javaw.exe");
        for (preference, code) in [
            (GpuPreference::System, "0"),
            (GpuPreference::HighPerformance, "1"),
            (GpuPreference::PowerSaving, "2"),
        ] {
            let args = gpu_reg_args(java_exe, gpu_preference_code(preference));
            assert_eq!(args[0], "add");
            assert_eq!(
                args[1],
                r"HKCU\Software\Microsoft\DirectX\UserGpuPreferences"
            );
            assert_eq!(args[2], "/v");
            assert_eq!(args[3], java_exe.display().to_string());
            assert_eq!(args[4], "/t");
            assert_eq!(args[5], "REG_SZ");
            assert_eq!(args[6], "/d");
            assert_eq!(args[7], format!("GpuPreference={code}"));
            assert_eq!(args[8], "/f");
        }
    }

    #[test]
    fn parse_reg_query_value_extracts_value_data() {
        let text = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\DirectX\\UserGpuPreferences\r\n    C:\\Program Files\\Java\\bin\\javaw.exe    REG_SZ    GpuPreference=1\r\n\r\n";
        assert_eq!(
            parse_reg_query_value(text, r"C:\Program Files\Java\bin\javaw.exe"),
            Some("GpuPreference=1".to_string())
        );
        assert_eq!(
            parse_reg_query_value(text, r"C:\Program Files\Other\bin\javaw.exe"),
            None
        );
    }
}
