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
binaries. Without `just`, follow the [native build instructions](../../crates/eidos-nif-preview/README.md)
and add the helper's absolute output directory to `PATH` before GUI tests.
Those tests require the real helper and do not silently skip model parsing.

CI also fails a pull request whose code is not formatted with `cargo fmt --all`
(rustfmt defaults, no `rustfmt.toml`), that has a Clippy warning
(`cargo clippy --workspace --locked -- -D warnings`, library and binary targets
only), or that no longer builds on the `rust-version` declared in the root
`Cargo.toml`. Dependencies are declared once, at their full version, in
`[workspace.dependencies]`; a crate inherits one with `name.workspace = true`
and adds its own features. The `build, clippy, test` job installs `par2` so
the `eidos-transfer` recovery-data tests run instead of skipping, and runs
`crates/eidos-transfer/tests/check_clock_resolution.py` and the installer test
`packaging/tests/install.sh`.

The `translations up to date` job runs `scripts/i18n-check.sh` and
`scripts/i18n-links.py`. `README.md`, `docs/README.md` and `docs/guide/*.md`
have translations at `docs/i18n/<lang>/<source path>` for cs, de, es, fr, it,
ja, ko, nl, pl, pt-BR, ru, tr, uk, zh-CN and zh-TW. Each starts with
`<!-- eidos-i18n: source=<path> sha=<git hash-object of the source> -->`, so
editing an English source without re-translating it and restamping the
translations fails CI. `scripts/i18n-check.sh --fix` restamps after a
re-translation, and `--stamp` adds a missing header. `docs/internals`,
`docs/project` and `SECURITY.md` have no translations.

The `eidos-fuse` integration suite is not a mock: it mounts a real union inside
its own private user+mount namespace (no root, no host mounts touched) and drives
32 checks through the kernel, including a writable `MAP_SHARED` mmap round-trip,
the negative-dentry cases, and a root union carrying a Data union inside it. If
the private namespace cannot be entered, it says so and skips the checks marked
as needing one, and the result line reports how many it skipped. It runs
`harness = false` (the namespace must be entered before any thread exists), so
cargo's own summary line reports zero for it: read the suite's own
`union integration result: N passed, M failed, K skipped` line instead.

Run the suite twice. By default the daemon declines `opendir`, and
`EIDOS_FUSE_OPENDIR=1` restores the per-handle snapshots; the two paths reach
`readdir` differently, so a listing or invalidation bug can live in either
alone. CI runs the default mode only.

```sh
cargo test -p eidos-fuse --test union
EIDOS_FUSE_OPENDIR=1 cargo test -p eidos-fuse --test union
```

Kernel passthrough needs CAP_SYS_ADMIN in the initial user namespace. To test
it, run the test binary as root or with `cap_sys_admin+ep`, and with
`EIDOS_FUSE_PASSTHROUGH=1`: the suite then keeps the initial user namespace
and every mount uses passthrough.


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
crates/eidos-gamedef       declarative per-game descriptor (one TOML file per game; MO2 schema)
crates/eidos-games         supported-game catalog, Steam / Heroic / Legendary install
                           detection and the stores' launch settings
crates/eidos-plugins       ESP/ESM/ESL plugin load order (via esplugin) + plugins.txt
crates/eidos-loot          LOOT graph sorting (libloot) + masterlist fetch/cache
crates/eidos-instance      instance model: global/portable, profiles, per-mod meta.ini,
                           manifest, global settings, nexus.ini, session records,
                           test runs and the bisect search
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
crates/eidos-nif-preview   the C++ NIF preview helper (CMake, vendored nifly)
packaging/                 the bundle installer, its tests, the Arch PKGBUILD, and
                           runtimes.txt with the Microsoft runtime license notices
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

One workflow, `release.yml`, runs on every push to `main`.

Its **release-please** job keeps a pull request open with the next version and
a changelog, both derived from the conventional-commit messages since the last
release. It decides *what* the release is; it builds nothing.

Merging that pull request makes release-please tag the commit and create the
release, held as a draft. The same run then builds the tag, tests it, checks
`colony.json` and hands the two assets to the organisation's shared
sign-and-publish workflow, which signs them and publishes the release. What runs
where: [packaging.md](packaging.md#how-a-release-is-built).

So a release is: land conventional commits on `main`, then merge the release PR
when you want one.

The prefixes that move the version: `feat:` bumps the minor, and a `!` or a
`BREAKING CHANGE:` footer bumps the major. `fix:`, `perf:`, `refactor:` and
`docs:` bump the patch, and each has its own changelog section: Fixes,
Performance, Internals and Documentation (1.19.2 was a release of
documentation commits alone, 1.11.2 of `refactor:` alone). `chore:`, `test:`
and `ci:` are hidden from the changelog and move nothing on their own
(`release-please-config.json`).

The workspace is a virtual manifest: one `[workspace.package]` version that all
22 crates inherit. So release-please runs with `release-type: simple` and bumps
that one line through `extra-files` (`Cargo.toml`, the line marked
`x-release-please-version`); no Cargo plugin is involved. The workflow then runs
`cargo update --workspace` on the open release pull request's branch, so it
carries the matching `Cargo.lock` bump that `--locked` builds need.
`.release-please-manifest.json` records where the version currently stands, and
`CHANGELOG.md` is written by release-please from the commit messages. Nobody
edits either by hand: a changelog entry is fixed by fixing the commit message
or the pull request title it came from.
