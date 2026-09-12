//! `eidos install` and `eidos import`: archives and existing folders in.

use std::process::exit;

use eidos_instance::InstanceKind;

use crate::*;

pub(crate) fn cmd_install(args: &[String]) {
    let (Some(id), Some(archive)) = (args.first(), args.get(1)) else {
        eidos_log::info!("usage: eidos install <game-id-or-instance-path> <archive> [name]");
        exit(2);
    };
    let target = resolve(id);
    let Some(game) = find_instance_game(&target) else {
        eidos_log::info!(
            "Game '{}' is not detected. Run `eidos games`.",
            target.game_id
        );
        exit(1);
    };
    let inst = target.inst;
    let _lock = inst.try_lock("eidos install").unwrap_or_else(|error| {
        eidos_log::warn!("Cannot install now: {error}");
        exit(1);
    });
    if let Err(error) = inst.create() {
        eidos_log::warn!("Cannot prepare this instance: {error}");
        exit(1);
    }
    if let Err(error) = inst.ensure_manifest(&target.game_id, InstanceKind::Global) {
        eidos_log::warn!("Cannot prepare this instance: {error}");
        exit(1);
    }

    // Optional overwrite policy; the positional name is the first non-flag arg.
    let backup = args.iter().any(|a| a == "--backup");
    let policy = if args.iter().any(|a| a == "--replace") {
        if backup { eidos_install::OverwritePolicy::ReplaceWithBackup } else { eidos_install::OverwritePolicy::Replace }
    } else if args.iter().any(|a| a == "--merge") {
        if backup { eidos_install::OverwritePolicy::MergeWithBackup } else { eidos_install::OverwritePolicy::Merge }
    } else {
        eidos_install::OverwritePolicy::Fail
    };
    let name = args
        .iter()
        .skip(2)
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| eidos_install::mod_name_for(std::path::Path::new(archive)));
    let fallback = game.prefix().and_then(|prefix| {
        game.plugin_spec()
            .map(|spec| eidos_plugins::plugins_txt_dir(&prefix, &spec))
    });
    let ctx = eidos_install::fomod_context_for_instance(
        &inst,
        &game.data_path,
        &target.game_id,
        fallback.as_deref(),
    );
    match eidos_install::install_archive_with_policy(
        std::path::Path::new(archive),
        &inst.mods_dir(),
        &name,
        &target.game_id,
        policy,
        &ctx,
    ) {
        Ok(r) => {
            if let Err(error) = inst.register_installed_mod(&r.name) {
                eidos_log::warn!("Installed '{}', but could not register it: {error}", r.name);
                exit(1);
            }

            // If this archive came from a Nexus download, flag its .meta installed
            // (MO2's markInstalled); a no-op when there is no sidecar.
            let _ = eidos_nexus::mark_installed(std::path::Path::new(archive));

            print!("Installed '{}' for {}", r.name, game.def.name);
            if r.fomod {
                print!(" (via FOMOD, default options)");
            } else if !r.stripped.is_empty() {
                print!(" (stripped wrapper '{}')", r.stripped.trim_end_matches('/'));
            }
            println!();
            println!("  -> {}", r.dest.display());
            if let Some(backup) = r.backup { println!("  backup: {}", backup.display()); }
            if !r.missing.is_empty() {
                eidos_log::warn!(
                    "  note: {} file(s) the installer expected were not in the archive:",
                    r.missing.len()
                );
                for m in &r.missing {
                    eidos_log::info!("    - {m}");
                }
            }
            println!("  Registered in the active profile; an existing mod keeps its priority and enabled state. `eidos play {id}` to use it.");
        }
        Err(e) => {
            eidos_log::warn!("install failed: {e}");
            if matches!(e, eidos_install::InstallError::Exists(_)) {
                eidos_log::info!(
                    "  (re-run with --replace to reinstall it, or --merge to install over it)"
                );
            }
            exit(1);
        }
    }
}

/// `eidos import <game-id> <mo2-profile-dir>`: adopt an existing Mod Organizer 2
/// profile's mod order, enabled states and load order.
pub(crate) fn cmd_import(args: &[String]) -> ! {
    let (Some(id), Some(dir)) = (args.first(), args.get(1)) else {
        usage()
    };
    let target = resolve(id);
    let inst = target.inst;
    if !inst.exists() {
        eidos_log::info!("eidos import: no instance for '{id}' - run `eidos init {id}` first.");
        exit(1);
    }
    match inst.import_mo2_profile(std::path::Path::new(dir)) {
        Ok(r) => {
            println!(
                "Imported {} mod(s) from {dir} into profile '{}'.",
                r.matched,
                inst.active_profile()
            );
            if r.kept_local > 0 {
                println!(
                    "{} local mod(s) MO2 did not list were kept at the bottom.",
                    r.kept_local
                );
            }
            if r.plugin_files > 0 {
                println!("Load order imported ({} file(s)).", r.plugin_files);
            }
            if !r.missing.is_empty() {
                println!(
                    "\n{} mod(s) MO2 listed are not installed here:",
                    r.missing.len()
                );
                for m in r.missing.iter().take(40) {
                    println!("  - {m}");
                }
                if r.missing.len() > 40 {
                    println!("  ... and {} more", r.missing.len() - 40);
                }
                println!("Install them, then run this again to place them in order.");
            }
            exit(0)
        }
        Err(e) => {
            eidos_log::info!("eidos import: {e}");
            exit(1)
        }
    }
}
