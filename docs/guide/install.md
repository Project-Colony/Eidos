# Installing Eidos

Three ways in. All of them give you the same two binaries - `eidos` (the CLI) and
`eidos-gui` - plus the `nxm://` handler that makes the Nexus "Mod Manager
Download" button land in your instance.

## What you need first

| | |
|---|---|
| **Linux with FUSE** | `fusermount3` on your PATH. Every current distribution ships it. |
| **A Proton game, launched once** | Steam only creates the game's Wine prefix on first launch, and Eidos works inside it. |
| **`7z`** | For installing mod archives. `p7zip` on most distributions. |

No root, no daemon, no `/etc/fuse.conf` edit, and nothing to add to your groups.
Eidos mounts inside a private namespace that belongs to the game process.

## Arch

```bash
cd packaging && makepkg -si
```

## A release tarball

```bash
./install.sh
```

Installs into `~/.local/bin` by default. `--system` puts it in `/usr/local/bin`,
`--bindir DIR` anywhere else. Re-running it is the supported way to upgrade.

## From source

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

## Then: point Steam at it

Eidos runs *as* your game's launch command, which is how it gets to mount before
the game starts. In Steam, right-click the game -> Properties -> Launch Options:

```
~/.local/bin/eidos-gui %command%
```

Press Play. Eidos opens on that game's instance; install mods, sort with LOOT,
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

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` enables kernel FUSE
passthrough. It is **off by default and you almost certainly want it that way**:
measured on Skyrim SE it stops the game opening its own archives and plugins, so
mods silently do not load. The switch exists to re-test the mechanism, not
because it is recommended.

Details, and the measurements behind that decision, in
[troubleshooting.md](troubleshooting.md).

## Something already wrong?

[troubleshooting.md](troubleshooting.md) covers the environment switches, how to
read the operation counters, and every issue that has bitten someone so far.
