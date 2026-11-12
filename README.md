<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**A native Linux mod manager that never touches your game folder.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#installation)

Eidos gives Bethesda games on Linux what Mod Organizer 2 gives them on Windows:
a merged view of your mods, built for each launch. It uses Linux mount
namespaces and FUSE instead of Windows API hooking, and the manager runs without
Wine. Nothing is copied into the game folder, so there is nothing to clean up
afterwards. Eidos also has definitions for Stellar Blade, Stardew Valley, 7 Days
to Die and Crimson Desert.

Steam copies run Eidos from the game's launch option. GOG and Epic copies in
Heroic or Legendary run it as a wrapper. The game always starts through the
store that owns the copy; Eidos never starts it with Proton itself.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Status:** Skyrim SE is played through Eidos daily, with SKSE,
> script-extender preloaders, Creation Club, LOOT-sorted load orders and
> per-profile saves. One game family is proven in real play so far. Ten more
> Bethesda games and four others are wired and waiting for testers.

## Why Eidos

The merged view is mounted in a private mount namespace that only the game's
process tree can see. Other programs, a second game included, never see it and
need no permission for it. When the game exits or is killed, or the machine
loses power, the namespace goes away with its processes and the install is as it
was.

Each profile owns its mod list, plugin order, INIs and saves. At launch the
plugin files and the save folder are bind-mounted over the game's own paths, so
what the game writes there lands in the profile. Switching profiles switches all
of it.

Eidos needs no setuid helper, no system daemon, no `sudo setcap` and no
`/etc/fuse.conf` edit. The FUSE daemon runs as your user for one launch and
exits with the game. You install one program and set one Steam launch option.

Before each run Eidos keeps a snapshot of the plugin list. If a crash damages
the list, Diagnostics compares it with that snapshot, and Restore the
pre-session order puts the old order back. A capture that would wipe your load
order is refused, with the reason.

Test run starts the game with the profile's mods, without mods, or with only the
selected mods that are switched on, plus their masters. The saves, INI changes,
plugin state, Overwrite output and other My Games files of that run, logs
included, go to `<instance>/test-run/`. When the game exits, Eidos lists those
five groups and you keep or throw away each one. Other writes, such as the Wine
registry, go where they always go. Find the mod behind a problem, in
Diagnostics, runs test runs with half of the suspect mods at a time plus the
masters they need. After each run you answer whether the problem happened, and
Eidos narrows the suspects until it names the mod, or the mods that need each
other's masters; 200 suspects take about 8 runs. Switched-on mods selected in
the list before you start stay on in every step. Details in
[Test runs](docs/guide/usage.md#test-runs) and
[Finding the mod behind a problem](docs/guide/usage.md#finding-the-mod-behind-a-problem).

### How it compares

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / link deployers |
|---|---|---|---|---|
| Manager runs natively | yes | no, a Windows app in Wine | yes (Qt port) | yes |
| Game dir untouched | yes, always | yes | yes | no, links are written into it |
| Mount visible to | the game only | the game only | the whole system | n/a |
| Crash cleanup needed | none | none | stale-mount recovery | manual un-deploy |
| Root mods (ENB, preloaders) | yes, natively | plugin required | plugin required | partial |
| Privileges required | none | none | `/etc/fuse.conf` edit | none |

### How fast it is

| | before | now |
|---|---|---|
| loading a save | ~20 seconds | 6-7 seconds |
| directory scans in one session | 5.6 million | 335 thousand |

Cell changes are immediate. Finding one file used to ask each of fifty mods in
turn, and listing one folder did that fifty times over. The read-only layers are
now indexed once at mount, so finding a file is one hash lookup, and a folder's
merged listing is also computed once at mount. The figures come from a real
instance played normally, not from a benchmark:
[internals/performance.md](docs/internals/performance.md).

### Prior art and thanks

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) and
  [usvfs](https://github.com/ModOrganizer2/usvfs): the semantics Eidos
  reproduces, and the codebase its parity was studied against
- [LOOT](https://loot.github.io/): the sorting engine, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) and the other Linux managers, which
  show there is a community that wants this solved

## What it does

### Mods

Eidos installs simple archives, FOMOD wizards, Wrye Bash BAIN packages, OMOD
containers with supported OBMM scripts, and anything else through a manual
picker. Root mods (script extender preloaders, ENB, Engine Fixes) work natively,
with no Root Builder plugin and nothing copied into your install. You can hide
single files, group mods with separators, move a selection to a target, keep
notes and categories per mod, and import an MO2 profile.

The list works like MO2's: eight optional columns and a sort on any of them,
grouping by category or by source, double-click gestures, type-to-jump, per-mod
backups that do nothing until you restore them, and flags for a mod whose layout
this game will not load or that was downloaded for another game. Filters pick
mods by Nexus state (endorsed, tracked, update available), notes, hidden files
and content type (plugins, archives, textures, meshes, scripts, sound and more).
The file tree creates folders, renames, deletes and opens files, and previews
images and text without launching anything.

Ctrl+Z and Ctrl+Shift+Z undo and redo changes to the mod list and the load
order. Disabling or removing a mod that one of the profile's three newest saves
uses asks first and names the plugins (Skyrim and Fallout 4 saves). Reinstalling
or updating a FOMOD mod opens the wizard on the answers of its last install.
A mod's information lists the mods it requires and the mods that require it,
read from the masters of the enabled plugins.

### Plugins

The load order has LOOT sorting built in, mod indexes as the game computes them,
missing-master warnings, and your DLC and Creation Club content shown as
unmanaged rows. The header counts the slots used in each index space (full,
medium, light). Hovering a plugin shows its author, description and header
version. On the Skyrim engines, a plugin whose same-named INI the game loads is
marked.

Plugins that LOOT flags as dirty are cleaned by xEdit's QuickAutoClean into a
`Cleaned plugins` mod, never in place. Flag as light, into a new mod, writes a
light copy of a plugin, and refuses when one of the newest saves loads that
plugin as a full plugin (Skyrim and Fallout 4 saves).

### Instances

An instance is global, kept under `~/.local/share/Colony/Eidos/instances/`, or
portable: a self-contained folder anywhere you want (a second drive, a games
partition), movable and isolated like MO2's. Portable instances are remembered
across sessions. The GUI, the Steam launch and every CLI command follow the one
you last used, and any command takes the folder wherever it takes a game id.
Details in [usage.md](docs/guide/usage.md#instances-global-and-portable).

An instance can also run the game from a copy of the game folder inside the
instance. When Eidos first opens a Wabbajack list, it runs the list's own copy:
the folder its `ModOrganizer.ini` names, else a `Stock Game` or `Game Root`
folder. Freeze this version, on the Game files card in Diagnostics, copies the
store install into `<instance>/Stock Game`, so a store update no longer changes
the files the game runs. When both folders are on one filesystem that can share
blocks, such as btrfs or XFS, the copy shares them with the store install
(reflink). On ext4, or across two filesystems, it is a full copy, and the size
and free space are shown first. The store still starts the game. Return to the
store version and Delete the copy undo it, and
`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
does the same in a terminal. Morrowind, and Oblivion with
`bUseMyGamesDirectory=0`, cannot run from a copy. Details in
[usage.md](docs/guide/usage.md#game-files-the-store-install-or-a-copy).

### Profiles

Each profile has its own mod order, plugin state, INIs and saves. Saves are
parsed and compared with your current plugins, with a button that enables what
a save needs, and copied back to the prefix for Steam Cloud after every session.
Saves that Steam Cloud brought from another device, or that a launch without
Eidos left in the prefix, are listed on the Saves tab. Import copies them into
the profile, and a save whose name the profile already uses comes in under a
new `orphan-` name instead of replacing it.

A profile can carry launch variables and wrappers (`env=NAME=value` and
`wrapper=command`, one per line, in the INI editor) for its game and tool runs.
For Skyrim SE, Enderal SE and Fallout 4, a Display card in Diagnostics sets the
window size and mode in the profile's INI copies. Backups keeps restore points
of the profile's INIs, and Eidos takes one before each BethINI Pie run. A
profile's desktop shortcut starts the game through its store. `eidos sessions`
lists the recorded runs and compares one with the current state.

### Nexus

Connect an account to receive Mod Manager Download links, check for updates and
see authors and their profiles. Each mod is checked, endorsed and linked on the
Nexus domain its file came from, so a Skyrim SE mod in an Enderal SE instance
stays on the Skyrim SE site. An update follows the successor chain the author
set on Nexus, replaces the mod's folder with the new file (a backup of the old
one is offered), keeps its priority and enabled state, and replays the FOMOD
answers stored at install. `eidos nexus update <instance> --apply` does the same
in a terminal on a premium account, and prints the file pages on a free one.
Track and Endorse are in the mod menu, and every update check syncs them with
your Nexus account. A mod's information has a Nexus page tab with its
description and changelog as plain text, and its General tab ends with what the
last update changed: the files added, removed and resized, and the masters added
or removed. Install downloads when they finish (Preferences, Nexus, off by
default) installs a download as soon as it finishes while the window is open.

Collections install from their recipes with resumable progress and explicit
reports for unsupported steps. Matching checks exact source files; incomplete
older metadata stays unverified. The collection pane browses and searches the
collections published for a game, shows when an installed collection has a newer
revision, and walks a free account through the missing files one page at a
time. When a drag breaks a collection's order rules, Diagnostics says so, and
Restore collection order puts its mods back. The Downloads tab filters, sorts,
hides and removes archives. Offline mode (Preferences, Nexus) stops Nexus access
and the daily release check.

### Tools

xEdit, BodySlide, DynDOLOD and other modding tools run through the merged view
inside the game's Wine prefix (Steam's Proton prefix, or the one Heroic set up).
They see your mods, their output lands in Overwrite, and one click turns that
output into a real mod. For Skyrim SE, Eidos finds SSEEdit, SSEEdit
QuickAutoClean, PGPatcher, Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen,
xLODGen, Synthesis and BethINI Pie in the game folder, inside a mod, or in the
tools folder you keep beside your games, each with the runtimes it needs. The
other games have their own list. Pandora, Nemesis, FNIS, BodySlide and Synthesis
write their output into a named mod (`Nemesis Output` and so on).

The release bundle ships Microsoft's Visual C++ 2015-2022 and .NET 8 Desktop
Runtime installers and the .NET 10 runtime. Eidos installs `vcrun2022` and
.NET 8 with those installers, without winetricks, and without the bundle it
downloads the same files from Microsoft, each checked against the SHA-256 built
into Eidos. Synthesis gets the .NET 10 SDK, a 300 MB download that the bundle
does not ship. The other prerequisites (.NET Framework 4.8, XACT and older .NET
versions) still go through winetricks. Nothing is downloaded until you ask to
install a tool's prerequisites.

Pin the tools you use, hide the ones you do not, give a tool its own Steam AppID
when it is its own Steam app, and write a `.desktop` shortcut that launches it
through the merged view without opening Eidos. `eidos tool <game> import-mo2`
lists the executables in the `ModOrganizer.ini` of a list's folder, and
`--write` adds them. An exe in the Data tab can be run once or added as a tool,
and `eidos tool <instance> run --exe <path>` runs one from a terminal. With Ask
before playing, Run asks first when a generator's output is out of date.

Stop ends a running game or tool. The File menu runs winecfg, regedit, the Wine
file explorer and the Windows uninstaller in the game's prefix. Rebuild prefix,
in Diagnostics, renames the prefix so Steam makes a new one, and deletes
nothing. `eidos shell <instance>` opens a shell in the merged view, or runs the
command given after `--`, and what it writes lands in Overwrite.

While the window is in the background, Eidos can send a desktop notification
when a tool run, a collection install, a pack or unpack, or the last running
download ends, and when a LOOT sort fails. Notify the desktop when a long job
ends, in Preferences, turns this on; it is off by default.

### Diagnostics

The Diagnostics tab lists missing masters, orphaned archives, mod-list drift and
damaged plugin sets, and after a run, what the script extender's own log says
loaded. It also reports:

- a script extender that is missing or does not match the game version
- a game that changed since the last clean launch
- a Steam launch option that does not start Eidos or has no `%command%` after it
- a Proton older than the one that last set up the prefix, or one from the
  Flatpak Steam
- `PROTON_ENABLE_NVAPI` missing for DLSS, and exclusive fullscreen, which
  stops Community Shaders' frame generation
- an `enblocal.ini` without `LinuxVersion=true`
- the crash log of the last session and the mods it names
- Nexus requirements that are missing or disabled
- an `nxm://` handler that is another program or a missing file
- `sTestFile` lines, which make the game ignore `plugins.txt`
- enabled `.esp` plugins in Starfield, and invalid or unpaired blueprint plugins
- a patch whose loose files lose to the mod it patches
- a mod whose copies of the script extender's Papyrus scripts win over the
  extender's own
- a low `vm.max_map_count` or open-files limit
- game files on a Windows file system
- BodySlide data paths that are not the game's Data folder, and folders in
  `Overwrite/Root` that belong in Data
- a list's `ModOrganizer.ini` that points tools at another game folder
- a game copy that is missing or has no game executable

`eidos check <instance> [--json]` runs the checks that need no window and exits
with 1 when it finds a problem. Copy report copies a report with your home
folder written as `~` and your user name as `<user>`, and Save tree on the Data
tab writes the merged Data tree with the source of each file, redacted the same
way.

See [Integrity checks and generated output](docs/internals/integrity.md) for
replacement protection, BSA/BA2 member conflicts, static SKSE diagnostics and
generator receipts, including their limits.

### Where it keeps its own files

Eidos uses the layout every program in the Colony family uses:
`~/.config/Colony/Eidos/` for what you chose (preferences, your Nexus session,
your instance list, the game and add-on definitions you wrote),
`~/.local/share/Colony/Eidos/` for global instances, tool runtimes (downloaded,
or copied from the bundle by `install.sh`) and session logs, and
`~/.cache/Colony/Eidos/` for LOOT masterlists. Inside an instance,
`Stock Game` holds a frozen copy of the game and `test-run/` holds a test run
waiting for review.

An older Eidos kept its files in `~/.config/eidos/`, `~/.local/share/eidos/` and
`~/.local/state/`. The first launch after upgrading carries them across, says so
in the log, and leaves the old directories as they were. Until a move has
completed, Eidos keeps using the old files. Details in
[internals/paths.md](docs/internals/paths.md).

### Supported games

Skyrim SE/AE is proven in real play. Fallout 4 is wired end to end too (F4SE
swapped in automatically, archive invalidation, asterisk load order, LOOT, `.fos`
saves); see [guide/fallout4.md](docs/guide/fallout4.md). Wired and looking for
testers: Skyrim LE, Skyrim VR, Enderal SE, Fallout 4 VR, Fallout 3, Fallout NV,
Starfield, Oblivion and Morrowind. Fallout 3, Fallout NV, Oblivion and Morrowind
order plugins by the files' dates: Eidos keeps one order per profile and shows
those dates inside the mount, without changing the files on disk.

Four other games are wired and also looking for testers: Stellar Blade (Unreal
`.pak` mods in `~mods`), Stardew Valley (SMAPI mods, with SMAPI installed
separately), 7 Days to Die (`Mods/` folders beside the game) and Crimson Desert.
Crimson Desert takes code mods only: ASI plugins, ReShade and loader DLLs in
`bin64`. Its data mods, which live in PAZ groups, are not supported.

A game is one TOML file, and yours can go in `~/.config/Colony/Eidos/games/`
without a rebuild: [internals/adding-games.md](docs/internals/adding-games.md).

## Installation

### Via Colony (recommended)

Search for **Eidos** in [Colony](https://github.com/Project-Colony/Colony) and
install it. Updates arrive through the launcher. Colony installs one file,
`eidos-linux`: the window, and also the `eidos` command line when it is called
through a link named `eidos`. That file carries neither the NIF model preview
helper nor the Microsoft runtimes of the bundle below. Without the helper the
preview says so and everything else works, and Eidos downloads a runtime from
Microsoft when you install a tool's prerequisites. What to do after installing,
and how to point Steam at it:
[docs/guide/install.md](docs/guide/install.md#via-colony).

### Direct binary download

Grab an asset from the [latest release](../../releases/latest):

| Asset | What it is |
|---|---|
| `eidos-linux` | the whole program in one file, as Colony installs it |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, the NIF preview helper, the Visual C++ 2015-2022 and .NET 8 Desktop Runtime installers and the .NET 10 runtime, icons, license notices and an installer that registers the desktop entry and the Nexus download handler |

Check the download first, as [Code signing policy](#code-signing-policy) shows,
then:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

The installer puts the binaries in `~/.local/bin` without sudo and copies the
bundle's `runtimes/` to `~/.local/share/Colony/Eidos/runtimes`. `--system` and
`--bindir DIR` put the binaries elsewhere, and `--from DIR` takes them from
another folder. `--cap` asks sudo for `CAP_SYS_ADMIN`, which only the
experimental FUSE passthrough needs, and passthrough stays off; `--no-cap` is the
default. Both assets need x86_64 Linux with glibc 2.39 or newer, FUSE
(`fusermount3`) and 7-Zip for mod archives.

Eidos does not update itself. Once a day at start, the window asks GitHub for
the latest release, and when a newer one is out it shows a link to its release
notes after Help in the menu bar. Preferences turns this off, and a copy under
`/usr` outside `/usr/local`, which a package manager updates, never asks.

### Build from source

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Requires Rust 1.92 or newer (the `rust-version` CI checks; CI and releases
build with 1.99.1), plus `just`, CMake 3.20+ and a C++17 compiler for the NIF
preview helper. `just build` uses the release profile: one codegen unit and fat
LTO. A cold build of `eidos` and `eidos-gui` with 3 build jobs took 334 seconds
that way and 243 seconds with thin LTO, and fat LTO makes `eidos` 1.2 MB and
`eidos-gui` 3.8 MB smaller (measurements in `Cargo.toml`).
`just profile=dev build` compiles faster. An Arch package builds from
`packaging/PKGBUILD`. Prerequisites and the commands without `just`:
[docs/guide/install.md](docs/guide/install.md).

### Steam launch options

Eidos runs as the game's launch command, which is how it mounts before the game
starts. The base line is all most setups need, with the absolute path of your
install (Steam does not read your `PATH`):

```
~/.local/bin/eidos-gui %command%
```

From Colony that path is `~/.local/share/Colony/apps/Eidos/eidos-linux`. Then
press Play: Eidos opens on that game's instance, and Run starts the game through
the merged view.

In a window you opened yourself, Run asks Steam to start the game
(`steam -applaunch <appid>`), so the launch option still applies, and you press
Run in the window Steam opens. Heroic gets a `heroic://launch` link, and
Legendary runs `legendary launch <app>`, once Eidos is set as the game's wrapper
there. When the store's setting is missing, Run starts nothing and says what to
set: [guide/usage.md](docs/guide/usage.md#launching-the-game). With Start the
game at once when a store opens Eidos (Preferences, off by default), Eidos runs
the Diagnostics checks and starts the game without showing its window when
nothing needs attention.

Everything else is environment variables stacked in front of it, and they
combine freely:

| You want... | Put in front |
|---|---|
| DLSS with Community Shaders | `PROTON_ENABLE_NVAPI=1`. Without it DLSS never initialises and says nothing; the full checklist is [guide/graphics.md](docs/guide/graphics.md) |
| an FPS counter on screen | `DXVK_HUD=fps` |
| driver-level frame interpolation, zero mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1`, never together with Community Shaders' own frame generation |
| verbose logs for a bug report | `EIDOS_LOG=debug` (session logs land in `~/.local/share/Colony/Eidos/logs/`) |
| a per-session I/O report from the mount | `EIDOS_FUSE_STATS=1` |
| a different FUSE worker count | `EIDOS_FUSE_THREADS=8` (default 4; `1` is the first thing to try when chasing a concurrency bug) |
| this launch pinned to one portable instance | `EIDOS_INSTANCE=/path/to/folder`. Without it Eidos opens the instance you last used |
| the window for one launch while Start the game at once is on | `EIDOS_WINDOW=1` |

The line to keep for a modern modded setup (Community Shaders, DLSS, frame
generation) is the final command, not an example:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Add `DXVK_HUD=fps` in front while verifying the setup works, and drop it once it
does.

The same variables can go in the profile's launch variables instead of the Steam
line, which keeps them per profile and applies them to tool runs too:
[guide/usage.md](docs/guide/usage.md#variables-and-wrappers-per-profile).

The deeper diagnostic switches (`EIDOS_FUSE_TRACE`, the cache and index
bisection toggles, why `EIDOS_FUSE_PASSTHROUGH` is off by default) are in
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Documentation

Everything is in [docs/](docs/), indexed by who is reading at
[docs/README.md](docs/README.md): the user guide (installation, the CLI and the
GUI, tools such as xEdit and DynDOLOD, graphics, troubleshooting), how Eidos
works inside, and why it exists. Security policy and how to report a
vulnerability: [SECURITY.md](SECURITY.md).

## Language

The pages a player needs are translated. English is the reference: when a
translation disagrees with it, the English file is right.

- **Français** - [README](docs/i18n/fr/README.md) · [index](docs/i18n/fr/docs/README.md) · [install](docs/i18n/fr/docs/guide/install.md) · [usage](docs/i18n/fr/docs/guide/usage.md) · [tools](docs/i18n/fr/docs/guide/tools.md) · [fallout4](docs/i18n/fr/docs/guide/fallout4.md) · [graphics](docs/i18n/fr/docs/guide/graphics.md) · [troubleshooting](docs/i18n/fr/docs/guide/troubleshooting.md) · [extensions](docs/i18n/fr/docs/guide/extensions.md)
- **Русский** - [README](docs/i18n/ru/README.md) · [index](docs/i18n/ru/docs/README.md) · [install](docs/i18n/ru/docs/guide/install.md) · [usage](docs/i18n/ru/docs/guide/usage.md) · [tools](docs/i18n/ru/docs/guide/tools.md) · [fallout4](docs/i18n/ru/docs/guide/fallout4.md) · [graphics](docs/i18n/ru/docs/guide/graphics.md) · [troubleshooting](docs/i18n/ru/docs/guide/troubleshooting.md) · [extensions](docs/i18n/ru/docs/guide/extensions.md)
- **Deutsch** - [README](docs/i18n/de/README.md) · [index](docs/i18n/de/docs/README.md) · [install](docs/i18n/de/docs/guide/install.md) · [usage](docs/i18n/de/docs/guide/usage.md) · [tools](docs/i18n/de/docs/guide/tools.md) · [fallout4](docs/i18n/de/docs/guide/fallout4.md) · [graphics](docs/i18n/de/docs/guide/graphics.md) · [troubleshooting](docs/i18n/de/docs/guide/troubleshooting.md) · [extensions](docs/i18n/de/docs/guide/extensions.md)
- **Español** - [README](docs/i18n/es/README.md) · [index](docs/i18n/es/docs/README.md) · [install](docs/i18n/es/docs/guide/install.md) · [usage](docs/i18n/es/docs/guide/usage.md) · [tools](docs/i18n/es/docs/guide/tools.md) · [fallout4](docs/i18n/es/docs/guide/fallout4.md) · [graphics](docs/i18n/es/docs/guide/graphics.md) · [troubleshooting](docs/i18n/es/docs/guide/troubleshooting.md) · [extensions](docs/i18n/es/docs/guide/extensions.md)
- **Português (BR)** - [README](docs/i18n/pt-BR/README.md) · [index](docs/i18n/pt-BR/docs/README.md) · [install](docs/i18n/pt-BR/docs/guide/install.md) · [usage](docs/i18n/pt-BR/docs/guide/usage.md) · [tools](docs/i18n/pt-BR/docs/guide/tools.md) · [fallout4](docs/i18n/pt-BR/docs/guide/fallout4.md) · [graphics](docs/i18n/pt-BR/docs/guide/graphics.md) · [troubleshooting](docs/i18n/pt-BR/docs/guide/troubleshooting.md) · [extensions](docs/i18n/pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](docs/i18n/zh-CN/README.md) · [index](docs/i18n/zh-CN/docs/README.md) · [install](docs/i18n/zh-CN/docs/guide/install.md) · [usage](docs/i18n/zh-CN/docs/guide/usage.md) · [tools](docs/i18n/zh-CN/docs/guide/tools.md) · [fallout4](docs/i18n/zh-CN/docs/guide/fallout4.md) · [graphics](docs/i18n/zh-CN/docs/guide/graphics.md) · [troubleshooting](docs/i18n/zh-CN/docs/guide/troubleshooting.md) · [extensions](docs/i18n/zh-CN/docs/guide/extensions.md)
- **Polski** - [README](docs/i18n/pl/README.md) · [index](docs/i18n/pl/docs/README.md) · [install](docs/i18n/pl/docs/guide/install.md) · [usage](docs/i18n/pl/docs/guide/usage.md) · [tools](docs/i18n/pl/docs/guide/tools.md) · [fallout4](docs/i18n/pl/docs/guide/fallout4.md) · [graphics](docs/i18n/pl/docs/guide/graphics.md) · [troubleshooting](docs/i18n/pl/docs/guide/troubleshooting.md) · [extensions](docs/i18n/pl/docs/guide/extensions.md)
- **Italiano** - [README](docs/i18n/it/README.md) · [index](docs/i18n/it/docs/README.md) · [install](docs/i18n/it/docs/guide/install.md) · [usage](docs/i18n/it/docs/guide/usage.md) · [tools](docs/i18n/it/docs/guide/tools.md) · [fallout4](docs/i18n/it/docs/guide/fallout4.md) · [graphics](docs/i18n/it/docs/guide/graphics.md) · [troubleshooting](docs/i18n/it/docs/guide/troubleshooting.md) · [extensions](docs/i18n/it/docs/guide/extensions.md)
- **Українська** - [README](docs/i18n/uk/README.md) · [index](docs/i18n/uk/docs/README.md) · [install](docs/i18n/uk/docs/guide/install.md) · [usage](docs/i18n/uk/docs/guide/usage.md) · [tools](docs/i18n/uk/docs/guide/tools.md) · [fallout4](docs/i18n/uk/docs/guide/fallout4.md) · [graphics](docs/i18n/uk/docs/guide/graphics.md) · [troubleshooting](docs/i18n/uk/docs/guide/troubleshooting.md) · [extensions](docs/i18n/uk/docs/guide/extensions.md)
- **日本語** - [README](docs/i18n/ja/README.md) · [index](docs/i18n/ja/docs/README.md) · [install](docs/i18n/ja/docs/guide/install.md) · [usage](docs/i18n/ja/docs/guide/usage.md) · [tools](docs/i18n/ja/docs/guide/tools.md) · [fallout4](docs/i18n/ja/docs/guide/fallout4.md) · [graphics](docs/i18n/ja/docs/guide/graphics.md) · [troubleshooting](docs/i18n/ja/docs/guide/troubleshooting.md) · [extensions](docs/i18n/ja/docs/guide/extensions.md)
- **繁體中文** - [README](docs/i18n/zh-TW/README.md) · [index](docs/i18n/zh-TW/docs/README.md) · [install](docs/i18n/zh-TW/docs/guide/install.md) · [usage](docs/i18n/zh-TW/docs/guide/usage.md) · [tools](docs/i18n/zh-TW/docs/guide/tools.md) · [fallout4](docs/i18n/zh-TW/docs/guide/fallout4.md) · [graphics](docs/i18n/zh-TW/docs/guide/graphics.md) · [troubleshooting](docs/i18n/zh-TW/docs/guide/troubleshooting.md) · [extensions](docs/i18n/zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](docs/i18n/cs/README.md) · [index](docs/i18n/cs/docs/README.md) · [install](docs/i18n/cs/docs/guide/install.md) · [usage](docs/i18n/cs/docs/guide/usage.md) · [tools](docs/i18n/cs/docs/guide/tools.md) · [fallout4](docs/i18n/cs/docs/guide/fallout4.md) · [graphics](docs/i18n/cs/docs/guide/graphics.md) · [troubleshooting](docs/i18n/cs/docs/guide/troubleshooting.md) · [extensions](docs/i18n/cs/docs/guide/extensions.md)
- **한국어** - [README](docs/i18n/ko/README.md) · [index](docs/i18n/ko/docs/README.md) · [install](docs/i18n/ko/docs/guide/install.md) · [usage](docs/i18n/ko/docs/guide/usage.md) · [tools](docs/i18n/ko/docs/guide/tools.md) · [fallout4](docs/i18n/ko/docs/guide/fallout4.md) · [graphics](docs/i18n/ko/docs/guide/graphics.md) · [troubleshooting](docs/i18n/ko/docs/guide/troubleshooting.md) · [extensions](docs/i18n/ko/docs/guide/extensions.md)
- **Türkçe** - [README](docs/i18n/tr/README.md) · [index](docs/i18n/tr/docs/README.md) · [install](docs/i18n/tr/docs/guide/install.md) · [usage](docs/i18n/tr/docs/guide/usage.md) · [tools](docs/i18n/tr/docs/guide/tools.md) · [fallout4](docs/i18n/tr/docs/guide/fallout4.md) · [graphics](docs/i18n/tr/docs/guide/graphics.md) · [troubleshooting](docs/i18n/tr/docs/guide/troubleshooting.md) · [extensions](docs/i18n/tr/docs/guide/extensions.md)
- **Nederlands** - [README](docs/i18n/nl/README.md) · [index](docs/i18n/nl/docs/README.md) · [install](docs/i18n/nl/docs/guide/install.md) · [usage](docs/i18n/nl/docs/guide/usage.md) · [tools](docs/i18n/nl/docs/guide/tools.md) · [fallout4](docs/i18n/nl/docs/guide/fallout4.md) · [graphics](docs/i18n/nl/docs/guide/graphics.md) · [troubleshooting](docs/i18n/nl/docs/guide/troubleshooting.md) · [extensions](docs/i18n/nl/docs/guide/extensions.md)

Everything else is English. `docs/internals/` and `docs/project/` are read by
people who are also reading the Rust, `CHANGELOG.md` is generated, and
`SECURITY.md` is not translated either. Translating the first two would add
more than 28,000 words to keep in sync for readers who do not need them.

Each translation carries the hash of the English file it was made from, and CI
fails while that hash no longer matches the English; see
[`scripts/i18n-check.sh`](scripts/i18n-check.sh). The translation is then
updated and restamped with `scripts/i18n-check.sh --fix`. An out-of-date
translation is updated, not removed.

A language is one directory. `docs/i18n/<lang>/` mirrors the repo root, so
`docs/i18n/de/docs/guide/install.md` is the German `docs/guide/install.md`. That
is why a link between two translated pages is the same string as the link
between their English originals, and why removing a language is one `rm -r`.

## Code signing policy

Every release asset is signed by the Project Colony organisation in CI, never on
a developer machine. Next to each asset on the release page:

| File | What it is |
|---|---|
| `<asset>.sig` | an ed25519 signature over the asset, made with the organisation's release key |
| `<asset>.meta` | three lines binding the asset to its file name, its sha256 and the release version |
| `<asset>.meta.sig` | an ed25519 signature over the `.meta` |

The private key is an organisation secret, used only by the shared
[sign-and-publish workflow](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
in a job that builds nothing; the jobs that compile Eidos never see it.
Colony checks all three files before it installs or updates Eidos, and
refuses a release older than the one installed. To check a download yourself
with OpenSSL 3 (the same commands work for every asset):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

How releases are built, signed and published:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

## Privacy

Eidos sends no telemetry, no analytics and no crash reports. It connects only to
the servers in this table, and only for the action named there. Offline mode
(Preferences, Nexus) stops every connection to Nexus and the daily release
check.

| Data | Stored or sent | Where, and why |
|---|---|---|
| Preferences, instance list, game and add-on definitions | stored | `~/.config/Colony/Eidos/` |
| Instances, tool runtimes, session logs | stored | `~/.local/share/Colony/Eidos/`, or the folder you chose for a portable instance |
| LOOT masterlists | stored | `~/.cache/Colony/Eidos/loot/` |
| Release check | sent | At start, at most once a day, one request to `api.github.com/repos/Project-Colony/Eidos/releases/latest`. Eidos reads the release tag only and downloads nothing. It is on by default and off in Preferences, and it is skipped in Offline mode and for a copy installed under `/usr` outside `/usr/local`, which a package manager updates. [GitHub privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Nexus sign-in | sent | Signing in opens `users.nexusmods.com` in your browser, which hands the result back to a listener Eidos opens on `127.0.0.1` for that sign-in only. Eidos then trades it for tokens at `users.nexusmods.com/oauth/token` and fetches the public keys Nexus signs them with. [Nexus Mods privacy policy](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus credentials | stored | The access and refresh tokens, in `~/.config/Colony/Eidos/nexus.ini` with mode 0600. An `api_key=` line written there by an older Eidos is kept but never read; delete it if you like. |
| Nexus requests | sent | `api.nexusmods.com`, when you use a Nexus feature: your account name and premium status, mod and file details, update checks, collections and download links. Your endorsements and tracked mods are read on each update check and written when you press Endorse or Track. Mod requirements, descriptions and changelogs, and collection search, go to `api.nexusmods.com/v2/graphql`. Every request carries your access token and names Eidos and its version. |
| Nexus downloads | sent | The files you ask for, from the Nexus CDN host the download link names, saved in the instance's downloads folder with an MO2-format `.meta` beside each one. |
| `nxm://` links | stored | `eidos nxm --register`, which `install.sh` runs for you, writes `~/.local/share/applications/eidos-nxm.desktop` and makes it the `nxm://` handler through `xdg-mime`, so the site's Mod Manager Download button hands its link to Eidos. The Arch package installs the same entry system-wide. |
| LOOT masterlists | sent | `raw.githubusercontent.com/loot/...`, on a game's first LOOT sort and whenever you ask for an update. Nothing of yours is sent. [GitHub privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Tool runtimes | sent | Only when you install a tool's prerequisites, and only for what the release bundle did not ship (the single `eidos-linux` file and a source build ship nothing): the .NET 10 runtime from `builds.dotnet.microsoft.com` and the Visual C++ and .NET 8 installers from `download.visualstudio.microsoft.com`, each refused unless it matches the SHA-256 built into Eidos; the .NET 10 SDK, always from `builds.dotnet.microsoft.com` and checked the same way; and `winetricks` for the other prerequisites (.NET Framework 4.8, XACT, .NET 6 and 7), which downloads what it installs from Microsoft and the other hosts winetricks uses. [Microsoft privacy statement](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Session logs | stored | `~/.local/share/Colony/Eidos/logs/`, mode 0600: ten sessions per instance, swept after 30 days. Your home folder is written as `~` and your user name as `<user>`, so a log can be pasted into an issue as it is. They never leave your machine unless you send them. |
| Run records | stored | `<instance>/logs/sessions/`: for each game or tool run, the profile, the enabled mods with their versions and Nexus file ids, the plugin order, a checksum of each profile INI, the game and Eidos versions, the exit code and the saves the run created. The 50 newest are kept, `eidos sessions` reads them, and `eidos pack` leaves them out. They stay on your machine. |
| Update reports | stored | `<instance>/logs/updates/<mod>.json`: what the last update or Replace changed in that mod's folder. `eidos pack` leaves them out. |
| Shortcuts | stored | `install.sh` writes `eidos.desktop` and its icons under `~/.local/share`; a tool or profile shortcut you create goes to `~/.local/share/applications`. |
| Desktop notifications | neither | Off by default. When on, they go to your desktop's notification service over D-Bus (`org.freedesktop.Notifications`) and name the job that ended. |
| Links you open | neither | Mod pages and other links open in your own browser through `xdg-open`. |
| Add-ons | neither | Extensions you install run as their own programs on your machine; what they do is theirs. |

## License

Eidos's original code is GPL-3.0-or-later; see [LICENSE](LICENSE). These parts
are GPL-3.0 without the "or later" option:

- the OMOD script translations in `eidos-install`; see their
  [upstream credits](crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- the `eidos-nif-preview` helper, built on the vendored nifly library; see its
  [component licenses and credits](crates/eidos-nif-preview/UPSTREAM.md);
- the `esplugin` and `libloadorder` crates the load-order code links, which
  declare `GPL-3.0`.

The other Rust crates in the binaries keep their own licenses (MIT, Apache-2.0,
BSD and similar). The release bundle carries their notices in `licenses/crates/`,
with an index of every crate and its license.

The binaries also embed eight genuine Microsoft DirectX redistributable DLLs
(`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` and `d3dx11_43`, 32- and 64-bit),
stored gzip-compressed (7.9 MB instead of 17.9 MB), which Eidos unpacks and
deploys into the game's prefix when a mod or tool needs them. They stay under
Microsoft's redistributable terms; see their
[provenance](crates/eidos-gamefeatures/assets/PROVENANCE.md).

The release bundle also carries, under `runtimes/`, Microsoft's Visual C++
2015-2022 installers, the .NET 8 Desktop Runtime installer and the .NET 10
runtime zip, as Microsoft publishes them. Their terms are in
`licenses/microsoft-runtimes/` (`vc_redist.txt`, `dotnet.txt`).
