//! Cached archive analysis on a worker; UI frames only consume completed maps.
use crate::*;
use eidos_gamefeatures::archives::{archive_plan, ArchivePlan};
use std::sync::mpsc::{self, Receiver, TryRecvError};
type Sources = HashMap<PathBuf, eidos_conflicts::ArchiveIdentity>;

type Parts = Vec<(Layer, (Vec<String>, bool))>;
pub(crate) struct ArchiveWorker {
    epoch: u64,
    receiver: Receiver<(ConflictMap, ArchivePlan, Vec<String>, Sources)>,
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
        let spec = game.plugin_spec();
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
                } else if let Some(dir) = game.plugin_state_dir() {
                    list.apply_prefix_state(&dir, spec)
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
        let mut sources: Sources = parts
            .iter()
            .flat_map(|(layer, (files, _))| {
                files
                    .iter()
                    .filter(|name| {
                        name.to_ascii_lowercase().ends_with(".bsa")
                            || name.to_ascii_lowercase().ends_with(".ba2")
                    })
                    .map(|name| layer.root.join(name))
            })
            .filter_map(|path| {
                eidos_conflicts::archive_identity(&path)
                    .ok()
                    .map(|identity| (path, identity))
            })
            .collect();
        let map = ConflictMap::build_with_archives_from(&parts, &active);
        sources.retain(|path, identity| {
            eidos_conflicts::archive_identity(path).is_ok_and(|current| current == *identity)
        });
        warnings.extend(plan.diagnostics.iter().cloned());
        warnings.extend(
            map.archive_diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.archive, d.error)),
        );
        let _ = sender.send((map, plan, warnings, sources));
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
        Ok((map, plan, warnings, sources)) => {
            app.conflicts = Some(map);
            app.archive_plan = Some(plan);
            app.archive_warnings = warnings;
            app.archive_sources = sources;
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
    let docs = game.prefix().and_then(|prefix| {
        game.plugin_spec()
            .map(|s| eidos_plugins::documents_my_games_dir(&prefix, &s))
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
    let filter = app.archive_filter.to_ascii_lowercase();
    let paths: Vec<_> = paths.iter().filter(|path| path.contains(&filter)).collect();
    let count = paths.len();
    let pages = count.div_ceil(limit.max(1)).max(1);
    let page = app.archive_page.min(pages - 1);
    let mut rows = Column::new().spacing(8);
    for node in paths
        .iter()
        .skip(page * limit)
        .take(limit)
        .filter_map(|key| map.asset_files.get(*key))
    {
        let mut row = Column::new()
            .spacing(3)
            .push(text(node.display_path.clone()).size(11.0));
        for (index, provider) in std::iter::once(&node.winner)
            .chain(&node.alternatives)
            .enumerate()
        {
            let label = if node.precedence_uncertain {
                "Unresolved"
            } else if index == 0 {
                "Winner"
            } else {
                "Alternative"
            };
            let action = |export| Message::ArchiveProviderAction {
                epoch: app.archive_completed_epoch.unwrap_or_default(),
                member: node.display_path.clone(),
                provider: provider.clone(),
                export,
            };
            let active = app.archive_completed_epoch == Some(app.archive_epoch.get());
            row = row.push(
                Row::new()
                    .spacing(6)
                    .push(
                        text(format!("{label}: {}", provider_label(map, provider)))
                            .size(10.0)
                            .width(Length::Fill),
                    )
                    .push(
                        button(text("Preview").size(10.0))
                            .on_press_maybe(active.then(|| action(false))),
                    )
                    .push(button(text("Export").size(10.0)).on_press_maybe(
                        (active && provider.archive.is_some()).then(|| action(true)),
                    )),
            );
        }
        rows = rows.push(row);
    }
    let header = Row::new()
        .spacing(6)
        .push(
            text(format!(
                "Archive member conflicts: {count} · page {}/{}",
                page + 1,
                pages
            ))
            .size(12.0),
        )
        .push(
            button(text("Previous")).on_press_maybe(page.checked_sub(1).map(Message::ArchivePage)),
        )
        .push(
            button(text("Next"))
                .on_press_maybe((page + 1 < pages).then_some(Message::ArchivePage(page + 1))),
        );
    Column::new()
        .spacing(4)
        .push(header)
        .push(
            text_input("Filter archive member paths", &app.archive_filter)
                .on_input(Message::ArchiveFilterChanged),
        )
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

#[derive(Debug, Clone)]
pub(crate) struct MemberSource {
    pub path: PathBuf,
    pub member: String,
    pub identity: eidos_conflicts::ArchiveIdentity,
}

pub(crate) struct ExportRequest {
    id: u64,
    target: Option<CollectionTarget>,
    source: MemberSource,
}

fn current_target(app: &App) -> Option<CollectionTarget> {
    let inst = app.created.as_ref()?;
    Some(CollectionTarget {
        instance: inst.root.clone(),
        profile: inst.active_profile(),
        installation: selected_game(app)?.selection_id(),
    })
}

pub(crate) fn provider_action(
    app: &mut App,
    epoch: u64,
    member: String,
    provider: eidos_conflicts::AssetProvider,
    export: bool,
) -> Task<Message> {
    let valid = app.archive_completed_epoch == Some(epoch)
        && epoch == app.archive_epoch.get()
        && app
            .conflicts
            .as_ref()
            .and_then(|m| m.asset_files.get(&member.to_ascii_lowercase()))
            .is_some_and(|node| node.winner == provider || node.alternatives.contains(&provider));
    if !valid {
        app.status = Some("Archive analysis changed. Refresh before selecting a provider.".into());
        return Task::none();
    }
    let Some(layer) = conflict_layers(app)
        .and_then(|layers| layers.into_iter().find(|l| l.origin == provider.origin))
    else {
        return Task::none();
    };
    let relative = provider.archive.as_deref().unwrap_or(&member);
    let actual = app
        .files_cache
        .borrow()
        .get(&layer.name)
        .and_then(|(files, _)| {
            files
                .iter()
                .find(|name| name.eq_ignore_ascii_case(relative))
                .cloned()
        });
    let actual = actual
        .or_else(|| {
            app.archive_sources.keys().find_map(|path| {
                let tail = path.strip_prefix(&layer.root).ok()?.to_str()?;
                tail.eq_ignore_ascii_case(relative)
                    .then(|| tail.to_string())
            })
        })
        .unwrap_or_else(|| relative.to_string());
    let Some(path) = resolve_in_mod(&layer.root, &actual).filter(|p| p.is_file()) else {
        app.status =
            Some("The selected provider is no longer present. Refresh the file list.".into());
        return Task::none();
    };
    if provider.archive.is_none() {
        return crate::file_preview::start(app, path, None, None, None);
    }
    let Some(identity) =
        app.archive_sources.get(&path).cloned().filter(|i| {
            eidos_conflicts::archive_identity(&path).is_ok_and(|current| current == *i)
        })
    else {
        app.status =
            Some("The archive changed since it was scanned. Refresh before reading it.".into());
        return Task::none();
    };
    let source = MemberSource {
        path,
        member,
        identity,
    };
    if !export {
        return crate::file_preview::start(app, source.path.clone(), None, None, Some(source));
    }
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = Path::new(&source.member)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    app.archive_export = Some(ExportRequest {
        id,
        target: current_target(app),
        source,
    });
    Task::perform(
        rfd::AsyncFileDialog::new()
            .set_title("Export archive member")
            .set_file_name(name)
            .save_file(),
        move |picked| Message::ArchiveExportDestination(id, picked.map(|h| h.path().to_path_buf())),
    )
}

pub(crate) fn export_destination(
    app: &mut App,
    id: u64,
    destination: Option<PathBuf>,
) -> Task<Message> {
    let Some(request) = app.archive_export.as_ref().filter(|r| r.id == id) else {
        return Task::none();
    };
    if request.target != current_target(app) {
        app.archive_export = None;
        app.status =
            Some("Export cancelled because the active profile or installation changed.".into());
        return Task::none();
    }
    let Some(destination) = destination else {
        app.archive_export = None;
        return Task::none();
    };
    let source = request.source.clone();
    Task::perform(
        async move {
            eidos_conflicts::export_archive_member_checked(
                &source.path,
                &source.member,
                &destination,
                16 * 1024 * 1024 * 1024,
                Some(&source.identity),
            )
            .map_err(|e| e.to_string())?;
            Ok(destination)
        },
        move |result| Message::ArchiveExportFinished(id, result),
    )
}

pub(crate) fn export_finished(app: &mut App, id: u64, result: Result<PathBuf, String>) {
    if app.archive_export.as_ref().is_none_or(|r| r.id != id) {
        return;
    }
    app.archive_export = None;
    app.status = Some(match result {
        Ok(path) => format!("Archive member exported to {}", path.display()),
        Err(error) => format!("Archive export failed: {error}"),
    });
}
