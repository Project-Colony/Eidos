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
no second generated implementation to drift out of date. From the bundle it
also copies `runtimes/` into `~/.local/share/Colony/Eidos/runtimes/` (under
`$XDG_DATA_HOME` when that is set), where Eidos looks for a Microsoft
installer or runtime before it downloads one. A `--system` install still puts
them in the data folder of the user who runs it.

## Release assets

Every release carries two assets, named after the Colony convention so Colony
finds the Linux one without any `releaseFiles` in `colony.json`:

| Asset | Contents |
|---|---|
| `eidos-linux` | `eidos-gui` under the asset's name. It links the CLI too: called through a link named `eidos`, or with `--eidos-cli` first, it runs the CLI instead of a window. The window looks for `eidos` beside itself, then for `~/.cargo/bin/eidos`, and with neither it launches games and tools through itself that way. This one file is what Colony installs. |
| `eidos-linux-bundle.tar.gz` | `eidos-linux-bundle/` with `eidos`, `eidos-gui`, `eidos-nif-preview`, `install.sh`, the icons, `README.md`, `LICENSE`, `SECURITY.md`, the third-party notices under `licenses/`, and under `runtimes/` the Microsoft files listed in `packaging/runtimes.txt`. Deterministic: fixed file order, owner and mtime. |

Each asset comes with `.sig`, `.meta` and `.meta.sig` (see the README's
[Code signing policy](../../README.md#code-signing-policy)). No separate
`.sha256` file is published: the `.meta` carries the sha256, and its signature
is what makes that digest worth checking.

`runtimes/` holds `VC_redist.x86.exe` and `VC_redist.x64.exe` (`vcrun2022`),
`windowsdesktop-runtime-8.0.12-win-x64.exe` (`dotnet8` and `dotnetdesktop8`)
and `dotnet-runtime-10.0.10-win-x64.zip` (`dotnet10`). Their license terms are
under `licenses/microsoft-runtimes/`. Eidos uses one of these copies only when
its SHA-256 matches the pin in `crates/eidos-gamefeatures`, and a test checks
that `runtimes.txt` and those pins agree. `eidos-linux` carries no
`runtimes/` folder, and neither does the Arch package, so those installs
download the same pinned files from Microsoft on `eidos prereqs --install`.
The .NET 10 SDK that Synthesis needs (`dotnetsdk10`, 10.0.403, about 300 MB)
is never shipped and is always a download.

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
   helper with CMake, runs the test suites, the real-mount FUSE tests, the
   transfer clock test and the installer test, and stages both assets. Staging
   downloads every file in `packaging/runtimes.txt` and checks it against the
   SHA-256 on its line, and it stops the release while a Microsoft license
   notice still holds its `TODO(owner)` placeholder. The job then smoke-tests
   every way in: each binary must print `eidos <version of the tag>` for
   `--version`, and every runtime must be in the bundle. It then validates
   `colony.json` against the asset names with a signature-checked `colony`
   binary, and uploads the two files as `build-*` artifacts. It holds no key and
   uses no build cache.
3. **sign-and-publish** calls the organisation's shared workflow in
   Project-Colony-Resources, pinned by commit. It signs each asset, verifies
   every signature and `.meta` on the release, and publishes the draft.

A failed run leaves the release a draft. To finish it once the cause is fixed,
start the workflow from the tag: `gh workflow run release.yml --ref vX.Y.Z -f
tag=vX.Y.Z`.

Release builds use `[profile.release]` in the root `Cargo.toml`:
`codegen-units = 1`, `lto = "fat"` and `strip = "symbols"`. The comment beside
the profile records what was measured.

One codegen unit instead of the default 16 lets LLVM see across the whole
crate, so the same monomorphised code does not ship several times over. When it
was set, on a real 182-layer instance, it took 22% off the binaries (`eidos-gui`
40.6 to 31.8 MB, `eidos` 29.3 to 26.8 MB) for 12% more build time. Speed did not
move: path resolution stayed at 2.8 µs and a forced directory walk at about
610 ms, because both are bound by syscalls.

The Colony release profile asks for thin LTO; Eidos takes fat LTO for the size.
Measured on 2026-11-10 with rustc 1.99.1, one cold
`cargo build --profile release -p eidos -p eidos-gui` per variant with 3 build
jobs, sizes in bytes after strip:

| Variant | `eidos` | `eidos-gui` | Build |
|---|---|---|---|
| thin LTO (the previous profile) | 22,117,176 | 53,567,752 | 243 s |
| fat LTO (the current profile) | 20,870,160 | 49,727,560 | 334 s |
| thin LTO, GUI stack at `opt-level = "s"` | 22,116,368 | 51,839,880 | 234 s |
| fat LTO, GUI stack at `opt-level = "s"` | 20,871,056 | 48,080,456 | 325 s |

Fat LTO takes 1.2 MB (5.6%) off `eidos` and 3.8 MB (7.2%) off `eidos-gui` for
37% more build time. The GUI stack (iced and its sub-crates, wgpu, wgpu-core,
wgpu-hal, naga, cosmic-text, swash, skrifa, read-fonts, fontdb, tiny-skia and
colony-ui) at `opt-level = "s"` saved another 1.6 MB (3.3%) of `eidos-gui`.
That is too little to give up speed in the renderer and in text shaping, so
every crate stays at opt-level 3.

The four DirectX DLLs Eidos deploys into a prefix (`d3dcompiler_47`,
`d3dcompiler_43`, `d3dx9_43` and `d3dx11_43`, each in an x86_64 and an i386
build) are embedded gzip-compressed. The eight files take 7.9 MB of each binary
instead of 17.9 MB, and one is unpacked, 4.7 MB at most, only while it is
deployed.

Without the symbol table, a panic backtrace from a release binary shows
addresses instead of function names, so a crash report needs the session log's
message and the steps, or a rebuild with symbols. `panic = "abort"` is left out
on purpose. About 15 worker threads run in production paths (archive
extraction, the game-exit watcher, downloads). Today a panic in one of them is
reported and the window survives; with abort, the whole window would die.

The Arch package and the release bundle install the license notices next to the
binaries: the GPL text, the OMOD and nifly credits, the provenance of the
embedded Microsoft DirectX DLLs under `microsoft-directx/`, and under `crates/`
the notices of every Rust crate linked into the binaries, with `INDEX.tsv`
naming each crate and its declared license. The release bundle adds the
license terms of the Microsoft runtimes under `microsoft-runtimes/`.
`scripts/third-party-licenses.sh` collects the crate notices from the fetched
crate sources; it builds nothing.

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

Flatpak support has not been implemented or verified. Eidos warns when a game's
prefix or the resolved Proton belongs to the Flatpak Steam
(`com.valvesoftware.Steam`). It never relaunches through `flatpak run`, because
the game would then run in Flatpak's sandbox, which cannot see the union Eidos
mounts in its private namespace, and would play without mods. Sandbox access to
game files, Steam/Proton, FUSE and mount namespaces would need an integration
design. Flatpak's restrictions on file capabilities do not, on their own, rule
out Eidos's default rootless mode.

The store hand-off reads the wrappers of a Flatpak Heroic
(`~/.var/app/com.heroicgameslauncher.hgl/config/heroic`) as well as a native
one, and its `heroic://launch` link reaches either. Eidos running as a wrapper
inside Flatpak Heroic has not been tested with a real launch. For a Flatpak
Steam the hand-off spawns `steam -applaunch <appid>`; with no `steam` on PATH
that spawn fails and the window shows the error.

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

`packaging/PKGBUILD` installs the binaries (`eidos-fuse` and `eidos-launch`
included), desktop handlers and icons without adding capabilities. Its checks
reuse the release build and run GUI tests serially, matching CI. It ships no
`runtimes/` folder. `vcrun2022`, `dotnet8`, `dotnetdesktop8` and the .NET 10
runtime and SDK need no winetricks; the other prerequisite verbs (`dotnet48`
for FNIS, `xact`, `dotnet6`, `dotnet7`) still run the system `winetricks`. No
`.deb` or `.rpm` package is maintained here.

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
