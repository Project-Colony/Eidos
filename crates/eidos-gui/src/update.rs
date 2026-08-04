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
    let task = update_inner(app, message);
    refresh_diagnostics(app);
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
pub(crate) fn is_ambient(m: &Message) -> bool {
    matches!(
        m,
        Message::PointerAt(_)
            | Message::WindowResized(_)
            | Message::FomodHover(_)
            | Message::FomodUnhover(..)
            // The downloads tick fires twice a second on its own. Left out of
            // this list it would disarm every confirmation before the second
            // click could land - the same defect as the pointer, on a timer.
            | Message::DownloadTick
    )
}

pub(crate) fn update_inner(app: &mut App, message: Message) -> Task<Message> {
    // A confirmation is armed by the first click and cancelled by any other
    // ACTION - including arming a different row. Ambient messages are not
    // actions and must leave it standing.
    if !is_ambient(&message) {
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
                let kind = app.kind;
                match inst.create() {
                    Ok(()) => {
                        if let Some(id) = &game_id {
                            let _ = inst.ensure_manifest(id, kind);
                        }
                        app.created = Some(inst);
                        reload_mods(app);
                        app.tab = Tab::Data;
                        app.error = None;
                        app.screen = Screen::Main;
                        load_tools(app);
                        app.conflicts = compute_conflicts(app);
                        refresh_meta_cache(app);
                        // Everything cached from a previously-open instance is
                        // stale for this one: plugin order, saves, downloads,
                        // selection and counts all belong to the old instance.
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
                        // The merged-view caches too, which the list above kept
                        // missing: they are keyed by directory and validated
                        // against `view_generation`, so without a bump the Data
                        // tab answers every already-listed directory out of the
                        // PREVIOUS instance. Switching from Skyrim to a game with
                        // no mods at all still showed Skyrim's merged Data tree,
                        // provenance labels and all.
                        drop_files_cache(app, None);
                        // And the tree's own navigation state, which names paths
                        // that need not exist in this game at all.
                        app.data_expanded.clear();
                        recompute_counts(app);
                    }
                    Err(e) => app.error = Some(e.to_string()),
                }
            }
        }
        Message::Restart => {
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
            let Some(path) = picked else { return Task::none() };
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
                        app.mods.iter().filter(|m| m.enabled && !m.is_separator()).map(|m| m.path.clone()).collect();
                    let disabled_roots: Vec<std::path::PathBuf> =
                        app.mods.iter().filter(|m| !m.enabled && !m.is_separator()).map(|m| m.path.clone()).collect();
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
                    app.picker = Some(InstallPicker {
                        rows: tree_rows(&tree),
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
                    app.picker = Some(InstallPicker {
                        rows: tree_rows(&tree),
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
                // `id` is Copy, so the immutable `game` borrow ends before `start_run`.
                if let Some(id) = selected_game(app).map(|g| g.def.id) {
                    let cmd = tool_command(id, &title);
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
                    .filter(|m| m.enabled && !m.is_separator())
                    .map(|m| m.path)
                    .collect();
                let both_active = eidos_gamefeatures::enb_cs_conflict(&game.install_path, &cs_roots);
                // `game`/`inst` are no longer used below; their borrows end here so
                // `start_run` can take `&mut app`.
                let (cmd, se_warning) = play_command(id, &app.launch_command);
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
            app.send_priority = Some((i, i.to_string()));
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
            let dest = dest.min(app.mods.len());
            let hidden = hidden_by_folds(app);
            let at = move_block(&mut app.mods, &targets, dest);
            app.selected_mod = Some(at);
            app.selected_mods.clear();
            settle_folds_after_move(app, at, targets.len(), &hidden);
            // The card hosting the editor is dismissed by the commit, not by the
            // click that armed it.
            app.menu_mod = None;
            mods_changed(app);
            app.status = Some(format!("Moved to priority {at}."));
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
            // A filter change can hide the menu's target row; keep it simple.
            app.menu_mod = None;
            app.rename = None;
            app.drag_state = None;
        }
        Message::CategoryFilterChanged(id) => {
            app.category_filter = id;
            app.menu_mod = None;
            app.rename = None;
            app.drag_state = None;
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
            let (lo, hi) = (anchor.min(i), anchor.max(i));
            app.selected_mods.clear();
            for idx in lo..=hi {
                if idx < app.mods.len() {
                    app.selected_mods.insert(idx);
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
                (Some(m), Some(inst)) if m.is_separator() => {
                    let mut meta = inst.mod_meta(&m.name);
                    meta.set_color(rgb);
                    Some((m.name.clone(), m.display_name().to_string(), meta.write(&inst.meta_path(&m.name))))
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
                Ok(report) => app.loot_report = Some(report),
                Err(e) => {
                    let base = app.status.take().unwrap_or_default();
                    app.status = Some(format!("{base} (LOOT report unavailable: {e})"));
                }
            }
        }
        Message::CollisionMerge => run_collision_install(app, eidos_install::OverwritePolicy::Merge),
        Message::CollisionReplace => run_collision_install(app, eidos_install::OverwritePolicy::Replace),
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
        Message::OpenInstanceFolder => {
            if let Some(inst) = &app.created {
                let _ = std::process::Command::new("xdg-open").arg(&inst.root).spawn();
                app.status = Some(format!("Opened {}", inst.root.display()));
            }
        }
        Message::SetupPrereqs => {
            let id = selected_game(app).map(|g| g.def.id);
            let has_prefix = selected_game(app).and_then(|g| g.compatdata.as_ref()).is_some();
            let log = app.created.as_ref().map(|i| i.root.join("prereqs.log"));
            match (id, log) {
                // A runtime needs no prefix and no Proton - it is a directory and
                // an environment variable - so a missing prefix must not block it.
                (Some(_), _) if !has_prefix && !any_runtime_pending(app) => {
                    app.status = Some(
                        "Launch the game once through Steam first so its Proton prefix exists, then run Tool Setup."
                            .to_string(),
                    );
                }
                (Some(id), Some(log)) => match run_prereqs_setup(id, &log) {
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
            let notes = match (app.created.as_ref(), app.mods.get(i)) {
                (Some(inst), Some(m)) => Some(inst.mod_meta(&m.name).notes().unwrap_or_default()),
                _ => None,
            };
            if let Some(notes) = notes {
                app.notes_edit = notes;
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
        Message::NotesSave => {
            let result = match (app.info_mod, app.created.as_ref()) {
                (Some(i), Some(inst)) => app.mods.get(i).map(|m| {
                    let mut meta = inst.mod_meta(&m.name);
                    meta.set_notes(&app.notes_edit);
                    (m.name.clone(), meta.write(&inst.meta_path(&m.name)))
                }),
                _ => None,
            };
            if let Some((name, r)) = result {
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
        Message::OverwriteToggleDir(rel) => {
            if !app.overwrite_expanded.remove(&rel) {
                app.overwrite_expanded.insert(rel);
            }
        }
        Message::ToggleFileHidden(i, rel) => {
            let Some(m) = app.mods.get(i).cloned() else { return Task::none() };
            let target = m.path.join(&rel);
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
            let hide = !path_is_hidden(&rel);
            match set_hidden(&target, hide) {
                Ok(_) => {
                    let verb = if hide { "Hid" } else { "Unhid" };
                    app.status = Some(format!("{verb} '{rel}' in '{}'.", m.name));
                    after_hidden_change(app, &m.name, &rel);
                }
                Err(e) => {
                    let verb = if hide { "hide" } else { "unhide" };
                    app.status = Some(format!("Could not {verb} '{rel}': {e}"));
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
            app.api_key_error = None;
            // Re-read the stored key so the field reflects what's on disk.
            app.settings_api_key = eidos_instance::settings::load_nexus_key().unwrap_or_default();
            app.settings_open = true;
        }
        Message::CloseSettings => {
            app.settings_open = false;
            app.api_key_error = None;
        }
        Message::SettingsTabSelected(t) => app.settings_tab = t,
        Message::ApiKeyChanged(s) => {
            app.settings_api_key = s;
            app.api_key_error = None;
        }
        Message::ApiKeyValidateStart => {
            let key = app.settings_api_key.trim().to_string();
            if key.is_empty() {
                app.api_key_error = Some("Enter your personal Nexus API key.".to_string());
                return Task::none();
            }
            if app.api_key_validating {
                return Task::none();
            }
            app.api_key_validating = true;
            app.api_key_error = None;
            // Blocking ureq inside the async closure, like SortPlugins.
            return Task::perform(
                async move {
                    let result = eidos_nexus::Nexus::new(&key).validate();
                    (key, result)
                },
                |(key, result)| Message::ApiKeyValidateResult(key, result),
            );
        }
        Message::ApiKeyValidateResult(key, result) => {
            app.api_key_validating = false;
            match result {
                Ok(account) => {
                    // Persist the key that was validated (the field may have been
                    // edited during the round-trip) so the CLI and a relaunch see it.
                    let saved = eidos_instance::settings::save_nexus_key(&key);
                    app.status = Some(match &saved {
                        Ok(()) => format!(
                            "Connected to Nexus as {} ({}).",
                            account.name,
                            if account.is_premium { "Premium" } else { "free" }
                        ),
                        Err(e) => format!("Validated, but could not save the key: {e}"),
                    });
                    app.nexus_account = Some(account);
                }
                Err(e) => {
                    app.api_key_error = Some(e);
                }
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
        Message::ToggleConflictMarks(on) => {
            app.prefs.conflict_marks = on;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::ToggleRememberWindow(on) => {
            app.prefs.remember_window = on;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
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
                let tool = Tool {
                    title: "New Tool".to_string(),
                    exe: PathBuf::new(),
                    args: Vec::new(),
                    workdir: None,
                    prereqs: Vec::new(),
                };
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
        Message::ToolArgsChanged(s) => {
            if let Some(state) = &mut app.executables {
                state.args = s;
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
                let user_tools: Vec<Tool> = state.merged[..state.user_len].to_vec();
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
        Message::CreateEmptyMod => {
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
                        // New mods land at the TOP of the list (highest priority,
                        // matching where a fresh install goes) - index = end of vec.
                        let idx = app.mods.len();
                        app.mods.push(entry);
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
                    let mut msg = format!(
                        "Update check: {} mods checked, {} update(s) found.",
                        r.checked, r.updates_found
                    );
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
        Message::OpenViewMenu => app.view_menu_open = true,
        Message::CloseViewMenu => app.view_menu_open = false,
        Message::ToggleToolbar => {
            app.ui_toolbar_visible = !app.ui_toolbar_visible;
            app.view_menu_open = false;
        }
        Message::ToggleStatusBar => {
            app.ui_statusbar_visible = !app.ui_statusbar_visible;
            app.view_menu_open = false;
        }
        Message::CollapseAllGroups => {
            // Collapse every separator's group (key by display name, like MO2).
            for m in &app.mods {
                if m.is_separator() {
                    app.collapsed.insert(m.display_name().to_string());
                }
            }
            save_collapsed(app);
            app.view_menu_open = false;
        }
        Message::ExpandAllGroups => {
            app.collapsed.clear();
            save_collapsed(app);
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
            // against it rather than left showing the old answer.
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
            let mut failed: Vec<String> = Vec::new();
            for e in fs::read_dir(&dir).into_iter().flatten().flatten() {
                let Ok(name) = e.file_name().into_string() else { continue };
                // The prefix is the whole guard, and it is checked HERE rather
                // than trusted from the diagnostic: the card the user clicked
                // may have been built before a refresh, and this loop deletes.
                if !name.starts_with(".eidos-install") {
                    continue;
                }
                match fs::remove_dir_all(e.path()) {
                    Ok(()) => gone += 1,
                    Err(err) => failed.push(format!("{name}: {err}")),
                }
            }
            app.status = Some(if failed.is_empty() {
                format!("Removed {gone} leftover extraction folder(s).")
            } else {
                format!("Removed {gone}; could not remove {}", failed.join(", "))
            });
            app.diag_dirty = true;
            reload_mods(app);
        }
        Message::DownloadTick => {
            // Cheap and bounded: one read_dir of a directory holding a few dozen
            // files. It is NOT called from view() - that lesson is already paid
            // for - so it costs twice a second, not once per frame.
            load_downloads(app);
        }
        Message::DeleteDownload(name) => {
            app.confirm_delete_download = Some(name);
        }
        Message::ConfirmDeleteDownload(name) => {
            // Armed and confirmed on the SAME file. The list re-sorts under a
            // background tick, so an index would have been a way to delete the
            // wrong archive by standing still.
            if app.confirm_delete_download.as_deref() == Some(name.as_str()) {
                if let Some(row) = app.downloads.iter().find(|r| r.name == name) {
                    let name = row.name.clone();
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
            app.drag_state = Some(DragState { from: i, gap: i, aimed: false });
        }
        Message::DragOverGap(gap) => {
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
            let Some(d) = app.drag_state.take() else { return Task::none() };
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
            let unchanged =
                !d.aimed || (block.len() == 1 && (d.gap == block[0] || d.gap == block[0] + 1));
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
            app.drag_scroll = edge.filter(|_| app.drag_state.is_some());
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
            if app.drag_state.is_none() {
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
            app.drag_state = None;
            app.plugin_drag = None;
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
        Message::WindowResized(s) => app.window = s,
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
                // Everything, separators included - MO2's Ctrl+A does the same, and
                // a reorder now carries them. The destructive batch actions still
                // spare them, but on their own terms (`real_selection`), so a
                // Select All followed by Remove does not delete the headers.
                app.selected_mods = (0..app.mods.len()).collect();
                app.selected_mod = app.selected_mod.or(Some(0)).filter(|_| !app.mods.is_empty());
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
pub(crate) fn real_selection(app: &App) -> Vec<usize> {
    let mut set = app.selected_mods.clone();
    if set.is_empty() {
        if let Some(f) = app.selected_mod {
            set.insert(f);
        }
    }
    set.into_iter()
        .filter(|&i| app.mods.get(i).is_some_and(|m| !m.is_separator()))
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
