//! Declarative per-game definitions for the games Eidos manages, modelled on Mod
//! Organizer 2's `IPluginGame` schema (the same field set MO2's `basic_games`
//! plugin exposes declaratively): identity, the Steam app id, the data and
//! Documents directories, the per-profile INIs, the plugin load-order mechanism
//! and primary masters, and the script-extender loader.
//!
//! This is the single source of truth the other crates read from, so adding a
//! game is one row here instead of edits scattered across detection, plugins, and
//! game features. The crate is pure data with no dependencies, so everything can
//! depend on it without cycles.
//!
//! Not yet modelled (MO2 has them; add when Eidos needs them): DLC/Creation-Club
//! plugin lists, `SortMechanism` (LOOT/BOSS), game variants (GOTY editions), and
//! the forced-load library list (we derive that from mods at launch instead).

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
        documents_dir: "Skyrim Special Edition",
        ini_files: &["Skyrim.ini", "SkyrimPrefs.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: SKYRIM_SE_MASTERS,
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
        documents_dir: "Skyrim VR",
        ini_files: &["SkyrimVR.ini", "SkyrimPrefs.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: SKYRIM_SE_MASTERS,
        script_extender: None,
    },
    GameDef {
        id: "skyrim",
        name: "Skyrim",
        steam_app_id: 72850,
        short_name: "Skyrim",
        nexus_game: "skyrim",
        data_dir: "Data",
        documents_dir: "Skyrim",
        ini_files: &["Skyrim.ini", "SkyrimPrefs.ini"],
        load_order: LoadOrder::PlainList,
        primary_plugins: &["Skyrim.esm", "Update.esm"],
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
        documents_dir: "Enderal Special Edition",
        ini_files: &["Enderal.ini", "EnderalPrefs.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: SKYRIM_SE_MASTERS,
        script_extender: None,
    },
    GameDef {
        id: "fallout4",
        name: "Fallout 4",
        steam_app_id: 377160,
        short_name: "Fallout4",
        nexus_game: "fallout4",
        data_dir: "Data",
        documents_dir: "Fallout4",
        ini_files: &["Fallout4.ini", "Fallout4Prefs.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: &["Fallout4.esm"],
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
        documents_dir: "Fallout4VR",
        ini_files: &["Fallout4.ini", "Fallout4Prefs.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: &["Fallout4.esm"],
        script_extender: None,
    },
    GameDef {
        id: "falloutnv",
        name: "Fallout: New Vegas",
        steam_app_id: 22380,
        short_name: "FalloutNV",
        nexus_game: "newvegas",
        data_dir: "Data",
        documents_dir: "FalloutNV",
        ini_files: &["Fallout.ini", "FalloutPrefs.ini"],
        load_order: LoadOrder::PlainList,
        primary_plugins: &["FalloutNV.esm"],
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
        documents_dir: "Fallout3",
        ini_files: &["Fallout.ini", "FalloutPrefs.ini"],
        load_order: LoadOrder::PlainList,
        primary_plugins: &["Fallout3.esm"],
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
        documents_dir: "Oblivion",
        ini_files: &["Oblivion.ini"],
        load_order: LoadOrder::FileTime,
        primary_plugins: &["Oblivion.esm"],
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
        documents_dir: "",
        ini_files: &[],
        load_order: LoadOrder::FileTime,
        primary_plugins: &["Morrowind.esm"],
        script_extender: None,
    },
    GameDef {
        id: "starfield",
        name: "Starfield",
        steam_app_id: 1716740,
        short_name: "Starfield",
        nexus_game: "starfield",
        data_dir: "Data",
        documents_dir: "Starfield",
        ini_files: &["StarfieldCustom.ini", "StarfieldPrefs.ini"],
        load_order: LoadOrder::Asterisk,
        primary_plugins: &["Starfield.esm", "Constellation.esm", "OldMars.esm"],
        script_extender: None,
    },
];

impl GameDef {
    /// The definition for an Eidos game id, or `None` if unknown.
    pub fn for_id(id: &str) -> Option<&'static GameDef> {
        GAMES.iter().find(|g| g.id == id)
    }
}

/// All known game definitions.
pub fn all() -> &'static [GameDef] {
    GAMES
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
    fn games_with_inis_have_a_documents_dir() {
        for g in GAMES {
            if !g.ini_files.is_empty() {
                assert!(!g.documents_dir.is_empty(), "{} has INIs but no documents_dir", g.id);
            }
        }
    }

    #[test]
    fn plugin_games_have_primaries() {
        // A game using a plugins.txt mechanism must declare its master plugins.
        for g in GAMES {
            if g.load_order != LoadOrder::FileTime {
                assert!(!g.primary_plugins.is_empty(), "{} has no primary plugins", g.id);
            }
        }
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
        assert!(GameDef::for_id("starfield").unwrap().script_extender.is_none());
    }
}
