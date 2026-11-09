//! A09: the crash log of the last session, and the mods it names.

use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use eidos_gamefeatures::crash_log::{newest_crash_log, parse_crash_log};
use eidos_gamefeatures::preflight::{extender, merged_data, merged_root};
use eidos_games::DetectedGame;
use eidos_instance::Instance;

use super::{Finding, Level};

/// The newest crash log written at or after `started` (seconds since the epoch).
fn crash_log_since(game: &DetectedGame, overwrite: &Path, started: u64) -> Option<PathBuf> {
    let docs = eidos_plugins::documents_my_games_dir(&game.prefix()?, &game.plugin_spec()?);
    newest_crash_log(
        game.def.id,
        &docs,
        overwrite,
        UNIX_EPOCH + Duration::from_secs(started),
    )
}

/// A crash logger wrote a log during the session that started at `started`.
pub fn crashed_since(game: &DetectedGame, overwrite: &Path, started: u64) -> bool {
    crash_log_since(game, overwrite, started).is_some()
}

/// The crash log of the last launch, with the DLLs and plugins it names that one
/// of the user's mods (or the Overwrite) installs. At most one finding.
pub fn crash_findings(
    game: &DetectedGame,
    inst: &Instance,
    mods: &[(String, PathBuf)],
) -> Vec<Finding> {
    let overwrite = inst.overwrite_dir();
    let Some(path) = inst
        .launch_record()
        .started
        .and_then(|started| crash_log_since(game, &overwrite, started))
    else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    let log = parse_crash_log(&path, &String::from_utf8_lossy(&bytes));

    let data = merged_data(&game.data_path, mods, &overwrite);
    let root = merged_root(&game.install_path, mods, &overwrite);
    let plugins_dir = extender(game.def.id).map(|e| e.plugins_dir);
    let modules = log.modules.iter().filter_map(|dll| {
        let in_data = plugins_dir.and_then(|dir| data.resolve(&format!("{dir}/{dll}")));
        let origin = match in_data {
            Some(p) => data.origin(&p),
            None => root.origin(&root.resolve(dll)?),
        };
        Some((dll, origin))
    });
    let plugins = log
        .plugins
        .iter()
        .filter_map(|esp| Some((esp, data.origin(&data.resolve(esp)?))));
    let culprits: Vec<String> = modules
        .chain(plugins)
        .filter(|(_, origin)| origin != "Game" && origin != "Unverified source")
        .take(8)
        .map(|(name, origin)| format!("{name} ({origin})"))
        .collect();

    let mut args = vec![
        ("logger", log.logger.to_string()),
        ("log", path.display().to_string()),
        (
            "exception",
            log.exception
                .unwrap_or_else(|| "no exception line".to_string()),
        ),
    ];
    let code = if culprits.is_empty() {
        "game_crashed_unattributed"
    } else {
        args.push(("culprits", culprits.join("; ")));
        "game_crashed"
    };
    vec![Finding {
        code,
        level: Level::Problem,
        args,
    }]
}

/// The log read lossily, home path and user name replaced.
pub fn redacted_log(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(eidos_log::redact(&String::from_utf8_lossy(&bytes)).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::Level;
    use eidos_instance::GameBuild;
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    struct Fx {
        game: DetectedGame,
        inst: Instance,
        mods: Vec<(String, PathBuf)>,
        docs: PathBuf,
        now: u64,
    }

    /// A Skyrim SE Steam copy with a prefix, one mod that installs a DLL and a
    /// plugin, and a launch recorded a minute ago.
    fn fixture(tag: &str) -> Fx {
        let root = std::env::temp_dir().join(format!("eidos-crash-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let game_dir = root.join("game");
        fs::create_dir_all(game_dir.join("Data")).unwrap();
        fs::write(game_dir.join("Data/Skyrim.esm"), b"").unwrap();
        let inst = Instance::portable(root.join("instance"));
        inst.create().unwrap();
        let mod_dir = root.join("instance/mods/My Mod");
        fs::create_dir_all(mod_dir.join("SKSE/Plugins")).unwrap();
        fs::write(mod_dir.join("SKSE/Plugins/MyPlugin.dll"), b"").unwrap();
        fs::write(mod_dir.join("MyMod.esp"), b"").unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        inst.record_launch_start(&GameBuild::default(), now - 60)
            .unwrap();
        let compat = root.join("compatdata");
        let docs =
            compat.join("pfx/drive_c/users/steamuser/Documents/My Games/Skyrim Special Edition");
        fs::create_dir_all(docs.join("SKSE")).unwrap();
        let game = DetectedGame {
            source: Default::default(),
            def: eidos_games::catalog()
                .iter()
                .find(|g| g.id == "skyrimse")
                .unwrap(),
            install_path: game_dir.clone(),
            data_path: game_dir.join("Data"),
            compatdata: Some(compat),
            steam_name: "synthetic".into(),
        };
        Fx {
            game,
            inst,
            mods: vec![("My Mod".into(), mod_dir)],
            docs,
            now,
        }
    }

    fn crash_log(module: &str, plugins: &str) -> String {
        format!(
            "CrashLoggerSSE v1-15-0-0 Jan 30 2026 10:00:00\n\n\
Unhandled exception \"EXCEPTION_ACCESS_VIOLATION\" at 0x7FF6A1B2C3D4 SkyrimSE.exe+0123456\n\n\
PROBABLE CALL STACK:\n\t[0] 0x7FF6A1B2C3D4 SkyrimSE.exe+0123456\n\t[1] 0x7FFB11112222 {module}+0012345\n\n\
REGISTERS:\n\tRAX 0x1        (TESObjectREFR*)\n\t\tModified by: {plugins}\n\n\
MODULES:\n\tSkyrimSE.exe 0x7FF6A0000000\n"
        )
    }

    fn write_log(fx: &Fx, text: &str) -> PathBuf {
        let p = fx.docs.join("SKSE/crash-2026-11-09-21-30-00.log");
        fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn a_crash_after_the_launch_names_the_mods_in_it() {
        let fx = fixture("named");
        let p = write_log(&fx, &crash_log("MyPlugin.dll", "Skyrim.esm -> MyMod.esp"));
        let f = crash_findings(&fx.game, &fx.inst, &fx.mods);
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].code, f[0].level), ("game_crashed", Level::Problem));
        let c = f[0].arg("culprits").unwrap();
        assert!(
            c.contains("MyPlugin.dll (My Mod)")
                && c.contains("MyMod.esp (My Mod)")
                && !c.contains("Skyrim.esm"),
            "{c}"
        );
        assert_eq!(f[0].arg("logger"), Some("Crash Logger"));
        assert_eq!(f[0].arg("log"), Some(p.to_string_lossy().as_ref()));
        assert!(f[0]
            .arg("exception")
            .unwrap()
            .starts_with("Unhandled exception"));
    }

    #[test]
    fn a_crash_before_the_last_launch_is_not_reported() {
        let fx = fixture("old");
        let p = write_log(&fx, &crash_log("MyPlugin.dll", "MyMod.esp"));
        fs::File::options()
            .write(true)
            .open(&p)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_secs(fx.now - 600))
            .unwrap();
        assert!(crash_findings(&fx.game, &fx.inst, &fx.mods).is_empty());
    }

    #[test]
    fn a_crash_naming_only_game_files_is_unattributed() {
        let fx = fixture("game-only");
        write_log(&fx, &crash_log("KERNELBASE.dll", "Skyrim.esm"));
        let f = crash_findings(&fx.game, &fx.inst, &fx.mods);
        assert_eq!(f[0].code, "game_crashed_unattributed");
        assert_eq!(f[0].arg("culprits"), None);
    }

    #[test]
    fn crashed_since_sees_a_log_written_after_the_start() {
        let fx = fixture("since");
        write_log(&fx, &crash_log("MyPlugin.dll", "MyMod.esp"));
        let ow = fx.inst.overwrite_dir();
        assert!(crashed_since(&fx.game, &ow, fx.now - 60));
        assert!(!crashed_since(&fx.game, &ow, fx.now + 60));
    }

    #[test]
    fn redacted_log_hides_the_home_path() {
        let fx = fixture("redact");
        let home = std::env::var("HOME").unwrap();
        let p = write_log(&fx, &format!("File: \"{home}/Games/thing\"\n"));
        assert!(!redacted_log(&p).unwrap().contains(&home));
        assert_eq!(redacted_log(&fx.docs.join("absent.log")), None);
    }
}
