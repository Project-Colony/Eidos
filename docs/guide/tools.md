# Tools: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

A tool run through Eidos sees **the merged view**, inside the game's own Proton
prefix. It reads what the game will read - every enabled mod, in priority order -
and whatever it writes lands in the Overwrite, where one click turns it into a
real mod.

## The ones Eidos finds by itself

Some tools are named uniquely enough to be found rather than declared, and xEdit
is the obvious case: `FO4Edit.exe` for Fallout 4, `SSEEdit.exe` for Skyrim SE,
`TES5Edit.exe` for the original, and so on - along with each one's
**QuickAutoClean** twin, which is the button for the dirty edits LOOT keeps
warning about. `PGPatcher.exe` is found the same way on the Creation Engine
games, and it arrives with an argument it cannot work without - see
[below](#why-pgpatcher-needs---ignore-mo2vfscheck). Eidos looks for them, by
file name, in:

- the game's install folder, and the `Root/` trees of enabled mods;
- **this instance's `mods/`**, which is where MO2 users install tools;
- the **tools folder** you set in Preferences (General -> Tools), for the
  directory shared between instances - `/mnt/Games/Tools` and the like.

The usual generators are found the same way, each on the games it supports: Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen and Synthesis.

The list is per game, so a Skyrim instance is never offered Fallout's editor.
The search stops four levels down, because a mod pool is hundreds of thousands
of files and this runs every time the tool list is built, and it does not follow
symlinks. A tool found this way is configured exactly like one you typed in: its
runtimes come from its name, by the same rule as everything below.

If a tool is somewhere else, or you want different arguments, add it by hand -
a user entry with the same title overrides anything found automatically.

## Cleaning a plugin LOOT flags as dirty

After a LOOT sort, the Diagnostics tab lists the plugins LOOT flags as dirty, for
the games that have both an xEdit and LOOT cleaning data (Skyrim LE, SE and VR,
Enderal SE, Fallout 3, New Vegas, Fallout 4 and 4 VR, Oblivion, Starfield). When
Eidos finds xEdit, registered in Executables or anywhere it looks for tools in
the instance, the card has Clean all and each dirty plugin has Clean with
QuickAutoClean on its row menu; otherwise the card says to add xEdit in
Executables. Eidos does not download xEdit. It runs xEdit's QuickAutoClean mode
on one plugin at a time, through the merged view, with `-qac -autoexit
-autoload`. xEdit saves the cleaned plugin over the one it loaded, which in the
merged view is a new file in the Overwrite; Eidos moves it into a mod named
`Cleaned plugins`, created enabled at the bottom of the list, where it wins every
conflict. The plugin in its own mod, or in the game's Data folder, is never
written. Disable `Cleaned plugins`, or hide one file in it, to undo a clean.

xEdit's backups, cache and log go to `<instance>/logs/qac-<time>/`, outside the
merged view, and the status line reports the counts from that log. Eidos deletes
that folder after a clean that worked and keeps it after one that failed.
Eidos records the size and date of the plugin each copy was made from in
`Cleaned plugins/meta.ini`. When that plugin changes, the Health tab reports the copy as out of date and offers to hide it.

## Adding one

In the GUI: **Tools -> Executables**, then Add. From the command line:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # run an exe once, without registering it
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` captures that run's output into `mods/<mod>`, whatever the tool's own setting says.

The script extender, the game binary and the launcher are detected
automatically; only extra tools need registering.

### Point it at the real file, wherever that is

Register the executable where it actually sits. If the tool was installed as a
mod, that is inside the mod folder:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

(that is the global instance's path - for a portable instance the same rule
applies under its own folder, `<instance>/mods/...`; note an absolute path like
this is the one thing that does not survive MOVING a portable folder later).

Eidos rewrites that path to the merged one before launching, so the tool runs
from `<game>/Data/CalienteTools/BodySlide/` and sees every other mod's files
there too. This matters more than it sounds: BodySlide ships an **empty**
`SliderSets` directory, and every body it can build comes from CBBE and the
outfit mods. Launched from its own mod folder it finds nothing and looks broken.

MO2 does the same rewriting, for the same reason - its own comment names FNIS.

A tool in a **disabled** mod cannot be rewritten, because its files are not in
the view either. Eidos says so and runs it from its own folder rather than
pretending.

## Tools from a Mod Organizer 2 list

A Wabbajack list or another MO2 instance opened as an Eidos instance keeps its `ModOrganizer.ini`, and the executables configured there can be copied into `tools.ini`:

```sh
eidos tool /path/to/list import-mo2           # show what would be added
eidos tool /path/to/list import-mo2 --write   # add them
```

In the window, the Executables dialog has **Import from ModOrganizer.ini**, with a box to untick for each entry; nothing is saved until you press Save.

Paths on `Z:` are Linux paths and stay as they are. A path on another drive letter moves under the list's folder when it names that folder (`D:\Lists\MyList\tools\SSEEdit.exe` becomes `<list>/tools/SSEEdit.exe`), in the executable, the working directory and the arguments. The script extender's loader and the game binary become plain names, so they run from the game folder Eidos mounts over, like the built-in entries. Entries that point elsewhere (a Windows system program, a tool outside the list) are listed as skipped with the reason, and a title already in `tools.ini` is never replaced.

## Sending a tool's output to its own mod

A generator - FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis - writes hundreds of
files. By default they land in the Overwrite with everything else. Set **Capture
output into** in the Executables editor and this run's output goes into that mod
instead:

```
Tools -> Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide and Synthesis arrive with a capture mod already set: `Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output` and `Synthesis Output`. DynDOLOD, TexGen and xLODGen arrive without one, because they write to the output folder you give them.

The mod is created if it does not exist. Only the files THIS run produced move;
anything that was already in the Overwrite stays there, so two tools with capture
targets do not steal each other's output. A run that wrote nothing leaves no
empty mod behind.

It is done after the run rather than by pointing the write layer at the mod,
which is how MO2 does it. Pointing the write layer at a mod would promote it to
top priority for the whole run - flipping every conflict it is in and flipping
them back afterwards - and would write straight through the mod's own files with
no copy-up. The capture reaches the same end state without either.

If the target mod is disabled, the output is still written but the game will not
see it, so the tool would regenerate the same files on the next run. Eidos warns
when that is the case.

### When its output is out of date

Eidos records which mods, plugin order, profile and executable a tool's output was made from. When one of them changes, the Diagnostics tab says so, and the card has a **Run <tool> again** button that runs the tool as it is configured now.

For a generator whose stale output breaks the game, such as Pandora or Nemesis, add `check_before_play=1` to its entry in `tools.ini`, or turn on **Ask before playing when its output is out of date** for it in the Executables dialog. Run then asks first: run the tool, or play anyway. Play anyway is remembered until the mods, plugins or profile change again, or until the window closes. Eidos never starts the tool by itself.

## The DLLs a tool needs are chosen by its NAME

This is the surprising part, so it is worth stating plainly: **the title you give
a tool decides which runtime prerequisites Eidos provisions for it.** The match
is a case-insensitive substring of the title.

| If the title contains | Eidos requests |
|---|---|
| `bodyslide`, `outfit` | `d3dx9_43`, `d3dcompiler_47` |
| `dyndolod`, `texgen`, `xlodgen` | `d3dcompiler_47`, `d3dx9_43`, `d3dx11_43`, `dotnet10` |
| `cathedral`, `cao` | `vcrun2022`, `d3dcompiler_47`, `d3dx11_43` |
| `synthesis` | `dotnetsdk10`, `vcrun2022` |
| `pandora` | `dotnet10` |
| `fnis` | `dotnet48` |
| `nemesis`, `loot` | `vcrun2022` |
| anything else | nothing |

So a tool registered as **`BodySlide`** gets its DirectX DLLs; the same
executable registered as **`BS`** gets nothing and may fail to start with an
error that says nothing about DLLs. Name tools after the program.

The list is in `default_prereqs` (`crates/eidos-instance/src/tools.rs`), and the
`Prereqs` field in the Executables dialog is editable - the detection is a
default, not a rule.

The title decides **arguments** the same way, for the one tool that needs one
nobody could guess: a title containing `pgpatcher` (or `parallaxgen`, its former
name) is given `--ignore-mo2vfscheck`. Why, and why it is not optional, is
[below](#why-pgpatcher-needs---ignore-mo2vfscheck).

### Three kinds of prerequisite

**Tier 1 - bundled DLLs** (`d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43`,
`d3dx11_43`). Eidos ships them and copies them into the prefix at launch.
Nothing to do, no network.

**Tier 2 - installers** (`vcrun2022`, `dotnet8`, `dotnetdesktop8`, `dotnet48`,
`xact`...). These write registry keys, the GAC and CLR hosts, so they cannot be
file-copied. Eidos installs three of them with no winetricks: `vcrun2022` runs
Microsoft's `VC_redist.x86.exe` and `VC_redist.x64.exe`, and `dotnet8` and
`dotnetdesktop8` run the .NET 8.0.12 Desktop Runtime installer, which carries the
.NET runtime too. The release bundle (`eidos-linux-bundle.tar.gz`) ships these
installers in its `runtimes/` folder, and `install.sh` copies them to
`~/.local/share/Colony/Eidos/runtimes/`. Without that copy (the single
`eidos-linux` file, a source build) Eidos **downloads them from Microsoft**
(`download.visualstudio.microsoft.com`) into the same folder. Either way a file is
refused unless its SHA-256 matches the one built into Eidos, and it runs silently
in the game's prefix with the Proton or Wine build that runs the game's tools. As
winetricks does, `vcrun2022` first sets the Visual C++ DLLs to `native,builtin` in
the prefix's DLL overrides. Afterwards Eidos checks that the prefix's C++ runtime
is not half Microsoft's and half Proton's, and reports the install as failed when
it is. Only the 64-bit .NET 8 is installed; a 32-bit .NET 8 tool still needs
`winetricks dotnetdesktop8`. The other verbs go through the system `winetricks`.

**Tier 3 - runtimes** (`dotnet10`, `dotnetsdk10`). A modern .NET runtime is 193
files that live in their own directory and are found through `DOTNET_ROOT`: never
registered, never installed into the prefix at all, so neither of the other tiers
can carry it. Eidos downloads it itself, checks it against a checksum built into
the binary, and caches it in `~/.local/share/Colony/Eidos/runtimes/` - **outside
any instance**, because 78 MB is not per-game and not per-profile. The release
bundle ships the `dotnet10` runtime zip in `runtimes/` too, so Eidos unpacks that
copy and downloads it only without one; `dotnetsdk10` is always downloaded.

`dotnetsdk10` is the .NET 10 SDK, a 301 MB download that unpacks to 808 MB.
Synthesis needs it to compile its patchers, and it calls a bare `dotnet` to build
and run them, so a tool that declares `dotnetsdk10` also gets the SDK folder on
`WINEPATH`, plus `NUGET_CERT_REVOCATION_MODE=offline`,
`DOTNET_GENERATE_ASPNET_CERTIFICATE=false` and `DOTNET_CLI_TELEMETRY_OPTOUT=1`.
A patcher build would leave MSBuild nodes and the VBCSCompiler server running,
and under Wine they keep the prefix's wineserver alive, so the tool also gets
`MSBUILDDISABLENODEREUSE=1`, `DOTNET_CLI_USE_MSBUILD_SERVER=0` and
`UseSharedCompilation=false`. `DOTNET_ROLL_FORWARD=Major` lets a patcher built
for .NET 8 (Mutagen before 0.54) run on the SDK's .NET 10 runtime when no .NET 8
runtime is found.
A Synthesis entry in `tools.ini` that still lists the old default, `dotnet8`,
`vcrun2022`, gets `dotnetsdk10`, `vcrun2022` when Eidos reads it.

NuGet checks the signature of every package it restores against the Windows root
store, and a Wine prefix lacks the code-signing and timestamping roots that the
SDK ships as `sdk/10.0.403/trustedroots/codesignctl.pem` and `timestampctl.pem`.
Without them, restoring `Mutagen.Bethesda.Synthesis` 0.36.6 fails with `NU3018`
and `NU3028`. So before a tool that declares `dotnetsdk10` runs, once the SDK is
downloaded, Eidos adds the 385 certificates of both files to the current user's
root store in the game's prefix, in the same `regedit` import that writes the
game path (see below). It imports them again if one goes missing. Every Windows
program in that prefix then trusts these roots. With them, in a fresh prefix
with an empty NuGet cache, the same restore and build succeed on system Wine
11.21 and on GE-Proton11-9.

Nothing in tiers 2 or 3 runs silently:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # fetch what is missing (downloads)
```

In the GUI the same states sit under the Prereqs field, and the missing ones are
buttons. A verb that is neither bundled, nor a runtime, nor a known winetricks
verb is reported as a probable typo rather than offered as a download.

### Why DynDOLOD needs `dotnet10`

DynDOLOD does not build object LOD itself: it shells out to LODGen, and it ships
three of them. `LODGenx64.exe` targets .NET Framework 4.8, which under Proton is
routed to Wine's Mono - whose `System.Uri` initialiser calls a method Mono does
not implement. It dies before its first line of work, leaving a log holding a
version banner and nothing else, and a DynDOLOD dialog that says only "failed for
one or more worlds".

Installing the real .NET Framework does not fix it: Proton replaces
`mscoree.dll` - the loader that would find it - with a symlink into its own
tree, and re-does that on every prefix update.

The build that works is `LODGenx64Win10.exe`, which targets modern .NET and never
touches `mscoree`. Point `DOTNET_ROOT` at a .NET 10 runtime and it runs. That is
what `dotnet10` provisions, and Eidos sets the variable when launching any tool
that declares it.

For the other Tier-2 verbs Eidos runs the system `winetricks` against Proton's
own `wine` and the game prefix, which sidesteps Steam's pressure-vessel
container and the protontricks + Proton-GE mismatch. A tool that declares an
uninstalled Tier-2 verb still launches, with a warning naming the verb and the
command to fix it - the user may have it from elsewhere.

### Why PGPatcher needs `--ignore-mo2vfscheck`

PGPatcher rewrites meshes and plugins so that whatever texture mods you have -
vanilla, parallax, complex material, PBR - use the right shader on every
surface. It refuses to run outside a mod manager, sensibly, because patching a
bare game folder would be destructive.

The trouble is HOW it checks: it looks for **usvfs**, the Windows DLL-hooking
layer MO2 injects into the game. Eidos does not have one and never will - the
merged view is a FUSE mount that Wine walks into without knowing it exists - so
the check finds nothing and the tool exits immediately with

```
Please verify that you are launching PGPatcher from MO2, VFS not detected.
```

naming a program the user is not running, before doing any work. Its author's
own answer to the two Linux reports of this ([#716][pg716] and [#725][pg725],
both from Fluorine, MO2's Linux fork, which is FUSE-based for the same reason)
is a flag that skips the check.

So Eidos seeds it. A PGPatcher found in a mod pool arrives with the argument
already set, and typing `PGPatcher` as the title in the Executables dialog fills
the Arguments field the same way. What you type always wins: the seeding only
fills a field you left empty, exactly like the prerequisites above.

The list is in `default_args` (`crates/eidos-instance/src/tools.rs`), beside
`default_prereqs`. It has one entry, and one entry is enough to be worth a
function: nobody can guess a flag, and the failure it prevents names the wrong
program.

Its runtime needs nothing, unusually for a .NET tool: it bundles its own
(`dotnetlib/`, with a `runtimeconfig.json` declaring `includedFrameworks`), and
ships the Microsoft CRT DLLs beside its executable. Leave Prereqs empty.

[pg716]: https://github.com/hakasapl/PGPatcher/issues/716
[pg725]: https://github.com/hakasapl/PGPatcher/issues/725

### PGPatcher and PGLite in Mod Organizer 2 mode

With Mod Manager set to `None`, PGPatcher sees one merged `Data` folder and
cannot tell which mod a mesh or texture came from, so it has no conflicts to
show and no mod order to follow. Its `Mod Organizer 2` mode reads the instance
itself: the profile's `modlist.txt`, the enabled mods in their priority order,
and the files in each mod's folder. PGLite, the in-game version, reads the
instance the same way. An Eidos instance already has MO2's layout (`mods/`,
`profiles/<name>/modlist.txt`, `overwrite/`), so this mode works on it.

In PGPatcher's launcher:

1. Set **Mod Manager** to `Mod Organizer 2`.
2. Set **MO2 Options -> Instance Location** to the Eidos instance folder,
   written as a Windows path on Wine's `Z:` drive, which is `/`. A portable
   instance at `/mnt/Games/Eidos-Skyrim` is `Z:\mnt\Games\Eidos-Skyrim`; a
   global one is
   `Z:\home\<you>\.local\share\Colony\Eidos\instances\<game id>`.
3. Keep `--ignore-mo2vfscheck` in the tool's arguments. In this mode PGPatcher
   looks for usvfs and stops when it is missing.

PGPatcher accepts the folder only if it holds a `modorganizer.ini`, so Eidos
writes `ModOrganizer.ini` at the instance root each time it starts the game or
a tool, for the profile being started. The file is rewritten only when its
content changes. It holds:

- in `[General]`: `gameName` (MO2's name for the game, so `New Vegas` for
  Fallout: New Vegas), `gamePath` (the game's install folder as a `Z:` path)
  and `selected_profile`;
- in `[Settings]`: `mod_directory`, `profiles_directory` and
  `overwrite_directory` as absolute `Z:` paths, plus `download_directory` when
  the instance has a `downloads/` folder.

Its first line is a comment saying Eidos wrote it. A `ModOrganizer.ini`
without that line, under any spelling of the name, is never changed or
deleted: an MO2 instance you opened in Eidos keeps its own, and PGPatcher reads
that file as it is. MO2 drops comments when it saves its settings, so a file
MO2 has saved over is MO2's from then on. An empty file counts as Eidos's.

Such a file names the profile MO2 last selected, and PGPatcher uses that
profile, not the one Eidos starts. When the two differ, Eidos logs a warning
at launch. Select the same profile in MO2, or rename or remove the file so
Eidos writes its own.

Eidos writes its file only when it launches something, so between launches it
names the profile of the last one. A tool that Eidos starts finds its own
profile there.

PGPatcher stops with an error when the profile has no enabled mod, and when
its output folder is the folder of an enabled mod, as it does under MO2.

## Wine's own programs in the game's prefix

The File menu runs four of Wine's programs in the game's prefix: winecfg, regedit, the Wine file explorer and the uninstaller. They start like any tool, with the Proton or Wine build that runs the game's tools, with the mods mounted, and the window waits for them. **Prefix drive_c** opens the prefix's `C:` drive in your file manager. A row is greyed out until the prefix has that program, which happens the first time the store starts the game.

Eidos writes one registry value itself, the game's `installed path` under `HKLM\Software\Bethesda Softworks\<game>`, before every tool run when it is wrong. A change made to it in regedit is put back at the next tool run.

## The game path in the prefix

Windows tools find their game by reading
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, a key the game's own
installer writes - and which Steam under Proton never runs. Without it xEdit,
Wrye Bash and DynDOLOD open on an empty path. Eidos writes it before running a
tool: idempotent, additive, and skipped if the prefix is uninitialised or in use.

## Reaching a tool: hide, pin, and a desktop shortcut

A game's defaults include tools you may never use, and a picker listing eight
entries to reach the second is a picker nobody reads. In the Executables dialog:

- **Pin to top** puts an entry at the head of the Run list.
- **Hide from picker** takes one out without deleting it.
- **Desktop shortcut** writes a `.desktop` into
  `~/.local/share/applications` - where a launcher belongs on a freedesktop
  system, so it turns up in your application menu and in a search rather than
  on the desktop. It runs `eidos tool <instance> run <title>` directly, which
  means the tool comes up **through the merged view with this instance's
  profile** without the Eidos window being open at all.

Hiding and pinning are about how a tool is *reached* rather than what it runs,
so they apply to the per-game defaults as well as to your own entries.

The script extender's loader, the launcher and the game binary have no
Desktop shortcut button. They start the game, which always starts through its
store, and the store's own shortcut already does that. A shortcut made for one
of them with Eidos 1.22 or earlier keeps working: like
`eidos tool <instance> run <title>` typed in a terminal, it asks the store to
start the game, as Run does in a window no store opened. When the store's
setting is missing, the command starts nothing and prints what to set. A
shortcut shows no message, so run the same command in a terminal to read it.

A profile can have its own shortcut for the game. Right-click the profile and
choose **Desktop shortcut**: the entry runs
`eidos tool <instance> run "<game>" --profile <name>`, which makes that profile
the active one, as picking it in the window does, and then asks the store to
start the game. The Eidos window the store opens shows that profile. `--profile`
works with any `eidos tool ... run` and selects the profile before the tool
starts; with `--print` it selects nothing.

## A tool that is its own Steam app

The Creation Kit is a separate Steam application and wants its own AppID; a few
other modding tools shipped through Steam are the same. Set **Steam AppID** on
the entry and Eidos launches it under that id instead of the game's.

On Windows this means a different launcher. Here it is two environment variables
on the run that was already being built - `SteamAppId` and `SteamGameId`, both,
because Proton reads one and Steam's own libraries read the other, and a tool
that sees them disagree fails oddly rather than clearly. `eidos tool ... --print`
shows exactly what the real run would get.

## Environment variables and tools that run without Proton

Three keys of `<instance>/tools.ini` have no field in the Executables dialog yet,
so add them to the tool's section by hand. Each `env=NAME=value` line sets one
variable for that tool's runs only:

```ini
[Tool/BodySlide]
exe=Data/CalienteTools/BodySlide/BodySlide x64.exe
env=PROTON_LOG=1
env=WINEDEBUG=-all
```

Eidos applies these lines after everything it sets itself, so a value here
replaces its own. That includes the `WINEDLLOVERRIDES` it builds from the DLLs
your mods ship: a tool that sets one must list in it every DLL the tool needs.
A name is letters, digits and `_`, and does not start with a digit. A line with
any other name is ignored.

`runner=native` runs `exe` itself, without Proton. The merged view is mounted
as for any tool, and the `arg0=`, `arg1=` lines are its arguments. Use it for a
Linux program, or for a tool on a copy Steam does not manage and Heroic records
no runner for: `exe` is that copy's Wine, the Windows program is the first
argument, and the prefix comes from `env=`.

```ini
[Tool/SSEEdit (GOG)]
exe=/opt/wine-ge/bin/wine
arg0=/home/me/Games/Skyrim/SSEEdit.exe
workdir=/home/me/Games/Skyrim
runner=native
env=WINEPREFIX=/home/me/Games/Heroic/Prefixes/default/Skyrim
```

`exe` must be an absolute path. The working directory defaults to the folder
that holds `exe`, here the runner's, so set `workdir=`. Only `exe` is rewritten
to its merged path: for a Windows program inside a mod, give its path under the
game's `Data` folder, where the merged view shows it. A native tool skips the
Proton steps: the registry key above, Steam's shader cache, the empty .NET
roots, and taking Steam's overlay out of `LD_PRELOAD`.

On a GOG or Epic copy installed with Heroic, tools need no such entry when the
game's Heroic settings (`GamesConfig/<appName>.json`) name a Wine or Proton
build: Eidos runs them with that build in the game's Heroic prefix, and
`eidos prereqs <instance> --install` uses it for the installers and for
winetricks. A copy installed with Legendary alone has no Heroic settings.

`slr=1` starts the tool, and the registry import before it, inside the Steam
Linux Runtime container that the tool's Proton build names (`require_tool_appid`
in its `toolmanifest.vdf`), the way Steam starts the game. That runtime must be
installed in a Steam library; if it is not, the run stops and names the Steam
app to install. `eidos prereqs` still runs its installers and winetricks on the
host, and a `runner=native` tool ignores the key. The container sees your home
folder and the game's Steam library. A tool, a mod or a runtime anywhere else, on another
drive for example, needs `env=STEAM_COMPAT_MOUNTS=/that/path` (several paths
separated by `:`). A Proton build installed under
`/usr/share/steam/compatibilitytools.d` is probably not usable with `slr=1`,
because `/usr` inside the container is the runtime's own. No tool run inside the
container has been tested yet.

`eidos tool <instance> run <title> --print` shows the final command and every
variable the run sets on top of the inherited environment, these lines
included.

## BethINI Pie

Eidos does not ship BethINI Pie. Put the release you downloaded from its Nexus page in this instance's `mods/` or in your tools folder, and `Bethini.exe` appears in the tool list. Run through Eidos, it edits the profile's copy of the game INIs. Before each run Eidos keeps a copy of those INIs; **Backups -> INIs** puts one back. An INI that did not exist when the copy was taken is left as it is.

## A tool's own settings are still its own

Eidos puts a tool in the right place with the right DLLs. What the tool then
does with its configuration is between you and the tool, and the failure is
usually silent.

The worked example, because it costs an hour otherwise: BodySlide's **Game Data
Path** (Settings) must point at the game's `Data` directory, not the game folder
above it. Set one level too high, a batch build reports "All sets processed
successfully" and writes 1439 meshes where the game will never look for them.
Eidos catches them - they land in `Overwrite/Root/` rather than in your
installation - but nothing is wrong from the game's point of view except that
your bodies are not built.

Eidos checks this for you. Before a BodySlide or Outfit Studio run, the status
line warns when `GameDataPath` or `OutputDataPath` in the winning
`CalienteTools/BodySlide/Config.xml` does not lead to the game's `Data` folder
during the run, and the Health tab shows the same card with the value to set. An
`S:` path is read the way Eidos maps `S:` for a tool, to the Steam library
folder. An `OutputDataPath` inside the instance's `mods/` folder, for an output
mod, is accepted. Health also shows a card when `Overwrite/Root/` holds a
`meshes` or `textures` folder.

Tool output belongs in the Overwrite. When a run produces something worth
keeping, **Overwrite -> Create mod...** turns it into an ordinary mod that can be
ordered, disabled and removed like any other.
