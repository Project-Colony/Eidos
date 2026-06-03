# Eidos

**A native Linux virtual filesystem for game mods.** Eidos reproduces what Mod
Organizer 2's `usvfs` does on Windows - a clean, per-launch merged view of your
mods over a game's files - but built from native Linux primitives instead of
Windows API hooking, so games run under Proton/Wine without the usual
Wine-shoehorned mod-manager pain.

> Status: **early**. The thesis is validated, the core engine is implemented and
> unit-tested, the daemon is next. See [Roadmap](#roadmap).

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
crates/eidos-core      the layer-resolution engine (pure, unit-tested)
docs/architecture.md   the design and the tradeoffs behind it
scripts/poc-overlay.sh runnable proof that the "virtualize under Wine" thesis
                       holds with native primitives, no root required
```

## Try the proof of concept

No game required. It proves union + copy-on-write + zero-touch + per-namespace
scope using only unprivileged OverlayFS in a user namespace (Linux >= 5.11):

```sh
./scripts/poc-overlay.sh
```

## Build and test the core

```sh
cargo test
```

## Roadmap

- [x] Validate the under-Wine virtualization thesis (PoC)
- [x] Layer-resolution engine + tests (`eidos-core`)
- [ ] FUSE union daemon with passthrough (`eidos-fuse`)
- [ ] Per-launch namespace wrapper + Steam launch-option integration (`eidos-launch`)
- [ ] Copy-up / Overwrite handling end to end
- [ ] Casing normalization at mod-import time
- [ ] Launch arbitrary tools (xEdit / FNIS) into the same view
- [ ] Mod management UI, or integrate with an existing native manager

## Prior art and references

- [`ModOrganizer2/usvfs`](https://github.com/ModOrganizer2/usvfs) - the Windows semantics we reproduce
- [`containers/fuse-overlayfs`](https://github.com/containers/fuse-overlayfs) - overlay semantics in FUSE, reference for the engine
- [Limo](https://github.com/limo-app/limo), [RadTux](https://www.nexusmods.com/fallout4/mods/105285), [MO2 Linux installer](https://github.com/Furglitch/modorganizer2-linux-installer) - the existing compromises

## License

GPL-3.0. See [LICENSE](LICENSE).
