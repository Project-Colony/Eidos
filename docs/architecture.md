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

We build the engine as a FUSE union filesystem in Rust. Rationale, against the
three goals (complete / performant / stable long-term):

- **Most complete.** Only a controlled implementation gives exact Windows-style
  case-insensitivity, precise Overwrite/write-redirection, correct handling of
  odd path probes, and no lowerdir wall (mods resolve through an in-memory index,
  not a kernel layer list).
- **Stable long-term.** Counterintuitively, owning the FS logic is *more* stable
  here: our behaviour is pinned by a test suite and does not shift when kernel
  OverlayFS semantics change. One engine we own beats a two-layer stack whose
  interaction we debug. `containers/fuse-overlayfs` made exactly this choice (do
  it all in FUSE) and is battle-tested under rootless Podman.
- **Fast enough now.** The historical "FUSE is slow" objection was expected to be
  answered by **FUSE passthrough** (merged in Linux 6.9), where the daemon
  resolves which real file backs a path and subsequent reads go straight
  kernel-to-disk. That turned out to be unusable in practice - it stops the game
  opening its archives and plugins entirely (see the caching section below), so
  it ships off and reads come from a cached backing fd via `pread` instead. What
  carries the load in its place is the metadata path: a game's startup cost is
  dominated by path probes, not bulk transfer, and those are answered kernel-side
  by negative dentries, long entry/attr TTLs, `FOPEN_CACHE_DIR` and the `.ciopfs`
  case-fold marker. Still: **measure** against a real heavy load order before
  claiming victory - and the read path in particular is now unmeasured without
  passthrough.

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

### Provisioning the prefix (DLLs + tool prerequisites)

A modded Bethesda game (and its tools) needs some Windows runtime bits present in
the Proton prefix that no Proton flavour supplies. Eidos owns this, in
`eidos-gamefeatures`:

- **Bundled native DLLs** (`native_dll`): Microsoft's native `d3dcompiler_47`
  (graphics mods import it for runtime HLSL compilation; Wine's builtin is a broken
  stub) and the DirectX helpers tools use (`d3dx9_43` / `d3dx11_43` /
  `d3dcompiler_43`). Detected by PE-import scan or declared per-tool, deployed into
  the prefix `system32`/`syswow64` (arch-aware) - unlinking Proton's builtin symlink
  first so the shared install is never written through, backing up any displaced
  file, idempotent, and forced native via `WINEDLLOVERRIDES=...=n,b`.
- **Installer verbs** (`prereqs`): the .NET / vcrun runtimes tools like Synthesis
  need can't be file-copied, so Eidos runs the system `winetricks` pointed straight
  at Proton's own `wine` + the game prefix (the NaK approach - this bypasses Steam's
  pressure-vessel and the protontricks + Proton-GE mismatch). Because these download
  from Microsoft, they run only on the user-consented `eidos prereqs --install`, and
  a per-instance sentinel makes re-runs no-ops.

Tools run in the **game's own prefix**, so one provisioning pass covers the game
and every tool.

## Open risks we are not hiding

The read-write daemon is now hardened and covered by a real-mount integration
suite (merge, case-insensitive reads, copy-up, deletes/whiteouts, rename,
readdir, rmdir semantics, symlinks, a multi-megabyte chunked read, and a
writable shared-mmap round-trip) plus resolver and inode-table unit tests.
Writable `MAP_SHARED` mmap round-trips correctly, with `setattr` guarded to
refuse a path that no longer resolves so the kernel's attribute flush on an
unlinked inode cannot resurrect a deleted file. `writeback_cache` is deliberately
**off**: it makes writable shared mmap marginally cheaper but breaks loading
Windows DLLs from the mount (the loader dirties `MAP_PRIVATE` copy-on-write image
pages and the kernel mishandles those over a FUSE backing) - and loading SKSE
plugin DLLs is essential. **FUSE passthrough** (the kernel serving the real
backing file natively, which needs `setcap cap_sys_admin+ep` and a bare mount
namespace) is negotiated only on request and is **off by default**: measured A/B
on Skyrim SE 1.6.1170 / kernel 7.1.4, turning it on makes the game fail to open
152 of its own files - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp` - all with
`STATUS_ACCESS_VIOLATION`, against 0 with it off, so no mod content loads at all.
The failure is generated by the kernel after the daemon replies
`opened_passthrough` and is therefore invisible to it (a full failing session
under `EIDOS_FUSE_TRACE=open` logs zero failed opens and zero refused backing
files), which is what made it expensive to diagnose. Reads are served by the
daemon's own `pread` instead; `EIDOS_FUSE_PASSTHROUGH=1` restores the old
behaviour. Other write corners are covered too: rename-over, in-place rewrites
via copy-up, and save integrity (`fsync` flushes the backing fd).

**Validated end-to-end on a real heavily-modded Skyrim SE (110 mods) under
Proton:** all ~50 SKSE plugin DLLs load and run via the mount (CommonLibSSE-NG
included), each writing its config into the Overwrite layer. Two non-obvious
parity requirements with MO2/usvfs were needed: the game is launched with its
**working directory = the game root** (CommonLibSSE-NG opens its address library
by a CWD-relative path), and `readdir` returns entries in **case-insensitive
alphabetical (NTFS-like) order** (the Creation Engine's loose-file indexer
assumes sorted enumeration; raw FUSE order crashed it at the main menu). What
that does **not** yet close:

- **That DLL run predates the passthrough default flip and was made with
  passthrough ON**, which we now know prevents the game loading any archive or
  plugin - so it validated DLL image-mapping, not a playable load order. Rootless,
  a small load order has since reached gameplay with its plugins, archives and
  Papyrus scripts live and SKSE running, but **~50 SKSE plugin DLLs image-mapping
  without passthrough is not re-validated**. If relocation-heavy DLLs turn out to
  need it, the fix is to make passthrough per-file (DLLs only) rather than global,
  which is why the switch was kept rather than the code deleted.
- **Performance** must be validated with a real 1000+ plugin load order and big
  texture packs, not assumed. Reads now use a cached backing fd (`pread`), so the
  per-read cost is gone, but path resolution still stats the real filesystem on
  every lookup: the in-memory index the engine is designed around is not built
  yet, so the "no lowerdir wall" property is still aspirational under a deep stack.
  Writable mmap is correct in a focused test but still worth re-checking under a
  real game's mmap-heavy load.
- **Directory rename across the lower boundary.** File rename works; renaming a
  directory that lives only in a lower layer (a recursive copy-up of the subtree)
  is not implemented yet. usvfs itself models rename as copy + delete here.
- **Casing beyond ASCII.** v1 folds ASCII case; Windows folds a wider Unicode
  table. This is at parity with usvfs (its tree uses ASCII `_strnicmp`), but a
  documented limitation to close later.
- **Steam churn.** Proton updates and the launch-option dance are an ongoing
  integration surface.
- **Telling Wine the mount folds case** (fixed). Wine has no API for the
  question, so `get_dir_case_sensitivity` sniffs for the marker CIOPFS leaves in
  each directory. Without it Wine assumes case-sensitive and answers every
  mis-cased lookup by reading the entire directory to search for a match - which,
  for a Bethesda game asking for `data/ccbgssse001-fish.bsa` against
  `ccBGSSSE001-Fish.bsa`, is nearly every asset. Measured before the fix: 4471
  marker probes and 2236 full directory re-reads in eight seconds, 195796
  enumerations of `Data` in ninety, the game frozen at 240 MB resident and the
  daemon at 92% of a core. Eidos had folded case in `resolve_read` since the
  first commit; the entire cost was never saying so. The lesson generalises past
  this bug: a VFS has to advertise its semantics, not merely implement them,
  because the layer above will otherwise pay to discover them empirically.
- **`opendir` is not a promise to enumerate** (fixed). It built the merged
  listing eagerly, and Wine opens a directory purely to `stat` that marker inside
  it - so the daemon paid a full multi-layer merge plus an NTFS-collation sort
  220000 times in ninety seconds for listings nobody read. The snapshot is taken
  by the first `readdir` now; offsets stay stable because it still happens
  exactly once per handle.
- **The merged listing is memoised by path**, not only per handle, so a directory
  enumerated repeatedly pays for one multi-layer walk and one NTFS-collation sort
  instead of one per enumeration. Justified by the same immutability as the long
  entry TTL: mod layers do not change for the life of the mount, and anything
  written through the mount goes through a handler that drops the affected parent
  (`dir_changed`; `grep` for it is the audit that no mutating handler is missing
  one). Inodes are deliberately NOT cached with the listing - `forget` drops an
  inode when the kernel releases its last reference and a later `intern` of the
  same path mints a fresh number, so cached entries would hand out inodes the
  daemon no longer knows. Interning is a hashmap hit against real disk I/O, so it
  is redone per enumeration and is always correct.
- **The op counters distinguish opening a directory from listing one.** They did
  not: the counter printed as `readdir` was incremented in `opendir`, which made a
  measured 516301 read as half a million enumerations when almost none of them
  enumerated. Directory opens, enumerations, merges, cache hits and `.ciopfs`
  marker lookups are now separate, plus `probe` for handles closed without a single
  `readdir` - the only honest measure of Wine's case-sensitivity probing, since the
  negative-dentry cache absorbs the lookups while every open still reaches us.

### Caching, and one flag that had to go

Metadata caching is what keeps a Bethesda startup from stalling in the daemon,
and three of its four kernel-side pieces are on: negative dentries for the paths
Wine probes and never finds, a long positive entry/attr TTL (mod layers are
immutable for the life of a mount), and `FOPEN_CACHE_DIR` on `opendir`.

`FOPEN_KEEP_CACHE` is **off**, and the reasoning that put it there is worth
recording because it was wrong in an instructive way. The argument ran: mod files
do not change behind the mount, the layers are immutable, every write goes
through this daemon - therefore the kernel can keep its page cache across opens.
Both premises are true. The conclusion does not follow, because a lower-layer
file does not keep its *identity*: the first write copies it up, so one virtual
path - and one FUSE inode, since inodes are keyed on the path - ends up backed by
a different file on disk while the kernel still holds pages read from the old
one. Under passthrough the kernel serves those reads without consulting the
daemon, so nothing downstream can notice.

That explanation is also not sufficient, which is why the flag is off rather than
narrowed. Restricting it to files outside the overwrite layer was tried and
Skyrim still crashed, reading files that were never written to. The measurement
that settled it: with all four caches on, the game dies on a null dereference at
a fixed address seconds after the main menu, with zero mods installed; each cache
was then disabled individually and only this one changed the outcome. The cost of
losing it is nil - `EIDOS_FUSE_STATS` reports `read 0` for a full load, because
passthrough serves every byte in the kernel, so those pages were already cached
against the backing file.

The lesson generalises: `EIDOS_FUSE_NO_CACHE` now takes names
(`attr,neg,keep,dir`) rather than being all-or-nothing. A switch that only
answers "is it the caching?" costs a rebuild per hypothesis; one that answers
"which caching?" costs four launches.

## References

- `ModOrganizer2/usvfs` - the semantics we reproduce.
- `containers/fuse-overlayfs` - overlay semantics done entirely in FUSE.
- `fuser` (Rust) - the FUSE binding we will use.
- Limo, RadTux, modorganizer2-linux-installer - the existing compromises and the
  Proton integration surface to learn from.
