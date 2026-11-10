//! The game files an instance runs: the store install, or a copy of the game
//! kept in the instance folder.

use std::path::Path;

use eidos_games::DetectedGame;
use eidos_instance::{GameFiles, Instance};

/// The folder a freeze copies the store install into.
pub const FROZEN_COPY: &str = "Stock Game";

/// The game files `inst` runs `game` from.
pub fn game_files(game: &DetectedGame, inst: &Instance) -> GameFiles {
    inst.game_files(&game.install_path, &game.data_path)
}

/// Whether the game folder `root` can be swapped for a copy: false for a game
/// that keeps its plugin list in the game folder (Morrowind, Oblivion with
/// `bUseMyGamesDirectory=0`), where the launch binds profile state over it.
pub fn supported(game: &DetectedGame, root: &Path) -> bool {
    game.plugin_spec().is_none_or(|spec| {
        eidos_plugins::plugin_state_dir(&game.prefix().unwrap_or_default(), root, &spec) != root
    })
}

/// Whether `root` holds the game's executable, in any case, also in a
/// subfolder (`bin64/CrimsonDesert.exe`).
pub fn has_game_binary(game: &DetectedGame, root: &Path) -> bool {
    let binary = game.def.game_binary;
    binary.is_empty()
        || binary
            .split('/')
            .try_fold(root.to_path_buf(), |dir, part| {
                std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
                    let n = e.file_name();
                    n.to_str()
                        .is_some_and(|n| n.eq_ignore_ascii_case(part))
                        .then(|| e.path())
                })
            })
            .is_some()
}

/// The folder of a Wabbajack list's game copy in `root` that `game` can run
/// from: it holds the game's executable, and neither it nor the store install
/// keeps the plugin list in the game folder. Writes nothing.
pub fn list_copy(game: &DetectedGame, root: &Path) -> Option<String> {
    let folder = Instance::stock_game_folder(root)?;
    let copy = root.join(&folder);
    (has_game_binary(game, &copy) && supported(game, &game.install_path) && supported(game, &copy))
        .then_some(folder)
}

/// Point a list folder adopted for the first time at its game copy, the way
/// MO2 runs the list. The folder's name, or None with nothing written.
pub fn adopt_list_copy(game: &DetectedGame, inst: &Instance) -> Option<String> {
    let folder = list_copy(game, &inst.root)?;
    let adopted = inst
        .try_lock("adopting the list's game copy")
        .and_then(|_lock| inst.edit_manifest(|m| m.base_game = Some(folder.clone())));
    match adopted {
        Ok(()) => Some(folder),
        Err(error) => {
            eidos_log::warn!("Cannot run this list from '{folder}': {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eidos_instance::InstanceKind;
    use std::fs;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eidos-stock-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("game")).unwrap();
        d
    }

    fn game(id: &str, install: PathBuf) -> DetectedGame {
        DetectedGame {
            def: eidos_games::GameDef::for_id(id).unwrap(),
            source: Default::default(),
            data_path: install.join("Data"),
            install_path: install,
            compatdata: None,
            steam_name: id.into(),
        }
    }

    fn list(t: &Path, id: &str) -> Instance {
        let inst = Instance::portable(t.join("list"));
        fs::create_dir_all(inst.root.join("Stock Game")).unwrap();
        inst.ensure_manifest(id, InstanceKind::Portable).unwrap();
        inst
    }

    #[test]
    fn adopting_a_list_folder_runs_its_copy_only_when_it_holds_the_game() {
        let t = temp("adopt");
        let game = game("skyrimse", t.join("game"));
        let inst = list(&t, "skyrimse");
        assert_eq!(adopt_list_copy(&game, &inst), None);
        assert_eq!(inst.read_manifest().unwrap().base_game, None);
        fs::write(inst.root.join("Stock Game/SKYRIMSE.EXE"), b"x").unwrap();
        assert_eq!(adopt_list_copy(&game, &inst).as_deref(), Some("Stock Game"));
        assert_eq!(
            inst.read_manifest().unwrap().base_game.as_deref(),
            Some("Stock Game")
        );
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn the_game_binary_is_found_in_a_subfolder_in_any_case() {
        let t = temp("subfolder");
        let game = game("crimsondesert", t.join("game"));
        let copy = t.join("Stock Game");
        fs::create_dir_all(copy.join("BIN64")).unwrap();
        assert!(!has_game_binary(&game, &copy));
        fs::write(copy.join("BIN64/crimsondesert.exe"), b"x").unwrap();
        assert!(has_game_binary(&game, &copy));
        let _ = fs::remove_dir_all(t);
    }

    #[test]
    fn morrowind_never_adopts_a_copy() {
        let t = temp("morrowind");
        let game = game("morrowind", t.join("game"));
        let inst = list(&t, "morrowind");
        fs::write(inst.root.join("Stock Game/Morrowind.exe"), b"x").unwrap();
        assert_eq!(adopt_list_copy(&game, &inst), None);
        assert_eq!(inst.read_manifest().unwrap().base_game, None);
        let _ = fs::remove_dir_all(t);
    }
}
