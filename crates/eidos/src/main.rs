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

use eidos_instance::Instance;

mod export;
mod games;
mod install;
mod launch;
mod nxm;
mod prepare;
mod prereqs;
mod sort;
mod tools;
#[cfg(test)]
mod tests;

use export::*;
use games::*;
use install::*;
use launch::*;
use nxm::*;
use prepare::*;
use prereqs::*;
use sort::*;
use tools::*;

/// `~/.config/eidos/nexus.ini`, holding the personal Nexus API key. Delegates to
/// the shared `eidos-instance` settings store so the CLI and the GUI can never
/// disagree on the path or the file format.
/// A connected Nexus client, or exit with a pointer to signing in.
fn nexus_client() -> eidos_nexus::Nexus {
    match eidos_nexus::Nexus::connect() {
        Ok(nexus) => nexus,
        Err(_) => {
            eprintln!(
                "Not signed in to Nexus. Sign in from the GUI (Settings -> Nexus); \
                 personal API keys are not supported."
            );
            exit(1);
        }
    }
}

/// `eidos nexus key|status|update` - account + update checks.
fn cmd_nexus(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("status") => {
            let nexus = nexus_client();
            match nexus.validate() {
                Ok(acct) => {
                    println!(
                        "Connected as {} ({}).",
                        acct.name,
                        if acct.is_premium { "premium" } else { "free" }
                    );
                    // Say it out loud. When adult metadata is being withheld the
                    // user needs to know it is a setting and where to change it,
                    // not wonder why a mod page came back blank.
                    println!(
                        "Adult content: {}",
                        match nexus.adult_policy() {
                            eidos_nexus::AdultPolicy::Allowed => "shown (enabled on your account)",
                            eidos_nexus::AdultPolicy::Denied =>
                                "hidden (turned off on your account, at nexusmods.com)",
                            eidos_nexus::AdultPolicy::Unknown =>
                                "hidden (Eidos could not read your account setting)",
                        }
                    );
                }
                Err(e) => {
                    eprintln!("not connected: {e}");
                    exit(1);
                }
            }
        }
        Some("update") => {
            let Some(id) = args.get(1) else {
                eprintln!("usage: eidos nexus update <game-id>");
                exit(2);
            };
            let Some(game) = find_game(id) else {
                eprintln!("Game '{id}' is not detected. Run `eidos games`.");
                exit(1);
            };
            let inst = Instance::global(id);
            let nexus = nexus_client();

            // MO2's approach: one "updated this month" query, then only fetch
            // the mods in the intersection (stays inside the API rate limits).
            let updated = match nexus.updated_mod_ids(game.def.nexus_game, "1m") {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("update query failed: {e}");
                    exit(1);
                }
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            const MONTH: u64 = 30 * 24 * 3600;

            let mut checked = 0u32;
            let mut updates = 0u32;
            let mut individual = 0u32;
            let mut rate_limited = false;
            for m in inst.modlist() {
                let mut meta = inst.mod_meta(&m.name);
                let Some(mod_id) = meta.mod_id() else { continue };
                checked += 1;
                // MO2: the `updated?period=1m` list is only trustworthy for mods
                // checked within that window. A mod never checked - or checked over
                // a month ago - gets an individual query regardless of the
                // intersection, else an update published >1 month ago is missed
                // forever (the common first-run case on an established mod list).
                let stale = meta
                    .last_nexus_update()
                    .map(|t| now.saturating_sub(t) > MONTH)
                    .unwrap_or(true);
                if !stale && !updated.contains(&mod_id) {
                    continue;
                }
                if stale {
                    individual += 1;
                }
                // Stop before the request is built, not after Nexus refuses it.
                if nexus.would_block() {
                    rate_limited = true;
                    eprintln!("  Nexus request budget spent - stopping; remaining mods unchecked.");
                    break;
                }
                match nexus.mod_info(game.def.nexus_game, mod_id) {
                    Ok(remote) => {
                        meta.set_newest_version(&remote.version);
                        meta.set_last_nexus_update(now);
                        let _ = meta.write(&inst.mods_dir().join(&m.name).join("meta.ini"));
                        if meta.update_available() {
                            updates += 1;
                            println!(
                                "  UPDATE {:<40} {} -> {}",
                                m.name,
                                meta.version().unwrap_or_default(),
                                remote.version
                            );
                        }
                    }
                    Err(e) => {
                        // The shared predicate, not a bare "429". This loop used
                        // to test for the status code while the library tested
                        // for the wording, so a pre-flight refusal - which has no
                        // status code - stopped one and left this one hammering.
                        if eidos_nexus::is_rate_limited(&e) {
                            rate_limited = true;
                            eprintln!("  {e} - stopping; remaining mods unchecked.");
                            break;
                        }
                        eprintln!("  {}: {e}", m.name);
                    }
                }
            }
            println!(
                "{updates} update(s) available ({checked} mod(s) with a Nexus id; \
                 {individual} queried individually; {} recently updated on Nexus).",
                updated.len()
            );
            let rl = nexus.rate_limits();
            if let Some(h) = rl.hourly_remaining {
                let daily = rl.daily_remaining.map(|d| format!(", {d} today")).unwrap_or_default();
                println!("Nexus budget: {h} request(s) left this hour{daily}.");
            }
            if rate_limited {
                eprintln!("Some mods were not checked (request budget spent). Re-run once it refills.");
            }
        }
        _ => {
            eprintln!(
                "usage:\n\
                 \x20 eidos nexus status          check the stored sign-in\n\
                 \x20 eidos nexus update <game>   check installed mods for updates"
            );
            exit(2);
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
         \x20 eidos play <game-id> -- <cmd...>  run <cmd> with mods mounted over the game\n\
         \x20 eidos install <id> <archive>      install a downloaded mod archive (.7z/.zip/.rar)\n\
         \x20 eidos tool <id> [...]             manage + run tools (xEdit/FNIS/...) through the view\n\
         \x20 eidos nexus status|update         check the Nexus sign-in / check for mod updates\n\
         \x20 eidos nxm <url> | --register      download a Nexus Mod Manager link / register the handler\n\
         \x20 eidos export <id> [-o file]       export the mod list to CSV (MO2 format; --active = enabled only)\n\
         \x20 eidos sort <id> [--dry-run]       LOOT-sort the plugin load order (--update-masterlist to refresh)\n\
         \x20 eidos import <id> <mo2-profile>   take over an MO2 profile's mod order + plugin state"
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
        Some("tool") => cmd_tool(&args[1..]),
        Some("prereqs") => cmd_prereqs(&args[1..]),
        Some("export") => cmd_export(&args[1..]),
        Some("sort") => cmd_sort(&args[1..]),
        Some("nexus") => cmd_nexus(&args[1..]),
        Some("nxm") => cmd_nxm(&args[1..]),
        Some("import") => cmd_import(&args[1..]),
        _ => usage(),
    }
}
