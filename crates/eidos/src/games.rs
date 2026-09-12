//! `eidos games` and `eidos init`: detection, and creating an instance.

use std::process::exit;

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::{Instance, InstanceKind};

pub(crate) fn find_game(id: &str) -> Option<DetectedGame> {
    eidos_games::select_installation(&detect(&home()), id, None).cloned()
}

pub(crate) fn find_instance_game(target: &crate::resolve::Target) -> Option<DetectedGame> {
    let manifest = match eidos_instance::Manifest::read_checked(&target.inst.manifest_path()) {
        Ok(value) => value,
        Err(error) => {
            eidos_log::warn!("Cannot read the instance identity: {error}");
            return None;
        }
    };
    let games = detect(&home());
    let selected = eidos_games::select_installation(&games, &target.game_id,
        manifest.as_ref().and_then(|m| m.installation.as_deref())).cloned();
    if selected.is_none() && games.iter().any(|g| g.def.id == target.game_id) {
        eidos_log::warn!("The saved installation is unavailable or multiple copies exist. Select the exact game path when creating/opening the instance; Eidos will not choose another copy.");
    }
    selected
}

pub(crate) fn cmd_games() {
    let games = detect(&home());
    if games.is_empty() {
        println!(
            "No supported games detected. Check the Steam, Heroic or Legendary installed manifests."
        );
        return;
    }
    println!("Supported games installed:");
    for g in &games {
        println!(
            "  {:<10} {}  ({})",
            g.def.id, g.def.name, g.source_name()
        );
        println!("             game: {}", g.install_path.display());
        println!("             data: {}", g.data_path.display());
    }
    println!("\nNext: `eidos init <id>` to create a modding instance.");
}

pub(crate) fn cmd_init(id: &str, folder: Option<&str>, game_path: Option<&str>) {
    let games = detect(&home());
    let game = if let Some(path) = game_path {
        let canonical = std::fs::canonicalize(crate::resolve::expand(path)).ok();
        let mut matched = games.iter().filter(|g| g.def.id == id && canonical.as_ref() == Some(&g.install_path));
        let first = matched.next().cloned();
        if matched.next().is_some() { None } else { first }
    } else { find_game(id) };
    let Some(game) = game else {
        eidos_log::info!("Game '{id}' is not detected. Run `eidos games` and select a copy with --game-path <folder>.");
        exit(1);
    };
    // With a folder the instance is PORTABLE: self-contained there, movable,
    // and remembered in the registry so the GUI and later commands find it.
    let (inst, kind) = match folder {
        Some(f) => {
            let root = crate::resolve::expand(f);
            // Never inside a game's install - any detected game's. Steam owns
            // those trees (updates/uninstalls rewrite or delete them) and
            // Eidos mounts over the game root, so an instance there would sit
            // inside its own mount target.
            if let Some(g) = detect(&home())
                .into_iter()
                .find(|g| Instance::root_inside_game(&root, &g.install_path))
            {
                eidos_log::warn!(
                    "'{}' is inside {}'s own folder - an instance cannot live there.\n\
                     Steam owns that tree (an update or uninstall can wipe it), and Eidos\n\
                     mounts over the game root, so the instance would live inside its own\n\
                     mount target. Put it NEXT to the game instead, e.g. a sibling folder.",
                    root.display(),
                    g.def.name
                );
                exit(1);
            }
            if let Some(m) = Instance::portable(root.clone()).read_manifest() {
                if m.game_id != id {
                    eidos_log::info!(
                        "'{}' already holds a '{}' instance - not stamping it as '{id}'.",
                        root.display(),
                        m.game_id
                    );
                    exit(1);
                }
            }
            (Instance::portable(root), InstanceKind::Portable)
        }
        None => (Instance::global(id), InstanceKind::Global),
    };
    let _lock = inst.try_lock("creating an instance").unwrap_or_else(|error| {
        eidos_log::warn!("Cannot create this instance: {error}"); exit(1);
    });
    if let Err(error) = inst.create().and_then(|()| inst.ensure_installation(id, kind, &game.selection_id())) {
        eidos_log::warn!("Cannot create this instance: {error}"); exit(1);
    }
    if kind == InstanceKind::Portable {
        let mut reg = eidos_instance::Registry::load();
        reg.set_last(eidos_instance::InstanceRef::Portable(inst.root.clone()));
        if let Err(error) = reg.save() {
            eidos_log::warn!("Instance created, but it could not be registered: {error}"); exit(1);
        }
    }
    // The old text promised a `../load_order.txt` that NOTHING has ever read
    // since profiles arrived: a user following it created a file with no effect
    // and believed their order applied. Describe the mechanism that exists.
    if let Err(error) = std::fs::write(
        inst.mods_dir().join("README.txt"),
        "Drop each mod here as its own folder.\n\
         A folder added by hand appears DISABLED at the bottom of the mod list;\n\
         enable and order it in the Eidos GUI (or edit the active profile's\n\
         profiles/<name>/modlist.txt: one +Name/-Name per line, top = highest\n\
         priority, wins file conflicts - MO2's format).\n",
    ) {
        eidos_log::warn!("Instance created, but its instructions could not be saved: {error}"); exit(1);
    }
    println!(
        "Created {} instance for {} ({id}).",
        if kind == InstanceKind::Portable {
            "portable"
        } else {
            "global"
        },
        game.def.name
    );
    println!("  instance : {}", inst.root.display());
    println!("  game data: {}", game.data_path.display());
    println!("  add mods : {}", inst.mods_dir().display());
    // A portable instance is addressed by its folder from here on.
    let inst_arg = match kind {
        InstanceKind::Global => id.to_string(),
        InstanceKind::Portable => inst.root.display().to_string(),
    };
    if game.is_steam() {
        println!("\nThen: `eidos play {inst_arg} -- %command%` (as a Steam launch option).");
    } else {
        println!("\nLaunch manually with `eidos play {inst_arg} -- <runner> <game>`. Use this installation's runner and prefix; automatic Heroic/Legendary launching is not configured.");
    }
}
