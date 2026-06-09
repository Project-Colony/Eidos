//! `eidos`: the front end that ties detection, instances, and launching together.
//!
//!   eidos games                       list supported games installed on this system
//!   eidos init <game-id>              create a (global) modding instance
//!   eidos play <game-id>              show how to launch / what is mounted
//!   eidos play <game-id> -- <cmd...>  run <cmd> with the mods mounted over the game
//!
//! Instances (global vs portable, layout, load order) live in `eidos-instance`.
//! `play` mounts the instance's mods over the game's own Data directory (via a
//! bind-stash) inside a private namespace, then runs the command through it.

use std::process::exit;

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::{Instance, InstanceKind, ModEntry};
use eidos_launch::{launch, LaunchSpec};

fn find_game(id: &str) -> Option<DetectedGame> {
    detect(&home()).into_iter().find(|g| g.def.id == id)
}

/// Before launch: discover this instance's plugins, preserve any existing load
/// order from the prefix, re-validate the invariants, and write
/// `plugins.txt`/`loadorder.txt` where the game reads them. Best-effort - a game
/// with no plugin system or no Proton prefix is simply skipped.
fn prepare_plugins(id: &str, game: &DetectedGame, inst: &Instance) {
    let Some(spec) = eidos_plugins::GameSpec::for_id(id) else { return };
    let Some(compatdata) = game.compatdata.as_ref() else {
        eprintln!("eidos play: no Proton prefix found, skipping plugins.txt");
        return;
    };
    let prefix = compatdata.join("pfx");

    // Sources: the game's own Data first (the base masters), then each enabled mod
    // in ascending plugin priority (the modlist is highest-first, so reverse it).
    let mut sources: Vec<(String, std::path::PathBuf)> =
        vec![(String::new(), game.data_path.clone())];
    let mut enabled: Vec<_> = inst.modlist().into_iter().filter(|m| m.enabled).collect();
    enabled.reverse();
    sources.extend(enabled.into_iter().map(|m| (m.name, m.path)));

    let mut list = eidos_plugins::PluginList::discover(&sources, &spec);

    // Preserve the user's existing order (their MO2 or prior-run plugins.txt).
    let dir = eidos_plugins::plugins_txt_dir(&prefix, &spec);
    let existing = eidos_plugins::PluginList::read_active(&dir, &spec);
    if !existing.is_empty() {
        list.apply_active(&existing);
    }
    list.refresh(&spec);

    for (p, m) in list.missing_masters() {
        eprintln!("eidos play: WARNING - {p} is missing master {m} (likely a crash)");
    }
    let active = list.plugins.iter().filter(|p| p.enabled).count();
    match list.write_load_order(&dir, &spec) {
        Ok(()) => eprintln!("eidos play: wrote {active} active plugins to plugins.txt"),
        Err(e) => eprintln!("eidos play: could not write plugins.txt: {e}"),
    }
}

fn cmd_games() {
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

fn cmd_init(id: &str) {
    let Some(game) = find_game(id) else {
        eprintln!("Game '{id}' is not detected. Run `eidos games` to see what's available.");
        exit(1);
    };
    let inst = Instance::global(id);
    inst.create().expect("create instance");
    let _ = inst.ensure_manifest(id, InstanceKind::Global);
    let _ = std::fs::write(
        inst.mods_dir().join("README.txt"),
        "Drop each mod here as its own folder.\n\
         Load order is alphabetical unless a ../load_order.txt lists folder\n\
         names (top line = highest priority, wins file conflicts).\n",
    );
    println!("Created instance for {} ({id}).", game.def.name);
    println!("  instance : {}", inst.root.display());
    println!("  game data: {}", game.data_path.display());
    println!("  add mods : {}", inst.mods_dir().display());
    println!("\nThen: `eidos play {id} -- %command%` (as a Steam launch option).");
}

fn cmd_play(args: &[String]) {
    let Some(id) = args.first() else {
        eprintln!("usage: eidos play <game-id> [-- <command>...]");
        exit(2);
    };
    let command: Vec<String> = match args.iter().position(|a| a == "--") {
        Some(i) => args[i + 1..].to_vec(),
        None => Vec::new(),
    };

    let Some(game) = find_game(id) else {
        eprintln!("Game '{id}' is not detected. Run `eidos games`.");
        exit(1);
    };

    let inst = Instance::global(id);
    inst.create().ok();
    let _ = inst.ensure_manifest(id, InstanceKind::Global);
    let layers = inst.load_order();

    if command.is_empty() {
        println!("Instance      : {}", inst.root.display());
        println!("Mount target  : {}  (the game's Data dir)", game.data_path.display());
        println!("Mod layers ({}):", layers.len());
        for (i, l) in layers.iter().enumerate() {
            println!("  {}. {}", i + 1, l.file_name().unwrap_or_default().to_string_lossy());
        }
        if layers.is_empty() {
            println!("  (none yet - drop mods into {})", inst.mods_dir().display());
        }
        println!("\nTo launch the game through Eidos, set this Steam launch option:");
        println!("    eidos play {id} -- %command%");
        println!("\nOr run any command through the view now, e.g.:");
        println!("    eidos play {id} -- ls \"{}\"", game.data_path.display());
        return;
    }

    prepare_plugins(id, &game, &inst);

    let spec = LaunchSpec {
        layers,
        overwrite: inst.overwrite_dir(),
        mountpoint: game.data_path.clone(),
        command,
        env: Vec::new(),
        base_bind: Some((game.data_path.clone(), inst.base_dir())),
    };
    match launch(spec) {
        Ok(status) => exit(status.code().unwrap_or(0)),
        Err(e) => {
            eprintln!("eidos play: {e}");
            exit(1)
        }
    }
}

fn cmd_install(args: &[String]) {
    let (Some(id), Some(archive)) = (args.first(), args.get(1)) else {
        eprintln!("usage: eidos install <game-id> <archive> [name]");
        exit(2);
    };
    let Some(game) = find_game(id) else {
        eprintln!("Game '{id}' is not detected. Run `eidos games`.");
        exit(1);
    };
    let inst = Instance::global(id);
    inst.create().ok();
    let _ = inst.ensure_manifest(id, InstanceKind::Global);

    let name = args.get(2).cloned().unwrap_or_else(|| guess_mod_name(archive));
    match eidos_install::install_archive(std::path::Path::new(archive), &inst.mods_dir(), &name, id) {
        Ok(r) => {
            // Activate the new mod at the top of the active profile's load order,
            // like MO2 (a freshly installed mod wins conflicts by default).
            let mut ml = inst.modlist();
            ml.retain(|m| m.name != r.name);
            ml.insert(0, ModEntry { name: r.name.clone(), enabled: true, path: r.dest.clone() });
            let _ = inst.save_modlist(&ml);

            print!("Installed '{}' for {}", r.name, game.def.name);
            if !r.stripped.is_empty() {
                print!(" (stripped wrapper '{}')", r.stripped.trim_end_matches('/'));
            }
            println!();
            println!("  -> {}", r.dest.display());
            println!("  enabled at the top of the load order. `eidos play {id}` to use it.");
        }
        Err(e) => {
            eprintln!("install failed: {e}");
            exit(1);
        }
    }
}

/// Guess a clean mod name from a (possibly Nexus-suffixed) archive filename, e.g.
/// `Foo - Bar-19181-1-7-1575746557.7z` -> `Foo - Bar`.
fn guess_mod_name(archive: &str) -> String {
    let stem = std::path::Path::new(archive).file_stem().and_then(|s| s.to_str()).unwrap_or("Mod");
    // Drop the trailing Nexus "-<modid>-<version parts>-<timestamp>" (all-digit groups).
    let mut parts: Vec<&str> = stem.split('-').collect();
    while parts.len() > 1
        && parts.last().is_some_and(|p| {
            let t = p.trim();
            !t.is_empty() && t.chars().all(|c| c.is_ascii_digit())
        })
    {
        parts.pop();
    }
    let name = parts.join("-");
    let name = name.trim().trim_end_matches('-').trim();
    if name.is_empty() {
        stem.to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod install_tests {
    use super::guess_mod_name;

    #[test]
    fn strips_nexus_suffix() {
        assert_eq!(
            guess_mod_name("/dl/Expressive Facial Animation - Female Edition-19181-1-7-1575746557.7z"),
            "Expressive Facial Animation - Female Edition"
        );
        assert_eq!(guess_mod_name("TrueHUD-62775-1-1-9-1703382929.7z"), "TrueHUD");
        assert_eq!(guess_mod_name("SkyUI_5_1.7z"), "SkyUI_5_1");
    }
}

fn usage() -> ! {
    eprintln!(
        "eidos - a native Linux mod manager\n\
         \n\
         usage:\n\
         \x20 eidos games                       list supported games installed here\n\
         \x20 eidos init <game-id>              create a modding instance\n\
         \x20 eidos play <game-id>              show what would be mounted\n\
         \x20 eidos play <game-id> -- <cmd...>  run <cmd> with mods mounted over the game\n\
         \x20 eidos install <id> <archive>      install a downloaded mod archive (.7z/.zip/.rar)"
    );
    exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("games") => cmd_games(),
        Some("init") => match args.get(1) {
            Some(id) => cmd_init(id),
            None => usage(),
        },
        Some("play") => cmd_play(&args[1..]),
        Some("install") => cmd_install(&args[1..]),
        _ => usage(),
    }
}
