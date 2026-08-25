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

The list is MO2's, with its habits: eight optional columns and a sort on any of
them, grouping by category or by source, double-click gestures, type-to-jump,
per-mod backups that are inert until you restore them, and advisory flags for a
mod whose layout this game will not load or that was downloaded for another one.
Its file tree does the ordinary operations - new folder, rename, delete, open -
and previews images and text without launching anything.

**Plugins.** The load order with LOOT sorting built in, mod indexes like the game
computes them, missing-master warnings, and your DLC and Creation Club content
shown as the unmanaged rows they are.

**Instances.** Global - managed centrally under `~/.local/share/eidos` - or
portable: a self-contained folder anywhere you want (a second drive, a games
partition), movable and isolated, like MO2's. Portable instances are remembered
across sessions; the GUI, the Steam launch and every CLI command follow the one
you last used, and any command takes the folder wherever it takes a game id.
Details in [usage.md](docs/guide/usage.md#instances-global-and-portable).

**Profiles.** Per-profile mod order, plugin state, INIs and saves. Saves are
parsed, diffed against your current plugins - with a button that enables what a
save needs - and synced back for Steam Cloud after every session.

**Nexus.** Connect an account and the site's "Mod Manager Download" button lands
straight in your instance, with update checks against what you have installed,
who made each mod and a link to their profile. A **collection** link lists its
members joined against your instance - installed, downloaded, missing - which is
reading a collection rather than installing one, and the pane says why. The
Downloads tab is an archive library: filter, sort, hide without deleting, and
purge the ones already installed. An **offline** switch stops all of it.

**Tools.** xEdit, BodySlide, DynDOLOD and friends run *through the merged view*
inside the game's Proton prefix - they see your mods, their output lands in
Overwrite, and one click turns it into a real mod. Whatever runtime each one
needs is fetched on request, so a missing DLL is a button rather than an
afternoon. xEdit and its QuickAutoClean twin are found for you - in the game
folder, inside a mod, or in the tools directory you keep beside your games -
with the right runtimes already chosen. Pin the ones you use, hide the ones you
do not, give a tool its own
Steam AppID when it is its own Steam app, and write a `.desktop` shortcut that
launches it through the merged view without opening Eidos at all.

**Diagnostics.** Missing masters, orphaned archives, mod-list drift, damaged
plugin sets - and, after a run, what the script extender's own log says actually
loaded.

**Where it keeps its own files.** `~/.config/Colony/Eidos/` for what you chose -
preferences, your Nexus session, your instance list, the game and add-on
definitions you wrote - with logs under `~/.local/state/Colony/Eidos/`. The
layout every program in the Colony family uses. An older Eidos kept these in
`~/.config/eidos/`; the first launch after upgrading copies them across, says so
in the log, and leaves the old directory exactly as it was.

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
| verbose logs for a bug report | `EIDOS_LOG=debug` (session logs land in `~/.local/state/Colony/Eidos/logs/`) |
| a per-session I/O report from the mount | `EIDOS_FUSE_STATS=1` |
| a different FUSE worker count | `EIDOS_FUSE_THREADS=8` (default 4; `1` is the first thing to try when chasing a concurrency bug) |
| this launch pinned to one portable instance | `EIDOS_INSTANCE=/path/to/folder` - without it Eidos opens the instance you last used, which is usually what you want |

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
| play Fallout 4 (F4SE, versions, the NVIDIA debris crash) | [guide/fallout4.md](docs/guide/fallout4.md) |
| get DLSS / frame generation working (Community Shaders) | [guide/graphics.md](docs/guide/graphics.md) |
| fix something that looks wrong | [guide/troubleshooting.md](docs/guide/troubleshooting.md) |
| know why it is fast, and check it yourself | [internals/performance.md](docs/internals/performance.md) |
| understand how it works inside | [internals/architecture.md](docs/internals/architecture.md) |
| build it, test it, contribute | [internals/contributing.md](docs/internals/contributing.md) |
| know why it exists at all | [project/landscape.md](docs/project/landscape.md) |

The whole index is at [docs/README.md](docs/README.md); security policy and how
to report a vulnerability at [SECURITY.md](SECURITY.md).

## Language

The pages a player needs are translated. **English is canonical**: when a
translation disagrees with it, the English file is right.

- **Français** - [README](docs/i18n/fr/README.md) · [index](docs/i18n/fr/docs/README.md) · [install](docs/i18n/fr/docs/guide/install.md) · [usage](docs/i18n/fr/docs/guide/usage.md) · [tools](docs/i18n/fr/docs/guide/tools.md) · [fallout4](docs/i18n/fr/docs/guide/fallout4.md) · [graphics](docs/i18n/fr/docs/guide/graphics.md) · [troubleshooting](docs/i18n/fr/docs/guide/troubleshooting.md) · [extensions](docs/i18n/fr/docs/guide/extensions.md)
- **Русский** - [README](docs/i18n/ru/README.md) · [index](docs/i18n/ru/docs/README.md) · [install](docs/i18n/ru/docs/guide/install.md) · [usage](docs/i18n/ru/docs/guide/usage.md) · [tools](docs/i18n/ru/docs/guide/tools.md) · [fallout4](docs/i18n/ru/docs/guide/fallout4.md) · [graphics](docs/i18n/ru/docs/guide/graphics.md) · [troubleshooting](docs/i18n/ru/docs/guide/troubleshooting.md) · [extensions](docs/i18n/ru/docs/guide/extensions.md)
- **Deutsch** - [README](docs/i18n/de/README.md) · [index](docs/i18n/de/docs/README.md) · [install](docs/i18n/de/docs/guide/install.md) · [usage](docs/i18n/de/docs/guide/usage.md) · [tools](docs/i18n/de/docs/guide/tools.md) · [fallout4](docs/i18n/de/docs/guide/fallout4.md) · [graphics](docs/i18n/de/docs/guide/graphics.md) · [troubleshooting](docs/i18n/de/docs/guide/troubleshooting.md) · [extensions](docs/i18n/de/docs/guide/extensions.md)
- **Español** - [README](docs/i18n/es/README.md) · [index](docs/i18n/es/docs/README.md) · [install](docs/i18n/es/docs/guide/install.md) · [usage](docs/i18n/es/docs/guide/usage.md) · [tools](docs/i18n/es/docs/guide/tools.md) · [fallout4](docs/i18n/es/docs/guide/fallout4.md) · [graphics](docs/i18n/es/docs/guide/graphics.md) · [troubleshooting](docs/i18n/es/docs/guide/troubleshooting.md) · [extensions](docs/i18n/es/docs/guide/extensions.md)
- **Português (BR)** - [README](docs/i18n/pt-BR/README.md) · [index](docs/i18n/pt-BR/docs/README.md) · [install](docs/i18n/pt-BR/docs/guide/install.md) · [usage](docs/i18n/pt-BR/docs/guide/usage.md) · [tools](docs/i18n/pt-BR/docs/guide/tools.md) · [fallout4](docs/i18n/pt-BR/docs/guide/fallout4.md) · [graphics](docs/i18n/pt-BR/docs/guide/graphics.md) · [troubleshooting](docs/i18n/pt-BR/docs/guide/troubleshooting.md) · [extensions](docs/i18n/pt-BR/docs/guide/extensions.md)
- **简体中文** - [README](docs/i18n/zh-CN/README.md) · [index](docs/i18n/zh-CN/docs/README.md) · [install](docs/i18n/zh-CN/docs/guide/install.md) · [usage](docs/i18n/zh-CN/docs/guide/usage.md) · [tools](docs/i18n/zh-CN/docs/guide/tools.md) · [fallout4](docs/i18n/zh-CN/docs/guide/fallout4.md) · [graphics](docs/i18n/zh-CN/docs/guide/graphics.md) · [troubleshooting](docs/i18n/zh-CN/docs/guide/troubleshooting.md) · [extensions](docs/i18n/zh-CN/docs/guide/extensions.md)
- **Polski** - [README](docs/i18n/pl/README.md) · [index](docs/i18n/pl/docs/README.md) · [install](docs/i18n/pl/docs/guide/install.md) · [usage](docs/i18n/pl/docs/guide/usage.md) · [tools](docs/i18n/pl/docs/guide/tools.md) · [fallout4](docs/i18n/pl/docs/guide/fallout4.md) · [graphics](docs/i18n/pl/docs/guide/graphics.md) · [troubleshooting](docs/i18n/pl/docs/guide/troubleshooting.md) · [extensions](docs/i18n/pl/docs/guide/extensions.md)
- **Italiano** - [README](docs/i18n/it/README.md) · [index](docs/i18n/it/docs/README.md) · [install](docs/i18n/it/docs/guide/install.md) · [usage](docs/i18n/it/docs/guide/usage.md) · [tools](docs/i18n/it/docs/guide/tools.md) · [fallout4](docs/i18n/it/docs/guide/fallout4.md) · [graphics](docs/i18n/it/docs/guide/graphics.md) · [troubleshooting](docs/i18n/it/docs/guide/troubleshooting.md) · [extensions](docs/i18n/it/docs/guide/extensions.md)
- **Українська** - [README](docs/i18n/uk/README.md) · [index](docs/i18n/uk/docs/README.md) · [install](docs/i18n/uk/docs/guide/install.md) · [usage](docs/i18n/uk/docs/guide/usage.md) · [tools](docs/i18n/uk/docs/guide/tools.md) · [fallout4](docs/i18n/uk/docs/guide/fallout4.md) · [graphics](docs/i18n/uk/docs/guide/graphics.md) · [troubleshooting](docs/i18n/uk/docs/guide/troubleshooting.md) · [extensions](docs/i18n/uk/docs/guide/extensions.md)
- **日本語** - [README](docs/i18n/ja/README.md) · [index](docs/i18n/ja/docs/README.md) · [install](docs/i18n/ja/docs/guide/install.md) · [usage](docs/i18n/ja/docs/guide/usage.md) · [tools](docs/i18n/ja/docs/guide/tools.md) · [fallout4](docs/i18n/ja/docs/guide/fallout4.md) · [graphics](docs/i18n/ja/docs/guide/graphics.md) · [troubleshooting](docs/i18n/ja/docs/guide/troubleshooting.md) · [extensions](docs/i18n/ja/docs/guide/extensions.md)
- **繁體中文** - [README](docs/i18n/zh-TW/README.md) · [index](docs/i18n/zh-TW/docs/README.md) · [install](docs/i18n/zh-TW/docs/guide/install.md) · [usage](docs/i18n/zh-TW/docs/guide/usage.md) · [tools](docs/i18n/zh-TW/docs/guide/tools.md) · [fallout4](docs/i18n/zh-TW/docs/guide/fallout4.md) · [graphics](docs/i18n/zh-TW/docs/guide/graphics.md) · [troubleshooting](docs/i18n/zh-TW/docs/guide/troubleshooting.md) · [extensions](docs/i18n/zh-TW/docs/guide/extensions.md)
- **Čeština** - [README](docs/i18n/cs/README.md) · [index](docs/i18n/cs/docs/README.md) · [install](docs/i18n/cs/docs/guide/install.md) · [usage](docs/i18n/cs/docs/guide/usage.md) · [tools](docs/i18n/cs/docs/guide/tools.md) · [fallout4](docs/i18n/cs/docs/guide/fallout4.md) · [graphics](docs/i18n/cs/docs/guide/graphics.md) · [troubleshooting](docs/i18n/cs/docs/guide/troubleshooting.md) · [extensions](docs/i18n/cs/docs/guide/extensions.md)
- **한국어** - [README](docs/i18n/ko/README.md) · [index](docs/i18n/ko/docs/README.md) · [install](docs/i18n/ko/docs/guide/install.md) · [usage](docs/i18n/ko/docs/guide/usage.md) · [tools](docs/i18n/ko/docs/guide/tools.md) · [fallout4](docs/i18n/ko/docs/guide/fallout4.md) · [graphics](docs/i18n/ko/docs/guide/graphics.md) · [troubleshooting](docs/i18n/ko/docs/guide/troubleshooting.md) · [extensions](docs/i18n/ko/docs/guide/extensions.md)
- **Türkçe** - [README](docs/i18n/tr/README.md) · [index](docs/i18n/tr/docs/README.md) · [install](docs/i18n/tr/docs/guide/install.md) · [usage](docs/i18n/tr/docs/guide/usage.md) · [tools](docs/i18n/tr/docs/guide/tools.md) · [fallout4](docs/i18n/tr/docs/guide/fallout4.md) · [graphics](docs/i18n/tr/docs/guide/graphics.md) · [troubleshooting](docs/i18n/tr/docs/guide/troubleshooting.md) · [extensions](docs/i18n/tr/docs/guide/extensions.md)
- **Nederlands** - [README](docs/i18n/nl/README.md) · [index](docs/i18n/nl/docs/README.md) · [install](docs/i18n/nl/docs/guide/install.md) · [usage](docs/i18n/nl/docs/guide/usage.md) · [tools](docs/i18n/nl/docs/guide/tools.md) · [fallout4](docs/i18n/nl/docs/guide/fallout4.md) · [graphics](docs/i18n/nl/docs/guide/graphics.md) · [troubleshooting](docs/i18n/nl/docs/guide/troubleshooting.md) · [extensions](docs/i18n/nl/docs/guide/extensions.md)

**Everything else is English on purpose, not by omission.** `docs/internals/` and
`docs/project/` are read by people who are also reading the Rust, and `CHANGELOG.md`
is generated. Translating them would be 17,678 more words to keep honest for an
audience that does not need them.

Each translation carries the hash of the English file it was made from, and CI
fails when the English moves ahead - see [`scripts/i18n-check.sh`](scripts/i18n-check.sh).
A translation that cannot be brought back up to date is **deleted**, not left in
place: a stale page still looks authoritative and hands out last month's
commands, which is worse for the reader than being sent to English.

A language is one directory. `docs/i18n/<lang>/` mirrors the repo root, so
`docs/i18n/de/docs/guide/install.md` is the German `docs/guide/install.md` - which
is what makes a link between two translated pages the SAME string as the link
between their English originals, and what makes retiring a language one `rm -r`.

## Supported games

**Skyrim SE/AE** - proven in real play. **Fallout 4** is wired end to end too
(F4SE swapped in automatically, archive invalidation, asterisk load order, LOOT,
`.fos` saves) - see [guide/fallout4.md](docs/guide/fallout4.md). Wired per the shared game descriptor and
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
