//! The plugin list: enabling, sorting with LOOT, the context menu, selection,
//! dragging and locks.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::TogglePlugin(i) => {
            // Compute the spec + prefix dir up front (immutable borrows of `app`)
            // before mutating `app.plugins`.
            let spec = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id));
            let name = app
                .plugins
                .as_ref()
                .and_then(|l| l.plugins.get(i))
                .map(|p| p.name.clone());
            let forced = app
                .plugins
                .as_ref()
                .and_then(|l| l.plugins.get(i))
                .map(|p| p.force_disabled)
                .unwrap_or(false);
            let implicit = app
                .plugins
                .as_ref()
                .and_then(|l| {
                    l.plugins
                        .get(i)
                        .map(|p| l.implicit.contains(&p.name.to_ascii_lowercase()))
                })
                .unwrap_or(false);
            if let (Some(spec), Some(name)) = (spec, name) {
                // Base-game masters are implicit and always loaded; refuse to toggle.
                if spec
                    .primary_plugins
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case(&name))
                {
                    app.status = Some(tf("status_plugin_base_master", &[("name", &name)]));
                } else if implicit {
                    // Creation Club content the engine loads from the .ccc file.
                    // It is deliberately kept out of plugins.txt (writing it in
                    // makes the game see every Creation twice and blank the
                    // file), so a toggle here had nothing to write: the checkbox
                    // came straight back on at the next refresh with no
                    // explanation, which reads as the click being ignored.
                    app.status = Some(tf("status_plugin_creation_club", &[("name", &name)]));
                } else if forced {
                    app.status = Some(tf("status_plugin_light_unsupported", &[("name", &name)]));
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
                        Ok(_) => tf(
                            if now {
                                "status_plugin_disabled"
                            } else {
                                "status_plugin_enabled"
                            },
                            &[("name", &name)],
                        ),
                        Err(e) => {
                            // Refused write: drop the phantom toggle, resync to disk.
                            app.plugins = compute_plugins(app);
                            tf("status_load_order_write_failed", &[("error", &e)])
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
            if let Some(r) = &app.running {
                app.status = Some(tf("status_sort_while_running", &[("title", &r.title)]));
                return Task::none();
            }
            // One at a time. Without this every impatient click during the
            // masterlist download queued another complete sort, and each one
            // re-opened the report over a dialog the user had already closed -
            // minutes later, since they run strictly one after another.
            if app.sorting {
                app.status = Some(t("status_sort_already_running").to_string());
                return Task::none();
            }
            // Gather everything the (static) async closure needs, cloned out of
            // `app`, then run the masterlist fetch + LOOT sort off the UI thread.
            let Some(game) = selected_game(app) else {
                return Task::none();
            };
            let id = game.def.id;
            if !eidos_loot::is_supported(id) {
                app.status = Some(tf("status_loot_unsupported", &[("game", &id)]));
                return Task::none();
            }
            let Some(spec) = game.plugin_spec() else {
                return Task::none();
            };
            let Some(prefix) = game.prefix() else {
                app.status = Some(t("status_no_prefix").to_string());
                return Task::none();
            };
            let Some(list) = app.plugins.as_ref() else {
                app.status = Some(t("status_no_plugins_yet").to_string());
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
                .unwrap_or_else(|| eidos_plugins::plugin_state_dir(&prefix, &install, &spec));
            let cache = app
                .created
                .as_ref()
                .map(|i| i.root.join("loot"))
                .unwrap_or_else(|| eidos_instance::Instance::global(&id).root.join("loot"));
            let plugins: Vec<(String, PathBuf)> = list
                .plugins
                .iter()
                .map(|p| (p.name.clone(), p.path.clone()))
                .collect();
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
                instance: app.created.as_ref().map(|i| i.root.clone()),
                epoch: app.archive_epoch.get(),
                game: id.clone(),
                profile: app
                    .created
                    .as_ref()
                    .map(|i| i.active_profile())
                    .unwrap_or_default(),
                names: list.plugins.iter().map(|p| p.name.clone()).collect(),
            };
            app.sorting = true;
            app.status = Some(t("status_sorting").to_string());
            // `unblock`, not an `async` block: every step below is blocking
            // download, file walk and libloot work, and iced polls futures on
            // smol's single executor thread, so in a bare `async` block the
            // sort froze every timer and input subscription for its duration.
            // Not `background_work::run` either: its one slot belongs to
            // previews, and a sort must not queue behind a NIF render.
            return Task::perform(
                smol::unblock(move || {
                    // `is_supported(id)` was checked above and loot_support is a pure
                    // map, so this is always Some here; handle None gracefully anyway
                    // rather than unwrap (robust to any future refactor of the guard).
                    let repo = match eidos_loot::loot_support(&id) {
                        Some((_, repo)) => repo,
                        None => return Err(tf("status_loot_unsupported", &[("game", &id)])),
                    };
                    // Refresh the masterlist on every sort, like MO2/LOOT; a failed
                    // download falls back to the cached copy.
                    let (ml, pre) =
                        eidos_loot::ensure_masterlist(repo, true).map_err(|e| e.to_string())?;
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
                    match eidos_loot::add_case_bridge(&ml, &mut mod_dirs, &install, &bridge_dir) {
                        Ok(bridged) if !bridged.is_empty() => {
                            eprintln!(
                                "eidos: LOOT case bridge: {} path(s) spelled differently on disk ({})",
                                bridged.len(),
                                bridged.join(", ")
                            );
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
                }),
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
                    app.status = Some(tf("status_sort_failed", &[("error", &e)]));
                    return Task::none();
                }
            };
            // Rebuild a dropped cache before comparing the input snapshot.
            // A refresh invalidates the result too: the same names can now
            // refer to different files, flags or active states.
            if app.plugins.is_none() {
                app.plugins = compute_plugins(app);
            }
            // Refuse an answer computed for a list that has since changed. The
            // order LOOT returns is a permutation of the names it was GIVEN;
            // applied to a different set - after a profile switch, a mod enabled
            // or disabled, a mod installed - it silently rearranges plugins
            // nobody asked about, and everything downstream reports a clean sort.
            let now = SortFingerprint {
                instance: app.created.as_ref().map(|i| i.root.clone()),
                epoch: app.archive_epoch.get(),
                game: selected_game(app)
                    .map(|g| g.def.id.to_string())
                    .unwrap_or_default(),
                profile: app
                    .created
                    .as_ref()
                    .map(|i| i.active_profile())
                    .unwrap_or_default(),
                names: app
                    .plugins
                    .as_ref()
                    .map(|l| l.plugins.iter().map(|p| p.name.clone()).collect())
                    .unwrap_or_default(),
            };
            if now != asked_about {
                app.status = Some(if now.game != asked_about.game {
                    tf(
                        "status_sort_discarded_game",
                        &[("then", &asked_about.game), ("now", &now.game)],
                    )
                } else if now.profile != asked_about.profile {
                    tf(
                        "status_sort_discarded_profile",
                        &[("then", &asked_about.profile), ("now", &now.profile)],
                    )
                } else {
                    t("status_sort_discarded_list").to_string()
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
            let held = if pinned > 0 {
                format!(" {}", tf("status_sort_pinned_kept", &[("n", &pinned)]))
            } else {
                String::new()
            };
            let written = app
                .plugins
                .as_ref()
                .map(|list| write_plugin_state(app, list, &spec))
                .transpose();
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
                if pinned > 0 {
                    format!(", {pinned} pinned position(s) kept")
                } else {
                    String::new()
                }
            );
            app.status = Some(match written {
                Ok(_) => {
                    if changed == 0 {
                        tf(
                            "status_sort_unchanged",
                            &[("n", &sorted.len()), ("held", &held)],
                        )
                    } else {
                        tf(
                            "status_sorted",
                            &[("n", &sorted.len()), ("moved", &changed), ("held", &held)],
                        )
                    }
                }
                Err(e) => {
                    // Refused write: drop the phantom sort, resync to disk.
                    app.plugins = compute_plugins(app);
                    tf("status_sorted_write_failed", &[("error", &e)])
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
                    app.diag_dirty = true;
                }
                Err(e) => {
                    app.loot_meta = None;
                    app.diag_dirty = true;
                    let base = app.status.take().unwrap_or_default();
                    app.status = Some(format!(
                        "{base} {}",
                        tf("status_loot_report_unavailable", &[("error", &e)])
                    ));
                }
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
            app.menu_at = Some(crate::subscription::pointer(app));
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
                None => app.status = Some(t("status_plugin_from_game").into()),
            }
        }
        Message::ShowPluginOriginInfo(i) => {
            app.menu_plugin = None;
            match plugin_origin_row(app, i) {
                Some(row) => return update(app, Message::ShowModInfo(row)),
                None => app.status = Some(t("status_plugin_from_game").into()),
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
            let Some(anchor) = anchor else {
                return Task::none();
            };
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
                    t("status_plugins_already_there").to_string()
                } else {
                    t("status_plugins_move_refused").to_string()
                });
                return Task::none();
            }
            commit_plugin_order(app, &spec);
        }
        Message::PluginSendToPriorityStart => {
            // The anchor lives on the menu, and the field replaces a row INSIDE
            // that menu - so the menu must stay open.
            let Some(row) = app.menu_plugin.or(app.selected_plugin) else {
                return Task::none();
            };
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
            let Some((row, typed)) = app.plugin_send_priority.take() else {
                return Task::none();
            };
            app.menu_plugin = None;
            let Ok(dest) = typed.trim().parse::<usize>() else {
                // "Row number", not "load index": the only numeric column this
                // pane draws is the game's hex load index (00, FE:003, `--` for
                // a disabled plugin), and that is NOT what this takes. Asking
                // for one thing and meaning another is worse than asking plainly.
                app.status = Some(t("status_enter_row_number").to_string());
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
                app.status = Some(tf(
                    "status_plugins_cannot_put",
                    &[("n", &rows.len()), ("row", &dest)],
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
                app.status = Some(tf(
                    "status_plugins_moved",
                    &[("n", &rows.len()), ("row", &dest)],
                ));
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
                let Some(list) = app.plugins.as_ref() else {
                    return Task::none();
                };
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
                    t("status_plugins_all_active").to_string()
                } else {
                    t("status_plugins_none_to_deactivate").to_string()
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
            app.status = Some(tf(
                if on {
                    "status_plugins_activated"
                } else {
                    "status_plugins_deactivated"
                },
                &[("n", &names.len())],
            ));
            commit_plugin_order(app, &spec);
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
                app.status = Some(t("status_select_plugin_first").to_string());
                return Task::none();
            }
            // Collect names first: the indices shift under refresh(), the names
            // do not, and a batch that half-applied would be worse than one that
            // did not start.
            let (names, refused) = {
                let Some(list) = app.plugins.as_ref() else {
                    return Task::none();
                };
                let mut names = Vec::new();
                let mut refused = 0usize;
                for &i in &rows {
                    let Some(p) = list.plugins.get(i) else {
                        continue;
                    };
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
                app.status = Some(t("status_plugins_game_loads").to_string());
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
            let tail = if refused > 0 {
                format!(" {}", tf("status_plugins_left_alone", &[("n", &refused)]))
            } else {
                String::new()
            };
            app.status = Some(tf(
                if on {
                    "status_plugins_enabled_n"
                } else {
                    "status_plugins_disabled_n"
                },
                &[("n", &names.len()), ("tail", &tail)],
            ));
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
            app.plugin_drag = range.map(|range| PluginDrag {
                from: i,
                gap: i,
                block,
                range,
                aimed: false,
            });
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
            let Some(d) = app.plugin_drag.take() else {
                return Task::none();
            };
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
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
