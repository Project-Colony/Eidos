//! A10: the script extender is there, and its runtime DLL matches the game.

use std::path::{Path, PathBuf};

use eidos_gamefeatures::preflight::{merged_root, runtime_version, ExtenderFacts};
use eidos_games::DetectedGame;
use eidos_instance::GameBuild;

use super::{Finding, Level};

pub fn extender_findings(f: &ExtenderFacts) -> Vec<Finding> {
    let mut out = Vec::new();
    if !f.loader_found && f.plugin_dlls > 0 {
        let mut args = vec![
            ("loader", f.loader.to_string()),
            ("dir", f.plugins_dir.to_string()),
            ("n", f.plugin_dlls.to_string()),
            ("page", f.page.to_string()),
        ];
        // No runtime for the 32-bit games (Skyrim LE, Fallout 3, New Vegas,
        // Oblivion) or an unreadable executable; the card then names no version.
        args.extend(f.runtime.map(|v| ("runtime", v.to_string())));
        out.push(Finding {
            code: "extender_missing",
            level: Level::Problem,
            args,
        });
    }
    if let (true, Some(dll), Some(runtime)) = (f.loader_found, &f.runtime_dll, f.runtime) {
        if !f.runtime_dll_found {
            out.push(Finding {
                code: "extender_runtime_missing",
                level: Level::Problem,
                args: vec![
                    ("loader", f.loader.to_string()),
                    ("dll", dll.clone()),
                    ("runtime", runtime.to_string()),
                    ("page", f.page.to_string()),
                ],
            });
        }
    }
    out
}

/// The game's executable version (the winning `Root/` copy counts) and, for a
/// Steam copy, Steam's build id.
pub fn current_build(
    game: &DetectedGame,
    mods: &[(String, PathBuf)],
    overwrite: &Path,
) -> GameBuild {
    let binary = game.def.game_binary;
    let runtime = (!binary.is_empty())
        .then(|| runtime_version(&merged_root(&game.install_path, mods, overwrite), binary).ok())
        .flatten()
        .map(|v| v.to_string());
    let build_id = game
        .is_steam()
        .then(|| eidos_games::steam_build_id(&game.install_path, game.def.steam_app_id))
        .flatten();
    GameBuild { runtime, build_id }
}

/// The game changed since the last launch that exited cleanly. Runtimes decide
/// when both are known (a content-only Steam update keeps the executable and
/// leaves script-extender plugins working); build ids decide otherwise.
pub fn game_update_findings(
    game_name: &str,
    last_good: Option<&GameBuild>,
    now: &GameBuild,
) -> Vec<Finding> {
    let Some(old) = last_good else {
        return Vec::new();
    };
    let changed = match (&old.runtime, &now.runtime) {
        (Some(a), Some(b)) => a != b,
        _ => matches!((&old.build_id, &now.build_id), (Some(a), Some(b)) if a != b),
    };
    if !changed {
        return Vec::new();
    }
    let show = |v: &Option<String>| v.clone().unwrap_or_else(|| "unknown".into());
    vec![Finding {
        code: "game_updated",
        level: Level::Advice,
        args: vec![
            ("game", game_name.to_string()),
            ("from", show(&old.runtime)),
            ("to", show(&now.runtime)),
            ("build_from", show(&old.build_id)),
            ("build_to", show(&now.build_id)),
        ],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use eidos_gamefeatures::preflight::Version;

    fn b(runtime: &str, id: &str) -> GameBuild {
        GameBuild {
            runtime: Some(runtime.into()),
            build_id: Some(id.into()),
        }
    }

    #[test]
    fn a_new_runtime_since_the_last_good_launch_is_reported() {
        let (b1, b2) = (b("1.6.1170.0", "1"), b("1.6.1179.0", "2"));
        let f = game_update_findings("Skyrim Special Edition", Some(&b1), &b2);
        assert_eq!((f[0].code, f[0].level), ("game_updated", Level::Advice));
        assert_eq!(
            (f[0].arg("from"), f[0].arg("to")),
            (Some("1.6.1170.0"), Some("1.6.1179.0"))
        );
        assert_eq!(
            (f[0].arg("build_from"), f[0].arg("build_to")),
            (Some("1"), Some("2"))
        );
    }

    #[test]
    fn a_content_update_with_the_same_exe_is_quiet() {
        let b1 = b("1.6.1170.0", "1");
        let same_exe = GameBuild {
            build_id: Some("3".into()),
            ..b1.clone()
        };
        assert!(game_update_findings("x", Some(&b1), &same_exe).is_empty());
    }

    #[test]
    fn without_runtimes_the_build_ids_decide() {
        let a = GameBuild {
            runtime: None,
            build_id: Some("1".into()),
        };
        let c = GameBuild {
            runtime: None,
            build_id: Some("2".into()),
        };
        assert_eq!(
            game_update_findings("x", Some(&a), &c)[0].arg("from"),
            Some("unknown")
        );
    }

    #[test]
    fn no_clean_launch_yet_says_nothing() {
        assert!(game_update_findings("x", None, &b("1.6.1179.0", "2")).is_empty());
    }

    fn facts() -> ExtenderFacts {
        ExtenderFacts {
            loader: "skse64_loader.exe",
            loader_found: true,
            plugins_dir: "SKSE/Plugins",
            page: "https://skse.silverlock.org/",
            plugin_dlls: 3,
            runtime: Some(Version([1, 6, 1179, 0])),
            runtime_dll: Some("skse64_1_6_1179.dll".into()),
            runtime_dll_found: true,
        }
    }

    #[test]
    fn a_matching_runtime_dll_says_nothing() {
        assert!(extender_findings(&facts()).is_empty());
    }

    #[test]
    fn a_loader_without_its_runtime_dll_is_a_problem() {
        let f = extender_findings(&ExtenderFacts {
            runtime_dll_found: false,
            ..facts()
        });
        assert_eq!(f.len(), 1);
        assert_eq!(
            (f[0].code, f[0].level),
            ("extender_runtime_missing", Level::Problem)
        );
        assert_eq!(f[0].arg("dll"), Some("skse64_1_6_1179.dll"));
        assert_eq!(f[0].arg("runtime"), Some("1.6.1179.0"));
    }

    #[test]
    fn extender_findings_carry_the_download_page() {
        let f = extender_findings(&ExtenderFacts {
            runtime_dll_found: false,
            ..facts()
        });
        assert_eq!(f[0].arg("page"), Some("https://skse.silverlock.org/"));
    }

    #[test]
    fn plugins_without_a_loader_are_a_problem() {
        let f = extender_findings(&ExtenderFacts {
            loader_found: false,
            plugin_dlls: 2,
            ..facts()
        });
        assert_eq!(f[0].code, "extender_missing");
        assert_eq!(f[0].arg("n"), Some("2"));
        assert_eq!(f[0].arg("loader"), Some("skse64_loader.exe"));
        assert_eq!(f[0].arg("runtime"), Some("1.6.1179.0"));
        let unread = extender_findings(&ExtenderFacts {
            loader_found: false,
            plugin_dlls: 2,
            runtime: None,
            ..facts()
        });
        assert_eq!(unread[0].code, "extender_missing");
        assert_eq!(unread[0].arg("runtime"), None);
    }

    #[test]
    fn a_vanilla_game_without_plugins_or_loader_says_nothing() {
        assert!(extender_findings(&ExtenderFacts {
            loader_found: false,
            plugin_dlls: 0,
            ..facts()
        })
        .is_empty());
    }

    #[test]
    fn an_unknown_runtime_skips_the_dll_check() {
        assert!(extender_findings(&ExtenderFacts {
            runtime: None,
            runtime_dll: None,
            runtime_dll_found: false,
            ..facts()
        })
        .is_empty());
    }
}
