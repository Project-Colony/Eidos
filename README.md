# Eidos

**A native Linux virtual filesystem for game mods.** Eidos reproduces what Mod
Organizer 2's `usvfs` does on Windows - a clean, per-launch merged view of your
mods over a game's files - but built from native Linux primitives instead of
Windows API hooking, so games run under Proton/Wine without the usual
Wine-shoehorned mod-manager pain.

> Status: **it runs real games**. A 110-mod Skyrim SE load order launches end to
> end under Proton with all ~50 SKSE plugin DLLs loading through the mount. The
> resolver, the read-write FUSE daemon, the private-namespace launch wrapper and
> the MO2-parity manager layer (installer, profiles, plugins, conflicts, LOOT
> sorting, Nexus) are implemented and covered by unit tests and a real-mount
> integration suite. What remains is breadth: more game families proven in-game,
> casing normalization at import time, and packaging. See [Roadmap](#roadmap).

## The problem

MO2 is two things bolted together:

1. A Qt mod-manager UI + plugin system. Cross-platform, ports easily.
2. **`usvfs`** - the feature that actually *matters*: a per-process virtual
   filesystem that merges your mods over the game directory **without ever
   touching the real game files**. It works by injecting a DLL and hooking the
   Windows NT syscall layer.

Part 2 is where every Linux attempt either succeeds or compromises. The
*mechanism* usvfs uses is intrinsically Windows and cannot be ported. The
*property* it delivers - a merged view that leaves the install byte-for-byte
alone - is reachable on Linux with a FUSE filesystem, and Eidos is not the first
project to reach for one. The honest landscape as of mid-2026:

| Tool | Approach | What it costs you |
|---|---|---|
| [MO2 + Wine installers](https://github.com/Furglitch/modorganizer2-linux-installer) | run the real MO2 + usvfs inside the Wine prefix | usvfs under Wine: slow, fragile, and the manager itself is a Windows app |
| [Limo](https://github.com/limo-app/limo) | native C++/Qt, hardlink/symlink deploy | writes links into the game dir, no per-process isolation; no commits since 2025-05-03 |
| [Amethyst Mod Manager](https://github.com/ChrisDKN/Amethyst-Mod-Manager) | native Python, hardlink/symlink deploy across 70+ games | renames the game's `Data/` to `Data_Core/` and builds a synthetic tree in its place. Broad and actively developed, but the exact opposite of zero-touch |
| [RadTux](https://www.nexusmods.com/fallout4/mods/105285) | native daemon + DLL shim, symlinks on launch | still leans on MO2-under-Wine; symlink write-back caveats |
| [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager) | the real MO2 C++/Qt codebase with usvfs swapped for a libfuse3 low-level daemon | a genuine VFS, and the most complete Linux MO2 today. The mount is **global** (its README has you enable `user_allow_other` in `/etc/fuse.conf`), so a daemon crash leaves the real game directory stale-mounted and the project has to carry a cleanup path and a crash hook. No kernel passthrough |
| LMO (Codeberg) | native Rust on `fuse-overlayfs` | young; inherits overlay semantics rather than usvfs semantics |
| **Eidos** | native Rust FUSE union, mounted in a **private user+mount namespace**, with kernel-side metadata caching (optional passthrough) | new; one game family proven in-game so far |

### What is actually exclusive here

Fluorine-Manager got to a real FUSE VFS first, from the MO2 codebase itself, and
it is the more complete mod manager today. Being late to the idea is fine. What
is left that is genuinely ours is not "a FUSE VFS" - it is **where the mount
lives**, and what the kernel does underneath it:

1. **The mount is private to the launched process tree.** Before mounting
   anything, Eidos calls `unshare(CLONE_NEWNS)` - or, without the capability,
   falls back to the fully rootless `CLONE_NEWUSER | CLONE_NEWNS` with
   `uid_map` / `gid_map` / `setgroups` - and then remounts `/` as
   `MS_REC | MS_PRIVATE` so nothing propagates back
   ([`crates/eidos-launch/src/lib.rs`](crates/eidos-launch/src/lib.rs)). The
   merged view therefore exists *only* inside the game's process tree. Your file
   manager, a second Steam game, a backup job, another user: none of them can see
   it, and none of them need permission to. The corollary matters more than the
   property. **There is no cleanup path to get wrong.** Kill the game, pull the
   power, panic the daemon: the namespace dies with the process tree and the game
   directory is exactly what it always was. A globally mounted VFS has to survive
   its own crash and un-stale the real game directory afterwards. A private one
   has nothing to survive.
2. **Kernel-side metadata caching.** What actually stalls a Bethesda game's
   startup is not read throughput, it is the enormous number of paths Wine probes
   that do not exist. Eidos answers that in the kernel rather than in the daemon:
   negative dentries for failed lookups, long entry/attr TTLs (mod layers are
   immutable for the life of a mount), `FOPEN_CACHE_DIR` on `opendir`, and a
   `.ciopfs` marker so Wine trusts the mount to fold case instead of brute-force
   rescanning every directory. Kernel `FUSE_PASSTHROUGH` (Linux 6.9+) is
   implemented as well but ships **off**, because it stops the game opening its
   own archives and plugins - see [Known issues](#known-issues).

## The Eidos approach

The key insight: **on Linux the game already runs in a box - Wine/Proton.** So we
do not hook anything Windows-side. We virtualize *underneath* Wine, at the Linux
filesystem level. Present a merged view at the directory Wine reads from, and the
game sees the merge for free, with no Windows-side injection at all.

Eidos reproduces the four properties that make `usvfs` valuable:

1. **Merged view** - game data + N mod layers, priority ordered (last-enabled
   mod wins on conflict).
2. **Copy-on-write** - the running game's writes (saves, regenerated configs)
   land in an *Overwrite* layer, never corrupting a mod source.
3. **Zero-touch** - the real game directory is never modified.
4. **Per-process scope** - the merge only exists for the launched game, inside a
   private mount namespace; the rest of the system sees the pristine directory.

### Architecture in one picture

```
              +------------ per-launch user+mount namespace ------------+
  Steam  -->  |   eidos launch wrapper   (`eidos %command%`)            |
              |        |                                                |
              |        v                                                |
  Proton   <--|   merged view  <--  Eidos FUSE union                    |
  sees ONE    |                         ^          ^           ^        |
  directory   |                    overwrite    mod N..1    game data   |
              +---------------------------------------------------------+
       (the rest of the system sees only the pristine game directory)
```

Engine choice: a **FUSE union filesystem in Rust**, which keeps full control of
the semantics OverlayFS cannot express (exact Windows-style case-insensitivity,
precise write redirection, no lowerdir scaling wall). Kernel **passthrough**
(Linux 6.9+) is implemented for the data path but off by default, so resolved
reads are served by the daemon from a cached backing fd. See
[docs/architecture.md](docs/architecture.md) for the full rationale, including
why FUSE over OverlayFS for completeness and long-term stability.

The data path was never the bottleneck anyway. Metadata (`lookup`, `getattr`,
`readdir`) crosses into the daemon, and that traffic is what stalls a
Bethesda game's startup, because Wine probes enormous numbers of paths that do
not exist (DLL search-order walks, `.ini` sidecars, script-extender config
probes). Eidos answers that kernel-side: failed lookups reply as **negative
dentries** with a short TTL rather than a bare `ENOENT`, positive entry/attr TTLs
run long because mod layers are immutable for the life of a mount, and `opendir`
sets `FOPEN_CACHE_DIR` so the kernel serves repeat enumerations itself. Requests
are served from several event loops over `clone_fd`.

`FOPEN_KEEP_CACHE` is deliberately **not** among them: it crashed Skyrim SE
outright (see [Known issues](#known-issues)), which settles it on its own. The
old argument that dropping it was free no longer holds, though - it rested on
passthrough serving every read, and passthrough is now off, so repeat reads do
cross into the daemon. Worth re-measuring, not worth re-enabling blind.

The escape hatches ship with the caching, because "the game sees stale data" has
to be testable against caching as the suspect in a single run:
`EIDOS_FUSE_NO_CACHE=1` turns all of it off and `EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir`
names them one at a time, which is what let the crash above be bisected to a
single flag in four launches. `EIDOS_FUSE_STATS=1` dumps per-op counters (lookup
hit/miss ratio, getattr, readdir, open, read, write) at unmount, and
`EIDOS_FUSE_THREADS=1` restores single-threaded serving when diagnosing a
concurrency bug.

## Repo layout

```
crates/eidos            the unified CLI front end (games / init / play / install / tool / ...)
crates/eidos-gui        the iced GUI (Colony parchment look)
crates/eidos-core       the layer-resolution engine (pure, unit-tested)
crates/eidos-fuse       the read-write FUSE union daemon
crates/eidos-games      supported-game catalog + Steam install detection
crates/eidos-launch     per-launch namespace wrapper: run a game through the view
crates/eidos-instance   instance model: global/portable, profiles, per-mod meta.ini, manifest, load order
crates/eidos-plugins    ESP/ESM/ESL plugin load order (via esplugin) + plugins.txt
crates/eidos-loot       LOOT graph sorting (libloot) + masterlist fetch/cache
crates/eidos-conflicts  per-file conflict analysis (winners / losers, per-mod state)
crates/eidos-install    mod installer: 7-Zip extract + Simple wrapper-strip + meta.ini
crates/eidos-fomod      FOMOD scripted-installer parser + condition/flag engine
crates/eidos-gamefeatures  BSA/archive invalidation + per-profile INIs/saves at launch
crates/eidos-gamedef    declarative per-game descriptor (one row per game; MO2 schema)
crates/eidos-ini        shared low-level INI primitives (newline / section / key / edit)
crates/eidos-nexus      Nexus Mods: v1 API client, nxm:// downloads, update checks
docs/architecture.md    the design and the tradeoffs behind it
scripts/poc-overlay.sh  runnable proof that the "virtualize under Wine" thesis
                        holds with native primitives, no root required
```

## Use it (CLI)

```sh
eidos games                       # supported games installed here (like MO2's list)
eidos init skyrimse               # create a modding instance
# ...drop each mod as a folder into ~/.local/share/eidos/skyrimse/mods/...
eidos install skyrimse mod.7z     # or install a downloaded archive (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # adopt an existing MO2 profile's order + plugin state
eidos sort skyrimse               # LOOT-sort the plugin load order
eidos play skyrimse               # show what would be mounted
eidos play skyrimse -- <command>  # run <command> with the mods mounted over the game
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` and `eidos export` round
out the set; run `eidos` with no arguments for the full list.

`play` mounts the instance's mods over the game's own `Data` directory (via a
bind-stash, so the daemon still reads the pristine files) inside a private
namespace, then runs the command through that view. Writes (saves, regenerated
configs) land in the instance's `overwrite/` layer; the game install and every
mod source stay byte-for-byte pristine.

### No privileged step required

Eidos runs fully rootless. It mounts in a private user + mount namespace, so no
setuid helper, no daemon, and nothing to grant.

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` is **optional** and gates
exactly one thing: kernel FUSE passthrough, which is off by default because it
breaks the game (below). With the capability Eidos takes a plain mount namespace
instead of a user namespace; mods deploy identically either way.

#### Why passthrough is off by default

Passthrough hands the kernel the real backing file so reads skip this daemon
entirely. It is a throughput win that costs correctness here. Measured A/B on
Skyrim SE 1.6.1170, proton-cachyos 11.0, kernel 7.1.4, the same 82-plugin load
order, the only variable being whether the binary carried the capability:

| passthrough | `NtCreateFile` failures with `STATUS_ACCESS_VIOLATION` |
|-------------|--------------------------------------------------------|
| on          | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
| off         | 0                                                      |

With it on the game opens none of its own archives or plugins, which surfaces
in-game as mods that simply are not there - no error, no log line. With it off
the same load order reaches gameplay with its plugins, archives and Papyrus
scripts live.

The failure is invisible from inside the daemon, which is what made it expensive
to find: our own `open` succeeds every time and the kernel never refuses a
backing file (verified across a full failing session with
`EIDOS_FUSE_TRACE=open`: zero `open FAILED`, zero `passthrough refused`). The
error is produced after the daemon replies `opened_passthrough`, so no
daemon-side logging can see it. It is not extension-specific either - it hits
archives and plugins alike, i.e. the files the game holds open for its whole run.

`EIDOS_FUSE_PASSTHROUGH=1` turns it back on, for measuring what it buys or for
re-testing the mechanism. The capability warnings in the launcher and the
Diagnostics tab only appear when you have asked for it.

To launch the game itself through Eidos, set its Steam launch option to:

```
eidos play skyrimse -- %command%
```

Prefix it with `WINEDLLOVERRIDES="d3dcompiler_47=n"` if Proton needs native
d3dcompiler for shader compilation; Eidos merges that with any DLL overrides a
mod ships (ENB/ReShade/`.asi` loaders).

## GUI

```sh
cargo run -p eidos-gui
```

An MO2-style first-launch wizard in the Colony parchment / burgundy look:
welcome -> instance type (portable / global) -> game -> name & location ->
summary -> create -> main screen.

The two-pane main window is built too: a profile picker (switch, or create a new
one by copying the current), a mod list you filter, select, reorder, group with
separators, narrow by category and right-click for actions, plus Data / Plugins /
Conflicts / Overwrite / Saves / Downloads / Diagnostics tabs and a Run button
with a run-target picker.

Reordering is not only send-to-top/bottom: MO2's targeted moves are here too -
send above the first conflicting mod, below the last, to an explicit priority, or
into a separator's group. They all run through one shared move helper, so the
off-by-one that comes from removing rows before re-inserting them exists in one
place instead of five.

**Data** is a real tree of the merged view, expanded one level at a time so
opening a node costs one directory read per layer that has it rather than a
recursive walk of every enabled mod. Every node names the layer that actually
provides it, in the same order the FUSE layer serves. **Plugins** is the
ESP/ESM/ESL load order (toggle, reorder by hand, or sort with LOOT and read the
post-sort report, whose advice links open in your browser). **Conflicts**
explains the per-file winners and losers. **Overwrite** turns what the game wrote
into a real mod in one step. **Saves** parses each save's header - character,
level, location, playtime - and diffs the plugin list baked into it against your
current one, with a button that enables the mods it needs, because naming them
and leaving you to it is the boring half.

"Information..." opens a per-mod dialog: general, conflicts, filetree, INI
tweaks, notes. From the filetree (and from the Data tree) any file can be
**hidden** - renamed to `<name>.mohidden`, which drops it out of the virtual view
without deleting it, so one mod's three stray meshes can be suppressed without
touching priorities. **INI Tweaks** lists the fragments a mod ships in its
`INI Tweaks/` folder; the enabled ones are merged into the profile's game INI at
launch, in priority order, and taken back off when the run's INIs are captured -
otherwise a tweak silently becomes a setting and disabling it would do nothing.

Installing accepts everything: the Simple and FOMOD paths, plus Wrye Bash
**BAIN** packages (tick the sub-packages, which merge in order) and a **manual**
picker that shows the archive tree and lets you point at the data root when no
heuristic recognises the layout. No archive is refused.

**Diagnostics** runs live health checks: the launch capability above all, missing
masters (the single most reliable crash predictor), archives no active plugin
will load, whether the mod list still matches the mods folder, and - after a run
- what the script extender's own log says about each of its plugin DLLs, which
turns "did my SKSE plugins load?" from an inference into evidence.

To launch the game through the GUI, set the game's Steam launch option to the
binary's absolute path (Steam doesn't see `~/.cargo/bin` on PATH):

```
~/.cargo/bin/eidos-gui %command%
```

Eidos opens on the game's instance; click Run to launch it through the merged
view. (The Run button shows this exact line, with the running binary's real
path, if you press it outside Steam.)

Steam's `%command%` for the Bethesda titles usually points at
`<Game>Launcher.exe`. Eidos never runs it: the launcher is a separate settings
app that re-scans `Data` and rewrites `plugins.txt`, undoing the load order that
was just deployed. It swaps in the script extender's loader if one is installed,
the game binary otherwise, and says so when it has to fall back - a game that
starts with every SKSE mod inert is worse than one that does not start.

Older instructions here forced `WINEDLLOVERRIDES="d3dcompiler_47=n"`. That is no
longer needed and was never quite right: an override to *native* only helps if a
genuine `d3dcompiler_47.dll` is already in the prefix. Eidos now scans the
enabled mods' DLL imports, deploys the real Microsoft DLL itself, and only then
sets the override.

## Try the proof of concept

No game required. It proves union + copy-on-write + zero-touch + per-namespace
scope using only unprivileged OverlayFS in a user namespace (Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Build and test

```sh
cargo test                 # workspace unit tests + the eidos-fuse real-mount suite
cargo build -p eidos-fuse  # the union daemon on its own
```

The `eidos-fuse` integration suite is not a mock: it mounts a real union inside
its own private user+mount namespace (no root, no host mounts touched) and drives
20 checks through the kernel, including a writable `MAP_SHARED` mmap round-trip,
the negative-dentry cases, and a root union carrying a Data union inside it. If
the user namespace is unavailable it says so and skips only the checks a racing
host service could disturb. It runs `harness = false` (the namespace must be
entered before any thread exists), so cargo's own summary line reports zero for
it - read the suite's own `union integration result:` line instead.

### Diagnosing the VFS

Two environment variables exist for when the game sees something the filesystem
does not agree with:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

The granular form is what found the crash described under Known issues: turning
all four off answers "is it the caching?", and only naming them answers "which
one". The counters answer the other half - a load that shows `read 0` is one
where `FUSE_PASSTHROUGH` served every byte in the kernel, so anything you were
about to tune on the read path is already free.

## Mount a union by hand

The first `--layer` wins on conflict; the last is your pristine game data. The
mount needs only `/dev/fuse` and `fusermount3` (no overlayfs, no Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read and write through /mnt/point ...
fusermount3 -u /mnt/point
```

Writes land in `--overwrite <dir>` (a temporary directory when omitted), so the
layers themselves stay pristine even here.

## Roadmap

- [x] Validate the under-Wine virtualization thesis (PoC)
- [x] Layer-resolution engine + tests (`eidos-core`)
- [x] FUSE union daemon (`eidos-fuse`) - live-verified: merge priority,
      fall-through, case-insensitive reads, no root / no overlayfs / no Wine
- [x] Copy-up / Overwrite layer (writes) - live-verified: new files, edits to
      game files, and edits to mod files all land in Overwrite; the game install
      and every mod source stay pristine
- [x] Deletes (whiteouts), rename, and statfs - live-verified: deleting a game
      or mod file hides it (via a whiteout in Overwrite) while the source stays
      pristine; rename works (saves), and df reports real free space
- [x] Supported-game catalog + Steam install detection (`eidos-games`) -
      live-verified: finds installed games across all libraries (incl. other
      drives) with their data dir + Proton prefix
- [x] Per-launch user+mount namespace wrapper (`eidos-launch`) - live-verified:
      runs a command through a private union view; the host sees no mount, the
      game install and mod sources stay pristine, writes land in Overwrite
- [x] Instance management + unified `eidos` CLI (`games` / `init` / `play`) -
      live-verified end to end: detect a game, create an instance, mount its
      mods over the game's Data dir, run a command through the view
- [x] Steam launch-option integration (`eidos %command%`) with a real Proton game
      - both the CLI (`eidos play <id> -- %command%`) and the GUI (which resolves
      which detected game Steam is launching from the `%command%` it was handed,
      then runs it through the view)
- [x] GUI first-launch wizard (`eidos-gui`, iced) - MO2-style screens (welcome ->
      portable/global -> game -> name -> summary -> main), Colony parchment theme
- [x] GUI main window (`eidos-gui`) - two-pane MO2 layout: a profile picker, a
      mod list (enable/disable, reorder, per-mod conflict flags), and Data /
      Plugins / Conflicts / Overwrite / Saves / Downloads tabs with a Run button
- [x] Per-mod `meta.ini`, byte-compatible with MO2 (`eidos-instance`) - existing
      MO2 instances round-trip unchanged (version / Nexus id / category /
      endorsed), with an `update_available` check ready for a future Nexus crate
- [x] Self-describing instance manifest `eidos-instance.ini` (`eidos-instance`) -
      records game id + schema version, so portable instances need no path-guessing
- [x] Profiles (`eidos-instance`) - per-profile enabled set + load order over one
      shared `mods/` pool, with a GUI picker (switch / new-by-copy)
- [x] ESP/ESM/ESL plugin load order (`eidos-plugins`) - esplugin-parsed headers +
      MO2-parity ordering (masters first, master-before-dependent) and FormID
      indexes (`FE` light / `FD` medium), written as `plugins.txt`/`loadorder.txt`
      into the Proton prefix right before launch; surfaced in the Plugins tab
- [x] Per-file conflict detection (`eidos-conflicts`) - one pass over the enabled
      layers builds a winners/losers tree + per-mod state (Overwrites /
      Overwritten / Mixed / Redundant), shown in the Conflicts tab and as per-mod
      flags in the mod list
- [x] Mod installer (`eidos-install` + `eidos-fomod`) - the MO2 Simple installer
      (7-Zip extract, wrapper-strip via a Gamebryo ModDataChecker) plus the FOMOD
      scripted installer (UTF-16 `ModuleConfig.xml` parse + condition/flag engine),
      driven from a CLI (`eidos install`) and the GUI Install button + an
      interactive FOMOD wizard; writes a MO2-compatible `meta.ini`
- [x] Nexus Mods integration (`eidos-nexus`) - connect with a personal API key
      (`eidos nexus key`), register the `nxm://` handler (`eidos nxm --register`)
      so the site's "Mod Manager Download" button downloads straight into the
      instance's `downloads/` (with MO2-format `.meta` sidecars), check installed
      mods for updates (`eidos nexus update`, MO2's rate-limit-friendly
      updated-this-month strategy), and install from the GUI Downloads tab in one
      click (Simple or FOMOD wizard)
- [x] Tools through the VFS (`eidos tool`) - run xEdit/FNIS/BodySlide through the
      same merged view, inside the game's Proton prefix, with no Steam `%command%`:
      a protontricks-style Proton resolver (config.vdf `CompatToolMapping` ->
      compatibilitytools.d / official Protons -> `STEAM_COMPAT_*` env), a
      per-instance tool list (MO2 `ExecutablesList` parity, script extender seeded
      by default), and a run-target picker next to the GUI Run button. Tool output
      lands in Overwrite, so the next launch picks it up - the generate-then-play
      loop. (MO2 itself rebuilds its mapping per run, so per-launch mounts are
      exact parity.)
- [x] Per-game Bethesda features (`eidos-gamefeatures` + `eidos-gamedef`) - all
      keyed off one declarative `GameDef` row per game (MO2's `IPluginGame`
      schema): **BSA/archive invalidation** so loose mod files override the
      vanilla BSAs (without it BSA-packed mods are silently ignored);
      **per-profile INIs** seeded from the prefix then deployed/captured around
      launch; and **per-profile saves** via a namespace bind-mount of the
      profile's saves over the prefix (the Linux-native equivalent of MO2's usvfs
      save mapping, no prefix changes). The INI writing shares one `eidos-ini`
      primitive (MO2's single-`QSettings` idea), keeping MO2 `meta.ini`
      round-trips byte-for-byte
- [x] Rootless perf tuning (1 MiB readahead / max_write, parallel dirops) and an
      opt-in `FUSE_PASSTHROUGH` path. Passthrough is **off by default**: measured
      on Skyrim SE it stops the game opening its own archives and plugins (152
      `STATUS_ACCESS_VIOLATION` opens vs 0 without it), so the daemon serves
      reads itself. `EIDOS_FUSE_PASSTHROUGH=1` re-enables it, and then the
      `CAP_SYS_ADMIN` it needs in the initial user namespace becomes relevant
- [x] Harden the daemon for real use - inode reference-counting + `forget`,
      offset-stable `readdir` (snapshot per directory handle), per-handle
      `pread`/`pwrite` (no re-resolve per syscall, lock released before I/O),
      case-insensitive whiteouts, opaque directories, POSIX errnos (`rmdir`
      ENOTEMPTY, `rename` NOREPLACE/EXCHANGE), `setattr` (mode / timestamps),
      xattr passthrough (Wine `DOSATTRIB`), symlinks, `fsync` durability.
      Covered by a real-mount integration suite that runs in a private
      namespace, including a writable `MAP_SHARED` mmap round-trip, with
      `setattr` guarded so the kernel's post-unlink attribute flush cannot
      resurrect a deleted file. (`writeback_cache` is off - it broke loading
      Windows DLLs from the mount.)
- [x] **Runs a real heavily-modded Skyrim SE (110 mods) end-to-end under Proton**
      - all ~50 SKSE plugin DLLs (CommonLibSSE-NG included) load and run via the
      mount, each writing its config into the Overwrite layer. Needed two
      MO2/usvfs parity fixes: launch with **CWD = game root** (CommonLibSSE-NG
      opens its address library by a CWD-relative path) and **NTFS-like sorted
      `readdir`** (the Creation Engine's loose-file indexer assumes it). That run
      predates the passthrough default flip and was made with passthrough **on**,
      so it proves DLL image-mapping, not a playable load order; rootless, a small
      load order has since reached gameplay with plugins, archives and Papyrus
      scripts live, but the ~50-DLL case is not re-validated without passthrough
- [x] Native DLL provisioning for Proton (`eidos-gamefeatures::native_dll`) - no
      Proton flavour ships Microsoft's native `d3dcompiler_47.dll` (they symlink
      Wine's builtin HLSL stub, which Community Shaders / ENB / ReShade reject), so
      Eidos scans enabled mods' DLL import tables (the `object` crate) and, when one
      imports it, deploys the bundled genuine MS redistributable into the prefix's
      `system32`/`syswow64` and forces it native - unlinking Proton's builtin symlink
      first, backing up any displaced file, idempotent and best-effort
- [x] Modding-tool prerequisites (`eidos prereqs`) - tools run in the game's shared
      Proton prefix through the merged view, so one prereq set covers them all.
      Tier 1 (bundled, zero network): the DirectX helpers BodySlide / DynDOLOD / CAO
      need (`d3dx9_43` / `d3dx11_43` / `d3dcompiler_43`), declared per-tool in
      `tools.ini` and provisioned at launch. Tier 2 (consented download): the .NET /
      vcrun verbs Synthesis / Pandora / FNIS need, installed via the system
      winetricks pointed straight at Proton's own wine (bypassing the protontricks +
      Proton-GE mismatch), behind an explicit `eidos prereqs <id> --install` / GUI
      "Tool Setup" button, recorded in a per-instance sentinel
- [x] GUI to MO2 daily-driver parity (`eidos-gui`) - a mod-list filter box,
      click-to-select, and a right-click action menu (enable/disable, send to
      top/bottom, open in explorer, visit on Nexus, reinstall, rename, remove,
      information); an interactive Plugins tab (toggle ESP/ESM, persisted to
      `plugins.txt`); a per-mod information dialog (general / conflicts / filetree /
      editable notes); a Version column; and wired Nexus / Change Game / Settings /
      Tool Setup toolbar buttons
- [x] LOOT-based plugin auto-sort (`eidos-loot`) - libloot's own graph sort behind
      `eidos sort` and the GUI's Sort button, with the masterlist fetched and
      cached per instance (refreshed on each sort, like MO2/LOOT) and a userlist
      for local overrides. The post-sort report pops the way MO2's dialog does,
      listing the plugin messages LOOT attached, with its advice links clickable.
      Games libloot cannot sort say so up front, in Diagnostics and on the tab
- [x] Mod-list separators + categories - MO2 group dividers (create, colour,
      collapse/expand, persisted by display name) and a category filter driven by
      each mod's primary `meta.ini` category, alongside a Category column
- [x] MO2 profile import (`eidos import`, GUI) - takes over an existing MO2
      profile's `modlist.txt` order and enabled states plus its
      `plugins.txt`/`loadorder.txt` verbatim (the formats are already identical).
      Mods MO2 listed that are not installed here are reported rather than dropped,
      and local mods MO2 never knew about are kept at the bottom
- [x] Overwrite-to-mod - turn what the game and the tools wrote into a first-class
      mod in one step, which is the other half of the generate-then-play loop
- [x] VFS correctness pass against Fluorine's implementation - copy-up no longer
      clones a read-only lower mode (a 0444 Steam depot file used to make the next
      write-open fail `EACCES`), one file has one inode whatever its casing (it
      used to split, poisoning `forget`/rename/passthrough bookkeeping), a
      directory rename re-keys its whole subtree and discards clobbered inodes,
      and `setxattr`/`removexattr` no longer resurrect a deleted file (Wine writes
      `user.DOSATTRIB` constantly, so that was expected traffic, not an edge case)
- [x] Kernel-side metadata caching - negative dentries for the paths Wine probes
      and never finds, a long positive entry/attr TTL, `FOPEN_CACHE_DIR` on
      `opendir`, `RLIMIT_NOFILE` raised at init, several event loops over
      `clone_fd`, and per-op counters under `EIDOS_FUSE_STATS` so a metadata storm
      is measurable instead of guessed at
- [x] MO2's remaining daily-driver features - hidden files (`.mohidden`, honoured
      by the resolver so a hidden file leaves the merged view *without claiming its
      name*, which is what lets the layer below win); a recursive Data tree with
      per-node origin; targeted mod-list moves (above/below the first/last
      conflicting mod, to a priority, into a separator); BAIN sub-package and
      manual installers so no archive is refused; save-header parsing with the
      save's own plugin list diffed against the current one; mod-shipped INI
      tweaks merged at launch and un-merged at capture; orphan-archive and
      script-extender-log diagnostics
- [x] Guards against the failure modes that quietly destroy a setup - the mod list
      is derived from the mods directory like MO2's, but unlike MO2's the
      reconciliation is not written back blind: a mods folder that is unreadable,
      or empty while the saved list is not, refuses the save instead of flattening
      a curated order (MO2 has no equivalent guard - `refreshModStatus` rewrites
      `modlist.txt` in the same pass that dropped the entries). Same rule on the
      plugin side: a capture that clears the active set entirely is refused at any
      size, because the names are still listed so nothing was uninstalled, and no
      edit produces that
- [ ] Casing normalization at mod-import time
- [ ] Packaging and distribution. Now unconstrained, since the launch capability
      became optional: Eidos runs rootless and only the opt-in passthrough path
      wants `CAP_SYS_ADMIN`. That matters because a file capability lives in the
      `security.capability` xattr of the executable and the kernel ignores it on a
      `nosuid` mount, which is exactly what an unprivileged FUSE mount is forced
      to be (check any FUSE mount on your own machine:
      `findmnt -t fuse -o TARGET,OPTIONS` shows `nosuid,nodev`) - so a
      self-mounting bundle can never carry it, and a sandbox that sets
      no-new-privs cannot gain it either. Self-contained formats (AppImage, and a
      Flatpak modulo its own namespace nesting) are therefore back on the table;
      only someone opting into passthrough needs the binary on a real filesystem
      where `setcap` reaches it

The manager layer above the VFS is complete per the MO2 + usvfs study that drove
this work ([docs/master-pieces.md](docs/master-pieces.md), all 6 master pieces
done): the mod installer (Simple + FOMOD wizard), plugins, conflicts, profiles,
`meta.ini`, the instance manifest, per-game Bethesda features (BSA invalidation +
per-profile INIs/saves off a declarative `GameDef`), and tools through the VFS
(`eidos tool`), plus Nexus integration beyond it (`eidos-nexus`: nxm:// downloads,
update checks, the GUI Downloads tab). Since then: the GUI brought up to MO2
daily-driver parity (separators, categories, diagnostics, Overwrite-to-mod, MO2
profile import), LOOT sorting, native DLL provisioning so Proton graphics mods
(Community Shaders) and tools (BodySlide) just work, the `eidos prereqs`
tool-prerequisite system, and a correctness-plus-caching pass over the daemon
itself. Next up: casing normalization at import, more game families proven
in-game, and packaging - plus the open `plugins.txt` question under Known issues,
which is the one thing standing between a working mount and a working playthrough.

## Known issues

**Wine could not tell that the mount folds case.** Fixed, and it was the one that
mattered.

There is no API for "is this filesystem case-insensitive", so Wine's
`get_dir_case_sensitivity` sniffs for the marker CIOPFS leaves in the directories
it serves. Absent, Wine assumes case-SENSITIVE, and every lookup whose spelling
does not match byte-for-byte falls back to reading the WHOLE directory to find a
case-insensitive match. Bethesda games ask for `data/ccbgssse001-fish.bsa` while
the file is `ccBGSSSE001-Fish.bsa`, so it fired on nearly every asset: 4471 marker
probes and 2236 full directory re-reads in eight seconds, and 195796 enumerations
of `Data` in ninety. Skyrim SE never reached its main menu - it sat at 240 MB
resident while the daemon burned 92% of a core.

Eidos had folded case in `resolve_read` from the start. The whole cost was never
saying so. `lookup` now answers `.ciopfs`; `readdir` still does not list it.

Two things made it fatal rather than merely slow. The cost scales with directory
size, so installing the Anniversary content (`Data` from 37 files to 177) tipped
it over. And `opendir` eagerly built the merged listing, which is pure waste when
Wine opens a directory only to `stat` that marker inside it - the snapshot is
taken on the first `readdir` now.

After: the main menu, 2.1 GB resident, daemon at 0% CPU.

`EIDOS_FUSE_TRACE=opendir` is what found it, and ships. The op counters say how
many; 195796 enumerations of one directory is invisible in a total.

**The game rewriting `plugins.txt` empty** was very likely the same thing - a
`Data` it could not enumerate in any reasonable time, so it concluded there was
nothing there and saved that. Not proven, and worth re-checking. Either way the
capture guard (a capture that clears the active set entirely is refused at any
size) means it can no longer damage the profile.

**`FOPEN_KEEP_CACHE` is off.** Fixed, and worth knowing why. It crashed Skyrim SE
on a null dereference seconds after the main menu, deterministically, with zero
mods installed; the other three kernel-side caches were bisected out individually
and only this one mattered. Losing it was measured as free at the time, but that
measurement was taken with `FUSE_PASSTHROUGH` active, where the daemon serves
*zero* reads (`EIDOS_FUSE_STATS` reported `read 0` for a full load) and the
kernel was already caching those pages against the backing file. Passthrough is
now off by default (below), so that argument no longer applies and the real cost
is unmeasured - the crash is reason enough to leave it off regardless. Re-enable
with `EIDOS_FUSE_KEEP_CACHE=1` to investigate; the two flags are no longer
entangled, so it can now be tested on its own.

### FUSE passthrough stops the game loading any mod content

Fixed by turning it off; `EIDOS_FUSE_PASSTHROUGH=1` brings it back. With
passthrough on, Skyrim SE fails to open 152 of its own files (75 `.bsa`, 65
`.esl`, 10 `.esm`, 2 `.esp`) with `STATUS_ACCESS_VIOLATION`, against 0 with it
off, on kernel 7.1.4 - so no mod content loads, silently. The kernel raises the
error after the daemon has replied `opened_passthrough`, so the daemon's own logs
show a clean run (zero failed opens, zero refused backing files). Root cause in
the kernel path is not established; the switch is kept so it can be re-tested,
and so passthrough could be narrowed to DLLs only if image-mapping turns out to
need it.

## Prior art and references

- [`ModOrganizer2/usvfs`](https://github.com/ModOrganizer2/usvfs) - the Windows semantics we reproduce
- [`SulfurNitride/Fluorine-Manager`](https://github.com/SulfurNitride/Fluorine-Manager) - MO2 itself with usvfs replaced by a libfuse3 VFS. The first Linux mod manager with a real virtual filesystem, and the closest thing to a peer this project has
- [`containers/fuse-overlayfs`](https://github.com/containers/fuse-overlayfs) - overlay semantics in FUSE, reference for the engine
- [Limo](https://github.com/limo-app/limo), [Amethyst Mod Manager](https://github.com/ChrisDKN/Amethyst-Mod-Manager), [RadTux](https://www.nexusmods.com/fallout4/mods/105285), [MO2 Linux installer](https://github.com/Furglitch/modorganizer2-linux-installer) - the deploy-by-links and MO2-under-Wine approaches
- LMO (Codeberg) - a native Rust manager built on `fuse-overlayfs`

## License

GPL-3.0. See [LICENSE](LICENSE).
