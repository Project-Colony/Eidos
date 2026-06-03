# Eidos architecture

This document records *what* we are building and *why* we made each call, so the
reasoning survives even when the code changes.

## What usvfs actually does

MO2's value is not its UI. It is `usvfs`, which gives a running game a virtual,
merged view of many mod folders stacked over the game's own files, with four
properties we must reproduce:

1. **Merged view** - game data plus N mod layers, in priority order. The
   last-enabled mod wins a file conflict.
2. **Per-process scope** - only the game (and tools launched through MO2) see the
   merge. The rest of the system sees the pristine game directory.
3. **Zero-touch** - the real game install is never modified. No copies, no links
   written into it.
4. **Copy-on-write** - when the game writes (saves, regenerated `.ini`, an edited
   record), the write must not corrupt the source mod. Reads see the merged tree,
   writes go to a writable *Overwrite* layer.

On Windows `usvfs` achieves this by injecting a DLL and hooking the NT syscall
layer (`NtCreateFile`, `NtQueryDirectoryFile`, ...). That mechanism is
intrinsically Windows; it has no Linux equivalent and cannot be ported.

## The unlock: virtualize underneath Wine

On Windows the game runs directly on the OS, so the only place to intervene is
the Windows syscall layer. On Linux the game does **not** run bare: it runs
inside Wine/Proton. Wine exposes a Linux directory to the Windows `.exe` as the
game's drive.

That changes everything. We never touch the Windows side. We present the merged
view at the **Linux** path Wine reads from, and Wine -> the game sees the merge
transparently. "Reimplement an OS-level kernel-hooking system" collapses into
"wire together native Linux filesystem primitives", which the container world has
already battle-tested.

## Mechanism options considered

| Mechanism | Union | COW | Per-process isolation | Perf | Casing |
|---|---|---|---|---|---|
| OverlayFS (kernel union) | native (ordered lowerdirs) | native (upperdir) | via mount namespace | near-native | does **not** case-fold |
| FUSE (userspace daemon) | we implement | we implement | via namespace | userspace round-trip, *unless passthrough* | we control it exactly |
| bind mounts | manual, messy | no | via namespace | native | no |

### Why not pure OverlayFS

OverlayFS maps cleanly onto the union + COW model and is fast, and since Linux
5.11 it can be mounted unprivileged inside a user namespace. But for a *complete*
usvfs replacement it has gaps we cannot close from outside:

- **No case-insensitivity.** A game probing `textures/foo.dds` when a mod ships
  `Textures/FOO.DDS` simply misses. This is the classic Linux modding failure.
- **Quirks we would inherit**, not control: copy-up edge cases, opaque-dir and
  whiteout handling, rename semantics.
- **Lowerdir scaling.** Deep stacks (a 2000-mod load order) hit practical limits
  and lookup cost.

You can bolt casefolding on (a casefold-enabled backing fs, or a thin FUSE shim),
but then you maintain a two-layer stack whose interaction is the hard part.

### Decision: a FUSE union filesystem we own

We build the engine as a FUSE union filesystem in Rust, designed around kernel
passthrough. Rationale, against the three goals (complete / performant / stable
long-term):

- **Most complete.** Only a controlled implementation gives exact Windows-style
  case-insensitivity, precise Overwrite/write-redirection, correct handling of
  odd path probes, and no lowerdir wall (mods resolve through an in-memory index,
  not a kernel layer list).
- **Stable long-term.** Counterintuitively, owning the FS logic is *more* stable
  here: our behaviour is pinned by a test suite and does not shift when kernel
  OverlayFS semantics change. One engine we own beats a two-layer stack whose
  interaction we debug. `containers/fuse-overlayfs` made exactly this choice (do
  it all in FUSE) and is battle-tested under rootless Podman.
- **Fast enough now.** The historical "FUSE is slow" objection is largely
  answered by **FUSE passthrough** (merged in Linux 6.9): once the daemon
  resolves which real file backs a path, subsequent reads go straight
  kernel-to-disk without bouncing through the daemon. Games are read-heavy on a
  modest number of files at load, so the bulk transfer bypasses userspace; the
  daemon only does path resolution. Still: **measure** against a real heavy load
  order before claiming victory.

OverlayFS keeps its place as the fast way to **prove the thesis** (see the PoC)
and as a possible optimization later, not as the foundation.

## Target architecture, end to end

1. **Staging.** Mods extracted into per-mod directories. At import, casing is
   normalized to a canonical scheme and a mapping recorded, so Nexus's
   mixed-case archives become consistent and the runtime resolver does less work.
2. **Launch wrapper** (`eidos-launch`). On "Play", it:
   - creates a user namespace (for unprivileged mount) + mount namespace (for
     isolation),
   - starts the Eidos FUSE union mounted at the Linux path Wine maps as the
     game's install dir, with `lowerdir = modN..mod1, gameData`, writable
     Overwrite layer on top,
   - launches Proton / the game inside that namespace,
   - on exit, the namespace dies, the mount vanishes, the game dir was never
     touched, and new/modified files sit in Overwrite.
3. **Engine** (`eidos-fuse` + `eidos-core`). `eidos-core` is the pure resolver
   (this repo, unit-tested). `eidos-fuse` binds it to the kernel via the `fuser`
   crate with passthrough.
4. **Everything else** (load order, conflict display, FOMOD, Nexus integration)
   is app code. Reuse an existing native manager or build minimal. Not the hard
   part.

## The Proton integration wrinkle

This is the one genuinely fiddly piece that remains, and it is the *same* surface
RadTux and the Furglitch installer already negotiate. Steam launches the game
with its own logic, so we insert the wrapper via the Steam launch option
(`eidos %command%`), or launch Proton ourselves outside Steam. Where to mount
matters: the merged view goes at the Linux directory Wine exposes as the game's
install dir (under `steamapps/common/<game>` or via a Wine drive mapping). Unlike
the per-mod pain of the Wine approach, this is a one-time wrapper registration.

The wrapper must also be able to **re-enter the namespace** to launch tools
(xEdit, FNIS) into the same merged view, exactly as MO2 runs tools "through" the
VFS.

## Open risks we are not hiding

- **Performance** must be validated with a real 1000+ plugin load order and big
  texture packs, not assumed. Passthrough fixes data reads; lookups still cross
  the boundary.
- **Write / COW corner cases**: memory-mapped files, rename-over, in-place `.ini`
  rewrites, BSA/BA2 access, save integrity. Test these explicitly.
- **Casing beyond ASCII.** v1 folds ASCII case; Windows folds a wider Unicode
  table. Most game/mod paths are ASCII, but this is a documented limitation to
  close later.
- **Steam churn.** Proton updates and the launch-option dance are an ongoing
  integration surface.

## References

- `ModOrganizer2/usvfs` - the semantics we reproduce.
- `containers/fuse-overlayfs` - overlay semantics done entirely in FUSE.
- `fuser` (Rust) - the FUSE binding we will use.
- Limo, RadTux, modorganizer2-linux-installer - the existing compromises and the
  Proton integration surface to learn from.
