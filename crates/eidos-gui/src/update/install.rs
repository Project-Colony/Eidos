//! Installing mods: the archive picker, BAIN and FOMOD installers, name
//! collisions, dropped files and downloads dropped on the list.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::InstallAt(gap) => {
            app.menu_mod = None;
            // Under a filter the gap between two VISIBLE rows has an unknown
            // number of hidden rows behind it, so "here" would be a lie. Same
            // promise the download drop makes, made the same way.
            if is_filtering(app) {
                app.pending_note = Some(
                    "installed at the end of the list - a filtered list cannot say what a gap means"
                        .to_string(),
                );
                return update(app, Message::InstallMod);
            }
            // The gap is remembered as a PLACE, not yet paired with an archive:
            // `ModPicked` pairs it once the user has chosen one, and only then
            // can `after_install` honour it.
            app.install_gap = Some(gap.min(app.mods.len()));
            return update(app, Message::InstallMod);
        }
        Message::InstallerArchiveResolved {
            request,
            target,
            name,
            result,
        } => {
            if request != app.mod_picker || target != collection_target(app) {
                return Task::none();
            }
            match result {
                Ok(Some(path)) => {
                    return update_inner(
                        app,
                        Message::ModPickedFor {
                            fresh: false,
                            request,
                            target,
                            name: Some(name),
                            path: Some(path),
                        },
                    )
                }
                Ok(None) => return pick_mod_archive(app, Some(name), false),
                Err(error) => {
                    app.status = Some(format!("Cannot reopen collection installer: {error}"))
                }
            }
        }
        Message::InstallMod => return pick_mod_archive(app, None, false),
        Message::ModPickedFor {
            fresh,
            request,
            target,
            name,
            path,
        } => {
            if request != app.mod_picker || target != collection_target(app) {
                return Task::none();
            }
            app.mod_picker = app.mod_picker.wrapping_add(1);
            if let (Some(path), Some(name)) = (&path, name) {
                return open_mod_archive(app, path.clone(), name, fresh);
            }
            return update_inner(app, Message::ModPicked(path));
        }
        Message::ModPicked(picked) => {
            let Some(path) = picked else {
                // Cancelled at the picker: the position must not outlive it.
                app.install_gap = None;
                app.pending_note = None;
                return Task::none();
            };
            // A position chosen from the menu becomes the aim now that there is
            // an archive to pair it with.
            if let Some(gap) = app.install_gap.take() {
                app.install_at = Some((gap, path.clone()));
            }
            let name = eidos_install::mod_name_for(&path);
            return open_mod_archive(app, path, name, false);
        }

        Message::PickerBainToggle(i) => {
            if let Some(PickerMode::Bain { picked, .. }) = app.picker.as_mut().map(|p| &mut p.mode)
            {
                if let Some(b) = picked.get_mut(i) {
                    *b = !*b;
                }
            }
        }
        Message::PickerBainConfirm(yes) => {
            let Some(p) = app.picker.as_mut() else {
                return Task::none();
            };
            match (&mut p.mode, yes) {
                (PickerMode::Bain { asking, .. }, true) => *asking = false,
                // "No, it is not a BAIN package": same extraction, manual picker.
                (PickerMode::Bain { .. }, false) => {
                    p.mode = PickerMode::Manual {
                        root: String::new(),
                    };
                    app.status = Some("Pick the folder that holds the game data.".to_string());
                }
                _ => {}
            }
        }
        Message::PickerSetRoot(r) => {
            if let Some(PickerMode::Manual { root }) = app.picker.as_mut().map(|p| &mut p.mode) {
                *root = r;
            }
        }
        Message::PickerNameChanged(s) => {
            app.typing = true;
            if let Some(p) = app.picker.as_mut() {
                p.name = s;
            }
        }
        Message::PickerInstall => run_picker_install(app),
        Message::PickerCancel => {
            // Dropping the picker drops the ExtractedTree, which removes the temp.
            app.picker = None;
            // A cancelled install must not leave its target priority behind for
            // the NEXT one to pick up.
            app.install_at = None;
            app.status = Some("Install cancelled.".to_string());
        }
        Message::FomodToggle(gi, pi) => {
            if let Some(w) = &mut app.fomod {
                let si = w.step;
                let gtype = w
                    .session
                    .config
                    .steps
                    .get(si)
                    .and_then(|s| s.groups.get(gi))
                    .map(|g| g.group_type);
                if let (Some(gtype), Some(g)) =
                    (gtype, w.selection.get_mut(si).and_then(|s| s.get_mut(gi)))
                {
                    use eidos_fomod::GroupType::*;
                    match gtype {
                        SelectAll => {}
                        SelectExactlyOne => {
                            g.iter_mut().for_each(|x| *x = false);
                            if let Some(s) = g.get_mut(pi) {
                                *s = true;
                            }
                        }
                        SelectAtMostOne => {
                            let was = g.get(pi).copied().unwrap_or(false);
                            g.iter_mut().for_each(|x| *x = false);
                            if let Some(s) = g.get_mut(pi) {
                                *s = !was;
                            }
                        }
                        _ => {
                            if let Some(s) = g.get_mut(pi) {
                                *s = !*s;
                            }
                        }
                    }
                }
                // Defense in depth behind the view's locks: whatever the click
                // just did, the step's hard rules (Required on, NotUsable off,
                // radio groups single-ticked) are re-imposed immediately.
                normalize_fomod_step(w);
            }
        }
        Message::FomodNext => {
            if let Some(w) = &mut app.fomod {
                let vis = eidos_fomod::visible_steps(&w.session.config, &w.selection, &w.ctx);
                let mut s = w.step + 1;
                while s < vis.len() && !vis[s] {
                    s += 1;
                }
                if s < vis.len() {
                    w.step = s;
                }
                normalize_fomod_step(w);
            }
        }
        Message::FomodBack => {
            if let Some(w) = &mut app.fomod {
                let vis = eidos_fomod::visible_steps(&w.session.config, &w.selection, &w.ctx);
                let mut s = w.step;
                while s > 0 {
                    s -= 1;
                    if vis.get(s).copied().unwrap_or(true) {
                        w.step = s;
                        break;
                    }
                }
                normalize_fomod_step(w);
            }
        }
        Message::FomodInstall => {
            let Some(target) = app.fomod.as_ref().map(|w| w.target.clone()) else {
                return Task::none();
            };
            let _guard = match lock_install_target(app, &target) {
                Ok(guard) => guard,
                Err(error) => {
                    app.status = Some(error);
                    return Task::none();
                }
            };
            let inst = app.created.as_ref().expect("validated install target");
            let mods_dir = inst.mods_dir();
            // Collision check BEFORE consuming the wizard: a reinstall must offer
            // Merge/Replace/Rename (MO2's QueryOverwriteDialog) with the user's
            // choices intact, not dead-end and discard them.
            if let Some(w) = app.fomod.as_ref() {
                if let Some(name) = eidos_install::collision_name(&mods_dir, w.session.mod_name()) {
                    let rename_to = suggest_free_name(&mods_dir, &name);
                    app.collision = Some(CollisionPrompt {
                        target: target.clone(),
                        backup: app.prefs.retain_install_backup,
                        archive: w.archive.clone(),
                        name: name.clone(),
                        game_id: w.game_id.clone(),
                        rename_to,
                        fomod: true,
                        // The wizard (still open) owns the extracted tree.
                        tree: None,
                        pick: None,
                    });
                    app.status = Some(format!("'{name}' already exists - choose how to install."));
                    return Task::none();
                }
            }
            if let Some(w) = app.fomod.take() {
                let archive = w.archive.clone();
                match eidos_install::finish_fomod(
                    w.session,
                    &w.selection,
                    &mods_dir,
                    &w.game_id,
                    &w.ctx,
                    eidos_install::OverwritePolicy::Fail,
                ) {
                    Ok(r) => after_install(app, &r.name, r.dest, true, Some(&archive)),
                    Err(e) => app.status = Some(format!("Install failed: {e}")),
                }
            }
        }
        Message::FomodCancel => {
            app.fomod = None;
            app.install_at = None;
            app.status = Some("FOMOD install cancelled.".to_string());
        }
        // Merge and Replace land on a mod that ALREADY has a place in the load
        // order. Honouring a drop's target priority there would yank an
        // established mod out of its slot and silently flip every conflict it is
        // in - so the aim is discarded, not consumed. (Rename keeps it: that
        // really does create a new folder.) MO2 guards the same case.
        Message::CollisionBackupChanged(enabled) => {
            if let Some(collision) = app.collision.as_mut() {
                collision.backup = enabled;
                app.prefs.retain_install_backup = enabled;
                if let Err(error) = app.prefs.save() {
                    app.status = Some(format!("Could not remember the backup choice: {error}"));
                }
            }
        }
        Message::CollisionMerge => {
            app.install_at = None;
            run_collision_install(app, eidos_install::OverwritePolicy::Merge)
        }
        Message::CollisionReplace => {
            app.install_at = None;
            run_collision_install(app, eidos_install::OverwritePolicy::Replace)
        }
        Message::CollisionRenameChanged(s) => {
            if let Some(c) = &mut app.collision {
                c.rename_to = s;
            }
        }
        Message::CollisionRenameCommit => {
            if let Some(new) = app
                .collision
                .as_ref()
                .map(|c| c.rename_to.trim().to_string())
            {
                if new.is_empty() {
                    app.status = Some("Enter a name to install under.".to_string());
                } else {
                    run_collision_install(app, eidos_install::OverwritePolicy::Rename(new));
                }
            }
        }
        Message::CollisionCancel => {
            app.collision = None;
            app.install_at = None;
            app.status = Some("Install cancelled.".to_string());
        }
        // ---- Files dropped in from a file manager ----------------------------
        Message::FilesHovering(on) => app.files_hovering = on,
        Message::FileDropped(path) => {
            app.files_hovering = false;
            // One message PER FILE. Handling each inline would run several
            // blocking extractions back to back and let each one clobber the
            // previous one's modal - the user answers a FOMOD and the answer
            // vanishes when the next file lands. Queue, then drain serially.
            app.dropped.push(path);
            return Task::done(Message::DrainDrops);
        }
        Message::DrainDrops => {
            // A modal is open: the current install is still being answered. The
            // queue is drained again from `after_install` and from every cancel.
            if app.installer.is_some() || app.fomod.is_some()
                || app.picker.is_some()
                || app.collision.is_some()
                // An extraction is already running. Starting a second one would
                // race it for `mods/<name>/` and put two 7-Zips on the same disk.
                || app.install_job.is_some()
                // Same reason, over twenty minutes rather than thirty seconds.
                || app.transfer_job.is_some()
            {
                return Task::none();
            }
            let Some(path) = app.dropped.first().cloned() else {
                return Task::none();
            };
            app.dropped.remove(0);
            if app.created.is_none() {
                app.dropped.clear();
                app.status = Some("Open a game instance before installing anything.".to_string());
                return Task::none();
            }
            // Refuse anything already inside the instance. Installing a mod from
            // mods/ would copy it onto itself, and from the overwrite would move
            // live output out from under a running game.
            if let Some(inst) = &app.created {
                let real = std::fs::canonicalize(&path).ok();
                // BOTH directions. Refusing only "inside the instance" left the
                // opposite case wide open: dropping a directory that CONTAINS
                // mods/ starts a recursive copy of that directory into a
                // subdirectory of itself, which never ends and fills the disk.
                let entangled = |dir: std::path::PathBuf| {
                    std::fs::canonicalize(&dir)
                        .ok()
                        .zip(real.clone())
                        .is_some_and(|(d, p)| p.starts_with(&d) || d.starts_with(&p))
                };
                if entangled(inst.mods_dir())
                    || entangled(inst.overwrite_dir())
                    || entangled(inst.root.clone())
                {
                    app.status = Some(
                        "That folder is part of this instance (or contains it) - it cannot be \
                         installed into itself."
                            .to_string(),
                    );
                    return Task::done(Message::DrainDrops);
                }
            }
            // A folder is an unpacked mod; an archive goes through the installer.
            // Anything else is something the user meant for another window.
            if path.is_dir() {
                return update(app, Message::FolderPicked(Some(path)));
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            if matches!(ext.as_str(), "7z" | "zip" | "rar") {
                return update(app, Message::ModPicked(Some(path)));
            }
            app.status = Some(format!(
                "Not a mod archive: {}. Drop a .7z, .zip or .rar, or a folder.",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
            return Task::done(Message::DrainDrops);
        }
        // ---- Installing a download AT a priority (MO2's drop onto the list) --
        Message::DownloadDragStart(row) => {
            let Some(d) = app.downloads.get(row) else {
                return Task::none();
            };
            // A partial has nothing to install; MO2 refuses the same gesture.
            if d.state == DownloadState::Downloading {
                return Task::none();
            }
            app.download_drag = Some(DownloadDrag {
                path: d.path.clone(),
                gap: app.mods.len(),
                aimed: false,
            });
        }
        Message::DownloadDragOverGap(gap) => {
            if let Some(d) = &mut app.download_drag {
                d.gap = gap.min(app.mods.len());
                // Unlike a reorder there is no "own edge": the archive is not in
                // the list yet, so every gap is a real target.
                d.aimed = true;
            }
        }
        Message::DownloadDragCancel => {
            app.download_drag = None;
            app.drag_scroll = None;
        }
        Message::DownloadDragDrop => {
            app.drag_scroll = None;
            let Some(d) = app.download_drag.take() else {
                return Task::none();
            };
            if !d.aimed {
                return Task::none();
            }
            // Under a filter or a fold the strip between two visible rows is an
            // insertion index with an unknown number of hidden rows behind it,
            // so "here" would land somewhere the user cannot see. MO2 refuses
            // the gesture outright when the list is not in priority order; this
            // installs at the end instead and says so, which is the same
            // promise kept honestly rather than a silent wrong answer.
            if is_filtering(app) {
                app.install_at = None;
                // Said through the installer, not before it: `ModPicked` sets its
                // own status a moment later and would overwrite this one before
                // it could ever be read.
                app.pending_note = Some(
                    "installed at the end of the list - a filtered list cannot say what a gap means"
                        .to_string(),
                );
            } else {
                app.install_at = Some((d.gap, d.path.clone()));
            }
            // Straight into the ordinary installer, so the FOMOD wizard, the
            // BAIN picker, the manual picker and the collision dialog all work
            // from a drop exactly as they do from the Install button.
            return update(app, Message::ModPicked(Some(d.path)));
        }
        Message::InstallPoll => {
            let Some(job) = app.install_job.as_ref() else {
                return Task::none();
            };
            if !job.done.load(std::sync::atomic::Ordering::SeqCst) {
                return Task::none();
            }
            let job = app.install_job.take().expect("just checked");
            let result = job.outcome.lock().ok().and_then(|mut o| o.take());
            // A poisoned mutex means the worker panicked mid-extract. Say so
            // rather than silently doing nothing: the user is watching a dialog.
            match result {
                Some(Err(error)) => {
                    app.status = Some(format!("Install failed: {error}"));
                }
                Some(Ok(opened)) => {
                    if let Some(target) = job.target.clone() {
                        return finish_open(
                            app,
                            Ok(opened),
                            job.archive.clone(),
                            job.name.clone(),
                            target,
                            job.fresh,
                        );
                    } else {
                        app.status =
                            Some("Installation has no original target; reopen the archive.".into());
                    }
                }
                None => {
                    app.status = Some(format!(
                        "Install failed: the worker opening '{}' stopped without a result.",
                        job.name
                    ))
                }
            }
        }
        Message::FomodHover(at) => {
            if let Some(w) = app.fomod.as_mut() {
                w.hover = at;
            }
        }
        Message::FomodUnhover(gi, pi) => {
            if let Some(w) = app.fomod.as_mut() {
                if w.hover == Some((gi, pi)) {
                    w.hover = None;
                }
            }
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
