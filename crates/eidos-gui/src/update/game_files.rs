//! The Game files card's actions: run a copy kept in the instance, or the
//! store install again after an arming press.

use super::*;

/// The `update` arm for `Message::GameFiles`.
pub(super) fn handle(app: &mut App, action: GameFilesAction) -> Task<Message> {
    // Any press ends an armed one; only a first Return press arms again.
    let armed = app.confirm_game_files.take();
    if let Some(r) = &app.running {
        app.status = Some(tf("status_game_files_wait_run", &[("title", &r.title)]));
        return Task::none();
    }
    let (Some(game), Some(inst)) = (selected_game(app).cloned(), app.created.clone()) else {
        return Task::none();
    };
    let failed = |e: &str| tf("status_game_files_failed", &[("error", &e)]);
    let status = match action {
        GameFilesAction::Use(folder) => {
            match eidos::stock_game::set_base(&game, &inst, Some(&folder)) {
                Ok(()) => {
                    switched(app);
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
                        switched(app);
                        tf("status_game_files_use_store", &[("folder", &folder)])
                    }
                    Err(e) => failed(&e),
                }
            }
        }
        // shortcut: freeze and delete are not wired yet, Task 112 runs them.
        GameFilesAction::Freeze | GameFilesAction::Delete => failed("not available yet"),
    };
    app.status = Some(status);
    Task::none()
}

/// After a switch: the card, the Data stack, the plugins and the conflicts all
/// read the game files, so each is rebuilt from the new ones.
fn switched(app: &mut App) {
    reload_mods(app);
    mod_views_changed(app);
    app.diag_dirty = true;
}
