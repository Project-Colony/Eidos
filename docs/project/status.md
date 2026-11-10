# Feature status and development ledger

The full done/remaining ledger, kept as the project's development history. The
README has the short version.

Releases 1.17 and 1.18 (September 2026) added replacement staging, exact
collection ownership and Nexus file identity, bounded plugin/SKSE preflight
checks, archive member conflicts, generated-output receipts, archive browsing,
DDS and NIF previews, OMOD installs, trusted extensions and timestamp-ordered
games. 1.18.3 and 1.18.4 (October 2026) fixed data-integrity and responsiveness
defects found by a review of 1.18.2. See
[integrity.md](../internals/integrity.md) for behavior and verification limits.

1.19 (October 2026) moved every file Eidos keeps onto the Colony layout
([paths.md](../internals/paths.md)), adopted the Colony design system with its
shared themes and fonts, added a French interface, and shipped `eidos-linux`,
one signed file that is both the window and the CLI. 1.19.6 keeps the layer
index alive between lookups and cuts FUSE round trips.

1.20 added Crimson Desert, for ASI and ReShade mods. Each `.eidos` pack is
checked with `7z t` before it gets its name, and carries PAR2 recovery data when
par2cmdline is installed.

1.21 writes a `ModOrganizer.ini` so PGPatcher and PGLite can run in their Mod
Organizer 2 mode. 1.21.1 gives Pandora 5 the .NET 10 runtime.

1.22 (November 2026) answers `FS_IOC_GETFLAGS` with the casefold flag, so Wine
skips its case probes, shares one passthrough backing per inode, and caches a
negative lookup whose spelling it records for an hour. Tools run on Heroic's
GOG and Epic copies, `tools.ini` takes a per-tool `env=`, `runner=native` and
`slr=1` (the Steam Linux Runtime container), and Synthesis gets the .NET 10
SDK. The Plugins tab has a filter and shows the selected plugin's masters and
source mod, an archive can be extracted into a new mod, and Health copies a
redacted diagnostic report.

1.23 added `eidos check`, the Health checks on the command line, with `--json`
output. Health names a missing or mismatched script extender, a game update
since the last clean launch, a Steam launch option that does not start Eidos
with `%command%`, and the mods a crash log mentions. Run in a window no store
opened hands the game to the store that owns the copy, a stuck run can be
stopped, a broken Proton prefix can be moved aside so Steam rebuilds it, and
MO2 executables can be imported as tools. Each mod is checked for updates on
its own Nexus game domain and along its file's update chain, and
`eidos nexus update --apply` installs the successor in place with the FOMOD
answers replayed. Missing or disabled Nexus requirements show in Health, a free
account is walked through a collection's missing files one Nexus page at a
time, and a newer revision of an installed collection is reported with what
changed. Every run leaves a session record that `eidos sessions` lists. Eidos
also warns before a change that would break saves, can undo edits, and cleans
and light-flags plugins on copies.

1.24 adds test runs, which send every write of one game run to
`<instance>/test-run/` until the user keeps or throws away each group of
writes, and bisect, which runs each step as a test run on half of the remaining
suspects and names the mod behind the problem. An instance can also run from a
copy of the game folder instead of the store install (`eidos stock-game`). The
ledger below has the details of both.

1.24 Health checks cover `sTestFile` lines that disable `plugins.txt`, Starfield
`.esp` and blueprint plugins, a patch whose loose files lose to the mod it
patches, a mod whose loose Papyrus scripts win over the script extender's own,
`vm.max_map_count` below 1048576, a hard open files limit below 524288, a game
folder, prefix or instance on NTFS, exFAT or FAT, BodySlide's data paths,
`Data` folders inside the Overwrite or a `Root/`, and an adopted list's
`ModOrganizer.ini` whose `gamePath` names another game. A Display card in
Diagnostics sets each profile's window size and mode.

Each profile can get a desktop shortcut that starts the game through its store,
and with the "Start the game at once when a store opens Eidos" preference on, a
window the store opened starts the game without showing itself when Health is
clear (`EIDOS_WINDOW=1` in the launch option always shows it). Each profile can
carry its own launch variables and wrappers. `eidos shell` runs a shell or one
command in the merged view, and the window runs winecfg, regedit, explorer and
the uninstaller in the game's prefix. A long job that ends while the window is
in the background sends a desktop notification.

On Nexus, 1.24 tracks mods and syncs endorsements and tracked mods on each
update check, browses and searches a game's published collections in the
collection pane, lists in Health the order rules of an installed collection
that the mod list no longer keeps (Restore collection order puts them back),
and shows a mod's description and changelog in a Nexus page tab
of the mod information dialog. After an update, Eidos shows which files the
update added, removed or resized and which plugins and masters changed, and
keeps that report in `<instance>/logs/updates/<mod>.json` for the mod
information dialog. A finished download can install itself, and Eidos asks
GitHub at most once a day whether a newer release is out.

The mod list filters by Nexus state, notes, hidden files and content type,
matching all or any. A plugin's author, description and header version show on
hover. Pandora, Nemesis, FNIS, BodySlide, DynDOLOD, xLODGen and Synthesis are
found with the runtime each needs and the mod their output goes to. An exe in the
Data tab can be added as a tool or run once (`eidos tool <instance> run --exe
<path>`), and the profile INIs get a restore point before each BethINI run.
Saves another device left in the prefix can be imported without replacing the
profile's saves, and the merged Data tree can be saved, redacted, with each
file's source for a bug report.

The 1.24 release bundle ships Microsoft's Visual C++ redistributable
(`vcrun2022`, x86 and x64), the .NET Windows Desktop Runtime 8.0.12 and the
.NET runtime 10.0.10 under `runtimes/`. `vcrun2022`, `dotnet8` and
`dotnetdesktop8` run Microsoft's own installers from that copy, or from a
pinned download, instead of winetricks. The .NET 10 SDK for Synthesis is still downloaded. The eight
embedded DirectX DLLs are stored gzip-compressed (17,906,544 bytes down to
7,889,812), about 10 MB less per binary, and release builds use fat LTO.

The in-game claims below refer to the builds they describe; automated tests do
not certify later releases in gameplay.

## Roadmap

- [x] Validate the under-Wine virtualization thesis (PoC)
- [x] Layer-resolution engine + tests (`eidos-core`)
- [x] FUSE union daemon (`eidos-fuse`) - live-verified: merge priority,
      fall-through, case-insensitive reads, no root / no overlayfs / no Wine
- [x] Copy-up / Overwrite layer (writes) - live-verified: new files, edits to
      game files, and edits to mod files all land in Overwrite; the game install
      and every mod source stay pristine
- [x] Deletes (whiteouts), rename, and statfs - live-verified: deleting a game
      or mod file hides it (via a whiteout in Overwrite) while the source stays
      pristine; rename works (saves), and df reports real free space
- [x] Supported-game catalog + Steam install detection (`eidos-games`) -
      live-verified: finds installed games across all libraries (incl. other
      drives) with their data dir + Proton prefix. Detection also reads the
      installed manifests of Heroic and Legendary, read-only, for GOG and Epic
      copies. The catalog has 15 built-in games: the 11 Bethesda ones plus
      Stellar Blade, Stardew Valley, 7 Days to Die and Crimson Desert
- [x] Per-launch user+mount namespace wrapper (`eidos-launch`) - live-verified:
      runs a command through a private union view; the host sees no mount, the
      game install and mod sources stay pristine, writes land in Overwrite
- [x] Instance management + unified `eidos` CLI (`games` / `init` / `play`) -
      live-verified end to end: detect a game, create an instance, mount its
      mods over the game's Data dir, run a command through the view
- [x] Steam launch-option integration (`eidos %command%`) with a real Proton game
      - both the CLI (`eidos play <id> -- %command%`) and the GUI (which resolves
      which detected game Steam is launching from the `%command%` it was handed,
      then runs it through the view)
- [x] GUI first-launch wizard (`eidos-gui`, iced) - MO2-style screens (welcome ->
      portable/global -> game -> name -> summary -> main), Colony parchment theme
- [x] GUI main window (`eidos-gui`) - two-pane MO2 layout: a profile picker, a
      mod list (enable/disable, reorder, per-mod conflict flags), and Data /
      Plugins / Conflicts / Overwrite / Saves / Downloads tabs with a Run button
- [x] Per-mod `meta.ini`, byte-compatible with MO2 (`eidos-instance`) - existing
      MO2 instances round-trip unchanged (version / Nexus id / category /
      endorsed), with an `update_available` check that the GUI reads after
      each `eidos-nexus` update check
- [x] Self-describing instance manifest `eidos-instance.ini` (`eidos-instance`) -
      records game id + schema version, so portable instances need no path-guessing
- [x] Profiles (`eidos-instance`) - per-profile enabled set + load order over one
      shared `mods/` pool, with a GUI picker (switch / new-by-copy)
- [x] ESP/ESM/ESL plugin load order (`eidos-plugins`) - esplugin-parsed headers +
      MO2-parity ordering (masters first, master-before-dependent) and FormID
      indexes (`FE` light / `FD` medium), written as `plugins.txt`/`loadorder.txt`
      into the Proton prefix right before launch; surfaced in the Plugins tab
- [x] Per-file conflict detection (`eidos-conflicts`) - one pass over the enabled
      layers builds a winners/losers tree + per-mod state (Overwrites /
      Overwritten / Mixed / Redundant), shown in the Conflicts tab and as per-mod
      flags in the mod list
- [x] Mod installer (`eidos-install` + `eidos-fomod`) - the MO2 Simple installer
      (7-Zip extract, wrapper-strip via a Gamebryo ModDataChecker) plus the FOMOD
      scripted installer (UTF-16 `ModuleConfig.xml` parse + condition/flag engine),
      driven from a CLI (`eidos install`) and the GUI Install button + an
      interactive FOMOD wizard; writes a MO2-compatible `meta.ini`
- [x] **Root mods, natively** - a mod's `Root/` folder is projected onto the game
      install directory by a second union mounted over the game root, with the Data
      union nested inside it. This is what a script-extender preloader, an ENB, a
      ReShade or an `.asi` loader needs, since the Windows loader only looks beside
      the executable. MO2 cannot do it at all without the third-party Root Builder
      plugin - not a usvfs limitation (usvfs redirects arbitrary absolute paths, and
      MO2 already maps saves outside Data) but a policy choice in
      `IPluginGame::getModMappings`, which maps every mod to `Data` and nothing
      else. Eidos also recognises the archive shapes: `Data/` beside loose
      executables, an explicit `Root/`, or a bare wrapper DLL with no Data half at
      all, each laid out automatically. And a wrapper in `Root/` is picked up by the
      `WINEDLLOVERRIDES` forcing, so the Wine builtin cannot silently win. Nothing
      is ever copied into the real game folder - unlike Root Builder's copy mode,
      which has to restore backups afterwards and leaves debris if it dies
- [x] Nexus Mods integration (`eidos-nexus`) - sign in with OAuth (authorization
      code + PKCE S256, loopback listener, no personal API key anywhere: Nexus
      requires them absent from a distributed client, so Eidos ships the
      `client_id` Nexus registered for it, `eidos`, and `EIDOS_NEXUS_CLIENT_ID`
      overrides it), register the `nxm://` handler (`eidos nxm --register`)
      so the site's "Mod Manager Download" button downloads straight into the
      instance's `downloads/` (with MO2-format `.meta` sidecars), check installed
      mods for updates (`eidos nexus update`, MO2's rate-limit-friendly
      updated-this-month strategy), and install from the GUI Downloads tab in one
      click (Simple or FOMOD wizard). Each mod is checked against its own Nexus
      game domain and its file's update chain. `eidos nexus update --apply`
      installs each successor file over its folder, with the FOMOD answers
      replayed and the old folder kept as a backup (a free account gets the file
      page to download from instead). It skips a mod folder that holds several
      Nexus files, and refuses a FOMOD that offers a choice with no recorded
      answer. Endorsements and tracked mods sync on each update check
- [x] Tools through the VFS (`eidos tool`) - run xEdit/FNIS/BodySlide through the
      same merged view, inside the game's Proton prefix, with no Steam `%command%`:
      a protontricks-style Proton resolver (config.vdf `CompatToolMapping` ->
      compatibilitytools.d / official Protons -> `STEAM_COMPAT_*` env), a
      per-instance tool list (MO2 `ExecutablesList` parity, script extender seeded
      by default), and a run-target picker next to the GUI Run button. Tool output
      lands in Overwrite, so the next launch picks it up - the generate-then-play
      loop. (MO2 itself rebuilds its mapping per run, so per-launch mounts are
      exact parity.) Run from outside a store-started process, an entry that
      starts the game (the script extender loader, the vanilla launcher or the
      game binary) goes to the store that owns the copy (`steam -applaunch`,
      `heroic://launch`, `legendary launch`), and `--print` shows that command.
      Tools also run on Heroic's GOG and Epic copies, in Heroic's prefix.
      `tools.ini` takes a per-tool `env=`, `runner=native` (no Proton) and
      `slr=1`. `eidos tool <id> import-mo2` lists the custom executables of a
      Mod Organizer 2 `ModOrganizer.ini` in the instance and `--write` adds
      them, and `eidos tool <id> run --exe <path>` runs an exe once
- [x] Per-game Bethesda features (`eidos-gamefeatures` + `eidos-gamedef`) - all
      keyed off one declarative `GameDef` row per game (MO2's `IPluginGame`
      schema): **BSA/archive invalidation** so loose mod files override the
      vanilla BSAs (without it BSA-packed mods are silently ignored);
      **per-profile INIs** seeded from the prefix then deployed/captured around
      launch; and **per-profile saves** via a namespace bind-mount of the
      profile's saves over the prefix (the Linux-native equivalent of MO2's usvfs
      save mapping, no prefix changes). The INI writing shares one `eidos-ini`
      primitive (MO2's single-`QSettings` idea), keeping MO2 `meta.ini`
      round-trips byte-for-byte
- [x] Rootless perf tuning (1 MiB readahead / max_write, parallel dirops) and an
      opt-in `FUSE_PASSTHROUGH` path. Passthrough is **off by default**, so the
      daemon serves reads itself. With it on, the daemon before 1.22 stopped
      Skyrim SE opening its own archives and plugins (152
      `STATUS_ACCESS_VIOLATION` opens vs 0 without it, measured on Skyrim SE
      1.6.1170 under proton-cachyos 11.0). 1.22 shares one backing id per inode
      and keeps an inode's live opens all passthrough or all normal, which
      removes the most likely cause, but no game run has measured it since.
      `EIDOS_FUSE_PASSTHROUGH=1` re-enables it, and then the `CAP_SYS_ADMIN` it
      needs in the initial user namespace becomes relevant
- [x] Harden the daemon for real use - inode reference-counting + `forget`,
      offset-stable `readdir` (snapshot per directory handle), per-handle
      `pread`/`pwrite` (no re-resolve per syscall, lock released before I/O),
      case-insensitive whiteouts, opaque directories, POSIX errnos (`rmdir`
      ENOTEMPTY, `rename` NOREPLACE/EXCHANGE), `setattr` (mode / timestamps),
      xattr passthrough (Wine `DOSATTRIB`), symlinks, `fsync` durability.
      Covered by a real-mount integration suite that runs in a private
      namespace, including a writable `MAP_SHARED` mmap round-trip, with
      `setattr` guarded so the kernel's post-unlink attribute flush cannot
      resurrect a deleted file. (`writeback_cache` is off - it broke loading
      Windows DLLs from the mount.)
- [x] **Runs a real heavily-modded Skyrim SE (110 mods) end-to-end under Proton**
      - all ~50 SKSE plugin DLLs (CommonLibSSE-NG included) load and run via the
      mount, each writing its config into the Overwrite layer. Needed two
      MO2/usvfs parity fixes: launch with **CWD = game root** (CommonLibSSE-NG
      opens its address library by a CWD-relative path) and **NTFS-like sorted
      `readdir`** (the Creation Engine's loose-file indexer assumes it). That run
      predates the passthrough default flip and was made with passthrough **on**,
      so it proves DLL image-mapping, not a playable load order; rootless, a small
      load order has since reached gameplay with plugins, archives and Papyrus
      scripts live, but the ~50-DLL case is not re-validated without passthrough
- [x] Native DLL provisioning for Proton (`eidos-gamefeatures::native_dll`) - no
      Proton flavour ships Microsoft's native `d3dcompiler_47.dll` (they symlink
      Wine's builtin HLSL stub, which Community Shaders / ENB / ReShade reject), so
      Eidos scans enabled mods' DLL import tables (the `object` crate) and, when one
      imports it, deploys the bundled genuine MS redistributable into the prefix's
      `system32`/`syswow64` and forces it native - unlinking Proton's builtin symlink
      first, backing up any displaced file, idempotent and best-effort. Since
      1.24 the eight embedded DLLs (four, each in an i386 and an x86_64 build)
      are stored gzip-compressed, and only the one being deployed is unpacked,
      in memory
- [x] Modding-tool prerequisites (`eidos prereqs`) - tools run in the game's shared
      Proton prefix through the merged view, so one prereq set covers them all.
      Tier 1 (bundled, zero network): the DirectX helpers, declared per-tool in
      `tools.ini` and provisioned at launch. By default BodySlide gets `d3dx9_43`
      and `d3dcompiler_47`, DynDOLOD / TexGen / xLODGen get `d3dcompiler_47`,
      `d3dx9_43` and `d3dx11_43`, and CAO gets `d3dcompiler_47` and `d3dx11_43`.
      `d3dcompiler_43` is still bundled, but no default uses it. Tier 2
      (installers, behind an explicit `eidos prereqs <id> --install` / GUI
      "Tool Setup" button): `dotnet48` for FNIS and `vcrun2022` for Synthesis,
      CAO, Nemesis and LOOT. Since 1.24, `vcrun2022`, `dotnet8` and
      `dotnetdesktop8` run Microsoft's own installers, the copy the release
      bundle ships first, else a pinned download, each refused unless its
      SHA-256 matches. The other verbs (`dotnet48` among them) go through the
      system winetricks pointed straight at Proton's own wine (bypassing the
      protontricks + Proton-GE mismatch). What was installed is recorded in a
      per-instance receipt tied to that one prefix. Tier 3 (unpacked once into
      a shared runtime folder, pinned SHA-256): `dotnet10` (runtime 10.0.10) for
      DynDOLOD / TexGen / xLODGen and Pandora, which the release bundle ships,
      and `dotnetsdk10` (the .NET SDK 10.0.403, about 300 MB) for Synthesis,
      which is not shipped and is downloaded once. Synthesis and Pandora no
      longer get .NET from winetricks
- [x] GUI to MO2 daily-driver parity (`eidos-gui`) - a mod-list filter box,
      click-to-select, and a right-click action menu (enable/disable, send to
      top/bottom, open in explorer, visit on Nexus, reinstall, rename, remove,
      information); an interactive Plugins tab (toggle ESP/ESM, persisted to
      `plugins.txt`); a per-mod information dialog (general / conflicts / filetree /
      editable notes); a Version column; and wired Nexus / Change Game / Settings /
      Tool Setup toolbar buttons
- [x] LOOT-based plugin auto-sort (`eidos-loot`) - libloot's own graph sort behind
      `eidos sort` and the GUI's Sort button. The masterlist and prelude are
      cached per game in `~/.cache/Colony/Eidos/loot/<repo>/`, shared by every
      instance. The Sort button refreshes them on each sort, like MO2/LOOT;
      `eidos sort` downloads them when they are missing and refreshes them
      with `--update-masterlist`. If a refresh fails and a cached copy exists,
      the sort uses the cached copy. Only
      `userlist.yaml`, for local overrides, stays in the instance. The
      post-sort report pops the way MO2's dialog does,
      listing the plugin messages LOOT attached, with its advice links clickable.
      Games libloot cannot sort say so up front, in Diagnostics and on the tab
- [x] Mod-list separators + categories - MO2 group dividers (create, colour,
      collapse/expand, persisted by display name) and a category filter driven by
      each mod's primary `meta.ini` category, alongside a Category column
- [x] MO2 profile import (`eidos import`, GUI) - takes over an existing MO2
      profile's `modlist.txt` order and enabled states plus its
      `plugins.txt`/`loadorder.txt` verbatim (the formats are already identical).
      Mods MO2 listed that are not installed here are reported rather than dropped,
      and local mods MO2 never knew about are kept at the bottom
- [x] Nexus collections, installed rather than listed (`eidos collection`, the GUI's
      collection pane, `eidos-collections`) - a collection is a RECIPE and almost
      none of it is visible from the API: the install phases, the answers its
      author gave each FOMOD, which mod wins a file conflict, the plugin LOOT
      rules and the INI tweaks all live in a `collection.json` inside the
      collection's own archive. Eidos fetches the archive (`downloadLink` is a
      relative API path, and it answers 200 on a FREE account - the member mods
      are the gated part, not the recipe), reads it, and applies it: phase order
      with optionals last, FOMOD answers replayed through the engine's own
      forward pass, `modRules` topologically sorted into the mod list, plugin
      rules merged into `userlist.yaml` as additions where the user's own
      choices win, INI fragments installed as their own mod, and each member
      switched ON as it installs - a mod nothing lists is a mod nothing loads,
      which is the one failure a report cannot see. The per-member state is
      written after EVERY member, beside the revision's folder rather than
      inside the directory the collection's own archive unpacks into, under a
      temporary name and renamed into place; an install interrupted at 147 of
      200 continues from 147, and a record that cannot be read stops the run
      instead of silently repeating hours of downloads. A member only the user
      can fetch is re-checked on every run, because "fetch it yourself and run
      this again" has to be an instruction that works. What it does not do is
      named in the report rather than passed over: binary patches, file
      overrides, bundled members, manifest sections this build has never seen,
      and - on a free account - each member with the page URL to fetch it from,
      because Nexus does not mint download links for free accounts and no retry
      gets past that. The report separates "installed" from "installed, but not
      the way the collection asks"; Vortex logs that distinction at level
      "info" with the words "This is normal"
- [x] Whole-instance transfer (`eidos pack` / `eidos unpack`, `eidos-transfer`) -
      an entire instance in ONE `.eidos` file: mods, load order, every profile,
      the Overwrite with its saves, and the archives it was all installed from.
      A 7-Zip archive under a name of its own, LZMA2 `-mx1` (43.0% of
      1199 MB of measured real dds/nif content, where `-mx9` spends ten times
      the time to reach ~38%), in solid blocks of at most 16 MB at that level,
      so one file can be pulled out of a 70 GB backup by decoding its block
      only (one bad byte can cost that whole block). The other levels stay
      non-solid: from level 3 up a solid block runs on one thread, and at
      level 5 it unpacked dds 6.8 times slower. Each pack is read back with
      `7z t` before it gets its name, and with par2cmdline installed it
      carries 5% of PAR2 recovery data that `eidos unpack` uses to repair the
      archive before extracting.
      The parts a `cp -r` gets wrong are the point: empty directories, which a
      file-only list drops (64 of them in a real 57 000-file instance); names
      7-Zip would read as patterns; symlinks it would FOLLOW into a foreign tree;
      and the signed `downloads/*.meta` URL that expires in hours and carries the
      downloader's `user_id`. Unpacking repairs the values naming the old
      instance root and only those (`exe`/`workdir`/`arg<N>` in `tools.ini`,
      `installationFile` in each `meta.ini`) and corrects central-vs-portable.
      The Proton prefix, the game and third-party tools stay out and are NAMED in
      `eidos-backup.ini` with every other exclusion and its reason. In the
      window too, in the File menu and - for unpacking - on the welcome screen,
      because the machine that needs a restore is the one with no instances on
      it yet. Both run on a worker thread against the 60 Hz poll the extraction
      dialog already used, and the progress bar becomes the report
- [x] Overwrite-to-mod - turn what the game and the tools wrote into a first-class
      mod in one step, which is the other half of the generate-then-play loop
- [x] VFS correctness pass against Fluorine's implementation - copy-up no longer
      clones a read-only lower mode (a 0444 Steam depot file used to make the next
      write-open fail `EACCES`), one file has one inode whatever its casing (it
      used to split, poisoning `forget`/rename/passthrough bookkeeping), a
      directory rename re-keys its whole subtree and discards clobbered inodes,
      and `setxattr`/`removexattr` no longer resurrect a deleted file (Wine writes
      `user.DOSATTRIB` constantly, so that was expected traffic, not an edge case)
- [x] Kernel-side metadata caching - negative dentries for the paths Wine probes
      and never finds, a long positive entry/attr TTL, `FOPEN_CACHE_DIR` on
      `opendir`, `RLIMIT_NOFILE` raised at init, several event loops over
      `clone_fd`, and per-op counters under `EIDOS_FUSE_STATS` so a metadata storm
      is measurable instead of guessed at. The first `OPENDIR` is answered
      `ENOSYS` when the kernel offers `FUSE_NO_OPENDIR_SUPPORT`, so the kernel
      serves directory opens itself (`EIDOS_FUSE_OPENDIR=1` restores the
      handles). 1.22 added two more: `FS_IOC_GETFLAGS` returns `FS_CASEFOLD_FL`,
      so Wine skips its `fstatfs` and `.ciopfs` probes, and a directory whose
      names need Unicode case folding gets neither hint; a negative entry whose
      spelling is recorded is cached for 3600 s, and only unrecorded spellings
      keep 60 s
- [x] MO2's remaining daily-driver features - hidden files (`.mohidden`, honoured
      by the resolver so a hidden file leaves the merged view *without claiming its
      name*, which is what lets the layer below win); a recursive Data tree with
      per-node origin; targeted mod-list moves (above/below the first/last
      conflicting mod, to a priority, into a separator); BAIN sub-package and
      manual installers so no archive is refused; save-header parsing with the
      save's own plugin list diffed against the current one; mod-shipped INI
      tweaks merged at launch and un-merged at capture; orphan-archive and
      script-extender-log diagnostics
- [x] Guards against the failure modes that quietly destroy a setup - the mod list
      is derived from the mods directory like MO2's, but unlike MO2's the
      reconciliation is not written back blind: a mods folder that is unreadable,
      or empty while the saved list is not, refuses the save instead of flattening
      a curated order (MO2 has no equivalent guard - `refreshModStatus` rewrites
      `modlist.txt` in the same pass that dropped the entries). Same rule on the
      plugin side: a capture that clears the active set entirely is refused at any
      size, because the names are still listed so nothing was uninstalled, and no
      edit produces that
- [x] MO2 parity, the last thirty-eight: filter criteria, editable categories,
      backups, MD5 recovery, output capture per tool, pause/resume/cancel, a real
      Data tree, an INI editor, a log pane, user extensions, an instance manager,
      Overwrite send-to-mods, an Archives tab that says WHY each BSA does or does
      not load, save screenshots and cross-profile transfer, unavailable-mod
      flags, CSV export, a File menu, plugin send-to-priority, colours and notes
      on the row, install-at-position, bulk enable/disable, and drag-to-install.
      Then the last eleven in one pass: offline mode, preferred CDN servers,
      collapse-others with hover-to-expand, the four mouse and keyboard gestures,
      the two state flags (silenced through MO2's own `validated=`), the
      Downloads tab as an archive library, eight optional columns with a sort on
      any of them, grouping by category or source, file operations in a mod's
      tree, per-mod backup and restore, executable extras (Steam AppID, hide,
      pin, `.desktop` shortcut), and image/text previews. All thirty-eight are
      closed: the last of them, Nexus **collection installation**, installs from
      the window on a worker thread whose progress bar becomes the report
- [x] The Colony filesystem layout (`eidos-paths`) - `~/.config/Colony/Eidos`,
      with global instances, runtimes and logs under `~/.local/share/Colony/Eidos`
      and the LOOT masterlists under `~/.cache/Colony/Eidos`, built on
      `colony_ui::paths` and migrated by COPY (hard links for instances) so a wrong
      migration cannot cost anybody a Nexus session. Four crates had each been
      answering "where do my files go" by hand; there is one answer now. The
      reason to look was worse than the layout: `Settings::save` resolved its
      path globally, so a GUI test rewrote the developer's own preferences on
      every run of the suite - which presented as "options revert after every
      rebuild"
- [x] Casing normalization at mod-import time, for collisions only (since
      2026-06-17) - extraction collapses siblings whose names differ only in
      case into one lower-cased entry. Two directories merge. Of two files, the
      one with the oldest mtime stays. When a file and a directory collide, the
      file keeps the name and the directory moves to `<name>_dir`. Names that do
      not collide keep their casing, because the VFS already folds case
- [x] Packaging and distribution - each release ships `eidos-linux`, the one
      file Colony installs, and a bundle that installs through the one tracked
      installer (`packaging/install.sh`: rootless by default, the capability
      only with `--cap`), both signed by the organisation's release key; and
      `packaging/PKGBUILD` builds an Arch package from git. Since 1.24 the
      bundle carries the Microsoft runtimes listed in `packaging/runtimes.txt`
      under `runtimes/`, which `install.sh` copies to
      `~/.local/share/Colony/Eidos/runtimes`; `eidos-linux` alone has none and
      downloads them when a tool needs one. Release builds use one codegen unit,
      fat LTO and no symbol table. Fat LTO took 1,247,016 bytes (5.6%) off
      `eidos` and 3,840,192 bytes (7.2%) off `eidos-gui` against thin LTO, for
      37% more build time (334 s instead of 243 s, measured with rustc 1.99.1).
      See [packaging.md](../internals/packaging.md)
- [x] Run through the store that owns the copy (1.23) - Run in a window no
      store opened does not start the game itself. It asks the store:
      `steam -applaunch <appid>`, `xdg-open heroic://launch?appName=<app>&runner=<gog|legendary>`
      or `legendary launch <app>`, and the store starts the game through its
      Eidos setting (the Steam launch option, or a Heroic or Legendary wrapper).
      When that setting does not start Eidos, the status line gives the exact
      line or wrapper to set, and nothing runs. Eidos never writes a store's
      configuration. A profile's desktop shortcut (1.24) runs `eidos tool
      <instance> run <game entry> --profile <name>`, which makes that profile
      active and hands the game to the store the same way
- [x] Test runs and bisect (1.24) - a test run runs the game with the
      profile's mods, with no mods, or with the selected mods, on a working copy
      of the profile. Its saves, INIs, plugin list, Data and Root output and
      logs land under `<instance>/test-run/` instead of the profile, the
      Overwrite and the prefix, and the review keeps or throws away each of
      those five groups as a whole. A test run never syncs to Steam Cloud, and
      the session list leaves it out of the last good run. Bisect runs each
      step as a test run on half of the remaining suspects, turns on the mod
      that provides each master their plugins need, and asks whether the
      problem showed (yes, no or unsure; an unsure answer rotates the split).
      It needs about `ceil(log2(n))` steps for `n` suspects and ends on the mod
      or mods left once no smaller split can run, or on none when every suspect
      was cleared. It keeps its state in
      `<instance>/bisect.json` between steps, and writes its log to
      `<instance>/logs/bisect-<time>.txt` when it ends
- [x] Game copies (1.24, `eidos stock-game <instance>`) - an instance can run
      from a copy of the game folder kept in the instance, mounted in place of
      the store install at the bottom of the root union; the game still starts
      through its store. `freeze --yes` copies the install into
      `<instance>/Stock Game`, sharing blocks where the filesystem supports
      reflinks (without `--yes` it prints the plan: size, file count, copy kind
      and free space). `use <folder>` picks an existing copy, such as the Stock
      Game a Wabbajack list brought, `store` goes back to the install, and
      `delete --yes` deletes only the folder the freeze recorded. Health, the
      plugin list, LOOT, the Data views and tool detection read the copy.
      Freezing refuses an instance that sits inside the game folder. Games that
      keep their plugin list in the game folder (Morrowind, and Oblivion with
      `bUseMyGamesDirectory=0`) cannot use a copy
- [ ] Self-contained formats (AppImage, Flatpak): unimplemented and unverified.
      They are possible in principle since the launch capability became
      optional: Eidos runs rootless and only the opt-in passthrough path wants
      `CAP_SYS_ADMIN`. That matters because a file capability lives in the
      `security.capability` xattr of the executable and the kernel ignores it on a
      `nosuid` mount, which is exactly what an unprivileged FUSE mount is forced
      to be (check any FUSE mount on your own machine:
      `findmnt -t fuse -o TARGET,OPTIONS` shows `nosuid,nodev`) - so a
      self-mounting bundle can never carry it, and a sandbox that sets
      no-new-privs cannot gain it either. Neither format is ruled out by that
      any more (a Flatpak modulo its own namespace nesting); only someone opting
      into passthrough needs the binary on a real filesystem where `setcap`
      reaches it

The manager layer above the VFS is complete per the MO2 + usvfs study that drove
this work (all six of the subsystems it identified are done): the mod installer
(Simple + FOMOD wizard), plugins, conflicts, profiles, `meta.ini`, the instance
manifest, per-game Bethesda features (BSA invalidation + per-profile INIs/saves
off a declarative `GameDef`), and tools through the VFS
(`eidos tool`), plus Nexus integration beyond it (`eidos-nexus`: nxm:// downloads,
update checks, the GUI Downloads tab). Since then: the GUI brought up to MO2
daily-driver parity (separators, categories, diagnostics, Overwrite-to-mod, MO2
profile import), LOOT sorting, native DLL provisioning so Community Shaders and
BodySlide load without manual DLL steps, the `eidos prereqs` tool-prerequisite
system, and a correctness-plus-caching pass over the daemon
itself.

Since then, and all measured on a real 50-layer instance rather than a benchmark:

- The layer index turned path resolution from a walk of every layer into a
  hash lookup, and a save load from twenty seconds into six or seven.
- Declining `opendir` cut directory opens from 516,301 per session to 1. Wine
  opens a directory to ask whether it folds case, on every path that fails to
  resolve, and the kernel can answer that itself.
- The merged-children map did the same for `readdir`, the one handler the path
  index could not help: 799 ms -> 105 ms across two sessions issuing the same
  2,063 listings.
- A layer that spells one directory two ways no longer loses everything under
  the second spelling. The bug hid 74 files of one real mod with no error
  anywhere. The index had been working around it, and the fallback documented as
  never wrong was the half that was wrong.
- Tier 3 prerequisites: Eidos provides `dotnet10` for DynDOLOD / TexGen /
  xLODGen and Pandora (from the release bundle since 1.24, else downloaded) and
  downloads `dotnetsdk10` for Synthesis, and the Executables dialog says whether
  each prerequisite is present instead of only what was typed.

What remains to validate: in-game runs of the other game families, AppImage and
Flatpak builds (unchecked above), and a new passthrough measurement that holds
the capability in both runs and changes only the switch.
The earlier `plugins.txt` launch investigation is historical; the current
preflight checks and capture protections are documented in the usage guide and
[integrity.md](../internals/integrity.md).

## On reviewing this work

The MO2-parity batches were each reread adversarially: independent readers hunt
for defects, other readers try to refute them, and only what survives is fixed.
Across the batches that is 154 confirmed findings, every one reproduced before
being touched.

Two of those findings are kept here because they show what a second reader
catches.

**A systemic defect hides behind a feature that works.** Adding sorting and
grouping to the mod list changed which rows the list DRAWS. Nothing else was
told: selection, keyboard navigation, shift-extend, select-all and the bulk
actions all went on asking which rows pass the *filter*, a different question the
moment the two orders disagree. Ten findings, one mistake - and its worst case
was Ctrl+A selecting mods inside a folded group, one click away from a batch
Remove that deletes from disk. The fix is one function, `drawn_mod_rows`, that
everything asks.

**A fix can look right and be unreachable.** The review of *that* fix found the
drag guard sitting in a branch no user gesture reaches - a mod row's press emits
`DragStart`, never `SelectMod` - and its test passed only because it sent a
message no user can produce. The second pass found that the first fix was
unreachable.
