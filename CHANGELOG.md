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
