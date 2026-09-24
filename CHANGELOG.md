# MONORYX v1.1.0 beta

This update mainly focuses on improving loader handling, cleaning up the interface, reducing resource usage, and fixing a number of window and process related issues.

## Loader and Metadata Changes

* Improved Fabric metadata parsing to support both flat and nested version structures.

* Fabric loader selection now prefers the latest stable release and ignores intermediary or unstable builds where appropriate.

* Reworked Forge and NeoForge metadata fetching with more reliable Maven metadata parsing.

* Added retry handling for temporary server and metadata errors.

* Forge and NeoForge versions are now sorted more consistently while preserving older version suffixes.

* Added additional dependency validation during Forge installation to reduce failed installations.

* Mod search and discovery results are now filtered based on the loader used by the selected instance. This currently supports Fabric, Forge, NeoForge, Quilt, and Vanilla instances.

## Interface Changes

* Simplified several parts of the interface and removed unnecessary borders, duplicate badges, and other visual elements.

* Cleaned up instance cards and sidebar navigation.

* Improved page transitions and general interface responsiveness.

* Reduced unnecessary redraws while the launcher is idle.

* Settings changes, update checks, and background operations now provide immediate notifications.

* Reorganized settings into Launcher, Minecraft, Java and GPU, and About sections.

* Updated the Check for Updates control with clearer loading and status feedback.

## Window Management

* Fixed an issue where the launcher could open partially outside the visible screen area when Windows display scaling was enabled.

* Improved startup window positioning on high DPI displays.

* Start Maximized now uses native Windows window handling for more reliable behaviour.

* Windowed mode now attempts to center the launcher on the primary display.

* Added controls for maximizing the current window, applying a configured window size, and centering the window.

## Memory and Resource Usage

* Reduced launcher memory usage while idle.

* Improved thumbnail and texture cache cleanup.

* Image data that is no longer required can now be released when leaving certain pages.

* Reduced unnecessary interface rendering while the launcher is inactive or Minecraft is running.

* Adjusted background polling behaviour to lower idle CPU and GPU usage.

## Nexeu Server Management

* Added a Nexeu Servers section to the launcher.

* Servers can now be started, restarted, stopped, and terminated directly from the launcher.

* Added a terminal view for streaming server logs.

* Added support for starting and monitoring remote server backups.

## Diagnostics and Crash Handling

* Moved Copy Report and Share on mclo.gs actions to the top of the crash report window.

* Improved detection of several common launch and crash problems.

* Diagnostics can now identify issues involving missing Java installations, invalid JVM arguments, graphics driver conflicts, and memory limits.

## Installer and Process Handling

* Updated the Inno Setup configuration so launcher updates can handle an already running Monoryx process more reliably.

* Improved single instance handling.

* Opening Monoryx while another instance is already running now restores and focuses the existing launcher instead of creating another background process.

## Codebase and Maintenance

* All 153 unit tests are currently passing.

* The project passes Clippy with warnings treated as errors.

* Formatting has been checked across the codebase.
