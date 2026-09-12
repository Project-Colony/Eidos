//! `eidos`: the front end that ties detection, instances, and launching together.
//!
//!   eidos games                       list supported games installed on this system
//!   eidos init <game-id> [folder]     create a modding instance (global, or portable at <folder>)
//!   eidos play <instance>             show how to launch / what is mounted
//!   eidos play <instance> -- <cmd...> run <cmd> with the mods mounted over the game
//!
//! `<instance>` is a game id (the central instance) or a portable instance's
//! folder - see `resolve.rs` for how the two are told apart.
//! Instances (global vs portable, layout, load order) live in `eidos-instance`.
//! `play` mounts the instance's mods over the game's own Data directory (via a
//! bind-stash) inside a private namespace, then runs the command through it.

use std::process::exit;

mod collection;
mod export;
mod games;
mod install;
mod launch;
mod nxm;
mod prepare;
mod prereqs;
mod resolve;
mod sort;
#[cfg(test)]
mod tests;
mod tools;
mod transfer;

use collection::*;
use export::*;
use games::*;
use install::*;
use launch::*;
use nxm::*;
use prepare::*;
use prereqs::*;
use resolve::*;
use sort::*;
use tools::*;
use transfer::*;

/// `~/.config/Colony/Eidos/nexus.ini`, holding the personal Nexus API key. Delegates to
/// the shared `eidos-instance` settings store so the CLI and the GUI can never
/// disagree on the path or the file format.
/// A connected Nexus client, or exit with a pointer to signing in.
fn nexus_client() -> eidos_nexus::Nexus {
    match eidos_nexus::Nexus::connect() {
        Ok(nexus) => nexus,
        // The message matters: `connect` refuses for two quite different
        // reasons, and offline mode is the user's own setting. Telling somebody
        // who turned it on that they are "not signed in" sends them to sign in
        // again, which will also refuse, with the same wrong explanation.
        Err(e) if e == eidos_nexus::OFFLINE_MESSAGE => {
            eidos_log::info!("{e}");
            exit(1);
        }
        Err(e) => {
            // The reason, not a guess at it. "Not signed in" was printed for
            // every failure, including a stored session that was rejected for
            // some quite different cause - which is exactly the kind of wrong
            // hint that sends somebody round a loop of signing in again.
            eidos_log::info!(
                "Not connected to Nexus: {e}\nSign in from the GUI (Settings -> Nexus); \
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
            match nexus.account() {
                Ok(acct) => {
                    println!(
                        "Connected as {} ({}).",
                        acct.name,
                        if acct.is_premium { "premium" } else { "free" }
                    );
                    // Say it out loud. When adult metadata is being withheld the
                    // user needs to know it is a setting and where to change it,
                    // not wonder why a mod page came back blank.
                    // Said only when it is NOT the good case, so the ordinary
                    // run stays quiet: Nexus signs access tokens with a key it
                    // publishes nowhere, so there is nothing to check against.
                    if !acct.verified {
                        println!(
                            "  (the session's signature could not be checked: Nexus does not \
                             publish the key it signs access tokens with)"
                        );
                    }
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
                    eidos_log::info!("not connected: {e}");
                    exit(1);
                }
            }
        }
        Some("update") => {
            let Some(id) = args.get(1) else {
                eidos_log::info!("usage: eidos nexus update <game-id-or-instance-path>");
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
            let nexus = nexus_client();

            let result = match eidos_nexus::check_updates(&nexus, &inst, game.def.nexus_game) {
                Ok(result) => result,
                Err(error) => {
                    eidos_log::info!("Update check failed: {error}");
                    exit(1);
                }
            };
            for update in &result.updates {
                println!(
                    "  UPDATE {:<40} {} -> {}",
                    update.name, update.installed, update.latest
                );
            }
            for (name, error) in &result.failures {
                eidos_log::warn!("Update not checked for {name}: {error}");
            }
            for name in &result.unavailable {
                println!("  UNAVAILABLE {name}");
            }
            println!(
                "{} update(s) available ({} mod(s) with a Nexus id; {} queried).",
                result.updates_found, result.checked, result.queried
            );
            if let Some(hourly) = result.hourly_remaining {
                let daily = result
                    .daily_remaining
                    .map(|count| format!(", {count} today"))
                    .unwrap_or_default();
                println!("Nexus budget: {hourly} request(s) left this hour{daily}.");
            }
            if result.rate_limited {
                eidos_log::info!(
                    "Some mods were not checked (request budget spent). Re-run once it refills."
                );
            }
        }
        _ => {
            eidos_log::info!(
                "usage:\n\
                 \x20 eidos nexus status          check the stored sign-in\n\
                 \x20 eidos nexus update <game>   check installed mods for updates"
            );
            exit(2);
        }
    }
}

fn usage() -> ! {
    eidos_log::info!(
        "eidos - a native Linux mod manager\n\
         \n\
         usage:\n\
         \x20 eidos games                       list supported games installed here\n\
         \x20 eidos init <game-id> [folder]     create a modding instance (with a folder: portable, there)\n\
         \x20 eidos play <instance>             show what would be mounted\n\
         \x20 eidos play <instance> -- <cmd...> run <cmd> with mods mounted over the game\n\
         \x20 eidos install <instance> <archive> install a downloaded mod archive (.7z/.zip/.rar)\n\
         \x20 eidos collection <instance> <link>  install a Nexus collection (--dry-run to look first)\n\
         \x20 eidos pack <instance> <file.eidos> write the whole instance into one file\n\
         \x20 eidos unpack <file.eidos> [folder] put a packed instance back (--info to look first)\n\
         \x20 eidos tool <instance> [...]       manage + run tools (xEdit/FNIS/...) through the view\n\
         \x20 eidos nexus status|update         check the Nexus sign-in / check for mod updates\n\
         \x20 eidos nxm <url> | --register      download a Nexus Mod Manager link / register the handler\n\
         \x20 eidos export <instance> [-o file] export the mod list to CSV (MO2 format; --active = enabled only)\n\
         \x20 eidos sort <instance> [--dry-run] LOOT-sort the plugin load order (--update-masterlist to refresh)\n\
         \x20 eidos import <instance> <mo2-profile> take over an MO2 profile's mod order + plugin state\n\
         \n\
         <instance> is a game id (skyrimse - the central instance) or the path of a\n\
         portable instance folder. EIDOS_INSTANCE=<folder> redirects a game id there."
    );
    exit(2);
}

/// Which rotation bucket a run's session log belongs to.
///
/// The subcommand's own first argument is usually the instance, and it makes a
/// better bucket than the verb; falling back to the verb keeps every run in a
/// named file rather than one shared one.
///
/// A URL is one exception, and an archive path is the other. The URL case
/// mattered: `eidos nxm <link>` bucketed by the
/// LINK, so every mod ever downloaded got a bucket of its own and the
/// ten-per-bucket retention never pruned anything. One collection's "fetch
/// missing" alone left a file per member, each named after the ids it fetched.
fn log_bucket(args: &[String]) -> &str {
    // `unpack` is the other verb whose first argument is not an instance: it is
    // an ARCHIVE PATH, and bucketing by it would give every backup file a log of
    // its own, so the ten-per-bucket retention would never prune - the same
    // defect the nxm test below exists for.
    if args.first().is_some_and(|v| v == "unpack") {
        return "unpack";
    }
    args.get(1)
        .filter(|a| !a.starts_with('-') && !a.contains("://"))
        .or_else(|| args.first())
        .map(String::as_str)
        .unwrap_or("eidos")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Open the session log before anything else runs.
    //
    // A launch started from Steam has no terminal at all, so stderr goes
    // nowhere and the file is the ONLY record of what happened - which is also
    // what the GUI's Log pane reads.
    let bucket = log_bucket(&args);
    let _ = eidos_log::init_with(
        eidos_log::Config::new(bucket).with_version(env!("CARGO_PKG_VERSION")),
    );
    // Onto the ecosystem's layout - `~/.config/Colony/Eidos` - before anything
    // reads a setting. Copies rather than moves, runs once, and cannot fail a
    // launch: see `eidos_paths::migrate_legacy_layout`. Logged rather than
    // silent, because a user who goes looking for their settings deserves to
    // find out from the log where they went.
    for note in eidos_paths::migrate_legacy_layout() {
        eidos_log::info!("{note}");
    }
    match args.first().map(String::as_str) {
        Some("games") => cmd_games(),
        Some("init") => match args.get(1) {
            Some(id) => {
                let mut folder = None;
                let mut game_path = None;
                let mut rest = args.iter().skip(2);
                while let Some(arg) = rest.next() {
                    if arg == "--game-path" {
                        game_path = rest.next().map(String::as_str);
                        if game_path.is_none() { eidos_log::info!("--game-path needs a folder"); exit(2); }
                    } else if folder.is_none() && !arg.starts_with("--") { folder = Some(arg.as_str()); }
                    else { eidos_log::info!("Unexpected init argument: {arg}"); exit(2); }
                }
                cmd_init(id, folder, game_path)
            },
            None => usage(),
        },
        Some("play") => cmd_play(&args[1..]),
        Some("install") => cmd_install(&args[1..]),
        Some("collection") => cmd_collection(&args[1..]),
        Some("pack") => cmd_pack(&args[1..]),
        Some("unpack") => cmd_unpack(&args[1..]),
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

#[cfg(test)]
mod bucket_tests {
    use super::log_bucket;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_nxm_link_is_never_its_own_rotation_bucket() {
        // The defect this exists for: bucketing by the link gave every mod its
        // own bucket, so the ten-per-bucket retention never pruned anything and
        // one collection fetch left a log file per member - each named after the
        // mod and file ids it went after.
        let a = v(&["nxm", "nxm://skyrimspecialedition/mods/36350/files/213426"]);
        assert_eq!(log_bucket(&a), "nxm");
        let a = v(&[
            "nxm",
            "nxm://skyrimspecialedition/collections/rqhcxy/revisions/latest",
        ]);
        assert_eq!(log_bucket(&a), "nxm");
    }

    #[test]
    fn a_backup_file_is_never_its_own_rotation_bucket_either() {
        // `unpack` takes an ARCHIVE first, not an instance. Bucketing by it gives
        // every backup file a log of its own, which is the nxm defect above with
        // a different first argument.
        let a = v(&["unpack", "/mnt/Jeux/skyrimse-2026-09-07-1340.eidos", "/mnt/x"]);
        assert_eq!(log_bucket(&a), "unpack");
        // `pack` is the other way round: its first argument IS the instance, and
        // one log per instance is exactly what the retention wants.
        assert_eq!(
            log_bucket(&v(&["pack", "/mnt/Jeux/Eidos-Skyrim", "/tmp/b.eidos"])),
            "/mnt/Jeux/Eidos-Skyrim"
        );
    }

    #[test]
    fn an_instance_argument_still_buckets_by_instance() {
        assert_eq!(log_bucket(&v(&["play", "skyrimse"])), "skyrimse");
        assert_eq!(
            log_bucket(&v(&["play", "/mnt/Jeux/Eidos-Skyrim"])),
            "/mnt/Jeux/Eidos-Skyrim"
        );
        // A flag is not an instance, and neither is nothing at all.
        assert_eq!(log_bucket(&v(&["games", "--json"])), "games");
        assert_eq!(log_bucket(&v(&["games"])), "games");
        assert_eq!(log_bucket(&[]), "eidos");
    }
}
