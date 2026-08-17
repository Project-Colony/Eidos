//! `eidos install` and `eidos import`: archives and existing folders in.

use std::process::exit;

use eidos_instance::{InstanceKind, ModEntry};

use crate::*;

pub(crate) fn cmd_install(args: &[String]) {
    let (Some(id), Some(archive)) = (args.first(), args.get(1)) else {
        eprintln!("usage: eidos install <game-id-or-instance-path> <archive> [name]");
        exit(2);
    };
    let target = resolve(id);
    let Some(game) = find_game(&target.game_id) else {
        eprintln!("Game '{}' is not detected. Run `eidos games`.", target.game_id);
        exit(1);
    };
    let inst = target.inst;
    inst.create().ok();
    let _ = inst.ensure_manifest(&target.game_id, InstanceKind::Global);

    // Optional overwrite policy; the positional name is the first non-flag arg.
    let policy = if args.iter().any(|a| a == "--replace") {
        eidos_install::OverwritePolicy::Replace
    } else if args.iter().any(|a| a == "--merge") {
        eidos_install::OverwritePolicy::Merge
    } else {
        eidos_install::OverwritePolicy::Fail
    };
    let name = args
        .iter()
        .skip(2)
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| eidos_install::mod_name_for(std::path::Path::new(archive)));
    // FOMOD condition context: plugins in enabled mods read Active, plugins in
    // disabled mods read Inactive, so a scripted installer's fileDependency /
    // gameDependency options evaluate correctly (MO2 distinguishes the two).
    let ml = inst.modlist();
    let enabled_roots: Vec<std::path::PathBuf> =
        ml.iter().filter(|m| m.enabled && !m.is_separator()).map(|m| m.path.clone()).collect();
    let disabled_roots: Vec<std::path::PathBuf> =
        ml.iter().filter(|m| !m.enabled && !m.is_separator()).map(|m| m.path.clone()).collect();
    let ctx = eidos_install::fomod_context(&game.data_path, &enabled_roots, &disabled_roots);
    match eidos_install::install_archive_with_policy(
        std::path::Path::new(archive),
        &inst.mods_dir(),
        &name,
        &target.game_id,
        policy,
        &ctx,
    ) {
        Ok(r) => {
            // Give the new mod the highest priority so it wins conflicts by default,
            // like MO2. modlist() is lowest-priority-first, so highest = the END.
            let mut ml = inst.modlist();
            ml.retain(|m| m.name != r.name);
            ml.push(ModEntry { name: r.name.clone(), enabled: true, path: r.dest.clone(), unmanaged: false });
            let _ = inst.save_modlist(&ml);

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
            if !r.missing.is_empty() {
                eprintln!("  note: {} file(s) the installer expected were not in the archive:", r.missing.len());
                for m in &r.missing {
                    eprintln!("    - {m}");
                }
            }
            println!("  enabled at the top of the load order. `eidos play {id}` to use it.");
        }
        Err(e) => {
            eprintln!("install failed: {e}");
            if matches!(e, eidos_install::InstallError::Exists(_)) {
                eprintln!("  (re-run with --replace to reinstall it, or --merge to install over it)");
            }
            exit(1);
        }
    }
}

/// `eidos import <game-id> <mo2-profile-dir>`: adopt an existing Mod Organizer 2
/// profile's mod order, enabled states and load order.
pub(crate) fn cmd_import(args: &[String]) -> ! {
    let (Some(id), Some(dir)) = (args.first(), args.get(1)) else { usage() };
    let target = resolve(id);
    let inst = target.inst;
    if !inst.exists() {
        eprintln!("eidos import: no instance for '{id}' - run `eidos init {id}` first.");
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
                println!("{} local mod(s) MO2 did not list were kept at the bottom.", r.kept_local);
            }
            if r.plugin_files > 0 {
                println!("Load order imported ({} file(s)).", r.plugin_files);
            }
            if !r.missing.is_empty() {
                println!("\n{} mod(s) MO2 listed are not installed here:", r.missing.len());
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
            eprintln!("eidos import: {e}");
            exit(1)
        }
    }
}
