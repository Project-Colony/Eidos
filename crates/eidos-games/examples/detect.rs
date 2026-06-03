//! Print the Steam libraries, scanned apps, and supported games found on this
//! machine. This is the headless version of the "pick your game" screen Eidos
//! will show on first launch.
//!
//!   cargo run --example detect -p eidos-games

use eidos_games::{catalog, detect, home, scan_installed, steam_libraries};

fn main() {
    let home = home();
    println!("HOME: {}\n", home.display());

    let libs = steam_libraries(&home);
    println!("Steam libraries ({}):", libs.len());
    for lib in &libs {
        println!("  {}", lib.display());
    }

    let installed = scan_installed(&home);
    println!("\nInstalled Steam apps scanned: {}", installed.len());

    let games = detect(&home);
    println!("\nSupported games installed: {} / {} in catalog\n", games.len(), catalog().len());
    if games.is_empty() {
        println!("  (none of the catalogued games are installed here)");
    }
    for g in &games {
        println!("  [{}] {}  (Steam: {})", g.def.id, g.def.name, g.steam_name);
        println!("      install : {}", g.install_path.display());
        println!("      data    : {}", g.data_path.display());
        match &g.compatdata {
            Some(p) => println!("      proton  : {}", p.display()),
            None => println!("      proton  : (no prefix yet)"),
        }
    }
}
