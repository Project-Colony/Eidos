//! The Data and Overwrite tabs, and the date and size formats they share.

use super::*;

/// How many tree rows the Data tab will draw. Generous for browsing, but finite:
/// expanding `meshes/` in a heavy setup is six figures of files and iced builds a
/// widget per row.
pub(crate) const DATA_TREE_ROWS: usize = 3000;

/// Data tab: the merged view as a real tree, each node labelled with the layer
/// that actually provides it. This is the virtual filesystem the game will see,
/// so a hidden file is absent here by construction - unhiding is done from the
/// owning mod's Filetree tab, which shows the mod's real contents.
pub(crate) fn data_panel<'a>(app: &App) -> Element<'a, Message> {
    // Column widths declared once so the header and the rows cannot drift apart.
    const C_SIZE: f32 = 72.0;
    const C_DATE: f32 = 96.0;
    const C_ACTION: f32 = 116.0;

    let controls = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(
            text_input(t("data_filter"), &app.data_query)
                .on_input(Message::DataQueryChanged)
                .padding(5)
                .size(sz(12))
                .width(Length::Fill),
        )
        .push(
            button(text(t("data_conflicts_only")).size(sz(11)))
                .padding([3, 8])
                .style(if app.data_conflicts_only {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::DataToggleConflictsOnly),
        )
        .push(tool_btn(t("data_expand_all"), Message::DataExpandAll))
        .push(tool_btn(t("data_collapse_all"), Message::DataCollapseAll));

    let header = Row::new()
        .spacing(6)
        .push(
            text(t("info_name"))
                .size(sz(11))
                .width(Length::FillPortion(3)),
        )
        .push(
            text(t("data_provided_by"))
                .size(sz(11))
                .width(Length::FillPortion(2)),
        )
        .push(
            text(t("col_size"))
                .size(sz(11))
                .width(Length::Fixed(scaled(C_SIZE))),
        )
        .push(
            text(t("col_modified"))
                .size(sz(11))
                .width(Length::Fixed(scaled(C_DATE))),
        )
        .push(text("").size(sz(11)).width(Length::Fixed(scaled(C_ACTION))));

    // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
    let rows = data_tree_rows(app, DATA_TREE_ROWS);
    if rows.is_empty() {
        list = list.push(
            text(
                if app.data_query.trim().is_empty() && !app.data_conflicts_only {
                    t("common_empty")
                } else {
                    t("data_no_match")
                },
            )
            .size(sz(12)),
        );
    }
    let truncated = rows.len() >= DATA_TREE_ROWS;
    for (idx, r) in rows.into_iter().enumerate() {
        // A folder gets a clickable disclosure triangle; a file gets a spacer of
        // the same width so names stay in one column.
        let lead: Element<'a, Message> = if r.row.is_dir {
            let glyph = if app.data_expanded.contains(&r.rel) {
                colony_ui::widgets::icons::CHEVRON_DOWN
            } else {
                colony_ui::widgets::icons::CHEVRON_RIGHT
            };
            button(text(glyph).size(sz(9)).font(colony_ui::fonts::glyph_font()))
                .padding([0, 4])
                .on_press(Message::DataToggleDir(r.rel.clone()))
                .style(button::text)
                .into()
        } else {
            Space::new().width(Length::Fixed(18.0)).into()
        };
        // A contested file is marked on the NAME, where the eye already is,
        // rather than in a column of its own that would cost width on every row.
        // The colour is the one the mod list already uses for "this wins the
        // file", because that is exactly what the row is saying.
        let label: Element<'a, Message> = if r.row.conflicted {
            text(format!("{} *", r.row.name))
                .size(sz(12))
                .color(conflict_wins_fg())
                .into()
        } else {
            text(r.row.name.clone()).size(sz(12)).into()
        };
        let name = Row::new()
            .spacing(2)
            .align_y(iced::Alignment::Center)
            .push(Space::new().width(Length::Fixed(r.depth as f32 * 14.0)))
            .push(lead)
            .push(label);

        // Hiding is only offered on rows a mod owns: the Overwrite is regenerated
        // by the game (it would just come back) and the game layer is the pristine
        // install, which Eidos never writes to.
        let owner = app
            .mods
            .iter()
            .position(|m| !m.is_separator() && m.name == r.row.source);
        let mut action = Row::new().spacing(3).align_y(iced::Alignment::Center);
        // Reveal works on every row: the whole reason to look at this tree is to
        // find out WHICH copy of a file the game gets, and the next question is
        // always "where is it".
        action = action.push(
            button(text(t("preview_reveal")).size(sz(10)))
                .padding([1, 5])
                .on_press(Message::DataReveal(r.row.real.clone()))
                .style(button::secondary),
        );
        if !r.row.is_dir {
            action = action.push(
                button(text(t("info_view")).size(sz(10)))
                    .padding([1, 5])
                    .on_press(Message::PreviewFile(r.row.real.clone()))
                    .style(button::text),
            );
        }
        if let Some(i) = owner {
            action = action.push(
                button(text(t("info_hide")).size(sz(10)))
                    .padding([1, 5])
                    .on_press(Message::ToggleFileHidden(i, r.rel.clone()))
                    .style(button::secondary),
            );
        }

        let row = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(container(name).width(Length::FillPortion(3)))
            .push(
                text(r.row.source.clone())
                    .size(sz(12))
                    .width(Length::FillPortion(2)),
            )
            .push(
                text(r.row.size.map(format_size).unwrap_or_default())
                    .size(sz(11))
                    .width(Length::Fixed(scaled(C_SIZE)))
                    .align_x(iced::alignment::Horizontal::Right),
            )
            .push(
                text(r.row.mtime.map(format_when).unwrap_or_default())
                    .size(sz(11))
                    .width(Length::Fixed(scaled(C_DATE))),
            )
            .push(container(action).width(Length::Fixed(scaled(C_ACTION))));
        list = list.push(striped(row.into(), idx % 2 == 0));
    }
    if truncated {
        list = list.push(text(tf("data_truncated", &[("n", &DATA_TREE_ROWS)])).size(sz(11)));
    }
    Column::new()
        .spacing(4)
        .push(controls)
        .push(header)
        .push(scrollable(list).height(Length::Fill))
        .into()
}

/// A file timestamp as `YYYY-MM-DD`, which is what a Data tree is read for -
/// "did this mod's copy land after that one's" - and short enough to sit in a
/// column beside a name.
pub(crate) fn format_when(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // The backups list already owns this arithmetic (Howard Hinnant's
    // civil_from_days); it renders `YYYY-MM-DD HH:MM` and the date is the half
    // a file column has room for.
    eidos_instance::format_stamp(secs)
        .split(' ')
        .next()
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn overwrite_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text(t("panel_no_instance")).into();
    };
    let dir = inst.overwrite_dir();
    let actions = Row::new()
        .spacing(6)
        .push(text(t("overwrite_intro")).size(sz(12)).width(Length::Fill))
        // MO2's central Overwrite workflow: turn what the game/tools generated into
        // a real, orderable mod instead of only being able to delete it.
        .push(tool_btn(
            t("overwrite_create_mod"),
            Message::OverwriteToModStart,
        ))
        // MO2's "Sync to Mods": send each file back to the mod that already
        // provides that path, instead of bundling everything into a new one.
        .push(
            button(
                text(if app.confirm_sync {
                    t("overwrite_confirm_send_back")
                } else {
                    t("overwrite_send_back")
                })
                .size(sz(12)),
            )
            .padding(5)
            .on_press(Message::OverwriteSyncToMods)
            .style(if app.confirm_sync {
                button::danger
            } else {
                button::secondary
            }),
        )
        .push(tool_btn(
            t("common_open_folder"),
            Message::OpenFolder(dir.clone()),
        ))
        .push(
            button(
                text(if app.confirm_clear {
                    t("overwrite_confirm_clear")
                } else {
                    t("overwrite_clear")
                })
                .size(sz(12)),
            )
            .padding(5)
            .on_press(Message::ClearOverwrite)
            .style(if app.confirm_clear {
                button::danger
            } else {
                button::secondary
            }),
        );

    // The inline name prompt, shown while "Create mod..." is armed. Typing an
    // existing mod's name merges into it (MO2's "move content to mod").
    let prompt: Option<Element<'a, Message>> = app.overwrite_to_mod.as_ref().map(|name| {
        let exists = inst.mods_dir().join(name.trim()).exists();
        let hint = if exists {
            t("overwrite_hint_merge")
        } else {
            t("overwrite_hint_new")
        };
        Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text(t("overwrite_mod_name")).size(sz(12)))
            .push(
                text_input(t("overwrite_mod_name"), name)
                    .on_input(Message::OverwriteToModName)
                    .on_submit(Message::OverwriteToModCommit)
                    .padding(5)
                    .size(sz(12))
                    .width(Length::Fixed(scaled(260.0))),
            )
            .push(text(hint).size(sz(10)).width(Length::Fill))
            .push(
                button(text(t("info_create")).size(sz(12)))
                    .padding([4, 12])
                    .on_press(Message::OverwriteToModCommit)
                    .style(button::primary),
            )
            .push(tool_btn(t("common_cancel"), Message::OverwriteToModCancel))
            .into()
    });

    // A tree, not 4902 full paths one under the other. Same grammar as the Data
    // tab - triangle, indent, name - because they are the same gesture.
    let entries = cached_entries(app, &dir);
    let mut c = Column::new().spacing(2);
    if entries.is_empty() {
        c = c.push(text(t("common_empty")).size(sz(12)));
    } else {
        c = c.push(text(tf("info_n_files", &[("n", &entries.len())])).size(sz(11)));
    }
    let rows = overwrite_tree_rows(app, &entries, DATA_TREE_ROWS);
    let truncated = rows.len() >= DATA_TREE_ROWS;
    for r in rows {
        let lead: Element<'a, Message> = match r.files {
            Some(_) => {
                let glyph = if app.overwrite_expanded.contains(&r.rel) {
                    colony_ui::widgets::icons::CHEVRON_DOWN
                } else {
                    colony_ui::widgets::icons::CHEVRON_RIGHT
                };
                button(text(glyph).size(sz(9)).font(colony_ui::fonts::glyph_font()))
                    .padding([0, 4])
                    .on_press(Message::OverwriteToggleDir(r.rel.clone()))
                    .style(button::text)
                    .into()
            }
            // Same width as the triangle, so names stay in one column.
            None => Space::new().width(Length::Fixed(18.0)).into(),
        };
        let mut row = Row::new()
            .spacing(2)
            .align_y(iced::Alignment::Center)
            .push(Space::new().width(Length::Fixed(r.depth as f32 * 14.0)))
            .push(lead)
            .push(text(r.name).size(sz(12)));
        if let Some(n) = r.files {
            // How much is under a folder, so a closed one still says something.
            row = row.push(text(format!("  {n}")).size(sz(10)).color(fomod_ink_faint()));
        }
        c = c.push(row);
    }
    if truncated {
        c = c.push(text(tf("overwrite_truncated", &[("n", &DATA_TREE_ROWS)])).size(sz(11)));
    }

    let mut col = Column::new().spacing(8).push(actions);
    if let Some(p) = prompt {
        col = col.push(p);
    }
    col.push(scrollable(c).height(Length::Fill)).into()
}

/// Format a file's modified time as `YYYY-MM-DD HH:MM` (UTC), with only std - no
/// chrono. Used for the Saves "Date" column.
pub(crate) fn format_mtime(t: std::time::SystemTime) -> String {
    let Ok(dur) = t.duration_since(std::time::UNIX_EPOCH) else {
        return "-".to_string();
    };
    let secs = dur.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (hh, mm) = ((tod / 3600) % 24, (tod % 3600) / 60);
    // Civil date from a day count (Howard Hinnant's algorithm), days since epoch.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}")
}

/// Human-readable byte size for the Saves / Downloads size columns.
pub(crate) fn format_size(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1} MiB", b / (1024.0 * 1024.0))
    } else if b >= 1024.0 {
        format!("{:.0} KiB", b / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
