# Eidos architecture

This document records *what* we are building and *why* we made each call, so the
reasoning survives even when the code changes.

## What usvfs actually does

MO2's value is not its UI. It is `usvfs`, which gives a running game a virtual,
merged view of many mod folders stacked over the game's own files. Eidos has to
reproduce four of its properties:

1. Merged view: game data plus N mod layers, in priority order. The
   last-enabled mod wins a file conflict.
2. Per-process scope: only the game, and tools launched through MO2, see the
   merge. The rest of the system sees the pristine game directory.
3. Zero-touch: the real game install is never modified. No copies, no links
   written into it.
4. Copy-on-write: when the game writes (saves, a regenerated `.ini`, an edited
   record), the write must not corrupt the source mod. Reads see the merged
   tree, writes go to a writable *Overwrite* layer.

On Windows `usvfs` does this by injecting a DLL and hooking the NT syscall layer
(`NtCreateFile`, `NtQueryDirectoryFile`, ...). That mechanism exists only on
Windows and cannot be ported.

## Virtualizing underneath Wine

On Windows the game runs directly on the OS, so the only place to intervene is
the Windows syscall layer. On Linux the game does not run bare: it runs inside
Wine or Proton, and Wine exposes a Linux directory to the Windows `.exe` as the
game's drive.

So Eidos never touches the Windows side. It presents the merged view at the
Linux path Wine reads from, and the game sees the merge through Wine. That turns
a reimplementation of an OS-level hooking system into a combination of native
Linux filesystem primitives (FUSE, mount namespaces, bind mounts), the same
ones rootless containers run on.

## Mechanism options considered

| Mechanism | Union | COW | Per-process isolation | Perf | Casing |
|---|---|---|---|---|---|
| OverlayFS (kernel union) | native (ordered lowerdirs) | native (upperdir) | via mount namespace | near-native | does **not** case-fold |
| FUSE (userspace daemon) | we implement | we implement | via namespace | userspace round-trip, *unless passthrough* | we control it exactly |
| bind mounts | manual, messy | no | via namespace | native | no |

### Why not pure OverlayFS

OverlayFS maps cleanly onto the union and copy-on-write model and is fast, and
since Linux 5.11 it can be mounted unprivileged inside a user namespace. As a
complete usvfs replacement it has gaps Eidos cannot close from outside:

- It does not fold case. A game probing `textures/foo.dds` when a mod ships
  `Textures/FOO.DDS` misses the file. This is the classic Linux modding
  failure.
- Eidos would inherit its quirks without controlling them: copy-up edge
  cases, opaque-dir and whiteout handling, rename semantics.
- Deep stacks (a 2000-mod load order) hit the practical limits and lookup cost
  of a long lowerdir list.

Casefolding can be added on top (a casefold-enabled backing filesystem, or a
thin FUSE shim), but then the project maintains a two-layer stack whose
interaction is the hard part.

### Decision: a FUSE union filesystem we own

The engine is a FUSE union filesystem written in Rust. Measured against the
three goals (completeness, long-term stability, speed), the reasons are these.

Completeness. Only an implementation Eidos controls gives exact Windows-style
case-insensitivity, precise Overwrite and write redirection, correct handling of
odd path probes, and no lowerdir limit: mods resolve through an in-memory index,
not a kernel layer list.

Stability. Owning the filesystem logic makes the behaviour more stable here: a
test suite pins it, and it does not shift when kernel OverlayFS semantics
change. One engine is easier to debug than a two-layer stack whose interaction
is the bug. `containers/fuse-overlayfs` made the same choice (the whole overlay
in FUSE), and rootless Podman uses it.

Speed. The usual objection that FUSE is slow was expected to be answered by
FUSE passthrough (merged in Linux 6.9): the daemon resolves which real file
backs a path, and later reads go from the kernel straight to the disk. The
first version stopped the game opening its archives and plugins, most likely
because it registered a new backing file on every open while the kernel accepts
one per inode (see the open risks below). The daemon now avoids that, but the
change has not been re-measured in a game, so passthrough ships off and reads
come from a cached backing fd through `pread`. What carries the load instead is
the metadata path. A game's startup cost is dominated by path probes, not bulk
transfer, and the kernel answers most of those itself: negative dentries, long
entry and attribute TTLs, the `.ciopfs` case-fold marker, and directory opens
the daemon declines. The read path without passthrough has not been measured
against a real heavy load order, and needs to be before any claim about it.

OverlayFS keeps its place as the fast way to prove the thesis
(`scripts/poc-overlay.sh`) and as a possible optimization later, not as the
foundation.

## Target architecture, end to end

1. Staging. Mods are extracted into per-mod directories with their original
   filename casing. Installation validates case-insensitive collisions; the
   runtime union resolves names case-insensitively without renaming source
   files.
2. Launch (`eidos-launch`, called by `eidos play`, `eidos tool run` and
   `eidos shell`). For each run it:
   - unshares a mount namespace (`CLONE_NEWNS`) when the `eidos` binary holds
     CAP_SYS_ADMIN, and otherwise a user namespace plus a mount namespace
     with a one-uid map (`0 <uid> 1`), then makes every mount in it private;
   - bind-mounts the game's real Data directory onto `<instance>/.base`, so
     the daemon can still read those files once the union covers that path.
     The stash is the lowest layer of the Data union;
   - bind-mounts the profile's saves and plugin-list directories over the
     prefix's, for the run only. This is fail-closed: a bind that fails
     refuses the launch and undoes the binds already made, because a run
     without one would write into the prefix and split the playthrough;
   - mounts a second union over the game root first, when one is needed, so
     the Data union then mounts inside it. The pristine root is bind-mounted
     onto `<instance>/.base-root` as that union's lowest layer, each enabled
     mod's `Root/` sits above it, and root writes land in `overwrite/Root`.
     The root union mounts when there are root layers, when `overwrite/Root`
     is not empty, when the game keeps its INIs and plugin list in the game
     folder (Morrowind, or Oblivion with `bUseMyGamesDirectory=0`; the
     profile's `runtime-root` folder is then the root upper, and
     `overwrite/Root` is read-only below it with its deletion and opaque
     markers still hiding lower files), when a profile redirect targets a
     directory inside the game root, when the instance runs a game copy, or
     during a test run;
   - for an instance that runs a game copy (a Wabbajack list's `Stock Game`,
     or the `Stock Game` that `eidos stock-game <instance> freeze --yes`
     makes), binds the copy onto `.base-root` and the copy's Data onto
     `.base` in place of the store install. The root union still mounts on
     the store install path, so the store starts the same path it always
     does and the copy's files answer it;
   - mounts the Data union at the game's Data directory, with
     `layers = modN..mod1, .base` and the Overwrite as the writable layer on
     top. Each mod layer's top-level `meta.ini` and `readme.txt`, and its
     `fomod/` and `Root/` folders, stay out of the Data view
     (`eidos_core::is_mod_metadata`), as in MO2;
   - runs Proton, the game, the tool or the shell as a child inside the
     namespace, from the game root for the game (tools pick their own
     directory), and waits for every descendant, since it is their subreaper.
     The profile's `profiles/<name>/launch-env.ini` adds `env=NAME=value`
     variables after Eidos's own and before a tool's `env=` lines (a profile
     `WINEDLLOVERRIDES` is appended to Eidos's value instead of replacing it),
     and its `wrapper=` lines go around the command, the first one outermost.
     `eidos shell` gets the variables and no wrapper;
   - unmounts Data before the root union, then the binds. The game dir was
     never touched, and new or modified files sit in the Overwrite.
3. Engine (`eidos-fuse` + `eidos-core`). `eidos-core` is the pure resolver,
   unit-tested. `eidos-fuse` binds it to the kernel through the `fuser` crate;
   kernel passthrough is opt-in and off by default (see below).
4. Everything else (load order, conflict display, FOMOD, Nexus integration) is
   application code in the library crates listed below.

## Crate map

The workspace is 22 crates under `crates/`, plus one C++ helper,
`crates/eidos-nif-preview`, built with CMake. They build into a few processes,
and the process boundaries decide where a privilege, a crash or untrusted input
can reach:

```
Steam, Heroic or Legendary (eidos-gui %command%), or the desktop
  |
  +-- eidos-gui          the window. Unprivileged, never mounts anything.
  |     +-- runs  eidos play | tool | prereqs ...   every mount goes through the CLI
  |     +-- runs  steam -applaunch, xdg-open heroic://launch..., legendary launch
  |     |                                           the hand-off when no store opened it
  |     +-- runs  eidos-nif-preview <model>         one short-lived process per preview
  |     +-- runs  7z, add-on programs, xdg-open
  |
  +-- eidos              the CLI, and the only binary that mounts.
        play / tool / shell:  unshare a namespace in this process, run the FUSE
                              daemon on background threads of this same
                              process, then spawn Proton, the game, the tool
                              or a shell as a child inside it. When the child
                              exits, the namespace and its mounts go too.
```

The optional `cap_sys_admin` capability goes on `eidos` and on nothing else:
the GUI hands every run that mounts to the `eidos` binary beside it, so the
privileged work stays in one file.

`eidos-gui` also links the CLI. Started as `eidos` (a link with that name) or
with the hidden first argument `--eidos-cli`, it runs the CLI instead of a
window. That is what lets one file, the `eidos-linux` release asset Colony
installs, be the whole product: with no `eidos` beside it and none in
`~/.cargo/bin`, the window runs itself with `--eidos-cli` for every launch.
Before a window opens, the binary clears every capability it holds, so a
capability set on that file never reaches the window; SECURITY.md still says
never to set one there. Everything the GUI does besides launching (installs,
Nexus, LOOT, conflicts, previews) runs in its own process through the library
crates below.

### Programs

| Crate | Builds | Role |
|---|---|---|
| `eidos` | `eidos`, and a library `eidos-gui` links | The CLI: `games`, `init`, `play`, `shell`, `install`, `collection`, `pack`/`unpack`, `tool`, `prereqs`, `export`, `sort`, `nexus`, `nxm`, `import`, `check`, `sessions`, `stock-game`. The store launch path (`eidos play <id> -- %command%`), the hand-off to a store, the test-run mounts, and in `check/` the Health checks the window runs too. |
| `eidos-gui` | `eidos-gui` | The iced window, MO2-style. Finds `eidos` beside itself first, then in `~/.cargo/bin`, then runs the CLI it links itself (`--eidos-cli`). Answers `--version`. |
| `eidos-fuse` | library, plus the dev CLI `eidos-fuse` | The read-write union daemon. Runs inside `eidos`; the binary mounts a union by hand for debugging. |
| `eidos-launch` | library, plus the dev CLI `eidos-launch` | Namespace and mount orchestration, and stopping a run. Runs inside `eidos`; the binary runs any command through a union given on the command line. |
| `eidos-nif-preview` | `eidos-nif-preview` (CMake, not a workspace member) | Bounded static NIF parser on vendored nifly. A separate process so a hostile mesh costs the preview, not the window. |

The two dev CLIs are optional in an installation; `eidos` and `eidos-gui` are
the product.

### Libraries

| Crate | Owns |
|---|---|
| `eidos-core` | The layer-resolution engine, kept apart from any FUSE binding: which real file serves a virtual path, where a write lands in Overwrite, whiteouts, case folding, the layer index and the Overwrite index. |
| `eidos-paths` | Where Eidos keeps its files, on `colony_ui::paths`, and the migration onto that layout ([paths.md](paths.md)). |
| `eidos-log` | Session logs: levels, rotation, home-path redaction. |
| `eidos-ini` | Shared INI primitives: newline, section and key parsing, format-preserving edits. |
| `eidos-gamedef` | The declarative per-game descriptor on MO2's `IPluginGame` schema, one embedded TOML file per built-in game. |
| `eidos-games` | The supported-game catalog, install detection for Steam, Heroic (GOG, Epic) and Legendary, and the stores' launch settings (Steam launch options, Heroic wrappers, Legendary's `wrapper =` line). |
| `eidos-plugins` | ESP/ESM/ESL load order: MO2-parity ordering, FormID indexes, `plugins.txt`. |
| `eidos-loot` | LOOT sorting through libloot, and the masterlist fetch and cache. |
| `eidos-instance` | Instances (global and portable), profiles and their restore points and `launch-env.ini`, per-mod `meta.ini`, the manifest (with the game copy it runs), global settings, `nexus.ini`, session records, `logs/launch.ini`, update reports, test runs and the bisect search. |
| `eidos-conflicts` | Per-file and archive-member conflicts: winners, losers, per-mod state. |
| `eidos-sevenzip` | The 7-Zip process seam: find the binary, drive it, read its progress. |
| `eidos-install` | The mod installers (Simple, Root/Data, BAIN, OMOD, the manual picker, add-on installers), staged publication, and the report of what a Replace changed. |
| `eidos-fomod` | The FOMOD scripted-installer parser, the condition/flag engine, and the replay of recorded answers. |
| `eidos-gamefeatures` | Per-game launch work: archive invalidation, per-profile INIs and saves, script-extender plugin checks, crash logs and save files, prefix DLLs, tool prerequisites (Microsoft's own installers for `vcrun2022` and .NET 8, `winetricks` for the other verbs) and shared .NET runtimes. |
| `eidos-nexus` | Nexus Mods: OAuth sign-in, the v1 API, `nxm://` downloads, update checks along each file's update chain, endorsements and tracked mods, a mod's page text, and GraphQL for collections, collection search and requirements. Also the daily GitHub check for a newer Eidos. |
| `eidos-collections` | Nexus collections: read a collection's manifest and install it as its author built it. |
| `eidos-transfer` | `eidos pack`/`unpack`: one instance into one `.eidos` file and back. |
| `eidos-addons` | User extensions: TOML manifests whose programs run out of process ([guide/extensions.md](../guide/extensions.md)). |

`eidos-core`, `eidos-paths`, `eidos-ini`, `eidos-sevenzip`, `eidos-conflicts`
and `eidos-fomod` depend on no other Eidos crate. The two programs depend on
nearly all of them, and no library depends on a program.

A pull request that adds, removes or splits a crate updates this map in the
same pull request.

## Starting the game through its store

Steam, Heroic and Legendary each start the game with their own logic, so Eidos
goes in as a wrapper around the store's command. It is the same integration
point RadTux and the Furglitch installer use. Eidos never writes a store's
config: Health and `eidos tool run` print the setting to enter
(`crates/eidos/src/check/launch_option.rs`).

- Steam: the game's launch option. `<path of eidos-gui or eidos-linux>
  %command%` opens the window with the game's command. For the dedicated
  `eidos` binary the line is `eidos play <game-id> -- %command%`, which mounts
  and runs the command with no window. `eidos %command%` does not work: the
  CLI reads the game's path as a verb and prints its usage.
- Heroic: a wrapper in the game's settings, with the Eidos executable and,
  for the `eidos` binary, the arguments `play <game-id> --`.
- Legendary, the bare CLI: `wrapper = <the same line without %command%>`
  under `[<app>]` in `~/.config/legendary/config.ini`.

The merged view goes at the Linux directory Wine exposes as the game's install
dir (for Steam, under `steamapps/common/<game>`). The wrapper is set once per
game, not once per mod.

Eidos never starts the game through Proton itself. When Run starts the game
from a window no store opened (the window started by hand, or from the desktop),
the window mounts nothing and hands the game to the store that owns the copy.
`eidos tool <instance> run <entry>` does the same for the script-extender
loader, the launcher and the game binary when no store started the process,
and so does a per-profile desktop shortcut, which runs that command with
`--profile <name>`:

| Copy | What Eidos spawns |
|---|---|
| Steam | `steam -applaunch <appid>` |
| Heroic, GOG | `xdg-open heroic://launch?appName=<app>&runner=gog` |
| Heroic, Epic | `xdg-open heroic://launch?appName=<app>&runner=legendary` |
| Legendary | `legendary launch <app>` |

It does this only when that store's setting already starts Eidos, and for
Steam only when the game's compatdata exists, so the copy has been started from
Steam before. Otherwise nothing starts: the window shows the value to set and
where, and `eidos tool run` prints it and exits 1. A script-extender
loader that is not beside the game's executable in the merged root stops the
hand-off before the store is asked. A GOG copy outside Heroic, or an app id
that is not a plain id, gets a message to start the game from its store.

A process a store started is recognised by `SteamGameId`, `SteamAppId`,
`HEROIC_APP_NAME`, `HEROIC_APP_RUNNER` or `EIDOS_STORE_STARTED` in its
environment, and inside it a game entry runs through the view. The bare
Legendary CLI exports none of these, so a window it opened sets
`EIDOS_STORE_STARTED=1` on the `eidos tool` child.

With the setting "Start the game at once when a store opens Eidos" (off by
default), a window a store opened checks Health first and starts the game
without showing itself. It shows itself instead when Health lists a problem or
the profile has no window size yet. `EIDOS_WINDOW=1` before the command in the
launch option always opens the window.

Tools such as xEdit and FNIS see the same merged view as the game, as MO2 runs
tools through its VFS. No process joins another run's namespace: each
`eidos tool run` and each `eidos shell` unshares a fresh namespace, mounts its
own union of the same layers, and runs the tool inside it.
`eidos shell <instance> [-- <cmd>]` runs `$SHELL` (else `/bin/sh`), or the
command, from the game root, with writes landing in the Overwrite; leaving the
shell unmounts the view.

A tool runs with the Steam copy's Proton and prefix, with the Wine or Proton
and the prefix that Heroic's settings name for a Heroic copy, or as a native
program with `runner=native` in `tools.ini`. A tool's `env=NAME=value` lines
add variables, and `--output-mod <mod>` captures what it wrote into that mod.
`eidos tool <instance> run --exe <path>` runs any executable once, which is
what the Data tab's run action uses. The window's File menu runs Wine's
`winecfg`, `regedit`, `explorer` and `uninstaller` in the game's prefix the
same way, and Rebuild moves a broken Steam prefix aside to
`compatdata/<appid>.eidos-backup-<stamp>` after a confirmation, so Steam makes
a new one at the next launch.

### Provisioning the prefix (DLLs + tool prerequisites)

A modded Bethesda game and its tools need some Windows runtime pieces in the
Proton prefix that no Proton build supplies. `eidos-gamefeatures` provides them
in three tiers:

- Bundled native DLLs (`native_dll`): Microsoft's native `d3dcompiler_47`
  (graphics mods import it for runtime HLSL compilation; Wine's builtin is a
  broken stub) and the DirectX helpers tools use (`d3dx9_43`, `d3dx11_43`,
  `d3dcompiler_43`). They are detected by a PE-import scan or declared per
  tool, and deployed into the prefix's `system32` or `syswow64` by
  architecture. Proton's builtin symlink is unlinked first so the shared
  install is never written through, a displaced file is backed up, a second
  deploy changes nothing, and `WINEDLLOVERRIDES=...=n,b` forces the native
  copy. The eight DLLs (an x86_64 and an i386 build of each) are embedded in
  the binary gzip-compressed and unpacked one at a time when deployed.
- Installer verbs (`prereqs`): the .NET and vcrun runtimes tools like
  Synthesis need cannot be file-copied. For `vcrun2022`, `dotnet8` and
  `dotnetdesktop8`, Eidos runs Microsoft's own installers in the prefix
  (`VC_redist.x86.exe` then `VC_redist.x64.exe`, or the .NET 8.0.12 Windows
  Desktop Runtime, which carries `Microsoft.NETCore.App` too). It takes the
  copy the release bundle ships when one matches its pinned SHA-256, else a
  download from Microsoft kept only when it matches. A prefix that Proton has
  not created yet (no `user.reg`) is refused before anything is downloaded.
  After `vcrun2022`, a C++ runtime left split against the Proton build's
  `default_pfx` counts as a failure. The other verbs (`dotnet48`, `dotnet6`, `dotnet7`,
  `dotnetdesktop6`, `xact`, `xact_x64`) still run the system `winetricks`
  pointed at Proton's own `wine` and the game prefix (the NaK approach, which
  bypasses Steam's pressure-vessel and the protontricks and Proton-GE
  mismatch). Because both paths may download from Microsoft, they run only on
  the user-consented `eidos prereqs --install`. What is installed is read
  from the prefix's own `winetricks.log` and from `<instance>/prereqs.done`,
  which is bound to one prefix by a hash of its path, device and inode, so a
  re-run installs nothing twice.
- Self-contained runtimes (`runtime`): `dotnet10`, the .NET 10.0.10 runtime
  that DynDOLOD's LODGen and Pandora need, and `dotnetsdk10`, the 10.0.403 SDK
  that Synthesis uses to compile its patchers. `eidos prereqs --install` takes
  the `dotnet10` zip from the release bundle when it is there, downloads
  anything else once, refuses a file unless its SHA-256 matches the one pinned
  in the source, and unpacks it into
  `~/.local/share/Colony/Eidos/runtimes/<verb>-<version>/`, shared by every
  instance. Nothing goes into the prefix: the tool finds the runtime through
  `DOTNET_ROOT`, and the SDK also through `WINEPATH`.

A known tool gets its verbs from its title when Eidos creates its entry
(`eidos_instance::default_prereqs`): Synthesis `dotnetsdk10` and `vcrun2022`,
Pandora `dotnet10`, FNIS `dotnet48`, BodySlide and Outfit Studio `d3dx9_43`
and `d3dcompiler_47`, DynDOLOD, TexGen and xLODGen `d3dcompiler_47`,
`d3dx9_43`, `d3dx11_43` and `dotnet10`, Cathedral Assets Optimizer
`vcrun2022`, `d3dcompiler_47` and `d3dx11_43`, Nemesis and LOOT `vcrun2022`.
A `prereqs=` line in `tools.ini` wins.

Tools run in the game's own prefix, so one provisioning pass covers the game
and every tool.

## Session records and stopping a run

Every run that goes through `run_through_view` (`eidos play`, `eidos tool run`
and `eidos shell`) writes a session record,
`<instance>/logs/sessions/<started>-<pid>.json`, before the launch and again
when the run ends. Session records are separate from the session logs that
`eidos-log` writes. A record holds the Eidos version, the profile, the tool
(none for a game run), the game build read from the winning executable, the
enabled managed mods in mod-list order with their version and Nexus
(mod, file) ids, the plugin states, and a CRC32 of each profile INI. The
second write adds the end time, the exit code and the saves written during the
run. The newest 50 records are kept. `logs/` is specific to the machine, so
`eidos pack` leaves it out. A record that cannot be written only logs a
warning; the launch goes on.

`eidos sessions <instance>` lists the records.
`eidos sessions <instance> diff [<id>]` compares one record with the active
profile as it is now. Without an id it takes the last good run: the newest
game run that exited 0. Tool and shell runs are never picked, and neither is a
test run: it ran a working copy, so its record is marked `test` and names the
real profile.

`eidos` turns SIGTERM and SIGINT into a stop request. Nothing more is started,
the whole process tree of the run gets SIGTERM and, 10 seconds later, SIGKILL,
and `eidos` keeps waiting until every process is gone. The post-run captures
still run and the union unmounts last. The window's Stop sends that SIGTERM,
and Ctrl+C in a terminal stops a run the same way, except in an interactive
`eidos shell`, where Ctrl+C stays with the shell. Steam's wrapper chain can
turn the SIGTERM into exit 0 or 1, and a wrapper that handles SIGTERM itself
can return any code, so every stopped run is recorded as 143, whatever its
exit code (`sessions::recorded_exit`). It never becomes the last good run or a
clean launch, and a stopped test run or bisect step pre-selects no answer.

## Test runs and bisect

A test run is one game run whose writes are held apart until the user reviews
them. The window writes the request to `<instance>/test-run/test-run.json`:
the profile, the game, and the mods to run with (the profile's own set, none,
or a chosen list plus the mods that provide their masters). The next
`eidos play` for that game and profile begins it on a working copy of the
profile, `test-run/profile`. A run for another profile or game stays a normal
run and says so in the log.

During a test run every writable path is under `test-run/` (`test_mounts` in
`crates/eidos/src/test_run.rs`):

- the Data union writes to `test-run/overwrite`, with the real Overwrite
  read-only below it;
- the root union writes to the working profile's `runtime-root`, with
  `overwrite/Root` read-only below it;
- one-layer unions under `test-run/views/` put an upper over the game's
  `Documents/My Games/<game>` folder (`test-run/docs`) and over the profile's
  saves, and are bound over the real folders for the run.

A test run never syncs Steam Cloud. When it ends, `eidos play` prints how many
files wait in each group and where. The window lists them in five groups:
saves, INIs, plugins, output (the Data and Root uppers) and logs (the rest of
My Games). Each group is kept or thrown away as a whole. Keeping the saves
never applies a deletion. Keeping the output also deletes the Overwrite files
the game deleted. After a run with a different mod set the plugin group is not
offered, since its plugin list differs by construction. `eidos pack` leaves
`test-run/` out.

Bisect finds the mod behind a reproducible problem with a series of test runs.
Each step turns on half of the remaining suspects plus the mods that provide
their masters (`eidos_instance::with_masters`), with the always-on (pinned)
mods on in every step, and the user answers whether the problem happened.
"Yes, it happened" keeps the tested mods as suspects, "No, it did not" clears
them, and "Not sure" rotates the split for the next step. An answer throws the
step's writes away. The search is kept in `<instance>/bisect.json` between
steps, so it survives the window closing, and `eidos pack` leaves it out. When
the search ends, its log is kept as `logs/bisect-<time>.txt`.

## Game copies

An instance can run its game from a folder inside the instance instead of the
store install: a Wabbajack list's `Stock Game` (or `Game Root`), or a frozen
copy. `base_game=<folder>` in `eidos-instance.ini` names that folder, and the
launch binds it under the root union as described above, while the store still
starts the game. Health, the plugin list, LOOT, the Data views, tool
detection, installers and collections read the game's files from the copy.
A copy must hold the game's executable, and a game that keeps its plugin list
in the game folder (Morrowind, or Oblivion with `bUseMyGamesDirectory=0`)
cannot run from one. A broken copy refuses the launch.

`eidos stock-game <instance> [use <folder> | store | freeze [--yes] | delete --yes]`
shows the game files an instance runs, switches between a copy and the store
install, or freezes the game version. `freeze` prints the plan; `freeze --yes`
copies the store install into `Stock Game.eidos-tmp`, with `FICLONE` reflinks
when a probe clone into the instance folder works and a byte copy otherwise.
The probe decides, not `st_dev`, because two btrfs subvolumes report different
devices and still clone. The freeze then renames the copy to `Stock Game` and
records `base_game` and `frozen_copy`. It stops when Steam changes the game
build while it copies, and it refuses an instance that sits inside the game
folder.
`delete --yes` removes only the folder `frozen_copy` names (always
`Stock Game`) and an unfinished `Stock Game.eidos-tmp`, never a list's own
copy, and refuses while the instance still runs the frozen copy.

## Open risks

The read-write daemon is covered by a real-mount integration suite (merge,
case-insensitive reads, copy-up, deletes and whiteouts, rename, readdir, rmdir
semantics, symlinks, a multi-megabyte chunked read, and a writable shared-mmap
round-trip) plus resolver and inode-table unit tests. Writable `MAP_SHARED` mmap
round-trips correctly, with `setattr` guarded to refuse a path that no longer
resolves, so the kernel's attribute flush on an unlinked inode cannot resurrect
a deleted file. `writeback_cache` is off on purpose. It makes writable shared
mmap slightly cheaper but breaks loading Windows DLLs from the mount: the loader
dirties `MAP_PRIVATE` copy-on-write image pages and the kernel mishandles those
over a FUSE backing, and a modded game has to load its SKSE plugin DLLs.

FUSE passthrough (the kernel serving the real backing file itself, which needs
`setcap cap_sys_admin+ep` and a bare mount namespace) is negotiated only on
request and is off by default. In an A/B run on Skyrim SE 1.6.1170 with kernel
7.1.4 and an earlier daemon, turning it on made the game fail to open 152 of
its own files (75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`), all with
`STATUS_ACCESS_VIOLATION`, against 0 with it off, so no mod content loaded at
all. The kernel produced the failure after the daemon replied
`opened_passthrough`, so the daemon never saw it: a full failing session under
`EIDOS_FUSE_TRACE=open` logs zero failed opens and zero refused backing files.
The capability also made the launcher keep the initial user namespace, so that
A/B changed two things at once.

The most likely cause is the kernel's rule of one backing file per inode
(`fs/fuse/iomode.c`, read from the kernel source; it matches the 152 failed
opens). An open that brings a second backing id for an inode fails with EIO, as
do a normal open of an inode that has a backing and a passthrough open that
carries `FOPEN_KEEP_CACHE`, and Wine reports EIO as `STATUS_ACCESS_VIOLATION`.
That daemon registered a new backing id on every open. `passthrough.rs` now
avoids those cases: it registers one per inode, shared by all its passthrough
opens, keeps the live opens of an inode all passthrough or all normal, and
masks `FOPEN_KEEP_CACHE` out of passthrough replies. Only read-only opens of
lower-layer files that are 1 MiB or larger or end in `.dll`, `.exe` or `.asi`
go through passthrough. Files in the Overwrite never do, and a mount that
projects plugin timestamps does not negotiate it. A writer that opens a file
while passthrough readers hold it gets ETXTBSY (Wine:
`STATUS_SHARING_VIOLATION`) before any copy-up, so an `O_TRUNC` open cannot
leave an empty Overwrite copy that hides the mod's file. The change has not
been re-measured in a game, so the default stays off: reads are served by the
daemon's own `pread`, and `EIDOS_FUSE_PASSTHROUGH=1` turns passthrough on.
Other write paths are covered too: rename-over, in-place rewrites through
copy-up, and save integrity (`fsync` flushes the backing fd).

A real Skyrim SE with 110 mods ran under Proton through the mount: all ~50
SKSE plugin DLLs loaded and ran (CommonLibSSE-NG included), each writing its
config into the Overwrite layer. Two parity requirements with MO2 and usvfs
came out of it. The game is launched with its working directory at the game
root, because CommonLibSSE-NG opens its address library by a CWD-relative path.
And `readdir` returns entries in case-insensitive alphabetical (NTFS-like)
order, because the Creation Engine's loose-file indexer assumes sorted
enumeration; raw FUSE order crashed it at the main menu. What that run does not
cover:

- That DLL run predates the switch of the passthrough default and was made
  with passthrough on, which that daemon broke for every archive and plugin the
  game held open. It tested DLL image mapping, not a playable load order.
  Rootless, a small load order has since reached gameplay with its plugins,
  archives and Papyrus scripts live and SKSE running, but ~50 SKSE plugin DLLs
  mapping their images without passthrough have not been tested again. If
  relocation-heavy DLLs turn out to need passthrough, it already covers
  `.dll`, `.exe` and `.asi` files at any size.
- Performance has to be checked with a real 1000+ plugin load order and big
  texture packs, not assumed. Reads use a cached backing fd (`pread`), so the
  per-read cost is gone. Path resolution goes through the layer index and the
  Overwrite index described below. Writable mmap is correct in a focused test
  but still worth re-checking under a real game's mmap-heavy load.
- Casing beyond ASCII. v1 folds ASCII case; Windows folds a wider Unicode
  table. This is at parity with usvfs (its tree uses ASCII `_strnicmp`), and a
  documented limitation to close later. Wine's own directory scan does fold
  Unicode case, and it skips that scan in a directory that says it folds case.
  So a directory where any layer holds a name with a non-ASCII letter that has
  a case mapping (`Café.dds`, `Ωmega.esp`) does not say so: `ioctl` answers
  ENOTTY there and `lookup` withholds `.ciopfs`, and Wine finds `Café.dds` when
  asked for `CAFÉ.dds`. The cost is a full Wine scan of that directory for
  every lookup of a name it does not hold.
- Store updates. Proton updates and changes to how Steam, Heroic and Legendary
  pass the launch command can break the launch path, and each needs re-testing.
  The Heroic and Legendary hand-offs are tested on fixture settings files
  only, not with a real launch: whether the window they open finds the game,
  whether SKSE loads when the store starts the binary directly, and Eidos as a
  wrapper inside Flatpak Heroic are all unverified. A Flatpak Steam has no
  `steam` on PATH, so the Steam hand-off fails there with an error.
- Telling Wine the mount folds case (fixed). Wine's
  `get_dir_case_sensitivity` asks the directory through `FS_IOC_GETFLAGS`,
  then looks for the marker CIOPFS leaves in each directory. Eidos answers
  both: `ioctl` returns the casefold flag on directories (4 bytes, ENOTTY for
  everything else) and `lookup` serves `.ciopfs` for older Wine, both except in
  the directories described under casing beyond ASCII. Without either, Wine
  assumes the directory is case-sensitive and answers every mis-cased lookup by
  reading the entire directory to search for a match. For a Bethesda game
  asking for `data/ccbgssse001-fish.bsa` against `ccBGSSSE001-Fish.bsa`, that is
  nearly every asset. Measured before the fix: 4471 marker probes and 2236 full
  directory re-reads in eight seconds, 195796 enumerations of `Data` in ninety,
  the game frozen at 240 MB resident and the daemon at 92% of a core. Eidos had
  folded case in `resolve_read` since the first commit; the whole cost came
  from never telling Wine.
- `opendir` is not a promise to enumerate (fixed). The daemon built the merged
  listing eagerly, and Wine opens a directory only to `stat` that marker inside
  it, so the daemon paid a full multi-layer merge plus an NTFS-collation sort
  220000 times in ninety seconds for listings nobody read. The snapshot then
  moved to the first `readdir`. Since 2026-07-28 the daemon declines `opendir`
  when the kernel advertises `FUSE_NO_OPENDIR_SUPPORT`: it answers ENOSYS once,
  and the kernel stops sending OPENDIR and opens directories without asking.
  `readdir` then resolves the path from the inode and reads the by-path listing
  cache. The listing of an enumeration in progress is pinned by inode until the
  enumeration ends, so offsets stay stable even when the directory changes
  between two chunks. Measured on Skyrim SE before the change: 273214
  directory opens, 94.4% of which never enumerated anything. The per-handle
  snapshot, taken once per handle by its first `readdir`, applies only with
  `EIDOS_FUSE_OPENDIR=1` or on a kernel without that capability.
- The merged listing is memoised by path, as well as per handle, so a
  directory enumerated repeatedly pays for one multi-layer walk and one
  NTFS-collation sort instead of one per enumeration. The same immutability
  that allows the long entry TTL allows this: mod layers do not change for the
  life of the mount, and anything written through the mount goes through a
  handler that drops the affected parent (`dir_changed`; `grep` for it is the
  audit that no mutating handler is missing one). Inodes are not cached with
  the listing. `forget` drops an inode when the kernel releases its last
  reference, and a later `intern` of the same path mints a fresh number, so
  cached entries would hand out inodes the daemon no longer knows. Interning is
  a hashmap hit against real disk I/O, so it is redone per enumeration and
  stays correct.
- The op counters distinguish opening a directory from listing one. At first
  they did not: the counter printed as `readdir` was incremented in `opendir`,
  which made a measured 516301 read as half a million enumerations when almost
  none of them enumerated. Directory opens, enumerations, merges, cache hits
  and `.ciopfs` marker lookups are now separate, plus `probe` for handles closed
  without a single `readdir`. With handles (`EIDOS_FUSE_OPENDIR=1`), `probe` is
  the direct measure of Wine's case-sensitivity probing, since the
  negative-dentry cache absorbs the lookups while every directory open still
  reaches the daemon. By default the kernel stops sending OPENDIR after the
  first one is declined, and the report shows `opendir 1`.

### Caching, and one flag that had to go

Metadata caching is what keeps a Bethesda startup from stalling in the daemon.
`EIDOS_FUSE_NO_CACHE` names the four kernel-side caches: `attr`, `neg`, `keep`
and `dir`. Negative dentries for the paths Wine probes and never finds, and a
long positive entry and attribute TTL (mod layers are immutable for the life of
a mount), are on. `FOPEN_CACHE_DIR` goes on every directory handle the daemon
opens, which by default is none: the daemon declines `opendir` (see the open
risks above), unless `EIDOS_FUSE_OPENDIR=1` is set or the kernel lacks
`FUSE_NO_OPENDIR_SUPPORT`.

`FOPEN_KEEP_CACHE` is off. It was first turned on with this argument: mod files
do not change behind the mount, the layers are immutable, and every write goes
through this daemon, so the kernel can keep its page cache across opens. Both
premises are true, but the conclusion does not follow, because a lower-layer
file does not keep its identity. The first write copies it up, so one virtual
path (and one FUSE inode, since inodes are keyed on the path) ends up backed by
a different file on disk while the kernel still holds pages read from the old
one. Under passthrough the kernel serves those reads without consulting the
daemon, so nothing downstream can notice.

That explanation does not cover everything either, which is why the flag was
turned off instead of narrowed. Restricting it to files outside the overwrite
layer was tried, and Skyrim still crashed, reading files that were never
written to. The measurement that settled it: with all four caches on, the game
dies on a null dereference at a fixed address seconds after the main menu, with
zero mods installed; each cache was then disabled on its own, and only this one
changed the outcome. That bisect ran with passthrough on, and the kernel fails a
passthrough open that carries `FOPEN_KEEP_CACHE` with EIO ("conflicting open
flags"), so with the flag on every passthrough open failed. That fits the
overwrite-layer result, and it means the flag on its own, with passthrough off,
has never been tested. `EIDOS_FUSE_KEEP_CACHE=1` turns it back on for that
test; it then also goes on directory opens. Passthrough replies drop it now
(`passthrough_flags`). The cost of losing it was nil only while passthrough
served every byte in the kernel (`EIDOS_FUSE_STATS` reported `read 0` for a
full load); with passthrough off by default it is unmeasured.

`EIDOS_FUSE_NO_CACHE` takes `1` for all four caches or a list of names
(`attr,neg,keep,dir`). A switch that only answers "is it the caching?" costs a
rebuild per hypothesis; one that names the cache answers "which caching?" in
four launches.

`EIDOS_FUSE_THREADS` sets the number of FUSE event-loop threads. The default is
the machine's CPU count capped at 4, each thread with its own `/dev/fuse` fd
(`clone_fd`); `1` makes the daemon single-threaded, which is the first thing to
try for a suspected concurrency bug. The stats report prints the count, since
an environment variable leaves no trace in the recorded command line.

## Path resolution, and why it was quadratic

Every `lookup`, `getattr` and `open` has to answer one question: which real
file does this virtual path name. The first implementation asked each layer in
turn (`resolve_read` -> `ci_lookup`), walking the path one component at a time.

Case folding is what made that expensive, and it is not optional. Bethesda
games ask for `data/ccbgssse001-fish.bsa` while the file on disk is
`ccBGSSSE001-Fish.bsa`; Windows does not distinguish them and the game was
written for Windows. So `ci_lookup` tries the exact join first and, when that
misses, reads the whole directory looking for a case-insensitive match.

The cost falls on the layers that do not have the file. A layer without the
path misses on its *first* component and pays an enumeration to be sure the
name was not merely spelled differently. A file provided by one mod therefore
cost an enumeration in each of the others, on every resolve. With 27 layers at
median depth 6, one path cost 95 probes and 37 directory reads; a measured
session made 5,608,084 of the latter.

### The index, and why it needs no invalidation

The read-only layers are walked once at `LayerStack::new` into a map from
folded virtual path to real path. A resolve becomes a hash lookup.

The design rests on one property: the read-only layers never change while
mounted. Every mutation in the tree (create, unlink, rename, mkdir, copy-up)
lands in the Overwrite, so the layer index needs no invalidation. That is the
reason this design was chosen over faster ones that needed it.

The Overwrite has had its own index since #13 (2026-08-04), `OwIndex` in
`eidos-core`. It holds folded keys for the Overwrite's entries, whiteouts,
opaque directories, symlinks and names that collide by case. Every mutation of
the Overwrite goes through the stack's own methods, and those keep the index
current: a creation, copy-up, mkdir or link inserts its entries, and a delete,
rename or truncating rewrite drops the index, which is rebuilt on its next use.
The `EIDOS_FUSE_STATS` report counts those builds ("overwrite index built N
time(s)", counter `ow_rebuilds`), the first one included. A name that collides
by case, and any path below a directory symlink, go to the walk.
`EIDOS_NO_OW_INDEX=1` turns the Overwrite index off.

Three rules apply to the layer index:

- All or nothing. An unreadable directory, or a tree too deep or too large,
  and the whole index is discarded and every query walks the layers as before.
  The index answers "no layer provides this" as confidently as it answers
  "layer 7 does", and that is only sound if the walk that built it completed.
  Slow is a cost; a mod file that silently is not there is a corruption. The
  mount prints the reason and the path once at startup
  (`eidos-fuse: layer index off (...)`), because the fallback never fails and
  would otherwise go unnoticed.
- A name that is not UTF-8 is listed, not indexed. 7-Zip extracts ZIP entries
  that lack the UTF-8 flag as raw bytes, so these names do reach the layers.
  The walk cannot resolve them either (queries are UTF-8, and
  `eq_ignore_case` compares raw bytes when one side is not), so leaving them
  out of the map changes no answer. Listings carry them under their lossy
  spelling, deduplicated the same way the walk does.
- Ambiguity defers. Two names in one directory differing only by case cannot
  be represented by a folded key, because `ci_lookup` prefers the exact
  spelling and so the winner depends on the query. Those keys hand the
  question back to the walk.

`EIDOS_NO_INDEX=1` forces the fallback. `examples/index_agrees.rs` compares
both paths over every path of a real instance; `examples/resolve_cost.rs`
reports what one resolve costs in syscalls, which is deterministic where a
duration is not; `examples/index_health.rs` checks whether the index was built
without launching anything and without trusting the mount's own output. That
output exists: a mount whose index was declined prints
`eidos-fuse: layer index off (<reason>); every lookup walks all layers` once,
and every union whose index was built prints
`eidos: <data|root> union index: N layer(s), N entries, N dirs, built in N ms`.
Both go to stderr, which for a run the window started is
`<instance>/logs/run-<stamp>.log`.

### Listings need a different map

`readdir` was the one handler the index did nothing for. A resolve asks *which
layer wins*, which is what the map holds. A listing needs *every* layer's copy
of one directory, merged, so it went on asking each layer separately, and a
listing on a 50-layer stack cost 50 case-folding walks whatever the index knew.

The same invariant that makes the path index safe makes this safe: the layers
cannot change while mounted, so neither can their merge. It is computed once at
build time, in priority order, deduplicated by folded name, with `.mohidden`
entries dropped before they can claim a slot. A lower layer's copy of a file a
hidden one shadowed then becomes the winner, which is what hiding one mod's
stray override is for. Whiteouts stay out of it: they live in the Overwrite,
which changes.

Measured on two real sessions issuing the same 2,063 listings: `readdir` 799 ms
-> 105 ms, directory scans 800,722 -> 464,564. `examples/listing_cost.rs`
reproduces it.

### Two spellings of one directory in a layer

`ci_lookup` used to take the exact-case match and commit to it, which loses
whole layers without reporting anything. A layer may hold two directories
differing only in case (ext4 keeps them apart, the merged view does not), and
real mods do: XP32 Maximum Skeleton ships both `meshes/` and `Meshes/`, with
its animations and its FNIS behaviour file under the capitalised one. The
greedy walk entered `meshes/`, failed to find the rest of the path, and
abandoned the layer. On a real instance that hid 74 files with no error
anywhere.

Exact case is still tried first, because it is the common answer and costs one
`exists`; only when the remainder fails underneath it does the scan look for
fold-equal siblings. Listings had the same fault a directory higher, and
`ci_lookup_all` now returns every fold-equal directory so the merge reads them
all.

The index never had this bug, since it walks every directory it finds. For a
while it returned files that the fallback, the path meant to be always correct,
could not find.

### What is left

The 464,564 directory scans above were measured by 2026-07-31, before the
Overwrite index. They were dominated by the Overwrite, then read live on every
resolve, and by the paths no layer provides. #13 indexed the Overwrite on
2026-08-04. On the real 84-layer Skyrim stack, `resolve_bench` (40,000 resolves
per class) went from 51.7 to 4.0 µs for exact paths, from 50.2 to 1.2 µs for
case-mangled ones and from 26.7 to 0.9 µs for misses, and its 20,000 probes
and 20,000 scans fell to zero. `read` is still where the time goes, and that
is the disk.

## References

- `ModOrganizer2/usvfs`: the semantics we reproduce.
- `containers/fuse-overlayfs`: overlay semantics done entirely in FUSE.
- `fuser` (Rust): the FUSE binding `eidos-fuse` is built on.
- Limo, RadTux, modorganizer2-linux-installer: the existing compromises and
  the Proton integration surface to learn from.
