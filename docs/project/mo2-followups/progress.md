# Eidos follow-up progress

Plan: [implementation checklist](plan.md). Base: `9f01d7dc94cf257ccbc579fee8ccf0f39de73f19` (v1.17.1).
Branch: `eidos-mo2-followups`. Release: [**1.18.0**](https://github.com/Project-Colony/Eidos/releases/tag/v1.18.0).

## Current boundary

A–G are implemented. H code review and local acceptance are complete. The release
workflow separately builds and tests the official Ubuntu package before publication.
The isolated checkout is under `~/improvedcamera-build/`; no original checkout,
real mod/game/profile files, or selected installed binaries were changed.
Git and GitHub mutations use MotherSphere, without assistant attribution.
The completed v1.17.1 release and the unpublished v1.17.0 candidate remain immutable.

## Delivered behavior

| Area | Implementation and finite contract |
| --- | --- |
| A — Archives | Bounded TES3/BSA103/104/105 and BA2 GNRL/DX10 payload reading, checked decompression/DDS reconstruction, physical-provider preview/export, search/paging and stale-result rejection. |
| B — Engine ordering | Profile-specific timestamp projection/capture for Morrowind, Oblivion, Fallout 3 and New Vegas; Enderal SE LOOT mapping; private Morrowind.ini and Saves routing, including MWSE cosaves. |
| C — Reinstall backups | Remembered GUI backup choice, CLI backup flag, inert retained Replace/Merge backups, and a shared checked manual backup helper. Transforming installers refuse live Merge before mutation. |
| D — Previews | Bounded BC1–BC7/RGBA/BGRA DDS selection and static LE/SSE NIF geometry, transforms, current-provider diffuse textures, orbit/tilt/zoom/wireframe. Native parser and license notices are packaged. |
| E — Installers | ZIP/LZMA OMOD, bounded OBMM control flow/prompts/effects, reviewed native DarNified UI 1.3.2 / DarkUI'd DarN 1.6 / Horse Armor Revamped 1.8 handlers, owned shader projections and recoverable approved profile effects. Trusted protocol-1 custom installers share checked staging and identity-bound choices across GUI, CLI and collections. |
| F — Stores/extensions | Native/Flatpak Heroic GOG/Epic and Legendary discovery, explicit installation identities and real Wine prefixes; named-folder Stardew/SMAPI and 7 Days to Die layouts; Preview/SaveInfo callers and runnable examples. |
| G — Collections | Archive digest/size checks, installed bundles, hash-based file selection/renames, bounded BSDIFF with original CRC, provider-specific exclusions, exact runtime decisions, validated HTTP resumes and custom/OMOD interventions. |

## Review corrections

Independent reviews reproduced and corrected the following concrete failures:

- Unrelated directory mutations repeatedly rebuilt cold listings. Per-directory
  cache identities now reject only genuinely stale scans, including slot removal/recreation.
- Root setup indexed the child Data tree needlessly; private Root capture mishandled
  shared whiteouts and opaque directories. Recovery is retryable and rejects ambiguity.
- Failed timestamp receipt writes could lose completed namespace bookkeeping.
  Pending receipts and durable profile writes preserve the next-launch recovery path.
- Disabled timestamp projection still locked/cloned per-file metadata. The ordinary
  path now skips that work; opt-in passthrough follows the projection capability rule.
- Late preview/picker/installer results could follow another instance or source.
  Requests retain their original target, physical input and generation.
- Scripted receipts could hash a different archive from the one decoded. Decoders now
  read a held original file descriptor and verify its pre-decode identity through publication.
- A plain collection could carry a forged pending OMOD receipt and request profile
  writes during recovery. Native payloads now reject reserved host metadata before
  publication; legitimate receipts are created by the host after payload validation.
- Custom callbacks could change inputs after the last check; publication now checks
  again after callbacks. Store-specific control directories bind the actual context.
- Shader limits disagreed between installation and launch; both enforce 8 MiB inputs.
  Generated handler files and every approval-critical effect/warning remain visible.
- Ordinary GUI publication needed a held instance lock, including registration.
  Original-target checks also cover dialogs that stay open across a profile switch,
  changed on-disk manifests, and legacy keys that become ambiguous among known copies.
- Auto-provisioned DLLs used Steam's path for external installs; they now use the
  selected prefix. Cloud save copies are restricted to the Steam prefix outside the game tree.

## Acceptance evidence

Final local verification used Rust 1.94.1 with the locked, offline dependency graph:

- Installer: 230 unit tests plus 4 corpus checks; one opt-in upstream OMOD archive check
  was also run explicitly and passed for all 157 supplied members. Collections: 56 unit tests plus 29 integration tests.
- FUSE: 32 unit tests and 27 real mounted tests in each directory mode, no skips;
  projected timestamps were also exercised with opt-in passthrough enabled.
- Source routing: 25 isolated launch attempts include refusal cases, read-only SDP
  composition, profile switches, Morrowind saves and the separate user Mods mount.
- GUI: all **383 serial release tests** passed, including the final ordinary-dialog
  and queued-worker profile/manifest/legacy-key guards. Three tests build, send real pointer/keyboard events to, and render actual iced
  widget trees offscreen, including a completed synthetic installation, DDS/NIF
  controls and an archive-member preview. Nine PNGs were visually inspected.
- Native NIF parser: 66 release and sanitizer fixture cases; geometry/pixel/transform,
  warning and input bounds are covered separately by Rust tests.
- Translation check: 135 pages, zero stale stamps. All 168 then-present Markdown
  files had resolving internal links. Only affected translated sections changed.
- Rootless installer smoke check preserves all three executables and upstream notices
  without invoking sudo or setcap. `just` is unavailable on this host; equivalent
  commands were run directly, its recipes inspected, and no system package installed.

The full workspace release build and non-GUI workspace tests passed. The CLI was
rerun after its final identity correction: **41 tests passed**. Both mounted directory
modes passed **27/27 without skips**, alongside the isolated root/SDP/save launch
harness. The opt-in OMOD reference test also passed explicitly; no unavailable
reference archive is silently counted as tested. Production workspace Clippy passed
with warnings denied. The six existing GUI test-only style lints are outside the
production lint gate; no test failure is waived. Global rustfmt remains advisory,
matching CI rather than rewriting unrelated historical files.

Reproduce the main checks from the checkout (with isolated XDG state and the built
native helper on PATH):

```sh
cargo +1.94.1 build --workspace --release --locked --offline
cargo +1.94.1 test --workspace --exclude eidos-gui --release --locked --offline
cargo +1.94.1 test -p eidos-gui --release --locked --offline -- --test-threads=1
cargo +1.94.1 clippy --workspace --release --locked --offline -- -D warnings
EIDOS_FUSE_OPENDIR=1 EIDOS_FUSE_PASSTHROUGH=1 cargo +1.94.1 test -p eidos-fuse --test union -p eidos-launch --test root_overwrite --release --locked --offline
bash packaging/tests/install.sh
./scripts/i18n-check.sh
./scripts/i18n-links.py
```

The release page identifies the published source tag and official archive checksum.
Local Arch builds are not substituted for the Ubuntu glibc-2.39 compatibility package.

## Performance evidence and limits

Exact-tag synthetic directory churn: v1.16.0 median **13.11 ms / 8 builds**;
v1.17.1 **541.62 ms / 353 builds**. The isolated correction measured
**478.218 ms / 305 builds → 12.552 ms / 8 builds**. Quiet/warm controls were comparable.
Root indexing measured **20,151 entries / 38.605 ms → 51 / 0.098 ms**;
the separate Data index retained 20,100 entries.

A warmed exact-method attribute experiment measured **52.112 → 10.835 ns/call**
with projection disabled; the published baseline measured 10.071 ns/call. Nine
alternating samples of one million calls asserted identical output attributes.
These are method-level synthetic measurements, not mounted streaming or game FPS.

Optimized GUI medians for 1k/10k synthetic rows were 0.889/9.656 ms for
construction and 0.927/12.207 ms for layout (1280×800, native tiny-skia renderer,
ten warmed samples, upper-middle statistic). Debug measurements were much slower
and are not representative of the package. The existing list remains O(n),
without virtualization; 10k entries can exceed one 60 Hz frame. These measurements
cover widget construction/layout, not desktop compositor or GPU presentation.

NIF rendering is static, orthographic and approximate; unsupported blocks and missing
textures remain visible. OMOD support is a finite native contract, not arbitrary
C#/VB execution or full MO2 scripting parity. Trusted extensions have the user's
filesystem access; replies are checked but there is no operating-system sandbox.

External copies require an explicit compatible runner; external Tier-2 prerequisite
installation remains manual. 7 Days to Die resolves its user-data location at launch;
discovery cannot infer a future custom command. Nonstandard wrappers need an explicit
UserDataFolder. No Flatpak/AppImage package integration is claimed.

Profile-effect writes and Root capture are retryable, not one atomic transaction
across all files. Failure of every receipt path or a kill before persistence remains
non-durable. A killed shader session can leave unused owned staging files. An empty
Root upper with no Root mods still skips Root capture. At narrow pane widths the
existing eight-tab row can clip; the split divider exposes the remaining tabs.

No Skyrim/Proton playthrough, GPU shader compatibility, hardware diagnosis or FPS
improvement is claimed. All runtime tests use isolated synthetic installations.

## Historical performance investigation

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
