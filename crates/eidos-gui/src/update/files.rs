//! Files: the Data and Overwrite tabs, the per-mod file tree, hidden files, INI
//! tweaks, archives and previews.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::OverwriteToModStart => {
            if app.created.as_ref().is_some_and(|i| i.overwrite_is_empty()) {
                app.status =
                    Some("The Overwrite is empty - nothing to turn into a mod.".to_string());
            } else {
                // Default to a fresh, non-colliding name, like the installer does.
                let suggestion = app
                    .created
                    .as_ref()
                    .map(|i| suggest_free_name(&i.mods_dir(), "Overwrite output"))
                    .unwrap_or_else(|| "Overwrite output".to_string());
                app.overwrite_to_mod = Some(suggestion);
            }
        }
        Message::ToolOutputModChanged(name) => {
            if let Some(state) = &mut app.executables {
                state.output_mod = name;
            }
        }
        Message::OverwriteToModName(s) => {
            app.typing = true;
            if app.overwrite_to_mod.is_some() {
                app.overwrite_to_mod = Some(s);
            }
        }
        Message::OverwriteToModCancel => {
            app.overwrite_to_mod = None;
        }
        Message::OverwriteToModCommit => {
            let Some(name) = app.overwrite_to_mod.take().map(|s| s.trim().to_string()) else {
                return Task::none();
            };
            // Owned: the staleness check below needs `app` mutably.
            let Some(inst) = app.created.clone() else {
                return Task::none();
            };
            let existing = inst.mods_dir().join(&name).exists();
            // Same reason as Clear: this MOVES the whole Overwrite into a mod
            // folder, so doing it mid-session pulls the write layer out from under
            // a running game.
            let _guard = match inst.try_lock("the Eidos window") {
                Ok(g) => g,
                Err(e) => {
                    app.status = Some(format!("Cannot turn the Overwrite into a mod: {e}."));
                    return Task::none();
                }
            };
            // Checked before the move, as Rename does: a save refused after it
            // reloads a list that does not name the new mod, which comes back
            // DISABLED - generated output the game then silently stops loading.
            if refuse_stale_modlist(app) {
                return Task::none();
            }
            match inst.overwrite_into_mod(&name) {
                Ok(dest) => {
                    // Highest priority (the end of the display order), which is where
                    // the Overwrite's content effectively sat.
                    if !app.mods.iter().any(|m| m.name == name) {
                        app.mods.push(ModEntry {
                            name: name.clone(),
                            enabled: true,
                            path: dest,
                            unmanaged: false,
                        });
                    }
                    drop_files_cache(app, None);
                    // Before mods_changed, which replaces it with the reason if
                    // the save is refused.
                    app.status = Some(if existing {
                        format!("Moved the Overwrite into '{name}'.")
                    } else {
                        format!("Created mod '{name}' from the Overwrite.")
                    });
                    mods_changed(app);
                }
                Err(e) => {
                    app.status = Some(format!("Could not create the mod: {e}"));
                    // Keep the prompt open so the name can be fixed.
                    app.overwrite_to_mod = Some(name);
                }
            }
        }
        Message::ClearOverwrite => {
            if let Some(inst) = &app.created {
                let dir = inst.overwrite_dir();
                if app.confirm_clear {
                    app.confirm_clear = false;
                    // The Overwrite is the running game's WRITE LAYER. Emptying it
                    // mid-session deletes files the game has open - shader caches,
                    // MCM configs, script-extender cosaves - and the game finds out
                    // by failing. The lock is what makes that impossible; arming the
                    // confirmation above deliberately does not take it, so a
                    // refusal is reported once, on the click that would act.
                    let held = inst.try_lock("the Eidos window");
                    app.status = Some(match held {
                        Err(e) => format!("Cannot clear the Overwrite: {e}."),
                        Ok(_guard) => match clear_dir_contents(&dir) {
                            Ok(()) => "Overwrite cleared.".to_string(),
                            Err(e) => format!("Clear failed: {e}"),
                        },
                    });
                    drop_files_cache(app, Some("Overwrite"));
                    app.conflicts = compute_conflicts(app);
                } else {
                    app.confirm_clear = true;
                    app.status = Some(
                        "Click Clear again to confirm - this permanently deletes everything the game wrote to the Overwrite (configs, new saves, generated files)."
                            .to_string(),
                    );
                }
            }
        }
        // ---- hidden files (MO2 filetree.cpp HIDE/UNHIDE) ----------------------
        Message::DataToggleDir(rel) => {
            if !app.data_expanded.remove(&rel) {
                app.data_expanded.insert(rel);
            }
        }
        Message::DataQueryChanged(q) => {
            app.typing = true;
            app.data_query = q;
        }
        Message::DataToggleConflictsOnly => {
            app.data_conflicts_only = !app.data_conflicts_only;
        }
        Message::DataExpandAll => {
            // Walk the tree to the row budget, expanding as it goes, so this
            // costs exactly what DRAWING the expanded tree costs and no more. A
            // fully-expanded Skyrim Data is six figures of files; expanding the
            // whole thing eagerly would read every enabled mod to the leaves.
            for _ in 0..32 {
                let rows = data_tree_rows(app, DATA_TREE_ROWS);
                let mut added = false;
                for r in rows {
                    if r.row.is_dir && app.data_expanded.insert(r.rel) {
                        added = true;
                    }
                }
                if !added {
                    break;
                }
            }
            app.status = Some(format!(
                "Expanded to the first {DATA_TREE_ROWS} entries. Filter to narrow it down."
            ));
        }
        Message::DataCollapseAll => app.data_expanded.clear(),
        Message::DataReveal(path) => {
            // The DIRECTORY, not the file: xdg-open on a .esp hands it to
            // whatever claims that type, which on a modding machine is usually
            // nothing, and on an unlucky one is an editor that rewrites it.
            let target = if path.is_dir() {
                path.clone()
            } else {
                path.parent()
                    .map(std::path::Path::to_path_buf)
                    .unwrap_or(path.clone())
            };
            let _ = std::process::Command::new("xdg-open").arg(&target).spawn();
            app.status = Some(format!("Opened {}", target.display()));
        }
        Message::OverwriteToggleDir(rel) => {
            if !app.overwrite_expanded.remove(&rel) {
                app.overwrite_expanded.insert(rel);
            }
        }
        Message::ToggleFileHidden(i, rel) => {
            let Some(m) = app.mods.get(i).cloned() else {
                return Task::none();
            };
            let hide = !path_is_hidden(&rel);
            // What actually gets renamed. Unhiding a row that sits UNDER a
            // hidden directory ("meshes.mohidden/foo.nif") must strip the
            // suffix from the FIRST hidden component: the leaf carries none, so
            // aiming set_hidden at it answered "not hidden" for every file the
            // tab itself had just labelled Unhide. The directory comes back
            // whole - the only rename that exists to make.
            let subject: String = if hide {
                rel.clone()
            } else {
                let mut parts: Vec<&str> = Vec::new();
                for p in rel.split('/') {
                    parts.push(p);
                    if eidos_core::is_hidden_name(p) {
                        break;
                    }
                }
                parts.join("/")
            };
            let target = m.path.join(&subject);
            // The path came from a listing that may be a redraw old; a stale row
            // must report a miss, not act on whatever now sits at that path.
            if target.symlink_metadata().is_err() {
                app.status = Some(format!("'{rel}' is no longer there."));
                return Task::none();
            }
            // Hiding renames a file INSIDE a mod directory, which is a layer of
            // a live mount while a session is running - and this was the one
            // mutation in this file that took no lock, so it could rename a file
            // out from under a playing game. Every other mutating handler here
            // takes it; this one did not, and the omission is invisible until it
            // is not.
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot change hidden files: {e}."));
                    return Task::none();
                }
            };
            match set_hidden(&target, hide) {
                Ok(_) => {
                    let verb = if hide { "Hid" } else { "Unhid" };
                    app.status = Some(format!("{verb} '{subject}' in '{}'.", m.name));
                    // A directory unhide brings back EVERYTHING under it, plugin
                    // files included, whatever leaf the click came from - so
                    // assume the worst, exactly as RestoreHiddenFiles does.
                    if subject == rel {
                        after_hidden_change(app, &m.name, &rel);
                    } else {
                        after_hidden_change(app, &m.name, "restored.esp");
                    }
                }
                Err(e) => {
                    let verb = if hide { "hide" } else { "unhide" };
                    app.status = Some(format!("Could not {verb} '{subject}': {e}"));
                }
            }
        }
        Message::ToggleIniTweak(i, name) => {
            let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) else {
                return Task::none();
            };
            let result = edit_mod_meta(inst, &m.name, |meta| {
                let mut list = meta.ini_tweaks().to_vec();
                let was_on = list.iter().any(|e| e.eq_ignore_ascii_case(&name));
                if was_on {
                    list.retain(|e| !e.eq_ignore_ascii_case(&name));
                } else {
                    list.push(name.clone());
                }
                meta.set_ini_tweaks(&list);
                was_on
            });
            match result {
                Ok(was_on) => {
                    let verb = if was_on { "Disabled" } else { "Enabled" };
                    app.status = Some(format!("{verb} INI tweak '{name}' for '{}'.", m.name));
                    bump_views(app);
                }
                Err(e) => app.status = Some(format!("Could not save the tweak list: {e}")),
            }
        }

        Message::RestoreHiddenFiles(i) => {
            app.menu_mod = None;
            let Some(m) = app.mods.get(i).cloned() else {
                return Task::none();
            };
            // The bulk twin of ToggleFileHidden's rename, behind the same flock:
            // "Unhide all" strips dozens of .mohidden suffixes inside a layer of
            // a live mount - the exact mutation the single-file handler's guard
            // exists for, reachable one menu item away.
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot change hidden files: {e}."));
                    return Task::none();
                }
            };
            match restore_hidden_files(&m.path) {
                Ok(0) => app.status = Some(format!("'{}' has no hidden files.", m.name)),
                Ok(n) => {
                    app.status = Some(format!("Unhid {n} file(s) in '{}'.", m.name));
                    // No single path to key the plugin refresh on, so assume the
                    // worst: a restored .esp changes the load order.
                    after_hidden_change(app, &m.name, "restored.esp");
                }
                Err(e) => {
                    app.status = Some(format!("Could not unhide files: {e}"));
                    after_hidden_change(app, &m.name, "restored.esp");
                }
            }
        }
        Message::OverwriteSyncToMods => {
            if !app.confirm_sync {
                app.confirm_sync = true;
                return Task::none();
            }
            app.confirm_sync = false;
            let Some(inst) = app.created.clone() else {
                return Task::none();
            };
            // The instance lock, like every other mutation: this moves files the
            // mount is serving from.
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot sync now: {e}."));
                    return Task::none();
                }
            };
            let Some(owners) = overwrite_owners(app) else {
                app.status = Some(
                    "Open the Conflicts tab once so Eidos knows which mod provides what."
                        .to_string(),
                );
                return Task::none();
            };
            if owners.is_empty() {
                app.status = Some(
                    "Nothing in the Overwrite is provided by a mod - there is nowhere to send it \
                     back to. Use Create mod... instead."
                        .to_string(),
                );
                return Task::none();
            }
            match inst.sync_overwrite_to_mods(&owners) {
                Ok((moved, failures)) => {
                    drop(_lock);
                    // Every touched mod's tree changed, and so did the merged
                    // view and the conflict map derived from it.
                    drop_files_cache(app, None);
                    reload_mods(app);
                    app.conflicts = compute_conflicts(app);
                    app.status = Some(match (moved, failures.len()) {
                        (0, 0) => "Nothing to send back.".to_string(),
                        (n, 0) => format!("Sent {n} file(s) back to the mods that provide them."),
                        (n, f) => format!(
                            "Sent {n} file(s) back; {f} could not be moved, e.g. {}",
                            failures[0]
                        ),
                    });
                }
                Err(e) => app.status = Some(format!("Sync failed: {e}")),
            }
        }
        Message::PreviewFile(path) => {
            return crate::file_preview::start(app, path, None, None, None);
        }
        Message::ArchiveProviderAction {
            epoch,
            member,
            provider,
            export,
        } => {
            return crate::archive_conflicts::provider_action(app, epoch, member, provider, export)
        }
        Message::ArchiveFilterChanged(filter) => {
            app.archive_filter = filter;
            app.archive_page = 0;
        }
        Message::ArchivePage(page) => app.archive_page = page,
        Message::ArchiveExportDestination(id, path) => {
            return crate::archive_conflicts::export_destination(app, id, path)
        }
        Message::ArchiveExportFinished(id, result) => {
            crate::archive_conflicts::export_finished(app, id, result)
        }
        Message::RunFileExtension(operation, path) => {
            return crate::file_preview::start(app, path, Some(operation), None, None);
        }
        Message::ChooseExtensionFile(operation) => {
            app.extension_picker = app.extension_picker.wrapping_add(1);
            let request = app.extension_picker;
            let target = collection_target(app);
            let epoch = app.archive_epoch.get();
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_title("Choose a file for an extension")
                    .pick_file(),
                move |file| Message::ExtensionFilePicked {
                    request,
                    target,
                    epoch,
                    operation,
                    path: file.map(|f| f.path().to_path_buf()),
                },
            );
        }
        Message::ExtensionFilePicked {
            request,
            target,
            epoch,
            operation,
            path,
        } => {
            if request != app.extension_picker
                || target != collection_target(app)
                || epoch != app.archive_epoch.get()
            {
                return Task::none();
            }
            app.extension_picker = app.extension_picker.wrapping_add(1);
            if let Some(path) = path {
                app.addons_open = false;
                return crate::file_preview::start(app, path, Some(operation), None, None);
            }
        }
        Message::PreviewReady(id, preview) => {
            if let Some(task) = crate::nif_preview::prepare_textures(app, id, &preview) {
                return task;
            }
            crate::file_preview::complete(app, id, preview);
        }
        Message::PreviewNifView(view) => {
            if let Some(preview) = &app.preview {
                return crate::file_preview::start_nif(app, preview.path().to_path_buf(), view);
            }
        }
        Message::PreviewDdsSelection(selection) => {
            if app.preview_pending.is_none() {
                if let Some(preview) = &app.preview {
                    let path = preview.path();
                    return crate::file_preview::start(
                        app,
                        path.to_path_buf(),
                        None,
                        Some(selection),
                        None,
                    );
                }
            }
        }
        Message::ClosePreview => {
            app.extension_picker = app.extension_picker.wrapping_add(1);
            app.preview_pending.take();
            app.preview = None;
        }
        Message::FiletreeOpen(i, rel) => {
            let Some(base) = app.mods.get(i).map(|m| m.path.clone()) else {
                return Task::none();
            };
            let Some(path) = resolve_in_mod(&base, &rel) else {
                app.error = Some(format!(
                    "Refused to open {rel}: not a path inside this mod."
                ));
                return Task::none();
            };
            return update(app, Message::OpenFolder(path));
        }
        Message::FiletreeRenameStart(i, rel) => {
            // Prefill with the NAME, not the path: a rename box holding
            // `Meshes/armour/x.nif` invites somebody to edit the directories,
            // which is a move, which is not what this is.
            app.tree_rename_text = rel.rsplit('/').next().unwrap_or(&rel).to_string();
            app.tree_rename = app.mods.get(i).map(|m| (m.name.clone(), rel));
            app.tree_delete_armed = None;
        }
        Message::FiletreeRenameChanged(t) => {
            app.typing = true;
            app.tree_rename_text = t;
        }
        Message::FiletreeRenameCancel => {
            app.tree_rename = None;
            app.tree_rename_text.clear();
        }
        Message::FiletreeRenameCommit => {
            let Some((mod_name, rel)) = app.tree_rename.take() else {
                return Task::none();
            };
            let name = app.tree_rename_text.trim().to_string();
            app.tree_rename_text.clear();
            // By name, resolved now: an index would have been read against a
            // list anything could have reloaded while the box was open.
            let Some(base) = app
                .mods
                .iter()
                .find(|m| m.name == mod_name)
                .map(|m| m.path.clone())
            else {
                return Task::none();
            };
            // The new name replaces the LAST component and nothing else, so a
            // rename can never become a move out of its directory.
            let parent = rel.rsplit_once('/').map(|(p, _)| p.to_string());
            let dest_rel = match &parent {
                Some(p) => format!("{p}/{name}"),
                None => name.clone(),
            };
            let (Some(from), Some(to)) = (
                resolve_in_mod(&base, &rel),
                resolve_in_mod(&base, &dest_rel),
            ) else {
                app.error = Some(format!("Refused to rename to {name}: not a name."));
                return Task::none();
            };
            if from == to {
                return Task::none();
            }
            // Never over something that is already there. `fs::rename` would
            // replace a file silently, and this is a mod's own contents.
            if to.symlink_metadata().is_ok() {
                app.error = Some(format!("{name} already exists in that folder."));
                return Task::none();
            }
            let done = {
                let _lock = match app
                    .created
                    .as_ref()
                    .expect("checked above")
                    .try_lock("the Eidos window")
                {
                    Ok(l) => l,
                    Err(e) => {
                        app.status = Some(format!("Busy: {e}."));
                        return Task::none();
                    }
                };
                match std::fs::rename(&from, &to) {
                    Ok(()) => true,
                    Err(e) => {
                        app.error = Some(format!("Could not rename: {e}"));
                        false
                    }
                }
            };
            if done {
                refresh_after_tree_change(app);
                app.status = Some(format!("Renamed to {name}."));
            }
        }
        Message::FiletreeDelete(i, rel) => {
            app.tree_delete_armed = app.mods.get(i).map(|m| (m.name.clone(), rel));
            app.tree_rename = None;
        }
        Message::ConfirmFiletreeDelete(name, rel) => {
            app.tree_delete_armed = None;
            // By name, so a reload between the clicks cannot point this at
            // another mod's folder - where the same relative path may well
            // exist, and would be deleted without a word.
            let Some(base) = app
                .mods
                .iter()
                .find(|m| m.name == name)
                .map(|m| m.path.clone())
            else {
                return Task::none();
            };
            let Some(path) = resolve_in_mod(&base, &rel) else {
                app.error = Some(format!(
                    "Refused to delete {rel}: not a path inside this mod."
                ));
                return Task::none();
            };
            // The instance lock spans the delete and nothing else - see
            // `refresh_after_tree_change` for why the refresh must be outside it.
            let r = {
                let _lock = match app
                    .created
                    .as_ref()
                    .expect("checked above")
                    .try_lock("the Eidos window")
                {
                    Ok(l) => l,
                    Err(e) => {
                        app.status = Some(format!("Busy: {e}."));
                        return Task::none();
                    }
                };
                // A directory goes whole, which is why this needed two clicks.
                let md = match path.symlink_metadata() {
                    Ok(md) => md,
                    Err(e) => {
                        app.error = Some(format!("Could not delete {rel}: {e}"));
                        return Task::none();
                    }
                };
                if md.is_dir() && !md.file_type().is_symlink() {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                }
            };
            match r {
                Ok(()) => {
                    refresh_after_tree_change(app);
                    app.status = Some(format!("Deleted {rel}."));
                }
                Err(e) => app.error = Some(format!("Could not delete {rel}: {e}")),
            }
        }
        Message::FiletreeNewFolderStart(i) => {
            // The mod is captured HERE. Reading `app.info_mod` at commit time
            // put the folder in whichever mod the panel happened to be showing
            // by then, which is not the one the button was pressed in.
            let Some(name) = app.mods.get(i).map(|m| m.name.clone()) else {
                return Task::none();
            };
            app.tree_new_folder = Some((name, String::new()));
            app.tree_rename = None;
            app.tree_delete_armed = None;
        }
        Message::FiletreeNewFolderChanged(t) => {
            app.typing = true;
            if let Some((mod_name, _)) = app.tree_new_folder.take() {
                app.tree_new_folder = Some((mod_name, t));
            }
        }
        Message::FiletreeNewFolderCommit => {
            let Some((mod_name, name)) = app.tree_new_folder.take() else {
                return Task::none();
            };
            let name = name.trim().to_string();
            let Some(base) = app
                .mods
                .iter()
                .find(|m| m.name == mod_name)
                .map(|m| m.path.clone())
            else {
                return Task::none();
            };
            let Some(path) = resolve_in_mod(&base, &name) else {
                app.error = Some(format!("Refused to create {name}: not a folder name."));
                return Task::none();
            };
            let r = {
                let _lock = match app
                    .created
                    .as_ref()
                    .expect("checked above")
                    .try_lock("the Eidos window")
                {
                    Ok(l) => l,
                    Err(e) => {
                        app.status = Some(format!("Busy: {e}."));
                        return Task::none();
                    }
                };
                std::fs::create_dir_all(&path)
            };
            match r {
                Ok(()) => {
                    refresh_after_tree_change(app);
                    app.status = Some(format!("Created {name}/."));
                }
                Err(e) => app.error = Some(format!("Could not create {name}: {e}")),
            }
        }
        // Nothing to store. Every animated value is a function of the instant
        // its phase began, so a frame's whole job is to have arrived: reaching
        // `update` is what makes iced call `view` again.
        Message::ArchivePoll => poll_archive_conflicts(app),
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
