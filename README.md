# Eidos

**The native Linux mod manager that never touches your game.**

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
  profile - MO2's usvfs virtualization, done with a mount namespace. Switching
  profiles switches everything.
- 🐧 **Fully rootless.** No setuid helper, no daemon, no `sudo setcap`, no
  `/etc/fuse.conf` edits. One binary, one Steam launch option.
- 🛡️ **Guards with receipts.** A crash that wrecks your plugin list is flagged
  against a pre-session snapshot, with a one-click restore in Diagnostics. A
  capture that would wipe your load order is refused and says why. The rules
  came out of a 37-agent audit of the exact ways mod setups die quietly.

## What it does

**Mods.** Install anything: Simple archives, FOMOD wizards, Wrye Bash BAIN
packages, a manual picker for the rest - and **root mods natively** (script
extender preloaders, ENB, Engine Fixes): a mod's `Root/` folder is projected
onto the game directory by a second union, no Root Builder plugin, no files
copied into your install. Hide single files (`.mohidden`), group with
separators, targeted moves (above the first conflict, into a group), per-mod
notes, categories, and an MO2 profile importer.

**Plugins.** The ESP/ESM/ESL load order with LOOT sorting built in (plus its
post-sort report), mod indexes like the game computes them, missing-master
warnings, and the DLCs / Creation Club content shown as the unmanaged rows they
are - so the list answers "is my DLC even there?" instead of raising it.

**Profiles.** Per-profile mod order, plugin state, INIs, INI tweaks and saves.
Saves are bind-mounted, parsed (character, level, playtime), diffed against
your current plugins - with a button that enables what a save needs - and
synced back for Steam Cloud after every session.

**Nexus.** Connect an API key, register `nxm://`, and the site's
"Mod Manager Download" button downloads straight into the instance, with
update checks against your installed versions.

**Tools.** xEdit, BodySlide, FNIS and friends run *through the merged view*
inside the game's Proton prefix - they see your mods, their output lands in
Overwrite, and one click turns it into a real mod.

**Diagnostics.** Live health checks: missing masters, orphaned archives,
mod-list drift, damaged plugin sets (with the restore button), and - after a
run - what the script extender's own log says actually loaded.

## How it compares

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / link deployers |
|---|---|---|---|---|
| Manager runs natively | ✅ | ❌ Windows app in Wine | ✅ (Qt port) | ✅ |
| Game dir untouched | ✅ always | ✅ | ✅ | ❌ links written into it |
| Mount visible to | the game only | the game only | **the whole system** | n/a |
| Crash cleanup needed | none, by design | none | stale-mount recovery | manual un-deploy |
| Root mods (ENB, preloaders) | ✅ native | plugin required | plugin required | partial |
| Privileges required | none | none | `/etc/fuse.conf` edit | none |

The long-form analysis - every Linux approach, what each costs, and which
properties are genuinely exclusive - is in [docs/landscape.md](docs/landscape.md).

## Quick start

```sh
git clone https://github.com/Project-Colony/Eidos && cd Eidos
cargo build --release
install -m755 target/release/eidos target/release/eidos-gui ~/.local/bin/
```

Then set the game's Steam launch option and play:

```
~/.local/bin/eidos-gui %command%
```

Eidos opens on the game's instance; install mods, sort with LOOT, click Run.
Prefer a terminal? The whole thing drives from the CLI too:

```sh
eidos init skyrimse               # create an instance
eidos install skyrimse mod.7z     # Simple / FOMOD / BAIN / root mods
eidos sort skyrimse               # LOOT-sort the load order
eidos play skyrimse -- %command%  # run anything through the merged view
```

Full walkthrough (GUI tour, tools, MO2 import): [docs/usage.md](docs/usage.md).

## Documentation

| | |
|---|---|
| [usage.md](docs/usage.md) | CLI, GUI tour, Steam setup, building from source |
| [landscape.md](docs/landscape.md) | the problem, every Linux approach, what is exclusive here |
| [architecture.md](docs/architecture.md) | why FUSE, the daemon's design, caching, write semantics |
| [troubleshooting.md](docs/troubleshooting.md) | env switches, op counters, known issues and their history |
| [status.md](docs/status.md) | the full done/remaining ledger |
| [master-pieces.md](docs/master-pieces.md) | the MO2 + usvfs study that drove parity |
| [adding-games.md](docs/adding-games.md) | wiring a new game family |
| [packaging.md](docs/packaging.md) | distribution notes (AppImage viable - no capability needed) |

## Supported games

**Skyrim SE/AE** - proven in real play. Wired per the shared game descriptor
and looking for testers: Skyrim LE, Skyrim VR, Enderal SE, Fallout 3,
Fallout NV, Fallout 4 (+ VR), Starfield, Oblivion and Morrowind (the last two
mount and manage mods; their timestamp-ordered plugin lists are not managed
yet). Adding a family is one descriptor row: [docs/adding-games.md](docs/adding-games.md).

## Prior art and thanks

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) and
  [usvfs](https://github.com/ModOrganizer2/usvfs) - the semantics Eidos
  reproduces, and the codebase its parity was studied against
- [LOOT](https://loot.github.io/) - the sorting engine, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) and the other Linux managers -
  proof there is a community that wants this solved

## License

GPL-3.0. Mod management belongs to everyone.
