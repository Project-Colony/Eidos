//! The Preferences page and the settings it writes.

use super::*;

/// The `update` arms for this group. [`update_inner`] routes only these
/// messages here.
pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ---- Settings / Preferences ------------------------------------------
        Message::TogglePreferences => {
            app.settings_open = !app.settings_open;
            app.nexus_error = None;
            app.toast = None;
            if app.settings_open {
                app.menu_mod = None;
                // Every visit starts on General with every section shut.
                app.settings_tab = SettingsTab::General;
                app.settings_expanded.clear();
            }
        }
        Message::SettingsTabSelected(t) => app.settings_tab = t,
        Message::Preference(change) => {
            app.ui_prefs.standard.update(change);
            if matches!(change, colony_ui::preferences::Change::Theme { .. }) {
                app.toast = Some(colony_ui::i18n::t("theme_applied"));
            }
            if let Err(e) = app.ui_prefs.save() {
                app.status = Some(tf("status_prefs_save_failed", &[("error", &e)]));
            }
        }
        Message::DismissToast => app.toast = None,
        Message::LanguageChanged(language) => {
            app.ui_prefs.language = language.to_string();
            crate::i18n::apply(language);
            if let Err(e) = app.ui_prefs.save() {
                app.status = Some(tf("status_prefs_save_failed", &[("error", &e)]));
            }
        }
        Message::NexusSignInStart => {
            if app.nexus_signing_in {
                return Task::none();
            }
            // Signing in opens a BROWSER and talks to Nexus, so offline mode has
            // to stop it here: `connect` is not on this path at all, and the
            // guard there does not cover the one action that begins by leaving
            // the program.
            if app.prefs.offline {
                app.nexus_error = Some(t("nexus_offline").to_string());
                return Task::none();
            }
            app.nexus_signing_in = true;
            app.nexus_error = None;
            app.status = Some(t("status_nexus_opening_browser").to_string());
            // The whole dance on smol's blocking pool: browser hand-off,
            // loopback listener, code exchange. An `async` block would NOT be
            // a worker: iced polls it on smol's single executor thread, and the
            // listener waits up to five minutes for the browser, every timer
            // and input subscription frozen behind it.
            return Task::perform(smol::unblock(nexus_sign_in), Message::NexusSignInResult);
        }
        Message::NexusSignInResult(result) => {
            app.nexus_signing_in = false;
            match result {
                Ok(account) => {
                    app.status = Some(tf(
                        "status_nexus_signed_in",
                        &[
                            ("name", &account.name),
                            (
                                "tier",
                                &if account.is_premium {
                                    t("settings_tier_premium")
                                } else {
                                    t("settings_tier_free")
                                },
                            ),
                        ],
                    ));
                    app.nexus_account = Some(account);
                }
                Err(e) => app.nexus_error = Some(e),
            }
        }
        Message::NexusSignOut => match eidos_instance::settings::clear_nexus_tokens() {
            Ok(()) => {
                app.nexus_account = None;
                app.status = Some(t("status_nexus_signed_out").to_string());
            }
            Err(e) => app.nexus_error = Some(tf("status_nexus_sign_out_failed", &[("error", &e)])),
        },
        Message::DragScrollSpeedChanged(v) => {
            app.prefs.drag_scroll_speed = v.clamp(0.25, 4.0);
            if let Err(e) = app.prefs.save() {
                app.status = Some(tf("status_prefs_save_failed", &[("error", &e)]));
            }
        }
        Message::SettingsToggleSection(key) => {
            if !app.settings_expanded.remove(key) {
                app.settings_expanded.insert(key.to_string());
            }
        }
        Message::ToggleRememberWindow(on) => {
            app.prefs.remember_window = on;
            if let Err(e) = app.prefs.save() {
                app.status = Some(tf("status_prefs_save_failed", &[("error", &e)]));
            }
        }
        Message::ToggleOffline(on) => {
            app.prefs.offline = on;
            if let Err(e) = app.prefs.save() {
                app.error = Some(tf("status_settings_save_failed", &[("error", &e)]));
            }
            // The status bar's account line is now wrong either way: offline
            // means Eidos will not check, and coming back online means it can.
            app.nexus_error = on.then(|| t("nexus_offline").to_string());
        }
        Message::PreferredServersChanged(t) => {
            app.typing = true;
            // Stored as typed, so there is nothing to save by hand. The field
            // keeps the text as typed: echoing the parsed list back mid-word
            // would eat the comma the user is about to follow with a name.
            app.prefs.preferred_servers = t
                .split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .collect();
            app.servers_edit = t;
            if let Err(e) = app.prefs.save() {
                app.error = Some(tf("status_settings_save_failed", &[("error", &e)]));
            }
        }
        Message::DefaultGameChanged(g) => {
            app.prefs.default_game = g;
            if let Err(e) = app.prefs.save() {
                app.status = Some(tf("status_prefs_save_failed", &[("error", &e)]));
            }
        }
        Message::ToggleLockGui(v) => {
            app.prefs.lock_gui = v;
            if let Err(e) = app.prefs.save() {
                app.status = Some(tf("status_prefs_save_failed", &[("error", &e)]));
            }
        }
        Message::ToolsDirChanged(t) => {
            app.typing = true;
            let dir = t.trim().to_string();
            // An empty box clears the setting rather than storing "": there is
            // no such directory, and a stored empty string would be a path that
            // fails a `is_dir` check on every tool list build for ever.
            app.prefs.tools_dir = Some(dir).filter(|d| !d.is_empty());
            app.tools_dir_edit = t;
            if let Err(e) = app.prefs.save() {
                app.error = Some(tf("status_settings_save_failed", &[("error", &e)]));
            }
            // The list is built from this, so it is rebuilt as soon as the box
            // names a folder (or nothing), not on every half-typed path.
            if app
                .prefs
                .tools_dir
                .as_deref()
                .is_none_or(|d| std::path::Path::new(d).is_dir())
            {
                load_tools(app);
            }
        }
        Message::BrowseToolsDir => {
            let mut dlg = rfd::AsyncFileDialog::new().set_title(t("dialog_pick_tools_dir"));
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
