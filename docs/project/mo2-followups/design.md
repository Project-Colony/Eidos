# Eidos follow-up design — 2026-09-12

The user explicitly authorized every named remaining improvement in the final
audit report, after publishing the current release. The completed first audit
and its test receipts remain historical evidence. New code belongs to a new
branch and does not change published tags.

## Decisions

Extend the current Rust crate boundaries and iced message flows. Use shared,
validated staging and instance locks for all publication. Directory-only archive
indexing remains cheap; payload extraction, codecs and previews run on demand.
Reuse the locked codecs/7-Zip and existing add-on system; add a maintained codec
only where the current dependency graph cannot correctly decode the format.

Archive export supports the families currently indexed by Eidos, including BSA
compression toggles/embedded names and BA2 texture reconstruction. DDS decoding
must cover common BC1–BC7 and uncompressed Bethesda textures with explicit
subresource/channel controls and allocation limits. NIF must be a genuine model
view, using a pinned established parser/helper; no header inspector is labeled a
3D preview. Start with static LE/SSE geometry, transforms, texture references and
orbit/zoom/wireframe; animation and engine shader parity remain stated limits.

OMOD decoding uses existing 7-Zip for concatenated ZIP/LZMA payloads, with exact
CRC/path/size validation. The native script contract is OBMM plus verified fixed
handlers from the later OMODFramework implementation. Unknown executable script
bodies stop with an actionable error; no arbitrary archive C#/VB compilation is
introduced. This is useful finite OMOD support, not full scripting parity with
MO2's older .NET Framework host. Preserve upstream notices for translated code.
Custom installer, preview and save-info extensions are trusted user-installed
programs with validated bounded results, not archive-auto-registered executables
or an implied operating-system sandbox.

Collections must follow actual Vortex semantics: bundles are installed folder
trees; source hashes/sizes validate sources; installed hashes can prescribe a
hash-based file-selection/rename recipe; patches validate original CRC before
BSDIFF; fileOverrides exclude that provider's files. Never modify a personal
provider to satisfy a collection. Expose unknown or ambiguous paths/runtime
versions instead of guessing. Resume verifies recipe effects, not only ownership.

Timestamp ordering covers all engines sharing that mechanism: Morrowind,
Oblivion, Fallout 3 and Fallout New Vegas. Project desired times through the
isolated union without touching original game/mod timestamps; capture tool
changes and retain profile identity. Enderal SE uses the verified Enderal LOOT
masterlist and its explicit primary plugins. Morrowind.ini must enter the root
projection before enabling its previously unreachable preparation path.

Broader game/store support is bounded to read-only Heroic GOG/Epic/Legendary
manifests and named-folder schemas with Stardew/SMAPI and 7 Days to Die examples.
Persist the chosen installation identity; never select a different Steam copy or
fabricate a Steam compatdata path for a Wine prefix. Optional integrations must
have an observable supported caller and working example.

## Acceptance and evidence

The detailed source-backed contracts live in the preceding implementation
folder's next-plan-inventory.md, archive-preview-design.md,
installer-followup-design.md and collection-order-design.md. Those are design
inputs, not completion claims. Each work package has behavioral tests and
independent review; the final branch gets full locked release/GUI/FUSE/Clippy,
translation and packaging checks. Visual previews require a synthetic isolated
visual check. Real game files, accounts and installed Eidos stay untouched.
