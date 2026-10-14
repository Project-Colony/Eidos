<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/png/eidos-logo-512.png">
  <img src="assets/brand/png/eidos-logo-light-1024.png" alt="Eidos" width="360">
</picture>

**The native Linux mod manager that never touches your game.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-system-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux-lightgrey)](#installation)

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

### How it compares

| | Eidos | MO2 via Wine | Fluorine-Manager | Limo / link deployers |
|---|---|---|---|---|
| Manager runs natively | ✅ | ❌ Windows app in Wine | ✅ (Qt port) | ✅ |
| Game dir untouched | ✅ always | ✅ | ✅ | ❌ links written into it |
| Mount visible to | the game only | the game only | **the whole system** | n/a |
| Crash cleanup needed | none, by design | none | stale-mount recovery | manual un-deploy |
| Root mods (ENB, preloaders) | ✅ native | plugin required | plugin required | partial |
| Privileges required | none | none | `/etc/fuse.conf` edit | none |

### How fast it is

| | before | now |
|---|---|---|
| loading a save | ~20 seconds | **6-7 seconds** |
| directory reads in one session | 5.6 million | 465 thousand |

Cell changes are immediate. The gain came from asking your mods fewer questions:
finding one file used to interrogate all fifty of them in turn, and listing one
folder used to do it fifty times over. Neither does any more. Measured on a real
instance played normally, not on a benchmark.

### Prior art and thanks

- [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) and
  [usvfs](https://github.com/ModOrganizer2/usvfs) - the semantics Eidos
  reproduces, and the codebase its parity was studied against
- [LOOT](https://loot.github.io/) - the sorting engine, via libloot
- [Fluorine-Manager](https://github.com/SulfurNitride/Fluorine-Manager),
  [Limo](https://github.com/limo-app/limo) and the other Linux managers -
  proof there is a community that wants this solved

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

**Instances.** Global - managed centrally under
`~/.local/share/Colony/Eidos/instances/` - or
portable: a self-contained folder anywhere you want (a second drive, a games
partition), movable and isolated, like MO2's. Portable instances are remembered
across sessions; the GUI, the Steam launch and every CLI command follow the one
you last used, and any command takes the folder wherever it takes a game id.
Details in [usage.md](docs/guide/usage.md#instances-global-and-portable).

**Profiles.** Per-profile mod order, plugin state, INIs and saves. Saves are
parsed, diffed against your current plugins - with a button that enables what a
save needs - and synced back for Steam Cloud after every session.

**Nexus.** Connect an account to receive Mod Manager Download links, check
updates and see authors and their profiles. Collections install from their
recipes with resumable progress and explicit reports for unsupported steps.
Matching checks exact source files; incomplete older metadata remains
unverified. The Downloads tab filters, sorts, hides and removes archives.
An **offline** switch stops Nexus access.

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

**Where it keeps its own files.** The layout every program in the Colony family
uses: `~/.config/Colony/Eidos/` for what you chose - preferences, your Nexus
session, your instance list, the game and add-on definitions you wrote -
`~/.local/share/Colony/Eidos/` for global instances, downloaded runtimes and
session logs, and `~/.cache/Colony/Eidos/` for LOOT masterlists. An older Eidos
kept these in `~/.config/eidos/`, `~/.local/share/eidos/` and `~/.local/state/`;
the first launch after upgrading carries them across, says so in the log, and
leaves the old directories exactly as they were. Until a move has completed,
Eidos keeps using the old files. Details in
[internals/paths.md](docs/internals/paths.md).

See [Integrity checks and generated output](docs/internals/integrity.md) for
replacement protection, BSA/BA2 member conflicts, static SKSE diagnostics and
generator receipts, including their limits.

### Supported games

**Skyrim SE/AE** - proven in real play. **Fallout 4** is wired end to end too
(F4SE swapped in automatically, archive invalidation, asterisk load order, LOOT,
`.fos` saves) - see [guide/fallout4.md](docs/guide/fallout4.md). Wired per the shared game descriptor and
looking for testers: Skyrim LE, Skyrim VR, Enderal SE, Fallout 3, Fallout NV,
Fallout 4 (+ VR), Starfield, Oblivion and Morrowind (the last two mount and
manage mods; their timestamp-ordered plugin lists are not managed yet).

Adding a family is one descriptor row:
[internals/adding-games.md](docs/internals/adding-games.md).

## Installation

### Via Colony (recommended)

Search for **Eidos** in [Colony](https://github.com/Project-Colony/Colony) and
install it. Updates arrive through the launcher. Colony installs one file,
`eidos-linux`: the window, and also the `eidos` command line when it is called
through a link named `eidos`. The NIF model preview needs a helper that only the
bundle below carries; without it the preview says so and everything else works.
What to do after installing, and how to point Steam at it:
[docs/guide/install.md](docs/guide/install.md#via-colony).

### Direct binary download

Grab an asset from the [latest release](../../releases/latest):

| Asset | What it is |
|---|---|
| `eidos-linux` | the whole program in one file, as Colony installs it |
| `eidos-linux-bundle.tar.gz` | `eidos`, `eidos-gui`, the NIF preview helper, icons and an installer that registers the desktop entry and the Nexus download handler |

Check the download first, as [Code signing policy](#code-signing-policy) shows,
then:

```bash
tar xzf eidos-linux-bundle.tar.gz
cd eidos-linux-bundle
./install.sh
```

The installer puts everything in `~/.local/bin` without sudo; `--system` and
`--bindir DIR` choose elsewhere. Both assets need x86_64 Linux with glibc 2.39
or newer, FUSE (`fusermount3`) and 7-Zip for mod archives.

### Build from source

```bash
git clone https://github.com/Project-Colony/Eidos && cd Eidos
just build
just install
```

Requires Rust 1.89 or newer (the `rust-version` CI checks; CI and releases
build with 1.99.0), plus `just`, CMake 3.20+ and a C++17 compiler for the NIF
preview helper. An Arch
package builds from `packaging/PKGBUILD`. Prerequisites and the commands without
`just`: [docs/guide/install.md](docs/guide/install.md).

### Steam launch options

Eidos runs *as* the game's launch command, which is how it mounts before the
game starts. The base line is all most setups need, with the absolute path of
your install (Steam does not read your `PATH`):

```
~/.local/bin/eidos-gui %command%
```

From Colony that path is `~/.local/share/Colony/apps/Eidos/eidos-linux`. Then
press Play: Eidos opens on that game's instance, and Run starts the game through
the merged view.

Everything else is environment variables stacked in front of it, and they
combine freely:

| You want... | Put in front |
|---|---|
| DLSS with Community Shaders | `PROTON_ENABLE_NVAPI=1` - without it DLSS silently never initialises; the full checklist is [guide/graphics.md](docs/guide/graphics.md) |
| an FPS counter on screen | `DXVK_HUD=fps` |
| driver-level frame interpolation, zero mods (RTX 40/50) | `NVPRESENT_ENABLE_SMOOTH_MOTION=1` - never together with Community Shaders' own frame generation |
| verbose logs for a bug report | `EIDOS_LOG=debug` (session logs land in `~/.local/share/Colony/Eidos/logs/`) |
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

## Documentation

Everything is in [docs/](docs/), indexed by who is reading at
[docs/README.md](docs/README.md): the user guide (installation, the CLI and the
GUI, tools such as xEdit and DynDOLOD, graphics, troubleshooting), how Eidos
works inside, and why it exists. Security policy and how to report a
vulnerability: [SECURITY.md](SECURITY.md).

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

## Code signing policy

Every release asset is signed by the Project Colony organisation in CI, never on
a developer machine. Next to each asset on the release page:

| File | What it is |
|---|---|
| `<asset>.sig` | an ed25519 signature over the asset, made with the organisation's release key |
| `<asset>.meta` | three lines binding the asset to its file name, its sha256 and the release version |
| `<asset>.meta.sig` | an ed25519 signature over the `.meta` |

The private key is an organisation secret, used only by the shared
[sign-and-publish workflow](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
in a job that builds nothing; the jobs that compile Eidos never see it.
Colony checks all three files before it installs or updates Eidos, and
refuses a release older than the one installed. To check a download yourself
with OpenSSL 3 (the same commands work for every asset):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=eidos-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

How releases are built, signed and published:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

## Privacy

Eidos sends no telemetry, no analytics and no crash reports. It connects to no
server except the ones in this table, and only for the action named there.
**Offline** mode (Preferences -> Nexus) stops every connection to Nexus.

| Data | Stored or sent | Where, and why |
|---|---|---|
| Preferences, instance list, game and add-on definitions | stored | `~/.config/Colony/Eidos/` |
| Instances, downloaded runtimes, session logs | stored | `~/.local/share/Colony/Eidos/`, or the folder you chose for a portable instance |
| LOOT masterlists | stored | `~/.cache/Colony/Eidos/loot/` |
| Nexus sign-in | sent | Signing in opens `users.nexusmods.com` in your browser, which hands the result back to a listener Eidos opens on `127.0.0.1` for that sign-in only. Eidos then trades it for tokens at `users.nexusmods.com/oauth/token` and fetches the public keys Nexus signs them with. [Nexus Mods privacy policy](https://help.nexusmods.com/article/20-privacy-policy) |
| Nexus credentials | stored | The access and refresh tokens, in `~/.config/Colony/Eidos/nexus.ini` with mode 0600. An `api_key=` line written there by an older Eidos is kept but never read; delete it if you like. |
| Nexus requests | sent | `api.nexusmods.com`, when you use a Nexus feature: your account name and premium status, mod and file details, update checks, collections and download links. Every request carries your access token and names Eidos and its version. |
| Nexus downloads | sent | The files you ask for, from the Nexus CDN host the download link names, saved in the instance's downloads folder with an MO2-format `.meta` beside each one. |
| `nxm://` links | stored | `eidos nxm --register`, which `install.sh` runs for you, writes `~/.local/share/applications/eidos-nxm.desktop` and makes it the `nxm://` handler through `xdg-mime`, so the site's Mod Manager Download button hands its link to Eidos. The Arch package installs the same entry system-wide. |
| LOOT masterlists | sent | `raw.githubusercontent.com/loot/...`, on a game's first LOOT sort and whenever you ask for an update. Nothing of yours is sent. [GitHub privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) |
| Tool runtimes | sent | Only when you install a tool prerequisite or a mod needs one: the .NET runtime from `builds.dotnet.microsoft.com`, refused unless it matches the SHA-256 built into Eidos, and `winetricks`, which downloads what it installs into the game's Proton prefix (Visual C++ and .NET redistributables) from Microsoft and the other hosts winetricks uses. [Microsoft privacy statement](https://www.microsoft.com/en-us/privacy/privacystatement) |
| Session logs | stored | `~/.local/share/Colony/Eidos/logs/`, mode 0600: ten sessions per instance, swept after 30 days. Your home folder is written as `~` and your user name as `<user>`, so a log can be pasted into an issue as it is. They never leave your machine unless you send them. |
| Shortcuts | stored | `install.sh` writes `eidos.desktop` and its icons under `~/.local/share`; a tool shortcut you create goes to `~/.local/share/applications`. |
| Links you open | neither | Mod pages and other links open in your own browser through `xdg-open`. |
| Add-ons | neither | Extensions you install run as their own programs on your machine; what they do is theirs. |

## License

Eidos's original code is GPL-3.0-or-later; see [LICENSE](LICENSE). These parts
are GPL-3.0 without the "or later" option:

- the OMOD script translations in `eidos-install`; see their
  [upstream credits](crates/eidos-install/src/install/omod/known_handlers/UPSTREAM.md);
- the `eidos-nif-preview` helper, built on the vendored nifly library; see its
  [component licenses and credits](crates/eidos-nif-preview/UPSTREAM.md);
- the `esplugin` and `libloadorder` crates the load-order code links, which
  declare `GPL-3.0`.

The other Rust crates in the binaries keep their own licenses (MIT, Apache-2.0,
BSD and similar). The release bundle carries their notices in `licenses/crates/`,
with an index of every crate and its license.

The binaries also embed eight genuine Microsoft DirectX redistributable DLLs
(`d3dcompiler_47`, `d3dcompiler_43`, `d3dx9_43` and `d3dx11_43`, 32- and 64-bit),
which Eidos deploys into the game's prefix when a mod or tool needs them. They
stay under Microsoft's redistributable terms; see their
[provenance](crates/eidos-gamefeatures/assets/PROVENANCE.md).

Mod management belongs to everyone.
