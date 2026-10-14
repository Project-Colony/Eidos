//! Tools: user extensions, the log pane, the INI editor and the executables
//! dialog.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
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
                0 => tf("status_extensions_loaded", &[("n", &app.addons.len())]),
                n => tf(
                    "status_extensions_loaded_refused",
                    &[("n", &app.addons.len()), ("refused", &n)],
                ),
            });
        }
        Message::RunAddon(id) => {
            let Some(a) = app.addons.iter().find(|a| a.id == id).cloned() else {
                return Task::none();
            };
            if let Some(why) = a.unavailable() {
                app.status = Some(tf("status_cannot_run", &[("name", &a.name), ("reason", &why)]));
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
                app.status = Some(tf(
                    "status_extension_needs_instance",
                    &[("name", &a.name), ("needs", &missing.join(", "))],
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
                    app.status = Some(tf("status_started_pid", &[("name", &a.name), ("pid", &child.id())]))
                }
                Err(e) => app.status = Some(tf("status_start_failed", &[("name", &a.name), ("error", &e)])),
            }
        }
        Message::OpenAddonsFolder => {
            let dir = eidos_addons::user_addons_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
            app.status = Some(tf("status_opened", &[("path", &dir.display())]));
        }
        // ---- Log pane ---------------------------------------------------------
        Message::ShowLogPane => {
            let files = eidos_log::sessions();
            app.view_menu_open = false;
            app.file_menu_open = false;
            let Some(newest) = files.first().cloned() else {
                app.status = Some(tf(
                    "status_no_session_logs",
                    &[("path", &eidos_log::log_dir().display())],
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
                let current = if files.contains(&pane.current) {
                    pane.current.clone()
                } else {
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
            let Some(pane) = &app.log_pane else {
                return Task::none();
            };
            let text: String = pane
                .lines
                .iter()
                .map(|(lvl, msg)| format!("{:<5} {msg}", lvl.as_str()))
                .collect::<Vec<_>>()
                .join("\n");
            app.status = Some(tf("status_log_copied", &[("n", &pane.lines.len())]));
            // The level is kept in the copied text: a pasted log with the
            // severities stripped is the half that stops being diagnosable.
            return iced::clipboard::write(text);
        }
        Message::LogOpenFolder => {
            let dir = eidos_log::log_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
            app.status = Some(tf("status_opened", &[("path", &dir.display())]));
        }
        // ---- INI editor -------------------------------------------------------
        Message::ShowIniEditor => {
            app.menu_mod = None;
            // These are all opened FROM the View dropdown, which otherwise stays
            // up over the dialog and swallows the first click aimed at it.
            app.view_menu_open = false;
            let (Some(inst), Some(game)) = (app.created.as_ref(), selected_game(app)) else {
                app.status = Some(t("status_open_instance_first").to_string());
                return Task::none();
            };
            let files: Vec<String> = game.def.ini_files.iter().map(|f| f.to_string()).collect();
            let Some(first) = files.first().cloned() else {
                app.status = Some(tf("status_no_managed_inis", &[("game", &game.def.name)]));
                return Task::none();
            };
            let inst = inst.clone();
            if let Err(e) = seed_ini_from_game(app, &first) {
                app.status = Some(e);
                return Task::none();
            }
            app.ini_editor = Some(load_ini_editor(&inst.active(), files, first));
        }
        Message::CloseIniEditor => app.ini_editor = None,
        Message::IniEditorPick(name) => {
            let Some(inst) = app.created.clone() else {
                return Task::none();
            };
            let Some(ed) = &app.ini_editor else {
                return Task::none();
            };
            if ed.current == name {
                return Task::none();
            }
            // Switching away from unsaved edits would lose them silently, and
            // this is a file the game reads - say so and stay put.
            if ed.dirty {
                app.status = Some(tf("status_ini_unsaved_switch", &[("file", &ed.current)]));
                return Task::none();
            }
            let files = ed.files.clone();
            if let Err(e) = seed_ini_from_game(app, &name) {
                app.status = Some(e);
                return Task::none();
            }
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
            let Some(inst) = app.created.clone() else {
                return Task::none();
            };
            let Some(ed) = &mut app.ini_editor else {
                return Task::none();
            };
            let path = inst.active().ini_path(&ed.current);
            if ed.unreadable {
                app.status = Some(tf("status_ini_unreadable", &[("file", &ed.current)]));
                return Task::none();
            }
            // Under the lock, and only over the file the buffer was read from.
            // The buffer may be minutes old: since then `eidos sort` or
            // `eidos collection` may have rewritten Morrowind.ini's
            // [Game Files], or a session written its INIs back into the
            // profile, and writing the buffer would revert that silently (the
            // re-read below would then even record the revert as current).
            let _lock = match inst.try_lock("the Eidos window") {
                Ok(l) => l,
                Err(e) => {
                    app.status = Some(tf("status_not_saved", &[("error", &e)]));
                    return Task::none();
                }
            };
            if content_hash(&std::fs::read(&path).ok()) != ed.on_disk {
                app.status = Some(tf("status_ini_changed_on_disk", &[("file", &ed.current)]));
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
                    ed.on_disk = content_hash(&std::fs::read(&path).ok());
                    app.archive_epoch
                        .set(app.archive_epoch.get().wrapping_add(1));
                    app.status = Some(tf(
                        "status_ini_saved",
                        &[("file", &ed.current), ("profile", &inst.active().name)],
                    ));
                    // Morrowind.ini (and plugins.txt) live in the profile's plugin
                    // state: the cached load order and the hash its next write is
                    // checked against are both stale now, and left so that write
                    // was refused as another process's change. Re-read both, which
                    // also picks up a `[Game Files]` line typed here.
                    if path.starts_with(inst.active().plugins_state_dir()) {
                        invalidate_plugins(app);
                    }
                }
                Err(e) => app.status = Some(tf("status_ini_save_failed", &[("file", &ed.current), ("error", &e)])),
            }
        }
        Message::IniEditorRevert => {
            if let Some(ed) = &mut app.ini_editor {
                ed.content = iced::widget::text_editor::Content::with_text(&ed.original);
                ed.dirty = false;
                app.status = Some(tf("status_ini_reverted", &[("file", &ed.current)]));
            }
        }
        Message::IniEditorOpenExternal => {
            let Some(inst) = app.created.as_ref() else {
                return Task::none();
            };
            let Some(ed) = &app.ini_editor else {
                return Task::none();
            };
            let path = inst.active().ini_path(&ed.current);
            if !path.is_file() {
                app.status = Some(tf("status_ini_not_yet", &[("file", &ed.current)]));
                return Task::none();
            }
            let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
            app.status = Some(tf("status_opened_externally", &[("path", &path.display())]));
        }
        // ---- Executables dialog ----------------------------------------------
        Message::ShowExecutablesDialog => {
            app.menu_mod = None;
            match open_executables_dialog(app) {
                Some(state) => app.executables = Some(state),
                None => app.status = Some(t("status_open_instance_first").to_string()),
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
                    title: t("new_tool_name").to_string(),
                    ..Default::default()
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
                // Same rule for arguments, and the reason is stronger: a missing
                // prereq usually surfaces as a runtime error naming the DLL, while
                // PGPatcher's missing `--ignore-mo2vfscheck` makes it exit at once
                // complaining about MO2, a program the user is not running. The
                // field is one per line, like the editor shows it.
                if state.args_editor.text().trim().is_empty() {
                    let seeded = eidos_instance::default_args(&s).join("\n");
                    if !seeded.is_empty() {
                        state.args_editor = iced::widget::text_editor::Content::with_text(&seeded);
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
                .add_filter(t("dialog_executables"), &["exe"])
                .set_title(t("dialog_pick_tool"));
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
            let mut dlg = rfd::AsyncFileDialog::new().set_title(t("dialog_pick_working_dir"));
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
                    app.status =
                        Some(t("status_tool_title_required").to_string());
                    return Task::none();
                }
                // A tool with no executable is a row that write_tools drops on
                // the way out - the entry looks saved, and is gone next time.
                let blank = state.merged[..state.user_len]
                    .iter()
                    .find(|t| t.exe.as_os_str().is_empty())
                    .map(|t| t.title.clone());
                if let Some(title) = blank {
                    app.status = Some(tf("status_tool_no_exe", &[("title", &title)]));
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
                        && !user_tools
                            .iter()
                            .any(|t| t.title.eq_ignore_ascii_case(&d.title))
                    {
                        user_tools.push(d.clone());
                    }
                }
                if let Some(inst) = &app.created {
                    match inst.save_tools(&user_tools) {
                        Ok(()) => {
                            app.executables = None;
                            load_tools(app); // refresh the run-target picker
                            app.status = Some(t("status_tools_saved").to_string());
                        }
                        Err(e) => app.status = Some(tf("status_tools_save_failed", &[("error", &e)])),
                    }
                }
            }
        }
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
            let (Some(state), Some(inst), Some(game)) = (
                app.executables.as_ref(),
                app.created.as_ref(),
                selected_game(app),
            ) else {
                return Task::none();
            };
            let Some(tool) = state.selected.and_then(|i| state.merged.get(i)).cloned() else {
                return Task::none();
            };
            match write_desktop_entry(inst, game.def.id, &tool) {
                Ok(path) => {
                    app.status = Some(tf("status_shortcut_written", &[("path", &path.display())]));
                }
                Err(e) => app.error = Some(tf("status_shortcut_failed", &[("error", &e)])),
            }
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
