# Installing Eidos

Four ways in. Colony installs one file, `eidos-linux`, that is the whole
program. The release bundle, the Arch package and a source build give you
`eidos` (the CLI), `eidos-gui` and the static NIF preview helper
`eidos-nif-preview` side by side. The Arch package and a source build also
install `eidos-fuse` and `eidos-launch`, the two lower-level CLIs. Every route
can register the `nxm://` handler that makes the Nexus "Mod Manager Download"
button land in your instance.

## What you need first

| Requirement | Why |
|---|---|
| Linux with FUSE | `fusermount3` on your PATH. Every current distribution ships it. |
| A Proton game, launched once | Steam only creates the game's Wine prefix on first launch, and Eidos works inside it. Eidos also detects the GOG and Epic copies of Skyrim Special Edition, the Epic copies of Fallout 3 and Fallout: New Vegas, and the GOG copy of Stardew Valley. Those run through Heroic or Legendary with Eidos as the game's wrapper there: see [usage.md](usage.md#launching-the-game). |
| A 7-Zip binary: `7z`, `7zz` or `7za` | For installing mod archives. `7zip` on Arch, `p7zip` on most other distributions. |
| `par2` (optional) | PAR2 recovery data for `eidos pack` backups. `par2cmdline` on Arch and Fedora, `par2` on Debian and Ubuntu. |

No root, no daemon, no `/etc/fuse.conf` edit, and nothing to add to your groups.
Eidos mounts inside a private namespace that belongs to the game process.

Some system settings only show up as crashes. The Diagnostics tab checks
`vm.max_map_count` (1048576 or more), the hard open files limit (524288 or
more) and whether the game, its prefix or the instance sits on NTFS, exFAT or
FAT; [troubleshooting.md](troubleshooting.md#the-system) has the fix for each.

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
release bundle, the Arch package and source builds carry; without it the
preview says so, and nothing else is affected. `eidos-linux` also carries none
of the Microsoft runtimes the bundle ships
([below](#the-microsoft-runtimes-in-the-bundle)): when a tool needs
one, Eidos downloads it from Microsoft once you start the prerequisite install.
Point Steam at the file itself, as below.

## A release bundle

Download `eidos-linux-bundle.tar.gz` and its `.sig`, `.meta` and `.meta.sig`
from the [latest release](https://github.com/Project-Colony/Eidos/releases/latest),
and check them as the README's
[Code signing policy](../../README.md#code-signing-policy) shows. The bundle
holds `eidos`, `eidos-gui` and `eidos-nif-preview` at its top level, with
`install.sh`, `README.md`, `LICENSE`, `SECURITY.md`, the icons in
`assets/brand/png` and the third-party notices in `licenses/` (nifly, omod,
microsoft-directx, microsoft-runtimes and the Rust crates) and the `runtimes/`
folder below. It does not carry `eidos-fuse` or `eidos-launch`. Then:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

`install.sh` copies the binaries into `~/.local/bin`, or into `$XDG_BIN_HOME`
when that is set. `--system` puts them in `/usr/local/bin` through sudo,
`--bindir DIR` anywhere else, and `--from DIR` takes the binaries from another
directory. The script also:

- registers the `nxm://` handler by running `eidos nxm --register`, so the
  register step in the Colony section is not needed;
- writes `~/.local/share/applications/eidos.desktop` and the Eidos icons under
  `~/.local/share/icons/hicolor`;
- copies the licence notices to `~/.local/share/licenses/eidos`
  (`/usr/local/share/licenses/eidos` with `--system`);
- copies `runtimes/` to `~/.local/share/Colony/Eidos/runtimes`, also with
  `--system`, so the runtimes belong to the user who runs the script;
- ends by printing the Steam launch option with the absolute path of the
  `eidos-gui` it installed.

`--cap` asks sudo to grant `CAP_SYS_ADMIN` to the installed `eidos` (see FUSE
passthrough below). `--no-cap` is the default and leaves it rootless.
Re-running the script is the supported way to upgrade. The bundle and
`eidos-linux` need x86_64 Linux with glibc 2.39 or newer.

### The Microsoft runtimes in the bundle

The bundle's `runtimes/` folder holds the Microsoft files that tools ask for:
`VC_redist.x86.exe` and `VC_redist.x64.exe` (Visual C++ 2015-2022, the
`vcrun2022` prerequisite), `windowsdesktop-runtime-8.0.12-win-x64.exe` (the .NET
8 Desktop Runtime, for `dotnet8` and `dotnetdesktop8`) and
`dotnet-runtime-10.0.10-win-x64.zip` (the .NET 10 runtime, for `dotnet10`).
Eidos runs the Visual C++ and .NET 8 installers itself in the game's prefix,
without winetricks, and unpacks the .NET 10 zip once into
`~/.local/share/Colony/Eidos/runtimes`, where every instance shares it (78 MB
unpacked). Each file is used only when its SHA-256 matches the one pinned in
Eidos. The .NET 10 SDK that Synthesis needs (`dotnetsdk10`, about 300 MB) is
not in the bundle and is always downloaded.

An unpacked bundle also runs in place: Eidos looks for `runtimes/` beside its
own binary after the data folder above.

## Arch

```bash
cd packaging && makepkg -si
```

The PKGBUILD builds the current `main` from source. It installs `eidos`,
`eidos-gui`, `eidos-nif-preview`, `eidos-fuse` and `eidos-launch` into
`/usr/bin`, with a system-wide `nxm://` handler, a desktop entry and the icons.
It depends on `fuse3`, `7zip` and `xdg-utils`. It ships no `runtimes/` folder,
so the Microsoft runtimes are downloaded when a tool needs them. `winetricks`
is optional: Eidos still uses it for the prerequisites it does not install
itself, such as `dotnet48` and `xact`.

## From source

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Source builds require Rust 1.92 or newer, `just`, CMake 3.20+, a C++17 compiler and Python 3
for the native helper's tests. `just build` builds the whole workspace, so
`eidos-fuse` and `eidos-launch` are built too, and `just install` runs
`packaging/install.sh` on the build output, which copies them with the rest.
For commands without `just`, see
[the native helper instructions](../../crates/eidos-nif-preview/README.md),
build Rust with `cargo build --release --locked`, copy the helper into
`target/release`, then run `packaging/install.sh --from target/release`. A
source build has no `runtimes/` folder either, so the Microsoft runtimes are
downloaded when a tool needs them.

The release profile uses one codegen unit, fat LTO and stripped symbols. With
rustc 1.99.1 and 3 build jobs, a cold build of `eidos` and `eidos-gui` took
334 s, against 243 s with thin LTO, and gave a 20.9 MB `eidos` and a 49.7 MB
`eidos-gui`. Each binary embeds eight DirectX DLLs stored gzip-compressed: 7.9
MB instead of 17.9 MB. `just profile=dev build` builds without optimisation.

## Then: point Steam at it

Eidos runs *as* your game's launch command, which is how it gets to mount before
the game starts. In Steam, right-click the game -> Properties -> Launch Options,
and enter the absolute path of the Eidos window followed by `%command%`:

```
~/.local/bin/eidos-gui %command%
```

Use the absolute path - Steam does not read your shell's `PATH`. The path
depends on how you installed Eidos, and `install.sh` prints the line for the
directory it used:

| Install | Launch option |
|---|---|
| Release bundle or `just install` | `~/.local/bin/eidos-gui %command%` |
| `install.sh --system` | `/usr/local/bin/eidos-gui %command%` |
| Arch package | `/usr/bin/eidos-gui %command%` |
| Colony | `~/.local/share/Colony/apps/Eidos/eidos-linux %command%` |

Press Play. Eidos opens on that game's instance; install mods, sort with LOOT,
click Run. When you quit, the mount goes with it and your installation is exactly
as it was.

The game always starts through its store. In a window you opened yourself, Run
runs `steam -applaunch <appid>`, Steam starts Eidos through the launch option,
and a second Eidos window opens where you press Run. Until the launch option
starts Eidos, Run starts nothing and the status line shows the line to set.
With Start the game at once when a store opens Eidos (Preferences, General,
Running a game; off by default), Play in Steam starts the game without the
window when Health lists no problem. Heroic and Legendary work the same way
through their wrapper setting; [usage.md](usage.md#launching-the-game) has the
details.

The Diagnostics tab checks the line. A Health card shows the line to set when
the launch option is empty, names no Eidos binary, points at a file that does
not exist, has no `%command%` after Eidos, or starts another copy of Eidos than
the one running.

### If you prefer the terminal

```sh
eidos init skyrimse               # create an instance (add a folder to make it portable)
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root mods
eidos sort skyrimse               # LOOT-sort the load order
eidos check skyrimse              # the Health checks that need no window
eidos play skyrimse -- %command%  # run anything through the merged view
eidos shell skyrimse              # a shell in the merged view, writes land in Overwrite
```

Every command that takes a game id also takes a portable instance's folder -
see [usage.md](usage.md#instances-global-and-portable). The full tour is in
[usage.md](usage.md).

## Updating

Colony updates `eidos-linux` through its launcher. For the bundle, run the new
bundle's `install.sh`; for the Arch package, run `makepkg -si` again; for a
source build, pull and run `just install`. The window asks GitHub for the
latest release at start, at most once a day, and when a newer one is out it
shows a link to its release notes after Help in the menu bar. It never
downloads or replaces itself. Preferences, About, Version turns the check off,
offline mode skips it, and a copy installed under `/usr` (the Arch package)
never asks, because the package manager updates it.

## Optional: FUSE passthrough

`sudo setcap cap_sys_admin+ep "$(command -v eidos)"` grants the optional
capability; it does not enable passthrough. `./install.sh --cap` and
`just setcap` grant it to the `eidos` they install or build. On a `nosuid`
mount the kernel ignores file capabilities, so `install.sh` skips the grant
with a warning and `just setcap` stops with an error. The capability goes on
the separate `eidos` binary of the bundle, the Arch package or a source build,
never on `eidos-linux`, which is also the window
([SECURITY.md](../../SECURITY.md)). A rebuilt or reinstalled binary is a new
file without the capability, so grant it again after each upgrade if you use
it.

`EIDOS_FUSE_PASSTHROUGH=1` is the separate runtime switch, and it is off by
default. With passthrough on, an earlier daemon stopped Skyrim SE loading any
mod content. The most likely cause is that it registered a new backing file on
every open, while the kernel accepts one per inode. The daemon now shares one
backing between all the passthrough opens of a file, but that has not been
re-measured in a game. Until it has, turn passthrough on only to test it. The
measurements and the cause are in [troubleshooting.md](troubleshooting.md).

## Something already wrong?

[troubleshooting.md](troubleshooting.md) covers the environment switches, how to
read the operation counters, the Health cards, and every issue that has bitten
someone so far.
