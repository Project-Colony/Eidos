//! Health decisions with no GUI in them: facts in, `Finding`s out. The GUI
//! renders a finding through `diag_<code>` / `diag_<code>_detail` strings; the
//! CLI prints the code and its args.

pub mod crash;
pub mod extender;
pub mod graphics;
pub mod launch_option;

use std::path::{Path, PathBuf};

use eidos_gamefeatures::RegistryStatus;
use eidos_games::DetectedGame;
use eidos_instance::Instance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Problem,
    Advice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub code: &'static str,
    pub level: Level,
    pub args: Vec<(&'static str, String)>,
}

impl Finding {
    pub fn arg(&self, key: &str) -> Option<&str> {
        self.args
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
    }

    /// `key=value; key=value`
    pub fn args_line(&self) -> String {
        self.args
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Every code `host_findings` and `prefix_findings` can emit; the GUI has en/fr strings for each.
pub const CODES: &[&str] = &[
    "extender_missing",
    "extender_runtime_missing",
    "game_updated",
    "launch_option_missing",
    "launch_option_no_eidos",
    "launch_option_binary_missing",
    "launch_option_no_command",
    "launch_option_other_copy",
    "nvapi_missing",
    "fullscreen_blocks_framegen",
    "enb_linux_version",
    "game_crashed",
    "game_crashed_unattributed",
    "prefix_uninitialised",
    "registry_stale",
    "registry_missing",
    "gamedrive_foreign",
];

/// The instance's enabled mods, highest priority first.
pub fn active_mods(inst: &Instance) -> Vec<(String, PathBuf)> {
    inst.modlist()
        .into_iter()
        .rev()
        .filter(|m| m.is_active())
        .map(|m| (m.name, m.path))
        .collect()
}

pub fn host_findings(
    home: &Path,
    game: &DetectedGame,
    inst: &Instance,
    mods: &[(String, PathBuf)],
) -> Vec<Finding> {
    let overwrite = inst.overwrite_dir();
    let mut out = eidos_gamefeatures::preflight::extender_facts(
        game.def,
        &game.data_path,
        &game.install_path,
        mods,
        &overwrite,
    )
    .map(|f| extender::extender_findings(&f))
    .unwrap_or_default();
    out.extend(extender::game_update_findings(
        game.def.name,
        inst.launch_record().last_good.as_ref(),
        &extender::current_build(game, mods, &overwrite),
    ));
    let this_exe = std::env::current_exe().ok();
    let mut options = None;
    if game.is_steam() && game.compatdata.is_some() {
        options = eidos_games::steam_launch_options(home, game.def.steam_app_id);
        let path: Vec<PathBuf> = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        if let Some(this_exe) = &this_exe {
            out.extend(launch_option::launch_option_findings(
                options.as_deref(),
                this_exe,
                home,
                &path,
                game.def,
            ));
        }
    }
    if game.prefix().is_some() {
        let this_exe = this_exe.unwrap_or_else(|| PathBuf::from("eidos"));
        out.extend(graphics::graphics_findings(&graphics::graphics_facts(
            game,
            inst,
            mods,
            options.as_deref(),
            &this_exe,
            home,
        )));
    }
    out.extend(crash::crash_findings(game, inst, mods));
    out
}

/// Everything the prefix checks need, read once so the checks themselves stay
/// pure and testable. Gathering is I/O; deciding is not.
pub struct PrefixFacts {
    /// The game's display name, for messages a user has to act on.
    pub game_name: String,
    pub registry: RegistryStatus,
    /// Where `dosdevices/s:` points, if it exists.
    pub gamedrive_found: Option<PathBuf>,
    /// Where EIDOS points it for a tool launch: the library ROOT, the directory
    /// that HOLDS `steamapps`. Taken from `library_path`, the same function whose
    /// value is handed to Proton, so this can never disagree with what Eidos
    /// passes. Steam, launching the game, leaves it on `<root>/steamapps`
    /// instead - both are sound, and [`prefix_findings`] accepts both.
    pub gamedrive_want: Option<PathBuf>,
}

/// Read the prefix's health off disk. `None` when the game has no prefix path at
/// all, which is not a fault worth reporting - the game may simply never have
/// been set up to run under Proton. Unmemoised: the GUI, which asks on every
/// refresh, gathers the same facts through its own memo of `system.reg`.
pub fn prefix_facts(game: &DetectedGame) -> Option<PrefixFacts> {
    let compat = game.compatdata.as_ref()?;
    // Read once and hand to both consumers: the registry check resolves the
    // launcher's `S:`-relative value through THIS link, so the two facts must
    // describe the same instant.
    let gamedrive_found = std::fs::read_link(compat.join("pfx/dosdevices/s:")).ok();
    Some(PrefixFacts {
        game_name: game.def.name.to_string(),
        registry: eidos_gamefeatures::registry_status(
            compat,
            &game.install_path,
            game.def.registry_name,
            gamedrive_found.as_deref(),
        ),
        gamedrive_found,
        gamedrive_want: eidos_games::library_path(&game.install_path),
    })
}

/// Turn the facts into findings, one per fault. Pure: no I/O.
pub fn prefix_findings(f: &PrefixFacts) -> Vec<Finding> {
    let mut out = Vec::new();
    let game = ("game", f.game_name.clone());
    let problem = |code, args| Finding {
        code,
        level: Level::Problem,
        args,
    };

    match &f.registry {
        RegistryStatus::NotApplicable | RegistryStatus::Correct => {}
        RegistryStatus::PrefixUninitialised => {
            out.push(problem("prefix_uninitialised", vec![game.clone()]));
        }
        // Showing the wrong value is the point: it is the exact path the tool
        // is about to complain about, and the only thing that lets someone
        // connect that complaint to Eidos.
        RegistryStatus::Stale {
            found: Some(bad),
            want,
        } => out.push(problem(
            "registry_stale",
            vec![game.clone(), ("found", bad.clone()), ("want", want.clone())],
        )),
        RegistryStatus::Stale { found: None, want } => out.push(problem(
            "registry_missing",
            vec![game.clone(), ("want", want.clone())],
        )),
    }

    // The game drive. Proton recreates `dosdevices/s:` from whatever it is
    // handed on EVERY run, and the two things that hand it something disagree
    // on purpose: Eidos passes the library ROOT for a tool (so BodySlide's
    // `<drive>\steamapps\common\<game>` search lands), Steam passes
    // `<root>/steamapps` for the game. Both are sound; each launcher re-points
    // the link before it runs anything. The old version of this check called
    // Steam's value a fault, which lit the card after every game launch for as
    // long as the instance existed. What IS a fault is a link that matches
    // neither: a library that moved, or a prefix copied from another machine.
    // Then every `S:`-relative path already stored in the prefix - the launcher's
    // `installed path` above all - resolves nowhere, and a tool that WRITES
    // there creates the directory outside the mount instead of failing.
    if let (Some(found), Some(want)) = (&f.gamedrive_found, &f.gamedrive_want) {
        let steam = want.join("steamapps");
        if found != want && *found != steam {
            out.push(problem(
                "gamedrive_foreign",
                vec![
                    ("found", found.display().to_string()),
                    ("tool", want.display().to_string()),
                    ("steam", steam.display().to_string()),
                    game,
                ],
            ));
        }
    }

    out
}

/// What a bug report states about the machine and the setup, gathered apart
/// from the text so the builder can be tested without a real host.
pub struct ReportFacts {
    pub version: String,
    pub game_id: String,
    pub proton_build: String,
    /// None when no Proton command could be resolved for the selected game.
    pub flatpak_steam: Option<bool>,
    pub passthrough_requested: bool,
    pub cap_missing: bool,
    pub session: &'static str,
    pub distro: String,
    pub kernel: String,
    pub dev_fuse: bool,
    pub fusermount3: bool,
}

/// Read the facts off the running host and the selected game.
pub fn report_facts(home: &Path, game: Option<&DetectedGame>, cap_missing: bool) -> ReportFacts {
    let run = game.and_then(|g| {
        let cd = g.compatdata.as_ref()?;
        eidos_games::proton_command(home, g.def.steam_app_id, cd, &g.install_path)
    });
    let proton_build = run
        .as_ref()
        .and_then(|r| r.proton.parent()?.file_name())
        .map_or_else(|| "unknown".into(), |n| n.to_string_lossy().into_owned());
    let distro = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                .map(|v| v.trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "unknown".into());
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map_or_else(|_| "unknown".into(), |s| s.trim().to_string());
    let fusermount3 = std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("fusermount3").is_file()));
    let set = |var| std::env::var_os(var).is_some_and(|v| !v.is_empty());
    ReportFacts {
        version: env!("CARGO_PKG_VERSION").to_string(),
        game_id: game.map_or_else(|| "none".into(), |g| g.def.id.to_string()),
        proton_build,
        flatpak_steam: run.map(|r| r.flatpak),
        passthrough_requested: passthrough_requested(),
        cap_missing,
        session: if set("WAYLAND_DISPLAY") {
            "Wayland"
        } else if set("DISPLAY") {
            "X11"
        } else {
            "none"
        },
        distro,
        kernel,
        dev_fuse: Path::new("/dev/fuse").exists(),
        fusermount3,
    }
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

/// The report's first lines, one fact each. Not redacted: the caller redacts
/// the whole report once, with the findings in it.
pub fn report_header(f: &ReportFacts) -> Vec<String> {
    let passthrough = match (f.passthrough_requested, f.cap_missing) {
        (false, _) => "off (default)",
        (true, true) => "requested, launch capability missing",
        (true, false) => "requested, launch capability present",
    };
    vec![
        format!("Eidos: {}", f.version),
        format!("Game: {}", f.game_id),
        format!("Proton: {}", f.proton_build),
        format!(
            "Flatpak Steam: {}",
            f.flatpak_steam.map_or("unknown", yes_no)
        ),
        format!("Passthrough: {passthrough}"),
        format!("Session: {}", f.session),
        format!("Distro: {}", f.distro),
        format!("Kernel: {}", f.kernel),
        format!(
            "/dev/fuse: {}",
            if f.dev_fuse { "present" } else { "missing" }
        ),
        format!(
            "fusermount3: {}",
            if f.fusermount3 { "found" } else { "not found" }
        ),
    ]
}

/// Whether the user opted into FUSE passthrough. Read from this process's own
/// environment because the launch environment is inherited from it (Steam launch
/// options land here first, then on the `eidos` child).
///
/// It is off by default: passthrough stops the game opening its own archives and
/// plugins (see `passthrough_enabled` in eidos-fuse for the measurement). This
/// gates the capability warnings, which are only meaningful to someone who
/// actually wants passthrough.
pub fn passthrough_requested() -> bool {
    std::env::var("EIDOS_FUSE_PASSTHROUGH").is_ok_and(|v| {
        let v = v.trim();
        !v.is_empty() && v != "0"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_findings_keep_one_finding_per_fault() {
        let f = PrefixFacts {
            game_name: "Fallout 4".into(),
            registry: RegistryStatus::Stale {
                found: Some(r"S:\common\Fallout 4\".into()),
                want: r"Z:\real\Fallout 4\".into(),
            },
            gamedrive_found: Some("/mnt/OldDisk/SteamLibrary".into()),
            gamedrive_want: Some("/mnt/Jeux/SteamLibrary".into()),
        };
        let codes: Vec<_> = prefix_findings(&f).iter().map(|f| f.code).collect();
        assert_eq!(codes, ["registry_stale", "gamedrive_foreign"]);
        let steam = PrefixFacts {
            gamedrive_found: Some("/mnt/Jeux/SteamLibrary/steamapps".into()),
            registry: RegistryStatus::Correct,
            ..f
        };
        assert!(prefix_findings(&steam).is_empty());
    }
}
