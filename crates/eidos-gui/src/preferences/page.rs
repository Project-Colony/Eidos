//! The Preferences page: one category at a time from the rail on its left,
//! each a column of collapsible sections.
//!
//! It replaces the content area rather than opening over it, so the program's
//! own chrome stays put while it is open.

use iced::widget::slider;

use crate::theme::*;
use crate::widgets::*;
use crate::*;

/// A wrapped game id for the default-game `pick_list` (so it has a Display label).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefaultGameChoice {
    /// `None` = "(none)".
    id: Option<String>,
    label: String,
}

impl std::fmt::Display for DefaultGameChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

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
/// popover, which the Colony convention names as the three counter-examples -
/// and this was the first of them. The name is `preferences_page` rather than
/// `settings_dialog` because it is no longer a dialog, and a function whose name
/// disagrees with what it returns is how the next reader is misled.
/// The theme picker: Eidos's own parchment first, then the 25 families of the
/// shared Colony catalogue, 57 palettes in all.
///
/// Drawn here rather than with `colony_ui::widgets::theme_picker`. The catalogue
/// is the valuable part and it is shared; the widget is a hundred lines built to
/// Colony's type scale, whose body text is 13 where this window's is 12 - a
/// picker a size and a half larger than the page around it. The DATA is shared,
/// the drawing is this program's.
fn theme_picker<'a>(app: &App) -> Element<'a, Message> {
    let chosen = (
        app.prefs.theme_family.as_str(),
        app.prefs.theme_variant.as_str(),
    );

    // Eidos's own, on its own row, because it is not in the catalogue and must
    // still be reachable - a picker you cannot come back through is a trap.
    let mut col = Column::new().spacing(10).push(
        Column::new()
            .spacing(4)
            .push(text(theme::OWN_LABEL).size(12.0))
            .push(Row::new().spacing(6).push(theme_card(
                theme::OWN_VARIANT_LABEL,
                theme::PARCHMENT.bg_primary,
                theme::PARCHMENT.accent_blue,
                chosen == (theme::OWN_FAMILY, theme::OWN_VARIANT),
                Message::ThemeChanged(
                    theme::OWN_FAMILY.to_string(),
                    theme::OWN_VARIANT.to_string(),
                ),
            ))),
    );

    // And the catalogue, straight from `THEME_FAMILIES`. Adding a family
    // upstream needs no change here and no new arm: that is the whole point of
    // the generated catalogue.
    for family in colony_ui::THEME_FAMILIES {
        let mut variants = Row::new().spacing(6);
        for variant in family.variants {
            variants = variants.push(theme_card(
                colony_ui::i18n::t(variant.label_key),
                variant.swatch_bg_color(),
                variant.swatch_accent_color(),
                chosen == (family.key, variant.key),
                Message::ThemeChanged(family.key.to_string(), variant.key.to_string()),
            ));
        }
        col = col.push(
            Column::new()
                .spacing(4)
                .push(text(colony_ui::i18n::t(family.label_key)).size(12.0))
                .push(variants),
        );
    }

    scrollable(col).height(Length::Fixed(300.0)).into()
}

/// One card: a field of the variant's background crossed by a bar of its accent,
/// its name underneath, a border when it is the chosen one.
///
/// The swatch comes from the tokens rather than being recomputed, so a card
/// always resembles the theme it selects. A picker whose cards do not is a
/// picker that lies.
fn theme_card<'a>(
    label: &'a str,
    bg: Color,
    accent_of: Color,
    active: bool,
    msg: Message,
) -> Element<'a, Message> {
    let swatch = container(
        container(Space::new().width(Length::Fill).height(Length::Fixed(3.0))).style(
            move |_t: &Theme| container::Style {
                background: Some(Background::Color(accent_of)),
                border: Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
        ),
    )
    .width(Length::Fill)
    .height(Length::Fixed(24.0))
    .padding(iced::Padding {
        top: 16.0,
        right: 5.0,
        bottom: 3.0,
        left: 5.0,
    })
    .style(move |_t: &Theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    button(
        Column::new()
            .spacing(2)
            .width(Length::Fixed(78.0))
            .push(swatch)
            .push(text(label).size(10.0)),
    )
    .padding(3)
    .on_press(msg)
    .style(move |_t: &Theme, status: button::Status| button::Style {
        background: Some(Background::Color(pal().bg_card)),
        text_color: pal().text_primary,
        border: Border {
            color: if active {
                accent()
            } else if matches!(status, button::Status::Hovered) {
                pal().text_dimmer
            } else {
                pal().border_subtle
            },
            width: if active { 2.0 } else { 1.0 },
            radius: 5.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// The eight accent overrides, plus the way back to the theme's own.
///
/// The list and its ORDER come from `ACCENT_OVERRIDES`, generated from
/// `tokens/accents.toml`. Copying it here would be the mistake Colony had made:
/// the order is load-bearing across the ecosystem.
fn accent_picker<'a>(app: &App) -> Element<'a, Message> {
    let chosen = app.prefs.accent.as_deref();
    let mut row = Row::new().spacing(6).align_y(iced::Alignment::Center);

    for a in colony_ui::ACCENT_OVERRIDES {
        let active = chosen == Some(a.key);
        let dot = colony_ui::hex(a.color);
        row = row.push(
            button(
                Space::new()
                    .width(Length::Fixed(20.0))
                    .height(Length::Fixed(20.0)),
            )
            .padding(0)
            .on_press(Message::AccentChanged(Some(a.key.to_string())))
            .style(move |_t: &Theme, status: button::Status| button::Style {
                background: Some(Background::Color(dot)),
                border: Border {
                    color: if active {
                        pal().text_primary
                    } else if matches!(status, button::Status::Hovered) {
                        pal().text_dimmer
                    } else {
                        Color::TRANSPARENT
                    },
                    width: if active { 2.0 } else { 0.0 },
                    radius: 10.0.into(),
                },
                ..Default::default()
            }),
        );
    }

    // "Auto" is the ABSENCE of an override, not a ninth colour - so it is a
    // button that clears, not a swatch that sets.
    let clear = button(text("Theme's own").size(11.0))
        .padding([3, 8])
        .on_press(Message::AccentChanged(None))
        .style(if chosen.is_none() {
            button::secondary
        } else {
            button::text
        });

    Column::new()
        .spacing(6)
        .push(row)
        .push(clear)
        .push(
            text("With no accent picked, the theme's own is used.")
                .size(10.0)
                .color(text_muted()),
        )
        .into()
}

pub(crate) fn preferences_page<'a>(app: &App) -> Element<'a, Message> {
    // "Preferences", not "Settings". The convention settles the user-facing word
    // and Colony, Grape and Xion all say Preferences; only the code here is still
    // named `settings_*`, which costs the user nothing and is not worth a rename
    // on its own. Size and padding are the convention's too.
    let header = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text("Preferences").size(22.0).width(Length::Fill))
        .push(
            // The same message the title button sends to OPEN the page: two ways
            // out, one message, so they cannot drift apart.
            button(text("Close").size(13.0))
                .padding([6, 14])
                .on_press(Message::CloseSettings)
                .style(button::secondary),
        );

    // A vertical rail, as in Colony: five sections do not fit across a dialog,
    // and a rail takes a sixth without re-laying anything out.
    // Wide enough for "Accessibility", the longest of the six, so the rail does
    // not resize when the category changes.
    let mut rail = Column::new().spacing(2).width(Length::Fixed(148.0));
    for tab in SettingsTab::ALL {
        let active = app.settings_tab == tab;
        rail = rail.push(
            button(text(tab.label()).size(12.0).width(Length::Fill))
                // The convention's own numbers, and the same selection rules as
                // the main list: selected is a filled background, hover is the
                // card hover, and the two must not look alike.
                .padding([8, 14])
                .width(Length::Fill)
                .on_press(Message::SettingsTabSelected(tab))
                .style(if active {
                    button::primary
                } else {
                    button::text
                }),
        );
    }

    let open = |k: &'static str| app.settings_expanded.contains(k);
    let body: Element<'a, Message> = match app.settings_tab {
        SettingsTab::General => {
            let mut games = vec![DefaultGameChoice { id: None, label: "(none)".to_string() }];
            for g in eidos_games::catalog() {
                games.push(DefaultGameChoice { id: Some(g.id.to_string()), label: g.name.to_string() });
            }
            let selected_game = games
                .iter()
                .find(|c| c.id == app.prefs.default_game)
                .cloned()
                .unwrap_or_else(|| DefaultGameChoice { id: None, label: "(none)".to_string() });
            let picker = pick_list(games, Some(selected_game), |c: DefaultGameChoice| {
                Message::DefaultGameChanged(c.id)
            })
            .text_size(12.0)
            .padding(6);

            Column::new()
                .spacing(2)
                .push(settings_section(
                    "startup",
                    "Startup",
                    open("startup"),
                    Column::new()
                        .spacing(2)
                        .push(settings_row(
                            "Default game",
                            "Opened when Eidos starts without being told which.",
                            picker.into(),
                        ))
                        .push(settings_toggle(
                            "Remember the window size",
                            "Restore the last size on launch instead of letting the compositor choose.",
                            app.prefs.remember_window,
                            Message::ToggleRememberWindow(!app.prefs.remember_window),
                        ))
                        .into(),
                ))
                .push(settings_section(
                    "tools",
                    "Tools",
                    open("tools"),
                    Column::new()
                        .spacing(6)
                        .push(settings_row(
                            "Tools folder",
                            "Where you keep xEdit, DynDOLOD and the like. Eidos already looks in \
                             the game folder and inside this instance's mods; this is for the \
                             directory shared between instances. A tool found here is offered \
                             with the runtimes its name calls for.",
                            Row::new()
                                .spacing(6)
                                .align_y(iced::Alignment::Center)
                                .push(
                                    text_input("/mnt/Games/Tools", &app.tools_dir_edit)
                                        .on_input(Message::ToolsDirChanged)
                                        .on_submit(Message::ToolsDirSave)
                                        .padding(5)
                                        .size(12.0),
                                )
                                .push(
                                    button(text("Browse").size(11.0))
                                        .padding([4, 10])
                                        .style(button::secondary)
                                        .on_press(Message::BrowseToolsDir),
                                )
                                .push(
                                    button(text("Save").size(11.0))
                                        .padding([4, 10])
                                        .style(button::secondary)
                                        .on_press(Message::ToolsDirSave),
                                )
                                .into(),
                        ))
                        .into(),
                ))
                .push(settings_section(
                    "running",
                    "Running a game",
                    open("running"),
                    settings_toggle(
                        "Lock the window while a game runs",
                        "Blocks the main window behind an overlay until the game exits, with an Unlock escape hatch.",
                        app.prefs.lock_gui,
                        Message::ToggleLockGui(!app.prefs.lock_gui),
                    ),
                ))
                .into()
        }
        SettingsTab::Appearance => Column::new()
            .spacing(2)
            .push(settings_section(
                "theme",
                "Theme",
                open("theme"),
                theme_picker(app),
            ))
            .push(settings_section(
                "accent",
                "Colours",
                open("accent"),
                accent_picker(app),
            ))
            .into(),
        // The convention puts motion under Accessibility, not Appearance, and
        // the distinction is not filing: Appearance is what the window looks
        // like, Accessibility is what it does to somebody who needs it to do
        // less. Reduced motion is the second.
        SettingsTab::Accessibility => Column::new()
            .spacing(2)
            .push(settings_section(
                "motion",
                "Motion",
                open("motion"),
                settings_toggle(
                    "Animate the window",
                    "The tab strips cross-fade and a new status message fades in. Off means \
                     they change instantly - nothing is animated more quickly, none of it \
                     runs, and no frame timer is started. No animation moves the layout \
                     either way, so turning this off changes no spacing.",
                    app.prefs.motion,
                    Message::ToggleMotion(!app.prefs.motion),
                ),
            ))
            .push(settings_section(
                "vision",
                "Vision",
                open("vision"),
                Column::new()
                    .spacing(2)
                    .push(settings_toggle(
                        "High contrast",
                        "Strengthens the separation between surfaces and text. Derived from \
                         whichever theme is on, so it works on the parchment and on all 57 \
                         palettes - no theme ships a separate high-contrast twin.",
                        app.prefs.high_contrast,
                        Message::ToggleHighContrast(!app.prefs.high_contrast),
                    ))
                    .push(settings_row(
                        "Text size",
                        "Not implemented. The convention asks for a text scale and a \
                         dyslexia-friendly font here; every size in this window is a literal, \
                         so a scale is a change to the whole GUI rather than a setting. Said \
                         out loud rather than shown as a switch that does nothing.",
                        Space::new().into(),
                    ))
                    .into(),
            ))
            .into(),
        SettingsTab::ModList => {
            let speed = app.prefs.drag_scroll_speed;
            let slider_row = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(
                    slider(0.25..=4.0, speed, Message::DragScrollSpeedChanged)
                        .step(0.25_f32)
                        .width(Length::Fixed(160.0)),
                )
                .push(text(format!("{speed:.2}x")).size(12.0).width(Length::Fixed(42.0)));
            Column::new()
                .spacing(2)
                .push(settings_section(
                    "dragging",
                    "Dragging",
                    open("dragging"),
                    settings_row(
                        "Auto-scroll speed",
                        "How fast the list moves when a dragged mod rests on its top or bottom edge. Speed already rises the closer to the edge; this scales the whole range.",
                        slider_row.into(),
                    ),
                ))
                .into()
        }
        SettingsTab::Nexus => {
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
            let mut action = button(text(label).size(12.0)).padding([5, 12]).style(button::primary);
            if !app.nexus_signing_in {
                action = action.on_press(Message::NexusSignInStart);
            }
            let mut controls = Row::new().spacing(8).push(action);
            if signed_in {
                controls = controls.push(
                    button(text("Sign out").size(12.0))
                        .padding([5, 12])
                        .on_press(Message::NexusSignOut)
                        .style(button::secondary),
                );
            }

            let mut account = Column::new().spacing(6).push(controls);
            match &app.nexus_account {
                Some(a) => {
                    let tier = if a.is_premium { "Premium" } else { "free" };
                    account =
                        account.push(text(format!("Signed in as {} ({tier}).", a.name)).size(11.0));
                    // Say what is being withheld and why. Adult mods coming back
                    // blank with no explanation reads as Eidos being broken, and
                    // "could not check" is the case the user can actually act on.
                    account = account.push(
                        text(match adult_content_state() {
                            Some(true) => "Adult content: shown (enabled on your Nexus account).",
                            Some(false) => {
                                "Adult content: hidden. It is turned off on your Nexus account; \
                                 change it on nexusmods.com, then sign in again here."
                            }
                            None => {
                                "Adult content: hidden. Eidos could not read your Nexus content \
                                 settings, so it withholds adult mods until it can."
                            }
                        })
                        .size(10.0),
                    );
                }
                None => account = account.push(text("Not signed in.").size(11.0)),
            }
            if let Some(err) = &app.nexus_error {
                account = account
                    .push(text(format!("Error: {err}")).size(11.0).color(pal().error));
            }

            Column::new()
                .spacing(2)
                .push(settings_section(
                    "account",
                    "Account",
                    open("account"),
                    Column::new()
                        .spacing(6)
                        .push(
                            text("Signing in opens your browser. The session is stored in nexus.ini and shared with the CLI.")
                                .size(10.0),
                        )
                        .push(account)
                        .into(),
                ))
                .push(settings_section(
                    "downloads",
                    "Downloads",
                    open("downloads"),
                    Column::new()
                        .spacing(2)
                        .push(settings_info(
                            "Folder",
                            app.created
                                .as_ref()
                                .map(|i| i.downloads_dir().display().to_string())
                                .unwrap_or_else(|| "(no instance open)".to_string()),
                        ))
                        .push(
                            text("The site's Mod Manager Download button lands here once the nxm:// handler is registered (eidos nxm --register).")
                                .size(10.0),
                        )
                        .push(settings_row(
                            "Preferred servers",
                            "Comma-separated CDN names, best first - a mod downloads from the \
                             first one Nexus offers today. Only a premium account is given more \
                             than one to choose between; for everyone else Nexus picks, and this \
                             changes nothing.",
                            Row::new()
                                .spacing(6)
                                .align_y(iced::Alignment::Center)
                                .push(
                                    text_input("Nexus CDN, Paris, Chicago", &app.servers_edit)
                                        .on_input(Message::PreferredServersChanged)
                                        .on_submit(Message::PreferredServersSave)
                                        .padding(5)
                                        .size(12.0),
                                )
                                // Enter is not discoverable, and the field
                                // showed a preference that had never been
                                // stored - the one state a settings box must
                                // never be in.
                                .push(
                                    button(text("Save").size(11.0))
                                        .padding([4, 10])
                                        .style(button::secondary)
                                        .on_press(Message::PreferredServersSave),
                                )
                                .into(),
                        ))
                        .into(),
                ))
                .push(settings_section(
                    "offline",
                    "Offline",
                    open("offline"),
                    settings_toggle(
                        "Offline mode",
                        "Stops Eidos contacting Nexus at all. Update checks, sign-in, downloads \
                         and collections say so instead of failing with a connection error.",
                        app.prefs.offline,
                        Message::ToggleOffline(!app.prefs.offline),
                    ),
                ))
                .into()
        }
        SettingsTab::About => Column::new()
            .spacing(2)
            .push(settings_section(
                "paths",
                "Where things live",
                open("paths"),
                Column::new()
                    .spacing(2)
                    .push(settings_info(
                        "Instance",
                        app.created
                            .as_ref()
                            .map(|i| i.root.display().to_string())
                            .unwrap_or_else(|| "(none open)".to_string()),
                    ))
                    .push(settings_info(
                        "Settings",
                        eidos_instance::settings::settings_path().display().to_string(),
                    ))
                    .push(settings_info(
                        "Nexus",
                        eidos_instance::settings::nexus_key_path().display().to_string(),
                    ))
                    .push(settings_info("Games", "~/.config/Colony/Eidos/games/*.toml".to_string()))
                    .into(),
            ))
            .push(settings_section(
                "shortcuts",
                "Shortcuts",
                open("shortcuts"),
                Column::new()
                    .spacing(2)
                    .push(settings_info("Run", "Ctrl+R".to_string()))
                    .push(settings_info("Refresh", "F5".to_string()))
                    .push(settings_info("Select", "Ctrl+click, Shift+click for a range".to_string()))
                    .push(settings_info("Select all", "Ctrl+A".to_string()))
                    .push(settings_info("Clear", "Esc".to_string()))
                    .push(settings_info("Reorder", "drag a row, or Ctrl+Up/Down".to_string()))
                    .into(),
            ))
            .push(settings_section(
                "version",
                "Version",
                open("version"),
                Column::new()
                    .spacing(2)
                    .push(settings_info("Eidos", env!("CARGO_PKG_VERSION").to_string()))
                    .push(
                        text("A Linux-native mod manager modelled on Mod Organizer 2: isolated instances, a virtual file system over the game, FOMOD installs, LOOT sorting, and Nexus integration.")
                            .size(10.0),
                    )
                    .into(),
            ))
            .into(),
    };

    // Each category opens with its own heading and one line saying what it
    // changes. General's line carries the contract: there is no Save button.
    let titled = Column::new()
        .spacing(2)
        .push(text(app.settings_tab.label()).size(16.0))
        .push(
            text(app.settings_tab.description())
                .size(11.0)
                .color(text_muted()),
        )
        .push(Space::new().height(Length::Fixed(8.0)))
        .push(body);

    let panes = Row::new().spacing(14).height(Length::Fill).push(rail).push(
        container(scrollable(titled))
            .width(Length::Fill)
            .height(Length::Fill),
    );

    // A PAGE, not a card floating on a scrim. The convention names a modal, a
    // separate window and a popover as the three things this must not be, and
    // this was the first of them: it replaces the content area between the
    // toolbar and the status bar, and the program's own chrome stays put.
    //
    // What that buys is not tidiness. A modal capped at 620x240 could not grow,
    // so every category was read through a 240-pixel slot however big the window
    // was - and the page that most needs room to be scanned was the one with the
    // least.
    Column::new()
        .spacing(12)
        .padding(4)
        .width(Length::Fill)
        .height(Length::Fill)
        .push(header)
        .push(panes)
        .into()
}
