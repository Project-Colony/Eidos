//! A game copy the instance runs from that cannot run: launches are refused.

use eidos_games::DetectedGame;
use eidos_instance::{GameFiles, Instance};

use super::{Finding, Level};
use crate::stock_game::{has_game_binary, supported};

/// At most one Problem, empty without a copy: the copy folder is gone, it does
/// not hold the game's executable, or the game keeps its plugin list in the
/// game folder.
pub fn findings(game: &DetectedGame, inst: &Instance, files: &GameFiles) -> Vec<Finding> {
    let Some(folder) = files.copy.clone() else {
        return Vec::new();
    };
    let (code, args) = if !files.root.is_dir() {
        (
            "game_files_missing",
            vec![
                ("folder", folder),
                ("root", inst.root.display().to_string()),
            ],
        )
    } else if !has_game_binary(game, &files.root) {
        (
            "game_files_not_a_copy",
            vec![
                ("folder", folder),
                ("binary", game.def.game_binary.to_owned()),
            ],
        )
    } else if !supported(game, &game.install_path) || !supported(game, &files.root) {
        (
            "game_files_root_mode",
            vec![("game", game.def.name.to_owned())],
        )
    } else {
        return Vec::new();
    };
    vec![Finding {
        code,
        level: Level::Problem,
        args,
    }]
}

/// The CLI's English line for a launch refused by `f`.
pub fn refusal(f: &Finding) -> String {
    let arg = |k| f.arg(k).unwrap_or_default();
    match f.code {
        "game_files_missing" => format!(
            "this instance runs the game from '{}', which is missing from {}; put it back, \
             or run eidos stock-game <instance> store",
            arg("folder"),
            arg("root")
        ),
        "game_files_not_a_copy" => format!(
            "'{}' has no {}, so it is not a copy of the game; run eidos stock-game <instance> \
             store, or use another folder",
            arg("folder"),
            arg("binary")
        ),
        _ => format!(
            "{} keeps its plugin list in the game folder, which Eidos does not support with a \
             game copy yet; run eidos stock-game <instance> store",
            arg("game")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stock_game;
    use eidos_games::DetectedGame;
    use eidos_instance::{Instance, InstanceKind};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn fixture(tag: &str, id: &str, copy: bool) -> (PathBuf, DetectedGame, Instance) {
        let t = std::env::temp_dir().join(format!("eidos-gf-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&t);
        fs::create_dir_all(t.join("game")).unwrap();
        let inst = Instance::portable(t.join("i"));
        inst.create().unwrap();
        inst.ensure_manifest(id, InstanceKind::Portable).unwrap();
        if copy {
            inst.edit_manifest(|m| m.base_game = Some("Stock Game".into()))
                .unwrap();
        }
        let def = eidos_games::GameDef::for_id(id).unwrap();
        let game = DetectedGame {
            def,
            install_path: t.join("game"),
            data_path: t.join("game").join(def.data_dir),
            compatdata: None,
            source: Default::default(),
            steam_name: String::new(),
        };
        (t, game, inst)
    }

    fn of(game: &DetectedGame, inst: &Instance) -> Vec<Finding> {
        findings(game, inst, &stock_game::game_files(game, inst))
    }

    #[test]
    fn a_missing_copy_folder_refuses_the_launch_and_names_it() {
        let (t, game, inst) = fixture("missing", "skyrimse", true);
        let f = of(&game, &inst);
        assert_eq!(f.len(), 1);
        assert_eq!(
            (f[0].code, f[0].arg("folder")),
            ("game_files_missing", Some("Stock Game"))
        );
        assert_eq!(f[0].level, Level::Problem);
        assert!(
            refusal(&f[0]).contains("'Stock Game'"),
            "{}",
            refusal(&f[0])
        );
        let (t2, game2, store) = fixture("missing-store", "skyrimse", false);
        assert!(of(&game2, &store).is_empty());
        for d in [t, t2] {
            fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn a_folder_without_the_game_binary_is_not_a_copy() {
        let (t, game, inst) = fixture("not-a-copy", "skyrimse", true);
        fs::create_dir_all(inst.root.join("Stock Game")).unwrap();
        let f = of(&game, &inst);
        assert_eq!(
            (f.len(), f[0].code, f[0].arg("binary")),
            (1, "game_files_not_a_copy", Some("SkyrimSE.exe"))
        );
        assert!(
            refusal(&f[0]).contains("no SkyrimSE.exe"),
            "{}",
            refusal(&f[0])
        );
        fs::write(inst.root.join("Stock Game/SkyrimSE.exe"), b"x").unwrap();
        assert!(of(&game, &inst).is_empty());
        fs::remove_dir_all(t).unwrap();
    }

    #[test]
    fn morrowind_cannot_run_from_a_copy() {
        let (t, game, inst) = fixture("morrowind", "morrowind", true);
        fs::create_dir_all(inst.root.join("Stock Game")).unwrap();
        fs::write(inst.root.join("Stock Game/Morrowind.exe"), b"x").unwrap();
        let f = of(&game, &inst);
        assert_eq!((f.len(), f[0].code), (1, "game_files_root_mode"));
        assert_eq!(f[0].arg("game"), Some(game.def.name));
        assert!(Path::new(&inst.root.join("Stock Game")).is_dir());
        fs::remove_dir_all(t).unwrap();
    }
}
