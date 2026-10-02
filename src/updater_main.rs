use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
#[cfg(not(target_os = "windows"))]
use std::time::Instant;

#[derive(Debug, Default)]
struct UpdaterArgs {
    source: Option<PathBuf>,
    target: Option<PathBuf>,
    wait_pid: Option<u32>,
    relaunch: bool,
}

fn parse_args() -> UpdaterArgs {
    parse_args_from(std::env::args().skip(1))
}

fn parse_args_from<I: Iterator<Item = String>>(mut iter: I) -> UpdaterArgs {
    let mut args = UpdaterArgs::default();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--update-source" | "--source" => {
                if let Some(val) = iter.next() {
                    args.source = Some(PathBuf::from(val));
                }
            }
            "--target-dest" | "--target" => {
                if let Some(val) = iter.next() {
                    args.target = Some(PathBuf::from(val));
                }
            }
            "--wait-pid" | "--pid" => {
                if let Some(val) = iter.next() {
                    args.wait_pid = val.parse().ok();
                }
            }
            "--relaunch" => {
                args.relaunch = true;
            }
            _ => {}
        }
    }
    args
}

#[cfg(target_os = "windows")]
fn wait_for_process_exit(pid: u32, timeout: Duration) -> bool {
    const SYNCHRONIZE: u32 = 0x00100000;
    const WAIT_OBJECT_0: u32 = 0;
    unsafe {
        let handle = windows_sys::Win32::System::Threading::OpenProcess(SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return true;
        }
        let millis = timeout.as_millis().min(u32::MAX as u128) as u32;
        let wait = windows_sys::Win32::System::Threading::WaitForSingleObject(handle, millis);
        windows_sys::Win32::Foundation::CloseHandle(handle);
        wait == WAIT_OBJECT_0
    }
}

#[cfg(not(target_os = "windows"))]
fn wait_for_process_exit(pid: u32, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if !Path::new(&format!("/proc/{pid}")).exists() {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    !Path::new(&format!("/proc/{pid}")).exists()
}

fn backup_path_for(target: &Path) -> PathBuf {
    let old_name = format!(
        "{}.old",
        target.file_name().and_then(|n| n.to_str()).unwrap_or("app")
    );
    target.with_file_name(old_name)
}

fn perform_swap(source: &Path, target: &Path) -> std::io::Result<()> {
    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Update source file does not exist: {}", source.display()),
        ));
    }

    let old_backup = backup_path_for(target);
    if old_backup.exists() {
        let _ = std::fs::remove_file(&old_backup);
    }

    if target.exists() {
        let mut renamed = false;
        let mut last_err = None;
        for _ in 0..25 {
            match std::fs::rename(target, &old_backup) {
                Ok(_) => {
                    renamed = true;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }
        if !renamed {
            if let Some(err) = last_err {
                return Err(err);
            }
        }
    }

    let mut moved = false;
    let mut last_err = None;
    for _ in 0..10 {
        match std::fs::copy(source, target) {
            Ok(_) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ =
                        std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755));
                }
                moved = true;
                break;
            }
            Err(e) => {
                last_err = Some(e);
                thread::sleep(Duration::from_millis(150));
            }
        }
    }

    if !moved {
        if old_backup.exists() {
            let _ = std::fs::rename(&old_backup, target);
        }
        if let Some(err) = last_err {
            return Err(err);
        }
    }

    let _ = std::fs::remove_file(&old_backup);
    let _ = std::fs::remove_file(source);

    Ok(())
}

fn main() {
    let args = parse_args();
    let Some(source) = args.source else {
        eprintln!("monoryx-updater: missing required --update-source argument");
        std::process::exit(1);
    };
    let Some(target) = args.target else {
        eprintln!("monoryx-updater: missing required --target-dest argument");
        std::process::exit(1);
    };

    if let Some(pid) = args.wait_pid {
        if !wait_for_process_exit(pid, Duration::from_secs(25)) {
            eprintln!("monoryx-updater error: launcher process {pid} did not exit in time");
            std::process::exit(3);
        }
    }

    thread::sleep(Duration::from_millis(250));

    if let Err(e) = perform_swap(&source, &target) {
        eprintln!("monoryx-updater error: {e}");
        std::process::exit(2);
    }

    if args.relaunch {
        let _ = std::process::Command::new(&target).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_all_flags() {
        let raw = vec![
            "--source".to_string(),
            "/path/to/new.exe".to_string(),
            "--target".to_string(),
            "/path/to/app.exe".to_string(),
            "--pid".to_string(),
            "1234".to_string(),
            "--relaunch".to_string(),
        ];
        let parsed = parse_args_from(raw.into_iter());
        assert_eq!(parsed.source, Some(PathBuf::from("/path/to/new.exe")));
        assert_eq!(parsed.target, Some(PathBuf::from("/path/to/app.exe")));
        assert_eq!(parsed.wait_pid, Some(1234));
        assert!(parsed.relaunch);
    }

    #[test]
    fn parse_args_empty() {
        let parsed = parse_args_from(Vec::new().into_iter());
        assert_eq!(parsed.source, None);
        assert_eq!(parsed.target, None);
        assert_eq!(parsed.wait_pid, None);
        assert!(!parsed.relaunch);
    }

    #[test]
    fn perform_swap_missing_source_fails() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("nonexistent.exe");
        let target = temp.path().join("app.exe");
        let res = perform_swap(&src, &target);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn perform_swap_success() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("update.exe");
        let target = temp.path().join("app.exe");

        std::fs::write(&target, b"original-version").unwrap();
        std::fs::write(&src, b"new-version").unwrap();

        let res = perform_swap(&src, &target);
        assert!(res.is_ok());

        assert!(!src.exists());
        assert!(target.exists());
        assert_eq!(std::fs::read(&target).unwrap(), b"new-version");
        assert!(!backup_path_for(&target).exists());
    }
}
