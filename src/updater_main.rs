mod update_swap;

fn main() {
    if let Err(error) = update_swap::run() {
        eprintln!("monoryx-updater: {error}");
        let log_path = std::env::temp_dir().join("monoryx-updater.log");
        let _ = std::fs::write(&log_path, format!("{error}\n"));
        std::process::exit(1);
    }
}
