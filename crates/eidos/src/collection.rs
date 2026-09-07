//! `eidos collection`: install a Nexus collection the way its author built it.
//!
//!   eidos collection <instance> <nxm-link-or-slug> [--dry-run] [--no-optional]
//!
//! The chain, in the order the order matters: fetch the revision (for its
//! download link), fetch and extract the ARCHIVE, read `collection.json` - which
//! is the only place the recipe exists - then install the members by phase,
//! recording each one before moving on, and only THEN apply the three things
//! that are about the collection as a whole: the deployment order, the plugin
//! rules, and the INI tweaks. Once, at the end, rather than after every member:
//! under a union mount there is no per-mod barrier to wait for.


use std::process::exit;

use eidos_collections::driver;
use eidos_collections::manifest::{Collection, Mod, SourceType};
use eidos_collections::state::{InstallState, Status};

use crate::*;

fn usage() -> ! {
    eidos_log::info!(
        "usage: eidos collection <instance> <nxm-link or slug> [options]\n\
         \x20 --dry-run      read the collection and say what would happen\n\
         \x20 --no-optional  skip the members the collection marks optional\n\
         \n\
         The link is the one the site's \"Add to Eidos\" button produces, or just\n\
         the collection's slug."
    );
    exit(2);
}

/// `eidos collection <instance> <link>`.
pub(crate) fn cmd_collection(args: &[String]) {
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if let Some(bad) = args
        .iter()
        .find(|a| a.starts_with('-') && *a != "--dry-run" && *a != "--no-optional")
    {
        eidos_log::info!("eidos collection: unknown option '{bad}'");
        usage();
    }
    let (Some(id), Some(link)) = (positional.first(), positional.get(1)) else {
        usage();
    };
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let no_optional = args.iter().any(|a| a == "--no-optional");

    let target = resolve(id);
    let Some(game) = find_game(&target.game_id) else {
        eidos_log::info!(
            "Game '{}' is not detected. Run `eidos games`.",
            target.game_id
        );
        exit(1);
    };
    let inst = target.inst;
    if !inst.exists() {
        eidos_log::info!(
            "eidos collection: '{}' is not an instance folder.",
            inst.root.display()
        );
        exit(1);
    }

    let nexus = nexus_client();
    let want = match eidos_nexus::NxmLink::parse(link) {
        Ok(eidos_nexus::NxmLink::Collection(c)) => c,
        // A bare slug is what somebody reads off the page, so accept it.
        _ if !link.contains("://") => eidos_nexus::NxmCollection {
            game: game.def.nexus_game.to_string(),
            slug: (*link).clone(),
            revision: None,
        },
        Ok(_) => {
            eidos_log::info!("That is a mod link, not a collection link.");
            exit(1);
        }
        Err(e) => {
            eidos_log::info!("eidos collection: {e}");
            exit(1);
        }
    };

    let rev = match nexus.collection_revision(&want) {
        Ok(r) => r,
        Err(e) => {
            eidos_log::warn!("eidos collection: {e}");
            exit(1);
        }
    };
    if !rev.visible() {
        eidos_log::info!(
            "This collection's details are withheld by your account's content settings."
        );
        exit(1);
    }
    println!("{}  ·  revision {}", rev.name, rev.revision_number);
    if !rev.author.is_empty() {
        println!("by {}", rev.author);
    }

    // The archive. Everything that makes this a recipe rather than a list is
    // inside it, and no API call returns any of it.
    let dir = eidos_collections::state::revision_dir(&inst.root, &rev.slug, rev.revision_number);
    let manifest = match driver::fetch_manifest(&nexus, &rev, &dir) {
        Ok(m) => m,
        Err(e) => {
            eidos_log::warn!("eidos collection: {e}");
            exit(1);
        }
    };
    let read = match eidos_collections::read(&manifest) {
        Ok(r) => r,
        Err(e) => {
            eidos_log::warn!("eidos collection: {e}");
            exit(1);
        }
    };
    let c = &read.collection;
    if !c.info.install_instructions.is_empty() {
        println!("\n--- the author's notes ---\n{}\n", c.info.install_instructions);
    }

    let state_path = InstallState::path(&inst.root, &rev.slug, rev.revision_number);
    let mut state = match InstallState::load(&state_path) {
        Ok(Some(s)) => s,
        Ok(None) => InstallState {
            slug: rev.slug.clone(),
            revision: rev.revision_number,
            game_domain: rev.game_domain.clone(),
            ..InstallState::default()
        },
        // Not a first run. Starting over means downloading everything again and
        // wipe-reinstalling every member, so it is the user's call, not a
        // default.
        Err(e) => {
            eidos_log::warn!("eidos collection: {e}");
            exit(1);
        }
    };
    if no_optional {
        for m in c.mods.iter().filter(|m| m.optional) {
            let key = eidos_collections::state::key_for(m, &c.info.domain_name);
            // Only what is not already on disk. A skip is the user's word and
            // nothing automatic can undo it, so writing it over a member that is
            // installed would drop that mod out of the deployment order for
            // good, with no way back.
            if !matches!(
                state.status(&key),
                Status::Installed(_) | Status::Approximate(_, _)
            ) {
                state.set_by_user(&key, Status::Skipped);
            }
        }
    }

    println!(
        "{} member(s), {} phase(s){}",
        c.mods.len(),
        c.phases().len(),
        if c.mod_rules.is_empty() {
            String::new()
        } else {
            format!(", {} ordering rule(s)", c.mod_rules.len())
        }
    );
    if dry_run {
        preview(c, &state);
        println!("\n(dry run - nothing installed; drop --dry-run to do it)");
        return;
    }

    let _lock = match inst.try_lock("eidos collection") {
        Ok(l) => l,
        Err(e) => {
            eidos_log::warn!("Cannot install now: {e}.");
            exit(1);
        }
    };

    let mut say = |line: String| println!("  {line}");
    let mut hooks = driver::RealHooks {
        nexus: &nexus,
        inst: &inst,
        game: &game,
        game_id: target.game_id.clone(),
        say: &mut say,
        collection_domain: c.info.domain_name.clone(),
        known_members: state.members.keys().cloned().collect(),
        renamed: Vec::new(),
        installed_now: Vec::new(),
    };
    let mut said_save_failed = false;
    let mut save = |s: &InstallState| {
        if let Err(e) = s.save(&state_path) {
            // Once. A failing disk would otherwise print this twice per member.
            if !said_save_failed {
                said_save_failed = true;
                eidos_log::warn!(
                    "Could not record what has been installed ({e}). If this run is \
                     interrupted it will start over."
                );
            }
        }
    };
    let mut report = eidos_collections::install::run(c, &mut state, &mut hooks, &mut save);
    report.unknown_sections = read.unknown_sections.clone();
    for (member, folder) in std::mem::take(&mut hooks.renamed) {
        report.renamed.push(eidos_collections::report::Note {
            subject: member,
            detail: format!("installed as \"{folder}\", because a mod of yours already had that name"),
        });
    }

    driver::apply_ordering(&inst, c, &state, &mut report);
    driver::apply_plugin_states(&inst, &game, c, &mut report);
    driver::apply_plugin_rules(&inst, &game, c, &mut report);
    driver::apply_ini_tweaks(&inst, &dir, c, &mut report);

    println!("\n{}", report.render());
    if !report.is_complete() {
        println!(
            "Run this again once you have what is missing; it picks up where it left off."
        );
        exit(1);
    }
}

/// What a `--dry-run` shows: the plan, not a promise.
fn preview(c: &Collection, state: &InstallState) {
    let order = eidos_collections::install::install_order(c);
    println!("\nWould install, in this order:");
    for (n, &i) in order.iter().enumerate() {
        let m = &c.mods[i];
        let key = eidos_collections::state::key_for(m, &c.info.domain_name);
        let st = state.status(&key);
        println!(
            "  {:>3}. [phase {}] {}{}{}",
            n + 1,
            m.phase,
            m.name,
            if m.optional { "  (optional)" } else { "" },
            match st {
                Status::Pending => String::new(),
                other => format!("  - already {}", other.word()),
            }
        );
    }
    let non_nexus: Vec<&Mod> = c
        .mods
        .iter()
        .filter(|m| m.source.kind != SourceType::Nexus)
        .collect();
    if !non_nexus.is_empty() {
        println!("\n{} member(s) do not come from a Nexus mod page:", non_nexus.len());
        for m in non_nexus {
            println!("  {}  ({:?})", m.name, m.source.kind);
        }
    }
    if !c.tools.is_empty() {
        println!("\nTools it expects you to already have:");
        for t in &c.tools {
            println!("  {}  {}", t.name, t.exe);
        }
    }
}

