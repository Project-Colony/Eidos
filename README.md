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
crates/eidos           the unified CLI front end (games / init / play)
crates/eidos-gui       the iced GUI (Colony parchment look)
crates/eidos-core      the layer-resolution engine (pure, unit-tested)
crates/eidos-fuse      the read-write FUSE union daemon
crates/eidos-games     supported-game catalog + Steam install detection
crates/eidos-launch    per-launch namespace wrapper: run a game through the view
crates/eidos-instance  instance model (global / portable, layout, load order)
docs/architecture.md   the design and the tradeoffs behind it
scripts/poc-overlay.sh runnable proof that the "virtualize under Wine" thesis
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
summary -> create -> main screen. The two-pane main window (mod list with
drag-reorder load order, Plugins / Data / Saves tabs, Play button) comes next.

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
- [ ] GUI main window: two-pane mod list with drag-reorder load order,
      enable/disable, Plugins / Data / Saves tabs, Play button
- [x] FUSE passthrough wired (best-effort) + rootless perf tuning (1 MiB
      readahead / max_write). Note: kernel passthrough needs real root, so the
      rootless daemon falls back to serving reads/writes itself - correct, just
      not zero-copy
- [x] Harden the daemon for real use - inode reference-counting + `forget`,
      offset-stable `readdir` (snapshot per directory handle), per-handle
      `pread`/`pwrite` (no re-resolve per syscall, lock released before I/O),
      case-insensitive whiteouts, opaque directories, POSIX errnos (`rmdir`
      ENOTEMPTY, `rename` NOREPLACE/EXCHANGE), `setattr` (mode / timestamps),
      xattr passthrough (Wine `DOSATTRIB`), symlinks, `fsync` durability.
      Covered by a real-mount integration suite that runs in a private
      namespace. `writeback_cache` is deliberately off: with copy-up it
      resurrects deleted files.
- [ ] Casing normalization at mod-import time

## Prior art and references

- [`ModOrganizer2/usvfs`](https://github.com/ModOrganizer2/usvfs) - the Windows semantics we reproduce
- [`containers/fuse-overlayfs`](https://github.com/containers/fuse-overlayfs) - overlay semantics in FUSE, reference for the engine
- [Limo](https://github.com/limo-app/limo), [RadTux](https://www.nexusmods.com/fallout4/mods/105285), [MO2 Linux installer](https://github.com/Furglitch/modorganizer2-linux-installer) - the existing compromises

## License

GPL-3.0. See [LICENSE](LICENSE).
