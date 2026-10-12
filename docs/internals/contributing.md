# Contributing

Building, testing, and where everything lives. For what Eidos *is*, start at the
[README](../../README.md); for how the daemon works,
[architecture.md](architecture.md).

## Build and test

```sh
just test                  # native helper, workspace tests, then serial GUI tests
cargo build -p eidos-fuse  # the union daemon on its own
```

The complete application also needs CMake 3.20+, a C++17 compiler and Python 3
for the native helper's tests. `just build` builds the helper beside the Rust
binaries. Without `just`, follow the [native build instructions](../../native/eidos-nif-preview/README.md)
and add the helper's absolute output directory to `PATH` before GUI tests.
Those tests require the real helper and do not silently skip model parsing.

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
crates/eidos               the CLI front end (games / init / play / install / tool / ...)
crates/eidos-gui           the iced GUI (Colony parchment look)
crates/eidos-core          the layer-resolution engine (pure, unit-tested)
crates/eidos-fuse          the read-write FUSE union daemon (+ a dev CLI)
crates/eidos-launch        per-launch namespace wrapper: run a game through the view (+ a dev CLI)
crates/eidos-paths         where Eidos keeps its files: the Colony layout from
                           colony_ui::paths, and the migration onto it (see paths.md)
crates/eidos-log           session logs: levels, rotation, home-path redaction
crates/eidos-ini           shared low-level INI primitives (newline / section / key / edit)
crates/eidos-gamedef       declarative per-game descriptor (one row per game; MO2 schema)
crates/eidos-games         supported-game catalog + Steam / Heroic install detection
crates/eidos-plugins       ESP/ESM/ESL plugin load order (via esplugin) + plugins.txt
crates/eidos-loot          LOOT graph sorting (libloot) + masterlist fetch/cache
crates/eidos-instance      instance model: global/portable, profiles, per-mod meta.ini,
                           manifest, global settings and nexus.ini
crates/eidos-conflicts     per-file and archive-member conflict analysis
crates/eidos-sevenzip      the 7-Zip process seam: find it, drive it, read its progress
crates/eidos-install       mod installers: Simple / Root / BAIN / OMOD / manual + staged publication
crates/eidos-fomod         FOMOD scripted-installer parser + condition/flag engine
crates/eidos-gamefeatures  archive invalidation, per-profile INIs/saves, prefix DLLs and prerequisites
crates/eidos-nexus         Nexus Mods: OAuth, v1 API, nxm:// downloads, update checks,
                           v2 GraphQL for collections
crates/eidos-collections   Nexus collections: install one the way its author built it
crates/eidos-transfer      eidos pack / unpack: one instance in one .eidos file
crates/eidos-addons        user extensions, read as out-of-process TOML manifests
native/eidos-nif-preview   the C++ NIF preview helper (CMake, vendored nifly)
packaging/                 the tarball installer, its tests and the Arch PKGBUILD
docs/                      user guide, internals and project pages (index: docs/README.md)
scripts/                   CI helpers, and poc-overlay.sh: runnable proof that the
                           "virtualize under Wine" thesis holds with native
                           primitives, no root required
```

That is all 22 workspace crates. What each one owns, and which process it runs
in, is the [crate map in architecture.md](architecture.md#crate-map). A pull
request that adds, removes or splits a crate updates both lists in the same pull
request.

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

The workspace is a virtual manifest: one `[workspace.package]` version that all
22 crates inherit. So release-please runs with `release-type: simple` and bumps
that one line through `extra-files` (`Cargo.toml`, the line marked
`x-release-please-version`); no Cargo plugin is involved. The workflow then runs
`cargo update --workspace` on the release branch, so the release pull request
carries the matching `Cargo.lock` bump that `--locked` builds need.
`.release-please-manifest.json` records where the version currently stands, and
`CHANGELOG.md` is written by release-please from the commit messages. Nobody
edits either by hand: a changelog entry is fixed by fixing the commit message
or the pull request title it came from.
