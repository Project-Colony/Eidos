//! The mod list: rows, selection, the context menu, separators, batch actions,
//! ordering, dragging, filters, columns and export.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
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
            if app
                .mods
                .get(i)
                .is_some_and(|m| m.is_separator() || m.is_unmanaged())
            {
                return Task::none();
            }
            if let Some(m) = app.mods.get_mut(i) {
                m.enabled = !m.enabled;
            }
            mods_changed(app);
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
                    if let Some(mc) = map.mod_conflicts(origin) {
                        let set = if first {
                            &mc.overwrites
                        } else {
                            &mc.overwritten_by
                        };
                        related.extend(
                            set.iter()
                                .filter(|&&o| o != 0 && o != u32::MAX)
                                .map(|&o| (o - 1) as usize),
                        );
                    }
                }
            }
            let dest = if first {
                related.iter().min()
            } else {
                related.iter().max()
            };
            let Some(&dest) = dest else {
                app.status = Some(
                    if first {
                        t("status_overrides_nothing")
                    } else {
                        t("status_nothing_overrides")
                    }
                    .to_string(),
                );
                return Task::none();
            };
            // "Just below the last mod that overrides it" is one slot past it.
            let dest = if first {
                dest
            } else {
                (dest + 1).min(app.mods.len())
            };
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
            let Some((i, text)) = app.send_priority.take() else {
                return Task::none();
            };
            let Ok(dest) = text.trim().parse::<usize>() else {
                app.status = Some(t("status_enter_priority").to_string());
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
            // The toast quotes the same 1-based numbering the column shows.
            if mods_changed(app) {
                app.status = Some(tf("status_moved_to_priority", &[("n", &(at + 1))]));
            }
        }
        Message::SendToSeparatorStart(i) => {
            // Same as above: the chooser lives inside the menu card.
            app.menu_mod = Some(i);
            app.send_priority = None;
            app.send_separator = Some(i);
        }
        Message::SendToSeparatorPick(sep) => {
            let Some(i) = app.send_separator.take() else {
                return Task::none();
            };
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
            app.drag_state = Some(DragState {
                from: i,
                gap: i,
                aimed: false,
            });
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
                app.status = Some(tf("status_opened_in_file_manager", &[("name", &m.name)]));
            }
        }
        Message::ModVisitNexus(i) => {
            app.menu_mod = None;
            let domain = selected_game(app)
                .map(|g| g.def.nexus_game)
                .filter(|s| !s.is_empty());
            let mod_id = app
                .mods
                .get(i)
                .and_then(|m| app.meta_cache.get(&m.name))
                .and_then(|r| r.mod_id);
            match (domain, mod_id) {
                (Some(domain), Some(id)) => {
                    let url = format!("https://www.nexusmods.com/{domain}/mods/{id}");
                    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                    app.status = Some(tf("status_opening", &[("url", &url)]));
                }
                _ => {
                    app.status = Some(t("status_no_nexus_id").to_string());
                }
            }
        }
        Message::ModReinstallFresh(i) => {
            app.menu_mod = None;
            let Some(m) = app.mods.get(i) else {
                return Task::none();
            };
            if app
                .created
                .as_ref()
                .is_some_and(|inst| inst.mod_meta(&m.name).collection_owner().is_some())
            {
                app.status = Some(t("status_reinstall_collection_member").into());
                return Task::none();
            }
            return pick_mod_archive(app, Some(m.name.clone()), true);
        }
        Message::ModReinstall(i) => {
            app.menu_mod = None;
            let Some(m) = app.mods.get(i) else {
                return Task::none();
            };
            let name = m.name.clone();
            let owner = app
                .created
                .as_ref()
                .and_then(|inst| inst.mod_meta(&name).collection_owner());
            if let Some(owner) = owner {
                let folder = m.path.clone();
                app.mod_picker = app.mod_picker.wrapping_add(1);
                let request = app.mod_picker;
                let target = collection_target(app);
                return Task::perform(
                    crate::background_work::run(move || {
                        eidos_collections::installer_answers::archive(&folder, &owner)
                    }),
                    move |result| Message::InstallerArchiveResolved {
                        request,
                        target: target.clone(),
                        name: name.clone(),
                        result,
                    },
                );
            }
            return pick_mod_archive(app, Some(name), false);
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
                let Some(inst) = app.created.as_ref() else {
                    return Task::none();
                };
                let _lock = match inst.try_lock("the Eidos window") {
                    Ok(l) => l,
                    Err(e) => {
                        app.status = Some(tf("status_cannot_remove_now", &[("error", &e)]));
                        return Task::none();
                    }
                };
                // And known current before the deletion, as Rename does: the
                // save after it would refuse a list another process rewrote,
                // and its "redo the change" would then cover a delete that
                // already happened.
                if refuse_stale_modlist(app) {
                    return Task::none();
                }
                if let Some(m) = app.mods.get(i).cloned() {
                    match fs::remove_dir_all(&m.path) {
                        Ok(()) => {
                            app.mods.remove(i);
                            app.selected_mod = None;
                            app.selected_mods.clear();
                            app.drag_state = None;
                            drop_files_cache(app, Some(&m.name));
                            // Forgotten in every profile, not only this one: a line
                            // left in another profile counts as a missing mod there,
                            // and enough of them wedge it (see ConfirmBatchRemove).
                            let forgot = forget_removed_mods(app, std::slice::from_ref(&m.name));
                            app.status = Some(match forgot {
                                Some(Err(e)) => tf(
                                    "status_removed_list_failed",
                                    &[("name", &m.name), ("error", &e)],
                                ),
                                _ => tf("status_removed", &[("name", &m.name)]),
                            });
                            mods_changed(app);
                        }
                        Err(e) => app.status = Some(tf("status_remove_failed", &[("error", &e)])),
                    }
                }
            } else {
                app.confirm_remove = Some(i);
                if let Some(m) = app.mods.get(i) {
                    app.status = Some(tf("status_click_remove_again", &[("name", &m.name)]));
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
                    let new_name = if old.is_separator() {
                        format!("{typed}_separator")
                    } else {
                        typed.clone()
                    };
                    if typed.is_empty() || typed.contains('/') || typed.contains('\\') {
                        app.status = Some(t("status_invalid_name").to_string());
                    } else if new_name == old.name {
                        // no-op
                    } else if let Some(mods_dir) = app.created.as_ref().map(|inst| inst.mods_dir())
                    {
                        let dest = mods_dir.join(&new_name);
                        if dest.exists() {
                            app.status = Some(tf("status_name_exists", &[("name", &typed)]));
                        } else {
                            // Same flock as every sibling mutation: renaming the
                            // folder moves a live union layer out from under a
                            // running session, and the refused save afterwards
                            // would leave modlist.txt naming a folder that no
                            // longer exists. For the same reason the list must be
                            // known current BEFORE the rename: the lock is
                            // re-entrant, so the save can then only refuse a
                            // list another process rewrote, and that has to be
                            // caught while the folder still has its old name.
                            let Some(inst) = app.created.as_ref() else {
                                return Task::none();
                            };
                            let _lock = match inst.try_lock("the Eidos window") {
                                Ok(l) => l,
                                Err(e) => {
                                    app.status =
                                        Some(tf("status_cannot_rename_now", &[("error", &e)]));
                                    return Task::none();
                                }
                            };
                            if refuse_stale_modlist(app) {
                                return Task::none();
                            }
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
                                    // Renamed in every profile, not only this one:
                                    // the pool is shared, and a profile still
                                    // naming the old folder loses the mod's state
                                    // and order (see Profile::rename_mod).
                                    let renamed = rename_mod_lines(app, &old.name, &new_name);
                                    // Before mods_changed, which replaces it with
                                    // the reason if the save is refused.
                                    app.status = Some(match renamed {
                                        Some(Err(e)) => tf(
                                            "status_renamed_profiles_failed",
                                            &[("name", &typed), ("error", &e)],
                                        ),
                                        _ => tf("status_renamed", &[("name", &typed)]),
                                    });
                                    mods_changed(app);
                                }
                                Err(e) => {
                                    app.status = Some(tf("status_rename_failed", &[("error", &e)]))
                                }
                            }
                        }
                    }
                }
            }
        }
        Message::AddSeparator(i) => {
            app.menu_mod = None;
            // Under the lock and before the folder exists, as Rename does: a
            // refused save after it reloads the list, and the new folder comes
            // back unlisted at the end. Without the lock another process could
            // still write between the check and the save.
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(tf("status_cannot_add_separator_now", &[("error", &e)]));
                    return Task::none();
                }
            };
            if refuse_stale_modlist(app) {
                return Task::none();
            }
            let mods_dir = app.created.as_ref().map(|inst| inst.mods_dir());
            if let Some(mods_dir) = mods_dir {
                // A unique "Separator N" display name -> folder "<name>_separator".
                let mut n = 1usize;
                let mut display = t("new_separator_name").to_string();
                while mods_dir.join(format!("{display}_separator")).exists() {
                    n += 1;
                    display = format!("{} {n}", t("new_separator_name"));
                }
                let folder = format!("{display}_separator");
                let dest = mods_dir.join(&folder);
                match fs::create_dir_all(&dest) {
                    Ok(()) => {
                        // Minimal meta.ini, mirroring MO2's createMod.
                        let _ = fs::write(dest.join("meta.ini"), "[General]\nmodid=0\nversion=\n");
                        let idx = i.min(app.mods.len());
                        app.mods.insert(
                            idx,
                            ModEntry {
                                name: folder,
                                enabled: true,
                                path: dest,
                                unmanaged: false,
                            },
                        );
                        // Indices at/after the insertion point shifted.
                        app.selected_mods.clear();
                        // Only once the save landed: a refusal (a trust check,
                        // say) reloads the list, and the editor would then sit
                        // on whichever unrelated mod took this index.
                        if mods_changed(app) {
                            app.selected_mod = Some(idx);
                            // Open its rename editor so the user names it straight away.
                            app.rename = Some((idx, display));
                            app.menu_mod = Some(idx);
                        }
                    }
                    Err(e) => app.status = Some(tf("status_separator_failed", &[("error", &e)])),
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
                (Some(m), Some(inst)) if !m.is_unmanaged() => Some((
                    m.name.clone(),
                    m.display_name().to_string(),
                    edit_mod_meta(inst, &m.name, |meta| meta.set_color(rgb)),
                )),
                // Refused, and said out loud: a menu entry that does nothing at
                // all reads as a bug, not as a rule.
                (Some(m), _) if m.is_unmanaged() => {
                    app.status = Some(tf("status_game_content", &[("name", &m.display_name())]));
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
                        app.status = Some(tf("status_colour_set", &[("name", &display)]));
                    }
                    Err(e) => app.status = Some(tf("status_colour_failed", &[("error", &e)])),
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
        // ---- Mod-list filter pane (MO2's Filters tree) -----------------------
        Message::CycleFilter(field) => {
            let next = app.filters.get(field).next();
            app.filters.set(field, next);
            // The visible set just changed underneath every row index the
            // selection, the focus and the drag hold.
            forget_hidden_rows(app);
        }
        Message::ToggleFilterPane => {
            // Closing needs no measurement; opening does.
            if app.filters_open {
                app.filters_open = false;
                return Task::none();
            }
            return crate::widgets::measure(crate::widgets::menu_anchor_id(
                crate::view::FILTERS_ANCHOR,
            ))
            .map(Message::FiltersAt);
        }
        Message::ClearFilters => {
            app.filters = ModFilters::default();
            forget_hidden_rows(app);
        }
        // ---- per-mod local flags (Track / Ignore update) --------------------
        Message::ModTrack(i) => {
            app.menu_mod = None;
            if let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) {
                if !m.is_separator() {
                    let result = edit_mod_meta(inst, &m.name, |meta| {
                        let now = !meta.tracked();
                        meta.set_tracked(now);
                        now
                    });
                    let now = match result {
                        Ok(now) => now,
                        Err(e) => {
                            app.status = Some(tf("status_meta_save_failed", &[("error", &e)]));
                            return Task::none();
                        }
                    };
                    app.status = Some(tf(
                        if now {
                            "status_tracking"
                        } else {
                            "status_untracked"
                        },
                        &[("name", &m.display_name())],
                    ));
                }
            }
        }
        Message::ModIgnoreUpdate(i) => {
            app.menu_mod = None;
            if let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) {
                if !m.is_separator() {
                    let result = edit_mod_meta(inst, &m.name, |meta| {
                        let now = !meta.ignore_update();
                        meta.set_ignore_update(now);
                        now
                    });
                    let now = match result {
                        Ok(now) => now,
                        Err(e) => {
                            app.status = Some(tf("status_meta_save_failed", &[("error", &e)]));
                            return Task::none();
                        }
                    };
                    app.status = Some(tf(
                        if now {
                            "status_ignoring_updates"
                        } else {
                            "status_checking_updates"
                        },
                        &[("name", &m.display_name())],
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
            // Same as AddSeparator: checked under the lock, before the folder
            // is created.
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(tf("status_cannot_create_now", &[("error", &e)]));
                    return Task::none();
                }
            };
            if refuse_stale_modlist(app) {
                return Task::none();
            }
            if let Some(inst) = &app.created {
                // A unique "New Mod N" name, never colliding on disk.
                let mut n = 1usize;
                let mut name = t("new_mod_name").to_string();
                while inst.mods_dir().join(&name).exists() {
                    n += 1;
                    name = format!("{} {n}", t("new_mod_name"));
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
                        // Only once the save landed, as in AddSeparator.
                        if mods_changed(app) {
                            app.selected_mod = Some(idx);
                            app.selected_mods.clear();
                            // Open its rename editor so the user names it straight away.
                            app.rename = Some((idx, name));
                            app.menu_mod = Some(idx);
                        }
                    }
                    Err(e) => app.status = Some(tf("status_create_mod_failed", &[("error", &e)])),
                }
            }
        }
        Message::InstallFromFolder => {
            app.menu_mod = None;
            // Pick an already-unpacked mod directory off-thread.
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_title(t("dialog_pick_mod_folder"))
                    .pick_folder(),
                |handle| Message::FolderPicked(handle.map(|h| h.path().to_path_buf())),
            );
        }
        Message::FolderPicked(picked) => {
            let Some(src) = picked else {
                return Task::none();
            };
            let mods_dir = app.created.as_ref().map(|i| i.mods_dir());
            let Some(mods_dir) = mods_dir else {
                return Task::none();
            };
            // Name the new mod after the chosen folder (sanitized + de-duplicated).
            let raw = src
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let base = eidos_install::fix_directory_name(&raw)
                .unwrap_or_else(|| t("new_mod_name").to_string());
            let name = suggest_free_name(&mods_dir, &base);
            let dest = mods_dir.join(&name);
            // Copy the folder's CONTENTS into mods/<name>/ (not the folder itself),
            // mirroring how an archive's root is laid out.
            match copy_dir_contents(&src, &dest) {
                Ok(()) => after_install(app, &name, dest, false, None),
                Err(e) => {
                    let _ = fs::remove_dir_all(&dest);
                    app.status = Some(tf("status_install_folder_failed", &[("error", &e)]));
                }
            }
        }
        // ---- Export the mod list (MO2's Export to csv) -----------------------
        Message::ShowExportDialog => {
            app.file_menu_open = false;
            app.view_menu_open = false;
            if app.created.is_none() {
                app.status = Some(t("status_open_instance_first").to_string());
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
            let Some(d) = &app.export else {
                return Task::none();
            };
            if d.picked().is_empty() {
                app.status = Some(t("export_tick_column").to_string());
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
                    .set_title(t("export_title"))
                    .save_file(),
                |handle| Message::ExportPicked(handle.map(|h| h.path().to_path_buf())),
            );
        }
        Message::ExportPicked(picked) => {
            let Some(path) = picked else {
                return Task::none();
            };
            let Some(inst) = app.created.clone() else {
                return Task::none();
            };
            let Some(d) = app.export.take() else {
                return Task::none();
            };
            // Re-checked HERE, not only at the click that opened the picker: the
            // window keeps handling events while the native save dialog is up,
            // so the ticks can be cleared in between.
            if d.picked().is_empty() {
                app.status = Some(t("status_export_no_columns").to_string());
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
                    app.status = Some(tf(
                        "status_exported",
                        &[("n", &count), ("path", &path.display())],
                    ))
                }
                Err(e) => {
                    app.status = Some(tf(
                        "status_write_failed",
                        &[("path", &path.display()), ("error", &e)],
                    ))
                }
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
                app.status = Some(t("status_nothing_on_screen").to_string());
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
                app.status = Some(tf(
                    if enable {
                        "status_already_enabled"
                    } else {
                        "status_already_disabled"
                    },
                    &[("n", &targets.len())],
                ));
                return Task::none();
            }
            let saved = mods_changed(app);
            app.view_menu_open = false;
            // A refused save left its reason in the status; keep it there.
            if !saved {
                return Task::none();
            }
            // A collapsed group hides rows exactly as a filter does, and the
            // status has to admit either.
            let total = app
                .mods
                .iter()
                .filter(|m| !m.is_separator() && !m.is_unmanaged())
                .count();
            let narrowed = targets.len() < total;
            app.status = Some(match (enable, narrowed) {
                (true, false) => tf("status_enabled_n", &[("n", &changed)]),
                (false, false) => tf("status_disabled_n", &[("n", &changed)]),
                (true, true) => tf(
                    "status_enabled_n_on_screen",
                    &[
                        ("n", &changed),
                        ("shown", &targets.len()),
                        ("total", &total),
                    ],
                ),
                (false, true) => tf(
                    "status_disabled_n_on_screen",
                    &[
                        ("n", &changed),
                        ("shown", &targets.len()),
                        ("total", &total),
                    ],
                ),
            });
        }
        Message::BatchToggleMods => {
            // MO2-style batch enable/disable: if any selected real mod is enabled,
            // the whole selection is disabled; otherwise the whole selection is
            // enabled. Separators carry no toggle and are skipped.
            // A backup is inert: ticking one would write `+X_backup` (see ToggleMod).
            let targets: Vec<usize> = real_selection(app)
                .into_iter()
                .filter(|&i| app.mods.get(i).is_some_and(|m| !m.is_backup()))
                .collect();
            if targets.is_empty() {
                app.status = Some(t("status_select_mods_first").to_string());
                return Task::none();
            }
            let any_on = targets
                .iter()
                .any(|&i| app.mods.get(i).is_some_and(|m| m.enabled));
            let new_state = !any_on;
            for &i in &targets {
                if let Some(m) = app.mods.get_mut(i) {
                    m.enabled = new_state;
                }
            }
            app.menu_mod = None;
            if mods_changed(app) {
                app.status = Some(tf(
                    if new_state {
                        "status_enabled_n"
                    } else {
                        "status_disabled_n"
                    },
                    &[("n", &targets.len())],
                ));
            }
        }
        Message::BatchRemoveMods => {
            let n = real_selection(app).len();
            if n == 0 {
                app.status = Some(t("status_select_mods_first").to_string());
                return Task::none();
            }
            app.confirm_batch_remove = true;
            app.status = Some(tf("status_click_remove_again_n", &[("n", &n)]));
        }
        Message::ConfirmBatchRemove => {
            app.confirm_batch_remove = false;
            app.menu_mod = None;
            // Same flock as the single-row Remove, before the first deletion: a
            // batch of remove_dir_all against layers of a live union is the same
            // hazard multiplied, and refusing after the loop would protect nothing.
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(tf("status_cannot_remove_mods_now", &[("error", &e)]));
                    return Task::none();
                }
            };
            // Known current before the first deletion too, as in ModRemove.
            if refuse_stale_modlist(app) {
                return Task::none();
            }
            // Delete from the highest index down so the lower indices stay valid.
            let mut targets = real_selection(app);
            targets.sort_unstable();
            let mut gone: Vec<String> = Vec::new();
            let mut failed = 0usize;
            for &i in targets.iter().rev() {
                if let Some(m) = app.mods.get(i).cloned() {
                    match fs::remove_dir_all(&m.path) {
                        Ok(()) => {
                            app.mods.remove(i);
                            drop_files_cache(app, Some(&m.name));
                            gone.push(m.name);
                        }
                        Err(_) => failed += 1,
                    }
                }
            }
            app.selected_mods.clear();
            app.selected_mod = None;
            let removed = gone.len();
            let mut status = if failed == 0 {
                tf("status_removed_n", &[("n", &removed)])
            } else {
                tf(
                    "status_removed_n_failed",
                    &[("n", &removed), ("failed", &failed)],
                )
            };
            // Before the save: every profile's modlist.txt still lists the deleted
            // folders, and a big enough batch of listed-but-missing mods is exactly
            // what an unmounted drive looks like - the save, and every one after
            // it, would be refused. Only the folders really deleted are forgotten:
            // one that failed keeps its line, its slot and its enabled state.
            let forgot = forget_removed_mods(app, &gone);
            if let Some(Err(e)) = forgot {
                status = format!(
                    "{status} {}",
                    tf("status_list_update_failed", &[("error", &e)])
                );
            }
            // Set BEFORE mods_changed, which replaces it with the reason if the
            // save is refused; set after, it hid that refusal behind "Removed".
            app.status = Some(status);
            mods_changed(app);
        }
        Message::ForgetMissingMods => {
            // No confirmation step, unlike Remove: nothing on disk but list lines
            // goes, every profile it edits is backed up first (Backups in that
            // profile undoes it), and the case a second click would guard - the
            // unmounted drive, where everything looks missing - never gets here:
            // `missing_mods` refuses an unreadable or empty `mods/`.
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            // Held across the recheck, the backups and the edit, so no other
            // process rewrites a list between being backed up and being edited.
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(tf("status_cannot_do_that", &[("error", &e)]));
                    return Task::none();
                }
            };
            // Recomputed, not taken from the card: it may predate a refresh,
            // and an unreadable folder may be the drive that is not mounted.
            // Same rule as the card: only when the active list's own mods are
            // missing, or the verdict is about modlist.txt itself, which this
            // cannot fix.
            let missing = match inst.missing_mods() {
                Ok(m) if !inst.active().listed_mods().iter().any(|n| m.contains(n)) => {
                    app.status = Some(t("status_forget_missing_none").to_string());
                    return Task::none();
                }
                Ok(m) => m,
                Err(e) => {
                    app.status = Some(tf("status_forget_missing_unreadable", &[("error", &e)]));
                    return Task::none();
                }
            };
            for name in inst.profiles() {
                let profile = inst.profile(&name);
                if !profile.listed_mods().iter().any(|n| missing.contains(n)) {
                    continue;
                }
                if let Err(e) = profile.create_backup(eidos_instance::BackupKind::ModList) {
                    app.status = Some(tf("status_forget_missing_failed", &[("error", &e)]));
                    return Task::none();
                }
            }
            app.status = Some(match forget_removed_mods(app, &missing) {
                Some(Err(e)) => tf("status_forget_missing_failed", &[("error", &e)]),
                _ => tf("status_forgot_missing", &[("n", &missing.len())]),
            });
            // The window never held these rows (the scan drops them), so there
            // is no in-memory edit to save: re-read the lists and what derives
            // from them, the way a refused stale save does.
            reload_mods(app);
            mod_views_changed(app);
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
                app.drag_state = Some(DragState {
                    from: i,
                    gap: i,
                    aimed: false,
                });
                return Task::none();
            }
            // Arm a drag and (re)select the row, unless a modifier means the click
            // was a multi-select gesture (then leave the existing selection alone).
            if app.modifiers.control() || app.modifiers.command() || app.modifiers.shift() {
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
            app.drag_state = Some(DragState {
                from: i,
                gap: i,
                aimed: false,
            });
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
        Message::ModBackup(i) => {
            app.menu_mod = None;
            let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) else {
                return Task::none();
            };
            if m.is_separator() || m.is_backup() || m.unmanaged {
                return Task::none();
            }
            let src = m.path.clone();
            let name = m.name.clone();
            let copied = inst
                .try_lock("backing up a mod")
                .and_then(|_lock| eidos_instance::backup_mod(&src));
            match copied {
                Ok(dest) => {
                    reload_mods(app);
                    refresh_meta_cache(app);
                    bump_views(app);
                    app.conflicts = compute_conflicts(app);
                    app.status = Some(tf("status_backup_saved", &[("path", &dest.display())]));
                }
                Err(error) => {
                    app.error = Some(tf(
                        "status_backup_failed",
                        &[("name", &name), ("error", &error)],
                    ))
                }
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
            let (Some(inst), Some(m)) = (
                app.created.as_ref(),
                app.mods.iter().find(|m| m.name == name),
            ) else {
                return Task::none();
            };
            if !m.is_backup() {
                return Task::none();
            }
            // The name minus the suffix - `X_backup` and `X_backup3` both came
            // from `X`.
            let stem = m.name.trim_end_matches(|c: char| c.is_ascii_digit());
            let Some(orig) = stem.strip_suffix("_backup").filter(|s| !s.is_empty()) else {
                app.error = Some(t("status_backup_unnamed").to_string());
                return Task::none();
            };
            let target = inst.mods_dir().join(orig);
            if !target.is_dir() {
                app.error = Some(tf("status_backup_original_gone", &[("name", &orig)]));
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
                        app.status = Some(tf("status_busy", &[("error", &e)]));
                        return Task::none();
                    }
                };
                let _ = std::fs::remove_dir_all(&stash);
                if let Err(e) = std::fs::rename(&target, &stash) {
                    app.error = Some(tf(
                        "status_restore_failed",
                        &[("name", &orig), ("error", &e)],
                    ));
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
                        app.error = Some(tf(
                            "status_restore_failed_unchanged",
                            &[("name", &orig), ("error", &e)],
                        ));
                        false
                    }
                }
            };
            if restored {
                reload_mods(app);
                // Its meta.ini came back with the folder, and the refresh only
                // computes rows it does not already hold.
                invalidate_meta(app, &orig);
                refresh_meta_cache(app);
                refresh_after_tree_change(app, Some(&orig));
                app.status = Some(tf("status_restored", &[("name", &orig)]));
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
            app.prefs.mod_columns = Some(
                app.mod_columns
                    .iter()
                    .map(|c| c.key().to_string())
                    .collect(),
            );
            if let Err(e) = app.prefs.save() {
                app.error = Some(tf("status_settings_save_failed", &[("error", &e)]));
            }
            app.view_menu_open = false;
        }
        Message::CycleModSort(key) => {
            // Ascending, descending, then off. Getting BACK to load order has to
            // be one click away: it is the only order in which dragging works,
            // and a list somebody cannot un-sort is a list they cannot reorder.
            app.mod_sort = match app.mod_sort {
                Some(s) if s.by == key && s.ascending => Some(ModSort {
                    by: key,
                    ascending: false,
                }),
                Some(s) if s.by == key => None,
                _ => Some(ModSort {
                    by: key,
                    ascending: true,
                }),
            };
            // A drag armed under the old order would drop somewhere that no
            // longer means what the user aimed at.
            app.drag_state = None;
            app.drag_hover_group = None;
        }
        Message::ModMarkValid(i) => {
            app.menu_mod = None;
            let (Some(inst), Some(m)) = (app.created.as_ref(), app.mods.get(i)) else {
                return Task::none();
            };
            let name = m.name.clone();
            match edit_mod_meta(inst, &name, |meta| meta.set_validated(true)) {
                // MO2's own key, so this silences the mod over there too.
                Ok(()) => {
                    invalidate_meta(app, &name);
                    refresh_meta_cache(app);
                    app.status = Some(tf("status_marked_valid", &[("name", &name)]));
                }
                Err(e) => app.error = Some(tf("status_meta_ini_failed", &[("error", &e)])),
            }
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
            let hit = (0..rows.len())
                .map(|k| (start + k) % rows.len())
                .find(|&pos| {
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
            let Some((name, ticks)) = app.drag_hover_group.take() else {
                return Task::none();
            };
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
            let Some(d) = app.drag_state.take() else {
                return Task::none();
            };
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
            let Some(edge) = app.drag_scroll else {
                return Task::none();
            };
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
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
