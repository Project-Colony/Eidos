//! Cached archive analysis on a worker; UI frames only consume completed maps.
use crate::*;
use eidos_gamefeatures::archives::{ArchivePlan, archive_plan};
use std::sync::mpsc::{self, Receiver, TryRecvError};

type Parts = Vec<(Layer, (Vec<String>, bool))>;
pub(crate) struct ArchiveWorker {
    epoch: u64,
    receiver: Receiver<(ConflictMap, ArchivePlan, Vec<String>)>,
}

/// A single worker is allowed at a time. Changes during a scan invalidate its
/// epoch; its result is discarded and the next update starts the latest input.
pub(crate) fn schedule_archive_conflicts(app: &mut App) {
    if app.archive_job.is_some() || app.archive_completed_epoch == Some(app.archive_epoch.get()) {
        return;
    }
    let (Some(game), Some(inst), Some(layers)) = (
        selected_game(app).cloned(),
        app.created.clone(),
        conflict_layers(app),
    ) else {
        return;
    };
    let epoch = app.archive_epoch.get();
    let cache = app.files_cache.borrow().clone();
    let plugins = app.plugins.clone();
    let tweaks = inst.enabled_ini_tweaks(&app.mods);
    let (sender, receiver) = mpsc::channel();
    app.archive_job = Some(ArchiveWorker { epoch, receiver });
    app.archive_warnings.clear();
    std::thread::spawn(move || {
        let parts: Parts = layers
            .into_iter()
            .map(|l| {
                let files = cache
                    .get(&l.name)
                    .cloned()
                    .unwrap_or_else(|| eidos_conflicts::collect_files(&l.root));
                (l, files)
            })
            .collect();
        let stack = eidos_core::LayerStack::new(
            parts
                .iter()
                .filter(|(l, _)| l.origin != u32::MAX)
                .map(|(l, _)| l.root.clone())
                .collect(),
            inst.overwrite_dir(),
        );
        let parts = visible_conflict_parts(parts, &stack);
        let spec = GameSpec::for_id(game.def.id);
        let mut plugins = plugins.or_else(|| {
            spec.as_ref().map(|spec| {
                let sources: Vec<_> = parts
                    .iter()
                    .rev()
                    .map(|(l, _)| (l.name.clone(), l.root.clone()))
                    .collect();
                let mut list = PluginList::discover(&sources, spec);
                let profile = inst.active();
                if profile.has_plugin_state() {
                    list.apply_prefix_state(&profile.plugins_state_dir(), spec)
                } else if let Some(cd) = &game.compatdata {
                    list.apply_prefix_state(&plugins_txt_dir(&cd.join("pfx"), spec), spec)
                }
                list.locked = profile.read_locked_order();
                list.refresh(spec);
                list
            })
        });
        if let Some(list) = &mut plugins {
            list.plugins.retain(|p| {
                stack
                    .resolve_read(&p.name)
                    .is_some_and(|path| path.is_file())
            });
        }
        let active: Vec<_> = plugins
            .as_ref()
            .map(|l| {
                l.plugins
                    .iter()
                    .filter(|p| p.enabled)
                    .map(|p| p.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let (mut texts, mut warnings) = archive_ini_texts(&game, &inst);
        for path in tweaks {
            if let Some((text, _)) = eidos_instance::read_text_lossy(&path) {
                texts.push(text)
            } else {
                warnings.push(format!(
                    "Could not read enabled INI tweak {}.",
                    path.display()
                ));
            }
        }
        let loose = ConflictMap::build_from(&parts);
        let available: Vec<_> = loose
            .files
            .iter()
            .filter(|(p, _)| !p.contains('/') && (p.ends_with(".bsa") || p.ends_with(".ba2")))
            .map(|(_, n)| n.display_path.clone())
            .collect();
        let plan = archive_plan(game.def.id, &available, &active, &texts, "en");
        let active: Vec<_> = plan
            .archives
            .iter()
            .map(|a| eidos_conflicts::ActiveArchive {
                name: a.name.clone(),
                plugin: a.plugin.clone(),
                order_uncertain: a.order_uncertain,
            })
            .collect();
        let map = ConflictMap::build_with_archives_from(&parts, &active);
        warnings.extend(plan.diagnostics.iter().cloned());
        warnings.extend(
            map.archive_diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.archive, d.error)),
        );
        let _ = sender.send((map, plan, warnings));
    });
}

pub(crate) fn poll_archive_conflicts(app: &mut App) {
    let Some(job) = &app.archive_job else { return };
    let epoch = job.epoch;
    let result = match job.receiver.try_recv() {
        Ok(v) => Some(Ok(v)),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => Some(Err(())),
    };
    let Some(result) = result else { return };
    app.archive_job = None;
    if epoch != app.archive_epoch.get() {
        return;
    }
    app.archive_completed_epoch = Some(epoch);
    match result {
        Ok((map, plan, warnings)) => {
            app.conflicts = Some(map);
            app.archive_plan = Some(plan);
            app.archive_warnings = warnings;
        }
        Err(()) => {
            app.archive_warnings =
                vec!["Archive analysis worker stopped without a result. Refresh to retry.".into()]
        }
    }
    app.archives_cache.borrow_mut().take();
    app.diag_dirty = true;
}

/// Reuse the union's own whiteout/opacity query. Keep lower losing providers
/// only when they actually survive the writable layer's deletion rules.
pub(crate) fn visible_conflict_parts(mut parts: Parts, stack: &eidos_core::LayerStack) -> Parts {
    for (layer, (files, _)) in &mut parts {
        let mut seen = HashSet::new();
        files.retain(|p| {
            if layer.origin != u32::MAX && !stack.lower_path_visible(p) {
                return false;
            }
            let Some(winner) = stack.resolve_read(p) else {
                return false;
            };
            if winner.starts_with(&layer.root) && winner != layer.root.join(p) {
                return false;
            }
            seen.insert(p.to_ascii_lowercase())
        });
    }
    parts
}

/// Game defaults first, then each effective profile/Proton INI. Profile files
/// replace their matching Proton files; absent profile copies fall back.
pub(crate) fn archive_ini_texts(
    game: &DetectedGame,
    inst: &Instance,
) -> (Vec<String>, Vec<String>) {
    let mut paths = Vec::new();
    if let Some(base) = eidos_gamefeatures::ini_file_for(game.def.id) {
        let stem = base.trim_end_matches(".ini");
        for p in [
            game.install_path.join(format!("{stem}_Default.ini")),
            game.install_path.join(format!("{stem}_default.ini")),
            game.install_path.join(base),
        ] {
            if p.is_file() && !paths.contains(&p) {
                paths.push(p)
            }
        }
    }
    let docs = game.compatdata.as_ref().and_then(|cd| {
        GameSpec::for_id(game.def.id)
            .map(|s| eidos_plugins::documents_my_games_dir(&cd.join("pfx"), &s))
    });
    for name in eidos_gamefeatures::ini_files_for(game.def.id) {
        let profile = inst.active().ini_path(name);
        if profile.exists() {
            paths.push(profile)
        } else if let Some(docs) = &docs {
            let p = docs.join(name);
            if p.exists() {
                paths.push(p)
            }
        }
    }
    let mut texts = Vec::new();
    let mut warnings = Vec::new();
    for path in paths {
        match eidos_instance::read_text_lossy(&path) {
            Some((text, _)) => texts.push(text),
            None => warnings.push(format!(
                "Could not read archive configuration {}.",
                path.display()
            )),
        }
    }
    (texts, warnings)
}

pub(crate) fn provider_label(map: &ConflictMap, p: &eidos_conflicts::AssetProvider) -> String {
    match (&p.archive, &p.plugin) {
        (Some(a), Some(plugin)) => format!("{} / {a} / {plugin}", map.name(p.origin)),
        (Some(a), None) => format!("{} / {a} / INI", map.name(p.origin)),
        (None, _) => format!("{} / loose", map.name(p.origin)),
    }
}

/// Borrow the completed member tree without allocating one provider per frame.
/// The same-origin case is displayed too: two archives in one mod can collide.
pub(crate) fn archive_member_rows<'a>(
    app: &App,
    map: &ConflictMap,
    origin: u32,
    limit: usize,
) -> Element<'a, Message> {
    let paths = map
        .asset_conflicts
        .get(&origin)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let count = paths.len();
    let mut rows = Column::new().spacing(4);
    for n in paths
        .iter()
        .take(limit)
        .filter_map(|key| map.asset_files.get(key))
    {
        let providers = || std::iter::once(&n.winner).chain(&n.alternatives);
        let verdict = if n.precedence_uncertain {
            format!(
                "Winner unresolved: {}",
                providers()
                    .map(|p| provider_label(map, p))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        } else {
            format!(
                "Winner: {}; alternatives: {}",
                provider_label(map, &n.winner),
                n.alternatives
                    .iter()
                    .map(|p| provider_label(map, p))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        };
        rows = rows
            .push(text(n.display_path.clone()).size(11.0))
            .push(text(verdict).size(10.0));
    }
    let pending = if app.archive_job.is_some() {
        " (updating)"
    } else {
        ""
    };
    Column::new()
        .spacing(4)
        .push(
            text(format!(
                "Archive member conflicts: {count}{pending} (showing up to {limit})"
            ))
            .size(12.0),
        )
        .push(scrollable(rows).height(Length::Fill))
        .into()
}
