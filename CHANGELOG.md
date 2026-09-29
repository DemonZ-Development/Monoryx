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
