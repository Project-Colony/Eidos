//! The Preferences page: one category at a time from the list on its left,
//! each a column of collapsible sections.
//!
//! Drawn with the shared Colony kit (`colony_ui::widgets`): the header, the
//! category list, Appearance and Accessibility come from it whole, and Eidos's
//! own sections use its rows and toggles, so every Colony program's page reads
//! the same. It replaces the content area rather than opening over it, so the
//! program's own chrome stays put while it is open.

use colony_ui::widgets::{
    category_heading, category_list, collapsible_section, dropdown, functional_toggle,
    section_description, setting_row, PreferencesView,
};
use colony_ui::Typography;
use iced::widget::{button, slider, text_input, Column, Row};
use iced::{Alignment, Element, Length};

use crate::preferences::UiPreferences;
use crate::theme::{pal, text, text_muted};
use crate::{t, tf, App, Message, SettingsTab};

/// The cached "show adult content" answer for the signed-in account: `Some(true)`
/// shown, `Some(false)` turned off by the user, `None` not known.
///
/// Read straight from the credential store rather than plumbed through app state,
/// because that store IS what the client consults - a second copy in the UI could
/// disagree with what is actually being withheld.
fn adult_content_state() -> Option<bool> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    eidos_instance::settings::load_nexus_creds().adult_pref(now)
}

/// The Preferences page.
///
/// It **replaces the content area**: not a modal, not a separate window, not a
/// popover, which the Colony convention names as the three counter-examples.
pub(crate) fn preferences_page<'a>(app: &App) -> Element<'a, Message> {
    let typo = Typography::current();
    let kit = PreferencesView::new(
        typo,
        &app.ui_prefs.standard,
        &app.settings_expanded,
        Message::Preference,
        Message::SettingsToggleSection,
    );
    let s = Sections { app, typo };
    let content: Element<'a, Message> = match app.settings_tab {
        SettingsTab::General => kit
            .general()
            .push(s.startup())
            .push(s.language())
            .push(s.tools())
            .push(s.running())
            .into(),
        SettingsTab::Appearance => kit.appearance().into(),
        SettingsTab::Accessibility => kit.accessibility().into(),
        SettingsTab::ModList => s.heading(SettingsTab::ModList).push(s.dragging()).into(),
        SettingsTab::Nexus => s
            .heading(SettingsTab::Nexus)
            .push(s.account())
            .push(s.downloads())
            .push(s.offline())
            .into(),
        SettingsTab::About => s
            .heading(SettingsTab::About)
            .push(s.paths())
            .push(s.shortcuts())
            .push(s.version())
            .into(),
    };

    let selected = SettingsTab::ALL
        .iter()
        .position(|t| *t == app.settings_tab)
        .unwrap_or(0);
    let categories = category_list(
        &typo,
        SettingsTab::ALL.map(SettingsTab::label),
        selected,
        |i| Message::SettingsTabSelected(SettingsTab::ALL[i]),
    );
    colony_ui::widgets::preferences_page(&typo, categories, content, Message::TogglePreferences)
}

/// Eidos's own sections, each a collapsible row of the kit.
struct Sections<'s> {
    app: &'s App,
    typo: Typography,
}

impl Sections<'_> {
    /// An Eidos category's heading and the line saying what it changes.
    fn heading<'a>(&self, tab: SettingsTab) -> Column<'a, Message> {
        category_heading(&self.typo, tab.label(), tab.description())
    }

    /// One collapsible section: its description, then its rows.
    fn section<'a>(
        &self,
        key: &'static str,
        title: &str,
        description: &str,
        rows: impl IntoIterator<Item = Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let body = Column::new()
            .spacing(2)
            .push(section_description(&self.typo, description))
            .extend(rows);
        collapsible_section(
            &self.typo,
            title,
            self.app.settings_expanded.contains(key),
            Message::SettingsToggleSection(key),
            body.into(),
        )
    }

    /// A read-only fact: a fixed-width label and its value.
    fn info<'a>(&self, label: &str, value: String) -> Element<'a, Message> {
        let typo = &self.typo;
        Row::new()
            .spacing(8)
            .padding([2, 4])
            .push(
                text(label.to_string())
                    .size(typo.sz(12))
                    .font(typo.regular)
                    .color(text_muted())
                    .width(Length::Fixed(140.0 * typo.scale)),
            )
            .push(text(value).size(typo.sz(12)).font(typo.regular))
            .into()
    }

    /// A small text line under a section's rows.
    fn note<'a>(&self, line: String) -> Element<'a, Message> {
        text(line)
            .size(self.typo.sz(11))
            .font(self.typo.regular)
            .color(text_muted())
            .into()
    }

    fn startup<'a>(&self) -> Element<'a, Message> {
        let (app, typo) = (self.app, &self.typo);
        let games = std::iter::once((None, t("settings_none").to_string())).chain(
            eidos_games::catalog()
                .iter()
                .map(|g| (Some(g.id.to_string()), g.name.to_string())),
        );
        self.section(
            "startup",
            t("settings_section_startup"),
            t("settings_startup_desc"),
            [
                setting_row(
                    typo,
                    t("settings_default_game"),
                    t("settings_default_game_desc"),
                    dropdown(
                        typo,
                        games,
                        &app.prefs.default_game,
                        Message::DefaultGameChanged,
                    ),
                ),
                functional_toggle(
                    typo,
                    t("settings_remember_window"),
                    t("settings_remember_window_desc"),
                    app.prefs.remember_window,
                    Message::ToggleRememberWindow(!app.prefs.remember_window),
                ),
            ],
        )
    }

    fn language<'a>(&self) -> Element<'a, Message> {
        let typo = &self.typo;
        let choices =
            crate::i18n::LANGUAGES.map(|l| (l, t(&format!("settings_language_{l}")).to_string()));
        let current = crate::i18n::LANGUAGES
            .into_iter()
            .find(|l| *l == self.app.ui_prefs.language)
            .unwrap_or("system");
        self.section(
            "language",
            t("settings_section_language"),
            t("settings_language_desc"),
            [setting_row(
                typo,
                t("settings_language"),
                t("settings_language_choice_desc"),
                dropdown(typo, choices, &current, Message::LanguageChanged),
            )],
        )
    }

    fn tools<'a>(&self) -> Element<'a, Message> {
        let (app, typo) = (self.app, &self.typo);
        let field = Row::new()
            .spacing(6)
            .align_y(Alignment::Center)
            .push(
                text_input("/mnt/Games/Tools", &app.tools_dir_edit)
                    .on_input(Message::ToolsDirChanged)
                    .padding(5)
                    .size(typo.sz(12))
                    .font(typo.regular)
                    .width(Length::Fixed(260.0 * typo.scale)),
            )
            .push(
                button(
                    text(t("settings_browse"))
                        .size(typo.sz(12))
                        .font(typo.regular),
                )
                .padding([4, 10])
                .style(button::secondary)
                .on_press(Message::BrowseToolsDir),
            );
        self.section(
            "tools",
            t("settings_section_tools"),
            t("settings_tools_desc"),
            [setting_row(
                typo,
                t("settings_tools_folder"),
                t("settings_tools_folder_desc"),
                field,
            )],
        )
    }

    fn running<'a>(&self) -> Element<'a, Message> {
        let app = self.app;
        self.section(
            "running",
            t("settings_section_running"),
            t("settings_running_desc"),
            [functional_toggle(
                &self.typo,
                t("settings_lock_gui"),
                t("settings_lock_gui_desc"),
                app.prefs.lock_gui,
                Message::ToggleLockGui(!app.prefs.lock_gui),
            )],
        )
    }

    fn dragging<'a>(&self) -> Element<'a, Message> {
        let typo = &self.typo;
        let speed = self.app.prefs.drag_scroll_speed;
        let control = Row::new()
            .spacing(8)
            .align_y(Alignment::Center)
            .push(
                slider(0.25..=4.0, speed, Message::DragScrollSpeedChanged)
                    .step(0.25_f32)
                    .width(Length::Fixed(160.0)),
            )
            .push(
                text(format!("{speed:.2}x"))
                    .size(typo.sz(12))
                    .font(typo.regular)
                    .width(Length::Fixed(48.0 * typo.scale)),
            );
        self.section(
            "dragging",
            t("settings_section_dragging"),
            t("settings_dragging_desc"),
            [setting_row(
                typo,
                t("settings_drag_speed"),
                t("settings_drag_speed_desc"),
                control,
            )],
        )
    }

    fn account<'a>(&self) -> Element<'a, Message> {
        let (app, typo) = (self.app, &self.typo);
        // Sign-in, and only sign-in. There is no personal-API-key field, on
        // purpose: Nexus requires personal keys absent from a distributed
        // client, not merely unused, so there is nothing here to enter.
        let signed_in = app.nexus_account.is_some();
        let label = if app.nexus_signing_in {
            t("settings_sign_in_waiting")
        } else if signed_in {
            t("settings_sign_in_again")
        } else {
            t("settings_sign_in")
        };
        let mut action = button(text(label).size(typo.sz(12)).font(typo.regular))
            .padding([5, 12])
            .style(button::primary);
        if !app.nexus_signing_in {
            action = action.on_press(Message::NexusSignInStart);
        }
        let mut controls = Row::new().spacing(8).push(action);
        if signed_in {
            controls = controls.push(
                button(
                    text(t("settings_sign_out"))
                        .size(typo.sz(12))
                        .font(typo.regular),
                )
                .padding([5, 12])
                .on_press(Message::NexusSignOut)
                .style(button::secondary),
            );
        }

        let mut rows: Vec<Element<'a, Message>> = vec![controls.into()];
        match &app.nexus_account {
            Some(a) => {
                let tier = t(if a.is_premium {
                    "settings_tier_premium"
                } else {
                    "settings_tier_free"
                });
                rows.push(self.note(tf(
                    "settings_signed_in_as",
                    &[("name", &a.name), ("tier", &tier)],
                )));
                // Say what is being withheld and why. Adult mods coming back
                // blank with no explanation reads as Eidos being broken, and
                // "could not check" is the case the user can actually act on.
                rows.push(
                    self.note(
                        t(match adult_content_state() {
                            Some(true) => "settings_adult_shown",
                            Some(false) => "settings_adult_off",
                            None => "settings_adult_unknown",
                        })
                        .to_string(),
                    ),
                );
            }
            None => rows.push(self.note(t("settings_not_signed_in").to_string())),
        }
        if let Some(err) = &app.nexus_error {
            rows.push(
                text(tf("settings_error", &[("error", err)]))
                    .size(typo.sz(12))
                    .font(typo.regular)
                    .color(pal().error)
                    .into(),
            );
        }
        self.section(
            "account",
            t("settings_section_account"),
            t("settings_account_desc"),
            rows,
        )
    }

    fn downloads<'a>(&self) -> Element<'a, Message> {
        let (app, typo) = (self.app, &self.typo);
        let servers = text_input("Nexus CDN, Paris, Chicago", &app.servers_edit)
            .on_input(Message::PreferredServersChanged)
            .padding(5)
            .size(typo.sz(12))
            .font(typo.regular)
            .width(Length::Fixed(260.0 * typo.scale));
        self.section(
            "downloads",
            t("settings_section_downloads"),
            t("settings_downloads_desc"),
            [
                self.info(
                    t("settings_downloads_folder"),
                    app.created
                        .as_ref()
                        .map(|i| i.downloads_dir().display().to_string())
                        .unwrap_or_else(|| t("settings_no_instance_open").to_string()),
                ),
                self.note(t("settings_nxm_note").to_string()),
                setting_row(
                    typo,
                    t("settings_preferred_servers"),
                    t("settings_preferred_servers_desc"),
                    servers,
                ),
            ],
        )
    }

    fn offline<'a>(&self) -> Element<'a, Message> {
        let app = self.app;
        self.section(
            "offline",
            t("settings_section_offline"),
            t("settings_offline_section_desc"),
            [functional_toggle(
                &self.typo,
                t("settings_offline"),
                t("settings_offline_desc"),
                app.prefs.offline,
                Message::ToggleOffline(!app.prefs.offline),
            )],
        )
    }

    fn paths<'a>(&self) -> Element<'a, Message> {
        let app = self.app;
        self.section(
            "paths",
            t("settings_section_paths"),
            t("settings_paths_desc"),
            [
                self.info(
                    t("settings_path_instance"),
                    app.created
                        .as_ref()
                        .map(|i| i.root.display().to_string())
                        .unwrap_or_else(|| t("settings_none_open").to_string()),
                ),
                self.info(
                    t("settings_path_preferences"),
                    UiPreferences::default_path().display().to_string(),
                ),
                self.info(
                    t("settings_path_shared"),
                    eidos_instance::settings::settings_path()
                        .display()
                        .to_string(),
                ),
                self.info(
                    t("settings_path_nexus"),
                    eidos_instance::settings::nexus_key_path()
                        .display()
                        .to_string(),
                ),
                self.info(
                    t("settings_path_games"),
                    eidos_paths::config_dir()
                        .join("games/*.toml")
                        .display()
                        .to_string(),
                ),
            ],
        )
    }

    fn shortcuts<'a>(&self) -> Element<'a, Message> {
        self.section(
            "shortcuts",
            t("settings_section_shortcuts"),
            t("settings_shortcuts_desc"),
            [
                self.info(t("settings_key_run"), "Ctrl+R".to_string()),
                self.info(t("settings_key_refresh"), "F5".to_string()),
                self.info(
                    t("settings_key_select"),
                    t("settings_key_select_how").to_string(),
                ),
                self.info(t("settings_key_select_all"), "Ctrl+A".to_string()),
                self.info(t("settings_key_clear"), t("settings_key_esc").to_string()),
                self.info(
                    t("settings_key_reorder"),
                    t("settings_key_reorder_how").to_string(),
                ),
            ],
        )
    }

    fn version<'a>(&self) -> Element<'a, Message> {
        self.section(
            "version",
            t("settings_section_version"),
            t("settings_version_desc"),
            [self.info("Eidos", env!("CARGO_PKG_VERSION").to_string())],
        )
    }
}
