//! The modal dialogs that are not the mod info one: the executables editor, the
//! about box, the LOOT sort report, and the cards that report a running game or
//! a missing capability. Preferences has its own module, `preferences`.
//!
//! Split out of `main.rs` unchanged.

use iced::widget::slider;

use crate::theme::*;
use crate::widgets::*;
use crate::*;

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
            .push(text(title).size(sz(13)))
            .push(text(what).size(sz(11)))
            .push(
                button(text(t("backups_back_up_now")).size(sz(12)))
                    .padding([4, 10])
                    .style(button::primary)
                    .on_press(Message::CreateBackup(kind)),
            );
        if list.is_empty() {
            col = col.push(text(t("backups_none_yet")).size(sz(11)));
        } else {
            let mut rows = Column::new().spacing(3);
            for b in list {
                rows = rows.push(
                    Row::new()
                        .spacing(8)
                        .align_y(iced::Alignment::Center)
                        .push(text(b.when()).size(sz(12)).width(Length::Fill))
                        .push(
                            button(text(t("backups_restore")).size(sz(11)))
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
            t("backups_mod_list"),
            t("backups_mod_list_desc"),
            BackupKind::ModList,
            &state.mods,
        ))
        .push(column(
            t("backups_load_order"),
            t("backups_load_order_desc"),
            BackupKind::LoadOrder,
            &state.order,
        ));

    let card = Column::new()
        .spacing(12)
        .push(
            Row::new()
                .align_y(iced::Alignment::Center)
                .push(text(t("backups_title")).size(sz(18)).width(Length::Fill))
                .push(
                    button(text(t("common_close")).size(sz(12)))
                        .padding([4, 12])
                        .style(button::secondary)
                        .on_press(Message::CloseBackupsDialog),
                ),
        )
        .push(body)
        .push(
            text(t("backups_restore_note"))
                .size(sz(11)),
        );

    container(card)
        .width(Length::Fixed(620.0))
        .padding(18)
        .style(card_style)
        .into()
}

pub(crate) fn executables_dialog<'a>(
    app: &App,
    state: &'a ExecutablesDialogState,
) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(6)
        .push(text(t("exe_title")).size(sz(16)).width(Length::Fill))
        .push(
            button(text("x").size(sz(14)))
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
            list = list.push(text(crate::i18n::t("exe_defaults")).size(sz(10)));
        }
        let selected = state.selected == Some(i);
        let label = if t.title.trim().is_empty() {
            crate::i18n::t("exe_unnamed")
        } else {
            t.title.trim()
        };
        let is_default = i >= state.user_len;
        let display = if is_default {
            crate::i18n::tf("exe_default_suffix", &[("name", &label)])
        } else {
            label.to_string()
        };
        list = list.push(
            button(text(display).size(sz(12)))
                .width(Length::Fill)
                .padding([3, 6])
                .on_press(Message::SelectExecutableTool(i))
                .style(if selected {
                    button::primary
                } else {
                    button::text
                }),
        );
    }
    if state.merged.is_empty() {
        list = list.push(text(t("exe_none_yet")).size(sz(11)));
    }
    let list_pane = container(scrollable(list).height(Length::Fill))
        .width(Length::Fixed(scaled(200.0)))
        .height(Length::Fixed(280.0))
        .padding(4)
        .style(panel_style);

    let list_actions = Row::new()
        .spacing(4)
        .push(tool_btn(t("exe_add"), Message::AddExecutableTool))
        .push(del_button(state))
        .push(move_button(
            t("exe_up"),
            Message::MoveExecutableUp,
            can_move_up(state),
        ))
        .push(move_button(
            t("exe_down"),
            Message::MoveExecutableDown,
            can_move_down(state),
        ));

    let left = Column::new().spacing(6).push(list_pane).push(list_actions);

    // The editor pane (only meaningful for a selected user tool).
    let editor: Element<'a, Message> = if state.selected_is_user() {
        Column::new()
            .spacing(8)
            .push(exe_field(t("exe_field_title"), &state.title, Message::ToolTitleChanged))
            .push(exe_field_browse(
                t("exe_field_path"),
                &state.exe,
                Message::ToolExeChanged,
                Message::BrowseToolExe,
            ))
            .push(exe_field_browse(
                t("exe_field_workdir"),
                &state.workdir,
                Message::ToolWorkdirChanged,
                Message::BrowseToolWorkdir,
            ))
            // One argument per line, and the widget has to be able to HOLD a
            // newline: a `text_input` drops them, so the label promised
            // something the field refused, and a tool needing two arguments
            // could not be created through this dialog at all.
            .push(text(t("exe_field_args")).size(sz(11)))
            .push(
                iced::widget::text_editor::TextEditor::new(&state.args_editor)
                    .on_action(Message::ToolArgsAction)
                    .padding(6)
                    .height(Length::Fixed(72.0)),
            )
            .push(exe_field(
                t("exe_field_prereqs"),
                &state.prereqs,
                Message::ToolPrereqsChanged,
            ))
            .push(prereq_status_rows(app, &state.prereqs))
            .push(output_mod_field(state))
            .push(exe_field(
                t("exe_field_appid"),
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
            .push(text(t("exe_default_readonly")).size(sz(12)))
            .push(text(t("exe_default_override")).size(sz(10)))
            .push(tool_flags_row(state))
            .into()
    } else {
        Column::new()
            .spacing(8)
            .push(text(t("exe_select_hint")).size(sz(12)))
            .into()
    };
    let right = container(editor).width(Length::Fill).padding(4);

    let panes = Row::new().spacing(12).push(left).push(right);

    let footer = Row::new()
        .spacing(8)
        .push(Space::new().width(Length::Fill))
        .push(tool_btn(t("common_cancel"), Message::CloseExecutablesDialog))
        .push(
            button(text(t("common_save")).size(sz(12)))
                .padding([5, 14])
                .on_press(Message::SaveExecutablesDialog)
                .style(button::primary),
        );

    let card = Column::new()
        .spacing(12)
        .push(header)
        .push(panes)
        .push(footer);
    container(card)
        .max_width(720.0)
        .padding(16)
        .style(card_style)
        .into()
}

/// A labelled single-line field for the Executables editor.
pub(crate) fn exe_field<'a>(
    label: &'a str,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    Column::new()
        .spacing(2)
        .push(text(label).size(sz(11)))
        .push(
            text_input("", value)
                .on_input(on_input)
                .padding(6)
                .size(sz(12))
                .width(Length::Fill),
        )
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
        .push(
            text_input("", value)
                .on_input(on_input)
                .padding(6)
                .size(sz(12))
                .width(Length::Fill),
        )
        .push(
            button(text(t("common_browse_ellipsis")).size(sz(11)))
                .padding([5, 8])
                .on_press(browse)
                .style(button::secondary),
        );
    Column::new()
        .spacing(2)
        .push(text(label).size(sz(11)))
        .push(row)
        .into()
}

/// The Delete button: active only when a user tool is selected.
pub(crate) fn del_button<'a>(state: &ExecutablesDialogState) -> Element<'a, Message> {
    let mut b = button(text(t("common_delete")).size(sz(12)))
        .padding(6)
        .style(button::danger);
    if state.selected_is_user() {
        b = b.on_press(Message::DeleteExecutableTool);
    }
    b.into()
}

/// A reorder button, greyed when the move is not possible.
pub(crate) fn move_button<'a>(label: &'a str, msg: Message, enabled: bool) -> Element<'a, Message> {
    let mut b = button(text(label).size(sz(12)))
        .padding(6)
        .style(button::secondary);
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

/// One prerequisite's state, as a label and whether it needs the user to act.
///
/// Pure so the wording and the classification can be tested: a verb reported as
/// present when it is not sends the user to look for the fault in their mods.
fn prereq_state(verb: &str, done: &std::collections::HashSet<String>) -> (&'static str, bool) {
    if eidos_gamefeatures::is_runtime_verb(verb) {
        let ok =
            eidos_gamefeatures::runtime(verb).is_some_and(eidos_gamefeatures::runtime_is_installed);
        if ok {
            (t("prereq_downloaded"), false)
        } else {
            (t("prereq_missing_download"), true)
        }
    } else if eidos_gamefeatures::is_tier2_verb(verb) {
        if done.contains(verb) {
            (t("prereq_installed"), false)
        } else {
            (t("prereq_missing_install"), true)
        }
    } else {
        // Neither a runtime nor a winetricks verb: almost always a
        // typo. Saying so beats a tool that fails later for a reason that names
        // nothing the user recognises. Not offered as a click - there is nothing
        // to install.
        (t("prereq_unknown"), false)
    }
}

/// What Eidos can say about each prerequisite a tool declares, and a way to fix
/// the ones it can.
///
/// The field above accepts any text, so this is the only place that answers the
/// question the user actually has - "will this tool start?". A verb that is
/// merely typed is not a verb that is present, and the difference used to be
/// invisible until the tool failed with an error naming neither Eidos nor the
/// missing runtime.
pub(crate) fn prereq_status_rows<'a>(app: &App, prereqs: &str) -> Element<'a, Message> {
    let verbs: Vec<String> = prereqs
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if verbs.is_empty() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }

    let prefix = selected_game(app).and_then(|g| g.prefix());
    let done: std::collections::HashSet<String> = app
        .created
        .as_ref()
        .map(|i| {
            eidos_gamefeatures::satisfied_prereqs_in(&i.root, prefix.as_deref())
                .into_iter()
                .collect()
        })
        .unwrap_or_default();

    let mut col = Column::new()
        .spacing(2)
        .push(text(t("prereq_status")).size(sz(11)).color(fomod_ink_faint()));
    let mut any_missing = false;
    for v in verbs {
        let (label, missing) = prereq_state(&v, &done);
        any_missing |= missing;

        let row = Row::new()
            .spacing(8)
            .push(text(v.clone()).size(sz(11)).width(Length::Fixed(scaled(150.0))))
            .push(text(label).size(sz(11)).color(if missing {
                pal().error
            } else {
                fomod_ink_soft()
            }));
        col = if missing {
            col.push(
                button(row)
                    .padding(2)
                    .on_press(Message::SetupPrereqs)
                    .style(button::text),
            )
        } else {
            col.push(row)
        };
    }
    if any_missing {
        col = col.push(
            text(t("prereq_downloads_note"))
                .size(sz(10))
                .color(fomod_ink_faint()),
        );
    }
    col.into()
}

// ---- About box --------------------------------------------------------------

pub(crate) fn about_dialog<'a>() -> Element<'a, Message> {
    let card = Column::new()
        .spacing(8)
        .push(text("Eidos").size(sz(20)))
        .push(text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(sz(12)))
        .push(
            text(t("about_blurb"))
                .size(sz(12)),
        )
        .push(Space::new().height(Length::Fixed(6.0)))
        .push(text(t("about_shortcuts")).size(sz(13)))
        .push(
            text(t("about_shortcut_list"))
                .size(sz(11)),
        )
        .push(Space::new().height(Length::Fixed(6.0)))
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([5, 14])
                .on_press(Message::CloseAbout)
                .style(button::primary),
        );
    container(card)
        .max_width(440.0)
        .padding(16)
        .style(card_style)
        .into()
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
    let cmd = setcap_hint();
    let row = Row::new()
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .push(text(t("passthrough_missing_cap")).size(sz(11)))
        .push(
            container(text(cmd).size(sz(11)))
                .padding([2, 8])
                .style(|_| container::Style {
                    background: Some(Background::Color(pal().bg_card)),
                    border: Border {
                        color: pal().warning,
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..Default::default()
                }),
        )
        .push(Space::new().width(Length::Fill))
        .push(flat_btn(t("passthrough_recheck"), Message::Refresh));
    container(row)
        .width(Length::Fill)
        .padding([4, 8])
        .style(|_| container::Style {
            background: Some(Background::Color(pal().warning_bg)),
            border: Border {
                color: pal().warning,
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: Some(pal().warning),
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
        .push(text(t("lock_title")).size(sz(18)))
        .push(text(tf("lock_running", &[("title", &run.title), ("pid", &run.pid)])).size(sz(13)))
        .push(
            text(t("lock_body"))
                .size(sz(11)),
        )
        .push(Space::new().height(Length::Fixed(6.0)))
        .push(
            button(text(t("lock_unlock")).size(sz(13)))
                .padding([6, 22])
                .on_press(Message::ForceUnlock)
                .style(button::primary),
        )
        .push(
            text(t("lock_unlock_note"))
                .size(sz(10))
                .color(text_muted()),
        );
    container(card)
        .max_width(470.0)
        .padding(20)
        .style(card_style)
        .into()
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
        MessageType::Error => (t("loot_prefix_error"), pal().error),
        MessageType::Warn => (t("loot_prefix_warning"), pal().warning),
        MessageType::Say => ("", pal().text_secondary),
    };
    let parts = split_markdown_links(&m.text);
    if parts.iter().all(|(_, url)| url.is_none()) {
        return text(format!("{prefix}{}", m.text))
            .size(sz(11))
            .color(color)
            .into();
    }
    let mut row = Row::new().spacing(0).align_y(iced::Alignment::Center);
    if !prefix.is_empty() {
        row = row.push(text(prefix).size(sz(11)).color(color));
    }
    for (label, url) in parts {
        row = match url {
            Some(u) => row.push(
                button(text(label).size(sz(11)).color(accent()))
                    .padding(0)
                    .on_press(Message::OpenUrl(u))
                    .style(button::text),
            ),
            None => row.push(text(label).size(sz(11)).color(color)),
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
        t("loot_clean").to_string()
    } else {
        let mut parts: Vec<String> = Vec::new();
        if report.error_count() > 0 {
            parts.push(tf("loot_n_errors", &[("n", &report.error_count())]));
        }
        if report.warning_count() > 0 {
            parts.push(tf("loot_n_warnings", &[("n", &report.warning_count())]));
        }
        if report.missing_master_count() > 0 {
            parts.push(tf(
                "loot_n_missing_masters",
                &[("n", &report.missing_master_count())],
            ));
        }
        if report.dirty_count() > 0 {
            parts.push(tf("loot_n_dirty", &[("n", &report.dirty_count())]));
        }
        if parts.is_empty() {
            t("loot_messages").to_string()
        } else {
            parts.join(", ")
        }
    };

    let mut body = Column::new().spacing(12);

    if !report.general.is_empty() {
        let mut sec = Column::new()
            .spacing(3)
            .push(text(t("loot_general")).size(sz(14)));
        for m in &report.general {
            sec = sec.push(loot_message_row(m));
        }
        body = body.push(sec);
    }

    for p in &report.plugins {
        let mut sec = Column::new()
            .spacing(2)
            .push(text(p.name.clone()).size(sz(13)));
        if !p.missing_masters.is_empty() {
            sec = sec.push(
                text(tf("loot_missing_masters", &[("list", &p.missing_masters.join(", "))]))
                    .size(sz(11))
                    .color(pal().error),
            );
        }
        for m in &p.messages {
            sec = sec.push(loot_message_row(m));
        }
        for d in &p.dirty {
            let util = if d.cleaning_utility.is_empty() {
                "?"
            } else {
                d.cleaning_utility.as_str()
            };
            sec = sec.push(
                text(tf(
                    "loot_dirty",
                    &[
                        ("util", &util),
                        ("itm", &d.itm_count),
                        ("refs", &d.deleted_reference_count),
                        ("navmeshes", &d.deleted_navmesh_count),
                    ],
                ))
                .size(sz(11))
                .color(pal().warning),
            );
        }
        body = body.push(sec);
    }

    let card = Column::new()
        .spacing(10)
        .push(text(t("loot_report")).size(sz(20)))
        .push(text(summary).size(sz(12)))
        .push(scrollable(body).height(Length::Fixed(360.0)))
        .push(
            Row::new()
                .spacing(8)
                .push(
                    button(text(t("common_close")).size(sz(12)))
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
                    button(text(t("loot_copy_report")).size(sz(12)))
                        .padding([5, 14])
                        .on_press(Message::CopyLootReport)
                        .style(button::secondary),
                ),
        );
    container(card)
        .max_width(580.0)
        .padding(16)
        .style(card_style)
        .into()
}

/// The report as plain text, for the clipboard.
///
/// Deliberately not the on-screen layout: colour carries the severity there, and
/// a paste into a text editor would lose it silently. Each line says what it is.
pub(crate) fn loot_report_text(report: &eidos_loot::LootReport) -> String {
    let mut out = format!("{}\n", t("loot_report"));
    if report.is_empty() {
        out.push_str(&format!("\n{}\n", t("loot_report_clean")));
        return out;
    }
    if !report.general.is_empty() {
        out.push_str(&format!("\n{}\n", t("loot_general")));
        for m in &report.general {
            out.push_str(&format!("  [{}] {}\n", loot_severity_label(m.kind), m.text));
        }
    }
    for p in &report.plugins {
        out.push_str(&format!("\n{}\n", p.name));
        if !p.missing_masters.is_empty() {
            out.push_str(&format!(
                "  {}\n",
                tf("loot_missing_masters", &[("list", &p.missing_masters.join(", "))])
            ));
        }
        for m in &p.messages {
            out.push_str(&format!("  [{}] {}\n", loot_severity_label(m.kind), m.text));
        }
        for d in &p.dirty {
            let util = if d.cleaning_utility.is_empty() {
                "?"
            } else {
                d.cleaning_utility.as_str()
            };
            let line = tf(
                "loot_dirty",
                &[
                    ("util", &util),
                    ("itm", &d.itm_count),
                    ("refs", &d.deleted_reference_count),
                    ("navmeshes", &d.deleted_navmesh_count),
                ],
            );
            out.push_str(&format!("  {line}\n"));
        }
    }
    out
}

pub(crate) fn loot_severity_label(kind: eidos_loot::MessageType) -> &'static str {
    match kind {
        eidos_loot::MessageType::Error => t("loot_sev_error"),
        eidos_loot::MessageType::Warn => t("loot_sev_warning"),
        eidos_loot::MessageType::Say => t("loot_sev_note"),
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
        tf("cat_title_one", &[("name", &state.names[0])])
    } else {
        tf("cat_title_many", &[("n", &state.names.len())])
    };

    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(title).size(sz(18)).width(Length::Fill))
        .push(
            button(
                text(if state.editing {
                    t("cat_done_editing")
                } else {
                    t("cat_edit_list")
                })
                .size(sz(12)),
            )
            .padding([4, 10])
            .style(button::secondary)
            .on_press(Message::ToggleCategoryEditor),
        )
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseCategoriesDialog),
        );

    let body = if state.editing {
        catalog_editor(state)
    } else {
        category_picker(state)
    };

    let mut foot = Row::new().spacing(8).align_y(iced::Alignment::Center);
    // What Apply will actually write, spelled out - the pending pick is invisible
    // otherwise once the tree is scrolled away from the checked rows.
    let summary = match state.chosen.split_first() {
        None => t("cat_none_clears").to_string(),
        Some((p, rest)) => {
            let primary = state.catalog.name_for_id(*p).unwrap_or("?").to_string();
            if rest.is_empty() {
                primary
            } else {
                tf("cat_primary_more", &[("name", &primary), ("n", &rest.len())])
            }
        }
    };
    foot = foot
        .push(text(summary).size(sz(12)).width(Length::Fill))
        .push(
            button(text(t("cat_apply")).size(sz(12)))
                .padding([4, 14])
                .style(button::primary)
                .on_press(Message::ApplyCategories),
        );

    let mut card = Column::new().spacing(12).push(header).push(body).push(foot);
    if state.names.len() > 1 {
        card = card.push(
            text(
                t("cat_apply_note"),
            )
            .size(sz(11)),
        );
    }

    container(card)
        .width(Length::Fixed(620.0))
        .padding(18)
        .style(card_style)
        .into()
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
                button(text(format!("{mark} {name}")).size(sz(12)))
                    .padding([2, 6])
                    .width(Length::Fill)
                    .style(if checked {
                        button::secondary
                    } else {
                        button::text
                    })
                    .on_press(Message::ToggleCategory(id)),
            );
        // Primary is what the mod list column shows, so it needs to be both
        // visible and settable without unchecking everything else first.
        if is_primary {
            row = row.push(text(t("cat_primary")).size(sz(10)));
        } else if checked {
            row = row.push(
                button(text(t("cat_make_primary")).size(sz(10)))
                    .padding([2, 6])
                    .style(button::text)
                    .on_press(Message::SetPrimaryCategory(id)),
            );
        }
        rows = rows.push(row);
    }
    if shown == 0 {
        rows = rows.push(text(t("cat_no_match")).size(sz(12)));
    }

    let search = text_input(t("cat_filter"), &state.query)
        .on_input(Message::CategoryQueryChanged)
        .padding(5)
        .size(sz(12));

    let nexus = Row::new()
        .spacing(6)
        .push(tool_btn(t("cat_fetch_nexus"), Message::FetchNexusCategories))
        .push(tool_btn(
            t("cat_use_nexus"),
            Message::AssignCategoriesFromNexus,
        ));

    Column::new()
        .spacing(8)
        .push(search)
        .push(scrollable(rows).height(Length::Fixed(320.0)))
        .push(nexus)
        .push(
            text(
                t("cat_fetch_note"),
            )
            .size(sz(10)),
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
                            text_input(t("cat_name"), pending)
                                .on_input(Message::RenameCategoryChanged)
                                .on_submit(Message::RenameCategoryCommit)
                                .padding(4)
                                .size(sz(12)),
                        )
                        .push(
                            button(text(t("common_save")).size(sz(11)))
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
                .push(text(name).size(sz(12)).width(Length::Fill))
                .push(text(format!("#{id}")).size(sz(10)))
                .push(
                    button(text(t("common_rename")).size(sz(10)))
                        .padding([2, 6])
                        .style(button::text)
                        .on_press(Message::RenameCategoryStart(id)),
                )
                .push(
                    button(text(if armed { t("common_confirm") } else { t("common_delete") }).size(sz(10)))
                        .padding([2, 6])
                        .style(if armed { button::danger } else { button::text })
                        .on_press(Message::DeleteCategory(id)),
                ),
        );
    }

    // Adding: name + a parent picked from the same tree.
    let parents: Vec<CategoryChoice> = std::iter::once(CategoryChoice {
        id: 0,
        label: t("cat_top_level").to_string(),
    })
    .chain(
        state
            .catalog
            .tree()
            .into_iter()
            .map(|(id, name, depth)| CategoryChoice {
                id,
                label: format!("{}{name}", "  ".repeat(depth)),
            }),
    )
    .collect();
    let selected = parents.iter().find(|c| c.id == state.new_parent).cloned();
    let add = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(
            text_input(t("cat_new_name"), &state.new_name)
                .on_input(Message::NewCategoryNameChanged)
                .on_submit(Message::AddCategory)
                .padding(5)
                .size(sz(12)),
        )
        .push(
            pick_list(parents, selected, |c: CategoryChoice| {
                Message::NewCategoryParentChanged(c.id)
            })
            .text_size(sz(12))
            .padding(4),
        )
        .push(
            button(text(t("cat_add")).size(sz(12)))
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
                t("cat_delete_note"),
            )
            .size(sz(10)),
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
            button(text(if pinned { t("exe_unpin") } else { t("exe_pin") }).size(sz(11)))
                .padding([4, 10])
                .style(if pinned {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::ExecTogglePinned),
        )
        .push(
            button(
                text(if hidden {
                    t("exe_show_in_picker")
                } else {
                    t("exe_hide_from_picker")
                })
                .size(sz(11)),
            )
            .padding([4, 10])
            .style(if hidden {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::ExecToggleHidden),
        )
        .push(
            button(text(t("exe_desktop_shortcut")).size(sz(11)))
                .padding([4, 10])
                .style(button::secondary)
                .on_press(Message::ExecMakeShortcut),
        )
        .into()
}

fn output_mod_field<'a>(state: &ExecutablesDialogState) -> Element<'a, Message> {
    let none = t("exe_output_none");
    let mut choices: Vec<String> = vec![none.to_string()];
    choices.extend(state.mod_names.iter().cloned());
    let selected = if state.output_mod.trim().is_empty() {
        Some(none.to_string())
    } else {
        Some(state.output_mod.clone())
    };
    Column::new()
        .spacing(4)
        .push(text(t("exe_capture_output")).size(sz(11)))
        .push(
            pick_list(choices, selected, move |c: String| {
                Message::ToolOutputModChanged(if c == none { String::new() } else { c })
            })
            .text_size(sz(12))
            .padding(5)
            .width(Length::Fill),
        )
        .push(
            text(
                t("exe_capture_note"),
            )
            .size(sz(10)),
        )
        .into()
}

/// The INI editor (MO2 ships one as a bundled tool plugin).
///
/// Worth more on Linux than it is on Windows: the copy the game reads lives deep
/// inside the Proton prefix, under a path nobody navigates to by hand. This edits
/// the PROFILE's copy, which is the durable one - the prefix copy is overwritten
/// from it at every launch.
pub(crate) fn ini_editor_dialog<'a>(app: &App, state: &'a IniEditorState) -> Element<'a, Message> {
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
            button(text(f.clone()).size(sz(12)))
                .padding([3, 10])
                .style(if on {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::IniEditorPick(f.clone())),
        );
    }

    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(
            text(tf("ini_title", &[("profile", &profile)]))
                .size(sz(18))
                .width(Length::Fill),
        )
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseIniEditor),
        );

    let editor = iced::widget::text_editor(&state.content)
        .on_action(Message::IniEditorAction)
        .height(Length::Fixed(420.0))
        .font(colony_ui::ui_font())
        .size(sz(12))
        .padding(8);

    // Say what this file IS before the user changes it: which one, whether it
    // exists yet, and that the game reads a deployed copy rather than this path.
    let note = if state.unreadable {
        tf("ini_unreadable", &[("file", &state.current)])
    } else if state.missing {
        tf("ini_missing", &[("file", &state.current)])
    } else {
        t("ini_note").to_string()
    };

    let mut actions = Row::new().spacing(8).align_y(iced::Alignment::Center);
    actions = actions
        .push(
            text(if state.dirty { t("ini_unsaved") } else { "" })
                .size(sz(11))
                .width(Length::Fill),
        )
        .push(tool_btn(t("ini_open_externally"), Message::IniEditorOpenExternal));
    if state.dirty {
        actions = actions.push(tool_btn(t("ini_revert"), Message::IniEditorRevert));
    }
    let save = button(text(t("common_save")).size(sz(12)))
        .padding([4, 14])
        .style(if state.dirty {
            button::primary
        } else {
            button::secondary
        });
    // Greyed rather than merely refused when the file could not be read: the
    // handler stops it too, but a button that looks live and does nothing is its
    // own bug report.
    actions = actions.push(if state.unreadable {
        save
    } else {
        save.on_press(Message::IniEditorSave)
    });

    let card = Column::new()
        .spacing(10)
        .push(header)
        .push(tabs)
        .push(editor)
        .push(text(note).size(sz(10)))
        .push(actions);

    container(card)
        .width(Length::Fixed(760.0))
        .padding(18)
        .style(card_style)
        .into()
}

/// The log pane (MO2's dockable log view).
pub(crate) fn log_pane_dialog<'a>(state: &LogPaneState) -> Element<'a, Message> {
    use eidos_log::Level;

    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(t("log_title")).size(sz(18)).width(Length::Fill))
        .push(tool_btn(t("common_open_folder"), Message::LogOpenFolder))
        .push(tool_btn(t("log_copy"), Message::LogCopy))
        .push(tool_btn(t("log_refresh"), Message::LogRefresh))
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseLogPane),
        );

    // The level floor. Four buttons rather than a dropdown: the whole point is
    // to flip between them while reading, which a dropdown costs two clicks.
    let mut levels = Row::new().spacing(4).align_y(iced::Alignment::Center);
    levels = levels.push(text(t("log_show")).size(sz(11)));
    for lvl in [Level::Debug, Level::Info, Level::Warn, Level::Error] {
        levels = levels.push(
            button(text(lvl.as_str()).size(sz(11)))
                .padding([2, 8])
                .style(if state.level == lvl {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::LogLevel(lvl)),
        );
    }
    levels = levels.push(
        text(tf(
            "log_n_of_total",
            &[("shown", &state.lines.len()), ("total", &state.total)],
        ))
        .size(sz(10))
        .width(Length::Fill),
    );

    // Which session. Newest first, and only a handful are kept, so they all fit.
    let mut sessions = Row::new().spacing(4).align_y(iced::Alignment::Center);
    for f in state.files.iter().take(8) {
        let label = f
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // The instance and the time; the pid is noise on a button.
        let short: String = label.split('.').take(2).collect::<Vec<_>>().join(" ");
        sessions = sessions.push(
            button(text(short).size(sz(10)))
                .padding([2, 6])
                .style(if *f == state.current {
                    button::primary
                } else {
                    button::text
                })
                .on_press(Message::LogPick(f.clone())),
        );
    }

    let mut body = Column::new().spacing(1);
    if state.truncated {
        body = body.push(text(t("log_truncated")).size(sz(10)));
    }
    if state.lines.is_empty() {
        body = body.push(
            text(if state.total == 0 {
                t("log_empty")
            } else {
                t("log_none_at_level")
            })
            .size(sz(12)),
        );
    }
    for (lvl, msg) in &state.lines {
        let colour = match lvl {
            Level::Error => Some(conflict_loses_fg()),
            Level::Warn => Some(pal().warning),
            _ => None,
        };
        let mut line = text(format!("{:<5} {msg}", lvl.as_str()))
            .size(sz(11))
            .font(colony_ui::ui_font());
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
        .push(
            scrollable(body)
                .height(Length::Fixed(420.0))
                .width(Length::Fill),
        );

    container(card)
        .width(Length::Fixed(820.0))
        .padding(18)
        .style(card_style)
        .into()
}

/// The Extensions list: what is installed, whether it can run, and how to add one.
///
/// Called "extensions", never "plugins": in this window a plugin is an `.esp`,
/// and the Plugins tab three inches away shows exactly those.
pub(crate) fn addons_dialog<'a>(app: &App) -> Element<'a, Message> {
    use eidos_addons::AddonKind;

    let game_id = app
        .games
        .get(app.selected.unwrap_or(0))
        .map(|g| g.def.id.to_string());

    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(t("ext_title")).size(sz(18)).width(Length::Fill))
        .push(
            button(text(t("ext_preview_file")).size(sz(11))).on_press(Message::ChooseExtensionFile(
                eidos_addons::protocol::Operation::Preview,
            )),
        )
        .push(
            button(text(t("ext_read_save")).size(sz(11))).on_press(Message::ChooseExtensionFile(
                eidos_addons::protocol::Operation::SaveInfo,
            )),
        )
        .push(tool_btn(t("common_open_folder"), Message::OpenAddonsFolder))
        .push(tool_btn(t("ext_reload"), Message::ReloadAddons))
        .push(
            button(text(t("common_close")).size(sz(12)))
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
                    text(
                        path.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    )
                    .size(sz(13))
                    .color(conflict_loses_fg()),
                )
                .push(text(tf("ext_refused", &[("reason", &why)])).size(sz(11))),
        );
    }
    if app.addons.is_empty() && app.addon_rejected.is_empty() {
        rows = rows.push(
            text(
                t("ext_none_yet"),
            )
            .size(sz(12)),
        );
    }
    for a in &app.addons {
        let kind = match a.kind {
            AddonKind::Tool => t("ext_kind_tool"),
            AddonKind::Diagnose => t("ext_kind_check"),
            AddonKind::Installer => t("ext_kind_installer"),
            AddonKind::Preview => t("ext_kind_preview"),
            AddonKind::SaveInfo => t("ext_kind_save_info"),
        };
        let mut title = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text(a.name.clone()).size(sz(13)))
            .push(text(format!("({kind})")).size(sz(10)));
        if !a.version.is_empty() {
            title = title.push(text(format!("v{}", a.version)).size(sz(10)));
        }
        title = title.push(Space::new().width(Length::Fill));

        // Why it cannot be used, if it cannot - the missing program, or a game
        // it does not apply to. Said on the row rather than only on failure.
        let blocked = a.unavailable().or_else(|| {
            game_id
                .as_deref()
                .filter(|g| !a.applies_to(g))
                .map(|_| t("ext_not_for_game").to_string())
        });
        match (&blocked, a.kind) {
            (Some(why), _) => title = title.push(text(why.clone()).size(sz(10))),
            (None, AddonKind::Tool) => {
                title = title.push(
                    button(text(t("ext_run")).size(sz(11)))
                        .padding([3, 10])
                        .style(button::primary)
                        .on_press(Message::RunAddon(a.id.clone())),
                );
            }
            (None, AddonKind::Diagnose) => {
                title = title.push(text(t("ext_runs_on_refresh")).size(sz(10)));
            }
            (None, _) => {
                title = title.push(text(t("ext_runs_for_files")).size(sz(10)));
            }
        }

        let mut col = Column::new().spacing(2).push(title);
        if !a.description.is_empty() {
            col = col.push(text(a.description.clone()).size(sz(11)));
        }
        let by = if a.author.is_empty() {
            a.source.display().to_string()
        } else {
            format!("{} - {}", a.author, a.source.display())
        };
        col = col.push(text(by).size(sz(10)));
        rows = rows.push(col);
    }

    let card = Column::new()
        .spacing(10)
        .push(header)
        .push(
            scrollable(rows)
                .height(Length::Fixed(360.0))
                .width(Length::Fill),
        )
        .push(
            text(
                t("ext_explainer"),
            )
            .size(sz(10)),
        );

    container(card)
        .width(Length::Fixed(680.0))
        .padding(18)
        .style(card_style)
        .into()
}

/// The Export dialog (MO2's Export to csv): which rows, which columns.
pub(crate) fn export_dialog<'a>(app: &App, state: &ExportDialogState) -> Element<'a, Message> {
    // Aliased: `Column` is also iced's vertical layout, used three lines down.
    use eidos_instance::Column as Col;

    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(t("export_title")).size(sz(18)).width(Length::Fill))
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseExportDialog),
        );

    // How many rows each scope would actually write, said up front - the whole
    // reason to offer a scope is that the numbers differ.
    let total = app.mods.iter().filter(|m| !m.is_separator()).count();
    let active = app
        .mods
        .iter()
        .filter(|m| !m.is_separator() && m.enabled)
        .count();
    let scope = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text(t("export_rows")).size(sz(11)))
        .push(
            button(text(tf("export_all", &[("n", &total)])).size(sz(12)))
                .padding([3, 10])
                .style(if state.scope == ExportScope::All {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::ExportScopeChanged(ExportScope::All)),
        )
        .push(
            button(text(tf("export_enabled_only", &[("n", &active)])).size(sz(12)))
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
                        .push(
                            text(if *on { "[x]" } else { "[ ]" })
                                .size(sz(11))
                                .font(colony_ui::ui_font()),
                        )
                        .push(text(label).size(sz(12))),
                )
                .width(Length::Fill)
                .padding([2, 6])
                .style(button::text)
                .on_press(Message::ExportToggleColumn(i)),
            )
        },
    );
    if state.picked().is_empty() {
        cols = cols.push(
            text(t("export_tick_column"))
                .size(sz(11))
                .color(conflict_loses_fg()),
        );
    }

    let run = button(text(t("export_run")).size(sz(12)))
        .padding([4, 14])
        .style(button::primary);
    let footer = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(
            text(
                t("export_note"),
            )
            .size(sz(10))
            .width(Length::Fill),
        )
        .push(if state.picked().is_empty() { run } else { run.on_press(Message::ExportRun) });

    let card = Column::new()
        .spacing(12)
        .push(header)
        .push(scope)
        .push(scrollable(cols).height(Length::Fixed(300.0)))
        .push(footer);

    container(card)
        .width(Length::Fixed(560.0))
        .padding(18)
        .style(card_style)
        .into()
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
        .push(text(t("instances_title")).size(sz(18)).width(Length::Fill))
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseInstanceManager),
        );

    let mut rows = Column::new().spacing(6);
    if app.known.is_empty() {
        rows = rows.push(text(t("instances_none")).size(sz(12)));
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
                            text_input(t("instances_folder_name"), typed)
                                .on_input(Message::InstanceRenameChanged)
                                .on_submit(Message::InstanceRenameCommit)
                                .padding(5)
                                .size(sz(12)),
                        )
                        .push(
                            button(text(t("common_save")).size(sz(11)))
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
            .push(text(k.label.clone()).size(sz(12)).width(Length::Fill));
        if open_now {
            title = title.push(text(t("instances_open_now")).size(sz(10)));
        } else {
            title = title.push(
                button(text(t("common_open")).size(sz(11)))
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
                button(text(t("common_rename")).size(sz(11)))
                    .padding([3, 8])
                    .style(button::secondary)
                    .on_press(Message::InstanceRenameStart(i)),
            );
            let armed = app.confirm_forget == Some(i);
            title = title.push(
                button(text(if armed { t("common_confirm") } else { t("instances_forget") }).size(sz(11)))
                    .padding([3, 8])
                    .style(if armed {
                        button::danger
                    } else {
                        button::secondary
                    })
                    .on_press(Message::InstanceForget(i)),
            );
        }
        title = title.push(
            button(text(t("common_folder")).size(sz(11)))
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
                t("instances_forget_note"),
            )
            .size(sz(10)),
        );

    container(card)
        .width(Length::Fixed(680.0))
        .padding(18)
        .style(card_style)
        .into()
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
pub(crate) fn preview_dialog<'a>(p: &'a Preview) -> Element<'a, Message> {
    let name = p
        .path()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = match p {
        Preview::Archive { source, .. } => format!("{} - {}", source.member, name),
        _ => name,
    };
    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(name).size(sz(15)).width(Length::Fill))
        .push(button(text(t("preview_extension")).size(sz(11))).on_press_maybe(
            (!matches!(p, Preview::Archive { .. })).then(|| {
                Message::RunFileExtension(
                    eidos_addons::protocol::Operation::Preview,
                    p.path().into(),
                )
            }),
        ))
        .push(
            button(text(t("preview_reveal")).size(sz(11)))
                .padding([4, 10])
                .style(button::secondary)
                .on_press(Message::DataReveal(p.path().to_path_buf())),
        )
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::ClosePreview),
        );
    let content = match p {
        Preview::Archive { content, .. } => content.as_ref(),
        _ => p,
    };
    let body: Element<'a, Message> = match content {
        Preview::Archive { .. } => text(t("preview_nested_archive")).into(),
        Preview::Dds {
            info,
            selection,
            image,
            ..
        } => {
            let selection = *selection;
            let mut controls = Row::new().spacing(8).align_y(iced::Alignment::Center);
            for (label, shown, value, count) in [
                ("Mip", t("preview_mip"), selection.mip, info.mips),
                ("Layer", t("preview_layer"), selection.layer, info.layers),
                ("Face", t("preview_face"), selection.face, info.faces),
            ] {
                let change = |value| {
                    let mut selected = selection;
                    match label {
                        "Mip" => selected.mip = value,
                        "Layer" => selected.layer = value,
                        _ => selected.face = value,
                    };
                    Message::PreviewDdsSelection(selected)
                };
                controls = controls
                    .push(button(text("−")).on_press_maybe(value.checked_sub(1).map(change)))
                    .push(text(format!("{shown} {value}/{}", count - 1)).size(sz(11)))
                    .push(
                        button(text("+"))
                            .on_press_maybe((value + 1 < count).then(|| change(value + 1))),
                    );
            }
            let channel = match selection.channel {
                crate::dds_preview::Channel::Rgba => "RGBA",
                crate::dds_preview::Channel::Rgb => "RGB",
                crate::dds_preview::Channel::Alpha => "Alpha",
            };
            controls = controls.push(pick_list(
                ["RGBA", "RGB", "Alpha"],
                Some(channel),
                move |channel| {
                    let mut selected = selection;
                    selected.channel = match channel {
                        "RGB" => crate::dds_preview::Channel::Rgb,
                        "Alpha" => crate::dds_preview::Channel::Alpha,
                        _ => crate::dds_preview::Channel::Rgba,
                    };
                    Message::PreviewDdsSelection(selected)
                },
            ));
            let body: Element<'a, Message> = match image {
                Ok(handle) => container(
                    iced::widget::image(handle.clone()).content_fit(iced::ContentFit::Contain),
                )
                .center(Length::Fill)
                .into(),
                Err(error) => container(text(error.clone())).center(Length::Fill).into(),
            };
            let note = if info.hdr {
                t("preview_hdr_note")
            } else {
                ""
            };
            Column::new()
                .spacing(8)
                .push(controls)
                .push(
                    text(format!(
                        "{} × {} · {}{note}",
                        info.width, info.height, info.format
                    ))
                    .size(sz(10)),
                )
                .push(body)
                .into()
        }
        Preview::Nif { model, .. } => {
            let view = model.view;
            let orbit = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text(t("preview_orbit")).size(sz(11)))
                .push(
                    slider(
                        -std::f32::consts::PI..=std::f32::consts::PI,
                        view.yaw,
                        move |yaw| Message::PreviewNifView(crate::nif_render::View { yaw, ..view }),
                    )
                    .step(0.03_f32)
                    .width(Length::Fill),
                )
                .push(text(t("preview_tilt")).size(sz(11)))
                .push(
                    slider(-1.5..=1.5, view.pitch, move |pitch| {
                        Message::PreviewNifView(crate::nif_render::View { pitch, ..view })
                    })
                    .step(0.03_f32)
                    .width(Length::Fill),
                );
            let controls = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text(t("preview_zoom")).size(sz(11)))
                .push(
                    slider(0.1..=10.0, view.zoom, move |zoom| {
                        Message::PreviewNifView(crate::nif_render::View { zoom, ..view })
                    })
                    .step(0.1_f32)
                    .width(Length::Fill),
                )
                .push(
                    checkbox(view.wireframe)
                        .label(t("preview_wireframe"))
                        .on_toggle(move |wireframe| {
                            Message::PreviewNifView(crate::nif_render::View { wireframe, ..view })
                        }),
                )
                .push(button(text(t("preview_reset"))).on_press(Message::PreviewNifView(Default::default())));
            let image: Element<'a, Message> = match &model.image {
                Some(Ok(handle)) => container(
                    iced::widget::image(handle.clone()).content_fit(iced::ContentFit::Contain),
                )
                .center(Length::Fill)
                .into(),
                Some(Err(error)) => container(text(error.clone())).center(Length::Fill).into(),
                None => container(text(t("preview_loading_textures")))
                    .center(Length::Fill)
                    .into(),
            };
            let mut body = Column::new().spacing(8).push(orbit).push(controls)
                .push(text(tf("preview_model_note", &[("n", &model.scene.meshes.len())])).size(sz(10)))
                .push(image);
            if !model.warnings.is_empty() {
                body = body.push(
                    scrollable(text(model.warnings.as_ref()).size(sz(10)))
                        .height(Length::Fixed(58.0)),
                );
            }
            body.into()
        }
        Preview::Image { handle, .. } => {
            container(iced::widget::image(handle.clone()).content_fit(iced::ContentFit::Contain))
                .center(Length::Fill)
                .into()
        }
        Preview::Text {
            body, truncated, ..
        } => {
            let mut col = Column::new().spacing(4).push(
                // Monospaced, because everything that reaches here - an INI, a
                // log, a Papyrus source - is written in columns.
                text(body.clone()).size(sz(11)).font(colony_ui::ui_font()),
            );
            if *truncated {
                col = col.push(
                    text(tf(
                        "preview_text_truncated",
                        &[("kb", &(PREVIEW_TEXT_CAP / 1024))],
                    ))
                    .size(sz(10)),
                );
            }
            scrollable(col).height(Length::Fill).into()
        }
        Preview::Unsupported { why, .. } => container(text(why.clone()).size(sz(12)))
            .center(Length::Fill)
            .into(),
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
        .push(text(t("collection_title")).size(sz(18)).width(Length::Fill))
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseCollection),
        );

    let field = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(
            text_input(
                "nxm://<game>/collections/<slug>/revisions/latest",
                &state.link,
            )
            .on_input(Message::CollectionLinkChanged)
            .on_submit(Message::CollectionFetch)
            .padding(5)
            .size(sz(12)),
        )
        .push(
            button(text(if state.loading { "..." } else { t("collection_look_up") }).size(sz(12)))
                .padding([4, 12])
                .style(button::primary)
                .on_press_maybe((!state.loading).then_some(Message::CollectionFetch)),
        );

    let mut card = Column::new().spacing(10).push(header).push(field);

    if let Some(e) = &state.error {
        card = card.push(text(e.clone()).size(sz(12)).color(conflict_loses_fg()));
    }

    if let Some(rev) = &state.revision {
        if !rev.visible() {
            // The gate answered. Say which rule, not "no".
            let why = rev.hidden.map(|h| h.message()).unwrap_or("");
            card = card.push(text(why.to_string()).size(sz(12)));
            return container(card)
                .width(Length::Fixed(720.0))
                .padding(18)
                .style(card_style)
                .into();
        }

        let installed = state
            .states
            .iter()
            .filter(|s| **s == MemberState::Installed)
            .count();
        let downloaded = state
            .states
            .iter()
            .filter(|s| **s == MemberState::Downloaded)
            .count();
        let missing = state
            .states
            .iter()
            .filter(|s| **s == MemberState::Missing)
            .count();
        let unverified = state
            .states
            .iter()
            .filter(|s| **s == MemberState::Unverified)
            .count();
        let other_version = state
            .states
            .iter()
            .filter(|s| **s == MemberState::OtherVersion)
            .count();

        let mut title = Column::new()
            .spacing(2)
            .push(text(tf("collection_revision", &[("name", &rev.name), ("n", &rev.revision_number)])).size(sz(15)));
        if !rev.author.is_empty() {
            title = title.push(text(tf("collection_by", &[("author", &rev.author)])).size(sz(11)));
        }
        if !rev.summary.is_empty() {
            title = title.push(text(rev.summary.clone()).size(sz(11)));
        }
        card = card.push(title);

        let mut summary = Row::new()
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .push(
                text(tf(
                    "collection_counts",
                    &[
                        ("installed", &installed),
                        ("other", &other_version),
                        ("unverified", &unverified),
                        ("downloaded", &downloaded),
                        ("missing", &missing),
                    ],
                ))
                .size(sz(12))
                .width(Length::Fill),
            );
        // The real action. It sits ahead of "fetch missing" because fetching is
        // now a step INSIDE it rather than a thing to do instead of it.
        summary = summary.push(
            button(
                text(if state.runtime.is_some() {
                    t("collection_continue_mismatch")
                } else {
                    t("collection_install")
                })
                .size(sz(11)),
            )
            .padding([3, 10])
            .style(button::primary)
            .on_press_maybe((!state.loading).then_some(Message::CollectionInstall)),
        );
        if missing > 0 {
            let label = if state.confirm_fetch {
                t("collection_fetch_confirm").to_string()
            } else {
                tf("collection_fetch_missing", &[("n", &missing)])
            };
            summary = summary.push(
                button(text(label).size(sz(11)))
                    .padding([3, 10])
                    .style(if state.confirm_fetch {
                        button::danger
                    } else {
                        button::secondary
                    })
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
                        .push(text(t("collection_instructions")).size(sz(10)))
                        .push(text(rev.instructions.clone()).size(sz(11))),
                )
                .padding(8)
                .style(card_style),
            );
        }

        let mut rows = Column::new().spacing(1);
        for (i, (m, st)) in rev.mods.iter().zip(&state.states).enumerate() {
            let (label, colour) = match st {
                MemberState::Installed => (t("collection_member_installed"), Some(conflict_wins_fg())),
                MemberState::Unverified => (t("collection_member_unverified"), Some(pal().warning)),
                // Its own word, and its own colour: the mod is there but not the
                // version the collection was built against.
                MemberState::OtherVersion => (t("collection_member_other"), Some(pal().warning)),
                MemberState::Downloaded => (t("collection_member_downloaded"), None),
                MemberState::Missing => (t("collection_member_missing"), Some(conflict_loses_fg())),
            };
            let mut status = text(label.to_string())
                .size(sz(11))
                .width(Length::Fixed(scaled(120.0)));
            if let Some(c) = colour {
                status = status.color(c);
            }
            let name = if m.optional {
                tf("collection_optional", &[("name", &m.name)])
            } else {
                m.name.clone()
            };
            let row = Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text(name).size(sz(12)).width(Length::Fill))
                .push(
                    text(m.version.clone())
                        .size(sz(11))
                        .width(Length::Fixed(scaled(70.0))),
                )
                .push(status)
                .push(
                    button(text(t("common_open")).size(sz(10)))
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
                text(tf(
                    "collection_members_gone",
                    &[
                        ("n", &(rev.mod_count as usize - rev.mods.len())),
                        ("total", &rev.mod_count),
                    ],
                ))
                .size(sz(11)),
            );
        }
        card = card.push(scrollable(rows).height(Length::Fixed(320.0)));
    }

    card = card.push(
        text(
            t("collection_explainer"),
        )
        .size(sz(10)),
    );

    // Scrollable, and capped. A collection's own installation notes are free
    // Markdown from its author and routinely run to several screens; drawn into
    // an uncapped card the pane grew past the window and the end of the text was
    // simply unreachable, with no way to scroll to it.
    container(scrollable(card).height(Length::Shrink))
        .width(Length::Fixed(720.0))
        .max_height(640.0)
        .padding(18)
        .style(card_style)
        .into()
}

/// Roughly what share of an instance a `-mx` level leaves.
///
/// Measured for level 1 - 43.0% of 1199 MB of real Skyrim `.dds`/`.nif` on a
/// 16-thread machine - which is the one that matters, being the default. Level 0
/// is exact: it stores. The rest are interpolated towards the ~38% a `-mx9` run
/// of the same corpus reached, and the dialog says "roughly" because that is
/// what they are: textures and meshes are already-compressed formats, so a mod
/// list heavy in loose scripts or ESPs will do better than any of these.
fn estimated_share(level: u8) -> f64 {
    match level {
        0 => 1.00,
        1 => 0.43,
        3 => 0.41,
        5 => 0.40,
        7 => 0.385,
        _ => 0.38,
    }
}

/// The Pack dialog: a whole instance into one file, and the preview of what
/// that means before twenty minutes are spent on it.
pub(crate) fn pack_dialog<'a>(app: &App, state: &PackDialogState) -> Element<'a, Message> {
    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(t("pack_title")).size(sz(18)).width(Length::Fill))
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::ClosePackDialog),
        );

    let root = app
        .created
        .as_ref()
        .map(|i| i.root.display().to_string())
        .unwrap_or_default();
    let blurb = text(tf("pack_blurb", &[("root", &root)]))
    .size(sz(11))
    .color(text_muted());

    let dest = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text(t("pack_file")).size(sz(11)).width(Length::Fixed(scaled(56.0))))
        .push(
            text_input("/home/you/skyrimse.eidos", &state.dest)
                .on_input(Message::PackDestChanged)
                .padding(5)
                .size(sz(12)),
        )
        .push(
            button(text(t("common_browse")).size(sz(11)))
                .padding([4, 10])
                .style(button::secondary)
                .on_press(Message::PackBrowse),
        );

    // The numbers, without downloads/ when the box is unticked. Subtracted
    // rather than re-walked: this is a checkbox, and walking 57 000 files under
    // the user's finger is not what a checkbox should feel like.
    let (files, bytes) = if state.downloads {
        (state.plan.files, state.plan.bytes)
    } else {
        (
            state.plan.files.saturating_sub(state.plan.downloads_files),
            state.plan.bytes.saturating_sub(state.plan.downloads_bytes),
        )
    };
    let downloads = checkbox(state.downloads)
        .label(tf(
            "pack_include_downloads",
            &[("size", &eidos_transfer::human_bytes(state.plan.downloads_bytes))],
        ))
        .on_toggle(|_| Message::PackToggleDownloads)
        .size(sz(13))
        .text_size(sz(12));

    let mut levels = Row::new()
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .push(text(t("pack_compression")).size(sz(11)).width(Length::Fixed(scaled(76.0))));
    for n in eidos_transfer::LEVELS {
        levels = levels.push(
            button(text(n.to_string()).size(sz(11)))
                .padding([3, 9])
                .style(if state.level == n {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::PackLevelChanged(n)),
        );
    }
    levels = levels.push(
        text(if state.level == eidos_transfer::DEFAULT_LEVEL {
            format!("  {}", t("pack_level_default"))
        } else if state.level == 0 {
            format!("  {}", t("pack_level_stored"))
        } else {
            format!("  {}", t("pack_level_slower"))
        })
        .size(sz(10))
        .color(text_muted()),
    );

    let expect = (bytes as f64 * estimated_share(state.level)) as u64;
    let preview = Column::new()
        .spacing(3)
        .push(
            text(tf(
                "pack_plan",
                &[
                    ("files", &files),
                    ("dirs", &state.plan.dirs),
                    ("empty", &state.plan.empty_dirs),
                    ("size", &eidos_transfer::human_bytes(bytes)),
                ],
            ))
            .size(sz(12)),
        )
        .push(
            text(tf(
                "pack_expect",
                &[("size", &eidos_transfer::human_bytes(expect))],
            ))
            .size(sz(11))
            .color(text_muted()),
        );

    let mut notes = Column::new().spacing(3).push(
        text(
            t("pack_left_out"),
        )
        .size(sz(10))
        .color(text_muted()),
    );
    if !state.plan.tools_outside.is_empty() {
        notes = notes.push(
            text(tf(
                "pack_tools_outside",
                &[
                    ("n", &state.plan.tools_outside.len()),
                    (
                        "tools",
                        &state
                            .plan
                            .tools_outside
                            .iter()
                            .map(|tool| tool.title.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                ],
            ))
            .size(sz(10))
            .color(text_muted()),
        );
    }
    let path = std::path::Path::new(state.dest.trim());
    let ready = !state.dest.trim().is_empty();
    if ready && path.is_file() {
        notes = notes.push(
            text(t("pack_file_exists"))
                .size(sz(10))
                .color(pal().warning),
        );
    }

    let run = button(text(t("pack_run")).size(sz(12)))
        .padding([4, 16])
        .style(button::primary);
    let footer = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(
            text(t("pack_locked_note"))
                .size(sz(10))
                .color(text_muted())
                .width(Length::Fill),
        )
        .push(if ready {
            run.on_press(Message::PackRun)
        } else {
            run
        });

    let card = Column::new()
        .spacing(11)
        .push(header)
        .push(blurb)
        .push(dest)
        .push(downloads)
        .push(levels)
        .push(preview)
        .push(notes)
        .push(footer);
    container(card)
        .width(Length::Fixed(600.0))
        .padding(18)
        .style(card_style)
        .into()
}

/// The Unpack dialog. Reachable with no instance open, which is the point.
pub(crate) fn unpack_dialog<'a>(state: &UnpackDialogState) -> Element<'a, Message> {
    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(t("unpack_title")).size(sz(18)).width(Length::Fill))
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([4, 12])
                .style(button::secondary)
                .on_press(Message::CloseUnpackDialog),
        );

    let archive = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text(t("unpack_backup")).size(sz(11)).width(Length::Fixed(scaled(52.0))))
        .push(
            text_input("/home/you/skyrimse.eidos", &state.archive)
                .padding(5)
                .size(sz(12)),
        )
        .push(
            button(text(t("common_choose")).size(sz(11)))
                .padding([4, 10])
                .style(button::secondary)
                .on_press(Message::UnpackBrowseArchive),
        );

    // What the file says about itself, once it has been read. This is the whole
    // reason the dialog reads the manifest at the moment the file is chosen:
    // "is this the right backup" is a question only the file can answer.
    let mut about = Column::new().spacing(2);
    if let Some(e) = &state.error {
        about = about.push(text(e.clone()).size(sz(11)).color(pal().warning));
    }
    if let Some(m) = &state.manifest {
        about = about
            .push(
                text(tf(
                    "unpack_made_by",
                    &[("game", &m.game_id), ("date", &m.created), ("version", &m.eidos_version)],
                ))
                .size(sz(11)),
            )
            .push(
                text(tf(
                    "unpack_contents",
                    &[
                        ("files", &m.files),
                        ("size", &eidos_transfer::human_bytes(m.bytes)),
                        ("profiles", &m.profiles.join(", ")),
                    ],
                ))
                .size(sz(11))
                .color(text_muted()),
            )
            .push(
                text(tf("unpack_was_at", &[("path", &m.source_root)]))
                    .size(sz(10))
                    .color(text_muted()),
            );
        if !m.downloads {
            about = about.push(
                text(t("unpack_no_downloads"))
                    .size(sz(10))
                    .color(text_muted()),
            );
        }
        let missing: Vec<&str> = m
            .tools_outside
            .iter()
            .filter(|t| !std::path::Path::new(&t.exe).exists())
            .map(|t| t.title.as_str())
            .collect();
        if !missing.is_empty() {
            about = about.push(
                text(tf("unpack_tools_missing", &[("tools", &missing.join(", "))]))
                .size(sz(10))
                .color(pal().warning),
            );
        }
    }

    let dest = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text(t("common_folder")).size(sz(11)).width(Length::Fixed(scaled(52.0))))
        .push(
            text_input("/mnt/games/EidosSkyrim", &state.dest)
                .on_input(Message::UnpackDestChanged)
                .padding(5)
                .size(sz(12)),
        )
        .push(
            button(text(t("common_choose")).size(sz(11)))
                .padding([4, 10])
                .style(button::secondary)
                .on_press(Message::UnpackBrowseDest),
        );

    let force = checkbox(state.force)
        .label(t("unpack_force").to_string())
        .on_toggle(|_| Message::UnpackToggleForce)
        .size(sz(13))
        .text_size(sz(12));

    let ready = state.manifest.is_some() && !state.dest.trim().is_empty();
    let run = button(text(t("unpack_run")).size(sz(12)))
        .padding([4, 16])
        .style(button::primary);
    let footer = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(
            text(
                t("unpack_prefix_note"),
            )
            .size(sz(10))
            .color(text_muted())
            .width(Length::Fill),
        )
        .push(if ready {
            run.on_press(Message::UnpackRun)
        } else {
            run
        });

    let card = Column::new()
        .spacing(11)
        .push(header)
        .push(archive)
        .push(about)
        .push(dest)
        .push(force)
        .push(footer);
    container(card)
        .width(Length::Fixed(600.0))
        .padding(18)
        .style(card_style)
        .into()
}

/// A pack or unpack in flight, and afterwards what it said.
///
/// One card for both states on purpose. The bar becomes the report, in the place
/// the user is already looking - and the report has somewhere to live at all,
/// which the status bar could not give it: an unpack happens on the welcome
/// screen, where there is no status bar to write to.
pub(crate) fn transfer_dialog<'a>(job: &crate::TransferJob) -> Element<'a, Message> {
    let mut card = Column::new()
        .spacing(10)
        .push(text(job.title.clone()).size(sz(15)));
    match &job.finished {
        None => {
            let pct = job
                .percent
                .load(std::sync::atomic::Ordering::SeqCst)
                .min(100);
            card = card
                .push(
                    iced::widget::progress_bar(0.0..=100.0, pct as f32)
                        .length(Length::Fill)
                        .girth(Length::Fixed(10.0)),
                )
                .push(
                    text(if pct >= 100 {
                        t("transfer_finishing").to_string()
                    } else {
                        format!("{pct}%")
                    })
                    .size(sz(12)),
                )
                .push(
                    text(
                        t("transfer_note"),
                    )
                    .size(sz(10))
                    .color(text_muted()),
                );
        }
        Some(result) => {
            let (body, colour) = match result {
                Ok(summary) => (summary.clone(), None),
                Err(why) => (why.clone(), Some(pal().warning)),
            };
            let mut line = text(body).size(sz(12));
            if let Some(c) = colour {
                line = line.color(c);
            }
            card = card
                .push(scrollable(line).height(Length::Fixed(180.0)))
                .push(
                    Row::new().push(Space::new().width(Length::Fill)).push(
                        button(text(t("common_close")).size(sz(12)))
                            .padding([4, 16])
                            .style(button::primary)
                            .on_press(Message::CloseTransferResult),
                    ),
                );
        }
    }
    container(card)
        .width(Length::Fixed(560.0))
        .padding(16)
        .style(card_style)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_links_are_split_from_surrounding_text() {
        // A real LOOT message shape: prose, a link, more prose.
        let parts = split_markdown_links(
            "Please install [SSEEdit v4.1.5d](https://www.nexusmods.com/x/mods/164) to clean it.",
        );
        assert_eq!(
            parts,
            vec![
                ("Please install ".to_string(), None),
                (
                    "SSEEdit v4.1.5d".to_string(),
                    Some("https://www.nexusmods.com/x/mods/164".to_string())
                ),
                (" to clean it.".to_string(), None),
            ]
        );
    }

    #[test]
    fn plain_text_and_malformed_links_stay_verbatim() {
        assert_eq!(
            split_markdown_links("no links here"),
            vec![("no links here".to_string(), None)]
        );
        // Unclosed / empty forms are not links and must round-trip unchanged.
        for s in [
            "[label](",
            "[label] (url)",
            "[](url)",
            "[label]()",
            "a [b c",
        ] {
            let parts = split_markdown_links(s);
            let rebuilt: String = parts.iter().map(|(t, _)| t.as_str()).collect();
            assert_eq!(rebuilt, s, "{s:?} must survive unchanged");
            assert!(
                parts.iter().all(|(_, u)| u.is_none()),
                "{s:?} is not a link"
            );
        }
    }

    #[test]
    fn multibyte_text_around_a_link_is_not_split_mid_character() {
        // A non-ASCII message must not panic or corrupt (byte-indexed scanning).
        let parts = split_markdown_links("Réglé - voir [le fil](https://loot.example/é) déjà");
        let rebuilt: String = parts.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(rebuilt, "Réglé - voir le fil déjà");
        assert_eq!(parts[1].1.as_deref(), Some("https://loot.example/é"));
    }

    // ---- the LOOT report as a worklist ---------------------------------------

    fn sample_report() -> eidos_loot::LootReport {
        use eidos_loot::{LootDirtyInfo, LootMessage, MessageType, PluginReport};
        eidos_loot::LootReport {
            plugin_meta: Default::default(),
            general: vec![LootMessage {
                kind: MessageType::Warn,
                text: "Skyrim Script Extender scripts seem to be missing".into(),
            }],
            plugins: vec![PluginReport {
                name: "Update.esm".into(),
                missing_masters: vec!["Nope.esm".into()],
                messages: vec![LootMessage {
                    kind: MessageType::Say,
                    text: "a note".into(),
                }],
                dirty: vec![LootDirtyInfo {
                    crc: 1,
                    cleaning_utility: "SSEEdit v4.1.5d".into(),
                    itm_count: 386,
                    deleted_reference_count: 93,
                    deleted_navmesh_count: 3,
                }],
            }],
        }
    }

    #[test]
    fn the_copied_report_carries_what_the_screen_shows() {
        let out = loot_report_text(&sample_report());
        // The counts are the point: they are what gets read off while xEdit runs.
        assert!(out.contains("386 ITM"), "{out}");
        assert!(out.contains("93 deleted refs"), "{out}");
        assert!(out.contains("Update.esm"), "{out}");
        assert!(out.contains("Missing masters: Nope.esm"), "{out}");
        assert!(out.contains("SSEEdit v4.1.5d"), "{out}");
    }

    #[test]
    fn severity_survives_the_loss_of_colour() {
        // On screen the severity is a colour. Pasted into a text editor a colour
        // is nothing, so each line has to say it.
        let out = loot_report_text(&sample_report());
        assert!(out.contains("[warning] Skyrim Script Extender"), "{out}");
        assert!(out.contains("[note] a note"), "{out}");
    }

    #[test]
    fn a_clean_report_still_says_something() {
        let out = loot_report_text(&eidos_loot::LootReport::default());
        assert!(out.contains("clean"), "{out}");
        assert!(!out.trim().is_empty());
    }

    #[test]
    fn an_unnamed_cleaning_utility_does_not_print_an_empty_gap() {
        let mut r = sample_report();
        r.plugins[0].dirty[0].cleaning_utility = String::new();
        assert!(
            loot_report_text(&r).contains("Dirty - ? found"),
            "empty utility left a hole"
        );
    }

    // ---- the prerequisite status line ---------------------------------------
    //
    // The Prereqs field accepts any text, so this is the only thing that answers
    // "will this tool start?". A verb reported as present when it is not sends
    // the user hunting through their mods for a fault that is not there.

    #[test]
    fn a_directx_dll_is_installed_through_winetricks() {
        // Eidos ships no Microsoft DLL, so a missing one is something to click,
        // and one the prefix has reads as installed.
        let none = std::collections::HashSet::new();
        for v in ["d3dcompiler_47", "d3dx9_43", "d3dx11_43"] {
            let (label, missing) = prereq_state(v, &none);
            assert!(missing, "{v} reported present with nothing installed");
            assert!(label.contains("click"), "{v}: {label}");
            let done = std::collections::HashSet::from([v.to_string()]);
            assert_eq!(prereq_state(v, &done), ("installed", false), "{v}");
        }
    }

    #[test]
    fn a_winetricks_verb_reads_from_what_the_instance_recorded() {
        let mut done = std::collections::HashSet::new();
        let (_, missing) = prereq_state("dotnetdesktop8", &done);
        assert!(missing, "nothing recorded, so it cannot be installed");
        done.insert("dotnetdesktop8".to_string());
        let (label, missing) = prereq_state("dotnetdesktop8", &done);
        assert!(!missing);
        assert_eq!(label, "installed");
    }

    #[test]
    fn a_typo_is_named_as_one_and_offers_nothing() {
        // `dotnet10` is real; `dotnet1O` (letter O) is what a tired user types.
        // Offering to install it would spend a download on nothing.
        let none = std::collections::HashSet::new();
        let (label, missing) = prereq_state("dotnet1O", &none);
        assert!(!missing, "offered to install a verb that does not exist");
        assert!(label.contains("unknown"), "{label}");
    }

    #[test]
    fn a_runtime_answers_from_the_shared_cache_not_the_instance() {
        // Tier 3 lives outside any instance, so an empty `prereqs.done` must not
        // make an installed runtime look missing.
        let none = std::collections::HashSet::new();
        let installed = eidos_gamefeatures::runtime("dotnet10")
            .is_some_and(eidos_gamefeatures::runtime_is_installed);
        let (label, missing) = prereq_state("dotnet10", &none);
        assert_eq!(
            missing, !installed,
            "state disagrees with the cache: {label}"
        );
        if missing {
            assert!(
                label.contains("click"),
                "a missing runtime must say what to do: {label}"
            );
        }
    }

    #[test]
    fn the_wording_tells_the_user_what_to_do() {
        // Every actionable state has to say so; a red label with no verb is just
        // bad news.
        let none = std::collections::HashSet::new();
        for v in ["dotnetdesktop8", "dotnet10"] {
            let (label, missing) = prereq_state(v, &none);
            if missing {
                assert!(label.contains("click"), "{v}: {label}");
                assert!(label.contains("NOT FOUND"), "{v}: {label}");
            }
        }
    }
}
