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

`read` not moving is the point, not a disappointment. It resolves nothing - it
`pread`s a handle that is already open - so it is the disk, and no amount of
cleverness in a filesystem makes a disk faster. After this change 82% of what
remains is the disk, which is where a mod manager should stop being the answer.

**What was slow.** Resolving one virtual path asked every layer in turn, and a
name that did not match byte-for-byte fell back to reading the whole directory to
find a case-insensitive match - which Bethesda games need constantly, since the
game asks for `ccbgssse001-fish.bsa` while the file is `ccBGSSSE001-Fish.bsa`.

The trap is who pays. Not the layer that has the file: every layer that does
not. A layer without the path misses on its *first* component and pays an
enumeration to be sure the name was not merely spelled differently. So a file
provided by one mod cost an enumeration in each of the others, on every lookup,
getattr and open. The cost grew with the thing users add most.

The read-only layers are now indexed once at mount, so a resolve is a hash
lookup. The Overwrite is deliberately *not* indexed - it is the layer that
changes - so there is no invalidation to get wrong.

## Directories

Two things were still asking the layers on every call.

**Opening one.** A measured session sent **516,301** directory opens and
enumerated almost none of them: Wine opens a directory to ask whether it folds
case, on every path that fails to resolve. The kernel can answer that itself, so
Eidos accepts `FUSE_NO_OPENDIR_SUPPORT` and stops being asked.

**Listing one.** `readdir` needs every layer's copy of a directory, merged, while
the path index only knew which layer wins - so a listing on a 50-layer stack cost
50 case-folding walks whatever the index held. The layers cannot change while
mounted, so their merge cannot either: it is computed once at mount and read as a
hash lookup.

| | before | today | |
|---|---|---|---|
| `opendir` | 516,301 | **1** | gone |
| `readdir` | 799 ms | 105 ms | **7.6x** |
| directory scans | 800,722 | 464,564 | 1.7x |

Measured on two sessions that happened to issue **exactly the same 2,063
listings**. That coincidence is why the number is worth printing: the workload
either side is identical, so the difference is the change and nothing else.

One honest caveat. Total handler time fell from 9,757 ms to 5,529 ms across those
two sessions, but a large part of that is `read` (7,485 -> 4,011 ms) and that is a
warmer page cache, not this work. What is attributable is `readdir` and the
scans.

## Checking it rather than trusting it

The index is all-or-nothing and built in silence: `LayerStack::new` either gets a
complete map of the read-only layers or `None`, after which every query walks
them exactly as before. Nothing in a session log tells the two apart, so a stack
that quietly fell back looks identical to one that is working - while paying the
old cost.

```sh
cargo run --release -p eidos-core --example index_health  -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example index_agrees  -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example listing_cost  -- <mods-dir> <overwrite-dir>
cargo run --release -p eidos-core --example resolve_cost
```

`index_agrees` is the one that matters. It requires the indexed and the walked
answer to be identical on every path AND every listing of a real instance -
37,127 paths and 1,492 directories, zero disagreements.

It is also what caught a bug the index had been hiding. A mod shipping both
`meshes/` and `Meshes/` - ext4 keeps them apart, the merged view must not - lost
everything under the second spelling, because the resolver took the exact-case
match for a component and committed to it. On a real instance that hid 74 files
of one mod with no error anywhere. The index never had the bug, because it walks
every directory it finds; the fallback documented as never wrong was the half
that was wrong. Both agree now, and there are tests that fail if they stop.

`EIDOS_NO_INDEX=1` forces the walk, for when the difference between the two
answers is the thing being debugged.

## Reading the counters yourself

```sh
EIDOS_FUSE_STATS=1     # op counters and per-handler timings, dumped at unmount
```

Set it in the game's Steam launch option, play, and quit normally - the report is
written when the mount goes away, so a killed game leaves nothing. There are two
mounts (the Data union and, when a mod provides root-level files, the game-root
one), so there are two reports; the big one is the Data union.

The counters distinguish `opendir` from `readdir` on purpose. A session once
reported 516,301 of the former against 26,999 of the latter, and the number that
mattered - how much of that was Wine probing rather than enumerating - was
invisible until they were counted apart.

See [architecture.md](architecture.md) for how the resolver and the index are
built, and [troubleshooting.md](../guide/troubleshooting.md) for the environment switches.
