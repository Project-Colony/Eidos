//! `eidos prereqs`: the per-game prerequisite installers (runtimes, redists).

use std::process::exit;

use eidos_games::home;
use eidos_instance::{Instance, InstanceKind};

use crate::*;

/// The Tier-2 prereq verbs already installed into the prefix (the `prereqs.done`
/// sentinel in the instance dir), so a re-run is a no-op and the tool warning is quiet.
pub(crate) fn satisfied_prereqs(inst: &Instance) -> std::collections::BTreeSet<String> {
    std::fs::read_to_string(inst.root.join("prereqs.done"))
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}

/// What this machine already has, from BOTH records: Eidos's own, and the
/// prefix's. winetricks appends every verb it installs to `winetricks.log`
/// inside the prefix, and protontricks is winetricks - so a user who set a
/// runtime up years ago is not asked to download it again.
pub(crate) fn satisfied_prereqs_in(
    inst: &Instance,
    compatdata: Option<&std::path::PathBuf>,
) -> std::collections::BTreeSet<String> {
    let mut done = satisfied_prereqs(inst);
    if let Some(c) = compatdata {
        done.extend(eidos_gamefeatures::verbs_in_prefix(&c.join("pfx")));
    }
    done
}

/// `eidos prereqs <game-id> [--install]`: show, or install, the runtime
/// prerequisites the instance's tools declare. Tier-1 (bundled DirectX DLLs) copy
/// with no network; Tier-2 (vcrun/dotnet) DOWNLOAD from Microsoft via winetricks and
/// so run only on the explicit `--install`.
pub(crate) fn cmd_prereqs(args: &[String]) {
    let Some(id) = args.first() else {
        eprintln!("usage: eidos prereqs <game-id> [--install]");
        exit(2);
    };
    let install = args.iter().any(|a| a == "--install");
    let target = resolve(id);
    let Some(game) = find_game(&target.game_id) else {
        eprintln!("Game '{}' is not detected. Run `eidos games`.", target.game_id);
        exit(1);
    };
    let inst = target.inst;
    inst.create().ok();
    let _ = inst.ensure_manifest(&target.game_id, InstanceKind::Global);

    // Union of every tool's declared prereqs, split by tier.
    let tools = eidos_instance::merge_tools(inst.tools(), default_tools_for(&game, Some(&inst)));
    let mut verbs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for t in &tools {
        verbs.extend(t.prereqs.iter().cloned());
    }
    let tier1: Vec<String> = verbs.iter().filter(|v| eidos_gamefeatures::is_tier1_dll(v)).cloned().collect();
    let tier2: Vec<String> = verbs.iter().filter(|v| eidos_gamefeatures::is_tier2_verb(v)).cloned().collect();
    // A verb that is neither a bundled DLL nor a known winetricks verb (a tools.ini
    // typo, or one Eidos hasn't catalogued): surface it rather than silently drop it.
    // Tier 3: a self-contained runtime Eidos fetches itself. Not a bundled DLL
    // and not a winetricks verb, so it needs its own bucket - and must not fall
    // into `unknown`, which exists to catch typos.
    let tier3: Vec<String> =
        verbs.iter().filter(|v| eidos_gamefeatures::is_runtime_verb(v)).cloned().collect();
    let unknown: Vec<String> = verbs
        .iter()
        .filter(|v| {
            !eidos_gamefeatures::is_tier1_dll(v)
                && !eidos_gamefeatures::is_tier2_verb(v)
                && !eidos_gamefeatures::is_runtime_verb(v)
        })
        .cloned()
        .collect();
    let pending3: Vec<String> = tier3
        .iter()
        .filter(|v| eidos_gamefeatures::runtime(v).is_some_and(|r| !eidos_gamefeatures::runtime_is_installed(r)))
        .cloned()
        .collect();
    let satisfied = satisfied_prereqs_in(&inst, game.compatdata.as_ref());
    let pending2: Vec<String> = tier2.iter().filter(|v| !satisfied.contains(*v)).cloned().collect();

    if !install {
        println!("Tool prerequisites for {id}:");
        for t in &tools {
            if !t.prereqs.is_empty() {
                println!("  {:<16} {}", t.title, t.prereqs.join(", "));
            }
        }
        let t1 = if tier1.is_empty() { "(none)".to_string() } else { tier1.join(", ") };
        println!("\nTier 1 (bundled, applied at launch, no download): {t1}");
        let t2 = if tier2.is_empty() {
            "(none)".to_string()
        } else {
            tier2
                .iter()
                .map(|v| format!("{v} [{}]", if satisfied.contains(v) { "done" } else { "pending" }))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!("Tier 2 (winetricks, DOWNLOADS from Microsoft): {t2}");
        if !tier3.is_empty() {
            let t3 = tier3
                .iter()
                .map(|v| {
                    let done = eidos_gamefeatures::runtime(v)
                        .is_some_and(eidos_gamefeatures::runtime_is_installed);
                    format!("{v} [{}]", if done { "done" } else { "missing" })
                })
                .collect::<Vec<_>>()
                .join(", ");
            println!("Tier 3 (runtime, DOWNLOADS once, shared by every instance): {t3}");
        }
        if !unknown.is_empty() {
            println!("Unknown verbs (ignored - typo or uncatalogued): {}", unknown.join(", "));
        }
        let waiting: Vec<String> =
            pending2.iter().chain(pending3.iter()).cloned().collect();
        if !waiting.is_empty() {
            println!(
                "\nRun `eidos prereqs {id} --install` to download + install: {}",
                waiting.join(", ")
            );
        }
        return;
    }

    if !unknown.is_empty() {
        eprintln!("eidos prereqs: ignoring unknown verb(s): {}", unknown.join(", "));
    }

    // --install: Tier 1 (copy bundled DLLs) then the consented Tier 2 (winetricks).
    let Some(compat) = game.compatdata.as_ref() else {
        eprintln!("No Proton prefix for {id} - launch the game once through Steam first.");
        exit(1);
    };
    let win = compat.join("pfx").join("drive_c").join("windows");
    for v in &tier1 {
        match eidos_gamefeatures::ensure_native_dll(&win, v) {
            Ok(true) => println!("provisioned {v} (bundled)"),
            Ok(false) => {}
            Err(e) => eprintln!("could not provision {v}: {e}"),
        }
    }
    // Tier 3 first: it needs no prefix and no Proton, so a machine that cannot
    // run winetricks can still get its runtime.
    for v in &pending3 {
        match eidos_gamefeatures::install_runtime(v, |step| println!("  {v}: {step}")) {
            Ok(true) => println!("installed {v}"),
            Ok(false) => {}
            Err(e) => eprintln!("could not install {v}: {e}"),
        }
    }
    if pending2.is_empty() {
        if pending3.is_empty() {
            println!("Tier 2 already satisfied (nothing to download).");
        }
        return;
    }
    let Some(run) =
        eidos_games::proton_command(&home(), game.def.steam_app_id, compat, &game.install_path)
    else {
        eprintln!("Could not resolve Proton for {id}.");
        exit(1);
    };
    warn_if_flatpak_proton(&run);
    if !eidos_gamefeatures::cabextract_available() {
        eprintln!("warning: cabextract not on PATH - some winetricks verbs need it (e.g. `pacman -S cabextract`).");
    }
    let prefix = compat.join("pfx");
    // Refuse rather than corrupt. A game, Steam or a stale wineserver still
    // attached to this prefix holds registry and filesystem locks that a
    // winetricks run waits on forever - and the prefix may belong to a session
    // the user is deliberately keeping open, so naming the processes and stopping
    // is the right move. We do not kill anything on the user's behalf.
    let busy = eidos_gamefeatures::prefix_busy(&prefix, compat);
    if !busy.is_empty() {
        eprintln!("This prefix is still in use by {} process(es):", busy.len());
        for (pid, cmd) in busy.iter().take(10) {
            eprintln!("  pid {pid}: {}", cmd.chars().take(100).collect::<String>());
        }
        eprintln!("Close the game, its tools and Steam, then run this again.");
        exit(1);
    }
    println!(
        "Installing {} via winetricks (downloads from Microsoft).",
        pending2.join(", ")
    );
    // One verb at a time, recording each success, so a later failure does not lose
    // the earlier installs (a single batched winetricks call would).
    let mut done = satisfied;
    let mut failed: Option<(String, String)> = None;
    for v in &pending2 {
        println!("  installing {v}...");
        match eidos_gamefeatures::install_tier2_verb(&run.proton, &prefix, &run.env, v) {
            Ok(()) => {
                done.insert(v.clone());
                let body: String = done.iter().map(|x| format!("{x}\n")).collect();
                let _ = std::fs::write(inst.root.join("prereqs.done"), body);
            }
            Err(e) => {
                failed = Some((v.clone(), e.to_string()));
                break;
            }
        }
    }
    match failed {
        None => println!("Done."),
        Some((v, e)) => {
            eprintln!("winetricks failed on '{v}': {e}");
            eprintln!("(earlier verbs were recorded; re-run `eidos prereqs {id} --install` to resume.)");
            exit(1);
        }
    }
}
