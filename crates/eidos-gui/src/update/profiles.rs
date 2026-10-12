//! Profiles: switching, creating, renaming, copying and deleting them, and the
//! load-order backups.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::SwitchProfile(name) => {
            // Refused while the game runs: the run's post-exit steps write into
            // the profile that was LAUNCHED, and the profile's plugins dir is
            // bind-mounted into the live session - switching under it corrupted
            // the profile that was never played.
            if app.running.is_some() {
                app.status = Some("Cannot switch profiles while the game is running.".to_string());
                return Task::none();
            }
            // One shared path (switch_to_profile) so the reload steps - incl.
            // recompute_counts, which this handler used to skip - never drift.
            if app.created.is_some() && switch_to_profile(app, &name) {
                app.status = Some(format!("Switched to profile '{name}'."));
            }
        }
        Message::NewProfile => {
            let created = app.created.as_ref().map(|inst| {
                let existing = inst.profiles();
                let mut n = existing.len() + 1;
                let mut name = format!("Profile {n}");
                while existing.contains(&name) {
                    n += 1;
                    name = format!("Profile {n}");
                }
                let src = inst.active();
                let ok = inst.profile(&name).create_from(&src).is_ok();
                (name, src.name, ok)
            });
            if let Some((name, src_name, true)) = created {
                if switch_to_profile(app, &name) {
                    app.status = Some(format!("Created '{name}' (copy of '{src_name}')."));
                }
            }
        }
        // ---- profile management (rename / delete / named copy) --------------
        Message::ProfileMenuOpen(name) => {
            app.profile_menu = Some(name);
            app.menu_at = Some(app.cursor);
            app.profile_rename = None;
            app.profile_copy = None;
            app.profile_delete_confirm = None;
        }
        Message::ProfileCloseMenu => {
            app.menu_at = None;
            app.profile_menu = None;
            app.profile_rename = None;
            app.profile_copy = None;
            app.profile_delete_confirm = None;
        }
        Message::ProfileRenameStart(name) => {
            app.profile_rename = Some((name.clone(), name));
            app.profile_copy = None;
            app.profile_delete_confirm = None;
        }
        Message::ProfileRenameChanged(s) => {
            app.typing = true;
            if let Some((_, edited)) = &mut app.profile_rename {
                *edited = s;
            }
        }
        Message::ProfileRenameCommit => {
            if let (Some(inst), Some((old, edited))) = (&app.created, app.profile_rename.clone()) {
                let new = edited.trim().to_string();
                if new.is_empty() || new.contains('/') || new.contains('\\') {
                    app.status = Some("Invalid profile name.".to_string());
                } else if new == old {
                    // no-op: just close the editor
                    app.profile_rename = None;
                    app.profile_menu = None;
                } else if app.running.is_some() {
                    // A rename mid-run would pull the played profile out from
                    // under the session's post-exit steps (and the bound dirs).
                    app.status =
                        Some("Cannot rename a profile while the game is running.".to_string());
                } else if let Err(e) = probe_lock(inst) {
                    // app.running only sees runs THIS window started; the flock
                    // also covers a session launched from the CLI or Steam.
                    app.status = Some(format!("Cannot rename: {e}."));
                } else {
                    let was_active = inst.active_profile() == old;
                    match inst.rename_profile(&old, &new) {
                        Ok(()) => {
                            app.profile_rename = None;
                            app.profile_menu = None;
                            // rename_profile already followed the active pointer; reload
                            // the view when the renamed profile was the active one.
                            if !was_active || switch_to_profile(app, &new) {
                                app.status = Some(format!("Renamed profile to '{new}'."));
                            }
                        }
                        // Keep the editor open on a collision so the user can retype.
                        Err(e) => app.status = Some(format!("Rename failed: {e}")),
                    }
                }
            }
        }
        Message::ProfileCopyStart(name) => {
            // Prefill a free "<name> Copy" target so the editor never collides at once.
            let suggested = app
                .created
                .as_ref()
                .map(|inst| suggest_free_profile_name(inst, &format!("{name} Copy")))
                .unwrap_or_else(|| format!("{name} Copy"));
            app.profile_copy = Some((name, suggested));
            app.profile_rename = None;
            app.profile_delete_confirm = None;
        }
        Message::ProfileCopyChanged(s) => {
            app.typing = true;
            if let Some((_, edited)) = &mut app.profile_copy {
                *edited = s;
            }
        }
        Message::ProfileCopyCommit => {
            if let (Some(inst), Some((src_name, edited))) = (&app.created, app.profile_copy.clone())
            {
                let new = edited.trim().to_string();
                if new.is_empty() || new.contains('/') || new.contains('\\') {
                    app.status = Some("Invalid profile name.".to_string());
                } else if inst.profile(&new).dir().exists() {
                    app.status = Some(format!("Profile '{new}' already exists."));
                } else {
                    let src = inst.profile(&src_name);
                    let dest = inst.profile(&new);
                    match dest.create_from(&src) {
                        Ok(()) => {
                            app.profile_copy = None;
                            app.profile_menu = None;
                            if switch_to_profile(app, &new) {
                                app.status =
                                    Some(format!("Created '{new}' (copy of '{src_name}')."));
                            }
                        }
                        Err(e) => app.status = Some(format!("Copy failed: {e}")),
                    }
                }
            }
        }
        Message::ProfileDeleteConfirm(name) => {
            // First click arms; clicking the same profile again commits.
            app.profile_delete_confirm = Some(name);
            app.profile_rename = None;
            app.profile_copy = None;
        }
        Message::ProfileDeleteCommit(name) => {
            app.profile_delete_confirm = None;
            if let Some(inst) = &app.created {
                match inst.delete_profile(&name) {
                    Ok(()) => {
                        app.profile_menu = None;
                        app.status = Some(format!("Deleted profile '{name}'."));
                    }
                    // Backend guards the active / last profile; surface its reason.
                    Err(e) => app.status = Some(format!("Delete failed: {e}")),
                }
            }
        }
        Message::ShowBackupsDialog => {
            app.menu_mod = None;
            match load_backups(app) {
                Some(state) => app.backups = Some(state),
                None => app.status = Some("Open a game instance first.".to_string()),
            }
        }
        Message::CloseBackupsDialog => app.backups = None,
        Message::CreateBackup(kind) => {
            let Some(prof) = app.created.as_ref().map(|i| i.active()) else {
                return Task::none();
            };
            match prof.create_backup(kind) {
                Ok(b) => {
                    app.status = Some(format!("Backed up the {} ({}).", kind.label(), b.when()));
                    app.backups = load_backups(app);
                }
                Err(e) => app.status = Some(format!("Could not back up the {}: {e}", kind.label())),
            }
        }
        Message::RestoreBackup(kind, stamp) => {
            let Some(inst) = app.created.clone() else {
                return Task::none();
            };
            // Restoring rewrites the same files a running game is deploying
            // from, so it takes the instance lock like every other mutation.
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot restore now: {e}."));
                    return Task::none();
                }
            };
            match inst.active().restore_backup(kind, stamp) {
                Ok(()) => {
                    let when = eidos_instance::format_stamp(stamp);
                    app.status = Some(format!(
                        "Restored the {} from {when}. The state it replaced was backed up first.",
                        kind.label()
                    ));
                    app.backups = None;
                    drop(_lock);
                    // Both lists are read from disk again: the restore changed
                    // them underneath every cache the window holds.
                    reload_mods(app);
                    refresh_after_tree_change(app);
                    recompute_counts(app);
                }
                Err(e) => app.status = Some(format!("Could not restore the {}: {e}", kind.label())),
            }
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
