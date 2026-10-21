//! The window itself: tabs, menus, the toolbar and status bar, pointer,
//! keyboard, resizing and the timers.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::SelectTab(t) => {
            if app.tab != t {
                app.tab_prev = Some(app.tab);
                app.tab_anim.start();
            }
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
        Message::ClearStatus => {
            app.status = None;
        }
        Message::OpenUrl(url) => {
            // Only ever hand a real web link to the browser.
            if url.starts_with("https://") || url.starts_with("http://") {
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                app.status = Some(tf("status_opened", &[("path", &url)]));
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
                app.status = Some(t("status_refreshed").to_string());
            }
            load_tools(app);
            // F5 is also the "I just ran setcap" recheck for the warning banner.
            app.cap_missing = !eidos_launch::binary_has_cap_sys_admin(&find_eidos_binary().0);
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
                app.status = Some(tf("status_does_not_exist", &[("path", &p.display())]));
                return Task::none();
            }
            let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
            app.status = Some(tf(
                "status_opened_in_file_manager_path",
                &[("path", &p.display())],
            ));
        }
        // ---- menu bar --------------------------------------------------------
        Message::ShowAbout => {
            app.menu_mod = None;
            app.about_open = true;
        }
        Message::CloseAbout => app.about_open = false,
        Message::OpenViewMenu => {
            app.file_menu_open = false;
            return crate::widgets::measure(crate::widgets::menu_anchor_id(
                crate::view::VIEW_MENU_ANCHOR,
            ))
            .map(Message::ViewMenuAt);
        }
        Message::CloseViewMenu => app.view_menu_open = false,
        Message::OpenFileMenu => {
            // Only one dropdown at a time, or the two cards overlap and the one
            // underneath eats clicks aimed at the one on top.
            app.view_menu_open = false;
            // Opened by `FileMenuAt`, not here: the menu hangs from where the
            // button actually is, and that is a question only iced can answer.
            // One frame, and no wrong position ever drawn.
            return crate::widgets::measure(crate::widgets::menu_anchor_id(
                crate::view::FILE_MENU_ANCHOR,
            ))
            .map(Message::FileMenuAt);
        }
        Message::FileMenuAt(at) => {
            if at.is_some() {
                app.file_menu_at = at;
            }
            app.file_menu_open = true;
        }
        Message::ViewMenuAt(at) => {
            if at.is_some() {
                app.view_menu_at = at;
            }
            app.view_menu_open = true;
        }
        Message::FiltersAt(at) => {
            if at.is_some() {
                app.filters_at = at;
            }
            app.filters_open = true;
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
        Message::PointerReleased if app.split_drag => {
            app.split_drag = false;
            // Written once, on release, rather than on every pointer move: the
            // drag emits a message per frame and settings.ini is a file.
            app.prefs.split = app.split;
            if let Err(e) = app.prefs.save() {
                app.status = Some(tf("status_pane_width_failed", &[("error", &e)]));
            }
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
        Message::PointerAt(p) => {
            app.cursor = p;
            if app.split_drag && app.window.width > 1.0 {
                // Clamped well inside the edges: a pane dragged to zero is a
                // pane the user cannot grab again, and the divider goes with it.
                app.split = (p.x / app.window.width).clamp(0.15, 0.85);
            }
        }
        Message::SplitGrab => app.split_drag = true,
        Message::AnimationTick => {
            // The same frame tick drives every worker-thread dialog, and it has
            // to reach ALL of them. An early return on the first was fine while
            // there was only one; with two, a pack finishing while an extraction
            // was in flight would never be noticed, and its dialog would sit
            // there at 100% for as long as the window stayed open.
            let mut polls = Vec::new();
            if app.install_job.is_some() {
                polls.push(update_inner(app, Message::InstallPoll));
            }
            if app.transfer_job.as_ref().is_some_and(|j| j.running()) {
                polls.push(update_inner(app, Message::TransferPoll));
            }
            if !polls.is_empty() {
                return Task::batch(polls);
            }
        }
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
                app.selected_mod = app
                    .selected_mod
                    .filter(|i| app.selected_mods.contains(i))
                    .or(first);
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
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
