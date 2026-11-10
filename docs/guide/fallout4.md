# Fallout 4 through Eidos

With Eidos, Fallout 4 needs no special launch option, no renamed executable
and no wrapper script. It takes the same launch option as every other game.

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

or to copy `f4se_loader.exe` over `Fallout4Launcher.exe`, which Steam restores
on every game update - after which you are playing without F4SE and nothing
says so.

Eidos does the swap itself, from the game descriptor. When the window starts
the game, it replaces `Fallout4Launcher.exe` with `f4se_loader.exe` if the
loader is in the game folder, an enabled mod's `Root/` or `Overwrite/Root/`.
Without the loader it runs `Fallout4.exe`, and the status line says
`f4se_loader.exe is not installed - launching Fallout4.exe directly, so script-extender mods will not load.`
When neither file is found there, it runs the launcher unchanged and says that
too. With `eidos play fallout4 -- %command%` as the launch option instead, the
CLI swaps only when the loader exists, and otherwise runs
`Fallout4Launcher.exe`.

There is a second reason to avoid the launcher: it rescans `Data` and rewrites
`plugins.txt`, undoing the load order that was just deployed.

When you open the window yourself and press Run, Eidos asks Steam to start the
game (`steam -applaunch 377160`), and Steam starts Eidos through the launch
option above. If `f4se_loader.exe` is not next to the game's executable in the
instance, it does not, and the status line asks you to install the script
extender first. Fallout 4 VR works the same way, with `Fallout4VRLauncher.exe`
and `f4sevr_loader.exe`.

## What Eidos handles for you

| Area | What Eidos does |
|---|---|
| Archive invalidation | `Fallout4Custom.ini` gets `[Archive]` `bInvalidateOlderFiles=1` and an empty `sResourceDataDirsFinal=`, the two keys that let loose files outside `Data` be seen at all. Written into the profile, not the game folder. |
| INIs | `Fallout4.ini`, `Fallout4Prefs.ini` and `Fallout4Custom.ini` belong to the profile. BethINI Pie run through Eidos edits the profile's copies, and Backups -> INIs puts back the copy Eidos keeps before each of its runs ([tools.md](tools.md#bethini-pie)). |
| Load order | `plugins.txt` in the asterisk format Fallout 4 uses (`*` marks active), with `Fallout4.ccc` honoured for the implicit Creation Club plugins |
| LOOT | Sorting works the same as for Skyrim - `eidos sort <instance>` fetches the `fallout4` masterlist |
| Saves | `.fos` saves and their `.f4se` cosaves are listed, copied and kept per profile; the detail pane reads the save's own plugin table, so a save that needs a plugin you have disabled says so before you load it. Disabling or removing a mod whose plugin one of the profile's three newest saves uses asks first. |
| Root mods | Anything a mod ships beside the executable (F4SE itself, ENB, a `dxvk.conf`) lands there through the same `Root/` mechanism Skyrim uses |

## The version question

Fallout 4 is not the frozen game it was between 2019 and 2024. As of August 2026
there are three live branches, and a mod DLL built for one will not load on
another:

| Branch | Version | F4SE |
|---|---|---|
| Classic ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 to 1.11.240 | 0.7.4 to 0.7.9 |

Two consequences to know before building a mod list:

- `Creations/` and `Mods/` folders in the game root mean you are on the 1.11.x
  line. A save's detail pane in Eidos also shows the build it was written by:
  Fallout writes that into the save, and Eidos shows it as "Game build".
- F4SE usually ships within a day or two of a Bethesda update, but *Address
  Library for F4SE Plugins* - which most DLL mods resolve their offsets
  through - follows on its own schedule. Between the two, DLL mods do not
  load. Mods without a DLL (textures, meshes, plugins) are unaffected, so a
  fresh patch is a bad day to start a mod list.

Once your stack works, freeze the game version into the instance: Freeze this
version on the Game files card of the Diagnostics tab, or
`eidos stock-game fallout4 freeze` to see the size and
`eidos stock-game fallout4 freeze --yes` to copy. Eidos copies the store install
into `<instance>/Stock Game` and runs the game from that copy, still started by
Steam. A later patch changes the store install and not the copy, so the DLLs
you installed keep their game version. On btrfs or XFS the copy shares the
store's blocks and takes almost no space until Steam changes a file; on ext4 it
is a full copy as large as the game folder. Eidos refuses while Steam reports
the game busy or not fully installed. The details are in
[usage.md](usage.md#game-files-the-store-install-or-a-copy).

Without a copy, turn Steam's automatic updates off for Fallout 4 (Properties,
Updates, "Only update this game when I launch it"), or the next patch will
break every DLL you installed.

## Hardware note: weapon debris crashes on NVIDIA

Fallout 4's weapon-debris effect runs on NVIDIA FleX, a PhysX derivative NVIDIA
stopped supporting after the Pascal generation. On any Turing or newer card -
GTX 16, RTX 20 through RTX 50 - it crashes the game. This is a game bug, nothing
to do with Linux, Proton or Eidos.

Two fixes, either works: turn "Weapon Debris" off in the game's settings, or
install *Weapon Debris Crash Fix* (Nexus 48078), which disables the fragments'
collision rather than the effect.

## Health cards for Fallout 4

The Diagnostics tab shows a Health card for each of these:

- F4SE plugins are installed in `F4SE/Plugins`, but `f4se_loader.exe` is in no
  enabled mod and not in the game folder. The game starts without them.
- `f4se_loader.exe` is installed, but the `f4se_<version>.dll` for the running
  game version (`f4se_1_10_984.dll` for 1.10.984) is not. The loader stops with
  an error; this usually follows a game update.
- The game executable or its Steam build changed since the last launch through
  Eidos that exited without an error. F4SE plugins built for the old version
  may stop loading.
- A mod with a higher priority than F4SE has loose `.pex` copies of F4SE's own
  scripts. An old copy breaks the F4SE functions it declares, and the game
  shows no error.
- The profile's `Fallout4Custom.ini` has `sTestFile1` to `sTestFile10` lines in
  `[General]`. While one is set, the game loads those plugins and ignores
  `plugins.txt`. Fix removes them and keeps the previous file as
  `Fallout4Custom.ini.pre-fix`.
- A mod's BA2 archive stores `.wav` or `.xwm` audio compressed. The game can
  crash when it plays that sound. Repack the archive with audio uncompressed.
  Fallout 4 VR gets the same card.
- A crash logger wrote a log during the last launch: Buffout 4's
  `crash-*.log` in `Documents/My Games/Fallout4/F4SE`, or a `.log` in
  `F4SE/Crashlogs`. The card names the exception and the DLLs and plugins in
  the log that your mods install.
- The Display card shows the window size and mode set in the profile's
  `Fallout4Prefs.ini`, and Apply writes `iSize W`, `iSize H`, `bFull Screen`
  and `bBorderless` there.
- A Nexus requirement names a DLC. Eidos looks for a plugin with the DLC's
  name, and Fallout 4's DLC plugins have other names (`DLCCoast.esm` for Far
  Harbor), so this card is advice: check by hand, then press Ignore.

The two loader cards have an Open the download page button for
<https://f4se.silverlock.org/> and an Install the downloaded archive button,
which puts the loader in the mod's `Root/`. `eidos check fallout4` prints the
same findings except the `.pex` card, the Display card and the Nexus
requirement cards, which only the window computes.

## If something looks wrong

The general checklist is in [troubleshooting.md](troubleshooting.md); the
Fallout-specific first question is always *which executable actually started*.
When the window starts the game, it writes the full launch command into the
instance's run log, so:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

If it names `f4se_loader.exe`, the swap happened. If it names `Fallout4.exe`,
F4SE is not installed where Eidos can find it - it belongs beside the game
executable, which for a mod-managed setup means a mod's `Root/` directory (or
the game folder itself, installed by hand). If it names `Fallout4Launcher.exe`,
Eidos found neither the loader nor `Fallout4.exe` in the root view.

When the right executable starts and the game still crashes, Test run and Find
the mod behind a problem narrow it down to one mod without touching the
profile's saves ([usage.md](usage.md#test-runs)).
