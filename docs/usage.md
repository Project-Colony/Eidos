# Using Eidos

The practical manual: the CLI, the GUI, the Steam launch option, building from
source, and the proof-of-concept script. For what to do when something looks
wrong, see [troubleshooting.md](troubleshooting.md).

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


Why the old `setcap` advice is gone - and why FUSE passthrough ships off - is
explained in [troubleshooting.md](troubleshooting.md#why-passthrough-is-off-by-default).

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
crates/eidos-install    mod installer: 7-Zip extract + Simple wrapper-strip + Root split + meta.ini
crates/eidos-fomod      FOMOD scripted-installer parser + condition/flag engine
crates/eidos-gamefeatures  BSA/archive invalidation + per-profile INIs/saves at launch
crates/eidos-gamedef    declarative per-game descriptor (one row per game; MO2 schema)
crates/eidos-ini        shared low-level INI primitives (newline / section / key / edit)
crates/eidos-nexus      Nexus Mods: v1 API client, nxm:// downloads, update checks
docs/architecture.md    the design and the tradeoffs behind it
scripts/poc-overlay.sh  runnable proof that the "virtualize under Wine" thesis
                        holds with native primitives, no root required
```
