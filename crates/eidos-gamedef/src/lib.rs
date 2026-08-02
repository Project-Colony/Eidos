//! Declarative per-game definitions for the games Eidos manages, modelled on Mod
//! Organizer 2's `IPluginGame` schema (the same field set MO2's `basic_games`
//! plugin exposes declaratively): identity, the Steam app id, the data and
//! Documents directories, the per-profile INIs, the plugin load-order mechanism
//! and primary masters, and the script-extender loader.
//!
//! This is the single source of truth the other crates read from, so adding a
//! built-in game is one row in [`GAMES`] instead of edits scattered across
//! detection, plugins, and game features.
//!
//! Games can also be added WITHOUT recompiling, the way MO2's `basic_games` plugin
//! lets simple games be declared as data: drop a `<id>.toml` into
//! `$XDG_CONFIG_HOME/eidos/games/` (or `~/.config/eidos/games/`) and it joins the
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
    /// The data directory under the game install (`Data`, or `Data Files` for
    /// Morrowind).
    pub data_dir: &'static str,
    /// Top-level directory names that mark a level inside an archive as this
    /// game's mod root - MO2's per-game `ModDataChecker::possibleFolderNames`.
    ///
    /// **Empty means the Gamebryo vocabulary**, which is what every built-in game
    /// here uses and why they all leave this unset. It does NOT mean "nothing is a
    /// mod root": an empty list taken literally would reject every archive and send
    /// every install to the manual picker.
    pub valid_folders: &'static [&'static str],
    /// File extensions, without the dot, that mark a level as this game's mod root
    /// (`esp`, `esm`, ... for Gamebryo; `pak` for Unreal; `archive` for Cyberpunk).
    /// Empty means the Gamebryo set, with the same caveat as [`Self::valid_folders`].
    pub valid_suffixes: &'static [&'static str],
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
}

/// Shared across the three Skyrim SE-engine games (SE, VR, Enderal SE).
const SKYRIM_SE_MASTERS: &[&str] =
    &["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"];

/// Every game Eidos knows about. Add a game by adding one row.
pub static GAMES: &[GameDef] = &[
    GameDef {
        id: "skyrimse",
        name: "Skyrim Special Edition",
        steam_app_id: 489830,
        short_name: "SkyrimSE",
        nexus_game: "skyrimspecialedition",
        data_dir: "Data",
        valid_folders: &[],
        valid_suffixes: &[],
        documents_dir: "Skyrim Special Edition",
        ini_files: &["Skyrim.ini", "SkyrimPrefs.ini", "SkyrimCustom.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: SKYRIM_SE_MASTERS,
        game_binary: "SkyrimSE.exe",
        registry_name: "Skyrim Special Edition",
        script_extender: Some(ScriptExtender {
            launcher: "SkyrimSELauncher.exe",
            loader: "skse64_loader.exe",
        }),
    },
    GameDef {
        id: "skyrimvr",
        name: "Skyrim VR",
        steam_app_id: 611670,
        short_name: "SkyrimVR",
        nexus_game: "skyrimspecialedition",
        data_dir: "Data",
        valid_folders: &[],
        valid_suffixes: &[],
        documents_dir: "Skyrim VR",
        ini_files: &["SkyrimVR.ini", "SkyrimPrefs.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: SKYRIM_SE_MASTERS,
        game_binary: "SkyrimVR.exe",
        registry_name: "Skyrim VR",
        script_extender: Some(ScriptExtender {
            launcher: "SkyrimVRLauncher.exe",
            loader: "sksevr_loader.exe",
        }),
    },
    GameDef {
        id: "skyrim",
        name: "Skyrim",
        steam_app_id: 72850,
        short_name: "Skyrim",
        nexus_game: "skyrim",
        data_dir: "Data",
        valid_folders: &[],
        valid_suffixes: &[],
        documents_dir: "Skyrim",
        // The gamebryo engine reads SkyrimCustom.ini in LE too (MO2 manages it as a
        // per-profile INI, same as SkyrimSE), so include it in the per-profile set.
        ini_files: &["Skyrim.ini", "SkyrimPrefs.ini", "SkyrimCustom.ini"],
        load_order: LoadOrder::PlainList,
        primary_plugins: &["Skyrim.esm", "Update.esm"],
        game_binary: "TESV.exe",
        registry_name: "Skyrim",
        script_extender: Some(ScriptExtender {
            launcher: "SkyrimLauncher.exe",
            loader: "skse_loader.exe",
        }),
    },
    GameDef {
        id: "enderalse",
        name: "Enderal: Special Edition",
        steam_app_id: 976620,
        short_name: "EnderalSE",
        nexus_game: "enderalspecialedition",
        data_dir: "Data",
        valid_folders: &[],
        valid_suffixes: &[],
        documents_dir: "Enderal Special Edition",
        ini_files: &["Enderal.ini", "EnderalPrefs.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: SKYRIM_SE_MASTERS,
        game_binary: "SkyrimSE.exe",
        registry_name: "",
        // Enderal SE ships as a Skyrim SE reskin and uses SKSE64 unchanged.
        script_extender: Some(ScriptExtender {
            launcher: "Enderal Launcher.exe",
            loader: "skse64_loader.exe",
        }),
    },
    GameDef {
        id: "fallout4",
        name: "Fallout 4",
        steam_app_id: 377160,
        short_name: "Fallout4",
        nexus_game: "fallout4",
        data_dir: "Data",
        valid_folders: &[],
        valid_suffixes: &[],
        documents_dir: "Fallout4",
        ini_files: &["Fallout4.ini", "Fallout4Prefs.ini", "Fallout4Custom.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: &["Fallout4.esm"],
        game_binary: "Fallout4.exe",
        registry_name: "Fallout4",
        script_extender: Some(ScriptExtender {
            launcher: "Fallout4Launcher.exe",
            loader: "f4se_loader.exe",
        }),
    },
    GameDef {
        id: "fallout4vr",
        name: "Fallout 4 VR",
        steam_app_id: 611660,
        short_name: "Fallout4VR",
        nexus_game: "fallout4",
        data_dir: "Data",
        valid_folders: &[],
        valid_suffixes: &[],
        documents_dir: "Fallout4VR",
        ini_files: &["Fallout4.ini", "Fallout4Prefs.ini", "Fallout4Custom.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: &["Fallout4.esm"],
        game_binary: "Fallout4VR.exe",
        registry_name: "Fallout 4 VR",
        script_extender: Some(ScriptExtender {
            launcher: "Fallout4VRLauncher.exe",
            loader: "f4sevr_loader.exe",
        }),
    },
    GameDef {
        id: "falloutnv",
        name: "Fallout: New Vegas",
        steam_app_id: 22380,
        short_name: "FalloutNV",
        nexus_game: "newvegas",
        data_dir: "Data",
        valid_folders: &[],
        valid_suffixes: &[],
        documents_dir: "FalloutNV",
        ini_files: &["Fallout.ini", "FalloutPrefs.ini", "FalloutCustom.ini"],
        load_order: LoadOrder::PlainList,
        primary_plugins: &["FalloutNV.esm"],
        game_binary: "FalloutNV.exe",
        registry_name: "FalloutNV",
        script_extender: Some(ScriptExtender {
            launcher: "FalloutNVLauncher.exe",
            loader: "nvse_loader.exe",
        }),
    },
    GameDef {
        id: "fallout3",
        name: "Fallout 3 (GOTY)",
        steam_app_id: 22370,
        short_name: "Fallout3",
        nexus_game: "fallout3",
        data_dir: "Data",
        valid_folders: &[],
        valid_suffixes: &[],
        documents_dir: "Fallout3",
        ini_files: &["Fallout.ini", "FalloutPrefs.ini", "FalloutCustom.ini"],
        load_order: LoadOrder::PlainList,
        primary_plugins: &["Fallout3.esm"],
        game_binary: "Fallout3.exe",
        registry_name: "Fallout3",
        script_extender: Some(ScriptExtender {
            launcher: "FalloutLauncher.exe",
            loader: "fose_loader.exe",
        }),
    },
    GameDef {
        id: "oblivion",
        name: "Oblivion",
        steam_app_id: 22330,
        short_name: "Oblivion",
        nexus_game: "oblivion",
        data_dir: "Data",
        valid_folders: &[],
        valid_suffixes: &[],
        documents_dir: "Oblivion",
        ini_files: &["Oblivion.ini", "OblivionPrefs.ini"],
        load_order: LoadOrder::FileTime,
        primary_plugins: &["Oblivion.esm"],
        game_binary: "Oblivion.exe",
        registry_name: "Oblivion",
        script_extender: Some(ScriptExtender {
            launcher: "OblivionLauncher.exe",
            loader: "obse_loader.exe",
        }),
    },
    GameDef {
        id: "morrowind",
        name: "Morrowind",
        steam_app_id: 22320,
        short_name: "Morrowind",
        nexus_game: "morrowind",
        data_dir: "Data Files",
        valid_folders: &[],
        valid_suffixes: &[],
        // Morrowind keeps Morrowind.ini in the install directory, not My Games; the
        // per-profile INI machinery is pointed there by game id (see prepare_inis).
        documents_dir: "",
        ini_files: &["Morrowind.ini"],
        load_order: LoadOrder::FileTime,
        primary_plugins: &["Morrowind.esm"],
        game_binary: "Morrowind.exe",
        registry_name: "Morrowind",
        script_extender: None,
    },
    GameDef {
        id: "starfield",
        name: "Starfield",
        steam_app_id: 1716740,
        short_name: "Starfield",
        nexus_game: "starfield",
        data_dir: "Data",
        valid_folders: &[],
        valid_suffixes: &[],
        documents_dir: "Starfield",
        ini_files: &["StarfieldCustom.ini", "StarfieldPrefs.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: &["Starfield.esm", "Constellation.esm", "OldMars.esm"],
        game_binary: "Starfield.exe",
        registry_name: "Starfield",
        script_extender: Some(ScriptExtender {
            launcher: "Starfield.exe",
            loader: "sfse_loader.exe",
        }),
    },
    // The first non-Bethesda game here, and the first to declare its own vocabulary.
    //
    // Unreal Engine, so no plugins.txt, no BSA, no My Games INI. A mod is a
    // `.pak`/`.ucas`/`.utoc` triplet under `SB/Content/Paks`, either loose or in one
    // of the two folders the community sorts them into: `~mods` (the `~` is what
    // makes the engine scan it last, so those paks win) and `LogicMods` for UE4SS.
    // `Paks` is the deploy root rather than `~mods` precisely so an archive can
    // choose either, and so both land in the right place from one union.
    //
    // Declaring `valid_suffixes` is what makes `valid_folders` mean "only these":
    // see `From<&GameDef> for LayoutRules`. That is deliberate. A Stellar Blade mod
    // is identified by carrying a pak, and inheriting the Gamebryo folder list would
    // let an archive shipping a `textures/` folder read as a valid mod root.
    //
    // UE4SS is this game's script extender - its SKSE. `script_extender` stays None
    // because that field models an EXE SWAP (run the loader instead of the vanilla
    // launcher), and UE4SS does not work that way: it is a proxy DLL side-loaded
    // next to the game binary. Its own Lua mods ship a `SB/Binaries/Win64/ue4ss/...`
    // tree, addressed from the install root; `root_builder_split` recognises that
    // shape from `data_dir` naming `SB` as the game's own directory, and routes it
    // to the `Root/` surface.
    GameDef {
        id: "stellarblade",
        name: "Stellar Blade",
        steam_app_id: 3489700,
        short_name: "StellarBlade",
        nexus_game: "stellarblade",
        data_dir: "SB/Content/Paks",
        valid_folders: &["~mods", "logicmods"],
        valid_suffixes: &["pak", "utoc", "ucas"],
        documents_dir: "",
        ini_files: &[],
        load_order: LoadOrder::None,
        primary_plugins: &[],
        game_binary: "SB.exe",
        registry_name: "",
        script_extender: None,
    },
];

impl GameDef {
    /// The definition for an Eidos game id (built-in or user TOML), or `None`.
    pub fn for_id(id: &str) -> Option<&'static GameDef> {
        all().iter().find(|g| g.id.eq_ignore_ascii_case(id))
    }
}

/// Every game definition Eidos knows: the built-in [`GAMES`] plus any user TOML
/// definitions in `$XDG_CONFIG_HOME/eidos/games/`, loaded once. A user definition
/// whose `id` matches a built-in overrides it; the rest are appended.
pub fn all() -> &'static [GameDef] {
    static REGISTRY: OnceLock<Vec<GameDef>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut games: Vec<GameDef> = GAMES.to_vec();
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
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| {
            std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".config")
        });
    base.join("eidos").join("games")
}

/// Parse every `*.toml` game definition in `dir`, ignoring invalid ones with a
/// warning. Public for tooling and tests.
pub fn load_games_from(dir: &Path) -> Vec<GameDef> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("toml")) {
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
    toml::from_str::<RawGameDef>(toml).ok().map(RawGameDef::into_gamedef)
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
    steam_app_id: u32,
    data_dir: String,
    #[serde(default)]
    valid_folders: Vec<String>,
    #[serde(default)]
    valid_suffixes: Vec<String>,
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
}

#[derive(serde::Deserialize)]
struct RawScriptExtender {
    launcher: String,
    loader: String,
}

fn default_load_order() -> String {
    "None".to_string()
}

impl RawGameDef {
    fn into_gamedef(self) -> GameDef {
        GameDef {
            id: leak(self.id),
            name: leak(self.name),
            short_name: leak(self.short_name),
            nexus_game: leak(self.nexus_game),
            steam_app_id: self.steam_app_id,
            data_dir: leak(self.data_dir),
            valid_folders: leak_vec(self.valid_folders),
            valid_suffixes: leak_vec(self.valid_suffixes),
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
        }
    }
}

fn parse_load_order(s: &str) -> LoadOrder {
    match s.trim().to_ascii_lowercase().as_str() {
        "asterisk" => LoadOrder::Asterisk,
        "plainlist" => LoadOrder::PlainList,
        "filetime" => LoadOrder::FileTime,
        _ => LoadOrder::None,
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
    Box::leak(v.into_iter().map(leak).collect::<Vec<_>>().into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        for (i, g) in GAMES.iter().enumerate() {
            assert!(GAMES.iter().skip(i + 1).all(|o| o.id != g.id), "duplicate id {}", g.id);
        }
    }

    #[test]
    fn archive_ini_is_first() {
        // ini_files[0] is the [Archive]-carrying INI for every game that has INIs.
        assert_eq!(GameDef::for_id("skyrimse").unwrap().ini_files.first(), Some(&"Skyrim.ini"));
        assert_eq!(GameDef::for_id("fallout4").unwrap().ini_files.first(), Some(&"Fallout4.ini"));
    }

    #[test]
    fn custom_inis_are_listed_after_the_archive_ini() {
        // MO2's per-game iniFiles() includes the Custom/Prefs INIs that hold
        // ENB/mod settings (and FO4 invalidation); they must be in the set so they
        // are seeded/captured per profile, but never displace element 0.
        let custom = |id: &str, file: &str| {
            let inis = GameDef::for_id(id).unwrap().ini_files;
            assert!(inis.contains(&file), "{id} ini_files missing {file}");
            assert_ne!(inis[0], file, "{file} must not be the [Archive] INI for {id}");
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
        for g in GAMES {
            // Morrowind's INI lives in the install dir, not My Games (special-cased).
            if !g.ini_files.is_empty() && g.id != "morrowind" {
                assert!(!g.documents_dir.is_empty(), "{} has INIs but no documents_dir", g.id);
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
    fn load_games_from_reads_only_toml() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir()
            .join(format!("eidos-games-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
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
    fn plugin_games_have_primaries() {
        // A game using a plugins.txt mechanism must declare its master plugins.
        //
        // Written as "anything but FileTime" this held only while every game here
        // was Bethesda's. `LoadOrder::None` is a real answer now (Stellar Blade),
        // and a game with no plugin system at all has no masters to declare - so
        // the condition has to name the two mechanisms it is actually about.
        for g in GAMES {
            if matches!(g.load_order, LoadOrder::Asterisk | LoadOrder::PlainList) {
                assert!(!g.primary_plugins.is_empty(), "{} has no primary plugins", g.id);
            }
        }
    }

    /// A game with no plugin system declares no plugin machinery at all, so the
    /// rest of the workspace can key off `LoadOrder::None` alone.
    #[test]
    fn a_game_without_a_load_order_declares_no_plugin_machinery() {
        for g in GAMES.iter().filter(|g| g.load_order == LoadOrder::None) {
            assert!(g.primary_plugins.is_empty(), "{} has masters but no load order", g.id);
            assert!(g.ini_files.is_empty(), "{} has per-profile INIs but no load order", g.id);
        }
        // And the one that exists is the one we expect, so this cannot pass vacuously.
        assert!(GAMES.iter().any(|g| g.id == "stellarblade" && g.load_order == LoadOrder::None));
    }

    #[test]
    fn nexus_mapping_is_populated() {
        // Every game has the MO2 short name + the Nexus domain; VR editions
        // share their parent game's Nexus.
        for g in GAMES {
            assert!(!g.short_name.is_empty(), "{} missing short_name", g.id);
            assert!(!g.nexus_game.is_empty(), "{} missing nexus_game", g.id);
        }
        let se = GameDef::for_id("skyrimse").unwrap();
        assert_eq!((se.short_name, se.nexus_game), ("SkyrimSE", "skyrimspecialedition"));
        assert_eq!(GameDef::for_id("skyrimvr").unwrap().nexus_game, "skyrimspecialedition");
        assert_eq!(GameDef::for_id("falloutnv").unwrap().nexus_game, "newvegas");
    }

    #[test]
    fn script_extender_swap_is_populated() {
        let se = GameDef::for_id("skyrimse").unwrap().script_extender.unwrap();
        assert_eq!(se.launcher, "SkyrimSELauncher.exe");
        assert_eq!(se.loader, "skse64_loader.exe");
        // Games whose launcher we do not know stay None rather than guessing.
        // Every script-extended game now declares its loader (VR + Enderal + SFSE).
        assert_eq!(GameDef::for_id("starfield").unwrap().script_extender.unwrap().loader, "sfse_loader.exe");
        assert_eq!(GameDef::for_id("skyrimvr").unwrap().script_extender.unwrap().loader, "sksevr_loader.exe");
        assert_eq!(GameDef::for_id("fallout4vr").unwrap().script_extender.unwrap().loader, "f4sevr_loader.exe");
        assert_eq!(GameDef::for_id("enderalse").unwrap().script_extender.unwrap().loader, "skse64_loader.exe");
    }
}
