# Tools: xEdit, BodySlide, DynDOLOD, PGPatcher, FNIS

A tool run through Eidos sees the merged view, inside the game's Wine prefix
(Steam's Proton prefix, or the prefix Heroic set up). It reads what the game will
read (every enabled mod, in priority order), and whatever it writes lands in the
Overwrite, where one click turns it into a real mod. A `runner=native` tool runs
without Proton; see
[below](#environment-variables-and-tools-that-run-without-proton).

## The ones Eidos finds by itself

Some tools are named uniquely enough to be found rather than declared, and xEdit
is the obvious case: `FO4Edit.exe` for Fallout 4, `SSEEdit.exe` for Skyrim SE,
`TES5Edit.exe` for the original, and so on. Each comes with a QuickAutoClean
entry, which cleans the dirty edits LOOT warns about. `PGPatcher.exe` is found
the same way on every Bethesda game except Morrowind, and it arrives with an
argument it cannot work without (see
[below](#why-pgpatcher-needs---ignore-mo2vfscheck)). Eidos looks for them, by
file name, in:

- `Overwrite/Root/`, the game folder the instance runs from (the store install,
  or its game copy), and the `Root/` trees of enabled mods;
- this instance's `mods/`, which is where MO2 users install tools;
- the tools folder you set in Preferences (General -> Tools), for a directory
  shared between instances, such as `/mnt/Games/Tools`.

The usual generators are found the same way, each on the games it supports: Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen and Synthesis. So is BethINI Pie (`Bethini.exe`), on the same games as PGPatcher; see [BethINI Pie](#bethini-pie). On Stardew Valley, SMAPI (`StardewModdingAPI.exe`) is found the same way. It is also that game's script extender loader, so it starts the game through the store, as described in [Reaching a tool](#reaching-a-tool-hide-pin-and-a-desktop-shortcut).

The list is per game, so a Skyrim instance is never offered Fallout's editor.
The search stops four levels down, because a mod pool is hundreds of thousands
of files and this runs every time the tool list is built, and it does not follow
symlinks. A tool found this way is configured exactly like one you typed in: its
prerequisites, its arguments (PGPatcher's `--ignore-mo2vfscheck`) and its capture
mod come from its title, by the rules below.

If a tool is somewhere else, or you want different arguments, add it by hand.
A user entry with the same title overrides anything found automatically.

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
written. Disable `Cleaned plugins`, or hide one file in it, to undo a clean. A
plugin that sits in the Overwrite is not cleaned, because xEdit would rewrite
that file itself; turn the Overwrite into a mod first.

xEdit's backups, cache and log go to `<instance>/logs/qac-<time>/`, outside the
merged view, and the status line reports the counts from that log. Eidos deletes
that folder after a clean that worked and keeps it after one that failed.
Eidos records the size and date of the plugin each copy was made from in
`Cleaned plugins/meta.ini`. When that plugin changes, the Diagnostics tab shows
**Out-of-date copy of <plugin>**, with a button that hides the old copy.

## Adding one

In the GUI: **Tools** in the menu bar, or **Executables** on the toolbar, then
Add. From the command line:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse                       # list what is registered
eidos tool skyrimse run BodySlide         # run it through the merged view
eidos tool skyrimse run BodySlide --print # show the command without running it
eidos tool skyrimse rm BodySlide          # remove an entry you added
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"   # run an exe once, without registering it
eidos tool skyrimse run "SSEEdit QuickAutoClean" --output-mod "Cleaned plugins" -- -qac -autoexit -autoload Dawnguard.esm
```

`--output-mod <mod>` captures that run's output into `mods/<mod>`, whatever the tool's own setting says. `rm` removes only entries you added; a per-game default can be hidden, not removed.

An exe run with `--exe` whose file name is one Eidos knows for the game, ignoring case, gets that tool's title, and with it the prerequisites, arguments and capture mod below. `GenerateFNISforUsers.exe` is FNIS, so the `--exe` line above asks for `dotnet48` and captures into `FNIS Output`. Any other exe is titled by its file name. In the Data tab, an `.exe` row has **Add tool**, which opens the Executables dialog with the entry filled in, and **Run**, which runs the file once as `--exe` does. Both title the file the same way.

For a command that is not a Windows tool, `eidos shell skyrimse` opens your
`$SHELL` (`/bin/sh` when it is unset) in the game folder with the merged view
mounted. What the shell writes lands in the Overwrite, and leaving the shell
unmounts the view. `eidos shell skyrimse -- <command>` runs one command instead.
Ctrl+C in the interactive shell stops the command you typed and leaves the
shell open. The shell runs without Proton; a Windows program goes through
`eidos tool`.

In the window, **Stop** ends a tool run that hangs: press it twice, and Eidos
ends the run's programs, then handles its files as after any exit. With
**Notify the desktop when a long job ends** on (Preferences, General, Running a
game), a tool run that ends while the window is in the background sends a
desktop notification.

The script extender, the game binary and the launcher are detected
automatically; only extra tools need registering. On an instance that runs from
a game copy they are detected in the copy; see
[Tools on a game copy](#tools-on-a-game-copy).

### Point it at the real file, wherever that is

Register the executable where it actually sits. If the tool was installed as a
mod, that is inside the mod folder:

```
~/.local/share/Colony/Eidos/instances/skyrimse/mods/BodySlide.../CalienteTools/BodySlide/BodySlide.exe
```

That is the global instance's path. For a portable instance the same rule
applies under its own folder, `<instance>/mods/...`. An absolute path like this
is the one thing that does not survive moving a portable folder later.

Eidos rewrites that path to the merged one before launching, so the tool runs
from `<game>/Data/CalienteTools/BodySlide/` and sees every other mod's files
there too. BodySlide ships an empty `SliderSets` directory, and every body it
can build comes from CBBE and the outfit mods. Launched from its own mod folder
it finds nothing and looks broken.

MO2 does the same rewriting, for the same reason: its own comment names FNIS.

A tool in a disabled mod cannot be rewritten, because its files are not in the
view either. Eidos says so and runs it from its own folder.

## Tools from a Mod Organizer 2 list

A Wabbajack list or another MO2 instance opened as an Eidos instance keeps its `ModOrganizer.ini`, and the executables configured there can be copied into `tools.ini`:

```sh
eidos tool /path/to/list import-mo2           # show what would be added
eidos tool /path/to/list import-mo2 --write   # add them
```

In the window, the Executables dialog has **Import from ModOrganizer.ini**, with a box to untick for each entry. **Add selected** puts the ticked entries in the list, and nothing is saved until you press Save. Only a `ModOrganizer.ini` that Eidos did not write counts; see [PGPatcher and PGLite](#pgpatcher-and-pglite-in-mod-organizer-2-mode) for the one Eidos writes.

Paths on `Z:` are Linux paths and stay as they are. A path on another drive letter moves under the list's folder when it names that folder (`D:\Lists\MyList\tools\SSEEdit.exe` becomes `<list>/tools/SSEEdit.exe`), in the executable, the working directory and the arguments. The script extender's loader, the launcher and the game binary become plain names, so they run from the game folder Eidos mounts over, like the built-in entries. MO2's `hide` becomes **Hide from picker**, and `steamAppID` becomes **Steam AppID**. Entries that point elsewhere (a Windows system program, a tool outside the list) are listed as skipped with the reason, and a title already in `tools.ini` is never replaced.

An imported entry keeps MO2's title and arguments, and Eidos fills nothing from its title: no prerequisites, no extra argument and no capture mod. Set them in the Executables dialog. A PGPatcher entry from a list made on Windows usually lacks `--ignore-mo2vfscheck`; add it (see [below](#why-pgpatcher-needs---ignore-mo2vfscheck)).

An imported path that points into the list's `Stock Game` folder is moved onto the store install at run time only when the instance runs from that copy; see the next section.

## Tools on a game copy

When the instance runs from a game copy (a list's `Stock Game` or `Game Root`
folder, or a frozen copy; see
[Game files](usage.md#game-files-the-store-install-or-a-copy)), a tool sees the
copy's files, not the store install's. The merged view is still mounted on the
store install: the root view has the copy at the bottom, and the Data view reads
the copy's `Data`. Eidos looks for the script extender, the launcher, the game
binary and the known tools in the copy instead of the store install.

A tool path, an argument or a working directory that points into the copy is
moved onto the store install before the run. It can be written as a Linux path
or as a Wine `Z:` path: `-D:"Z:\...\Stock Game\Data"` becomes the same path
under the store install. Entries imported from a list's `ModOrganizer.ini` need
this. When the instance runs from the store install nothing is moved, so such an
entry reads the bare copy without your mods.

The game's `installed path` in the prefix registry and the `gamePath` in the
`ModOrganizer.ini` that Eidos writes still name the store install, where the
merged view is. If the copy folder is missing, or holds no game binary, a tool
run stops, as a game launch does, and the message names
`eidos stock-game <instance> store` to go back to the store install.

`eidos stock-game <instance>` shows which game files the instance runs and
which copies it could switch to. `use <folder>` and `store` switch them, and
`freeze --yes` copies the store install into the instance's `Stock Game` folder
and runs from it. The Game files card in the Diagnostics tab does the same; see
[Game files](usage.md#game-files-the-store-install-or-a-copy).

## Sending a tool's output to its own mod

A generator (FNIS, Nemesis, BodySlide, DynDOLOD, Synthesis) writes hundreds of
files. By default they land in the Overwrite with everything else. Set **Capture
output into** in the Executables editor and this run's output goes into that mod
instead:

```
Executables -> (your tool) -> Capture output into: FNIS Output
```

Pandora, Nemesis, FNIS, BodySlide and Synthesis arrive with a capture mod already set: `Pandora Output`, `Nemesis Output`, `FNIS Output`, `BodySlide Output` and `Synthesis Output`. DynDOLOD, TexGen and xLODGen arrive without one, because they write to the output folder you give them.

The mod is created if it does not exist. Only the files this run produced move;
anything that was already in the Overwrite stays there, so two tools with capture
targets do not take each other's output. A run that wrote nothing leaves no
empty mod behind.

Eidos moves the files after the run. MO2 instead points the write layer at the
mod, which promotes that mod to top priority for the whole run: every conflict it
is in flips, and flips back afterwards. The writes also go straight through the
mod's own files with no copy-up. The capture reaches the same end state without
either.

If the target mod is disabled, the output is still written but the game will not
see it, so the tool would regenerate the same files on the next run. Eidos warns
when that is the case.

### When its output is out of date

Eidos records which mods, plugin order and profile a tool's output was made from, the size and date of the tool's executable, and the entry's path and arguments in `tools.ini`. When one of them changes, the Diagnostics tab shows **Generated output inputs changed**, and the card has a **Run <tool> again** button that runs the tool as it is configured now.

For a generator whose stale output breaks the game, such as Pandora or Nemesis, add `check_before_play=1` to its entry in `tools.ini`, or turn on **Ask before playing when its output is out of date** for it in the Executables dialog. Run then asks first: run the tool, or play anyway. Play anyway is remembered until the mods, plugins or profile change again, or until the window closes. Eidos never starts the tool by itself. With **Start the game at once when a store opens Eidos** on, such a question keeps the window open instead of starting the game.

## Prerequisites and arguments come from the title

The title you give a tool decides which runtime prerequisites Eidos provisions
for it. The match is a case-insensitive substring of the title.

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

So a tool registered as `BodySlide` gets its DirectX DLLs; the same executable
registered as `BS` gets nothing and may fail to start with an error that says
nothing about DLLs. Name tools after the program.

The list is in `default_prereqs` (`crates/eidos-instance/src/tools.rs`), and the
`Prereqs` field in the Executables dialog is editable: the title only fills in
the default.

The title decides arguments the same way, for the one tool that needs one
nobody could guess: a title containing `pgpatcher` (or `parallaxgen`, its former
name) is given `--ignore-mo2vfscheck`. Why, and why it is not optional, is
[below](#why-pgpatcher-needs---ignore-mo2vfscheck). The capture mod comes from
the title too (see [above](#sending-a-tools-output-to-its-own-mod)).

### Three kinds of prerequisite

Tier 1 is the bundled DLLs: `d3dx9_43`, `d3dcompiler_47`, `d3dcompiler_43` and
`d3dx11_43`. Eidos carries them inside its binaries and copies them into the
prefix at launch. In a 64-bit prefix, Proton's default, the 64-bit build goes
into `system32` and the 32-bit one into `syswow64`. Nothing to do, no network.
Each build is stored gzip-compressed, so the eight files take 7.9 MB of each
binary instead of 17.9 MB, and only the one being copied is unpacked (4.7 MB at
most).

Tier 2 is the installers: `vcrun2022`, `dotnet6`, `dotnet7`, `dotnet8`,
`dotnetdesktop6`, `dotnetdesktop8`, `dotnet48`, `xact` and `xact_x64`. These write
registry keys, the GAC and CLR hosts, so they cannot be file-copied. Eidos
installs three of them with no winetricks: `vcrun2022` runs Microsoft's
`VC_redist.x86.exe` and `VC_redist.x64.exe`, and `dotnet8` and `dotnetdesktop8`
run the .NET 8.0.12 Desktop Runtime installer, which carries the .NET runtime
too. The release bundle (`eidos-linux-bundle.tar.gz`) ships these installers in
its `runtimes/` folder, and `install.sh` copies them to
`~/.local/share/Colony/Eidos/runtimes/` for the user who runs it, also with
`--system`. Eidos looks for each file there first, then in a `runtimes/` folder
beside the `eidos` binary (an unpacked bundle run in place). Without either copy
(the single `eidos-linux` file, a source build, or another user of a system
install) Eidos downloads them from Microsoft (`download.visualstudio.microsoft.com`)
into `~/.local/share/Colony/Eidos/runtimes/` and keeps them there. Either way a
file is refused unless its SHA-256 matches the one built into Eidos, and it runs
with no window (`/install /quiet /norestart`) in the game's prefix, with the
Proton or Wine build that runs the game's tools. As winetricks does, `vcrun2022`
first sets the Visual C++ DLLs to `native,builtin` in the prefix's DLL
overrides. Afterwards, with a Proton build, Eidos checks that the prefix's C++
runtime is not half Microsoft's and half Proton's. When it is, the install fails:
the message names the DLL pairs that come from different builds, and says to
install `vcrun2022` with winetricks instead, which replaces `msvcp140.dll`
itself. Only the 64-bit .NET 8 is installed; a 32-bit .NET 8 tool still needs
`winetricks dotnetdesktop8`.

The other six verbs go through the system `winetricks`, which must be on `PATH`.
Eidos runs it with the `wine` of the Proton or Wine build that runs the game's
tools, against the game prefix, which sidesteps Steam's pressure-vessel container
and the protontricks + Proton-GE mismatch. Some winetricks verbs need
`cabextract`, and Eidos warns when it is missing. A tool that declares an
uninstalled Tier 2 verb still launches, with a warning naming the verb and the
command to fix it, because you may have installed it another way.

A Tier 2 verb counts as done when the prefix's own `winetricks.log` lists it,
whoever ran winetricks or protontricks, or when Eidos installed it into that same
prefix. Eidos records its own installs in `prereqs.done` in the instance, tied to
the prefix's path and inode, so a rebuilt prefix shows its verbs as pending
again. **Rebuild prefix**, a Diagnostics action for a Steam copy, renames
`compatdata/<appid>` to `<appid>.eidos-backup-<time>` and deletes nothing;
Steam makes a new prefix the next time it starts the game.

Tier 3 is the runtimes, `dotnet10` and `dotnetsdk10`. A modern .NET runtime is 193
files that live in their own directory and are found through `DOTNET_ROOT`. It
is never registered and never installed into the prefix, so neither of the other
tiers can carry it. Eidos keeps it outside any instance, because 78 MB is not
per-game and not per-profile: each runtime is unpacked to
`~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/` (`dotnet10-10.0.10`,
`dotnetsdk10-10.0.403`). Eidos looks for the runtime's zip under its download
name in the same two folders as the Tier 2 installers, and unpacks the copy the
release bundle ships (`dotnet-runtime-10.0.10-win-x64.zip`, for `dotnet10` only)
or downloads it. The bundle does not ship the SDK's
`dotnet-sdk-10.0.403-win-x64.zip`, but a copy put there by hand is used and
saves the download. Eidos refuses either file unless its SHA-256 matches the one
built into Eidos. It unpacks the zip with `7z`, the same 7-Zip it uses for mod
archives.

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

Eidos installs nothing from tiers 2 or 3 until you ask:

```sh
eidos prereqs skyrimse            # show what the registered tools need, and their state
eidos prereqs skyrimse --install  # install what is missing (the bundle's copies first, then downloads)
```

In the GUI the same states sit under the Prereqs field, and the missing ones are
buttons. A verb that is neither bundled, nor a runtime, nor a Tier 2 verb is
reported as a probable typo rather than offered as a download.

`eidos prereqs <instance> --install` refuses while the game, Steam or a tool
still holds the prefix. Tier 3 runtimes install even before the prefix exists.
The installers Eidos runs itself need a prefix that exists: before the store has
started the game once, they stop with "start the game once through its store,
then install again". Tier 2 verbs are installed one at a time and each success
is recorded, so after a failure the same command resumes. For a GOG or Epic copy
without Heroic settings, `--install` copies the bundled DLLs and then stops
before the Tier 2 verbs: it says to use that runner's own winetricks, and reads
the prefix's `winetricks.log` at the next check.

### Why DynDOLOD needs `dotnet10`

DynDOLOD does not build object LOD itself: it shells out to LODGen, and it ships
three of them. `LODGenx64.exe` targets .NET Framework 4.8, which under Proton is
routed to Wine's Mono, and Mono's `System.Uri` initialiser calls a method Mono
does not implement. LODGen dies before its first line of work, leaving a log
holding a version banner and nothing else, and a DynDOLOD dialog that says only
"failed for one or more worlds".

Installing the real .NET Framework does not fix it: Proton replaces
`mscoree.dll`, the loader that would find it, with a symlink into its own tree,
and does that again on every prefix update.

The build that works is `LODGenx64Win10.exe`, which targets modern .NET and never
touches `mscoree`. Point `DOTNET_ROOT` at a .NET 10 runtime and it runs. That is
what `dotnet10` provisions, and Eidos sets the variable when launching any tool
that declares it.

### Why Pandora needs `dotnet10`

Pandora is an Avalonia app, not a WPF one, and it has targeted `net10.0` since
Pandora 4.1.0, so the .NET 10 runtime alone runs it. When it finds no runtime it
says to install the .NET Desktop Runtime. That is the generic text the .NET
launcher shows for any GUI program that finds no runtime, and `dotnetdesktop8`
does not fix it.

Eidos 1.21.0 and older gave Pandora `dotnetdesktop8`, and an entry keeps its
`prereqs=` line. When Eidos reads a Pandora entry that still lists only
`dotnetdesktop8`, and the `.runtimeconfig.json` beside its executable names
`net10.0`, the entry gets `dotnet10` instead. Pandora 2.x targets .NET 8 and
keeps what it has.

### Why PGPatcher needs `--ignore-mo2vfscheck`

PGPatcher rewrites meshes and plugins so that whatever texture mods you have
(vanilla, parallax, complex material, PBR) use the right shader on every
surface. It refuses to run outside a mod manager, because patching a bare game
folder would be destructive.

PGPatcher checks for usvfs, the Windows DLL-hooking layer MO2 injects into the
game. Eidos has none: the merged view is a FUSE mount that Wine walks into
without knowing it exists. So the check finds nothing and the tool exits
immediately with

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
`default_prereqs`.

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
  Fallout: New Vegas), `gamePath` (the game's store install folder as a `Z:`
  path, also on an instance that runs from a game copy) and `selected_profile`;
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

Tools in Mod Organizer 2 mode also read the game from that file's `gamePath`.
When it names neither the game folder the instance runs from nor the store
install, the Diagnostics tab shows **ModOrganizer.ini points tools at another
game folder**, with that path and the game folder this instance runs. Change
`gamePath` in the file, or remove the file so Eidos writes its own.

Eidos writes its file only when it launches something, so between launches it
names the profile of the last one. A tool that Eidos starts finds its own
profile there.

PGPatcher stops with an error when the profile has no enabled mod, and when
its output folder is the folder of an enabled mod, as it does under MO2.

## Wine's own programs in the game's prefix

The File menu runs four of Wine's programs in the game's prefix: winecfg, regedit, the Wine file explorer and the uninstaller. They start like any tool, with the Proton or Wine build that runs the game's tools, with the mods mounted, and the window waits for them. **Prefix drive_c** opens the prefix's `C:` drive in your file manager. A row is greyed out until the prefix has that program, which happens the first time the store starts the game. From a terminal, `eidos tool <instance> run winecfg` does the same, and so do `regedit`, `explorer` and `uninstaller`, when no tool in the list has that title.

A change made in regedit to the game's `installed path` is put back at the next tool run; see the next section.

## The game path in the prefix

Windows tools find their game by reading
`HKLM\Software\Bethesda Softworks\<game>` `installed path`, a key the game's own
installer writes. Steam under Proton never runs that installer. Without the key
xEdit, Wrye Bash and DynDOLOD open on an empty path.

Before a tool run, Eidos writes the key with a `regedit` import in the game's
prefix, unless an earlier import already wrote the same path and the prefix
still holds it. The key names the store install, also on an instance that runs
from a game copy, because the merged view is mounted there. For a tool that
declares `dotnetsdk10`, the same import adds the NuGet signing roots (see
[above](#three-kinds-of-prerequisite)). A `runner=native` tool skips this step.
When the prefix does not exist yet, or another program holds it, the tool still
runs, and Eidos prints a warning that names the store to start the game from
once, or the program to close. When the key names a folder other than the
store install, the Diagnostics tab shows **Windows tools cannot find <game>**
until the next tool run writes it again.

## Reaching a tool: hide, pin, and a desktop shortcut

A game's defaults include tools you may never use. In the Executables dialog:

- **Pin to top** puts an entry at the head of the Run list.
- **Hide from picker** takes one out without deleting it.
- **Desktop shortcut** writes a `.desktop` into
  `~/.local/share/applications`, so it turns up in your application menu and in
  a search, not on the desktop. It runs `eidos tool <instance> run <title>`
  directly, so the tool comes up through the merged view with this instance's
  profile without the Eidos window being open.

Hiding and pinning change only how a tool is reached, so they work on the
per-game defaults as well as on your own entries.

The script extender's loader, the launcher and the game binary have no
Desktop shortcut button. They start the game, which always starts through its
store, and the store's own shortcut already does that. A shortcut made for one
of them with Eidos 1.22 or earlier keeps working: like
`eidos tool <instance> run <title>` typed in a terminal, it asks the store to
start the game, as Run does in a window no store opened. When the store's
setting is missing, the command starts nothing and prints what to set. A
shortcut shows no message, so run the same command in a terminal to read it.

Eidos hands any entry to the store this way when its executable has the file
name of the script extender's loader, the launcher or the game binary, ignoring
case: a default entry, one you added, one imported from MO2, or a `--exe` run.
It first marks this instance as the last one used, so the window the store opens
shows this instance, then runs `steam -applaunch <id>`,
`xdg-open heroic://launch?appName=<app>&runner=<runner>` for Heroic, or
`legendary launch <app>`. With `--print` it prints that store command and starts
nothing. The `steam` command must be on `PATH`: the Flatpak Steam does not put
it there, so the start fails and the message says to start the game from Steam.

The entry runs directly through the merged view, like any tool, when a store
started the process: one of `SteamGameId`, `SteamAppId`, `HEROIC_APP_NAME`,
`HEROIC_APP_RUNNER` or `EIDOS_STORE_STARTED` is set. Without one of them, Eidos
does not start the game with Proton itself. When the instance has no script extender loader (in the game
folder it runs from, in an enabled mod's `Root/` or in `Overwrite/Root/`),
nothing starts and the line says to install the script extender first. For a GOG copy outside Heroic, or a store id
Eidos cannot pass on, it says to start the game from that store.

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
on the same run, `SteamAppId` and `SteamGameId`. Eidos sets both, because Proton
reads one and Steam's own libraries read the other, and a tool that sees them
disagree fails without a clear error. `eidos tool ... --print` shows exactly
what the real run would get.

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

Eidos applies these lines after everything it sets itself and after the
variables in the profile's `launch-env.ini`, so a value here replaces both.
That includes the `WINEDLLOVERRIDES` Eidos builds from the DLLs your mods ship:
a tool that sets one must list in it every DLL the tool needs. The profile's
wrappers go around a tool's command as they do around the game's; see
[Variables and wrappers per profile](usage.md#variables-and-wrappers-per-profile).
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
folder and the game's Steam library. A tool, a mod or a runtime anywhere else,
on another drive for example, needs `env=STEAM_COMPAT_MOUNTS=/that/path`
(several paths separated by `:`). A Proton build installed under
`/usr/share/steam/compatibilitytools.d` is probably not usable with `slr=1`,
because `/usr` inside the container is the runtime's own. No tool run inside the
container has been tested yet.

`eidos tool <instance> run <title> --print` shows the final command and every
variable the run sets on top of the inherited environment, these lines
included. For an entry that starts the game it prints the store command instead
(see [Reaching a tool](#reaching-a-tool-hide-pin-and-a-desktop-shortcut)).

## BethINI Pie

Eidos does not ship BethINI Pie. Put the release you downloaded from its Nexus page in this instance's `mods/` or in your tools folder, and `Bethini.exe` appears in the tool list, on the same games as PGPatcher. Run through Eidos, it edits the profile's copy of the game INIs. Before each run of a tool whose title contains `bethini`, Eidos keeps a restore point of those INIs; **Backups** on the toolbar, then **INIs**, puts one back. An INI that did not exist when the copy was taken is left as it is. The dialog keeps the 10 newest restore points of each kind, and restoring one first backs up the INIs as they are, so a wrong pick can be undone.

## A tool's own settings are still its own

Eidos does not check most tool settings; a wrong one usually fails with no
error.

BodySlide's **Game Data Path** (Settings) must point at the game's `Data`
directory, not the game folder above it. Set one level too high, a batch build
reports "All sets processed successfully" and writes 1439 meshes where the game
will never look for them. Eidos catches them in `Overwrite/Root/`, so your game
installation is untouched, and the only sign in the game is that your bodies are
not built.

Before a BodySlide or Outfit Studio run, the status line warns when
`GameDataPath` or `OutputDataPath` in the winning
`CalienteTools/BodySlide/Config.xml` does not lead to the game's `Data` folder
during the run, and the Diagnostics tab shows the same warning as a card with
the value to set. `eidos check <instance>` reports it too. An `S:` path is read
the way Eidos maps `S:` for a tool, to the Steam library folder. An
`OutputDataPath` inside the instance's `mods/` folder, for an output mod, is
accepted. Diagnostics also shows a card when `Overwrite/Root/` holds a `meshes`
or `textures` folder.

Tool output belongs in the Overwrite. When a run produces something worth
keeping, **Overwrite -> Create mod...** turns it into an ordinary mod that can be
ordered, disabled and removed like any other.
