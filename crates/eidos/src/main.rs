//! `eidos`: the front end that ties detection, instances, and launching together.
//!
//!   eidos games                       list supported games installed on this system
//!   eidos init <game-id>              create a modding instance for a detected game
//!   eidos play <game-id>              show how to launch / what is mounted
//!   eidos play <game-id> -- <cmd...>  run <cmd> with the mods mounted over the game
//!
//! An instance lives at `$XDG_DATA_HOME/eidos/<game-id>/`:
//!   mods/<name>/...   one folder per mod (load order: alphabetical, or a
//!                     `load_order.txt` listing folder names, top = highest)
//!   overwrite/        the writable layer (saves, regenerated configs)
//!
//! `play` mounts the mods over the game's own Data directory (via a bind-stash,
//! so the daemon still reads the pristine files), then runs the command inside a
//! private namespace. The eventual Steam launch option is `eidos play <id> -- %command%`.

use std::fs;
use std::path::PathBuf;
use std::process::exit;

use eidos_games::{detect, home, DetectedGame};
use eidos_launch::{launch, LaunchSpec};

fn data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home().join(".local/share"))
}

fn instance_dir(id: &str) -> PathBuf {
    data_home().join("eidos").join(id)
}

fn find_game(id: &str) -> Option<DetectedGame> {
    detect(&home()).into_iter().find(|g| g.def.id == id)
}

/// Mod layers for an instance, highest priority first. Honours a
/// `load_order.txt` (top line wins); otherwise alphabetical.
fn load_order(id: &str) -> Vec<PathBuf> {
    let inst = instance_dir(id);
    let mut dirs: Vec<PathBuf> = fs::read_dir(inst.join("mods"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    match fs::read_to_string(inst.join("load_order.txt")) {
        Ok(content) => {
            let order: Vec<&str> = content.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
            dirs.sort_by_key(|p| {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                order.iter().position(|o| *o == name).unwrap_or(usize::MAX)
            });
        }
        Err(_) => dirs.sort(),
    }
    dirs
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
    let inst = instance_dir(id);
    let mods = inst.join("mods");
    fs::create_dir_all(&mods).expect("create mods dir");
    fs::create_dir_all(inst.join("overwrite")).expect("create overwrite dir");
    let _ = fs::write(
        mods.join("README.txt"),
        "Drop each mod here as its own folder.\n\
         Load order is alphabetical unless a ../load_order.txt lists folder\n\
         names (top line = highest priority, wins file conflicts).\n",
    );
    println!("Created instance for {} ({id}).", game.def.name);
    println!("  instance : {}", inst.display());
    println!("  game data: {}", game.data_path.display());
    println!("  add mods : {}", mods.display());
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

    let inst = instance_dir(id);
    fs::create_dir_all(inst.join("mods")).ok();
    fs::create_dir_all(inst.join("overwrite")).ok();
    let layers = load_order(id);

    if command.is_empty() {
        println!("Instance      : {}", inst.display());
        println!("Mount target  : {}  (the game's Data dir)", game.data_path.display());
        println!("Mod layers ({}):", layers.len());
        for (i, l) in layers.iter().enumerate() {
            println!("  {}. {}", i + 1, l.file_name().unwrap_or_default().to_string_lossy());
        }
        if layers.is_empty() {
            println!("  (none yet - drop mods into {})", inst.join("mods").display());
        }
        println!("\nTo launch the game through Eidos, set this Steam launch option:");
        println!("    eidos play {id} -- %command%");
        println!("\nOr run any command through the view now, e.g.:");
        println!("    eidos play {id} -- ls \"{}\"", game.data_path.display());
        return;
    }

    let spec = LaunchSpec {
        layers,
        overwrite: inst.join("overwrite"),
        mountpoint: game.data_path.clone(),
        command,
        base_bind: Some((game.data_path.clone(), inst.join(".base"))),
    };
    match launch(spec) {
        Ok(status) => exit(status.code().unwrap_or(0)),
        Err(e) => {
            eprintln!("eidos play: {e}");
            exit(1)
        }
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
         \x20 eidos play <game-id> -- <cmd...>  run <cmd> with mods mounted over the game"
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
        _ => usage(),
    }
}
