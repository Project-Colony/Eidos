//! The Preferences page and the settings it writes.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ---- Settings / Preferences ------------------------------------------
        Message::OpenSettings => {
            app.menu_mod = None;
            app.nexus_error = None;
            app.settings_open = true;
        }
        Message::CloseSettings => {
            app.settings_open = false;
            app.nexus_error = None;
        }
        Message::SettingsTabSelected(t) => app.settings_tab = t,
        Message::NexusSignInStart => {
            if app.nexus_signing_in {
                return Task::none();
            }
            // Signing in opens a BROWSER and talks to Nexus, so offline mode has
            // to stop it here: `connect` is not on this path at all, and the
            // guard there does not cover the one action that begins by leaving
            // the program.
            if app.prefs.offline {
                app.nexus_error = Some(eidos_nexus::OFFLINE_MESSAGE.to_string());
                return Task::none();
            }
            app.nexus_signing_in = true;
            app.nexus_error = None;
            app.status = Some("Opening your browser to sign in to Nexus...".to_string());
            // The whole dance on a worker: browser hand-off, loopback listener,
            // code exchange. Blocking calls inside the async closure, like the
            // other network work here.
            return Task::perform(async move { nexus_sign_in() }, Message::NexusSignInResult);
        }
        Message::NexusSignInResult(result) => {
            app.nexus_signing_in = false;
            match result {
                Ok(account) => {
                    app.status = Some(format!(
                        "Signed in to Nexus as {} ({}).",
                        account.name,
                        if account.is_premium {
                            "Premium"
                        } else {
                            "free"
                        }
                    ));
                    app.nexus_account = Some(account);
                }
                Err(e) => app.nexus_error = Some(e),
            }
        }
        Message::NexusSignOut => match eidos_instance::settings::clear_nexus_tokens() {
            Ok(()) => {
                app.nexus_account = None;
                app.status = Some("Signed out of Nexus.".to_string());
            }
            Err(e) => app.nexus_error = Some(format!("could not sign out: {e}")),
        },
        Message::DragScrollSpeedChanged(v) => {
            app.prefs.drag_scroll_speed = v.clamp(0.25, 4.0);
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::SettingsToggleSection(key) => {
            if !app.settings_expanded.remove(key) {
                app.settings_expanded.insert(key);
            }
        }
        Message::ToggleRememberWindow(on) => {
            app.prefs.remember_window = on;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::ToggleMotion(on) => {
            app.prefs.motion = on;
            // The live copy too, not only the saved one: every animated value is
            // read through `anim::at`, which asks `app.motion`. Saving alone
            // would leave the window animating until the next launch.
            app.motion = on;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::ToggleOffline(on) => {
            app.prefs.offline = on;
            if let Err(e) = app.prefs.save() {
                app.error = Some(format!("Could not save settings: {e}"));
            }
            // The status bar's account line is now wrong either way: offline
            // means Eidos will not check, and coming back online means it can.
            app.nexus_error = on.then(|| eidos_nexus::OFFLINE_MESSAGE.to_string());
        }
        Message::PreferredServersChanged(t) => {
            app.typing = true;
            app.servers_edit = t;
        }
        Message::PreferredServersSave => {
            app.prefs.preferred_servers = app
                .servers_edit
                .split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .collect();
            if let Err(e) = app.prefs.save() {
                app.error = Some(format!("Could not save settings: {e}"));
            }
            // Echo back what was actually stored, so a trailing comma or a
            // double space does not survive in the field looking meaningful.
            app.servers_edit = app.prefs.preferred_servers.join(", ");
        }
        Message::ThemeChanged(family, variant) => {
            app.prefs.theme_family = family;
            app.prefs.theme_variant = variant;
            repaint(app);
        }
        Message::AccentChanged(key) => {
            app.prefs.accent = key;
            repaint(app);
        }
        Message::ToggleHighContrast(on) => {
            app.prefs.high_contrast = on;
            repaint(app);
        }
        Message::DefaultGameChanged(g) => {
            app.prefs.default_game = g;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::ToggleLockGui(v) => {
            app.prefs.lock_gui = v;
            if let Err(e) = app.prefs.save() {
                app.status = Some(format!("Could not save preferences: {e}"));
            }
        }
        Message::ToolsDirChanged(t) => {
            app.typing = true;
            app.tools_dir_edit = t;
        }
        Message::ToolsDirSave => {
            let dir = app.tools_dir_edit.trim().to_string();
            // An empty box clears the setting rather than storing "": there is
            // no such directory, and a stored empty string would be a path that
            // fails a `is_dir` check on every tool list build for ever.
            app.prefs.tools_dir = Some(dir).filter(|d| !d.is_empty());
            if let Err(e) = app.prefs.save() {
                app.error = Some(format!("Could not save settings: {e}"));
            }
            app.tools_dir_edit = app.prefs.tools_dir.clone().unwrap_or_default();
            // The list is built from this, so it has to be rebuilt now rather
            // than at some later refresh the user has to guess at.
            load_tools(app);
        }
        Message::BrowseToolsDir => {
            let mut dlg = rfd::AsyncFileDialog::new().set_title("Where your modding tools live");
            if let Some(cur) = app.prefs.tools_dir.as_deref().filter(|d| !d.is_empty()) {
                dlg = dlg.set_directory(cur);
            }
            return Task::perform(dlg.pick_folder(), |h| match h {
                Some(h) => Message::ToolsDirChanged(h.path().display().to_string()),
                None => Message::Noop,
            });
        }
        _ => unreachable!("routed by update_inner"),
    }
    Task::none()
}
