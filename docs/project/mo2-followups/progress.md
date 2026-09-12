# Eidos follow-up progress

Plan: docs/project/mo2-followups/plan.md
Base: 9f01d7dc94cf257ccbc579fee8ccf0f39de73f19 (v1.17.1).
Branch: eidos-mo2-followups.

## Publication boundary — complete

[Eidos 1.17.1](https://github.com/Project-Colony/Eidos/releases/tag/v1.17.1)
is public. MotherSphere authored the release, uploaded both final assets, and
owns the commits/tag. PRs #65 and #66 are merged. Ubuntu release run
34702845103 and CI run 34702423465 passed. The archive checksum is
`b0e9f1b43e26282fcc5da0a783394d0bfc7b22880c2feb758fb0acfa85ce8aec`.
Both packaged executables are x86-64 ELF and require at most glibc 2.39.
The CLI does not implement --version; that probe returned usage, not a version.
The unpublished v1.17.0 candidate remains immutable and was superseded.

## Execution

A–H are authorized. Integration is in progress; the complete follow-up branch
has not yet passed its final review or been released.
Root owns Git, dependency locks, add-on protocols, and shared GUI types.
Independent archive, installer and collection work uses distinct owned files.
Installer C owns staging/publication; collection G consumes its pre-publication
transform API. Reviews and test evidence will be recorded per completed slice.

### Verified slices

- A backend: bounded archive member reading/export implemented; independent
  review corrected LZ4 end-marker validation, combined BSA path limits and
  substituted export stages. 40 tests and all-target Clippy passed. GUI provider
  actions, search/paging, checked export and asynchronous previews are wired;
  final independent GUI review and visual acceptance remain pending.
- B: four timestamp engines, Enderal mapping, isolated INI/order projections and
  durable capture implemented. Worker evidence: 450 unit/integration cases and
  three mounted root sessions, both FUSE modes, clean production Clippy.
  Independent integration review remains pending; Morrowind save routing is a
  separate identified gap and is not claimed complete.
- C: retained reinstall backups, remembered collision checkbox, CLI --backup and
  shared manual backup are integrated. Transformation callbacks reject Merge
  before mutation; ordinary Merge remains supported. Final caller review pending.
- D: bounded DDS decoder and mip/layer/face/alpha controls are integrated. Pinned
  licensed native NIF geometry helper implemented and tested independently; Rust
  renderer, provider texture resolution and GUI/packaging integration remain pending.
- E1: ZIP/LZMA OMOD containers and unscripted installation use ordinary staging,
  including collection recipes. 248 installer/collection tests and production
  Clippy passed; upstream reference checked all 157 members. Script interpreter,
  named fixed handlers and their supported effects remain pending.
- F: Heroic GOG/Epic/Legendary discovery, installation identity and actual prefixes
  are integrated. Folder mod units and named game examples pass installer tests.
  Trusted Preview/SaveInfo protocols, callers and three executable examples are
  implemented; custom installer caller integration and remaining routing review
  are still pending. Add-on tests: 13 passed. GUI suite: 342 passed.
- G: recipe transforms, receipts, runtime decisions and resumable HTTP identity
  implemented; 182 backend tests passed before the additional OMOD integration.
  GUI forwards receipt recovery and requires explicit runtime continuation.
- H: whole-branch review, full acceptance and publication remain pending.

### Performance investigation checkpoint — 2026-09-13

The user requested preserving all unfinished follow-up work on its branch and a
compiled reference before the recent release. Feature implementation is paused
at safe ownership boundaries for this investigation. This checkpoint is work in
progress, not a release or a claim of complete A–H acceptance.

The comparison is v1.16.0 (17e2883) versus published v1.17.1 (9f01d7d).
An isolated clean worktree at Eidos-performance-v1.16.0 contains the baseline.
Read-only process inspection found that the currently open GUI (PID 86497) is
actually version 1.15.0, from the original checkout's September 7 binary; its
application startup string and live executable digest match that disk binary.
Consequently the currently running GUI does not contain the September 12 audit
release or this unreleased follow-up work. No gameplay benchmark is claimed.

Ruling: reject transforming Merge before staging or backups — callbacks must
never receive live installed data; ordinary non-transforming Merge stays usable.
Ruling: folder-game layout validation precedes publication and live Merge — a
bare game marker must not change existing metadata before being rejected.

## Interface and acceptance review

| Packages | Shared boundary / finding |
| --- | --- |
| A / D | One bounded member reader feeds DDS/NIF preview. Root integrates GUI. |
| A / G | Archive payloads differ from collection ZIP extraction; no shared mutation. |
| C / G | Recipes must transform staging before C publishes. Never patch a live mod. |
| C / E | One installer owner handles backups and OMOD publication sequentially. |
| E / F | Root owns trusted add-on JSON types and actual GUI/CLI callers. |
| B / F | Game identity/schema changes are root coordinated before launch work. |
| A / B / D / F | Root owns shared GUI messages/state; leaf APIs integrate sequentially. |
| A | Payload bounds, source identity and DDS reconstruction are required together. |
| B | All four timestamp engines share the correction; Morrowind root INI first. |
| C | Retention failure prevents mutation; every installer route consumes it. |
| D | NIF is an actual model view; unsupported blocks remain explicit. |
| E | OBMM and exact fixed handlers are finite support, not arbitrary C# parity. |
| F | Selected source identity persists; no fabricated Steam prefix. |
| G | Known runtime mismatch needs product continuation before member mutation. |
| H | New tests and independent review precede any completion claim. |

Ruling: implement independent leaf areas concurrently with exclusive ownership,
then integrate shared types centrally. This follows the task's parallel-work
instructions and avoids shared-file edit conflicts.

## Preserved checkpoint and naming correction

All implementation work through this checkpoint is committed as e60849a and
pushed with MotherSphere. Production workspace Clippy passed with -D warnings.
The branch is eidos-mo2-followups. The prior assistant-name prefix was removed
locally and on GitHub at the user's request; the same correction applies to
eidos-audit-completion and the local eidos-performance-baseline-v1.16.0 branch.
The user's global instructions now prohibit assistant/provider names in branches.

The clean v1.16.0 reference was compiled with Rust 1.94.1 in release mode, with
locked offline dependencies. 72 core/FUSE/launch unit tests passed, and 23 real
FUSE integration tests passed in each of the two directory modes, without skips.
Its paired CLI/GUI binaries are selected through the user's existing local-bin
links for the next Steam launch. The previously selected 1.15.0 binaries are
preserved separately with their hashes and a tested restoration helper.
No real game/profile/mod files or original checkout files were changed.
The already-open GUI retains its old executable until the user closes it.

## Continuing performance review

The user clarified that the reported frame-time spikes already occurred with
v1.15.0 and asked to continue reviewing the code and Root Overwrite behavior.
No claim is made that the release or hardware caused those observed spikes.

An exact-tag synthetic experiment reproduced the v1.17.1 global directory-cache
generation retry issue: with dense unrelated create/rename churn, eight cold
listings had median 13.11 ms/8 builds on v1.16.0 versus 541.62 ms/353 builds
on v1.17.1. Quiescent and warm-cache controls were comparable. This was direct
daemon-method execution, not a mounted FUSE or game/FPS benchmark.
The narrowly scoped cache fix, independent Root Overwrite review and GUI worker
review are now active. The compiled reference remains unchanged for user testing.

### GUI performance and snapshot corrections

- Preview/export blocking operations now run through the existing smol blocking
  pool with one asynchronous global permit retained until the actual work ends,
  including when the awaiting future is canceled. No new dependency version was
  introduced; the GUI now declares the smol dependency already used by iced.
- Archive analysis defers during tracked game/tool runs and cooperatively cancels
  old epochs between phases. Completion polling is 100 ms rather than a 16 ms
  animation timer. Automatic diagnostics preserve dirty flags until the run ends.
  Cancellation is not claimed immediate inside a filesystem index or parser call.
- Archive source identities are validated at worker completion and GUI publication.
  A failed/changed scan clears old archive providers and waits for explicit refresh.
- Cached DDS controls retain the original instance/profile/view epoch and physical
  source identity, including archive members; replacement files or changed views
  cannot relabel old pixels as current.
- Independent review resolved both remaining stale-state findings. Regression
  evidence: 345 passing / 2 failing before the final guards; 347 passing / 0 failing
  afterward. Tests also cover canceled worker resume and blocked-executor behavior.

### Filesystem and Root Overwrite review completed

- Directory scans now use per-directory slot identities. An unrelated mutation
  cannot restart a cold scan; same-directory/global invalidation still rejects
  stale publication, including a removed and recreated cache slot. In the
  fix-only synthetic comparison, dense-churn median fell from 478.218 ms/305
  builds to 12.552 ms/8 builds. Quiet and warm controls were comparable. This
  directly exercises daemon methods, not gameplay or mounted streaming.
- The root mount no longer recursively indexes the Data subtree covered by its
  child mount. The directory remains visible through the existing live fallback.
  Synthetic root indexing fell from 20,151 entries/38.605 ms to 51/0.098 ms;
  the separate Data index retained all 20,100 entries. Data reads do not traverse
  two FUSE daemons: the physical Data stash is bound before the root mount.
- Private root sessions preserve shared whiteouts and opacity. Independent review
  caught directory recreation and post-session capture defects; both are fixed.
  Capture applies deletion to shared payloads, records completed opaque cleanup
  before moving new children, preserves retries, and refuses ambiguous case
  collisions. Shared opacity never carries the private capture phase marker.
- Timestamp receipt errors now preserve completed namespace/cache/inode updates
  and retry dirty order. Normal next-launch capture validates and promotes the
  newest recovery receipt. Corrupt receipts, missing plugin discovery and failed
  profile writes leave recovery available. Profile files sync before consuming
  their receipt. Loss of both writable receipt paths or a kill before persistence
  remains explicitly non-durable.
- Evidence after final review: 59 core, 216 instance, 3 launch and 32 CLI tests;
  30 actual launch sessions across normal/opendir/index-disabled modes; 31 FUSE
  unit tests and 27 mounted FUSE tests in each directory mode; 53 plugin unit and
  6 plugin integration tests. Production/all-target checks passed in their
  respective scopes. The GUI's earlier 347 passing tests predate NIF integration.

The existing empty/missing Root upper with no Root mods still skips the root
mount; root-level writes in that mode are not captured. This limit was verified
only with synthetic installations. The user's selected v1.16.0 reference binaries
remain unchanged. No Skyrim/Proton frame-time test or hardware diagnosis is claimed.

### Continued follow-ups after the performance review

- Store discovery rejects invalid IDs, missing/escaping/root install paths,
  malformed DLC fields and root Wine prefixes. External identities distinguish
  different prefixes over the same installed files; legacy keys match only when
  unambiguous. 17 store tests and all-target Clippy passed. Reinitializing a legacy
  instance still refuses a literal key mismatch; old noncanonical Steam keys
  require reselection instead of silently following a retargeted symlink.
- NIF rendering, provider texture resolution and packaging integration are now
  active. The existing bounded helper process runner is reused.
- The OBMM interpreter and the exact known-handler inventory are active in
  separate ownership areas. Full E2/E3 effects and caller acceptance remain pending.

These slices do not claim complete A–H acceptance or a new release.
