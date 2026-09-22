use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashInfo {
    pub instance_id: String,
    pub instance_name: String,
    pub exit_code: i32,
    pub summary: String,
    pub details: String,
    pub source_label: String,
    pub report_path: Option<PathBuf>,
    pub crash_reports_dir: PathBuf,
    pub logs_dir: PathBuf,
}

#[must_use]
pub fn format_exit_code(code: i32) -> String {
    if code < 0 {
        format!("{code} (0x{:08X})", code as u32)
    } else {
        format!("{code}")
    }
}

#[must_use]
pub fn find_latest_crash_report(
    crash_reports_dir: &Path,
    since: Option<std::time::SystemTime>,
) -> Option<(PathBuf, String)> {
    if !crash_reports_dir.exists() {
        return None;
    }
    let entries = std::fs::read_dir(crash_reports_dir).ok()?;
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let is_txt = path.extension().is_some_and(|ext| ext == "txt");
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if is_txt || name.starts_with("crash-") {
                let mtime = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                if let Some(cutoff) = since {
                    let cutoff_with_margin = cutoff
                        .checked_sub(Duration::from_secs(5))
                        .unwrap_or(cutoff);
                    if mtime < cutoff_with_margin {
                        continue;
                    }
                }
                files.push((path, mtime));
            }
        }
    }
    files.sort_by_key(|a| std::cmp::Reverse(a.1));
    for (path, _) in files {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if !content.trim().is_empty() {
                return Some((path, content));
            }
        }
    }
    None
}

#[must_use]
pub fn find_latest_log(
    logs_dir: &Path,
    session_log: Option<&Path>,
    since: Option<std::time::SystemTime>,
) -> Option<(PathBuf, String)> {
    if let Some(p) = session_log {
        if p.is_file() {
            if let Ok(content) = std::fs::read_to_string(p) {
                if !content.trim().is_empty() {
                    return Some((p.to_path_buf(), content));
                }
            }
        }
    }

    if !logs_dir.exists() {
        return None;
    }
    let mut candidates: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

    let latest_log = logs_dir.join("latest.log");
    if latest_log.is_file() {
        if let Ok(meta) = latest_log.metadata() {
            let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let valid = since.is_none_or(|cutoff| {
                let margin = cutoff
                    .checked_sub(Duration::from_secs(5))
                    .unwrap_or(cutoff);
                mtime >= margin
            });
            if meta.len() > 0 && valid {
                candidates.push((latest_log, mtime));
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(logs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.is_file() && name.starts_with("monoryx-") && name.ends_with(".log") {
                if let Ok(meta) = path.metadata() {
                    let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let valid = since.is_none_or(|cutoff| {
                        let margin = cutoff
                            .checked_sub(Duration::from_secs(5))
                            .unwrap_or(cutoff);
                        mtime >= margin
                    });
                    if meta.len() > 0 && valid {
                        candidates.push((path, mtime));
                    }
                }
            }
        }
    }

    candidates.sort_by_key(|a| std::cmp::Reverse(a.1));
    for (path, _) in candidates {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if !content.trim().is_empty() {
                return Some((path, content));
            }
        }
    }
    None
}

#[must_use]
pub fn tail_lines(content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= max_lines {
        content.to_string()
    } else {
        let skip = lines.len() - max_lines;
        lines[skip..].join("\n")
    }
}

#[must_use]
pub fn extract_crash_summary(content: &str, exit_code: i32) -> String {
    let trimmed = content.trim();

    if !trimmed.is_empty() {

        if trimmed.contains("---- Minecraft Crash Report ----")
            || trimmed.contains("Description:")
            || trimmed.contains("-- Head --")
            || trimmed.contains("-- System Details --")
        {
            let mut description = None;
            let mut exception = None;
            let mut suspected_mods = Vec::new();
            let mut solution = Vec::new();

            let lines: Vec<&str> = trimmed.lines().map(str::trim).collect();
            let mut i = 0;
            while i < lines.len() {
                let line = lines[i];

                if let Some(desc) = line.strip_prefix("Description:") {
                    let d = desc.trim();
                    if !d.is_empty() {
                        description = Some(d.to_string());
                    }
                    let mut j = i + 1;
                    while j < lines.len() {
                        let next = lines[j];
                        if next.is_empty() {
                            j += 1;
                            continue;
                        }
                        if next.starts_with("A detailed walkthrough")
                            || next.starts_with("-- Head --")
                            || next.starts_with("//")
                        {
                            break;
                        }
                        if exception.is_none() {
                            let mut ex = next.to_string();
                            if j + 1 < lines.len() && lines[j + 1].starts_with("Caused by:") {
                                ex = format!("{}\n{}", ex, lines[j + 1]);
                            }
                            exception = Some(ex);
                        }
                        break;
                    }
                } else if let Some(mods) = line.strip_prefix("Suspected Mods:") {
                    let inline = mods.trim();
                    if !inline.is_empty() && inline != "None" {
                        suspected_mods.push(inline.to_string());
                    } else {
                        let mut j = i + 1;
                        while j < lines.len() {
                            let raw_line = trimmed.lines().nth(j).unwrap_or("");
                            if raw_line.starts_with('\t')
                                || raw_line.starts_with("  ")
                                || raw_line.trim_start().starts_with('-')
                            {
                                let mod_entry = raw_line.trim().trim_start_matches('-').trim();
                                if !mod_entry.is_empty() {
                                    suspected_mods.push(mod_entry.to_string());
                                }
                                j += 1;
                            } else {
                                break;
                            }
                        }
                    }
                } else if let Some(sol) =
                    line.strip_prefix("A potential solution has been determined:")
                {
                    let inline = sol.trim();
                    if !inline.is_empty() {
                        solution.push(inline.to_string());
                    }
                    let mut j = i + 1;
                    while j < lines.len() {
                        let raw_line = trimmed.lines().nth(j).unwrap_or("");
                        if raw_line.starts_with('\t')
                            || raw_line.starts_with("  ")
                            || raw_line.trim_start().starts_with('-')
                        {
                            let s = raw_line.trim().trim_start_matches('-').trim();
                            if !s.is_empty() {
                                solution.push(s.to_string());
                            }
                            j += 1;
                        } else {
                            break;
                        }
                    }
                }
                i += 1;
            }

            if exception.is_none() {
                for (idx, line) in lines.iter().enumerate() {
                    if line.starts_with("-- Head --") || line.starts_with("-- System Details --") {
                        break;
                    }
                    if (line.contains("Exception") || line.contains("Error:"))
                        && !line.starts_with("//")
                        && !line.starts_with("----")
                        && !line.starts_with("Time:")
                    {
                        let mut ex = (*line).to_string();
                        if idx + 1 < lines.len() && lines[idx + 1].starts_with("Caused by:") {
                            ex = format!("{}\n{}", ex, lines[idx + 1]);
                        }
                        exception = Some(ex);
                        break;
                    }
                }
            }

            let mut summary_parts = Vec::new();
            if let Some(d) = description {
                summary_parts.push(format!("Description: {d}"));
            }
            if let Some(e) = exception {
                summary_parts.push(format!("Exception: {e}"));
            }
            if !suspected_mods.is_empty() {
                summary_parts.push(format!("Suspected Mods: {}", suspected_mods.join(", ")));
            }
            if !solution.is_empty() {
                summary_parts.push(format!("Solution: {}", solution.join(" ")));
            }

            if !summary_parts.is_empty() {
                return summary_parts.join("\n");
            }
        }

        if trimmed.contains("# A fatal error has been detected by the Java Runtime Environment") {
            let mut crash_type = "Fatal JVM Crash";
            let mut frame = String::new();
            for line in trimmed.lines() {
                let l = line.trim();
                if l.starts_with("#  EXCEPTION_ACCESS_VIOLATION") || l.starts_with("#  SIGSEGV") {
                    crash_type = l.trim_start_matches("#  ");
                } else if l.starts_with("# C  [")
                    || l.starts_with("# V  [")
                    || l.starts_with("# J  [")
                {
                    frame = l.trim_start_matches("# ").to_string();
                }
            }
            if !frame.is_empty() {
                return format!(
                    "Fatal JVM Crash ({crash_type}): In {frame}.\nCommon causes: graphics driver crash, conflicting overlay (e.g. Discord, RivaTuner), or incompatible native mod."
                );
            }
            return format!(
                "Fatal JVM Crash: {crash_type}. Check graphics drivers and system memory."
            );
        }

        for line in trimmed.lines() {
            let l = line.trim();
            if l.starts_with("Unrecognized VM option") || l.starts_with("Unrecognized option") {
                return format!(
                    "{l}\nCheck and remove invalid JVM flags in Instance Settings or Global Settings."
                );
            }
            if l.starts_with("Invalid maximum heap size")
                || l.starts_with("Invalid initial heap size")
            {
                return format!(
                    "{l}\nThe specified memory allocation is not valid for this Java runtime."
                );
            }
            if l.starts_with("Error: Could not create the Java Virtual Machine") {
                return "Could not create the Java Virtual Machine: Check memory allocation and JVM arguments in settings.".to_string();
            }
            if l.starts_with("Error: Could not find or load main class") {
                return format!("{l}\nMinecraft installation or client jar is corrupt. Try clicking Repair in instance settings.");
            }
        }

        if trimmed.contains("java.lang.OutOfMemoryError")
            || trimmed.contains("Could not reserve enough space for object heap")
        {
            return "Out of Memory: Minecraft ran out of RAM (Java heap space). Increase allocated memory in instance settings or enable Eco Mode.".to_string();
        }

        if trimmed.contains("UnsupportedClassVersionError") {
            return "Java Version Incompatible: Minecraft requires a newer Java runtime than the one currently selected. Please select Automatic or a compatible Java version in Instance Settings.".to_string();
        }

        if trimmed.contains("UnsatisfiedLinkError") || trimmed.contains("Failed to locate library") {
            for line in trimmed.lines() {
                let l = line.trim();
                if l.contains("UnsatisfiedLinkError") || l.contains("Failed to locate library") {
                    return format!("Missing Native Library: {l}");
                }
            }
        }

        if trimmed.contains("MixinApplyError") || trimmed.contains("MixinTransformerError") {
            for line in trimmed.lines() {
                let l = line.trim();
                if l.contains("MixinApplyError") || l.contains("MixinTransformerError") {
                    return format!("Mixin Error: {l}");
                }
            }
        }

        for line in trimmed.lines().rev() {
            let l = line.trim();
            if l.contains("/FATAL]") || l.contains("/ERROR]") {
                if let Some(pos) = l.find("]: ") {
                    let msg = &l[pos + 3..];
                    if !msg.is_empty() {
                        return format!("Error: {msg}");
                    }
                }
                return format!("Error: {l}");
            }
            if l.starts_with("Exception in thread") {
                return l.to_string();
            }
            if l.starts_with("Error: ") {
                return l.to_string();
            }
        }
    }

    match exit_code {
        -1073740791 => "STATUS_STACK_BUFFER_OVERRUN (-1073740791 / 0xC0000409): Buffer overrun detected. Common causes: GPU driver issues, overlays (Discord, RivaTuner/RTSS), or incompatible native mods.".to_string(),
        -1073741819 => "STATUS_ACCESS_VIOLATION (-1073741819 / 0xC0000005): Memory access violation. Common causes: GPU graphics driver crash, corrupted Java runtime, or a faulty mod.".to_string(),
        -1073741571 => "STATUS_CONTROL_C_EXIT (-1073741571 / 0xC000013D): Application terminated abruptly.".to_string(),
        -1073741515 => "STATUS_DLL_NOT_FOUND (-1073741515 / 0xC0000135): Missing required Windows runtime DLL (e.g. Visual C++ Redistributable).".to_string(),
        1 => "Exit Code 1: Game terminated unexpectedly with an unhandled error or early crash.".to_string(),
        255 => "Exit Code 255: Java virtual machine failed during startup or initialization.".to_string(),
        130 => "Exit Code 130: Process terminated by user (SIGINT).".to_string(),
        137 => "Exit Code 137: Process terminated by system out-of-memory killer (SIGKILL).".to_string(),
        -1 => "Process failed to start or connection was terminated.".to_string(),
        other => format!("Minecraft exited abnormally with code {}.", format_exit_code(other)),
    }
}

#[must_use]
pub fn detect_crash(
    instance_id: &str,
    instance_name: &str,
    game_dir: &Path,
    exit_code: i32,
    session_log: Option<&Path>,
    launched_at: Option<std::time::SystemTime>,
) -> CrashInfo {
    let crash_reports_dir = game_dir.join("crash-reports");
    let logs_dir = game_dir.join("logs");

    if let Some((path, content)) = find_latest_crash_report(&crash_reports_dir, launched_at) {
        let summary = extract_crash_summary(&content, exit_code);
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("crash-report.txt");
        return CrashInfo {
            instance_id: instance_id.to_string(),
            instance_name: instance_name.to_string(),
            exit_code,
            summary,
            details: content,
            source_label: format!("Crash report: {filename}"),
            report_path: Some(path),
            crash_reports_dir,
            logs_dir,
        };
    }

    if let Some((path, content)) = find_latest_log(&logs_dir, session_log, launched_at) {
        let summary = extract_crash_summary(&content, exit_code);
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("log");
        let details = tail_lines(&content, 200);
        return CrashInfo {
            instance_id: instance_id.to_string(),
            instance_name: instance_name.to_string(),
            exit_code,
            summary,
            details,
            source_label: format!("Session log (last 200 lines): {filename}"),
            report_path: Some(path),
            crash_reports_dir,
            logs_dir,
        };
    }

    let summary = extract_crash_summary("", exit_code);
    let details = format!(
        "Minecraft process exited abnormally with code {}.\n\nNo recent crash report or session log was found in:\n- {}\n- {}",
        format_exit_code(exit_code),
        crash_reports_dir.display(),
        logs_dir.display()
    );

    CrashInfo {
        instance_id: instance_id.to_string(),
        instance_name: instance_name.to_string(),
        exit_code,
        summary,
        details,
        source_label: "No report or log found".to_string(),
        report_path: None,
        crash_reports_dir,
        logs_dir,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_exit_code_positive_and_negative() {
        assert_eq!(format_exit_code(0), "0");
        assert_eq!(format_exit_code(1), "1");
        assert_eq!(format_exit_code(255), "255");
        assert_eq!(format_exit_code(-1073740791), "-1073740791 (0xC0000409)");
        assert_eq!(format_exit_code(-1073741819), "-1073741819 (0xC0000005)");
        assert_eq!(format_exit_code(-1073741515), "-1073741515 (0xC0000135)");
    }

    #[test]
    fn extract_summary_from_minecraft_crash_report() {
        let report = r#"
---- Minecraft Crash Report ----
// Don't do that.

Time: 2026-09-17 15:34:47
Description: Loading library LWJGL system

java.lang.UnsatisfiedLinkError: Failed to locate library: lwjgl.dll
	at knot//org.lwjgl.system.Library.loadSystem(Library.java:177)

-- Head --
Thread: main
"#;
        let summary = extract_crash_summary(report, 1);
        assert!(summary.contains("Description: Loading library LWJGL system"));
        assert!(summary.contains("Exception: java.lang.UnsatisfiedLinkError: Failed to locate library: lwjgl.dll"));
    }

    #[test]
    fn extract_summary_with_suspected_mods() {
        let report = r#"
---- Minecraft Crash Report ----
Time: 2026-09-20 18:22:10
Description: Initializing game

java.lang.NullPointerException: Registry Object not present: create:brass_casing
	at net.minecraftforge.registries.RegistryObject.get(RegistryObject.java:120)

-- System Details --
Details:
	Minecraft Version: 1.20.1
	Suspected Mods: Create (create), Flywheel (flywheel)
"#;
        let summary = extract_crash_summary(report, 1);
        assert!(summary.contains("Description: Initializing game"));
        assert!(summary.contains("Exception: java.lang.NullPointerException: Registry Object not present: create:brass_casing"));
        assert!(summary.contains("Suspected Mods: Create (create), Flywheel (flywheel)"));
    }

    #[test]
    fn extract_summary_with_multiline_suspected_mods() {
        let report = "---- Minecraft Crash Report ----\nTime: 2026-09-22\nDescription: Loading mods\njava.lang.RuntimeException: Mod load failed\n\nSuspected Mods:\n\t- Create (create)\n\t- JEI (jei)\n";
        let summary = extract_crash_summary(report, 1);
        assert!(summary.contains("Description: Loading mods"));
        assert!(summary.contains("Suspected Mods: Create (create), JEI (jei)"));
    }

    #[test]
    fn extract_summary_crash_report_without_description() {
        let report = "---- Minecraft Crash Report ----\nTime: 2026-09-22 12:00:00\njava.lang.IllegalStateException: Missing block entity\n\tat net.minecraft.world.World.getBlockEntity\n-- Head --\n";
        let summary = extract_crash_summary(report, 1);
        assert!(summary.contains("Exception: java.lang.IllegalStateException: Missing block entity"));
    }

    #[test]
    fn extract_summary_unrecognized_vm_option() {
        let log = "Unrecognized VM option 'BogusArg'\nError: Could not create the Java Virtual Machine.\nError: A fatal exception has occurred. Program will exit.";
        let summary = extract_crash_summary(log, 1);
        assert!(summary.contains("Unrecognized VM option 'BogusArg'"));
        assert!(summary.contains("Check and remove invalid JVM flags"));
    }

    #[test]
    fn extract_summary_could_not_create_jvm() {
        let log = "Error: Could not create the Java Virtual Machine.\nError: A fatal exception has occurred.";
        let summary = extract_crash_summary(log, 1);
        assert!(summary.contains("Could not create the Java Virtual Machine"));
    }

    #[test]
    fn extract_summary_fatal_jvm_crash() {
        let dump = r#"
#
# A fatal error has been detected by the Java Runtime Environment:
#
#  EXCEPTION_ACCESS_VIOLATION (0xc0000005) at pc=0x00007ff812345678, pid=1234, tid=5678
#
# JRE version: OpenJDK Runtime Environment (17.0.8)
# Problematic frame:
# C  [nvoglv64.dll+0x123456]
#
"#;
        let summary = extract_crash_summary(dump, -1073741819);
        assert!(summary.contains("Fatal JVM Crash"));
        assert!(summary.contains("nvoglv64.dll"));
        assert!(summary.contains("graphics driver crash"));
    }

    #[test]
    fn extract_summary_out_of_memory() {
        let log = "Some log line\njava.lang.OutOfMemoryError: Java heap space\nMore lines";
        let summary = extract_crash_summary(log, 1);
        assert!(summary.contains("Out of Memory"));
    }

    #[test]
    fn extract_summary_unsupported_class_version() {
        let log = "java.lang.UnsupportedClassVersionError: net/minecraft/client/main/Main has been compiled by a more recent version";
        let summary = extract_crash_summary(log, 1);
        assert!(summary.contains("Java Version Incompatible"));
    }

    #[test]
    fn extract_summary_exit_codes() {
        let summary_409 = extract_crash_summary("", -1073740791);
        assert!(summary_409.contains("STATUS_STACK_BUFFER_OVERRUN"));

        let summary_005 = extract_crash_summary("", -1073741819);
        assert!(summary_005.contains("STATUS_ACCESS_VIOLATION"));

        let summary_135 = extract_crash_summary("", -1073741515);
        assert!(summary_135.contains("STATUS_DLL_NOT_FOUND"));

        let summary_1 = extract_crash_summary("", 1);
        assert!(summary_1.contains("Exit Code 1"));
    }

    #[test]
    fn tail_lines_helper() {
        let text = "1\n2\n3\n4\n5";
        assert_eq!(tail_lines(text, 3), "3\n4\n5");
        assert_eq!(tail_lines(text, 10), "1\n2\n3\n4\n5");
    }

    #[test]
    fn detect_crash_with_real_files() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path();
        let crash_dir = game_dir.join("crash-reports");
        std::fs::create_dir_all(&crash_dir).unwrap();
        let file_path = crash_dir.join("crash-2026-09-22-client.txt");
        std::fs::write(
            &file_path,
            "---- Minecraft Crash Report ----\nDescription: Testing crash\njava.lang.RuntimeException: Boom!\n",
        )
        .unwrap();

        let info = detect_crash("test-id", "Test Instance", game_dir, 1, None, None);
        assert_eq!(info.instance_name, "Test Instance");
        assert_eq!(info.exit_code, 1);
        assert!(info.summary.contains("Description: Testing crash"));
        assert!(info.summary.contains("Exception: java.lang.RuntimeException: Boom!"));
        assert!(info.source_label.contains("crash-2026-09-22-client.txt"));
        assert_eq!(info.report_path, Some(file_path));
    }

    #[test]
    fn detect_crash_ignores_stale_report_when_since_is_newer() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path();
        let crash_dir = game_dir.join("crash-reports");
        std::fs::create_dir_all(&crash_dir).unwrap();
        let file_path = crash_dir.join("crash-old.txt");
        std::fs::write(
            &file_path,
            "---- Minecraft Crash Report ----\nDescription: Old Crash From Last Week\njava.lang.RuntimeException: Old\n",
        )
        .unwrap();

        let future_launch = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
        let info = detect_crash("test-id", "Test Instance", game_dir, 1, None, Some(future_launch));

        assert_ne!(info.report_path, Some(file_path));
        assert!(!info.summary.contains("Old Crash From Last Week"));
        assert!(info.summary.contains("Exit Code 1"));
    }

    #[test]
    fn detect_crash_prefers_session_log() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path();
        let logs_dir = game_dir.join("logs");
        std::fs::create_dir_all(&logs_dir).unwrap();
        let session_log = logs_dir.join("monoryx-special.log");
        std::fs::write(
            &session_log,
            "Unrecognized VM option 'BogusArg'\nError: Could not create the Java Virtual Machine.\n",
        )
        .unwrap();

        let info = detect_crash(
            "test-id",
            "Test Instance",
            game_dir,
            1,
            Some(&session_log),
            Some(std::time::SystemTime::now() - std::time::Duration::from_secs(10)),
        );
        assert_eq!(info.report_path, Some(session_log));
        assert!(info.summary.contains("Unrecognized VM option 'BogusArg'"));
        assert!(info.source_label.contains("monoryx-special.log"));
    }

    #[test]
    fn detect_crash_fallback_to_log() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path();
        let logs_dir = game_dir.join("logs");
        std::fs::create_dir_all(&logs_dir).unwrap();
        let log_path = logs_dir.join("latest.log");
        std::fs::write(&log_path, "[main/ERROR]: Failed to load mod configuration\n").unwrap();

        let info = detect_crash("test-id", "Test Instance", game_dir, 1, None, None);
        assert_eq!(info.exit_code, 1);
        assert!(info.summary.contains("Failed to load mod configuration"));
        assert!(info.source_label.contains("latest.log"));
        assert_eq!(info.report_path, Some(log_path));
    }

    #[test]
    fn detect_crash_empty_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let info = detect_crash("test-id", "Test Instance", dir.path(), -1073740791, None, None);
        assert_eq!(info.exit_code, -1073740791);
        assert!(info.summary.contains("STATUS_STACK_BUFFER_OVERRUN"));
        assert_eq!(info.report_path, None);
    }
}
