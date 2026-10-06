mod update_swap;

fn main() {
    if let Err(error) = update_swap::run() {
        eprintln!("monoryx-updater: {error}");
        std::process::exit(1);
    }
}
