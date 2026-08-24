//! The modal dialogs that are not the mod info one: preferences, the executables
//! editor, the about box, the LOOT sort report, and the cards that report a
//! running game or a missing capability.
//!
//! Split out of `main.rs` unchanged.

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

/// A wrapped theme for the theme `pick_list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThemeChoice(PrefTheme);

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            PrefTheme::System => "Follow system",
            PrefTheme::Light => "Light",
            PrefTheme::Dark => "Dark",
        })
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

pub(crate) fn settings_dialog<'a>(app: &App) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text("Settings").size(18.0).width(Length::Fill))
        .push(
            button(text("Close").size(12.0))
                .padding([5, 12])
                .on_press(Message::CloseSettings)
                .style(button::secondary),
        );

    // A vertical rail, as in Colony: five sections do not fit across a dialog,
    // and a rail takes a sixth without re-laying anything out.
    let mut rail = Column::new().spacing(2).width(Length::Fixed(132.0));
    for tab in SettingsTab::ALL {
        let active = app.settings_tab == tab;
        rail = rail.push(
            button(text(tab.label()).size(12.0).width(Length::Fill))
                .padding([6, 10])
                .width(Length::Fill)
                .on_press(Message::SettingsTabSelected(tab))
                .style(if active { button::primary } else { button::text }),
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
        SettingsTab::Appearance => {
            let themes = vec![
                ThemeChoice(PrefTheme::System),
                ThemeChoice(PrefTheme::Light),
                ThemeChoice(PrefTheme::Dark),
            ];
            let picker = pick_list(themes, Some(ThemeChoice(app.prefs.theme)), |c: ThemeChoice| {
                Message::ThemeChanged(c.0)
            })
            .text_size(12.0)
            .padding(6);
            Column::new()
                .spacing(2)
                .push(settings_section(
                    "theme",
                    "Theme",
                    open("theme"),
                    settings_row(
                        "Colour theme",
                        "System follows your desktop's light/dark preference.",
                        picker.into(),
                    ),
                ))
                .into()
        }
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
                    .push(text(format!("Error: {err}")).size(11.0).color(Color::from_rgb8(0x8A, 0x2A, 0x2A)));
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

    let panes = Row::new()
        .spacing(14)
        .push(rail)
        .push(container(scrollable(body)).width(Length::Fill).height(Length::Fixed(240.0)));
    let card = Column::new().spacing(12).push(header).push(panes);
    container(card).max_width(620.0).padding(16).style(card_style).into()
}

// ---- Executables editor (MO2's Modify Executables) --------------------------

/// The Backups dialog: one column per list, each with a Back up now button and
/// its restore points newest first.
///
/// Both lists in one dialog on purpose - they are the two halves of "what my
/// setup looked like", and a bad LOOT sort touches the load order while a bad
/// drag touches the mod list. Splitting them across two menus would make the
/// user learn which button to look for while already in trouble.
pub(crate) fn backups_dialog<'a>(state: &BackupsDialogState) -> Element<'a, Message> {
    use eidos_instance::BackupKind;

    fn column<'a>(
        title: &'a str,
        what: &'a str,
        kind: BackupKind,
        list: &[eidos_instance::Backup],
    ) -> Element<'a, Message> {
        let mut col = Column::new()
            .spacing(6)
            .push(text(title).size(13.0))
            .push(text(what).size(11.0))
            .push(
                button(text("Back up now").size(12.0))
                    .padding([4, 10])
                    .style(button::primary)
                    .on_press(Message::CreateBackup(kind)),
            );
        if list.is_empty() {
            col = col.push(text("No restore points yet.").size(11.0));
        } else {
            let mut rows = Column::new().spacing(3);
            for b in list {
                rows = rows.push(
                    Row::new()
                        .spacing(8)
                        .align_y(iced::Alignment::Center)
                        .push(text(b.when()).size(12.0).width(Length::Fill))
                        .push(
                            button(text("Restore").size(11.0))
                                .padding([3, 8])
                                .style(button::secondary)
                                .on_press(Message::RestoreBackup(kind, b.stamp)),
                        ),
                );
            }
            col = col.push(scrollable(rows).height(Length::Fixed(180.0)));
        }
        col.width(Length::Fill).into()
    }

    let body = Row::new()
        .spacing(24)
        .push(column(
            "Mod list",
            "Order and enabled state of every mod.",
            BackupKind::ModList,
            &state.mods,
        ))
        .push(column(
            "Load order",
            "plugins.txt and loadorder.txt together.",
            BackupKind::LoadOrder,
            &state.order,
        ));

    let card = Column::new()
        .spacing(12)
        .push(
            Row::new()
                .align_y(iced::Alignment::Center)
                .push(text("Backups").size(18.0).width(Length::Fill))
                .push(
                    button(text("Close").size(12.0))
                        .padding([4, 12])
                        .style(button::secondary)
                        .on_press(Message::CloseBackupsDialog),
                ),
        )
        .push(body)
        .push(
            text("Restoring backs up the current state first, so a wrong pick can be undone.")
                .size(11.0),
        );

    container(card)
        .width(Length::Fixed(620.0))
        .padding(18)
        .style(card_style)
        .into()
}

pub(crate) fn executables_dialog<'a>(app: &App, state: &'a ExecutablesDialogState) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(6)
        .push(text("Executables").size(16.0).width(Length::Fill))
        .push(
            button(text("x").size(14.0))
                .padding([1, 8])
                .on_press(Message::CloseExecutablesDialog)
                .style(button::text),
        );

    // The tool list: user tools first (editable), then a "(defaults)" divider and
    // the read-only per-game defaults.
    // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
    for (i, t) in state.merged.iter().enumerate() {
        if i == state.user_len && i < state.merged.len() {
            list = list.push(text("Defaults (read-only)").size(10.0));
        }
        let selected = state.selected == Some(i);
        let label = if t.title.trim().is_empty() { "(unnamed)" } else { t.title.trim() };
        let is_default = i >= state.user_len;
        let display = if is_default { format!("{label}  (default)") } else { label.to_string() };
        list = list.push(
            button(text(display).size(12.0))
                .width(Length::Fill)
                .padding([3, 6])
                .on_press(Message::SelectExecutableTool(i))
                .style(if selected { button::primary } else { button::text }),
        );
    }
    if state.merged.is_empty() {
        list = list.push(text("No tools yet. Click Add to create one.").size(11.0));
    }
    let list_pane = container(scrollable(list).height(Length::Fill))
        .width(Length::Fixed(200.0))
        .height(Length::Fixed(280.0))
        .padding(4)
        .style(panel_style);

    let list_actions = Row::new()
        .spacing(4)
        .push(tool_btn("Add", Message::AddExecutableTool))
        .push(del_button(state))
        .push(move_button("Up", Message::MoveExecutableUp, can_move_up(state)))
        .push(move_button("Down", Message::MoveExecutableDown, can_move_down(state)));

    let left = Column::new().spacing(6).push(list_pane).push(list_actions);

    // The editor pane (only meaningful for a selected user tool).
    let editor: Element<'a, Message> = if state.selected_is_user() {
        Column::new()
            .spacing(8)
            .push(exe_field("Title", &state.title, Message::ToolTitleChanged))
            .push(exe_field_browse(
                "Executable (path)",
                &state.exe,
                Message::ToolExeChanged,
                Message::BrowseToolExe,
            ))
            .push(exe_field_browse(
                "Working dir (optional)",
                &state.workdir,
                Message::ToolWorkdirChanged,
                Message::BrowseToolWorkdir,
            ))
            // One argument per line, and the widget has to be able to HOLD a
            // newline: a `text_input` drops them, so the label promised
            // something the field refused, and a tool needing two arguments
            // could not be created through this dialog at all.
            .push(text("Arguments (one per line)").size(11.0))
            .push(
                iced::widget::text_editor::TextEditor::new(&state.args_editor)
                    .on_action(Message::ToolArgsAction)
                    .padding(6)
                    .height(Length::Fixed(72.0)),
            )
            .push(exe_field("Prereqs (comma-separated)", &state.prereqs, Message::ToolPrereqsChanged))
            .push(prereq_status_rows(app, &state.prereqs))
            .push(output_mod_field(state))
            .push(exe_field(
                "Steam AppID (blank = the game's)",
                &state.app_id,
                Message::ExecAppIdChanged,
            ))
            .push(tool_flags_row(state))
            .into()
    } else if state.selected.is_some() {
        // A default cannot be edited, but hiding and pinning are about the
        // PICKER rather than the tool, so they apply to a default too - and the
        // list of eight defaults is exactly what somebody wants to prune.
        Column::new()
            .spacing(8)
            .push(text("This is a per-game default and cannot be edited.").size(12.0))
            .push(text("Add a user tool with the same title to override it.").size(10.0))
            .push(tool_flags_row(state))
            .into()
    } else {
        Column::new()
            .spacing(8)
            .push(text("Select a tool to edit, or click Add to create one.").size(12.0))
            .into()
    };
    let right = container(editor).width(Length::Fill).padding(4);

    let panes = Row::new().spacing(12).push(left).push(right);

    let footer = Row::new()
        .spacing(8)
        .push(Space::new().width(Length::Fill))
        .push(tool_btn("Cancel", Message::CloseExecutablesDialog))
        .push(
            button(text("Save").size(12.0))
                .padding([5, 14])
                .on_press(Message::SaveExecutablesDialog)
                .style(button::primary),
        );

    let card = Column::new().spacing(12).push(header).push(panes).push(footer);
    container(card).max_width(720.0).padding(16).style(card_style).into()
}

/// A labelled single-line field for the Executables editor.
pub(crate) fn exe_field<'a>(
    label: &'a str,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    Column::new()
        .spacing(2)
        .push(text(label).size(11.0))
        .push(text_input("", value).on_input(on_input).padding(6).size(12.0).width(Length::Fill))
        .into()
}

/// Like [`exe_field`] but with a Browse button that opens a native file/folder
/// picker (`browse` message), so the user can pick the path instead of typing it.
pub(crate) fn exe_field_browse<'a>(
    label: &'a str,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
    browse: Message,
) -> Element<'a, Message> {
    let row = Row::new()
        .spacing(4)
        .push(text_input("", value).on_input(on_input).padding(6).size(12.0).width(Length::Fill))
        .push(button(text("Browse...").size(11.0)).padding([5, 8]).on_press(browse).style(button::secondary));
    Column::new().spacing(2).push(text(label).size(11.0)).push(row).into()
}

/// The Delete button: active only when a user tool is selected.
pub(crate) fn del_button<'a>(state: &ExecutablesDialogState) -> Element<'a, Message> {
    let mut b = button(text("Delete").size(12.0)).padding(6).style(button::danger);
    if state.selected_is_user() {
        b = b.on_press(Message::DeleteExecutableTool);
    }
    b.into()
}

/// A reorder button, greyed when the move is not possible.
pub(crate) fn move_button<'a>(label: &'a str, msg: Message, enabled: bool) -> Element<'a, Message> {
    let mut b = button(text(label).size(12.0)).padding(6).style(button::secondary);
    if enabled {
        b = b.on_press(msg);
    }
    b.into()
}

pub(crate) fn can_move_up(state: &ExecutablesDialogState) -> bool {
    matches!(state.selected, Some(i) if i > 0 && i < state.user_len)
}

pub(crate) fn can_move_down(state: &ExecutablesDialogState) -> bool {
    matches!(state.selected, Some(i) if i + 1 < state.user_len)
}

// ---- About box --------------------------------------------------------------

pub(crate) fn about_dialog<'a>() -> Element<'a, Message> {
    let card = Column::new()
        .spacing(8)
        .push(text("Eidos").size(20.0))
        .push(text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(12.0))
        .push(
            text("A Linux-native mod manager modelled on Mod Organizer 2: isolated instances, a virtual file system over the game, FOMOD installs, LOOT sorting, and Nexus integration.")
                .size(12.0),
        )
        .push(Space::new().height(Length::Fixed(6.0)))
        .push(text("Shortcuts").size(13.0))
        .push(
            text("Ctrl+R run   ·   F5 refresh   ·   Ctrl+click multi-select   ·   Shift+click range   ·   Esc clear   ·   drag a row to reorder")
                .size(11.0),
        )
        .push(Space::new().height(Length::Fixed(6.0)))
        .push(
            button(text("Close").size(12.0))
                .padding([5, 14])
                .on_press(Message::CloseAbout)
                .style(button::primary),
        );
    container(card).max_width(440.0).padding(16).style(card_style).into()
}

/// Whether the user opted into FUSE passthrough. Read from this process's own
/// environment because the launch environment is inherited from it (Steam launch
/// options land here first, then on the `eidos` child).
///
/// It is off by default: passthrough stops the game opening its own archives and
/// plugins (see `passthrough_enabled` in eidos-fuse for the measurement). This
/// gates the capability warnings, which are only meaningful to someone who
/// actually wants passthrough.
pub(crate) fn passthrough_requested() -> bool {
    std::env::var("EIDOS_FUSE_PASSTHROUGH").is_ok_and(|v| {
        let v = v.trim();
        !v.is_empty() && v != "0"
    })
}

/// The CAP_SYS_ADMIN warning banner, shown only when the user asked for
/// passthrough and the launch binary cannot deliver it (every rebuild wipes the
/// file capability). Shows the exact fix command; F5 rechecks after running it.
pub(crate) fn cap_warning_banner<'a>() -> Element<'a, Message> {
    let cmd = format!(
        "sudo setcap cap_sys_admin+ep {}",
        find_eidos_binary().display()
    );
    let row = Row::new()
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .push(text("Passthrough was requested but the launch binary has no CAP_SYS_ADMIN (a rebuild wipes it), so reads go through the daemon. Fix, then press F5:").size(11.0))
        .push(
            container(text(cmd).size(11.0))
                .padding([2, 8])
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
                    border: Border {
                        color: Color::from_rgb8(0xB0, 0x6A, 0x10),
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..Default::default()
                }),
        )
        .push(Space::new().width(Length::Fill))
        .push(flat_btn("Re-check (F5)", Message::Refresh));
    container(row)
        .width(Length::Fill)
        .padding([4, 8])
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xF6, 0xE3, 0xC0))),
            border: Border {
                color: Color::from_rgb8(0xB0, 0x6A, 0x10),
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: Some(Color::from_rgb8(0x6B, 0x42, 0x0A)),
            ..Default::default()
        })
        .into()
}

/// MO2's run-lock overlay card: the GUI is locked while the launched application
/// runs. Shows what is running and offers Unlock (which stops waiting but leaves
/// the game running - MO2's force-unlock never kills the process).
pub(crate) fn running_lock_card<'a>(run: &RunningState) -> Element<'a, Message> {
    let card = Column::new()
        .spacing(10)
        .align_x(iced::alignment::Horizontal::Center)
        .push(text("Eidos is locked while the application runs").size(18.0))
        .push(text(format!("{}  (pid {})", run.title, run.pid)).size(13.0))
        .push(
            text("It is being run through the merged mod view. Loading a save or starting a new game writes the load order; Eidos refreshes when it exits.")
                .size(11.0),
        )
        .push(Space::new().height(Length::Fixed(6.0)))
        .push(
            button(text("Unlock").size(13.0))
                .padding([6, 22])
                .on_press(Message::ForceUnlock)
                .style(button::primary),
        )
        .push(
            text("Unlock re-enables the GUI but leaves the game running.")
                .size(10.0)
                .color(Color::from_rgb8(0x6A, 0x5A, 0x40)),
        );
    container(card).max_width(470.0).padding(20).style(card_style).into()
}

/// Split a CommonMark string into plain runs and `[label](url)` links, in order.
/// LOOT's messages are markdown, and rendering them verbatim showed the bracket
/// syntax to the user; this keeps the label and hands back the URL to open.
pub(crate) fn split_markdown_links(text: &str) -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    let bytes = text.as_bytes();
    let mut plain = String::new();
    let mut i = 0;
    while i < bytes.len() {
        // A link is `[label](url)` with no nested bracket in the label.
        if bytes[i] == b'[' {
            if let Some(close) = text[i + 1..].find(']').map(|p| i + 1 + p) {
                if text.as_bytes().get(close + 1) == Some(&b'(') {
                    if let Some(end) = text[close + 2..].find(')').map(|p| close + 2 + p) {
                        let label = &text[i + 1..close];
                        let url = &text[close + 2..end];
                        if !label.is_empty() && !url.is_empty() {
                            if !plain.is_empty() {
                                out.push((std::mem::take(&mut plain), None));
                            }
                            out.push((label.to_string(), Some(url.to_string())));
                            i = end + 1;
                            continue;
                        }
                    }
                }
            }
        }
        plain.push(text[i..].chars().next().unwrap_or('\0'));
        i += text[i..].chars().next().map(char::len_utf8).unwrap_or(1);
    }
    if !plain.is_empty() {
        out.push((plain, None));
    }
    out
}

/// One LOOT message rendered with MO2's severity prefix and a severity colour
/// (Error red, Warning amber, Say muted). Markdown links become clickable
/// buttons that open in the browser, instead of showing raw `[label](url)`.
pub(crate) fn loot_message_row<'a>(m: &eidos_loot::LootMessage) -> Element<'a, Message> {
    use eidos_loot::MessageType;
    let (prefix, color) = match m.kind {
        MessageType::Error => ("Error: ", Color::from_rgb8(0x8A, 0x2A, 0x2A)),
        MessageType::Warn => ("Warning: ", Color::from_rgb8(0xB0, 0x6A, 0x10)),
        MessageType::Say => ("", Color::from_rgb8(0x4A, 0x40, 0x30)),
    };
    let parts = split_markdown_links(&m.text);
    if parts.iter().all(|(_, url)| url.is_none()) {
        return text(format!("{prefix}{}", m.text)).size(11.0).color(color).into();
    }
    let mut row = Row::new().spacing(0).align_y(iced::Alignment::Center);
    if !prefix.is_empty() {
        row = row.push(text(prefix).size(11.0).color(color));
    }
    for (label, url) in parts {
        row = match url {
            Some(u) => row.push(
                button(text(label).size(11.0).color(Color::from_rgb8(0x2B, 0x4F, 0x8A)))
                    .padding(0)
                    .on_press(Message::OpenUrl(u))
                    .style(button::text),
            ),
            None => row.push(text(label).size(11.0).color(color)),
        };
    }
    // `wrap` keeps a long advisory readable instead of running off the dialog.
    row.wrap().into()
}

/// MO2's post-sort LOOT report dialog: a summary line, then LOOT's general messages
/// and a per-plugin list of problems (missing masters, messages, dirty-plugin
/// cleaning advice). Shown after every sort, like MO2's LOOT dialog.
pub(crate) fn loot_report_dialog<'a>(report: &eidos_loot::LootReport) -> Element<'a, Message> {
    let summary = if report.is_empty() {
        "LOOT found no issues - your load order is clean.".to_string()
    } else {
        let mut parts: Vec<String> = Vec::new();
        if report.error_count() > 0 {
            parts.push(format!("{} error(s)", report.error_count()));
        }
        if report.warning_count() > 0 {
            parts.push(format!("{} warning(s)", report.warning_count()));
        }
        if report.missing_master_count() > 0 {
            parts.push(format!("{} with missing masters", report.missing_master_count()));
        }
        if report.dirty_count() > 0 {
            parts.push(format!("{} need cleaning", report.dirty_count()));
        }
        if parts.is_empty() {
            "LOOT messages".to_string()
        } else {
            parts.join(", ")
        }
    };

    let mut body = Column::new().spacing(12);

    if !report.general.is_empty() {
        let mut sec = Column::new().spacing(3).push(text("General messages").size(14.0));
        for m in &report.general {
            sec = sec.push(loot_message_row(m));
        }
        body = body.push(sec);
    }

    for p in &report.plugins {
        let mut sec = Column::new().spacing(2).push(text(p.name.clone()).size(13.0));
        if !p.missing_masters.is_empty() {
            sec = sec.push(
                text(format!("Missing masters: {}", p.missing_masters.join(", ")))
                    .size(11.0)
                    .color(Color::from_rgb8(0x8A, 0x2A, 0x2A)),
            );
        }
        for m in &p.messages {
            sec = sec.push(loot_message_row(m));
        }
        for d in &p.dirty {
            let util = if d.cleaning_utility.is_empty() { "?" } else { d.cleaning_utility.as_str() };
            sec = sec.push(
                text(format!(
                    "Dirty - {util} found {} ITM, {} deleted refs, {} deleted navmeshes (clean with xEdit)",
                    d.itm_count, d.deleted_reference_count, d.deleted_navmesh_count
                ))
                .size(11.0)
                .color(Color::from_rgb8(0xB0, 0x6A, 0x10)),
            );
        }
        body = body.push(sec);
    }

    let card = Column::new()
        .spacing(10)
        .push(text("LOOT report").size(20.0))
        .push(text(summary).size(12.0))
        .push(scrollable(body).height(Length::Fixed(360.0)))
        .push(
            Row::new()
                .spacing(8)
                .push(
                    button(text("Close").size(12.0))
                        .padding([5, 14])
                        .on_press(Message::CloseLootReport)
                        .style(button::primary),
                )
                // The report is a worklist: the plugins to clean get read off it
                // while xEdit runs on another screen. Selecting rich text inside a
                // modal is not something this toolkit does, so hand over the whole
                // thing in one press - which is what the Ctrl+A/Ctrl+C people are
                // really after anyway. Ctrl+C does the same while this is open.
                .push(
                    button(text("Copy report").size(12.0))
                        .padding([5, 14])
                        .on_press(Message::CopyLootReport)
                        .style(button::secondary),
                ),
        );
    container(card).max_width(580.0).padding(16).style(card_style).into()
}

/// The report as plain text, for the clipboard.
///
/// Deliberately not the on-screen layout: colour carries the severity there, and
/// a paste into a text editor would lose it silently. Each line says what it is.
pub(crate) fn loot_report_text(report: &eidos_loot::LootReport) -> String {
    let mut out = String::from("LOOT report\n");
    if report.is_empty() {
        out.push_str("\nNo issues - the load order is clean.\n");
        return out;
    }
    if !report.general.is_empty() {
        out.push_str("\nGeneral messages\n");
        for m in &report.general {
            out.push_str(&format!("  [{}] {}\n", loot_severity_label(m.kind), m.text));
        }
    }
    for p in &report.plugins {
        out.push_str(&format!("\n{}\n", p.name));
        if !p.missing_masters.is_empty() {
            out.push_str(&format!("  Missing masters: {}\n", p.missing_masters.join(", ")));
        }
        for m in &p.messages {
            out.push_str(&format!("  [{}] {}\n", loot_severity_label(m.kind), m.text));
        }
        for d in &p.dirty {
            let util = if d.cleaning_utility.is_empty() { "?" } else { d.cleaning_utility.as_str() };
            out.push_str(&format!(
                "  Dirty - {util} found {} ITM, {} deleted refs, {} deleted navmeshes (clean with xEdit)\n",
                d.itm_count, d.deleted_reference_count, d.deleted_navmesh_count
            ));
        }
    }
    out
}

pub(crate) fn loot_severity_label(kind: eidos_loot::MessageType) -> &'static str {
    match kind {
        eidos_loot::MessageType::Error => "error",
        eidos_loot::MessageType::Warn => "warning",
        eidos_loot::MessageType::Say => "note",
    }
}

/// The Categories dialog (MO2's Change Categories, plus its category editor).
///
/// Two modes in one card because they are one job: assigning a category the
/// catalog does not have yet means editing the catalog, and MO2 makes the user
/// leave the mod, open Settings, find the Categories tab, come back, and find the
/// mod again to do it.
pub(crate) fn categories_dialog<'a>(state: &CategoriesDialogState) -> Element<'a, Message> {
    let title = if state.names.len() == 1 {
        format!("Categories - {}", state.names[0])
    } else {
        format!("Categories - {} mods", state.names.len())
    };

    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(title).size(18.0).width(Length::Fill))
        .push(button(text(if state.editing { "Done editing" } else { "Edit list..." }).size(12.0))
            .padding([4, 10])
            .style(button::secondary)
            .on_press(Message::ToggleCategoryEditor))
        .push(
            button(text("Close").size(12.0))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseCategoriesDialog),
        );

    let body = if state.editing { catalog_editor(state) } else { category_picker(state) };

    let mut foot = Row::new().spacing(8).align_y(iced::Alignment::Center);
    // What Apply will actually write, spelled out - the pending pick is invisible
    // otherwise once the tree is scrolled away from the checked rows.
    let summary = match state.chosen.split_first() {
        None => "No category (clears it)".to_string(),
        Some((p, rest)) => {
            let primary = state.catalog.name_for_id(*p).unwrap_or("?").to_string();
            if rest.is_empty() {
                primary
            } else {
                format!("{primary} (+{} more)", rest.len())
            }
        }
    };
    foot = foot
        .push(text(summary).size(12.0).width(Length::Fill))
        .push(
            button(text("Apply").size(12.0))
                .padding([4, 14])
                .style(button::primary)
                .on_press(Message::ApplyCategories),
        );

    let mut card = Column::new().spacing(12).push(header).push(body).push(foot);
    if state.names.len() > 1 {
        card = card.push(
            text("Applying sets the same categories on every selected mod, replacing what they had.")
                .size(11.0),
        );
    }

    container(card).width(Length::Fixed(620.0)).padding(18).style(card_style).into()
}

/// The assign side: a filtered tree of checkboxes, with the primary marked.
fn category_picker<'a>(state: &CategoriesDialogState) -> Element<'a, Message> {
    let q = state.query.trim().to_lowercase();
    let mut rows = Column::new().spacing(1);
    let mut shown = 0usize;
    for (id, name, depth) in state.catalog.tree() {
        // A filter hides a parent whose name does not match, but never a checked
        // row: the user must always be able to see - and uncheck - what is set.
        let checked = state.chosen.contains(&id);
        if !q.is_empty() && !name.to_lowercase().contains(&q) && !checked {
            continue;
        }
        shown += 1;
        let is_primary = state.chosen.first() == Some(&id);
        let mark = if checked { "[x]" } else { "[ ]" };
        let mut row = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(Space::new().width(Length::Fixed(14.0 * depth as f32)))
            .push(
                button(text(format!("{mark} {name}")).size(12.0))
                    .padding([2, 6])
                    .width(Length::Fill)
                    .style(if checked { button::secondary } else { button::text })
                    .on_press(Message::ToggleCategory(id)),
            );
        // Primary is what the mod list column shows, so it needs to be both
        // visible and settable without unchecking everything else first.
        if is_primary {
            row = row.push(text("primary").size(10.0));
        } else if checked {
            row = row.push(
                button(text("make primary").size(10.0))
                    .padding([2, 6])
                    .style(button::text)
                    .on_press(Message::SetPrimaryCategory(id)),
            );
        }
        rows = rows.push(row);
    }
    if shown == 0 {
        rows = rows.push(text("No category matches.").size(12.0));
    }

    let search = text_input("Filter categories", &state.query)
        .on_input(Message::CategoryQueryChanged)
        .padding(5)
        .size(12.0);

    let nexus = Row::new()
        .spacing(6)
        .push(tool_btn("Fetch from Nexus", Message::FetchNexusCategories))
        .push(tool_btn("Use Nexus category", Message::AssignCategoriesFromNexus));

    Column::new()
        .spacing(8)
        .push(search)
        .push(scrollable(rows).height(Length::Fixed(320.0)))
        .push(nexus)
        .push(
            text(
                "Fetch pulls this game's official category list; Use Nexus sets the pick from what \
                 the download recorded.",
            )
            .size(10.0),
        )
        .into()
}

/// The catalog side: rename, re-parent, delete, and add.
fn catalog_editor<'a>(state: &CategoriesDialogState) -> Element<'a, Message> {
    let mut rows = Column::new().spacing(1);
    for (id, name, depth) in state.catalog.tree() {
        // The row turns into an editor while it is the one being renamed.
        if let Some((rid, pending)) = &state.rename {
            if *rid == id {
                rows = rows.push(
                    Row::new()
                        .spacing(6)
                        .align_y(iced::Alignment::Center)
                        .push(Space::new().width(Length::Fixed(14.0 * depth as f32)))
                        .push(
                            text_input("Name", pending)
                                .on_input(Message::RenameCategoryChanged)
                                .on_submit(Message::RenameCategoryCommit)
                                .padding(4)
                                .size(12.0),
                        )
                        .push(
                            button(text("Save").size(11.0))
                                .padding([2, 8])
                                .style(button::primary)
                                .on_press(Message::RenameCategoryCommit),
                        ),
                );
                continue;
            }
        }
        let armed = state.confirm_delete == Some(id);
        rows = rows.push(
            Row::new()
                .spacing(6)
                .align_y(iced::Alignment::Center)
                .push(Space::new().width(Length::Fixed(14.0 * depth as f32)))
                .push(text(name).size(12.0).width(Length::Fill))
                .push(text(format!("#{id}")).size(10.0))
                .push(
                    button(text("Rename").size(10.0))
                        .padding([2, 6])
                        .style(button::text)
                        .on_press(Message::RenameCategoryStart(id)),
                )
                .push(
                    button(text(if armed { "Confirm?" } else { "Delete" }).size(10.0))
                        .padding([2, 6])
                        .style(if armed { button::danger } else { button::text })
                        .on_press(Message::DeleteCategory(id)),
                ),
        );
    }

    // Adding: name + a parent picked from the same tree.
    let parents: Vec<CategoryChoice> = std::iter::once(CategoryChoice { id: 0, label: "(top level)".to_string() })
        .chain(
            state
                .catalog
                .tree()
                .into_iter()
                .map(|(id, name, depth)| CategoryChoice { id, label: format!("{}{name}", "  ".repeat(depth)) }),
        )
        .collect();
    let selected = parents.iter().find(|c| c.id == state.new_parent).cloned();
    let add = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(
            text_input("New category name", &state.new_name)
                .on_input(Message::NewCategoryNameChanged)
                .on_submit(Message::AddCategory)
                .padding(5)
                .size(12.0),
        )
        .push(
            pick_list(parents, selected, |c: CategoryChoice| Message::NewCategoryParentChanged(c.id))
                .text_size(12.0)
                .padding(4),
        )
        .push(
            button(text("Add").size(12.0))
                .padding([4, 12])
                .style(button::primary)
                .on_press(Message::AddCategory),
        );

    Column::new()
        .spacing(8)
        .push(scrollable(rows).height(Length::Fixed(300.0)))
        .push(add)
        .push(
            text(
                "Deleting a category lifts its children onto its parent and leaves the mods using \
                 it alone - they show a bare id until re-categorised. Nothing is written until Apply.",
            )
            .size(10.0),
        )
        .into()
}

/// A wrapped category for the parent `pick_list` (so it has a Display label).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CategoryChoice {
    id: i32,
    label: String,
}

impl std::fmt::Display for CategoryChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}


/// MO2's "Create files in mod instead of overwrite", as a picker over the
/// instance's own mods rather than MO2's editable combo box.
///
/// A picker, not a text field, because MO2's whole "output mod not found" error
/// class exists only because its combo is editable: a typo there produces a tool
/// that appears configured and captures into nothing.
/// Hide, pin, and the desktop shortcut - three things about how a tool is
/// REACHED rather than what it runs, which is why they sit together and apply to
/// per-game defaults as well as to the user's own entries.
fn tool_flags_row<'a>(state: &ExecutablesDialogState) -> Element<'a, Message> {
    let sel = state.selected.and_then(|i| state.merged.get(i));
    let (hidden, pinned) = sel.map(|t| (t.hidden, t.pinned)).unwrap_or((false, false));
    Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(
            button(text(if pinned { "Unpin" } else { "Pin to top" }).size(11.0))
                .padding([4, 10])
                .style(if pinned { button::primary } else { button::secondary })
                .on_press(Message::ExecTogglePinned),
        )
        .push(
            button(text(if hidden { "Show in picker" } else { "Hide from picker" }).size(11.0))
                .padding([4, 10])
                .style(if hidden { button::primary } else { button::secondary })
                .on_press(Message::ExecToggleHidden),
        )
        .push(
            button(text("Desktop shortcut").size(11.0))
                .padding([4, 10])
                .style(button::secondary)
                .on_press(Message::ExecMakeShortcut),
        )
        .into()
}

fn output_mod_field<'a>(state: &ExecutablesDialogState) -> Element<'a, Message> {
    const NONE: &str = "(none - leave it in the Overwrite)";
    let mut choices: Vec<String> = vec![NONE.to_string()];
    choices.extend(state.mod_names.iter().cloned());
    let selected = if state.output_mod.trim().is_empty() {
        Some(NONE.to_string())
    } else {
        Some(state.output_mod.clone())
    };
    Column::new()
        .spacing(4)
        .push(text("Capture output into").size(11.0))
        .push(
            pick_list(choices, selected, |c: String| {
                Message::ToolOutputModChanged(if c == NONE { String::new() } else { c })
            })
            .text_size(12.0)
            .padding(5)
            .width(Length::Fill),
        )
        .push(
            text(
                "What this run writes goes into that mod instead of the Overwrite. Only the files \
                 THIS run produced move - anything already in the Overwrite stays put.",
            )
            .size(10.0),
        )
        .into()
}

/// The INI editor (MO2 ships one as a bundled tool plugin).
///
/// Worth more on Linux than it is on Windows: the copy the game reads lives deep
/// inside the Proton prefix, under a path nobody navigates to by hand. This edits
/// the PROFILE's copy, which is the durable one - the prefix copy is overwritten
/// from it at every launch.
pub(crate) fn ini_editor_dialog<'a>(
    app: &App,
    state: &'a IniEditorState,
) -> Element<'a, Message> {
    let profile = app
        .created
        .as_ref()
        .map(|i| i.active().name.clone())
        .unwrap_or_default();

    // One button per INI - two or three files, so tabs beat a dropdown.
    let mut tabs = Row::new().spacing(4);
    for f in &state.files {
        let on = *f == state.current;
        tabs = tabs.push(
            button(text(f.clone()).size(12.0))
                .padding([3, 10])
                .style(if on { button::primary } else { button::secondary })
                .on_press(Message::IniEditorPick(f.clone())),
        );
    }

    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(format!("INI editor - profile '{profile}'")).size(18.0).width(Length::Fill))
        .push(
            button(text("Close").size(12.0))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseIniEditor),
        );

    let editor = iced::widget::text_editor(&state.content)
        .on_action(Message::IniEditorAction)
        .height(Length::Fixed(420.0))
        .font(iced::Font::MONOSPACE)
        .size(12.0)
        .padding(8);

    // Say what this file IS before the user changes it: which one, whether it
    // exists yet, and that the game reads a deployed copy rather than this path.
    let note = if state.unreadable {
        format!(
            "{} exists but could not be read. Saving is refused - it would replace the file with \
             an empty one. Check its permissions.",
            state.current
        )
    } else if state.missing {
        format!(
            "{} is not in this profile yet. Saving creates it; until then the game uses whatever \
             is already in the prefix.",
            state.current
        )
    } else {
        "Edits the profile's copy. It is deployed into the Proton prefix at launch, and what \
         the game writes back is captured into the profile when it exits."
            .to_string()
    };

    let mut actions = Row::new().spacing(8).align_y(iced::Alignment::Center);
    actions = actions
        .push(text(if state.dirty { "Unsaved changes" } else { "" }).size(11.0).width(Length::Fill))
        .push(tool_btn("Open externally", Message::IniEditorOpenExternal));
    if state.dirty {
        actions = actions.push(tool_btn("Revert", Message::IniEditorRevert));
    }
    let save = button(text("Save").size(12.0))
        .padding([4, 14])
        .style(if state.dirty { button::primary } else { button::secondary });
    // Greyed rather than merely refused when the file could not be read: the
    // handler stops it too, but a button that looks live and does nothing is its
    // own bug report.
    actions = actions.push(if state.unreadable { save } else { save.on_press(Message::IniEditorSave) });

    let card = Column::new()
        .spacing(10)
        .push(header)
        .push(tabs)
        .push(editor)
        .push(text(note).size(10.0))
        .push(actions);

    container(card).width(Length::Fixed(760.0)).padding(18).style(card_style).into()
}

/// The log pane (MO2's dockable log view).
pub(crate) fn log_pane_dialog<'a>(state: &LogPaneState) -> Element<'a, Message> {
    use eidos_log::Level;

    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text("Log").size(18.0).width(Length::Fill))
        .push(tool_btn("Open folder", Message::LogOpenFolder))
        .push(tool_btn("Copy", Message::LogCopy))
        .push(tool_btn("Refresh", Message::LogRefresh))
        .push(
            button(text("Close").size(12.0))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseLogPane),
        );

    // The level floor. Four buttons rather than a dropdown: the whole point is
    // to flip between them while reading, which a dropdown costs two clicks.
    let mut levels = Row::new().spacing(4).align_y(iced::Alignment::Center);
    levels = levels.push(text("Show").size(11.0));
    for lvl in [Level::Debug, Level::Info, Level::Warn, Level::Error] {
        levels = levels.push(
            button(text(lvl.as_str()).size(11.0))
                .padding([2, 8])
                .style(if state.level == lvl { button::primary } else { button::secondary })
                .on_press(Message::LogLevel(lvl)),
        );
    }
    levels = levels.push(
        text(format!("{} of {} record(s)", state.lines.len(), state.total))
            .size(10.0)
            .width(Length::Fill),
    );

    // Which session. Newest first, and only a handful are kept, so they all fit.
    let mut sessions = Row::new().spacing(4).align_y(iced::Alignment::Center);
    for f in state.files.iter().take(8) {
        let label = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        // The instance and the time; the pid is noise on a button.
        let short: String = label.split('.').take(2).collect::<Vec<_>>().join(" ");
        sessions = sessions.push(
            button(text(short).size(10.0))
                .padding([2, 6])
                .style(if *f == state.current { button::primary } else { button::text })
                .on_press(Message::LogPick(f.clone())),
        );
    }

    let mut body = Column::new().spacing(1);
    if state.truncated {
        body = body.push(text("(showing the end of the file)").size(10.0));
    }
    if state.lines.is_empty() {
        body = body.push(
            text(if state.total == 0 {
                "This session logged nothing."
            } else {
                "Nothing at this level. Lower it to see more."
            })
            .size(12.0),
        );
    }
    for (lvl, msg) in &state.lines {
        let colour = match lvl {
            Level::Error => Some(CONFLICT_LOSES_FG),
            Level::Warn => Some(Color::from_rgb8(0x8A, 0x5A, 0x00)),
            _ => None,
        };
        let mut line = text(format!("{:<5} {msg}", lvl.as_str())).size(11.0).font(iced::Font::MONOSPACE);
        if let Some(c) = colour {
            line = line.color(c);
        }
        body = body.push(line);
    }

    let card = Column::new()
        .spacing(8)
        .push(header)
        .push(sessions)
        .push(levels)
        .push(scrollable(body).height(Length::Fixed(420.0)).width(Length::Fill));

    container(card).width(Length::Fixed(820.0)).padding(18).style(card_style).into()
}

/// The Extensions list: what is installed, whether it can run, and how to add one.
///
/// Called "extensions", never "plugins": in this window a plugin is an `.esp`,
/// and the Plugins tab three inches away shows exactly those.
pub(crate) fn addons_dialog<'a>(app: &App) -> Element<'a, Message> {
    use eidos_addons::AddonKind;

    let game_id = app.games.get(app.selected.unwrap_or(0)).map(|g| g.def.id.to_string());

    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text("Extensions").size(18.0).width(Length::Fill))
        .push(tool_btn("Open folder", Message::OpenAddonsFolder))
        .push(tool_btn("Reload", Message::ReloadAddons))
        .push(
            button(text("Close").size(12.0))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseAddons),
        );

    let mut rows = Column::new().spacing(6);
    // The refusals FIRST. A manifest that failed to parse simply does not
    // appear, and "no extensions yet" then tells the user their file is not
    // there when it is - one typo away from working.
    for (path, why) in &app.addon_rejected {
        rows = rows.push(
            Column::new()
                .spacing(2)
                .push(
                    text(path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
                        .size(13.0)
                        .color(CONFLICT_LOSES_FG),
                )
                .push(text(format!("refused: {why}")).size(11.0)),
        );
    }
    if app.addons.is_empty() && app.addon_rejected.is_empty() {
        rows = rows.push(
            text(
                "No extensions yet. Drop a .toml manifest into the folder above; the format is in \
                 docs/guide/extensions.md.",
            )
            .size(12.0),
        );
    }
    for a in &app.addons {
        let kind = match a.kind {
            AddonKind::Tool => "tool",
            AddonKind::Diagnose => "check",
        };
        let mut title = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text(a.name.clone()).size(13.0))
            .push(text(format!("({kind})")).size(10.0));
        if !a.version.is_empty() {
            title = title.push(text(format!("v{}", a.version)).size(10.0));
        }
        title = title.push(Space::new().width(Length::Fill));

        // Why it cannot be used, if it cannot - the missing program, or a game
        // it does not apply to. Said on the row rather than only on failure.
        let blocked = a.unavailable().or_else(|| {
            game_id
                .as_deref()
                .filter(|g| !a.applies_to(g))
                .map(|_| "not for this game".to_string())
        });
        match (&blocked, a.kind) {
            (Some(why), _) => title = title.push(text(why.clone()).size(10.0)),
            (None, AddonKind::Tool) => {
                title = title.push(
                    button(text("Run").size(11.0))
                        .padding([3, 10])
                        .style(button::primary)
                        .on_press(Message::RunAddon(a.id.clone())),
                );
            }
            (None, AddonKind::Diagnose) => {
                title = title.push(text("runs on refresh").size(10.0));
            }
        }

        let mut col = Column::new().spacing(2).push(title);
        if !a.description.is_empty() {
            col = col.push(text(a.description.clone()).size(11.0));
        }
        let by = if a.author.is_empty() {
            a.source.display().to_string()
        } else {
            format!("{} - {}", a.author, a.source.display())
        };
        col = col.push(text(by).size(9.5));
        rows = rows.push(col);
    }

    let card = Column::new()
        .spacing(10)
        .push(header)
        .push(scrollable(rows).height(Length::Fixed(360.0)).width(Length::Fill))
        .push(
            text(
                "An extension is a manifest and a program Eidos runs - nothing is loaded into \
                 Eidos itself. A 'tool' gets a Run button; a 'check' runs on every refresh and \
                 its output appears in Health, under its own name.",
            )
            .size(10.0),
        );

    container(card).width(Length::Fixed(680.0)).padding(18).style(card_style).into()
}

/// The Export dialog (MO2's Export to csv): which rows, which columns.
pub(crate) fn export_dialog<'a>(app: &App, state: &ExportDialogState) -> Element<'a, Message> {
    // Aliased: `Column` is also iced's vertical layout, used three lines down.
    use eidos_instance::Column as Col;

    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text("Export the mod list").size(18.0).width(Length::Fill))
        .push(
            button(text("Close").size(12.0))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseExportDialog),
        );

    // How many rows each scope would actually write, said up front - the whole
    // reason to offer a scope is that the numbers differ.
    let total = app.mods.iter().filter(|m| !m.is_separator()).count();
    let active = app.mods.iter().filter(|m| !m.is_separator() && m.enabled).count();
    let scope = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text("Rows").size(11.0))
        .push(
            button(text(format!("All ({total})")).size(12.0))
                .padding([3, 10])
                .style(if state.scope == ExportScope::All { button::primary } else { button::secondary })
                .on_press(Message::ExportScopeChanged(ExportScope::All)),
        )
        .push(
            button(text(format!("Enabled only ({active})")).size(12.0))
                .padding([3, 10])
                .style(if state.scope == ExportScope::Active {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::ExportScopeChanged(ExportScope::Active)),
        );

    let mut cols = Col::ALL.iter().zip(&state.columns).enumerate().fold(
        Column::new().spacing(2),
        |col, (i, (c, on))| {
            // A column Eidos has no source for is offered but labelled, because
            // MO2 emits it too - the shape has to match for a parser written
            // against MO2's output - and an always-blank column with no
            // explanation reads as a bug.
            let label = c.label().to_string();
            col.push(
                button(
                    Row::new()
                        .spacing(6)
                        .push(text(if *on { "[x]" } else { "[ ]" }).size(11.0).font(iced::Font::MONOSPACE))
                        .push(text(label).size(12.0)),
                )
                .width(Length::Fill)
                .padding([2, 6])
                .style(button::text)
                .on_press(Message::ExportToggleColumn(i)),
            )
        },
    );
    if state.picked().is_empty() {
        cols = cols.push(text("Tick at least one column.").size(11.0).color(CONFLICT_LOSES_FG));
    }

    let run = button(text("Export...").size(12.0))
        .padding([4, 14])
        .style(button::primary);
    let footer = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(
            text(
                "MO2's own CSV format: CRLF line endings, every string quoted, the Nexus id bare - \
                 so a sheet or a script written for MO2 reads it unchanged.",
            )
            .size(10.0)
            .width(Length::Fill),
        )
        .push(if state.picked().is_empty() { run } else { run.on_press(Message::ExportRun) });

    let card = Column::new()
        .spacing(12)
        .push(header)
        .push(scope)
        .push(scrollable(cols).height(Length::Fixed(300.0)))
        .push(footer);

    container(card).width(Length::Fixed(560.0)).padding(18).style(card_style).into()
}

/// The instance manager (MO2's Manage Instances).
///
/// It offers Open, Rename and Forget - and deliberately NOT Delete. An instance
/// holds a mod pool that routinely runs to hundreds of gigabytes, and no button
/// behind one confirmation is going to remove that. Forget stops listing it and
/// says so; the folder button is there for anyone who really does want it gone.
pub(crate) fn instances_dialog<'a>(app: &App) -> Element<'a, Message> {
    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text("Instances").size(18.0).width(Length::Fill))
        .push(
            button(text("Close").size(12.0))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseInstanceManager),
        );

    let mut rows = Column::new().spacing(6);
    if app.known.is_empty() {
        rows = rows.push(text("No instances found yet.").size(12.0));
    }
    for (i, k) in app.known.iter().enumerate() {
        // Rename takes over the row while it is armed, exactly as the profile
        // menu's does - there is nowhere better to put a one-field editor.
        if let Some((ri, typed)) = &app.instance_rename {
            if *ri == i {
                rows = rows.push(
                    Row::new()
                        .spacing(6)
                        .align_y(iced::Alignment::Center)
                        .push(
                            text_input("Folder name", typed)
                                .on_input(Message::InstanceRenameChanged)
                                .on_submit(Message::InstanceRenameCommit)
                                .padding(5)
                                .size(12.0),
                        )
                        .push(
                            button(text("Save").size(11.0))
                                .padding([3, 10])
                                .style(button::primary)
                                .on_press(Message::InstanceRenameCommit),
                        ),
                );
                continue;
            }
        }

        let open_now = app.created.as_ref().is_some_and(|c| c.root == k.inst.root);
        let mut title = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text(k.label.clone()).size(12.0).width(Length::Fill));
        if open_now {
            title = title.push(text("open").size(10.0));
        } else {
            title = title.push(
                button(text("Open").size(11.0))
                    .padding([3, 10])
                    .style(button::primary)
                    .on_press(Message::InstanceOpen(i)),
            );
        }
        // Rename and Forget are portable-only: a global instance lives at a path
        // derived from the game id, so it has no folder to rename and nothing in
        // the registry to drop.
        if k.portable && !open_now {
            title = title.push(
                button(text("Rename").size(11.0))
                    .padding([3, 8])
                    .style(button::secondary)
                    .on_press(Message::InstanceRenameStart(i)),
            );
            let armed = app.confirm_forget == Some(i);
            title = title.push(
                button(text(if armed { "Confirm?" } else { "Forget" }).size(11.0))
                    .padding([3, 8])
                    .style(if armed { button::danger } else { button::secondary })
                    .on_press(Message::InstanceForget(i)),
            );
        }
        title = title.push(
            button(text("Folder").size(11.0))
                .padding([3, 8])
                .style(button::text)
                .on_press(Message::OpenFolder(k.inst.root.clone())),
        );
        rows = rows.push(title);
    }

    let card = Column::new()
        .spacing(12)
        .push(header)
        .push(scrollable(rows).height(Length::Fixed(300.0)))
        .push(
            text(
                "Forget removes an instance from this list and touches nothing on disk. There is \
                 no Delete: an instance holds your whole mod pool, and that is not something to \
                 lose to one confirmation - use Folder and delete it yourself if you mean it.",
            )
            .size(10.0),
        );

    container(card).width(Length::Fixed(680.0)).padding(18).style(card_style).into()
}

/// The collection browser.
///
/// It lists what a published collection contains and what you already have. It
/// deliberately does not INSTALL one, and the card says so rather than leaving
/// the user to discover it: a collection's members are ordinary mod files, and
/// without a per-file key from the site's own button only a premium account can
/// fetch them - so a progress bar here would stall on the first mod for most
/// people. What it can do exactly, it does exactly.
/// The preview pane: one file, shown as far as it can be.
pub(crate) fn preview_dialog<'a>(p: &Preview) -> Element<'a, Message> {
    let name = p.path().file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(name).size(15.0).width(Length::Fill))
        .push(
            button(text("Reveal").size(11.0))
                .padding([4, 10])
                .style(button::secondary)
                .on_press(Message::DataReveal(p.path().to_path_buf())),
        )
        .push(
            button(text("Close").size(12.0))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::ClosePreview),
        );
    let body: Element<'a, Message> = match p {
        Preview::Image { handle, .. } => container(
            iced::widget::image(handle.clone()).content_fit(iced::ContentFit::Contain),
        )
        .center(Length::Fill)
        .into(),
        Preview::Text { body, truncated, .. } => {
            let mut col = Column::new().spacing(4).push(
                // Monospaced, because everything that reaches here - an INI, a
                // log, a Papyrus source - is written in columns.
                text(body.clone()).size(11.0).font(iced::Font::MONOSPACE),
            );
            if *truncated {
                col = col.push(
                    text(format!(
                        "Showing the first {} KB. The rest is there, just not here.",
                        PREVIEW_TEXT_CAP / 1024
                    ))
                    .size(10.0),
                );
            }
            scrollable(col).height(Length::Fill).into()
        }
        Preview::Unsupported { why, .. } => {
            container(text(why.clone()).size(12.0)).center(Length::Fill).into()
        }
    };
    container(Column::new().spacing(10).push(header).push(body))
        .width(Length::Fixed(760.0))
        .height(Length::Fixed(560.0))
        .padding(18)
        .style(card_style)
        .into()
}

pub(crate) fn collection_dialog<'a>(state: &CollectionState) -> Element<'a, Message> {
    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text("Nexus collection").size(18.0).width(Length::Fill))
        .push(
            button(text("Close").size(12.0))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseCollection),
        );

    let field = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(
            text_input("nxm://<game>/collections/<slug>/revisions/latest", &state.link)
                .on_input(Message::CollectionLinkChanged)
                .on_submit(Message::CollectionFetch)
                .padding(5)
                .size(12.0),
        )
        .push(
            button(text(if state.loading { "..." } else { "Look up" }).size(12.0))
                .padding([4, 12])
                .style(button::primary)
                .on_press_maybe((!state.loading).then_some(Message::CollectionFetch)),
        );

    let mut card = Column::new().spacing(10).push(header).push(field);

    if let Some(e) = &state.error {
        card = card.push(text(e.clone()).size(12.0).color(CONFLICT_LOSES_FG));
    }

    if let Some(rev) = &state.revision {
        if !rev.visible() {
            // The gate answered. Say which rule, not "no".
            let why = rev.hidden.map(|h| h.message()).unwrap_or("");
            card = card.push(text(why.to_string()).size(12.0));
            return container(card).width(Length::Fixed(720.0)).padding(18).style(card_style).into();
        }

        let installed = state.states.iter().filter(|s| **s == MemberState::Installed).count();
        let downloaded = state.states.iter().filter(|s| **s == MemberState::Downloaded).count();
        let missing = state.states.iter().filter(|s| **s == MemberState::Missing).count();

        let mut title = Column::new()
            .spacing(2)
            .push(text(format!("{}  ·  revision {}", rev.name, rev.revision_number)).size(15.0));
        if !rev.author.is_empty() {
            title = title.push(text(format!("by {}", rev.author)).size(11.0));
        }
        if !rev.summary.is_empty() {
            title = title.push(text(rev.summary.clone()).size(11.0));
        }
        card = card.push(title);

        let mut summary = Row::new().spacing(10).align_y(iced::Alignment::Center).push(
            text(format!("{installed} installed  ·  {downloaded} downloaded  ·  {missing} missing"))
                .size(12.0)
                .width(Length::Fill),
        );
        if missing > 0 {
            let label = if state.confirm_fetch {
                "Click again to start them".to_string()
            } else {
                format!("Try to fetch {missing} missing")
            };
            summary = summary.push(
                button(text(label).size(11.0))
                    .padding([3, 10])
                    .style(if state.confirm_fetch { button::danger } else { button::secondary })
                    .on_press(Message::CollectionFetchMissing),
            );
        }
        card = card.push(summary);

        // The collection author's own notes. Worth showing precisely because
        // Eidos is not applying them: they are the part a person still has to do.
        if !rev.instructions.trim().is_empty() {
            card = card.push(
                container(
                    Column::new()
                        .spacing(3)
                        .push(text("The collection's own instructions").size(10.0))
                        .push(text(rev.instructions.clone()).size(11.0)),
                )
                .padding(8)
                .style(card_style),
            );
        }

        let mut rows = Column::new().spacing(1);
        for (i, (m, st)) in rev.mods.iter().zip(&state.states).enumerate() {
            let (label, colour) = match st {
                MemberState::Installed => ("installed", Some(CONFLICT_WINS_FG)),
                MemberState::Downloaded => ("downloaded", None),
                MemberState::Missing => ("missing", Some(CONFLICT_LOSES_FG)),
            };
            let mut status = text(label.to_string()).size(11.0).width(Length::Fixed(84.0));
            if let Some(c) = colour {
                status = status.color(c);
            }
            let name = if m.optional {
                format!("{}  (optional)", m.name)
            } else {
                m.name.clone()
            };
            let row = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text(name).size(12.0).width(Length::Fill))
                .push(text(m.version.clone()).size(11.0).width(Length::Fixed(70.0)))
                .push(status)
                .push(
                    button(text("Open").size(10.0))
                        .padding([2, 8])
                        .style(button::text)
                        .on_press(Message::CollectionOpenMod(i)),
                );
            rows = rows.push(striped(container(row).padding(3).into(), i % 2 == 0));
        }
        // The API's own count, said separately: a member whose mod has since
        // been deleted comes back with nothing to show, so the two numbers
        // disagreeing is information rather than an error.
        if rev.mods.len() < rev.mod_count as usize {
            rows = rows.push(
                text(format!(
                    "{} of the {} members are no longer on Nexus and cannot be listed.",
                    rev.mod_count as usize - rev.mods.len(),
                    rev.mod_count
                ))
                .size(11.0),
            );
        }
        card = card.push(scrollable(rows).height(Length::Fixed(320.0)));
    }

    card = card.push(
        text(
            "Eidos reads a collection; it does not install one. Its mods are ordinary Nexus \
             files, so without the site's own download button only a premium account can fetch \
             them - and the load order rules, FOMOD answers and patches a collection carries are \
             not applied here. Open takes you to the exact file the collection pins.",
        )
        .size(10.0),
    );

    container(card).width(Length::Fixed(720.0)).padding(18).style(card_style).into()
}
