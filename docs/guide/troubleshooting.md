# Troubleshooting and diagnostics

The environment switches, how to read the op counters, the Health cards of the
Diagnostics tab, how to find the mod behind a crash, what to attach to a bug
report, and the known issues with their history.

## Diagnosing the VFS

Two environment variables cover most cases where the game sees something the
filesystem does not agree with:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

The granular form is what found the `FOPEN_KEEP_CACHE` crash under Known
issues: turning all four off answers "is it the caching?", and only naming them
answers "which one". The counters answer the other half. With passthrough on
(it is off by default), a load that shows `read 0` is one where
`FUSE_PASSTHROUGH` served every byte in the kernel, so anything you were about
to tune on the read path is already free.

A few more switches narrow a fault down further:

```sh
EIDOS_FUSE_TRACE=open,opendir       # log each open / opendir (1 = every channel)
EIDOS_FUSE_THREADS=1                # one daemon thread (default: CPU count, at most 4)
EIDOS_FUSE_OPENDIR=1                # keep opendir handles even when the kernel can skip them
EIDOS_NO_INDEX=1                    # walk the layers instead of using the layer index
EIDOS_NO_OW_INDEX=1                 # the same for the Overwrite's membership index
EIDOS_FUSE_KEEP_CACHE=1             # FOPEN_KEEP_CACHE back on, see Known issues
```

For a Steam launch, put them before the command in the launch option, for
example `EIDOS_FUSE_STATS=1 ~/.local/bin/eidos-gui %command%`. The output goes
to the run's log: the instance's `logs/run-*.log` for a window launch, Steam's
console for a Steam launch.

To look at the merged view itself, `eidos shell <instance>` opens a shell in
the game folder as the game sees it, and
`eidos shell <instance> -- ls Data/meshes` runs one command there; anything
written lands in the Overwrite
([usage.md](usage.md#a-shell-in-the-merged-view)). The Data tab's Save tree
button writes the whole merged `Data` tree to a text file, one file per line
with the mod it comes from, with your home folder hidden.

## Mount a union by hand

`eidos-fuse` comes with the Arch package and with source builds: `just build`
builds it, and `just install` or `install.sh` copies it when it is there. The
release bundle and `eidos-linux` do not carry it. Its usage line:

```
eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
```

The first `--layer` wins on conflict; the last is your pristine game data. The
mount needs only `/dev/fuse` and `fusermount3` (no overlayfs, no Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read and write through /mnt/point ...
fusermount3 -u /mnt/point
```

Writes land in `--overwrite <dir>` (a temporary directory when omitted), so the
layers themselves stay pristine even here.

## Why passthrough is off by default

Passthrough hands the kernel the real backing file so reads skip this daemon
entirely. The A/B below was measured with an earlier daemon, before the change
described in the known issue "FUSE passthrough stops the game loading any mod
content" further down. That change avoids the most likely cause but has not
been re-measured in a game, so passthrough stays off by default until it is.
Measured A/B on Skyrim SE 1.6.1170, proton-cachyos 11.0, kernel 7.1.4, the same
82-plugin load order, the only variable being whether the binary carried the
capability:

| passthrough | `NtCreateFile` failures with `STATUS_ACCESS_VIOLATION` |
|-------------|--------------------------------------------------------|
| on          | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| off         | 0                                                      |

With it on, the game opened none of its own archives or plugins, which surfaced
in-game as mods that were not there - no error, no log line. With it off the
same load order reached gameplay with its plugins, archives and Papyrus scripts
live.

The failure was invisible from inside the daemon: its own `open` succeeded
every time and the kernel never refused a backing file (verified across a full
failing session with `EIDOS_FUSE_TRACE=open`: zero `open FAILED`, zero
`passthrough refused`). The error was produced after the daemon replied
`opened_passthrough`, so no daemon-side logging could see it. It was not
extension-specific either - it hit archives and plugins alike, i.e. the files
the game holds open for its whole run.

`EIDOS_FUSE_PASSTHROUGH=1` turns it back on, for measuring what it buys or for
re-testing the mechanism. The capability warnings in the launcher and the
Diagnostics tab only appear when you have asked for it.

## Is the layer index actually in use?

`LayerStack::new` either builds a complete map of the read-only layers or none
at all. Without one, every query walks the layers as before: the answers are the
same, a save load takes about three times as long, and nothing else looks
different.

When the index is off, the mount says why, once, at startup:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

The reason names an unreadable directory, a layer more than 64 directories
deep, more than 4,000,000 entries, or `EIDOS_NO_INDEX`. The line goes to the
run's log: the instance's `logs/run-*.log` for a window launch, Steam's console
for a Steam launch. With `EIDOS_FUSE_STATS=1`, the dump at unmount also has a
`layer index:` line, and a non-zero `unindexed` count on a normal mount means the
index was off.

The examples below check the same thing offline, against an instance's folders,
with no mount:

```sh
cargo run --release -p eidos-core --example index_health -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost -- <mods-dir> <overwrite-dir>
```

`index_health` resolves real paths with and without the index and compares the
directory scans. `index_agrees` checks the two answer the SAME thing, on every
path and every listing of a real instance. `listing_cost` measures what the
merged-children map saves on `readdir`.

`EIDOS_NO_INDEX=1` forces the walk, for when the difference between the two
answers is the thing being debugged.

## Starting the game with the CLI

To launch the game through the `eidos` CLI rather than the window, set its
Steam launch option to the absolute path of `eidos`, for example:

```
~/.local/bin/eidos play skyrimse -- %command%
```

No `WINEDLLOVERRIDES` prefix is needed. When an enabled mod imports
`d3dcompiler_47`, Eidos deploys its bundled copy into the prefix, forces it
native, and merges that with any DLL overrides a mod ships (ENB/ReShade/`.asi`
loaders).

Eidos never starts the game through Proton by itself.
`eidos tool <instance> run <title>` with an entry that starts the game (the script extender's loader,
the launcher or the game binary) asks the store that owns the copy to start it,
as Run does in a window no store opened, and prints what to set when the store's
setting does not start Eidos. `--print` prints the store command instead of
running it.

## A game or tool that will not close

While a game or tool runs, the Run button becomes Stop, and the lock card has a
Stop button too. Press it twice. The second press ends every program the run
started: SIGTERM first, then SIGKILL after 10 seconds for whatever is left.
Anything the game had not saved is lost. Eidos then does what it does after any
exit: INI and plugin changes go back into the profile, tool output is recorded,
and the mods are unmounted. The run is recorded with exit code 143 whatever
the program returned, so `eidos sessions <instance> diff` never takes it as the
last good run. Unlock is different: it gives the window back and leaves the
program running. From a terminal, `kill <pid>` on the `eidos` process (the pid
is on the lock card) does the same as Stop.

## Finding what broke

To find which change broke the game:

- `eidos sessions <instance> diff` lists what changed since the last game run
  that exited with code 0: mods, versions, plugin order, INIs and the game
  executable ([usage.md](usage.md#what-changed-since-the-last-good-run)).
- Test run, next to Run, plays with the profile's mods, without mods, or with
  the selected mods and the masters they need, and sends everything the game
  writes to `<instance>/test-run/`. After the run you keep or throw away each
  group of files: saves, INI changes, plugin state, Overwrite output and logs
  ([usage.md](usage.md#test-runs)).
- Find the mod behind a problem, on the Diagnostics tab, runs test runs with
  half of the remaining suspects each time and asks whether the problem
  happened. 200 suspects take about 8 runs, and the search survives closing
  Eidos ([usage.md](usage.md#finding-the-mod-behind-a-problem)).

A stopped run never pre-fills a Yes in that search, because it is recorded with
exit code 143 (see above).

## Health cards

The Diagnostics tab shows a Health card for each problem it finds, with its
cause and, for some, a button that fixes it. The launch option cards are
described in [install.md](install.md), the NVAPI, fullscreen and ENB cards in
[graphics.md](graphics.md), and the Proton and prefix cards in the next
section. `eidos check <instance>` prints the findings that need no window
([usage.md](usage.md#health-checks-from-the-terminal) lists them), the prefix,
registry and `S:` drive cards among them. The window alone shows the Proton
card with its Flatpak, unknown-build and older-build variants, patch order,
extender scripts, archives no plugin loads, Nexus requirements, out-of-date
plugin copies and the Display card.

### The system

- "vm.max_map_count is <value>" appears below 1048576, the default on Arch Linux,
  Fedora 39 and later, and Ubuntu 24.04 and later. Some games under Proton map
  more memory regions and then crash with no error. Put
  `vm.max_map_count = 1048576` in a file under `/etc/sysctl.d`, for example
  `80-gamecompatibility.conf`, then run `sudo sysctl --system`.
- "The open files limit is <value>" appears when the hard limit is below 524288,
  which Proton's esync needs; games can fail to start or crash below it.
  systemd 240 and later set it. On an older setup, set
  `DefaultLimitNOFILE=524288` in `/etc/systemd/system.conf` and
  `/etc/systemd/user.conf`, then log in again.
- "Game files on a Windows file system" names the game folder, the prefix or the
  instance root when it is on `ntfs`, `ntfs3`, `fuseblk` (ntfs-3g, exfat-fuse),
  `exfat` or `vfat`. Proton needs symlinks, Unix permissions and case-sensitive
  names. Move the game, its prefix and the instance to a Linux file system such
  as ext4, btrfs or xfs.

### The game files

The Game files card at the top of the tab shows whether the instance runs the
store install or a copy of the game inside the instance
([usage.md](usage.md#game-files-the-store-install-or-a-copy)). Launches are
refused, with a card that says why, in three cases:

- "The game copy '<folder>' is missing": the instance runs a copy that is gone.
  Put it back, or return to the store version.
- "'<folder>' has no <binary>": the folder the instance runs holds no game
  executable, so it is not a copy of the game.
- "<game> cannot run from a copy yet": Morrowind, and Oblivion with
  `bUseMyGamesDirectory=0`, keep their plugin list in the game folder.

"ModOrganizer.ini points tools at another game folder" appears when a list's
own `ModOrganizer.ini`, not written by Eidos, has a `gamePath` other than the
game files the instance runs. Tools in Mod Organizer 2 mode, such as PGPatcher,
read the game from there. Change `gamePath` in that file, or remove the file so
Eidos writes its own.

### The load order

- "sTestFile lines make the game ignore plugins.txt" appears for Fallout 4,
  Fallout 4 VR and Starfield when the profile's `Fallout4Custom.ini` or
  `StarfieldCustom.ini` has `sTestFile1` to `sTestFile10` lines in `[General]`.
  While one is set, the game loads those plugins and the load order Eidos writes
  has no effect. Fix removes the lines from the profile's copy and keeps the
  previous file as `<ini>.pre-fix`.
- Starfield: enabled `.esp` plugins without the master flag get a card, because
  Starfield cannot sort them among `.esm` and master-flagged plugins and keeps
  some of their records loaded for the whole session. An `.esm` release from the
  author avoids both.
- Starfield: "Invalid blueprint plugins" lists plugins that have some but not all
  of the blueprint flag, a name that starts with `BlueprintShips-` and the
  `.esm` extension. "Blueprint plugins without their main plugin" lists those
  whose main plugin is not enabled: `BlueprintShips-Example.esm` needs
  `Example.esm`, `.esp` or `.esl`.
- "<patch> loses <n> file(s) to <master>" appears when a plugin in one mod needs
  a master from another mod, and that other mod has the higher priority, so its
  copies of the files both ship win. A patch usually goes below the mod it
  patches, and the card's Move button puts it there. Keep the order if the patch
  is meant to lose those files.
- "Out-of-date copy of <plugin>" appears when a mod holds a cleaned or
  light-flagged copy of a plugin whose original changed since (a mod or game
  update). The game loads the old copy. Hide it, then clean or flag the new
  plugin again.

### The mod list

- "Order rules of the collection <name> that no longer hold" appears when the
  mods of an installed collection no longer sit in the order its rules ask for.
  Restore collection order moves only that collection's mods back, and Ctrl+Z
  undoes it.
- "<mod> requires <mod>, which is not installed" (or "which is disabled") comes
  from the Nexus requirements that Check for updates reads. Open on Nexus goes
  to the required mod's page, Download main file fetches it on a premium
  account, and Ignore hides a requirement that another mod already meets. A DLC requirement is
  advice only: Eidos looks for a plugin with the DLC's name.
- "fontconfig.txt names a font library that is missing" (Skyrim engines) names
  the fonts that no layer and no archive holds, and the mod whose
  `Interface/fontconfig.txt` wins. Menu text can show as boxes until the font
  is installed or the line removed.
- "BA2 archive stores audio compressed" (Fallout 4 and Fallout 4 VR) counts the
  `.wav` and `.xwm` files a mod's BA2 stores compressed. The game can crash when
  it plays them. Repack the archive with audio uncompressed.

### The script extender and game updates

- "<loader> is not installed" appears when script-extender plugins are installed
  (in `SKSE/Plugins`, `F4SE/Plugins` and so on) but the loader is in no enabled
  mod and not in the game folder. The game starts without them.
- "The script extender does not match game version <version>" appears when the
  loader is installed but the runtime DLL for the running game version is not,
  for example `skse64_1_6_1170.dll`. The loader stops with an error. This
  usually follows a game update. This card and the one above have an Open the
  download page button for the extender's site and an Install the downloaded
  archive button, which puts the loader in the mod's `Root/` folder.
- "<game> changed since the last clean launch" appears when the game executable,
  or for a Steam copy its build id, changed since the last launch through Eidos
  that exited without an error. A Steam update that leaves the executable as it
  was does not raise it. Script-extender plugins built for the old version may
  stop loading, and the cards below it name the ones that declare it. To keep
  the game version a list was built on, freeze it into the instance with
  Freeze this version on the Game files card, or with
  `eidos stock-game <instance> freeze --yes` after `freeze` alone has shown
  the size: later store updates change the store install, not the copy.
- "<mod> replaces <n> <extender> script(s)" appears when a mod with a higher
  priority than the script extender has loose `.pex` copies of the extender's
  own scripts. An old copy breaks the functions it declares, and the game shows
  no error. Move the extender below that mod (the card's Move button does it
  when both are mods), or hide those files in the mod. When the extender's
  scripts sit loose in the game's `Data`, install the extender as a mod so it
  can be ordered. Archives are not checked, only loose files.

### Tool output

- "Overwrite/Root holds meshes" (or textures) means a tool whose data path
  pointed at the game folder wrote those folders there, where the game never
  reads them. Correct the tool's path, delete the folders and run the tool
  again. The Open folder button opens `Overwrite/Root`.
- "BodySlide's GameDataPath is not the game's Data folder" (or
  `OutputDataPath`): see
  [Tools](tools.md#a-tools-own-settings-are-still-its-own).

### A crash in the last session

"The game crashed in the last session" appears when a crash logger wrote a log
during the last launch. The card names the exception and the DLLs and plugins in
the log that your mods or the Overwrite install. Treat those as leads: a mod
named in a crash log is often where the crash showed, not its cause. When the
log names no file from your mods, the card says so. Open the log opens it, and
Copy the log (home path hidden) copies it with your home folder hidden.

### The window size

For Skyrim SE, Enderal SE and Fallout 4, a Display card shows the window size
and mode the profile starts the game with, and says "No window size set for
this profile" when neither the profile's Prefs INI nor the prefix's sets one.
Apply writes the size and mode into the profile's copy, and into a copy of SSE
Display Tweaks' INI in the Overwrite when that mod sets the window
([usage.md](usage.md#diagnostics)).

## Which Proton runs the game, and a broken prefix

The Diagnostics tab has a Proton card: the build Eidos and Steam use for the
game, Steam's own setting, and the build that last ran the prefix. When a newer
Proton than the selected one last set up the prefix, the card becomes advice,
because an older Proton on a newer prefix can reset its Wine files or fail to
start. Pick a build at least as new under the game's Properties, Compatibility
in Steam, or rebuild the prefix.

The other Proton and prefix cards:

- "Proton comes from the Flatpak Steam install": that Proton keeps its runtime
  and steamclient libraries inside the Flatpak sandbox, so running it from the
  host may fail. Eidos does not start it through `flatpak run`, which would put
  the game in the sandbox, outside the merged view. Install a Proton in
  `~/.steam/root/compatibilitytools.d/` and select it for the game.
- "Eidos cannot tell which Proton runs this game": Steam's `config.vdf` names no
  compatibility tool for it, and the prefix records no build that is installed.
  Tools do not start until one is chosen in Steam (the game's Properties,
  Compatibility).
- "<game> has no Proton prefix yet": launch the game once through Steam, then
  press F5. If Steam does not create the prefix, check that the game's folder
  under `steamapps/compatdata` is writable; an empty one owned by root blocks it
  without a message.
- "Windows tools cannot find <game>": the prefix's record of the install path is
  wrong or missing, and xEdit, DynDOLOD, Wrye Bash and the Creation Kit read it.
  Eidos writes the right path the next time you run a tool.
- "The prefix's S: drive points outside the Steam library": `S:` matches
  neither the library root, where Eidos points it for a tool, nor
  `<library>/steamapps`, where Steam points it for the game. The library has
  probably moved, or the prefix came from another machine. Every `S:`-relative
  path stored in the prefix resolves nowhere until the next launch through
  Eidos or Steam points it again.
- "<store> installation: started through its store" is the card of a GOG or
  Epic copy. Run asks Heroic or Legendary to start the game when Eidos is the
  game's wrapper there, and tools run with the Wine or Proton build Heroic
  records for the copy.

The File menu runs winecfg, regedit, the Wine file explorer and the uninstaller
in the game's prefix, with the Proton that runs the game's tools
([tools.md](tools.md#wines-own-programs-in-the-games-prefix)). Their rows stay
greyed out until the prefix has those programs.

The Rebuild prefix button on the Proton card is for a prefix that no longer
works. Press it twice. Eidos deletes nothing: it renames
`steamapps/compatdata/<appid>` to `<appid>.eidos-backup-<date>-<time>` in the
same folder, and Steam creates a new prefix the next time you start the game
from Steam. It refuses while a game, a tool or any Wine program uses the
prefix. Until the new prefix exists, Run in the window cannot hand the game to
Steam and shows the launch option instead, so press Play in Steam. Saves and
settings kept only in the old prefix stay in the renamed folder: copy back what
you need, and delete that folder yourself when you no longer want it.

## What to attach to a bug report

- Copy report, on the Diagnostics tab, copies the Eidos version, the game, the
  Proton build, whether Steam is the Flatpak, the passthrough state, the
  session type, the distribution, the kernel, `vm.max_map_count`, the open
  files limit, whether `/dev/fuse` and `fusermount3` are there, the names of
  the profile's launch variables and wrapper programs, and every Health card.
  Your home folder shows as `~` and your login name as `<user>`.
  `eidos check <instance>` prints the same header in a terminal, and `--json`
  gives it as JSON.
- The run's log, `<instance>/logs/run-*.log`, starts with the exact command
  Eidos ran. File, Eidos logs opens the folder of Eidos's own logs,
  `~/.local/share/Colony/Eidos/logs`.
- Copy the log (home path hidden), on the crash card, copies the crash log.
- Save tree, on the Data tab, saves the merged `Data` tree with the source of
  each file.

## Known issues

### DLSS or frame generation silently does nothing

Three separate causes, each without any error message: NVAPI not enabled in the
launch options, exclusive fullscreen, or a stale Reflex FPS cap. The whole
checklist lives in [graphics.md](graphics.md).

### Run does nothing with the Flatpak Steam

In a window no store opened, Run asks Steam to start the game by running
`steam -applaunch <appid>`. The Flatpak Steam puts no `steam` on the host's
`PATH`, so the status line says "Could not start Steam" with the error. Press
Play in Steam instead.

### A save from another device does not show up

A save that reaches the prefix without going through Eidos, such as a Steam
Cloud download from a Steam Deck, stays hidden while the profile's saves are
mounted. Nothing deletes it. The Saves tab counts these saves under its heading,
and Import copies them into the profile without replacing a profile save
([usage.md](usage.md#saves-belong-to-the-profile)).

### A mod that spells one directory two ways lost everything under the second

Fixed. ext4 keeps `meshes/` and `Meshes/` apart; the merged view must not, and
real mods ship both - XP32 Maximum Skeleton has its animations and its FNIS
behaviour file under the capitalised one, its `character assets` under the other.

The resolver took the exact-case match for each path component and committed to
it: it entered `meshes/`, failed to find the rest of the path there, and
abandoned the WHOLE LAYER. Every file under the other spelling was invisible to
the game - no error, no log, nothing in any diagnostic. On a real 50-layer
instance that was 74 files.

A component that matches is now a candidate, not a decision; exact case is still
tried first, and only when the remainder fails underneath it does the scan look
for fold-equal siblings. Listings had the same fault a directory higher and now
read every fold-equal directory per layer.

The path index never had this bug, because it walks every directory it finds.
So for a while the index returned files the fallback could not, although the
fallback is the answer meant to be never wrong.

### DynDOLOD's LODGen dies leaving an empty log

Fixed by `dotnet10`; see [tools.md](tools.md). Every world's
`LODGen_SSE_<world>_log.txt` holds a version banner, a `.NET Version:` line and
nothing else, and a dialog says only "failed to generate object LOD for one or
more worlds". The cause is Wine's Mono answering for .NET Framework, and no
amount of installing .NET Framework fixes it - Proton replaces `mscoree.dll`
with a symlink into its own tree on every prefix update. The release bundle
ships the .NET 10 runtime; other installs download it once.

### Wine could not tell that the mount folds case

Fixed.

Wine's `get_dir_case_sensitivity` first asks the directory through
`FS_IOC_GETFLAGS`, then sniffs for the marker CIOPFS leaves in the directories
it serves. With neither, Wine assumes case-SENSITIVE, and every lookup whose
spelling does not match byte-for-byte falls back to reading the WHOLE directory
to find a case-insensitive match. Bethesda games ask for
`data/ccbgssse001-fish.bsa` while the file is `ccBGSSSE001-Fish.bsa`, so it
fired on nearly every asset: 4471 marker probes and 2236 full directory re-reads
in eight seconds, and 195796 enumerations of `Data` in ninety. Skyrim SE never
reached its main menu - it sat at 240 MB resident while the daemon burned 92% of
a core.

Eidos had folded case in `resolve_read` from the start. The whole cost was never
saying so. `lookup` now answers `.ciopfs`; `readdir` still does not list it.
`ioctl` also answers `FS_IOC_GETFLAGS` on directories with the casefold flag,
which current Wine asks before it looks for the marker.

Two things made it fatal rather than merely slow. The cost scales with directory
size, so installing the Anniversary content (`Data` from 37 files to 177) tipped
it over. And `opendir` eagerly built the merged listing, which is pure waste when
Wine opens a directory only to `stat` that marker inside it - the snapshot is
taken on the first `readdir` now.

After: the main menu, 2.1 GB resident, daemon at 0% CPU.

`EIDOS_FUSE_TRACE=opendir` is what found it, and ships. The op counters say how
many; 195796 enumerations of one directory is invisible in a total.

### The game rewrote `plugins.txt` empty

Very likely the same thing as the case folding above - a `Data` it could not
enumerate in any reasonable time, so it concluded there was nothing there and
saved that. Not proven, and worth re-checking. Either way the capture guard (a
capture that clears the active set entirely is refused at any size) means it can
no longer damage the profile.

### `FOPEN_KEEP_CACHE` is off

Fixed. It crashed Skyrim SE on a null dereference seconds after the main menu,
deterministically, with zero mods installed; the other three kernel-side caches
were bisected out individually and only this one mattered. That bisect ran with
`FUSE_PASSTHROUGH` on, and the kernel fails a passthrough open that carries
`FOPEN_KEEP_CACHE` with EIO, so with the flag on every passthrough open failed.
Those failed opens may be what crashed the game, and the flag on its own, with
passthrough off, has never been tested. Passthrough replies no longer carry it.
Losing it was measured as free at the time, but with passthrough on the daemon
serves *zero* reads (`EIDOS_FUSE_STATS` reported `read 0` for a full load), so
with passthrough off by default the real cost is unmeasured. Re-enable with
`EIDOS_FUSE_KEEP_CACHE=1` to test it on its own.

### FUSE passthrough stops the game loading any mod content

The daemon now avoids the most likely cause (below), but that has not been
re-measured in a game, so passthrough stays off by default;
`EIDOS_FUSE_PASSTHROUGH=1` turns it on. Before the change, with passthrough on,
Skyrim SE failed to open 152 of its own files with `STATUS_ACCESS_VIOLATION`,
so no mod content loaded, silently. The measurement and why the daemon's own
logs showed a clean run are under "Why passthrough is off by default" above.

The kernel binds one backing file per inode (`fs/fuse/iomode.c`). An open that
brings a second backing id for an inode that already has one fails with EIO, and
so does a normal open (without `FOPEN_PASSTHROUGH`) of an inode that has a
backing. A passthrough open that carries `FOPEN_KEEP_CACHE` fails with EIO too.
The daemon used to register a new backing id on every open, so the second open
of a file the game already held open failed. Wine reports EIO as
`STATUS_ACCESS_VIOLATION`, the error behind the 152 failed opens. This rule is
read from the kernel source and matches those 152 failures, which makes it the
most likely cause.

The daemon now registers one backing per inode, shared by all its passthrough
opens, and never sets `FOPEN_KEEP_CACHE` on a passthrough open. Passthrough
covers only read-only opens of files from the game and mod layers that are 1 MiB
or larger or end in `.dll`, `.exe` or `.asi`. Once a file has a backing, every
read-only open of it uses that backing, and while it has normal opens, new
readers get normal opens too. A file in the Overwrite never goes through
passthrough, and a mount that projects plugin timestamps does not use it at all.
A writer that opens a file while passthrough readers hold it gets ETXTBSY (Wine:
`STATUS_SHARING_VIOLATION`) before any copy-up, so an `O_TRUNC` open cannot
leave an empty Overwrite copy that hides the mod's file.

### The Mod Manager Download button does nothing

Preferences, Nexus, Downloads shows which program receives `nxm://` links. A
Health card appears when that program is not Eidos, is missing, or when none is
set. The Register button writes `eidos-nxm.desktop` and makes it the default
handler for `x-scheme-handler/nxm`. From a terminal, `eidos nxm --register` does
the same.

If the status already names Eidos and the button still does nothing, the
browser may be using a choice it made earlier. Clear the saved choice for nxm
links in the browser's settings so that it asks again. A Flatpak browser cannot
start host programs itself and hands the link over through the desktop portal,
which reads the handler registered on the host.
