//! The message handler: every `Message` the GUI can receive, and what it does to
//! `App`.
//!
//! Split out of `main.rs` unchanged. This is the half of the program that
//! DECIDES; everything under `view` only draws. Keeping them in one file made
//! both harder to find.

use crate::*;

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
    refresh_diagnostics(app);
    // A multi-file drop is walked one file at a time, because each install can
    // open a modal that has to be answered before the next archive is touched.
    // Re-armed here rather than at the end of every install path, because the
    // ones that matter - FOMOD, the manual picker, the collision prompt - finish
    // in six different places and would each have to remember.
    if draining
        && !app.dropped.is_empty()
        && app.fomod.is_none()
        && app.picker.is_none()
        && app.collision.is_none()
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
    // A confirmation is armed by the first click and cancelled by any other
    // ACTION - including arming a different row. Ambient messages are not
    // actions and must leave it standing.
    if !is_ambient(app, &message) {
        // Any action other than a second Clear click cancels the clear confirmation.
        if !matches!(message, Message::ClearOverwrite) {
            app.confirm_clear = false;
        }
        if !matches!(message, Message::DeleteSave(_) | Message::ConfirmDeleteSave(_)) {
            app.confirm_delete_save = None;
        }
        if !matches!(message, Message::DeleteDownload(_) | Message::ConfirmDeleteDownload(_)) {
            app.confirm_delete_download = None;
        }
        // The batch-remove confirmation is armed by the first click; any other
        // action (including merely re-rendering on a modifier change) cancels it.
        if !matches!(message, Message::BatchRemoveMods | Message::ConfirmBatchRemove) {
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
        if !matches!(message, Message::PurgeInstalledDownloads | Message::ConfirmPurgeInstalled) {
            app.confirm_purge_installed = false;
        }
        if !matches!(message, Message::FiletreeDelete(..) | Message::ConfirmFiletreeDelete(..)) {
            app.tree_delete_armed = None;
        }
        if !matches!(message, Message::ModRestoreBackup(_) | Message::ConfirmModRestoreBackup(_)) {
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
        Message::Next => {
            app.screen = match app.screen {
                Screen::Welcome => Screen::Kind,
                Screen::Kind => Screen::Game,
                Screen::Game => Screen::NameLoc,
                Screen::NameLoc => Screen::Summary,
                other => other,
            };
        }
        Message::Back => {
            app.screen = match app.screen {
                Screen::Kind => Screen::Welcome,
                Screen::Game => Screen::Kind,
                Screen::NameLoc => Screen::Game,
                Screen::Summary => Screen::NameLoc,
                other => other,
            };
        }
        Message::PickKind(k) => app.kind = k,
        Message::PickGame(i) => {
            app.selected = Some(i);
            if app.name.trim().is_empty() {
                if let Some(g) = app.games.get(i) {
                    app.name = g.def.name.to_string();
                }
            }
        }
        Message::NameChanged(s) => app.name = s,
        Message::PortableChanged(s) => app.portable_path = s,
        Message::Finish => {
            if let Some(inst) = planned_instance(app) {
                let game_id = selected_game(app).map(|g| g.def.id.to_string());
                // Never inside a game's install - ANY detected game's, not just
                // the selected one. Steam owns those trees (an update, a
                // "verify integrity" or an uninstall rewrites or deletes them,
                // taking the whole modding setup along), and Eidos mounts over
                // the game root, so an instance in there would sit inside its
                // own mount target. MO2 veterans reflexively put the manager
                // in the game folder; this is the place to stop them.
                let inside = app
                    .games
                    .iter()
                    .find(|g| Instance::root_inside_game(&inst.root, &g.install_path));
                // Adopting an EXISTING folder must not relabel it. A root whose
                // manifest names another game is opened as that game or not at
                // all - `ensure_manifest` would silently keep the old game id
                // while everything else (mod list, plugins, launch) treated the
                // folder as the wizard's selection.
                let foreign = inst
                    .read_manifest()
                    .filter(|m| game_id.as_deref().is_some_and(|id| m.game_id != id));
                if let Some(g) = inside {
                    app.error = Some(format!(
                        "'{}' is inside {}'s own folder. Steam owns that tree (an update or \
                         uninstall can wipe it) and Eidos mounts over the game root, so the \
                         instance would live inside its own mount target. Put it NEXT to the \
                         game instead - a sibling folder on the same drive works great.",
                        inst.root.display(),
                        g.def.name
                    ));
                } else if let Some(m) = foreign {
                    app.error = Some(format!(
                        "'{}' already holds a '{}' instance. Pick that game in the wizard, or choose another folder.",
                        inst.root.display(),
                        m.game_id
                    ));
                } else {
                    let kind = app.kind;
                    match inst.create() {
                        Ok(()) => {
                            if let Some(id) = &game_id {
                                let _ = inst.ensure_manifest(id, kind);
                                // Into the registry BEFORE opening: a portable
                                // root that is never recorded is orphaned at
                                // the next start - the original portable bug.
                                remember_open(&inst, id);
                            }
                            open_instance(app, inst);
                        }
                        Err(e) => app.error = Some(e.to_string()),
                    }
                }
            }
        }
        Message::OpenKnown(i) => {
            // The welcome screen's "open existing" list - the instance
            // switcher. The entry was built from the registry at a different
            // moment, so re-check the root before committing to it.
            if let Some(k) = app.known.get(i).cloned() {
                if let (true, Some(g)) = (k.inst.exists(), app.games.get(k.game_index)) {
                    let id = g.def.id;
                    app.selected = Some(k.game_index);
                    let _ = k.inst.ensure_manifest(id, InstanceKind::Global);
                    let _ = k.inst.ensure_profiles();
                    remember_open(&k.inst, id);
                    open_instance(app, k.inst);
                } else {
                    app.status = Some(format!(
                        "'{}' is not reachable right now (moved folder, unmounted drive?).",
                        k.inst.root.display()
                    ));
                }
            }
        }
        Message::Restart => {
            // Restart IS the instance switcher: the welcome screen's known
            // list must reflect instances created since the app started.
            app.known = known_instances(&app.games);
            app.selected = None;
            app.name.clear();
            app.portable_path.clear();
            app.created = None;
            app.error = None;
            app.mods.clear();
            app.status = None;
            app.kind = InstanceKind::Global;
            app.fomod = None;
            app.screen = Screen::Welcome;
        }
        Message::ToggleMod(i) => {
            // A backup contributes nothing to the game whatever modlist.txt
            // says, so ticking one would write `+X_backup` into a file that
            // then disagrees with the inert checkbox it was drawn as - and a
            // bulk toggle reaches this same arm.
            if app.mods.get(i).is_some_and(|m| m.is_backup()) {
                return Task::none();
            }
            // A separator is a group divider, not content - it has no toggle (MO2's
            // canBeEnabled() == false). Unmanaged rows are the game's own DLC and
            // Creation Club content: they are not in modlist.txt, so a flipped
            // flag would be silently lost on the next save, which reads as the
            // click having done nothing.
            if app.mods.get(i).is_some_and(|m| m.is_separator() || m.is_unmanaged()) {
                return Task::none();
            }
            if let Some(m) = app.mods.get_mut(i) {
                m.enabled = !m.enabled;
            }
            mods_changed(app);
        }
        Message::SelectTab(t) => {
            app.tab = t;
            if t == Tab::Plugins && app.plugins.is_none() {
                app.plugins = compute_plugins(app);
                // Opening this tab FILLS the active plugin set - including
                // replacing the "we have not looked" the Archives tab memoised
                // while it was absent. That memo is exactly what this click is
                // meant to fix, so it must not survive the click.
                plugin_state_changed(app);
            }
            if t == Tab::Conflicts && app.conflicts.is_none() {
                app.conflicts = compute_conflicts(app);
            }
            // Lazily fill the Saves / Downloads caches the first time each tab opens.
            if t == Tab::Saves && app.saves.is_empty() {
                load_saves(app);
            }
            // Downloads reloads on EVERY visit, not just the first: the point of
            // the tab is now what is happening right now, and the tick that keeps
            // it fresh only runs while the tab is open - so arriving here with a
            // list built minutes ago would show a stale picture for a whole tick.
            if t == Tab::Downloads {
                load_downloads(app);
            }
        }
        Message::SwitchProfile(name) => {
            // Refused while the game runs: the run's post-exit steps write into
            // the profile that was LAUNCHED, and the profile's plugins dir is
            // bind-mounted into the live session - switching under it corrupted
            // the profile that was never played.
            if app.running.is_some() {
                app.status =
                    Some("Cannot switch profiles while the game is running.".to_string());
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
            if let (Some(inst), Some((src_name, edited))) = (&app.created, app.profile_copy.clone()) {
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
        Message::InstallMod => {
            // Open a native file picker off-thread; the result comes back as ModPicked.
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .add_filter("Mod archives", &["7z", "zip", "rar"])
                    .set_title("Select a mod archive to install")
                    .pick_file(),
                |handle| Message::ModPicked(handle.map(|h| h.path().to_path_buf())),
            );
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
            let game_id = selected_game(app).map(|g| g.def.id.to_string());
            let mods_dir = app.created.as_ref().map(|i| i.mods_dir());
            let (Some(gid), Some(mods_dir)) = (game_id, mods_dir) else {
                return Task::none();
            };
            let name = eidos_install::mod_name_for(&path);
            // One extraction, then classify: a plain archive installs straight from
            // the extracted tree instead of being unpacked a second time.
            match eidos_install::open_archive(&path, &mods_dir, &name, &gid) {
                Ok(eidos_install::Opened::Fomod(session)) => {
                    let enabled_roots: Vec<std::path::PathBuf> =
                        app.mods.iter().filter(|m| m.is_active()).map(|m| m.path.clone()).collect();
                    let disabled_roots: Vec<std::path::PathBuf> =
                        app.mods.iter().filter(|m| !m.is_active() && !m.is_separator()).map(|m| m.path.clone()).collect();
                    let ctx = match selected_game(app) {
                        Some(g) => eidos_install::fomod_context(&g.data_path, &enabled_roots, &disabled_roots),
                        None => eidos_fomod::Context::default(),
                    };
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
                        app.fomod =
                            Some(FomodWizard {
                            session,
                            step: first,
                            selection,
                            game_id: gid,
                            archive: path,
                            ctx,
                            hover: None,
                        });
                        app.status = Some("FOMOD installer: choose your options, then Install.".to_string());
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
                Ok(eidos_install::Opened::Bain { tree, subpackages, invalid }) => {
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
                        rows: archive_tree.flatten(),
                        archive_tree,
                        archive: path,
                        name,
                        game_id: gid,
                        tree,
                        // `invalid` folders are MO2's cue to ASK rather than assume.
                        mode: PickerMode::Bain { subpackages, picked, asking: invalid > 0 },
                    });
                }
                // No heuristic recognised the layout. Rather than refuse the
                // archive, show its tree and let the user point at the data root.
                Ok(eidos_install::Opened::Manual(tree)) => {
                    app.status =
                        Some(format!("'{name}': pick the folder that holds the game data."));
                    let archive_tree = parsed_tree(&tree);
                    app.picker = Some(InstallPicker {
                        rows: archive_tree.flatten(),
                        archive_tree,
                        archive: path,
                        name,
                        game_id: gid,
                        tree,
                        mode: PickerMode::Manual { root: String::new() },
                    });
                }
                Err(e) => app.status = Some(format!("Install failed: {e}")),
            }
        }
        Message::PickerBainToggle(i) => {
            if let Some(PickerMode::Bain { picked, .. }) = app.picker.as_mut().map(|p| &mut p.mode) {
                if let Some(b) = picked.get_mut(i) {
                    *b = !*b;
                }
            }
        }
        Message::PickerBainConfirm(yes) => {
            let Some(p) = app.picker.as_mut() else { return Task::none() };
            match (&mut p.mode, yes) {
                (PickerMode::Bain { asking, .. }, true) => *asking = false,
                // "No, it is not a BAIN package": same extraction, manual picker.
                (PickerMode::Bain { .. }, false) => {
                    p.mode = PickerMode::Manual { root: String::new() };
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
                let gtype =
                    w.session.config.steps.get(si).and_then(|s| s.groups.get(gi)).map(|g| g.group_type);
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
                let vis = eidos_fomod::visible_steps(
                    &w.session.config,
                    &w.selection,
                    &w.ctx,
                );
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
                let vis = eidos_fomod::visible_steps(
                    &w.session.config,
                    &w.selection,
                    &w.ctx,
                );
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
            let Some(mods_dir) = app.created.as_ref().map(|i| i.mods_dir()) else {
                return Task::none();
            };
            // Collision check BEFORE consuming the wizard: a reinstall must offer
            // Merge/Replace/Rename (MO2's QueryOverwriteDialog) with the user's
            // choices intact, not dead-end and discard them.
            if let Some(w) = app.fomod.as_ref() {
                if let Some(name) = eidos_install::collision_name(&mods_dir, w.session.mod_name()) {
                    let rename_to = suggest_free_name(&mods_dir, &name);
                    app.collision = Some(CollisionPrompt {
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
        Message::ToolPicked(choice) => {
            app.tool_choice = (choice != RUN_GAME).then_some(choice);
        }
        Message::Run => {
            if app.running.is_some() {
                // Already waiting on a launched application (MO2 won't launch a
                // second one while locked); ignore the repeat Run.
                // Unlock only drops the overlay - it deliberately KEEPS the run
                // tracked so the post-exit refresh still happens - so telling the
                // user to unlock was advice that could not work.
                let what = app.running.as_ref().map(|r| r.title.clone()).unwrap_or_default();
                app.status = Some(format!(
                    "{what} is still running. Eidos re-enables launching and LOOT sorting when it exits."
                ));
            } else if let Some(title) = app.tool_choice.clone() {
                // A tool: the CLI resolves Proton itself, no Steam command needed.
                // The owned arg ends the immutable borrow before `start_run`.
                if let Some(arg) = instance_arg(app) {
                    let cmd = tool_command(&arg, &title);
                    start_run(app, title, cmd);
                } else {
                    app.status = Some("Create or open an instance first.".to_string());
                }
            } else if app.launch_command.is_empty() {
                // Standalone: we don't have Steam's Proton command, so we cannot
                // build the launch environment. Point the user at the option, with
                // this binary's absolute path (Steam's launch options don't see
                // ~/.cargo/bin on PATH) and native d3dcompiler forced so the game's
                // shader compilation works under Proton. Eidos merges that with any
                // mod-shipped DLL overrides at launch.
                let exe = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.to_str().map(str::to_string))
                    .unwrap_or_else(|| "eidos-gui".to_string());
                app.status = Some(format!(
                    "Set the game's Steam launch option to:  WINEDLLOVERRIDES=\"d3dcompiler_47=n\" {exe} %command%  then press Play in Steam (Eidos opens, then click Run)."
                ));
            } else if let (Some(game), Some(inst)) = (selected_game(app), &app.created) {
                let id = game.def.id;
                let game_name = game.def.name.to_string();
                // Soft advisory if an ENB (game root) and Community Shaders (an
                // enabled mod) are both active - prepended to the launch status,
                // never blocking. The CLI emits the same note to stderr, which the
                // GUI does not surface, so we recompute it here.
                let cs_roots: Vec<std::path::PathBuf> = inst
                    .modlist()
                    .into_iter()
                    .filter(|m| m.is_active())
                    .map(|m| m.path)
                    .collect();
                let both_active = eidos_gamefeatures::enb_cs_conflict(&game.install_path, &cs_roots);
                // How the child names the instance: the portable folder when one
                // is open, the game id otherwise.
                let inst_arg = instance_arg(app).unwrap_or_else(|| id.to_string());
                // `game`/`inst` are no longer used below; their borrows end here so
                // `start_run` can take `&mut app`.
                let (cmd, se_warning) = play_command(id, &inst_arg, &app.launch_command);
                start_run(app, game_name, cmd);
                // Prepend advisories to whatever status start_run set.
                for note in [se_warning, both_active.then(|| {
                    "Note: ENB and Community Shaders are both active (if visuals look wrong, disable one in its INI).".to_string()
                })].into_iter().flatten() {
                    if let Some(s) = app.status.take() {
                        app.status = Some(format!("{note} {s}"));
                    }
                }
            } else {
                app.status = Some("Create or open an instance first.".to_string());
            }
        }
        Message::PollRunning => {
            // The poll subscription fires while a launch is being waited on; once
            // the wait thread reports the child exited, unlock and refresh.
            let exited = app
                .running
                .as_ref()
                .map(|r| r.done.load(std::sync::atomic::Ordering::SeqCst))
                .unwrap_or(false);
            if exited {
                finish_run(app);
            }
        }
        Message::ForceUnlock => {
            // Drop the overlay but KEEP tracking (MO2 stops waiting entirely; we
            // keep the exit poll so the afterRun refresh still happens and the
            // game's own plugins.txt rewrite is never clobbered by stale GUI state).
            // The game is never killed.
            if let Some(r) = app.running.as_mut() {
                r.lock = false;
                let title = r.title.clone();
                app.status = Some(format!("Unlocked - {title} is still running."));
            }
        }
        Message::CloseLootReport => {
            app.loot_report = None;
        }
        Message::CopyLootReport => {
            // No report open means this arrived from the Ctrl+C shortcut with
            // nothing to copy; leaving the clipboard alone is the right answer.
            let Some(report) = &app.loot_report else { return Task::none() };
            let text = loot_report_text(report);
            let lines = text.lines().count();
            app.status = Some(format!("LOOT report copied ({lines} lines)."));
            return iced::clipboard::write(text);
        }
        Message::SendToFirstConflict(i) | Message::SendToLastConflict(i) => {
            let first = matches!(message, Message::SendToFirstConflict(_));
            app.menu_mod = None;
            let targets = selection_or(app, i);
            // The conflict sets are already computed for the emblems; reuse them.
            // Origins are `index + 1`, so BASE_ORIGIN (0, the game data) and the
            // Overwrite pseudo-layer (u32::MAX) are not rows and must be dropped.
            let mut related: Vec<usize> = Vec::new();
            if let Some(map) = app.conflicts.as_ref() {
                for &t in &targets {
                    let origin = (t + 1) as u32;
                    if let Some(mc) = map.mods.get(&origin) {
                        let set = if first { &mc.overwrites } else { &mc.overwritten_by };
                        related.extend(
                            set.iter()
                                .filter(|&&o| o != 0 && o != u32::MAX)
                                .map(|&o| (o - 1) as usize),
                        );
                    }
                }
            }
            let dest = if first { related.iter().min() } else { related.iter().max() };
            let Some(&dest) = dest else {
                app.status = Some(
                    if first { "This mod overrides nothing." } else { "Nothing overrides this mod." }
                        .to_string(),
                );
                return Task::none();
            };
            // "Just below the last mod that overrides it" is one slot past it.
            let dest = if first { dest } else { (dest + 1).min(app.mods.len()) };
            let hidden = hidden_by_folds(app);
            let at = move_block(&mut app.mods, &targets, dest);
            app.selected_mod = Some(at);
            app.selected_mods.clear();
            settle_folds_after_move(app, at, targets.len(), &hidden);
            mods_changed(app);
        }
        Message::SendToPriorityStart(i) => {
            // The menu STAYS open, exactly as `RenameStart` keeps it: the editor
            // this arms is drawn by `send_to_targets`, inside the menu card, which
            // `view` only renders while `menu_mod` is set. Clearing it here closed
            // the menu over the editor - so the item did nothing visible, and the
            // armed state then hijacked the next right-click on that row.
            app.menu_mod = Some(i);
            app.send_separator = None;
            // 1-BASED, because the number column is: the row the user reads as
            // '#7' must prefill '7', and typing '3' must land on the row whose
            // column says 3. The 0-based prefill contradicted the visible
            // number on arming, and every send landed one slot below the row
            // the user named, confirmed by a toast quoting a third numbering.
            app.send_priority = Some((i, (i + 1).to_string()));
        }
        Message::SendToPriorityChanged(text) => {
            app.typing = true;
            if let Some((_, t)) = app.send_priority.as_mut() {
                *t = text;
            }
        }
        Message::SendToPriorityCommit => {
            let Some((i, text)) = app.send_priority.take() else { return Task::none() };
            let Ok(dest) = text.trim().parse::<usize>() else {
                app.status = Some("Enter a priority number.".to_string());
                return Task::none();
            };
            let targets = selection_or(app, i);
            // Display numbering -> insertion index: '3' means "become the row
            // whose column reads 3", i.e. land at index 2.
            let dest = dest.saturating_sub(1).min(app.mods.len());
            let hidden = hidden_by_folds(app);
            let at = move_block(&mut app.mods, &targets, dest);
            app.selected_mod = Some(at);
            app.selected_mods.clear();
            settle_folds_after_move(app, at, targets.len(), &hidden);
            // The card hosting the editor is dismissed by the commit, not by the
            // click that armed it.
            app.menu_mod = None;
            mods_changed(app);
            // The toast quotes the same 1-based numbering the column shows.
            app.status = Some(format!("Moved to priority {}.", at + 1));
        }
        Message::SendToSeparatorStart(i) => {
            // Same as above: the chooser lives inside the menu card.
            app.menu_mod = Some(i);
            app.send_priority = None;
            app.send_separator = Some(i);
        }
        Message::SendToSeparatorPick(sep) => {
            let Some(i) = app.send_separator.take() else { return Task::none() };
            let targets = selection_or(app, i);
            // Land in the chosen separator's GROUP: the slot just before the next
            // separator, or the end of the list when it is the last group.
            let dest = app
                .mods
                .iter()
                .enumerate()
                .skip(sep + 1)
                .find(|(_, m)| m.is_separator())
                .map(|(idx, _)| idx)
                .unwrap_or(app.mods.len());
            let hidden = hidden_by_folds(app);
            let at = move_block(&mut app.mods, &targets, dest);
            app.selected_mod = Some(at);
            app.selected_mods.clear();
            settle_folds_after_move(app, at, targets.len(), &hidden);
            app.menu_mod = None;
            mods_changed(app);
        }
        Message::SendToTargetCancel => {
            app.send_priority = None;
            app.send_separator = None;
        }
        Message::ClearStatus => {
            app.status = None;
        }
        Message::OverwriteToModStart => {
            if app.created.as_ref().is_some_and(|i| i.overwrite_is_empty()) {
                app.status = Some("The Overwrite is empty - nothing to turn into a mod.".to_string());
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
        Message::OpenUrl(url) => {
            // Only ever hand a real web link to the browser.
            if url.starts_with("https://") || url.starts_with("http://") {
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                app.status = Some(format!("Opened {url}"));
            }
        }
        Message::ImportMo2Pick => {
            if app.created.is_none() {
                app.status = Some("Open a game instance first.".to_string());
                return Task::none();
            }
            app.profile_menu = None;
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_title("Select the MO2 profile folder (the one holding modlist.txt)")
                    .pick_folder(),
                |h| Message::ImportMo2Picked(h.map(|h| h.path().to_path_buf())),
            );
        }
        Message::ImportMo2Picked(picked) => {
            let Some(dir) = picked else { return Task::none() };
            // Same gates as every other mutation: the import rewrites the modlist
            // AND the plugin state dir, which is bind-mounted into a running
            // session - importing under the game's feet mixed the two states and
            // half-undid the import at the next launch.
            if app.running.is_some() {
                app.status =
                    Some("Cannot import while the game is running.".to_string());
                return Task::none();
            }
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot import: {e}."));
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
                    let mut s = format!("Imported {} mod(s) from MO2.", r.matched);
                    if r.plugin_files > 0 {
                        s.push_str(" Load order imported.");
                    }
                    if !r.missing.is_empty() {
                        s.push_str(&format!(
                            " {} mod(s) MO2 listed are not installed here (install them, then import again).",
                            r.missing.len()
                        ));
                    }
                    app.status = Some(s);
                }
                Err(e) => app.status = Some(format!("MO2 import failed: {e}")),
            }
        }
        Message::OverwriteToModCommit => {
            let Some(name) = app.overwrite_to_mod.take().map(|s| s.trim().to_string()) else {
                return Task::none();
            };
            let Some(inst) = app.created.as_ref() else { return Task::none() };
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
            match inst.overwrite_into_mod(&name) {
                Ok(dest) => {
                    // Highest priority (the end of the display order), which is where
                    // the Overwrite's content effectively sat.
                    if !app.mods.iter().any(|m| m.name == name) {
                        app.mods.push(ModEntry { name: name.clone(), enabled: true, path: dest, unmanaged: false });
                    }
                    drop_files_cache(app, None);
                    mods_changed(app);
                    app.status = Some(if existing {
                        format!("Moved the Overwrite into '{name}'.")
                    } else {
                        format!("Created mod '{name}' from the Overwrite.")
                    });
                }
                Err(e) => {
                    app.status = Some(format!("Could not create the mod: {e}"));
                    // Keep the prompt open so the name can be fixed.
                    app.overwrite_to_mod = Some(name);
                }
            }
        }
        Message::Refresh => {
            if app.created.is_some() {
                reload_mods(app);
                // F5 = full re-scan: every cached file walk may be stale.
                drop_files_cache(app, None);
                invalidate_plugins(app);
                app.conflicts = compute_conflicts(app);
                // Refresh is the "re-read everything from disk" affordance, and
                // the only place that pays the full meta scan on purpose.
                app.meta_cache.clear();
                refresh_meta_cache(app);
                recompute_counts(app);
                // The list was rebuilt; selection / drag indices no longer hold.
                app.selected_mods.clear();
                app.drag_state = None;
                app.status = Some("Refreshed mod list.".to_string());
            }
            load_tools(app);
            // F5 is also the "I just ran setcap" recheck for the warning banner.
            app.cap_missing = !eidos_launch::binary_has_cap_sys_admin(&find_eidos_binary());
        }
        Message::OpenFolder(p) => {
            // Every other entry in either dropdown dismisses it. Leaving this
            // one up means coming back from the file manager to a card sitting
            // over the window with a full-screen click catcher behind it.
            app.file_menu_open = false;
            app.view_menu_open = false;
            // Created if absent. Several of these are Eidos's own and are made on
            // first use - downloads before the first download, overwrite before
            // the first run - and "the folder you are looking for is not there
            // yet" is a worse answer than an empty folder.
            if !p.exists() {
                let _ = std::fs::create_dir_all(&p);
            }
            if !p.is_dir() {
                app.status = Some(format!("{} does not exist.", p.display()));
                return Task::none();
            }
            let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
            app.status = Some(format!("Opened {} in your file manager.", p.display()));
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
        Message::SearchChanged(q) => {
            app.typing = true;
            app.search = q;
            forget_hidden_rows(app);
        }
        Message::CategoryFilterChanged(id) => {
            app.category_filter = id;
            forget_hidden_rows(app);
        }
        Message::SelectMod(i) => {
            // A held modifier turns a plain click into a multi-select gesture (iced
            // can only fire a fixed `on_press` message, so we branch on the live
            // modifier state captured by the keyboard subscription).
            if app.modifiers.control() || app.modifiers.command() {
                return update(app, Message::SelectModToggle(i));
            }
            if app.modifiers.shift() {
                return update(app, Message::SelectModExtend(i));
            }
            // Plain click: single focus + collapse the multi-selection to just it,
            // and arm a potential drag from this row (committed only if it moves).
            app.focus = Pane::Mods;
            app.typing = false;
            app.selected_mod = Some(i);
            app.sel_anchor = Some(i);
            app.selected_mods.clear();
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            // Only in load order. Under a sort or a grouping the insertion gaps
            // address the REAL list while the rows on screen are somewhere
            // else, so a drop moves a mod nobody aimed at - and an armed drag
            // also hijacks the edge auto-scroll on its way to being refused.
            // Here, at the press, because this is where a row actually arms one:
            // a mod row's `on_press` is `DragStart`, never `SelectMod`.
            if !can_reorder(app) {
                return Task::none();
            }
            app.drag_state = Some(DragState { from: i, gap: i, aimed: false });
        }
        Message::SelectModToggle(i) => {
            // A modifier click is still a press on this list: it has to take the
            // keyboard, or the arrows would go on driving the other pane.
            app.focus = Pane::Mods;
            app.typing = false;
            // Ctrl+click: flip this row's membership; the first toggle also seeds the
            // set from the current focus so the anchor row stays selected.
            if app.selected_mods.is_empty() {
                if let Some(f) = app.selected_mod {
                    app.selected_mods.insert(f);
                }
            }
            if !app.selected_mods.remove(&i) {
                app.selected_mods.insert(i);
            }
            app.selected_mod = Some(i);
            app.sel_anchor = Some(i);
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = None;
        }
        Message::SelectModExtend(i) => {
            app.focus = Pane::Mods;
            app.typing = false;
            // Shift+click: select the contiguous run from the ANCHOR to `i`. The
            // anchor is not the focus - it stays where the selection began, so a
            // second Shift gesture grows the same run instead of starting a new
            // two-row one. With no anchor yet, behaves like a plain select.
            let anchor = app.sel_anchor.or(app.selected_mod).unwrap_or(i);
            // Pin it: the fallback above must be taken ONCE. Left unset, the next
            // Shift would fall back to the focus this gesture is about to move,
            // and the run would never grow past two rows.
            app.sel_anchor = Some(anchor);
            // The run is over the rows ON SCREEN, not over the raw index range:
            // once the list can be sorted or grouped the two differ, and a
            // Shift+click would select mods that are nowhere between the two
            // rows the user clicked - then a batch action would act on them.
            let drawn = drawn_mod_rows(app);
            app.selected_mods.clear();
            match (
                drawn.iter().position(|&r| r == anchor),
                drawn.iter().position(|&r| r == i),
            ) {
                (Some(a), Some(b)) => {
                    for &idx in &drawn[a.min(b)..=a.max(b)] {
                        app.selected_mods.insert(idx);
                    }
                }
                // One end is not drawn - the filter moved under it. Selecting a
                // run between a row on screen and one that is not would be a
                // selection nobody could see the shape of.
                _ => {
                    app.selected_mods.insert(i);
                }
            }
            app.selected_mod = Some(i);
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = None;
        }
        Message::ClearSelection => {
            // Escape reaches here. A modal on screen owns it: dismissing the LOOT
            // report is what the key means while it is up, not clearing a
            // selection the user cannot even see.
            if app.loot_report.is_some() {
                app.loot_report = None;
                return Task::none();
            }
            app.typing = false;
            app.confirm_remove = None;
            app.selected_mods.clear();
            app.selected_plugins.clear();
            app.drag_state = None;
            app.plugin_drag = None;
            app.drag_scroll = None;
            app.menu_mod = None;
            app.menu_plugin = None;
        }
        Message::OpenModMenu(i) => {
            // Right-clicking a row already in the multi-selection keeps the whole
            // set (MO2 batch context menu); right-clicking outside it selects just
            // that row first.
            if !app.selected_mods.contains(&i) {
                app.selected_mods.clear();
            }
            app.selected_mod = Some(i);
            app.menu_mod = Some(i);
            // Frozen here: the pointer keeps moving and a menu that followed it
            // would be impossible to aim at.
            app.menu_at = Some(app.cursor);
            app.rename = None;
            app.confirm_remove = None;
            app.drag_state = None;
            app.send_priority = None;
            app.send_separator = None;
        }
        Message::CloseMenu => {
            app.menu_at = None;
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            // An armed inline editor must not outlive the menu that hosts it, or
            // the next right-click on that row opens straight into it.
            app.send_priority = None;
            app.send_separator = None;
        }
        Message::ModSendTop(i) => {
            if i < app.mods.len() {
                let hidden = hidden_by_folds(app);
                let at = move_block(&mut app.mods, &[i], 0);
                app.selected_mod = Some(at);
                // Every other row's index shifted: a stale multi-selection here
                // could feed the wrong rows into a batch remove.
                app.selected_mods.clear();
                settle_folds_after_move(app, at, 1, &hidden);
                mods_changed(app);
            }
            app.menu_mod = None;
        }
        Message::ModSendBottom(i) => {
            if i < app.mods.len() {
                let end = app.mods.len();
                let hidden = hidden_by_folds(app);
                let at = move_block(&mut app.mods, &[i], end);
                app.selected_mod = Some(at);
                app.selected_mods.clear();
                settle_folds_after_move(app, at, 1, &hidden);
                mods_changed(app);
            }
            app.menu_mod = None;
        }
        Message::ModOpenFolder(i) => {
            app.menu_mod = None;
            if let Some(m) = app.mods.get(i) {
                let _ = std::process::Command::new("xdg-open").arg(&m.path).spawn();
                app.status = Some(format!("Opened '{}' in your file manager.", m.name));
            }
        }
        Message::ModVisitNexus(i) => {
            app.menu_mod = None;
            let domain = selected_game(app).map(|g| g.def.nexus_game).filter(|s| !s.is_empty());
            let mod_id = app.mods.get(i).and_then(|m| app.meta_cache.get(&m.name)).and_then(|r| r.mod_id);
            match (domain, mod_id) {
                (Some(domain), Some(id)) => {
                    let url = format!("https://www.nexusmods.com/{domain}/mods/{id}");
                    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                    app.status = Some(format!("Opening {url}"));
                }
                _ => {
                    app.status =
                        Some("No Nexus mod id on record for this mod (install it from Nexus to link it).".to_string());
                }
            }
        }
        Message::ModReinstall(i) => {
            app.menu_mod = None;
            if let Some(m) = app.mods.get(i) {
                app.status = Some(format!(
                    "Reinstalling '{}': pick the archive to install over it.",
                    m.name
                ));
            }
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .add_filter("Mod archives", &["7z", "zip", "rar"])
                    .set_title("Select the archive to reinstall")
                    .pick_file(),
                |handle| Message::ModPicked(handle.map(|h| h.path().to_path_buf())),
            );
        }
        Message::ModRemove(i) => {
            if app.confirm_remove == Some(i) {
                app.confirm_remove = None;
                app.menu_mod = None;
                // The flock, BEFORE the deletion - not after, where mods_changed's
                // own lock only refuses the save once the folder is already gone.
                // The mod directory is a LAYER of a live union while a session runs
                // (launched from a terminal, app.running knows nothing about it),
                // and deleting it mid-session takes the playing game's meshes and
                // scripts with it. Same guard, same wording as ToggleFileHidden.
                let Some(inst) = app.created.as_ref() else { return Task::none() };
                let _lock = match inst.try_lock("the Eidos window") {
                    Ok(l) => l,
                    Err(e) => {
                        app.status = Some(format!("Cannot remove a mod now: {e}."));
                        return Task::none();
                    }
                };
                if let Some(m) = app.mods.get(i).cloned() {
                    match fs::remove_dir_all(&m.path) {
                        Ok(()) => {
                            app.mods.remove(i);
                            app.selected_mod = None;
                            app.selected_mods.clear();
                            app.drag_state = None;
                            drop_files_cache(app, Some(&m.name));
                            mods_changed(app);
                            app.status = Some(format!("Removed '{}'.", m.name));
                        }
                        Err(e) => app.status = Some(format!("Remove failed: {e}")),
                    }
                }
            } else {
                app.confirm_remove = Some(i);
                if let Some(m) = app.mods.get(i) {
                    app.status =
                        Some(format!("Click Remove again to permanently delete '{}' from disk.", m.name));
                }
            }
        }
        Message::RenameStart(i) => {
            if let Some(m) = app.mods.get(i) {
                // Edit the display name; a separator's `_separator` suffix is stripped
                // for editing and re-applied on commit (MO2 getDisplayName/makeInternalName).
                app.rename = Some((i, m.display_name().to_string()));
                app.menu_mod = Some(i);
                // NOT re-anchored: this reopens the same menu around an inline
                // editor, and moving it to wherever the pointer had drifted
                // would yank it out from under the user mid-gesture.
                app.confirm_remove = None;
            }
        }
        Message::RenameChanged(s) => {
            app.typing = true;
            if let Some((_, name)) = &mut app.rename {
                *name = s;
            }
        }
        Message::RenameCommit => {
            if let Some((i, typed)) = app.rename.take() {
                app.menu_mod = None;
                let typed = typed.trim().to_string();
                let old = app.mods.get(i).cloned();
                if let Some(old) = old {
                    // A separator keeps its `_separator` suffix on disk + in modlist.txt.
                    let new_name =
                        if old.is_separator() { format!("{typed}_separator") } else { typed.clone() };
                    if typed.is_empty() || typed.contains('/') || typed.contains('\\') {
                        app.status = Some("Invalid name.".to_string());
                    } else if new_name == old.name {
                        // no-op
                    } else if let Some(mods_dir) = app.created.as_ref().map(|inst| inst.mods_dir()) {
                        let dest = mods_dir.join(&new_name);
                        if dest.exists() {
                            app.status = Some(format!("'{typed}' already exists."));
                        } else {
                            // Same flock as every sibling mutation: renaming the
                            // folder moves a live union layer out from under a
                            // running session, and the refused save afterwards
                            // would leave modlist.txt naming a folder that no
                            // longer exists.
                            let Some(inst) = app.created.as_ref() else {
                                return Task::none();
                            };
                            let _lock = match inst.try_lock("the Eidos window") {
                                Ok(l) => l,
                                Err(e) => {
                                    app.status =
                                        Some(format!("Cannot rename a mod now: {e}."));
                                    return Task::none();
                                }
                            };
                            match fs::rename(&old.path, &dest) {
                                Ok(()) => {
                                    if let Some(m) = app.mods.get_mut(i) {
                                        m.name = new_name.clone();
                                        m.path = dest;
                                    }
                                    // The cache is keyed by name; the old key is stale.
                                    drop_files_cache(app, Some(&old.name));
                                    // So is the fold state, which is keyed by DISPLAY
                                    // name. Left alone, a folded group springs open on
                                    // rename and the dead key is written back forever -
                                    // until some future separator happens to take that
                                    // name and folds itself the moment it is created.
                                    // `AddSeparator` opens this editor immediately, so
                                    // every separator passes through here.
                                    if old.is_separator()
                                        && app.collapsed.remove(old.display_name())
                                    {
                                        app.collapsed.insert(typed.clone());
                                        save_collapsed(app);
                                    }
                                    mods_changed(app);
                                    app.status = Some(format!("Renamed to '{typed}'."));
                                }
                                Err(e) => app.status = Some(format!("Rename failed: {e}")),
                            }
                        }
                    }
                }
            }
        }
        Message::AddSeparator(i) => {
            app.menu_mod = None;
            let mods_dir = app.created.as_ref().map(|inst| inst.mods_dir());
            if let Some(mods_dir) = mods_dir {
                // A unique "Separator N" display name -> folder "<name>_separator".
                let mut n = 1usize;
                let mut display = "Separator".to_string();
                while mods_dir.join(format!("{display}_separator")).exists() {
                    n += 1;
                    display = format!("Separator {n}");
                }
                let folder = format!("{display}_separator");
                let dest = mods_dir.join(&folder);
                match fs::create_dir_all(&dest) {
                    Ok(()) => {
                        // Minimal meta.ini, mirroring MO2's createMod.
                        let _ = fs::write(dest.join("meta.ini"), "[General]\nmodid=0\nversion=\n");
                        let idx = i.min(app.mods.len());
                        app.mods.insert(idx, ModEntry { name: folder, enabled: true, path: dest, unmanaged: false });
                        // Indices at/after the insertion point shifted.
                        app.selected_mods.clear();
                        mods_changed(app);
                        app.selected_mod = Some(idx);
                        // Open its rename editor so the user names it straight away.
                        app.rename = Some((idx, display));
                        app.menu_mod = Some(idx);
                    }
                    Err(e) => app.status = Some(format!("Could not create separator: {e}")),
                }
            }
        }
        Message::SetSeparatorColor(i, rgb) => {
            app.menu_mod = None;
            let result = match (app.mods.get(i).cloned(), app.created.as_ref()) {
                // Any mod, not just a separator. The colour was always stored per
                // mod in meta.ini and cached for every row; only the picker was
                // withheld. Unmanaged rows stay out: Eidos never writes into the
                // game's own Data.
                (Some(m), Some(inst)) if !m.is_unmanaged() => {
                    let mut meta = inst.mod_meta(&m.name);
                    meta.set_color(rgb);
                    Some((m.name.clone(), m.display_name().to_string(), meta.write(&inst.meta_path(&m.name))))
                }
                // Refused, and said out loud: a menu entry that does nothing at
                // all reads as a bug, not as a rule.
                (Some(m), _) if m.is_unmanaged() => {
                    app.status = Some(format!(
                        "'{}' is the game's own content - Eidos never writes into its Data.",
                        m.display_name()
                    ));
                    None
                }
                _ => None,
            };
            if let Some((changed, display, r)) = result {
                match r {
                    Ok(()) => {
                        // The colour lives in this mod's meta.ini: drop its row so
                        // the refresh below recomputes exactly that one.
                        invalidate_meta(app, &changed);
                        refresh_meta_cache(app);
                        app.status = Some(format!("Set the colour for '{display}'."));
                    }
                    Err(e) => app.status = Some(format!("Could not set colour: {e}")),
                }
            }
        }
        Message::ToggleCollapse(name) => {
            if !app.collapsed.remove(&name) {
                app.collapsed.insert(name);
            }
            save_collapsed(app);
            // Folding a group hides rows exactly like a filter does, and the
            // keyboard reads the focus without checking whether it is drawn.
            forget_hidden_rows(app);
        }
        Message::TogglePlugin(i) => {
            // Compute the spec + prefix dir up front (immutable borrows of `app`)
            // before mutating `app.plugins`.
            let spec = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id));
            let name = app.plugins.as_ref().and_then(|l| l.plugins.get(i)).map(|p| p.name.clone());
            let forced = app.plugins.as_ref().and_then(|l| l.plugins.get(i)).map(|p| p.force_disabled).unwrap_or(false);
            let implicit = app
                .plugins
                .as_ref()
                .and_then(|l| l.plugins.get(i).map(|p| l.implicit.contains(&p.name.to_ascii_lowercase())))
                .unwrap_or(false);
            if let (Some(spec), Some(name)) = (spec, name) {
                // Base-game masters are implicit and always loaded; refuse to toggle.
                if spec.primary_plugins.iter().any(|p| p.eq_ignore_ascii_case(&name)) {
                    app.status = Some(format!("{name} is a base-game master and is always loaded."));
                } else if implicit {
                    // Creation Club content the engine loads from the .ccc file.
                    // It is deliberately kept out of plugins.txt (writing it in
                    // makes the game see every Creation twice and blank the
                    // file), so a toggle here had nothing to write: the checkbox
                    // came straight back on at the next refresh with no
                    // explanation, which reads as the click being ignored.
                    app.status = Some(format!(
                        "{name} is Creation Club content the engine loads itself - it cannot be turned off here."
                    ));
                } else if forced {
                    app.status =
                        Some(format!("{name} is a light plugin this game can't load and stays off."));
                } else if app.plugins.is_some() {
                    let held = hold_plugin_selection(app);
                    let mut now = false;
                    if let Some(list) = app.plugins.as_mut() {
                        now = list.plugins.get(i).map(|p| p.enabled).unwrap_or(false);
                        list.set_enabled(&name, !now);
                        list.refresh(&spec);
                    }
                    put_plugin_selection(app, held);
                    // Persist to the profile (which owns the order) and the prefix.
                    // Both borrows below are shared, so this is fine after the
                    // mutation above has ended.
                    let written = app
                        .plugins
                        .as_ref()
                        .map(|list| write_plugin_state(app, list, &spec))
                        .transpose();
                    // A toggled plugin changes which archives the engine loads.
                    plugin_state_changed(app);
                    app.status = Some(match written {
                        Ok(_) => format!("{} {name}.", if now { "Disabled" } else { "Enabled" }),
                        Err(e) => {
                            // Refused write: drop the phantom toggle, resync to disk.
                            app.plugins = compute_plugins(app);
                            format!("Could not write the load order: {e}")
                        }
                    });
                }
            }
        }
        Message::SortPlugins => {
            // Refused up front while the game runs: the sort's async completion
            // would only be refused by the lock anyway (and resynced), so
            // starting it just wastes a masterlist download to throw the result
            // away - and shows a "Sorting..." status for a sort that cannot land.
            if app.running.is_some() {
                app.status =
                    Some("Cannot sort while the game is running.".to_string());
                return Task::none();
            }
            // One at a time. Without this every impatient click during the
            // masterlist download queued another complete sort, and each one
            // re-opened the report over a dialog the user had already closed -
            // minutes later, since they run strictly one after another.
            if app.sorting {
                app.status = Some("A LOOT sort is already running.".to_string());
                return Task::none();
            }
            // Gather everything the (static) async closure needs, cloned out of
            // `app`, then run the masterlist fetch + LOOT sort off the UI thread.
            let Some(game) = selected_game(app) else { return Task::none() };
            let id = game.def.id;
            if !eidos_loot::is_supported(id) {
                app.status = Some(format!("LOOT sorting is not available for {id}."));
                return Task::none();
            }
            let Some(spec) = GameSpec::for_id(id) else { return Task::none() };
            let Some(cd) = game.compatdata.as_ref() else {
                app.status =
                    Some("Launch the game once through Steam first so its prefix exists.".to_string());
                return Task::none();
            };
            let Some(list) = app.plugins.as_ref() else {
                app.status = Some("No plugins computed yet.".to_string());
                return Task::none();
            };
            let id = id.to_string();
            let install = game.install_path.clone();
            // The PROFILE is the load-order authority; the prefix copy is a
            // shadow that can be stale. Fall back to the prefix only before the
            // profile owns a state (pre-first-launch).
            let local_dir = app
                .created
                .as_ref()
                .map(|i| i.active())
                .filter(|p| p.has_plugin_state())
                .map(|p| p.plugins_state_dir())
                .unwrap_or_else(|| plugins_txt_dir(&cd.join("pfx"), &spec));
            let cache = app
                .created
                .as_ref()
                .map(|i| i.root.join("loot"))
                .unwrap_or_else(|| eidos_instance::Instance::global(&id).root.join("loot"));
            let plugins: Vec<(String, PathBuf)> =
                list.plugins.iter().map(|p| (p.name.clone(), p.path.clone())).collect();
            // Where LOOT must look besides the vanilla Data dir. Highest priority
            // first, Overwrite ahead of everything, matching the union's own
            // precedence - without these every file-conditioned masterlist rule
            // is evaluated against a directory the mods are not in.
            let mod_dirs = loot_data_paths(app);
            // The enabled (active) plugin names, lowercased - drives which plugins the
            // LOOT report covers and what counts as a missing master.
            let enabled_lower: std::collections::HashSet<String> = list
                .plugins
                .iter()
                .filter(|p| p.enabled)
                .map(|p| p.name.to_ascii_lowercase())
                .collect();
            // What this answer will be checked against when it comes back.
            let fingerprint = SortFingerprint {
                game: id.clone(),
                profile: app
                    .created
                    .as_ref()
                    .map(|i| i.active_profile())
                    .unwrap_or_default(),
                names: list.plugins.iter().map(|p| p.name.clone()).collect(),
            };
            app.sorting = true;
            app.status =
                Some("Sorting plugins with LOOT (updating the masterlist)...".to_string());
            return Task::perform(
                async move {
                    // `is_supported(id)` was checked above and loot_support is a pure
                    // map, so this is always Some here; handle None gracefully anyway
                    // rather than unwrap (robust to any future refactor of the guard).
                    let repo = match eidos_loot::loot_support(&id) {
                        Some((_, repo)) => repo,
                        None => return Err(format!("LOOT sorting is not available for {id}.")),
                    };
                    // Refresh the masterlist on every sort, like MO2/LOOT; a failed
                    // download falls back to the cached copy.
                    let (ml, pre) = eidos_loot::ensure_masterlist(repo, &cache, true)
                        .map_err(|e| e.to_string())?;
                    let userlist = cache.join("userlist.yaml");
                    // Close libloot's case gap before it evaluates anything. Its
                    // condition evaluator is a bare `exists()`, and the masterlist
                    // is written in Windows casing, so on Linux a rule like
                    // `not file("scripts/skse.pex")` misses `Scripts/skse.pex` and
                    // warns that a correctly installed SKSE is missing its scripts.
                    // The bridge hands libloot the spelling the masterlist asks
                    // for, pointing at the real file, and nothing else.
                    let bridge_dir = cache.join("case-bridge");
                    let mut mod_dirs = mod_dirs;
                    match eidos_loot::build_case_bridge(&ml, &mod_dirs, &install, &bridge_dir) {
                        Ok(bridged) if !bridged.is_empty() => {
                            eprintln!(
                                "eidos: LOOT case bridge: {} path(s) spelled differently on disk ({})",
                                bridged.len(),
                                bridged.join(", ")
                            );
                            // LAST, so a real file always answers before a link.
                            mod_dirs.push(eidos_loot::case_bridge_data_dir(&bridge_dir));
                        }
                        Ok(_) => {}
                        // Never fatal: a sort with the old blind spot beats no sort.
                        Err(e) => eprintln!("eidos: could not build the LOOT case bridge: {e}"),
                    }
                    // One view, used by both calls, so the report can never be
                    // built from a different picture than the sort.
                    let view = eidos_loot::GameView {
                        game_id: &id,
                        game_path: &install,
                        local_path: &local_dir,
                        plugins: &plugins,
                        mod_dirs: &mod_dirs,
                        masterlist: &ml,
                        prelude: &pre,
                        userlist: Some(&userlist),
                    };
                    let order = eidos_loot::sort(&view).map_err(|e| e.to_string())?;
                    // Build the post-sort report (general messages + per-plugin
                    // missing masters / messages / dirty info) for the modal, the
                    // same way MO2 shows its LOOT dialog after a sort. This is
                    // advisory: a report failure must NOT discard the successful
                    // sort, so it is an inner Result the handler tolerates.
                    let report =
                        eidos_loot::report(&view, &enabled_lower).map_err(|e| e.to_string());
                    Ok((fingerprint, order, report))
                },
                Message::PluginsSorted,
            );
        }
        Message::PluginsSorted(result) => {
            // Cleared on EVERY path, including the failures below, or a single
            // bad sort would leave the button dead for the rest of the session.
            app.sorting = false;
            let (asked_about, sorted, report_res) = match result {
                Ok(x) => x,
                Err(e) => {
                    app.status = Some(format!("LOOT sort failed: {e}"));
                    return Task::none();
                }
            };
            // A Refresh while LOOT ran drops the cached list. Rebuild it BEFORE
            // fingerprinting, or a harmless refresh would look like a changed
            // list and throw away a sort that is still perfectly valid.
            if app.plugins.is_none() {
                app.plugins = compute_plugins(app);
            }
            // Refuse an answer computed for a list that has since changed. The
            // order LOOT returns is a permutation of the names it was GIVEN;
            // applied to a different set - after a profile switch, a mod enabled
            // or disabled, a mod installed - it silently rearranges plugins
            // nobody asked about, and everything downstream reports a clean sort.
            let now = SortFingerprint {
                game: selected_game(app).map(|g| g.def.id.to_string()).unwrap_or_default(),
                profile: app.created.as_ref().map(|i| i.active_profile()).unwrap_or_default(),
                names: app
                    .plugins
                    .as_ref()
                    .map(|l| l.plugins.iter().map(|p| p.name.clone()).collect())
                    .unwrap_or_default(),
            };
            if now != asked_about {
                app.status = Some(if now.game != asked_about.game {
                    format!(
                        "Discarded the LOOT sort: it was computed for {}, and {} is open now.",
                        asked_about.game, now.game
                    )
                } else if now.profile != asked_about.profile {
                    format!(
                        "Discarded the LOOT sort: it was computed for profile '{}', and '{}' is active now.",
                        asked_about.profile, now.profile
                    )
                } else {
                    "Discarded the LOOT sort: the plugin list changed while it ran. Sort again."
                        .to_string()
                });
                return Task::none();
            }
            // Recompute spec + prefix dir (immutable borrows) before mutating plugins.
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            let before: Vec<String> = app
                .plugins
                .as_ref()
                .map(|l| l.plugins.iter().map(|p| p.name.clone()).collect())
                .unwrap_or_default();
            let held = hold_plugin_selection(app);
            if let Some(list) = app.plugins.as_mut() {
                list.apply_sorted_order(&sorted);
                // NOT repin_to_current: refresh() puts the pinned plugins back
                // where the user pinned them, over LOOT's opinion. Holding a slot
                // against the sorter is the entire purpose of a pin.
                list.refresh(&spec);
            }
            put_plugin_selection(app, held);
            // How much actually moved. Without this the status reads the same
            // whether the sort rearranged forty plugins or had nothing to do,
            // and a correct no-op on an already-sorted list is indistinguishable
            // from a broken button - which is exactly how it was reported.
            let changed = app
                .plugins
                .as_ref()
                .map(|l| {
                    l.plugins
                        .iter()
                        .zip(before.iter())
                        .filter(|(p, was)| &p.name != *was)
                        .count()
                })
                .unwrap_or(0);
            // Say when the sort was partly overruled, rather than reporting a
            // clean LOOT sort the list does not actually match.
            let pinned = app.plugins.as_ref().map(|l| l.locked.len()).unwrap_or(0);
            let held = if pinned > 0 { format!(" ({pinned} pinned position(s) kept)") } else { String::new() };
            let written =
                app.plugins.as_ref().map(|list| write_plugin_state(app, list, &spec)).transpose();
            let landed = written.is_ok();
            // The same numbers, into the log. The status bar carries them already,
            // but it is gone the moment anything else sets a status - and these
            // two numbers are the ones that expose a broken sort. LOOT deciding
            // nothing on a large list is legitimate ONCE, on an order that really
            // is optimal; run after run on hundreds of plugins it means the sort
            // is not running, which is precisely how the overlap stage managed to
            // be dead for 69 days while the status said "checked 211 plugins".
            // A user cannot see "every time" in a status bar. They can in a log.
            eidos_log::info!(
                "eidos loot: sorted {} plugins, {changed} moved{}",
                sorted.len(),
                if pinned > 0 { format!(", {pinned} pinned position(s) kept") } else { String::new() }
            );
            app.status = Some(match written {
                Ok(_) => {
                    if changed == 0 {
                        format!(
                            "LOOT checked {} plugins - the load order was already correct, nothing moved.{held}",
                            sorted.len()
                        )
                    } else {
                        format!("LOOT sorted {} plugins - {changed} moved.{held}", sorted.len())
                    }
                }
                Err(e) => {
                    // Refused write: drop the phantom sort, resync to disk.
                    app.plugins = compute_plugins(app);
                    format!("Sorted, but writing the load order failed: {e}")
                }
            });
            // A refused write means the sort was rolled back and the list on
            // screen is the one from disk. Popping the report here would present
            // advice about an order that no longer exists, on top of a dialog
            // whose very appearance reads as success - so the failure would be
            // announced by a success-shaped modal. The status line already says
            // what went wrong; leave it standing.
            if !landed {
                return Task::none();
            }
            // Show the LOOT report (MO2 always pops its dialog after a sort), so the
            // user sees missing masters / warnings / cleaning advice - or a clean bill.
            // The order was already applied above; a report failure only costs the
            // dialog, never the sort.
            match report_res {
                Ok(mut report) => {
                    // The per-plugin bundles feed the Plugins-tab badges and
                    // outlive the dialog; the dialog keeps the rest.
                    app.loot_meta = Some(std::mem::take(&mut report.plugin_meta));
                    app.loot_report = Some(report);
                }
                Err(e) => {
                    let base = app.status.take().unwrap_or_default();
                    app.status = Some(format!("{base} (LOOT report unavailable: {e})"));
                }
            }
        }
        // Merge and Replace land on a mod that ALREADY has a place in the load
        // order. Honouring a drop's target priority there would yank an
        // established mod out of its slot and silently flip every conflict it is
        // in - so the aim is discarded, not consumed. (Rename keeps it: that
        // really does create a new folder.) MO2 guards the same case.
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
            if let Some(new) = app.collision.as_ref().map(|c| c.rename_to.trim().to_string()) {
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
        Message::ChangeGame => {
            // Re-open the game picker; keep detection and any selection.
            app.menu_mod = None;
            app.info_mod = None;
            app.executables = None;
            app.selected_mod = None;
            app.selected_mods.clear();
            app.drag_state = None;
            app.profile_menu = None;
            app.profile_rename = None;
            app.profile_copy = None;
            app.profile_delete_confirm = None;
            app.error = None;
            app.screen = Screen::Game;
        }
        Message::OpenNexusGame => {
            let domain = selected_game(app).map(|g| g.def.nexus_game).filter(|s| !s.is_empty());
            let url = match domain {
                Some(d) => format!("https://www.nexusmods.com/{d}"),
                None => "https://www.nexusmods.com".to_string(),
            };
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            app.status = Some(format!("Opening {url}"));
        }
        Message::SetupPrereqs => {
            let arg = instance_arg(app);
            let has_prefix = selected_game(app).and_then(|g| g.compatdata.as_ref()).is_some();
            let log = app.created.as_ref().map(|i| i.root.join("prereqs.log"));
            match (arg, log) {
                // A runtime needs no prefix and no Proton - it is a directory and
                // an environment variable - so a missing prefix must not block it.
                (Some(_), _) if !has_prefix && !any_runtime_pending(app) => {
                    app.status = Some(
                        "Launch the game once through Steam first so its Proton prefix exists, then run Tool Setup."
                            .to_string(),
                    );
                }
                (Some(arg), Some(log)) => match run_prereqs_setup(&arg, &log) {
                    Ok(()) => {
                        app.status = Some(format!(
                            "Installing tool prerequisites: bundled DLLs copy now; .NET/vcrun download via winetricks. Progress + errors -> {}",
                            log.display()
                        ));
                    }
                    Err(e) => app.status = Some(format!("Could not start prereq setup: {e}")),
                },
                _ => app.status = Some("Open a game instance first.".to_string()),
            }
        }
        Message::ShowModInfo(i) => {
            app.menu_mod = None;
            let seeded = match (app.created.as_ref(), app.mods.get(i)) {
                (Some(inst), Some(m)) => {
                    let meta = inst.mod_meta(&m.name);
                    Some((meta.notes().unwrap_or_default(), meta.url().unwrap_or_default()))
                }
                _ => None,
            };
            if let Some((notes, url)) = seeded {
                app.notes_edit = notes;
                app.url_edit = url;
                app.info_mod = Some(i);
                app.info_tab = InfoTab::General;
            }
        }
        Message::CloseInfo => app.info_mod = None,
        Message::InfoSelectTab(t) => app.info_tab = t,
        Message::NotesChanged(s) => {
            app.typing = true;
            app.notes_edit = s;
        }
        Message::ModUrlChanged(s) => {
            app.typing = true;
            app.url_edit = s;
        }
        Message::ModUrlSave => {
            // The game's own content has no folder under mods/, so there is
            // nowhere to put a meta.ini - and inventing one would plant an empty
            // directory the next reconcile lists as a real mod.
            if app.info_mod.and_then(|i| app.mods.get(i)).is_some_and(|m| m.is_unmanaged()) {
                app.status =
                    Some("The game's own content has no Eidos metadata to write.".to_string());
                return Task::none();
            }
            let typed = app.url_edit.trim().to_string();
            // Refuse anything that is not a web link, HERE rather than at the
            // click that opens it: a value that cannot be opened must not be
            // storable in the first place, or the button becomes a dead end that
            // looks live.
            let is_web_link = typed.starts_with("http://") || typed.starts_with("https://");
            if !typed.is_empty() && !is_web_link {
                app.status = Some("A mod page has to be an http:// or https:// address.".to_string());
                return Task::none();
            }
            let result = match (app.info_mod, app.created.as_ref()) {
                (Some(i), Some(inst)) => app.mods.get(i).map(|m| {
                    let mut meta = inst.mod_meta(&m.name);
                    meta.set_url(&typed);
                    (m.name.clone(), meta.write(&inst.meta_path(&m.name)))
                }),
                _ => None,
            };
            if let Some((name, r)) = result {
                app.status = Some(match r {
                    Ok(()) if typed.is_empty() => format!("Cleared the page link for '{name}'."),
                    Ok(()) => format!("Saved the page link for '{name}'."),
                    Err(e) => format!("Could not save the link: {e}"),
                });
            }
        }
        Message::NotesSave => {
            if app.info_mod.and_then(|i| app.mods.get(i)).is_some_and(|m| m.is_unmanaged()) {
                app.status =
                    Some("The game's own content has no Eidos metadata to write.".to_string());
                return Task::none();
            }
            let result = match (app.info_mod, app.created.as_ref()) {
                (Some(i), Some(inst)) => app.mods.get(i).map(|m| {
                    let mut meta = inst.mod_meta(&m.name);
                    meta.set_notes(&app.notes_edit);
                    (m.name.clone(), meta.write(&inst.meta_path(&m.name)))
                }),
                _ => None,
            };
            if let Some((name, r)) = result {
                // The row caches the note text for its glyph, so the write has to
                // drop the cached copy or the list keeps showing the old one -
                // and shows no glyph at all the first time a note is added.
                invalidate_meta(app, &name);
                refresh_meta_cache(app);
                app.status = Some(match r {
                    Ok(()) => format!("Saved notes for '{name}'."),
                    Err(e) => format!("Could not save notes: {e}"),
                });
            }
        }
        // ---- hidden files (MO2 filetree.cpp HIDE/UNHIDE) ----------------------
        Message::DataToggleDir(rel) => {
            if !app.data_expanded.remove(&rel) {
                app.data_expanded.insert(rel);
            }
        }
        // ---- User extensions --------------------------------------------------
        Message::ShowAddons => {
            app.menu_mod = None;
            // These are all opened FROM the View dropdown, which otherwise stays
            // up over the dialog and swallows the first click aimed at it.
            app.view_menu_open = false;
            // Re-read on open, so a manifest just written or just fixed appears
            // without a restart. Discovery is a directory of small TOML files.
            let (ok, bad) = eidos_addons::scan_addons_from(&eidos_addons::user_addons_dir());
            app.addons = ok;
            app.addon_rejected = bad;
            app.addons_open = true;
        }
        Message::CloseAddons => app.addons_open = false,
        Message::ReloadAddons => {
            let (ok, bad) = eidos_addons::scan_addons_from(&eidos_addons::user_addons_dir());
            app.addons = ok;
            app.addon_rejected = bad;
            // Reload is the button someone presses after fixing a check that
            // failed, so it is what clears the "do not retry" memory.
            app.addon_failed.clear();
            app.diag_dirty = true;
            app.status = Some(match app.addon_rejected.len() {
                0 => format!("Loaded {} extension(s).", app.addons.len()),
                n => format!("Loaded {} extension(s); {n} manifest(s) refused.", app.addons.len()),
            });
        }
        Message::RunAddon(id) => {
            let Some(a) = app.addons.iter().find(|a| a.id == id).cloned() else {
                return Task::none();
            };
            if let Some(why) = a.unavailable() {
                app.status = Some(format!("Cannot run '{}': {why}.", a.name));
                return Task::none();
            }
            let ctx = addon_context(app);
            // Every placeholder must resolve BEFORE anything is spawned. An
            // unresolved one is left literal on purpose, so the program would
            // otherwise be handed `{data}` as a path and fail somewhere far from
            // the cause - or worse, treat it as a relative one.
            let missing: Vec<String> = a
                .args
                .iter()
                .chain(std::iter::once(&a.workdir))
                .flat_map(|s| ctx.missing(s))
                .collect();
            if !missing.is_empty() {
                app.status = Some(format!(
                    "'{}' needs {} - open a game instance first.",
                    a.name,
                    missing.join(", ")
                ));
                return Task::none();
            }
            let mut cmd = std::process::Command::new(&a.exec);
            for arg in &a.args {
                cmd.arg(ctx.expand(arg));
            }
            if !a.workdir.is_empty() {
                cmd.current_dir(ctx.expand(&a.workdir));
            }
            // Detached, unlike a `diagnose` add-on: a tool is something the user
            // watches, may take minutes, and has no output Eidos parses. Waiting
            // on it would lock the window with no lock overlay to explain why.
            match cmd
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(child) => {
                    app.status = Some(format!("Started '{}' (pid {}).", a.name, child.id()))
                }
                Err(e) => app.status = Some(format!("Could not start '{}': {e}", a.name)),
            }
        }
        Message::OpenAddonsFolder => {
            let dir = eidos_addons::user_addons_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
            app.status = Some(format!("Opened {}", dir.display()));
        }
        // ---- Log pane ---------------------------------------------------------
        Message::ShowLogPane => {
            let files = eidos_log::sessions();
            app.view_menu_open = false;
            app.file_menu_open = false;
            let Some(newest) = files.first().cloned() else {
                app.status = Some(format!(
                    "No session logs yet. They appear in {} once Eidos runs a game or a tool.",
                    eidos_log::log_dir().display()
                ));
                return Task::none();
            };
            app.log_pane = Some(load_log_pane(files, newest, eidos_log::Level::Info));
            app.menu_mod = None;
            // Opened from the View dropdown, which otherwise stays up over the
            // pane and swallows the first click aimed at it.
            app.view_menu_open = false;
        }
        Message::CloseLogPane => app.log_pane = None,
        Message::LogPick(path) => {
            if let Some(pane) = &app.log_pane {
                let (files, level) = (pane.files.clone(), pane.level);
                app.log_pane = Some(load_log_pane(files, path, level));
            }
        }
        Message::LogLevel(level) => {
            if let Some(pane) = &app.log_pane {
                let (files, current) = (pane.files.clone(), pane.current.clone());
                app.log_pane = Some(load_log_pane(files, current, level));
            }
        }
        Message::LogRefresh => {
            if let Some(pane) = &app.log_pane {
                // The file LIST is re-read too: a launch started while the pane
                // is open creates a new session, and it is the one worth seeing.
                let files = eidos_log::sessions();
                let current =
                    if files.contains(&pane.current) { pane.current.clone() } else {
                        match files.first() {
                            Some(f) => f.clone(),
                            None => return Task::none(),
                        }
                    };
                let level = pane.level;
                app.log_pane = Some(load_log_pane(files, current, level));
            }
        }
        Message::LogCopy => {
            let Some(pane) = &app.log_pane else { return Task::none() };
            let text: String = pane
                .lines
                .iter()
                .map(|(lvl, msg)| format!("{:<5} {msg}", lvl.as_str()))
                .collect::<Vec<_>>()
                .join("\n");
            app.status = Some(format!("Copied {} log line(s).", pane.lines.len()));
            // The level is kept in the copied text: a pasted log with the
            // severities stripped is the half that stops being diagnosable.
            return iced::clipboard::write(text);
        }
        Message::LogOpenFolder => {
            let dir = eidos_log::log_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
            app.status = Some(format!("Opened {}", dir.display()));
        }
        // ---- INI editor -------------------------------------------------------
        Message::ShowIniEditor => {
            app.menu_mod = None;
            // These are all opened FROM the View dropdown, which otherwise stays
            // up over the dialog and swallows the first click aimed at it.
            app.view_menu_open = false;
            let (Some(inst), Some(game)) = (app.created.as_ref(), selected_game(app)) else {
                app.status = Some("Open a game instance first.".to_string());
                return Task::none();
            };
            let files: Vec<String> = game.def.ini_files.iter().map(|f| f.to_string()).collect();
            let Some(first) = files.first().cloned() else {
                app.status = Some(format!("{} has no INI files Eidos manages.", game.def.name));
                return Task::none();
            };
            let prof = inst.active();
            app.ini_editor = Some(load_ini_editor(&prof, files, first));
        }
        Message::CloseIniEditor => app.ini_editor = None,
        Message::IniEditorPick(name) => {
            let Some(inst) = app.created.clone() else { return Task::none() };
            let Some(ed) = &app.ini_editor else { return Task::none() };
            if ed.current == name {
                return Task::none();
            }
            // Switching away from unsaved edits would lose them silently, and
            // this is a file the game reads - say so and stay put.
            if ed.dirty {
                app.status = Some(format!(
                    "Save or revert {} before switching - it has unsaved changes.",
                    ed.current
                ));
                return Task::none();
            }
            let files = ed.files.clone();
            app.ini_editor = Some(load_ini_editor(&inst.active(), files, name));
        }
        Message::IniEditorAction(action) => {
            if let Some(ed) = &mut app.ini_editor {
                let edits = action.is_edit();
                ed.content.perform(action);
                // Only an EDIT dirties it. Clicking, selecting and scrolling are
                // actions too, and treating those as changes would arm the
                // "unsaved changes" guard for looking at the file.
                if edits {
                    ed.dirty = ed.content.text() != ed.original;
                }
            }
        }
        Message::IniEditorSave => {
            let Some(inst) = app.created.clone() else { return Task::none() };
            let Some(ed) = &mut app.ini_editor else { return Task::none() };
            let path = inst.active().ini_path(&ed.current);
            if ed.unreadable {
                app.status = Some(format!(
                    "{} could not be read, so saving would replace it with an empty file. \
                     Fix its permissions first.",
                    ed.current
                ));
                return Task::none();
            }
            // No trailing-newline surgery. iced 0.14's `Content::text()`
            // round-trips exactly, so the guard that used to sit here did not
            // prevent growth - it DELETED newlines the user had typed at the end
            // of a file that happened to start without one.
            let text = ed.content.text();
            match eidos_instance::write_text(&path, &text, ed.cp1252) {
                Ok(()) => {
                    ed.original = ed.content.text();
                    ed.dirty = false;
                    ed.missing = false;
                    app.status = Some(format!(
                        "Saved {} into profile '{}'. It is deployed at the next launch.",
                        ed.current,
                        inst.active().name
                    ));
                }
                Err(e) => app.status = Some(format!("Could not save {}: {e}", ed.current)),
            }
        }
        Message::IniEditorRevert => {
            if let Some(ed) = &mut app.ini_editor {
                ed.content = iced::widget::text_editor::Content::with_text(&ed.original);
                ed.dirty = false;
                app.status = Some(format!("Reverted {}.", ed.current));
            }
        }
        Message::IniEditorOpenExternal => {
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            let Some(ed) = &app.ini_editor else { return Task::none() };
            let path = inst.active().ini_path(&ed.current);
            if !path.is_file() {
                app.status =
                    Some(format!("{} does not exist yet - save once to create it.", ed.current));
                return Task::none();
            }
            let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
            app.status = Some(format!("Opened {} externally.", path.display()));
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
            let target = if path.is_dir() { path.clone() } else {
                path.parent().map(std::path::Path::to_path_buf).unwrap_or(path.clone())
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
            let Some(m) = app.mods.get(i).cloned() else { return Task::none() };
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
            let Some(inst) = app.created.as_ref() else { return Task::none() };
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
            let mut meta = inst.mod_meta(&m.name);
            let mut list: Vec<String> = meta.ini_tweaks().to_vec();
            let was_on = list.iter().any(|e| e.eq_ignore_ascii_case(&name));
            // Order is application order, so enabling appends rather than inserting:
            // the fragment a user just ticked should win over the ones already on.
            if was_on {
                list.retain(|e| !e.eq_ignore_ascii_case(&name));
            } else {
                list.push(name.clone());
            }
            meta.set_ini_tweaks(&list);
            match meta.write(&inst.meta_path(&m.name)) {
                Ok(()) => {
                    let verb = if was_on { "Disabled" } else { "Enabled" };
                    app.status = Some(format!("{verb} INI tweak '{name}' for '{}'.", m.name));
                }
                Err(e) => app.status = Some(format!("Could not save the tweak list: {e}")),
            }
        }
        Message::RestoreHiddenFiles(i) => {
            app.menu_mod = None;
            let Some(m) = app.mods.get(i).cloned() else { return Task::none() };
            // The bulk twin of ToggleFileHidden's rename, behind the same flock:
            // "Unhide all" strips dozens of .mohidden suffixes inside a layer of
            // a live mount - the exact mutation the single-file handler's guard
            // exists for, reachable one menu item away.
            let Some(inst) = app.created.as_ref() else { return Task::none() };
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
                Err(e) => app.status = Some(format!("Could not unhide files: {e}")),
            }
        }
        // ---- Settings / Preferences ------------------------------------------
        Message::OpenSettings => {
            app.menu_mod = None;
            app.nexus_error = None;
            app.settings_open = true;
        }
        Message::CloseSettings => {
            app.settings_open = false;
            app.nexus_error = None;
        }
        Message::SettingsTabSelected(t) => app.settings_tab = t,
        Message::NexusSignInStart => {
            if app.nexus_signing_in {
                return Task::none();
            }
            // Signing in opens a BROWSER and talks to Nexus, so offline mode has
            // to stop it here: `connect` is not on this path at all, and the
            // guard there does not cover the one action that begins by leaving
            // the program.
            if app.prefs.offline {
                app.nexus_error = Some(eidos_nexus::OFFLINE_MESSAGE.to_string());
                return Task::none();
            }
            app.nexus_signing_in = true;
            app.nexus_error = None;
            app.status = Some("Opening your browser to sign in to Nexus...".to_string());
            // The whole dance on a worker: browser hand-off, loopback listener,
            // code exchange. Blocking calls inside the async closure, like the
            // other network work here.
            return Task::perform(async move { nexus_sign_in() }, Message::NexusSignInResult);
        }
        Message::NexusSignInResult(result) => {
            app.nexus_signing_in = false;
            match result {
                Ok(account) => {
                    app.status = Some(format!(
                        "Signed in to Nexus as {} ({}).",
                        account.name,
                        if account.is_premium { "Premium" } else { "free" }
                    ));
                    app.nexus_account = Some(account);
                }
                Err(e) => app.nexus_error = Some(e),
            }
        }
        Message::NexusSignOut => {
            match eidos_instance::settings::clear_nexus_tokens() {
                Ok(()) => {
                    app.nexus_account = None;
                    app.status = Some("Signed out of Nexus.".to_string());
                }
                Err(e) => app.nexus_error = Some(format!("could not sign out: {e}")),
            }
        }
        Message::DragScrollSpeedChanged(v) => {
            app.prefs.drag_scroll_speed = v.clamp(0.25, 4.0);
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::SettingsToggleSection(key) => {
            if !app.settings_expanded.remove(key) {
                app.settings_expanded.insert(key);
            }
        }
        Message::ToggleRememberWindow(on) => {
            app.prefs.remember_window = on;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::ToggleOffline(on) => {
            app.prefs.offline = on;
            if let Err(e) = app.prefs.save() {
                app.error = Some(format!("Could not save settings: {e}"));
            }
            // The status bar's account line is now wrong either way: offline
            // means Eidos will not check, and coming back online means it can.
            app.nexus_error = on.then(|| eidos_nexus::OFFLINE_MESSAGE.to_string());
        }
        Message::PreferredServersChanged(t) => {
            app.typing = true;
            app.servers_edit = t;
        }
        Message::PreferredServersSave => {
            app.prefs.preferred_servers = app
                .servers_edit
                .split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .collect();
            if let Err(e) = app.prefs.save() {
                app.error = Some(format!("Could not save settings: {e}"));
            }
            // Echo back what was actually stored, so a trailing comma or a
            // double space does not survive in the field looking meaningful.
            app.servers_edit = app.prefs.preferred_servers.join(", ");
        }
        Message::ThemeChanged(t) => {
            app.prefs.theme = t;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::DefaultGameChanged(g) => {
            app.prefs.default_game = g;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::ToggleLockGui(v) => {
            app.prefs.lock_gui = v;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        // ---- Identify an archive by its MD5 (MO2's Query Metadata) -----------
        Message::IdentifyDownload(name) => {
            // One at a time. The row greys itself, but a keyboard repeat or a
            // second row's button would otherwise start another whole-file hash
            // on the same executor - and the two would race to write one
            // sidecar.
            if app.identifying_download.is_some() {
                return Task::none();
            }
            let Some(row) = app.downloads.iter().find(|d| d.name == name) else {
                return Task::none();
            };
            let Some(game) = selected_game(app) else {
                app.status = Some("Open a game instance first.".to_string());
                return Task::none();
            };
            let (path, domain, short) =
                (row.path.clone(), game.def.nexus_game, game.def.short_name);
            app.identifying_download = Some(name.clone());
            app.status = Some(format!("Identifying {name} by checksum..."));
            // Hashing hundreds of megabytes and a round trip to Nexus: off the
            // draw thread, or the window freezes for the duration.
            // Hashing hundreds of megabytes and a blocking HTTP round trip do
            // not belong on the async executor iced drives the window with:
            // held there they freeze every other task, including the one that
            // would repaint this row. A thread does the work and the channel
            // carries the answer back.
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let outcome = (|| {
                    let md5 = eidos_nexus::md5_file(&path)
                        .map_err(|e| format!("could not read the archive: {e}"))?;
                    let nexus = eidos_nexus::Nexus::connect()?;
                    let found = nexus.md5_search(domain, &md5)?;
                    let label = found.remote.name.clone();
                    eidos_nexus::write_recovered_meta(&path, short, &found).map_err(|e| {
                        format!("identified it, but could not write the sidecar: {e}")
                    })?;
                    Ok(label)
                })();
                let _ = tx.send(outcome);
            });
            return Task::perform(
                async move {
                    rx.recv().unwrap_or_else(|_| Err("the lookup stopped unexpectedly".into()))
                },
                Message::IdentifiedDownload,
            );
        }
        Message::IdentifiedDownload(result) => {
            app.identifying_download = None;
            match result {
                Ok(name) => {
                    app.status = Some(format!("Identified as '{name}'."));
                    // The row reads its state from the sidecar that now exists.
                    load_downloads(app);
                }
                Err(e) => app.status = Some(format!("Could not identify it: {e}")),
            }
        }
        // ---- Plugin context menu (MO2's plugin right-click) ------------------
        Message::OpenPluginMenu(i) => {
            // Right-clicking a row outside the selection selects just it first,
            // exactly like the mod list - otherwise a batch action would run on
            // rows the user cannot see any more.
            if !app.selected_plugins.contains(&i) {
                app.selected_plugins.clear();
                app.selected_plugin = Some(i);
            }
            app.menu_mod = None;
            app.menu_plugin = Some(i);
            // A field left open on another row would reopen aimed at that one.
            app.plugin_send_priority = None;
            // Freeze where it was summoned from. Drawn at the LIVE cursor the
            // card chases the pointer, and its own items can never be reached.
            app.menu_at = Some(app.cursor);
            app.focus = Pane::Plugins;
        }
        Message::ClosePluginMenu => {
            app.menu_plugin = None;
            // The field lives inside the card: closing the card must not leave a
            // half-typed index armed for the next menu to commit.
            app.plugin_send_priority = None;
        }
        Message::OpenPluginOrigin(i) => {
            app.menu_plugin = None;
            match plugin_origin_row(app, i) {
                Some(row) => {
                    if let Some(m) = app.mods.get(row) {
                        let _ = std::process::Command::new("xdg-open").arg(&m.path).spawn();
                    }
                }
                None => {
                    app.status =
                        Some("That plugin comes from the game's own Data, not from a mod.".into())
                }
            }
        }
        Message::ShowPluginOriginInfo(i) => {
            app.menu_plugin = None;
            match plugin_origin_row(app, i) {
                Some(row) => return update(app, Message::ShowModInfo(row)),
                None => {
                    app.status =
                        Some("That plugin comes from the game's own Data, not from a mod.".into())
                }
            }
        }
        Message::PluginsSendTop | Message::PluginsSendBottom => {
            // Read the anchor BEFORE closing the menu - it is stored there.
            let anchor = app.menu_plugin.or(app.selected_plugin);
            app.menu_plugin = None;
            let to_top = matches!(message, Message::PluginsSendTop);
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            // Right-clicking inside a multi-selection acts on the whole set,
            // outside it on that row alone - the same rule the mod list uses,
            // applied by OpenPluginMenu above.
            let Some(anchor) = anchor else { return Task::none() };
            let rows = plugin_selection_or(app, anchor);
            if rows.is_empty() {
                return Task::none();
            }
            let held = hold_plugin_selection(app);
            let mut moved = false;
            let mut already = false;
            if let Some(list) = app.plugins.as_mut() {
                // The engine's own masters sit above every mod plugin, so gap 0
                // is refused for all of them: the destination has to be the
                // outermost slot the load order actually allows, which is what
                // edge_gap works out.
                match list.edge_gap(&rows, to_top, &spec) {
                    Some(gap) => {
                        moved = list.move_plugins_to(&rows, gap, &spec);
                        if moved {
                            list.refresh(&spec);
                        }
                    }
                    None => already = true,
                }
            }
            put_plugin_selection(app, held);
            if !moved {
                app.status = Some(if already {
                    "Those plugins are already as far as the load order allows.".to_string()
                } else {
                    "The load order will not take that move: a master or a pin is in the way."
                        .to_string()
                });
                return Task::none();
            }
            commit_plugin_order(app, &spec);
        }
        Message::PluginSendToPriorityStart => {
            // The anchor lives on the menu, and the field replaces a row INSIDE
            // that menu - so the menu must stay open.
            let Some(row) = app.menu_plugin.or(app.selected_plugin) else { return Task::none() };
            app.menu_plugin = Some(row);
            app.plugin_send_priority = Some((row, String::new()));
        }
        Message::PluginSendToPriorityChanged(t) => {
            app.typing = true;
            if let Some((_, typed)) = &mut app.plugin_send_priority {
                *typed = t;
            }
        }
        Message::PluginSendToPriorityCommit => {
            let Some((row, typed)) = app.plugin_send_priority.take() else { return Task::none() };
            app.menu_plugin = None;
            let Ok(dest) = typed.trim().parse::<usize>() else {
                // "Row number", not "load index": the only numeric column this
                // pane draws is the game's hex load index (00, FE:003, `--` for
                // a disabled plugin), and that is NOT what this takes. Asking
                // for one thing and meaning another is worse than asking plainly.
                app.status = Some("Enter a row number (1 = the first plugin).".to_string());
                return Task::none();
            };
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            let rows = plugin_selection_or(app, row);
            if rows.is_empty() {
                return Task::none();
            }
            // The column shows 1-based indices, so '3' means "become the third
            // row", which is insertion index 2 - the same translation the mod
            // list makes for the same reason.
            let gap = dest.saturating_sub(1);
            let held = hold_plugin_selection(app);
            let moved = match app.plugins.as_mut() {
                Some(list) => {
                    let gap = gap.min(list.plugins.len());
                    let ok = list.move_plugins_to(&rows, gap, &spec);
                    if ok {
                        list.refresh(&spec);
                    }
                    ok
                }
                None => false,
            };
            put_plugin_selection(app, held);
            if !moved {
                // `move_plugins_to` refuses a destination the tiers forbid. Say
                // which rule stopped it rather than "no".
                app.status = Some(format!(
                    "Cannot put {} plugin(s) at {dest}: a master tier or a pin is in the way.",
                    rows.len()
                ));
                return Task::none();
            }
            let before = app.status.clone();
            commit_plugin_order(app, &spec);
            // Only claim success if the write did not report a problem.
            // `commit_plugin_order` sets the status when the order is REFUSED -
            // a running game holds the lock - and overwriting that turned "your
            // change was not saved" into "moved".
            if app.status == before {
                app.status = Some(format!("Moved {} plugin(s) to {dest}.", rows.len()));
            }
        }
        Message::PluginsSetAll(on) => {
            app.menu_plugin = None;
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            // By NAME, like the batch toggle: enabling changes the tier a plugin
            // sorts into, so the rows move under any index collected first.
            let names: Vec<String> = {
                let Some(list) = app.plugins.as_ref() else { return Task::none() };
                list.plugins
                    .iter()
                    .filter(|p| {
                        // The game's own masters and an .esl on a no-light
                        // engine cannot be toggled at all; writing them would be
                        // a lie the next refresh silently corrects.
                        let engine_owned = spec
                            .primary_plugins
                            .iter()
                            .any(|pp| pp.eq_ignore_ascii_case(&p.name))
                            || list.implicit.contains(&p.name.to_ascii_lowercase());
                        !engine_owned && !p.force_disabled && p.enabled != on
                    })
                    .map(|p| p.name.clone())
                    .collect()
            };
            if names.is_empty() {
                app.status = Some(if on {
                    "Every plugin that can be active already is.".to_string()
                } else {
                    "Nothing left to deactivate: the rest are loaded by the game itself.".to_string()
                });
                return Task::none();
            }
            let held = hold_plugin_selection(app);
            if let Some(list) = app.plugins.as_mut() {
                for n in &names {
                    list.set_enabled(n, on);
                }
                list.refresh(&spec);
            }
            put_plugin_selection(app, held);
            app.status = Some(format!(
                "{} {} plugin(s).",
                if on { "Activated" } else { "Deactivated" },
                names.len()
            ));
            commit_plugin_order(app, &spec);
        }
        // ---- Mod-list filter pane (MO2's Filters tree) -----------------------
        Message::CycleFilter(field) => {
            let next = app.filters.get(field).next();
            app.filters.set(field, next);
            // The visible set just changed underneath every row index the
            // selection, the focus and the drag hold.
            forget_hidden_rows(app);
        }
        Message::ToggleFilterPane => app.filters_open = !app.filters_open,
        Message::ClearFilters => {
            app.filters = ModFilters::default();
            forget_hidden_rows(app);
        }
        // ---- Backups dialog (MO2's Create Backup / Restore Backup) -----------
        // ---- Categories (MO2's Change Categories + the category editor) ------
        Message::ShowCategoriesDialog(row) => {
            app.menu_mod = None;
            let Some(inst) = app.created.clone() else {
                app.status = Some("Open a game instance first.".to_string());
                return Task::none();
            };
            // The right-clicked row, or the whole selection when it is part of
            // one (MO2's multi-row Change Categories).
            let sel = real_selection(app);
            let targets: Vec<usize> = if sel.len() > 1 && sel.contains(&row) {
                sel
            } else if app.mods.get(row).is_some_and(|m| !m.is_separator() && !m.is_unmanaged()) {
                vec![row]
            } else {
                Vec::new()
            };
            if targets.is_empty() {
                app.status = Some("Separators and the game's own content have no categories.".to_string());
                return Task::none();
            }
            let names: Vec<String> =
                targets.iter().filter_map(|&i| app.mods.get(i)).map(|m| m.name.clone()).collect();
            // The starting selection is the FIRST target's, so a single mod opens
            // on what it actually has. Across a multi-selection MO2 does the same
            // and applying overwrites the rest - the dialog says so.
            let chosen = names
                .first()
                .map(|n| eidos_instance::parse_all(&inst.mod_meta(n).category().unwrap_or_default()))
                .unwrap_or_default();
            app.categories_dialog = Some(CategoriesDialogState {
                names,
                chosen,
                catalog: inst.category_factory(),
                editing: false,
                new_name: String::new(),
                new_parent: 0,
                rename: None,
                confirm_delete: None,
                query: String::new(),
            });
        }
        Message::CloseCategoriesDialog => app.categories_dialog = None,
        Message::ToggleCategory(id) => {
            if let Some(d) = &mut app.categories_dialog {
                match d.chosen.iter().position(|&c| c == id) {
                    Some(at) => {
                        d.chosen.remove(at);
                    }
                    None => d.chosen.push(id),
                }
            }
        }
        Message::SetPrimaryCategory(id) => {
            if let Some(d) = &mut app.categories_dialog {
                // Primary is simply "first in the list" on disk, so promoting is a
                // move, not a separate field. Checking it too, if it was not, keeps
                // the two controls from disagreeing.
                d.chosen.retain(|&c| c != id);
                d.chosen.insert(0, id);
            }
        }
        Message::CategoryQueryChanged(q) => {
            if let Some(d) = &mut app.categories_dialog {
                d.query = q;
            }
        }
        Message::ToggleCategoryEditor => {
            if let Some(d) = &mut app.categories_dialog {
                d.editing = !d.editing;
                d.rename = None;
                d.confirm_delete = None;
            }
        }
        Message::NewCategoryNameChanged(name) => {
            if let Some(d) = &mut app.categories_dialog {
                d.new_name = name;
            }
        }
        Message::NewCategoryParentChanged(parent) => {
            if let Some(d) = &mut app.categories_dialog {
                d.new_parent = parent;
            }
        }
        Message::AddCategory => {
            if let Some(d) = &mut app.categories_dialog {
                let name = d.new_name.trim().to_string();
                if name.is_empty() {
                    app.status = Some("Give the category a name first.".to_string());
                    return Task::none();
                }
                let id = d.catalog.add(&name, d.new_parent);
                d.new_name.clear();
                app.status = Some(format!("Added category '{name}' (#{id}). Apply to save."));
            }
        }
        Message::RenameCategoryStart(id) => {
            if let Some(d) = &mut app.categories_dialog {
                let current = d.catalog.name_for_id(id).unwrap_or_default().to_string();
                d.rename = Some((id, current));
                d.confirm_delete = None;
            }
        }
        Message::RenameCategoryChanged(name) => {
            if let Some(d) = &mut app.categories_dialog {
                if let Some((_, pending)) = &mut d.rename {
                    *pending = name;
                }
            }
        }
        Message::RenameCategoryCommit => {
            if let Some(d) = &mut app.categories_dialog {
                if let Some((id, name)) = d.rename.take() {
                    if !name.trim().is_empty() {
                        d.catalog.rename(id, &name);
                    }
                }
            }
        }
        Message::DeleteCategory(id) => {
            if let Some(d) = &mut app.categories_dialog {
                // Two clicks: the first arms, the second deletes.
                if d.confirm_delete == Some(id) {
                    d.catalog.remove(id);
                    d.confirm_delete = None;
                    // A deleted category cannot stay assigned in the pending pick.
                    d.chosen.retain(|&c| c != id);
                } else {
                    d.confirm_delete = Some(id);
                }
            }
        }
        Message::FetchNexusCategories => {
            if !eidos_nexus::Nexus::have_credentials() {
                app.status =
                    Some("Connect a Nexus account first (Settings, or `eidos nexus key <KEY>`).".to_string());
                return Task::none();
            }
            let Some(domain) = selected_game(app).map(|g| g.def.nexus_game.to_string()) else {
                return Task::none();
            };
            app.status = Some("Fetching the category list from Nexus...".to_string());
            return Task::perform(
                async move {
                    let nexus = eidos_nexus::Nexus::connect()?;
                    nexus.game_categories(&domain)
                },
                Message::NexusCategoriesFetched,
            );
        }
        Message::NexusCategoriesFetched(result) => {
            let Some(d) = &mut app.categories_dialog else { return Task::none() };
            let remote = match result {
                Ok(r) => r,
                Err(e) => {
                    app.status = Some(format!("Could not fetch categories: {e}"));
                    return Task::none();
                }
            };
            // Match a remote category onto a local one by NAME first - MO2's
            // built-in list was derived from Nexus's, so most of them already
            // line up and mapping by name avoids creating 40 duplicates named
            // exactly like the ones already there. Only the rest are created.
            let mut mapped = 0usize;
            let mut created = 0usize;
            // Two passes so a child's parent exists before it is placed, whatever
            // order Nexus returned them in.
            let mut local_of: HashMap<i32, i32> = HashMap::new();
            let mut created_ids: std::collections::HashSet<i32> = std::collections::HashSet::new();
            for (nexus_id, name, _) in &remote {
                let existing = d
                    .catalog
                    .all()
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(name))
                    .map(|c| c.id);
                let local = match existing {
                    Some(id) => id,
                    None => {
                        created += 1;
                        let id = d.catalog.add(name, 0);
                        created_ids.insert(id);
                        id
                    }
                };
                local_of.insert(*nexus_id, local);
                if d.catalog.learn_nexus_id(local, *nexus_id, name) {
                    mapped += 1;
                }
            }
            for (nexus_id, _, parent) in &remote {
                // ONLY the categories this fetch created. The previous rule was
                // "currently top-level", which is not the same thing at all: it
                // swept in every pre-existing top-level category - MO2's own
                // defaults included - and re-parented the user's shared tree on
                // every fetch.
                let (Some(&local), Some(parent)) = (local_of.get(nexus_id), *parent) else {
                    continue;
                };
                if !created_ids.contains(&local) {
                    continue;
                }
                if let Some(&local_parent) = local_of.get(&parent) {
                    d.catalog.set_parent(local, local_parent);
                }
            }
            app.status = Some(format!(
                "Nexus: {} categories, {created} new, {mapped} mapping(s) learned. Apply to save.",
                remote.len()
            ));
        }
        Message::AssignCategoriesFromNexus => {
            let Some(inst) = app.created.clone() else { return Task::none() };
            let Some(d) = &mut app.categories_dialog else { return Task::none() };
            if d.catalog.all().iter().all(|c| c.nexus.is_empty()) {
                app.status = Some(
                    "No Nexus mappings yet - use 'Fetch from Nexus' first.".to_string(),
                );
                return Task::none();
            }
            // Only the targeted mods, so this stays the dialog's own action and
            // never rewrites the whole instance from a button the user pressed to
            // categorise one mod. Batch-select to do more.
            let mut hits: Vec<i32> = Vec::new();
            for name in &d.names {
                let Some(remote) = nexus_category_of(&inst, name) else { continue };
                if let Some(local) = d.catalog.for_nexus_id(remote) {
                    hits.push(local);
                }
            }
            match hits.first() {
                Some(&first) => {
                    // The dialog holds ONE pending pick for the whole selection,
                    // so a mixed batch would have to pick a winner. Say which.
                    let all_same = hits.iter().all(|&h| h == first);
                    d.chosen = vec![first];
                    let label = d.catalog.name_for_id(first).unwrap_or("?").to_string();
                    app.status = Some(if all_same {
                        format!("Nexus says '{label}'. Apply to set it.")
                    } else {
                        format!(
                            "The selection spans several Nexus categories; '{label}' is the first. \
                             Apply sets it on all of them."
                        )
                    });
                }
                None => {
                    app.status = Some(
                        "None of these mods records a Nexus category (no download .meta, or an \
                         unmapped id)."
                            .to_string(),
                    );
                }
            }
        }
        Message::ApplyCategories => {
            let Some(inst) = app.created.clone() else { return Task::none() };
            let Some(d) = app.categories_dialog.as_ref() else { return Task::none() };
            // The catalog first: if it fails, nothing has been written yet, and a
            // mod pointing at an id the catalog does not carry would show a bare
            // number with no way to name it.
            //
            // The dialog is NOT taken until that save succeeds. Taking it first
            // meant a failed write - a read-only instance, a full disk - threw
            // away every pending edit with nothing to retry from.
            if let Err(e) = d.catalog.save(inst.categories_root()) {
                app.status = Some(format!("Could not save the category list: {e}"));
                return Task::none();
            }
            let Some(d) = app.categories_dialog.take() else { return Task::none() };
            let (primary, others) = match d.chosen.split_first() {
                Some((p, rest)) => (Some(*p), rest),
                None => (None, &[][..]),
            };
            let mut written = 0usize;
            let mut failed: Vec<String> = Vec::new();
            for name in &d.names {
                let mut meta = inst.mod_meta(name);
                meta.set_categories(primary, others);
                match meta.write(&inst.meta_path(name)) {
                    Ok(()) => written += 1,
                    Err(e) => failed.push(format!("{name}: {e}")),
                }
            }
            // The cache maps name -> category; every touched mod's row is stale.
            app.meta_cache.clear();
            refresh_meta_cache(app);
            app.categories = Some(inst.category_factory());
            recompute_counts(app);
            // Recategorising changes what a category filter shows, so the same
            // rule applies as to the filter pane itself: nothing may stay aimed
            // at a row that just left the screen.
            forget_hidden_rows(app);
            app.status = Some(if failed.is_empty() {
                match (primary, written) {
                    (None, n) => format!("Cleared the category on {n} mod(s)."),
                    (Some(p), n) => {
                        let label = d.catalog.name_for_id(p).unwrap_or("?");
                        format!("Set category '{label}' on {n} mod(s).")
                    }
                }
            } else {
                format!("{written} mod(s) updated, {} failed: {}", failed.len(), failed.join("; "))
            });
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
            let Some(prof) = app.created.as_ref().map(|i| i.active()) else { return Task::none() };
            match prof.create_backup(kind) {
                Ok(b) => {
                    app.status =
                        Some(format!("Backed up the {} ({}).", kind.label(), b.when()));
                    app.backups = load_backups(app);
                }
                Err(e) => app.status = Some(format!("Could not back up the {}: {e}", kind.label())),
            }
        }
        Message::RestoreBackup(kind, stamp) => {
            let Some(inst) = app.created.clone() else { return Task::none() };
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
                    app.plugins = None;
                    app.conflicts = compute_conflicts(app);
                    recompute_counts(app);
                }
                Err(e) => app.status = Some(format!("Could not restore the {}: {e}", kind.label())),
            }
        }
        // ---- Executables dialog ----------------------------------------------
        Message::ShowExecutablesDialog => {
            app.menu_mod = None;
            match open_executables_dialog(app) {
                Some(state) => app.executables = Some(state),
                None => app.status = Some("Open a game instance first.".to_string()),
            }
        }
        Message::CloseExecutablesDialog => app.executables = None,
        Message::SelectExecutableTool(i) => {
            if let Some(state) = &mut app.executables {
                state.commit_buffers();
                state.selected = Some(i);
                state.load_buffers();
            }
        }
        Message::AddExecutableTool => {
            if let Some(state) = &mut app.executables {
                state.commit_buffers();
                let tool = Tool { title: "New Tool".to_string(), ..Default::default() };
                // User tools sit at the front, ahead of the read-only defaults.
                state.merged.insert(state.user_len, tool);
                state.selected = Some(state.user_len);
                state.user_len += 1;
                state.load_buffers();
            }
        }
        Message::DeleteExecutableTool => {
            if let Some(state) = &mut app.executables {
                if state.selected_is_user() {
                    if let Some(i) = state.selected {
                        state.merged.remove(i);
                        state.user_len -= 1;
                        state.selected = None;
                        state.load_buffers();
                    }
                }
            }
        }
        Message::MoveExecutableUp => {
            if let Some(state) = &mut app.executables {
                state.commit_buffers();
                if let Some(i) = state.selected {
                    if i > 0 && i < state.user_len {
                        state.merged.swap(i, i - 1);
                        state.selected = Some(i - 1);
                    }
                }
            }
        }
        Message::MoveExecutableDown => {
            if let Some(state) = &mut app.executables {
                state.commit_buffers();
                if let Some(i) = state.selected {
                    if i + 1 < state.user_len {
                        state.merged.swap(i, i + 1);
                        state.selected = Some(i + 1);
                    }
                }
            }
        }
        Message::ToolTitleChanged(s) => {
            if let Some(state) = &mut app.executables {
                // Auto-seed prereqs from the title for known tools (e.g. BodySlide ->
                // d3dx9_43, d3dcompiler_47), mirroring the CLI, but only when the user
                // has not entered any prereqs yet (never clobber their edit).
                if state.prereqs.trim().is_empty() {
                    let seeded = eidos_instance::default_prereqs(&s).join(", ");
                    if !seeded.is_empty() {
                        state.prereqs = seeded;
                    }
                }
                state.title = s;
            }
        }
        Message::ToolExeChanged(s) => {
            if let Some(state) = &mut app.executables {
                state.exe = s;
            }
        }
        Message::ToolWorkdirChanged(s) => {
            if let Some(state) = &mut app.executables {
                state.workdir = s;
            }
        }
        Message::ToolPrereqsChanged(s) => {
            if let Some(state) = &mut app.executables {
                state.prereqs = s;
            }
        }
        Message::BrowseToolExe => {
            // Start the picker in the game install dir (where tool exes usually live).
            let start = selected_game(app).map(|g| g.install_path.clone());
            let mut dlg = rfd::AsyncFileDialog::new()
                .add_filter("Executables", &["exe"])
                .set_title("Select the tool executable");
            if let Some(dir) = start {
                dlg = dlg.set_directory(dir);
            }
            return Task::perform(dlg.pick_file(), |h| match h {
                Some(h) => Message::ToolExeChanged(h.path().display().to_string()),
                None => Message::Noop,
            });
        }
        Message::BrowseToolWorkdir => {
            let start = selected_game(app).map(|g| g.install_path.clone());
            let mut dlg = rfd::AsyncFileDialog::new().set_title("Select the working directory");
            if let Some(dir) = start {
                dlg = dlg.set_directory(dir);
            }
            return Task::perform(dlg.pick_folder(), |h| match h {
                Some(h) => Message::ToolWorkdirChanged(h.path().display().to_string()),
                None => Message::Noop,
            });
        }
        Message::SaveExecutablesDialog => {
            if let Some(state) = &mut app.executables {
                state.commit_buffers();
                // Reject a blank or control-char title up front (write_tools would
                // silently drop it, losing the user's edit without warning).
                let bad = state.merged[..state.user_len].iter().find(|t| {
                    let title = t.title.trim();
                    title.is_empty() || title.chars().any(char::is_control)
                });
                if bad.is_some() {
                    app.status = Some("Every tool needs a non-empty, single-line title.".to_string());
                    return Task::none();
                }
                // A tool with no executable is a row that write_tools drops on
                // the way out - the entry looks saved, and is gone next time.
                let blank = state.merged[..state.user_len]
                    .iter()
                    .find(|t| t.exe.as_os_str().is_empty())
                    .map(|t| t.title.clone());
                if let Some(title) = blank {
                    app.status = Some(format!("'{title}' has no executable - it would not save."));
                    return Task::none();
                }
                let mut user_tools: Vec<Tool> = state.merged[..state.user_len].to_vec();
                // Hide and pin are about the PICKER, so they apply to a per-game
                // default too - and a default is not in the user's list, so
                // setting one on it was silently discarded on save. A default
                // that carries a flag is written out as a user entry holding
                // nothing but that flag; `merge_tools` gives a user entry
                // precedence over the default of the same title, so the tool
                // itself still comes from the default.
                for d in &state.merged[state.user_len..] {
                    if (d.hidden || d.pinned)
                        && !user_tools.iter().any(|t| t.title.eq_ignore_ascii_case(&d.title))
                    {
                        user_tools.push(d.clone());
                    }
                }
                if let Some(inst) = &app.created {
                    match inst.save_tools(&user_tools) {
                        Ok(()) => {
                            app.executables = None;
                            load_tools(app); // refresh the run-target picker
                            app.status = Some("Saved executables.".to_string());
                        }
                        Err(e) => app.status = Some(format!("Could not save executables: {e}")),
                    }
                }
            }
        }
        // ---- Endorse ---------------------------------------------------------
        Message::ModEndorse(i) => {
            if app.endorsing.is_some() {
                return Task::none();
            }
            // Cheap check here (it reads a file); the actual credential is chosen
            // inside the task, because renewing an OAuth token costs a round trip
            // and must not run on the UI thread.
            if !eidos_nexus::Nexus::have_credentials() {
                app.status = Some(
                    "Connect a Nexus account first (Settings, or `eidos nexus key <KEY>`).".to_string(),
                );
                return Task::none();
            }
            let domain = selected_game(app).map(|g| g.def.nexus_game.to_string());
            let folder = app.mods.get(i).map(|m| m.name.clone()).unwrap_or_default();
            let info = app.created.as_ref().zip(app.mods.get(i)).filter(|(_, m)| !m.is_separator()).map(
                |(inst, m)| {
                    let meta = inst.mod_meta(&m.name);
                    (meta.mod_id(), meta.version().unwrap_or_default(), meta.endorsed())
                },
            );
            let (Some(domain), Some((Some(mod_id), version, endorsed))) = (domain, info) else {
                app.status = Some("This mod has no Nexus id to endorse.".to_string());
                return Task::none();
            };
            // Toggle: endorse when not yet endorsed, abstain when already endorsed.
            let endorse = !endorsed;
            app.endorsing = Some(i);
            app.status = Some(
                if endorse { "Endorsing on Nexus...".to_string() } else { "Abstaining on Nexus...".to_string() },
            );
            return Task::perform(
                async move {
                    eidos_nexus::Nexus::connect()?
                        .set_endorsed(&domain, mod_id, &version, endorse)
                },
                move |r| Message::ModEndorsed(folder.clone(), r),
            );
        }
        Message::ModEndorsed(folder, result) => {
            app.endorsing = None;
            match result {
                Ok(now_endorsed) => {
                    // Persist by folder name: the row index from before the network
                    // round-trip may point at a different mod by now.
                    if let (Some(inst), Some(m)) =
                        (app.created.as_ref(), app.mods.iter().find(|m| m.name == folder))
                    {
                        let mut meta = inst.mod_meta(&m.name);
                        meta.set("endorsed", if now_endorsed { "1" } else { "0" });
                        let _ = meta.write(&inst.meta_path(&m.name));
                        app.status = Some(format!(
                            "{} '{}' on Nexus.",
                            if now_endorsed { "Endorsed" } else { "Abstained from" },
                            m.display_name()
                        ));
                    }
                    recompute_counts(app);
                }
                Err(e) => app.status = Some(format!("Endorse failed: {e}")),
            }
        }
        // ---- per-mod local flags (Track / Ignore update) --------------------
        Message::ModTrack(i) => {
            app.menu_mod = None;
            if let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) {
                if !m.is_separator() {
                    let mut meta = inst.mod_meta(&m.name);
                    let now = !meta.tracked();
                    meta.set_tracked(now);
                    let _ = meta.write(&inst.meta_path(&m.name));
                    app.status = Some(format!(
                        "{} '{}'.",
                        if now { "Tracking" } else { "Untracked" },
                        m.display_name()
                    ));
                }
            }
        }
        Message::ModIgnoreUpdate(i) => {
            app.menu_mod = None;
            if let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) {
                if !m.is_separator() {
                    let mut meta = inst.mod_meta(&m.name);
                    let now = !meta.ignore_update();
                    meta.set_ignore_update(now);
                    let _ = meta.write(&inst.meta_path(&m.name));
                    app.status = Some(format!(
                        "{} updates for '{}'.",
                        if now { "Ignoring" } else { "Checking" },
                        m.display_name()
                    ));
                    // The ignore flag lives in this mod's meta.ini.
                    let changed = m.name.clone();
                    invalidate_meta(app, &changed);
                    refresh_meta_cache(app);
                    recompute_counts(app);
                }
            }
        }
        // ---- mod creation (Create empty mod / Install from folder) ----------
        Message::CreateEmptyMod => return update(app, Message::CreateEmptyModAt(app.mods.len())),
        Message::CreateEmptyModAt(at) => {
            app.menu_mod = None;
            if let Some(inst) = &app.created {
                // A unique "New Mod N" name, never colliding on disk.
                let mut n = 1usize;
                let mut name = "New Mod".to_string();
                while inst.mods_dir().join(&name).exists() {
                    n += 1;
                    name = format!("New Mod {n}");
                }
                match inst.create_empty_mod(&name) {
                    Ok(entry) => {
                        // Where the caller asked. The toolbar button still means
                        // the end of the list - highest priority, where a fresh
                        // install goes - but the context menu can name a place.
                        let idx = at.min(app.mods.len());
                        app.mods.insert(idx, entry);
                        // Every index at or after the insertion point shifted.
                        app.selected_mods.clear();
                        // If it landed inside a COLLAPSED group the list would
                        // never draw it - a new mod that exists on disk, holds a
                        // priority, and cannot be seen or renamed. Unfold the
                        // group that swallowed it.
                        if let Some(sep) = app.mods[..idx].iter().rposition(|m| m.is_separator()) {
                            let name = app.mods[sep].display_name().to_string();
                            if app.collapsed.remove(&name) {
                                save_collapsed(app);
                            }
                        }
                        mods_changed(app);
                        app.selected_mod = Some(idx);
                        app.selected_mods.clear();
                        // Open its rename editor so the user names it straight away.
                        app.rename = Some((idx, name));
                        app.menu_mod = Some(idx);
                    }
                    Err(e) => app.status = Some(format!("Could not create mod: {e}")),
                }
            }
        }
        Message::InstallFromFolder => {
            app.menu_mod = None;
            // Pick an already-unpacked mod directory off-thread.
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_title("Select an unpacked mod folder to install")
                    .pick_folder(),
                |handle| Message::FolderPicked(handle.map(|h| h.path().to_path_buf())),
            );
        }
        Message::FolderPicked(picked) => {
            let Some(src) = picked else { return Task::none() };
            let mods_dir = app.created.as_ref().map(|i| i.mods_dir());
            let Some(mods_dir) = mods_dir else {
                return Task::none();
            };
            // Name the new mod after the chosen folder (sanitized + de-duplicated).
            let raw = src.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let base = eidos_install::fix_directory_name(&raw).unwrap_or_else(|| "New Mod".to_string());
            let name = suggest_free_name(&mods_dir, &base);
            let dest = mods_dir.join(&name);
            // Copy the folder's CONTENTS into mods/<name>/ (not the folder itself),
            // mirroring how an archive's root is laid out.
            match copy_dir_contents(&src, &dest) {
                Ok(()) => after_install(app, &name, dest, false, None),
                Err(e) => {
                    let _ = fs::remove_dir_all(&dest);
                    app.status = Some(format!("Install from folder failed: {e}"));
                }
            }
        }
        // ---- Mod update check ------------------------------------------------
        Message::CheckUpdates => {
            if app.update_in_progress {
                return Task::none();
            }
            if !eidos_nexus::Nexus::have_credentials() {
                app.status = Some(
                    "Connect a Nexus account first (Settings, or `eidos nexus key <KEY>`).".to_string(),
                );
                return Task::none();
            }
            let Some(domain) = selected_game(app).map(|g| g.def.nexus_game.to_string()) else {
                return Task::none();
            };
            let Some(inst) = app.created.clone() else {
                app.status = Some("Open a game instance first.".to_string());
                return Task::none();
            };
            app.update_in_progress = true;
            app.status = Some("Checking Nexus for mod updates...".to_string());
            return Task::perform(
                async move {
                    let nexus = eidos_nexus::Nexus::connect()?;
                    eidos_nexus::check_updates(&nexus, &inst, &domain)
                },
                Message::UpdatesChecked,
            );
        }
        Message::UpdatesChecked(result) => {
            app.update_in_progress = false;
            match result {
                Ok(r) => {
                    // The check rewrote a version line in an unknown number of
                    // meta.ini files, so this is the one case that really does
                    // need the whole map back.
                    app.meta_cache.clear();
                    refresh_meta_cache(app);
                    recompute_counts(app);
                    // The budget the server reported on the way through. It was
                    // already in the result and thrown away; the status bar shows
                    // it now, so an update check stops being the only way to find
                    // out how much of the hour is left.
                    app.nexus_hourly_left = r.hourly_remaining;
                    app.nexus_daily_left = r.daily_remaining;
                    let mut msg = format!(
                        "Update check: {} mods checked, {} update(s) found.",
                        r.checked, r.updates_found
                    );
                    if !r.unavailable.is_empty() {
                        // Said out loud rather than left to a glyph: an
                        // unavailable mod never shows an update, so nothing else
                        // in this check would ever mention it.
                        msg.push_str(&format!(
                            " {} mod(s) are no longer on Nexus: {}.",
                            r.unavailable.len(),
                            r.unavailable.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                        ));
                    }
                    if r.rate_limited {
                        msg.push_str(" Hourly Nexus limit reached - some mods were left unchecked.");
                    }
                    app.status = Some(msg);
                }
                Err(e) => app.status = Some(format!("Update check failed: {e}")),
            }
        }
        // ---- menu bar --------------------------------------------------------
        Message::ShowAbout => {
            app.menu_mod = None;
            app.about_open = true;
        }
        Message::CloseAbout => app.about_open = false,
        Message::OpenViewMenu => {
            app.file_menu_open = false;
            app.view_menu_open = true;
        }
        Message::CloseViewMenu => app.view_menu_open = false,
        Message::OverwriteSyncToMods => {
            if !app.confirm_sync {
                app.confirm_sync = true;
                return Task::none();
            }
            app.confirm_sync = false;
            let Some(inst) = app.created.clone() else { return Task::none() };
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
        // ---- Nexus collections ------------------------------------------------
        Message::ShowCollection(link) => {
            app.file_menu_open = false;
            app.collection = Some(CollectionState {
                link,
                revision: None,
                states: Vec::new(),
                loading: false,
                error: None,
                confirm_fetch: false,
                asked: std::collections::HashSet::new(),
            });
            if app.collection.as_ref().is_some_and(|c| !c.link.trim().is_empty()) {
                return update(app, Message::CollectionFetch);
            }
        }
        Message::CloseCollection => app.collection = None,
        Message::CollectionLinkChanged(t) => {
            app.typing = true;
            if let Some(c) = &mut app.collection {
                c.link = t;
                c.error = None;
            }
        }
        Message::CollectionFetch => {
            let Some(c) = &mut app.collection else { return Task::none() };
            let parsed = match eidos_nexus::NxmLink::parse(c.link.trim()) {
                Ok(eidos_nexus::NxmLink::Collection(c)) => c,
                Ok(eidos_nexus::NxmLink::Mod(_)) => {
                    c.error = Some(
                        "That is a link to a single mod, not a collection. Use the Install button \
                         for one mod."
                            .to_string(),
                    );
                    return Task::none();
                }
                Err(e) => {
                    c.error = Some(e);
                    return Task::none();
                }
            };
            // The collection's game must be the OPEN instance's game. Without
            // this the member list is joined against a different game's mods and
            // downloads, so every "installed" and every "missing" is noise
            // wearing the shape of an answer - and "Try to fetch missing" would
            // send the downloads to a third place again, since the nxm handler
            // routes by domain and the window does not.
            let here = selected_game(app).map(|g| (g.def.nexus_game, g.def.name));
            match here {
                Some((domain, _)) if domain.eq_ignore_ascii_case(&parsed.game) => {}
                Some((_, name)) => {
                    let c = app.collection.as_mut().expect("checked above");
                    c.error = Some(format!(
                        "That collection is for '{}', but the open instance is {name}. Open a {} \
                         instance first - a collection can only be compared against its own game.",
                        parsed.game, name
                    ));
                    return Task::none();
                }
                None => {
                    let c = app.collection.as_mut().expect("checked above");
                    c.error = Some("Open a game instance first.".to_string());
                    return Task::none();
                }
            }
            let c = app.collection.as_mut().expect("checked above");
            if !eidos_nexus::Nexus::have_credentials() {
                c.error = Some("Connect a Nexus account first (Settings).".to_string());
                return Task::none();
            }
            c.loading = true;
            c.error = None;
            c.revision = None;
            c.states.clear();
            // A different revision is a different member list; what the previous
            // one asked for says nothing about this one.
            c.asked.clear();
            return Task::perform(
                async move {
                    let nexus = eidos_nexus::Nexus::connect()?;
                    nexus.collection_revision(&parsed)
                },
                Message::CollectionFetched,
            );
        }
        Message::CollectionFetched(result) => {
            let Some(c) = &mut app.collection else { return Task::none() };
            c.loading = false;
            match result {
                Ok(rev) => {
                    c.revision = Some(rev);
                    c.error = None;
                }
                Err(e) => {
                    c.error = Some(e);
                    c.revision = None;
                }
            }
            // Joined against the instance HERE rather than in the view: the view
            // runs every frame and this reads every mod's meta.ini once.
            recompute_collection_states(app);
        }
        Message::CollectionOpenMod(i) => {
            let Some(c) = app.collection.as_ref() else { return Task::none() };
            let Some(m) = c.revision.as_ref().and_then(|r| r.mods.get(i)) else {
                return Task::none();
            };
            // The mod's FILES tab, at the exact file the collection pins - not
            // the mod's front page, which shows whatever is newest. Clicking the
            // Mod Manager Download button there fires the nxm:// link that the
            // registered handler already knows how to service, so the existing
            // download, sidecar and install pipeline runs unchanged.
            let url = format!(
                "https://www.nexusmods.com/{}/mods/{}?tab=files&file_id={}",
                m.domain, m.mod_id, m.file_id
            );
            return update(app, Message::OpenUrl(url));
        }
        Message::CollectionFetchMissing => {
            let Some(c) = app.collection.as_ref() else { return Task::none() };
            let Some(rev) = c.revision.as_ref() else { return Task::none() };
            // The pane's instance, not whatever `eidos nxm` would resolve on its
            // own. The handler picks by game domain, so with two instances of
            // one game it can send a collection's downloads somewhere other than
            // the window that asked for them.
            let Some(inst) = app.created.as_ref().map(|i| i.root.clone()) else {
                app.status = Some("Open an instance first.".to_string());
                return Task::none();
            };
            let (batch, left) = next_fetch_batch(rev, &c.states, &c.asked, FETCH_BATCH);
            let missing = batch;
            if missing.is_empty() {
                // The escape hatch matters: a transfer that failed leaves its
                // member missing AND already-asked, so without saying this the
                // pane looks stuck.
                app.status = Some(
                    "Nothing left to ask for - the rest were already started. Look up again to \
                     retry any that did not land."
                        .to_string(),
                );
                return Task::none();
            }
            // Two clicks, like every other action here that does something real.
            if !c.confirm_fetch {
                let c = app.collection.as_mut().expect("checked above");
                c.confirm_fetch = true;
                return Task::none();
            }
            let bin = find_eidos_binary();
            let mut started = 0usize;
            for (mod_id, file_id, domain) in &missing {
                let link = format!("nxm://{domain}/mods/{mod_id}/files/{file_id}");
                match std::process::Command::new(&bin)
                    .arg("nxm")
                    .arg(&link)
                    .env("EIDOS_INSTANCE", &inst)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(_) => {
                        started += 1;
                        if let Some(c) = app.collection.as_mut() {
                            c.asked.insert(*file_id);
                        }
                    }
                    Err(e) => {
                        app.status = Some(format!("Could not start the downloads: {e}"));
                        if let Some(c) = app.collection.as_mut() {
                            c.confirm_fetch = false;
                        }
                        return Task::none();
                    }
                }
            }
            if let Some(c) = app.collection.as_mut() {
                c.confirm_fetch = false;
            }
            app.status = Some(if left > 0 {
                format!(
                    "Started {started}, {left} still to go. Click again for the next batch. A \
                     free Nexus account cannot mint links without the site's own button - if \
                     they do not start, use Open on each row."
                )
            } else {
                format!(
                    "Started {started} download(s). A free Nexus account cannot mint links \
                     without the site's own button - if they do not start, use Open on each row."
                )
            });
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
            let Some(k) = app.known.get(i).cloned() else { return Task::none() };
            if !k.portable {
                app.status =
                    Some("A global instance is derived from the game id - there is nothing to forget."
                        .to_string());
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
                    app.status = Some(format!(
                        "No longer listing {}. Nothing on disk was touched.",
                        k.inst.root.display()
                    ));
                }
                Err(e) => app.status = Some(format!("Could not update the instance list: {e}")),
            }
        }
        Message::InstanceRenameStart(i) => {
            app.confirm_forget = None;
            let Some(k) = app.known.get(i) else { return Task::none() };
            let current =
                k.inst.root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            app.instance_rename = Some((i, current));
        }
        Message::InstanceRenameChanged(t) => {
            app.typing = true;
            if let Some((_, name)) = &mut app.instance_rename {
                *name = t;
            }
        }
        Message::InstanceRenameCommit => {
            let Some((i, typed)) = app.instance_rename.take() else { return Task::none() };
            let Some(k) = app.known.get(i).cloned() else { return Task::none() };
            let name = typed.trim().to_string();
            if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
                app.status = Some("That is not a folder name.".to_string());
                return Task::none();
            }
            if !k.portable {
                app.status = Some(
                    "A global instance lives at a path derived from the game id; it has no folder \
                     to rename."
                        .to_string(),
                );
                return Task::none();
            }
            // Renaming the folder out from under the OPEN instance would leave
            // every cached path in the window pointing at somewhere that no
            // longer exists - including the lock it is holding.
            if app.created.as_ref().is_some_and(|c| c.root == k.inst.root) {
                app.status = Some(
                    "That is the instance you have open. Switch to another one first.".to_string(),
                );
                return Task::none();
            }
            // And not one ANOTHER process is using. `app.created` only knows
            // about this window; the flock is what covers a second Eidos window,
            // a running game, or a CLI session - all of which hold paths into
            // the folder about to move.
            if let Err(e) = probe_lock(&k.inst) {
                app.status = Some(format!("That instance is in use: {e}."));
                return Task::none();
            }
            let Some(parent) = k.inst.root.parent() else { return Task::none() };
            let dest = parent.join(&name);
            if dest.exists() {
                app.status = Some(format!("{} already exists.", dest.display()));
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
                            app.status = Some(format!("Renamed to {}.", dest.display()));
                        }
                        Err(e) => {
                            app.status = Some(format!(
                                "Renamed the folder to {}, but the instance list could not be \
                                 updated: {e}. Open it from that path to re-register it.",
                                dest.display()
                            ));
                        }
                    }
                }
                Err(e) => app.status = Some(format!("Could not rename: {e}")),
            }
        }
        // ---- Export the mod list (MO2's Export to csv) -----------------------
        Message::ShowExportDialog => {
            app.file_menu_open = false;
            app.view_menu_open = false;
            if app.created.is_none() {
                app.status = Some("Open a game instance first.".to_string());
                return Task::none();
            }
            app.export = Some(ExportDialogState {
                scope: ExportScope::All,
                // Everything ticked, so the default export is byte-identical to
                // what the CLI has always produced and to what MO2 writes.
                columns: vec![true; eidos_instance::Column::ALL.len()],
            });
        }
        Message::CloseExportDialog => app.export = None,
        Message::ExportScopeChanged(scope) => {
            if let Some(d) = &mut app.export {
                d.scope = scope;
            }
        }
        Message::ExportToggleColumn(i) => {
            if let Some(d) = &mut app.export {
                if let Some(on) = d.columns.get_mut(i) {
                    *on = !*on;
                }
            }
        }
        Message::ExportRun => {
            let Some(d) = &app.export else { return Task::none() };
            if d.picked().is_empty() {
                app.status = Some("Tick at least one column.".to_string());
                return Task::none();
            }
            let name = app
                .created
                .as_ref()
                .and_then(|i| i.root.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "modlist".to_string());
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .add_filter("CSV", &["csv"])
                    .set_file_name(format!("{name}.csv"))
                    .set_title("Export the mod list")
                    .save_file(),
                |handle| Message::ExportPicked(handle.map(|h| h.path().to_path_buf())),
            );
        }
        Message::ExportPicked(picked) => {
            let Some(path) = picked else { return Task::none() };
            let Some(inst) = app.created.clone() else { return Task::none() };
            let Some(d) = app.export.take() else { return Task::none() };
            // Re-checked HERE, not only at the click that opened the picker: the
            // window keeps handling events while the native save dialog is up,
            // so the ticks can be cleared in between.
            if d.picked().is_empty() {
                app.status = Some("Nothing was exported: no columns were ticked.".to_string());
                return Task::none();
            }
            let domain = selected_game(app).map(|g| g.def.nexus_game).unwrap_or("");
            // What the WINDOW is showing, not what the file says: the list in
            // `app.mods` carries unsaved reordering, and exporting a different
            // order from the one on screen would be its own small betrayal.
            let (csv, count) =
                eidos_instance::mod_list_csv(&inst, &app.mods, d.scope, &d.picked(), domain);
            match std::fs::write(&path, csv) {
                Ok(()) => {
                    app.status =
                        Some(format!("Exported {count} mod(s) to {}.", path.display()))
                }
                Err(e) => app.status = Some(format!("Could not write {}: {e}", path.display())),
            }
        }
        Message::OpenFileMenu => {
            // Only one dropdown at a time, or the two cards overlap and the one
            // underneath eats clicks aimed at the one on top.
            app.view_menu_open = false;
            app.file_menu_open = true;
        }
        Message::CloseFileMenu => app.file_menu_open = false,
        Message::ToggleToolbar => {
            app.ui_toolbar_visible = !app.ui_toolbar_visible;
            app.view_menu_open = false;
        }
        Message::ToggleStatusBar => {
            app.ui_statusbar_visible = !app.ui_statusbar_visible;
            app.view_menu_open = false;
        }
        Message::CollapseOthers(keep) => {
            // Every separator but this one. Keyed by display name like the rest
            // of the fold state, which is also MO2's key - two separators with
            // the same name fold together there too, and matching that is worth
            // more than being cleverer about a list somebody wrote themselves.
            for m in &app.mods {
                if m.is_separator() && m.display_name() != keep {
                    app.collapsed.insert(m.display_name().to_string());
                }
            }
            app.collapsed.remove(&keep);
            save_collapsed(app);
            app.menu_mod = None;
        }
        Message::CollapseAllGroups => {
            // Whichever fold set is ON SCREEN. Under a grouping the separators
            // are not drawn and their folds are suspended, so folding them
            // would be a menu entry that visibly does nothing.
            if app.group_by.is_some() {
                let labels: Vec<String> = display_entries(app)
                    .into_iter()
                    .filter_map(|e| match e {
                        ListEntry::Group(l, _) => Some(l),
                        ListEntry::Row(_) => None,
                    })
                    .collect();
                app.groups_collapsed.extend(labels);
            } else {
                // Key by display name, like MO2.
                for m in &app.mods {
                    if m.is_separator() {
                        app.collapsed.insert(m.display_name().to_string());
                    }
                }
                save_collapsed(app);
            }
            app.view_menu_open = false;
        }
        Message::ExpandAllGroups => {
            if app.group_by.is_some() {
                app.groups_collapsed.clear();
            } else {
                app.collapsed.clear();
                save_collapsed(app);
            }
            app.view_menu_open = false;
        }
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
                let focus = app.selected_save.and_then(|i| app.saves.get(i)).map(|s| s.path.clone());
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
            let paths: Vec<std::path::PathBuf> =
                targets.iter().filter_map(|&i| app.saves.get(i)).map(|s| s.path.clone()).collect();
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
            let Some(inst) = app.created.clone() else { return Task::none() };
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
                let Some(save) = app.saves.get(i) else { continue };
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
                    let Some(name) = src.file_name() else { continue };
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
            let wanted: HashSet<String> =
                app.save_missing.iter().flat_map(|m| m.providers.iter().cloned()).collect();
            let mut enabled = 0usize;
            for m in app.mods.iter_mut() {
                if !m.enabled && wanted.contains(&m.name) {
                    m.enabled = true;
                    enabled += 1;
                }
            }
            if enabled == 0 {
                app.status =
                    Some("Those mods are already enabled - the plugins still need turning on in the Plugins tab.".to_string());
                return Task::none();
            }
            mods_changed(app);
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
            let Some(inst) = app.created.as_ref() else { return Task::none() };
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
            let Some(inst) = app.created.as_ref() else { return Task::none() };
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
        // ---- Downloads manager ----
        Message::RefreshDownloads => {
            load_downloads(app);
            app.status = Some(format!("Found {} download(s).", app.downloads.len()));
        }
        Message::CleanInstallDebris => {
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            let dir = inst.mods_dir();
            let mut gone = 0usize;
            let mut in_use = 0usize;
            let mut failed: Vec<String> = Vec::new();
            for e in fs::read_dir(&dir).into_iter().flatten().flatten() {
                let Ok(name) = e.file_name().into_string() else { continue };
                // The prefix is the whole guard, and it is checked HERE rather
                // than trusted from the diagnostic: the card the user clicked
                // may have been built before a refresh, and this loop deletes.
                if !name.starts_with(".eidos-install") {
                    continue;
                }
                // The temp's name embeds the pid that made it
                // (`.eidos-install-<pid>-<n>`), and a LIVE pid means a CLI
                // install is extracting into it right now - deleting it out
                // from under 7-Zip failed that install while this button
                // called it safe debris. Alive = not debris; skip it.
                let live = name
                    .strip_prefix(".eidos-install-")
                    .and_then(|r| r.split('-').next())
                    .and_then(|p| p.parse::<u32>().ok())
                    .is_some_and(|pid| std::path::Path::new(&format!("/proc/{pid}")).exists());
                if live {
                    in_use += 1;
                    continue;
                }
                match fs::remove_dir_all(e.path()) {
                    Ok(()) => gone += 1,
                    Err(err) => failed.push(format!("{name}: {err}")),
                }
            }
            let busy = if in_use > 0 {
                format!(" ({in_use} in use by a running install, left alone)")
            } else {
                String::new()
            };
            app.status = Some(if failed.is_empty() {
                format!("Removed {gone} leftover extraction folder(s).{busy}")
            } else {
                format!("Removed {gone}; could not remove {}{busy}", failed.join(", "))
            });
            app.diag_dirty = true;
            reload_mods(app);
        }
        Message::DownloadTick => {
            // Cheap and bounded: one read_dir of a directory holding a few dozen
            // files. It is NOT called from view() - that lesson is already paid
            // for - so it costs twice a second, not once per frame.
            load_downloads(app);
            // A resume that FAILED has to say so. The child is also reaped here:
            // left unwaited it becomes a zombie, and a zombie's /proc entry is
            // what made `stop_download` wait out its whole timeout and refuse to
            // clear the recorded pid.
            if let Some((name, child, log)) = &mut app.resuming {
                if let Ok(Some(status)) = child.try_wait() {
                    let (name, log) = (name.clone(), log.clone());
                    app.resuming = None;
                    if !status.success() {
                        let why = std::fs::read_to_string(&log)
                            .unwrap_or_default()
                            .lines()
                            .rev()
                            .find(|l| !l.trim().is_empty())
                            .unwrap_or("no reason given")
                            .to_string();
                        app.status = Some(format!("Could not resume '{name}': {why}"));
                    }
                }
            }
        }
        Message::DeleteDownload(name) => {
            app.confirm_delete_download = Some(name);
        }
        Message::PauseDownload(name) => {
            let Some(row) = app.downloads.iter().find(|r| r.name == name) else {
                return Task::none();
            };
            let path = row.path.clone();
            // The flag goes down FIRST. If the process dies between the two
            // writes, the row still reads as paused rather than reverting to
            // "stalled" - which is the same state on disk but a different thing
            // to tell the user, and the one that says something went wrong.
            let _ = eidos_nexus::set_download_meta_key(&path, "paused", "true");
            if eidos_nexus::stop_download(&path) {
                app.status = Some(format!("Paused '{name}'. Resume picks up where it stopped."));
            } else {
                // No live process: it had already stopped. The flag still tells
                // the truth about what the user wants.
                app.status = Some(format!("'{name}' was not running; marked paused."));
            }
            load_downloads(app);
        }
        Message::ResumeDownload(name) => {
            let Some(row) = app.downloads.iter().find(|r| r.name == name) else {
                return Task::none();
            };
            let path = row.path.clone();
            if eidos_nexus::live_download_pid(&path).is_some() {
                app.status = Some(format!("'{name}' is already downloading."));
                return Task::none();
            }
            let _ = eidos_nexus::set_download_meta_key(&path, "paused", "false");
            // A separate process, exactly like a browser-initiated download: the
            // window must not be blocked for the length of a transfer, and the
            // downloads tick already reports progress from the partial's size.
            // The child's stderr is CAPTURED, not discarded. The one failure
            // this path exists to surface - a non-premium account cannot mint a
            // fresh link, so an unattended resume is impossible - is printed
            // there, and sending it to /dev/null left the window saying
            // "Resuming..." while the row quietly went back to Stalled.
            let log = eidos_log::log_dir().join("resume.log");
            let _ = std::fs::create_dir_all(eidos_log::log_dir());
            let sink = std::fs::File::create(&log).ok();
            let mut cmd = std::process::Command::new(find_eidos_binary());
            cmd.arg("nxm").arg("--resume").arg(&path).stdin(std::process::Stdio::null());
            match sink {
                Some(f) => {
                    let dup = f.try_clone().ok();
                    cmd.stdout(std::process::Stdio::from(f));
                    if let Some(d) = dup {
                        cmd.stderr(std::process::Stdio::from(d));
                    }
                }
                None => {
                    cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
                }
            }
            match cmd.spawn() {
                Ok(child) => {
                    app.resuming = Some((name.clone(), child, log));
                    app.status = Some(format!("Resuming '{name}'..."));
                }
                Err(e) => app.status = Some(format!("Could not resume '{name}': {e}")),
            }
            load_downloads(app);
        }
        Message::ConfirmDeleteDownload(name) => {
            // Armed and confirmed on the SAME file. The list re-sorts under a
            // background tick, so an index would have been a way to delete the
            // wrong archive by standing still.
            if app.confirm_delete_download.as_deref() == Some(name.as_str()) {
                if let Some(row) = app.downloads.iter().find(|r| r.name == name) {
                    let name = row.name.clone();
                    // Stop the transfer BEFORE unlinking anything. It writes to
                    // `<archive>.unfinished` and finishes with a rename onto the
                    // real name: delete those out from under a live process and
                    // it keeps filling the orphaned inode, then puts the archive
                    // back on disk seconds after the user removed it.
                    eidos_nexus::stop_download(&row.path);
                    // Remove the archive and its `.meta` sidecar together (MO2 keeps
                    // them paired). A missing sidecar is fine.
                    let meta = PathBuf::from(format!("{}.meta", row.path.display()));
                    // The partial goes too. A stalled download has no archive at
                    // all - only `<name>.unfinished` - so removing the archive
                    // and the sidecar would leave the very file that puts the row
                    // back on the next tick.
                    let partial = eidos_nexus::unfinished_path(&row.path);
                    let had_partial = partial.is_file();
                    let archive_res = std::fs::remove_file(&row.path);
                    let _ = std::fs::remove_file(&partial);
                    let _ = std::fs::remove_file(&meta);
                    match archive_res {
                        Ok(()) => app.status = Some(format!("Deleted download '{name}'.")),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound && had_partial => {
                            app.status =
                                Some(format!("Removed the unfinished download '{name}'."));
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            app.status = Some(format!("Download '{name}' was already gone."));
                        }
                        Err(e) => app.status = Some(format!("Could not delete '{name}': {e}")),
                    }
                }
                load_downloads(app);
            }
        }
        Message::SetAllModsEnabled(enable) => {
            // Two clicks, holding the TARGET state: arming "Enable all" and then
            // clicking "Disable all" must not fire the second one.
            if app.confirm_set_all != Some(enable) {
                app.confirm_set_all = Some(enable);
                return Task::none();
            }
            app.confirm_set_all = None;
            let targets = mods_visible_for_bulk(app);
            if targets.is_empty() {
                app.status = Some("Nothing on screen to change.".to_string());
                return Task::none();
            }
            let mut changed = 0usize;
            for &i in &targets {
                if let Some(m) = app.mods.get_mut(i) {
                    if m.enabled != enable {
                        m.enabled = enable;
                        changed += 1;
                    }
                }
            }
            if changed == 0 {
                app.status = Some(format!(
                    "Already {}: all {} mod(s) on screen.",
                    if enable { "enabled" } else { "disabled" },
                    targets.len()
                ));
                return Task::none();
            }
            mods_changed(app);
            app.view_menu_open = false;
            // A collapsed group hides rows exactly as a filter does, and the
            // status has to admit either.
            let total = app.mods.iter().filter(|m| !m.is_separator() && !m.is_unmanaged()).count();
            let narrowed = targets.len() < total;
            app.status = Some(format!(
                "{} {changed} mod(s){}.",
                if enable { "Enabled" } else { "Disabled" },
                if narrowed {
                    format!(" - only the {} on screen, of {total}", targets.len())
                } else {
                    String::new()
                }
            ));
        }
        Message::BatchToggleMods => {
            // MO2-style batch enable/disable: if any selected real mod is enabled,
            // the whole selection is disabled; otherwise the whole selection is
            // enabled. Separators carry no toggle and are skipped.
            let targets: Vec<usize> = real_selection(app);
            if targets.is_empty() {
                app.status = Some("Select one or more mods first.".to_string());
                return Task::none();
            }
            let any_on = targets.iter().any(|&i| app.mods.get(i).is_some_and(|m| m.enabled));
            let new_state = !any_on;
            for &i in &targets {
                if let Some(m) = app.mods.get_mut(i) {
                    m.enabled = new_state;
                }
            }
            mods_changed(app);
            app.menu_mod = None;
            app.status = Some(format!(
                "{} {} mod(s).",
                if new_state { "Enabled" } else { "Disabled" },
                targets.len()
            ));
        }
        Message::BatchRemoveMods => {
            let n = real_selection(app).len();
            if n == 0 {
                app.status = Some("Select one or more mods first.".to_string());
                return Task::none();
            }
            app.confirm_batch_remove = true;
            app.status =
                Some(format!("Click Remove again to permanently delete {n} mod(s) from disk."));
        }
        Message::ConfirmBatchRemove => {
            app.confirm_batch_remove = false;
            app.menu_mod = None;
            // Same flock as the single-row Remove, before the first deletion: a
            // batch of remove_dir_all against layers of a live union is the same
            // hazard multiplied, and refusing after the loop would protect nothing.
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(format!("Cannot remove mods now: {e}."));
                    return Task::none();
                }
            };
            // Delete from the highest index down so the lower indices stay valid.
            let mut targets = real_selection(app);
            targets.sort_unstable();
            let mut removed = 0usize;
            let mut failed = 0usize;
            for &i in targets.iter().rev() {
                if let Some(m) = app.mods.get(i).cloned() {
                    match fs::remove_dir_all(&m.path) {
                        Ok(()) => {
                            app.mods.remove(i);
                            drop_files_cache(app, Some(&m.name));
                            removed += 1;
                        }
                        Err(_) => failed += 1,
                    }
                }
            }
            app.selected_mods.clear();
            app.selected_mod = None;
            mods_changed(app);
            app.status = Some(if failed == 0 {
                format!("Removed {removed} mod(s).")
            } else {
                format!("Removed {removed} mod(s); {failed} could not be deleted.")
            });
        }
        Message::BatchSendTop => {
            // Lift the whole selection (keeping its relative order) to the top.
            let mut targets = move_selection(app);
            if targets.is_empty() {
                return Task::none();
            }
            targets.sort_unstable();
            let hidden = hidden_by_folds(app);
            let at = move_block(&mut app.mods, &targets, 0);
            // The selection is now a contiguous block at the destination.
            app.selected_mods = (at..at + targets.len()).collect();
            app.selected_mod = Some(at);
            settle_folds_after_move(app, at, targets.len(), &hidden);
            mods_changed(app);
            app.menu_mod = None;
        }
        Message::BatchSendBottom => {
            let mut targets = move_selection(app);
            if targets.is_empty() {
                return Task::none();
            }
            targets.sort_unstable();
            let end = app.mods.len();
            let hidden = hidden_by_folds(app);
            let at = move_block(&mut app.mods, &targets, end);
            app.selected_mods = (at..at + targets.len()).collect();
            app.selected_mod = Some(at);
            settle_folds_after_move(app, at, targets.len(), &hidden);
            mods_changed(app);
            app.menu_mod = None;
        }
        Message::DragStart(i) => {
            // Alt on a group header takes the whole group, header included - MO2's
            // gesture for moving a section rather than its label
            // (ModListView::mousePressEvent, modlistview.cpp:1444). It works on a
            // FOLDED group too: the hidden rows are still rows, so they travel.
            if app.modifiers.alt() && app.mods.get(i).is_some_and(|m| m.is_separator()) {
                app.focus = Pane::Mods;
                app.typing = false;
                let end = group_children(&app.mods, i).end;
                app.selected_mods = (i..end).collect();
                app.selected_mod = Some(i);
                app.sel_anchor = Some(i);
                app.menu_mod = None;
                app.rename = None;
                app.confirm_remove = None;
            // Only in load order. Under a sort or a grouping the insertion gaps
            // address the REAL list while the rows on screen are somewhere
            // else, so a drop moves a mod nobody aimed at - and an armed drag
            // also hijacks the edge auto-scroll on its way to being refused.
            // Here, at the press, because this is where a row actually arms one:
            // a mod row's `on_press` is `DragStart`, never `SelectMod`.
            if !can_reorder(app) {
                return Task::none();
            }
                app.drag_state = Some(DragState { from: i, gap: i, aimed: false });
                return Task::none();
            }
            // Arm a drag and (re)select the row, unless a modifier means the click
            // was a multi-select gesture (then leave the existing selection alone).
            if app.modifiers.control()
                || app.modifiers.command()
                || app.modifiers.shift()
            {
                return update(app, Message::SelectMod(i));
            }
            app.selected_mod = Some(i);
            // Pressing a row that is NOT already in the multi-selection collapses the
            // selection to it; pressing one that IS keeps the group (so a mis-press
            // does not wipe a careful Ctrl/Shift selection). Drag still moves one row.
            if !app.selected_mods.contains(&i) {
                app.selected_mods.clear();
            }
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
            // Only in load order. Under a sort or a grouping the insertion gaps
            // address the REAL list while the rows on screen are somewhere
            // else, so a drop moves a mod nobody aimed at - and an armed drag
            // also hijacks the edge auto-scroll on its way to being refused.
            // Here, at the press, because this is where a row actually arms one:
            // a mod row's `on_press` is `DragStart`, never `SelectMod`.
            if !can_reorder(app) {
                return Task::none();
            }
            app.drag_state = Some(DragState { from: i, gap: i, aimed: false });
        }
        Message::ModDoubleClick(i) => {
            // The modifier set is read HERE, not in the closure that emitted
            // this: `mouse_area` reports the click, and only `update` can see
            // which keys are down. Same split a plain row click already uses.
            let m = app.modifiers;
            if m.control() || m.command() {
                return update(app, Message::ModOpenFolder(i));
            }
            if m.shift() {
                return update(app, Message::ModVisitNexus(i));
            }
            return update(app, Message::ShowModInfo(i));
        }
        Message::PreviewFile(path) => {
            app.preview = Some(build_preview(&path));
        }
        Message::ClosePreview => app.preview = None,
        Message::ToolArgsAction(action) => {
            app.typing = true;
            if let Some(state) = app.executables.as_mut() {
                state.args_editor.perform(action);
                state.commit_buffers();
            }
        }
        Message::ExecAppIdChanged(t) => {
            app.typing = true;
            if let Some(state) = app.executables.as_mut() {
                state.app_id = t;
                state.commit_buffers();
            }
        }
        Message::ExecToggleHidden => {
            if let Some(state) = app.executables.as_mut() {
                if let Some(t) = state.selected.and_then(|i| state.merged.get_mut(i)) {
                    t.hidden = !t.hidden;
                }
            }
        }
        Message::ExecTogglePinned => {
            if let Some(state) = app.executables.as_mut() {
                if let Some(t) = state.selected.and_then(|i| state.merged.get_mut(i)) {
                    t.pinned = !t.pinned;
                }
            }
        }
        Message::ExecMakeShortcut => {
            // The buffers hold what has been TYPED; `merged` holds what was
            // committed. Without this the shortcut names the previous title,
            // because a field commits on the next keystroke, not on this click.
            if let Some(state) = app.executables.as_mut() {
                state.commit_buffers();
            }
            let (Some(state), Some(inst), Some(game)) =
                (app.executables.as_ref(), app.created.as_ref(), selected_game(app))
            else {
                return Task::none();
            };
            let Some(tool) = state.selected.and_then(|i| state.merged.get(i)).cloned() else {
                return Task::none();
            };
            match write_desktop_entry(inst, game.def.id, &tool) {
                Ok(path) => {
                    app.status = Some(format!("Shortcut written to {}", path.display()));
                }
                Err(e) => app.error = Some(format!("Could not write the shortcut: {e}")),
            }
        }
        Message::ModBackup(i) => {
            app.menu_mod = None;
            let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) else {
                return Task::none();
            };
            if m.is_separator() || m.is_backup() || m.unmanaged {
                return Task::none();
            }
            let mods_dir = inst.mods_dir();
            // `<name>_backup`, then `_backup2`, `_backup3`... A backup that
            // silently replaced the previous one would lose the state somebody
            // took a backup to keep.
            let mut dest = mods_dir.join(format!("{}_backup", m.name));
            let mut n = 2;
            while dest.exists() {
                dest = mods_dir.join(format!("{}_backup{n}", m.name));
                n += 1;
                if n > 99 {
                    app.error = Some("Too many backups of that mod already.".to_string());
                    return Task::none();
                }
            }
            let src = m.path.clone();
            let name = m.name.clone();
            let label = dest.file_name().unwrap_or_default().to_string_lossy().into_owned();
            // The lock covers the WRITE and nothing else. Held across the
            // refresh below it would deadlock this very handler: `flock` denies
            // a second descriptor even to the process that already holds one,
            // so the plugin refresh inside `reload_mods` would fail against a
            // lock we are holding ourselves.
            let copied = {
                let _lock = match inst.try_lock("the Eidos window") {
                    Ok(l) => l,
                    Err(e) => {
                        app.status = Some(format!("Busy: {e}."));
                        return Task::none();
                    }
                };
                // Claimed BEFORE the copy, so the cleanup below can only ever
                // remove a directory this handler brought into existence.
                // Removing `dest` unconditionally on failure would delete
                // whatever was already at that name if the loop above raced
                // another process.
                if let Err(e) = std::fs::create_dir(&dest) {
                    app.error = Some(format!("Could not back up {name}: {e}"));
                    return Task::none();
                }
                match copy_dir_contents(&src, &dest) {
                    Ok(()) => true,
                    Err(e) => {
                        // Ours, so safe to take back whole.
                        let _ = std::fs::remove_dir_all(&dest);
                        app.error = Some(format!("Could not back up {name}: {e}"));
                        false
                    }
                }
            };
            if copied {
                reload_mods(app);
                refresh_meta_cache(app);
                bump_views(app);
                app.conflicts = compute_conflicts(app);
                app.status = Some(format!("Saved a copy as {label}."));
            }
        }
        Message::ModRestoreBackup(i) => {
            app.menu_mod = None;
            app.confirm_restore = app.mods.get(i).map(|m| m.name.clone());
        }
        Message::ConfirmModRestoreBackup(name) => {
            app.confirm_restore = None;
            // Resolved by NAME at commit time, so a reload between the two
            // clicks cannot aim this at a different backup.
            let (Some(inst), Some(m)) =
                (app.created.as_ref(), app.mods.iter().find(|m| m.name == name))
            else {
                return Task::none();
            };
            if !m.is_backup() {
                return Task::none();
            }
            // The name minus the suffix - `X_backup` and `X_backup3` both came
            // from `X`.
            let stem = m.name.trim_end_matches(|c: char| c.is_ascii_digit());
            let Some(orig) = stem.strip_suffix("_backup").filter(|s| !s.is_empty()) else {
                app.error = Some("That backup's name does not say what it came from.".to_string());
                return Task::none();
            };
            let target = inst.mods_dir().join(orig);
            if !target.is_dir() {
                app.error =
                    Some(format!("{orig} is not in this instance any more - nothing to restore over."));
                return Task::none();
            }
            let src = m.path.clone();
            let orig = orig.to_string();
            // The old contents go FIRST, into a sibling that is removed only
            // once the copy succeeded: restoring over a half-copied folder is
            // how a restore loses both versions.
            let stash = inst.mods_dir().join(format!("{orig}.eidos-restoring"));
            // Again: the lock spans the write, and is released before anything
            // reads the instance back - see the note in `ModBackup`.
            let restored = {
                let _lock = match inst.try_lock("the Eidos window") {
                    Ok(l) => l,
                    Err(e) => {
                        app.status = Some(format!("Busy: {e}."));
                        return Task::none();
                    }
                };
                let _ = std::fs::remove_dir_all(&stash);
                if let Err(e) = std::fs::rename(&target, &stash) {
                    app.error = Some(format!("Could not restore {orig}: {e}"));
                    return Task::none();
                }
                match copy_dir_contents(&src, &target) {
                    Ok(()) => {
                        let _ = std::fs::remove_dir_all(&stash);
                        true
                    }
                    Err(e) => {
                        // Put it back exactly as it was.
                        let _ = std::fs::remove_dir_all(&target);
                        let _ = std::fs::rename(&stash, &target);
                        app.error =
                            Some(format!("Could not restore {orig}: {e} - nothing changed."));
                        false
                    }
                }
            };
            if restored {
                reload_mods(app);
                refresh_meta_cache(app);
                bump_views(app);
                // The restored mod's files are different ones now.
                app.conflicts = compute_conflicts(app);
                app.plugins = None;
                app.status = Some(format!("{orig} restored from its backup."));
            }
        }
        Message::FiletreeOpen(i, rel) => {
            let Some(base) = app.mods.get(i).map(|m| m.path.clone()) else { return Task::none() };
            let Some(path) = resolve_in_mod(&base, &rel) else {
                app.error = Some(format!("Refused to open {rel}: not a path inside this mod."));
                return Task::none();
            };
            return update(app, Message::OpenFolder(path));
        }
        Message::FiletreeRenameStart(i, rel) => {
            // Prefill with the NAME, not the path: a rename box holding
            // `Meshes/armour/x.nif` invites somebody to edit the directories,
            // which is a move, which is not what this is.
            app.tree_rename_text =
                rel.rsplit('/').next().unwrap_or(&rel).to_string();
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
            let Some((mod_name, rel)) = app.tree_rename.take() else { return Task::none() };
            let name = app.tree_rename_text.trim().to_string();
            app.tree_rename_text.clear();
            // By name, resolved now: an index would have been read against a
            // list anything could have reloaded while the box was open.
            let Some(base) = app.mods.iter().find(|m| m.name == mod_name).map(|m| m.path.clone())
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
            let (Some(from), Some(to)) =
                (resolve_in_mod(&base, &rel), resolve_in_mod(&base, &dest_rel))
            else {
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
            let Some(base) = app.mods.iter().find(|m| m.name == name).map(|m| m.path.clone())
            else {
                return Task::none();
            };
            let Some(path) = resolve_in_mod(&base, &rel) else {
                app.error = Some(format!("Refused to delete {rel}: not a path inside this mod."));
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
            let Some(name) = app.mods.get(i).map(|m| m.name.clone()) else { return Task::none() };
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
            let Some((mod_name, name)) = app.tree_new_folder.take() else { return Task::none() };
            let name = name.trim().to_string();
            let Some(base) = app.mods.iter().find(|m| m.name == mod_name).map(|m| m.path.clone())
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
        Message::ClearListOrder => {
            app.mod_sort = None;
            app.group_by = None;
            app.groups_collapsed.clear();
            app.view_menu_open = false;
        }
        Message::SetGroupBy(by) => {
            app.group_by = by;
            // A drag armed under the old shape would drop somewhere that no
            // longer means what the user aimed at - the same reason sorting
            // drops one.
            app.drag_state = None;
            app.drag_hover_group = None;
            app.groups_collapsed.clear();
            app.view_menu_open = false;
        }
        Message::ToggleGroupFold(label) => {
            if !app.groups_collapsed.remove(&label) {
                app.groups_collapsed.insert(label);
            }
            // Whatever the fold just hid stops being selected. A focus on a row
            // nobody can see is a Delete away from removing a mod off screen,
            // and the same rule already applies to the filter.
            forget_hidden_rows(app);
        }
        Message::ToggleModColumn(col) => {
            if let Some(pos) = app.mod_columns.iter().position(|c| *c == col) {
                app.mod_columns.remove(pos);
                // Ordering a list by a column nobody can see is a list that
                // looks shuffled for no reason.
                if app.mod_sort.is_some_and(|s| s.by == SortKey::Column(col)) {
                    app.mod_sort = None;
                }
            } else {
                app.mod_columns.push(col);
                // Redrawn in the canonical order, so toggling one on twice
                // cannot move another.
                app.mod_columns = ModColumn::ALL
                    .into_iter()
                    .filter(|c| app.mod_columns.contains(c))
                    .collect();
            }
            app.prefs.mod_columns =
                Some(app.mod_columns.iter().map(|c| c.key().to_string()).collect());
            if let Err(e) = app.prefs.save() {
                app.error = Some(format!("Could not save settings: {e}"));
            }
            app.view_menu_open = false;
        }
        Message::CycleModSort(key) => {
            // Ascending, descending, then off. Getting BACK to load order has to
            // be one click away: it is the only order in which dragging works,
            // and a list somebody cannot un-sort is a list they cannot reorder.
            app.mod_sort = match app.mod_sort {
                Some(s) if s.by == key && s.ascending => {
                    Some(ModSort { by: key, ascending: false })
                }
                Some(s) if s.by == key => None,
                _ => Some(ModSort { by: key, ascending: true }),
            };
            // A drag armed under the old order would drop somewhere that no
            // longer means what the user aimed at.
            app.drag_state = None;
            app.drag_hover_group = None;
        }
        Message::HideDownload(name) => {
            // MO2's `removed=` in the sidecar, so hiding here hides there too -
            // and, crucially, the ARCHIVE stays. That is the whole point: the
            // list is a library, and putting a book away is not burning it.
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            let archive = inst.downloads_dir().join(&name);
            // Toggle, so the same button brings it back - a one-way hide with no
            // visible undo is how a library loses things.
            let hiding = !app.downloads.iter().any(|r| r.name == name && r.hidden);
            // Through `ModMeta`, which CREATES the sidecar when there is none.
            // `set_download_meta_key` edits an existing file, and an archive
            // copied in by hand has no sidecar at all - which is exactly the
            // pile somebody wants to hide.
            let meta_path = eidos_nexus::meta_path_for(&archive);
            let mut meta = eidos_instance::ModMeta::read(&meta_path);
            meta.set("removed", if hiding { "true" } else { "false" });
            match meta.write(&meta_path) {
                Ok(()) => {
                    load_downloads(app);
                    app.status = Some(if hiding {
                        format!("{name} hidden. Show hidden to bring it back.")
                    } else {
                        format!("{name} is back in the list.")
                    });
                }
                Err(e) => app.error = Some(format!("Could not update {name}: {e}")),
            }
        }
        Message::ToggleShowHiddenDownloads => {
            app.dl_show_hidden = !app.dl_show_hidden;
            load_downloads(app);
        }
        Message::DownloadFilterChanged(t) => {
            app.typing = true;
            app.dl_filter = t;
            load_downloads(app);
        }
        Message::DownloadSortChanged(sort) => {
            app.dl_sort = sort;
            load_downloads(app);
        }
        Message::PurgeInstalledDownloads => {
            // Two clicks, like every other action here that removes something -
            // and this one removes many at once.
            app.confirm_purge_installed = true;
        }
        Message::ConfirmPurgeInstalled => {
            app.confirm_purge_installed = false;
            let Some(inst) = app.created.as_ref() else { return Task::none() };
            let dir = inst.downloads_dir();
            // Only what is on screen AND installed. Not every installed archive
            // in the folder: the filter is how the user said which ones they
            // meant, and a bulk delete that ignores it deletes things they were
            // not looking at.
            let doomed: Vec<String> = app
                .downloads
                .iter()
                .filter(|r| r.state == DownloadState::Installed)
                .map(|r| r.name.clone())
                .collect();
            if doomed.is_empty() {
                app.status = Some("Nothing installed in this list to remove.".to_string());
                return Task::none();
            }
            let (mut gone, mut failed) = (0usize, 0usize);
            for name in &doomed {
                let archive = dir.join(name);
                // The sidecar goes with it. Leaving one behind would make the
                // archive reappear as a ghost row with no file.
                let sidecar = PathBuf::from(format!("{}.meta", archive.display()));
                match std::fs::remove_file(&archive) {
                    Ok(()) => {
                        let _ = std::fs::remove_file(&sidecar);
                        gone += 1;
                    }
                    Err(_) => failed += 1,
                }
            }
            load_downloads(app);
            app.status = Some(if failed == 0 {
                format!("Removed {gone} installed archive(s).")
            } else {
                format!("Removed {gone}; {failed} could not be deleted.")
            });
        }
        Message::ModMarkValid(i) => {
            app.menu_mod = None;
            let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) else {
                return Task::none();
            };
            let mut meta = inst.mod_meta(&m.name);
            meta.set_validated(true);
            let name = m.name.clone();
            match meta.write(&inst.meta_path(&name)) {
                // MO2's own key, so this silences the mod over there too.
                Ok(()) => {
                    invalidate_meta(app, &name);
                    refresh_meta_cache(app);
                    app.status = Some(format!("{name}: warnings marked as checked."));
                }
                Err(e) => app.error = Some(format!("Could not write meta.ini: {e}")),
            }
        }
        Message::ToolsDirChanged(t) => {
            app.typing = true;
            app.tools_dir_edit = t;
        }
        Message::ToolsDirSave => {
            let dir = app.tools_dir_edit.trim().to_string();
            // An empty box clears the setting rather than storing "": there is
            // no such directory, and a stored empty string would be a path that
            // fails a `is_dir` check on every tool list build for ever.
            app.prefs.tools_dir = Some(dir).filter(|d| !d.is_empty());
            if let Err(e) = app.prefs.save() {
                app.error = Some(format!("Could not save settings: {e}"));
            }
            app.tools_dir_edit = app.prefs.tools_dir.clone().unwrap_or_default();
            // The list is built from this, so it has to be rebuilt now rather
            // than at some later refresh the user has to guess at.
            load_tools(app);
        }
        Message::BrowseToolsDir => {
            let mut dlg = rfd::AsyncFileDialog::new().set_title("Where your modding tools live");
            if let Some(cur) = app.prefs.tools_dir.as_deref().filter(|d| !d.is_empty()) {
                dlg = dlg.set_directory(cur);
            }
            return Task::perform(dlg.pick_folder(), |h| match h {
                Some(h) => Message::ToolsDirChanged(h.path().display().to_string()),
                None => Message::Noop,
            });
        }
        Message::FocusFilter => {
            app.typing = true;
            return operation::focus(filter_input_id());
        }
        Message::JumpToLetter(c) => {
            // Only the mod list, and only the rows it is DRAWING: jumping to a
            // row hidden by the filter or folded into a collapsed group moves a
            // highlight nobody can see.
            if effective_focus(app) != Pane::Mods {
                return Task::none();
            }
            let rows = drawn_mod_rows(app);
            if rows.is_empty() {
                return Task::none();
            }
            let want = c.to_ascii_lowercase();
            // From the row AFTER the current one, wrapping, so pressing the same
            // letter walks every match instead of sticking on the first.
            let start = app
                .selected_mod
                .and_then(|sel| rows.iter().position(|&r| r == sel))
                .map(|p| p + 1)
                .unwrap_or(0);
            let hit = (0..rows.len()).map(|k| (start + k) % rows.len()).find(|&pos| {
                app.mods
                    .get(rows[pos])
                    .and_then(|m| m.display_name().chars().next())
                    .is_some_and(|f| f.to_ascii_lowercase() == want)
            });
            let Some(pos) = hit else { return Task::none() };
            app.selected_mod = Some(rows[pos]);
            app.sel_anchor = Some(rows[pos]);
            app.selected_mods.clear();
            app.menu_mod = None;
            app.confirm_remove = None;
            return scroll_focus_into_view(mod_scroll_id(), pos, rows.len());
        }
        Message::DragHoverTick => {
            // One tick of rest, not zero: brushing past a collapsed group on the
            // way somewhere else must not open it. Two ticks would be safer and
            // slower than the gesture is worth.
            let Some((name, ticks)) = app.drag_hover_group.take() else { return Task::none() };
            if ticks == 0 {
                app.drag_hover_group = Some((name, 1));
                return Task::none();
            }
            // Expanding inserts rows AFTER the separator, so the gap the drag is
            // aimed at - the separator's own index - still means what it meant.
            app.collapsed.remove(&name);
            save_collapsed(app);
        }
        Message::DragOverGap(_) if !can_reorder(app) => {
            // Belt and braces: without this a drag_state left over from before
            // a sort was applied would still aim, and aiming is what turns the
            // edge auto-scroll bands on.
            app.drag_state = None;
            app.drag_hover_group = None;
        }
        Message::DragOverGap(gap) => {
            // Resting on a collapsed group opens it, so a mod can be dropped
            // inside one without abandoning the drag to expand it first.
            let hovering = app
                .mods
                .get(gap)
                .filter(|m| m.is_separator() && app.collapsed.contains(m.display_name()))
                .map(|m| m.display_name().to_string());
            match (hovering, &app.drag_hover_group) {
                // Still the same group: leave the counter alone, it is what the
                // tick is counting.
                (Some(name), Some((cur, _))) if *cur == name => {}
                (Some(name), _) => app.drag_hover_group = Some((name, 0)),
                (None, _) => app.drag_hover_group = None,
            }
            if let Some(d) = &mut app.drag_state {
                // Every gap is a target now, the top of the list included. The
                // clamp here matched the one in `move_mod_rows` and rested on the
                // same premise - unmanaged rows outside modlist.txt - which no
                // longer holds.
                let want = gap.min(app.mods.len());
                d.aimed |= want != d.from && want != d.from + 1;
                d.gap = want;
            }
        }
        Message::DragDrop => {
            app.drag_hover_group = None;
            let Some(d) = app.drag_state.take() else { return Task::none() };
            if !can_reorder(app) {
                return Task::none();
            }
            if d.from >= app.mods.len() {
                return Task::none();
            }
            // Drag the whole selection when the grabbed row belongs to it (MO2
            // moves the block); otherwise just the grabbed row. Same helper every
            // other row-targeted action uses, so a drag and a "send to top" agree
            // about what "the rows I am acting on" means.
            let block = selection_or(app, d.from);
            if block.is_empty() {
                return Task::none();
            }
            // A drop that changes nothing: the pointer never left the grabbed
            // row's own edges, so this was a click. `aimed` is what makes that
            // true for a MULTI-row selection too, which has no single edge - and
            // where committing would compact a non-contiguous set and save it.
            // THE SHARED PREDICATE, not a local re-derivation: the view draws
            // the insertion line through the same function, so "no line" and
            // "no move" can never disagree - they did, and a multi-row drag
            // released back on its own row moved the block with no line shown.
            let unchanged = mod_drop_is_noop(app, &d);
            if !unchanged {
                let hidden = hidden_by_folds(app);
                let at = move_block(&mut app.mods, &block, d.gap);
                app.selected_mod = Some(at);
                // The anchor was left pointing at a pre-move index, so the next
                // Shift+click built its run from a row nobody had chosen.
                app.sel_anchor = Some(at);
                // A block dragged as one stays selected where it landed, so it can
                // be dragged again without rebuilding the selection; a single row
                // still collapses to a plain focus, as it always did.
                app.selected_mods = if block.len() > 1 {
                    (at..(at + block.len()).min(app.mods.len())).collect()
                } else {
                    HashSet::new()
                };
                settle_folds_after_move(app, at, block.len(), &hidden);
                mods_changed(app);
            }
        }
        Message::DragCancel => {
            app.drag_state = None;
        }
        Message::DragScrollEdge(edge) => {
            // Only meaningful mid-drag: the bands are not rendered otherwise, but
            // a stale message must not start a scroll on its own.
            app.drag_scroll =
                edge.filter(|_| app.drag_state.is_some() || app.download_drag.is_some());
            // Entering starts mid-range: `on_move` has not fired yet, and a band
            // that began at full speed would lurch before the user had aimed.
            if app.drag_scroll.is_some() {
                app.drag_scroll_depth = 0.5;
            }
        }
        Message::DragScrollDepth(d) => {
            app.drag_scroll_depth = d.clamp(0.0, 1.0);
        }
        Message::DragScrollTick => {
            let Some(edge) = app.drag_scroll else { return Task::none() };
            if app.drag_state.is_none() && app.download_drag.is_none() {
                app.drag_scroll = None;
                return Task::none();
            }
            // RELATIVE, so this never needs to know where the list already is -
            // which is exactly what the first version got wrong.
            // Speed follows how deep into the band the pointer is: a nudge
            // creeps a row at a time, the very edge crosses the list.
            let px = (DRAG_SCROLL_SLOW_PX
                + (DRAG_SCROLL_FAST_PX - DRAG_SCROLL_SLOW_PX) * app.drag_scroll_depth)
                * app.prefs.drag_scroll_speed;
            let y = match edge {
                ScrollEdge::Up => -px,
                ScrollEdge::Down => px,
            };
            return operation::scroll_by(
                mod_scroll_id(),
                iced::widget::scrollable::AbsoluteOffset { x: 0.0, y },
            );
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
            if app.fomod.is_some() || app.picker.is_some() || app.collision.is_some() {
                return Task::none();
            }
            let Some(path) = app.dropped.first().cloned() else { return Task::none() };
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
            let Some(d) = app.downloads.get(row) else { return Task::none() };
            // A partial has nothing to install; MO2 refuses the same gesture.
            if d.state == DownloadState::Downloading {
                return Task::none();
            }
            app.download_drag =
                Some(DownloadDrag { path: d.path.clone(), gap: app.mods.len(), aimed: false });
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
            let Some(d) = app.download_drag.take() else { return Task::none() };
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
        Message::PointerReleased => {
            // Letting go is a DROP wherever a gap is aimed, and a cancel
            // otherwise - regardless of where the pointer happens to be. A user
            // who dragged upward to scroll and released over the toolbar means
            // the gap they were aiming at, not "nowhere".
            // Cleared FIRST, on every path: the drop branches below return
            // early, so leaving it to the end meant a drag that ended on a gap
            // kept its timer running with nothing left to scroll for.
            app.drag_scroll = None;
            if app.drag_state.is_some_and(|d| d.aimed) {
                return update(app, Message::DragDrop);
            }
            if app.plugin_drag.as_ref().is_some_and(|d| d.aimed) {
                return update(app, Message::PluginDragDrop);
            }
            if app.download_drag.as_ref().is_some_and(|d| d.aimed) {
                return update(app, Message::DownloadDragDrop);
            }
            // Through the cancel messages rather than clearing the fields here,
            // so "what disarming means" has one definition and the keyboard path
            // (Escape) and this one cannot drift apart.
            let _ = update(app, Message::DragCancel);
            let _ = update(app, Message::DownloadDragCancel);
            return update(app, Message::PluginDragCancel);
        }
        Message::SelectPlugin(i) => {
            if app.modifiers.control() || app.modifiers.command() {
                return update(app, Message::SelectPluginToggle(i));
            }
            if app.modifiers.shift() {
                return update(app, Message::SelectPluginExtend(i));
            }
            app.focus = Pane::Plugins;
            app.typing = false;
            app.selected_plugin = Some(i);
            app.plugin_anchor = Some(i);
            // Pressing a row that is NOT already in the set collapses it, so a
            // mis-press does not silently wipe a careful Ctrl/Shift selection.
            if !app.selected_plugins.contains(&i) {
                app.selected_plugins.clear();
            }
            // One press, both jobs - selecting and arming the drag - exactly as
            // the mod list does it. Splitting them would mean a row could be
            // dragged without ever becoming the row the menus act on.
            return update(app, Message::PluginDragStart(i));
        }
        Message::SelectPluginToggle(i) => {
            app.typing = false;
            // The first toggle seeds the set from the current focus, so the
            // anchor row stays selected instead of vanishing.
            if app.selected_plugins.is_empty() {
                if let Some(f) = app.selected_plugin {
                    app.selected_plugins.insert(f);
                }
            }
            if !app.selected_plugins.remove(&i) {
                app.selected_plugins.insert(i);
            }
            app.focus = Pane::Plugins;
            app.selected_plugin = Some(i);
            app.plugin_anchor = Some(i);
            // A modifier click builds a selection; it must not also start a drag.
            app.plugin_drag = None;
        }
        Message::SelectPluginExtend(i) => {
            app.typing = false;
            let anchor = app.plugin_anchor.or(app.selected_plugin).unwrap_or(i);
            app.plugin_anchor = Some(anchor);
            let len = app.plugins.as_ref().map(|l| l.plugins.len()).unwrap_or(0);
            app.selected_plugins.clear();
            for idx in anchor.min(i)..=anchor.max(i) {
                if idx < len {
                    app.selected_plugins.insert(idx);
                }
            }
            app.focus = Pane::Plugins;
            app.selected_plugin = Some(i);
            // A modifier click builds a selection; it must not also start a drag.
            app.plugin_drag = None;
        }
        Message::SetSelectedPluginsEnabled(on) => {
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            // The SET when there is one. Going through the focus row would act on
            // a row the user had just Ctrl-clicked OFF: deselecting leaves the
            // focus on it, so `plugin_selection_or` would see it outside the set
            // and answer with that row alone - the one row they excluded.
            let len = app.plugins.as_ref().map(|l| l.plugins.len()).unwrap_or(0);
            let mut rows: Vec<usize> = if !app.selected_plugins.is_empty() {
                app.selected_plugins.iter().copied().collect()
            } else {
                app.selected_plugin.into_iter().collect()
            };
            rows.retain(|&i| i < len);
            rows.sort_unstable();
            if rows.is_empty() {
                app.status = Some("Select a plugin first.".to_string());
                return Task::none();
            }
            // Collect names first: the indices shift under refresh(), the names
            // do not, and a batch that half-applied would be worse than one that
            // did not start.
            let (names, refused) = {
                let Some(list) = app.plugins.as_ref() else { return Task::none() };
                let mut names = Vec::new();
                let mut refused = 0usize;
                for &i in &rows {
                    let Some(p) = list.plugins.get(i) else { continue };
                    let engine_owned = spec
                        .primary_plugins
                        .iter()
                        .any(|pp| pp.eq_ignore_ascii_case(&p.name))
                        || list.implicit.contains(&p.name.to_ascii_lowercase());
                    if engine_owned || p.force_disabled {
                        refused += 1;
                        continue;
                    }
                    names.push(p.name.clone());
                }
                (names, refused)
            };
            if names.is_empty() {
                app.status = Some(
                    "Nothing to change: the game loads those plugins itself.".to_string(),
                );
                return Task::none();
            }
            let held = hold_plugin_selection(app);
            if let Some(list) = app.plugins.as_mut() {
                for n in &names {
                    list.set_enabled(n, on);
                }
                // Enabling changes the tier a plugin sorts into, so this can
                // reorder the very rows the selection points at.
                list.refresh(&spec);
            }
            put_plugin_selection(app, held);
            let verb = if on { "Enabled" } else { "Disabled" };
            let tail = if refused > 0 {
                format!(" ({refused} left alone - the game loads them itself)")
            } else {
                String::new()
            };
            app.status = Some(format!("{verb} {} plugin(s).{tail}", names.len()));
            commit_plugin_order(app, &spec);
        }
        Message::PluginDragStart(i) => {
            // The legal range is resolved once, here, and not per frame: it can
            // only change when the list itself changes, which a drag cannot do.
            // The block this press will move: the whole selection when the
            // grabbed row belongs to it, so the range is computed for what will
            // actually travel rather than for one row of it.
            let block = plugin_selection_or(app, i);
            let range = selected_game(app)
                .and_then(|g| GameSpec::for_id(g.def.id))
                .zip(app.plugins.as_ref())
                .and_then(|(spec, list)| list.block_movable_range(&block, &spec));
            app.plugin_drag =
                range.map(|range| PluginDrag { from: i, gap: i, block, range, aimed: false });
        }
        Message::PluginDragOverGap(gap) => {
            if let Some(d) = &mut app.plugin_drag {
                // Clamped rather than rejected, so the indicator parks on the
                // nearest legal slot instead of vanishing when the pointer
                // wanders past the boundary. MO2 clamps illegal drops the same
                // way (pluginlist.cpp:1940-2016). A slot a pinned plugin owns is
                // skipped over, not clamped to: it is a hole in the middle of
                // the range, and resting the line there would promise a landing
                // the pin is going to take back.
                let want = gap.clamp(d.range.lo, d.range.hi);
                if !d.range.blocked.contains(&want) {
                    let (lo, hi) = (
                        d.block.first().copied().unwrap_or(d.from),
                        d.block.last().copied().unwrap_or(d.from),
                    );
                    // Only a gap OUTSIDE the block counts as aiming; the ones
                    // inside it are where the block already is.
                    d.aimed |= want < lo || want > hi + 1;
                    d.gap = want;
                }
            }
        }
        Message::PluginDragCancel => {
            app.plugin_drag = None;
        }
        Message::PluginDragDrop => {
            let Some(d) = app.plugin_drag.take() else { return Task::none() };
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            // A press that never travelled is a click, not a drag.
            if !d.aimed {
                return Task::none();
            }
            let held = hold_plugin_selection(app);
            let mut moved = false;
            if let Some(list) = app.plugins.as_mut() {
                // move_plugins_to carries the pin of what it moved across with
                // it, so a pinned plugin the user dragged keeps its NEW slot
                // instead of being snapped back by its own lock.
                moved = list.move_plugins_to(&d.block, d.gap, &spec);
                if moved {
                    list.refresh(&spec);
                }
            }
            // The rows just changed places: without this the highlight, the
            // "N selected" count and every batch action stay on the numbers,
            // which now name different plugins.
            put_plugin_selection(app, held);
            if !moved {
                // The gesture did nothing. If the plugin was boxed in by the
                // engine, say which plugins boxed it in rather than leaving the
                // row to snap back in silence - that silence is what made a
                // correct refusal read as a broken feature.
                if d.range.is_stuck(d.block.first().copied().unwrap_or(d.from)) {
                    app.status = Some(pinned_by(&d.range));
                }
                return Task::none();
            }
            commit_plugin_order(app, &spec);
        }
        Message::TogglePluginLock(i) => {
            let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
                return Task::none();
            };
            let mut changed = false;
            if let Some(list) = app.plugins.as_mut() {
                let now = list.is_locked(i);
                changed = list.set_locked(i, !now);
            }
            if !changed {
                return Task::none();
            }
            commit_plugin_order(app, &spec);
        }
        Message::PointerAt(p) => app.cursor = p,
        Message::WindowResized(s) => {
            app.window = s;
            // "Remember the window size" is a real setting now: it was stored,
            // and neither written nor applied, so the toggle did nothing at all.
            // Written on resize rather than at exit, because a window manager
            // can take the window away without an exit anybody sees.
            if app.prefs.remember_window {
                let want = Some((s.width.max(1.0) as u32, s.height.max(1.0) as u32));
                if app.prefs.window_size != want {
                    app.prefs.window_size = want;
                    // A failed write costs the next launch its size, nothing
                    // more - not worth an error banner over a resize.
                    let _ = app.prefs.save();
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
        Message::CycleFocus => {
            // Only somewhere there is a list to drive.
            app.focus = match app.focus {
                Pane::Mods if app.tab == Tab::Plugins => Pane::Plugins,
                _ => Pane::Mods,
            };
        }
        Message::SelectAllInFocus => match effective_focus(app) {
            Pane::Mods => {
                // Everything the list is DRAWING, separators included - MO2's
                // Ctrl+A does the same, and a reorder now carries them. The
                // destructive batch actions still spare separators, but on their
                // own terms (`real_selection`), so a Select All followed by
                // Remove does not delete the headers.
                //
                // What the list is DRAWING, because "all" has to mean what is on
                // screen: under a filter, a fold or a grouping, a wider set
                // swept in rows the user could not see, and the batch Remove
                // then aimed `remove_dir_all` at every one of them. Asking
                // `mod_row_visibility` was not enough - it knows the separator
                // folds and the filter, and nothing about a folded GROUP.
                app.selected_mods = drawn_mod_rows(app).into_iter().collect();
                let first = app.selected_mods.iter().min().copied();
                // The focus stays only if it is one of the rows just selected.
                app.selected_mod =
                    app.selected_mod.filter(|i| app.selected_mods.contains(i)).or(first);
            }
            Pane::Plugins => {
                let len = app.plugins.as_ref().map(|l| l.plugins.len()).unwrap_or(0);
                app.selected_plugins = (0..len).collect();
                app.selected_plugin = app.selected_plugin.or((len > 0).then_some(0));
            }
        },
        Message::KeyNav(nav) => return key_nav(app, nav),
        Message::ModifiersChanged(mods) => {
            app.modifiers = mods;
        }
        Message::Noop => {}
    }
    Task::none()
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
    let file = std::path::Path::new(&archive.replace('\\', "/")).file_name()?.to_owned();
    let sidecar = inst.downloads_dir().join(&file).with_extension({
        let mut ext = std::path::Path::new(&file).extension().unwrap_or_default().to_owned();
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
    bump_views(app);
    app.conflicts = compute_conflicts(app);
    app.plugins = None;
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
        .filter(|&i| app.mods.get(i).is_some_and(|m| !m.is_separator() && !m.is_unmanaged()))
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
    let Some(step_sel) = w.selection.get_mut(si) else { return };
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
        files,
        current,
    }
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
    LogPaneState { files, current, lines, level, total, truncated }
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
        let Some(m) = app.mods.get((origin as usize).saturating_sub(1)) else { continue };
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
        libc::timespec { tv_sec: 0, tv_nsec: libc::UTIME_OMIT },
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
    let Some(state) = app.collection.as_ref() else { return };
    let Some(rev) = state.revision.as_ref() else { return };

    let installed: std::collections::HashSet<u64> =
        app.meta_cache.values().filter_map(|r| r.mod_id).collect();
    // Every downloaded archive's file id, from the sidecars.
    let downloaded: std::collections::HashSet<u64> = match app.created.as_ref() {
        Some(inst) => std::fs::read_dir(inst.downloads_dir())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "meta"))
            .filter_map(|p| eidos_instance::ModMeta::read(&p).file_id())
            .collect(),
        None => std::collections::HashSet::new(),
    };

    let states: Vec<MemberState> = rev
        .mods
        .iter()
        .map(|m| {
            if installed.contains(&m.mod_id) {
                MemberState::Installed
            } else if downloaded.contains(&m.file_id) {
                MemberState::Downloaded
            } else {
                MemberState::Missing
            }
        })
        .collect();
    if let Some(state) = app.collection.as_mut() {
        state.states = states;
    }
}
