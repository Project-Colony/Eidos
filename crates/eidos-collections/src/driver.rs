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
}

impl RealHooks<'_> {
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
        (self.say)(format!("[{done}/{total}] {member}"));
    }
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
    if reordered.len() == list.len() {
        let _ = inst.save_modlist(&reordered);
    }
}

/// Enable the plugins the collection expects, and merge its LOOT rules.
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
