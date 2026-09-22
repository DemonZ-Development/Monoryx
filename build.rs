#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.set("FileDescription", "MONORYX - Minecraft Launcher");
    res.set("ProductName", "MONORYX");
    res.set("ProductVersion", "1.0.0 Beta");
    res.set("FileVersion", "1.0.0.0");
    res.set("LegalCopyright", "DemonZ Development");
    res.set("OriginalFilename", "monoryx.exe");
    if let Err(e) = res.compile() {
        panic!("Failed to compile Windows resource: {e}");
    }
}

#[cfg(not(windows))]
fn main() {}
