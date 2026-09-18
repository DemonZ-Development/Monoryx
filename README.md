# MONORYX

**Minecraft, without the clutter.**

MONORYX is a lightweight, native Minecraft Java Edition launcher written in Rust. No Electron. No browser. No Chromium. Just a fast, monochrome desktop utility focused on speed, instance isolation, offline profiles, and first-class Modrinth integration.

By **DemonZDevelopment**. Licensed under **Apache-2.0**.

## Screenshots

> Placeholder. Add screenshots here:
>
> - `docs/screenshots/home.png`
> - `docs/screenshots/discover.png`
> - `docs/screenshots/library.png`
> - `docs/screenshots/instances.png`

## Features

- Native Rust GUI (egui/eframe), responsive during downloads and installs
- Offline profiles with deterministic `OfflinePlayer:<username>` UUIDs
- Isolated instances with portable `instance.toml` configs
- Real Minecraft installation from Mojang launcher metadata (client, libraries, natives, assets, logging config)
- Loaders: Vanilla, Fabric, Quilt, NeoForge, Forge via official metadata/Maven
- Central download manager: concurrency, resume, retries, atomic writes, SHA-1/SHA-512 verification
- Java discovery (`JAVA_HOME`, `PATH`, system locations, managed runtimes) plus curated Adoptium Temurin installs
- Modrinth search, filters, one-click installs, recursive required-dependency resolution, conflict detection
- Library with enable/disable, updates, Update All, safe removal
- Modrinth `.mrpack` installation into new isolated instances with path-traversal protection
- Resource packs and shader packs
- Repair, export/import, per-instance logs, monochrome theme

## Quick Start

### Requirements

- Windows 10/11 x86_64, Linux x86_64, or macOS (primary: Windows)
- Rust stable 1.85+ (1.96+ recommended)
- On Windows with the GNU toolchain: a MinGW-w64 GCC (e.g. `winget install BrechtSanders.WinLibs.POSIX.UCRT`)
- Java installed, or let MONORYX fetch a managed Temurin JRE on first launch

### Build

```powershell
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features
cargo test
cargo run --release
```

Windows release build produces `target/release/monoryx.exe` with no console window. Linux/macOS produce `target/release/monoryx`.

### First Run

1. Start MONORYX.
2. Enter an offline username (3-16 chars, letters/numbers/underscore).
3. Create an instance, pick a Minecraft version and loader.
4. Press PLAY. Missing files download automatically.
5. Open Discover, search "Sodium", press Install. Dependencies resolve automatically.

## Offline Mode

MONORYX supports **offline profiles only** in this release. Your username maps to a deterministic offline UUID using Java's `UUID.nameUUIDFromBytes("OfflinePlayer:<name>")` algorithm. Offline profiles work for singleplayer and servers in offline mode. They do not grant access to Realms, online-mode servers, or Microsoft services. The account layer is abstracted (`Account::Offline`) so authenticated providers can be added later.

## Modrinth Integration

Public browsing and installation use the official Modrinth API (`https://api.modrinth.com/v2`) with the User-Agent `monoryx/<version> (https://github.com/DemonZDevelopment/monoryx)`. No login required. Rate limits and `Retry-After` are respected. Hashes are verified, required dependencies install recursively, cycles and conflicts are blocked, and installed files are tracked in `content.json` for reliable updates and uninstalls.

## Supported Platforms

- Windows 10/11 x86_64 (primary)
- Linux x86_64
- macOS (best effort; not the primary target but no intentional blockers)

Shared launcher code avoids OS assumptions. OS-specific behavior uses conditional compilation.

## Directory Layout

```text
src/
  main.rs
  app/          state, events, background tasks
  ui/           theme, shell, components, pages
  minecraft/    manifest, version, rules, arguments, libraries, assets, natives, installer, launcher
  loaders/      vanilla, fabric, quilt, neoforge, forge
  modrinth/     api, models, search, install, dependencies, updates, modpack
  downloads/    manager, job, hash
  instance/     config, manager, export
  java/         discovery, runtime, managed
  account/      offline profiles
  storage/      paths, cache, atomic
  content/      installed-content tracking
  config/       launcher config
  utils/        fs, hash, net, system, validation
```

Data root (never touches `.minecraft`):

```text
MONORYX/
  config.toml
  cache/manifests, cache/metadata, cache/images
  minecraft/assets, minecraft/libraries, minecraft/versions
  java/runtimes
  instances/<id>/instance.toml + game/
  logs/
```

## License

Apache-2.0. See `LICENSE`.

## Disclaimer

MONORYX is an independent project by DemonZDevelopment and is not affiliated with Mojang Studios or Microsoft. Minecraft files are downloaded from official Mojang sources at runtime and are never bundled.
