# MONORYX Installation Guide & Tutorials

Welcome to **MONORYX** — a fast, lightweight Minecraft Java Edition launcher built for high performance, isolated instances, offline & free Microsoft authentication, direct companion updates, and built-in Modrinth & CurseForge integration.

> 📢 **Release Notes**: To see what is new in the current release, check out [CHANGELOG.md](CHANGELOG.md).

---

## Table of Contents

1. [Installing MONORYX](#installing-monoryx)
   - [Method A: Windows Installation Wizard (Recommended)](#method-a-windows-installation-wizard-recommended)
   - [Method B: Portable Zip (No Install Required)](#method-b-portable-zip-no-install-required)
   - [Method C: macOS Disk Image (.dmg)](#method-c-macos-disk-image-dmg)
   - [Method D: Building from Source](#method-d-building-from-source)
2. [First Launch & Onboarding](#first-launch--onboarding)
3. [Tutorials](#tutorials)
   - [Tutorial 1: Creating Your First Minecraft Instance](#tutorial-1-creating-your-first-minecraft-instance)
   - [Tutorial 2: Discovering & Installing Mods (Modrinth & CurseForge)](#tutorial-2-discovering--installing-mods-modrinth--curseforge)
   - [Tutorial 3: Free Microsoft Login & Azure Setup](#tutorial-3-free-microsoft-login--azure-setup)
   - [Tutorial 4: Companion Updater (`monoryx-updater`)](#tutorial-4-companion-updater-monoryx-updater)
   - [Tutorial 5: Performance Optimization & Eco Mode](#tutorial-5-performance-optimization--eco-mode)
   - [Tutorial 6: Worlds, Snapshots & Backups](#tutorial-6-worlds-snapshots--backups)
4. [Troubleshooting & FAQ](#troubleshooting--faq)

---

## Installing MONORYX

### Method A: Windows Installation Wizard (Recommended)

The Windows Setup Wizard installs MONORYX cleanly, sets up desktop and start menu shortcuts, and bundles the atomic companion updater.

#### 1. Download the Installer
Head to the [official MONORYX website](https://demonz.org/projects/monoryx) and download `MONORYX-Setup-1.5.0.exe`.

![Releases Page](./assets/instalation-guide/Screenshot%202026-09-29%20110323.png)

#### 2. Open Downloads & Launch Setup
Open File Explorer, go to your **Downloads** folder, and double-click `MONORYX-Setup-1.5.0.exe`.

![Downloads Folder](./assets/instalation-guide/Screenshot%202026-09-29%20110659.png)

#### 3. Windows SmartScreen Prompt
If Windows displays "Windows protected your PC" because the installer is an open-source release without an expensive corporate certificate:
- Click **More info**.

![SmartScreen More Info](./assets/instalation-guide/Screenshot%202026-09-29%20110706.png)

- Click **Run anyway** to start the setup wizard.

![SmartScreen Run Anyway](./assets/instalation-guide/Screenshot%202026-09-29%20110712.png)

#### 4. Choose Installation Scope
- **Only for me** (Recommended): Installs into `%LOCALAPPDATA%\Programs\MONORYX` without needing administrator rights.
- **For all users**: Installs into `C:\Program Files\MONORYX`.

![Install Scope](./assets/instalation-guide/Screenshot%202026-09-29%20110722.png)

#### 5. License Agreement
Review and accept the Apache 2.0 open-source license agreement.

![License Agreement](./assets/instalation-guide/Screenshot%202026-09-29%20110748.png)

#### 6. Destination Location & Shortcuts
Select the destination folder (default recommended) and configure your Start Menu folder and shortcut preferences.

![Destination Directory](./assets/instalation-guide/Screenshot%202026-09-29%20110755.png)

![Start Menu Folder](./assets/instalation-guide/Screenshot%202026-09-29%20110801.png)

![Additional Tasks](./assets/instalation-guide/Screenshot%202026-09-29%20110809.png)

#### 7. Install & Complete
Review your chosen settings and click **Install**. The files and `monoryx-updater.exe` companion will be installed in seconds.

![Ready to Install](./assets/instalation-guide/Screenshot%202026-09-29%20110818.png)

![Installation Progress](./assets/instalation-guide/Screenshot%202026-09-29%20110825.png)

### Method B: Portable Zip (No Install Required)

For USB drives or isolated development environments:

1. Download `monoryx-v1.5.0-windows-x64.zip` from the [official website](https://demonz.org/projects/monoryx).
2. Extract the archive into any preferred folder (e.g. `C:\Games\MONORYX` or `D:\PortableApps\MONORYX`).
3. Ensure both `monoryx.exe` and `monoryx-updater.exe` reside in the same directory.
4. Launch `monoryx.exe` directly. All configurations and instances are saved portably or in user profile data.

### Method C: macOS Disk Image (.dmg)

For macOS 11.0 or newer on Apple Silicon or Intel:

1. Download `monoryx-v1.5.0-macos-universal.dmg` from the [official website](https://demonz.org/projects/monoryx).
2. Double-click the downloaded `.dmg` file to mount it.
3. Drag **MONORYX** into the **Applications** folder shortcut.
4. Open your **Applications** folder and launch **MONORYX**.
5. If macOS displays an unverified developer prompt on first launch, right-click `MONORYX.app` in Finder and select **Open**, or run:
   ```bash
   xattr -cr /Applications/MONORYX.app
   ```

### Method D: Building from Source

If you prefer compiling directly from source on Windows, Linux, or macOS:

#### Prerequisites
- **Rust Toolchain**: Stable Rust 1.88+ (install via [rustup.rs](https://rustup.rs/)).
- **C Compiler (Windows GNU only)**: If using the `x86_64-pc-windows-gnu` target, install MinGW-w64 via `winget install BrechtSanders.WinLibs.POSIX.UCRT`.
- **Java**: Java 8, 17, or 21 (or allow MONORYX to automatically download managed Temurin runtimes).

#### Build & Run
```bash
# Clone the repository
git clone https://github.com/DemonZ-Development/Monoryx.git
cd Monoryx

# Compile both launcher and updater binaries
cargo build --release --bins

# The compiled binaries will be available at:
# target/release/monoryx.exe
# target/release/monoryx-updater.exe
```

---

## First Launch & Onboarding

When starting MONORYX for the first time, you are greeted with the Onboarding Wizard:

![Onboarding Welcome](./assets/instalation-guide/Screenshot%202026-09-29%20111129.png)

1. **Profile Identity**:
   Choose your player name. MONORYX supports both offline player profiles and free Microsoft accounts. Your profile automatically generates a lightweight procedural avatar featuring cute anime-style cat ears and blush cheeks.

![Profile Setup](./assets/instalation-guide/Screenshot%202026-09-29%20111149.png)

2. **Launcher Defaults**:
   Select your preferred theme (Dark, Midnight, Solarized, Monokai, Nord, etc.) and parallel download concurrency (up to 16 simultaneous threads).

![Theme Selection](./assets/instalation-guide/Screenshot%202026-09-29%20111157.png)

3. **Complete & Ready**:
   Click Finish to jump straight into the Home dashboard.

![Onboarding Complete](./assets/instalation-guide/Screenshot%202026-09-29%20111203.png)

---

## Tutorials

### Tutorial 1: Creating Your First Minecraft Instance

MONORYX keeps each instance strictly isolated from others, meaning mods, config files, worlds, and resource packs from one instance never contaminate another.

1. Navigate to the **Instances** tab from the left sidebar navigation.
2. Click **Create Instance** at the top right.
3. Choose:
   - **Instance Name**: e.g., `Survival Fabric 1.21.1`.
   - **Minecraft Version**: Pick any release from `1.0` through the latest snapshot or release (`1.21.4`).
   - **Mod Loader**:
     - `Vanilla`: Pure unmodified Minecraft.
     - `Fabric`: Ultra-fast, modern, lightweight modding.
     - `NeoForge`: Modern successor to Forge with high modpack compatibility.
     - `Forge`: Traditional mod loader for historic or classic modpacks.
     - `Quilt`: Advanced modular loader compatible with Fabric mods.
4. Click **Create**.
5. Hit **Play**. Missing client jars, assets, natives, and libraries are downloaded concurrently with SHA-1 integrity checks.

---

### Tutorial 2: Discovering & Installing Mods (Modrinth & CurseForge)

MONORYX features native search and one-click installation for both **Modrinth** and **CurseForge**.

1. Select your target instance from the top bar or Home page.
2. Click **Discover** in the sidebar.
3. Choose your content category: **Mods**, **Modpacks**, **Resource Packs**, or **Shaders**.
4. Switch **Source** between **Modrinth** and **CurseForge**:
   - **Modrinth**: Free, unauthenticated public search with automatic dependency resolution.
   - **CurseForge**: Connects directly via MONORYX Services out of the box, with optional custom API key support in **Settings** to search millions of mods hosted on CurseForge.
5. Filter by compatible Minecraft version and loader with one click.
6. Click **Install**. MONORYX downloads the correct `.jar` file directly into your instance's `mods` folder and verifies checksums.
7. Go to **Library** to enable, disable, or update installed mods at any time.

---

### Tutorial 3: Free Microsoft Login & Azure Setup

MONORYX includes full Microsoft authentication using Microsoft's standard **OAuth 2.0 Device Code Flow**. This lets you authenticate safely with your existing Microsoft account without paying any extra fees.

#### Logging In with Device Code Flow
1. Click **Accounts** in the left sidebar.
2. Under **Microsoft Account**, click **Login with Microsoft**.
3. MONORYX generates a unique verification code and provides a link to `https://microsoft.com/devicelogin`.
4. Click **Copy Code & Open Browser**.
5. Paste the code into Microsoft's login portal, sign in with your Microsoft account, and approve the app.
6. MONORYX completes the Xbox Live, XSTS, and Minecraft Java service token exchange automatically, loading your verified player name and UUID.

#### Using Your Own Free Azure App Registration (Optional)
If you want to use your own Azure app registration:
1. Go to [portal.azure.com](https://portal.azure.com/#view/Microsoft_AAD_RegisteredApps/ApplicationsListBlade).
2. Click **New registration**.
3. Name it `MONORYX Launcher`, set account type to **Personal Microsoft accounts only**.
4. In **Authentication**, add a platform for **Mobile and desktop applications** and enable `https://login.microsoftonline.com/common/oauth2/nativeclient`.
5. Under **Advanced settings**, set **Allow public client flows** to **Yes**.
6. Copy the **Application (client) ID**.
7. In MONORYX, go to **Accounts** -> **Advanced Client Settings (Optional)** and paste your Client ID.

---

### Tutorial 4: Companion Updater (`monoryx-updater`)

MONORYX checks the [official update API](https://demonz.org/api) using the installed version and platform. It compares versions locally, so older releases and beta releases on stable installations do not trigger an update.

1. Open **Settings** and check for updates. Available updates include release notes from the official API.
2. On Windows, click **Download Update**. The launcher requests the exact version through the official download endpoint and verifies the executable's size and SHA-256 checksum. Checksums come from the API or the selected release's checksum files; installation stops if verification fails or no checksum is available.
3. Click **Restart to apply update**. Binary updates use `monoryx-updater.exe` to wait for the launcher to exit, replace the executable, keep a `.old` backup, and relaunch. Setup packages run the installer in silent mode.
4. On Linux and macOS, **Download Update** opens the [official website](https://demonz.org/projects/monoryx), where you can choose the package for your platform.

---

### Tutorial 5: Performance Optimization & Eco Mode

MONORYX is built in native Rust without Electron or Chromium overhead, keeping idle RAM under 50 MB.

- **Eco Mode**:
  - Enable **Eco Mode** on instances or globally in Settings.
  - Automatically caps memory allocations during casual play and suspends GUI animations when Minecraft is active.
- **When Game Starts**:
  - In Settings -> Launcher, set "When game starts" to **Close / Hide**. This frees launcher GPU and CPU resources entirely while Minecraft is running.
- **JVM Flags Preset**:
  - Recommended Aikar-tuned G1GC arguments are provided out-of-the-box for silky smooth frame times and minimal garbage-collection stutter.
- **Dedicated GPU Preference**:
  - On laptops with dual GPUs (Intel/AMD integrated + NVIDIA/AMD dedicated), MONORYX configures Windows High Performance GPU preference to ensure Minecraft runs on your dedicated card.

---

### Tutorial 6: Worlds, Snapshots & Backups

Never lose your survival progress:

1. Go to the **Worlds** page in the sidebar.
2. Select any world from your active instance.
3. Click **Back Up World**. MONORYX creates a compressed timestamped archive in your instance's backup folder.
4. If a world is corrupted or you want to revert changes, click **Restore**. MONORYX restores the world as a separate, safe copy so your current world is never accidentally destroyed.

---

## Troubleshooting & FAQ

#### Q: Minecraft fails to launch or reports missing Java?
Go to **Settings** -> **Java & GPU**. Click **Detect Installed Runtimes** or select **Install Managed Temurin JRE** to let MONORYX configure the correct Java version automatically.

#### Q: Do I need a CurseForge API key?
No. CurseForge search works automatically via MONORYX Services with no setup required. If you prefer to use your own personal developer key, you can enter it in **Settings** -> **Launcher** under **CurseForge Integration**.

#### Q: Can I play offline without internet?
Yes! MONORYX has full first-class offline support. Simply create an offline profile in **Accounts** and play any installed instance anytime without network connectivity.
