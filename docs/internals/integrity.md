# Installation integrity, archive conflicts and generated output

These checks describe the current implementation on `main`.
Automated tests and read-only inspection of real files do not establish that a
mod or a game runtime works in a playthrough.

## Installing and replacing mods

Simple, FOMOD, BAIN, manual and Root/Data installations share one publication
path. A new install or replacement is prepared beside the destination before
its old directory is moved. An extraction, selection or metadata failure leaves
the old mod intact. If publication fails, Eidos restores the original; if that
restoration also fails, the error identifies the retained recovery directory.
Temporary extraction and staging directories never enter the profile mod list.
This protects against operation failures; it is not a power-loss recovery
journal.

An OMOD install first copies its decoded files into a staging directory of its
own (`.eidos-install-omod-publish-*`), checking each file's size and CRC32
again, and publishes from there. An OMOD whose files go through a transform
refuses Merge ("OMOD transforms require a staged replacement; Merge is not
supported").

Merge retains the existing overlay behavior: successfully written files remain
if a later merge operation fails. Before touching live content it clears
collection ownership and exact-source claims and records an incomplete-install
warning. A successful merge restores its verified source metadata. Destinations
are resolved case-insensitively and unsafe symlink destinations are refused.
Use Replace when the archive contains a whole replacement and transactional
publication is required. A successful reinstall preserves the mod's priority,
enabled state and unrelated metadata. An explicit install-at-position action
still uses the requested position.

FOMOD takes precedence over automatic loose-file and BAIN classification.
Installer conditions use the active profile's plugin selections, including
Overwrite and masked paths. A plugin being present in an enabled mod does not
make it active if the profile disables it. Missing selected archive sources
remain a nonfatal installation warning, saved in `meta.ini` and shown in the
status bar, mod information and Diagnostics. Collection reports mark these
installs as approximate. A complete Replace clears the previous warning;
Merge retains it because it cannot prove an earlier omission has been repaired.

Unambiguous wrapper directories around Root/Data payloads are removed without
losing files beside Data. Ambiguous layouts still require the manual picker.

## Updating a mod in place

Every FOMOD install records its answers in the mod's `meta.ini`, under
`eidosFomodChoices`. A reinstall or an update replays them through the same
engine that computes the installer's flags, matching options by name with the
recorded position only as a tiebreak. In the window, an answer that no longer
matches stops the replay at that step, with the recorded answers pre-ticked.
`eidos install --no-fomod-defaults` refuses such an installer instead of
falling back to its defaults.

The update check follows the Nexus update chain of the installed file, so a
mod whose page version moved on while its file has no successor is not flagged.
Update installs the successor with Replace, keeps a backup of the old folder
(ticked by default in the window, `--backup` on the CLI), and replays the
stored answers. `eidos nexus update --apply` does the same with
`--no-fomod-defaults` for a Premium account and prints the file page for a free
one. It skips a mod that holds several Nexus files, since replacing it with one
would drop the others. Every Replace writes what changed
to `<instance>/logs/updates/<mod>.json`, outside the mod so its files stay as
the archive made them: files added, removed and resized, plugins added and
removed, masters added and removed, and plugin kinds that changed. Files are
compared by name and size, so a same-size rewrite of a non-plugin file is not
listed.

## Exact Nexus files and collection recovery

The MO2-compatible `[installedFiles]` array records exact `(mod ID, file ID)`
pairs. Merge retains distinct sources; Replace records only its new source.
The QSettings array size is respected, including zero. Older metadata with only
a matching mod ID or version is shown as unverified, not as the exact requested
collection file. Download matching also checks the source game and complete
archive, rather than accepting a sidecar or interrupted transfer alone.

A collection reserves its actual destination name and writes its ownership
marker before persisting the reservation and beginning installation. Failed,
unavailable or downloaded member status gives no authority over an existing
personal mod. Name collisions create a separate folder; resume reuses the saved
owned folder. A missing finished folder can be retried. A changed ownership
marker, invalid folder name, unreadable state or mismatched revision is reported
without replacing the unverified folder. An ordinary manual reinstall revokes the
collection ownership marker. Collection INI fragments also reserve an owned
folder and cannot overwrite a personal mod with the same display name. State
writes are atomic and synced; a checkpoint failure stops the remaining
installation and ordering changes.

State is stored outside the extracted collection archive at
`collections/<slug>-<revision>.state.json`. Unsupported collection recipes,
unmatched installer answers and manual actions remain explicit in the report.
The existing Nexus account rules still apply; resumability does not bypass
Premium-only automatic downloads. A free account is walked through a
collection's missing files one Nexus page at a time: Eidos opens each mod's
page and picks up the download the user starts there, queues only required
members, and never downloads for a free account by itself.

A collection's before/after rules stay checked after install. Health reports
the rules the mod list no longer keeps, read from the cached recipe of each
installed collection, and the window warns when a drag breaks one and can
restore the order folder by folder.

## Plugin and script-extender diagnostics

Plugin discovery retains header errors and form versions. An unreadable active
plugin cannot produce a clean missing-master result or an invented valid index.
Full, light and medium indexes respect the selected game's capacity. Relevant
Starfield blueprint and update flags follow the parser's engine-specific
interpretation. Older Skyrim form versions are advice to inspect the port,
not a claim that every such plugin crashes.

The existing full LOOT scan performs record-level light, medium and update
validation. Header-only checks explicitly leave those questions unverified.
Changing plugin inputs invalidates the previous deep result.

Eidos never writes a mod's own plugin. Clean runs xEdit's QuickAutoClean on a
copy in a `Cleaned plugins` mod, and flagging a plugin as light writes the
copy into a new `<plugin> (light)` mod, refused when a save loads that plugin
as full (Skyrim and Fallout 4 saves are read). Health reports a cleaned or
light copy whose source plugin has changed since. Eidos warns before a mod
whose plugins the newest saves use is disabled or removed.

Launch preparation and GUI Diagnostics inspect the script-extender plugin DLLs
the game will see, in the merged view. Root mods, root Overwrite,
case-insensitive paths and whiteouts use the same layer resolver as launch. No
DLL is loaded or executed; bounded PE metadata parsing does the checks, which
depend on the game:

- Skyrim SE, Enderal SE and Skyrim VR: the winning EXE, the SKSE loader and
  `SKSE/Plugins/*.dll`. The checks cover architecture, explicit runtime
  declarations, minimum SKSE versions and the required, readable Address
  Library file.
- Fallout 4 and Starfield: `F4SE/Plugins/*.dll` and `SFSE/Plugins/*.dll`. Each
  plugin is checked for 64-bit PE, and its `F4SEPlugin_Version` or
  `SFSEPlugin_Version` data against the runtime version read from the winning
  game executable, including the Address Library file that data requires.
  Runtimes whose loader does not read that declaration (Fallout 4 before
  1.10.980, Starfield before 1.15.216), or an executable whose version cannot
  be read, get the architecture check only.
- Oblivion, Fallout New Vegas and Fallout 3: `OBSE/Plugins`, `NVSE/Plugins` and
  `FOSE/Plugins`, checked for 32-bit PE DLLs.

Unsupported declarations, unknown flags and unavailable metadata are reported
as unverified. The Address Library database contents, relocation IDs,
machine-code hooks and gameplay behavior are not validated by these static
checks.

Health also reports a loader whose runtime DLL for the game build is missing
(for example `skse64_1_6_1170.dll`). When the instance runs a game copy, the
winning executable, the extender files and the crash logs are read from the
copy, and for Steam the store's build id is not used, since it says nothing
about a copy.

Automatic script-extender selection also resolves the merged root. A loader
supplied only by an enabled Root mod or root Overwrite can be selected before
the launch mount exists.

## Archive member conflicts

The conflict worker reads directory metadata from TES3 BSA, BSA 103/104/105,
and supported BA2 GNRL/DX10 formats. It does not extract or decompress assets.
Unsupported layouts, corrupt counts, offsets and name tables produce diagnostics
rather than fabricated file lists.

Archive activation uses the selected game's rules, effective INIs and active
plugin load order. First, mod priority chooses the physical copy of each archive.
Then archive load order determines its asset providers; visible loose files
override archive members where that engine's configuration supports it.
Disabled-plugin archives are excluded unless independently activated by INI.
Skyrim SE's exact and ` - Textures` associations are distinct from Skyrim LE's
exact-name rule. Legacy archive invalidation and unresolved same-plugin order
are reported explicitly. An unresolved comparison is labelled as such instead
of presenting a definite winner.

The Conflicts and per-mod information views show member paths, archive names,
plugin associations and providers. Analysis runs on a worker, and obsolete
results are discarded after input changes. Use Refresh after modifying source
archives or configuration outside Eidos. This is an asset collision analysis,
not a detector for semantic conflicts between plugin records or scripts.

## Generated output receipts

Launching a configured tool, from the CLI or window, records the tool name,
command, executable size/mtime fingerprint when available, active profile,
enabled mod order and versions, exact Nexus source IDs, and plugin order/state.
A pending receipt is saved before launch. An interrupted run remains explicitly
unverified after restart; later files are never attributed to it automatically.

Three kinds of run record no receipt: a tool entry that starts the game itself
(its script-extender loader, launcher or binary), Wine's own programs run in
the game's prefix when no configured tool has that title (`winecfg`,
`regedit`, `explorer`, `uninstaller`), and `eidos shell`. What the game writes
is the session's own output, not tool output. These runs still write a
[session record](architecture.md#session-records-and-stopping-a-run).

After the tool exits, changed Overwrite paths are associated with that run,
including partial output from a failed tool. Existing untouched files keep
their own origin. Receipts follow successful moves through Create mod from
Overwrite, capture to the configured output mod, and Sync to Mods. Failed moves
retain the source data and its attribution. All receipt data lives in the
instance's `.generated-output.json`, outside the game-visible Data layers.

Open a generated mod's General tab to inspect its receipts and recorded inputs.
Diagnostics warns when the inputs have changed. Its card has a button that
runs the tool again, as it is configured now. With `check_before_play=1` on a
tool in `tools.ini` ("Ask before playing when its output is out of date" in
the Executables dialog), Run asks first when
that generator's output is out of date; it is off by default. Creating this
run's own output mod/plugin does not itself make the receipt stale. Unrecorded external rewrites
invalidate file attribution rather than assigning them to an old run.

These receipts compare recorded metadata and filesystem stamps, not every
input file's contents. A changed file with unchanged version metadata may need
manual review. Receipts do not infer which mods a tool actually consumed or
whether regeneration is necessary. The executable fingerprint is not a claimed
tool version. Outputs written directly outside Overwrite are not captured by
this mechanism.

## Filesystem and save protection

Directory renames reject nonempty merged destinations, incompatible types and
moves beneath themselves. Directory-cache invalidation prevents an older read
from publishing stale entries after a mutation. Regression coverage includes
actual isolated FUSE mounts.

Before Steam Cloud synchronization replaces a divergent prefix save, Eidos must
preserve and verify its rescue copy. A rescue error or an unverified name
collision aborts the batch. Save/co-save groups remain together; errors are
surfaced instead of silently continuing over the divergent save.

Saves another device left in the prefix (a Steam Cloud download, a launch
without Eidos) are hidden by the profile's saves bind while the game runs. The
Saves tab lists them and imports them on request, each with its co-saves.
Files are created new, so a profile save is never replaced: a save whose name
clashes is written as `orphan-<secs>-<name>`, with `secs` from its mtime, and a
file with the same bytes counts as done. A group that fails part-way removes
what it wrote.

The profile's restore points live in `<profile>/backups/`, never beside the
originals, because the plugin state directory is bind-mounted over the game's
own folder at launch. They cover the mod list, the load order (`plugins.txt`
and `loadorder.txt` together) and the profile INIs, 10 of each
kind. Eidos takes an INI restore point before each run of a tool that rewrites
the INIs (BethINI Pie, recognised by its title). Restoring backs up the current
state first.

A test run writes nothing to the profile, the Overwrite or the prefix until a
group of its writes is kept
([architecture.md](architecture.md#test-runs-and-bisect)). A game copy is
built in `Stock Game.eidos-tmp` and renamed into place only when the copy is
complete and the game build did not change during it
([architecture.md](architecture.md#game-copies)).
