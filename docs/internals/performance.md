# Performance: what was slow, and what it costs now

Every figure here comes from a real Skyrim SE instance played normally, not from
a benchmark. Where two sessions are compared, what makes the comparison worth
printing is said alongside it.

Loading a save took about twenty seconds. It now takes six to seven, and cell
changes are immediate.

## Resolving a path

| | before | after | |
|---|---|---|---|
| `exists()` probes | 6,408,527 | 481,826 | **13x** |
| directory scans | 5,608,084 | 335,493 | **17x** |
| `lookup` | 1627 ms | 276 ms | 5.9x |
| `getattr` | 505 ms | 77 ms | 6.6x |
| `open` | 1150 ms | 236 ms | 4.9x |
| `read` | 3173 ms | 3459 ms | unchanged |

`read` did not move, and was not expected to. It resolves nothing: it `pread`s
a handle that is already open, so its time is the disk's. After this change 82%
of what remains is disk time.

Resolving one virtual path used to ask every layer in turn, and a name that did
not match byte for byte fell back to reading the whole directory to find a
case-insensitive match. Bethesda games need that constantly, since the game asks
for `ccbgssse001-fish.bsa` while the file is `ccBGSSSE001-Fish.bsa`.

The cost fell on every layer that did not have the file. A layer without the
path misses on its *first* component and pays an enumeration to be sure the name
was not merely spelled differently. So a file provided by one mod cost an
enumeration in each of the others, on every lookup, getattr and open, and the
cost grew with the number of mods.

The read-only layers are now indexed once at mount, so a resolve is a hash
lookup. That index needs no invalidation, because those layers do not change
while mounted. The Overwrite, the layer that does change, has a separate index
that its own write methods keep current: creations insert into it, and deletes,
renames and truncating rewrites drop it for a rebuild on next use.
`EIDOS_NO_OW_INDEX=1` turns that one off. [architecture.md](architecture.md)
has the details and the bench.

## Directories

Two things were still asking the layers on every call.

The first was opening one. A measured session sent 516,301 directory opens and
enumerated almost none of them: Wine opens a directory to ask whether it folds
case, on every path that fails to resolve. The kernel can open a directory
without the daemon, so when it advertises `FUSE_NO_OPENDIR_SUPPORT`, Eidos
answers the first `opendir` with ENOSYS and the kernel sends no more.
`EIDOS_FUSE_OPENDIR=1` brings back the per-handle path, for bisecting a
directory bug against it.

The second was listing one. `readdir` needs every layer's copy of a directory,
merged, while the path index only knew which layer wins, so a listing on a
50-layer stack cost 50 case-folding walks whatever the index held. The layers
cannot change while mounted, so their merge cannot either: it is computed once
at mount and read as a hash lookup.

| | before | today | |
|---|---|---|---|
| `opendir` | 516,301 | **1** | gone |
| `readdir` | 799 ms | 105 ms | **7.6x** |
| directory scans | 800,722 | 464,564 | 1.7x |

The `readdir` and scan rows come from two sessions that happened to issue
exactly the same 2,063 listings. Since the workload on either side is
identical, the difference is the change and nothing else.

Total handler time fell from 9,757 ms to 5,529 ms across those two sessions,
but a large part of that is `read` (7,485 -> 4,011 ms), which a warmer page
cache explains, not this work. Only `readdir` and the scans can be put down to
the change.

## Checking it rather than trusting it

The index is all or nothing: `LayerStack::new` either gets a complete map of
the read-only layers or `None`, after which every query walks them exactly as
before, at the old cost. A mount whose index was declined says so once on
stderr:

```
eidos-fuse: layer index off (<reason>); every lookup walks all layers
```

and every union whose index was built prints its size and build time:

```
eidos: data union index: N layer(s), N entries, N dirs, built in N ms
```

For a run the window started, stderr is `<instance>/logs/run-<stamp>.log`. The
examples below check the index without launching the game and without trusting
those lines:

```sh
cargo run --release -p eidos-core --example index_health  -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees  -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost  -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example resolve_cost
cargo run --release -p eidos-core --example resolve_bench -- <layers.txt> <overwrite-dir> [samples-per-class]
```

`index_agrees` requires the indexed and the walked answer to be identical on
every path and every listing of a real instance: 37,127 paths and 1,492
directories, zero disagreements.

`resolve_bench` splits a session's directory scans between the Overwrite
pre-check and the index fallbacks that walk the layers. `layers.txt` lists one
directory per line, highest priority first and the game's own data directory
last, the order the mount uses. The workload is three classes of equal size:
existing paths spelled exactly, the same paths case-mangled, and paths that do
not exist.

`index_agrees` is also what caught a bug the index had been hiding. A mod
shipping both `meshes/` and `Meshes/` (ext4 keeps them apart, the merged view
must not) lost everything under the second spelling, because the resolver took
the exact-case match for a component and committed to it. On a real instance
that hid 74 files of one mod with no error anywhere. The index never had the
bug, because it walks every directory it finds; the wrong half was the fallback,
the path documented as always correct. Both agree now, and tests fail if they
stop agreeing.

`EIDOS_NO_INDEX=1` forces the walk, for when the difference between the two
answers is the thing being debugged.

## Reading the counters yourself

```sh
EIDOS_FUSE_STATS=1     # op counters and per-handler timings, dumped at unmount
```

Set it in the game's launch option, play, and quit. Each union prints its
report to stderr when it unmounts. For a run the window started, that is
`<instance>/logs/run-<stamp>.log`; with `eidos play <id> -- %command%` it goes
wherever the store sends the wrapper's stderr. The window's Stop, or a SIGTERM
to `eidos`, ends the game's process tree and still unmounts, so the report is
written. Only killing the `eidos` process itself (SIGKILL) loses it.

The Data union always mounts. The game-root union mounts too when a mod
provides root-level files, when `overwrite/Root` is not empty, when the game
keeps its INIs in the game folder, when a profile redirect lands inside the
game root, when the instance runs a game copy, or during a test run, and then
there are two reports. The big one is the Data union's. A test run also mounts
one-layer unions over the My Games folder and the saves, each with a report of
its own.

The counters distinguish `opendir` from `readdir` on purpose. A session once
reported 516,301 of the former against 26,999 of the latter, and the number that
mattered (how much of that was Wine probing rather than enumerating) was
invisible until they were counted apart.

The window has its own switch:

```sh
EIDOS_GUI_TIMING=1     # GUI steps over 16 ms, written to the gui log
```

It logs each message handler, the archive-worker scheduling and the health
checks that run after it, and the building of each frame's widget tree, when one
of them takes longer than 16 ms. Startup gets two lines of its own: the whole of
`App::new`, and the conflict walk inside it, which reads every layer before the
first frame. Lines name the message variant only, never its contents. At most
eight such lines are written per second. Once a second, a summary line gives
the number of frames built, the slowest one, and how many slow steps were left
out. Layout and drawing happen inside iced and are not timed. A dev build with
iced's `debug` feature covers those.

See [architecture.md](architecture.md) for how the resolver and the index are
built, and [troubleshooting.md](../guide/troubleshooting.md) for the environment
switches.
