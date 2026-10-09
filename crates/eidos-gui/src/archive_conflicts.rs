//! Cached archive analysis on a worker; UI frames only consume completed maps.
use crate::*;
use eidos_gamefeatures::archives::{archive_plan, ArchivePlan};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
type Sources = HashMap<PathBuf, eidos_conflicts::ArchiveIdentity>;
type Analysis = (ConflictMap, ArchivePlan, Vec<String>, Sources);

fn validate_sources(sources: &Sources) -> Result<(), String> {
    if sources.iter().any(|(path, identity)| {
        !eidos_conflicts::archive_identity(path).is_ok_and(|current| current == *identity)
    }) {
        Err("Archives changed during analysis. Refresh to retry.".into())
    } else {
        Ok(())
    }
}

type Parts = Vec<(Layer, (Vec<String>, bool))>;
pub(crate) struct ArchiveWorker {
    epoch: u64,
    receiver: Receiver<Result<Analysis, String>>,
    cancel: Arc<AtomicBool>,
}

impl Drop for ArchiveWorker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// A single worker is allowed at a time. Changes during a scan invalidate its
/// epoch; its result is discarded and the next update starts the latest input.
pub(crate) fn schedule_archive_conflicts(app: &mut App) {
    if let Some(job) = &app.archive_job {
        if app.running.is_some() || job.epoch != app.archive_epoch.get() {
            job.cancel.store(true, Ordering::Relaxed);
        }
        return;
    }
    if app.running.is_some() || app.archive_completed_epoch == Some(app.archive_epoch.get()) {
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
    let cancel = Arc::new(AtomicBool::new(false));
    app.archive_job = Some(ArchiveWorker {
        epoch,
        receiver,
        cancel: cancel.clone(),
    });
    app.archive_warnings.clear();
    std::thread::spawn(move || {
        let mut parts: Parts = Vec::with_capacity(layers.len());
        for l in layers {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let files = cache
                .get(&l.name)
                .cloned()
                .unwrap_or_else(|| eidos_conflicts::collect_files(&l.root));
            parts.push((l, files));
        }
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let stack = eidos_core::LayerStack::new(
            parts
                .iter()
                .filter(|(l, _)| l.origin != u32::MAX)
                .map(|(l, _)| l.root.clone())
                .collect(),
            inst.overwrite_dir(),
        );
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let parts = visible_conflict_parts(parts, &stack, &cancel);
        if cancel.load(Ordering::Relaxed) {
            return;
        }
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
        if cancel.load(Ordering::Relaxed) {
            return;
        }
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
        let sources: Sources = parts
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
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let map = ConflictMap::build_with_archives_from(&parts, &active);
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        if let Err(error) = validate_sources(&sources) {
            let _ = sender.send(Err(error));
            return;
        }
        warnings.extend(plan.diagnostics.iter().cloned());
        warnings.extend(
            map.archive_diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.archive, d.error)),
        );
        let _ = sender.send(Ok((map, plan, warnings, sources)));
    });
}

pub(crate) fn poll_archive_conflicts(app: &mut App) {
    let Some(job) = &app.archive_job else { return };
    let epoch = job.epoch;
    let result = match job.receiver.try_recv() {
        Ok(v) => Some(v),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => Some(Err(
            "Archive analysis worker stopped without a result. Refresh to retry.".into(),
        )),
    };
    let Some(result) = result else { return };
    let cancelled = job.cancel.load(Ordering::Relaxed);
    app.archive_job = None;
    if cancelled || epoch != app.archive_epoch.get() {
        return;
    }
    let result = result.and_then(|analysis| {
        validate_sources(&analysis.3)?;
        Ok(analysis)
    });
    app.archive_completed_epoch = Some(epoch);
    match result {
        Ok((map, plan, warnings, sources)) => {
            app.conflicts = Some(map);
            app.archive_plan = Some(plan);
            app.archive_warnings = warnings;
            app.archive_sources = sources;
        }
        Err(error) => {
            // A failed new epoch cannot certify providers from an older scan.
            if let Some(map) = &mut app.conflicts {
                map.asset_files.clear();
                map.asset_mods.clear();
                map.asset_conflicts.clear();
                map.archive_diagnostics.clear();
            }
            app.archive_plan = None;
            app.archive_sources.clear();
            app.archive_warnings = vec![error];
        }
    }
    app.archives_cache.borrow_mut().take();
    app.diag_dirty = true;
}

/// Reuse the union's own whiteout/opacity query. Keep lower losing providers
/// only when they actually survive the writable layer's deletion rules.
fn visible_conflict_parts(
    mut parts: Parts,
    stack: &eidos_core::LayerStack,
    cancel: &AtomicBool,
) -> Parts {
    for (layer, (files, _)) in &mut parts {
        if cancel.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let mut seen = HashSet::new();
        files.retain(|p| {
            if cancel.load(Ordering::Relaxed) {
                return false;
            }
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

#[derive(Debug, Clone)]
pub(crate) enum ProviderSource {
    Loose(PathBuf),
    Archive(MemberSource),
}

/// Resolve one already-selected provider without cloning the complete asset map.
pub(crate) fn resolve_provider(
    app: &App,
    member: &str,
    provider: &eidos_conflicts::AssetProvider,
) -> Result<ProviderSource, String> {
    let layer = conflict_layers(app)
        .and_then(|layers| layers.into_iter().find(|l| l.origin == provider.origin))
        .ok_or("The selected provider is no longer enabled")?;
    let relative = provider.archive.as_deref().unwrap_or(member);
    // Winning map paths already retain their physical spelling. Only losing
    // providers need the cached spelling fallback.
    let direct = resolve_in_mod(&layer.root, relative).filter(|p| p.is_file());
    let path = direct
        .or_else(|| {
            let actual = app
                .files_cache
                .borrow()
                .get(&layer.name)
                .and_then(|(files, _)| {
                    files
                        .iter()
                        .find(|name| name.eq_ignore_ascii_case(relative))
                        .cloned()
                })
                .or_else(|| {
                    app.archive_sources.keys().find_map(|path| {
                        let tail = path.strip_prefix(&layer.root).ok()?.to_str()?;
                        tail.eq_ignore_ascii_case(relative)
                            .then(|| tail.to_string())
                    })
                })?;
            resolve_in_mod(&layer.root, &actual).filter(|p| p.is_file())
        })
        .ok_or("The selected provider is no longer present. Refresh the file list.")?;
    if provider.archive.is_none() {
        return Ok(ProviderSource::Loose(path));
    }
    let identity = app
        .archive_sources
        .get(&path)
        .cloned()
        .filter(|i| eidos_conflicts::archive_identity(&path).is_ok_and(|current| current == *i))
        .ok_or("The archive changed since it was scanned. Refresh before reading it.")?;
    Ok(ProviderSource::Archive(MemberSource {
        path,
        member: member.into(),
        identity,
    }))
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
    let source = match resolve_provider(app, &member, &provider) {
        Ok(ProviderSource::Loose(path)) => {
            return crate::file_preview::start(app, path, None, None, None);
        }
        Ok(ProviderSource::Archive(source)) => source,
        Err(error) => {
            app.status = Some(error);
            return Task::none();
        }
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
        crate::background_work::run(move || {
            eidos_conflicts::export_archive_member_checked(
                &source.path,
                &source.member,
                &destination,
                16 * 1024 * 1024 * 1024,
                Some(&source.identity),
            )
            .map_err(|e| e.to_string())?;
            Ok(destination)
        }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn failed_scans_do_not_certify_old_archive_providers() {
        for failure in ["disconnected", "changed_during_scan", "changed_after_send"] {
            let (mut app, root) = crate::test_support::data_app(&[("old.bsa", "archive")], &[]);
            let mut map = compute_conflicts(&app).unwrap();
            let loose_count = map.files.len();
            map.asset_mods = map.mods.clone();
            map.asset_conflicts.insert(1, vec!["texture.dds".into()]);
            map.asset_files.insert(
                "texture.dds".into(),
                eidos_conflicts::AssetNode {
                    winner: eidos_conflicts::AssetProvider {
                        origin: 1,
                        archive: Some("old.bsa".into()),
                        plugin: None,
                    },
                    alternatives: Vec::new(),
                    display_path: "texture.dds".into(),
                    precedence_uncertain: false,
                },
            );
            app.conflicts = Some(map);
            app.archive_plan = Some(archive_plan("skyrimse", &[], &[], &[], "en"));
            let source = app.mods[0].path.join("old.bsa");
            app.archive_sources.insert(
                source.clone(),
                eidos_conflicts::archive_identity(&source).unwrap(),
            );
            let (sender, receiver) = mpsc::channel();
            if failure == "changed_during_scan" {
                sender
                    .send(Err("Archives changed during analysis".into()))
                    .unwrap();
            } else if failure == "changed_after_send" {
                sender
                    .send(Ok((
                        app.conflicts.clone().unwrap(),
                        app.archive_plan.clone().unwrap(),
                        Vec::new(),
                        app.archive_sources.clone(),
                    )))
                    .unwrap();
                fs::write(&source, "replacement archive").unwrap();
            }
            drop(sender);
            app.archive_job = Some(ArchiveWorker {
                epoch: app.archive_epoch.get(),
                receiver,
                cancel: Arc::new(AtomicBool::new(false)),
            });
            poll_archive_conflicts(&mut app);
            let map = app.conflicts.as_ref().unwrap();
            assert_eq!(map.files.len(), loose_count);
            assert!(map.asset_files.is_empty());
            assert!(map.asset_mods.is_empty());
            assert!(map.asset_conflicts.is_empty());
            assert!(app.archive_plan.is_none());
            assert!(app.archive_sources.is_empty());
            assert!(!app.archive_warnings.is_empty());
            schedule_archive_conflicts(&mut app);
            assert!(
                app.archive_job.is_none(),
                "failure must wait for explicit refresh"
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    fn archive_bsa_fixture() -> Vec<u8> {
        let folder = b"textures\0";
        let name = b"shared.dds\0";
        let record = 60 + 1 + folder.len();
        let names = record + 16;
        let payload = names + name.len();
        let mut b = vec![0; payload + 1];
        b[..4].copy_from_slice(b"BSA\0");
        for (at, n) in [
            (4, 105),
            (8, 36),
            (12, 3),
            (16, 1),
            (20, 1),
            (24, folder.len() as u32),
            (28, name.len() as u32),
            (44, 1),
            (52, (60 + name.len()) as u32),
            (record + 8, 1),
            (record + 12, payload as u32),
        ] {
            b[at..at + 4].copy_from_slice(&n.to_le_bytes())
        }
        b[60] = folder.len() as u8;
        b[61..61 + folder.len()].copy_from_slice(folder);
        b[names..payload].copy_from_slice(name);
        b
    }

    fn finish_archive_worker(app: &mut App) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.archive_job.is_some() {
            poll_archive_conflicts(app);
            assert!(
                std::time::Instant::now() < deadline,
                "archive worker timeout"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn archive_analysis_defers_during_runs_and_restarts_cancelled_work() {
        let (mut app, root) = data_app(&[("example.txt", "data")], &[]);
        start_run(
            &mut app,
            "test child".into(),
            std::process::Command::new("/bin/true"),
        );
        schedule_archive_conflicts(&mut app);
        assert!(
            app.archive_job.is_none(),
            "started an archive scan during a run"
        );
        app.running.take();
        schedule_archive_conflicts(&mut app);
        assert!(app.archive_job.is_some());
        start_run(
            &mut app,
            "test child".into(),
            std::process::Command::new("/bin/true"),
        );
        schedule_archive_conflicts(&mut app);
        finish_archive_worker(&mut app);
        assert!(
            app.archive_completed_epoch.is_none(),
            "published cancelled analysis"
        );
        app.running.take();
        schedule_archive_conflicts(&mut app);
        finish_archive_worker(&mut app);
        assert_eq!(app.archive_completed_epoch, Some(app.archive_epoch.get()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archive_worker_uses_plugin_order_ini_registration_and_cached_members() {
        let (mut app, root) = data_app(&[("A.esp", "plugin")], &[]);
        let a = app.mods[0].path.clone();
        let b = root.join("mods/BBB");
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("A.bsa"), archive_bsa_fixture()).unwrap();
        fs::write(a.join("Standalone.bsa"), archive_bsa_fixture()).unwrap();
        fs::write(b.join("B.esp"), b"plugin").unwrap();
        fs::write(b.join("B.bsa"), archive_bsa_fixture()).unwrap();
        app.mods.push(ModEntry {
            name: "BBB".into(),
            enabled: true,
            path: b,
            unmanaged: false,
        });
        let mut plugins = PluginList::default();
        plugins.plugins = vec![plugin_row("A.esp", "AAA"), plugin_row("B.esp", "BBB")];
        for p in &mut plugins.plugins {
            p.enabled = true
        }
        app.plugins = Some(plugins);
        let profile = app.created.as_ref().unwrap().active();
        fs::write(
            profile.ini_path("Skyrim.ini"),
            "[Archive]\nsResourceArchiveList=Standalone.bsa\n",
        )
        .unwrap();
        schedule_archive_conflicts(&mut app);
        finish_archive_worker(&mut app);
        let map = app.conflicts.as_ref().unwrap();
        let node = &map.asset_files["textures/shared.dds"];
        assert_eq!(node.winner.origin, 2);
        assert_eq!(node.winner.plugin.as_deref(), Some("B.esp"));
        assert_eq!(map.state(1), ConflictState::Overwritten);
        assert_eq!(node.alternatives.len(), 2);
        assert!(provider_label(map, &node.winner).contains("B.bsa / B.esp"));
        assert!(app.archive_warnings.is_empty());
        app.plugins.as_mut().unwrap().plugins[1].enabled = false;
        plugin_state_changed(&app);
        schedule_archive_conflicts(&mut app);
        finish_archive_worker(&mut app);
        assert_eq!(
            app.conflicts.as_ref().unwrap().asset_files["textures/shared.dds"]
                .winner
                .plugin
                .as_deref(),
            Some("A.esp")
        );
        fs::write(a.join("A.bsa"), b"broken").unwrap();
        let epoch = app.archive_epoch.get();
        let provider = app.conflicts.as_ref().unwrap().asset_files["textures/shared.dds"]
            .winner
            .clone();
        let _ = update_inner(
            &mut app,
            Message::ArchiveProviderAction {
                epoch,
                member: "textures/shared.dds".into(),
                provider,
                export: false,
            },
        );
        assert!(app.preview_pending.is_none());
        assert!(app.status.as_deref().unwrap().contains("changed since"));
        schedule_archive_conflicts(&mut app);
        assert!(
            app.archive_job.is_none(),
            "unchanged input uses cached member map"
        );
        bump_views(&app);
        schedule_archive_conflicts(&mut app);
        finish_archive_worker(&mut app);
        assert!(app.archive_warnings.iter().any(|w| w.contains("A.bsa")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cold_archive_worker_honors_the_profiles_pinned_plugin_order() {
        let (mut app, root) = data_app(&[("A.esp", "plugin"), ("B.esp", "plugin")], &[]);
        let mod_root = app.mods[0].path.clone();
        for name in ["A.bsa", "B.bsa"] {
            fs::write(mod_root.join(name), archive_bsa_fixture()).unwrap();
        }
        let profile = app.created.as_ref().unwrap().active();
        fs::write(
            profile.plugins_state_dir().join("plugins.txt"),
            "*B.esp\n*A.esp\n",
        )
        .unwrap();
        profile
            .write_locked_order(&std::collections::BTreeMap::from([("a.esp".into(), 0)]))
            .unwrap();
        app.plugins = None;
        schedule_archive_conflicts(&mut app);
        finish_archive_worker(&mut app);
        let node = &app.conflicts.as_ref().unwrap().asset_files["textures/shared.dds"];
        assert_eq!(node.winner.plugin.as_deref(), Some("B.esp"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archive_analysis_is_invalidated_by_file_edits_and_both_restore_messages() {
        for action in ["rename", "mod_restore", "order_restore"] {
            let (mut app, root) = data_app(&[("A.esp", "plugin"), ("B.esp", "plugin")], &[]);
            let inst = app.created.clone().unwrap();
            let mod_root = app.mods[0].path.clone();
            let profile = inst.active();
            fs::write(mod_root.join("A.bsa"), archive_bsa_fixture()).unwrap();
            let stamp = if action == "order_restore" {
                fs::write(mod_root.join("B.bsa"), archive_bsa_fixture()).unwrap();
                fs::write(
                    profile.plugins_state_dir().join("plugins.txt"),
                    "A.esp\n*B.esp\n",
                )
                .unwrap();
                fs::write(
                    profile.plugins_state_dir().join("loadorder.txt"),
                    "A.esp\nB.esp\n",
                )
                .unwrap();
                let backup = profile
                    .create_backup(eidos_instance::BackupKind::LoadOrder)
                    .unwrap();
                fs::write(
                    profile.plugins_state_dir().join("plugins.txt"),
                    "*A.esp\nB.esp\n",
                )
                .unwrap();
                backup.stamp
            } else {
                0
            };
            if action == "mod_restore" {
                let backup = root.join("mods/AAA_backup");
                fs::create_dir_all(&backup).unwrap();
                fs::write(backup.join("B.esp"), "restored plugin").unwrap();
                fs::write(backup.join("B.bsa"), archive_bsa_fixture()).unwrap();
                reload_mods(&mut app);
            }
            app.conflicts = compute_conflicts(&app);
            schedule_archive_conflicts(&mut app);
            finish_archive_worker(&mut app);
            let epoch = app.archive_epoch.get();
            assert_eq!(
                app.conflicts.as_ref().unwrap().asset_files["textures/shared.dds"]
                    .winner
                    .plugin
                    .as_deref(),
                Some("A.esp")
            );
            app.loot_meta = Some(HashMap::new());
            match action {
                "rename" => {
                    let index = app.mods.iter().position(|m| m.name == "AAA").unwrap();
                    let _ = update_inner(
                        &mut app,
                        Message::FiletreeRenameStart(index, "A.bsa".into()),
                    );
                    let _ = update_inner(&mut app, Message::FiletreeRenameChanged("B.bsa".into()));
                    let _ = update_inner(&mut app, Message::FiletreeRenameCommit);
                }
                "mod_restore" => {
                    let _ = update_inner(
                        &mut app,
                        Message::ConfirmModRestoreBackup("AAA_backup".into()),
                    );
                }
                "order_restore" => {
                    let _ = update_inner(
                        &mut app,
                        Message::RestoreBackup(eidos_instance::BackupKind::LoadOrder, stamp),
                    );
                }
                _ => unreachable!(),
            }
            assert!(
                app.archive_epoch.get() > epoch,
                "{action}: previous worker generation must be invalidated"
            );
            assert!(
                app.loot_meta.is_none(),
                "{action}: old record validity must be discarded"
            );
            schedule_archive_conflicts(&mut app);
            finish_archive_worker(&mut app);
            assert_eq!(
                app.conflicts.as_ref().unwrap().asset_files["textures/shared.dds"]
                    .winner
                    .plugin
                    .as_deref(),
                Some("B.esp"),
                "{action}: {:?}",
                app.archive_warnings
            );
            assert!(
                app.archive_warnings.is_empty(),
                "{action}: {:?}",
                app.archive_warnings
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn archive_worker_discards_stale_results_and_honors_whiteouts_and_opacity() {
        let (mut app, root) = data_app(
            &[("A.esp", "plugin"), ("textures/shared.dds", "lower")],
            &[
                (".eidoswh.A.bsa", ""),
                ("textures/.eidoswh_opaque", ""),
                ("textures/shared.dds", "upper"),
            ],
        );
        fs::write(app.mods[0].path.join("A.bsa"), archive_bsa_fixture()).unwrap();
        let mut plugins = PluginList::default();
        plugins.plugins = vec![plugin_row("A.esp", "AAA")];
        plugins.plugins[0].enabled = true;
        app.plugins = Some(plugins);
        schedule_archive_conflicts(&mut app);
        bump_views(&app);
        finish_archive_worker(&mut app);
        assert!(
            app.conflicts.is_none(),
            "old generation must never install its winners"
        );
        schedule_archive_conflicts(&mut app);
        finish_archive_worker(&mut app);
        let map = app.conflicts.as_ref().unwrap();
        assert!(!map.files.contains_key("a.bsa"));
        assert_eq!(map.files["textures/shared.dds"].winner, u32::MAX);
        assert!(map.files["textures/shared.dds"].alternatives.is_empty());
        assert!(map.asset_files["textures/shared.dds"]
            .winner
            .archive
            .is_none());
        assert!(archive_rows(&app, "skyrimse")
            .unwrap()
            .iter()
            .all(|r| !r.loaded()));
        fs::remove_dir_all(root).unwrap();
    }
}
