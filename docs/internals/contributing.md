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
crates/eidos-nexus      Nexus Mods: v1 API client, nxm:// downloads, update checks
docs/architecture.md    the design and the tradeoffs behind it
scripts/poc-overlay.sh  runnable proof that the "virtualize under Wine" thesis
                        holds with native primitives, no root required
```
