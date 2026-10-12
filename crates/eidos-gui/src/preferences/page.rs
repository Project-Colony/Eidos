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
use crate::{App, Message, SettingsTab};

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
        let games = std::iter::once((None, "(none)".to_string())).chain(
            eidos_games::catalog().iter().map(|g| (Some(g.id.to_string()), g.name.to_string())),
        );
        self.section(
            "startup",
            "Startup",
            "What Eidos opens, and at what size.",
            [
                setting_row(
                    typo,
                    "Default game",
                    "Opened when Eidos starts without being told which.",
                    dropdown(typo, games, &app.prefs.default_game, Message::DefaultGameChanged),
                ),
                functional_toggle(
                    typo,
                    "Remember the window size",
                    "Restore the last size on launch instead of letting the compositor choose.",
                    app.prefs.remember_window,
                    Message::ToggleRememberWindow(!app.prefs.remember_window),
                ),
            ],
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
                button(text("Browse").size(typo.sz(12)).font(typo.regular))
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press(Message::BrowseToolsDir),
            );
        self.section(
            "tools",
            "Tools",
            "Where Eidos looks for xEdit, DynDOLOD and the like.",
            [setting_row(
                typo,
                "Tools folder",
                "Eidos already looks in the game folder and inside this instance's mods; this \
                 is for the directory shared between instances. A tool found here is offered \
                 with the runtimes its name calls for.",
                field,
            )],
        )
    }

    fn running<'a>(&self) -> Element<'a, Message> {
        let app = self.app;
        self.section(
            "running",
            "Running a game",
            "What the window does while a game or a tool runs.",
            [functional_toggle(
                &self.typo,
                "Lock the window while a game runs",
                "Blocks the main window behind an overlay until the game exits, with an Unlock \
                 escape hatch.",
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
            "Dragging",
            "How the list moves while a mod is dragged.",
            [setting_row(
                typo,
                "Auto-scroll speed",
                "How fast the list moves when a dragged mod rests on its top or bottom edge. \
                 Speed already rises the closer to the edge; this scales the whole range.",
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
            "Waiting for your browser..."
        } else if signed_in {
            "Sign in again"
        } else {
            "Sign in to Nexus Mods"
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
                button(text("Sign out").size(typo.sz(12)).font(typo.regular))
                    .padding([5, 12])
                    .on_press(Message::NexusSignOut)
                    .style(button::secondary),
            );
        }

        let mut rows: Vec<Element<'a, Message>> = vec![controls.into()];
        match &app.nexus_account {
            Some(a) => {
                let tier = if a.is_premium { "Premium" } else { "free" };
                rows.push(self.note(format!("Signed in as {} ({tier}).", a.name)));
                // Say what is being withheld and why. Adult mods coming back
                // blank with no explanation reads as Eidos being broken, and
                // "could not check" is the case the user can actually act on.
                rows.push(self.note(
                    match adult_content_state() {
                        Some(true) => "Adult content: shown (enabled on your Nexus account).",
                        Some(false) => {
                            "Adult content: hidden. It is turned off on your Nexus account; \
                             change it on nexusmods.com, then sign in again here."
                        }
                        None => {
                            "Adult content: hidden. Eidos could not read your Nexus content \
                             settings, so it withholds adult mods until it can."
                        }
                    }
                    .to_string(),
                ));
            }
            None => rows.push(self.note("Not signed in.".to_string())),
        }
        if let Some(err) = &app.nexus_error {
            rows.push(
                text(format!("Error: {err}"))
                    .size(typo.sz(12))
                    .font(typo.regular)
                    .color(pal().error)
                    .into(),
            );
        }
        self.section(
            "account",
            "Account",
            "Signing in opens your browser. The session is stored in nexus.ini and shared with \
             the CLI.",
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
            "Downloads",
            "Where downloads land and which servers they come from.",
            [
                self.info(
                    "Folder",
                    app.created
                        .as_ref()
                        .map(|i| i.downloads_dir().display().to_string())
                        .unwrap_or_else(|| "(no instance open)".to_string()),
                ),
                self.note(
                    "The site's Mod Manager Download button lands here once the nxm:// handler \
                     is registered (eidos nxm --register)."
                        .to_string(),
                ),
                setting_row(
                    typo,
                    "Preferred servers",
                    "Comma-separated CDN names, best first - a mod downloads from the first one \
                     Nexus offers today. Only a premium account is given more than one to choose \
                     between; for everyone else Nexus picks, and this changes nothing.",
                    servers,
                ),
            ],
        )
    }

    fn offline<'a>(&self) -> Element<'a, Message> {
        let app = self.app;
        self.section(
            "offline",
            "Offline",
            "Whether Eidos talks to Nexus at all.",
            [functional_toggle(
                &self.typo,
                "Offline mode",
                "Stops Eidos contacting Nexus at all. Update checks, sign-in, downloads and \
                 collections say so instead of failing with a connection error.",
                app.prefs.offline,
                Message::ToggleOffline(!app.prefs.offline),
            )],
        )
    }

    fn paths<'a>(&self) -> Element<'a, Message> {
        let app = self.app;
        self.section(
            "paths",
            "Where things live",
            "The files Eidos reads and writes for this window.",
            [
                self.info(
                    "Instance",
                    app.created
                        .as_ref()
                        .map(|i| i.root.display().to_string())
                        .unwrap_or_else(|| "(none open)".to_string()),
                ),
                self.info(
                    "Preferences file",
                    UiPreferences::default_path().display().to_string(),
                ),
                self.info(
                    "Shared with the CLI",
                    eidos_instance::settings::settings_path().display().to_string(),
                ),
                self.info(
                    "Nexus session",
                    eidos_instance::settings::nexus_key_path().display().to_string(),
                ),
                self.info("Games", "~/.config/Colony/Eidos/games/*.toml".to_string()),
            ],
        )
    }

    fn shortcuts<'a>(&self) -> Element<'a, Message> {
        self.section(
            "shortcuts",
            "Shortcuts",
            "The keys the main window answers to.",
            [
                self.info("Run", "Ctrl+R".to_string()),
                self.info("Refresh", "F5".to_string()),
                self.info("Select", "Ctrl+click, Shift+click for a range".to_string()),
                self.info("Select all", "Ctrl+A".to_string()),
                self.info("Clear", "Esc".to_string()),
                self.info("Reorder", "drag a row, or Ctrl+Up/Down".to_string()),
            ],
        )
    }

    fn version<'a>(&self) -> Element<'a, Message> {
        self.section(
            "version",
            "Version",
            "A Linux-native mod manager modelled on Mod Organizer 2: isolated instances, a \
             virtual file system over the game, FOMOD installs, LOOT sorting, and Nexus \
             integration.",
            [self.info("Eidos", env!("CARGO_PKG_VERSION").to_string())],
        )
    }
}
