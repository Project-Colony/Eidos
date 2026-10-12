# Packaging and distribution

Eidos ships through Colony, as a release bundle, as a native Arch package, or as
a source build. Normal operation uses rootless FUSE and does not require a file
capability.

## Install

| Channel | Command | Privileges |
|---|---|---|
| Colony | install **Eidos** in the launcher | None |
| Release bundle | `./install.sh` in the extracted `eidos-linux-bundle/` | None for the default user installation |
| Arch package | `cd packaging && makepkg -si` | Package installation uses pacman |
| From source | `just build`, then `just install` | None for the default user installation |

The installer keeps `eidos`, `eidos-gui` and the bundled `eidos-nif-preview`
helper together, registers the Nexus download
handler and desktop entry, and installs bundled icons. Use `--bindir DIR` for a
custom destination or `--system` for `/usr/local/bin`. A system destination may
require sudo. The release bundle carries this same tracked installer; there is
no second generated implementation to drift out of date.

## Release assets

Every release carries two assets, named after the Colony convention so Colony
finds the Linux one without any `releaseFiles` in `colony.json`:

| Asset | Contents |
|---|---|
| `eidos-linux` | `eidos-gui` under the asset's name. It links the CLI too: called through a link named `eidos`, or with `--eidos-cli` first, it runs the CLI instead of a window, and with no `eidos` beside it the window launches games and tools through itself that way. This one file is what Colony installs. |
| `eidos-linux-bundle.tar.gz` | `eidos-linux-bundle/` with `eidos`, `eidos-gui`, `eidos-nif-preview`, `install.sh`, the icons, `README.md`, `LICENSE`, `SECURITY.md` and the third-party notices under `licenses/`. Deterministic: fixed file order, owner and mtime. |

Each one comes with `.sig`, `.meta` and `.meta.sig` (see the README's
[Code signing policy](../../README.md#code-signing-policy)). No separate
`.sha256` file is published: the `.meta` carries the sha256, and its signature
is what makes that digest worth checking.

## How a release is built

`.github/workflows/release.yml` is the whole chain, in one run per merge to
`main`:

1. **release-please** keeps the release pull request up to date and, on the
   merge of that pull request, tags the commit and creates the release. The
   release is put back to draft at once, so `/releases/latest` never points at a
   release without files. The same job brings the open release pull request's
   `Cargo.lock` in line with the new version.
2. **build** checks out the tag on `ubuntu-24.04` (the runner image sets the
   glibc floor, 2.39), builds `eidos` and `eidos-gui` in release mode and the NIF
   helper with CMake, runs the test suites, the real-mount FUSE tests and the
   installer test, stages both assets and smoke-tests every way in: each binary
   must print `eidos <version of the tag>` for `--version`. It then validates
   `colony.json` against the asset names with a signature-checked `colony`
   binary, and uploads the two files as `build-*` artifacts. It holds no key and
   uses no build cache.
3. **sign-and-publish** calls the organisation's shared workflow in
   Project-Colony-Resources, pinned by commit. It signs each asset, verifies
   every signature and `.meta` on the release, and publishes the draft.

A failed run leaves the release a draft. To finish it once the cause is fixed,
start the workflow from the tag: `gh workflow run release.yml --ref vX.Y.Z -f
tag=vX.Y.Z`.

## The constraint

Kernel FUSE passthrough is **off by default**. The implementation records observed
Skyrim/Proton failures with it enabled; the supported default serves files through
the FUSE daemon. Lack of CAP_SYS_ADMIN does not establish that script-extender
DLLs cannot load.

Passthrough experiments require the relevant kernel support and CAP_SYS_ADMIN
in the initial user namespace. `./install.sh --cap` or `just setcap` explicitly
requests that capability on `eidos` only. Neither command enables passthrough;
`EIDOS_FUSE_PASSTHROUGH=1` is the separate runtime opt-in. The GUI's Diagnostics
panel reports this mode and missing prerequisites.

File capabilities are ignored on `nosuid` mounts and may be lost when a binary
is replaced. Only reapply one when deliberately using the optional mode.

## Flatpak integration remains unverified

The earlier title is retained for existing links, but its original absolute
claim was incorrect. Flatpak support has not been implemented or verified.
Sandbox access to game files, Steam/Proton, FUSE and mount namespaces would need
an integration design. Its restrictions on file capabilities alone do not prove
that Eidos's default rootless mode is impossible.

## AppImage integration remains unverified

AppImage support is also unimplemented and unverified. An image mounted with
`nosuid` cannot grant a file capability, but normal rootless operation does not
need that capability. Packaging and launch integration still need testing before
such a format can be supported.

## Verify it yourself

`just doctor` reports the kernel, `/dev/fuse`, `fusermount3`, user namespaces and
optional capability state. Absence of a capability is normal. `just test-fuse`
runs real mount tests when the environment permits; inspect their skip count.

## The Arch package

`packaging/PKGBUILD` installs the binaries, desktop handlers and icons without
adding capabilities. Its checks reuse the release build and run GUI tests
serially, matching CI. No `.deb` or `.rpm` package is maintained here.

## The bundle installer

`packaging/install.sh` is the single installer used by source builds and the
release bundle.
Re-running it replaces installed binaries. The default destination is
`~/.local/bin` (or `XDG_BIN_HOME`); `--from DIR` selects the build output.

## Building from source

```sh
just build        # build without sudo
just doctor       # report mount prerequisites and optional capability state
just run-gui      # build and start the GUI
just test         # workspace tests, with the GUI suite run serially
just install      # build and install for the current user
```

The binaries must remain together because the GUI finds its CLI launcher beside
its own executable before falling back to `~/.cargo/bin/eidos` and then to
running the CLI it links itself.

Complete source builds require CMake 3.20+, a C++17 compiler and Python 3 for
native tests. The NIF helper uses the vendored, pinned nifly source; no model
assets are downloaded. Release archives and Arch packages carry its upstream
and dependency notices. A Rust-only development build remains usable but reports
NIF previews unavailable until the helper is placed beside the GUI or on PATH.
