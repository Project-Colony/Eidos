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
