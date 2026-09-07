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

use std::path::{Path, PathBuf};
use std::process::exit;

use eidos_collections::install::{Hooks, Installed, Obtained};
use eidos_collections::manifest::{Collection, Mod, SourceType};
use eidos_collections::report::Note;
use eidos_collections::state::{InstallState, Status};
use eidos_instance::Instance;

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
    let dir = inst
        .root
        .join("collections")
        .join(format!("{}-{}", rev.slug, rev.revision_number));
    let manifest = match fetch_manifest(&nexus, &rev, &dir) {
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
    let mut state = std::fs::read_to_string(&state_path)
        .ok()
        .and_then(|t| InstallState::from_json(&t))
        .unwrap_or(InstallState {
            slug: rev.slug.clone(),
            revision: rev.revision_number,
            game_domain: rev.game_domain.clone(),
            ..InstallState::default()
        });
    if no_optional {
        for m in c.mods.iter().filter(|m| m.optional) {
            let key = eidos_collections::state::member_key(
                m.source.file_id,
                m.source.mod_id,
                &m.name,
            );
            state.set_by_user(&key, Status::Skipped);
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

    let mut hooks = CliHooks {
        nexus: &nexus,
        inst: &inst,
        game: &game,
        game_id: target.game_id.clone(),
    };
    let mut save = |s: &InstallState| {
        if let Some(d) = state_path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(&state_path, s.to_json());
    };
    let mut report = eidos_collections::install::run(c, &mut state, &mut hooks, &mut save);
    report.unknown_sections = read.unknown_sections.clone();

    apply_ordering(&inst, c, &state, &mut report);
    apply_plugin_rules(&inst, &game, c, &mut report);
    apply_ini_tweaks(&inst, &dir, c, &mut report);

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
        let key = eidos_collections::state::member_key(m.source.file_id, m.source.mod_id, &m.name);
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

/// Download the collection archive if it is not already here, and hand back its
/// `collection.json`.
fn fetch_manifest(
    nexus: &eidos_nexus::Nexus,
    rev: &eidos_nexus::collections::CollectionRevision,
    dir: &Path,
) -> Result<String, String> {
    let manifest = dir.join("collection.json");
    if manifest.is_file() {
        return std::fs::read_to_string(&manifest).map_err(|e| e.to_string());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let url = nexus.collection_archive_url(&rev.download_link)?;
    let archive = dir.join("collection.7z");
    println!("fetching the collection's recipe...");
    nexus.download(&url, &archive)?;
    let bin = eidos_sevenzip::find_7z().ok_or_else(|| {
        "7-Zip is not installed (looked for 7z, 7zz and 7za on PATH)".to_string()
    })?;
    eidos_sevenzip::extract_all_with(bin, &archive, dir, |_| {})
        .map_err(|e| format!("could not open the collection archive: {e}"))?;
    std::fs::read_to_string(&manifest)
        .map_err(|_| "that archive has no collection.json in it".to_string())
}

struct CliHooks<'a> {
    nexus: &'a eidos_nexus::Nexus,
    inst: &'a Instance,
    game: &'a eidos_games::DetectedGame,
    game_id: String,
}

impl CliHooks<'_> {
    /// The archive already in `downloads/` for this member, if it is whole.
    fn already_here(&self, m: &Mod) -> Option<PathBuf> {
        let want = m.source.file_id?;
        let dl = self.inst.downloads_dir();
        std::fs::read_dir(&dl)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "meta"))
            .find_map(|p| {
                (eidos_instance::ModMeta::read(&p).file_id()? == want).then(|| p.with_extension(""))
            })
            .filter(|a| {
                a.is_file() && !PathBuf::from(format!("{}.unfinished", a.display())).exists()
            })
    }
}

impl Hooks for CliHooks<'_> {
    fn obtain(&mut self, m: &Mod) -> Obtained {
        if let Some(p) = self.already_here(m) {
            return Obtained::Ready(p);
        }
        match m.source.kind {
            SourceType::Nexus => {
                let (Some(mod_id), Some(file_id)) = (m.source.mod_id, m.source.file_id) else {
                    return Obtained::Unavailable(
                        "the collection names it as a Nexus file but gives no ids".into(),
                    );
                };
                let domain = if m.domain_name.is_empty() {
                    self.game.def.nexus_game
                } else {
                    &m.domain_name
                };
                // The gate first, as every download path here does: a mod whose
                // page Eidos may not describe is one whose files it does not
                // fetch either. It costs one API call per member and there is no
                // cheaper way to obtain a gate honestly.
                let gate = match self.nexus.mod_info(domain, mod_id) {
                    Ok(remote) => remote.gate,
                    Err(e) => return Obtained::Failed(e),
                };
                let nxm = eidos_nexus::NxmUrl {
                    game: domain.to_string(),
                    mod_id,
                    file_id,
                    key: None,
                    expires: None,
                    user_id: None,
                };
                match self.nexus.download_link(&gate, &nxm) {
                    Ok(url) => {
                        let dest = self
                            .inst
                            .downloads_dir()
                            .join(format!("{}-{mod_id}-{file_id}.archive", safe(&m.name)));
                        match self.nexus.download(&url, &dest) {
                            Ok(_) => Obtained::Ready(dest),
                            Err(e) => Obtained::Failed(e),
                        }
                    }
                    // The free-account wall, and the honest wording for it: this
                    // is a Nexus rule, not something a retry will get past.
                    Err(e) if e.contains("403") || e.to_lowercase().contains("premium") => {
                        Obtained::NeedsUser(format!(
                            "fetch it yourself from https://www.nexusmods.com/{domain}/mods/{mod_id}?tab=files&file_id={file_id} \
                             (a free Nexus account cannot mint download links)"
                        ))
                    }
                    Err(e) => Obtained::Failed(e),
                }
            }
            SourceType::Direct if !m.source.url.is_empty() => {
                let dest = self
                    .inst
                    .downloads_dir()
                    .join(format!("{}.archive", safe(&m.name)));
                match self.nexus.download(&m.source.url, &dest) {
                    Ok(_) => Obtained::Ready(dest),
                    Err(e) => Obtained::Failed(e),
                }
            }
            SourceType::Browse => Obtained::NeedsUser(if m.source.url.is_empty() {
                "the collection says to fetch this one from its page".into()
            } else {
                format!("fetch it yourself from {}", m.source.url)
            }),
            SourceType::Bundle => Obtained::Unavailable(
                "this member travels inside the collection archive, which this version does \
                 not unpack yet"
                    .into(),
            ),
            _ => Obtained::Unavailable("no usable source".into()),
        }
    }

    fn install(&mut self, m: &Mod, archive: &Path) -> Installed {
        let mods_dir = self.inst.mods_dir();
        let ml = self.inst.modlist();
        let enabled: Vec<PathBuf> = ml
            .iter()
            .filter(|x| x.is_active())
            .map(|x| x.path.clone())
            .collect();
        let disabled: Vec<PathBuf> = ml
            .iter()
            .filter(|x| !x.is_active() && !x.is_separator())
            .map(|x| x.path.clone())
            .collect();
        let ctx = eidos_install::fomod_context(&self.game.data_path, &enabled, &disabled);
        let name = safe(&m.name);

        match eidos_install::open_archive_with(archive, &mods_dir, &name, &self.game_id, |_| {}) {
            Ok(eidos_install::Opened::Fomod(session)) => {
                let recorded = recorded_answers(m);
                let r = eidos_fomod::replay(&session.config, &ctx, &recorded);
                let unmatched = r.unmatched.clone();
                match eidos_install::finish_fomod(
                    *session,
                    &r.selection,
                    &mods_dir,
                    &self.game_id,
                    &ctx,
                    eidos_install::OverwritePolicy::Replace,
                ) {
                    Ok(rep) if unmatched.is_empty() => Installed::Ok(rep.name),
                    Ok(rep) => Installed::Approximate(rep.name, unmatched),
                    Err(e) => Installed::Failed(e.to_string()),
                }
            }
            Ok(_) => match eidos_install::install_archive_with_policy(
                archive,
                &mods_dir,
                &name,
                &self.game_id,
                eidos_install::OverwritePolicy::Replace,
                &ctx,
            ) {
                Ok(rep) => Installed::Ok(rep.name),
                Err(e) => Installed::Failed(e.to_string()),
            },
            Err(e) => Installed::Failed(e.to_string()),
        }
    }

    fn progress(&mut self, done: usize, total: usize, member: &str) {
        println!("  [{done}/{total}] {member}");
    }
}

fn safe(name: &str) -> String {
    eidos_install::fix_directory_name(name).unwrap_or_else(|| "Mod".to_string())
}

/// The collection's recorded FOMOD answers, in the shape the replay wants.
fn recorded_answers(m: &Mod) -> Vec<eidos_fomod::RecordedStep> {
    let Some(ch) = m.choices.as_ref() else {
        return Vec::new();
    };
    // Only FOMOD answers are replayable here. Anything else is left to the
    // installer's defaults, and the engine's `unmatched` list says so.
    if !ch.kind.is_empty() && !ch.kind.eq_ignore_ascii_case("fomod") {
        return Vec::new();
    }
    ch.options
        .as_ref()
        .map(|steps| {
            steps
                .iter()
                .map(|s| eidos_fomod::RecordedStep {
                    name: s.name.clone(),
                    groups: s
                        .groups
                        .iter()
                        .map(|g| eidos_fomod::RecordedGroup {
                            name: g.name.clone(),
                            selected: g
                                .choices
                                .iter()
                                .map(|o| eidos_fomod::RecordedOption {
                                    name: o.name.clone(),
                                    idx: o.idx,
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Put the collection's members in the order its rules ask for.
fn apply_ordering(
    inst: &Instance,
    c: &Collection,
    state: &InstallState,
    report: &mut eidos_collections::report::Report,
) {
    let (edges, lost) = eidos_collections::rules::edges(c);
    for u in lost {
        report.rules_lost.push(Note {
            subject: u.rule,
            detail: u.why.to_string(),
        });
    }
    if edges.is_empty() {
        return;
    }
    let ordered = eidos_collections::rules::order(c.mods.len(), &edges);
    for i in &ordered.cycles {
        if let Some(m) = c.mods.get(*i) {
            report.rule_cycles.push(m.name.clone());
        }
    }
    let folders = eidos_collections::install::installed_folders(c, state);
    let list = inst.modlist();
    let names: Vec<String> = list.iter().map(|m| m.name.clone()).collect();
    let merged = eidos_collections::rules::merge(&names, &folders, &ordered.sequence);
    if merged == names {
        return;
    }
    // Rebuilt from the existing entries so nothing but the order changes.
    let reordered: Vec<eidos_instance::ModEntry> = merged
        .iter()
        .filter_map(|n| list.iter().find(|m| &m.name == n).cloned())
        .collect();
    if reordered.len() == list.len() {
        let _ = inst.save_modlist(&reordered);
    }
}

/// Enable the plugins the collection expects, and merge its LOOT rules.
fn apply_plugin_rules(
    inst: &Instance,
    game: &eidos_games::DetectedGame,
    c: &Collection,
    report: &mut eidos_collections::report::Report,
) {
    if c.plugins.is_empty() && c.plugin_rules.plugins.is_empty() {
        return;
    }
    let rules: Vec<eidos_loot::UserRule> = c
        .plugin_rules
        .plugins
        .iter()
        .filter_map(|p| {
            let name = p.get("name")?.as_str()?.to_string();
            Some(eidos_loot::UserRule {
                plugin: name,
                after: p
                    .get("after")
                    .and_then(|a| a.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
                group: p
                    .get("group")
                    .and_then(|g| g.as_str())
                    .map(str::to_string),
            })
        })
        .collect();
    let groups: Vec<eidos_loot::GroupDef> = c
        .plugin_rules
        .groups
        .iter()
        .filter_map(|g| {
            Some(eidos_loot::GroupDef {
                name: g.get("name")?.as_str()?.to_string(),
                after: g
                    .get("after")
                    .and_then(|a| a.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect();
    if rules.is_empty() && groups.is_empty() {
        return;
    }
    let Some(compatdata) = game.compatdata.as_ref() else {
        report.loot_notes.push(Note {
            subject: "load order".into(),
            detail: "no Proton prefix yet, so the collection's LOOT rules were not merged".into(),
        });
        return;
    };
    let cache = inst.root.join("loot");
    let Some((_, repo)) = eidos_loot::loot_support(game.def.id) else {
        return;
    };
    let Ok((masterlist, prelude)) = eidos_loot::ensure_masterlist(repo, &cache, false) else {
        report.loot_notes.push(Note {
            subject: "load order".into(),
            detail: "the LOOT masterlist could not be fetched, so the rules were not merged".into(),
        });
        return;
    };
    let userlist = cache.join("userlist.yaml");
    let prefix = compatdata.join("pfx");
    let Some(spec) = eidos_plugins::GameSpec::for_id(game.def.id) else {
        return;
    };
    let local = eidos_plugins::plugins_txt_dir(&prefix, &spec);
    let view = eidos_loot::GameView {
        game_id: game.def.id,
        game_path: &game.install_path,
        local_path: &local,
        plugins: &[],
        mod_dirs: &[],
        masterlist: &masterlist,
        prelude: &prelude,
        userlist: Some(&userlist),
    };
    match eidos_loot::merge_user_rules(&view, &rules, &groups) {
        Ok(m) => {
            for p in m.kept_user_group {
                report.loot_notes.push(Note {
                    subject: p,
                    detail: "your own group assignment was kept".into(),
                });
            }
            for d in m.dangling_groups {
                report.loot_notes.push(Note {
                    subject: d,
                    detail: "no such LOOT group exists, so the assignment was dropped".into(),
                });
            }
            if m.rules_added > 0 {
                println!(
                    "  merged {} plugin rule(s) into the load order",
                    m.rules_added
                );
            }
        }
        Err(e) => report.loot_notes.push(Note {
            subject: "load order".into(),
            detail: format!("the collection's rules could not be merged: {e}"),
        }),
    }
}

/// Copy the collection's INI fragments in as a mod of their own.
fn apply_ini_tweaks(
    inst: &Instance,
    dir: &Path,
    c: &Collection,
    report: &mut eidos_collections::report::Report,
) {
    // The archive's own directory is `INI Tweaks`; MO2's convention inside a mod
    // is `Ini Tweaks`. On Windows those are one directory and on Linux they are
    // two, so the source is matched case-insensitively and the destination is
    // written the way MO2 spells it.
    let Some(src) = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("ini tweaks"))
        })
    else {
        return;
    };
    let name = safe(&format!("{} - INI Tweaks", c.info.name));
    let dest = inst.mods_dir().join(&name).join("Ini Tweaks");
    if let Err(e) = std::fs::create_dir_all(&dest) {
        report.loot_notes.push(Note {
            subject: "INI tweaks".into(),
            detail: e.to_string(),
        });
        return;
    }
    let mut n = 0;
    for f in std::fs::read_dir(&src).into_iter().flatten().flatten() {
        let p = f.path();
        if p.is_file() {
            if let Some(base) = p.file_name() {
                if std::fs::copy(&p, dest.join(base)).is_ok() {
                    n += 1;
                }
            }
        }
    }
    if n > 0 {
        println!("  {n} INI tweak(s) installed as \"{name}\" (enable them in its Ini Tweaks tab)");
    }
}
