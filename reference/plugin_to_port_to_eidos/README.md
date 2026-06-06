# MO2 plugins - porting reference

Copied from Mod Organizer 2's `plugins/` folder as a **reference for porting
game support to Eidos**. This is NOT Eidos code - it is upstream MO2 material
kept here so we can study how MO2 describes games and tools.

- **Source:** [ModOrganizer2/modorganizer](https://github.com/ModOrganizer2/modorganizer)
  (and [ModOrganizer2/modorganizer-basic-games](https://github.com/ModOrganizer2/modorganizer-basic-games)
  for `basic_games`).
- **License:** GPL-3.0 (same as Eidos), (c) the Mod Organizer 2 Team.

## What's here (the portable, useful parts)

- **`basic_games/`** - the gold mine. 61 game definitions in Python
  (`basic_games/games/*.py`) plus the `BasicGame` base class (`basic_game.py`)
  and store helpers (Steam/GOG/Epic/EA/Origin). This is the cleanest reference
  for what Eidos needs to know per game: executables, data dir, documents dir,
  Steam app id, launcher name, etc. See `basic_game.py` -> `executables()`,
  `dataDirectory()`, `documentsDirectory()`, `gameDirectory()`.
- **Root `*.py`** - tool plugins (FNISTool, ScriptExtenderPluginChecker,
  Form43Checker, DDSPreview, pyCfg...) - examples of MO2's tool/diagnostic plugins.

## What was EXCLUDED (and why)

- **`plugin_python/`** (~35 MB) - the embedded CPython runtime. Pure interpreter,
  nothing to port.
- **Compiled C++ game plugins** (`game_*.dll`) - binaries, not source. The major
  games (incl. Skyrim SE) use these dedicated C++ plugins, NOT `basic_games`.
  Their source lives in the MO2 repos. Games covered by these DLLs:
  `enderal, enderalse, fallout3, fallout4, fallout4vr, fallout76, falloutNV,
  morrowind, nehrim, oblivion, skyrim, skyrimse, skyrimvr, starfield, ttw`.
- Compiled installer/preview plugins (`installer_*.dll`, `preview_*.dll`, etc.) -
  binaries.

## Why this matters for Eidos

Eidos already identifies a game from its install dir (`eidos-gui::identify_game`)
and models an instance (`eidos-instance`). When we want to support many games
without hand-rolling each, this `basic_games` schema (a small per-game data file:
name, binary, data path, documents path, steam id) is the model to port - one
Eidos game-definition format derived from these 61 examples.
