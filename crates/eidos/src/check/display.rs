//! B04: the window size and mode a profile starts the game with, for the games
//! whose Prefs INI keys are known. Apply writes the profile's INI copies and a
//! copy of SSE Display Tweaks' INI in the Overwrite, never the game folder or a mod.

use std::io;
use std::path::{Path, PathBuf};

use eidos_gamefeatures::preflight::merged_data;
use eidos_games::DetectedGame;
use eidos_instance::{read_text_lossy, Instance};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    Fullscreen,
    Borderless,
    Windowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
    pub width: u32,
    pub height: u32,
    pub mode: WindowMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayFacts {
    /// The Prefs INI the card writes, e.g. `SkyrimPrefs.ini`.
    pub ini: &'static str,
    /// What the profile's copy (else the prefix's) sets, when it sets both sizes.
    pub current: Option<Display>,
    /// The largest connected output, landscape.
    pub screen: Option<(u32, u32)>,
    /// The winning SSEDisplayTweaks.ini sets the window.
    pub tweaks: bool,
}

pub const DRM: &str = "/sys/class/drm";

const TWEAKS_INI: &str = "SKSE/Plugins/SSEDisplayTweaks.ini";

/// The gamedef's Prefs INI for a game whose `[Display]` keys are known.
// shortcut: three games whose four keys are known; add a game once its Prefs INI is checked
pub fn prefs_ini(game_id: &str) -> Option<&'static str> {
    if !matches!(game_id, "skyrimse" | "enderalse" | "fallout4") {
        return None;
    }
    eidos_gamefeatures::ini_files_for(game_id)
        .iter()
        .find(|f| f.ends_with("Prefs.ini"))
        .copied()
}

fn flag(text: &str, key: &str) -> bool {
    eidos_ini::get_key(text, "Display", key).is_some_and(|v| v.trim() == "1")
}

/// `[Display]` `iSize W`/`iSize H` and the mode; `None` unless both sizes parse.
pub fn read_display(prefs: &str) -> Option<Display> {
    let size = |key| {
        eidos_ini::get_key(prefs, "Display", key)?
            .trim()
            .parse()
            .ok()
    };
    let mode = if flag(prefs, "bFull Screen") {
        WindowMode::Fullscreen
    } else if flag(prefs, "bBorderless") {
        WindowMode::Borderless
    } else {
        WindowMode::Windowed
    };
    Some(Display {
        width: size("iSize W")?,
        height: size("iSize H")?,
        mode,
    })
}

/// `1920x1080`, with anything after the height digits ignored (`1920x1080i`).
fn mode_line(line: &str) -> Option<(u32, u32)> {
    let (w, rest) = line.trim().split_once('x')?;
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    Some((w.parse().ok()?, rest[..end].parse().ok()?))
}

/// The largest connected output under `drm` (`card*-*/status` is `connected`,
/// the first line of `modes`), turned landscape.
pub fn output_size(drm: &Path) -> Option<(u32, u32)> {
    std::fs::read_dir(drm)
        .ok()?
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("card") && name.contains('-')
        })
        .filter(|e| {
            std::fs::read_to_string(e.path().join("status")).is_ok_and(|s| s.trim() == "connected")
        })
        .filter_map(|e| {
            let modes = std::fs::read_to_string(e.path().join("modes")).ok()?;
            let (w, h) = mode_line(modes.lines().next()?)?;
            Some((w.max(h), w.min(h)))
        })
        .max_by_key(|&(w, h)| u64::from(w) * u64::from(h))
}

/// Typed width and height, each a whole number in `320..=16384`.
pub fn parse_size(width: &str, height: &str) -> Option<(u32, u32)> {
    let one = |s: &str| {
        s.trim()
            .parse::<u32>()
            .ok()
            .filter(|n| (320..=16384).contains(n))
    };
    Some((one(width)?, one(height)?))
}

/// The winning SSEDisplayTweaks.ini when it sets the window (Skyrim SE, Enderal SE).
fn display_tweaks(
    game: &DetectedGame,
    inst: &Instance,
    mods: &[(String, PathBuf)],
) -> Option<PathBuf> {
    if !matches!(game.def.id, "skyrimse" | "enderalse") {
        return None;
    }
    let data = crate::stock_game::game_files(game, inst).data;
    let winner = merged_data(&data, mods, &inst.overwrite_dir()).resolve(TWEAKS_INI)?;
    let (text, _) = read_text_lossy(&winner)?;
    ["Resolution", "Fullscreen", "Borderless"]
        .iter()
        .any(|k| eidos_ini::get_key(&text, "Render", k).is_some())
        .then_some(winner)
}

/// What the Display card shows, or `None` for a game without one.
pub fn display_facts(
    game: &DetectedGame,
    inst: &Instance,
    mods: &[(String, PathBuf)],
    drm: &Path,
) -> Option<DisplayFacts> {
    let ini = prefs_ini(game.def.id)?;
    // The profile's copy replaces the prefix's at launch, so it wins when it exists.
    let text = read_text_lossy(&inst.active().ini_path(ini)).or_else(|| {
        let docs = eidos_plugins::documents_my_games_dir(&game.prefix()?, &game.plugin_spec()?);
        read_text_lossy(&docs.join(ini))
    });
    Some(DisplayFacts {
        ini,
        current: text.and_then(|(t, _)| read_display(&t)),
        screen: output_size(drm),
        tweaks: display_tweaks(game, inst, mods).is_some(),
    })
}

/// Write `d` into the profile's Prefs INI, after the launch path's own seeding,
/// and into a copy of SSE Display Tweaks' INI in the Overwrite when it sets the
/// window. Returns the Prefs path, then the copy when one was written.
pub fn apply_display(
    game: &DetectedGame,
    inst: &Instance,
    mods: &[(String, PathBuf)],
    d: Display,
) -> io::Result<Vec<PathBuf>> {
    let ini = prefs_ini(game.def.id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "no Display keys known"))?;
    let profile = inst.active();
    crate::prepare::seed_profile_inis(game.def.id, game, inst, &profile)?;
    let prefs = profile.ini_path(ini);
    let (full, borderless) = match d.mode {
        WindowMode::Fullscreen => ("1", "0"),
        WindowMode::Borderless => ("0", "1"),
        WindowMode::Windowed => ("0", "0"),
    };
    for (key, value) in [
        ("iSize W", d.width.to_string().as_str()),
        ("iSize H", d.height.to_string().as_str()),
        ("bFull Screen", full),
        ("bBorderless", borderless),
    ] {
        eidos_gamefeatures::set_ini_key(&prefs, "Display", key, value)?;
    }
    let mut written = vec![prefs];
    if let Some(winner) = display_tweaks(game, inst, mods) {
        let overwrite = inst.overwrite_dir();
        let copy = if winner.starts_with(&overwrite) {
            winner
        } else {
            let copy = overwrite.join(TWEAKS_INI);
            std::fs::create_dir_all(copy.parent().unwrap_or(&overwrite))?;
            std::fs::copy(&winner, &copy)?;
            copy
        };
        let on = |b: bool| if b { "true" } else { "false" };
        for (key, value) in [
            ("Resolution", format!("{}x{}", d.width, d.height)),
            ("Fullscreen", on(d.mode == WindowMode::Fullscreen).into()),
            ("Borderless", on(d.mode == WindowMode::Borderless).into()),
        ] {
            eidos_gamefeatures::set_ini_key(&copy, "Render", key, &value)?;
        }
        written.push(copy);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn tmp() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!(
            "eidos-display-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn only_games_with_known_display_keys_get_a_prefs_ini() {
        assert_eq!(prefs_ini("skyrimse"), Some("SkyrimPrefs.ini"));
        assert_eq!(prefs_ini("enderalse"), Some("EnderalPrefs.ini"));
        assert_eq!(prefs_ini("fallout4"), Some("Fallout4Prefs.ini"));
        for id in [
            "morrowind",
            "oblivion",
            "falloutnv",
            "skyrimvr",
            "fallout4vr",
            "starfield",
            "stellarblade",
        ] {
            assert_eq!(prefs_ini(id), None, "{id}");
        }
    }

    #[test]
    fn read_display_needs_both_sizes_and_reads_the_mode() {
        let d = |t: &str| read_display(t);
        assert_eq!(d("[Display]\niSize W=1920\n"), None);
        assert_eq!(
            d("[Display]\niSize W=1920\niSize H=1080\nbFull Screen=1\nbBorderless=1\n")
                .unwrap()
                .mode,
            WindowMode::Fullscreen
        );
        assert_eq!(
            d("[Display]\niSize W=1920\niSize H=1080\nbFull Screen=0\nbBorderless=1\n"),
            Some(Display {
                width: 1920,
                height: 1080,
                mode: WindowMode::Borderless
            })
        );
        assert_eq!(
            d("[Display]\niSize W=1280\niSize H=800\n").unwrap().mode,
            WindowMode::Windowed
        );
    }

    #[test]
    fn the_largest_connected_output_wins_and_a_portrait_panel_turns_landscape() {
        let drm = tmp();
        let port = |name: &str, status: &str, modes: &str| {
            fs::create_dir_all(drm.join(name)).unwrap();
            fs::write(drm.join(name).join("status"), status).unwrap();
            fs::write(drm.join(name).join("modes"), modes).unwrap();
        };
        assert_eq!(output_size(&drm), None);
        port("card0", "connected\n", "9999x9999\n"); // a card, not a connector
        port("card0-eDP-1", "connected\n", "800x1280\n640x480\n");
        port("card0-DP-1", "disconnected\n", "3840x2160\n");
        assert_eq!(output_size(&drm), Some((1280, 800)));
        port("card0-DP-1", "connected\n", "3840x2160\n1920x1080\n");
        assert_eq!(output_size(&drm), Some((3840, 2160)));
    }

    #[test]
    fn sizes_outside_320_to_16384_or_not_numbers_are_refused() {
        assert_eq!(parse_size(" 1280 ", "800"), Some((1280, 800)));
        for (w, h) in [
            ("", "800"),
            ("abc", "800"),
            ("319", "800"),
            ("1280", "16385"),
            ("-1", "800"),
            ("1280.5", "800"),
        ] {
            assert_eq!(parse_size(w, h), None, "{w}x{h}");
        }
    }

    /// A Steam Skyrim SE copy with its prefix, a portable instance whose active
    /// profile owns no INI, and the prefix's My Games folder (created).
    fn steam_skyrim() -> (DetectedGame, Instance, PathBuf) {
        let root = tmp();
        let inst = Instance::portable(root.join("instance"));
        inst.create().unwrap();
        let game = DetectedGame {
            def: eidos_games::GameDef::for_id("skyrimse").unwrap(),
            install_path: root.join("game"),
            data_path: root.join("game/Data"),
            compatdata: Some(root.join("compatdata")),
            source: Default::default(),
            steam_name: String::new(),
        };
        fs::create_dir_all(&game.data_path).unwrap();
        let docs = eidos_plugins::documents_my_games_dir(
            &game.prefix().unwrap(),
            &game.plugin_spec().unwrap(),
        );
        fs::create_dir_all(&docs).unwrap();
        (game, inst, docs)
    }

    #[test]
    fn display_apply_adopts_the_prefix_prefs_before_writing() {
        let (game, inst, docs) = steam_skyrim();
        let prefix_prefs =
            "[Display]\niSize W=2560\niSize H=1440\nbFull Screen=1\n[Grass]\niMinGrassSize=60\n";
        fs::write(docs.join("SkyrimPrefs.ini"), prefix_prefs).unwrap();
        let facts = display_facts(&game, &inst, &[], &tmp()).unwrap();
        assert_eq!(
            facts.current,
            Some(Display {
                width: 2560,
                height: 1440,
                mode: WindowMode::Fullscreen
            })
        );
        let written = apply_display(
            &game,
            &inst,
            &[],
            Display {
                width: 1280,
                height: 800,
                mode: WindowMode::Borderless,
            },
        )
        .unwrap();
        let profile_prefs = inst.active().ini_path("SkyrimPrefs.ini");
        assert_eq!(written, std::slice::from_ref(&profile_prefs));
        let text = fs::read_to_string(&profile_prefs).unwrap();
        assert!(text.contains("iMinGrassSize=60"), "{text}");
        assert_eq!(
            read_display(&text),
            Some(Display {
                width: 1280,
                height: 800,
                mode: WindowMode::Borderless
            })
        );
        assert_eq!(
            fs::read_to_string(docs.join("SkyrimPrefs.ini")).unwrap(),
            prefix_prefs,
            "the prefix copy is not written"
        );
    }

    #[test]
    fn display_apply_writes_sse_display_tweaks_copy_and_leaves_the_mod_alone() {
        let (game, inst, _docs) = steam_skyrim();
        let sdt = inst.mods_dir().join("SDT/SKSE/Plugins");
        fs::create_dir_all(&sdt).unwrap();
        fs::write(
            sdt.join("SSEDisplayTweaks.ini"),
            "[Render]\nFullscreen=true\nBorderless=false\n",
        )
        .unwrap();
        let mods = vec![("SDT".to_string(), inst.mods_dir().join("SDT"))];
        assert!(display_facts(&game, &inst, &mods, &tmp()).unwrap().tweaks);
        let written = apply_display(
            &game,
            &inst,
            &mods,
            Display {
                width: 1920,
                height: 1080,
                mode: WindowMode::Borderless,
            },
        )
        .unwrap();
        let copy = inst
            .overwrite_dir()
            .join("SKSE/Plugins/SSEDisplayTweaks.ini");
        assert_eq!(written.len(), 2);
        assert!(written.contains(&copy));
        let text = fs::read_to_string(&copy).unwrap();
        for (k, v) in [
            ("Resolution", "1920x1080"),
            ("Fullscreen", "false"),
            ("Borderless", "true"),
        ] {
            assert_eq!(eidos_ini::get_key(&text, "Render", k), Some(v), "{k}");
        }
        assert_eq!(
            fs::read_to_string(
                inst.mods_dir()
                    .join("SDT/SKSE/Plugins/SSEDisplayTweaks.ini")
            )
            .unwrap(),
            "[Render]\nFullscreen=true\nBorderless=false\n"
        );
    }

    #[test]
    fn with_a_game_copy_the_display_tweaks_winner_comes_from_the_copy() {
        let (game, inst, _docs) = steam_skyrim();
        let ini = "SKSE/Plugins/SSEDisplayTweaks.ini";
        let put = |data: &Path| {
            fs::create_dir_all(data.join("SKSE/Plugins")).unwrap();
            fs::write(data.join(ini), "[Render]\nBorderless=true\n").unwrap();
        };
        put(&game.data_path);
        fs::create_dir_all(inst.root.join("Stock Game/Data")).unwrap();
        inst.ensure_manifest("skyrimse", eidos_instance::InstanceKind::Portable)
            .unwrap();
        inst.edit_manifest(|m| m.base_game = Some("Stock Game".into()))
            .unwrap();
        assert!(
            !display_facts(&game, &inst, &[], &tmp()).unwrap().tweaks,
            "the store install's file is not read"
        );
        put(&inst.root.join("Stock Game/Data"));
        assert!(display_facts(&game, &inst, &[], &tmp()).unwrap().tweaks);
    }
}
