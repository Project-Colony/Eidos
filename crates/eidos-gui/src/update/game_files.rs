//! The Game files card's actions: run a copy kept in the instance, or the
//! store install again after an arming press; freeze the install into a copy
//! and delete that copy, each after an arming press.

use super::*;
use eidos::stock_game::{CopyKind, FreezePlan};
use eidos_transfer::human_bytes;

/// `eidos stock-game <inst> freeze --yes`: copy the install into the instance.
pub(crate) fn freeze_command(inst_arg: &str) -> std::process::Command {
    let mut cmd = eidos_command();
    cmd.args(["stock-game", inst_arg, "freeze", "--yes"]);
    cmd
}

/// `eidos stock-game <inst> delete --yes`: delete the copy a freeze made.
pub(crate) fn delete_copy_command(inst_arg: &str) -> std::process::Command {
    let mut cmd = eidos_command();
    cmd.args(["stock-game", inst_arg, "delete", "--yes"]);
    cmd
}

/// The first Freeze press's status: what the copy takes and how, or why it
/// does not fit.
pub(crate) fn freeze_status(p: &FreezePlan) -> Result<String, String> {
    let size = human_bytes(p.bytes);
    let free = p
        .free
        .map_or_else(|| t("freeze_free_unknown").to_string(), human_bytes);
    if !p.fits() {
        return Err(tf(
            "status_freeze_no_space",
            &[("size", &size), ("free", &free)],
        ));
    }
    let fs = p.fs.unwrap_or_else(|| t("freeze_fs_unknown"));
    let room: &[(&str, &dyn std::fmt::Display)] = &[("fs", &fs), ("size", &size), ("free", &free)];
    let how = match p.kind {
        CopyKind::Reflink => tf("freeze_how_reflink", room),
        CopyKind::Full => tf("freeze_how_full", room),
        CopyKind::OtherFilesystem => tf("freeze_how_other_fs", room),
    };
    Ok(tf(
        "status_freeze_confirm",
        &[
            ("size", &size),
            ("files", &p.files),
            ("from", &p.from.display()),
            ("to", &p.to.display()),
            ("how", &how),
        ],
    ))
}

/// The `update` arm for `Message::GameFiles`.
pub(super) fn handle(app: &mut App, action: GameFilesAction) -> Task<Message> {
    // Any press ends an armed one; only a first Return press arms again.
    let armed = app.confirm_game_files.take();
    if let Some(r) = &app.running {
        app.status = Some(tf("status_game_files_wait_run", &[("title", &r.title)]));
        return Task::none();
    }
    let (Some(game), Some(inst), Some(arg)) = (
        selected_game(app).cloned(),
        app.created.clone(),
        instance_arg(app),
    ) else {
        return Task::none();
    };
    let failed = |e: &str| tf("status_game_files_failed", &[("error", &e)]);
    let status = match action {
        GameFilesAction::Use(folder) => {
            match eidos::stock_game::set_base(&game, &inst, Some(&folder)) {
                Ok(()) => {
                    switched(app, game.def.id);
                    tf("status_game_files_use_copy", &[("folder", &folder)])
                }
                Err(e) => failed(&e),
            }
        }
        GameFilesAction::Store => {
            let Some(folder) = eidos::stock_game::game_files(&game, &inst).copy else {
                return Task::none();
            };
            if armed != Some(GameFilesAction::Store) {
                app.confirm_game_files = Some(GameFilesAction::Store);
                let version =
                    (app.game_files_facts.as_ref()).and_then(|f| f.store_version.as_deref());
                tf(
                    "status_game_files_store_confirm",
                    &[
                        ("version", &game_files_version(version)),
                        ("folder", &folder),
                    ],
                )
            } else {
                match eidos::stock_game::set_base(&game, &inst, None) {
                    Ok(()) => {
                        switched(app, game.def.id);
                        tf("status_game_files_use_store", &[("folder", &folder)])
                    }
                    Err(e) => failed(&e),
                }
            }
        }
        // Each press measures again, so the copy that starts is the one shown
        // and a disk that filled up meanwhile is caught before the run.
        GameFilesAction::Freeze => {
            // shortcut: the plan walks the install on the window thread, move it to background_work::run if a slow disk makes the press lag
            let plan = match eidos::stock_game::freeze_plan(&game.install_path, &inst) {
                Ok(plan) => plan,
                Err(e) => {
                    app.status = Some(tf("status_freeze_plan_failed", &[("error", &e)]));
                    return Task::none();
                }
            };
            match freeze_status(&plan) {
                Err(no_space) => no_space,
                Ok(confirm) if armed != Some(GameFilesAction::Freeze) => {
                    app.confirm_game_files = Some(GameFilesAction::Freeze);
                    confirm
                }
                Ok(_) => {
                    // Stoppable and locked like any run: the CLI stops between
                    // files on SIGTERM and keeps the unfinished folder.
                    start_run(app, t("freeze_run_title").into(), freeze_command(&arg));
                    if app.running.is_none() {
                        return Task::none(); // start_run named the failure
                    }
                    tf(
                        "status_freezing",
                        &[
                            ("size", &human_bytes(plan.bytes)),
                            ("to", &plan.to.display()),
                        ],
                    )
                }
            }
        }
        GameFilesAction::Delete => {
            let f = eidos::stock_game::facts(&game, &inst);
            let Some(path) = (f.frozen.map(|d| inst.root.join(d))).or(f
                .unfinished
                .then(|| eidos::stock_game::unfinished_path(&inst)))
            else {
                return Task::none();
            };
            if armed != Some(GameFilesAction::Delete) {
                app.confirm_game_files = Some(GameFilesAction::Delete);
                let size = eidos::stock_game::tree_size(&path).map_or_else(
                    |_| t("freeze_free_unknown").to_string(),
                    |(bytes, _)| human_bytes(bytes),
                );
                tf(
                    "status_delete_copy_confirm",
                    &[("path", &path.display()), ("size", &size)],
                )
            } else {
                start_run(
                    app,
                    t("delete_copy_run_title").into(),
                    delete_copy_command(&arg),
                );
                if let Some(r) = app.running.as_mut() {
                    // Stopping between two removals would leave half a folder.
                    r.stoppable = false;
                }
                return Task::none(); // start_run's status names the run or its failure
            }
        }
    };
    app.status = Some(status);
    Task::none()
}

/// After a switch: the card, the Data stack, the plugins and the conflicts all
/// read the game files, so each is rebuilt from the new ones.
fn switched(app: &mut App, game_id: &str) {
    // The game layer keeps its name whichever root it reads, so its cached
    // walk would outlive the switch.
    drop_files_cache(app, Some(&format!("[{game_id}]")));
    reload_mods(app);
    mod_views_changed(app);
    app.diag_dirty = true;
}
