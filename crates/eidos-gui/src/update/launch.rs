//! Running the game and tools, the LOOT report, changing game and the tool
//! prerequisites.

use super::*;

/// True when the Run list entry `title` starts the game: the script-extender
/// loader, the vanilla launcher or the game binary.
fn picks_the_game(app: &App, title: &str) -> bool {
    let Some(game) = selected_game(app) else {
        return false;
    };
    app.tools
        .iter()
        .find(|t| t.title.eq_ignore_ascii_case(title))
        .is_some_and(|t| game_executables(game).launches_the_game(&t.exe))
}

/// Run in a window no store opened (A28): ask the store that owns the copy to
/// start the game, or say what to set first. Returns the status to show.
fn hand_off(app: &App) -> String {
    use eidos::check::launch_option::{launch_handoff, Handoff};
    let (Some(game), Some(inst)) = (selected_game(app), &app.created) else {
        return t("status_create_or_open_instance").to_string();
    };
    let loader_found = game.def.script_extender.as_ref().is_none_or(|se| {
        eidos_instance::root_executable(
            &game.install_path,
            &inst.root_layers(),
            &inst.root_overwrite_dir(),
            se.loader,
        )
        .is_some()
    });
    let this_exe = std::env::current_exe().unwrap_or_else(|_| "eidos-gui".into());
    let search_path: Vec<std::path::PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let (store, name) = (game.source_name(), game.def.name);
    match launch_handoff(
        game,
        loader_found,
        &this_exe,
        &eidos_games::home(),
        &search_path,
    ) {
        Handoff::Start { store: via, argv } => {
            // shortcut: Flatpak Steam has no steam on PATH, so the spawn fails and the status says so; use flatpak run com.valvesoftware.Steam when a user asks.
            match std::process::Command::new(&argv[0])
                .args(&argv[1..])
                .spawn()
            {
                Ok(_) => {
                    remember_handoff_pick(inst, app.tool_choice.as_deref());
                    tf(
                        "status_handed_to_store",
                        &[("store", &via), ("game", &name)],
                    )
                }
                Err(e) => tf(
                    "status_handoff_failed",
                    &[("store", &via), ("error", &e), ("game", &name)],
                ),
            }
        }
        Handoff::SetSteamOption { line } => {
            tf("status_set_steam_launch_option", &[("line", &line)])
        }
        Handoff::SetHeroicWrapper { exe, args } => tf(
            "status_set_heroic_wrapper",
            &[("game", &name), ("exe", &exe), ("args", &args)],
        ),
        Handoff::SetLegendaryWrapper {
            file,
            app: section,
            wrapper,
        } => tf(
            "status_set_legendary_wrapper",
            &[
                ("game", &name),
                ("app", &section),
                ("file", &file.display()),
                ("wrapper", &wrapper),
            ],
        ),
        Handoff::NoLoader { loader } => tf(
            "status_handoff_no_loader",
            &[("loader", &loader), ("store", &store), ("game", &name)],
        ),
        Handoff::ThroughStore => tf(
            "status_launch_through_store",
            &[("game", &name), ("store", &store)],
        ),
    }
}

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::ToolPicked(choice) => {
            app.tool_choice = (choice != run_game_label()).then_some(choice);
        }
        Message::PlayAnyway => {
            if let Some(p) = app.play_prompt.take() {
                app.play_anyway.push(p.key);
                return handle(app, Message::Run);
            }
        }
        Message::ClosePlayPrompt => app.play_prompt = None,
        Message::RunTool(title) => {
            app.play_prompt = None;
            let previous = app.tool_choice.replace(title);
            let task = handle(app, Message::Run);
            app.tool_choice = previous;
            return task;
        }
        Message::Run => {
            if app.running.is_some() {
                // Already waiting on a launched application (MO2 won't launch a
                // second one while locked); ignore the repeat Run.
                // Unlock only drops the overlay - it deliberately KEEPS the run
                // tracked so the post-exit refresh still happens - so telling the
                // user to unlock was advice that could not work.
                let what = app
                    .running
                    .as_ref()
                    .map(|r| r.title.clone())
                    .unwrap_or_default();
                app.status = Some(tf("status_still_running", &[("title", &what)]));
            } else if let Some(why) = app
                .created
                .as_ref()
                .and_then(|i| i.modlist_checked().1.reason().map(str::to_string))
            {
                // `eidos play` and `eidos tool` both refuse this list (they share
                // `run_through_view`), and the refusal only reached the window as
                // "exited with an error". Said here, before spawning, with where
                // the way out is. The Diagnostics card is cached and nothing
                // watches `mods/`, so resync first, the way a refused stale save
                // does: folders deleted since the last refresh must show there.
                reload_mods(app);
                mod_views_changed(app);
                app.status = Some(tf("status_launch_blocked_modlist", &[("reason", &why)]));
            } else if let Some(title) = app
                .tool_choice
                .clone()
                .filter(|t| !(app.launch_command.is_empty() && picks_the_game(app, t)))
            {
                // A tool: the CLI resolves Proton itself, no Steam command needed.
                // A game entry outside a store-started window takes the hand-off below.
                // The owned arg ends the immutable borrow before `start_run`.
                if let Some(arg) = instance_arg(app) {
                    let cmd = tool_command(&arg, &title, !app.launch_command.is_empty());
                    start_run(app, title, cmd);
                } else {
                    app.status = Some(t("status_create_or_open_instance").to_string());
                }
            } else if let Some(prompt) = play_prompt_for(app) {
                // Only the game or a game entry gets here (a tool run took the branch above).
                app.play_prompt = Some(prompt);
            } else if app.launch_command.is_empty() {
                // No game runs outside its store: the store that owns the copy
                // starts it through its Eidos setting, and when that setting is
                // missing the status says what to set and where.
                app.status = Some(hand_off(app));
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
                let both_active =
                    eidos_gamefeatures::enb_cs_conflict(&game.install_path, &cs_roots);
                // How the child names the instance: the portable folder when one
                // is open, the game id otherwise.
                let inst_arg = instance_arg(app).unwrap_or_else(|| id.to_string());
                // `game`/`inst` are no longer used below; their borrows end here so
                // `start_run` can take `&mut app`.
                let (cmd, se_warning) =
                    play_command(id, &inst_arg, &game.install_path, inst, &app.launch_command);
                // A loader whose runtime DLL for this game version is missing
                // stops at startup; the loader-missing case has its own note.
                let extender_note = eidos_gamefeatures::preflight::extender_facts(
                    game.def,
                    &game.data_path,
                    &game.install_path,
                    &eidos::check::active_mods(inst),
                    &inst.overwrite_dir(),
                )
                .and_then(|f| {
                    eidos::check::extender::extender_findings(&f)
                        .into_iter()
                        .find(|f| f.code == "extender_runtime_missing")
                })
                .map(|f| crate::health::finding_card(&f).title);
                start_run(app, game_name, cmd);
                // Marked here, where the game is the thing launched: its exit
                // re-walks only the Overwrite (see `finish_run`).
                if let Some(r) = app.running.as_mut() {
                    r.game = true;
                }
                // Prepend advisories to whatever status start_run set.
                for note in [
                    se_warning,
                    extender_note,
                    both_active.then(|| t("status_enb_and_cs").to_string()),
                ]
                .into_iter()
                .flatten()
                {
                    if let Some(s) = app.status.take() {
                        app.status = Some(format!("{note} {s}"));
                    }
                }
            } else {
                app.status = Some(t("status_create_or_open_instance").to_string());
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
                app.status = Some(tf("status_unlocked", &[("title", &title)]));
            }
        }
        Message::StopRun => {
            // Two presses: the first arms, the disarm block in `update_inner`
            // cancels it on any other action. SIGTERM, not SIGKILL: the `eidos`
            // child ends its programs itself and then captures and unmounts.
            let Some(r) = app.running.as_mut().filter(|r| r.stoppable) else {
                return Task::none();
            };
            let title = r.title.clone();
            if app.confirm_stop {
                // Once `done` is set the wait thread has reaped the child and
                // its pid may belong to another process.
                if !r.done.load(std::sync::atomic::Ordering::SeqCst) {
                    // SAFETY: kill(2) takes no pointer; the pid is our unreaped child.
                    unsafe { libc::kill(r.pid as libc::pid_t, libc::SIGTERM) };
                }
                r.stopped = true;
                app.confirm_stop = false;
                app.status = Some(tf("status_stopping", &[("title", &title)]));
            } else {
                app.confirm_stop = true;
                app.status = Some(tf("status_stop_armed", &[("title", &title)]));
            }
        }
        Message::RebuildPrefix => {
            // Two presses, like Stop. A tracked run is refused before
            // `prefix_busy` is asked: its Wine processes may not exist yet.
            let Some(compat) = selected_game(app).and_then(|g| g.compatdata.clone()) else {
                return Task::none();
            };
            if let Some(r) = &app.running {
                app.status = Some(tf("status_rebuild_prefix_wait_run", &[("title", &r.title)]));
                return Task::none();
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64);
            let stamp = backup_stamp(now);
            if app.confirm_rebuild_prefix {
                app.confirm_rebuild_prefix = false;
                app.status = Some(
                    match eidos_gamefeatures::move_prefix_aside(&compat, &stamp) {
                        Ok(to) => {
                            app.diag_dirty = true;
                            tf("status_rebuild_prefix_done", &[("to", &to.display())])
                        }
                        Err(e) => tf("status_rebuild_prefix_failed", &[("error", &e)]),
                    },
                );
            } else {
                app.confirm_rebuild_prefix = true;
                let to = format!("{}.eidos-backup-{stamp}", compat.display());
                app.status = Some(tf(
                    "status_rebuild_prefix_confirm",
                    &[("from", &compat.display()), ("to", &to)],
                ));
            }
        }
        Message::CloseLootReport => {
            app.loot_report = None;
        }
        Message::CopyLootReport => {
            // No report open means this arrived from the Ctrl+C shortcut with
            // nothing to copy; leaving the clipboard alone is the right answer.
            let Some(report) = &app.loot_report else {
                return Task::none();
            };
            let text = loot_report_text(report);
            let lines = text.lines().count();
            app.status = Some(tf("status_loot_report_copied", &[("n", &lines)]));
            return iced::clipboard::write(text);
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
            let domain = selected_game(app)
                .map(|g| g.def.nexus_game)
                .filter(|s| !s.is_empty());
            let url = match domain {
                Some(d) => format!("https://www.nexusmods.com/{d}"),
                None => "https://www.nexusmods.com".to_string(),
            };
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            app.status = Some(tf("status_opening", &[("url", &url)]));
        }
        Message::SetupPrereqs => {
            // Never under a running game or tool: winetricks writes the prefix
            // they are running in, and the install would only be refused by the
            // instance lock the run holds anyway.
            if let Some(r) = &app.running {
                app.status = Some(tf("status_prereqs_wait_run", &[("title", &r.title)]));
                return Task::none();
            }
            let arg = instance_arg(app);
            let has_prefix = selected_game(app).and_then(|g| g.prefix()).is_some();
            let log = app.created.as_ref().map(|i| i.root.join("prereqs.log"));
            match (arg, log) {
                // A runtime needs no prefix and no Proton - it is a directory and
                // an environment variable - so a missing prefix must not block it.
                (Some(_), _) if !has_prefix && !any_runtime_pending(app) => {
                    app.status = Some(t("status_prefix_needed_for_prereqs").to_string());
                }
                (Some(arg), Some(log)) => {
                    // Tracked like any run, so Run waits for it and the exit is
                    // reported. start_run_with_log sets the failure status itself.
                    let title = t("prereqs_run_title").to_string();
                    start_run_with_log(app, title, prereqs_command(&arg), Some(log.clone()));
                    if let Some(r) = app.running.as_mut() {
                        // No lock overlay: its text is about a game in the merged
                        // view, and it would hide the status naming the log.
                        r.lock = false;
                        // `eidos prereqs` has no SIGTERM handler: Stop would
                        // kill winetricks mid-write in the prefix.
                        r.stoppable = false;
                        app.status =
                            Some(tf("status_installing_prereqs", &[("log", &log.display())]));
                    }
                }
                _ => app.status = Some(t("status_open_instance_first").to_string()),
            }
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}

/// `YYYYMMDD-HHMMSS` in UTC: the suffix of a prefix backup, which sorts by date.
pub(super) fn backup_stamp(unix_secs: i64) -> String {
    let (y, mo, d, h, mi, s) = eidos_log::civil_from_unix(unix_secs);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}
