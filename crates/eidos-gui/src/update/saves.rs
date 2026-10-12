//! The Saves tab.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ---- Saves tab ----
        Message::RefreshSaves => {
            load_saves(app);
            app.status = Some(format!("Found {} save file(s).", app.saves.len()));
        }
        Message::DeleteSave(i) => {
            // First click arms the confirm; clicking a different row re-arms it.
            app.confirm_delete_save = Some(i);
        }
        Message::SaveToggleSelect(i) => {
            if !app.selected_saves.remove(&i) {
                app.selected_saves.insert(i);
            }
        }
        Message::SavesTick => {
            // Compare a fingerprint, do NOT reload. Reloading twice a second
            // would rebuild the list under the user's hands - dropping the
            // selection and closing the details pane every tick.
            let now = saves_fingerprint(app);
            if now != app.saves_fingerprint {
                // Everything the user has aimed at, by PATH: a new autosave
                // renumbers every index, so nothing index-shaped survives a
                // reload. The MULTI-selection matters as much as the focus - it
                // is what the batch bar is built on, and losing it mid-gesture
                // takes the bar off screen with the ticks in it.
                let focus = app
                    .selected_save
                    .and_then(|i| app.saves.get(i))
                    .map(|s| s.path.clone());
                let picked: Vec<std::path::PathBuf> = app
                    .selected_saves
                    .iter()
                    .filter_map(|&i| app.saves.get(i))
                    .map(|s| s.path.clone())
                    .collect();
                load_saves(app);
                app.selected_saves = app
                    .saves
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| picked.contains(&s.path))
                    .map(|(i, _)| i)
                    .collect();
                if let Some(p) = focus {
                    if let Some(i) = app.saves.iter().position(|s| s.path == p) {
                        app.selected_save = Some(i);
                        load_save_details(app);
                    }
                }
            }
        }
        Message::SavesDeleteSelected => {
            let targets: Vec<usize> = app.selected_saves.iter().copied().collect();
            if targets.is_empty() {
                return Task::none();
            }
            if !app.confirm_saves_delete {
                app.confirm_saves_delete = true;
                return Task::none();
            }
            app.confirm_saves_delete = false;
            // By PATH, collected before anything is removed: deleting by index
            // shifts every index after it, so the second removal would take the
            // wrong file.
            let paths: Vec<std::path::PathBuf> = targets
                .iter()
                .filter_map(|&i| app.saves.get(i))
                .map(|s| s.path.clone())
                .collect();
            let mut gone = 0usize;
            let mut failed = 0usize;
            for p in &paths {
                // The co-save travels with its save, and is removed even when the
                // save itself was already gone - otherwise it orphans invisibly.
                for co in eidos_instance::cosave_siblings(p) {
                    let _ = std::fs::remove_file(co);
                }
                match std::fs::remove_file(p) {
                    Ok(()) => gone += 1,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => gone += 1,
                    Err(_) => failed += 1,
                }
            }
            load_saves(app);
            app.status = Some(match failed {
                0 => format!("Deleted {gone} save(s)."),
                f => format!("Deleted {gone} save(s); {f} could not be removed."),
            });
        }
        Message::SavesCopyToProfile(target) => {
            let Some(inst) = app.created.clone() else {
                return Task::none();
            };
            let targets: Vec<usize> = app.selected_saves.iter().copied().collect();
            if targets.is_empty() {
                app.status = Some("Select the saves to copy first (Ctrl+click).".to_string());
                return Task::none();
            }
            let dest_dir = inst.profile(&target).saves_dir();
            if let Err(e) = std::fs::create_dir_all(&dest_dir) {
                app.status = Some(format!("Could not open profile '{target}': {e}"));
                return Task::none();
            }
            let mut copied = 0usize;
            let mut skipped = 0usize;
            let mut failed: Vec<String> = Vec::new();
            for i in targets {
                let Some(save) = app.saves.get(i) else {
                    continue;
                };
                // Every file that belongs to this save: the save itself and its
                // co-saves. Copying one without the others produces a save the
                // script extender cannot restore its state for.
                let mut group = vec![save.path.clone()];
                group.extend(eidos_instance::cosave_siblings(&save.path));
                // The guard is on the WHOLE group, checked before anything is
                // written. Per file it was worse than useless: a save that
                // already existed at the destination was skipped while its
                // co-save was still copied - landing beside a DIFFERENT
                // character's save under the same stem, which is a co-save that
                // silently belongs to the wrong game state.
                let clash = group
                    .iter()
                    .filter_map(|p| p.file_name())
                    .any(|n| dest_dir.join(n).exists());
                if clash {
                    skipped += 1;
                    continue;
                }
                let mut written: Vec<std::path::PathBuf> = Vec::new();
                let mut ok = true;
                for src in &group {
                    let Some(name) = src.file_name() else {
                        continue;
                    };
                    let dest = dest_dir.join(name);
                    // COPY, never move: this is somebody's character.
                    if let Err(e) = std::fs::copy(src, &dest) {
                        failed.push(format!("{}: {e}", name.to_string_lossy()));
                        ok = false;
                        break;
                    }
                    // The save's own timestamp, so it sorts where it belongs in
                    // the destination profile instead of jumping to the top as
                    // the newest thing there.
                    if let Ok(t) = std::fs::metadata(src).and_then(|m| m.modified()) {
                        let _ = set_file_mtime(&dest, t);
                    }
                    written.push(dest);
                }
                if ok {
                    copied += 1;
                } else {
                    // A half-copied group is a save the extender cannot restore
                    // state for. Undo it rather than leave one behind.
                    for p in written {
                        let _ = std::fs::remove_file(p);
                    }
                }
            }
            app.status = Some(match (copied, skipped, failed.len()) {
                (n, 0, 0) => format!("Copied {n} save(s) to '{target}'."),
                (n, s, 0) => format!(
                    "Copied {n} save(s) to '{target}'; {s} already existed there and were left alone."
                ),
                (n, _, f) => format!("Copied {n} save(s); {f} file(s) failed, e.g. {}", failed[0]),
            });
        }
        Message::SelectSave(i) => {
            // Clicking the open row closes the pane, so the list can go full width.
            if app.selected_save == Some(i) {
                clear_save_selection(app);
            } else {
                app.selected_save = Some(i);
                load_save_details(app);
            }
        }
        Message::FixSaveMods => {
            // Enable every mod that supplies one of the save's missing plugins.
            // MO2 stops at naming them; doing it is the whole point of knowing.
            let wanted: HashSet<String> = app
                .save_missing
                .iter()
                .flat_map(|m| m.providers.iter().cloned())
                .collect();
            let mut enabled = 0usize;
            for m in app.mods.iter_mut() {
                if !m.enabled && !m.is_backup() && wanted.contains(&m.name) {
                    m.enabled = true;
                    enabled += 1;
                }
            }
            if enabled == 0 {
                app.status =
                    Some("Those mods are already enabled - the plugins still need turning on in the Plugins tab.".to_string());
                return Task::none();
            }
            let saved = mods_changed(app);
            // The plugin list changed shape, so the save's diff has to be redone
            // against it rather than left showing the old answer. But
            // mods_changed just INVALIDATED that list, and on the Saves tab
            // nothing recomputes it (invalidate_plugins only repopulates while
            // the Plugins tab is open) - load_save_details reads "no list" as
            // "nothing missing", so the recheck below always declared victory
            // and the 'still need enabling' branch was unreachable. Recompute
            // first, so the verdict is judged against the NEW plugin set.
            if app.plugins.is_none() && app.created.is_some() {
                app.plugins = compute_plugins(app);
            }
            load_save_details(app);
            // A refused save left its reason in the status; keep it there.
            if !saved {
                return Task::none();
            }
            let left = app.save_missing.len();
            app.status = Some(if left == 0 {
                format!("Enabled {enabled} mod(s); this save's plugins are all available now.")
            } else {
                format!("Enabled {enabled} mod(s); {left} plugin(s) still need enabling in the Plugins tab.")
            });
        }
        Message::RestorePreSessionPlugins => {
            // Same gates as every other mutation: the plugins dir is bind-mounted
            // into a running session, and restoring under the game's feet races
            // its own writes.
            if app.running.is_some() {
                app.status = Some("Cannot restore while the game is running.".to_string());
                return Task::none();
            }
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot restore: {e}."));
                    return Task::none();
                }
            };
            match inst.active().restore_plugin_snapshot() {
                Ok(()) => {
                    // The on-disk state changed under the in-memory list: recompute
                    // rather than patch, same as every other external change.
                    app.plugins = compute_plugins(app);
                    app.status = Some("Restored the pre-session plugin order.".to_string());
                }
                Err(e) => {
                    app.status = Some(format!("Could not restore the pre-session order: {e}"));
                }
            }
        }
        Message::AcceptPluginState => {
            if app.running.is_some() {
                app.status = Some("Cannot do that while the game is running.".to_string());
                return Task::none();
            }
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot do that: {e}."));
                    return Task::none();
                }
            };
            app.status = Some(match inst.active().snapshot_plugin_state() {
                Ok(()) => "Kept the current plugin set; the warning is cleared.".to_string(),
                Err(e) => format!("Could not accept the current set: {e}"),
            });
        }
        Message::ConfirmDeleteSave(i) => {
            // Only act on the armed row, and re-check the index (the list may have
            // shifted if the file vanished out from under us).
            if app.confirm_delete_save == Some(i) {
                if let Some(save) = app.saves.get(i) {
                    let name = save.filename.clone();
                    match std::fs::remove_file(&save.path) {
                        Ok(()) => {
                            // The co-save travels with its save: leaving it made
                            // an orphan the Saves tab cannot show, the user
                            // cannot delete, and the cloud sync pushed forever.
                            for co in eidos_instance::cosave_siblings(&save.path) {
                                let _ = std::fs::remove_file(co);
                            }
                            app.status = Some(format!("Deleted save '{name}'."))
                        }
                        // Already gone is success enough; surface real errors.
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            // The save may be gone while its co-save is not
                            // (deleted in-game, which knows nothing of co-saves):
                            // clean those up here too or they orphan invisibly.
                            for co in eidos_instance::cosave_siblings(&save.path) {
                                let _ = std::fs::remove_file(co);
                            }
                            app.status = Some(format!("Save '{name}' was already gone."));
                        }
                        Err(e) => app.status = Some(format!("Could not delete '{name}': {e}")),
                    }
                }
                load_saves(app);
            }
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
