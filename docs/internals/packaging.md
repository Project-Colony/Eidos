# Packaging and distribution

Eidos ships as a native Arch package, a release tarball, or a source build.
Normal operation uses rootless FUSE and does not require a file capability.

## Install

| Channel | Command | Privileges |
|---|---|---|
| Arch package | `cd packaging && makepkg -si` | Package installation uses pacman |
| Tarball | `./install.sh` | None for the default user installation |
| From source | `just build`, then `just install` | None for the default user installation |

The installer keeps `eidos` and `eidos-gui` together, registers the Nexus download
handler and desktop entry, and installs bundled icons. Use `--bindir DIR` for a
custom destination or `--system` for `/usr/local/bin`. A system destination may
require sudo. Release packaging copies this same tracked installer; there is no
second generated implementation to drift out of date.

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

## Why Flatpak is impossible

The earlier title is retained for existing links, but its original absolute
claim was incorrect. Flatpak support has not been implemented or verified.
Sandbox access to game files, Steam/Proton, FUSE and mount namespaces would need
an integration design. Its restrictions on file capabilities alone do not prove
that Eidos's default rootless mode is impossible.

## Why AppImage is a trap

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

## The tarball installer

`packaging/install.sh` is the single installer used by source and release builds.
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
its own executable before falling back to PATH.
