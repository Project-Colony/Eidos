# Contributing

Building, testing, and where everything lives. For what Eidos *is*, start at the
[README](../../README.md); for how the daemon works,
[architecture.md](architecture.md).

## Build and test

```sh
cargo test                 # workspace unit tests + the eidos-fuse real-mount suite
cargo build -p eidos-fuse  # the union daemon on its own
```

The `eidos-fuse` integration suite is not a mock: it mounts a real union inside
its own private user+mount namespace (no root, no host mounts touched) and drives
20 checks through the kernel, including a writable `MAP_SHARED` mmap round-trip,
the negative-dentry cases, and a root union carrying a Data union inside it. If
the user namespace is unavailable it says so and skips only the checks a racing
host service could disturb. It runs `harness = false` (the namespace must be
entered before any thread exists), so cargo's own summary line reports zero for
it - read the suite's own `union integration result:` line instead.


## Repo layout

```
crates/eidos            the unified CLI front end (games / init / play / install / tool / ...)
crates/eidos-gui        the iced GUI (Colony parchment look)
crates/eidos-core       the layer-resolution engine (pure, unit-tested)
crates/eidos-fuse       the read-write FUSE union daemon
crates/eidos-games      supported-game catalog + Steam install detection
crates/eidos-launch     per-launch namespace wrapper: run a game through the view
crates/eidos-instance   instance model: global/portable, profiles, per-mod meta.ini, manifest, load order
crates/eidos-plugins    ESP/ESM/ESL plugin load order (via esplugin) + plugins.txt
crates/eidos-loot       LOOT graph sorting (libloot) + masterlist fetch/cache
crates/eidos-conflicts  per-file conflict analysis (winners / losers, per-mod state)
crates/eidos-install    mod installer: 7-Zip extract + Simple wrapper-strip + Root split + meta.ini
crates/eidos-fomod      FOMOD scripted-installer parser + condition/flag engine
crates/eidos-gamefeatures  BSA/archive invalidation + per-profile INIs/saves at launch
crates/eidos-gamedef    declarative per-game descriptor (one row per game; MO2 schema)
crates/eidos-ini        shared low-level INI primitives (newline / section / key / edit)
crates/eidos-nexus      Nexus Mods: v1 API client, nxm:// downloads, update checks,
                        v2 GraphQL for collections
crates/eidos-log        session logs: levels, rotation, home-path redaction
crates/eidos-addons     user extensions, read as out-of-process TOML manifests
crates/eidos-paths      where Eidos keeps its files: the Colony layout, and the
                        migration onto it. Depended on by every leaf crate, so
                        std-only on purpose
docs/internals/architecture.md  the design and the tradeoffs behind it
scripts/poc-overlay.sh  runnable proof that the "virtualize under Wine" thesis
                        holds with native primitives, no root required
```

## Releasing

Two workflows, and they do different jobs.

**release-please** watches `main` and keeps a pull request open with the next
version and a changelog, both derived from the conventional-commit messages since
the last release. It decides *what* the release is; it builds nothing.

**release.yml** triggers on a `vX.Y.Z` tag and does the building, testing and
publishing.

So a release is: land conventional commits on `main`, then merge the release PR
when you want one. Merging creates the tag, the tag builds the artifacts.

The prefixes that move the version: `feat:` bumps the minor, `fix:` and `perf:`
the patch, and a `!` or a `BREAKING CHANGE:` footer bumps the major. `docs:`,
`chore:`, `test:` and `ci:` ride along without moving anything.

The workspace is a virtual manifest - one `[workspace.package]` version that all
seventeen crates inherit - which is why the config uses the `cargo-workspace`
plugin. `.release-please-manifest.json` records where the version currently
stands; nothing else should edit it.
