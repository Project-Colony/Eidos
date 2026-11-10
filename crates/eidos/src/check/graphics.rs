//! A16: graphics settings that break DLSS, frame generation and ENB under Proton.
//! Fixes write the profile INI or a copy in the Overwrite, never a mod's file.

use std::io;
use std::path::{Path, PathBuf};

use eidos_gamefeatures::preflight::{merged_data, merged_root};
use eidos_games::DetectedGame;
use eidos_instance::{read_text_lossy, Instance};

use super::launch_option::recommended_line;
use super::{Finding, Level};

pub struct GraphicsFacts {
    /// skyrimse | enderalse
    pub cs_game: bool,
    pub nvidia: bool,
    /// env PROTON_ENABLE_NVAPI == "1" or the launch options contain it
    pub nvapi_set: bool,
    /// `nvapi_on_by_default(<the game's proton script>) == Some(false)`
    pub nvapi_off_by_default: bool,
    /// The proton script's parent folder name, for the card.
    pub proton_build: String,
    /// Merged Data "Shaders/Upscaling" lists at least one entry.
    pub upscaling: bool,
    /// The profile copy of SkyrimPrefs.ini.
    pub prefs_ini: Option<(PathBuf, String)>,
    pub display_tweaks_ini: Option<String>,
    /// Winning root `enblocal.ini`: path, text, origin.
    pub enblocal: Option<(PathBuf, String, String)>,
    /// What the NVAPI card tells the user to set.
    pub launch_line: String,
}

/// Whether a Proton build turns NVAPI on without PROTON_ENABLE_NVAPI=1, read from
/// the first `use_nvapi =` line of its `proton` script: an opt-out switch
/// (`disablenvapi`) means on by default, an opt-in switch (`enablenvapi`) means off.
/// No such line: None.
pub fn nvapi_on_by_default(proton_script: &str) -> Option<bool> {
    let line = proton_script.lines().find(|l| l.contains("use_nvapi ="))?;
    if line.contains("disablenvapi") {
        Some(true)
    } else if line.contains("enablenvapi") {
        Some(false)
    } else {
        None
    }
}

pub fn graphics_facts(
    game: &DetectedGame,
    inst: &Instance,
    mods: &[(String, PathBuf)],
    launch_options: Option<&str>,
    this_exe: &Path,
    home: &Path,
) -> GraphicsFacts {
    let cs_game = matches!(game.def.id, "skyrimse" | "enderalse");
    let overwrite = inst.overwrite_dir();
    let files = crate::stock_game::game_files(game, inst);
    let options = launch_options.unwrap_or("");
    let nvapi_set = std::env::var("PROTON_ENABLE_NVAPI").is_ok_and(|v| v == "1")
        || options.contains("PROTON_ENABLE_NVAPI=1")
        || inst
            .active()
            .launch_env()
            .env
            .iter()
            .rev()
            .find(|(k, _)| k == "PROTON_ENABLE_NVAPI")
            .is_some_and(|(_, v)| v == "1");

    let mut nvapi_off_by_default = false;
    let mut proton_build = String::new();
    // Only the NVAPI card reads the Proton build, and it is for these two games.
    if cs_game && game.is_steam() {
        if let Some(compat) = &game.compatdata {
            let run = eidos_games::proton_command(
                home,
                game.def.steam_app_id,
                compat,
                &game.install_path,
            )
            .filter(|r| !r.plain_wine);
            if let Some(run) = run {
                if let Ok(bytes) = std::fs::read(&run.proton) {
                    let text = String::from_utf8_lossy(&bytes);
                    nvapi_off_by_default = nvapi_on_by_default(&text) == Some(false);
                    proton_build = run
                        .proton
                        .parent()
                        .and_then(Path::file_name)
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                }
            }
        }
    }

    let (mut upscaling, mut prefs_ini, mut display_tweaks_ini) = (false, None, None);
    if cs_game {
        let data = merged_data(&files.data, mods, &overwrite);
        upscaling = !data.list_dir("Shaders/Upscaling").is_empty();
        display_tweaks_ini = data
            .resolve("SKSE/Plugins/SSEDisplayTweaks.ini")
            .and_then(|p| read_text_lossy(&p))
            .map(|(t, _)| t);
        let p = inst.active().ini_path("SkyrimPrefs.ini");
        prefs_ini = read_text_lossy(&p).map(|(t, _)| (p, t));
    }

    let root = merged_root(&files.root, mods, &overwrite);
    let enblocal = root.resolve("enblocal.ini").and_then(|p| {
        let (text, _) = read_text_lossy(&p)?;
        let origin = root.origin(&p);
        Some((p, text, origin))
    });

    let launch_line = if options.trim().is_empty() {
        format!(
            "PROTON_ENABLE_NVAPI=1 {}",
            recommended_line(this_exe, game.def.id)
        )
    } else {
        format!("PROTON_ENABLE_NVAPI=1 {options}")
    };

    GraphicsFacts {
        cs_game,
        nvidia: Path::new("/proc/driver/nvidia/version").exists(),
        nvapi_set,
        nvapi_off_by_default,
        proton_build,
        upscaling,
        prefs_ini,
        display_tweaks_ini,
        enblocal,
        launch_line,
    }
}

fn is(text: &str, section: &str, key: &str, want: &str) -> bool {
    eidos_ini::get_key(text, section, key).is_some_and(|v| v.trim().eq_ignore_ascii_case(want))
}

pub fn graphics_findings(f: &GraphicsFacts) -> Vec<Finding> {
    let mut out = Vec::new();
    if f.cs_game && f.nvidia && f.upscaling && !f.nvapi_set && f.nvapi_off_by_default {
        out.push(Finding {
            code: "nvapi_missing",
            level: Level::Advice,
            args: vec![
                ("line", f.launch_line.clone()),
                ("build", f.proton_build.clone()),
            ],
        });
    }
    if let (true, true, Some((path, text))) = (f.cs_game, f.upscaling, &f.prefs_ini) {
        let borderless = f.display_tweaks_ini.as_deref().is_some_and(|dt| {
            is(dt, "Render", "Fullscreen", "false") && is(dt, "Render", "Borderless", "true")
        });
        if is(text, "Display", "bFull Screen", "1") && !borderless {
            out.push(Finding {
                code: "fullscreen_blocks_framegen",
                level: Level::Advice,
                args: vec![("ini", path.display().to_string())],
            });
        }
    }
    if let Some((path, text, origin)) = &f.enblocal {
        if !is(text, "GLOBAL", "LinuxVersion", "true") {
            out.push(Finding {
                code: "enb_linux_version",
                level: Level::Advice,
                args: vec![("ini", path.display().to_string()), ("mod", origin.clone())],
            });
        }
    }
    out
}

/// Borderless in the profile's copy of the prefs INI.
pub fn fix_fullscreen(prefs_ini: &Path) -> io::Result<()> {
    eidos_gamefeatures::set_ini_key(prefs_ini, "Display", "bFull Screen", "0")?;
    eidos_gamefeatures::set_ini_key(prefs_ini, "Display", "bBorderless", "1")
}

/// Write `LinuxVersion=true` into a copy of the winning `enblocal.ini` in
/// `overwrite/Root`; the mod's own file stays as it is.
pub fn fix_enb_linux_version(
    game: &DetectedGame,
    inst: &Instance,
    mods: &[(String, PathBuf)],
) -> io::Result<PathBuf> {
    let files = crate::stock_game::game_files(game, inst);
    let root = merged_root(&files.root, mods, &inst.overwrite_dir());
    let winner = root
        .resolve("enblocal.ini")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "enblocal.ini"))?;
    let dir = inst.root_overwrite_dir();
    let target = if winner.starts_with(&dir) {
        winner
    } else {
        std::fs::create_dir_all(&dir)?;
        let target = dir.join(winner.file_name().unwrap_or_default());
        std::fs::copy(&winner, &target)?;
        target
    };
    eidos_gamefeatures::set_ini_key(&target, "GLOBAL", "LinuxVersion", "true")?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> GraphicsFacts {
        GraphicsFacts {
            cs_game: true,
            nvidia: true,
            nvapi_set: false,
            nvapi_off_by_default: true,
            proton_build: "proton_9".into(),
            upscaling: true,
            prefs_ini: None,
            display_tweaks_ini: None,
            enblocal: None,
            launch_line: "PROTON_ENABLE_NVAPI=1 x %command%".into(),
        }
    }

    #[test]
    fn nvidia_with_upscaling_and_no_nvapi_asks_for_the_variable() {
        let f = graphics_findings(&base());
        assert_eq!((f[0].code, f[0].level), ("nvapi_missing", Level::Advice));
        assert!(f[0]
            .arg("line")
            .unwrap()
            .starts_with("PROTON_ENABLE_NVAPI=1 "));
        assert_eq!(f[0].arg("build"), Some("proton_9"));
    }

    #[test]
    fn nvapi_set_or_on_by_default_or_no_nvidia_or_no_upscaling_says_nothing() {
        for f in [
            GraphicsFacts {
                nvapi_set: true,
                ..base()
            },
            GraphicsFacts {
                nvapi_off_by_default: false,
                ..base()
            },
            GraphicsFacts {
                nvidia: false,
                ..base()
            },
            GraphicsFacts {
                upscaling: false,
                ..base()
            },
        ] {
            assert!(!graphics_findings(&f)
                .iter()
                .any(|f| f.code == "nvapi_missing"));
        }
    }

    #[test]
    fn nvapi_default_is_read_from_the_proton_script() {
        // The opt-out line as GE-Proton11-9 ships it; Valve's Proton 11.0 has the same switch inside a longer expression (checked 2026-11-09).
        let ge = "            use_wined3d = \"wined3d\" in g_session.compat_config\n            use_nvapi = 'disablenvapi' not in g_session.compat_config or 'forcenvapi' in g_session.compat_config\n";
        // The opt-in form, written for this test.
        let opt_in = "            use_nvapi = 'enablenvapi' in g_session.compat_config\n";
        assert_eq!(nvapi_on_by_default(ge), Some(true));
        assert_eq!(nvapi_on_by_default(opt_in), Some(false));
        assert_eq!(nvapi_on_by_default("#!/usr/bin/env python3\n"), None);
    }

    #[test]
    fn exclusive_fullscreen_with_upscaling_is_reported_unless_display_tweaks_forces_borderless() {
        let prefs = Some((
            PathBuf::from("/p/SkyrimPrefs.ini"),
            "[Display]\r\nbFull Screen=1\r\n".to_string(),
        ));
        let f = GraphicsFacts {
            nvapi_set: true,
            prefs_ini: prefs,
            ..base()
        };
        assert_eq!(graphics_findings(&f)[0].code, "fullscreen_blocks_framegen");
        let dt = Some("[Render]\nFullscreen=false\nBorderless=true\n".to_string());
        assert!(graphics_findings(&GraphicsFacts {
            display_tweaks_ini: dt,
            ..f
        })
        .is_empty());
    }

    #[test]
    fn enb_without_linux_version_is_reported() {
        let enb = |text: &str| GraphicsFacts {
            cs_game: false,
            enblocal: Some(("/r/enblocal.ini".into(), text.into(), "ENB Binaries".into())),
            ..base()
        };
        assert_eq!(
            graphics_findings(&enb("[GLOBAL]\nUsePatchSpeedhackWithoutGraphics=false\n"))[0].code,
            "enb_linux_version"
        );
        assert!(graphics_findings(&enb("[GLOBAL]\nLinuxVersion=TRUE\n")).is_empty());
    }

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eidos-gfx-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn only_skyrim_se_and_enderal_se_read_the_proton_script() {
        let root = scratch("proton");
        let steam = root.join("home/.steam/steam");
        std::fs::create_dir_all(steam.join("config")).unwrap();
        std::fs::write(
            steam.join("config/config.vdf"),
            "\"InstallConfigStore\"\n{\n}\n",
        )
        .unwrap();
        let tool = steam.join("compatibilitytools.d/Proton-Test");
        std::fs::create_dir_all(&tool).unwrap();
        std::fs::write(
            tool.join("proton"),
            "    use_nvapi = 'enablenvapi' in g_session.compat_config\n",
        )
        .unwrap();
        let compat = root.join("compatdata");
        std::fs::create_dir_all(&compat).unwrap();
        std::fs::write(compat.join("version"), "Proton-Test\n").unwrap();
        let inst = Instance::portable(root.join("instance"));
        inst.create().unwrap();
        let game_dir = root.join("game");
        std::fs::create_dir_all(game_dir.join("Data")).unwrap();
        let facts = |id: &str| {
            let game = DetectedGame {
                source: Default::default(),
                def: eidos_games::catalog().iter().find(|g| g.id == id).unwrap(),
                install_path: game_dir.clone(),
                data_path: game_dir.join("Data"),
                compatdata: Some(compat.clone()),
                steam_name: "synthetic".into(),
            };
            graphics_facts(
                &game,
                &inst,
                &[],
                None,
                Path::new("eidos"),
                &root.join("home"),
            )
        };
        let se = facts("skyrimse");
        assert!(se.nvapi_off_by_default);
        assert_eq!(se.proton_build, "Proton-Test");
        let fo4 = facts("fallout4");
        assert!(!fo4.nvapi_off_by_default);
        assert_eq!(fo4.proton_build, "");
    }

    #[test]
    fn a_profile_variable_counts_as_nvapi_set() {
        let root = scratch("nvapi-profile");
        let inst = Instance::portable(root.join("instance"));
        inst.create().unwrap();
        let game_dir = root.join("game");
        std::fs::create_dir_all(game_dir.join("Data")).unwrap();
        let game = DetectedGame {
            source: Default::default(),
            def: eidos_games::GameDef::for_id("skyrimse").unwrap(),
            install_path: game_dir.clone(),
            data_path: game_dir.join("Data"),
            compatdata: None,
            steam_name: "synthetic".into(),
        };
        let nvapi_set = || {
            graphics_facts(
                &game,
                &inst,
                &[],
                None,
                Path::new("eidos"),
                &root.join("home"),
            )
            .nvapi_set
        };
        if std::env::var("PROTON_ENABLE_NVAPI").map_or(true, |v| v != "1") {
            assert!(!nvapi_set());
        }
        let prof = inst.active();
        std::fs::create_dir_all(prof.dir()).unwrap();
        std::fs::write(
            prof.dir().join(eidos_instance::LAUNCH_ENV_FILE),
            "env=PROTON_ENABLE_NVAPI=1\n",
        )
        .unwrap();
        assert!(nvapi_set());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fix_fullscreen_writes_borderless_into_the_profile_ini() {
        let p = scratch("fs").join("SkyrimPrefs.ini");
        std::fs::write(&p, "[Display]\r\nbFull Screen=1\r\niSize W=2560\r\n").unwrap();
        fix_fullscreen(&p).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(
            eidos_ini::get_key(&text, "Display", "bFull Screen"),
            Some("0")
        );
        assert_eq!(
            eidos_ini::get_key(&text, "Display", "bBorderless"),
            Some("1")
        );
        assert_eq!(
            eidos_ini::get_key(&text, "Display", "iSize W"),
            Some("2560")
        );
    }

    #[test]
    fn fix_enb_writes_a_copy_into_overwrite_and_leaves_the_mod_alone() {
        let root = scratch("enb");
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let mod_dir = root.join("mods/ENB Binaries");
        std::fs::create_dir_all(mod_dir.join("Root")).unwrap();
        let mod_file = mod_dir.join("Root/enblocal.ini");
        std::fs::write(
            &mod_file,
            "[GLOBAL]\nUsePatchSpeedhackWithoutGraphics=false\n",
        )
        .unwrap();
        let original_bytes = std::fs::read(&mod_file).unwrap();
        let game_dir = root.join("game");
        std::fs::create_dir_all(game_dir.join("Data")).unwrap();
        let game = DetectedGame {
            source: Default::default(),
            def: eidos_games::catalog()
                .iter()
                .find(|g| g.id == "skyrimse")
                .unwrap(),
            install_path: game_dir.clone(),
            data_path: game_dir.join("Data"),
            compatdata: None,
            steam_name: "synthetic".into(),
        };
        let mods = vec![("ENB Binaries".to_string(), mod_dir)];

        let out = fix_enb_linux_version(&game, &inst, &mods).unwrap();

        assert_eq!(out, inst.root_overwrite_dir().join("enblocal.ini"));
        let text = std::fs::read_to_string(&out).unwrap();
        assert_eq!(
            eidos_ini::get_key(&text, "GLOBAL", "LinuxVersion"),
            Some("true")
        );
        assert_eq!(std::fs::read(mod_file).unwrap(), original_bytes);
    }
}
