//! Driving a collection install: fetching its archive, installing its members,
//! and applying the three things that are about the collection as a whole.
//!
//! Separate from the pure modules beside it, and heavier: this is where the
//! network, the archiver and the installer live. It exists so the terminal and
//! the window run the SAME code - the alternative was a second copy of it in the
//! GUI, which is how this workspace ended up with three `find_7z` implementations
//! that had quietly drifted apart.

use std::path::{Path, PathBuf};

use eidos_instance::Instance;

use crate::install::{Hooks, Installed, Obtained};
use crate::manifest::{Collection, Mod, SourceType};
use crate::report::{Note, Report};
use crate::state::InstallState;

/// Download the collection archive if it is not already here, and hand back its
/// `collection.json`.
pub fn fetch_manifest(
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

/// Everything the engine needs from the outside world.
pub struct RealHooks<'a> {
    pub nexus: &'a eidos_nexus::Nexus,
    pub inst: &'a Instance,
    pub game: &'a eidos_games::DetectedGame,
    pub game_id: String,
    /// Where per-member progress goes. The terminal prints it; the window
    /// stores it for its dialog.
    pub say: &'a mut dyn FnMut(String),
    /// The collection's own Nexus domain, for members that name none.
    pub collection_domain: String,
    /// The member keys the state already has a record for.
    ///
    /// A member this collection has attempted before may replace its own folder;
    /// a member it has never attempted must not, because a folder of that name
    /// belongs to somebody else. `Replace` is documented as wipe-and-reinstall,
    /// collections are largely made of the popular mods a user already has, and
    /// the member's name IS the Nexus mod name their folder is called after - so
    /// the collision is the ordinary case, not the exotic one.
    pub known_members: std::collections::HashSet<String>,
    /// Members installed under a name that was free, and what it was.
    pub renamed: Vec<(String, String)>,
    /// The mod folders this run has installed, in order.
    ///
    /// A collection installs a mod and then, two members later, a patch whose
    /// FOMOD asks whether that mod is present. The mod list cannot answer: a
    /// folder appended to it arrives disabled, and nothing in this pass enables
    /// it. Left alone, every such question reads "no" and the patch installs its
    /// files for a game that does not have the mod - or refuses the author's own
    /// recorded answer. So the members installed so far are handed to the
    /// installer as active, which is what they are.
    pub installed_now: Vec<PathBuf>,
}

impl RealHooks<'_> {
    /// Put a freshly installed member in the profile, enabled, at the top.
    ///
    /// Without this the folder exists and nothing loads it: an unlisted folder
    /// is discovered as DISABLED, `load_order` filters the inactive out, and the
    /// union mount gets none of the collection - while the report says it is
    /// installed the way its author built it. `eidos install` has always done
    /// this half; the collection path did not.
    ///
    /// Appended last, which is the highest priority, in install order - so a
    /// later member wins a file against an earlier one until `apply_ordering`
    /// says otherwise, and that pass is then permuting real entries instead of
    /// finding nothing to permute.
    ///
    /// Also the point where a rename becomes a fact worth reporting: one that
    /// led to a failed install is not something to put in front of the user.
    fn register(&mut self, name: &str, mods_dir: &Path, member: &str, renamed_to: &Option<String>) {
        if let Some(f) = renamed_to {
            self.renamed.push((member.to_string(), f.clone()));
        }
        let mut ml = self.inst.modlist();
        ml.retain(|m| m.name != name);
        ml.push(eidos_instance::ModEntry {
            name: name.to_string(),
            enabled: true,
            path: mods_dir.join(name),
            unmanaged: false,
        });
        let _ = self.inst.save_modlist(&ml);
    }

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

impl Hooks for RealHooks<'_> {
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
                let remote = match self.nexus.mod_info(domain, mod_id) {
                    Ok(remote) => remote,
                    Err(e) => return Obtained::Failed(e),
                };
                let gate = remote.gate;
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
                            Ok(_) => {
                                // The `.meta` sidecar. Without it this archive is
                                // invisible to `already_here`, so an interrupted
                                // collection re-downloads everything it had - and
                                // the rest of Eidos shows the row as Untracked.
                                // Written AFTER the download so a free account,
                                // which never gets this far, never pays for the
                                // extra request.
                                if let Ok(file) =
                                    self.nexus.file_info(&gate, domain, mod_id, file_id)
                                {
                                    let _ = eidos_nexus::write_download_meta(
                                        &dest, domain, &nxm, &url, &file, &remote,
                                    );
                                }
                                Obtained::Ready(dest)
                            }
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
        let mut enabled: Vec<PathBuf> = ml
            .iter()
            .filter(|x| x.is_active())
            .map(|x| x.path.clone())
            .collect();
        for p in &self.installed_now {
            if !enabled.contains(p) {
                enabled.push(p.clone());
            }
        }
        let disabled: Vec<PathBuf> = ml
            .iter()
            .filter(|x| !x.is_active() && !x.is_separator())
            .filter(|x| !self.installed_now.contains(&x.path))
            .map(|x| x.path.clone())
            .collect();
        let ctx = eidos_install::fomod_context(&self.game.data_path, &enabled, &disabled);
        let mut name = safe(&m.name);
        let key = crate::state::key_for(m, &self.collection_domain);
        // Reported only if the install then succeeds: a rename that led nowhere
        // is not something to put in front of the user.
        let mut renamed_to: Option<String> = None;
        if mods_dir.join(&name).is_dir() && !self.known_members.contains(&key) {
            name = free_name(&mods_dir, &name);
            renamed_to = Some(name.clone());
        }
        // Whatever it lands as, this collection owns it from here: a retry must
        // replace its own folder rather than step aside from it.
        self.known_members.insert(key);

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
                    Ok(rep) => {
                        self.register(&rep.name, &mods_dir, &m.name, &renamed_to);
                        self.installed_now.push(mods_dir.join(&rep.name));
                        if unmatched.is_empty() {
                            Installed::Ok(rep.name)
                        } else {
                            Installed::Approximate(rep.name, unmatched)
                        }
                    }
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
                Ok(rep) => {
                    self.register(&rep.name, &mods_dir, &m.name, &renamed_to);
                    self.installed_now.push(mods_dir.join(&rep.name));
                    Installed::Ok(rep.name)
                }
                Err(e) => Installed::Failed(e.to_string()),
            },
            Err(e) => Installed::Failed(e.to_string()),
        }
    }

    fn progress(&mut self, done: usize, total: usize, member: &str) {
        (self.say)(format!("[{done}/{total}] {member}"));
    }
}

/// A folder name under `mods/` that nothing is using yet.
fn free_name(mods_dir: &Path, wanted: &str) -> String {
    for n in 2..1000 {
        let candidate = format!("{wanted} ({n})");
        if !mods_dir.join(&candidate).exists() {
            return candidate;
        }
    }
    format!("{wanted} (collection)")
}

pub fn safe(name: &str) -> String {
    eidos_install::fix_directory_name(name).unwrap_or_else(|| "Mod".to_string())
}

/// The collection's recorded FOMOD answers, in the shape the replay wants.
pub fn recorded_answers(m: &Mod) -> Vec<eidos_fomod::RecordedStep> {
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
pub fn apply_ordering(
    inst: &Instance,
    c: &Collection,
    state: &InstallState,
    report: &mut Report,
) {
    let (edges, lost) = crate::rules::edges(c);
    for u in lost {
        report.rules_lost.push(Note {
            subject: u.rule,
            detail: u.why.to_string(),
        });
    }
    if edges.is_empty() {
        return;
    }
    let ordered = crate::rules::order(c.mods.len(), &edges);
    for i in &ordered.cycles {
        if let Some(m) = c.mods.get(*i) {
            report.rule_cycles.push(m.name.clone());
        }
    }
    let folders = crate::install::installed_folders(c, state);
    let list = inst.modlist();
    let names: Vec<String> = list.iter().map(|m| m.name.clone()).collect();
    let merged = crate::rules::merge(&names, &folders, &ordered.sequence);
    if merged == names {
        return;
    }
    // Rebuilt from the existing entries so nothing but the order changes.
    let reordered: Vec<eidos_instance::ModEntry> = merged
        .iter()
        .filter_map(|n| list.iter().find(|m| &m.name == n).cloned())
        .collect();
    // Only ever a permutation. Counting the entries is not enough: a name that
    // landed in two slots keeps the count and loses a mod.
    let mut before: Vec<&String> = names.iter().collect();
    let mut after: Vec<&String> = merged.iter().collect();
    before.sort();
    after.sort();
    if reordered.len() == list.len() && before == after {
        let _ = inst.save_modlist(&reordered);
    }
}

/// Turn on the plugins the collection expects, and turn off the ones it does
/// not.
///
/// `collection.plugins` is a list of ACTIVATION, not a load order - Vortex
/// writes a position into it and its own reader ignores that too; ordering is
/// expressed as LOOT rules, which [`apply_plugin_rules`] merges. Under an
/// opt-out plugin model most entries are already what they ask for; the one that
/// matters is `"enabled": false`, an ESP the author deliberately unchecked,
/// which would otherwise load.
pub fn apply_plugin_states(
    inst: &Instance,
    game: &eidos_games::DetectedGame,
    c: &Collection,
    report: &mut Report,
) {
    if c.plugins.is_empty() {
        return;
    }
    let Some(spec) = eidos_plugins::GameSpec::for_id(game.def.id) else {
        return;
    };
    let Some(compatdata) = game.compatdata.as_ref() else {
        report.loot_notes.push(Note {
            subject: "plugins".into(),
            detail: format!(
                "no Proton prefix yet, so the {} plugin(s) the collection names were not \
                 switched on",
                c.plugins.len()
            ),
        });
        return;
    };
    let local_dir = eidos_plugins::plugins_txt_dir(&compatdata.join("pfx"), &spec);
    let _ = inst.ensure_profiles();
    let prof = inst.active();
    if prof.seed_plugin_state(&local_dir, &spec).is_err() {
        report.loot_notes.push(Note {
            subject: "plugins".into(),
            detail: "the profile's plugin state could not be read, so the collection's \
                     activations were not applied"
                .into(),
        });
        return;
    }
    let state_dir = prof.plugins_state_dir();

    // The same discovery a launch does: the game's own Data lowest, each enabled
    // mod in list order, Overwrite last.
    let mut sources: Vec<(String, PathBuf)> = vec![(String::new(), game.data_path.clone())];
    sources.extend(
        inst.modlist()
            .into_iter()
            .filter(|m| m.is_active())
            .map(|m| (m.name.clone(), m.path.clone())),
    );
    sources.push(("overwrite".to_string(), inst.overwrite_dir()));
    let mut list = eidos_plugins::PluginList::discover(&sources, &spec);
    list.apply_prefix_state(&state_dir, &spec);
    list.locked = prof.read_locked_order();
    list.refresh(&spec);

    let before: Vec<bool> = list.plugins.iter().map(|p| p.enabled).collect();
    let mut missing: Vec<String> = Vec::new();
    for p in &c.plugins {
        if p.name.trim().is_empty() {
            continue;
        }
        if !list.set_enabled(&p.name, p.enabled) {
            missing.push(p.name.clone());
        }
    }
    // Under an opt-out model most of a collection's list is already what it
    // asks for, and rewriting the load order to say nothing new is a write into
    // the user's profile for no reason.
    if list.plugins.iter().map(|p| p.enabled).eq(before) {
        for name in missing {
            report.loot_notes.push(Note {
                subject: name,
                detail: "the collection expects this plugin and the instance does not have it"
                    .into(),
            });
        }
        return;
    }
    list.refresh(&spec);
    if let Err(e) = list.write_load_order(&state_dir, &spec) {
        report.loot_notes.push(Note {
            subject: "plugins".into(),
            detail: format!("the collection's plugin activations could not be written: {e}"),
        });
        return;
    }
    // Shadow for tools that read the prefix; never fatal.
    let _ = list.write_load_order(&local_dir, &spec);
    for name in missing {
        report.loot_notes.push(Note {
            subject: name,
            detail: "the collection expects this plugin and the instance does not have it".into(),
        });
    }
}

/// Merge the collection's LOOT rules into the instance's userlist.
pub fn apply_plugin_rules(
    inst: &Instance,
    game: &eidos_games::DetectedGame,
    c: &Collection,
    report: &mut Report,
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
            for d in m.dangling_after {
                report.loot_notes.push(Note {
                    subject: d,
                    detail: "the collection's group would load after a group nothing defines, \
                             which stops LOOT sorting entirely, so that part was dropped"
                        .into(),
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
pub fn apply_ini_tweaks(
    inst: &Instance,
    dir: &Path,
    c: &Collection,
    report: &mut Report,
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
