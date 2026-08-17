<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**The native Linux mod manager that never touches your game.**

</div>

Eidos gives Bethesda games on Linux what Mod Organizer 2 gives them on Windows -
a virtual, per-launch merged view of your mods - built from Linux primitives
instead of Windows API hooking. No Wine for the manager. No files copied into
the game directory. No cleanup path, because there is nothing to clean up.

```
Steam ──> eidos-gui %command% ──> [ private namespace ]
                                  │  mods ⊕ game  ──> what the game sees
                                  └─ dies with the game; the install stays pristine
```

> **Status:** Skyrim SE is played through Eidos daily - SKSE, script-extender
> preloaders, Creation Club, LOOT-sorted load orders, per-profile saves, the
> lot. One game family proven in real play so far; ten more are wired and
> waiting for testers.

## Why Eidos

- 🔒 **A mount only your game can see.** The merged view lives in a private
  mount namespace: your file manager, your backup job, a second game - none of
  them see it, none of them need permission for it. Kill the game, pull the
  power: the namespace dies with the process tree and your install is exactly
  as it was. There is no residue *by construction*.
- 🧾 **One copy of the truth.** Your profile owns its mod list, plugin order,
  INIs and saves. The plugin files and the save directory are bind-mounted over
  the game's own paths at launch, so even the game's own writes land in your
  profile. Switching profiles switches everything.
- 🐧 **Fully rootless.** No setuid helper, no daemon, no `sudo setcap`, no
  `/etc/fuse.conf` edits. One binary, one Steam launch option.
- 🛡️ **Guards with receipts.** A crash that wrecks your plugin list is flagged
  against a pre-session snapshot, with a one-click restore. A capture that would
  wipe your load order is refused and says why.

## What it does

**Mods.** Simple archives, FOMOD wizards, Wrye Bash BAIN packages, a manual
picker for the rest - and **root mods natively** (script extender preloaders,
ENB, Engine Fixes), with no Root Builder plugin and nothing copied into your
install. Hide single files, group with separators, targeted moves, per-mod notes
and categories, and an MO2 profile importer.

**Plugins.** The load order with LOOT sorting built in, mod indexes like the game
computes them, missing-master warnings, and your DLC and Creation Club content
shown as the unmanaged rows they are.

**Profiles.** Per-profile mod order, plugin state, INIs and saves. Saves are
parsed, diffed against your current plugins - with a button that enables what a
save needs - and synced back for Steam Cloud after every session.

**Nexus.** Connect an account and the site's "Mod Manager Download" button lands
straight in your instance, with update checks against what you have installed.

**Tools.** xEdit, BodySlide, DynDOLOD and friends run *through the merged view*
inside the game's Proton prefix - they see your mods, their output lands in
Overwrite, and one click turns it into a real mod. Whatever runtime each one
needs is fetched on request, so a missing DLL is a button rather than an
afternoon.

**Diagnostics.** Missing masters, orphaned archives, mod-list drift, damaged
plugin sets - and, after a run, what the script extender's own log says actually
loaded.

## How it compares

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / link deployers |
|---|---|---|---|---|
| Manager runs natively | ✅ | ❌ Windows app in Wine | ✅ (Qt port) | ✅ |
| Game dir untouched | ✅ always | ✅ | ✅ | ❌ links written into it |
| Mount visible to | the game only | the game only | **the whole system** | n/a |
| Crash cleanup needed | none, by design | none | stale-mount recovery | manual un-deploy |
| Root mods (ENB, preloaders) | ✅ native | plugin required | plugin required | partial |
| Privileges required | none | none | `/etc/fuse.conf` edit | none |

## How fast it is

| | before | now |
|---|---|---|
| loading a save | ~20 seconds | **6-7 seconds** |
| directory reads in one session | 5.6 million | 465 thousand |

Cell changes are immediate. The gain came from asking your mods fewer questions:
finding one file used to interrogate all fifty of them in turn, and listing one
folder used to do it fifty times over. Neither does any more. Measured on a real
instance played normally, not on a benchmark.

## Get started

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Then set your game's Steam launch option to `~/.local/bin/eidos-gui %command%`
and press Play.

Arch packages and release tarballs, what you need installed first, and the CLI
route: **[docs/guide/install.md](docs/guide/install.md)**.

## Steam launch options

The base line is all most setups need:

```
~/.local/bin/eidos-gui %command%
```

Everything else is environment variables stacked in front of it, and they
combine freely:

| You want... | Put in front |
|---|---|
| DLSS with Community Shaders | `PROTON_ENABLE_NVAPI=1` - without it DLSS silently never initialises; the full checklist is [guide/graphics.md](docs/guide/graphics.md) |
| an FPS counter on screen | `DXVK_HUD=fps` |
| driver-level frame interpolation, zero mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - never together with Community Shaders' own frame generation |
| verbose logs for a bug report | `EIDOS_LOG=debug` (session logs land in `~/.local/state/eidos/logs/`) |
| a per-session I/O report from the mount | `EIDOS_FUSE_STATS=1` |
| a different FUSE worker count | `EIDOS_FUSE_THREADS=8` (default 4; `1` is the first thing to try when chasing a concurrency bug) |

The line to keep for a modern modded setup (Community Shaders, DLSS, frame
generation) - this is the final command, not an example:

```
PROTON_ENABLE_NVAPI=1 ~/.local/bin/eidos-gui %command%
```

Add `DXVK_HUD=fps` in front while verifying the setup works, drop it once it
does.

The deeper diagnostic switches (`EIDOS_FUSE_TRACE`, the cache and index
bisection toggles, why `EIDOS_FUSE_PASSTHROUGH` is off by default) live in
[guide/troubleshooting.md](docs/guide/troubleshooting.md).

## Where to go next

| If you want to... | |
|---|---|
| install it | [guide/install.md](docs/guide/install.md) |
| learn the CLI and the GUI | [guide/usage.md](docs/guide/usage.md) |
| set up xEdit, BodySlide or DynDOLOD | [guide/tools.md](docs/guide/tools.md) |
| get DLSS / frame generation working (Community Shaders) | [guide/graphics.md](docs/guide/graphics.md) |
| fix something that looks wrong | [guide/troubleshooting.md](docs/guide/troubleshooting.md) |
| know why it is fast, and check it yourself | [internals/performance.md](docs/internals/performance.md) |
| understand how it works inside | [internals/architecture.md](docs/internals/architecture.md) |
| build it, test it, contribute | [internals/contributing.md](docs/internals/contributing.md) |
| know why it exists at all | [project/landscape.md](docs/project/landscape.md) |

The whole index is at [docs/README.md](docs/README.md); security policy and how
to report a vulnerability at [SECURITY.md](SECURITY.md).

## Supported games

**Skyrim SE/AE** - proven in real play. Wired per the shared game descriptor and
looking for testers: Skyrim LE, Skyrim VR, Enderal SE, Fallout 3, Fallout NV,
Fallout 4 (+ VR), Starfield, Oblivion and Morrowind (the last two mount and
manage mods; their timestamp-ordered plugin lists are not managed yet).

Adding a family is one descriptor row:
[internals/adding-games.md](docs/internals/adding-games.md).

## Prior art and thanks

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) and
  [usvfs](https://github.com/ModOrganizer2/usvfs) - the semantics Eidos
  reproduces, and the codebase its parity was studied against
- [LOOT](https://loot.github.io/) - the sorting engine, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) and the other Linux managers -
  proof there is a community that wants this solved

## License

GPL-3.0-or-later. Mod management belongs to everyone.
