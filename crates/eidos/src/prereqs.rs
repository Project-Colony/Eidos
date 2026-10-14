//! `eidos prereqs`: the per-game prerequisite installers (runtimes, redists).

use std::process::exit;

use eidos_games::home;
use eidos_instance::{Instance, InstanceKind};

use crate::*;

/// Successful verbs for the selected actual Wine prefix, including its own
/// winetricks log and Eidos's prefix-bound instance receipt.
pub(crate) fn satisfied_prereqs_in(
    inst: &Instance,
    prefix: Option<&std::path::Path>,
) -> std::collections::BTreeSet<String> {
    eidos_gamefeatures::satisfied_prereqs_in(&inst.root, prefix)
}

/// `eidos prereqs <game-id> [--install]`: show, or install, the runtime
/// prerequisites the instance's tools declare. Tier-1 (bundled DirectX DLLs) copy
/// with no network; Tier-2 (vcrun/dotnet) DOWNLOAD from Microsoft via winetricks and
/// so run only on the explicit `--install`.
pub(crate) fn cmd_prereqs(args: &[String]) {
    let Some(id) = args.first() else {
        eidos_log::info!("usage: eidos prereqs <game-id> [--install]");
        exit(2);
    };
    let install = args.iter().any(|a| a == "--install");
    let target = resolve(id);
    let Some(game) = find_instance_game(&target) else {
        eidos_log::info!(
            "Game '{}' is not detected. Run `eidos games`.",
            target.game_id
        );
        exit(1);
    };
    let inst = target.inst;

    // Union of every tool's declared prereqs, split by tier.
    let tools = eidos_instance::merge_tools(inst.tools(), default_tools_for(&game, Some(&inst)));
    let mut verbs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for t in &tools {
        verbs.extend(t.prereqs.iter().cloned());
    }
    let tier1: Vec<String> = verbs
        .iter()
        .filter(|v| eidos_gamefeatures::is_tier1_dll(v))
        .cloned()
        .collect();
    let tier2: Vec<String> = verbs
        .iter()
        .filter(|v| eidos_gamefeatures::is_tier2_verb(v))
        .cloned()
        .collect();
    // A verb that is neither a bundled DLL nor a known winetricks verb (a tools.ini
    // typo, or one Eidos hasn't catalogued): surface it rather than silently drop it.
    // Tier 3: a self-contained runtime Eidos fetches itself. Not a bundled DLL
    // and not a winetricks verb, so it needs its own bucket - and must not fall
    // into `unknown`, which exists to catch typos.
    let tier3: Vec<String> = verbs
        .iter()
        .filter(|v| eidos_gamefeatures::is_runtime_verb(v))
        .cloned()
        .collect();
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
        .filter(|v| {
            eidos_gamefeatures::runtime(v)
                .is_some_and(|r| !eidos_gamefeatures::runtime_is_installed(r))
        })
        .cloned()
        .collect();
    let prefix = game.prefix();
    let satisfied = satisfied_prereqs_in(&inst, prefix.as_deref());
    let pending2: Vec<String> = tier2
        .iter()
        .filter(|v| !satisfied.contains(*v))
        .cloned()
        .collect();

    if !install {
        println!("Tool prerequisites for {id}:");
        for t in &tools {
            if !t.prereqs.is_empty() {
                println!("  {:<16} {}", t.title, t.prereqs.join(", "));
            }
        }
        let t1 = if tier1.is_empty() {
            "(none)".to_string()
        } else {
            tier1.join(", ")
        };
        println!("\nTier 1 (bundled, applied at launch, no download): {t1}");
        let t2 = if tier2.is_empty() {
            "(none)".to_string()
        } else {
            tier2
                .iter()
                .map(|v| {
                    format!(
                        "{v} [{}]",
                        if satisfied.contains(v) {
                            "done"
                        } else {
                            "pending"
                        }
                    )
                })
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
            println!(
                "Unknown verbs (ignored - typo or uncatalogued): {}",
                unknown.join(", ")
            );
        }
        let waiting: Vec<String> = pending2.iter().chain(pending3.iter()).cloned().collect();
        if !waiting.is_empty() {
            println!(
                "\nRun `eidos prereqs {id} --install` to download + install: {}",
                waiting.join(", ")
            );
        }
        return;
    }

    if !unknown.is_empty() {
        eidos_log::warn!(
            "eidos prereqs: ignoring unknown verb(s): {}",
            unknown.join(", ")
        );
    }

    let _lock = match inst.try_lock("installing tool prerequisites") {
        Ok(lock) => lock,
        Err(e) => {
            eidos_log::warn!("Cannot install prerequisites: {e}");
            exit(1);
        }
    };
    if let Err(e) = inst
        .create()
        .and_then(|_| inst.ensure_manifest(&target.game_id, InstanceKind::Global))
    {
        eidos_log::warn!("Cannot prepare the instance: {e}");
        exit(1);
    }
    // Inspect the prefix before changing any bundled DLL or launching winetricks.
    if let Some(prefix) = prefix.as_deref() {
        let compat = game
            .is_steam()
            .then_some(game.compatdata.as_deref())
            .flatten();
        let busy = eidos_gamefeatures::prefix_busy_at(prefix, compat);
        if !busy.is_empty() {
            eidos_log::warn!("The prefix is still in use by {} process(es); close the game and its tools before installing prerequisites.", busy.len());
            exit(1);
        }
    }
    let mut failed = false;
    // Shared runtimes can be installed even before Steam creates a prefix.
    for v in &pending3 {
        match eidos_gamefeatures::install_runtime(v, |step| println!("  {v}: {step}")) {
            Ok(true) => println!("installed {v}"),
            Ok(false) => {}
            Err(e) => {
                eidos_log::warn!("could not install {v}: {e}");
                failed = true;
            }
        }
    }
    if tier1.is_empty() && pending2.is_empty() {
        if failed {
            exit(1);
        }
        return;
    }
    let Some(prefix) = prefix.as_deref() else {
        eidos_log::warn!("No Wine prefix is configured for {id}. Launch the game with its selected runner or configure its actual prefix first.");
        exit(1);
    };
    let win = prefix.join("drive_c").join("windows");
    for v in &tier1 {
        match eidos_gamefeatures::ensure_native_dll(&win, v) {
            Ok(true) => println!("provisioned {v} (bundled)"),
            Ok(false) => {}
            Err(e) => {
                eidos_log::warn!("could not provision {v}: {e}");
                failed = true;
            }
        }
    }
    if pending2.is_empty() {
        if failed {
            exit(1);
        }
        println!("Tool prerequisites are ready.");
        return;
    }
    if failed {
        exit(1);
    }
    if !game.is_steam() {
        eidos_log::warn!("Bundled DLLs were handled in the selected prefix. Installing Tier-2 prerequisites for an external copy requires its actual Wine runner; use that runner's winetricks integration. Eidos will read its winetricks.log on the next check.");
        exit(1);
    }
    let Some(compat) = game.compatdata.as_ref() else {
        eidos_log::warn!("No Steam Proton compatdata is configured for {id}.");
        exit(1);
    };
    let Some(run) =
        eidos_games::proton_command(&home(), game.def.steam_app_id, compat, &game.install_path)
    else {
        eidos_log::warn!("Could not resolve Proton for {id}.");
        exit(1);
    };
    warn_if_flatpak_proton(&run);
    if !eidos_gamefeatures::cabextract_available() {
        eidos_log::info!("warning: cabextract not on PATH - some winetricks verbs need it (e.g. `pacman -S cabextract`).");
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
        match eidos_gamefeatures::install_tier2_verb(&run.proton, prefix, &run.env, v) {
            Ok(()) => {
                done.insert(v.clone());
                if let Err(e) = eidos_gamefeatures::record_prereqs(&inst.root, prefix, &done) {
                    eidos_log::warn!("Installed {v}, but could not record it: {e}");
                    exit(1);
                }
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
            eidos_log::warn!("winetricks failed on '{v}': {e}");
            eidos_log::info!(
                "(earlier verbs were recorded; re-run `eidos prereqs {id} --install` to resume.)"
            );
            exit(1);
        }
    }
}
