//! The per-mod information dialog: opening it, its tabs, notes and the Nexus
//! URL.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::ShowModInfo(i) => {
            app.menu_mod = None;
            let seeded = match (app.created.as_ref(), app.mods.get(i)) {
                (Some(inst), Some(m)) => {
                    let meta = inst.mod_meta(&m.name);
                    Some((
                        meta.notes().unwrap_or_default(),
                        meta.url().unwrap_or_default(),
                    ))
                }
                _ => None,
            };
            if let Some((notes, url)) = seeded {
                app.notes_edit = notes;
                app.url_edit = url;
                app.info_provenance = app
                    .mods
                    .get(i)
                    .map(|m| generated_summary(app, &m.name))
                    .unwrap_or_default();
                app.info_mod = Some(i);
                app.info_tab = InfoTab::General;
            }
        }
        Message::CloseInfo => app.info_mod = None,
        Message::InfoSelectTab(t) => {
            if app.info_tab != t {
                app.info_prev = Some(app.info_tab);
                app.info_anim.start();
            }
            app.info_tab = t;
        }
        Message::NotesChanged(s) => {
            app.typing = true;
            app.notes_edit = s;
        }
        Message::ModUrlChanged(s) => {
            app.typing = true;
            app.url_edit = s;
        }
        Message::ModUrlSave => {
            // The game's own content has no folder under mods/, so there is
            // nowhere to put a meta.ini - and inventing one would plant an empty
            // directory the next reconcile lists as a real mod.
            if app
                .info_mod
                .and_then(|i| app.mods.get(i))
                .is_some_and(|m| m.is_unmanaged())
            {
                app.status = Some(t("status_game_content_no_meta").to_string());
                return Task::none();
            }
            let typed = app.url_edit.trim().to_string();
            // Refuse anything that is not a web link, HERE rather than at the
            // click that opens it: a value that cannot be opened must not be
            // storable in the first place, or the button becomes a dead end that
            // looks live.
            let is_web_link = typed.starts_with("http://") || typed.starts_with("https://");
            if !typed.is_empty() && !is_web_link {
                app.status = Some(t("status_page_must_be_http").to_string());
                return Task::none();
            }
            let result = match (app.info_mod, app.created.as_ref()) {
                (Some(i), Some(inst)) => app.mods.get(i).map(|m| {
                    (
                        m.name.clone(),
                        edit_mod_meta(inst, &m.name, |meta| meta.set_url(&typed)),
                    )
                }),
                _ => None,
            };
            if let Some((name, r)) = result {
                app.status = Some(match r {
                    Ok(()) if typed.is_empty() => tf("status_page_cleared", &[("name", &name)]),
                    Ok(()) => tf("status_page_saved", &[("name", &name)]),
                    Err(e) => tf("status_page_save_failed", &[("error", &e)]),
                });
            }
        }
        Message::NotesSave => {
            if app
                .info_mod
                .and_then(|i| app.mods.get(i))
                .is_some_and(|m| m.is_unmanaged())
            {
                app.status = Some(t("status_game_content_no_meta").to_string());
                return Task::none();
            }
            let result = match (app.info_mod, app.created.as_ref()) {
                (Some(i), Some(inst)) => app.mods.get(i).map(|m| {
                    (
                        m.name.clone(),
                        edit_mod_meta(inst, &m.name, |meta| meta.set_notes(&app.notes_edit)),
                    )
                }),
                _ => None,
            };
            if let Some((name, r)) = result {
                // The row caches the note text for its glyph, so the write has to
                // drop the cached copy or the list keeps showing the old one -
                // and shows no glyph at all the first time a note is added.
                invalidate_meta(app, &name);
                refresh_meta_cache(app);
                app.status = Some(match r {
                    Ok(()) => tf("status_notes_saved", &[("name", &name)]),
                    Err(e) => tf("status_notes_save_failed", &[("error", &e)]),
                });
            }
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
