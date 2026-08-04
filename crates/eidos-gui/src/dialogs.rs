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

    let body: Element<'a, Message> = match app.settings_tab {
        SettingsTab::Nexus => {
            // The validate/connect button greys out while a check is in flight.
            let connect_label = if app.api_key_validating { "Checking..." } else { "Validate & Save" };
            let mut connect = button(text(connect_label).size(12.0)).padding([5, 12]).style(button::primary);
            if !app.api_key_validating {
                connect = connect.on_press(Message::ApiKeyValidateStart);
            }
            // Masked. It is a credential, and this field sits in a window users
            // screenshot to ask for help - which is one of the ways a key leaks.
            // Nothing is lost by hiding it: validation names the account back, so
            // the user still gets told whether what they pasted was right.
            let field = text_input("Personal API key", &app.settings_api_key)
                .secure(true)
                .on_input(Message::ApiKeyChanged)
                .on_submit(Message::ApiKeyValidateStart)
                .padding(6)
                .size(12.0)
                .width(Length::Fill);

            let mut col = Column::new()
                .spacing(8)
                .push(text("Personal Nexus Mods API key").size(13.0))
                .push(
                    text("Get it from nexusmods.com -> Account -> API Keys (Personal API Key). It is stored at ~/.config/eidos/nexus.ini and shared with the CLI.")
                        .size(10.0),
                )
                .push(Row::new().spacing(8).push(field).push(connect));

            if let Some(account) = &app.nexus_account {
                let tier = if account.is_premium { "Premium" } else { "free" };
                col = col.push(text(format!("Connected as {} ({tier}).", account.name)).size(11.0));
            }
            if let Some(err) = &app.api_key_error {
                col = col.push(text(format!("Error: {err}")).size(11.0).color(Color::from_rgb8(0x8A, 0x2A, 0x2A)));
            }
            col.into()
        }
        SettingsTab::General => {
            // Default-game dropdown: "(none)" plus every supported game.
            let mut games = vec![DefaultGameChoice { id: None, label: "(none)".to_string() }];
            for g in eidos_games::catalog() {
                games.push(DefaultGameChoice { id: Some(g.id.to_string()), label: g.name.to_string() });
            }
            let selected_game = games
                .iter()
                .find(|c| c.id == app.prefs.default_game)
                .cloned()
                .unwrap_or_else(|| DefaultGameChoice { id: None, label: "(none)".to_string() });
            let game_row = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text("Default game").size(12.0).width(Length::Fixed(120.0)))
                .push(
                    pick_list(games, Some(selected_game), |c: DefaultGameChoice| {
                        Message::DefaultGameChanged(c.id)
                    })
                    .text_size(12.0)
                    .padding(6),
                );

            // MO2's "lock GUI while an executable runs" toggle.
            let lock_row = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text("Run behaviour").size(12.0).width(Length::Fixed(120.0)))
                .push(
                    checkbox(app.prefs.lock_gui).label("Lock the window while a game or tool is running")
                        .on_toggle(Message::ToggleLockGui)
                        .size(16)
                        .text_size(12.0),
                );

            Column::new()
                .spacing(10)
                .push(game_row)
                .push(lock_row)
                .push(text("Saved to ~/.config/eidos/settings.ini.").size(10.0))
                .into()
        }
        SettingsTab::Appearance => {
            let themes = vec![
                ThemeChoice(PrefTheme::System),
                ThemeChoice(PrefTheme::Light),
                ThemeChoice(PrefTheme::Dark),
            ];
            let theme_row = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text("Theme").size(12.0).width(Length::Fixed(120.0)))
                .push(
                    pick_list(themes, Some(ThemeChoice(app.prefs.theme)), |c: ThemeChoice| {
                        Message::ThemeChanged(c.0)
                    })
                    .text_size(12.0)
                    .padding(6),
                );

            Column::new()
                .spacing(10)
                .push(theme_row)
                .push(text("System follows your desktop's light/dark preference.").size(10.0))
                .into()
        }
        SettingsTab::ModList => {
            // The one knob worth exposing here: it was tuned twice by hand
            // against a 250-mod list, which is exactly the sign that the right
            // value depends on the list rather than on the program.
            let speed = app.prefs.drag_scroll_speed;
            let row = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text("Drag scrolling").size(12.0).width(Length::Fixed(120.0)))
                .push(
                    slider(0.25..=4.0, speed, Message::DragScrollSpeedChanged)
                        .step(0.25)
                        .width(Length::Fixed(200.0)),
                )
                .push(text(format!("{speed:.2}x")).size(12.0));
            Column::new()
                .spacing(10)
                .push(row)
                .push(
                    text(
                        "How fast the list scrolls when you drag a mod to its top or bottom edge. \
                         Speed already rises the closer to the edge you push; this scales the whole range.",
                    )
                    .size(10.0),
                )
                .into()
        }
        SettingsTab::About => {
            let line = |k: &'static str, v: String| {
                Row::new()
                    .spacing(8)
                    .push(text(k).size(11.0).width(Length::Fixed(120.0)))
                    .push(text(v).size(11.0))
            };
            let instance = app
                .created
                .as_ref()
                .map(|i| i.root.display().to_string())
                .unwrap_or_else(|| "(none open)".to_string());
            Column::new()
                .spacing(6)
                .push(text("Eidos").size(15.0))
                .push(text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(12.0))
                .push(Space::new().height(Length::Fixed(4.0)))
                .push(line("Instance", instance))
                .push(line(
                    "Settings",
                    eidos_instance::settings::settings_path().display().to_string(),
                ))
                .push(line(
                    "Nexus",
                    eidos_instance::settings::nexus_key_path().display().to_string(),
                ))
                .push(line("Games", "~/.config/eidos/games/*.toml".to_string()))
                .push(Space::new().height(Length::Fixed(4.0)))
                .push(
                    text("Ctrl+R run   ·   F5 refresh   ·   Ctrl+click multi-select   ·   Shift+click range   ·   Esc clear")
                        .size(10.0),
                )
                .into()
        }
    };

    let panes = Row::new()
        .spacing(14)
        .push(rail)
        .push(container(scrollable(body)).width(Length::Fill).height(Length::Fixed(240.0)));
    let card = Column::new().spacing(12).push(header).push(panes);
    container(card).max_width(620.0).padding(16).style(card_style).into()
}

// ---- Executables editor (MO2's Modify Executables) --------------------------

pub(crate) fn executables_dialog<'a>(app: &App, state: &ExecutablesDialogState) -> Element<'a, Message> {
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
            .push(text("Arguments (one per line)").size(11.0))
            .push(
                text_input("", &state.args)
                    .on_input(Message::ToolArgsChanged)
                    .padding(6)
                    .size(12.0)
                    .width(Length::Fill),
            )
            .push(exe_field("Prereqs (comma-separated)", &state.prereqs, Message::ToolPrereqsChanged))
            .push(prereq_status_rows(app, &state.prereqs))
            .into()
    } else if state.selected.is_some() {
        Column::new()
            .spacing(8)
            .push(text("This is a per-game default and cannot be edited.").size(12.0))
            .push(text("Add a user tool with the same title to override it.").size(10.0))
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
