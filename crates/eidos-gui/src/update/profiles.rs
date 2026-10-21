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
            if let Some(r) = &app.running {
                app.status = Some(tf("status_switch_while_running", &[("title", &r.title)]));
                return Task::none();
            }
            // One shared path (switch_to_profile) so the reload steps - incl.
            // recompute_counts, which this handler used to skip - never drift.
            if app.created.is_some() && switch_to_profile(app, &name) {
                app.status = Some(tf("status_switched_profile", &[("name", &name)]));
            }
        }
        Message::NewProfile => {
            let created = app.created.as_ref().map(|inst| {
                let existing = inst.profiles();
                let mut n = existing.len() + 1;
                let mut name = tf("new_profile_name", &[("n", &n)]);
                while existing.contains(&name) {
                    n += 1;
                    name = tf("new_profile_name", &[("n", &n)]);
                }
                let src = inst.active();
                let ok = inst.profile(&name).create_from(&src).is_ok();
                (name, src.name, ok)
            });
            if let Some((name, src_name, true)) = created {
                if switch_to_profile(app, &name) {
                    app.status = Some(tf(
                        "status_profile_copied",
                        &[("name", &name), ("source", &src_name)],
                    ));
                }
            }
        }
        // ---- profile management (rename / delete / named copy) --------------
        Message::ProfileMenuOpen(name) => {
            app.profile_menu = Some(name);
            app.menu_at = Some(crate::subscription::pointer(app));
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
                    app.status = Some(t("status_invalid_profile_name").to_string());
                } else if new == old {
                    // no-op: just close the editor
                    app.profile_rename = None;
                    app.profile_menu = None;
                } else if let Some(r) = &app.running {
                    // A rename mid-run would pull the played profile out from
                    // under the session's post-exit steps (and the bound dirs).
                    app.status = Some(tf(
                        "status_rename_profile_while_running",
                        &[("title", &r.title)],
                    ));
                } else if let Err(e) = probe_lock(inst) {
                    // app.running only sees runs THIS window started; the flock
                    // also covers a session launched from the CLI or Steam.
                    app.status = Some(tf("status_cannot_rename", &[("error", &e)]));
                } else {
                    let was_active = inst.active_profile() == old;
                    match inst.rename_profile(&old, &new) {
                        Ok(()) => {
                            app.profile_rename = None;
                            app.profile_menu = None;
                            // rename_profile already followed the active pointer; reload
                            // the view when the renamed profile was the active one.
                            if !was_active || switch_to_profile(app, &new) {
                                app.status = Some(tf("status_profile_renamed", &[("name", &new)]));
                            }
                        }
                        // Keep the editor open on a collision so the user can retype.
                        Err(e) => app.status = Some(tf("status_rename_failed", &[("error", &e)])),
                    }
                }
            }
        }
        Message::ProfileCopyStart(name) => {
            // Prefill a free "<name> Copy" target so the editor never collides at once.
            let suggested = app
                .created
                .as_ref()
                .map(|inst| {
                    suggest_free_profile_name(inst, &tf("profile_copy_name", &[("name", &name)]))
                })
                .unwrap_or_else(|| tf("profile_copy_name", &[("name", &name)]));
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
                    app.status = Some(t("status_invalid_profile_name").to_string());
                } else if inst.profile(&new).dir().exists() {
                    app.status = Some(tf("status_profile_exists", &[("name", &new)]));
                } else {
                    let src = inst.profile(&src_name);
                    let dest = inst.profile(&new);
                    match dest.create_from(&src) {
                        Ok(()) => {
                            app.profile_copy = None;
                            app.profile_menu = None;
                            if switch_to_profile(app, &new) {
                                app.status = Some(tf(
                                    "status_profile_copied",
                                    &[("name", &new), ("source", &src_name)],
                                ));
                            }
                        }
                        Err(e) => app.status = Some(tf("status_copy_failed", &[("error", &e)])),
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
                        app.status = Some(tf("status_profile_deleted", &[("name", &name)]));
                    }
                    // Backend guards the active / last profile; surface its reason.
                    Err(e) => app.status = Some(tf("status_delete_failed_plain", &[("error", &e)])),
                }
            }
        }
        Message::ShowBackupsDialog => {
            app.menu_mod = None;
            match load_backups(app) {
                Some(state) => app.backups = Some(state),
                None => app.status = Some(t("status_open_instance_first").to_string()),
            }
        }
        Message::CloseBackupsDialog => app.backups = None,
        Message::CreateBackup(kind) => {
            let Some(prof) = app.created.as_ref().map(|i| i.active()) else {
                return Task::none();
            };
            match prof.create_backup(kind) {
                Ok(b) => {
                    app.status = Some(tf(
                        "status_backed_up",
                        &[("what", &backup_what(kind)), ("when", &b.when())],
                    ));
                    app.backups = load_backups(app);
                }
                Err(e) => {
                    app.status = Some(tf(
                        "status_back_up_failed",
                        &[("what", &backup_what(kind)), ("error", &e)],
                    ))
                }
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
                    app.status = Some(tf("status_cannot_restore_now", &[("error", &e)]));
                    return Task::none();
                }
            };
            match inst.active().restore_backup(kind, stamp) {
                Ok(()) => {
                    let when = eidos_instance::format_stamp(stamp);
                    app.status = Some(tf(
                        "status_backup_restored",
                        &[("what", &backup_what(kind)), ("when", &when)],
                    ));
                    app.backups = None;
                    drop(_lock);
                    // Both lists are read from disk again: the restore changed
                    // them underneath every cache the window holds. No mod
                    // folder changed, so the file walks stay; the views derived
                    // from the list and the plugin order are recomputed.
                    reload_mods(app);
                    mod_views_changed(app);
                }
                Err(e) => {
                    app.status = Some(tf(
                        "status_backup_restore_failed",
                        &[("what", &backup_what(kind)), ("error", &e)],
                    ))
                }
            }
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}

/// What a backup of `kind` holds, as the status line names it.
fn backup_what(kind: eidos_instance::BackupKind) -> &'static str {
    match kind {
        eidos_instance::BackupKind::ModList => t("backup_what_mod_list"),
        eidos_instance::BackupKind::LoadOrder => t("backup_what_load_order"),
    }
}
