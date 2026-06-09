# Eidos: the master pieces

> Derived from a 12-agent deep study of [`ModOrganizer2/modorganizer`](https://github.com/ModOrganizer2/modorganizer)
> + [`ModOrganizer2/usvfs`](https://github.com/ModOrganizer2/usvfs) source, each subsystem
> compared against the Eidos code. 2026-06-09. Raw per-subsystem findings:
> [`mo2-study-findings.json`](mo2-study-findings.json).

## Executive summary

Eidos has already won the hard, novel battle that defines the project: **the VFS
mechanism itself**. `eidos-core` (priority resolve, copy-up, whiteouts, case-fold,
NTFS-sorted readdir) + `eidos-fuse` (kernel passthrough for DLL image-mapping, COW
Overwrite, xattr) + `eidos-launch` (per-launch user+mount namespace) are a
complete, behaviour-equivalent replacement for usvfs's ~30 NT/Win32 hooks - because
Wine translates every NT path call into a host syscall that lands on the FUSE mount.

The single most important conclusion of the study is therefore a **negative** one:
**do NOT port the hooking layer**, the redirection-tree-as-IPC, the re-entrancy
guards, or module-path spoofing. Those are artifacts of intercepting *above* Wine;
Eidos intercepts *below* Wine and gets them for free. The 110-mod Skyrim SE run with
~50 SKSE DLLs loading proves the mechanism is real.

What Eidos is **not yet** is a mod **manager**. It merges files the user
hand-arranges, but it cannot install a mod, cannot control which ESP plugins load or
in what order, cannot tell the game which BSAs to read, cannot explain a conflict,
and cannot run a tool (xEdit/FNIS/BodySlide) against the merged view. Every one of
those is table-stakes for replacing MO2, and **none of them live in the FUSE layer -
they live in new crates above it.**

The 2-3 biggest levers:

1. **ESP/ESM plugin load order** + writing `plugins.txt`/`loadorder.txt` into the
   Proton prefix. Without this, the 110-mod test only worked because it never
   exercised ESP plugins. This is THE correctness gap, and it forces building a
   reusable prefix-path resolver (compatdata -> AppData/Local + Documents) that four
   other features reuse.
2. **A mod installer** (Simple/wrapper-strip first, then FOMOD) - the difference
   between a hand-curated tech demo and a daily-driver tool.
3. **Tools-through-VFS re-entry** - turns the validated merged view from game-only
   into the generate-then-play loop that is the entire point of a Bethesda mod manager.

## What Eidos already has (the hard part is done)

- **Path-interception mechanism**: `eidos-core` LayerStack + `eidos-fuse` are a
  complete, behaviour-equivalent replacement for usvfs's ~30 NT/Win32 hooks. Wine
  translates `NtCreateFile`/`NtOpenFile`/`NtQueryAttributesFile`/`NtQueryDirectoryFile`/
  `GetFileAttributes`/`MoveFile`/`CopyFile`/`GetPrivateProfileString`/`FindFirstFile`
  into host syscalls that hit the mount, so all are covered implicitly. **Do not port
  any individual hook.**
- **Re-entrancy guards, GetLastError preservation, device-file/`hid#` filtering,
  per-handle search-state tracking**: Windows-in-process artifacts with no FUSE
  equivalent and no need.
- **Module-path spoofing / inverse tree**: obviated by mounting in-place AT the game
  Data dir - a DLL's mmap path IS its virtual path, so `GetModuleFileName` is correct
  with zero hooks (validated: ~50 SKSE plugins, CommonLibSSE-NG included).
- **Write redirection / Overwrite**: `eidos-core` implements MO2's single
  create-target more cleanly than usvfs, with real COW copy-up and `.eidoswh`
  whiteouts. Exactly one writable layer, matching MO2.
- **Directory enumeration merge**: `eidos-core` `list_dir` is functionally correct
  and, in the ways that matter (fully sorted, deterministic, highest-layer wins,
  whiteout-aware), *better* than usvfs (which intentionally does not globally sort).
- **DLL image-mapping**: `eidos-fuse` kernel passthrough (Linux 6.9+); `writeback_cache`
  deliberately off.
- **Per-launch isolation + child-process propagation**: the user+mount namespace is
  inherited by every child of the game - covering usvfs's `CreateProcessInternalW`
  injection purpose with zero injection.
- **Steam install detection**: `eidos-games` parses `libraryfolders.vdf` +
  `appmanifest_*.acf` across libraries.

## The master pieces (ranked)

### 1. ESP/ESM/ESL plugin load order + `plugins.txt`/`loadorder.txt` into the prefix
- **Complexity:** high &nbsp;|&nbsp; **Target:** `eidos-plugins` (new) + `eidos-games` + `eidos-launch` + `eidos-gui` &nbsp;|&nbsp; **Depends on:** nothing
- **Why:** For every game the MOD-FOLDER order (which loose file wins - solved by the
  union) and the PLUGIN order (which `.esp`/`.esm` record wins, which FormID modindex
  each gets) are two INDEPENDENT axes, and BOTH must be correct or the game crashes
  (a plugin loaded before its master = instant CTD). The 110-mod run only worked
  because SKSE DLLs are loose files; the moment a user enables a plugin-bearing mod
  they have no way to activate, order, or sort it.
- **Gap:** Zero plugin model. `eidos-instance` has one axis (`ModEntry` + `modlist.txt`).
  No TES4 header parser, no mod-index, no master-before-child constraint, no
  missing-master detection. Nothing writes `plugins.txt`/`loadorder.txt`, and nothing
  knows the prefix path where they live. (The earlier belief that Eidos copies
  `plugins.txt` is **not** implemented anywhere.)
- **Build:** New crate `eidos-plugins`. Add the [`esplugin`](https://crates.io/crates/esplugin)
  crate (the one libloot itself uses) to parse the TES4 header (is_master/is_light/
  is_medium, masters()) with zero hand-rolled binary parsing. Build a `PluginList`
  mirroring MO2's `ESPInfo`; port MO2's three battle-tested invariants verbatim
  (compact priorities; pin game masters; masters-above-normals + master-before-dependent)
  and index computation (`FE:xxx` light, `FD:xx` medium). Discover plugins by scanning
  each ENABLED mod folder in instance load order. Persist via a per-game
  `LoadOrderMechanism`: asterisk `plugins.txt` (active prefixed `*`) for
  skyrimse/vr/fo4/starfield/enderalse; plain `plugins.txt`+`loadorder.txt` for
  skyrim/fo3/fnv. **Build the prefix-path resolver here** (compatdata -> AppData/Local +
  Documents) - pieces 3 and 4 reuse it. Add a Plugins tab; write `plugins.txt` right
  before launch. (Defer libloot sorting: it is GPL-3.0-or-later - flag the licensing
  decision before linking.)

### 2. Mod installer: archive extraction + Simple/wrapper-strip, then FOMOD
- **Complexity:** high &nbsp;|&nbsp; **Target:** `eidos-install` (new) + `eidos-gui` + `eidos` CLI &nbsp;|&nbsp; **Depends on:** per-mod meta.ini (Tier 1 writes it)
- **Why:** Turns a downloaded Nexus archive into a usable `mods/<name>/`. Today the GUI
  tells the user to extract by hand, which breaks the two most common cases: a wrapper
  folder (`ModName-1234/Data/...`, one level too deep so nothing resolves) and a FOMOD
  (`fomod/ModuleConfig.xml`, the dominant format for any non-trivial mod).
- **Gap:** No installer at all - zero archive/xml deps. The install TARGET is ready:
  `eidos-instance::mods_dir()` + auto-include of new folders.
- **Build:** New crate `eidos-install`, two tiers. **Tier 1** (days, unlocks most
  archives): archive backend (`sevenz-rust2` + `zip` + `unrar`, or shell to `7zz`); a
  light `ArchiveTree` reusing `eidos-core`'s comparator; port the Simple installer's
  single-folder recursive descent + a per-game `ModDataChecker` (Gamebryo folder/suffix
  sets); extract the valid subtree into `mods/<name>/` and write `meta.ini`. **Tier 2**
  (weeks, for "complete"): FOMOD via `quick-xml` - port the data model as Rust enums
  into a UI-agnostic `FomodEngine` (`visible_steps()`, `plugin_type()`,
  `build_tree(selections)`), unit-test the condition evaluator + tree assembly HEADLESS,
  then add the iced wizard.

### 3. Tools-through-VFS: persistent named namespace + re-entry for xEdit/FNIS/BodySlide
- **Complexity:** high &nbsp;|&nbsp; **Target:** `eidos-launch` + `eidos-instance` + `eidos-gui` &nbsp;|&nbsp; **Depends on:** plugin load order
- **Why:** The entire point of a Bethesda mod manager is the generate-then-play loop:
  FNIS/Nemesis build behaviour files, BodySlide builds meshes, xEdit cleans/patches -
  all MUST see the same merged Data and write into Overwrite so the next launch picks
  it up. Without this the validated view is usable by the game but by nothing else.
- **Gap:** Eidos is the structural INVERSE of usvfs - usvfs has one long-lived NAMED VFS
  many processes attach to; Eidos has a fresh ANONYMOUS namespace per process, destroyed
  the instant `launch()` returns. Zero `setns`/`nsenter`/persistent-mount/tool-list code.
- **Build:** **Part A** (the architectural fix, `eidos-launch`): refactor one-shot
  `launch()` into a long-lived VFS-session - a holder process does the privileged
  `unshare(CLONE_NEWNS)`, mounts the union ONCE, then PARKS holding ns + mountpoint open.
  Add `eidos enter --session <id> -- <cmd>` that `setns` into `/proc/<holder>/ns/mnt`
  (the analog of `usvfs_proxy --instance`). One FUSE mount = one inode space = one COW
  Overwrite (so tool output lands in Overwrite for the next launch). Persist the session
  under `XDG_RUNTIME_DIR/eidos/<instance>/session`. (Requires the privileged/passthrough
  mode.) **Part B** (`eidos-instance` + GUI): an ordered `Tool` struct persisted beside
  `modlist.txt`, seeded per-game; wire the stubbed Executables button.

### 4. Per-game features for Bethesda correctness (BSA invalidation + INIs + saves)
- **Complexity:** high &nbsp;|&nbsp; **Target:** `eidos-games` (schema) + `eidos-gamefeatures` (new) + `eidos-launch` &nbsp;|&nbsp; **Depends on:** plugin load order (prefix resolver), profiles
- **Why:** Three MO2 game features are not polish - without them mods are silently
  ignored. **BSAInvalidation + DataArchives**: Bethesda engines prefer files packed in
  vanilla BSAs over loose files unless invalidation is active, and many mods ship BSAs
  that must be added to `SResourceArchiveList` to load AT ALL. Per-profile documents-dir
  INIs (`skyrim.ini`/`skyrimprefs.ini`) and isolated saves are core MO2 value. The
  script-extender loader swap is the launch entry point itself.
- **Gap:** Eidos has 1 of MO2's ~10 per-game knobs (data_dir), plus a hardcoded SE-loader
  swap stranded in the GUI. No INI writer, no `[Archive]` management, no dummy-BSA, no
  documents-dir knowledge, no save redirection.
- **Build:** Mostly DATA. (1) Expand `eidos-games::GameDef` into a declarative descriptor
  mirroring MO2's `basic_games` schema: `documents_subdir`, `ini_files`,
  `primary_plugins`/`dlc_plugins`, `load_order_mechanism`, `script_extender` (move the
  GUI match here), `bethesda_archive` keys. (2) `eidos-gamefeatures` module implementing
  the three features as Linux/Proton-native ops invoked BEFORE the mount, reusing the
  prefix-path resolver: an INI writer for `[Archive]` keys + `bInvalidateOlderFiles`,
  port `DummyBSA` into Overwrite, per-profile INI copy into Documents, save redirection.
  (3) `eidos-launch::LaunchSpec` gains a `prepare()` pre-launch hook. Ship BSA
  invalidation first.

### 5. Per-file conflict detection + display (winners/losers)
- **Complexity:** medium &nbsp;|&nbsp; **Target:** `eidos-conflicts` (new) + `eidos-gui` &nbsp;|&nbsp; **Depends on:** nothing
- **Why:** Conflict resolution is THE day-to-day reason MO2 exists. Across 50-110 mods
  many files come from several mods and which wins decides whether the game looks right
  or breaks. A replacement that only says "highest wins" cannot explain WHY the merged
  view looks as it does.
- **Gap:** No conflict model. `resolve_read` returns the first hit and discards that lower
  layers also had the path; `list_dir` keeps only the winner. The GUI Flags column only
  prints "off".
- **Build:** New pure module `eidos-conflicts` (out of `eidos-fuse`, unit-testable). Walk
  every enabled layer ONCE (`walkdir`) into a `VirtualTree` keyed by lowercased relative
  path, reusing `eidos-core`'s comparator + honoring whiteouts; per path store
  `{winner, alternatives}`; derive per-mod `{overwrites, overwritten_by, state}` (MO2's
  Redundant/Mixed rules). GUI: recursive Data tree + per-mod Conflicts panel + a flag on
  each mod row. **Bonus:** fold this pre-merged tree into `eidos-core` to retire the
  per-lookup FS stat - the "no lowerdir wall" the architecture doc still calls aspirational.

### 6. Profiles (per-profile mod state, load order, INIs, saves)
- **Complexity:** medium &nbsp;|&nbsp; **Target:** `eidos-instance` + `eidos` CLI + `eidos-gui` &nbsp;|&nbsp; **Depends on:** nothing (but prerequisite for piece 4's per-profile INIs/saves)
- **Why:** One mod collection serving many playthroughs/configs, each with its own enabled
  set, order, INIs and optional saves, sharing the same downloads. A top reason people
  pick MO2 over Vortex.
- **Gap:** No profile concept. `eidos-instance` is flat (`modlist.txt` + `overwrite/` at the
  root, shared). "Default" is hard-coded in two cosmetic GUI spots.
- **Build:** In `eidos-instance`, thread one `--profile` param through CLI/GUI/launch. Keep
  `mods/` and `downloads/` SHARED; add `profiles/<name>/` holding per-profile
  `modlist.txt` (move it here), reserve `plugins.txt`/`loadorder.txt`, optional `saves/`,
  copied INIs. A `Profile` struct owns `modlist()`/`load_order()`. Auto-create Default and
  one-time-migrate existing flat instances. GUI: real picker + new/copy/rename/delete.

## Quick wins (connective tissue - ship first)

1. **Per-mod `meta.ini`** in `eidos-instance` (new `meta.rs`): a `ModMeta` struct
   read/written as `mods/<name>/meta.ini` keeping MO2's EXACT key names (gameName, modid,
   version, newestVersion, installationFile, category, repository, endorsed, ...) so
   existing MO2 instances **round-trip unchanged** - the highest-leverage non-FUSE work
   left and the enabler of update checks + categories + provenance + zero-friction MO2
   migration. ~1 day; folds into installer Tier 1.
2. **Instance manifest** `eidos-instance.ini` (schema_version, game_id, selected_profile)
   read to recover the game id instead of inferring from the path. Unblocks portable
   instances + forward-compat.
3. **`LaunchSpec` env field**: `env: Vec<(String,String)>` (+ optional cwd override),
   applied via `cmd.envs()` - ~10 lines, unlocks forced-libraries + tools-through-VFS env.
4. **Forced libraries via `WINEDLLOVERRIDES`** on top of the env field - the honest Linux
   equivalent of `usvfsForceLoadLibrary`, what makes ENB/ReShade/`.asi` mods viable.
5. **NTFS-collation comparator** (~20 lines) in `eidos-core::list_dir`: replace the
   `to_ascii_lowercase` sort key with an UPCASE-to-UTF-16 code-unit comparator (what
   NTFS/usvfs actually use), removing the "aspirational" caveat. **Also add a real
   emission-order test** - the current `union.rs` test sorts before asserting, so it
   tests nothing.
6. **Module-path-spoofing regression note**: one integration check that a plugin querying
   its own module path sees the mount path, then document why usvfs's inverse-tree is
   unnecessary - closes a usvfs subsystem with near-zero code.
7. **`update_available(meta)` helper** + `(game_id, nexus_id)` grouping fn (pure,
   unit-testable) so a future Nexus crate only fills `newestVersion` - do not build the
   HTTP client yet.

## Recommended sequencing

1. **Quick wins first** (meta.ini + instance manifest + env field + NTFS comparator,
   ~2-3 days). The meta.ini unlocks zero-friction migration from existing MO2 instances,
   making every subsequent feature testable against real data the user already has.
2. **Piece 1 - `eidos-plugins`** (top correctness gap + foundation): (a) esplugin parse +
   PluginList + the three ordering invariants + index computation, pure and unit-tested;
   (b) the writer AND the prefix-path resolver, validated against the real 110-mod prefix
   on disk. Steps a+b flip Eidos from "merges files" to "usable manager". The prefix
   resolver is reused by pieces 3 and 4.
3. **Piece 2 Tier 1 - `eidos-install`** (archive + Simple-installer + ModDataChecker +
   meta.ini write). Removes the biggest daily-use blocker. FOMOD (Tier 2) follows.
4. **Piece 6 - Profiles** BEFORE piece 4's per-profile INIs/saves, so those are built
   profile-scoped from day one.
5. **Piece 4 - per-game features** (BSA invalidation first, then per-profile INIs + saves)
   on top of pieces 1 (prefix resolver) and 6 (profiles).
6. **Piece 3 - tools-through-VFS** (Part A persistent namespace + setns, then Part B tool
   list + UI) - the genuinely novel Linux engineering; benefits from plugins.txt generation.
7. **Piece 5 - conflicts** woven throughout, opportunistically, paired with the recursive
   Data-tree GUI work in piece 2. Folding its tree into `eidos-core` also retires the
   per-lookup FS scan.
