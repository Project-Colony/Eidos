//! Supported-game catalog and Steam install detection.
//!
//! This is the Linux equivalent of Mod Organizer 2's game plugins + registry
//! probing. MO2 reads the Windows registry to locate Steam/GOG installs; here we
//! parse Steam's own metadata:
//!
//!   1. `steamapps/libraryfolders.vdf` lists every Steam library (including ones
//!      on other drives, e.g. `/mnt/Jeux/SteamLibrary`).
//!   2. each `steamapps/appmanifest_<appid>.acf` describes one installed game
//!      (appid, display name, install dir).
//!
//! We match the installed appids against [`catalog`] (our list of supported
//! games, each tagged with its Steam appid and the `Data`-style directory where
//! mods deploy) and report the games that are actually present, with their real
//! paths on disk and their Proton prefix.

use std::fs;
use std::path::{Path, PathBuf};

/// One supported game. Matching is by `app_id` alone; the install directory and
/// display name come from the game's own appmanifest, so a slightly wrong name
/// here never breaks detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDef {
    /// Eidos slug, e.g. `"skyrimse"`.
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Steam application id.
    pub app_id: u32,
    /// Directory (relative to the install root) where mods deploy, e.g. `"Data"`.
    pub data_dir: &'static str,
}

/// The built-in catalog of supported games. Extend freely; only `app_id` has to
/// be correct for detection to work.
static CATALOG: &[GameDef] = &[
    GameDef { id: "skyrimse",  name: "Skyrim Special Edition", app_id: 489830,  data_dir: "Data" },
    GameDef { id: "skyrim",    name: "Skyrim",                 app_id: 72850,   data_dir: "Data" },
    GameDef { id: "fallout4",  name: "Fallout 4",              app_id: 377160,  data_dir: "Data" },
    GameDef { id: "falloutnv", name: "Fallout: New Vegas",     app_id: 22380,   data_dir: "Data" },
    GameDef { id: "fallout3",  name: "Fallout 3 (GOTY)",       app_id: 22370,   data_dir: "Data" },
    GameDef { id: "oblivion",  name: "Oblivion",               app_id: 22330,   data_dir: "Data" },
    GameDef { id: "morrowind", name: "Morrowind",              app_id: 22320,   data_dir: "Data Files" },
    GameDef { id: "starfield", name: "Starfield",              app_id: 1716740, data_dir: "Data" },
    GameDef { id: "enderalse", name: "Enderal: Special Edition", app_id: 976620, data_dir: "Data" },
];

/// The supported-game catalog.
pub fn catalog() -> &'static [GameDef] {
    CATALOG
}

/// A game Steam reports as installed (whether or not it is one we support).
#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub app_id: u32,
    pub name: String,
    pub install_dir: String,
    pub library: PathBuf,
}

/// A supported game located on disk, ready to seed an Eidos instance.
#[derive(Debug, Clone)]
pub struct DetectedGame {
    pub def: &'static GameDef,
    /// `.../steamapps/common/<install_dir>`.
    pub install_path: PathBuf,
    /// The mod-deploy root, `install_path/<data_dir>`.
    pub data_path: PathBuf,
    /// The Proton prefix, if present (`.../steamapps/compatdata/<appid>`).
    pub compatdata: Option<PathBuf>,
    /// The name Steam shows for this install.
    pub steam_name: String,
}

/// The user's home directory (from `$HOME`).
pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

/// Every Steam library on the system, canonicalized and de-duplicated.
///
/// Several root paths (`~/.steam/steam`, `~/.local/share/Steam`, ...) often
/// symlink to the same place; canonicalizing collapses them. `libraryfolders.vdf`
/// adds libraries on other drives.
pub fn steam_libraries(home: &Path) -> Vec<PathBuf> {
    let roots = [
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        home.join(".steam/root"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ];
    let mut libs = Vec::new();
    for root in &roots {
        if let Ok(content) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            for path in kv_all(&content, "path") {
                add_lib(&mut libs, Path::new(&path));
            }
        }
        add_lib(&mut libs, root);
    }
    libs
}

/// Every installed app across all libraries (supported or not).
pub fn scan_installed(home: &Path) -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    for library in steam_libraries(home) {
        let Ok(entries) = fs::read_dir(library.join("steamapps")) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_manifest = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"));
            if !is_manifest {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else { continue };
            let (Some(app_id), Some(name), Some(install_dir)) = (
                kv_first(&content, "appid").and_then(|s| s.parse().ok()),
                kv_first(&content, "name"),
                kv_first(&content, "installdir"),
            ) else {
                continue;
            };
            apps.push(InstalledApp { app_id, name, install_dir, library: library.clone() });
        }
    }
    apps
}

/// Installed games that we support, with their on-disk paths resolved.
pub fn detect(home: &Path) -> Vec<DetectedGame> {
    scan_installed(home)
        .into_iter()
        .filter_map(|app| {
            let def = catalog().iter().find(|d| d.app_id == app.app_id)?;
            let install_path = app.library.join("steamapps/common").join(&app.install_dir);
            let data_path = install_path.join(def.data_dir);
            let compat = app.library.join("steamapps/compatdata").join(app.app_id.to_string());
            Some(DetectedGame {
                def,
                install_path,
                data_path,
                compatdata: compat.is_dir().then_some(compat),
                steam_name: app.name,
            })
        })
        .collect()
}

/// Add a library path if it holds a `steamapps` dir and is not already present
/// (compared by canonical path, so symlinked duplicates collapse).
fn add_lib(libs: &mut Vec<PathBuf>, path: &Path) {
    let canon = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if canon.join("steamapps").is_dir() && !libs.contains(&canon) {
        libs.push(canon);
    }
}

/// The two quoted strings on a Valve KeyValues line, e.g. `"appid" "489830"`.
fn quoted_pair(line: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = line.split('"').collect();
    // Quoted tokens land at odd indices: ["", key, sep, value, ...].
    (parts.len() >= 4).then(|| (parts[1], parts[3]))
}

/// First value for `key` in a KeyValues blob (`.acf` / `.vdf`).
fn kv_first(content: &str, key: &str) -> Option<String> {
    content
        .lines()
        .filter_map(quoted_pair)
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
}

/// Every value for `key` (e.g. the repeated `"path"` entries in libraryfolders).
fn kv_all(content: &str, key: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(quoted_pair)
        .filter(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct Tmp(PathBuf);
    impl Tmp {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("eidos-games-{}-{}", std::process::id(), n));
            fs::create_dir_all(&dir).unwrap();
            Tmp(dir)
        }
        fn write(&self, rel: &str, contents: &str) {
            let p = self.0.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, contents).unwrap();
        }
        fn mkdir(&self, rel: &str) {
            fs::create_dir_all(self.0.join(rel)).unwrap();
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn acf(appid: u32, name: &str, installdir: &str) -> String {
        format!(
            "\"AppState\"\n{{\n\t\"appid\"\t\t\"{appid}\"\n\t\"name\"\t\t\"{name}\"\n\t\"installdir\"\t\t\"{installdir}\"\n}}\n"
        )
    }

    #[test]
    fn parses_repeated_and_single_keys() {
        let vdf = "\t\t\"path\"\t\t\"/a\"\n\t\t\"path\"\t\t\"/b\"\n";
        assert_eq!(kv_all(vdf, "path"), vec!["/a", "/b"]);
        let acf = acf(489830, "Skyrim Special Edition", "Skyrim Special Edition");
        assert_eq!(kv_first(&acf, "appid").as_deref(), Some("489830"));
        assert_eq!(kv_first(&acf, "installdir").as_deref(), Some("Skyrim Special Edition"));
    }

    #[test]
    fn detects_supported_game_across_libraries() {
        let t = Tmp::new();
        let main = "home/.local/share/Steam";
        let lib2 = t.0.join("Games/Lib2");

        // Main library points at itself and at a second library on another path.
        t.mkdir(&format!("{main}/steamapps"));
        t.write(
            &format!("{main}/steamapps/libraryfolders.vdf"),
            &format!(
                "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
                t.0.join(main).display(),
                lib2.display()
            ),
        );
        // An app we do NOT support lives in the main library.
        t.write(&format!("{main}/steamapps/appmanifest_730.acf"), &acf(730, "Counter-Strike 2", "Counter-Strike Global Offensive"));

        // Skyrim SE lives on the second library, with its Data dir and a prefix.
        t.write("Games/Lib2/steamapps/appmanifest_489830.acf", &acf(489830, "Skyrim Special Edition", "Skyrim Special Edition"));
        t.mkdir("Games/Lib2/steamapps/common/Skyrim Special Edition/Data");
        t.mkdir("Games/Lib2/steamapps/compatdata/489830/pfx");

        // Detection stores canonical library paths; compare against those.
        let lib2c = fs::canonicalize(&lib2).unwrap();
        // HOME for steam_libraries() is <tmp>/home.
        let fake_home = t.0.join("home");

        let installed = scan_installed(&fake_home);
        assert_eq!(installed.len(), 2, "both apps scanned");

        let games = detect(&fake_home);
        assert_eq!(games.len(), 1, "only the supported one is detected");
        let g = &games[0];
        assert_eq!(g.def.id, "skyrimse");
        assert_eq!(g.steam_name, "Skyrim Special Edition");
        assert_eq!(g.install_path, lib2c.join("steamapps/common/Skyrim Special Edition"));
        assert_eq!(g.data_path, lib2c.join("steamapps/common/Skyrim Special Edition/Data"));
        assert_eq!(g.compatdata, Some(lib2c.join("steamapps/compatdata/489830")));
    }

    #[test]
    fn no_steam_no_games() {
        let t = Tmp::new();
        assert!(detect(&t.0.join("empty-home")).is_empty());
    }
}
