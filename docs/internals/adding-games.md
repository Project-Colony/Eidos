# Adding a game

Eidos knows a game from a small declarative descriptor (modelled on Mod Organizer
2's `IPluginGame` schema). There are two ways to add one.

## 1. A built-in game (ships with Eidos)

Add a row to `GAMES` in [`crates/eidos-gamedef/src/lib.rs`](../crates/eidos-gamedef/src/lib.rs)
and recompile. This is for the games Eidos supports out of the box: the
Bethesda/Creation family, plus Stellar Blade as the first Unreal title and the
worked example of a game declaring its own mod vocabulary.

## 2. A user game (no recompile, like MO2's `basic_games`)

Drop a `<id>.toml` file into:

```
~/.config/Colony/Eidos/games/
```

It joins the registry on the next launch. A file whose `id` matches a built-in
game overrides that built-in; any other `id` is added.

### Fields

| Field | Required | Default | Meaning |
|-------|----------|---------|---------|
| `id` | yes | - | Eidos game id (lowercase, e.g. `stardew`) |
| `name` | yes | - | Display name |
| `steam_app_id` | yes | - | Steam app id (how Eidos detects the install) |
| `data_dir` | yes | - | Mod-merge root, relative to the install dir (`Data`, `Mods`, `.` for the game root) |
| `valid_folders` | no | Gamebryo set | Top-level folder names that mark a mod root inside an archive |
| `valid_suffixes` | no | Gamebryo set | File extensions (no dot) that mark a mod root |
| `short_name` | no | `""` | MO2-style short name (e.g. `SkyrimSE`) |
| `nexus_game` | no | `""` | Nexus domain (for downloads/updates) |
| `documents_dir` | no | `""` | `My Games/<dir>` folder, if the game keeps per-profile INIs there |
| `ini_files` | no | `[]` | Per-profile INIs; the first carries the `[Archive]` section |
| `load_order` | no | `"None"` | `Asterisk`, `PlainList`, `FileTime`, or `None` |
| `primary_plugins` | no | `[]` | Implicit master plugins (omitted from `plugins.txt`) |
| `script_extender` | no | none | `{ launcher = "...", loader = "..." }` |

### Telling Eidos what a mod looks like

`valid_folders` and `valid_suffixes` are the per-game half of MO2's
`ModDataChecker`. When Eidos opens a downloaded archive it walks down through
wrapper folders (`ModName-1234/...`) asking, at each level, "is this a mod root?"
Only these two lists answer that question.

Leave them unset and the game uses the Gamebryo vocabulary (`textures`, `meshes`,
`scripts`, `SKSE`, ... plus `.esp`/`.esm`/`.esl`/`.bsa`/`.ba2`), which is what
every built-in game does. **An empty list means "use that default", not "nothing is
a mod root"** - the latter would send every install to the manual picker.

A game whose mods do not look like Bethesda mods must say so, or its archives will
never resolve automatically:

```toml
# A Unity game loaded by BepInEx.
valid_folders = ["BepInEx", "plugins", "patchers", "config"]
valid_suffixes = ["dll"]
```

**Declaring either list replaces the vocabulary as a whole**, including the list
you did not name. That is deliberate: Stellar Blade's mods are `.pak` files and
nothing else, and inheriting the Gamebryo folders would let an archive shipping a
`textures/` folder read as a valid mod root and install to the wrong place. A game
with no folder vocabulary declares only `valid_suffixes` and gets exactly that.

Matching ignores case, so spell the names the way the game's own documentation
does.

### Generic (non-Bethesda) game

Most non-Bethesda games mod by simply merging files over a directory. Leave every
Bethesda-specific field unset: `load_order` defaults to `None`, so Eidos runs only
its file union over `data_dir` - no `plugins.txt`, no BSA invalidation, no INI
deployment.

```toml
# ~/.config/Colony/Eidos/games/stardew.toml
id = "stardew"
name = "Stardew Valley"
short_name = "StardewValley"
nexus_game = "stardewvalley"
steam_app_id = 413150
data_dir = "Mods"
```

### Bethesda-style game

For a Creation/Gamebryo game, fill in the load-order mechanism, INIs and masters:

```toml
id = "skyrimse-gog"
name = "Skyrim Special Edition (GOG)"
short_name = "SkyrimSE"
nexus_game = "skyrimspecialedition"
steam_app_id = 489830
data_dir = "Data"
documents_dir = "Skyrim Special Edition"
ini_files = ["Skyrim.ini", "SkyrimPrefs.ini", "SkyrimCustom.ini"]
load_order = "Asterisk"
primary_plugins = ["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"]
script_extender = { launcher = "SkyrimSELauncher.exe", loader = "skse64_loader.exe" }
```

An invalid file (missing a required field, bad TOML) is ignored with a warning on
stderr; the rest still load.

## Limits

- Detection is by Steam app id, so a non-Steam install isn't auto-detected yet.
- A game whose modding needs bespoke logic (its own load order, a custom mod
  format - e.g. Cyberpunk REDmod, BG3) needs code in `eidos-gamefeatures`, not just
  a descriptor. The descriptor covers detection + the file union + the Bethesda
  family; the rest is per-game code as in MO2's full game plugins.
