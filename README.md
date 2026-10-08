# MONORYX

<div align="center">

![MONORYX Interface](assets/screenshots/home-halloween.png)

### A native Minecraft Java launcher written in Rust.

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/Release-v1.5.2-success.svg)](https://demonz.org/projects/monoryx)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-informational.svg)](https://demonz.org/projects/monoryx)

[**Download MONORYX**](https://demonz.org/projects/monoryx) • [**Installation Guide**](INSTALLATION.md) • [**Changelog**](CHANGELOG.md) • [**Marketplace Overview**](MARKETPLACE.md) • [**Issue Tracker**](https://github.com/DemonZ-Development/Monoryx/issues)

</div>

---

## Overview

MONORYX is a desktop launcher for Minecraft: Java Edition written in Rust, `egui`, and `eframe`. It compiles to a native binary with an idle memory footprint under 50 MB.

- **Isolated instance directories**: Every instance keeps its own mods, saves, configs, and screenshots in a dedicated directory. Your `.minecraft` directory remains untouched.
- **Dual-source content discovery**: Search projects across Modrinth and CurseForge, resolve required dependencies, and install `.mrpack` modpacks or individual mods from a single interface.
- **Offline and Microsoft accounts**: Sign in via Microsoft OAuth device code flow (`microsoft.com/link`) for authenticated Mojang servers, or create an offline profile for singleplayer and LAN worlds.
- **Automated Java management**: Detects local JREs and downloads matched Adoptium Temurin runtimes when an instance requires a specific Java version.
- **Launch tuning**: Features AppCDS class data sharing, Aikar garbage collection presets, customizable memory allocations, and discrete GPU selection on Windows.
- **Native Windows binary**: Runs as a standard Windows GUI application without opening a console terminal.

---

## Visual Showcase

<div align="center">

| Discovery (Modrinth & CurseForge) | Mod Library & Dependencies |
|:---:|:---:|
| ![Discover](assets/screenshots/discover-wide.png) | ![Library](assets/screenshots/library-wide.png) |

| Worlds & Compressed Backups | Classic Monochrome Theme |
|:---:|:---:|
| ![Worlds](assets/screenshots/worlds-wide.png) | ![Home Classic](assets/screenshots/home-wide.png) |

</div>

---

## Features

### Mod Browsing and Updates
- Search both Modrinth and CurseForge with Minecraft version and loader filters.
- Install mods, resource packs, and shaders into your active instance.
- Dependency resolution detects required libraries and warns on incompatible versions.
- Background update scanner checks local jar hashes and Murmur2 fingerprints for new releases.
- Dense Library table displays mod states, update badges, and dependency trees.

### Instance Management
- Supports Vanilla, Fabric, Quilt, NeoForge, and Forge loaders.
- Set per-instance memory caps, JVM flags, and Java paths.
- Export instances to portable `.zip` archives or import existing instance archives.
- Assign custom icons, inspect playtime, and launch with square control buttons.

### Diagnostics and World Tools
- Crash analyzer parses JVM logs, highlights failing mod IDs, and offers direct export.
- Upload logs to [mclo.gs](https://mclo.gs) with one click.
- Create compressed ZIP backups of local worlds with a restore-as-copy option.
- Browse world folders, inspect level metadata, and preview saved screenshots in full size.

### Integrations
- Discord Rich Presence displays your active instance, playtime, and world name with configurable privacy toggles.
- Nexeu game panel integration connects to remote server consoles, monitors CPU and memory load, and triggers power actions.
- Official update client verifies SHA-256 signatures before applying updates through the companion updater.

---

## Getting Started

### Windows

1. Download `MONORYX-Setup-1.5.2.exe` or the portable zip archive from the [official website](https://demonz.org/projects/monoryx).
2. Run the installer or extract the zip archive.
3. Open MONORYX, set your username or log in with Microsoft, and select your memory limit.
4. Click **Create Instance**, choose your Minecraft version and loader, then click **Play**.

### macOS

1. Download `monoryx-v1.5.2-macos-universal.dmg` for Intel or Apple Silicon Macs.
2. Open the `.dmg` and drag `MONORYX.app` to your Applications folder.
3. Launch MONORYX from Applications or Spotlight.

### Linux

1. Download `monoryx-v1.5.2-linux-x64.tar.gz`.
2. Extract the archive: `tar -xzf monoryx-v1.5.2-linux-x64.tar.gz`.
3. Run the executable: `./monoryx`.

For step-by-step setup guides and platform notes, refer to [INSTALLATION.md](INSTALLATION.md).

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
cargo test --all

cargo build --release --bin monoryx
```

The compiled binary is placed at:
- Windows: `target/release/monoryx.exe`
- Linux/macOS: `target/release/monoryx`

To compile the companion updater:

```bash
cargo build --release --bin monoryx-updater
```

---

## Storage Layout

MONORYX maintains all instance data in an isolated application directory:

```
%APPDATA%/DemonZDevelopment/MONORYX/
├── config.toml           # Launcher preferences and configuration
├── cache/                # Cached manifests, thumbnails, and API responses
├── minecraft/            # Assets, libraries, and client jars
├── java/                 # Downloaded Adoptium Temurin runtimes
├── instances/            # Isolated game profiles
│   └── <instance-id>/
│       ├── instance.toml # Instance settings and loader metadata
│       ├── content.json  # Installed mod manifest
│       └── game/         # mods, configs, saves, screenshots
└── logs/                 # Launcher execution logs
```

Credentials (Microsoft tokens and custom CurseForge API keys) are stored in your operating system credential store: Windows Credential Manager, macOS Keychain, or Linux Secret Service (via GNOME Keyring or KWallet). Plaintext credentials migrate to the system keyring on initial access.

---

## FAQ

**Do I need an existing Minecraft purchase to use MONORYX?**  
No. MONORYX supports offline profiles for singleplayer and local LAN play. If you own Minecraft Java Edition, you can log in with your Microsoft account to connect to official Mojang servers.

**Can I transfer worlds and mods from another launcher?**  
Yes. Copy your `mods/`, `saves/`, and config files into the `instances/<instance-id>/game/` directory. MONORYX indexes them on startup.

**How does memory allocation work?**  
Automatic memory detects system RAM and assigns a safe default cap. You can enable Eco Mode to reduce resource usage or configure custom values in instance settings.

---

## License

MONORYX is licensed under the [Apache License 2.0](LICENSE).

Minecraft is a registered trademark of Mojang Synergies AB. MONORYX is an independent project by DemonZ Development and is not affiliated with Mojang Studios or Microsoft.
