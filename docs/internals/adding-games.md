# Adding a game

Eidos knows a game from a small declarative descriptor (modelled on Mod Organizer
2's `IPluginGame` schema). There are two ways to add one.

## 1. A built-in game (ships with Eidos)

Add a `<id>.toml` to [`crates/eidos-gamedef/data/games/`](../../crates/eidos-gamedef/data/games/),
list it in `BUILTIN_DEFINITIONS` in
[`crates/eidos-gamedef/src/lib.rs`](../../crates/eidos-gamedef/src/lib.rs), and
recompile. The file uses exactly the user-game schema below and is embedded in
the binary. The snapshot test in that crate names every built-in game, so it fails
until you add the new one there too. This is for the games Eidos supports out of
the box, 15 files today: the Bethesda/Creation family; Stellar Blade, the first
Unreal title and the worked example of a game declaring its own mod vocabulary;
the folder-per-mod games (Stardew Valley, 7 Days to Die); and Crimson Desert.
Crimson Desert's `data_dir` is `bin64`, and its descriptor covers code mods
only (`.asi`, `.addon64`, `.dll`, `reshade-shaders/`): its data mods would need
a PAZ repack, which Eidos does not do.

## 2. A user game (no recompile, like MO2's `basic_games`)

Drop a `<id>.toml` file into:

```
~/.config/Colony/Eidos/games/
```

It joins the registry on the next launch. A file whose `id` matches a built-in
game replaces that built-in in place; any other `id` is added after the
built-ins. Detection takes the first definition that carries a given Steam, GOG
or Epic id, so a new `id` that reuses a built-in game's store id is never
detected. To change a built-in game, use its `id`.

### Fields

| Field | Required | Default | Meaning |
|-------|----------|---------|---------|
| `id` | yes | - | Eidos game id (lowercase, e.g. `stardewvalley`) |
| `name` | yes | - | Display name |
| `steam_app_id` | no | `0` | Steam app id. `0` means not a Steam game: Steam detection skips the definition, and `gog_ids` or `epic_ids` can detect it instead. Run hands a Steam copy to `steam -applaunch <steam_app_id>` |
| `data_dir` | yes | - | Mod-merge root, relative to the install dir (`Data`, `Mods`, `.` for the game root) |
| `valid_folders` | no | Gamebryo set (see below) | Top-level folder names that mark a mod root inside an archive |
| `valid_suffixes` | no | Gamebryo set (see below) | File extensions (no dot) that mark a mod root |
| `valid_files` | no | `[]` | Exact file names (any case) that mark a mod root, e.g. `manifest.json` |
| `mod_unit` | no | `"files"` | `files` for loose files, `folder` to keep each mod's named folder under `data_dir` |
| `gog_ids` | no | `[]` | GOG product ids accepted from installed manifests |
| `epic_ids` | no | `[]` | Epic/Legendary app names accepted from installed manifests |
| `short_name` | no | `""` | MO2-style short name (e.g. `SkyrimSE`), written as a mod's `gameName` when the mod comes from this game's domain |
| `nexus_game` | no | `""` | Nexus domain. A mod whose `meta.ini` names its own `gameName` uses that game's domain for update checks, endorsements, tracking and nxm routing; this domain is the fallback |
| `nexus_sources` | no | `[]` | Other Nexus domains whose mods this game accepts (nxm links, the "another game" flag). Enderal SE takes `skyrimspecialedition` |
| `documents_dir` | no | `""` | `My Games/<dir>` folder, if the game keeps per-profile INIs there |
| `ini_files` | no | `[]` | Per-profile INIs; the first carries the `[Archive]` section |
| `load_order` | no | `"None"` | `Asterisk`, `PlainList`, `FileTime`, or `None` |
| `primary_plugins` | no | `[]` | Implicit master plugins (omitted from `plugins.txt`) |
| `game_binary` | no | `""` | The game's executable in the install dir (a subfolder is allowed, `bin64/CrimsonDesert.exe`), offered as a default tool. Its version resource is the game build Health and session records use, and a game copy must contain it |
| `registry_name` | no | `""` | The game's key under `HKLM\Software\Bethesda Softworks\` in the Wine registry |
| `script_extender` | no | none | `{ launcher = "...", loader = "..." }`. A run the store starts with the launcher runs the loader instead, when the merged root has one |
| `known_tools` | no | `[]` | `[[known_tools]]` entries, `exe` + `title`: executables Eidos recognises wherever it finds them |

The script-extender loader, the launcher and `game_binary` are the game's own
entries. Run, in a window no store opened, never starts them itself: it hands
the game to the store that owns the copy
([architecture.md](architecture.md#starting-the-game-through-its-store)).

### Tools the game has

Eidos looks for these by FILE NAME, in the game folder (the game copy when the
instance runs one), in the enabled mods' `Root/` trees, in the instance's
`mods/` folder up to four levels down, and in the shared tools directory from
Preferences, and lists any it finds ready to run.
Declare the ones a Bethesda-shaped game would never have:

```toml
[[known_tools]]
exe = "WolvenKit.exe"
title = "WolvenKit"
```

A descriptor that overrides a built-in game and declares none inherits that
game's list, so overriding `fallout4` to change one field does not cost you
FO4Edit. Every other field comes from the override file alone: copy the
built-in file and change what you need.

The `title` also decides what a found tool starts with, so keep the usual
name. A title containing `Synthesis`, `Pandora`, `FNIS`, `BodySlide`,
`Outfit`, `DynDOLOD`, `TexGen`, `xLODGen`, `Cathedral`, `CAO`, `Nemesis` or
`LOOT` (any case) gets its runtime prerequisites
(`eidos_instance::default_prereqs`), and Pandora, Nemesis, FNIS,
BodySlide and Synthesis get a capture mod such as `Pandora Output`
(`default_output_mod`). A title containing `BethINI` makes Eidos keep a restore
point of the profile INIs before each run. A `prereqs=` or `output_mod=` line
in `tools.ini` wins over these defaults.

### Telling Eidos what a mod looks like

`valid_folders`, `valid_suffixes` and `valid_files` are the per-game half of
MO2's `ModDataChecker`. When Eidos opens a downloaded archive it walks down
through wrapper folders (`ModName-1234/...`) asking, at each level, "is this a
mod root?" Only these three marker lists answer that question.

With all three empty and `mod_unit = "files"`, the game uses the Gamebryo
vocabulary (`textures`, `meshes`, `scripts`, `SKSE`, ... plus
`.esp`/`.esm`/`.esl`/`.bsa`/`.ba2`), which is what the Bethesda games use. An
empty list means "use that default", not "nothing is a mod root": the latter
would send every install to the manual picker.

A game whose mods do not look like Bethesda mods must say so, or its archives will
never resolve automatically:

```toml
# A Unity game loaded by BepInEx.
valid_folders = ["BepInEx", "plugins", "patchers", "config"]
valid_suffixes = ["dll"]
```

Declaring any one of the three lists replaces the vocabulary as a whole,
including the lists you did not name. Stellar Blade's mods are `.pak` files and
nothing else, and inheriting the Gamebryo folders would let an archive shipping
a `textures/` folder read as a valid mod root and install to the wrong place.
So its descriptor declares only `valid_suffixes` (`pak`, `utoc`, `ucas`) and
gets exactly that. The other built-ins that declare their own vocabulary are
Crimson Desert (`asi`, `addon64` and `dll`, plus the `reshade-shaders` folder),
Stardew Valley (`manifest.json`) and 7 Days to Die (`ModInfo.xml`).

`mod_unit = "folder"` keeps the folder that holds the markers, under
`data_dir`. Stardew Valley and 7 Days to Die use it: a SMAPI archive's
`ContentPatcher/manifest.json` installs as `Mods/ContentPatcher/`, the folder
SMAPI finds the mod by.

Matching ignores case, so spell the names the way the game's own documentation
does.

### Generic (non-Bethesda) game

Most non-Bethesda games mod by merging files over a directory. Leave every
Bethesda-specific field unset: `load_order` defaults to `None`, so Eidos runs only
its file union over `data_dir`, with no `plugins.txt`, no BSA invalidation and
no INI deployment.

This file has the built-in Stardew Valley's `id`, so it replaces that
definition; a game no built-in covers takes the same fields under a new `id`:

```toml
# ~/.config/Colony/Eidos/games/stardewvalley.toml
id = "stardewvalley"
name = "Stardew Valley"
short_name = "stardewvalley"
nexus_game = "stardewvalley"
steam_app_id = 413150
gog_ids = ["1453375253"]
data_dir = "Mods"
valid_files = ["manifest.json"]
mod_unit = "folder"
game_binary = "Stardew Valley.exe"
script_extender = { launcher = "Stardew Valley.exe", loader = "StardewModdingAPI.exe" }
```

### Bethesda-style game

For a Creation/Gamebryo game, fill in the load-order mechanism, INIs and
masters. The plugin system (`plugins.txt` and the per-profile INIs) runs only
for the ids Eidos maps to an esplugin game: `skyrimse`, `skyrimvr`,
`enderalse`, `skyrim`, `fallout4`, `fallout4vr`, `falloutnv`, `fallout3`,
`morrowind`, `oblivion` and `starfield`. A Bethesda-style descriptor therefore
works only when it overrides one of those ids; under any other id the game gets
the file union only. This one overrides the built-in Skyrim SE, whose
`gog_ids` and `epic_ids` already detect the GOG and Epic copies:

```toml
id = "skyrimse"
name = "Skyrim Special Edition"
short_name = "SkyrimSE"
nexus_game = "skyrimspecialedition"
steam_app_id = 489830
gog_ids = ["1162721350", "1711230643"]
epic_ids = ["ac82db5035584c7f8a2c548d98c86b2c", "5d600e4f59974aeba0259c7734134e27"]
data_dir = "Data"
documents_dir = "Skyrim Special Edition"
ini_files = ["Skyrim.ini", "SkyrimPrefs.ini", "SkyrimCustom.ini"]
load_order = "Asterisk"
primary_plugins = ["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"]
game_binary = "SkyrimSE.exe"
registry_name = "Skyrim Special Edition"
script_extender = { launcher = "SkyrimSELauncher.exe", loader = "skse64_loader.exe" }
```

An invalid file (missing a required field, bad TOML) is ignored with a warning on
stderr; the rest still load.

## Limits

- Detection covers Steam (`steam_app_id`), Heroic's GOG and Epic installs, and
  the bare Legendary CLI's installs (`gog_ids`, `epic_ids`). The built-ins
  `skyrimse`, `falloutnv`, `fallout3` and `stardewvalley` carry such ids.
  Copies installed through any other launcher are not detected, and
  `eidos init --game-path` only chooses among detected copies.
- A game copy (a list's `Stock Game`, or `eidos stock-game <instance> freeze`)
  works for any descriptor whose `game_binary` the copy holds, except a game
  that keeps its plugin list in the game folder: Morrowind, and Oblivion with
  `bUseMyGamesDirectory=0`.
- A game whose modding needs bespoke logic (its own load order, a custom mod
  format, e.g. Cyberpunk REDmod, BG3) needs code in `eidos-gamefeatures`; a
  descriptor alone is not enough. The descriptor covers detection, the file union and the
  Bethesda family; the rest is per-game code, as in MO2's full game plugins.
