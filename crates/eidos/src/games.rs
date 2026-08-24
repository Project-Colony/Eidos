//! `eidos games` and `eidos init`: detection, and creating an instance.

use std::process::exit;

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::{Instance, InstanceKind};


pub(crate) fn find_game(id: &str) -> Option<DetectedGame> {
    detect(&home()).into_iter().find(|g| g.def.id == id)
}

pub(crate) fn cmd_games() {
    let games = detect(&home());
    if games.is_empty() {
        println!("No supported games detected. Make sure Steam is installed and the game is downloaded.");
        return;
    }
    println!("Supported games installed:");
    for g in &games {
        println!("  {:<10} {}  (Steam: {})", g.def.id, g.def.name, g.steam_name);
        println!("             data: {}", g.data_path.display());
    }
    println!("\nNext: `eidos init <id>` to create a modding instance.");
}

pub(crate) fn cmd_init(id: &str, folder: Option<&str>) {
    let Some(game) = find_game(id) else {
        eidos_log::info!("Game '{id}' is not detected. Run `eidos games` to see what's available.");
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
            if let Some(g) =
                detect(&home()).into_iter().find(|g| Instance::root_inside_game(&root, &g.install_path))
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
    inst.create().expect("create instance");
    let _ = inst.ensure_manifest(id, kind);
    if kind == InstanceKind::Portable {
        let mut reg = eidos_instance::Registry::load();
        reg.set_last(eidos_instance::InstanceRef::Portable(inst.root.clone()));
        let _ = reg.save();
    }
    // The old text promised a `../load_order.txt` that NOTHING has ever read
    // since profiles arrived: a user following it created a file with no effect
    // and believed their order applied. Describe the mechanism that exists.
    let _ = std::fs::write(
        inst.mods_dir().join("README.txt"),
        "Drop each mod here as its own folder.\n\
         A folder added by hand appears DISABLED at the bottom of the mod list;\n\
         enable and order it in the Eidos GUI (or edit the active profile's\n\
         profiles/<name>/modlist.txt: one +Name/-Name per line, top = highest\n\
         priority, wins file conflicts - MO2's format).\n",
    );
    println!("Created {} instance for {} ({id}).", if kind == InstanceKind::Portable { "portable" } else { "global" }, game.def.name);
    println!("  instance : {}", inst.root.display());
    println!("  game data: {}", game.data_path.display());
    println!("  add mods : {}", inst.mods_dir().display());
    // A portable instance is addressed by its folder from here on.
    let inst_arg = match kind {
        InstanceKind::Global => id.to_string(),
        InstanceKind::Portable => inst.root.display().to_string(),
    };
    println!("\nThen: `eidos play {inst_arg} -- %command%` (as a Steam launch option).");
}
