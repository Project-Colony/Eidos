> Automated re-audit (15-agent workflow): re-verified the 56 findings of
> [`mo2-parity-findings.json`](mo2-parity-findings.json) against current code and swept
> under-covered areas. Generated 2026-06-17. Spot-check a few items before acting -
> the synthesis flags 1-2 internal contradictions to confirm.

# Eidos vs MO2: What Is NOT Yet Implemented (2026-06-17)

## Headline

Eidos has closed the vast majority of the 2026-06-10 backend audit: of the **56 known findings, 47 are confirmed FIXED, 8 are PARTIAL, and 1 is fully OPEN** (`tools.ini` args splitting appears once as open and once as fixed across two re-verifications of the same subsystem — the fix landed at `tools.rs:61-62`, so it is effectively **FIXED**, leaving the per-tool steam-app-id divergence as the only genuinely untouched item there). The **engine is now at near-parity with MO2**: VFS union semantics, conflict detection, plugin load order, Bethesda BSA/INI handling, Nexus downloads, and the Proton launch pipeline are all solid and test-covered. **The gap has decisively shifted from "does the core work" to "is the daily-driver UI wired up."** The re-verify sweep surfaced ~13 new backend gaps (mostly FOMOD condition evaluation and GUI plumbing of already-implemented backends), and the discovery sweep confirms the real distance to a daily driver is **GUI features and LOOT metadata depth**, not core correctness. Honest summary: the backend is shippable; the GUI is a thin shell over a capable engine, and that shell is where the remaining work concentrates.

---

## Still Open (by theme, ranked by severity within each)

### Theme 1 — GUI is a shell: implemented backends not wired to buttons

These are the highest-leverage gaps because the backend already exists; only the GUI wire is missing.

| Sev | Kind | What MO2 does / Eidos gap | Effort |
|-----|------|---------------------------|--------|
| **high** | missing-behavior | **Overwrite-collision chooser.** `install_archive_with_policy()` fully implements Replace/Merge/Rename (`install.rs:168-266`), but the GUI still shows "target already exists" with no picker. CLI has `--replace/--merge`. | medium |
| **medium** | missing-feature | **Executables button is `Message::Noop`** (`eidos-gui/src/main.rs:1641`). The whole `tools.ini` backend exists; no dialog edits it. | medium |
| **medium** | missing-feature | **Endorse + Update toolbar buttons are `Noop`** (`main.rs:1645-1646`). No bulk-endorse, and Update being dead means the (already-implemented) `^` update marker at `main.rs:1678-1682` can never be populated from the GUI. | medium |
| **medium** | missing-feature | **Settings/Preferences dialog absent.** Settings button just opens the instance folder (`main.rs:1643`). Nexus API key is only settable via CLI. | large |
| **medium** | missing-behavior | **No drag-drop mod reordering.** `MoveUp/MoveDown` exist but the `scrollable Column` has no drag events. | large |
| **medium** | missing-behavior | **No multi-select / batch actions.** `selected_mod: Option<usize>` (`main.rs:205`) is single-selection; no Ctrl/Shift handlers, no bulk enable/disable/delete. | large |
| **medium** | missing-feature | **Menu bar entirely `Noop`** (File/View/Tools/Run/Help, `main.rs:1623-1631`). | large |
| **low** | missing-behavior | Profile rename/delete UI absent (backend `profile.rs:177-185` exists, no wiring). | small |
| **low** | missing-feature | Per-mod Endorse/Unendorse context-menu item absent; `endorsed()` has no setter (`main.rs:1962-2053`). | small |
| **low** | missing-behavior | No active/endorsed/update count summary in modlist header (`main.rs:1779` shows only `Active: N`). | small |
| **low** | missing-feature | No keyboard shortcuts (Run, Refresh) and no rebinding UI. | small/medium |
| **low** | missing-feature | No drag-drop archive install from file manager. | medium |
| **low** | missing-feature | No theme/appearance settings (palette hardcoded `main.rs:1280-1290`). | medium |

### Theme 2 — Missing whole GUI surfaces

| Sev | Kind | What MO2 does / Eidos gap | Effort |
|-----|------|---------------------------|--------|
| **high** | missing-feature | **No Saves tab.** `Tab` enum has no `Saves` variant (confirmed `main.rs:58-64`). No savegame browsing/management. | medium |
| **medium** | missing-behavior | **No real download manager** (active/queued/completed, speed, pause/resume). `downloads_panel()` is a read-only completed-archive list (`main.rs:2348-2410`). | large |
| **medium** | missing-behavior | **Data tab has no file-type filter / folder tree** (flat 3-col table, `main.rs:2287-2308`). | medium |
| **low** | missing-behavior | Conflicts tab is non-interactive (no tree/search/filter, `main.rs:2568-2610`). | large |
| **low** | missing-feature | No activity/message log panel (single-line `app.status` only). | medium |

### Theme 3 — FOMOD condition evaluation incomplete (core correctness, not just UI)

These are genuine behavior divergences from MO2, parsed-but-not-evaluated. The most important class of remaining *engine* work.

| Sev | Kind | What MO2 does / Eidos gap | Effort |
|-----|------|---------------------------|--------|
| **medium** | missing-behavior | **`<moduleDependencies>` parsed but never evaluated.** Captured at `model.rs:110` / `parse.rs:78`; `finish_fomod()` and `build_plan()` never test it. A FOMOD requiring e.g. SKSE silently installs instead of aborting like MO2. | medium |
| **medium** | missing-behavior | **`alwaysInstall` / `installIfUsable` parsed but never evaluated** (`parse.rs:111-112`); Eidos installs all selected-option files unconditionally. | medium |
| **medium** | divergence | **`fileDependency` cannot distinguish Inactive from Missing.** `fomod_context()` (`install.rs:374-390`) only marks enabled-mod plugins Active; a plugin in a disabled mod reads as Missing, so `state="Inactive"` checks fail. | medium |
| **low** | missing-behavior | FOMOD plan missing-sources surfaced on CLI but not in GUI (`after_install()` drops `report.missing`, `main.rs:779`). | low |
| **low** | refinement | Empty `<file>/<folder source="">` correctly skipped but, unlike MO2, no debug log/user feedback. | low |

### Theme 4 — LOOT integration is sort-only (no metadata depth)

Eidos sorts correctly but exposes none of LOOT's advisory metadata. This is the single largest *feature* gap vs a polished MO2 setup.

| Sev | Kind | What MO2 does / Eidos gap | Effort |
|-----|------|---------------------------|--------|
| **high** | missing-feature | **Masterlist messages/warnings never surfaced.** `sort()` calls `sort_plugins()` but never `game.plugin()`; `libloot::Plugin` is never read (`eidos-loot/src/lib.rs`). | large |
| **high** | missing-behavior | **Dirty-plugin CRC / cleaning recommendations absent.** No `dirty_info` access; Plugin struct (`eidos-plugins/src/lib.rs:60-71`) doesn't model it. | large |
| **high** | missing-feature | **No `userlist.yaml` editor** (load-after rules, groups, tags). Path is passed to `sort` but file is never created/edited. | large |
| **medium** | missing-feature | Plugin metadata (requirements, load-after, groups, messages, tags) not modeled in the Plugin struct. | medium |
| **medium** | missing-feature | No masterlist/prelude version + auto-update UI (CLI `--update-masterlist` exists; GUI has none). | small |
| **medium** | missing-behavior | Requirements/incompatibilities not validated pre-sort; `load_current_load_order_state()` not called. | medium |
| **medium** | missing-feature | No sort before/after diff or review-before-commit (GUI applies result directly). | medium |
| **medium** | missing-behavior | Sort failures not explained (generic `libloot: <msg>`, no culprit plugin). | small |
| **medium** | missing-feature | No per-plugin masterlist metadata in mod-info dialog (no Plugins sub-tab). | large |
| **low** | missing-behavior | Bash tags not extracted (`Plugin::bash_tags()` never called). | small |

### Theme 5 — Mod-management operations missing

| Sev | Kind | What MO2 does / Eidos gap | Effort |
|-----|------|---------------------------|--------|
| **medium** | missing-feature | Create empty mod folder (for manual/patch mods). | small |
| **medium** | missing-feature | Batch enable/disable mods (needs multi-select). | medium |
| **medium** | missing-feature | Install mod from an existing unpacked folder (only 7z archives supported). | medium |
| **low** | missing-feature | Merge two mods into one. | medium |
| **low** | missing-feature | Mark individual plugins optional (no per-file state in Plugin struct). | medium |
| **low** | missing-feature | Mod groups / load-order snapshots. | large |
| **low** | missing-feature | Ignore-update flag per mod (no `ignore_update` in ModMeta). | small |
| **low** | divergence | Multi-category storage/filtering (meta stores one category, MO2 stores comma-separated list). | small |
| **low** | partial | Version rollback / reinstall-old-version picker (Reinstall always grabs latest). | medium |
| **low** | partial | Restore modlist from `.bak` (backup written `profile.rs:232-234`, no restore UI). | small |
| **low** | partial | Per-mod `_backup` create/restore on Replace (policy exists, no backup generated). | small |
| **low** | partial | `nexusFileStatus` display in Downloads (field exists `meta.rs:215-219`, not shown). | small |

### Theme 6 — Instances, settings & first-run

| Sev | Kind | What MO2 does / Eidos gap | Effort |
|-----|------|---------------------------|--------|
| **high** | missing-feature | Register/discover **portable instances**. `Instance::portable()` exists (`lib.rs:95`) but nothing enumerates/registers them. | large |
| **medium** | missing-feature | **Manage non-Steam / non-Bethesda games** (MO2 basic_games). `detect()` only finds ~9 Steam Bethesda titles. | large |
| **medium** | missing-feature | **Global settings persistence** (window state, theme, default instance) — no `settings.ini` equivalent. | medium |
| **medium** | missing-feature | Instance enumeration / switching UI (only `ChangeGame` reopens the picker). | medium |
| **medium** | missing-behavior | **Nexus API key not persisted** — re-prompted every session (`main.rs:1032-1048`, env/stdin only). | medium |
| **medium** | missing-feature | "List all instances" command (CLI/GUI). | medium |
| **low** | missing-feature | Nexus Collections (`.ncc` / `nxm://collection`) — explicitly rejected (`eidos-nexus/src/lib.rs:38-48`). | large |
| **low** | partial | Toggle `tracked` / `endorsed` from GUI (accessors exist, no setters/UI). | small |
| **low** | missing-feature | Download install-status badge (now backed by `mark_installed()`, GUI not wired). | small |
| **low** | missing-feature | API key OS-keyring storage. | small |
| **low** | missing-feature | Per-tool custom icons. | small |
| **low** | partial | First-run detection / onboarding (wizard exists, no auto-trigger or API-key step). | small |

### Theme 7 — Remaining engine/backend correctness gaps

| Sev | Kind | What MO2 does / Eidos gap | Effort |
|-----|------|---------------------------|--------|
| **medium** | bug | **`eidos-launch` standalone binary doesn't map signal exits.** `main.rs:66` uses `exit(status.code().unwrap_or(0))` — confirmed; the eidos CLI wrapper maps `128+signal` correctly, but direct `eidos-launch` invocations report signal-killed children as exit 0. | small |
| **medium** | missing-behavior | **New plugins default to enabled** (`lib.rs:195` `enabled: !force_off` — confirmed); MO2 defaults fresh installs inactive. Deferred pending a plugin-toggle UI. | small |
| **medium** | missing-behavior | Profile `rename()`/`delete()` lack active-profile guards (`profile.rs:177-185`); a caller can rename/delete the active profile. | small |
| **low** | divergence | Skyrim LE missing `SkyrimCustom.ini` (`gamedef/src/lib.rs:133`); SkyrimSE has it. Out of original scope but a logical parity gap. | small |
| **low** | missing-feature | Tool model lacks per-tool steam-app-id override (`tools.rs:16-31`); the only genuinely untouched item from the executables subsystem. | low |
| **low** | missing-behavior | Overwrite layer included in plugin discovery but not in conflict-emblem computation (`eidos-conflicts/src/lib.rs:120-159`) — note this contradicts the conflicts subsystem re-verify, which marks the Overwrite-in-conflicts item *fixed* at `eidos-gui/src/main.rs:2554-2558`; the GUI inserts Overwrite at origin `u32::MAX`. Treat as **likely already fixed**; verify the two code paths agree. | low |

---

## Top highest-leverage next features (ranked)

1. **GUI overwrite-collision chooser (Replace/Merge/Rename).** The backend is done (`install.rs:168-266`); a GUI installer that errors on every re-install is a daily-driver blocker. Highest ratio of impact to effort. *(medium)*
2. **Wire the Executables dialog + Endorse/Update toolbar buttons.** Three dead `Noop` buttons over fully-built backends (`tools.ini`, endorsement, update check). Wiring Update also activates the already-implemented `^` update markers. *(medium)*
3. **Settings/Preferences dialog with persisted Nexus API key.** Re-entering the key every session is the single most grating friction point for downloads/updates; pairs naturally with a `settings.ini` for window/theme state. *(medium)*
4. **LOOT messages + dirty-plugin surfacing.** Eidos sorts but hides every warning, cleaning flag, and compatibility message LOOT produces — the biggest "feels unfinished vs MO2" gap. Start by modeling `libloot::Plugin` metadata into the Plugin struct, then render tooltips. *(large, but high payoff; can ship incrementally)*
5. **Multi-select + batch enable/disable (and drag-drop reorder).** Single-selection (`selected_mod: Option<usize>`) makes large lists painful; unlocks batch ops and "Send to top/bottom." *(large)*
6. **FOMOD `<moduleDependencies>` + `alwaysInstall`/`installIfUsable` evaluation.** The one class of *engine* correctness still diverging: FOMODs that should abort or conditionally install currently install everything. Parsed already; needs evaluation in `engine.rs`/`finish_fomod()`. *(medium)*
7. **Saves tab.** A first-class MO2 surface with no Eidos equivalent; medium effort, high visibility for per-profile workflows. *(medium)*
8. **Fix `eidos-launch/src/main.rs:66` signal mapping + add profile active-guard.** Two small, isolated correctness fixes that prevent silent "success on crash" and active-profile corruption. *(small)*

---

## Confirmed closed since 2026-06-10 (titles only)

**ESP/ESM/ESL load order**
- ESPFE light-flagged .esp no longer hoisted into master block
- PlainList disabled plugins no longer re-enabled / order preserved
- plugins.txt written as CP1252 (Windows ANSI) not UTF-8
- .esl force-disabled on games without light-plugin support
- Atomic plugins.txt writes + empty-list guard

**Archive installer + ModDataChecker**
- DataText layer support (`Data/` + readme.txt installs correctly)
- `guess_mod_name` extracts mod id, tolerates lettered version segments
- Sidecar `modName` given precedence over filename guess
- `fixDirectoryName`-style sanitization
- meta.ini seeding (date-fallback version, nexusFileStatus, absolute installationFile)

**FOMOD scripted installer**
- Empty `<file>/<folder>` source skipped
- Empty/trailing-slash destination resolved (copyLeaf)
- `order` attribute defaults to Ascending
- SelectExactlyOne/AtLeastOne fallback ordering (Optional beats CouldBeUsable)

**Conflict detection**
- Pairwise conflict relations among all providers
- Overwrite layer included in conflict detection (origin `u32::MAX`)
- `hasHiddenFiles` computed and surfaced

**Profiles**
- modlist.txt written atomically (temp + rename + .bak)
- `create_from` copies saves/ recursively
- Modlist parser handles `*` foreign lines and trims after +/- marker

**Per-game Bethesda features**
- Fallout4/Starfield `sResourceDataDirsFinal=` invalidation
- Oblivion/FNV/FO3 dummy BSA + SArchiveList + SInvalidationFile dance
- Morrowind INI/archive (`[Archives]`) support
- `[Launcher] bEnableFileSelection=1` before every run

**Nexus API + downloads**
- CDN filename sanitized (no `../` escape)
- `cmd_nxm` uniquifies instead of overwriting
- Update check no longer capped at one month (last_nexus_update tracking)
- HTTP Range download resumption (`.unfinished`)
- Rate-limit headers (X-RL-*) parsed; loop stops on 429
- `.unfinished` suffix appended not substituted
- Protocol-Version / Application-Name / Application-Version headers
- `.meta` `installed` flag maintained via `mark_installed()`

**Launch pipeline + Proton**
- Overwrite-layer plugins preserved in plugins.txt
- Tool launches set SteamAppId/SteamGameId/STEAM_COMPAT_APP_ID
- Union unmounted on full process tree (subreaper + reap_descendants)
- Forced-libraries (WINEDLLOVERRIDES) wired into launch
- Signal-killed child maps to 128+signal (in eidos CLI wrapper)
- Proton verb is `waitforexitandrun` + STEAM_COMPAT_INSTALL_PATH set
- Tool titles sanitized in tools.ini headers
- `--print` only honored before `--` separator
- **tools.ini args round-trip lossless (per-arg keys, `tools.rs:61-62`)** — listed open in one pass, fixed in another; confirmed fixed

**VFS union semantics**
- Case-folded write/delete paths (no lost deletes / split-brain copies)
- Recursive directory rename across layers
- Opaque marker prevents whiteout-resurrection on mkdir
- Copy-up preserves mtime + user xattrs
- O_TRUNC no longer copies content up before discarding

---

**Files most relevant to the remaining work:** `crates/eidos-gui/src/main.rs` (GUI shell — most "Still open" items live here), `crates/eidos-loot/src/lib.rs` (metadata depth), `crates/eidos-fomod/src/engine.rs` + `crates/eidos-install/src/install.rs` (FOMOD condition evaluation), `crates/eidos-launch/src/main.rs:66` (signal mapping), `crates/eidos-instance/src/profile.rs:177-185` (active-profile guards).
