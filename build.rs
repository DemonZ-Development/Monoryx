#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.set("FileDescription", "MONORYX - Minecraft Launcher");
    res.set("ProductName", "MONORYX");
    res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
    res.set("FileVersion", &format!("{}.0", env!("CARGO_PKG_VERSION")));
    res.set("LegalCopyright", "DemonZ Development");
    res.set("OriginalFilename", "monoryx.exe");
    if let Err(e) = res.compile() {
        panic!("Failed to compile Windows resource: {e}");
    }
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let resource = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("resource.o");
        println!("cargo:rustc-link-arg-bin=monoryx={}", resource.display());
    }
}

#[cfg(not(windows))]
fn main() {}
