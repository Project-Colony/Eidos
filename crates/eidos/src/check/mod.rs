//! Health decisions with no GUI in them: facts in, `Finding`s out. The GUI
//! renders a finding through `diag_<code>` / `diag_<code>_detail` strings; the
//! CLI prints the code and its args.

pub mod bodyslide;
pub mod crash;
pub mod extender;
pub mod graphics;
pub mod launch_option;
pub mod load_order;
pub mod system;

use std::path::{Path, PathBuf};
use std::process::exit;

use eidos_gamefeatures::preflight::{
    scan_compressed_audio, scan_extender_plugins, scan_fontconfig, DiagnosticSeverity,
};
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

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Problem => "problem",
            Level::Advice => "advice",
        }
    }
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
    "stestfile_present",
    "starfield_esp_active",
    "blueprint_invalid",
    "blueprint_unpaired",
    "max_map_count_low",
    "open_files_low",
    "windows_filesystem",
    "bodyslide_data_path",
    "overwrite_root_data",
];

/// The instance's enabled mods, highest priority first. Unmanaged rows (DLC,
/// Creation Club) are left out: their path is empty, so every probe under it
/// would read the working directory.
pub fn active_mods(inst: &Instance) -> Vec<(String, PathBuf)> {
    inst.modlist()
        .into_iter()
        .rev()
        .filter(|m| m.is_active() && !m.is_unmanaged())
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
    let mut out = load_order::test_file_findings(game.def, &inst.active());
    out.extend(
        eidos_gamefeatures::preflight::extender_facts(
            game.def,
            &game.data_path,
            &game.install_path,
            mods,
            &overwrite,
        )
        .map(|f| extender::extender_findings(&f))
        .unwrap_or_default(),
    );
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
    out.extend(system::system_findings(&system::system_facts(game, inst)));
    out.extend(bodyslide::bodyslide_findings(&bodyslide::bodyslide_facts(
        game, inst, mods,
    )));
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
    prefix_facts_with(game, |compat, game, s_drive| {
        eidos_gamefeatures::registry_status(
            compat,
            &game.install_path,
            game.def.registry_name,
            s_drive,
        )
    })
}

/// [`prefix_facts`] with the registry answered by `registry`, called with the
/// prefix path, the game and where `S:` points. Lets the GUI put its memo in
/// front of the shared predicate without keeping a second copy of the rest.
pub fn prefix_facts_with(
    game: &DetectedGame,
    registry: impl FnOnce(&Path, &DetectedGame, Option<&Path>) -> RegistryStatus,
) -> Option<PrefixFacts> {
    let compat = game.compatdata.as_ref()?;
    // Read once and hand to both consumers: the registry check resolves the
    // launcher's `S:`-relative value through THIS link, so the two facts must
    // describe the same instant.
    let gamedrive_found = std::fs::read_link(compat.join("pfx/dosdevices/s:")).ok();
    Some(PrefixFacts {
        game_name: game.def.name.to_string(),
        registry: registry(compat, game, gamedrive_found.as_deref()),
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
    pub max_map_count: Option<u64>,
    pub open_files_hard: Option<u64>,
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
        max_map_count: system::read_max_map_count(),
        open_files_hard: system::read_open_files_hard(),
    }
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

fn or_unknown(n: Option<u64>) -> String {
    n.map_or_else(|| "unknown".into(), |n| n.to_string())
}

fn passthrough_state(f: &ReportFacts) -> &'static str {
    match (f.passthrough_requested, f.cap_missing) {
        (false, _) => "off (default)",
        (true, true) => "requested, launch capability missing",
        (true, false) => "requested, launch capability present",
    }
}

/// The report's first lines, one fact each. Not redacted: the caller redacts
/// the whole report once, with the findings in it.
pub fn report_header(f: &ReportFacts) -> Vec<String> {
    let passthrough = passthrough_state(f);
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
        format!("vm.max_map_count: {}", or_unknown(f.max_map_count)),
        format!("Open files (hard limit): {}", or_unknown(f.open_files_hard)),
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

/// `eidos check <instance> [--json]`: the Health checks that need no window.
/// Exits 0 when nothing is a problem (advice allowed), 1 on a problem, 2 when
/// it could not check.
pub(crate) fn cmd_check(args: &[String]) {
    let json = args.iter().any(|a| a == "--json");
    let rest: Vec<&String> = args.iter().filter(|a| *a != "--json").collect();
    let arg = match rest.as_slice() {
        [arg] if !arg.starts_with('-') => arg.as_str(),
        _ => {
            eidos_log::info!("usage: eidos check <instance> [--json]");
            exit(2);
        }
    };
    let target = match crate::resolve::try_resolve(arg) {
        Ok(target) => target,
        Err(e) => {
            eidos_log::info!("eidos: {e}");
            exit(2);
        }
    };
    let Some(game) = crate::games::find_instance_game(&target) else {
        eidos_log::info!(
            "Game '{}' is not detected. Run `eidos games`.",
            target.game_id
        );
        exit(2);
    };
    let home = eidos_games::home();
    let findings = check_findings(&home, &game, &target.game_id, &target.inst);
    let cap_missing = !eidos_launch::binary_has_cap_sys_admin(&crate::this_exe().0);
    let facts = report_facts(&home, Some(&game), cap_missing);
    if json {
        println!("{}", to_json(&facts, &target.inst.root, &findings));
    } else {
        let mut lines = report_header(&facts);
        lines.push(String::new());
        lines.push("Health:".to_string());
        lines.extend(
            findings
                .iter()
                .map(|f| format!("- [{}] {}: {}", f.level.as_str(), f.code, f.args_line())),
        );
        for line in lines {
            println!("{}", eidos_log::redact(&line));
        }
    }
    exit(exit_code(&findings));
}

/// Every check `eidos check` runs: the prefix, plugin diagnostics, missing
/// masters, the host checks the window shares, and the preflight scans.
fn check_findings(
    home: &Path,
    game: &DetectedGame,
    game_id: &str,
    inst: &Instance,
) -> Vec<Finding> {
    let mut out = prefix_facts(game)
        .map(|f| prefix_findings(&f))
        .unwrap_or_default();
    let list = inst.plugin_list(&game.data_path, game_id, None);
    if let (Some(list), Some(spec)) = (list, eidos_plugins::GameSpec::for_id(game_id)) {
        let mut unverified = 0;
        for d in list.diagnostics(&spec) {
            if d.code == "plugin_records_unverified" {
                unverified += 1;
                continue;
            }
            out.push(Finding {
                code: d.code,
                level: if d.severity == eidos_plugins::DiagnosticSeverity::Error {
                    Level::Problem
                } else {
                    Level::Advice
                },
                args: vec![
                    ("plugin", d.plugin),
                    ("mod", d.origin_mod),
                    ("detail", d.detail),
                ],
            });
        }
        if unverified > 0 {
            out.push(Finding {
                code: "plugin_records_unverified",
                level: Level::Advice,
                args: vec![("n", unverified.to_string())],
            });
        }
        if game_id == "starfield" {
            out.extend(load_order::starfield_findings(&list));
        }
        out.extend(
            list.missing_masters()
                .into_iter()
                .map(|(plugin, master)| Finding {
                    code: "missing_master",
                    level: Level::Problem,
                    args: vec![("plugin", plugin), ("master", master)],
                }),
        );
    }
    let mods = active_mods(inst);
    out.extend(host_findings(home, game, inst, &mods));
    let overwrite = inst.overwrite_dir();
    let preflight = scan_extender_plugins(
        game.def,
        &game.data_path,
        &game.install_path,
        &mods,
        &overwrite,
    )
    .into_iter()
    .chain(scan_fontconfig(
        game.def,
        &game.data_path,
        &mods,
        &overwrite,
    ))
    .chain(scan_compressed_audio(game.def, &mods, &overwrite));
    out.extend(preflight.map(|d| Finding {
        code: d.code,
        level: if d.severity == DiagnosticSeverity::Error {
            Level::Problem
        } else {
            Level::Advice
        },
        args: vec![
            ("path", d.path.display().to_string()),
            ("mod", d.origin_mod),
            ("detail", d.detail),
        ],
    }));
    out
}

/// The machine-readable report. Every string goes through `eidos_log::redact`
/// before it enters the value, so no home path or login name leaves the host.
fn to_json(facts: &ReportFacts, instance: &Path, findings: &[Finding]) -> serde_json::Value {
    let r = |s: &str| serde_json::Value::from(eidos_log::redact(s).into_owned());
    let findings: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            let args: serde_json::Map<String, serde_json::Value> =
                f.args.iter().map(|(k, v)| (k.to_string(), r(v))).collect();
            serde_json::json!({"code": r(f.code), "level": r(f.level.as_str()), "args": args})
        })
        .collect();
    serde_json::json!({
        "eidos": r(&facts.version),
        "game": r(&facts.game_id),
        "instance": r(&instance.display().to_string()),
        "system": {
            "proton": r(&facts.proton_build),
            "flatpak_steam": facts.flatpak_steam,
            "passthrough": r(passthrough_state(facts)),
            "session": r(facts.session),
            "distro": r(&facts.distro),
            "kernel": r(&facts.kernel),
            "dev_fuse": facts.dev_fuse,
            "fusermount3": facts.fusermount3,
            "max_map_count": facts.max_map_count,
            "open_files_hard": facts.open_files_hard,
        },
        "findings": findings,
    })
}

/// 1 when any finding is a problem, else 0. Advice alone never fails a script.
fn exit_code(findings: &[Finding]) -> i32 {
    i32::from(findings.iter().any(|f| f.level == Level::Problem))
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

    fn report() -> ReportFacts {
        ReportFacts {
            version: "1.22.0".into(),
            game_id: "skyrimse".into(),
            proton_build: "GE-Proton11-9".into(),
            flatpak_steam: None,
            passthrough_requested: false,
            cap_missing: true,
            session: "none",
            distro: "Arch Linux".into(),
            kernel: "7.2.15-arch1-1".into(),
            dev_fuse: true,
            fusermount3: true,
            max_map_count: Some(65530),
            open_files_hard: None,
        }
    }

    #[test]
    fn exit_code_is_one_only_for_problems() {
        let advice = Finding {
            code: "game_updated",
            level: Level::Advice,
            args: vec![],
        };
        let problem = Finding {
            code: "missing_master",
            level: Level::Problem,
            args: vec![],
        };
        assert_eq!(exit_code(&[]), 0);
        assert_eq!(exit_code(std::slice::from_ref(&advice)), 0);
        assert_eq!(exit_code(&[advice, problem]), 1);
    }

    #[test]
    fn check_json_is_valid_and_redacted_with_windows_paths() {
        let home = std::env::var("HOME").unwrap();
        let win = format!(r"Z:{}\Games\Skyrim\", home.replace('/', "\\"));
        let f = Finding {
            code: "registry_stale",
            level: Level::Problem,
            args: vec![
                ("found", win.clone()),
                ("want", format!("{home}/Games/Skyrim \"quoted\"\n")),
            ],
        };
        let text = to_json(&report(), Path::new(&format!("{home}/Eidos")), &[f]).to_string();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["findings"][0]["level"], "problem");
        assert_eq!(v["findings"][0]["code"], "registry_stale");
        assert_eq!(v["findings"][0]["args"]["found"], r"Z:~\Games\Skyrim\");
        assert_eq!(v["instance"], "~/Eidos");
        assert_eq!(v["system"]["passthrough"], "off (default)");
        assert_eq!(v["system"]["flatpak_steam"], serde_json::Value::Null);
        assert_eq!(v["system"]["dev_fuse"], true);
        assert_eq!(v["system"]["max_map_count"], 65530);
        assert_eq!(v["system"]["open_files_hard"], serde_json::Value::Null);
        assert!(
            !text.contains(&home) && !text.contains(&home.replace('/', "\\\\")),
            "{text}"
        );
    }

    /// A Skyrim SE plugin header naming `master` as its only master.
    fn plugin_with_master(master: &str) -> Vec<u8> {
        let mut bytes = vec![0u8; 24];
        bytes[..4].copy_from_slice(b"TES4");
        bytes.extend_from_slice(b"MAST");
        bytes.extend_from_slice(&(master.len() as u16 + 1).to_le_bytes());
        bytes.extend_from_slice(master.as_bytes());
        bytes.push(0);
        let len = (bytes.len() - 24) as u32;
        bytes[4..8].copy_from_slice(&len.to_le_bytes());
        bytes
    }

    #[test]
    fn check_findings_map_preflight_and_plugin_diagnostics() {
        let root = std::env::temp_dir().join(format!("eidos-check-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let tmp_home = root.join("home");
        std::fs::create_dir_all(&tmp_home).unwrap();
        let game_dir = root.join("game");
        std::fs::create_dir_all(game_dir.join("Data")).unwrap();
        std::fs::write(game_dir.join("Data/Skyrim.esm"), b"").unwrap();
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
        let inst = Instance::portable(root.join("instance"));
        inst.create().unwrap();
        let mod_dir = inst.mods_dir().join("Needs");
        std::fs::create_dir_all(&mod_dir).unwrap();
        std::fs::write(mod_dir.join("Needs.esp"), plugin_with_master("Missing.esm")).unwrap();
        std::fs::create_dir_all(mod_dir.join("SKSE/Plugins")).unwrap();
        std::fs::write(mod_dir.join("SKSE/Plugins/Crash.dll"), b"invalid PE").unwrap();
        // A saved `*` row (DLC, Creation Club) comes back with an empty path.
        inst.save_modlist(&[
            eidos_instance::ModEntry {
                name: "DLC: Dawnguard".into(),
                enabled: true,
                path: PathBuf::new(),
                unmanaged: true,
            },
            eidos_instance::ModEntry {
                name: "Needs".into(),
                enabled: true,
                path: mod_dir,
                unmanaged: false,
            },
        ])
        .unwrap();
        assert!(inst.modlist().iter().any(|m| m.is_unmanaged()));
        assert!(active_mods(&inst).iter().all(|(n, _)| n == "Needs"));

        let f = check_findings(&tmp_home, &game, "skyrimse", &inst);
        assert!(
            f.iter()
                .all(|f| f.args.iter().all(|(_, v)| !v.contains("Dawnguard"))),
            "{f:?}"
        );

        let missing = f
            .iter()
            .find(|f| f.code == "missing_master")
            .expect("missing_master finding");
        assert_eq!(missing.level, Level::Problem);
        assert_eq!(missing.arg("plugin"), Some("Needs.esp"));
        assert_eq!(missing.arg("master"), Some("Missing.esm"));
        // The game's own Skyrim.esm is empty: a plugin diagnostic, from no mod.
        let header = f
            .iter()
            .find(|f| f.code == "plugin_header")
            .expect("plugin_header finding");
        assert_eq!(header.level, Level::Problem);
        assert_eq!(header.arg("plugin"), Some("Skyrim.esm"));
        assert_eq!(header.arg("mod"), Some(""));
        // A preflight diagnostic keeps its path and the mod it came from.
        let dll = f
            .iter()
            .find(|f| f.code == "pe_unverified")
            .expect("pe_unverified finding");
        assert_eq!(dll.level, Level::Advice);
        assert_eq!(dll.arg("mod"), Some("Needs"));
        assert!(dll.arg("path").is_some_and(|p| p.ends_with("Crash.dll")));
        assert_eq!(exit_code(&f), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
