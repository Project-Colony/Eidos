# Fallout 4 through Eidos

Fallout 4 needs no special launch option, no renamed executable and no wrapper
script. That is worth saying plainly, because every other Linux guide for F4SE
tells you otherwise - and their advice breaks on the next Steam update.

## The launch option

```
~/.local/bin/eidos-gui %command%
```

Steam's launch target for Fallout 4 is `Fallout4Launcher.exe`, never
`Fallout4.exe`, so making the script extender run at all is really the question
"how do I make Steam start a different program". The usual answers are to
rewrite `%command%` in bash:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

or to copy `f4se_loader.exe` over `Fallout4Launcher.exe`, which Steam quietly
restores on every game update - after which you are playing without F4SE and
nothing says so.

Eidos does the swap itself, from the game descriptor: it replaces the launcher
with `f4se_loader.exe` when one is installed, falls back to `Fallout4.exe` when
one is not, and **tells you** when it had to fall back. A game that starts with
every F4SE mod inert is worse than a game that does not start.

There is a second reason never to run the launcher: it rescans `Data` and
rewrites `plugins.txt`, undoing the load order that was just deployed. Eidos
never executes it.

## What Eidos handles for you

| | |
|---|---|
| Archive invalidation | `Fallout4Custom.ini` gets `[Archive]` `bInvalidateOlderFiles=1` and an empty `sResourceDataDirsFinal=`, the two keys that let loose files outside `Data` be seen at all. Written into the profile, not the game folder. |
| Load order | `plugins.txt` in the asterisk format Fallout 4 uses (`*` marks active), with `Fallout4.ccc` honoured for the implicit Creation Club plugins |
| LOOT | Sorting works the same as for Skyrim - `eidos sort <instance>` fetches the `fallout4` masterlist |
| Saves | `.fos` saves and their `.f4se` cosaves are listed, copied and kept per profile; the detail pane reads the save's own plugin table, so a save that needs a plugin you have disabled says so before you load it |
| Root mods | Anything a mod ships beside the executable (F4SE itself, ENB, a `dxvk.conf`) lands there through the same `Root/` mechanism Skyrim uses |

## The version question

Fallout 4 is not the frozen game it was between 2019 and 2024. As of August 2026
there are three live branches, and a mod DLL built for one will not load on
another:

| Branch | Version | F4SE |
|---|---|---|
| Classic ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

Two consequences worth knowing before building a mod list:

- **Check what you actually have.** `Creations/` and `Mods/` folders in the game
  root mean you are on the 1.11.x line. A save's detail pane in Eidos also shows
  the build it was written by - Fallout writes that into the save, and Eidos
  surfaces it as "Game build".
- **A fresh patch is not a good day to start.** F4SE usually ships within a day
  or two of a Bethesda update, but *Address Library for F4SE Plugins* - which
  most DLL mods resolve their offsets through - follows on its own schedule.
  Between the two, the DLL half of the ecosystem is down. Mods without a DLL
  (textures, meshes, plugins) are unaffected.

Once your stack works, turn Steam's automatic updates off for Fallout 4
(Properties → Updates → "Only update this game when I launch it"), or the next
patch will break every DLL you installed.

## Hardware note: weapon debris crashes on NVIDIA

Fallout 4's weapon-debris effect runs on NVIDIA FleX, a PhysX derivative NVIDIA
stopped supporting after the Pascal generation. On any Turing or newer card -
GTX 16, RTX 20 through RTX 50 - it crashes the game. This is a game bug, nothing
to do with Linux, Proton or Eidos.

Two fixes, either works: turn "Weapon Debris" off in the game's settings, or
install *Weapon Debris Crash Fix* (Nexus 48078), which disables the fragments'
collision rather than the effect.

## If something looks wrong

The general checklist is in [troubleshooting.md](troubleshooting.md); the
Fallout-specific first question is always *which executable actually started*.
Eidos writes the full launch command into the instance's run log, so:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

If it names `f4se_loader.exe`, the swap happened. If it names
`Fallout4Launcher.exe`, F4SE is not installed where Eidos can find it - it
belongs beside the game executable, which for a mod-managed setup means a mod's
`Root/` directory (or the game folder itself, installed by hand).
