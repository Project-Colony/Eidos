# Using Eidos

How to use Eidos: instances and game copies, the command line, test runs, the
window, Nexus collections and updates, moving an instance to another machine,
tools and extensions. For problems, see
[troubleshooting.md](troubleshooting.md); for building from source and the
repository layout, see [../internals/contributing.md](../internals/contributing.md).

## Instances: global and portable

Commands that take an instance accept a game id or a portable instance's
folder. `skyrimse` names the global instance, which Eidos keeps at
`~/.local/share/Colony/Eidos/instances/skyrimse`. A portable instance is a
self-contained folder wherever you want it (a second drive, a games partition),
and you can move it, as with MO2's portable instances:

```sh
eidos init skyrimse /mnt/games/EidosSkyrim   # create a portable instance there
eidos install /mnt/games/EidosSkyrim mod.7z  # every command accepts the folder
eidos play /mnt/games/EidosSkyrim -- %command%
```

The folder's `eidos-instance.ini` names the game, so the folder needs nothing
else. `EIDOS_INSTANCE=<folder>` in the environment redirects a game id to that
folder, for example in a Steam launch option. Portable instances you have
created or opened are remembered, most recently used first, in
`~/.config/Colony/Eidos/instances.ini`: the window's welcome screen lists them,
the Steam launch lands on the one you last played, and the `nxm://` handler
downloads into it.

Moving a portable folder by hand keeps everything except tool entries you
registered with absolute paths into the old location. `eidos pack` and
`eidos unpack` ([below](#moving-an-instance-to-another-machine)) fix those; a
plain `mv` does not. The runtime cache in `~/.local/share/Colony/Eidos/runtimes/`
is shared by every instance on the machine, so a 78 MB .NET host is not copied
into each one.

One place is refused: inside a game's install folder, where MO2 users often put
their instance. Steam owns that tree, and an update, a "verify integrity" or an
uninstall can rewrite or delete it along with your setup. Eidos also mounts over
the game root, so an instance in there would sit inside its own mount target.
The wizard, `eidos init`, `eidos play` and `eidos shell` refuse it. Put the
folder next to the game instead; a sibling folder on the same drive is as
convenient.

### Where Eidos keeps its files

Eidos keeps its own files under `Colony/Eidos`, the layout every program in the
Colony family uses: `~/.config/Colony/Eidos/` for what you chose (preferences,
your Nexus session, your instance list, the game and add-on definitions you
wrote), `~/.local/share/Colony/Eidos/` for global instances (`instances/`),
downloaded runtimes and session logs (`logs/`), and `~/.cache/Colony/Eidos/` for
the LOOT masterlists, which are downloaded again whenever they are missing.

When it finds files in the layout of an older Eidos (`~/.config/eidos/`,
`~/.local/share/eidos/` and `~/.local/state/`), the first launch copies them
across and says so in the log. A global instance is moved with hard links
instead of a byte copy, so its mods take no extra space, and its tool entries
are repointed at the new folder. Nothing in the old directories is changed or
deleted; remove them yourself once the new layout works. If a copy cannot be
made (a full or read-only disk, an old instance folder on another drive, a game
still running from it), Eidos keeps using the old location and tries again on
the next launch. A Steam launch option that still names the old instance folder
opens the moved one.

The hard links save space only at first. A mod or download you remove or
reinstall afterwards keeps its old copy on disk through the old instance folder,
and deleting `~/.local/share/eidos/<game>/` gives that space back.

A portable instance is never moved: it stays wherever you put it.

## The command line

```sh
eidos games                                # supported games installed here, like MO2's list
eidos init skyrimse [folder]               # create an instance; with a folder it is portable
eidos init skyrimse --game-path <folder>   # use the copy installed in <folder> when several are installed
eidos install skyrimse mod.7z [name]       # install an archive (Simple, FOMOD, BAIN, OMOD)
eidos install skyrimse mod.7z --replace --backup  # reinstall over the mod (--merge installs over its files)
eidos import skyrimse <mo2-profile>        # take over an MO2 profile's mod order and plugin state
eidos sort skyrimse [--dry-run]            # LOOT-sort the load order (--update-masterlist refreshes it first)
eidos play skyrimse                        # show what would be mounted
eidos play skyrimse -- <command>           # run <command> with the mods mounted over the game
eidos shell skyrimse                       # a shell in the merged view; writes land in Overwrite
eidos check skyrimse [--json]              # Health checks without the window
eidos sessions skyrimse                    # the recorded game and tool runs, newest first
eidos sessions skyrimse diff [<id>]        # what changed since the last run that exited with code 0
eidos stock-game skyrimse                  # which game files the instance runs; freeze a copy or switch
eidos tool skyrimse                        # list the tools (add, rm and run: see Tools below)
eidos tool skyrimse run --exe <path>       # run an exe once through the merged view
eidos tool skyrimse import-mo2 [--write]   # the executables of a ModOrganizer.ini; --write adds them
eidos prereqs skyrimse [--install]         # what the tools need; --install fetches what is missing
eidos collection skyrimse <link>           # install a Nexus collection (--dry-run, --no-optional)
eidos nexus status                         # check the stored Nexus sign-in
eidos nexus update skyrimse [--apply]      # check for mod updates; --apply installs them
eidos nxm <link>                           # download a Mod Manager Download link
eidos nxm --register                       # make the site's Mod Manager Download button open Eidos
eidos export skyrimse -o mods.csv --active # MO2's CSV format; --active lists enabled mods only
eidos pack skyrimse backup.eidos           # the whole instance in one file
eidos unpack backup.eidos [folder]         # without a folder a global instance goes back to its central place
eidos --version                            # print the version
```

`eidos` with no arguments prints the list of commands. Most commands print
their own options when an argument is missing, and `eidos tool --help` lists
the tool subcommands. `eidos pack` takes `--dry-run`, `--level`, `--no-downloads`, `--no-recovery`
and `--force`, and `eidos unpack` takes `--info` and `--force`; both are
described [below](#moving-an-instance-to-another-machine). `eidos stock-game`
is described under
[Game files](#game-files-the-store-install-or-a-copy).

### Running a game through the merged view

`eidos play` mounts the instance's mods over the game's own `Data` directory
inside a private namespace, then runs the command through that view. The game's
own files are bind-stashed first, so the daemon still reads them unchanged.
Writes (saves, regenerated configs) land in the instance's `overwrite/` layer;
the game install and every mod folder stay byte-for-byte as they were.

Eidos needs no root. It mounts in a private user and mount namespace, with no
setuid helper, no system service and nothing to grant. An optional capability on
the `eidos` binary allows FUSE passthrough, which stays off unless
`EIDOS_FUSE_PASSTHROUGH=1` is set; see
[install.md](install.md#optional-fuse-passthrough). With the capability Eidos
uses a plain mount namespace instead of a user namespace, and the mods deploy
the same way in both.

### A shell in the merged view

`eidos shell <instance>` starts your `$SHELL` (or `/bin/sh`) in the game's folder as the game would see it: every enabled mod mounted over `Data`, Root mods over the game root, and everything written there landing in the Overwrite. `eidos shell <instance> -- <command>` runs one command instead, for example `eidos shell skyrimse -- ls Data/meshes` or a native tool such as NifSkope. The run holds the instance lock like a game session, so the window cannot save changes until you exit, and it is recorded in `eidos sessions` under the name `shell`, never as a game launch. Ctrl+C on a command stops the run the way Stop does: Eidos ends the command, captures the session and unmounts. In the interactive shell Ctrl+C belongs to the shell, as usual; type `exit` to end the run.

### Health checks from the terminal

`eidos check <instance>` runs the Health checks that need no window, so they can
run over SSH or in Game Mode and go into a bug report:

- the Proton prefix and its registry;
- plugin diagnostics and missing masters;
- the script extender and its plugins;
- a game update since the last clean launch;
- the Steam launch option;
- graphics settings (NVAPI, fullscreen, ENB);
- the crash log of the last run;
- `sTestFile` lines in the profile INIs;
- Starfield `.esp` and blueprint plugins;
- `vm.max_map_count` and the open files limit;
- the game copy the instance runs, and a `gamePath` in a list's
  `ModOrganizer.ini` that names another game folder;
- collection order rules that no longer hold;
- game files on NTFS, exFAT or FAT;
- BodySlide data paths;
- `Data` folders left in `Overwrite/Root`;
- font libraries that `Interface/fontconfig.txt` names and no mod provides;
- compressed audio in the BA2 archives of Fallout 4 and Fallout 4 VR mods.

Some cards are worked out by the window only, on the Diagnostics tab: patch
order, extender scripts a mod replaces, archives nothing loads, Nexus
requirements, plugins LOOT flags as dirty, the Proton build that runs the game,
the Display card and tool output older than its inputs.

The command prints the same system lines as **Copy report**, then one line per
finding, with your home folder shown as `~` and your login name as `<user>`. The
exit code is 0 when no problem was found (advice alone still gives 0), 1 when at
least one problem was found, and 2 when Eidos could not check: wrong arguments,
an instance it cannot open, or a game it does not detect.

`eidos check <instance> --json` prints one JSON object instead. Its top-level
fields are `eidos` (the version), `game`, `instance`, `system` (`proton`,
`flatpak_steam`, `passthrough`, `session`, `distro`, `kernel`, `dev_fuse`,
`fusermount3`, `max_map_count`, `open_files_hard`) and `findings`.
`max_map_count` and `open_files_hard` are numbers, or `null` when Eidos could
not read them. Each finding is a `code`, a `level` (`problem` or `advice`) and
its `args`; the Diagnostics tab shows the sentence for each code.

### What changed since the last good run

Every run through `eidos play`, `eidos shell` or `eidos tool run`, started from the window, a terminal or Steam, writes a record to `<instance>/logs/sessions/`: the profile, each enabled mod with its version and Nexus file ids, the plugin order, a checksum of each profile INI, the game executable's version, the Eidos version, the exit code and the saves the run created. The 50 newest are kept, and `logs/` stays out of a packed instance. `eidos sessions <instance> diff` compares the newest game run that exited with code 0 against the current state; `diff <id>` picks another record from the list. The list marks a [test run](#test-runs) as `test`, and a test run is never picked as the last good run. A run you stopped records exit code 143, so it is never picked either. Eidos does not detect crashes here yet, and a game that crashed under Proton can still exit with code 0: if the run it picks was one of those, name an earlier one with `diff <id>`.

## The mod list

As in MO2, each profile has its own `modlist.txt` (which mods are enabled, in
what priority), while the mod folders live in the instance's shared `mods/`. A
folder you drop into `mods/` by hand is not listed yet: it shows up disabled at
the highest priority, at the bottom of the window's list as in MO2, so it wins
its conflicts once you enable it.

Because the folders are shared, removing or renaming a mod changes every
profile: its line is dropped from, or renamed in, every profile's
`modlist.txt`, and each profile keeps its own state and order for the rest. A
profile left with no mods has no `modlist.txt` at all.

The window works from its own copy of the mod list and load order. If another
Eidos process writes either one while it is open (`eidos install` or
`eidos sort` in a terminal, a collection, a session Steam started, a second
window), the window's next edit is refused instead of overwriting that work. It
reloads from disk and says *Not saved: another Eidos process changed the mod
list. It has been reloaded - redo the change.* (or the same for the load
order); only that one click is lost, so make it again. Rename, New mod, Add
separator, Remove and turning the Overwrite into a mod check before touching
`mods/`, so a refused edit changes nothing on disk. The INI editor likewise
refuses to save over a file that changed since it opened it.

## Saves belong to the profile

Each profile keeps its own saves in `profiles/<name>/saves/`, and for the length
of a run that folder is mounted over the game's save folder in the Proton
prefix. The first launch of a profile copies the saves already in the prefix
into it, once, so a save you later delete from the profile stays deleted. After
each run the profile's newest saves are copied back into the prefix for Steam
Cloud; a prefix save Eidos did not put there is first kept in the profile as
`orphan-<time>-<name>`, never overwritten.

A save that lands in the prefix after that first copy without going through
Eidos (a Steam Cloud download from a Steam Deck, a launch without Eidos) stays
hidden while the profile's saves are mounted. Nothing deletes it: the launch
names it in the run's log (the instance's `logs/run-*.log` for a window launch,
Steam's console for a Steam launch), once each time that set of files changes,
and the Saves tab counts these saves on the line under its heading. To play
one, press Import on the Saves tab. Import copies each one with its co-save into
the profile and never replaces a profile save: a save whose name the profile
already uses is imported as `orphan-<time>-<name>`.

## Test runs

The Test run button next to Run starts the game the way Run does, but everything the game writes goes to `<instance>/test-run/` instead of your profile, the Overwrite and the prefix: new and changed saves, INI changes, `plugins.txt`, files written into Data or the game folder, and the logs in `Documents/My Games/<game>`. You can test with the profile's mods, without any mod, or with only the selected mods that are switched on, plus the mods whose plugins they need as masters.

When the game exits, Eidos lists those files in five groups (saves, INI changes, plugin state, Overwrite output, logs) and asks you to keep or throw away each one. Apply waits until every group that holds files has an answer. Plugin state is not offered when the run used another set of mods than the profile's. Keeping saves never deletes a save from the profile, even one the game deleted during the run. Keeping the Overwrite output also removes from the Overwrite the files the game deleted, and the list counts them first. Throwing anything away takes a second press. Decide later closes the list and leaves the files in `test-run/`, and the list opens again the next time you open the instance or press F5. If Eidos or the computer stops during a test run, the files stay there too and the list opens the same way.

When the window was not started by the game's store, Test run hands the game to that store (Steam, Heroic or Legendary), as Run does, and the run the store starts is the test run. If the store does not start the game, the request waits: the next game run of that profile, from Run or from the store, is the test run, and Cancel the waiting test run drops it. A test run covers only the folders listed above: other writes of the game, such as the Wine registry, go where they always go. Saves from a test run reach Steam Cloud only after you keep them and play a normal run.

### Finding the mod behind a problem

Find the mod behind a problem, on the Diagnostics tab, looks for the mod that causes a problem you can reproduce, such as a crash at a given place. Each step is a test run with half of the remaining suspects, the masters they need and the mods you keep always on: the switched-on mods selected in the list when you start, and the mod that provides the script extender. After each run you answer whether the problem happened. Not sure splits the suspects differently, and Run it again repeats the step. Answering, or pressing Run it again, throws away the files the step wrote, after a second press. To keep some of them, press See the files before answering: it opens the same list as after a test run. When the game crashed and a crash logger wrote a log, or the game exited with an error code, Eidos marks Yes as the likely answer. 200 suspects take about 8 runs.

The search is saved in `<instance>/bisect.json`, so you can close Eidos between steps, and it belongs to the profile it started in. Stop the search drops it after a second press. The result names the mod, or the set of mods that need each other's masters, and is kept in `<instance>/logs/bisect-<time>.txt`. Generated mods (a Nemesis or DynDOLOD output, a Bashed Patch) are built for the full list and can fail with half of it: keep them always on, or leave them off for the whole search.

## Wabbajack lists

Eidos has no Wabbajack installer. Install the list with CLF3 or Jackify, outside Eidos, then pick the install folder as a portable instance in the wizard. Eidos reads the folder as MO2 lays it out: `mods/`, `profiles/<name>/modlist.txt` and the plugin lists. It keeps the `ModOrganizer.ini` the installer wrote and never overwrites it, so the list's own executables stay registered.

Many lists add a `Stock Game` or `Game Root` folder that holds a patched copy of the game, and MO2 runs the game from there. When the wizard or `eidos init` adopts a list folder for the first time and that folder holds the game's executable, the instance runs the game from it too. The game still starts through Steam or the store that owns it, so nothing has to be moved. Eidos finds the folder from the `gamePath` in the list's `ModOrganizer.ini`, or by those two names. An instance adopted with an earlier Eidos keeps the store install until you switch it (see the next section). When that `gamePath` names a game folder the instance does not run, Diagnostics says so, because tools in Mod Organizer 2 mode, such as PGPatcher, read the game from there. CLF3 rebuilds BSA and DDS files with its own tools, so the result can differ from a Wabbajack install, and list authors do not support it.

## Game files: the store install or a copy

By default the game runs from its store install, the folder Steam, Heroic or Legendary keeps it in. An instance can run from a copy of that folder inside the instance instead. The mods sit on top of the copy, and the store install is only the place where Eidos mounts the result. The store still starts the game, so Steam keeps its overlay, its shader cache and its cloud saves.

The Game files card at the top of the Diagnostics tab shows which files the instance runs, with the game version of the copy and of the store install, and switches between them. In a terminal, `eidos stock-game <instance>` prints the same facts, and `use <folder>`, `store`, `freeze` and `delete --yes` do what the card's buttons do. `freeze` prints the size and the kind of copy, and `freeze --yes` copies.

**Freeze this version** copies the store install into `<instance>/Stock Game` and runs the game from it, so a store update no longer breaks script extender plugins: the update changes the store install, not the copy. The first press shows the size and what the filesystem does with it, and the second press copies. On a filesystem that can share blocks (reflink), such as btrfs or XFS, the copy takes almost no space until the store changes a file. On ext4, or when the game and the instance are on different filesystems, it is a full copy as large as the game folder. Eidos refuses while Steam reports the game busy or not fully installed, while a game or tool of the instance runs, when the instance already holds a list's game copy, a frozen copy or an unfinished one, and when the filesystem has less free space than a full copy needs. Heroic and Legendary copies are not checked for a running update. Stop, a failure, or a Steam update to the game during the copy leaves `Stock Game.eidos-tmp`, which the card offers to delete; it is never used.

A copy hides every file the store adds, changes or removes in the game folder after the copy was made: new Creation Club files, a new executable, new DLLs. The game, the plugin list, LOOT and the Health checks all read the copy. It does not hide what happens outside the game folder: an update can still install runtimes into the Proton prefix at the next launch, and Steam can switch the game to another Proton.

**Return to the store version** takes the copy out of the next launch after a second press. The copy stays on disk, and **Delete the copy** removes it, again after a second press. Delete is refused while the instance still runs the copy. Eidos never deletes a list's own `Stock Game` or `Game Root` folder. If the copy folder goes missing, or no longer holds the game's executable, launches are refused and Diagnostics names the folder, so the store version never runs by surprise.

Morrowind, and Oblivion with `bUseMyGamesDirectory=0`, keep their plugin list in the game folder, and Eidos does not run them from a copy yet.

## Installing a Nexus collection

```sh
eidos collection skyrimse nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest
eidos collection skyrimse rqhcxy --dry-run      # read it and say what would happen
eidos collection skyrimse rqhcxy --no-optional  # skip the members it marks optional
```

In the window, paste the link into **File -> Open a Nexus collection...**, or
click one on the site. Eidos lists the revision's members, each matched against
this instance: installed, downloaded or missing. *Install this collection* runs
the whole recipe on a worker thread, with the progress bar the pack and unpack
jobs use, and the bar becomes the report when it finishes. The window and
`eidos collection` run the same install.

With no link, **File -> Open a Nexus collection...** opens on the **Browse**
tab. It lists the collections published for the open instance's game, 20 at a
time, sorted by most downloaded, recently updated or newest, and the search box
narrows the list to collections whose name or text matches. Each row gives the
author, the latest revision, the number of mods, the downloads and a date.
**View** opens that collection's latest revision on the **Link** tab, the same
as pasting its link. Collections marked adult are left out unless your Nexus
account shows adult content, collections with no rating are always left out,
and the line under the list counts both. Browsing needs a signed-in Nexus
account, like the rest of the pane.

A collection opens only against an instance of its own game. A Skyrim collection
opened while a Fallout 4 instance is loaded is refused by name, instead of
matching its members against the wrong mod list.

### What Eidos reads from a collection

Almost none of a collection is visible through the Nexus API. The install
order, the answers the author gave each mod's scripted installer, which mod wins
a file conflict, which plugins load where and the binary patches are all in a
`collection.json` inside the collection's own archive. Downloading the same mods
and installing them with their default options gives a different game.

Eidos downloads the archive, reads `collection.json` and applies it:

| The collection says | Eidos does |
| --- | --- |
| `phase` | installs in that order, optional members last |
| `choices` | replays the author's answers to each FOMOD, and says so when it cannot |
| `modRules` | reorders the mod list so the right mod wins each file |
| `plugins` / `pluginRules` | merges the LOOT rules into your userlist and sorts |
| `INI Tweaks/` | installs them as a mod of their own |
| `tools` | tells you which tools it expects; creates nothing |

Members are switched on as they install, at the end of the mod list in install
order, before `modRules` moves them. A member left out of the mod list would
not load, and the report would not show it.

The state is written to `<instance>/collections/<slug>-<revision>.state.json`
after every member. It sits beside that revision's folder, never inside it, so
a collection cannot ship a state file of its own, and an install interrupted at
member 147 of 200 continues from 147. It is written to a temporary name and
renamed into place, so an interruption leaves the old record or the new one and
never half of either. If it ever cannot be read, Eidos stops and says so instead
of starting the whole collection over. Run the same command again.

Installing a newer revision of a collection you already have takes over the
previous revision's folders: an unchanged member is verified in place, a
member whose file the author updated is replaced inside its old folder, and a
member the new revision dropped is left installed and named in the report for
you to disable or remove. A folder another profile enables is never taken
over, because the mods folder is shared and that profile would change under
you: the new revision installs its own copy beside it instead, and the report
asks you to disable the old one in this profile, never to remove it. Going back
to an older revision takes its folders back the same way, or installs fresh
copies of the ones it cannot take.
Whenever a member is replaced this way and you had changed or added files in
its folder, the old folder is first kept beside it as `<name>_backup`, which is
never loaded into the game, and the report names it.

A leftover from an earlier revision that stays enabled keeps the install from
counting as the one the author built; the note goes away once you disable it.
A member you skipped with `--no-optional` is not taken over and is named the
same way. If the takeover itself fails, the run stops with *previous revision:
its folders could not be taken over* and the reason; fix that and run the
command again.

### Free accounts and members you fetch yourself

A free Nexus account cannot fetch the member mods. Nexus gives free accounts no
download links through the API: every file needs the site's own "Mod Manager
Download" button. The collection archive downloads on a free account, so you
can read the whole recipe, but each member is reported with its exact page URL
for you to fetch yourself, and retrying does not change that. Fetch them through
the site's button and run the command again: those members are looked for in
`downloads/` on every run, so the collection finishes as you supply them.

In the window, a free account gets
*Fetch the N missing required files, one by one* in the collection pane in
place of the batch fetch. It opens the first missing required file's page on
Nexus, waits for that file to land in `downloads/`, then opens the next. You
still press the page's download button for each file. The pane shows how far
along it is (*3 of 12*), with *Skip* and *Stop*. When another file of the same
mod lands instead, the pane names both file ids and keeps waiting for the
pinned one. Optional members are not part of it and keep their own *Open*
button.

Eidos also applies binary patches (each checked against its recorded CRC), file
overrides (the named files are hidden with `.mohidden`, as MO2 does) and members
whose files travel inside the collection archive (`bundle`). Members the
collection expects you to fetch by hand (`browse`, `manual`) carry the author's
own instructions through to the report. A member whose name is already a mod of
yours is installed beside it under a free name, never over it, and the report
says which.

### The report

A member whose installer answers could not all be replayed gets its own line,
*installed, but not the way the collection asks*, apart from the members that
installed as the author built them, because its files on disk differ from the
author's. The report also lists ordering rules that named nothing in the
collection, mods caught in a loop of contradictory rules, LOOT groups that do
not exist, and any part of the manifest this version of Eidos does not
understand.

### Keeping the collection order

Eidos keeps each installed collection's rules after the install. When you
later move two of its mods against a before or after rule, the Diagnostics tab
shows one card for that collection, naming the first such pair and counting
the rules that no longer hold. Only mods enabled in the current profile count,
and rules caught in a loop are skipped, because no order can keep them.
**Restore collection order** moves the collection's mods back into an order
that keeps every rule, keeps your own order wherever the rules allow, and
leaves the mods that are not in the collection where they are. Ctrl+Z undoes
it. `eidos check` lists the same finding as `collection_order_broken`. The
check reads the revision's recipe in `collections/`, so a collection whose
cached recipe was removed is not checked.

## Moving an instance to another machine

`eidos pack` writes an entire instance (every mod, the load order, all
profiles, the Overwrite with your saves in it, and the archives everything was
installed from) into a single file. `eidos unpack` puts it back:

```sh
eidos pack skyrimse ~/backup.eidos                  # or give a folder: named for the game and the date
eidos unpack ~/backup.eidos /mnt/games/EidosSkyrim  # on the other machine
```

`eidos pack --dry-run` lists exactly what would go in, what would not and why,
and how much room it needs, without writing anything. `eidos unpack --info` does
the same for a file you already have.

If 7-Zip could not read something, or the walk had to skip something you would
expect in a backup (a symbolic link, a folder it could not read, a name 7-Zip
cannot be given safely), the archive is still written under the name you chose,
but `eidos pack` exits 1 and says what is missing, so a script that runs
`eidos pack ... && ...` stops there.

A `.eidos` file is a 7-Zip archive with its own extension. 7-Zip is already
required to install a mod, so this adds no dependency, and the backup opens
without Eidos in any archiver that reads 7-Zip. It is compressed with LZMA2 at
`-mx1`, to about 43% of the original on a texture-heavy list; `-mx9` takes much
longer and saves a few points more. Textures and meshes are already-compressed
formats, so a bigger dictionary finds very little left to compress. `--level`
(0, 1, 3, 5, 7 or 9) picks another level, and `--no-downloads` leaves
`downloads/` out.

At the default level 1 the files go into solid blocks of at most 16 MB.
Pulling one file out of a 70 GB backup decodes at most the block it sits in,
never everything in front of it, and packing game content runs 2 to 5 times
faster than with every file compressed on its own (unpacking 2 to 4 times). The
cost is on damage: one corrupt byte can take every file in its block with it, up
to 16 MB of neighbouring files. The recovery data described below covers that.
Every other level stays non-solid: each file is its own stream, and a corrupt
byte costs that one file. Level 0 stores the files. From level 3 up, LZMA2
already works in chunks of 16 MB or more, and solid blocks measured slower
there: at level 5, packing meshes took 1.4 times as long and unpacking textures
6.8 times as long.

Packing takes the instance lock, so it cannot run while the game or another
Eidos is writing to that instance, and nothing changes the instance while it is
read. The backup is written under a `.part` name, flushed to disk, and read back
whole with `7z t` before it gets the name you chose. That check decodes every
block and compares every checksum, the same work an unpack does on the other
machine. It shows what the disk gave back right after the write, not that the
drive will keep it; the recovery data below is for that. If it fails, the
`.part` is deleted, nothing is written under your name, and `eidos pack` quotes
the line where 7-Zip found the damage. An interrupted or failed pack leaves a
`.part` file or nothing, never a `.eidos` file that looks complete and is not.

When par2cmdline is installed (`sudo pacman -S par2cmdline` on Arch,
`sudo apt install par2` on Debian and Ubuntu, `sudo dnf install par2cmdline` on
Fedora), `eidos pack` also writes PAR2 recovery data beside the archive:
`backup.eidos.par2` and a few `backup.eidos.volNN+MM.par2` files, 5% of the
archive plus a small index. Keep them in the same folder as the `.eidos` file,
under the names they were written with, and copy them with it: a backup moved
without them has nothing to be repaired from, and the recovery data records the
archive's name, so a renamed backup no longer matches it. `eidos unpack` finds
them by name and checks the archive against them before extracting anything,
including a backup whose end was cut off so that 7-Zip cannot open it. If a few
bytes went bad, par2 repairs the archive first and keeps the damaged copy as
`backup.eidos.1`, which needs free space the size of the archive in its folder;
damage beyond what the 5% covers stops the unpack before a single file is
written. Recovery files that do not match the archive, such as those of an
older backup with the same name, are not used when 7-Zip reads the archive and
finds it intact, and the unpack says so. On a machine
without par2 the unpack still runs, with a warning that the recovery data was
not used. `--no-recovery` packs without it, and `eidos pack --dry-run` says
whether it will be written. `--force` replaces an existing file of the same
name.

### What is not in it, and why

| Left out | Why |
| --- | --- |
| The Proton prefix | It is machine-shaped (drive mappings, a `Z:` view of *this* filesystem) and Eidos rebuilds it in a minute with `eidos prereqs <instance> --install`. Carrying it would roughly double the archive to ship something the other machine has to regenerate anyway. |
| The game | An instance mods a game it does not contain. Install it from Steam over there. |
| Tools outside the instance | xEdit, DynDOLOD and other tools are third-party binaries with their own installers. They are named in the archive, so unpacking tells you exactly which ones are missing on the new machine. |
| `logs/`, `.eidos.lock`, `prereqs.done`, `prereqs.log` | Records of the machine that made the backup. `prereqs.done` in particular would claim the runtime libraries are already installed in a prefix that is not there. |
| `loot/`, except `userlist.yaml` | A masterlist cache Eidos fetches again on demand. Your own LOOT rules in `userlist.yaml` cannot be fetched again, so that one file goes in. |
| `.base/`, `.base-root/` | Empty mountpoints where the game's own files are stashed during a session. |
| `test-run/`, `bisect.json` | A [test run](#test-runs) waiting for its review, and a search for the mod behind a problem between two steps. Both belong to runs on this machine. |
| Half-written files | A paused download (`*.unfinished`), an atomic write in flight (`*.eidos-tmp*`). |
| Symbolic links | 7-Zip would follow one and copy whatever it points at, which for an absolute link means pulling a foreign tree into your backup. They are reported instead, and the backup counts as incomplete: a `mods/` or `downloads/` linked to another drive is not packed. To back up without a linked `downloads/` on purpose, pass `--no-downloads` (or untick *Include downloads/* in the window); a deliberate exclusion does not count as missing. |

A game copy in the instance (`Stock Game`) is packed like any other folder, so
the backup of a frozen instance is about the size of the game larger. An
unfinished copy (`Stock Game.eidos-tmp`) is left out.

Each of these is listed with its reason in `eidos-backup.ini` at the root of the
archive, so reading that file shows what was left out. The same file records
where the instance used to live, which profile was active, how big it unpacks
to, and which tools the new machine will need.

Download records (`downloads/*.meta`) go in with their `url=` line emptied.
That URL is a signed Nexus link that stops working within hours and carries the
`user_id` of the account that downloaded the file, and a backup may be handed
to somebody else. The mod id, the file id and the version stay, so the download
can be found again.

### In the window

Both are in the **File** menu: *Pack this instance...* and *Unpack a backup...*.
Unpacking is also on the welcome screen, under the list of instances, so a
machine with no instance yet can unpack one.

Before anything is written, the Pack dialog shows how many files, how big, how
much of that is `downloads/`, roughly what the archive will come to, and which
tools are named but not carried. Its *Write recovery data* box is ticked when
par2 is installed, and greyed out with the package to install when it is not.
The Unpack dialog reads the backup's manifest as soon as you choose the file, so
it can tell you which game it holds, when it was made, where it used to live and
which of its tools are missing here, before you choose a folder.

Both run on a worker thread with a progress bar, and the bar becomes the report
when they finish.

### What unpacking repairs

A plain copy of the folder would leave every tool button pointing at a path on
the old machine. `eidos unpack` rewrites the values that name the old instance
root and only those: `exe`, `workdir` and the numbered `arg` keys in
`tools.ini`, and `installationFile` in each mod's `meta.ini`. A tool that was
never inside the instance (`/opt/xedit/SSEEdit.exe`) is left exactly as it was
and listed as something to reinstall; Eidos does not guess where that tool is on
the new machine. It also corrects whether the instance calls itself central or
portable, so later commands look for it where it now is.

Unpacking refuses a folder that is not empty (pass `--force` if you mean to
overwrite), a disk too small for the manifest's own figure, an archive made by a
newer Eidos than yours, and any archive holding an entry that would be written
*outside* the folder you chose. Without a folder, a backup of a global instance
goes back to the central place for its game; a portable one needs a folder.

Afterwards:

```sh
eidos prereqs /mnt/games/EidosSkyrim --install   # rebuild the Proton prefix
eidos play /mnt/games/EidosSkyrim
```

## The window

Start `eidos-gui`, or the `eidos-linux` file Colony installs.

The first launch opens a wizard like MO2's: welcome, instance type (portable or
global), game, name and location, summary, then the main window. The welcome
screen also lists every known instance, global and portable, last used first, to
open with one click, so it doubles as the instance switcher. Pointing the wizard
at a folder that already holds an instance opens that instance as it is instead
of creating a new one over it, and a folder that belongs to another game is
refused.

The main window has two panes: the mod list, and the Data, Plugins, Conflicts,
Overwrite, Archives, Saves, Downloads and Diagnostics tabs, with a Run button, a
Test run button ([Test runs](#test-runs)) and a run-target picker. You filter the mod list, select, reorder, group with
separators, narrow by category and right-click for actions. Besides send to top
and bottom, the mod menu has MO2's targeted moves: send above the first
conflicting mod, below the last, to a given priority, or into a separator's
group.

The profile picker switches profiles. Right-click a profile for Rename,
Copy to new..., Desktop shortcut, Import from MO2... and delete, which takes two
clicks and refuses the active or the last profile. Import from MO2... takes over
an MO2 profile's mod order and load order. Desktop shortcut writes a desktop
entry that runs `eidos tool <instance> run "<game>" --profile <name>`: it makes
that profile the active one and asks the store to start the game (details in
[tools.md](tools.md#reaching-a-tool-hide-pin-and-a-desktop-shortcut)).

The **File** menu also runs winecfg, regedit, the Wine file explorer and the
uninstaller in the game's prefix, and Prefix drive_c opens the prefix's `C:`
drive; see [tools.md](tools.md#wines-own-programs-in-the-games-prefix).
*Instances...* lists the instances Eidos knows; Forget takes one off that list
and touches nothing on disk. *Export the mod list...* writes the list in MO2's
CSV format, as `eidos export` does.

### Preferences

Click **Eidos**, top left, to open Preferences in place of the two panes, and
click it again (or **Close**) to go back. General, Appearance and Accessibility
come first, as in every Colony program, then Mod list, Nexus and About. Every
change applies and saves at once; there is no Save button.

- General: the game Eidos opens, the window size, the interface language, the
  tools folder, and how a game is run. Lock the window while a game runs is on
  by default. Start the game at once when a store opens Eidos, and the desktop
  notification that says when a long job ends while the window is in the
  background, are off by default. The window speaks English and French;
  System follows `LC_ALL`, `LC_MESSAGES` or `LANG`, and the change applies at
  once. The `eidos` command line always answers in English.
- Appearance: the theme (26 families, 59 palettes, Eidos's parchment by
  default), the accent colour, the text size, effects and a live preview.
- Accessibility: high contrast, a dyslexia-friendly font, reduced motion and
  a reading text size that multiplies with the Appearance one.
- Mod list: the auto-scroll speed while a mod is dragged to the top or bottom
  edge of the list.
- Nexus: sign-in, the downloads folder, the `nxm://` handler and its Register
  button, preferred servers, Install downloads when they finish, and Offline
  mode.
- About: where the files live, the keyboard shortcuts, the version and the
  release check.

The look and the interface language live in
`~/.config/Colony/Eidos/preferences.json`. The other settings, which the
command line reads too (the default game, the tools folder, the Nexus servers,
Offline mode and the rest), live in `settings.ini` beside it, and the Nexus
session in `nexus.ini`. The theme an older Eidos kept in `settings.ini` is
carried over the first time.

**About -> Version** holds the release check, on by default. At start, at most
once a day, Eidos asks GitHub for the latest release of `Project-Colony/Eidos`.
When it is newer than the running copy, the menu bar shows `Eidos <version> is
available: release notes` after Help, and the link opens that release's page
with its changelog. Eidos never downloads or replaces itself. The check is
skipped in offline mode. It is off for a copy installed under `/usr` by a
package manager, which updates it; `/usr/local`, where `install.sh --system`
puts Eidos, counts as your own install.

### Columns, sorting and grouping

The list shows four columns by default (Category, Content, Version and Flags,
the ones MO2 shows) and offers eight: Category, Content, Version, Author,
Installed, Nexus id, Game, Flags. Tick them in the View menu. With all eight on,
the name column has little room left.

Click a heading to sort by it, again to reverse, and a third time to go back to
load order. The list can be dragged only in load order: a drop position
addresses the real list, and under a sort a row's place on screen is not its
place in that list. While a sort is on, the insertion strips are not drawn and a
drag is refused, as in MO2. The View menu says so and offers the way back.

The View menu can also group the whole list, by category or by source (from
Nexus, or installed by hand). Group headers are not separators: they cannot be
renamed, coloured or moved. They fold, and a folded header keeps its count.
Separators are hidden while a sort or a grouping is on, because a separator
heads the rows that follow it in load order, and both change that order on
screen.

### Mouse and keyboard

Double-click a mod for Information, Ctrl+double-click for its folder,
Shift+double-click for its Nexus page. Ctrl+F puts the caret in the filter box.
Typing a letter jumps to the next mod starting with it, and pressing it again
walks through the rest instead of staying on the first. None of them lands on a
row that the filter, a folded separator or a folded group hides, so the next
Space never toggles a mod you cannot see.

"Collapse others" on a separator's menu folds every group but that one. During a
drag, holding the pointer still over a folded group opens it, so the mod can be
dropped inside; passing over it does not.

Ctrl+Z undoes the last change to the mod list or the load order: a tick, a move, a
batch disable, a LOOT sort.
Ctrl+Shift+Z redoes it. The window keeps 50 steps and forgets them on F5, at the
end of a game or tool run, on a profile switch and when another Eidos process
changes the list. Installing, renaming or removing a mod changes the folders on
disk, so an undo past one of those is refused.

### Filtering the list

The Filters button opens one row per criterion: active, conflicted, update
available, no category, endorsed, tracked, has a Nexus id, has hidden files, no
valid game data, for another game, has notes, and one row per kind of content
(plugins, BSA or BA2 archives, textures, meshes, scripts, script extender
plugins, interface files, sound, FaceGen). A click cycles a row through off,
only and except. The row at the top switches between matching all of the
criteria that are on and matching any of them. The name box and the category
list always narrow the result further. The hidden files row looks at enabled
mods only.

### What the list tells you about a mod

Two advisory flags, both a glyph with the explanation on hover. No valid game
data means nothing at the top of the mod looks like something this game loads;
it may need its folders moved up a level, or it may not be a mod for this game.
Another game means the mod's own `meta.ini` names a different one. Neither
blocks anything: the mod still deploys. "Mark as valid" on the row menu silences
either through MO2's own `validated=` key, so a mod marked valid in one manager
is not flagged in the other.

The layout check flags a mod only when it is sure: a `Root/` tree, an
unreadable folder and an empty folder all count as valid game data.

### Disabling a mod your saves or other mods need

Before a mod is disabled or removed, Eidos reads the plugin lists of the
profile's three newest saves. When one of them uses a plugin from that mod, a
bar above the list names the plugins and waits for Disable anyway or Cancel
(Escape cancels too). Remove adds the same names to its click-again message.
Only Skyrim and Fallout 4 saves are read; other games disable without asking.

A mod's Information, General tab, lists the mods it requires and the mods that
require it, read from the masters of the enabled plugins. Disabling a mod that
others require shows the same bar with Disable them too, which also disables
every mod that needs it and the mods that need those, or Keep them enabled.
The missing-masters card on the Diagnostics tab has Enable what provides them: it enables the mod that holds each missing master and ticks a master plugin that is present but unticked.

### Backing a mod up before you touch it

"Back up this mod" copies its folder aside as `<name>_backup` (then `_backup2`,
and so on; a backup never replaces the previous one). The copy is not a mod: its
checkbox does nothing and it adds nothing to the merged view, because ticking it
would deploy two copies of one mod over each other. "Restore this backup over
the mod" puts it back, in two clicks; the current contents are moved aside first
and only discarded once the copy has succeeded.

**Backups** on the toolbar keeps restore points of the profile's mod list, its
load order (`plugins.txt` and `loadorder.txt` together) and its game INIs, 10 of
each, in `<profile>/backups/`. Back up now adds one, and Restore backs up the
current state first, so a wrong pick can be undone. Eidos also keeps an INI
restore point before each BethINI run (see [tools.md](tools.md#bethini-pie)).

### The tabs

**Data** is a tree of the merged view, expanded one level at a time: opening a
folder reads that folder in each layer that has it, never every enabled mod at
once. It is built from the same layer stack the mount serves, so whiteouts and
hidden files are respected and the tab shows what the game will see. Filter it
by name, narrow it to contested files only, sort out what is where with the Size
and Modified columns, and Reveal any row in a file manager. Hovering a file lists the other mods that
ship it. A mod's top-level
`meta.ini`, `readme.txt`, `fomod/` and `Root/` are left out of Data, in this tab
and in the mount: the first three are for the mod manager, and `Root/` goes to
the game folder instead. A file the game or a tool writes under one of those
names lands in the Overwrite and stays visible.

An `.exe` row in Data has Add tool, which opens the Executables editor with that file filled in (nothing is saved until you press Save), and Run, which runs it once through the merged view like a registered tool ([Tools](#tools)). Save tree writes the whole merged Data tree to a text file, one line per file with the mod that provides it, with your home folder and user name removed, for a bug report.

**Plugins** is the ESP/ESM/ESL load order: toggle, reorder by hand, or sort with
LOOT and read the post-sort report, whose advice links open in your browser. Its
header counts the enabled plugins in each index space against the game's limit,
for example `full 231/254, light 512/4096`, and turns orange when one is full.
The filter box narrows the list by name. Space, Ctrl+A, Shift ranges and the
row menu's Activate all shown and Deactivate all shown then act on the rows
shown only, and a drag is refused until the filter is cleared. Selecting a plugin tints its masters in the
plugin list and the mod that ships it in the mod list. Plugins LOOT flags as
dirty get a Diagnostics card that can clean them with xEdit
([tools.md](tools.md#cleaning-a-plugin-loot-flags-as-dirty)).

After a LOOT sort, a full plugin whose new records all fit the light range says
so on hover, and its row menu has Flag as light. Eidos writes a copy with the
light flag into a new mod named `<plugin> (light)` at the bottom of the mod list
(the highest priority) and leaves the original alone, so disabling that mod
undoes it. It refuses when one of the profile's three newest saves loads the
plugin as a full plugin. Only Skyrim and Fallout 4 saves are read, so on
Starfield no save is checked. LOOT checks record ranges only: look at a plugin
that adds cells or worldspaces in xEdit before flagging it.

Hovering a plugin shows where it comes from, its author, its header and form version, the first 300 characters of its description, its masters and, after a LOOT sort, LOOT's messages. On Skyrim, Skyrim SE, Skyrim VR and Enderal SE the game also reads an INI named after a loaded plugin; the INI column marks those plugins, and a click opens that file read-only.

**Conflicts** explains the per-file winners and losers. **Overwrite** turns what
the game wrote into a real mod in one step.

**Archives** lists every BSA or BA2 the enabled mods ship, the mod it comes from
and why it loads: an active plugin of the same name, or the INI registers it.
One that nothing loads is marked as dead weight. When another mod ships an
archive of the same name that wins the conflict, the row names that mod. Extract unpacks an
archive into a new mod named after it without its extension (`Foo.ba2` gives
`Foo (extracted)`) and leaves the archive in its mod; loose files win over
files in any archive, whatever the mod order.

**Saves** reads each save's header (character, level, location, playtime),
compares the plugin list stored in it with the current one, and has a button
that enables the mods it needs. Saves that reached the prefix without Eidos are
described under [Saves belong to the profile](#saves-belong-to-the-profile).

**Downloads** and **Diagnostics** have their own sections below.

### Mod information

"Information..." opens a per-mod dialog with General, Conflicts, Filetree,
INI Tweaks, Notes, Nexus files and Nexus page tabs. From the filetree (and from the Data
tree) any file can be hidden: it is renamed to `<name>.mohidden`, which takes it
out of the merged view without deleting it, so a few stray files of one mod can
be suppressed without changing priorities. The filetree also does the ordinary
file operations: new folder, rename, delete, open. Each refuses a path that is
not a plain path inside that mod: no `..`, no absolute path and no symlink along
the way, since following one could delete a file outside the mod folder. Rename
changes the last part of the path only, so it never moves a file, and it refuses
a name that is already taken. Delete takes two clicks and cannot be undone.

**View** on any row in the filetree or the Data tree previews the file: images,
text, DDS textures (pick the mip, layer, face and channel) and static NIF models
with their diffuse textures. The NIF preview needs `eidos-nif-preview` beside
Eidos or on `PATH`; the release bundle, the Arch package and source builds have
it, `eidos-linux` does not. Text is read up to 64 KB and says where it stopped.

**INI Tweaks** lists the fragments a mod ships in its `INI Tweaks/` folder; the
enabled ones are merged into the profile's game INI at launch, in priority
order, and taken out again when the run's INIs are captured, so disabling a
tweak removes its settings.

The **Nexus files** tab lists the mod's files on Nexus, newest first, and marks
the one installed here and the one that replaces it. That second row has an
**Update** button; see [Mod updates](#mod-updates).

The **Nexus page** tab shows the mod's description and, newest first, the
changelog of each file version on its page. Eidos reads both the first time you
open the tab, from Nexus's GraphQL API, which does not count against the hourly
request budget. It keeps them until Eidos closes or you press **Refresh**. They
are shown as plain text: BBCode and HTML formatting is removed, a link's
address is written after its text, and images and videos are left out.
**Endorse** and **Track** above the text do what the mod's right-click menu
does. The page of an adult mod stays hidden unless your Nexus account shows
adult content, and a page with no rating stays hidden in every case.

### Downloads

A download can be dragged from the Downloads list onto a position in the mod
list to install it at that priority, and archives or folders dropped onto the
window from a file manager install too. File drops need an X11 or XWayland
session: winit implements them for X11 only. Downloads can be paused and
resumed: pausing stops the transfer and keeps the partial file, and Resume asks
Nexus for a fresh link and continues from where it stopped.

The Downloads tab keeps the archives you downloaded. Filter it by name (the
friendly mod name too, so "skyui" finds `SkyUI_5_2_SE-12604-5-2SE.7z`), sort by
newest, name, size or state, and hide an archive you are done with: the file
stays and only the row goes. "Show hidden" brings them back, and the same button
unhides. "Remove N installed" deletes the archives of mods you have already
installed, in two clicks, and only the ones the filter shows.

**Preferences -> Nexus -> Install downloads when they finish** is off by
default. Turned on, a Nexus download that finishes while the window is open
installs without a visit to the Downloads tab. It is the usual install: a FOMOD
opens its installer, and a name already in the mod list asks what to do.
Archives that were in `downloads/` before the window opened or before the
option was turned on are left alone, and so is a download whose install you
cancelled. While a collection is open in the collection pane, no download
installs on its own, and one that finishes meanwhile does not install on its own
later either; the collection installs its members. The file a pending Update
waits for is installed by the Update. Only Nexus downloads are installed this
way, and one that the Downloads filter hides waits until the filter is cleared.
While a drag is in progress, or while the game or another Eidos holds the
instance, the install waits.

### Mod updates

The update button near the right end of the toolbar checks every mod that has a
Nexus id. When the mod page shows the version you have installed, nothing more
is asked. Otherwise Eidos reads the mod's file list (one more request, kept for
the rest of the check) and follows the update chain from the file id the mod's
`meta.ini` records: authors mark a new upload as the successor of an older one,
and MO2 reads the same chain. A newer file in that chain is the update. When the
chain names none and the installed file is still listed on the page, the mod
counts as current, even though the page shows another version. The page's
version decides only when no file id is recorded, when the installed file was
removed or archived, or when Nexus did not send the file list.

Each mod uses the Nexus domain of the game its own `meta.ini` names
(`gameName`), not the instance's: the update check, Endorse, Track and Visit on
Nexus all go there. Enderal SE also accepts `skyrimspecialedition` mods, so a
Skyrim SE mod installed in an Enderal SE instance is checked on its Skyrim SE
page. A Mod Manager Download link goes to the instance you used last when its
game accepts the link's domain. Otherwise it goes to an instance of the game
whose own domain it is, before one of a game that only accepts it.
`EIDOS_INSTANCE=<folder>` overrides both.

Track and Untrack in a mod's right-click menu add the mod to your tracked mods
on Nexus or remove it, so the site's notifications follow what you track here.
Endorse and Abstain in the same menu endorse the mod on Nexus or withdraw the
endorsement; the endorse button next to the update button endorses the selected
mod. Each update check also reads your endorsements and tracked mods from Nexus
(two requests) and sets every mod's Endorsed and Tracked flags to match, on the
mod's own Nexus domain. When the two disagree, Nexus wins. If either list cannot
be read, the check says so and leaves those flags as they were.

The **Update** button on the Nexus files tab of "Information..." installs the
newer file. With a premium account Eidos downloads the file itself. With a free
account it opens the file's page, where Mod Manager Download sends the file
back, and the install starts when the archive arrives in `downloads/`. From
there it is the usual install: a FOMOD opens on the answers the mod was
installed with, then the prompt offers Replace with "Keep a backup of the
existing mod" ticked, whatever you chose at the last prompt. Replace keeps the
mod's priority and enabled state.

Before a Replace swaps a mod's folder, Eidos compares the old folder with the
new one by file name and size and reads the headers of the plugins in both.
This covers Update, a collection update, and Replace picked at the install
prompt. The status line then counts the files added, removed and changed in
size, and names every master the new version needs that the old one did not.
The **General** tab of "Information..." ends with a **Last update** section
listing the versions, the plugins added or removed, the masters added or
removed, the plugins whose ESM, ESL or ESH flag changed, the mods this one now
shares files with or no longer does, and the files. A file rewritten at the
same size is not listed. The report is kept in `logs/updates/<mod>.json` in the
instance, which `eidos pack` leaves out; a fresh install or a Merge into the
folder removes it. `eidos install --replace` prints the same counts on a
`changes:` line.

A folder that holds several Nexus files, such as a main file with an optional
file merged into it, is refused: replacing it with the new main file would drop
the others. Update those by hand.

`eidos nexus update skyrimse --apply` runs the same check, then on a premium
account downloads each new file and installs it over its folder with
`--replace --backup`. On a free account it prints `OPEN <url>` for each file
page instead. Folders that merge several files are skipped with the reason. A
FOMOD mod with no recorded answers, such as one installed before Eidos kept
them, is not reinstalled with the installer's defaults: that update fails with a
message to update the mod once from the window, which records the answers. A
FOMOD whose installer has nothing to choose is installed as usual. The command
exits 1 if any update failed.

### Collection updates

When you run Check for updates, Eidos also asks Nexus for the latest revision of each collection installed in this instance. A line above the mod list names every collection with a newer revision and counts the mods added (+), updated (~) and dropped (-); hover it for their names. The counts are missing when the installed revision's recipe is no longer cached. **Update** opens the newer revision in the collection pane, where *Install this collection* runs it. The list is rebuilt on every check and is not saved.

### Nexus settings

**Preferences -> Nexus -> Offline** stops Eidos contacting Nexus at all, and
skips the daily release check on GitHub. Update checks, sign-in, downloads and
collections say so instead of failing with a connection error. It is off by
default.

**Preferred servers** ranks the CDN nodes a download prefers, best first. Only a
premium account is ever handed more than one mirror to choose between, so for
everyone else Nexus picks and this changes nothing. When none of the servers you
named is offered, the download still happens, from the first node Nexus offers.

### Categories

Categories can be assigned to one mod or a whole selection, the catalog can be
edited from the same dialog, and the game's official category list can be
pulled from Nexus. Both catalog files are MO2's own (`categories.dat` and
`nexuscatmap.dat`), so a shared instance keeps one catalog. A mod downloaded
from Nexus is categorised at install, once Fetch from Nexus has mapped the
catalog.

### INI editor, log and extensions

**View -> INI editor** edits the profile's game INIs, the copy Eidos keeps; the
copy in the Proton prefix is overwritten at every launch. Its last tab is the
profile's `launch-env.ini`
([Variables and wrappers per profile](#variables-and-wrappers-per-profile)).
**View -> Log** reads the session logs. **View -> Extensions** lists your own add-ons; see
[extensions.md](extensions.md).

When a launch finds the game's main INI (`Skyrim.ini`, `Fallout4.ini`,
`Oblivion.ini`...) neither in the profile nor in the prefix, Eidos first copies
the game folder's default (`Skyrim_Default.ini`, `Fallout4_Default.ini`...) into
the profile, as MO2 does. When the instance runs a game copy, the default comes
from the copy. The prefix and the game folder are not changed.

### Installing mods

The installer handles Simple archives, FOMOD, Wrye Bash BAIN packages (tick the
sub-packages, which merge in order) and Oblivion's OMOD packages. An OMOD
script runs when it uses the OBMM commands Eidos supports or is one of the three
C# installers Eidos knows; Eidos runs no other C# or Visual Basic code and says
so. When no heuristic recognises the layout, a manual picker shows the
archive tree and lets you point at the data root.

Reinstalling or updating a FOMOD mod opens the wizard with the answers of its
last install ticked, on the first step whose options changed since then, with
the changes listed. **Reinstall with new choices** starts from the installer's
defaults instead. `eidos install --replace` or `--merge` reuses the stored
answers and refuses, listing the changes, when they no longer fit.

### Diagnostics

The Diagnostics tab runs live health checks. It reports missing masters,
plugins LOOT flags as dirty, archives no active plugin will load, whether the
mod list still matches the mods folder, whether FUSE passthrough is on and, when
it is requested, whether the launch capability it needs is there. After a run it
shows what the script extender's own log says about each of its plugin DLLs, so
you can see whether your SKSE plugins loaded. It also reports a game update since
the last clean launch, a missing script extender or one that does not match the
game version (with the extender's download page and an install button for the
archive you download), a Steam launch option that does not start Eidos with
`%command%`, NVAPI left off for DLSS, exclusive fullscreen that stops frame
generation, an ENB `enblocal.ini` without `LinuxVersion=true`, the plugin
declarations of F4SE and SFSE plugins, and the crash log of the last session
with the mods it names.

The checks for the profile INIs and plugins look for `sTestFile` lines in
`[General]` of the Fallout 4, Fallout 4 VR and Starfield custom INIs, which make
the game ignore `plugins.txt` (Fix removes them from the
profile's copy and keeps the previous file as `<ini>.pre-fix`), enabled `.esp`
plugins in Starfield, and blueprint plugins that lack the blueprint flag, the
`BlueprintShips-` prefix or the `.esm` extension, or whose main plugin is not
enabled. The system checks flag a `vm.max_map_count` or a hard open files limit
below what Proton needs, and a game, prefix or instance on NTFS, exFAT or FAT.
The mod checks name BodySlide's data path when it does not point at the game's
`Data`, `Data` folders a tool wrote into `Overwrite/Root`, a patch whose loose
files lose to the mod it patches, and a mod that replaces the script extender's
Papyrus scripts.

Check for updates also reads each mod's Nexus requirements. Diagnostics lists a required mod that is missing or disabled, with Ignore for a requirement met another way.

The tab also shows which Proton build runs the game and warns before an older Proton runs a prefix a newer one set up; Rebuild moves a broken prefix aside so Steam makes a new one ([troubleshooting.md](troubleshooting.md#which-proton-runs-the-game-and-a-broken-prefix)). Other cards flag a prefix that was never set up or whose registry is stale, a program other than Eidos receiving Mod Manager Download links (Register fixes it), tool output older than its inputs (with a button that runs the tool again), and a cleaned or light copy of a plugin whose original changed since. Each card is explained in [troubleshooting.md](troubleshooting.md#health-cards). **Copy report** puts a report of the system and every finding on the clipboard, with your home folder and login name hidden, for a bug report.

For Skyrim SE, Enderal SE and Fallout 4, Diagnostics also has a Display card. It shows the window size and mode set in the profile's `SkyrimPrefs.ini` (`EnderalPrefs.ini`, `Fallout4Prefs.ini`), or in the prefix's copy when the profile has none yet. When neither sets a size, its fields start from the largest connected screen listed in `/sys/class/drm`, so a Steam Deck reads 1280x800. The width and the height are whole numbers from 320 to 16384. Apply writes `iSize W`, `iSize H`, `bFull Screen` and `bBorderless` into the profile's copy, after adopting the prefix's copy if the profile does not have one. When SSE Display Tweaks sets the window, Apply writes the same size and mode into its copy in the Overwrite; the mod's own file is not changed.

The Game files card at the top of the tab is described under [Game files](#game-files-the-store-install-or-a-copy), the collection order card under [Keeping the collection order](#keeping-the-collection-order), and Find the mod behind a problem under [Test runs](#finding-the-mod-behind-a-problem).

### Launching the game

To launch the game through the window, set the game's Steam launch option to the
absolute path of the program, because Steam does not read your shell's `PATH`
([install.md](install.md#then-point-steam-at-it)):

```
~/.local/bin/eidos-gui %command%
```

From Colony, the path is `~/.local/share/Colony/apps/Eidos/eidos-linux`. Eidos
opens on the game's instance, the one you last used, so a portable instance is
found again as the global one is; click Run to launch it through the merged
view.

With **Start the game at once when a store opens Eidos** on (Preferences,
General, Running a game; off by default), Eidos skips that click: it checks
Health and starts the game without showing its window. The window opens as
usual when Health lists a problem, when the mod list needs attention, when a
tool set to run before playing has an out-of-date output, or, for Skyrim SE,
Enderal SE and Fallout 4, when the profile has no window size yet (the window
then opens on Diagnostics). To get the window for one launch, put `EIDOS_WINDOW=1` before the command:
`EIDOS_WINDOW=1 ~/.local/bin/eidos-gui %command%`.

In a window no store opened, Run does not start the game itself, and Eidos never
starts a game through Proton or Wine directly. Run asks the store that owns the
copy to start it, and the store starts Eidos:

- Steam runs `steam -applaunch <appid>` when the launch option above is set
  and the game has a Proton prefix, which Steam makes the first time it starts
  the game. Steam starts first if it is closed.
- Heroic (a GOG or Epic copy, native or Flatpak) gets a `heroic://launch` link
  when Eidos is a wrapper in the game's settings in Heroic, with the absolute
  path of `eidos-gui` (or `eidos-linux`) as the executable and no arguments.
  Heroic starts first if it is closed.
- Legendary on its own runs `legendary launch <app>` when
  `~/.config/legendary/config.ini` has `wrapper =` and the absolute path of
  `eidos-gui` (or `eidos-linux`) under the game's `[<app>]` section, or under
  `[default]`.

The script extender's loader, the launcher and the game's executable in the Run
list are all handed to the store this way, and so is
`eidos tool <instance> run <entry>` for one of them in a terminal. Eidos cannot
hand some copies to their store, such as a GOG copy outside Heroic: Run then
says to start the game from that store.

The hand-off records this instance as the one you used last, and the store then
opens a second Eidos window on it. With Start the game at once off, you click
Run there; with it on, that window starts
the game without showing itself, unless one of the cases above opens it. The
first window does not follow that run, so press F5 in it after playing to reload
its lists. When the store's setting is missing, or the script extender's loader
is not next to the game's executable, Run starts nothing and shows what to set
and where, with the running binary's real path.

While a game or tool runs, Run becomes Stop. Two presses end every program the
run started, and Eidos then captures the session and unmounts as after any exit;
a stopped run never counts as a clean launch
([troubleshooting.md](troubleshooting.md#a-game-or-tool-that-will-not-close)).

Steam's `%command%` for the Bethesda titles usually points at
`<Game>Launcher.exe`. Eidos never runs it: the launcher is a separate settings
app that re-scans `Data` and rewrites `plugins.txt`, undoing the load order that
was just deployed. Eidos runs the script extender's loader instead if one is
installed, the game binary otherwise, and says so when it has to fall back to
the game binary.

Eidos scans the enabled mods' DLLs for a `d3dcompiler_47.dll` import, copies
Microsoft's DLL into the prefix and only then sets
`WINEDLLOVERRIDES=d3dcompiler_47=n,b`; you do not need to set it yourself.

### Variables and wrappers per profile

A profile can carry its own environment variables and wrapper commands in `profiles/<name>/launch-env.ini`, one entry per line:

```ini
env=PROTON_ENABLE_NVAPI=1
env=DXVK_HUD=fps
wrapper=gamemoderun
wrapper=gamescope -W 1280 -H 800 -f -- %command%
```

They apply to every `eidos play` of that profile (the game started by Steam's `%command%`, or by Run in a window a store opened) and to every tool run. `eidos shell` gets the variables and no wrapper. The first `wrapper=` line is the outermost. A `wrapper=` line that holds `%command%` gets the command in that place, as in Steam's launch options; a line without it gets the command at the end. Wrappers run inside Eidos's private namespace, so they see the mods.

A variable here replaces the value Eidos sets for the same name, and a tool's own `env=` line in `tools.ini` replaces both. `WINEDLLOVERRIDES` is added after the overrides Eidos sets for ENB, ReShade and the bundled DLLs instead, so a DLL named in both takes the profile's setting and the others keep theirs.

View -> INI editor opens the file as its last tab, for every game. Blank lines and lines that start with `#` or `;` are skipped. Saving lists the line numbers Eidos ignores: lines that are neither `env=NAME=value` nor `wrapper=command`, and a `wrapper=` line whose first word is `NAME=value`, as Steam's launch options allow; put that variable on an `env=` line instead. `eidos play <instance>` without a command lists the variables a run gets and the wrappers. Copy report in Diagnostics names the variables and the wrapper programs, without their values.

## Tools

xEdit, BodySlide, DynDOLOD and other tools run through the merged view inside
the game's Wine prefix (Steam's Proton prefix, or the one Heroic set up for a
GOG or Epic copy):

```sh
eidos tool skyrimse add BodySlide "<path>/CalienteTools/BodySlide/BodySlide.exe"
eidos tool skyrimse run BodySlide
eidos tool skyrimse run --exe "<path>/GenerateFNISforUsers.exe"  # run an exe once without registering it
eidos prereqs skyrimse            # what the registered tools need, and its state
eidos prereqs skyrimse --install  # fetch whatever is missing
```

Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, TexGen, xLODGen and Synthesis are
found without registering them. Pandora, Nemesis, FNIS, BodySlide and Synthesis
arrive with a capture mod already set, so their output goes into a mod of its
own (details in [tools.md](tools.md#sending-a-tools-output-to-its-own-mod)).

The title you give a tool decides which runtimes and DLLs Eidos provides for it:
`BodySlide` gets its DirectX libraries, `BS` gets nothing. In the window the
Executables dialog shows each prerequisite's real state under the field, and
the missing ones are buttons.

The release bundle carries the vcrun2022 installers (x86 and x64), the .NET 8
Desktop Runtime 8.0.12 and the .NET 10 runtime 10.0.10 in `runtimes/`, and
`install.sh` copies them to `~/.local/share/Colony/Eidos/runtimes/`, so
`eidos prereqs <instance> --install` installs them without a download. Eidos runs
the vcrun2022 and .NET 8 installers from Microsoft silently in the game's prefix,
without winetricks. `eidos-linux` and source builds carry no `runtimes/` folder,
so they download the same pinned files from Microsoft. The .NET 10 SDK that
Synthesis needs (about 300 MB) is always a download.

The table, the three prerequisite tiers, why DynDOLOD needs a .NET runtime that
winetricks cannot install, and why a tool installed as a mod is launched from the
merged path instead of its own folder are in [tools.md](tools.md).

## Extensions

Eidos can be extended without being rebuilt: a TOML manifest in
`~/.config/Colony/Eidos/addons/` adds a tool to the Extensions list or a check to the
Diagnostics tab. Nothing is loaded into Eidos: an extension is a program it runs.
See [extensions.md](extensions.md).

## Try the proof of concept

The script needs no game. It shows the merged view, copy-on-write, a game folder
left untouched, and a merge that exists only inside its namespace, using only
unprivileged OverlayFS in a user namespace (Linux 5.11 or later):

```sh
./scripts/poc-overlay.sh
```
