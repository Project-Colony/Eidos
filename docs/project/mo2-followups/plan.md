# Eidos remaining improvements implementation plan

> Use subagent-driven development for independent ownership areas, then review integration and run final checks.

**Goal:** Complete the named remaining improvements authorized after the health review.
**Architecture:** Extend current installer, archive, plugin, game and add-on paths; keep the shared GUI/CLI contracts and publish only validated staging.
**Tech stack:** Rust 1.94.1, iced, existing 7-Zip/codecs, bounded native preview/interpreter helpers and existing TOML/JSON mechanisms.
**Spec:** design.md and the source-backed design reports in ../eidos-implementation/.

## Global constraints

- Start implementation only after the requested current release is published.
- New branch eidos-mo2-followups in the isolated work clone; no changes to the original repository or real game/mod directories.
- MotherSphere Git identity and GitHub account only for agent-authored mutations; no AI attribution.
- Branch names must never include an AI assistant/provider name or prefix. Use plain task names, including for temporary worktrees.
- English code/comments/UI/docs. Preserve licenses/credits for reused code.
- No sudo or system package installs on the user's machine; use isolated synthetic fixtures and test XDG state.
- Root owns Git index/dependency-lock coordination, all eidos-addons protocol/types, and shared GUI Message/Preview/App types. Do not move an existing tag.
- C exclusively owns installer publication/backup changes. Before G integrates recipes, C and G agree and test a narrow pre-publication transform API; G never patches live published mods or edits C-owned staging code concurrently.
- Never mark unsupported behavior as handled or incomplete tests as successful.

## 0. Release and ledger boundary

- [x] Publish the versioned Ubuntu artifacts after CI/release checks; verify source tag, author, asset hashes and contents.
- [x] Create the follow-up branch and tracked plan/progress, retaining the completed audit.
- [ ] Correct stale casing/packaging ledgers and distinguish official DDS support from community NIF preview.

## A. Validated archive payloads and export

Owner: archive worker. Files: eidos-conflicts/src/archives.rs and focused payload/tests modules; root owns GUI actions.

- [ ] Extend the existing parsers to retrieve payload metadata on demand for TES3/BSA103/104/105 and existing BA2 GNRL/DX10 families.
- [ ] Decode raw, zlib, LZ4 frame/block correctly; reconstruct bounded DDS subresources and check exact output sizes/checksums.
- [ ] Provide one shared bounded member reader and staged single-member export with destination/source integrity checks.
- [ ] Wire winning/losing provider selection, search/paging beyond 100 entries, preview/export and stale request protection.
- [ ] Verify exact fixtures per family, malformed input/size/path bounds, original destination preservation and independent review.

## B. Engine ordering and Enderal

Owner assigned after a leaf worker completes. Files: eidos-plugins, eidos-loot, shared generic union attribute projection, launch/prepare and GUI consumers.

- [ ] Add verified Enderal SE LOOT engine/masterlist mapping and explicit primary plugins.
- [ ] Persist activation/order for all four timestamp engines; project ordering in isolated mounts, never by altering original files.
- [ ] Route Morrowind.ini through the root projection and capture tool ordering changes without cross-profile leakage.
- [ ] Test equal timestamps, masters/disabled entries, two profiles, round-trip capture and actual mounted timestamps/source preservation.

## C. Retained reinstall backups

Owner: installer worker. Files: shared eidos-install publication, shared instance backup helper; root owns collision/CLI wiring.

- [ ] Reuse inert unique backup naming. Retain old Replace directories and copy Merge inputs before mutation; expose the backup path.
- [ ] Add collision checkbox/remembered preference and CLI flag across Simple/FOMOD/BAIN/manual; manual backup uses the same helper.
- [ ] Test failed backup preventing mutation, failed publication/recovery, case/symlink collisions, hidden metadata and inert mod state.

## D. DDS and NIF viewing

Owner: archive worker for byte decoding, root for GUI; separate NIF worker after a slot frees.

- [ ] Decode supported BC1–BC7 and RGBA/BGRA DDS formats under checked allocation limits; expose mip/layer/face and alpha controls.
- [ ] Reuse a pinned licensed NIF parser/helper and render static LE/SSE shapes/transforms/material references with orbit/zoom/wireframe.
- [ ] Resolve model textures through current loose/archive providers; report missing textures/unsupported blocks explicitly.
- [ ] Verify pixels, malformed input, geometry bounds, fixture transforms and actual synthetic visual interaction.

## E. OMOD and trusted custom installers

Owner: installer worker after C; root integrates shared prompts and extension caller.

- [ ] Decode OMOD metadata, ZIP/LZMA payloads and CRC directories; install unscripted data/plugins through ordinary staging.
- [ ] Implement bounded OBMM evaluation with prompts, control flow/context, file decisions and explicit supported/unsupported effects.
- [ ] Port verified fixed inline handlers with exact script identity, recorded choices and original credits; reject unsupported code bodies.
- [ ] Apply supported file/INI/plugin/shader effects to owned projected data, retain pending effects on partial failure and support deterministic replay.
- [ ] Add versioned trusted installer add-on matching/protocol and checked results; handle decline/prompt/cancel/timeout/malformed output.
- [ ] Test each statement/effect family and named handler choices; end-to-end GUI/CLI/collection intervention and original-file preservation.

## F. Stores, folder mods and extension callers

Owner: root, with a leaf worker later if useful. Files: eidos-games, gamedef/layout rules, instance manifest, add-ons, GUI/CLI integration.

- [ ] Read native/Flatpak Heroic GOG/Epic/Legendary manifests and validate supported IDs/paths; preserve duplicate installation identities.
- [ ] Use the actual selected prefix/runner or explicit manual launch; preserve legacy Steam behavior and never invent a Steam prefix.
- [ ] Add valid_files/named-folder mod-unit rules and complete Stardew/SMAPI and 7 Days to Die examples; retain Bethesda/Unreal corpus behavior.
- [ ] Add trusted Preview/SaveInfo add-on result contracts with working examples and real callers, cancellation/error/identity checks.
- [ ] Test discovery/schema/selection, installs preserving folder names, bad helper outputs and user-visible invocation.

## G. Complete collection recipe fields

Owner: collection worker. Files: collections manifest/driver/install/state, existing Nexus digest/download helpers; coordinate shared installer APIs.

- [ ] Validate advertised archive digest/size for cached and downloaded members; preserve personal archives/mods on mismatch.
- [ ] Handle installed bundle directory trees and hash-based installed file selection/renames before generic FOMOD routing.
- [ ] Apply bounded BSDIFF after original CRC validation; implement provider-specific excluded file overrides without touching others.
- [ ] Compare declared game versions with observed runtime; known mismatch requires an explicit GUI continuation or CLI override/recorded decision before any member mutation. Surface unknown/ambiguous evidence and verify recipe effects on resume.
- [ ] Preserve download object identity across resumes; use a valid object validator and restart when changed, missing or unusable unless full-object identity is independently verified.
- [ ] Test wrong/missing hashes, bundles/traversal, patch bounds/base CRC, exclusions, runtime mismatch, phase interruption and personal collisions.

## H. Whole-branch verification and usability

- [ ] Independently review each new module and every modified caller; fix confirmed defects with regression checks.
- [ ] Exercise publication/rollback, receipts and mounted concurrency failures without a generic fault framework.
- [ ] Measure synthetic 1k/10k views and fix demonstrated UI blocking; check keyboard/focus of changed dialogs/previews.
- [ ] Run pinned locked release build, workspace tests, serial GUI tests, actual FUSE modes, production Clippy, i18n/link and packaging checks.
- [ ] Commit coherent scopes and update exact evidence/remaining format limits in progress and final report. Do not claim gameplay validation.
