# Installing Eidos

Four ways in. Colony installs one file, `eidos-linux`, that is the whole
program; the others give you `eidos` (the CLI), `eidos-gui` and the static NIF
preview helper side by side, plus the `nxm://` handler that makes the Nexus
"Mod Manager Download" button land in your instance.

## What you need first

| | |
|---|---|
| **Linux with FUSE** | `fusermount3` on your PATH. Every current distribution ships it. |
| **A Proton game, launched once** | Steam only creates the game's Wine prefix on first launch, and Eidos works inside it. |
| **`7z`** | For installing mod archives. `p7zip` on most distributions. |

No root, no daemon, no `/etc/fuse.conf` edit, and nothing to add to your groups.
Eidos mounts inside a private namespace that belongs to the game process.

## Via Colony

Search for **Eidos** in [Colony](https://github.com/Project-Colony/Colony) and
install it; updates arrive through the launcher. Colony keeps one file,
`~/.local/share/Colony/apps/Eidos/eidos-linux`. Run it and the window opens.
Called through a link named `eidos`, the same file is the command line. Put
one on your `PATH` and register the Nexus download handler:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/share/Colony/apps/Eidos/eidos-linux ~/.local/bin/eidos
eidos nxm --register    # the site's Mod Manager Download button now reaches Eidos
```

The window needs no link: whenever it launches a game or a tool, it runs its own
file as the CLI. The NIF model preview needs `eidos-nif-preview`, which only the
release bundle and source builds carry; without it the preview says so, and
nothing else is affected. Point Steam at the file itself, as below.

## A release bundle

Download `eidos-linux-bundle.tar.gz` and its `.sig`, `.meta` and `.meta.sig`
from the [latest release](https://github.com/Project-Colony/Eidos/releases/latest),
and check them as the README's
[Code signing policy](../../README.md#code-signing-policy) shows. Then:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

Installs into `~/.local/bin` by default. `--system` puts it in `/usr/local/bin`,
`--bindir DIR` anywhere else. Re-running it is the supported way to upgrade.
The bundle and `eidos-linux` need x86_64 Linux with glibc 2.39 or newer.

## Arch

```bash
cd packaging && makepkg -si
```

The PKGBUILD builds the current `main` from source.

## From source

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Source builds require Rust 1.89 or newer, `just`, CMake 3.20+, a C++17 compiler and Python 3
for the native helper's tests. For commands without `just`, see
[the native helper instructions](../../crates/eidos-nif-preview/README.md),
build Rust with `cargo build --release --locked`, copy the helper into
`target/release`, then run `packaging/install.sh --from target/release`.

## Then: point Steam at it

Eidos runs *as* your game's launch command, which is how it gets to mount before
the game starts. In Steam, right-click the game -> Properties -> Launch Options:

```
~/.local/bin/eidos-gui %command%
```

From Colony, the path is `~/.local/share/Colony/apps/Eidos/eidos-linux`. Press
Play. Eidos opens on that game's instance; install mods, sort with LOOT,
click Run. When you quit, the mount goes with it and your installation is exactly
as it was.

Use the absolute path - Steam does not read your shell's `PATH`.

### If you prefer the terminal

```sh
eidos init skyrimse               # create an instance (add a folder to make it portable)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root mods
eidos sort skyrimse               # LOOT-sort the load order
eidos play skyrimse -- %command%  # run anything through the merged view
```

Every command that takes a game id also takes a portable instance's folder -
see [usage.md](usage.md#instances-global-and-portable). The full tour is in
[usage.md](usage.md).

## Optional: FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` grants the optional capability; it does not enable passthrough.
It goes on the separate `eidos` binary of the bundle or a source build, never on
`eidos-linux`, which is also the window ([SECURITY.md](../../SECURITY.md)).
`EIDOS_FUSE_PASSTHROUGH=1` is the separate runtime switch. It is **off by default and you almost certainly want it that way**:
measured on Skyrim SE it stops the game opening its own archives and plugins, so
mods silently do not load. The switch exists to re-test the mechanism, not
because it is recommended.

Details, and the measurements behind that decision, in
[troubleshooting.md](troubleshooting.md).

## Something already wrong?

[troubleshooting.md](troubleshooting.md) covers the environment switches, how to
read the operation counters, and every issue that has bitten someone so far.
