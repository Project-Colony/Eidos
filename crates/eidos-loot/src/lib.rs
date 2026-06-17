//! LOOT-based plugin sorting for Eidos, via the pure-Rust `libloot` crate (the
//! LOOT team's 2025 Rust port). Eidos resolves the plugin set itself
//! (`eidos-plugins`); this crate runs LOOT's real graph sort over it using a
//! per-game masterlist, then hands the ordered names back. The caller reorders its
//! own `PluginList` and re-runs the existing invariant pass, so LOOT's order is a
//! suggestion that Eidos's master-before-dependent guard always backstops.

use std::fs;
use std::path::{Path, PathBuf};

use libloot::{Game, GameType};

/// The masterlist metadata-syntax branch libloot 0.29 reads (LOOT versions the
/// per-game masterlist repos by a `v<major>.<minor>` branch). Bump alongside libloot.
const MASTERLIST_BRANCH: &str = "v0.26";

#[derive(Debug)]
pub enum LootError {
    /// The game has no LOOT support wired (timestamp-ordered games, or unmapped).
    Unsupported(String),
    Io(std::io::Error),
    /// A masterlist/prelude download failed.
    Fetch(String),
    /// A libloot operation failed (game handle, masterlist parse, sort, ...).
    Loot(String),
}

impl std::fmt::Display for LootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LootError::Unsupported(g) => write!(f, "LOOT sorting is not supported for {g}"),
            LootError::Io(e) => write!(f, "{e}"),
            LootError::Fetch(e) => write!(f, "masterlist download failed: {e}"),
            LootError::Loot(e) => write!(f, "libloot: {e}"),
        }
    }
}

impl std::error::Error for LootError {}

impl From<std::io::Error> for LootError {
    fn from(e: std::io::Error) -> Self {
        LootError::Io(e)
    }
}

/// LOOT support for an Eidos game id: the libloot `GameType` and the masterlist
/// repo slug under `github.com/loot/<repo>`. `None` for games LOOT can't sort
/// (Morrowind/Oblivion are timestamp-ordered; Eidos doesn't manage those anyway).
pub fn loot_support(game_id: &str) -> Option<(GameType, &'static str)> {
    Some(match game_id {
        "skyrimse" => (GameType::SkyrimSE, "skyrimse"),
        "skyrim" => (GameType::Skyrim, "skyrim"),
        "skyrimvr" => (GameType::SkyrimVR, "skyrimvr"),
        "fallout4" => (GameType::Fallout4, "fallout4"),
        "fallout4vr" => (GameType::Fallout4VR, "fallout4vr"),
        "falloutnv" => (GameType::FalloutNV, "falloutnv"),
        "fallout3" => (GameType::Fallout3, "fallout3"),
        "starfield" => (GameType::Starfield, "starfield"),
        _ => return None,
    })
}

/// Whether Eidos can LOOT-sort this game.
pub fn is_supported(game_id: &str) -> bool {
    loot_support(game_id).is_some()
}

/// Ensure the masterlist + prelude are cached under `cache_dir`, fetching from
/// `github.com/loot/<repo>` (and the shared prelude) when missing or `update`.
/// Returns their paths.
pub fn ensure_masterlist(
    repo: &str,
    cache_dir: &Path,
    update: bool,
) -> Result<(PathBuf, PathBuf), LootError> {
    let masterlist = cache_dir.join("masterlist.yaml");
    let prelude = cache_dir.join("prelude.yaml");
    fs::create_dir_all(cache_dir)?;
    if update || !masterlist.is_file() {
        fetch(
            &format!("https://raw.githubusercontent.com/loot/{repo}/{MASTERLIST_BRANCH}/masterlist.yaml"),
            &masterlist,
        )?;
    }
    if update || !prelude.is_file() {
        fetch(
            &format!("https://raw.githubusercontent.com/loot/prelude/{MASTERLIST_BRANCH}/prelude.yaml"),
            &prelude,
        )?;
    }
    Ok((masterlist, prelude))
}

fn fetch(url: &str, dest: &Path) -> Result<(), LootError> {
    let body = ureq::get(url)
        .call()
        .map_err(|e| LootError::Fetch(format!("{url}: {e}")))?
        .into_string()
        .map_err(|e| LootError::Fetch(format!("{url}: {e}")))?;
    fs::write(dest, body)?;
    Ok(())
}

/// Run LOOT's sort and return the optimised order of `plugins` (by name).
///
/// `plugins` is `(name, real-path)` for each plugin to sort, in current order
/// (Eidos resolves the real paths across mod folders / Overwrite). `game_path` is
/// the game install dir; `game_local_path` is the prefix's AppData/Local game dir
/// (where `plugins.txt`/`loadorder.txt` live). Conditions in the masterlist are
/// evaluated against `game_path`.
pub fn sort(
    game_id: &str,
    game_path: &Path,
    game_local_path: &Path,
    plugins: &[(String, PathBuf)],
    masterlist: &Path,
    prelude: &Path,
    userlist: Option<&Path>,
) -> Result<Vec<String>, LootError> {
    let (game_type, _repo) =
        loot_support(game_id).ok_or_else(|| LootError::Unsupported(game_id.to_string()))?;

    let mut game = Game::with_local_path(game_type, game_path, game_local_path)
        .map_err(|e| LootError::Loot(e.to_string()))?;

    {
        let db = game.database();
        let mut db = db.write().map_err(|_| LootError::Loot("database lock poisoned".into()))?;
        db.load_masterlist_with_prelude(masterlist, prelude)
            .map_err(|e| LootError::Loot(e.to_string()))?;
        if let Some(ul) = userlist {
            if ul.is_file() {
                db.load_userlist(ul).map_err(|e| LootError::Loot(e.to_string()))?;
            }
        }
    }

    let paths: Vec<&Path> = plugins.iter().map(|(_, p)| p.as_path()).collect();
    game.load_plugin_headers(&paths).map_err(|e| LootError::Loot(e.to_string()))?;
    game.load_current_load_order_state().map_err(|e| LootError::Loot(e.to_string()))?;

    let names: Vec<&str> = plugins.iter().map(|(n, _)| n.as_str()).collect();
    game.sort_plugins(&names).map_err(|e| LootError::Loot(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_support_mapping() {
        assert!(matches!(loot_support("skyrimse"), Some((GameType::SkyrimSE, "skyrimse"))));
        assert!(matches!(loot_support("fallout4"), Some((GameType::Fallout4, "fallout4"))));
        assert!(loot_support("morrowind").is_none()); // timestamp-ordered, unsupported
        assert!(loot_support("nonsense").is_none());
        assert!(is_supported("skyrimse") && !is_supported("oblivion"));
    }
}
