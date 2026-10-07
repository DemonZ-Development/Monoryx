# MONORYX 1.5.1

Quick patch fixing a startup crash after signing into Microsoft, switching Microsoft login to our official Azure Application registration, and adding build timeouts to CI.

> See [INSTALLATION.md](INSTALLATION.md) for platform setup guides and tutorials.

## Authentication & Profiles

- **Official Azure Application Registration**: Swapped out the placeholder client ID for our official Monoryx Azure Application ID (`e6fe0b23-f185-4bbb-a6e1-7e72ed727a6f`). Microsoft device logins now display **MONORYX** rather than Prism Launcher.
- **Startup Crash Fix for Microsoft Accounts**: Fixed a crash that closed the launcher a split second after opening when an active Microsoft account was present. The avatar image fetcher now safely checks for an active Tokio runtime handle before dispatching tasks, preventing thread panics on the UI thread.

## Build Pipelines & CI

- **Workflow Timeouts**: Added a 15-minute cap to CI test jobs so hung package mirrors cannot run for hours.
- **Workflow Concurrency**: Pushing fresh commits now cancels older in-flight test runs automatically to save runner minutes.
- **Resilient Package Downloads**: Linux package steps now use retry and timeout flags to avoid stalling on dead mirrors.

---

# MONORYX 1.5.0

Major UI overhaul, seasonal Halloween theme with procedural artwork, comprehensive world backup and screenshot viewer suites, dual-source Discover architecture, streamlined onboarding, and memory optimizations.

> See [INSTALLATION.md](INSTALLATION.md) for platform setup guides and tutorials.

## Interface, Theming & Visuals

- **Home Rework:** Redesigned Home around the primary instance hero card with square action buttons (`Play Minecraft`, `Mods & packs`, `Worlds & backups`, `Instance settings`), instance stats (mods, memory, last played, playtime), clean subtle non-glowing borders, and secondary maintenance actions consolidated under the `More` menu.
- **Seasonal Spooky Theme:** Added the Halloween theme featuring procedural silhouetted vector backdrop artwork (haunted castle, glowing full moon, flying bats, and misty pine forest), custom bat header wordmark, carved pumpkin corner accents, and spiderweb card flourishes.
- **Theme Palette Expansion:** Added distinct theme options (Halloween, Gloss, High Contrast, Dark, Light) with refined accent palettes, crisp text contrast ratios, and persistent bottom-sidebar access for Accounts, Settings, and Logs.
- **Modernized Modal Dialogs:** Project and pack detail pages now open as centered floating dialogs with a dual-texture GPU blur effect and background dimming.
- **Windows Subsystem Configuration:** Attached `#![windows_subsystem = "windows"]` to production builds, eliminating command prompt console windows when launching `monoryx.exe` on Windows.
- **Onboarding Experience:** Streamlined initial onboarding into a compact three-step wizard with automatic memory allocation, detected GPU preference selection, high-contrast dropdown text styling, and reliable Back/Continue navigation.

## Discovery, Downloads & Mod Management

- **Dual-Source Content Discovery:** Browse, filter, and search both Modrinth and CurseForge catalogs directly from Discover, complete with dynamic context-aware search placeholders.
- **Installation Safety Guardrails:** Discover detects when instance core game files are still pending download and guides the user before allowing mod or pack installs.
- **Compact Library View:** Compacted mod list rows by 25% for high-density information display, featuring update indicators, direct version selection, and an integrated dependency hierarchy tree.
- **Isolated Multi-Stage Downloads:** Downloads page provides animated phase-by-phase tracking for client jars, libraries, assets, and third-party packs, with independent failure recovery and single-item retry.
- **Background Update Scanning:** Selected instances check for compatible package and loader updates on launch and every 30 minutes, presenting update notifications without intrusive popups.

## Worlds, Screenshots & System Tools

- **World Backup & Explorer:** Added a dedicated Worlds manager supporting ZIP-compressed backups, restore-as-copy capabilities, in-app folder browsing, and world metadata previews.
- **Integrated Screenshot Gallery:** Added full-size screenshot viewer with navigation controls, alongside a filmstrip preview strip directly on the Home page.
- **Diagnostic Crash Reporting:** Redesigned crash report modal with automated stack trace parsing, problem diagnosis summaries, and one-click log export.
- **Advanced Compatibility:** Relocated technical UUID RFC4122 specifications and hash details behind an Advanced Compatibility accordion on the Accounts page for a cleaner interface.

## Performance & Resource Utilization

- **Bounded Image Caching:** Implemented bounded texture capacities and aggressive negative thumbnail caching, pruning queues when switching views to conserve system RAM.
- **Persistent Font Fallback Cache:** CJK and system fallback fonts resolve once on startup and cache across theme switches, avoiding repetitive font file reads.
- **Reduced Idle Repaints:** Removed persistent per-download repaint threads and restricted frame requests strictly to active animations, maintaining a lightweight runtime footprint during idle states.

---

# MONORYX 1.4.1

CJK font support, asynchronous player skin avatar rendering, and stability improvements.

This patch shipped while 1.5.0 was in development, so the community fixes in [PR #3](https://github.com/DemonZ-Development/Monoryx/pull/3) could reach users sooner. These fixes are also included in 1.5.0.

> See [INSTALLATION.md](INSTALLATION.md) for platform setup guides and tutorials.

## Internationalization & Fonts
- **System fallback fonts for CJK characters:** Added automatic system font fallbacks (`Segoe UI`, `Malgun Gothic`, `Microsoft YaHei`, `MS Gothic` on Windows, with macOS and Linux fallbacks) to resolve missing glyphs on non-English / CJK systems (#3).
- **One-time font initialization:** Fallback font resolution is cached and executed once per session on startup rather than during theme switches, keeping UI theme changes instant.

## Player Profiles & Avatars
- **Async Mojang skin avatar rendering:** Microsoft account profiles now render the player's actual skin head avatar in the sidebar and accounts manager.
- **Non-blocking skin pipeline:** Player skins and head textures are fetched and decoded asynchronously in Tokio background tasks with an instant placeholder fallback, eliminating UI frame freezes during network calls.

## Maintenance
- Cleaned up backend dependencies and removed unused blocking network features.

---

# MONORYX 1.4.0

Universal macOS builds, integrated CurseForge browser with smart update scanning, in-place companion updater, Microsoft device authentication in the onboarding, and visual refinements.

> See [INSTALLATION.md](INSTALLATION.md) for platform setup guides and tutorials.

## macOS Package Release
- Universal 2 binary packages monoryx-v1.4.0-macos-universal.dmg built for Apple Silicon (M1/M2/M3/M4) and Intel Macs alike.
- Drag-and-drop .dmg contains MONORYX.app with Retina icons, app metadata and /Applications shortcut.
- Portable command-line and Homebrew-friendly monoryx-v1.4.0-macos.tar.gz for CLI-centric users.

## CurseForge Integration + Smart Updates
- Browse both Modrinth and CurseForge directly in Discover tab with dual-source filtering.
- Built-in MONORYX proxy transparently caches CurseForge resources with optional API key auth in Settings.
- Detect existing mods in instance folders via Murmur2 checksums even if copied manually.
- Smart update engine cross-references all installed jars with update sources and offers Update All.
- Forge CDN fallback on failed download attempts for jars and resource packs from both sources.
- Mods installed from CurseForge marked with source badge in the Library.

## Authentication and Onboarding
- Microsoft authentication via Step 2 of the initial onboarding wizard.
- OAuth 2.0 Device Code flow via microsoft.com/link is used for browser-based sign-ins and doesn't require local ports to be open or trigger any firewall prompts.
- Offline profiles can be created with custom username, but will have deterministic offline UUIDs.
- Tokens are automatically refreshed before starting any instance to keep in-memory and disk states in sync.
- Error conditions while refreshing tokens are shown as actionable errors explaining why account isn't working: license not claimed, no java profile name set on the account, or account banned.
- "Replay Onboarding Tour" action in the Settings menu allows restarting the onboarding wizard at any time.

## Companion Updater (monoryx-updater)
- Standalone helper binary hot-swaps the launcher executable after it exits.
- Native process synchronization waits for the main process to exit cleanly, implemented as WaitForSingleObject on Windows and /proc fs reading on Linux.
- Updates are downloaded to background and atomically applied through the companion updater.
- Update banner and Settings > General > Restart to apply update.

## Interface and Visuals
- Procedural vector avatars allow rendering glinting eyes, blushing cheeks, cat ears or any other accessories with no disk assets.
- Procedural 3D isometric block generation for instance thumbnails by mod loader type (Fabric, NeoForge, Forge, Quilt, Vanilla).
- Right-aligned Library context actions keep Remove | Disable | Open Folder in a single column.
- Settings page has columns stretched to equal width for easier visual scanning across tabs.
- Custom memory allocation UI reverts to -Xmx when auto-memory option is selected during onboarding.
- Full UI setup instructions moved to [INSTALLATION.md](INSTALLATION.md).

---

# MONORYX 1.3.1

Faster launches, less memory held for no reason, and several interface fixes.

## Startup speed

- **Faster pre-launch check.** The default `Fast check` verifies that game files exist and
  are the right size, and no longer hashes every jar before each launch. Set
  `Before launching, verify` to `Full check` in Settings to hash everything as before.
  `Repair game files` always does a complete hash check, so a repair is still a real audit.
- **No more asset index parsing at launch.** The multi-megabyte asset index is no longer
  read during the pre-launch check. It is only parsed during a full verification.
- **JVM tuning preset, on by default.** An Aikar-style G1 flag set for smoother play. It is
  only applied to instances that have no arguments of their own, so it can never override a
  choice you made. `AlwaysPreTouch` is deliberately left out, because committing the whole
  heap up front makes startup slower.
- **Startup archive (AppCDS), off by default.** Lets the JVM reuse class files it has already
  loaded. It is offered in Settings but not enabled, because it never demonstrated a win on a
  real Fabric instance: an archive binds to one exact classpath order and to the JVM's module
  flags, so it silently stops applying after a mod or library changes. When it is enabled,
  MONORYX now records the classpath order alongside the mod list, and asks the JVM to log its
  own verdict, so a rejected archive produces a visible message instead of nothing.

## Fixes

- **Eco mode actually works now.** It only applied when the memory field was left at the
  default, so for anyone with a hand-set value the toggle displayed "Eco Mode ON" while the
  game launched with the uncapped number. It is a cap, and now behaves like one. The Home
  page also explains when Eco is what is limiting the number on screen.
- **Stale startup archives are detected.** An archive only matches the exact mod set it was
  recorded against. Changing your mods used to make the JVM quietly fall back with no
  visible sign, so the speed-up just disappeared. Settings now compares a fingerprint of
  your mods and config and offers to re-record.
- **Long error messages are no longer cut off.** Download failures carry a file name, and the
  dialog clipped the tail at the window edge.
- **The startup archive no longer fails silently.** When the JVM cannot archive a classpath,
  the reason is shown in the app instead of only going to a log file.
- **Clicking outside a dialog now closes it** for the screenshot viewer, project images,
  content removal, error dialog, crash dialog and Edit Instance. `Escape` now closes the
  same set, which it previously did not.
- **Backup compression is saved.** The setting applied for the session and reverted on exit.
- **Installer log tails are read once** from the end of the file instead of loading a
  multi-megabyte log in full on every frame of the Logs page.

## Downloads and updates

- **A correct download is no longer rejected over a stale size.** Modrinth metadata and the
  CDN can disagree. The hash is now the deciding check, and a size that differs by a small
  amount is logged rather than turned into a hard "Download failed" that left the mod
  uninstalled. A large discrepancy is still refused, since that means a different file.
- **One failed update no longer cancels the rest.** `Update All` used to stop at the first
  error, so a single bad file left every later mod unapplied. Failures are now collected, the
  successful updates are applied, and the outcome is reported.
- **`Update All` clears its work when it succeeds.** The Library no longer keeps offering an
  update for a version that was just installed.
- **An update whose version id has been superseded falls back** to the newest compatible
  build instead of failing, since the version list can change between checking and installing.

## Interface

- The duplicate Screenshots shortcut was removed from Worlds & Files; the sidebar's
  Screenshots page already covers it.
- Selecting an image in the file browser no longer says "Archive error".

## Memory

- Screenshot thumbnails (up to 96 textures) and Discover thumbnails are released when you
  leave those pages, along with the Mojang changelog index and the Worlds file listing.
- The theme is rebuilt only when it actually changes, rather than every frame.
- The crash report held in memory is truncated to a readable tail.

## Not changed

- **A third JVM preset was not added.** The candidate flag, `TieredStopAtLevel=1`, was
  measured on a real JVM: it cut throughput roughly 10x for no measurable startup gain, so
  it was not worth shipping. The two presets remain `None` and `Aikar`.

# MONORYX 1.3.0

This release introduces a redesigned onboarding experience, full keyboard navigation with a global command palette, enhanced content safety and updater verification, flexible world backup compression, and comprehensive data management during uninstallation.

## Onboarding and navigation

- Redesigned the onboarding wizard into a focused, centered modal with a themed backdrop graphic and a step rail (`Step N of 3`) tracking your progress.
- Added profile preview cards for offline accounts with clean badge placement.
- Added complete keyboard navigation for the wizard: `Enter` to continue, `Escape` to step back.
- Introduced a global Command Palette (`Ctrl+K`) to quickly search and jump between pages, instances, and common launcher actions.
- Added keyboard shortcuts (`Ctrl+1` through `Ctrl+9` for direct page navigation, `F2` to open the screenshot gallery).

## Discover and content safety

- Added one-click loader filter pills (Fabric, Forge, NeoForge, Quilt, Vanilla) directly above the search bar for faster filtering.
- Prevented installing mods into instances whose base game files have not been downloaded, with clear guidance on creating or downloading the instance first.
- Loader filter pills now adapt to the loaders you actually have installed locally.
- Rebuilt the search field into a unified control with an integrated search icon, quick-clear button, and accented focus styling.
- Discover now displays a "CurseForge coming soon" indicator while backend integration is prepared.

## Security and data protection

- Self-updater downloads are now cryptographically validated against published SHA-256 checksums before launching the installer.
- Modpack overrides extraction now enforces safety caps (up to 10,000 files and 4 GiB uncompressed) to protect against decompression bomb archives.
- The Windows uninstaller now prompts before deleting user data, giving clear choice between keeping saves and configs (default) or performing a complete wipe.
- The About tab now displays the app data storage location with details on what is stored there.

## World backups

- Added backup compression modes in Settings: Fast (standard deflate), Maximum (deflate 9), and Smallest (zstd).
- World backups now use a streaming zip writer to handle large worlds efficiently without excessive memory overhead.

## Interface and quality of life

- Added character limits and real-time visual capacity indicators (`used/max`) across all text fields (usernames, instance names, JVM arguments, paths, server addresses, search).
- The sidebar width now scales adaptively with the window size.
- Pages now preserve their scroll positions when switching tabs.
- Progress bars now report transfer speeds, downloaded bytes, throughput, and estimated time remaining.
- Improved contrast between secondary and disabled text tones across all themes.

## Performance and engineering

- Cut executable size in half (from ~41 MB down to ~19 MB) for faster downloads and a lighter desktop footprint.
- Added automated CI workflows covering formatting, strict Clippy checks, and test suites across Windows and Linux.
- Added automated UI flow test coverage for the onboarding sequence and installer integrity verification.

## Community and documentation

- Added an end-to-end installation walkthrough to the README with step-by-step instructions for Windows. Special thanks to @SniffBakaSniff for contributing the guide!

# MONORYX 1.2.2

Another quick hotfix because updating wasn't feeling as seamless as it should be.

## Fixes

- The installer now actually opens itself automatically once the update finishes downloading, so you're not left staring at the screen wondering if it did anything.
- Swapped the main button to "Run installer" once the download finishes, just in case you closed the installer wizard and need to run it again.
- Bumped the update download timeout to 10 minutes so slower connections don't randomly cut off with "error decoding response body" near the end.

# MONORYX 1.2.1

Well... turns out our GPU preference logic was doing the exact opposite of what it was supposed to do. Quick patch so your actual graphics card gets used.

## Fixes

- Fixed Minecraft ignoring your dedicated graphics card and maxing out integrated graphics instead. Turns out Windows DirectX uses `2` for High Performance and `1` for Power Saving, and we had them backwards. So clicking "High performance" was essentially begging Windows to run the game on Intel UHD/integrated graphics. That should actually be fixed now.
- Added a trailing semicolon to the DirectX GPU registry value so Windows doesn't get confused reading it.

# MONORYX 1.2.0

This release makes it easier to get back into a game, manage its files, and see what went wrong when Minecraft closes unexpectedly.

## Home and instances

- Home puts the selected instance beside its latest screenshot. The rest of your instances stay in a compact list below, and selecting one no longer moves the cards around.
- The new instance form walks through the name, Minecraft version, and loader. Version search includes Mojang's changelog when one is available.
- Worlds & Files shows each world's saved name, icon, game mode, version, last-played time, and size. Backups sit beside the selected world; restoring one creates a separate save.
- The screenshot gallery collects images from MONORYX instances. You can open the gallery from Home or the sidebar.
- Library shows the instance you are editing, gives installed content room to breathe, and asks before removing it. Downloads focuses on completed installs and failures instead of every successful file transfer.

## Discover and servers

- Discover uses two columns on wide screens. Project details expand in the results list with a version picker, full rendered description, and image viewer.
- The installer explains when a project has no compatible build. Downloads, image decoding, and metadata requests now have tighter limits so browsing stays responsive.
- Nexeu Servers has a native dashboard for server status, resource use, console output, power controls, and backups.

## Discord and game performance

- Discord activity is on by default for new settings. It shows the current instance and, when detected, the active world or the name saved in Minecraft's server list. Privacy switches and elapsed time remain in Settings.
- Discord profiles include fixed links to MONORYX and DemonZ Development. Activity now updates while the launcher is hidden, and a renderer reload no longer looks like a disconnect.
- On Windows, Minecraft's generic third-party server title uses the saved multiplayer server name when the address matches your list.
- Eco mode is now a lower-memory preset without extra JVM tuning flags. It is off by default for new settings because a lower memory limit can reduce FPS in demanding games. Existing instance choices remain in place.

## Interface and fixes

- Buttons have consistent sizing and stronger contrast across themes. Monochrome, Gloss, Soft pink, and Soft brown now cover more of the interface.
- Hover and page transitions are smoother. The instance loader picker animates its selection, and update checks and downloads show activity and progress in the app.
- Crash reports lead with a plain explanation and useful next steps; the full log is still available below.
- Windows startup no longer mistakes an older installed launcher for this release. The default window size is 1280×720, and the sidebar and dialogs fit smaller screens better.
- Update checks include eligible beta releases. On Windows, MONORYX downloads the matching installer and lets you choose when to run it.

# MONORYX 1.1.0 beta

## Loaders and content

- Fabric metadata handling accepts the API's flat and nested responses and picks a stable loader release.
- Forge and NeoForge version lists are parsed and sorted more reliably, including older version suffixes. Installer dependency checks catch more broken installs before launch.
- Discover filters mods by the selected instance's loader. Temporary metadata and server errors are retried.

## Launcher

- The sidebar, instance cards, Settings, and update controls received a simpler layout. Idle rendering and texture cleanup were reduced to use fewer resources.
- Window positioning was fixed for scaled Windows displays. Maximized and windowed startup, single-instance restore, and running installer updates work more reliably.
- Nexeu Servers gained power controls, streaming logs, and remote backup creation.
- Crash diagnostics gained checks for Java setup, JVM arguments, memory limits, and graphics driver problems.
