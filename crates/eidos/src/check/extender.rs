//! A10: the script extender is there, and its runtime DLL matches the game.

use eidos_gamefeatures::preflight::ExtenderFacts;

use super::{Finding, Level};

pub fn extender_findings(f: &ExtenderFacts) -> Vec<Finding> {
    let mut out = Vec::new();
    if !f.loader_found && f.plugin_dlls > 0 {
        out.push(Finding {
            code: "extender_missing",
            level: Level::Problem,
            args: vec![
                ("loader", f.loader.to_string()),
                ("dir", f.plugins_dir.to_string()),
                ("n", f.plugin_dlls.to_string()),
            ],
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
                ],
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use eidos_gamefeatures::preflight::Version;

    fn facts() -> ExtenderFacts {
        ExtenderFacts {
            loader: "skse64_loader.exe",
            loader_found: true,
            plugins_dir: "SKSE/Plugins",
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
    fn plugins_without_a_loader_are_a_problem() {
        let f = extender_findings(&ExtenderFacts {
            loader_found: false,
            plugin_dlls: 2,
            ..facts()
        });
        assert_eq!(f[0].code, "extender_missing");
        assert_eq!(f[0].arg("n"), Some("2"));
        assert_eq!(f[0].arg("loader"), Some("skse64_loader.exe"));
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
