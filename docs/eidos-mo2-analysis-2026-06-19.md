> Automated 18-agent analysis (read-only) of Eidos vs the real MO2 source
> (modorganizer + usvfs, cloned). The synthesis adversarially re-verified the
> agents' findings and corrected two false-positive HIGH claims. 2026-06-19.

# Eidos vs MO2: Complete Analysis + Improvement Plan
**Date:** 2026-06-19 · **Context:** post-Phase B GUI batch (broad MO2 parity, commit 4a31527)

## 1. Headline

**Eidos is in good health.** The backend is genuinely strong: 232 passing unit tests across 14 crates, correct union-VFS semantics (case-folding, copy-on-write, whiteouts, opacity), MO2-faithful plugin ordering, conflict detection, instance/profile/gamedef handling, and a sound Proton launch pipeline. The architecture is clean MVU. Most "bugs" flagged by the audit agents are real but minor; **two of the highest-severity claims (the `make_dir` whiteout-ordering bug and the conflicts_panel "wrong indices" data bug) do not actually exist** - I read the code and the ordering/indexing is already correct (details below). That is good news: the freshly-generated, visually-untested Phase B GUI has **no confirmed runtime panic in normal flow**, with one real exception.

**The single most important thing to do next:** fix the one real crash-risk in Phase B - the unchecked `.unwrap()` in the async LOOT sort closure (`main.rs:1840`). It is a one-line fix and it is the only place in the new GUI code where a foreseeable error path panics the whole app instead of surfacing a status message. Right after that, fix the two genuine security holes (FUSE `..` traversal and FOMOD destination traversal), then give the 5195-line, zero-test GUI crate at least a thin integration-test harness so the next generated batch isn't flying blind.

---

## 2. CRITICAL / HIGH bugs to fix first

### CRITICAL

**C1. Unchecked `.unwrap()` in async LOOT sort closure (only real Phase B panic)**
`crates/eidos-gui/src/main.rs:1840` - `let repo = eidos_loot::loot_support(&id).unwrap().1;`
Runs inside the `Task::perform` async closure. The `is_supported` guard is at line 1813 on the UI thread, but `loot_support` can still return `None` later (missing/corrupt LOOT data, repo lookup failure). Any such case panics the GUI instead of showing an error.
**Fix:** `let repo = eidos_loot::loot_support(&id).map(|(_, r)| r).ok_or_else(|| "LOOT support unavailable".to_string())?;`
**Effort:** small.

**C2. Path traversal in FUSE virtual-path resolution**
`crates/eidos-core/src/lib.rs:400-416` (`ci_lookup`), `normalize` at 381-383.
`normalize` only trims a leading slash; `components()` yields `ParentDir` markers that are joined verbatim, so a vpath like `a/../../etc/passwd` escapes the layer root. A malicious mod/archive file path can read/write outside the union.
**Fix:** in `normalize` / `ci_lookup`, reject or strip non-`Normal` components (skip `ParentDir`/`RootDir`), or canonicalize and assert containment.
**Effort:** small.

**C3. Path traversal in FOMOD destination handling**
`crates/eidos-install/src/install.rs:521` - `let dst = dest.join(&destination);`
`destination` comes straight from FOMOD XML with only `\`→`/` normalization (line 511). A destination of `../../other_mod/x.esp` escapes the mod root. Compounds C2.
**Fix:** split `destination` on `/`, reject any `..` or empty/absolute segment before `join`; verify the result stays under `dest`.
**Effort:** small.

### HIGH

**H1. `set_enabled()` can enable a force-disabled plugin**
`crates/eidos-plugins/src/lib.rs:315-322` - assigns `p.enabled = enabled` with no `force_disabled` check. A `.esl` on a game without light support can be enabled via the library API, consuming a normal index and shifting every later plugin. The GUI pre-checks, but the library invariant is unsound.
**Fix:** `p.enabled = enabled && !p.force_disabled;` (matches the constraint already enforced in `apply_active`).
**Effort:** small.

**H2. Keyboard shortcuts fire through open About / View / Profile menus**
`crates/eidos-gui/src/main.rs:5175-5182`. The subscription gates on `settings_open`, `executables`, `collision`, `rename`, `info_mod`, `fomod` - but **not** `about_open`, `view_menu_open`, `profile_menu`. Pressing Escape with the About dialog open fires `ClearSelection`, silently wiping the mod-list selection.
**Fix:** add `&& !app.about_open && !app.view_menu_open && app.profile_menu.is_none()`; better, map Escape to close whichever of those is open first.
**Effort:** small.

**H3. `.meta` sidecar line endings normalized to LF, breaking MO2 round-trip**
`crates/eidos-nexus/src/lib.rs:539-548` (`mark_installed`, uses `.lines()` then writes `\n`) and `:583-603` (`write_download_meta`, hardcoded `\n`). MO2 writes CRLF. In a shared downloads folder this corrupts/rewrites every `.meta`, defeating byte-for-byte MO2 compatibility - which is an explicit Eidos design goal (see `eidos-instance/meta.rs` which already tracks a `crlf` flag).
**Fix:** preserve original EOL on read (like `meta.rs`), and write CRLF for newly created sidecars.
**Effort:** small (two functions).

**H4. `prctl(PR_SET_CHILD_SUBREAPER)` return value ignored**
`crates/eidos-launch/src/lib.rs:145`. If subreaper setup silently fails (old kernel, restricted container), tool-spawned children reparent to init; the launcher then exits and unmounts the FUSE union while processes still reference it → `ENOTCONN` and potential save/data loss.
**Fix:** check the return and at minimum `eprintln!` a warning.
**Effort:** small.

**H5. CWD derivation silently skipped for parentless game path**
`crates/eidos-launch/src/lib.rs:155-157`. `spec.mountpoint.parent()` returning `None` makes the `if let Some(...)` skip `current_dir()`, so the game runs with the launcher's CWD. CommonLibSSE-NG SKSE plugins resolve `Data/SKSE/Plugins/versionlib-*.bin` relative to CWD and break.
**Fix:** fail fast with a clear error when `parent()` is `None`.
**Effort:** small.

**H6. Missing FOMOD install collision handling**
`crates/eidos-gui/src/main.rs:1389-1399`. The Merge/Replace/Rename collision prompt is wired only for the Simple installer (line ~1312). FOMOD installs hitting an existing folder just show "Install failed: target already exists" with no recovery.
**Fix:** thread an `OverwritePolicy` into `finish_fomod`, catch `Exists`, open the collision dialog, retry.
**Effort:** medium.

### Corrections to the audit (claimed HIGH/CRITICAL that are NOT bugs)

- **`make_dir` whiteout ordering (claimed HIGH):** `crates/eidos-core/src/lib.rs:217-218` already calls `fs::create_dir_all(&dest)?` *before* `clear_whiteout`, and the `?` returns on failure before any whiteout is cleared. **Not a bug.** No fix needed.
- **`conflicts_panel` "wrong mod indices after filtering" (claimed HIGH/critical, two agents):** `origin = (i+1)` at `main.rs:4181` uses the index from `.enumerate()` which binds *before* `.filter()`, exactly matching the map built at `4144-4145`. **The data lookup is correct.** The only real defect here is cosmetic row striping (`i % 2 == 0` at line 4211 uses the unfiltered index, so stripes desync when disabled mods/separators are interleaved) - LOW severity.
- **`percent_decode` off-by-one (claimed MEDIUM):** `i + 2 < b.len()` correctly decodes a full `%XX` at end-of-string (e.g. `"name%20"`). **Not a bug.**

---

## 3. Findings by theme (ranked within theme)

### Parity gaps (vs MO2 daily-driver features)
| Sev | Finding | Recommendation | Effort |
|---|---|---|---|
| HIGH | No mod backup/restore (`ModBackup`/`ModRestore` absent) | Add `backup/` copy + context-menu restore | medium |
| MED | Missing list columns: Author, Uploader, ModID, Game-source, Install-time (`main.rs:3090-3099`) | Parse from meta.ini, render Author + Game columns | medium |
| MED | Mod-info dialog has 4 of MO2's 9 tabs (`main.rs:70-75`) - missing TextFiles, IniFiles, Images, ESPs, Categories | Add the high-value tabs (ESPs, IniFiles, Images) | large |
| MED | No dedicated Nexus tab / in-dialog endorse-track (`main.rs:3637-3665`) | Add `InfoTab::Nexus` | medium |
| MED | Settings has 2 of 7 MO2 tabs - **Paths** (relocate dirs) and **ModList** (column toggles) most valued | Add Paths + ModList tabs | medium |
| MED | No "Send to First/Last Conflict", no "Create Mod / Sync from Overwrite" actions | Add the conflict-positioning + overwrite-migration actions | small/medium |
| LOW | No three-state endorsement ("Won't endorse"), per-mod force-check, versioning-scheme toggle, remap-category, uploader/custom URL | Batch as a "context-menu completeness" follow-up | small each |
| LOW | No Python/plugin extensibility, themes (1 hardcoded vs 10+), collections (also unsupported upstream) | Defer; document as known limits | large |

### Quality / architecture
| Sev | Finding | Recommendation | Effort |
|---|---|---|---|
| MED | Cache invalidation scattered across 20+ `update()` sites (`main.rs:730…2352`) | Introduce a `CacheDirty` bit-flag set in `update()`, recompute lazily | large |
| MED | Modal state splayed across ~8 `App` fields, no mutual exclusion; multiple modals can stack (`main.rs:4318-4404`) | Collapse to single `modal: Option<Modal>` enum + one `CloseModal` | large |
| LOW | Unreachable dead arm `Screen::Main => welcome()` (`main.rs:5142`; 5133 already returns) | Delete the arm | small |
| LOW | Conflicts-panel row striping uses unfiltered index (`main.rs:4211`) | Use a `visible_idx` counter | small |
| LOW | Magic colors duplicated 3-4× (e.g. `0xF3,0xEA,0xD3`, `0x6E,0x24,0x2E`) | Extract `CARD_BG`/`ACCENT` constants | small |
| LOW | Monolithic 5195-line `main.rs` | Split into `ui/{panels,dialogs,modlist}.rs` | large |
| LOW | `set_enabled` index semantics, rediscovery-updates-headers diverge from MO2 (intentional) | Document; no fix | small |

### Performance (all observable at 50+ mods / 10k+ files)
| Sev | Finding | Recommendation | Effort |
|---|---|---|---|
| HIGH | O(n²) pairwise conflict rebuild runs **synchronously in `update()`** on every toggle/reorder/profile-switch (`eidos-conflicts/src/lib.rs:101-194`; `main.rs:913-919`) | Move to background `Task`; cache until enabled-set actually changes; lazy on Conflicts tab | large |
| HIGH | Per-component case-insensitive `read_dir` re-scans in `ci_lookup` on every casing miss (`eidos-core:400-415`) | Per-operation `(parent,component)` case-fold cache | medium |
| MED | `list_dir` re-scans all layers + NTFS-collation sorts per readdir (`eidos-core:306-368`) | Cache sorted listing per vpath, invalidate on mod change | medium |
| MED | `classify_content_dir` recursive scan per row on every profile switch (`main.rs:602-626`) | Cache result in `RowMeta` keyed by path | small |
| MED | `overwrite_entries()`/`merged_listing()` walk disk **every view frame** while Overwrite/Data tab open (`main.rs:2933-2993`) | Cache in `App`, invalidate on mod change | medium |
| MED | `mods_changed()` recomputes conflicts+plugins+meta unconditionally (`main.rs:913-919`) | Lazy conflicts like plugins already are | small |
| LOW | `icon()` calls `.to_vec()` per icon per frame (`main.rs:2765-2770`); `from_bytes` takes `&[u8]` | Drop the `.to_vec()` | small |
| LOW | Modals (settings/info) rebuilt every frame even when closed (`main.rs:4296-4405`) | Guard construction behind visibility flag | small |
| LOW | Category-filter ancestor walk + NTFS key encoding recomputed per frame | Cache per profile load | small |

### UX
| Sev | Finding | Recommendation | Effort |
|---|---|---|---|
| MED | Menu-bar clicks don't close the View dropdown (`main.rs:1927,1988,2052,2367`) | Set `view_menu_open = false` in those handlers | small |
| MED | Escape doesn't close About / View menu (falls through to ClearSelection) | Wire Escape to close them | small |
| MED | Executables dialog: selecting another tool silently discards mid-edit field text (`main.rs:2060-2065`) | Auto-save or warn before switching | medium |
| LOW | Settings General-tab dropdowns auto-save with no rollback on save failure; Nexus tab inconsistent | Pick one save model | medium |

### Test gaps
| Sev | Finding | Recommendation | Effort |
|---|---|---|---|
| CRIT | **`eidos-gui` (5195 lines) has zero tests**; 27+ Message handlers (batch ops, profile rename/copy/delete, executables state machine, drag-reorder index math, Nexus validate/endorse/update-check) entirely unverified | Add an integration harness driving `update()` over `App` against real `eidos-instance` APIs; cover batch index math, profile collision fallback, drag down-shift adjustment first | large |
| MED | `eidos-nexus` lacks HTTP-error / 429 / resume / bad-JSON tests | Add error-path tests | medium |
| MED | `eidos-loot` lacks DB-load / query-by-name / dirty-info tests | Add query tests | medium |
| LOW | FOMOD malformed-archive error surfacing untested | Add corrupted-XML case | small |

### Robustness / security
| Sev | Finding | Where | Effort |
|---|---|---|---|
| CRIT | FUSE `..` traversal (C2) | `eidos-core:400-416` | small |
| CRIT | FOMOD destination traversal (C3) | `eidos-install:521` | small |
| MED | `unique_download_name` unbounded `(1..)` iterator - theoretical DoS / hang | `eidos-nexus:520-528` | small |
| MED | `file_name_from_uri` accepts percent-decoded `../` after sanitize | `eidos-nexus:488-515` | small |
| MED | `.unwrap()` on file-handle map right after insert in `open`/`create` (concurrent-removal panic) | `eidos-fuse:409,624` | small |
| MED | `reap_descendants` exits on `EINTR`, leaving zombies | `eidos-launch:175-179` | small |
| MED | Bind-mount failures (saves/INI) silently ignored - game reads wrong location | `eidos-launch:123-126` | small |
| MED | CLI `.expect()`/`.unwrap()` panics: `inst.create()` (`eidos/src/main.rs:190`), `loot_support()` (`:878`) | replace with graceful exit | small |
| LOW | INI-capture / Proton env-path failures swallowed | `eidos/src/main.rs:298-304`; `eidos-games/proton.rs:193-209` | small |

---

## 4. Top 10 highest-leverage improvements (prioritized roadmap)

1. **Fix the async LOOT `.unwrap()` panic (`main.rs:1840`).** The only foreseeable crash in the new, untested Phase B GUI - one line. *(small)*
2. **Close both path-traversal holes (FUSE `eidos-core:400`, FOMOD `eidos-install:521`).** Security-critical, malicious-mod exploitable, both small. *(small ×2)*
3. **Stand up a minimal `eidos-gui` integration-test harness.** The crate is a third of the codebase with zero coverage; without it every generated batch ships blind. Cover batch index math, profile collision fallback, drag down-shift, endorse/validate first. *(large, but highest risk-reduction)*
4. **Make `set_enabled` honor `force_disabled` (`eidos-plugins:317`).** Restores a core load-order invariant the library currently lets callers violate. *(small)*
5. **Move conflict rebuild off the UI thread + cache it (`eidos-conflicts:101`, `main.rs:913`).** The single biggest perceived-performance win for large profiles; today every toggle blocks a frame on O(n²) work. *(large)*
6. **Gate keyboard shortcuts behind the remaining modals + wire Escape to close menus (`main.rs:5175`).** Removes a real data-loss surprise (selection wiped) and the most visible "feels broken" UX gap. *(small)*
7. **Preserve CRLF in `.meta` sidecars (`eidos-nexus:539,583`).** Protects the headline promise - drop-in MO2 downloads-folder compatibility. *(small)*
8. **Harden the launch pipeline: check subreaper prctl, fail-fast on missing CWD parent, retry `waitpid` on EINTR, log bind-mount failures (`eidos-launch:145,155,175,123`).** Cluster of small fixes that together prevent silent save/data loss on real launches. *(small cluster)*
9. **Add mod backup/restore + the Author/Game columns and Send-to-Conflict / Create-Mod-from-Overwrite actions.** The most-missed MO2 daily-driver features for migrators; medium effort, high adoption value. *(medium)*
10. **Cache per-frame disk walks (`overwrite_entries`/`merged_listing`/`classify_content_dir`) and drop the `icon().to_vec()` allocation.** Removes repeated I/O and per-frame allocs while a tab is open - cheap, broadly felt smoothness. *(small/medium cluster)*

**Honest bottom line:** the foundation is solid and the audit's two scariest claims were false alarms. The real exposure is concentrated in (a) one crash-prone unwrap, (b) two traversal vulns, and (c) a large untested GUI surface. Items 1-2 are an afternoon; item 3 is the structural investment that de-risks everything Phase C generates next.

Key files: `crates/eidos-gui/src/main.rs`, `crates/eidos-core/src/lib.rs`, `crates/eidos-install/src/install.rs`, `crates/eidos-plugins/src/lib.rs`, `crates/eidos-nexus/src/lib.rs`, `crates/eidos-launch/src/lib.rs`, `crates/eidos-conflicts/src/lib.rs`, `crates/eidos/src/main.rs`.
