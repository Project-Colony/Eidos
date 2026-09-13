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

mod paths;
mod stores;
pub use stores::{select_installation, GameSource, Store};
mod proton;
pub use proton::{is_flatpak_steam, library_path, proton_command, steam_root, ProtonRun};

/// A supported game. The catalog now lives in the shared `eidos-gamedef`
/// descriptor; re-exported so detection callers keep using `eidos_games::GameDef`.
pub use eidos_gamedef::{GameDef, LoadOrder};

/// The supported-game catalog (every game defined in `eidos-gamedef`).
pub fn catalog() -> &'static [GameDef] {
    eidos_gamedef::all()
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
    pub source: GameSource,
    /// Canonical directory on disk for the selected installation.
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
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// Every Steam library on the system, canonicalized and de-duplicated.
///
/// Several root paths (`~/.steam/steam`, `~/.local/share/Steam`, ...) often
/// symlink to the same place; canonicalizing collapses them. `libraryfolders.vdf`
/// adds libraries on other drives.
pub fn steam_libraries(home: &Path) -> Vec<PathBuf> {
    let mut libs = Vec::new();
    for root in steam_roots(home) {
        // Steam has shipped both spellings of this file over the years, and a
        // client that has never added a second library may only have the
        // singular one. Reading both costs nothing and avoids a "no games
        // found" that the user cannot explain.
        // Three known locations: the modern plural under steamapps, the singular
        // some (and older) clients write, and the copy under config/ that current
        // clients also maintain. Reading all three costs three failed opens and
        // avoids a "no games found" the user cannot explain.
        for rel in [
            "steamapps/libraryfolders.vdf",
            "steamapps/libraryfolder.vdf",
            "config/libraryfolders.vdf",
        ] {
            if let Ok(content) = fs::read_to_string(root.join(rel)) {
                for path in kv_all(&content, "path") {
                    add_lib(&mut libs, Path::new(&path));
                }
            }
        }
        add_lib(&mut libs, &root);
    }
    libs
}

/// Every plausible Steam install root on this machine, in preference order.
///
/// Distributions disagree about where Steam lives: the classic
/// `~/.steam/steam` symlink, the modern `~/.local/share/Steam`, Debian and
/// Ubuntu's `~/.steam/debian-installation`, and the Flatpak's own data dir. A
/// missing root here surfaces to the user as "Eidos cannot find my game", so
/// the list is deliberately generous - a non-existent path costs one failed
/// stat.
pub fn steam_roots(home: &Path) -> Vec<PathBuf> {
    [
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        home.join(".steam/root"),
        home.join(".steam/debian-installation"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/data/Steam"),
        home.join("snap/steam/common/.local/share/Steam"),
    ]
    .into_iter()
    .filter(|p| p.is_dir())
    .collect()
}

/// Every installed app across all libraries (supported or not).
pub fn scan_installed(home: &Path) -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    for library in steam_libraries(home) {
        let Ok(entries) = fs::read_dir(library.join("steamapps")) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let manifest_id = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("appmanifest_")?.strip_suffix(".acf"))
                .and_then(|id| id.parse::<u32>().ok())
                .filter(|id| *id != 0);
            let Some(manifest_id) = manifest_id else {
                continue;
            };
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let (Some(app_id), Some(name), Some(install_dir)) = (
                kv_first(&content, "appid")
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|id| *id == manifest_id),
                kv_first(&content, "name"),
                kv_first(&content, "installdir"),
            ) else {
                continue;
            };
            apps.push(InstalledApp {
                app_id,
                name,
                install_dir,
                library: library.clone(),
            });
        }
    }
    apps
}

/// Installed games that we support, with their on-disk paths resolved.
pub fn detect(home: &Path) -> Vec<DetectedGame> {
    let mut games: Vec<_> = scan_installed(home)
        .into_iter()
        .filter_map(|app| {
            let def = catalog()
                .iter()
                .find(|d| d.steam_app_id != 0 && d.steam_app_id == app.app_id)?;
            let relative = Path::new(&app.install_dir);
            if relative.as_os_str().is_empty()
                || !relative
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_)))
            {
                return None;
            }
            let install_path =
                canonical_directory(&app.library.join("steamapps/common").join(relative))?;
            let data_path = install_path.join(def.data_dir);
            let compat = app
                .library
                .join("steamapps/compatdata")
                .join(app.app_id.to_string());
            Some(DetectedGame {
                def,
                source: GameSource::Steam,
                install_path,
                data_path,
                compatdata: compat.is_dir().then_some(compat),
                steam_name: app.name,
            })
        })
        .collect();
    games.extend(stores::detect_stores(home));
    games
}

/// Store metadata may point through symlinks, but never at the filesystem root.
fn canonical_directory(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let canonical = path.canonicalize().ok()?;
    (canonical.parent().is_some() && canonical.is_dir()).then_some(canonical)
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
pub(crate) fn quoted_pair(line: &str) -> Option<(&str, &str)> {
    if !line.trim_start().starts_with('"') {
        return None;
    }
    let parts: Vec<&str> = line.split('"').collect();
    // Quoted tokens land at odd indices: ["", key, sep, value, ...].
    (parts.len() >= 5).then(|| (parts[1], parts[3]))
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
            let dir =
                std::env::temp_dir().join(format!("eidos-games-{}-{}", std::process::id(), n));
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
        assert_eq!(
            kv_first(&acf, "installdir").as_deref(),
            Some("Skyrim Special Edition")
        );
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
        t.write(
            &format!("{main}/steamapps/appmanifest_730.acf"),
            &acf(730, "Counter-Strike 2", "Counter-Strike Global Offensive"),
        );

        // Skyrim SE lives on the second library, with its Data dir and a prefix.
        t.write(
            "Games/Lib2/steamapps/appmanifest_489830.acf",
            &acf(489830, "Skyrim Special Edition", "Skyrim Special Edition"),
        );
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
        assert_eq!(
            g.install_path,
            lib2c.join("steamapps/common/Skyrim Special Edition")
        );
        assert_eq!(
            g.data_path,
            lib2c.join("steamapps/common/Skyrim Special Edition/Data")
        );
        assert_eq!(
            g.compatdata,
            Some(lib2c.join("steamapps/compatdata/489830"))
        );
    }

    #[test]
    fn heroic_and_legendary_copies_remain_distinct_from_steam() {
        let t = Tmp::new();
        let steam = "home/.local/share/Steam";
        t.write(
            &format!("{steam}/steamapps/appmanifest_489830.acf"),
            &acf(489830, "Skyrim SE", "Skyrim"),
        );
        t.mkdir(&format!("{steam}/steamapps/common/Skyrim/Data"));
        let gog = t.0.join("games/gog");
        let epic = t.0.join("games/epic");
        for path in [&gog, &epic] {
            fs::create_dir_all(path.join("Data")).unwrap();
            fs::write(path.join("SkyrimSE.exe"), b"fixture").unwrap();
        }
        t.write(
            "home/.config/heroic/gog_store/installed.json",
            &serde_json::json!({
                "installed": [{"appName":"1711230643", "install_path":gog, "platform":"windows"}]
            })
            .to_string(),
        );
        t.write("home/.var/app/com.heroicgameslauncher.hgl/config/legendary/installed.json", &serde_json::json!({
            "ac82db5035584c7f8a2c548d98c86b2c": {"app_name":"ac82db5035584c7f8a2c548d98c86b2c", "install_path":epic, "title":"Skyrim SE"},
            "unknown": {"app_name":"unknown", "install_path":gog, "title":"Skyrim SE"}
        }).to_string());
        let games = detect(&t.0.join("home"));
        assert_eq!(
            games.len(),
            3,
            "all supported installations, never title-only matches"
        );
        assert!(games.iter().all(|g| g.def.id == "skyrimse"));
        assert!(
            games.iter().all(|g| g.compatdata.is_none()),
            "no invented Steam prefixes"
        );
    }

    #[test]
    fn steam_manifests_require_a_nonzero_matching_app_id() {
        for (filename, declared) in [(0, 0), (489830, 0), (489830, 377160)] {
            let t = Tmp::new();
            t.write(
                &format!("home/.local/share/Steam/steamapps/appmanifest_{filename}.acf"),
                &acf(declared, "Game", "Game"),
            );
            assert!(
                scan_installed(&t.0.join("home")).is_empty(),
                "accepted appmanifest_{filename}.acf declaring {declared}"
            );
        }
    }

    #[test]
    fn steam_manifests_do_not_take_identity_from_comments_or_unclosed_values() {
        let t = Tmp::new();
        let manifest = "home/.local/share/Steam/steamapps/appmanifest_489830.acf";
        for content in [
            acf(489830, "Skyrim", "Skyrim").replace("\"489830\"", "\"489830"),
            "// \"appid\" \"489830\"\n\"name\" \"Skyrim\"\n\"installdir\" \"Skyrim\"\n".into(),
        ] {
            t.write(manifest, &content);
            assert!(
                scan_installed(&t.0.join("home")).is_empty(),
                "accepted {content}"
            );
        }
    }

    #[test]
    fn steam_detection_rejects_missing_and_escaping_install_directories() {
        for install in [
            "",
            ".",
            "..",
            "../common/Game",
            "/",
            "/tmp",
            "Missing",
            "RootLink",
        ] {
            let t = Tmp::new();
            let common = "home/.local/share/Steam/steamapps/common";
            t.mkdir(&format!("{common}/Game"));
            std::os::unix::fs::symlink("/", t.0.join(format!("{common}/RootLink"))).unwrap();
            t.write(
                "home/.local/share/Steam/steamapps/appmanifest_489830.acf",
                &acf(489830, "Skyrim", install),
            );
            assert!(detect(&t.0.join("home")).is_empty(), "accepted {install:?}");
        }
    }

    #[test]
    fn steam_detection_uses_canonical_install_identity() {
        let t = Tmp::new();
        t.mkdir("games/Skyrim/Data");
        t.mkdir("home/.local/share/Steam/steamapps/common");
        let real = fs::canonicalize(t.0.join("games/Skyrim")).unwrap();
        std::os::unix::fs::symlink(
            &real,
            t.0.join("home/.local/share/Steam/steamapps/common/Skyrim"),
        )
        .unwrap();
        t.write(
            "home/.local/share/Steam/steamapps/appmanifest_489830.acf",
            &acf(489830, "Skyrim", "Skyrim"),
        );
        let games = detect(&t.0.join("home"));
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].install_path, real);
        assert_eq!(games[0].data_path, real.join("Data"));
        let saved = games[0].selection_id();
        t.mkdir("home/.local/share/Steam/steamapps/compatdata/489830/pfx");
        let later = detect(&t.0.join("home"));
        assert_eq!(
            later[0].selection_id(),
            saved,
            "creating a Steam prefix must not change identity"
        );
        assert!(select_installation(&later, "skyrimse", Some(&saved)).is_some());
    }

    #[test]
    fn external_detection_rejects_root_and_malformed_dlc_entries() {
        let t = Tmp::new();
        t.mkdir("game/Data");
        std::os::unix::fs::symlink("/", t.0.join("root-link")).unwrap();
        let install = t.0.join("game");
        let id = "ac82db5035584c7f8a2c548d98c86b2c";
        let entries = [
            serde_json::json!({"install_path":"/"}),
            serde_json::json!({"install_path":t.0.join("root-link")}),
            serde_json::json!({"install_path":install, "app_name":17}),
            serde_json::json!({"install_path":install, "app_name":"different"}),
            serde_json::json!({"install_path":install, "is_dlc":false, "isDlc":true}),
            serde_json::json!({"install_path":install, "is_dlc":"false"}),
        ];
        for entry in entries {
            t.write(
                "home/.config/legendary/installed.json",
                &serde_json::json!({id:entry}).to_string(),
            );
            assert!(detect(&t.0.join("home")).is_empty(), "accepted {entry}");
        }
    }

    #[test]
    fn same_payload_with_distinct_external_prefixes_remains_selectable() {
        let t = Tmp::new();
        t.mkdir("game/Data");
        let install = fs::canonicalize(t.0.join("game")).unwrap();
        let roots = [
            "home/.config/heroic",
            "home/.var/app/com.heroicgameslauncher.hgl/config/heroic",
        ];
        for (i, root) in roots.iter().enumerate() {
            t.mkdir(&format!("prefix-{i}"));
            t.write(&format!("{root}/gog_store/installed.json"), &serde_json::json!({
                "installed":[{"appName":"1711230643", "install_path":install, "platform":"windows"}]
            }).to_string());
            t.write(&format!("{root}/GamesConfig/1711230643.json"), &serde_json::json!({
                "1711230643":{"winePrefix":t.0.join(format!("prefix-{i}")),"wineVersion":{"type":"wine"}}
            }).to_string());
        }
        let games = detect(&t.0.join("home"));
        assert_eq!(
            games.len(),
            2,
            "different selected prefixes must not collapse"
        );
        assert_ne!(games[0].selection_id(), games[1].selection_id());
        assert_ne!(games[0].prefix(), games[1].prefix());
        for game in &games {
            let key = game.selection_id();
            assert_eq!(
                select_installation(&games, "skyrimse", Some(&key))
                    .unwrap()
                    .prefix(),
                game.prefix()
            );
        }
        let legacy = serde_json::json!(["GOG:1711230643", install]).to_string();
        assert!(select_installation(&games, "skyrimse", Some(&legacy)).is_none());
        assert!(select_installation(&games[..1], "skyrimse", Some(&legacy)).is_some());
    }

    #[test]
    fn external_prefix_aliases_collapse_and_filesystem_root_is_never_a_prefix() {
        let t = Tmp::new();
        t.mkdir("game/Data");
        t.mkdir("wine-prefix");
        let prefix = fs::canonicalize(t.0.join("wine-prefix")).unwrap();
        let alias = t.0.join("prefix-alias");
        std::os::unix::fs::symlink(&prefix, &alias).unwrap();
        let roots = [
            "home/.config/heroic",
            "home/.var/app/com.heroicgameslauncher.hgl/config/heroic",
        ];
        for (root, configured) in roots.iter().zip([&prefix, &alias]) {
            t.write(
                &format!("{root}/gog_store/installed.json"),
                &serde_json::json!({
                    "installed":[{"appName":"1711230643", "install_path":t.0.join("game")}]
                })
                .to_string(),
            );
            t.write(
                &format!("{root}/GamesConfig/1711230643.json"),
                &serde_json::json!({
                    "winePrefix":configured,"wineVersion":{"type":"wine"}
                })
                .to_string(),
            );
        }
        let games = detect(&t.0.join("home"));
        assert_eq!(
            games.len(),
            1,
            "aliases of the same prefix are one installation"
        );
        assert_eq!(games[0].prefix(), Some(prefix));
        for root in roots {
            t.write(
                &format!("{root}/GamesConfig/1711230643.json"),
                &serde_json::json!({
                    "winePrefix":"/","wineVersion":{"type":"wine"}
                })
                .to_string(),
            );
        }
        assert!(
            detect(&t.0.join("home"))
                .iter()
                .all(|game| game.prefix().is_none()),
            "filesystem root cannot become the Wine prefix"
        );
    }

    #[test]
    fn no_steam_no_games() {
        let t = Tmp::new();
        assert!(detect(&t.0.join("empty-home")).is_empty());
    }
}
