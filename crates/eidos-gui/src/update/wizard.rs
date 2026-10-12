//! The first-run wizard: choosing a kind, a game and a place, and opening a
//! known instance.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
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
                                let selection =
                                    selected_game(app).expect("selected game").selection_id();
                                let saved =
                                    inst.try_lock("selecting a game installation").and_then(
                                        |_lock| inst.ensure_installation(id, kind, &selection),
                                    );
                                if let Err(e) = saved {
                                    app.error =
                                        Some(format!("Could not save the instance identity: {e}"));
                                    return Task::none();
                                }
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
                let current = app
                    .games
                    .get(k.game_index)
                    .and_then(|g| instance_game_index(&app.games, &k.inst, g.def.id));
                if let (true, Some(index)) = (k.inst.exists(), current) {
                    let id = app.games[index].def.id;
                    if let Err(error) = k
                        .inst
                        .ensure_manifest(id, InstanceKind::Global)
                        .and_then(|_| k.inst.ensure_profiles())
                    {
                        app.status = Some(format!("Cannot open this instance: {error}"));
                        return Task::none();
                    }
                    app.selected = Some(index);
                    remember_open(&k.inst, id);
                    open_instance(app, k.inst);
                } else {
                    app.status = Some(format!(
                        "'{}' is not reachable right now or its selected installation changed.",
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
            app.installer = None;
            app.fomod = None;
            app.screen = Screen::Welcome;
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
