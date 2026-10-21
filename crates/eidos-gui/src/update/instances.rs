//! Instances: importing from MO2, the instance manager, and packing and
//! unpacking an instance.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::ImportMo2Pick => {
            if app.created.is_none() {
                app.status = Some(t("status_open_instance_first").to_string());
                return Task::none();
            }
            app.profile_menu = None;
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_title(t("dialog_pick_mo2_profile"))
                    .pick_folder(),
                |h| Message::ImportMo2Picked(h.map(|h| h.path().to_path_buf())),
            );
        }
        Message::ImportMo2Picked(picked) => {
            let Some(dir) = picked else {
                return Task::none();
            };
            // Same gates as every other mutation: the import rewrites the modlist
            // AND the plugin state dir, which is bind-mounted into a running
            // session - importing under the game's feet mixed the two states and
            // half-undid the import at the next launch.
            if let Some(r) = &app.running {
                app.status = Some(tf("status_import_while_running", &[("title", &r.title)]));
                return Task::none();
            }
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(tf("status_cannot_import", &[("error", &e)]));
                    return Task::none();
                }
            };
            match inst.import_mo2_profile(&dir) {
                Ok(r) => {
                    // The import is the user speaking, exactly like a GUI edit:
                    // the snapshot follows it, or the damage card would flame on
                    // the imported (smaller) list and its Restore button would
                    // one-click undo the import.
                    let _ = inst.active().snapshot_plugin_state();
                    reload_mods(app);
                    drop_files_cache(app, None);
                    invalidate_plugins(app);
                    app.conflicts = compute_conflicts(app);
                    refresh_meta_cache(app);
                    recompute_counts(app);
                    app.selected_mod = None;
                    app.selected_mods.clear();
                    let mut s = tf("status_mo2_imported", &[("n", &r.matched)]);
                    if r.plugin_files > 0 {
                        s.push(' ');
                        s.push_str(t("status_mo2_load_order_imported"));
                    }
                    if !r.missing.is_empty() {
                        s.push(' ');
                        s.push_str(&tf("status_mo2_missing", &[("n", &r.missing.len())]));
                    }
                    app.status = Some(s);
                }
                Err(e) => app.status = Some(tf("status_mo2_import_failed", &[("error", &e)])),
            }
        }
        // ---- Instance manager (MO2's Manage Instances) -----------------------
        Message::ShowInstanceManager => {
            app.file_menu_open = false;
            // Read fresh: an instance created or moved since startup should be
            // here, and the welcome-screen list is only built once.
            app.known = known_instances(&app.games);
            app.instances_open = true;
            app.instance_rename = None;
            app.confirm_forget = None;
        }
        Message::CloseInstanceManager => {
            app.instances_open = false;
            app.instance_rename = None;
            app.confirm_forget = None;
        }
        Message::InstanceOpen(i) => {
            app.instances_open = false;
            return update(app, Message::OpenKnown(i));
        }
        Message::InstanceForget(i) => {
            // Two clicks, like every other action that removes something.
            if app.confirm_forget != Some(i) {
                app.confirm_forget = Some(i);
                return Task::none();
            }
            app.confirm_forget = None;
            let Some(k) = app.known.get(i).cloned() else {
                return Task::none();
            };
            if !k.portable {
                app.status = Some(t("status_global_nothing_to_forget").to_string());
                return Task::none();
            }
            let mut reg = eidos_instance::Registry::load_from(&app.registry_path);
            reg.forget_portable(&k.inst.root);
            match reg.save_to(&app.registry_path) {
                Ok(()) => {
                    app.known = known_instances(&app.games);
                    // FORGOTTEN, not deleted, and the difference is the whole
                    // point: an instance holds a mod pool that can run to
                    // hundreds of gigabytes, and no button here is going to
                    // remove that on one confirmation.
                    app.status = Some(tf(
                        "status_instance_forgotten",
                        &[("path", &k.inst.root.display())],
                    ));
                }
                Err(e) => app.status = Some(tf("status_instance_list_failed", &[("error", &e)])),
            }
        }
        Message::InstanceRenameStart(i) => {
            app.confirm_forget = None;
            let Some(k) = app.known.get(i) else {
                return Task::none();
            };
            let current = k
                .inst
                .root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            app.instance_rename = Some((i, current));
        }
        Message::InstanceRenameChanged(t) => {
            app.typing = true;
            if let Some((_, name)) = &mut app.instance_rename {
                *name = t;
            }
        }
        Message::InstanceRenameCommit => {
            let Some((i, typed)) = app.instance_rename.take() else {
                return Task::none();
            };
            let Some(k) = app.known.get(i).cloned() else {
                return Task::none();
            };
            let name = typed.trim().to_string();
            if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
                app.status = Some(t("status_not_a_folder_name").to_string());
                return Task::none();
            }
            if !k.portable {
                app.status = Some(t("status_global_no_folder").to_string());
                return Task::none();
            }
            // Renaming the folder out from under the OPEN instance would leave
            // every cached path in the window pointing at somewhere that no
            // longer exists - including the lock it is holding.
            if app.created.as_ref().is_some_and(|c| c.root == k.inst.root) {
                app.status = Some(t("status_instance_is_open").to_string());
                return Task::none();
            }
            // And not one ANOTHER process is using. `app.created` only knows
            // about this window; the flock is what covers a second Eidos window,
            // a running game, or a CLI session - all of which hold paths into
            // the folder about to move.
            if let Err(e) = probe_lock(&k.inst) {
                app.status = Some(tf("status_instance_in_use", &[("error", &e)]));
                return Task::none();
            }
            let Some(parent) = k.inst.root.parent() else {
                return Task::none();
            };
            let dest = parent.join(&name);
            if dest.exists() {
                app.status = Some(tf("status_path_exists", &[("path", &dest.display())]));
                return Task::none();
            }
            match std::fs::rename(&k.inst.root, &dest) {
                Ok(()) => {
                    // The registry points at the OLD path; it has to follow, or
                    // the instance vanishes from every list including this one.
                    let mut reg = eidos_instance::Registry::load_from(&app.registry_path);
                    reg.forget_portable(&k.inst.root);
                    reg.remember_portable(&dest);
                    // NOT discarded. The folder has already moved; if the
                    // registry cannot follow, the instance is orphaned - it
                    // exists at a path nothing lists - and reporting success
                    // would send the user looking for it in the one place it is
                    // guaranteed not to be.
                    match reg.save_to(&app.registry_path) {
                        Ok(()) => {
                            app.known = known_instances(&app.games);
                            app.status =
                                Some(tf("status_renamed_plain", &[("name", &dest.display())]));
                        }
                        Err(e) => {
                            app.status = Some(tf(
                                "status_instance_renamed_list_failed",
                                &[("path", &dest.display()), ("error", &e)],
                            ));
                        }
                    }
                }
                Err(e) => app.status = Some(tf("status_rename_failed", &[("error", &e)])),
            }
        }
        // ---- move an instance to another machine (eidos pack / eidos unpack) ----
        Message::ShowPackDialog => {
            app.file_menu_open = false;
            app.view_menu_open = false;
            let Some(inst) = app.created.clone() else {
                app.status = Some(t("status_open_instance_first").to_string());
                return Task::none();
            };
            if app.transfer_job.is_some() {
                app.status = Some(t("status_transfer_busy").to_string());
                return Task::none();
            }
            let opt = eidos_transfer::Options::default();
            // The walk, now, so the dialog can say how many files and how big
            // before anybody commits twenty minutes to it. It is a directory
            // walk of an instance already in the page cache - measured at 0.2 s
            // on a 57 000-file one - and it is the whole reason to have a dialog
            // rather than a menu item that just starts.
            let plan = eidos_transfer::plan(&inst, &opt);
            let game_id = selected_game(app)
                .map(|g| g.def.id.to_string())
                .or_else(|| inst.game_id())
                .unwrap_or_else(|| "instance".to_string());
            // Beside the user's own files, not inside the instance - which the
            // library refuses anyway, since an archive being written into the
            // tree it is reading grows for as long as it is read.
            let home = std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let dest = home
                .join(eidos_transfer::default_file_name(&game_id))
                .to_string_lossy()
                .into_owned();
            app.pack = Some(PackDialogState {
                dest,
                downloads: opt.downloads,
                level: eidos_transfer::DEFAULT_LEVEL,
                plan,
            });
        }
        Message::ClosePackDialog => app.pack = None,
        Message::PackDestChanged(v) => {
            if let Some(d) = &mut app.pack {
                d.dest = v;
            }
        }
        Message::PackToggleDownloads => {
            // No re-walk. The plan already counted `downloads/` separately, so
            // the preview subtracts - which matters because this is a checkbox
            // and walking 57 000 files under the user's finger is not what a
            // checkbox should feel like.
            if let Some(d) = &mut app.pack {
                d.downloads = !d.downloads;
            }
        }
        Message::PackLevelChanged(n) => {
            if let Some(d) = &mut app.pack {
                d.level = n;
            }
        }
        Message::PackBrowse => {
            let Some(d) = &app.pack else {
                return Task::none();
            };
            let start = std::path::PathBuf::from(&d.dest);
            let name = start
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("backup.{}", eidos_transfer::EXTENSION));
            let mut dlg = rfd::AsyncFileDialog::new()
                .add_filter(t("dialog_eidos_backup"), &[eidos_transfer::EXTENSION])
                .set_file_name(name)
                .set_title(t("dialog_where_backup"));
            if let Some(parent) = start.parent().filter(|p| p.is_dir()) {
                dlg = dlg.set_directory(parent);
            }
            return Task::perform(dlg.save_file(), |h| {
                Message::PackDestPicked(h.map(|h| h.path().to_path_buf()))
            });
        }
        Message::PackDestPicked(picked) => {
            // Re-read rather than trusted: the window keeps handling events
            // while the native dialog is up, so the dialog may be gone.
            if let (Some(path), Some(d)) = (picked, app.pack.as_mut()) {
                d.dest = path.to_string_lossy().into_owned();
            }
        }
        Message::PackRun => {
            let Some(inst) = app.created.clone() else {
                app.status = Some(t("status_open_instance_first").to_string());
                return Task::none();
            };
            if app.transfer_job.is_some() {
                return Task::none();
            }
            // Checked BEFORE the dialog is taken. The button is disabled while
            // the field is empty, so this is unreachable by clicking - and a
            // handler that loses what the user typed on a path nobody can take
            // is still a handler that loses it.
            let empty = app.pack.as_ref().is_none_or(|d| d.dest.trim().is_empty());
            if empty {
                app.status = Some(t("status_backup_needs_name").to_string());
                return Task::none();
            }
            let Some(d) = app.pack.take() else {
                return Task::none();
            };
            let dest = std::path::PathBuf::from(d.dest.trim());
            let opt = eidos_transfer::Options {
                downloads: d.downloads,
                level: d.level,
                // The window asked before it got here (the dialog says so when
                // the file exists), so the worker must not refuse it again.
                force: true,
            };
            app.transfer_job = Some(spawn_pack(inst, dest, opt));
        }
        Message::ShowUnpackDialog => {
            app.file_menu_open = false;
            app.view_menu_open = false;
            // Deliberately NO "open an instance first" guard. Restoring onto a
            // machine that has none is the entire point of the file.
            if app.transfer_job.is_some() {
                app.status = Some(t("status_transfer_busy").to_string());
                return Task::none();
            }
            app.unpack = Some(UnpackDialogState {
                archive: String::new(),
                dest: String::new(),
                manifest: None,
                error: None,
                force: false,
            });
        }
        Message::CloseUnpackDialog => app.unpack = None,
        Message::UnpackBrowseArchive => {
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .add_filter(t("dialog_eidos_backup"), &[eidos_transfer::EXTENSION])
                    .set_title(t("dialog_pick_backup"))
                    .pick_file(),
                |h| Message::UnpackArchivePicked(h.map(|h| h.path().to_path_buf())),
            );
        }
        Message::UnpackArchivePicked(picked) => {
            let Some(path) = picked else {
                return Task::none();
            };
            let Some(d) = app.unpack.as_mut() else {
                return Task::none();
            };
            d.archive = path.to_string_lossy().into_owned();
            d.manifest = None;
            d.error = None;
            // Reading the manifest is a HEADER read - measured at 39 ms on a
            // 57 000-entry archive - so it happens here rather than through the
            // worker machinery. It is also what tells a backup from any other
            // file, and the answer decides what the dialog can offer.
            match eidos_transfer::Transfer::new(eidos_transfer::Options::default())
                .and_then(|t| t.peek(&path))
            {
                Ok(m) => {
                    // A central backup knows where it belongs. A portable one
                    // does not, and guessing would put 70 GB somewhere nobody
                    // chose.
                    if d.dest.is_empty() && !m.portable && !m.game_id.is_empty() {
                        d.dest = Instance::global(&m.game_id)
                            .root
                            .to_string_lossy()
                            .into_owned();
                    }
                    d.manifest = Some(m);
                }
                Err(e) => d.error = Some(e.to_string()),
            }
        }
        Message::UnpackBrowseDest => {
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_title(t("dialog_where_instance"))
                    .pick_folder(),
                |h| Message::UnpackDestPicked(h.map(|h| h.path().to_path_buf())),
            );
        }
        Message::UnpackDestPicked(picked) => {
            if let (Some(path), Some(d)) = (picked, app.unpack.as_mut()) {
                d.dest = path.to_string_lossy().into_owned();
            }
        }
        Message::UnpackDestChanged(v) => {
            if let Some(d) = &mut app.unpack {
                d.dest = v;
            }
        }
        Message::UnpackToggleForce => {
            if let Some(d) = &mut app.unpack {
                d.force = !d.force;
            }
        }
        Message::UnpackRun => {
            if app.transfer_job.is_some() {
                return Task::none();
            }
            let Some(d) = app.unpack.take() else {
                return Task::none();
            };
            let (archive, dest) = (
                std::path::PathBuf::from(d.archive.trim()),
                std::path::PathBuf::from(d.dest.trim()),
            );
            if archive.as_os_str().is_empty() || dest.as_os_str().is_empty() {
                // Put it back rather than losing what they had typed.
                app.unpack = Some(UnpackDialogState {
                    error: Some(t("status_unpack_needs_both").to_string()),
                    ..d
                });
                return Task::none();
            }
            let opt = eidos_transfer::Options {
                force: d.force,
                ..eidos_transfer::Options::default()
            };
            app.transfer_job = Some(spawn_unpack(archive, dest, opt));
        }
        Message::TransferPoll => {
            let Some(job) = app.transfer_job.as_ref() else {
                return Task::none();
            };
            // `done` is read before anything is taken, and `finished` guards a
            // second collection: the poll rides a repeating tick, so it arrives
            // again the tick after it has already done its work.
            if job.finished.is_some() || !job.done.load(std::sync::atomic::Ordering::SeqCst) {
                return Task::none();
            }
            let (kind, target) = (job.kind, job.target.clone());
            let mut result = job
                .outcome
                .lock()
                .ok()
                .and_then(|mut o| o.take())
                // A poisoned mutex means the worker panicked. Say so: the user
                // is watching a dialog that would otherwise never finish.
                .unwrap_or_else(|| Err(t("status_worker_stopped").to_string()));
            if kind == crate::TransferKind::Unpack {
                if let Ok(summary) = &mut result {
                    if let Some(note) = open_unpacked(app, &target) {
                        summary.push_str(&note);
                    }
                }
            }
            if kind == crate::TransferKind::Collection {
                // The worker added folders to `mods/` and rewrote `modlist.txt`
                // from its own thread, so the window is holding the list as it
                // was BEFORE the install. Saving that stale list - which the
                // next tick of a checkbox does - writes the members straight
                // back out of the profile, and the rule-derived order with them.
                reload_mods(app);
                refresh_after_tree_change(app, None);
                // Whole, not appended: a member installed over a folder of the
                // same name rewrote a `meta.ini` this cache already holds, and
                // the health check and the counts now read it from here.
                app.meta_cache.clear();
                refresh_meta_cache(app);
            }
            // Set LAST: `open_unpacked` calls `open_instance`, which resets a
            // great deal of the window, and the card has to survive it.
            if let Some(j) = app.transfer_job.as_mut() {
                j.finished = Some(result);
            }
        }
        Message::CloseTransferResult => app.transfer_job = None,
        Message::Noop => {}
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
