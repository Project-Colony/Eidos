# Eidos

**A native Linux virtual filesystem for game mods.** Eidos reproduces what Mod
Organizer 2's `usvfs` does on Windows - a clean, per-launch merged view of your
mods over a game's files - but built from native Linux primitives instead of
Windows API hooking, so games run under Proton/Wine without the usual
Wine-shoehorned mod-manager pain.

> Status: **early but real**. The thesis is validated; the resolver and the
> read-write FUSE daemon are implemented, hardened, and covered by unit and
> real-mount integration tests. What remains is Proton launch integration,
> import-time casing, and performance validation on a heavy load order. See
> [Roadmap](#roadmap).

## The problem

MO2 is two things bolted together:

1. A Qt mod-manager UI + plugin system. Cross-platform, ports easily.
2. **`usvfs`** - the feature that actually *matters*: a per-process virtual
   filesystem that merges your mods over the game directory **without ever
   touching the real game files**. It works by injecting a DLL and hooking the
   Windows NT syscall layer.

Part 2 is intrinsically Windows. It cannot be "ported" - the mechanism has no
Linux equivalent. That is why every Linux option today is a compromise:

| Tool | Approach | Compromise |
|---|---|---|
| MO2 + Wine installers | run MO2 itself inside the Wine prefix | usvfs runs under Wine: slow, fragile, painful setup |
| Limo | native, symlink/hardlink deploy | writes links into the game dir, no per-process isolation |
| RadTux | native daemon + dll shim, symlinks on launch | still MO2-under-Wine, symlink write-back caveats |

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
  Proton   <--|   merged view  <--  Eidos FUSE union (passthrough)      |
  sees ONE    |                         ^          ^           ^        |
  directory   |                    overwrite    mod N..1    game data   |
              +---------------------------------------------------------+
       (the rest of the system sees only the pristine game directory)
```

Engine choice: a **FUSE union filesystem in Rust**, built around kernel
**passthrough** (Linux 6.9+) so resolved reads bypass userspace and run at
near-native speed, while we keep full control of the semantics OverlayFS cannot
express (exact Windows-style case-insensitivity, precise write redirection, no
lowerdir scaling wall). See [docs/architecture.md](docs/architecture.md) for the
full rationale, including why FUSE over OverlayFS for completeness and long-term
stability.

## Repo layout

```
crates/eidos            the unified CLI front end (games / init / play)
crates/eidos-gui        the iced GUI (Colony parchment look)
crates/eidos-core       the layer-resolution engine (pure, unit-tested)
crates/eidos-fuse       the read-write FUSE union daemon
crates/eidos-games      supported-game catalog + Steam install detection
crates/eidos-launch     per-launch namespace wrapper: run a game through the view
crates/eidos-instance   instance model: global/portable, profiles, per-mod meta.ini, manifest, load order
crates/eidos-plugins    ESP/ESM/ESL plugin load order (via esplugin) + plugins.txt
crates/eidos-conflicts  per-file conflict analysis (winners / losers, per-mod state)
crates/eidos-install    mod installer: 7-Zip extract + Simple wrapper-strip + meta.ini
crates/eidos-fomod      FOMOD scripted-installer parser + condition/flag engine
crates/eidos-gamefeatures  BSA/archive invalidation + per-profile INIs/saves at launch
crates/eidos-gamedef    declarative per-game descriptor (one row per game; MO2 schema)
crates/eidos-ini        shared low-level INI primitives (newline / section / key / edit)
docs/architecture.md    the design and the tradeoffs behind it
scripts/poc-overlay.sh  runnable proof that the "virtualize under Wine" thesis
                        holds with native primitives, no root required
```

## Use it (CLI)

```sh
eidos games                       # supported games installed here (like MO2's list)
eidos init skyrimse               # create a modding instance
# ...drop each mod as a folder into ~/.local/share/eidos/skyrimse/mods/...
eidos play skyrimse               # show what would be mounted
eidos play skyrimse -- <command>  # run <command> with the mods mounted over the game
```

`play` mounts the instance's mods over the game's own `Data` directory (via a
bind-stash, so the daemon still reads the pristine files) inside a private
namespace, then runs the command through that view. Writes (saves, regenerated
configs) land in the instance's `overwrite/` layer; the game install and every
mod source stay byte-for-byte pristine.

To launch the game itself through Eidos, set its Steam launch option to:

```
eidos play skyrimse -- %command%
```

## GUI

```sh
cargo run -p eidos-gui
```

An MO2-style first-launch wizard in the Colony parchment / burgundy look:
welcome -> instance type (portable / global) -> game -> name & location ->
summary -> create -> main screen.

The two-pane main window is built too: a profile picker (switch, or create a new
one by copying the current), a mod list you enable/disable and reorder (with a
per-mod conflict flag), and Data / Plugins / Conflicts / Overwrite / Downloads
tabs plus a Run button. The Plugins tab is the ESP/ESM/ESL load order; the
Conflicts tab explains the per-file winners and losers.

## Try the proof of concept

No game required. It proves union + copy-on-write + zero-touch + per-namespace
scope using only unprivileged OverlayFS in a user namespace (Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Build and test

```sh
cargo test                 # eidos-core resolver unit tests
cargo build -p eidos-fuse  # the read-only union daemon
```

## Mount a read-only union (works today, no root)

The first `--layer` wins on conflict; the last is your pristine game data. The
mount needs only `/dev/fuse` and `fusermount3` (no overlayfs, no Wine):

```sh
eidos-fuse --layer mod_b --layer mod_a --layer game_data /mnt/point
# ... read through /mnt/point ...
fusermount3 -u /mnt/point
```

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
- [ ] Steam launch-option integration (`eidos %command%`) with a real Proton game
- [x] GUI first-launch wizard (`eidos-gui`, iced) - MO2-style screens (welcome ->
      portable/global -> game -> name -> summary -> main), Colony parchment theme
- [x] GUI main window (`eidos-gui`) - two-pane MO2 layout: a profile picker, a
      mod list (enable/disable, reorder, per-mod conflict flags), and Data /
      Plugins / Conflicts / Overwrite / Downloads tabs with a Run button
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
- [x] FUSE passthrough + rootless perf tuning (1 MiB readahead / max_write).
      Passthrough negotiates `FUSE_PASSTHROUGH` and engages when the daemon runs
      privileged (`setcap cap_sys_admin+ep`, taken via a bare mount namespace):
      the kernel then serves reads/mmap straight from the real backing file,
      which is what lets Windows SKSE-plugin DLLs image-map natively. Rootless it
      falls back to the daemon's own reads (correct, but DLLs may not load -
      kernel passthrough needs CAP_SYS_ADMIN in the initial user namespace)
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
      Windows DLLs from the mount; passthrough serves DLL image-mapping instead.)
- [x] **Runs a real heavily-modded Skyrim SE (110 mods) end-to-end under Proton**
      - all ~50 SKSE plugin DLLs (CommonLibSSE-NG included) load and run via the
      mount, each writing its config into the Overwrite layer. Needed two
      MO2/usvfs parity fixes: launch with **CWD = game root** (CommonLibSSE-NG
      opens its address library by a CWD-relative path) and **NTFS-like sorted
      `readdir`** (the Creation Engine's loose-file indexer assumes it)
- [ ] Casing normalization at mod-import time

The manager layer above the VFS is complete per the MO2 + usvfs study that drove
this work ([docs/master-pieces.md](docs/master-pieces.md), all 6 master pieces
done): the mod installer (Simple + FOMOD wizard), plugins, conflicts, profiles,
`meta.ini`, the instance manifest, per-game Bethesda features (BSA invalidation +
per-profile INIs/saves off a declarative `GameDef`), and tools through the VFS
(`eidos tool`). What lies beyond the study: Nexus integration (downloads, updates),
LOOT sorting, and mod-list UX (separators, filtering).

## Prior art and references

- [`ModOrganizer2/usvfs`](https://github.com/ModOrganizer2/usvfs) - the Windows semantics we reproduce
- [`containers/fuse-overlayfs`](https://github.com/containers/fuse-overlayfs) - overlay semantics in FUSE, reference for the engine
- [Limo](https://github.com/limo-app/limo), [RadTux](https://www.nexusmods.com/fallout4/mods/105285), [MO2 Linux installer](https://github.com/Furglitch/modorganizer2-linux-installer) - the existing compromises

## License

GPL-3.0. See [LICENSE](LICENSE).
