# Troubleshooting and diagnostics

Everything for the day the game sees something the filesystem does not agree
with: the environment switches, how to read the op counters, the known issues
and their history, and the passthrough story.

### Diagnosing the VFS

Two environment variables exist for when the game sees something the filesystem
does not agree with:

```sh
EIDOS_FUSE_STATS=1                  # op counters, dumped at unmount
EIDOS_FUSE_NO_CACHE=1               # every kernel-side cache off
EIDOS_FUSE_NO_CACHE=attr,neg,keep,dir   # or name them individually
```

The granular form is what found the crash described in troubleshooting.md: turning
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


### Is the layer index actually in use?

The index is all-or-nothing and built in silence: `LayerStack::new` either gets a
complete map of the read-only layers or `None`, after which every query walks
them exactly as before. Nothing in a session log tells the two apart, so a stack
that quietly fell back looks identical to one that is working - while paying the
old cost.

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

## Known issues

**A mod that spells one directory two ways lost everything under the second.**
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

Worth knowing for the shape of it: the path index never had this bug, because it
walks every directory it finds. It had been quietly returning files the fallback
could not, which is the wrong way round - the fallback is the answer that is
meant to be never wrong.

**DynDOLOD's LODGen dies leaving an empty log.** Fixed by `dotnet10`; see
[tools.md](tools.md). The symptom is unmistakable: `LODGen_SSE_<world>_log.txt`
holding a version banner, a `.NET Version:` line and nothing else, for every
world, and a dialog saying only "failed to generate object LOD for one or more
worlds". The cause is Wine's Mono answering for .NET Framework, and no amount of
installing .NET Framework fixes it - Proton replaces `mscoree.dll` with a symlink
into its own tree on every prefix update.

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
