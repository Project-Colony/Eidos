# Feature status and development ledger

The full done/remaining ledger, kept as the project's development history. The
README carries only the short version; this is the receipts.

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
      drives) with their data dir + Proton prefix
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
      endorsed), with an `update_available` check ready for a future Nexus crate
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
      requires them absent from a distributed client, so Eidos needs a registered
      `client_id` and has no Nexus access without one), register the `nxm://`
      handler (`eidos nxm --register`)
      so the site's "Mod Manager Download" button downloads straight into the
      instance's `downloads/` (with MO2-format `.meta` sidecars), check installed
      mods for updates (`eidos nexus update`, MO2's rate-limit-friendly
      updated-this-month strategy), and install from the GUI Downloads tab in one
      click (Simple or FOMOD wizard)
- [x] Tools through the VFS (`eidos tool`) - run xEdit/FNIS/BodySlide through the
      same merged view, inside the game's Proton prefix, with no Steam `%command%`:
      a protontricks-style Proton resolver (config.vdf `CompatToolMapping` ->
      compatibilitytools.d / official Protons -> `STEAM_COMPAT_*` env), a
      per-instance tool list (MO2 `ExecutablesList` parity, script extender seeded
      by default), and a run-target picker next to the GUI Run button. Tool output
      lands in Overwrite, so the next launch picks it up - the generate-then-play
      loop. (MO2 itself rebuilds its mapping per run, so per-launch mounts are
      exact parity.)
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
      opt-in `FUSE_PASSTHROUGH` path. Passthrough is **off by default**: measured
      on Skyrim SE it stops the game opening its own archives and plugins (152
      `STATUS_ACCESS_VIOLATION` opens vs 0 without it), so the daemon serves
      reads itself. `EIDOS_FUSE_PASSTHROUGH=1` re-enables it, and then the
      `CAP_SYS_ADMIN` it needs in the initial user namespace becomes relevant
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
      first, backing up any displaced file, idempotent and best-effort
- [x] Modding-tool prerequisites (`eidos prereqs`) - tools run in the game's shared
      Proton prefix through the merged view, so one prereq set covers them all.
      Tier 1 (bundled, zero network): the DirectX helpers BodySlide / DynDOLOD / CAO
      need (`d3dx9_43` / `d3dx11_43` / `d3dcompiler_43`), declared per-tool in
      `tools.ini` and provisioned at launch. Tier 2 (consented download): the .NET /
      vcrun verbs Synthesis / Pandora / FNIS need, installed via the system
      winetricks pointed straight at Proton's own wine (bypassing the protontricks +
      Proton-GE mismatch), behind an explicit `eidos prereqs <id> --install` / GUI
      "Tool Setup" button, recorded in a per-instance sentinel
- [x] GUI to MO2 daily-driver parity (`eidos-gui`) - a mod-list filter box,
      click-to-select, and a right-click action menu (enable/disable, send to
      top/bottom, open in explorer, visit on Nexus, reinstall, rename, remove,
      information); an interactive Plugins tab (toggle ESP/ESM, persisted to
      `plugins.txt`); a per-mod information dialog (general / conflicts / filetree /
      editable notes); a Version column; and wired Nexus / Change Game / Settings /
      Tool Setup toolbar buttons
- [x] LOOT-based plugin auto-sort (`eidos-loot`) - libloot's own graph sort behind
      `eidos sort` and the GUI's Sort button, with the masterlist fetched and
      cached per instance (refreshed on each sort, like MO2/LOOT) and a userlist
      for local overrides. The post-sort report pops the way MO2's dialog does,
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
      is measurable instead of guessed at
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
      pin, `.desktop` shortcut), and image/text previews. Thirty-seven of the
      thirty-eight are closed; the last is Nexus **collection installation**,
      which is a decision rather than a remainder - Eidos reads a collection and
      says on screen why it does not install one
- [x] The Colony filesystem layout (`eidos-paths`) - `~/.config/Colony/Eidos`,
      with logs under `~/.local/state/Colony/Eidos`, migrated by COPY so a wrong
      migration cannot cost anybody a Nexus session. Four crates had each been
      answering "where do my files go" by hand; there is one answer now. The
      reason to look was worse than the layout: `Settings::save` resolved its
      path globally, so a GUI test rewrote the developer's own preferences on
      every run of the suite - which presented as "options revert after every
      rebuild"
- [ ] Casing normalization at mod-import time
- [ ] Packaging and distribution. Now unconstrained, since the launch capability
      became optional: Eidos runs rootless and only the opt-in passthrough path
      wants `CAP_SYS_ADMIN`. That matters because a file capability lives in the
      `security.capability` xattr of the executable and the kernel ignores it on a
      `nosuid` mount, which is exactly what an unprivileged FUSE mount is forced
      to be (check any FUSE mount on your own machine:
      `findmnt -t fuse -o TARGET,OPTIONS` shows `nosuid,nodev`) - so a
      self-mounting bundle can never carry it, and a sandbox that sets
      no-new-privs cannot gain it either. Self-contained formats (AppImage, and a
      Flatpak modulo its own namespace nesting) are therefore back on the table;
      only someone opting into passthrough needs the binary on a real filesystem
      where `setcap` reaches it

The manager layer above the VFS is complete per the MO2 + usvfs study that drove
this work ([docs/master-pieces.md](master-pieces.md), all 6 master pieces
done): the mod installer (Simple + FOMOD wizard), plugins, conflicts, profiles,
`meta.ini`, the instance manifest, per-game Bethesda features (BSA invalidation +
per-profile INIs/saves off a declarative `GameDef`), and tools through the VFS
(`eidos tool`), plus Nexus integration beyond it (`eidos-nexus`: nxm:// downloads,
update checks, the GUI Downloads tab). Since then: the GUI brought up to MO2
daily-driver parity (separators, categories, diagnostics, Overwrite-to-mod, MO2
profile import), LOOT sorting, native DLL provisioning so Proton graphics mods
(Community Shaders) and tools (BodySlide) just work, the `eidos prereqs`
tool-prerequisite system, and a correctness-plus-caching pass over the daemon
itself.

Since then, and all measured on a real 50-layer instance rather than a benchmark:

- **The layer index** turned path resolution from a walk of every layer into a
  hash lookup, and a save load from twenty seconds into six or seven.
- **Zero-message `opendir`**: 516,301 directory opens per session became 1. Wine
  opens a directory to ask whether it folds case, on every path that fails to
  resolve, and the kernel can answer that itself.
- **The merged-children map** did the same for `readdir`, the one handler the
  path index could not help: 799 ms -> 105 ms across two sessions issuing the
  same 2,063 listings.
- **A layer spelling one directory two ways** no longer loses everything under
  the second spelling - a bug that hid 74 files of one real mod with no error
  anywhere, and which the index had been quietly working around while the
  documented-as-never-wrong fallback was the half that was wrong.
- **Tier 3 prerequisites**: Eidos fetches the .NET runtime DynDOLOD's LOD
  generator needs, and the Executables dialog now says whether each prerequisite
  is actually present instead of only what was typed.

Next up: casing normalization at import, more game families proven in-game, and
packaging - plus the open `plugins.txt` question in troubleshooting.md, which is
the one thing standing between a working mount and a working playthrough.

## On reviewing this work

The MO2-parity batches were each relidden adversarially: independent readers hunt
for defects, other readers try to refute them, and only what survives is fixed.
Across the batches that is 154 confirmed findings, every one reproduced before
being touched.

Two results are worth keeping, because they are the argument for the method
rather than for the count.

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
message no user can produce. A second pass is not caution; it is what catches the
first pass being confidently wrong.
