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
used first) in `~/.config/Colony/Eidos/instances.ini`; the GUI's welcome screen lists
them to open with one click, the Steam launch lands on the one you last played,
and the `nxm://` handler downloads into it. Two caveats worth knowing: moving a
portable folder keeps everything except tool entries you registered with
absolute paths into the old location (re-add those), and the shared runtime
cache (`~/.local/share/Colony/Eidos/runtimes/`) deliberately stays machine-global -
a 78 MB .NET host is not per-instance.

Eidos keeps its own files under `Colony/Eidos`, the layout every program in the
Colony family uses: `~/.config/Colony/Eidos/` for what you chose (preferences,
your Nexus session, your instance list, the game and add-on definitions you
wrote), `~/.local/state/Colony/Eidos/logs/` for session logs, and
`~/.local/share/Colony/Eidos/` for what Eidos downloaded. An older Eidos kept
these in `~/.config/eidos/` and `~/.local/state/eidos/`; the first launch after
upgrading **copies** them across and says so in the log. The old directories are
left exactly as they were - nothing is deleted, so a bad upgrade cannot cost you
a sign-in - and you can remove them yourself once you are satisfied.

Your mods are not part of that. A global instance still lives at
`~/.local/share/eidos/<game>/`, and a portable one wherever you put it, because
those paths are written into your instance list and possibly into a Steam launch
option: moving them would break a link Eidos does not own both ends of.

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

### Columns, sorting and grouping

The list draws four columns out of the box and offers eight: Category, Content,
Version, Author, Installed, Nexus id, Game, Flags. Tick them in the View menu.
The default is not all eight on purpose - a list with every column showing has
no room left for the NAME, which is the column you are actually reading.

Click any heading to sort by it. Clicking again reverses, and a third click
returns to **load order**, which matters more than it sounds: load order is the
only order in which the list can be dragged, because an insertion gap addresses
the real list while a sorted row is somewhere else entirely. While a sort is on,
the insertion strips are not drawn and a drag is refused rather than landing
somewhere nobody aimed at - the same thing MO2 does, and for the same reason.
The View menu says so and offers the way back.

The View menu can also **group** the whole list, by category or by source (from
Nexus, or installed by hand). Group headers are not separators: there is nothing
behind them to rename, colour or move, they fold, and the count stays on the
header when folded. Separators leave the list under a sort or a grouping - a
separator heads the rows that follow it in load order, and both have moved them.

### Mouse and keyboard

Double-click a mod for Information, Ctrl+double-click for its folder,
Shift+double-click for its Nexus page. Ctrl+F puts the caret in the filter box.
Typing a letter jumps to the next mod starting with it, and pressing it again
walks the rest rather than sticking on the first. None of them can land on a row
the filter, a folded separator or a folded group is hiding - moving a highlight
you cannot see is how the next Space toggles a mod you were not looking at.

"Collapse others" on a separator's menu folds every group but that one. During a
drag, resting on a folded group opens it, so a mod can be dropped inside without
abandoning the drag first - resting, not brushing past.

### What the list tells you about a mod

Two advisory flags, both a glyph with the explanation on hover. **No valid game
data** means nothing at the top of the mod looks like something this game loads;
it may need its folders moved up a level, or it may not be a mod for this game.
**Another game** means the mod's own `meta.ini` names a different one. Neither
blocks anything - the mod still deploys - and "Mark as valid" on the row menu
silences either, through MO2's own `validated=` key, so a mod you have vouched
for in one manager arrives quiet in the other.

The layout check is deliberately generous: a `Root/` tree counts, an unreadable
folder counts, an empty one counts. A wrong warning on a five-hundred-row list
is worse than a missing one.

### Backing a mod up before you touch it

"Back up this mod" copies its folder aside as `<name>_backup` (then `_backup2`,
and so on - a backup never replaces the previous one). The copy is **inert**: it
is not a mod, its checkbox does nothing, and it contributes nothing to the merged
view, because ticking it would deploy two copies of one mod over each other.
"Restore this backup over the mod" puts it back, in two clicks; the current
contents are moved aside first and only discarded once the copy has succeeded.

**Data** is a real tree of the merged view, expanded one level at a time so
opening a node costs one directory read per layer that has it rather than a
recursive walk of every enabled mod. It is answered by the SAME layer stack the
mount serves from, so whiteouts and hidden files are respected and the tab
cannot disagree with what the game will see. Filter it by name, narrow it to
contested files only, sort out what is where with the Size and Modified columns,
and Reveal any row in a file manager. **Plugins** is the
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
touching priorities. The filetree also does the ordinary file operations: new
folder, rename, delete, open. They all go through one resolver that refuses
anything which is not a plain path inside that mod - no `..`, no absolute path,
and no component that is a symlink, since following one would put a delete
outside the mod folder entirely. Renaming replaces the last component only, so it
can never become a move, and it refuses a name already taken rather than
replacing that file in silence. Delete takes two clicks; it is the one action
here that clicking again cannot undo.

**View** on any row in the filetree or the Data tree previews the file: images
and text. Not DDS or NIF - those need a block decoder and a renderer this tree
does not have - but they say so rather than showing an empty box, and point at
Reveal. Text is read as far as 64 KB and says when it stopped, because a preview
is a glance and a Papyrus log can be a hundred megabytes. **INI Tweaks** lists the fragments a mod ships in its
`INI Tweaks/` folder; the enabled ones are merged into the profile's game INI at
launch, in priority order, and taken back off when the run's INIs are captured -
otherwise a tweak silently becomes a setting and disabling it would do nothing.

A download can be **dragged from the Downloads list onto a position in the mod
list** to install it at that priority, and archives or folders dropped onto the
window from a file manager install too (that half needs an X11 or XWayland
session - winit implements file drops for X11 only). Downloads themselves can be
paused and resumed: pausing stops the transfer and keeps the partial, and Resume
re-resolves a fresh link and continues from where it stopped.

The Downloads tab is an archive **library**, not a transfer queue. Filter it by
name (the friendly mod name too, so "skyui" finds
`SkyUI_5_2_SE-12604-5-2SE.7z`), sort by newest, name, size or state, and **hide**
an archive you are done with - which keeps the file and only drops the row, so
putting a book away is not burning it. "Show hidden" brings them back, and the
same button unhides. "Remove N installed" deletes the archives of mods you have
already installed, in two clicks, and only the ones **on screen**: the filter is
how you said which ones you meant.

### Nexus collections

Paste a collection link - or click one on the site - and Eidos lists the
revision's members, each joined against this instance: installed, downloaded, or
missing. It **reads** a collection; it does not install one, and the pane says
so. Four things make an installer dishonest rather than merely hard here: the
members are ordinary Nexus files needing a per-file key that only a premium
account can mint outside the site's own button; a full install is three API
calls per member against a budget this client refuses to overspend; the
manifest's phases, rules and replayed FOMOD answers could not be verified
against a real published Bethesda collection, and guessing produces a load order
that looks right and is not. Reading costs one request and is exact.

A collection can only be read against **its own game**. Open a Skyrim collection
with a Fallout 4 instance loaded and it refuses by name rather than joining the
members against the wrong mod list, where every "installed" and every "missing"
would be noise wearing the shape of an answer.

### Offline mode

**Settings -> Nexus -> Offline** stops Eidos contacting Nexus at all. Update
checks, sign-in, downloads and collections say so instead of failing with a
connection error. It is off unless you turn it on - a settings file written by an
older Eidos has no such key, and reading a missing one as "on" would cut the
network for everybody who upgrades.

**Preferred servers** ranks the CDN nodes a download prefers, best first. Only a
premium account is ever handed more than one mirror to choose between, so for
everyone else Nexus picks and this changes nothing. It is an ordering, not a
filter: if nothing you named is on offer today the download still happens, from
whichever node Nexus offered first.

**Categories** are editable, not just displayed: assign them to one mod or a
whole selection, edit the catalog itself from the same dialog, and pull the
game's official category list from Nexus. Both catalog files are MO2's own
(`categories.dat` and `nexuscatmap.dat`), so a shared instance keeps one catalog.

**View -> INI editor** edits the profile's game INIs - the copy that persists,
rather than the one buried in the Proton prefix that is overwritten at every
launch. **View -> Log** reads the session logs. **View -> Extensions** lists
your own add-ons; see [extensions.md](extensions.md).

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

## Extensions

Eidos can be extended without being rebuilt: a TOML manifest in
`~/.config/Colony/Eidos/addons/` adds a tool to the Extensions list or a check to the
Health tab. Nothing is loaded into Eidos - an extension is a program it runs.
See [extensions.md](extensions.md).
