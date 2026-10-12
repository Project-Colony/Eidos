//! Running the game and tools, the LOOT report, changing game and the tool
//! prerequisites.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
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
                let what = app
                    .running
                    .as_ref()
                    .map(|r| r.title.clone())
                    .unwrap_or_default();
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
            } else if selected_game(app).is_some_and(|g| !g.is_steam())
                && app.launch_command.is_empty()
            {
                app.status = Some("This installation needs a manual launch command. Add a native tool with the correct Heroic/Legendary runner and prefix, or use eidos play <instance> -- <runner> <game>. Eidos does not infer a Steam command for this copy.".into());
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
                let both_active =
                    eidos_gamefeatures::enb_cs_conflict(&game.install_path, &cs_roots);
                // How the child names the instance: the portable folder when one
                // is open, the game id otherwise.
                let inst_arg = instance_arg(app).unwrap_or_else(|| id.to_string());
                // `game`/`inst` are no longer used below; their borrows end here so
                // `start_run` can take `&mut app`.
                let (cmd, se_warning) =
                    play_command(id, &inst_arg, &game.install_path, inst, &app.launch_command);
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
            let Some(report) = &app.loot_report else {
                return Task::none();
            };
            let text = loot_report_text(report);
            let lines = text.lines().count();
            app.status = Some(format!("LOOT report copied ({lines} lines)."));
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
            app.status = Some(format!("Opening {url}"));
        }
        Message::SetupPrereqs => {
            let arg = instance_arg(app);
            let has_prefix = selected_game(app).and_then(|g| g.prefix()).is_some();
            let log = app.created.as_ref().map(|i| i.root.join("prereqs.log"));
            match (arg, log) {
                // A runtime needs no prefix and no Proton - it is a directory and
                // an environment variable - so a missing prefix must not block it.
                (Some(_), _) if !has_prefix && !any_runtime_pending(app) => {
                    app.status = Some(
                        "Set up the selected installation’s Wine prefix before installing prefix prerequisites."
                            .to_string(),
                    );
                }
                (Some(arg), Some(log)) => match run_prereqs_setup(&arg, &log) {
                    Ok(()) => {
                        app.status = Some(format!(
                            "Installing prerequisites: DirectX DLLs, .NET and vcrun download through winetricks. Progress + errors -> {}",
                            log.display()
                        ));
                    }
                    Err(e) => app.status = Some(format!("Could not start prereq setup: {e}")),
                },
                _ => app.status = Some("Open a game instance first.".to_string()),
            }
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
