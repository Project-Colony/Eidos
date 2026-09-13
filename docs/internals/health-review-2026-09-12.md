# Eidos health review — 2026-09-12

Base: `17e28836cfb0ee71aa43fa41dd80bec90b732dec` (1.16.0).
Work branch: `eidos-audit-completion`. This is the historical audit of the 1.16.0
base; its changes shipped in 1.17.1. The subsequent implementation and validation
are tracked in [the follow-up progress ledger](../project/mo2-followups/progress.md).

## Implemented scope

The approved audit adds archive-member conflict analysis, plugin/SKSE preflight,
durable collection recovery and generated-output receipts. It also corrects
installation, filesystem, save and launcher defects. See
[integrity.md](integrity.md) for the contracts and limitations of these features.

A subsequent review covered the workspace's 22 crates. Production mutation and
persistence paths were traced across callers; independent reviewers examined
installer/network, engine, and filesystem/instance code. Rendering modules were
reviewed for state and IO boundaries and exercised by their tests. This is not
formal verification or a visual accessibility certification.

## Additional defects corrected by the whole-code review

- New/Replace installation prepares content before publication; failed Merge
  explicitly revokes old ownership/source claims and retains a warning.
- Installation and Overwrite moves reject unsafe destination links, preserve
  existing data on move failure and respect case-insensitive destinations.
- Backup packing cannot include excluded children through empty-directory
  recursion or inject extra 7-Zip entries through newline-bearing filenames.
  Forced restore rejects destination links before extraction.
- Collection auxiliary INIs and downloads preserve personal name collisions;
  plugin-state application respects whiteouts; missing recorded FOMOD groups
  make replay approximate.
- HTTP resume validates response ranges and body sizes. Nexus update results
  reread metadata under the instance lock instead of overwriting concurrent edits;
  GUI and CLI both use this shared update path and surface individual request
  failures instead of presenting an incomplete check as clean.
- Modlist trust no longer mistakes newly discovered folders for retained saved
  entries. Unreadable orders, profile copies, manifest writes and INI safety-copy
  failures cannot silently report success.
- MO2 profile import handles BOM/Unicode and acquires the shared instance lock.
- Game-definition initialization avoids recursive registry locking. Unicode
  script-extender logs cannot panic fixed-byte prefix parsing.
- Native dependency scanning handles delay-only PE imports. Shared runtime
  installation is serialized across instances. Legacy INI updates preserve bytes.
- LOOT case bridging examines the actual higher-priority file in GUI and CLI.
  Archive analysis honors pins, Enderal SE rules and uncertain/corrupt providers.
- GUI results retain their original instance/request identity. Metadata edits
  use checked reads and locks; file changes, INI choices and backup restores
  invalidate the corresponding caches. Hidden-file restoration treats links as
  opaque and refuses case-insensitive collisions.
- Packaging uses one tracked installer, defaults to rootless operation and
  quotes desktop executable paths. The optional capability and runtime
  passthrough switch are documented separately.

## Verification

All tests used synthetic temporary instances and isolated configuration/state
paths. Read-only archive inspection covered 498 installed Bethesda archives and
965,114 member entries. No real mod directory or game installation was changed.

Final acceptance uses the CI-pinned Rust 1.94.1 with the committed lockfile:

```sh
cargo +1.94.1 build --workspace --release --locked --offline
cargo +1.94.1 test --workspace --exclude eidos-gui --release --locked --offline
cargo +1.94.1 test -p eidos-gui --release --locked --offline -- --test-threads=1
cargo +1.94.1 clippy --workspace --release --locked --offline -- -D warnings
EIDOS_FUSE_OPENDIR=1 cargo +1.94.1 test -p eidos-fuse --test union --release --locked --offline
bash packaging/tests/install.sh
./scripts/i18n-check.sh
./scripts/i18n-links.py
git diff --check
```

The test inventory is 1,336 distinct Cargo tests, 24 actual FUSE checks and one
isolated root/Overwrite launch check: **1,361 successful checks**. The logs contain
one additional execution of the registry test in its deliberate child process. The 24 FUSE checks also
pass with legacy directory handles enabled, with no skips in either mode.
The GUI contributes 328 tests and runs serially. Translation stamps and links
are checked separately. Production Clippy treats warnings as errors; this does
not claim all test-only Clippy style lints are enabled. Existing compiler warnings
in GUI tests and the resolver benchmark were also cleaned up.

No Skyrim/Proton playthrough, real Nexus/OAuth session, live runtime download or
system package installation was performed. Dependency CVE scanning was not run
because cargo-audit is not installed. The build and tests cannot certify every
third-party DLL, hook, archive payload or savegame.

## MO2 gaps worth considering

This table records the gaps at the time of this audit. Consult the follow-up
ledger linked above for their implementation status.
Current primary repository/release sources were checked; MO2's release page
exposes v2.5.2, while development repositories can include later work.
[MO2 releases](https://github.com/ModOrganizer2/modorganizer/releases).

| Area | Eidos status / useful next step | Primary reference |
|---|---|---|
| Bethesda archive contents | Conflict directory analysis works; member extraction and preview are separate future features. | [BSA extractor](https://github.com/ModOrganizer2/modorganizer-bsa_extractor) |
| File preview | Common image/text previews exist; DDS preview is missing. NIF rendering is a separate proposed Eidos enhancement; native MO2 parity was not verified. | [DDS preview plugin](https://github.com/ModOrganizer2/modorganizer-preview_dds) |
| Installer extensions | Built-in Simple/FOMOD/BAIN/manual flows exist; OMOD execution and custom installer hooks are missing. | [OMOD installer](https://github.com/ModOrganizer2/modorganizer-installer_omod) |
| Retained reinstall backup | Manual backups and temporary rollback exist; the collision dialog has no retained-backup option. | [Installation manager](https://github.com/ModOrganizer2/modorganizer/blob/master/src/installationmanager.cpp) |
| Older games / sorting | Morrowind/Oblivion timestamp plugin ordering is not implemented; Enderal SE LOOT mapping remains unsupported. | [Gamebryo support](https://github.com/ModOrganizer2/modorganizer-game_gamebryo) |
| Games, stores and extensions | Eidos has 12 built-in definitions plus TOML game/Tool/Diagnose extensions. More store discovery and richer preview/installer/save hooks would broaden support. | [Basic Games](https://github.com/ModOrganizer2/modorganizer-basic_games) |

Collection bundle payloads, binary patches/file overrides and manifest hash/game
version verification remain explicit coverage gaps. They are not claimed as MO2
native Nexus-collection features. Generated receipts record observed inputs and
outputs; they do not infer a complete dependency graph.

Merge, forced unpack and profile-copy failures can leave partial destination
content, with errors reported and original sources preserved where applicable.
There is no power-loss transaction spanning multiple files and receipt journals.
Additional crash/ENOSPC and mounted-concurrency fault injection would strengthen
coverage. No claim of complete MO2 parity or universally fault-free operation is
made.
