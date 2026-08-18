# Using Eidos

The practical manual: the CLI, the GUI, the Steam launch option, building from
source, and the proof-of-concept script. For what to do when something looks
wrong, see [troubleshooting.md](troubleshooting.md).

## Use it (CLI)

```sh
eidos games                       # supported games installed here (like MO2's list)
eidos init skyrimse               # create a modding instance
# ...drop each mod as a folder into <instance>/mods/ (the global instance lives
#    at ~/.local/share/eidos/skyrimse; `eidos init` prints yours)...
eidos install skyrimse mod.7z     # or install a downloaded archive (Simple / FOMOD)
eidos import skyrimse <mo2-profile>  # adopt an existing MO2 profile's order + plugin state
eidos sort skyrimse               # LOOT-sort the plugin load order
eidos play skyrimse               # show what would be mounted
eidos play skyrimse -- <command>  # run <command> with the mods mounted over the game
```

`eidos tool`, `eidos prereqs`, `eidos nexus`, `eidos nxm` and `eidos export` round
out the set; run `eidos` with no arguments for the full list.

### Instances: global and portable

Every command above addresses an instance. `skyrimse` names the **global** one -
stored centrally at `~/.local/share/eidos/skyrimse`, managed by Eidos. The other
kind is **portable**: a self-contained folder wherever you want it (a second
drive, a games partition), movable and isolated, exactly like MO2's portable
instances. Wherever a command takes a game id it also takes a portable
instance's folder:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # create a portable instance there
eidos install /mnt/games/EidosSkyrim mod.7z  # every command accepts the folder
eidos play /mnt/games/EidosSkyrim -- %command%
```

The folder is self-describing (its `eidos-instance.ini` names the game), so
nothing else is needed - and `EIDOS_INSTANCE=<folder>` in the environment
redirects a game id to that folder, which is handy in Steam launch options.
Portable instances you have created or opened are remembered (most recently
used first) in `~/.config/eidos/instances.ini`; the GUI's welcome screen lists
them to open with one click, the Steam launch lands on the one you last played,
and the `nxm://` handler downloads into it. Two caveats worth knowing: moving a
portable folder keeps everything except tool entries you registered with
absolute paths into the old location (re-add those), and the shared runtime
cache (`~/.local/share/eidos/runtimes/`) deliberately stays machine-global -
a 78 MB .NET host is not per-instance.

One place is refused outright: **inside a game's install folder** (the MO2
veteran reflex). Steam owns that tree - an update, a "verify integrity" or an
uninstall can rewrite or delete it, taking your whole setup along - and Eidos
mounts over the game root, so an instance in there would sit inside its own
mount target. The wizard, `eidos init` and `eidos play` all say no; put the
folder NEXT to the game instead (a sibling on the same drive gives you the
same convenience).

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
summary -> create -> main screen. The welcome screen also lists every known
existing instance (global and portable, last-used first) to open with one
click - it doubles as the instance switcher - and pointing the wizard at a
folder that already holds an instance ADOPTS it as-is instead of creating over
it (refusing outright if the folder belongs to another game).

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

Eidos opens on the game's instance - the one you last used, so a portable
instance is found again just like the global one; click Run to launch it
through the merged view. (The Run button shows this exact line, with the
running binary's real path, if you press it outside Steam.)

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

## Tools

xEdit, BodySlide, DynDOLOD and friends run through the merged view inside the
game's Proton prefix:

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos prereqs skyrimse            # what the registered tools need, and its state
eidos prereqs skyrimse --install  # fetch whatever is missing
```

One thing to know before naming a tool: **the title decides which runtime DLLs
Eidos provisions for it** - `BodySlide` gets its DirectX libraries, `BS` gets
nothing. In the GUI the Executables dialog shows each prerequisite's real state
under the field, and the missing ones are buttons.

The table, the three prerequisite tiers, why DynDOLOD needs a .NET runtime that
winetricks cannot install, and why a tool installed as a mod is launched from the
merged path rather than its own folder are in [tools.md](tools.md).

Building from source and the repository layout are in
[../internals/contributing.md](../internals/contributing.md).
