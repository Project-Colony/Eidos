//! The message handler: every `Message` the GUI can receive, and what it does to
//! `App`.
//!
//! This is the half of the program that DECIDES; everything under `view` only
//! draws. [`update_inner`] does the work every message shares, then routes each
//! message to the file for its group (`mod_list`, `plugins`, `install`, ...).
//! The route is one exhaustive `match`, so a new message that no group handles
//! does not compile.

use crate::*;

mod categories;
mod downloads;
mod files;
mod install;
mod instances;
mod launch;
mod mod_info;
mod mod_list;
mod nexus;
mod plugins;
mod preferences;
mod profiles;
mod saves;
mod tools;
mod window;
mod wizard;

/// Open `archive` on a worker thread, reporting 7-Zip's percentage as it goes.
///
/// Everything here used to happen inline in `update()`. The work is identical;
/// what changed is that the window is free to repaint and answer the compositor
/// while it runs.
fn spawn_open(
    archive: std::path::PathBuf,
    mods_dir: std::path::PathBuf,
    name: String,
    fresh: bool,
    target: Option<CollectionTarget>,
    game: DetectedGame,
    addons: Vec<eidos_addons::Addon>,
) -> crate::InstallJob {
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::{Arc, Mutex};
    let game_id = game.def.id.to_string();
    let percent = Arc::new(AtomicU8::new(0));
    let outcome = Arc::new(Mutex::new(None));
    let done = Arc::new(AtomicBool::new(false));
    let (p, o, d) = (percent.clone(), outcome.clone(), done.clone());
    let (a, m, n, g) = (
        archive.clone(),
        mods_dir.clone(),
        name.clone(),
        game_id.clone(),
    );
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let worker_target = target.clone();
    std::thread::spawn(move || {
        let r = (|| {
            let target = worker_target.ok_or_else(|| {
                eidos_install::InstallError::BadSelection("No installation selected".into())
            })?;
            let inst = Instance::portable(target.instance);
            let cancel = worker_cancel;
            let folder = m.join(&n);
            let meta = eidos_instance::ModMeta::read(&folder.join("meta.ini"));
            let answers = if fresh {
                None
            } else {
                meta.collection_owner()
                    .map(|owner| eidos_collections::installer_answers::read(&folder, owner))
                    .transpose()
                    .map_err(eidos_install::InstallError::BadSelection)?
                    .flatten()
            };
            let receipt = match answers {
                Some(eidos_collections::installer_answers::InstallerAnswers::Custom {
                    receipt,
                    ..
                }) => Some(receipt),
                _ if !fresh => eidos_install::custom::read_installed_receipt(&folder)?,
                _ => None,
            };
            if let Some(session) = eidos_install::try_open_omod_with(&a, &m, &cancel, |pct| {
                p.store(pct, Ordering::SeqCst)
            })? {
                if receipt.is_some() {
                    return Err(eidos_install::InstallError::BadSelection(
                        "Recorded custom installer no longer matches the archive format".into(),
                    ));
                }
                return Ok(eidos_install::Opened::Omod(Box::new(session)));
            }
            let context = if receipt.is_some() || eidos_install::custom::may_handle(&addons, &g, &a)
            {
                eidos_install::custom::context_for_instance(
                    &inst,
                    &g,
                    &game.install_path,
                    &game.data_path,
                    game.prefix().as_deref(),
                    &crate::installers::control_dirs(&game),
                    &cancel,
                )?
            } else {
                eidos_addons::Context::default()
            };
            eidos_install::open_archive_with_installers(
                &a,
                &m,
                &n,
                &g,
                &addons,
                &context,
                &cancel,
                receipt.as_ref(),
                |pct| {
                    p.store(pct, Ordering::SeqCst);
                },
            )
        })();
        if let Ok(mut slot) = o.lock() {
            *slot = Some(r);
        }
        // Last, and after the outcome is stored: the poller reads `done` first.
        d.store(true, Ordering::SeqCst);
    });
    crate::InstallJob {
        fresh,
        cancel,
        target,
        name,
        archive,
        percent,
        outcome,
        done,
    }
}

fn pick_mod_archive(app: &mut App, name: Option<String>, fresh: bool) -> Task<Message> {
    app.mod_picker = app.mod_picker.wrapping_add(1);
    let request = app.mod_picker;
    let target = collection_target(app);
    Task::perform(
        rfd::AsyncFileDialog::new()
            .add_filter("Mod archives", &["7z", "zip", "rar", "omod"])
            .set_title("Select a mod archive to install")
            .pick_file(),
        move |handle| Message::ModPickedFor {
            fresh,
            request,
            target: target.clone(),
            name: name.clone(),
            path: handle.map(|h| h.path().to_path_buf()),
        },
    )
}
pub(crate) fn open_mod_archive(
    app: &mut App,
    path: PathBuf,
    name: String,
    fresh: bool,
) -> Task<Message> {
    let (Some(game), Some(inst)) = (selected_game(app).cloned(), app.created.as_ref()) else {
        return Task::none();
    };
    app.install_job = Some(spawn_open(
        path,
        inst.mods_dir(),
        name,
        fresh,
        collection_target(app),
        game,
        app.addons.clone(),
    ));
    Task::none()
}

/// Pack the whole instance into one file, on a worker thread.
///
/// Everything the ORDER of matters is in `pack_on_worker`; this is the same
/// three-Arc handshake `spawn_open` uses, with `done` stored last so the poller
/// cannot see a finished job whose result is not there yet.
fn spawn_pack(
    inst: Instance,
    dest: std::path::PathBuf,
    opt: eidos_transfer::Options,
) -> crate::TransferJob {
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::{Arc, Mutex};
    let percent = Arc::new(AtomicU8::new(0));
    let outcome = Arc::new(Mutex::new(None));
    let done = Arc::new(AtomicBool::new(false));
    let (p, o, d) = (percent.clone(), outcome.clone(), done.clone());
    let title = format!(
        "Packing {}",
        inst.root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "this instance".to_string())
    );
    let target = dest.clone();
    std::thread::spawn(move || {
        let r = pack_on_worker(&inst, &dest, opt, &p);
        if let Ok(mut slot) = o.lock() {
            *slot = Some(r);
        }
        d.store(true, Ordering::SeqCst);
    });
    crate::TransferJob {
        kind: crate::TransferKind::Pack,
        title,
        target,
        percent,
        outcome,
        done,
        finished: None,
    }
}

/// The pack itself, in the order the order matters.
fn pack_on_worker(
    inst: &Instance,
    dest: &std::path::Path,
    opt: eidos_transfer::Options,
    percent: &std::sync::Arc<std::sync::atomic::AtomicU8>,
) -> Result<String, String> {
    use std::sync::atomic::Ordering;
    let started = std::time::Instant::now();
    // 7-Zip FIRST. Finding it costs up to three fork+exec, and a fork briefly
    // shares every open descriptor - the instance lock included - so a probe
    // made while holding that lock can be refused by our own child.
    let transfer = eidos_transfer::Transfer::new(opt).map_err(|e| e.to_string())?;
    // And the lock HERE, on the thread that will drop it. It is keyed by
    // ThreadId: one taken on the window's thread and dropped on this one would
    // leave its entry in the table and wedge the instance for the rest of the
    // session, reported to the user as "in use by the Eidos window" - by the
    // Eidos window.
    let _lock = inst
        .try_lock("the Eidos window (packing)")
        .map_err(|e| format!("Cannot pack now: {e}."))?;
    // Walked again, under the lock, rather than trusting the dialog's preview:
    // the window kept running the whole time the file picker was up.
    let plan = eidos_transfer::plan(inst, &opt);
    transfer
        .check_pack(inst, &plan, dest)
        .map_err(|e| e.to_string())?;
    let report = transfer
        .pack(inst, &plan, dest, &mut |pct| {
            // Clamped to its own maximum HERE rather than in the view. 7-Zip's
            // percentage is not monotonic and a pack makes two passes into one
            // archive, so the raw value climbs, resets and climbs again - and
            // the view is asked to draw it sixty times a second.
            percent.fetch_max(pct, Ordering::SeqCst);
        })
        .map_err(|e| e.to_string())?;
    let ratio = report
        .percent_of_source()
        .map(|p| {
            format!(
                ", {p:.0}% of {}",
                eidos_transfer::human_bytes(report.source_bytes)
            )
        })
        .unwrap_or_default();
    let head = format!(
        "Packed {} entries into {} ({}{ratio}) in {}.",
        report.entries,
        report.path.display(),
        eidos_transfer::human_bytes(report.bytes),
        eidos_transfer::human_secs(started.elapsed().as_secs())
    );
    if report.warnings.is_empty() {
        return Ok(head);
    }
    // An incomplete backup is not a success, however real the file is. The Err
    // side is what the card draws as something to read rather than something to
    // dismiss - the same call `eidos pack` makes when it exits 1.
    Err(format!(
        "{head}\n\n{}\n\nThe backup was written but it is NOT complete.",
        report.warnings.join("\n")
    ))
}

fn collection_runtime_on_worker(
    inst: &Instance,
    game: &eidos_games::DetectedGame,
    link: &str,
) -> Result<eidos_collections::recipe::RuntimeCheck, String> {
    let nexus = eidos_nexus::Nexus::connect()?;
    let want = match eidos_nexus::NxmLink::parse(link) {
        Ok(eidos_nexus::NxmLink::Collection(value)) => value,
        _ => return Err("That is not a collection link.".into()),
    };
    let revision = nexus.collection_revision(&want)?;
    if !revision.visible() {
        return Err("This collection is not visible to this account.".into());
    }
    if !revision
        .game_domain
        .eq_ignore_ascii_case(game.def.nexus_game)
    {
        return Err("The collection belongs to another game.".into());
    }
    let _lock = inst
        .try_lock("checking a collection recipe")
        .map_err(|e| e.to_string())?;
    let dir = eidos_collections::state::revision_dir(
        &inst.root,
        &revision.slug,
        revision.revision_number,
    );
    let manifest = eidos_collections::driver::fetch_manifest(&nexus, &revision, &dir)?;
    let read = eidos_collections::read(&manifest)?;
    Ok(eidos_collections::recipe::runtime_check(
        &read.collection,
        game,
    ))
}

pub(crate) fn collection_target(app: &App) -> Option<CollectionTarget> {
    collection_target_for(app.created.as_ref()?, selected_game(app)?, &app.games)
}

pub(crate) fn collection_target_for(
    instance: &Instance,
    game: &DetectedGame,
    candidates: &[DetectedGame],
) -> Option<CollectionTarget> {
    let index = instance_game_index(candidates, instance, game.def.id)?;
    if candidates[index].selection_id() != game.selection_id() {
        return None;
    }
    Some(CollectionTarget {
        instance: instance.root.clone(),
        profile: instance.active_profile(),
        installation: game.selection_id(),
    })
}

/// Install a Nexus collection on a worker thread.
///
/// The same three-Arc handshake as the other two long jobs, and the same reason:
/// a 200-mod collection is hours, and doing it inside `update()` would be hours
/// of a window the compositor calls dead.
fn spawn_collection(
    inst: Instance,
    game: eidos_games::DetectedGame,
    game_id: String,
    link: String,
    approved: Option<eidos_collections::recipe::RuntimeCheck>,
) -> crate::TransferJob {
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::{Arc, Mutex};
    let percent = Arc::new(AtomicU8::new(0));
    let outcome = Arc::new(Mutex::new(None));
    let done = Arc::new(AtomicBool::new(false));
    let (p, o, d) = (percent.clone(), outcome.clone(), done.clone());
    let target = inst.root.clone();
    let profile = inst.active_profile();
    std::thread::spawn(move || {
        let r = collection_on_worker(
            &inst,
            &game,
            &game_id,
            &link,
            &profile,
            approved.as_ref(),
            &p,
        );
        if let Ok(mut slot) = o.lock() {
            *slot = Some(r);
        }
        d.store(true, Ordering::SeqCst);
    });
    crate::TransferJob {
        kind: crate::TransferKind::Collection,
        title: "Installing a collection".to_string(),
        target,
        percent,
        outcome,
        done,
        finished: None,
    }
}

fn collection_on_worker(
    inst: &Instance,
    game: &eidos_games::DetectedGame,
    game_id: &str,
    link: &str,
    profile: &str,
    approved: Option<&eidos_collections::recipe::RuntimeCheck>,
    percent: &std::sync::Arc<std::sync::atomic::AtomicU8>,
) -> Result<String, String> {
    use std::sync::atomic::Ordering;
    let nexus = eidos_nexus::Nexus::connect()?;
    let want = match eidos_nexus::NxmLink::parse(link) {
        Ok(eidos_nexus::NxmLink::Collection(c)) => c,
        _ => return Err("that is not a collection link".to_string()),
    };
    let rev = nexus.collection_revision(&want)?;
    // The lock is taken HERE, on the thread that will drop it: it is keyed by
    // ThreadId, so one taken on the window's thread would leak its entry and
    // wedge the instance for the rest of the session.
    let _lock = inst
        .try_lock("the Eidos window (installing a collection)")
        .map_err(|e| format!("Cannot install now: {e}."))?;
    if inst.active_profile() != profile {
        return Err("The active profile changed before collection installation started; retry from that profile.".into());
    }
    let dir = eidos_collections::state::revision_dir(&inst.root, &rev.slug, rev.revision_number);
    let manifest = eidos_collections::driver::fetch_manifest(&nexus, &rev, &dir)?;
    let mut read = eidos_collections::read(&manifest)?;
    if read.collection.info.domain_name.trim().is_empty() {
        read.collection.info.domain_name = rev.game_domain.clone();
    }
    let c = &read.collection;

    let state_path =
        eidos_collections::state::InstallState::path(&inst.root, &rev.slug, rev.revision_number);
    // An unreadable state file is NOT a first run: starting over would download
    // everything again and wipe-reinstall every member, so it stops here.
    let mut state = match eidos_collections::state::InstallState::load(&state_path)? {
        Some(s) => s,
        None => eidos_collections::state::InstallState {
            slug: rev.slug.clone(),
            revision: rev.revision_number,
            game_domain: rev.game_domain.clone(),
            ..Default::default()
        },
    };

    state.validate_revision(&rev.slug, rev.revision_number, &rev.game_domain)?;

    let total = c.mods.len().max(1);
    let mut say = |_line: String| {};
    let mut hooks = eidos_collections::driver::RealHooks {
        nexus: &nexus,
        inst,
        game,
        game_id: game_id.to_string(),
        payload_root: dir.clone(),
        allow_runtime_mismatch: approved
            == Some(&eidos_collections::recipe::runtime_check(c, game)),
        say: &mut say,
        collection_domain: c.info.domain_name.clone(),
        owner: format!("{}:{}:{}", state.game_domain, state.slug, state.revision),
        renamed: Vec::new(),
        kept: Vec::new(),
    };
    // The bar counts members reached, which is the only honest measure here: a
    // collection that is half skipped still moves, and 7-Zip's own percentage
    // would restart on every single member.
    let seen = std::sync::atomic::AtomicUsize::new(0);
    let mut counting = CountingHooks {
        inner: &mut hooks,
        seen: &seen,
        total,
        percent,
    };
    let mut save =
        |s: &eidos_collections::state::InstallState| s.save(&state_path).map_err(|e| e.to_string());
    let mut report = eidos_collections::install::run(c, &mut state, &mut counting, &mut save);
    report.unknown_sections = read.unknown_sections.clone();
    hooks.drain_notes(&mut report);
    if report.aborted {
        return Err(report.render());
    }
    eidos_collections::driver::apply_ordering(inst, c, &state, &mut report);
    eidos_collections::driver::apply_plugin_states(inst, game, c, &mut report);
    eidos_collections::driver::apply_plugin_rules(inst, game, c, &mut report);
    eidos_collections::driver::apply_ini_tweaks(inst, &dir, c, &mut state, &mut save, &mut report);
    percent.store(100, Ordering::SeqCst);

    let text = report.render();
    // A collection that is not complete is not an error - it is waiting for the
    // user to fetch what a free account cannot. But it must not read as success.
    if report.is_complete() && report.is_faithful() {
        Ok(text)
    } else {
        Err(text)
    }
}

/// Wraps the real hooks to drive the progress bar off members reached.
struct CountingHooks<'a> {
    inner: &'a mut dyn eidos_collections::install::Hooks,
    seen: &'a std::sync::atomic::AtomicUsize,
    total: usize,
    percent: &'a std::sync::Arc<std::sync::atomic::AtomicU8>,
}

impl eidos_collections::install::Hooks for CountingHooks<'_> {
    fn validate_recipe(&mut self, c: &eidos_collections::Collection) -> Result<(), String> {
        self.inner.validate_recipe(c)
    }
    fn runtime_check(
        &mut self,
        c: &eidos_collections::Collection,
    ) -> eidos_collections::recipe::RuntimeCheck {
        self.inner.runtime_check(c)
    }
    fn allow_runtime_mismatch(&self) -> bool {
        self.inner.allow_runtime_mismatch()
    }
    fn adopt(
        &mut self,
        c: &eidos_collections::Collection,
        state: &mut eidos_collections::state::InstallState,
        save: &mut dyn FnMut(&eidos_collections::state::InstallState) -> Result<(), String>,
    ) -> Result<Vec<eidos_collections::report::Note>, String> {
        self.inner.adopt(c, state, save)
    }

    fn obtain(
        &mut self,
        m: &eidos_collections::manifest::Mod,
    ) -> eidos_collections::install::Obtained {
        self.inner.obtain(m)
    }
    fn reserve(
        &mut self,
        m: &eidos_collections::manifest::Mod,
        previous: Option<&str>,
    ) -> Result<String, String> {
        self.inner.reserve(m, previous)
    }
    fn verify_installed(
        &mut self,
        m: &eidos_collections::manifest::Mod,
        folder: &str,
    ) -> Result<bool, String> {
        self.inner.verify_installed(m, folder)
    }
    fn recover_installed(
        &mut self,
        m: &eidos_collections::manifest::Mod,
        folder: &str,
    ) -> Result<Option<eidos_collections::install::Installed>, String> {
        self.inner.recover_installed(m, folder)
    }
    fn install(
        &mut self,
        m: &eidos_collections::manifest::Mod,
        a: &std::path::Path,
        folder: &str,
    ) -> eidos_collections::install::Installed {
        self.inner.install(m, a, folder)
    }
    fn progress(&mut self, done: usize, total: usize, member: &str) {
        use std::sync::atomic::Ordering;
        self.seen.store(done, Ordering::SeqCst);
        let pct = ((done * 100) / self.total.max(1)).min(99) as u8;
        self.percent.fetch_max(pct, Ordering::SeqCst);
        self.inner.progress(done, total, member);
    }
}

/// Put a backup back, on a worker thread.
fn spawn_unpack(
    archive: std::path::PathBuf,
    dest: std::path::PathBuf,
    opt: eidos_transfer::Options,
) -> crate::TransferJob {
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::{Arc, Mutex};
    let percent = Arc::new(AtomicU8::new(0));
    let outcome = Arc::new(Mutex::new(None));
    let done = Arc::new(AtomicBool::new(false));
    let (p, o, d) = (percent.clone(), outcome.clone(), done.clone());
    let title = format!(
        "Unpacking {}",
        archive
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "the backup".to_string())
    );
    let target = dest.clone();
    std::thread::spawn(move || {
        let r = unpack_on_worker(&archive, &dest, opt, &p);
        if let Ok(mut slot) = o.lock() {
            *slot = Some(r);
        }
        d.store(true, Ordering::SeqCst);
    });
    crate::TransferJob {
        kind: crate::TransferKind::Unpack,
        title,
        target,
        percent,
        outcome,
        done,
        finished: None,
    }
}

fn unpack_on_worker(
    archive: &std::path::Path,
    dest: &std::path::Path,
    opt: eidos_transfer::Options,
    percent: &std::sync::Arc<std::sync::atomic::AtomicU8>,
) -> Result<String, String> {
    use std::sync::atomic::Ordering;
    let started = std::time::Instant::now();
    let transfer = eidos_transfer::Transfer::new(opt).map_err(|e| e.to_string())?;
    // Only when the folder is already there: nothing can be holding one that
    // does not exist, and `try_lock` CREATES what it locks, so asking first
    // would conjure the folder it was checking. Taken on this thread, dropped
    // on this thread, for the reason `pack_on_worker` gives.
    let _lock = if dest.is_dir() {
        Some(
            Instance::portable(dest.to_path_buf())
                .try_lock("the Eidos window (unpacking)")
                .map_err(|e| format!("Cannot unpack there: {e}."))?,
        )
    } else {
        None
    };
    let report = transfer
        .unpack(archive, dest, &mut |pct| {
            percent.fetch_max(pct, Ordering::SeqCst);
        })
        .map_err(|e| e.to_string())?;
    let mut lines = vec![format!(
        "Unpacked {} entries into {} in {}.",
        report.entries,
        report.root.display(),
        eidos_transfer::human_secs(started.elapsed().as_secs())
    )];
    if report.relocated.values > 0 {
        lines.push(format!(
            "{} path(s) in {} file(s) now point here instead of {}.",
            report.relocated.values,
            report.relocated.files.len(),
            report.manifest.source_root
        ));
    }
    if !report.missing_tools.is_empty() {
        lines.push(String::new());
        lines.push("Tools this instance uses that are not on this machine:".to_string());
        for t in &report.missing_tools {
            lines.push(format!("  {} - was at {}", t.title, t.exe));
        }
    }
    for w in &report.warnings {
        lines.push(String::new());
        lines.push(w.clone());
    }
    Ok(lines.join("\n"))
}

/// Make a just-unpacked folder the open instance, or say why it cannot be.
///
/// Returns the sentence to append to the report, if there is one. The window
/// cannot open an instance for a game it has not detected - `known_instances`
/// lists only those - and on the fresh machine this feature exists for, the
/// game arriving after the mods is the ordinary case, not the odd one. Saying
/// so is the whole difference between "nothing happened" and "install Skyrim".
fn open_unpacked(app: &mut App, root: &std::path::Path) -> Option<String> {
    let inst = Instance::portable(root.to_path_buf());
    let gid = inst.game_id()?;
    let Some(idx) = instance_game_index(&app.games, &inst, &gid) else {
        return Some(format!(
            "\n{gid} is not installed on this machine yet, so Eidos cannot open the \
             instance. Restore its selected game installation, then start Eidos again."
        ));
    };
    app.selected = Some(idx);
    let id = app.games[idx].def.id;
    let _ = inst.ensure_profiles();
    remember_open(&inst, id);
    open_instance(app, inst);
    // The welcome list is built once at startup and once on Restart; without
    // this the instance just restored is not among the ones offered.
    app.known = known_instances(&app.games);
    // Said out loud, because it is a real consequence nobody was told about:
    // opening an instance makes it the one Eidos comes back to, which is also
    // the one a Steam launch lands on. That is right when the restore IS the
    // move; it is a surprise when somebody only wanted a look inside a backup,
    // and they find out by launching the game into the wrong mod list.
    Some(
        "\nThis is now the instance Eidos opens, and the one a Steam launch will \
         use. Switch with \"New instance\" if you meant to keep using another."
            .to_string(),
    )
}

/// Pin an ordinary install to its original target under the publication lock.
pub(crate) fn lock_install_target(
    app: &App,
    target: &CollectionTarget,
) -> Result<eidos_instance::InstanceLock, String> {
    let changed = "The installation or profile changed; reopen the installer.";
    let instance = app.created.as_ref().ok_or(changed)?;
    if instance.root != target.instance {
        return Err(changed.into());
    }
    let guard = instance
        .try_lock("installing a mod")
        .map_err(|error| format!("Cannot install now: {error}"))?;
    if collection_target(app).as_ref() != Some(target) {
        return Err(changed.into());
    }
    Ok(guard)
}

pub(crate) fn finish_open(
    app: &mut App,
    result: Result<eidos_install::Opened, eidos_install::InstallError>,
    path: std::path::PathBuf,
    name: String,
    target: CollectionTarget,
    fresh: bool,
) -> Task<Message> {
    let _guard = match lock_install_target(app, &target) {
        Ok(guard) => guard,
        Err(error) => {
            app.status = Some(error);
            return Task::none();
        }
    };
    let mods_dir = app
        .created
        .as_ref()
        .expect("validated install target")
        .mods_dir();
    let gid = selected_game(app)
        .expect("validated installation")
        .def
        .id
        .to_owned();
    match result {
        Ok(eidos_install::Opened::Custom(session)) => {
            return crate::installers::begin(
                app,
                eidos_install::Opened::Custom(session),
                path,
                name,
                fresh,
            )
        }
        Ok(eidos_install::Opened::Omod(session)) if session.script.is_some() => {
            return crate::installers::begin(
                app,
                eidos_install::Opened::Omod(session),
                path,
                name,
                fresh,
            )
        }
        Ok(eidos_install::Opened::Omod(session)) => {
            let result = eidos_install::install_omod(
                &session,
                &mods_dir,
                &name,
                &gid,
                eidos_install::OverwritePolicy::Fail,
            );
            match result {
                Ok(report) => after_install_report(app, report, &path),
                Err(eidos_install::InstallError::Exists(_)) => {
                    app.collision = Some(CollisionPrompt {
                        target: target.clone(),
                        backup: app.prefs.retain_install_backup,
                        rename_to: suggest_free_name(&mods_dir, &name),
                        archive: path,
                        name: name.clone(),
                        game_id: gid,
                        fomod: false,
                        tree: None,
                        pick: None,
                    });
                    app.status = Some(format!("'{name}' already exists - choose how to install."));
                }
                Err(error) => app.status = Some(format!("Cannot install OMOD: {error}")),
            }
        }
        Ok(eidos_install::Opened::Fomod(session)) => {
            let ctx = current_fomod_context(app);
            let session = *session;
            // MO2 refuses a FOMOD whose <moduleDependencies> are unmet before
            // showing the wizard - tell the user what is missing and stop.
            if let Some(req) = session.unmet_dependencies(&ctx) {
                app.status = Some(format!("Cannot install: this mod requires {req}."));
            } else {
                let selection = eidos_fomod::default_selection(&session.config, &ctx);
                // Open on the first step that is actually shown. Next/Back
                // already skip invisible steps and build_plan ignores them,
                // but nothing seeked at open: a FOMOD whose first step is
                // conditional rendered that page fully interactive, and
                // every choice made on it was thrown away at install time.
                let first = eidos_fomod::visible_steps(&session.config, &selection, &ctx)
                    .iter()
                    .position(|v| *v)
                    .unwrap_or(0);
                app.fomod = Some(FomodWizard {
                    target: target.clone(),
                    session,
                    step: first,
                    selection,
                    game_id: gid,
                    archive: path,
                    ctx,
                    hover: None,
                });
                app.status =
                    Some("FOMOD installer: choose your options, then Install.".to_string());
            }
        }
        Ok(eidos_install::Opened::Simple(tree)) => {
            let ctx = eidos_fomod::Context::default();
            match eidos_install::install_extracted(
                &tree,
                &path,
                &mods_dir,
                &name,
                &gid,
                eidos_install::OverwritePolicy::Fail,
                &ctx,
            ) {
                Ok(r) => after_install(app, &r.name, r.dest, r.fomod, Some(&path)),
                Err(eidos_install::InstallError::Exists(_)) => {
                    // MO2's QueryOverwriteDialog: let the user Merge/Replace/
                    // Rename. The extracted tree rides along so resolving it
                    // needs no re-extract.
                    let rename_to = suggest_free_name(&mods_dir, &name);
                    app.collision = Some(CollisionPrompt {
                        target: target.clone(),
                        backup: app.prefs.retain_install_backup,
                        archive: path,
                        name: name.clone(),
                        game_id: gid,
                        rename_to,
                        fomod: false,
                        tree: Some(tree),
                        pick: None,
                    });
                    app.status = Some(format!("'{name}' already exists - choose how to install."));
                }
                Err(e) => app.status = Some(format!("Install failed: {e}")),
            }
        }
        // Wrye Bash complex package: let the user tick sub-packages. MO2
        // pre-ticks the `00`-prefixed ones plus whatever the last install
        // of this mod used, which its meta.ini remembers.
        Ok(eidos_install::Opened::Bain {
            tree,
            subpackages,
            invalid,
        }) => {
            let previous = app
                .created
                .as_ref()
                .map(|i| i.mod_meta(&name).bain_options().to_vec())
                .unwrap_or_default();
            let picked = eidos_install::bain_default_selection(&subpackages, &previous);
            app.status = Some(if invalid > 0 {
                format!("'{name}' may be a BAIN installer - {invalid} folder(s) do not look like sub-packages.")
            } else {
                format!("BAIN installer: choose the sub-packages to install for '{name}'.")
            });
            let archive_tree = parsed_tree(&tree);
            app.picker = Some(InstallPicker {
                target: target.clone(),
                rows: archive_tree.flatten(),
                archive_tree,
                archive: path,
                name,
                game_id: gid,
                tree,
                // `invalid` folders are MO2's cue to ASK rather than assume.
                mode: PickerMode::Bain {
                    subpackages,
                    picked,
                    asking: invalid > 0,
                },
            });
        }
        // No heuristic recognised the layout. Rather than refuse the
        // archive, show its tree and let the user point at the data root.
        Ok(eidos_install::Opened::Manual(tree)) => {
            app.status = Some(format!(
                "'{name}': pick the folder that holds the game data."
            ));
            let archive_tree = parsed_tree(&tree);
            app.picker = Some(InstallPicker {
                target: target.clone(),
                rows: archive_tree.flatten(),
                archive_tree,
                archive: path,
                name,
                game_id: gid,
                tree,
                mode: PickerMode::Manual {
                    root: String::new(),
                },
            });
        }
        Err(e) => app.status = Some(format!("Install failed: {e}")),
    }
    Task::none()
}

/// Whether any tool declares a runtime that is not downloaded yet.
///
/// Asked before refusing Tool Setup for want of a Proton prefix: a runtime is a
/// directory and an environment variable, so it can be fetched on a machine that
/// has never launched the game.
fn any_runtime_pending(app: &App) -> bool {
    app.created
        .as_ref()
        .map(|i| eidos_instance::read_tools(&i.root.join("tools.ini")))
        .unwrap_or_default()
        .iter()
        .flat_map(|t| t.prereqs.iter())
        .any(|v| {
            eidos_gamefeatures::runtime(v)
                .is_some_and(|r| !eidos_gamefeatures::runtime_is_installed(r))
        })
}

/// The iced entry point: run the handler, then bring the once-per-change caches
/// back in step.
///
/// A wrapper rather than a line at the end of `update_inner`, because that
/// function has 68 early returns and a refresh reachable from only some of them
/// is worse than none - the tab count would be right or wrong depending on which
/// branch ran.
pub(crate) fn update(app: &mut App, message: Message) -> Task<Message> {
    // Whether we were mid-drain BEFORE the message ran: a `DrainDrops` that
    // popped the last item must not re-arm itself off its own empty queue.
    let draining = !app.dropped.is_empty();
    let task = update_inner(app, message);
    schedule_archive_conflicts(app);
    refresh_diagnostics(app);
    // A new status message fades in. Detected by comparison HERE rather than by
    // starting the phase at each assignment: `app.status` is written from about
    // a dozen places across `state.rs`, and "remember to also start the fade" is
    // a rule that would be forgotten by the second one. The comparison is a
    // string compare per message; the clone happens only when it actually
    // changed, which is also the only time anything is drawn differently.
    if app.status != app.status_shown {
        app.status_shown = app.status.clone();
        app.status_anim.start();
    }
    // A multi-file drop is walked one file at a time, because each install can
    // open a modal that has to be answered before the next archive is touched.
    // Re-armed here rather than at the end of every install path, because the
    // ones that matter - FOMOD, the manual picker, the collision prompt - finish
    // in six different places and would each have to remember.
    if draining
        && !app.dropped.is_empty()
        && app.installer.is_none()
        && app.fomod.is_none()
        && app.picker.is_none()
        && app.collision.is_none()
        // Re-armed by the poll that finishes the job, not while it runs.
        && app.install_job.is_none()
        // And not while a pack is reading the instance: a queued drop would
        // install a mod into the very tree being archived, halfway through.
        && app.transfer_job.is_none()
    {
        return Task::batch([task, Task::done(Message::DrainDrops)]);
    }
    task
}

/// Whether a message reports where the pointer or the window IS, rather than
/// something the user DID.
///
/// The distinction decides whether a two-click confirmation survives. Arming
/// Delete and then moving the mouse a single pixel used to cancel it, because
/// pointer tracking publishes a message per `CursorMoved` and every disarm rule
/// read that as an action. The confirmation was unreachable in practice: the
/// pointer has to travel to the button, and travelling is a mouse move.
pub(crate) fn is_ambient(app: &App, m: &Message) -> bool {
    match m {
        Message::PointerAt(_)
        | Message::WindowResized(_)
        // Grabbing the divider resizes the window's furniture; it is not the
        // user deciding anything about the row a confirmation is armed on.
        | Message::SplitGrab
        | Message::FomodHover(_)
        | Message::FomodUnhover(..)
        // The downloads tick fires twice a second on its own. Left out of
        // this list it would disarm every confirmation before the second
        // click could land - the same defect as the pointer, on a timer.
        | Message::DownloadTick
        // Same for every other timer: the saves watcher (2.5s while its tab is
        // open) and the log tail (1.5s while the pane is). Both are the program
        // looking at a directory, not the user deciding anything - and left out
        // of this list they cancel every armed confirmation before the second
        // click can land, which is exactly what the note above describes.
        | Message::SavesTick
        | Message::LogRefresh
        // And the frame timer, for exactly the same reason and more sharply:
        // it fires SIXTY times a second, so leaving it out would not merely
        // shorten a confirmation's life, it would end it before the finger
        // came back down.
        | Message::AnimationTick
        | Message::ArchivePoll
        // Reached from that same tick, at the same rate, for the same reason.
        | Message::InstallPoll
        // And its twin for a pack or an unpack, which runs for twenty minutes -
        // long enough that a confirmation disarmed sixty times a second would
        // look like a window that had simply stopped working.
        | Message::TransferPoll
        | Message::PreviewReady(..)
        | Message::ArchiveExportFinished(..)
        | Message::CollectionInstallChecked { .. }
        // And the hover-to-expand timer, which fires only while a drag rests on
        // a collapsed group. Same reason: the program watching a pointer sit
        // still is not the user deciding anything.
        | Message::DragHoverTick
        // Ctrl is how a multi-selection is built, so pressing or releasing it
        // around a batch action belongs to that gesture. The handler only
        // stores the modifier set - it changes nothing the user can see.
        | Message::ModifiersChanged(_)
        // Cancelling a drag is the absence of an action: nothing moved, nothing
        // was written. These arrive from the release path above, so exempting
        // PointerReleased alone fixed nothing - it simply handed the job to the
        // two messages it emits.
        | Message::DragCancel
        | Message::PluginDragCancel
        | Message::DownloadDragCancel
        // A file merely hovering over the window is the mouse being somewhere,
        // not a decision. (Never fires on Wayland - see the subscription.)
        | Message::FilesHovering(_) => true,
        // Letting go of the very click that armed a confirmation is the END of
        // that click, not a new decision - and EVERY left release publishes
        // this, so treating it as an action made the confirmation live exactly
        // as long as the button was held down. The pointer fix covered moving
        // and missed this, which is why the bug came back wearing a hat.
        //
        // Not blanket, though: a release that commits an aimed drag really is
        // an action, and must cancel like any other.
        Message::PointerReleased => {
            !(app.drag_state.is_some_and(|d| d.aimed)
                || app.plugin_drag.as_ref().is_some_and(|d| d.aimed)
                || app.download_drag.as_ref().is_some_and(|d| d.aimed))
        }
        _ => false,
    }
}

pub(crate) fn update_inner(app: &mut App, message: Message) -> Task<Message> {
    if let Message::Installer(action) = message {
        return crate::installers::update(app, action);
    }
    if app.installer.is_some()
        && !is_ambient(app, &message)
        && !matches!(
            message,
            Message::ClosePreview
                | Message::WindowResized(_)
                | Message::Noop
                | Message::PreviewNifView(_)
                | Message::PreviewDdsSelection(_)
        )
    {
        return Task::none();
    }
    // A confirmation is armed by the first click and cancelled by any other
    // ACTION - including arming a different row. Ambient messages are not
    // actions and must leave it standing.
    if !is_ambient(app, &message) {
        // Any action other than a second Clear click cancels the clear confirmation.
        if !matches!(message, Message::ClearOverwrite) {
            app.confirm_clear = false;
        }
        if !matches!(
            message,
            Message::DeleteSave(_) | Message::ConfirmDeleteSave(_)
        ) {
            app.confirm_delete_save = None;
        }
        if !matches!(
            message,
            Message::DeleteDownload(_) | Message::ConfirmDeleteDownload(_)
        ) {
            app.confirm_delete_download = None;
        }
        // The batch-remove confirmation is armed by the first click; any other
        // action (including merely re-rendering on a modifier change) cancels it.
        if !matches!(
            message,
            Message::BatchRemoveMods | Message::ConfirmBatchRemove
        ) {
            app.confirm_batch_remove = false;
        }
        // Same rule for the bulk enable/disable: any other action disarms it.
        if !matches!(message, Message::SetAllModsEnabled(_)) {
            app.confirm_set_all = None;
        }
        if !matches!(message, Message::InstanceForget(_)) {
            app.confirm_forget = None;
        }
        if !matches!(message, Message::OverwriteSyncToMods) {
            app.confirm_sync = false;
        }
        if !matches!(message, Message::SavesDeleteSelected) {
            app.confirm_saves_delete = false;
        }
        if !matches!(
            message,
            Message::PurgeInstalledDownloads | Message::ConfirmPurgeInstalled
        ) {
            app.confirm_purge_installed = false;
        }
        if !matches!(
            message,
            Message::FiletreeDelete(..) | Message::ConfirmFiletreeDelete(..)
        ) {
            app.tree_delete_armed = None;
        }
        if !matches!(
            message,
            Message::ModRestoreBackup(_) | Message::ConfirmModRestoreBackup(_)
        ) {
            app.confirm_restore = None;
        }
        if !matches!(message, Message::CollectionFetchMissing) {
            if let Some(c) = app.collection.as_mut() {
                c.confirm_fetch = false;
            }
        }
    }
    // Anything that can create, rename, delete or switch a profile drops the
    // chip row's memo. Done centrally rather than in each handler: those have
    // many branches, only some of them bump the view generation, and a stale
    // chip row would keep offering a profile that no longer exists.
    if matches!(
        message,
        Message::NewProfile
            | Message::SwitchProfile(_)
            | Message::ProfileRenameCommit
            | Message::ProfileCopyCommit
            | Message::ProfileDeleteCommit(_)
    ) {
        app.profiles_cache.borrow_mut().take();
    }
    match message {
        Message::Installer(_) => unreachable!("handled before dispatch"),
        m @ (Message::Next
        | Message::Back
        | Message::PickKind(..)
        | Message::PickGame(..)
        | Message::NameChanged(..)
        | Message::PortableChanged(..)
        | Message::Finish
        | Message::OpenKnown(..)
        | Message::Restart) => wizard::handle(app, m),
        m @ (Message::SwitchProfile(..)
        | Message::NewProfile
        | Message::ProfileMenuOpen(..)
        | Message::ProfileCloseMenu
        | Message::ProfileRenameStart(..)
        | Message::ProfileRenameChanged(..)
        | Message::ProfileRenameCommit
        | Message::ProfileCopyStart(..)
        | Message::ProfileCopyChanged(..)
        | Message::ProfileCopyCommit
        | Message::ProfileDeleteConfirm(..)
        | Message::ProfileDeleteCommit(..)
        | Message::ShowBackupsDialog
        | Message::CloseBackupsDialog
        | Message::CreateBackup(..)
        | Message::RestoreBackup(..)) => profiles::handle(app, m),
        m @ (Message::ToggleMod(..)
        | Message::SendToFirstConflict(..)
        | Message::SendToLastConflict(..)
        | Message::SendToPriorityStart(..)
        | Message::SendToPriorityChanged(..)
        | Message::SendToPriorityCommit
        | Message::SendToSeparatorStart(..)
        | Message::SendToSeparatorPick(..)
        | Message::SendToTargetCancel
        | Message::SearchChanged(..)
        | Message::CategoryFilterChanged(..)
        | Message::SelectMod(..)
        | Message::SelectModToggle(..)
        | Message::SelectModExtend(..)
        | Message::ClearSelection
        | Message::OpenModMenu(..)
        | Message::CloseMenu
        | Message::ModSendTop(..)
        | Message::ModSendBottom(..)
        | Message::ModOpenFolder(..)
        | Message::ModVisitNexus(..)
        | Message::ModReinstallFresh(..)
        | Message::ModReinstall(..)
        | Message::ModRemove(..)
        | Message::RenameStart(..)
        | Message::RenameChanged(..)
        | Message::RenameCommit
        | Message::AddSeparator(..)
        | Message::SetSeparatorColor(..)
        | Message::ToggleCollapse(..)
        | Message::CycleFilter(..)
        | Message::ToggleFilterPane
        | Message::ClearFilters
        | Message::ModTrack(..)
        | Message::ModIgnoreUpdate(..)
        | Message::CreateEmptyMod
        | Message::CreateEmptyModAt(..)
        | Message::InstallFromFolder
        | Message::FolderPicked(..)
        | Message::ShowExportDialog
        | Message::CloseExportDialog
        | Message::ExportScopeChanged(..)
        | Message::ExportToggleColumn(..)
        | Message::ExportRun
        | Message::ExportPicked(..)
        | Message::SetAllModsEnabled(..)
        | Message::BatchToggleMods
        | Message::BatchRemoveMods
        | Message::ConfirmBatchRemove
        | Message::BatchSendTop
        | Message::BatchSendBottom
        | Message::DragStart(..)
        | Message::ModDoubleClick(..)
        | Message::ModBackup(..)
        | Message::ModRestoreBackup(..)
        | Message::ConfirmModRestoreBackup(..)
        | Message::ClearListOrder
        | Message::SetGroupBy(..)
        | Message::ToggleGroupFold(..)
        | Message::ToggleModColumn(..)
        | Message::CycleModSort(..)
        | Message::ModMarkValid(..)
        | Message::FocusFilter
        | Message::JumpToLetter(..)
        | Message::DragHoverTick
        | Message::DragOverGap(..)
        | Message::DragDrop
        | Message::DragCancel
        | Message::DragScrollEdge(..)
        | Message::DragScrollDepth(..)
        | Message::DragScrollTick) => mod_list::handle(app, m),
        m @ (Message::TogglePlugin(..)
        | Message::SortPlugins
        | Message::PluginsSorted(..)
        | Message::OpenPluginMenu(..)
        | Message::ClosePluginMenu
        | Message::OpenPluginOrigin(..)
        | Message::ShowPluginOriginInfo(..)
        | Message::PluginsSendTop
        | Message::PluginsSendBottom
        | Message::PluginSendToPriorityStart
        | Message::PluginSendToPriorityChanged(..)
        | Message::PluginSendToPriorityCommit
        | Message::PluginsSetAll(..)
        | Message::SelectPlugin(..)
        | Message::SelectPluginToggle(..)
        | Message::SelectPluginExtend(..)
        | Message::SetSelectedPluginsEnabled(..)
        | Message::PluginDragStart(..)
        | Message::PluginDragOverGap(..)
        | Message::PluginDragCancel
        | Message::PluginDragDrop
        | Message::TogglePluginLock(..)) => plugins::handle(app, m),
        m @ (Message::InstallAt(..)
        | Message::InstallerArchiveResolved { .. }
        | Message::InstallMod
        | Message::ModPickedFor { .. }
        | Message::ModPicked(..)
        | Message::PickerBainToggle(..)
        | Message::PickerBainConfirm(..)
        | Message::PickerSetRoot(..)
        | Message::PickerNameChanged(..)
        | Message::PickerInstall
        | Message::PickerCancel
        | Message::FomodToggle(..)
        | Message::FomodNext
        | Message::FomodBack
        | Message::FomodInstall
        | Message::FomodCancel
        | Message::CollisionBackupChanged(..)
        | Message::CollisionMerge
        | Message::CollisionReplace
        | Message::CollisionRenameChanged(..)
        | Message::CollisionRenameCommit
        | Message::CollisionCancel
        | Message::FilesHovering(..)
        | Message::FileDropped(..)
        | Message::DrainDrops
        | Message::DownloadDragStart(..)
        | Message::DownloadDragOverGap(..)
        | Message::DownloadDragCancel
        | Message::DownloadDragDrop
        | Message::InstallPoll
        | Message::FomodHover(..)
        | Message::FomodUnhover(..)) => install::handle(app, m),
        m @ (Message::ToolPicked(..)
        | Message::Run
        | Message::PollRunning
        | Message::ForceUnlock
        | Message::CloseLootReport
        | Message::CopyLootReport
        | Message::ChangeGame
        | Message::OpenNexusGame
        | Message::SetupPrereqs) => launch::handle(app, m),
        m @ (Message::OverwriteToModStart
        | Message::ToolOutputModChanged(..)
        | Message::OverwriteToModName(..)
        | Message::OverwriteToModCancel
        | Message::OverwriteToModCommit
        | Message::ClearOverwrite
        | Message::DataToggleDir(..)
        | Message::DataQueryChanged(..)
        | Message::DataToggleConflictsOnly
        | Message::DataExpandAll
        | Message::DataCollapseAll
        | Message::DataReveal(..)
        | Message::OverwriteToggleDir(..)
        | Message::ToggleFileHidden(..)
        | Message::ToggleIniTweak(..)
        | Message::RestoreHiddenFiles(..)
        | Message::OverwriteSyncToMods
        | Message::PreviewFile(..)
        | Message::ArchiveProviderAction { .. }
        | Message::ArchiveFilterChanged(..)
        | Message::ArchivePage(..)
        | Message::ArchiveExportDestination(..)
        | Message::ArchiveExportFinished(..)
        | Message::RunFileExtension(..)
        | Message::ChooseExtensionFile(..)
        | Message::ExtensionFilePicked { .. }
        | Message::PreviewReady(..)
        | Message::PreviewNifView(..)
        | Message::PreviewDdsSelection(..)
        | Message::ClosePreview
        | Message::FiletreeOpen(..)
        | Message::FiletreeRenameStart(..)
        | Message::FiletreeRenameChanged(..)
        | Message::FiletreeRenameCancel
        | Message::FiletreeRenameCommit
        | Message::FiletreeDelete(..)
        | Message::ConfirmFiletreeDelete(..)
        | Message::FiletreeNewFolderStart(..)
        | Message::FiletreeNewFolderChanged(..)
        | Message::FiletreeNewFolderCommit
        | Message::ArchivePoll) => files::handle(app, m),
        m @ (Message::ShowModInfo(..)
        | Message::CloseInfo
        | Message::InfoSelectTab(..)
        | Message::NotesChanged(..)
        | Message::ModUrlChanged(..)
        | Message::ModUrlSave
        | Message::NotesSave) => mod_info::handle(app, m),
        m @ (Message::ShowAddons
        | Message::CloseAddons
        | Message::ReloadAddons
        | Message::RunAddon(..)
        | Message::OpenAddonsFolder
        | Message::ShowLogPane
        | Message::CloseLogPane
        | Message::LogPick(..)
        | Message::LogLevel(..)
        | Message::LogRefresh
        | Message::LogCopy
        | Message::LogOpenFolder
        | Message::ShowIniEditor
        | Message::CloseIniEditor
        | Message::IniEditorPick(..)
        | Message::IniEditorAction(..)
        | Message::IniEditorSave
        | Message::IniEditorRevert
        | Message::IniEditorOpenExternal
        | Message::ShowExecutablesDialog
        | Message::CloseExecutablesDialog
        | Message::SelectExecutableTool(..)
        | Message::AddExecutableTool
        | Message::DeleteExecutableTool
        | Message::MoveExecutableUp
        | Message::MoveExecutableDown
        | Message::ToolTitleChanged(..)
        | Message::ToolExeChanged(..)
        | Message::ToolWorkdirChanged(..)
        | Message::ToolPrereqsChanged(..)
        | Message::BrowseToolExe
        | Message::BrowseToolWorkdir
        | Message::SaveExecutablesDialog
        | Message::ToolArgsAction(..)
        | Message::ExecAppIdChanged(..)
        | Message::ExecToggleHidden
        | Message::ExecTogglePinned
        | Message::ExecMakeShortcut) => tools::handle(app, m),
        m @ (Message::TogglePreferences
        | Message::SettingsTabSelected(..)
        | Message::NexusSignInStart
        | Message::NexusSignInResult(..)
        | Message::NexusSignOut
        | Message::Preference(..)
        | Message::DismissToast
        | Message::LanguageChanged(..)
        | Message::DragScrollSpeedChanged(..)
        | Message::SettingsToggleSection(..)
        | Message::ToggleRememberWindow(..)
        | Message::ToggleOffline(..)
        | Message::PreferredServersChanged(..)
        | Message::DefaultGameChanged(..)
        | Message::ToggleLockGui(..)
        | Message::ToolsDirChanged(..)
        | Message::BrowseToolsDir) => preferences::handle(app, m),
        m @ (Message::IdentifyDownload(..)
        | Message::IdentifiedDownload(..)
        | Message::ModEndorse(..)
        | Message::ModEndorsed(..)
        | Message::CheckUpdates
        | Message::UpdatesChecked(..)
        | Message::ShowCollection(..)
        | Message::CloseCollection
        | Message::CollectionLinkChanged(..)
        | Message::CollectionFetch
        | Message::CollectionFetched(..)
        | Message::CollectionOpenMod(..)
        | Message::CollectionFetchMissing
        | Message::CollectionInstall
        | Message::CollectionInstallChecked { .. }) => nexus::handle(app, m),
        m @ (Message::ShowCategoriesDialog(..)
        | Message::CloseCategoriesDialog
        | Message::ToggleCategory(..)
        | Message::SetPrimaryCategory(..)
        | Message::CategoryQueryChanged(..)
        | Message::ToggleCategoryEditor
        | Message::NewCategoryNameChanged(..)
        | Message::NewCategoryParentChanged(..)
        | Message::AddCategory
        | Message::RenameCategoryStart(..)
        | Message::RenameCategoryChanged(..)
        | Message::RenameCategoryCommit
        | Message::DeleteCategory(..)
        | Message::FetchNexusCategories
        | Message::NexusCategoriesFetched(..)
        | Message::AssignCategoriesFromNexus
        | Message::ApplyCategories) => categories::handle(app, m),
        m @ (Message::ImportMo2Pick
        | Message::ImportMo2Picked(..)
        | Message::ShowInstanceManager
        | Message::CloseInstanceManager
        | Message::InstanceOpen(..)
        | Message::InstanceForget(..)
        | Message::InstanceRenameStart(..)
        | Message::InstanceRenameChanged(..)
        | Message::InstanceRenameCommit
        | Message::ShowPackDialog
        | Message::ClosePackDialog
        | Message::PackDestChanged(..)
        | Message::PackToggleDownloads
        | Message::PackLevelChanged(..)
        | Message::PackBrowse
        | Message::PackDestPicked(..)
        | Message::PackRun
        | Message::ShowUnpackDialog
        | Message::CloseUnpackDialog
        | Message::UnpackBrowseArchive
        | Message::UnpackArchivePicked(..)
        | Message::UnpackBrowseDest
        | Message::UnpackDestPicked(..)
        | Message::UnpackDestChanged(..)
        | Message::UnpackToggleForce
        | Message::UnpackRun
        | Message::TransferPoll
        | Message::CloseTransferResult
        | Message::Noop) => instances::handle(app, m),
        m @ (Message::RefreshDownloads
        | Message::CleanInstallDebris
        | Message::DownloadTick
        | Message::DeleteDownload(..)
        | Message::PauseDownload(..)
        | Message::ResumeDownload(..)
        | Message::ConfirmDeleteDownload(..)
        | Message::HideDownload(..)
        | Message::ToggleShowHiddenDownloads
        | Message::DownloadFilterChanged(..)
        | Message::DownloadSortChanged(..)
        | Message::PurgeInstalledDownloads
        | Message::ConfirmPurgeInstalled) => downloads::handle(app, m),
        m @ (Message::RefreshSaves
        | Message::DeleteSave(..)
        | Message::SaveToggleSelect(..)
        | Message::SavesTick
        | Message::SavesDeleteSelected
        | Message::SavesCopyToProfile(..)
        | Message::SelectSave(..)
        | Message::FixSaveMods
        | Message::RestorePreSessionPlugins
        | Message::AcceptPluginState
        | Message::ConfirmDeleteSave(..)) => saves::handle(app, m),
        m @ (Message::SelectTab(..)
        | Message::ClearStatus
        | Message::OpenUrl(..)
        | Message::Refresh
        | Message::OpenFolder(..)
        | Message::ShowAbout
        | Message::CloseAbout
        | Message::OpenViewMenu
        | Message::CloseViewMenu
        | Message::OpenFileMenu
        | Message::FileMenuAt(..)
        | Message::ViewMenuAt(..)
        | Message::FiltersAt(..)
        | Message::CloseFileMenu
        | Message::ToggleToolbar
        | Message::ToggleStatusBar
        | Message::CollapseOthers(..)
        | Message::CollapseAllGroups
        | Message::ExpandAllGroups
        | Message::PointerReleased
        | Message::PointerAt(..)
        | Message::SplitGrab
        | Message::AnimationTick
        | Message::WindowResized(..)
        | Message::CycleFocus
        | Message::SelectAllInFocus
        | Message::KeyNav(..)
        | Message::ModifiersChanged(..)) => window::handle(app, m),
    }
}

/// The real (non-separator) mods in the current multi-selection, as indices into
/// `app.mods`. Falls back to the single focus row when the set is empty, so a batch
/// action invoked with just one row selected still does the obvious thing.
/// The Nexus category id recorded for an installed mod, or `None`.
///
/// It is NOT in the mod's `meta.ini`: MO2 records the remote category only in the
/// download's `.meta` sidecar (`category=<nexus id>`, a bare remote id, unlike the
/// mod's own `category=` which is a local id list). The join is `installationFile`,
/// the archive the mod came out of, so a mod installed from a folder, or one whose
/// archive has since been deleted, legitimately has no answer here.
fn nexus_category_of(inst: &eidos_instance::Instance, mod_name: &str) -> Option<i32> {
    let archive = inst.mod_meta(mod_name).installation_file()?;
    // MO2 stores a full path here on Windows instances; take the file name so an
    // imported instance resolves against OUR downloads directory.
    let file = std::path::Path::new(&archive.replace('\\', "/"))
        .file_name()?
        .to_owned();
    let sidecar = inst.downloads_dir().join(&file).with_extension({
        let mut ext = std::path::Path::new(&file)
            .extension()
            .unwrap_or_default()
            .to_owned();
        ext.push(".meta");
        ext
    });
    let raw = eidos_instance::ModMeta::read(&sidecar).category()?;
    // A bare remote id. Anything else (a local list, `-1,`) is not ours to read.
    raw.trim().parse::<i32>().ok().filter(|&id| id > 0)
}

/// Drop anything aimed at a mod row the list no longer shows.
///
/// Every path that narrows the list - the name box, the category dropdown, the
/// filter criteria - has to call this. Filtering does not renumber the rows, so
/// an index kept across it still resolves; it just resolves to a mod the user
/// cannot see. That is the dangerous part: `Space` toggled a mod off screen and
/// `Delete` armed a removal on it, with the status bar naming a mod nowhere in
/// the window. The multi-selection and the drag were already being cleared here;
/// the keyboard focus and its shift-select anchor were not.
///
/// A focus that is still visible is KEPT - narrowing the list around the row you
/// are working on should not cost you your place.
/// What has to be recomputed after a mod's files change on disk.
///
/// The conflict map and the plugin list were both computed from a tree that has
/// just changed: a deleted `.esp` still shows in Plugins, and a conflict emblem
/// still points at a file that is gone.
///
/// Deliberately NOT called while the instance lock is held. `flock` denies a
/// second descriptor even to the process that already owns one, so a refresh
/// inside a locked block deadlocks the handler against itself - which is
/// exactly what happened, and what the flake in `a_backup_is_inert...` was.
fn refresh_after_tree_change(app: &mut App) {
    drop_files_cache(app, None);
    invalidate_plugins(app);
    app.conflicts = compute_conflicts(app);
}

fn forget_hidden_rows(app: &mut App) {
    app.menu_mod = None;
    app.rename = None;
    app.drag_state = None;
    // What the list is DRAWING, not what passes the filter: a folded GROUP
    // hides rows the filter never touched, and a focus left on one is a Delete
    // away from removing a mod off screen.
    //
    // (`drawn_mod_rows` passes the category catalog through for us. It has to
    // be passed: with a category filter set and `None` there, nothing resolves
    // the id against it and every row reports hidden - which would drop the
    // selection on every keystroke in the name box.)
    let visible: std::collections::HashSet<usize> = drawn_mod_rows(app).into_iter().collect();
    let shown = |i: &usize| visible.contains(i);
    if !app.selected_mod.as_ref().is_some_and(shown) {
        app.selected_mod = None;
        // An armed removal aimed at the row that just vanished has to disarm with
        // it, or the next Delete confirms a deletion the user can no longer see.
        app.confirm_remove = None;
    }
    if !app.sel_anchor.as_ref().is_some_and(shown) {
        app.sel_anchor = None;
    }
    let before = app.selected_mods.len();
    app.selected_mods.retain(shown);
    // Same reasoning for the batch guard: it names a count, and the set behind
    // it just shrank.
    if app.selected_mods.len() != before {
        app.confirm_batch_remove = false;
    }
}

pub(crate) fn real_selection(app: &App) -> Vec<usize> {
    let mut set = app.selected_mods.clone();
    if set.is_empty() {
        if let Some(f) = app.selected_mod {
            set.insert(f);
        }
    }
    set.into_iter()
        // Separators carry no toggle; unmanaged rows (the game's own DLCs and
        // Creations) are not ours to toggle OR remove - `ToggleMod` refuses
        // them one by one, and the batch paths must agree. Left in, a Ctrl+A
        // batch-enable read the always-on DLC rows as "something is enabled"
        // and disabled everything forever; a batch-remove aimed
        // remove_dir_all at files inside the game's own Data.
        .filter(|&i| {
            app.mods
                .get(i)
                .is_some_and(|m| !m.is_separator() && !m.is_unmanaged())
        })
        .collect()
}

/// The same set, for the batch actions that REORDER rather than act on contents.
///
/// The split matters: filtering separators out of a move is what used to lift a
/// group's mods above their own header and leave it stranded. Filtering them out
/// of enable/disable/remove is right, because those act on files a separator does
/// not have.
pub(crate) fn move_selection(app: &App) -> Vec<usize> {
    let mut set = app.selected_mods.clone();
    if set.is_empty() {
        if let Some(f) = app.selected_mod {
            set.insert(f);
        }
    }
    set.into_iter().filter(|&i| i < app.mods.len()).collect()
}

/// Reconcile the current FOMOD step's ticks with its EFFECTIVE types, on entry
/// and after every click.
///
/// Earlier choices set condition flags that can flip an option's type after it
/// was ticked. A tick left on a now-NotUsable option still installs that
/// option's files (`build_plan` asks `on ||` before it ever looks at
/// usability) while the view withholds the click that could untick it - a
/// selection the user can see and not change. Required is the mirror image:
/// the engine's own defaults force it on, so a stale un-tick must not survive
/// either. Radio groups come out holding at most one tick, the Required one
/// winning.
fn normalize_fomod_step(w: &mut FomodWizard) {
    use eidos_fomod::{GroupType, PluginType};
    let si = w.step;
    let types = eidos_fomod::step_types(&w.session.config, &w.selection, &w.ctx, si);
    let gtypes: Vec<GroupType> = w
        .session
        .config
        .steps
        .get(si)
        .map(|s| s.groups.iter().map(|g| g.group_type).collect())
        .unwrap_or_default();
    let Some(step_sel) = w.selection.get_mut(si) else {
        return;
    };
    for (gi, g) in step_sel.iter_mut().enumerate() {
        let Some(ts) = types.get(gi) else { continue };
        for (pi, on) in g.iter_mut().enumerate() {
            match ts.get(pi) {
                Some(PluginType::NotUsable) => *on = false,
                Some(PluginType::Required) => *on = true,
                _ => {}
            }
        }
        // A radio group must not come out double-ticked (forcing a Required on
        // above can collide with the user's earlier pick): the Required row
        // keeps the tick, else the first ticked row does.
        if matches!(
            gtypes.get(gi),
            Some(GroupType::SelectExactlyOne | GroupType::SelectAtMostOne)
        ) && g.iter().filter(|&&on| on).count() > 1
        {
            let keep = (0..g.len())
                .find(|&pi| matches!(ts.get(pi), Some(PluginType::Required)))
                .or_else(|| g.iter().position(|&on| on));
            for (pi, on) in g.iter_mut().enumerate() {
                *on = Some(pi) == keep;
            }
        }
    }
}

/// Make `inst` the open instance and reset every piece of state and cache the
/// previous one owned. Shared by the wizard's Finish and the welcome screen's
/// open-existing path: the invalidation list below was learned defect by
/// defect (stale meta rows, the other game's merged Data tree), so there must
/// be exactly ONE copy of it.
pub(crate) fn open_instance(app: &mut App, inst: Instance) {
    app.created = Some(inst);
    // Another instance's LOOT verdicts must not decorate this one's plugins.
    app.loot_meta = None;
    app.endorsing = None;
    app.collection = None;
    reload_mods(app);
    app.tab = Tab::Data;
    app.error = None;
    app.screen = Screen::Main;
    load_tools(app);
    app.conflicts = compute_conflicts(app);
    // BEFORE the refresh, not after: the cache is keyed by mod FOLDER NAME with
    // no instance in the key, so a name that exists in both instances counted
    // as already computed and was never re-read - showing the other game's
    // version, category and content flags until the user pressed F5. Refresh
    // and UpdatesChecked already clear it here.
    app.meta_cache.clear();
    refresh_meta_cache(app);
    // Everything cached from a previously-open instance is stale for this one:
    // plugin order, saves, downloads, selection and counts all belong to the
    // old instance.
    app.plugins = None;
    app.saves = Vec::new();
    app.confirm_delete_save = None;
    app.downloads = Vec::new();
    app.confirm_delete_download = None;
    app.selected_mod = None;
    app.selected_mods.clear();
    app.drag_state = None;
    app.menu_mod = None;
    app.collapsed = load_collapsed(app);
    // The merged-view caches too, which the list above kept missing: they are
    // keyed by directory and validated against `view_generation`, so without a
    // bump the Data tab answers every already-listed directory out of the
    // PREVIOUS instance. Switching from Skyrim to a game with no mods at all
    // still showed Skyrim's merged Data tree, provenance labels and all.
    drop_files_cache(app, None);
    // And the tree's own navigation state, which names paths that need not
    // exist in this game at all.
    app.data_expanded.clear();
    recompute_counts(app);
}

/// Read one of a profile's INIs into a fresh editor state.
fn load_ini_editor(
    prof: &eidos_instance::Profile,
    files: Vec<String>,
    current: String,
) -> IniEditorState {
    let path = prof.ini_path(&current);
    // Hashed BEFORE the text is read: a write in between then refuses the
    // save, never lets the older buffer over it.
    let on_disk = content_hash(&std::fs::read(&path).ok());
    // `read_text_lossy` returns None on ANY read failure, and a file that exists
    // but could not be read is not an empty one. Collapsing the two meant an
    // EACCES or an I/O error opened a blank editor whose always-enabled Save
    // then truncated the real file to nothing.
    let read = eidos_instance::read_text_lossy(&path);
    let exists = path.is_file();
    let unreadable = exists && read.is_none();
    let (text, cp1252) = read.unwrap_or_default();
    IniEditorState {
        content: iced::widget::text_editor::Content::with_text(&text),
        original: text,
        cp1252,
        dirty: false,
        missing: !exists,
        unreadable,
        on_disk,
        files,
        current,
    }
}

/// Seed a file kept in the profile's plugin state (Morrowind.ini) from the
/// game's real state before the editor opens it, as `write_plugin_state` does.
/// Opened unseeded it showed an empty buffer, and saving that founded the
/// profile's Morrowind.ini on a stub that every later launch deployed in place
/// of the real INI, since the seed never replaces a file the profile owns.
/// Fails closed with the reason.
fn seed_ini_from_game(app: &mut App, file: &str) -> Result<(), String> {
    let (Some(inst), Some(game)) = (app.created.as_ref(), selected_game(app)) else {
        return Ok(());
    };
    let prof = inst.active();
    let path = prof.ini_path(file);
    if path.is_file() || !path.starts_with(prof.plugins_state_dir()) {
        return Ok(());
    }
    let (Some(dir), Some(spec)) = (game.plugin_state_dir(), game.plugin_spec()) else {
        return Ok(());
    };
    let seeded = inst
        .try_lock("the Eidos window")
        .and_then(|_lock| prof.seed_plugin_state(&dir, &spec))
        .map_err(|e| format!("Cannot open {file} now: {e}."))?;
    if seeded > 0 {
        // The profile's plugin state changed under the cached load order and
        // the hash its next write is checked against.
        invalidate_plugins(app);
    }
    Ok(())
}

/// How much of a session log the pane reads. A launch log runs to megabytes and
/// the interesting part is always the end, so only the tail is taken.
const LOG_TAIL_BYTES: u64 = 512 * 1024;

/// Read one session log into the pane, keeping records at or above `level`.
pub(crate) fn load_log_pane(
    files: Vec<PathBuf>,
    current: PathBuf,
    level: eidos_log::Level,
) -> LogPaneState {
    use std::io::{Read, Seek, SeekFrom};

    let mut text = String::new();
    let mut truncated = false;
    if let Ok(mut f) = std::fs::File::open(&current) {
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len > LOG_TAIL_BYTES {
            truncated = true;
            let _ = f.seek(SeekFrom::Start(len - LOG_TAIL_BYTES));
        }
        let mut buf = Vec::new();
        let _ = f.read_to_end(&mut buf);
        // Lossy: a log can carry a game's own output, which is not always UTF-8,
        // and refusing to show the file because one byte is odd is the wrong
        // trade for a diagnostic view.
        text = String::from_utf8_lossy(&buf).into_owned();
    }

    let mut lines: Vec<(eidos_log::Level, String)> = Vec::new();
    let mut total = 0usize;
    // Whether the record a continuation belongs to was KEPT. Without this, the
    // continuation lines of a filtered-out Debug record were appended to the last
    // record that survived - attaching a stack trace from a debug message to an
    // unrelated error, at a severity it never had.
    let mut attaching = false;
    for line in text.lines() {
        match eidos_log::parse_line(line) {
            Some((lvl, msg)) => {
                total += 1;
                attaching = lvl >= level;
                if attaching {
                    lines.push((lvl, msg.to_string()));
                }
            }
            // A continuation of a multi-line message belongs to the record above
            // it - shown when that one is, dropped when it is filtered out. A
            // seek into the middle of the file can also land mid-line, which is
            // why a leading orphan (nothing to attach to) is skipped rather than
            // guessed at.
            None => {
                if attaching {
                    if let Some(last) = lines.last_mut() {
                        last.1.push('\n');
                        last.1.push_str(line);
                    }
                }
            }
        }
    }
    LogPaneState {
        files,
        current,
        lines,
        level,
        total,
        truncated,
    }
}

/// Which mod should take each Overwrite file back, keyed by lowercased relative
/// path.
///
/// Read straight off the conflict map the window already keeps: for a path the
/// Overwrite wins, the highest-priority ALTERNATIVE that is a real mod is the one
/// that provides it underneath. Deriving it again here would be a second answer
/// to a question that already has one, and the two could disagree.
///
/// `None` when there is no conflict map yet - that is not "nothing to do", it is
/// "the question has not been asked", and the two must not look the same.
pub(crate) fn overwrite_owners(app: &App) -> Option<HashMap<String, String>> {
    let map = app.conflicts.as_ref()?;
    let mut out = HashMap::new();
    for (rel, node) in &map.files {
        if node.winner != u32::MAX {
            continue; // not the Overwrite's file
        }
        // Descending priority: the first real mod under it wins the file back.
        // BASE_ORIGIN (the game's own Data) is skipped - Eidos never writes there.
        let Some(&origin) = node.alternatives.iter().find(|&&o| o != 0 && o != u32::MAX) else {
            continue;
        };
        let Some(m) = app.mods.get((origin as usize).saturating_sub(1)) else {
            continue;
        };
        if m.is_separator() || m.is_unmanaged() {
            continue;
        }
        out.insert(rel.clone(), m.name.clone());
    }
    Some(out)
}

/// Set a file's modification time. `std::fs` has no portable setter, and a
/// transferred save that carries the COPY date sorts to the top of the
/// destination profile as if it were the newest thing there - which is exactly
/// backwards for a save being moved for safekeeping.
fn set_file_mtime(path: &Path, when: std::time::SystemTime) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let secs = when
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| std::io::Error::other("pre-epoch mtime"))?;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::other("path contains a NUL"))?;
    let times = [
        // atime is left to now: nothing reads it and pinning it would be a
        // second claim this function has no business making.
        libc::timespec {
            tv_sec: 0,
            tv_nsec: libc::UTIME_OMIT,
        },
        libc::timespec {
            tv_sec: secs.as_secs() as libc::time_t,
            tv_nsec: i64::from(secs.subsec_nanos()),
        },
    ];
    // SAFETY: `c` is a valid NUL-terminated path and `times` is a two-element
    // array, which is what utimensat's contract asks for.
    let rc = unsafe { libc::utimensat(libc::AT_FDCWD, c.as_ptr(), times.as_ptr(), 0) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// Join a fetched collection's members against what the instance already holds.
///
/// Entirely local: no request, one pass over the mod list and one over the
/// downloads. Computed on arrival rather than in `view`, which runs every frame.
///
/// A mod counts as INSTALLED when some mod in the list carries that Nexus id -
/// not that exact file. That is deliberate: a collection pins a version, and
/// having a different version of the same mod is a different situation from not
/// having it at all, which the row says separately.
/// How many `eidos nxm` children one click may start.
///
/// Capped rather than fanned out over the whole list: each child is a process
/// holding its own connection, so a hundred-mod collection would otherwise be a
/// hundred processes at once, all racing the same hourly rate budget and all
/// writing into one downloads directory.
pub(crate) const FETCH_BATCH: usize = 5;

/// The next members to ask for, and how many are still queued behind them.
///
/// Split out of the handler so the cap is tested rather than read: the handler
/// itself spawns processes, which a test must not do.
///
/// `asked` is what this pane already started. It is needed because a member
/// stays `Missing` for the whole of its download - the state only turns
/// `Downloaded` once the sidecar lands - so the state alone cannot tell
/// "not started" from "running", and a second click would restart the same
/// first few forever.
pub(crate) fn next_fetch_batch(
    rev: &eidos_nexus::collections::CollectionRevision,
    states: &[MemberState],
    asked: &std::collections::HashSet<u64>,
    limit: usize,
) -> (Vec<(u64, u64, String)>, usize) {
    let queued: Vec<(u64, u64, String)> = rev
        .mods
        .iter()
        .zip(states)
        .filter(|(_, s)| **s == MemberState::Missing)
        .filter(|(m, _)| !asked.contains(&m.file_id))
        .map(|(m, _)| (m.mod_id, m.file_id, m.domain.clone()))
        .collect();
    let left = queued.len().saturating_sub(limit);
    (queued.into_iter().take(limit).collect(), left)
}

pub(crate) fn recompute_collection_states(app: &mut App) {
    let Some(state) = app.collection.as_ref() else {
        return;
    };
    let Some(rev) = state.revision.as_ref() else {
        return;
    };

    // Nexus mod ids are per GAME, so a bare id is ambiguous across games - and a
    // collection may legitimately pull a member from another game's page (an SE
    // collection using an LE asset). `meta.ini` records the game by its SHORT
    // name ("SkyrimSE") while a collection names the DOMAIN
    // ("skyrimspecialedition"), so the two are translated through the game
    // definitions rather than compared as strings.
    let short_for_domain = |domain: &str| -> Option<&'static str> {
        eidos_gamedef::all()
            .iter()
            .find(|d| d.nexus_game.eq_ignore_ascii_case(domain))
            .map(|d| d.short_name)
    };
    let downloaded: Vec<_> = app
        .created
        .as_ref()
        .into_iter()
        .flat_map(|inst| {
            std::fs::read_dir(inst.downloads_dir())
                .into_iter()
                .flatten()
        })
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "meta"))
        .filter(|p| {
            let archive = p.with_extension("");
            archive.is_file()
                && !PathBuf::from(format!("{}.unfinished", archive.display())).exists()
        })
        .map(|p| eidos_instance::ModMeta::read(&p))
        .collect();

    let states = rev
        .mods
        .iter()
        .map(|m| {
            let want = short_for_domain(&m.domain).unwrap_or(&m.domain);
            let game_matches = |have: Option<&str>| {
                have.is_some_and(|have| {
                    have.eq_ignore_ascii_case(want) || have.eq_ignore_ascii_case(&m.domain)
                })
            };
            let candidates: Vec<_> = app
                .meta_cache
                .values()
                .filter(|r| {
                    (r.mod_id == Some(m.mod_id)
                        || r.installed_files.iter().any(|(id, _)| *id == m.mod_id))
                        && r.game_name
                            .as_deref()
                            .is_none_or(|have| game_matches(Some(have)))
                })
                .collect();
            if candidates.iter().any(|r| {
                game_matches(r.game_name.as_deref())
                    && r.install_warning.is_none()
                    && r.installed_files.contains(&(m.mod_id, m.file_id))
            }) {
                MemberState::Installed
            } else if downloaded.iter().any(|meta| {
                meta.mod_id() == Some(m.mod_id)
                    && meta.file_id() == Some(m.file_id)
                    && game_matches(meta.game_name().as_deref())
            }) {
                MemberState::Downloaded
            } else if candidates.iter().any(|r| {
                r.installed_files.is_empty()
                    && (r.version.as_deref().is_none_or(|have| {
                        have.is_empty() || m.version.is_empty() || have == m.version
                    }))
            }) || candidates
                .iter()
                .any(|r| r.game_name.is_none() || r.install_warning.is_some())
            {
                MemberState::Unverified
            } else if !candidates.is_empty() {
                MemberState::OtherVersion
            } else {
                MemberState::Missing
            }
        })
        .collect();
    if let Some(state) = app.collection.as_mut() {
        state.states = states;
    }
}

#[cfg(test)]
mod tests;
