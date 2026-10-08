use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Default)]
struct UpdaterArgs {
    source: Option<PathBuf>,
    target: Option<PathBuf>,
    wait_pid: Option<u32>,
    relaunch: bool,
}

fn parse_args(mut iter: impl Iterator<Item = String>) -> UpdaterArgs {
    let mut args = UpdaterArgs::default();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--update-source" | "--source" => args.source = iter.next().map(PathBuf::from),
            "--target-dest" | "--target" => args.target = iter.next().map(PathBuf::from),
            "--wait-pid" | "--pid" => {
                args.wait_pid = iter.next().and_then(|value| value.parse().ok())
            }
            "--relaunch" => args.relaunch = true,
            _ => {}
        }
    }
    args
}

#[cfg(target_os = "windows")]
fn wait_for_process_exit(pid: u32, timeout: Duration) -> bool {
    unsafe {
        let handle = windows_sys::Win32::System::Threading::OpenProcess(0x00100000, 0, pid);
        if handle.is_null() {
            std::thread::sleep(Duration::from_millis(200));
            return true;
        }
        let result = windows_sys::Win32::System::Threading::WaitForSingleObject(
            handle,
            timeout.as_millis().min(u32::MAX as u128) as u32,
        );
        windows_sys::Win32::Foundation::CloseHandle(handle);
        if result == 0 {
            std::thread::sleep(Duration::from_millis(200));
            true
        } else {
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn wait_for_process_exit(pid: u32, timeout: Duration) -> bool {
    use sysinfo::{Pid, ProcessesToUpdate, System};
    let process = Pid::from_u32(pid);
    let mut system = System::new();
    let start = std::time::Instant::now();
    loop {
        system.refresh_processes(ProcessesToUpdate::Some(&[process]), true);
        if system.process(process).is_none() {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn unique_backup_path_for(target: &Path) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    target.with_file_name(format!(
        "{}.old.{}.{}",
        target.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id(),
        timestamp
    ))
}

fn clean_stale_backups(target: &Path) {
    let Some(parent) = target.parent() else {
        return;
    };
    let Some(file_name) = target.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let prefix = format!("{file_name}.old");
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if name.starts_with(&prefix) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

fn retry(mut operation: impl FnMut() -> Result<()>) -> Result<()> {
    for attempt in 0..50 {
        match operation() {
            Ok(()) => return Ok(()),
            Err(error) if attempt == 49 => return Err(error),
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    unreachable!()
}

fn apply_with_launch(
    source: &Path,
    target: &Path,
    launch: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    if !source.is_file() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "Update source is not a file",
        ));
    }
    if source.canonicalize().ok() == target.canonicalize().ok() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Update source and target must be different files",
        ));
    }
    if source.metadata()?.len() == 0 {
        return Err(Error::new(ErrorKind::InvalidData, "Update source is empty"));
    }
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(&mut std::fs::File::open(source)?, &mut staged)?;
    staged.as_file().sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        staged
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o755))?;
    }
    let backup = unique_backup_path_for(target);
    let had_target = target.exists();
    if had_target {
        retry(|| std::fs::rename(target, &backup))
            .map_err(|e| Error::other(format!("renaming target to backup failed: {e}")))?;
    }
    let persisted = staged
        .persist(target)
        .map_err(|error| Error::other(format!("persisting staged file failed: {}", error.error)));
    let result = match persisted {
        Ok(file) => {
            drop(file);
            launch(target)
                .map_err(|e| Error::other(format!("relaunching updated executable failed: {e}")))
        }
        Err(error) => Err(error),
    };
    if let Err(error) = result {
        if target.exists() {
            let _ = retry(|| std::fs::remove_file(target));
        }
        if had_target {
            retry(|| std::fs::rename(&backup, target)).map_err(|restore| {
                Error::other(format!(
                    "{error}; rollback failed: {restore}. Previous executable is preserved at {}",
                    backup.display()
                ))
            })?;
        }
        return Err(error);
    }
    if had_target {
        let _ = retry(|| std::fs::remove_file(&backup));
    }
    let _ = retry(|| std::fs::remove_file(source));
    clean_stale_backups(target);
    Ok(())
}

fn spawn_with_retry(path: &Path) -> Result<()> {
    let mut cmd = std::process::Command::new(path);
    if let Some(parent) = path.parent() {
        cmd.current_dir(parent);
    }
    for attempt in 0..50 {
        match cmd.spawn() {
            Ok(_) => return Ok(()),
            Err(error) if attempt == 49 => return Err(error),
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    unreachable!()
}

pub fn run() -> Result<()> {
    let args = parse_args(std::env::args().skip(1));
    let source = args
        .source
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Missing --update-source"))?;
    let target = args
        .target
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Missing --target-dest"))?;
    if args
        .wait_pid
        .is_some_and(|pid| !wait_for_process_exit(pid, Duration::from_secs(30)))
    {
        return Err(Error::new(
            ErrorKind::TimedOut,
            "Launcher did not exit before the update timeout",
        ));
    }
    apply_with_launch(&source, &target, |path| {
        if args.relaunch {
            spawn_with_retry(path)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_launcher_and_companion_flags() {
        for flags in [
            ["--source", "--target", "--pid"],
            ["--update-source", "--target-dest", "--wait-pid"],
        ] {
            let args = parse_args(
                [
                    flags[0],
                    "new.exe",
                    flags[1],
                    "app.exe",
                    flags[2],
                    "123",
                    "--relaunch",
                ]
                .into_iter()
                .map(str::to_string),
            );
            assert_eq!(args.source, Some("new.exe".into()));
            assert_eq!(args.target, Some("app.exe".into()));
            assert_eq!(args.wait_pid, Some(123));
            assert!(args.relaunch);
        }
    }

    #[test]
    fn failed_relaunch_restores_previous_executable_and_preserves_download() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("new.exe");
        let target = dir.path().join("app.exe");
        std::fs::write(&source, b"new").unwrap();
        std::fs::write(&target, b"old").unwrap();
        assert!(
            apply_with_launch(&source, &target, |_| Err(Error::other("launch failed"))).is_err()
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        assert_eq!(std::fs::read(&source).unwrap(), b"new");
    }

    #[test]
    fn invalid_source_leaves_previous_executable_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("app.exe");
        std::fs::write(&target, b"old").unwrap();
        assert!(apply_with_launch(&dir.path().join("missing"), &target, |_| Ok(())).is_err());
        assert!(apply_with_launch(&target, &target, |_| Ok(())).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
    }

    #[test]
    fn successful_swap_cleans_up_download_and_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("new.exe");
        let target = dir.path().join("app.exe");
        std::fs::write(&source, b"new").unwrap();
        std::fs::write(&target, b"old").unwrap();
        apply_with_launch(&source, &target, |path| {
            assert_eq!(std::fs::read(path)?, b"new");
            Ok(())
        })
        .unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(!source.exists());
        let stale = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".old"))
            .count();
        assert_eq!(stale, 0);
    }
}
