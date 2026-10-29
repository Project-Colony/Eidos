//! Declarative per-game definitions for the games Eidos manages, modelled on Mod
//! Organizer 2's `IPluginGame` schema (the same field set MO2's `basic_games`
//! plugin exposes declaratively): identity, the Steam app id, the data and
//! Documents directories, the per-profile INIs, the plugin load-order mechanism
//! and primary masters, and the script-extender loader.
//!
//! This is the single source of truth the other crates read from, so adding a
//! built-in game is one data file in `data/games/` (see [`builtin`]) instead of
//! edits scattered across detection, plugins, and game features.
//!
//! Games can also be added WITHOUT recompiling, the way MO2's `basic_games` plugin
//! lets simple games be declared as data: drop a `<id>.toml` into
//! `~/.config/Colony/Eidos/games/` and it joins the
//! registry on next launch (see [`all`]). A definition with `load_order = "None"`
//! and no `ini_files`/`primary_plugins` is a "generic" game - just the file union
//! over its data dir, no plugins.txt / BSA / INI machinery - which covers most
//! non-Bethesda games (Stardew, Unity/Unreal titles, ...).
//!
//! Not yet modelled (MO2 has them; add when Eidos needs them): DLC/Creation-Club
//! plugin lists, `SortMechanism` (LOOT/BOSS), game variants (GOTY editions), and
//! the forced-load library list (we derive that from mods at launch instead).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// How a game persists its plugin load order on disk (MO2's `LoadOrderMechanism`,
/// specialised to what Eidos actually writes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOrder {
    /// One `plugins.txt`, each active line prefixed `*`, masters first. Skyrim
    /// SE/VR, Fallout 4, Starfield, Enderal SE.
    Asterisk,
    /// `plugins.txt` (active set) + `loadorder.txt` (full order). Skyrim LE,
    /// Fallout 3, Fallout New Vegas.
    PlainList,
    /// Load order is the plugins' file modification time. Oblivion, Morrowind -
    /// not managed by Eidos's plugin system yet.
    FileTime,
    /// No plugin load order at all (MO2's `LoadOrderMechanism::None`): a generic,
    /// non-Bethesda game that mods purely by merging files over its data dir.
    None,
}

/// A game's script-extender launch swap. MO2 surfaces the loader as an executable
/// (plus a forced-load DLL); Eidos swaps the vanilla launcher for the loader in
/// the Steam launch command, so a modded game is launched the way it is played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptExtender {
    /// The vanilla launcher executable Steam would run (what to swap out).
    pub launcher: &'static str,
    /// The script-extender loader to run instead (SKSE/F4SE/...).
    pub loader: &'static str,
}

/// Whether installed payloads are loose files or named loader folders.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModUnit {
    #[default]
    Files,
    Folder,
}

/// One game Eidos can manage: every per-game knob in one place.
#[derive(Debug, Clone)]
pub struct GameDef {
    /// Eidos game id, e.g. `skyrimse`.
    pub id: &'static str,
    /// Display name, e.g. `Skyrim Special Edition`.
    pub name: &'static str,
    /// MO2's `gameShortName` (used in `meta.ini`/`.meta` gameName), e.g. `SkyrimSE`.
    pub short_name: &'static str,
    /// The Nexus Mods game domain (nxm:// host, API path), e.g.
    /// `skyrimspecialedition`. VR editions share their parent game's Nexus.
    pub nexus_game: &'static str,
    /// Steam application id, used to locate the install via the Steam library.
    pub steam_app_id: u32,
    /// Exact GOG product IDs accepted from installed manifests.
    pub gog_ids: &'static [&'static str],
    /// Exact Epic/Legendary app names accepted from installed manifests.
    pub epic_ids: &'static [&'static str],
    /// The data directory under the game install (`Data`, or `Data Files` for
    /// Morrowind).
    pub data_dir: &'static str,
    /// Top-level directory names that mark a level inside an archive as this
    /// game's mod root - MO2's per-game `ModDataChecker::possibleFolderNames`.
    ///
    /// All three marker lists empty with `mod_unit = "files"` means the
    /// Gamebryo vocabulary. Declaring any marker replaces the whole vocabulary.
    pub valid_folders: &'static [&'static str],
    /// File extensions, without the dot, that mark a level as this game's mod root
    /// (`esp`, `esm`, ... for Gamebryo; `pak` for Unreal; `archive` for Cyberpunk).
    /// Defaults with the other marker lists; see [`Self::valid_folders`].
    pub valid_suffixes: &'static [&'static str],
    /// Exact filenames that identify mod content, matched case-insensitively.
    pub valid_files: &'static [&'static str],
    /// Folder units keep the directory containing the markers under `data_dir`.
    pub mod_unit: ModUnit,
    /// The game's folder under the prefix `Documents/My Games` and
    /// `AppData/Local`, e.g. `Skyrim Special Edition`. Empty for games that keep
    /// their config in the install dir (Morrowind).
    pub documents_dir: &'static str,
    /// Per-profile user INIs, the `[Archive]`-carrying one first (used by BSA
    /// invalidation). Empty for games without My Games INIs.
    pub ini_files: &'static [&'static str],
    /// How the plugin load order is stored on disk.
    pub load_order: LoadOrder,
    /// The game's own master plugins, pinned to the top of the order.
    pub primary_plugins: &'static [&'static str],
    /// The game's main executable in the install dir, e.g. `SkyrimSE.exe`. Eidos
    /// auto-detects it (and the launcher + script extender) as a default tool when
    /// the file is present, mirroring MO2's game-plugin `executables()`. Empty if
    /// unknown.
    pub game_binary: &'static str,
    /// The game's own key under `HKLM\\Software\\Bethesda Softworks\\` in the Wine
    /// registry. Bethesda's spelling, which is NOT always `short_name` (e.g.
    /// `FalloutNV`, `Fallout 4 VR`). Empty when the game has no such key, which
    /// makes the registry writer a no-op.
    pub registry_name: &'static str,
    /// The script-extender launch swap, if known - the vanilla launcher and the
    /// loader (SKSE/F4SE/...) to run instead. `None` where the launcher is not
    /// known with confidence (add a row to enable it for a game).
    pub script_extender: Option<ScriptExtender>,
    /// Third-party tools worth finding for this game, as `(executable, title)`.
    ///
    /// The xEdit family above all: it is named per game - `SSEEdit.exe`,
    /// `FO4Edit.exe`, `TES5Edit.exe` - so a shared list would offer a Skyrim
    /// user Fallout's editor. Everything here is matched by FILE NAME wherever
    /// Eidos is told to look (see `eidos_instance::default_tools_in`), because a
    /// modder keeps these anywhere: in the game folder, inside a mod, or in one
    /// shared tools directory used by every instance.
    ///
    /// The runtime each one needs is NOT here - it is chosen from the title by
    /// `eidos_instance::default_prereqs`, so a tool the user adds by hand gets
    /// the same answer as one found here.
    pub known_tools: &'static [(&'static str, &'static str)],
}

/// The built-in game descriptors, in the order Eidos lists them. Each one is a
/// data file in `data/games/` with exactly the schema of a user definition, so a
/// built-in game is added by writing one more file and listing it here.
const BUILTIN_DEFINITIONS: &[&str] = &[
    include_str!("../data/games/skyrimse.toml"),
    include_str!("../data/games/skyrimvr.toml"),
    include_str!("../data/games/skyrim.toml"),
    include_str!("../data/games/enderalse.toml"),
    include_str!("../data/games/fallout4.toml"),
    include_str!("../data/games/fallout4vr.toml"),
    include_str!("../data/games/falloutnv.toml"),
    include_str!("../data/games/fallout3.toml"),
    include_str!("../data/games/oblivion.toml"),
    include_str!("../data/games/morrowind.toml"),
    include_str!("../data/games/starfield.toml"),
    include_str!("../data/games/stellarblade.toml"),
    include_str!("../data/games/stardewvalley.toml"),
    include_str!("../data/games/7daystodie.toml"),
    include_str!("../data/games/crimsondesert.toml"),
];

/// Every game Eidos ships with, parsed once from [`BUILTIN_DEFINITIONS`].
///
/// A built-in file that does not parse is a build defect, not a user error, so it
/// panics here (and in the test that parses each file) instead of being skipped.
pub fn builtin() -> &'static [GameDef] {
    static BUILTIN: OnceLock<Vec<GameDef>> = OnceLock::new();
    BUILTIN.get_or_init(|| {
        BUILTIN_DEFINITIONS
            .iter()
            .map(|text| match toml::from_str::<RawGameDef>(text) {
                Ok(raw) => raw.into_gamedef(&[]),
                Err(e) => panic!("invalid built-in game definition: {e}"),
            })
            .collect()
    })
}

impl GameDef {
    /// Pinned primary plugins that the engine loads without an activation entry.
    /// Enderal SE runs on the Skyrim SE engine, which loads only Skyrim's own
    /// masters implicitly: Enderal's master and SkyUI are required but must be
    /// explicitly active.
    pub fn implicit_plugins(&self) -> &'static [&'static str] {
        let skyrim_se = || builtin().iter().find(|g| g.id == "skyrimse");
        match self.id {
            "enderalse" => skyrim_se().map_or(self.primary_plugins, |se| se.primary_plugins),
            _ => self.primary_plugins,
        }
    }

    /// The definition for an Eidos game id (built-in or user TOML), or `None`.
    pub fn for_id(id: &str) -> Option<&'static GameDef> {
        all().iter().find(|g| g.id.eq_ignore_ascii_case(id))
    }
}

/// Every game definition Eidos knows: the [`builtin`] games plus any user TOML
/// definitions in `~/.config/Colony/Eidos/games/`, loaded once. A user definition
/// whose `id` matches a built-in overrides it; the rest are appended.
pub fn all() -> &'static [GameDef] {
    static REGISTRY: OnceLock<Vec<GameDef>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut games: Vec<GameDef> = builtin().to_vec();
        for g in load_games_from(&user_games_dir()) {
            match games.iter_mut().find(|b| b.id.eq_ignore_ascii_case(g.id)) {
                Some(slot) => *slot = g,
                None => games.push(g),
            }
        }
        games
    })
}

/// The directory user TOML game definitions are read from.
fn user_games_dir() -> PathBuf {
    eidos_paths::config_dir().join("games")
}

/// Parse every `*.toml` game definition in `dir`, ignoring invalid ones with a
/// warning. Public for tooling and tests.
pub fn load_games_from(dir: &Path) -> Vec<GameDef> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        if p.extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("toml"))
        {
            match fs::read_to_string(&p).ok().and_then(|t| parse_game(&t)) {
                Some(g) => out.push(g),
                None => eprintln!("eidos: ignoring invalid game definition {}", p.display()),
            }
        }
    }
    out
}

/// Parse one TOML game definition into a (leaked, `'static`) [`GameDef`].
pub fn parse_game(toml: &str) -> Option<GameDef> {
    toml::from_str::<RawGameDef>(toml)
        .ok()
        .map(|raw| raw.into_gamedef(builtin()))
}

/// The owned, deserializable shape of a TOML game definition. The Bethesda-specific
/// fields all default to empty, so a generic game needs only `id`, `name`,
/// `steam_app_id`, and `data_dir`.
#[derive(serde::Deserialize)]
struct RawGameDef {
    id: String,
    name: String,
    #[serde(default)]
    short_name: String,
    #[serde(default)]
    nexus_game: String,
    #[serde(default)]
    steam_app_id: u32,
    #[serde(default)]
    gog_ids: Vec<String>,
    #[serde(default)]
    epic_ids: Vec<String>,
    data_dir: String,
    #[serde(default)]
    valid_folders: Vec<String>,
    #[serde(default)]
    valid_suffixes: Vec<String>,
    #[serde(default)]
    valid_files: Vec<String>,
    #[serde(default)]
    mod_unit: ModUnit,
    #[serde(default)]
    documents_dir: String,
    #[serde(default)]
    ini_files: Vec<String>,
    #[serde(default = "default_load_order")]
    load_order: String,
    #[serde(default)]
    primary_plugins: Vec<String>,
    #[serde(default)]
    game_binary: String,
    #[serde(default)]
    registry_name: String,
    #[serde(default)]
    script_extender: Option<RawScriptExtender>,
    #[serde(default)]
    known_tools: Vec<RawKnownTool>,
}

#[derive(serde::Deserialize)]
struct RawScriptExtender {
    launcher: String,
    loader: String,
}

/// One entry of a user game's `[[known_tools]]`: an executable Eidos should
/// recognise wherever it finds it, and the title to list it under.
#[derive(serde::Deserialize)]
struct RawKnownTool {
    exe: String,
    title: String,
}

fn default_load_order() -> String {
    "None".to_string()
}

impl RawGameDef {
    /// `inherit_tools_from` is where an empty `known_tools` list is filled from:
    /// the built-in games for a user definition, nothing for a built-in one.
    /// Registry initialization parses user rows, so inheritance must only consult
    /// built-ins rather than re-entering the registry's OnceLock.
    fn into_gamedef(self, inherit_tools_from: &[GameDef]) -> GameDef {
        let inherited_tools = inherit_tools_from
            .iter()
            .find(|g| g.id.eq_ignore_ascii_case(&self.id))
            .map(|g| g.known_tools)
            .unwrap_or(&[]);
        GameDef {
            id: leak(self.id),
            name: leak(self.name),
            short_name: leak(self.short_name),
            nexus_game: leak(self.nexus_game),
            steam_app_id: self.steam_app_id,
            gog_ids: leak_vec(self.gog_ids),
            epic_ids: leak_vec(self.epic_ids),
            data_dir: leak(self.data_dir),
            valid_folders: leak_vec(self.valid_folders),
            valid_suffixes: leak_vec(self.valid_suffixes),
            valid_files: leak_vec(self.valid_files),
            mod_unit: self.mod_unit,
            documents_dir: leak(self.documents_dir),
            ini_files: leak_vec(self.ini_files),
            load_order: parse_load_order(&self.load_order),
            primary_plugins: leak_vec(self.primary_plugins),
            game_binary: leak(self.game_binary),
            registry_name: leak(self.registry_name),
            script_extender: self.script_extender.map(|s| ScriptExtender {
                launcher: leak(s.launcher),
                loader: leak(s.loader),
            }),
            // Declared tools win; an empty list inherits, which is what makes a
            // TOML that overrides a built-in row keep that row's editors without
            // restating them.
            //
            // This list used to be inheritance-only, on the reasoning that xEdit
            // is per ENGINE and a hand-written row is for a game Eidos does not
            // know, which by definition has no xEdit build. True of xEdit, and
            // wrong about everything else: the detection is generic and
            // name-based, and a game somebody adds by hand is exactly the game
            // whose tools Eidos cannot possibly know - WolvenKit, a BepInEx
            // configuration editor, whatever the community uses. Refusing to let
            // the descriptor say so left the one mechanism that could have helped
            // available only to games that did not need it.
            known_tools: match self.known_tools.is_empty() {
                true => inherited_tools,
                false => leak_tools(self.known_tools),
            },
        }
    }
}

fn parse_load_order(s: &str) -> LoadOrder {
    match s.trim().to_ascii_lowercase().as_str() {
        "asterisk" => LoadOrder::Asterisk,
        "plainlist" => LoadOrder::PlainList,
        "filetime" => LoadOrder::FileTime,
        "none" | "" => LoadOrder::None,
        _ => {
            // Say so. This crate's contract is "invalid definitions are ignored
            // WITH a warning", and a typo here ("Asterisks") is the one field
            // that used to degrade silently: the game loaded as generic, the
            // whole plugin system stayed off, and nothing said why.
            eprintln!(
                "eidos: unknown load_order \"{}\" in a game definition; \
                 treating it as \"None\" (no plugin system)",
                s.trim()
            );
            LoadOrder::None
        }
    }
}

/// Leak an owned string to `'static`. Game definitions are loaded once at startup
/// and live for the whole run, so this permanent allocation is intentional: it lets
/// a loaded game share the exact `&'static str` shape of a built-in, with zero
/// ripple on the ~35 read sites across the workspace.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn leak_vec(v: Vec<String>) -> &'static [&'static str] {
    Box::leak(
        v.into_iter()
            .map(leak)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}

fn leak_tools(v: Vec<RawKnownTool>) -> &'static [(&'static str, &'static str)] {
    Box::leak(
        v.into_iter()
            .map(|t| (leak(t.exe), leak(t.title)))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every embedded data file parses on its own, and together they are exactly
    /// the catalog Eidos has always shipped: same ids in the same order, same
    /// masters, same implicit set. Changing a built-in game must change this.
    #[test]
    fn the_embedded_data_files_are_the_built_in_catalog() {
        for text in BUILTIN_DEFINITIONS {
            toml::from_str::<RawGameDef>(text).expect("built-in definition parses");
        }
        let skyrim_se: &[&str] = &[
            "Skyrim.esm",
            "Update.esm",
            "Dawnguard.esm",
            "HearthFires.esm",
            "Dragonborn.esm",
        ];
        let snapshot: &[(&str, u32, LoadOrder, &[&str])] = &[
            ("skyrimse", 489830, LoadOrder::Asterisk, skyrim_se),
            ("skyrimvr", 611670, LoadOrder::Asterisk, skyrim_se),
            (
                "skyrim",
                72850,
                LoadOrder::PlainList,
                &["Skyrim.esm", "Update.esm"],
            ),
            (
                "enderalse",
                976620,
                LoadOrder::Asterisk,
                &[
                    "Skyrim.esm",
                    "Update.esm",
                    "Dawnguard.esm",
                    "HearthFires.esm",
                    "Dragonborn.esm",
                    "Enderal - Forgotten Stories.esm",
                    "SkyUI_SE.esp",
                ],
            ),
            ("fallout4", 377160, LoadOrder::Asterisk, &["Fallout4.esm"]),
            ("fallout4vr", 611660, LoadOrder::Asterisk, &["Fallout4.esm"]),
            ("falloutnv", 22380, LoadOrder::FileTime, &["FalloutNV.esm"]),
            ("fallout3", 22370, LoadOrder::FileTime, &["Fallout3.esm"]),
            ("oblivion", 22330, LoadOrder::FileTime, &["Oblivion.esm"]),
            ("morrowind", 22320, LoadOrder::FileTime, &["Morrowind.esm"]),
            (
                "starfield",
                1716740,
                LoadOrder::Asterisk,
                &["Starfield.esm", "Constellation.esm", "OldMars.esm"],
            ),
            ("stellarblade", 3489700, LoadOrder::None, &[]),
            ("stardewvalley", 413150, LoadOrder::None, &[]),
            ("7daystodie", 251570, LoadOrder::None, &[]),
            ("crimsondesert", 3321460, LoadOrder::None, &[]),
        ];
        let actual: Vec<_> = builtin()
            .iter()
            .map(|g| (g.id, g.steam_app_id, g.load_order, g.primary_plugins))
            .collect();
        assert_eq!(actual, snapshot);

        // Only Enderal loads fewer masters implicitly than it pins.
        for g in builtin() {
            let expected = if g.id == "enderalse" {
                skyrim_se
            } else {
                g.primary_plugins
            };
            assert_eq!(g.implicit_plugins(), expected, "{}", g.id);
        }

        // The xEdit family travels as a set, per game.
        let tools = |id: &str| {
            builtin()
                .iter()
                .find(|g| g.id == id)
                .unwrap()
                .known_tools
                .iter()
                .map(|(exe, _)| *exe)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            tools("skyrimse"),
            ["SSEEdit.exe", "SSEEditQuickAutoClean.exe", "PGPatcher.exe"]
        );
        assert_eq!(tools("stardewvalley"), ["StardewModdingAPI.exe"]);
        assert!(tools("morrowind").is_empty());
    }

    #[test]
    fn folder_games_declare_their_marker_files() {
        for (id, marker) in [
            ("stardewvalley", "manifest.json"),
            ("7daystodie", "ModInfo.xml"),
        ] {
            let game = builtin().iter().find(|g| g.id == id).unwrap();
            assert_eq!(game.mod_unit, ModUnit::Folder);
            assert_eq!(game.valid_files, [marker]);
            assert!(game.valid_folders.is_empty() && game.valid_suffixes.is_empty());
            assert_eq!(game.data_dir, "Mods");
        }
        let legacy = parse_game("id = \"legacy\"\nname = \"Legacy\"\ndata_dir = \"Data\"").unwrap();
        assert_eq!(legacy.mod_unit, ModUnit::Files);
        assert!(legacy.valid_files.is_empty());
    }

    #[test]
    fn folder_game_schema_rejects_misspelled_units() {
        let base = "id = \"custom\"\nname = \"Custom\"\ndata_dir = \"Mods\"\nvalid_files = [\"manifest.json\"]\n";
        assert!(parse_game(&format!("{base}mod_unit = \"folders\"\n")).is_none());
        assert!(parse_game(&format!("{base}mod_unit = \"folder\"\n")).is_some());
        assert!(parse_game(&format!("{base}mod_unit = \"files\"\n")).is_some());
    }

    #[test]
    fn lookup_by_id() {
        let se = GameDef::for_id("skyrimse").unwrap();
        assert_eq!(se.name, "Skyrim Special Edition");
        assert_eq!(se.steam_app_id, 489830);
        assert_eq!(se.load_order, LoadOrder::Asterisk);
        assert!(GameDef::for_id("nope").is_none());
    }

    #[test]
    fn ids_are_unique() {
        // Case-insensitively, because that is how `for_id` matches: two ids
        // differing only in case would pass an exact-match check here and then
        // shadow each other at every lookup.
        for (i, g) in builtin().iter().enumerate() {
            assert!(
                builtin()
                    .iter()
                    .skip(i + 1)
                    .all(|o| !o.id.eq_ignore_ascii_case(g.id)),
                "duplicate id {}",
                g.id
            );
        }
    }

    #[test]
    fn archive_ini_is_first() {
        // ini_files[0] is the [Archive]-carrying INI for every game that has INIs.
        assert_eq!(
            GameDef::for_id("skyrimse").unwrap().ini_files.first(),
            Some(&"Skyrim.ini")
        );
        assert_eq!(
            GameDef::for_id("fallout4").unwrap().ini_files.first(),
            Some(&"Fallout4.ini")
        );
    }

    #[test]
    fn custom_inis_are_listed_after_the_archive_ini() {
        // MO2's per-game iniFiles() includes the Custom/Prefs INIs that hold
        // ENB/mod settings (and FO4 invalidation); they must be in the set so they
        // are seeded/captured per profile, but never displace element 0.
        let custom = |id: &str, file: &str| {
            let inis = GameDef::for_id(id).unwrap().ini_files;
            assert!(inis.contains(&file), "{id} ini_files missing {file}");
            assert_ne!(
                inis[0], file,
                "{file} must not be the [Archive] INI for {id}"
            );
        };
        // skyrimse keeps Skyrim.ini first and gains SkyrimCustom.ini.
        let se = GameDef::for_id("skyrimse").unwrap().ini_files;
        assert_eq!(se.first(), Some(&"Skyrim.ini"));
        assert!(se.contains(&"SkyrimCustom.ini"));
        custom("fallout4", "Fallout4Custom.ini");
        custom("fallout4vr", "Fallout4Custom.ini");
        custom("falloutnv", "FalloutCustom.ini");
        custom("fallout3", "FalloutCustom.ini");
        custom("oblivion", "OblivionPrefs.ini");
    }

    #[test]
    fn games_with_inis_have_a_documents_dir() {
        for g in builtin() {
            // Morrowind's INI lives in the install dir, not My Games (special-cased).
            if !g.ini_files.is_empty() && g.id != "morrowind" {
                assert!(
                    !g.documents_dir.is_empty(),
                    "{} has INIs but no documents_dir",
                    g.id
                );
            }
        }
    }

    #[test]
    fn parses_a_generic_game_definition() {
        // The minimum a non-Bethesda game needs: id, name, app id, data dir.
        let g = parse_game(
            r#"
            id = "stardew"
            name = "Stardew Valley"
            steam_app_id = 413150
            data_dir = "Mods"
            "#,
        )
        .unwrap();
        assert_eq!(g.id, "stardew");
        assert_eq!(g.steam_app_id, 413150);
        assert_eq!(g.data_dir, "Mods");
        // Generic: no Bethesda machinery, so the engine is just the file union.
        assert_eq!(g.load_order, LoadOrder::None);
        assert!(g.ini_files.is_empty());
        assert!(g.primary_plugins.is_empty());
        assert!(g.script_extender.is_none());
    }

    #[test]
    fn parses_a_full_bethesda_style_definition() {
        let g = parse_game(
            r#"
            id = "skyrimse-custom"
            name = "Skyrim SE (custom)"
            short_name = "SkyrimSE"
            nexus_game = "skyrimspecialedition"
            steam_app_id = 489830
            data_dir = "Data"
            documents_dir = "Skyrim Special Edition"
            ini_files = ["Skyrim.ini", "SkyrimPrefs.ini"]
            load_order = "Asterisk"
            primary_plugins = ["Skyrim.esm", "Update.esm"]
            script_extender = { launcher = "SkyrimSELauncher.exe", loader = "skse64_loader.exe" }
            "#,
        )
        .unwrap();
        assert_eq!(g.load_order, LoadOrder::Asterisk);
        assert_eq!(g.ini_files, ["Skyrim.ini", "SkyrimPrefs.ini"].as_slice());
        assert_eq!(g.primary_plugins, ["Skyrim.esm", "Update.esm"].as_slice());
        assert_eq!(g.script_extender.unwrap().loader, "skse64_loader.exe");
    }

    #[test]
    fn a_user_game_can_declare_its_own_tools() {
        // The gap this closes: the detection is generic and name-based, but the
        // descriptor could not name anything, so it only ever helped games that
        // already shipped with Eidos.
        let g = parse_game(
            r#"
            id = "cyberpunk2077"
            name = "Cyberpunk 2077"
            steam_app_id = 1091500
            data_dir = "."
            [[known_tools]]
            exe = "WolvenKit.exe"
            title = "WolvenKit"
            [[known_tools]]
            exe = "REDmodTool.exe"
            title = "REDmod"
            "#,
        )
        .expect("a game with tools must parse");
        assert_eq!(
            g.known_tools,
            &[("WolvenKit.exe", "WolvenKit"), ("REDmodTool.exe", "REDmod")]
        );
    }

    #[test]
    fn a_user_game_that_declares_no_tools_inherits_the_built_in_row_s() {
        // Overriding a built-in to change one field must not silently cost the
        // user xEdit.
        let g = parse_game(
            r#"
            id = "fallout4"
            name = "Fallout 4"
            steam_app_id = 377160
            data_dir = "Data"
            "#,
        )
        .expect("an override must parse");
        let built_in = GameDef::for_id("fallout4").unwrap();
        assert_eq!(g.known_tools, built_in.known_tools);
        assert!(
            !g.known_tools.is_empty(),
            "fallout4 ships with FO4Edit declared"
        );
    }

    #[test]
    fn a_brand_new_game_with_no_tools_declares_none() {
        let g = parse_game(
            r#"
            id = "nosuchgame"
            name = "No Such Game"
            steam_app_id = 1
            data_dir = "Mods"
            "#,
        )
        .expect("must parse");
        assert!(g.known_tools.is_empty());
    }

    #[test]
    fn load_games_from_reads_only_toml() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "eidos-games-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("stardew.toml"),
            "id = \"stardew\"\nname = \"Stardew\"\nsteam_app_id = 413150\ndata_dir = \"Mods\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();
        let games = load_games_from(&dir);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "stardew");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_definition_is_skipped() {
        // Missing the required id / steam_app_id / data_dir -> not parsed.
        assert!(parse_game("name = \"incomplete\"").is_none());
    }

    #[test]
    fn a_misspelled_load_order_degrades_to_none_but_keeps_the_game() {
        // "Asterisks" is not a mechanism; the definition still loads (dropping a
        // whole game over one field would be harsher than the typo), it just
        // loads as generic - and parse_load_order warns, where it used to say
        // nothing at all.
        let g = parse_game(
            "id = \"typo\"\nname = \"Typo\"\nsteam_app_id = 1\ndata_dir = \"Data\"\n\
             load_order = \"Asterisks\"\n",
        )
        .unwrap();
        assert_eq!(g.load_order, LoadOrder::None);
    }

    #[test]
    fn plugin_games_have_primaries() {
        // A game using a plugins.txt mechanism must declare its master plugins.
        //
        // Written as "anything but FileTime" this held only while every game here
        // was Bethesda's. `LoadOrder::None` is a real answer now (Stellar Blade),
        // and a game with no plugin system at all has no masters to declare - so
        // the condition has to name the two mechanisms it is actually about.
        for g in builtin() {
            if matches!(g.load_order, LoadOrder::Asterisk | LoadOrder::PlainList) {
                assert!(
                    !g.primary_plugins.is_empty(),
                    "{} has no primary plugins",
                    g.id
                );
            }
        }
    }

    /// A game with no plugin system declares no plugin machinery at all, so the
    /// rest of the workspace can key off `LoadOrder::None` alone.
    #[test]
    fn a_game_without_a_load_order_declares_no_plugin_machinery() {
        for g in builtin().iter().filter(|g| g.load_order == LoadOrder::None) {
            assert!(
                g.primary_plugins.is_empty(),
                "{} has masters but no load order",
                g.id
            );
            assert!(
                g.ini_files.is_empty(),
                "{} has per-profile INIs but no load order",
                g.id
            );
        }
        // And the one that exists is the one we expect, so this cannot pass vacuously.
        assert!(builtin()
            .iter()
            .any(|g| g.id == "stellarblade" && g.load_order == LoadOrder::None));
    }

    #[test]
    fn nexus_mapping_is_populated() {
        // Every game has the MO2 short name + the Nexus domain; VR editions
        // share their parent game's Nexus.
        for g in builtin() {
            assert!(!g.short_name.is_empty(), "{} missing short_name", g.id);
            assert!(!g.nexus_game.is_empty(), "{} missing nexus_game", g.id);
        }
        let se = GameDef::for_id("skyrimse").unwrap();
        assert_eq!(
            (se.short_name, se.nexus_game),
            ("SkyrimSE", "skyrimspecialedition")
        );
        assert_eq!(
            GameDef::for_id("skyrimvr").unwrap().nexus_game,
            "skyrimspecialedition"
        );
        assert_eq!(GameDef::for_id("falloutnv").unwrap().nexus_game, "newvegas");
    }

    #[test]
    fn script_extender_swap_is_populated() {
        let se = GameDef::for_id("skyrimse")
            .unwrap()
            .script_extender
            .unwrap();
        assert_eq!(se.launcher, "SkyrimSELauncher.exe");
        assert_eq!(se.loader, "skse64_loader.exe");
        // Games whose launcher we do not know stay None rather than guessing.
        // Every script-extended game now declares its loader (VR + Enderal + SFSE).
        assert_eq!(
            GameDef::for_id("starfield")
                .unwrap()
                .script_extender
                .unwrap()
                .loader,
            "sfse_loader.exe"
        );
        assert_eq!(
            GameDef::for_id("skyrimvr")
                .unwrap()
                .script_extender
                .unwrap()
                .loader,
            "sksevr_loader.exe"
        );
        assert_eq!(
            GameDef::for_id("fallout4vr")
                .unwrap()
                .script_extender
                .unwrap()
                .loader,
            "f4sevr_loader.exe"
        );
        assert_eq!(
            GameDef::for_id("enderalse")
                .unwrap()
                .script_extender
                .unwrap()
                .loader,
            "skse64_loader.exe"
        );
    }
}
