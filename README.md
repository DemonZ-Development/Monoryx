# MONORYX

<div align="center">

![MONORYX Banner](assets/onboarding-bg.jpg)

### A native Minecraft Java launcher written in Rust.

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/Release-v1.5.0-success.svg)](https://demonz.org/projects/monoryx)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-informational.svg)](https://demonz.org/projects/monoryx)

[**Download MONORYX**](https://demonz.org/projects/monoryx) • [**Installation Guide**](INSTALLATION.md) • [**Changelog**](CHANGELOG.md) • [**Issue Tracker**](https://github.com/DemonZ-Development/Monoryx/issues)

</div>

---

## Overview

MONORYX is a desktop launcher for Minecraft: Java Edition. It runs as a native Rust binary using `egui` and `eframe`. The executable is 20 MB and idles at roughly 30 MB of memory.

- **Separate instance folders**: Each profile keeps its own mods, saves, configs, and screenshots in an isolated directory. It leaves your `.minecraft` folder untouched.
- **Modrinth and CurseForge support**: Search projects, resolve required dependencies, and install `.mrpack` modpacks or individual mods from the UI.
- **Offline and Microsoft accounts**: Authenticate through Microsoft OAuth device code flow (`microsoft.com/link`) for Mojang servers, or pick an offline profile for singleplayer and LAN play.
- **Java management**: Detects installed JREs on your system and downloads Adoptium Temurin runtimes when an instance needs a specific Java version.
- **Launch tuning**: Includes AppCDS class caching, Aikar GC flags, memory controls, and discrete GPU selection on Windows.

---

## Features

### Mod Browsing and Updates
- Search Modrinth and CurseForge with version and loader filters.
- Install mods, resource packs, and shaders into your selected instance.
- Dependency resolution detects required libraries and warns on conflicting versions.
- The update scanner reads local jar hashes and Murmur2 fingerprints to check for new releases across both platforms.

### Instance Management
- Supports Vanilla, Fabric, Quilt, NeoForge, and Forge.
- Set per-instance memory bounds, JVM flags, and Java paths.
- Export instances to portable `.zip` archives or import existing archives.
- Assign custom icons and toggle Minecraft snapshots.

### Diagnostics and World Tools
- Crash analyzer parses JVM logs and highlights failing mod IDs.
- Upload logs to [mclo.gs](https://mclo.gs) with a single click.
- Create compressed backups of local worlds with a restore-as-copy option.
- View world metadata, seeds, and screenshots from the launcher.

### Integrations
- Discord Rich Presence shows your instance, playtime, and world or server name with per-item privacy toggles.
- Nexeu game panel integration connects to remote server consoles, monitors CPU and memory load, and triggers power actions.
- Update checks and release notes come from the [official MONORYX API](https://demonz.org/api). Windows updates require SHA-256 verification before installation; the companion updater handles binary replacement.

---

## Getting Started

### Windows

1. Download `MONORYX-Setup-1.5.0.exe` or the portable zip from the [official website](https://demonz.org/projects/monoryx).
2. Run the installer or extract the zip archive.
3. Open MONORYX, set your username or log in with Microsoft, and select your memory limit.
4. Click **Create Instance**, choose your Minecraft version and loader, then click **Play**.

For detailed setup steps and screenshots, read [INSTALLATION.md](INSTALLATION.md).

---

## Building from Source

### Requirements

- Rust 1.88 or newer
- Windows, Linux, or macOS
- C compiler toolchain (MSVC on Windows, GCC/Clang on Linux/macOS)
- Linux dependencies: `libssl-dev`, `pkg-config`, `libasound2-dev`, `libfontconfig1-dev`

### Compilation

```bash
git clone https://github.com/DemonZ-Development/Monoryx.git
cd Monoryx

cargo check --all-targets
cargo test

cargo build --release --bin monoryx
```

The output binary is placed in:
- Windows: `target/release/monoryx.exe`
- Linux/macOS: `target/release/monoryx`

To build the companion updater:

```bash
cargo build --release --bin monoryx-updater
```

---

## Storage Layout

MONORYX stores all files in an isolated data directory:

```
%APPDATA%/DemonZDevelopment/MONORYX/
├── config.toml           # Launcher settings and credential references
├── cache/                # Cached manifests, images, and API responses
├── minecraft/            # Assets, libraries, and client jars
├── java/                 # Downloaded Temurin runtimes
├── instances/            # Game profiles
│   └── <instance-id>/
│       ├── instance.toml # Instance settings
│       ├── content.json  # Installed mod index
│       └── game/         # mods, configs, saves, screenshots
└── logs/                 # Launcher log files
```

Microsoft tokens and custom CurseForge API keys are stored in Windows Credential Manager, macOS Keychain, or the Linux Secret Service. Existing plaintext credentials migrate automatically after the credential store accepts them. If secure storage is unavailable, the original settings file is preserved. Linux sign-in requires an unlocked Secret Service provider, such as GNOME Keyring or KWallet.

Settings saves keep a `config.toml.bak` backup without Microsoft credentials or custom API keys. If the main settings file becomes malformed, the launcher preserves it separately and restores that backup; Microsoft sign-in and custom API keys must then be configured again.

---

## Source Tree

```
src/
├── main.rs               # Launcher entry point and UI loop
├── updater_main.rs       # Standalone updater binary
├── account/              # Offline and Microsoft OAuth authentication
├── app/                  # Application state, background tasks, AppCDS
├── config/               # Settings persistence
├── content/              # Installed mod tracking
├── curseforge.rs         # CurseForge API and Murmur2 hashing
├── downloads/            # Chunked downloader with SHA verification
├── instance/             # Instance configuration, export, world backups
├── java/                 # Java detection and Temurin downloads
├── loaders/              # Fabric, Quilt, NeoForge, Forge installers
├── minecraft/            # Manifest parser, launch arguments, crash analyzer
├── modrinth/             # Modrinth API, search, dependency resolution
├── nexeu.rs              # Nexeu game server panel client
├── storage/              # Cache management and atomic file writes
├── ui/                   # egui interface components, themes, pages
└── utils/                # System metrics, file utilities, validation
```

---

## FAQ

**Do I need to own Minecraft to use MONORYX?**  
No. MONORYX supports offline profiles for singleplayer and LAN servers. If you own Minecraft Java Edition, you can sign in with your Microsoft account to join online Mojang servers.

**Can I move worlds and mods from another launcher?**  
Yes. Copy your `mods/`, `saves/`, and config files into the `instances/<instance-id>/game/` directory. MONORYX detects them on startup.

**How does memory allocation work?**  
Automatic memory detects your installed RAM and sets a safe cap. You can turn on Eco Mode to reduce allocation or enter custom values in settings.

---

## License

MONORYX is released under the [Apache License 2.0](LICENSE).

Minecraft is a registered trademark of Mojang Synergies AB. MONORYX is an independent project by DemonZ Development and is not affiliated with Mojang Studios or Microsoft.
