//! The per-mod information dialog, Eidos's answer to MO2's `modinfodialog`: the
//! tabs for a single mod's files, conflicts, categories, notes and Nexus data.
//!
//! Split out of `main.rs` unchanged, and by far the largest single block in it.

use crate::fomod::{FOMOD_INK_FAINT, FOMOD_INK_SOFT};
use crate::theme::*;
use crate::widgets::*;
use crate::*;

/// One tab of the mod-information strip. Same contract as [`tab_btn`].
pub(crate) fn info_tab_btn<'a>(label: &'a str, tab: InfoTab, mix: f32) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .padding([4, 10])
        .on_press(Message::InfoSelectTab(tab))
        .style(move |theme, status| {
            crate::anim::mix_button(
                button::secondary(theme, status),
                button::primary(theme, status),
                mix,
            )
        })
        .into()
}

pub(crate) fn info_kv<'a>(k: &'a str, v: String) -> Element<'a, Message> {
    Row::new()
        .spacing(8)
        .push(text(k).size(12.0).width(Length::Fixed(120.0)))
        .push(text(v).size(12.0).width(Length::Fill))
        .into()
}

/// General tab: name/version/category/Nexus id/source/endorsed/tracked + counts.
pub(crate) fn info_general<'a>(app: &App, m: &ModEntry) -> Element<'a, Message> {
    let meta = app.created.as_ref().map(|inst| inst.mod_meta(&m.name));
    let files = cached_entries(app, &m.path).len();
    let mut col = Column::new().spacing(4).push(info_kv("Name", m.name.clone()));
    if let Some(meta) = &meta {
        if let Some(v) = meta.version() {
            col = col.push(info_kv("Version", v));
        }
        if let Some(nv) = meta.newest_version() {
            col = col.push(info_kv("Newest", nv));
        }
        if let Some(c) = app.meta_cache.get(&m.name).and_then(|r| r.category_name.clone()) {
            col = col.push(info_kv("Category", c));
        }
        if let Some(id) = meta.mod_id() {
            col = col.push(info_kv("Nexus id", id.to_string()));
        }
        if let Some(a) = meta.author() {
            col = col.push(info_kv("Author", a));
        }
        // The uploader is shown only when it says something the Author line does
        // not: on most mods they are the same person, and a second identical row
        // is noise pretending to be information.
        if let Some(u) = meta.uploader().filter(|u| Some(u) != meta.author().as_ref()) {
            col = col.push(info_kv("Uploaded by", u));
        }
        if let Some(src) = meta.installation_file() {
            col = col.push(info_kv("Installed from", src));
        }
        col = col
            .push(info_kv("Endorsed", if meta.endorsed() { "yes".into() } else { "no".into() }))
            .push(info_kv("Tracked", if meta.tracked() { "yes".into() } else { "no".into() }));
    }
    // A page for mods that are not on Nexus. Without it every such mod is a dead
    // end in the interface: "Visit on Nexus" only appears when a Nexus id
    // exists, so a LoversLab or GitHub mod has nowhere to go from here.
    let url_row = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text("Page").size(11.0).width(Length::Fixed(90.0)))
        .push(
            text_input("https://...", &app.url_edit)
                .on_input(Message::ModUrlChanged)
                .on_submit(Message::ModUrlSave)
                .padding(4)
                .size(12.0),
        )
        .push(
            button(text("Save").size(11.0))
                .padding([3, 8])
                .style(button::secondary)
                .on_press(Message::ModUrlSave),
        );

    col.push(info_kv("Enabled", if m.enabled { "yes".into() } else { "no".into() }))
        .push(info_kv("Files", files.to_string()))
        .push(info_kv("Folder", m.path.display().to_string()))
        .push(url_row)
        .into()
}

/// Conflicts tab: which files this mod overrides, and which it loses, by mod name.
pub(crate) fn info_conflicts<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(cmap) = &app.conflicts else {
        return text("Conflicts not computed yet.").size(12.0).into();
    };
    let origin = (i + 1) as u32;
    // Only what will be DRAWN is materialised. This runs in view(), once per
    // redraw, over every file in the conflict map - and it used to clone a path
    // String (plus a joined loser list) for every match while the panel shows at
    // most 300 of each. A texture pack winning 50k paths allocated 100k Strings
    // per pointer event; now it allocates 300 and counts the rest.
    const SHOWN: usize = 300;
    let mut wins: Vec<(String, String)> = Vec::new();
    let mut loses: Vec<(String, String)> = Vec::new();
    let (mut wins_n, mut loses_n) = (0usize, 0usize);
    for node in cmap.files.values() {
        if node.winner == origin && node.is_conflicted() {
            wins_n += 1;
            if wins.len() < SHOWN {
                let losers: Vec<&str> =
                    node.alternatives.iter().filter(|&&a| a != 0).map(|&a| cmap.name(a)).collect();
                wins.push((node.display_path.clone(), losers.join(", ")));
            }
        } else if node.winner != origin && node.winner != 0 && node.alternatives.contains(&origin) {
            loses_n += 1;
            if loses.len() < SHOWN {
                loses.push((node.display_path.clone(), cmap.name(node.winner).to_string()));
            }
        }
    }
    let mut col = Column::new().spacing(2);
    col = col.push(text(format!("Overrides ({wins_n}):")).size(13.0));
    if wins.is_empty() {
        col = col.push(text("  (none)").size(11.0));
    }
    for (p, who) in &wins {
        col = col.push(text(format!("  {p}   >   {who}")).size(11.0));
    }
    col = col
        .push(Space::new().height(Length::Fixed(8.0)))
        .push(text(format!("Overridden by ({loses_n}):")).size(13.0));
    if loses.is_empty() {
        col = col.push(text("  (none)").size(11.0));
    }
    for (p, who) in &loses {
        col = col.push(text(format!("  {p}   <   {who}")).size(11.0));
    }
    col.into()
}

/// Filetree tab: every file the mod ships, relative to its root, each with a
/// Hide / Unhide toggle. Unlike the Data tab this is the mod's REAL contents, so
/// hidden files are listed (with their suffix) and are the only place to unhide
/// one individually.
pub(crate) fn info_filetree<'a>(app: &App, i: usize, m: &ModEntry) -> Element<'a, Message> {
    let entries = cached_entries(app, &m.path);
    let hidden = entries.iter().filter(|e| path_is_hidden(e)).count();
    let summary = if hidden == 0 {
        format!("{} file(s):", entries.len())
    } else {
        format!("{} file(s), {hidden} hidden:", entries.len())
    };
    let mut col = Column::new().spacing(1).push(text(summary).size(12.0));
    let mut tools = Row::new().spacing(6).align_y(iced::Alignment::Center);
    if hidden > 0 {
        tools = tools.push(tool_btn("Unhide all", Message::RestoreHiddenFiles(i)));
    }
    tools = tools.push(tool_btn("New folder", Message::FiletreeNewFolderStart(i)));
    col = col.push(tools);
    if let Some((_, name)) = app.tree_new_folder.as_ref().filter(|(n, _)| *n == m.name) {
        col = col.push(
            Row::new()
                .spacing(6)
                .align_y(iced::Alignment::Center)
                .push(
                    text_input("folder name", name)
                        .on_input(Message::FiletreeNewFolderChanged)
                        .on_submit(Message::FiletreeNewFolderCommit)
                        .padding(4)
                        .size(11.0),
                )
                .push(
                    button(text("Create").size(10.0))
                        .padding([1, 5])
                        .style(button::primary)
                        .on_press(Message::FiletreeNewFolderCommit),
                ),
        );
    }
    for e in entries.iter().take(2000) {
        let is_hidden = path_is_hidden(e);
        let label = if is_hidden { "Unhide" } else { "Hide" };
        // Renaming replaces the last component only, so the box holds the NAME.
        let renaming =
            app.tree_rename.as_ref().is_some_and(|(mn, r)| *mn == m.name && r == e);
        let armed =
            app.tree_delete_armed.as_ref().is_some_and(|(mn, r)| *mn == m.name && r == e);
        let name_cell: Element<'a, Message> = if renaming {
            text_input("new name", &app.tree_rename_text)
                .on_input(Message::FiletreeRenameChanged)
                .on_submit(Message::FiletreeRenameCommit)
                .padding(2)
                .size(11.0)
                .width(Length::Fill)
                .into()
        } else {
            text(e.clone()).size(11.0).width(Length::Fill).into()
        };
        let mut row = Row::new().spacing(6).align_y(iced::Alignment::Center).push(name_cell);
        if renaming {
            row = row
                .push(
                    button(text("Rename").size(10.0))
                        .padding([1, 5])
                        .style(button::primary)
                        .on_press(Message::FiletreeRenameCommit),
                )
                .push(
                    button(text("Cancel").size(10.0))
                        .padding([1, 5])
                        .style(button::secondary)
                        .on_press(Message::FiletreeRenameCancel),
                );
        } else {
            row = row
                .push(
                    button(text("View").size(10.0))
                        .padding([1, 5])
                        .style(button::text)
                        .on_press_maybe(
                            resolve_in_mod(&m.path, e).map(Message::PreviewFile),
                        ),
                )
                .push(
                    button(text("Open").size(10.0))
                        .padding([1, 5])
                        .style(button::text)
                        .on_press(Message::FiletreeOpen(i, e.clone())),
                )
                .push(
                    button(text("Rename").size(10.0))
                        .padding([1, 5])
                        .style(button::text)
                        .on_press(Message::FiletreeRenameStart(i, e.clone())),
                )
                .push(
                    button(text(label).size(10.0))
                        .padding([1, 5])
                        .on_press(Message::ToggleFileHidden(i, e.clone()))
                        .style(if is_hidden { button::primary } else { button::secondary }),
                )
                // Deleting a mod's own file is the one action here that cannot
                // be undone by clicking again - hiding renames, and renaming
                // renames back. Two clicks, like everything else that removes.
                .push(
                    button(text(if armed { "Delete?" } else { "Delete" }).size(10.0))
                        .padding([1, 5])
                        .style(if armed { button::danger } else { button::secondary })
                        .on_press(if armed {
                            Message::ConfirmFiletreeDelete(m.name.clone(), e.clone())
                        } else {
                            Message::FiletreeDelete(i, e.clone())
                        }),
                );
        }
        col = col.push(row);
    }
    // Same honesty as the Data and Overwrite panels: a big texture pack blows
    // past the cap routinely, and a list that stops without saying so reads as
    // "the mod does not ship that file" to whoever scrolled looking for it.
    if entries.len() > 2000 {
        col = col.push(
            text(format!(
                "Showing the first 2000 of {} files - the rest exist but are not listed here.",
                entries.len()
            ))
            .size(11.0),
        );
    }
    col.into()
}

/// Resolve a `/`-joined relative path INSIDE a mod folder, or refuse.
///
/// Everything in the Filetree tab that writes goes through this, and it is the
/// only thing standing between a rename box and the rest of the disk. The rules
/// are all refusals, because the safe answer to anything unusual here is "no":
///
/// * No empty path, no absolute path, no Windows drive or UNC prefix.
/// * No `..` and no `.`, at any depth. Not normalised away - refused. A path
///   that needed normalising is not one this tab produced.
/// * No component that is a SYMLINK. A mod may legitimately contain one, and
///   following it would put a delete or a rename outside the mod folder
///   entirely - which is exactly how `create_dirs_no_symlinks` went wrong
///   earlier in this tree, on a symlinked `mods/`.
/// * The result must still be under `base` after joining, checked rather than
///   assumed.
///
/// Returns the absolute path, which may or may not exist yet - a new folder's
/// does not.
pub(crate) fn resolve_in_mod(base: &Path, rel: &str) -> Option<PathBuf> {
    if rel.trim().is_empty() {
        return None;
    }
    let mut out = base.to_path_buf();
    for (n, part) in rel.split('/').enumerate() {
        if part.is_empty() || part == "." || part == ".." || part.contains('\\') {
            return None;
        }
        // A path component is a NAME, never a path.
        if Path::new(part).components().count() != 1 {
            return None;
        }
        // Every component BELOW the base must not be a link. `base` itself is
        // skipped on purpose: a mod folder is very often a symlink (a pool on
        // another disk, a stow tree), and refusing there disabled every action
        // in this tab for those mods rather than protecting anything - the
        // question is whether the path leaves the mod, and the mod's own root
        // cannot.
        if n > 0 && out.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) {
            return None;
        }
        out.push(part);
    }
    // The LAST component too. Without this the walk checked every ancestor and
    // then handed back a path whose final element is a link - which a preview
    // reads through, and a delete unlinks (harmlessly) while a rename moves.
    if out.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) {
        return None;
    }
    out.starts_with(base).then_some(out)
}

/// Whether a `/`-joined relative path names a hidden entry, or lies under one.
pub(crate) fn path_is_hidden(rel: &str) -> bool {
    rel.split('/').any(eidos_core::is_hidden_name)
}

/// INI Tweaks tab: the fragments this mod ships in its `INI Tweaks/` folder, each
/// individually enabled. Enabled fragments are merged into the profile's game INI
/// at launch, in mod priority order, and undone again when the run's INIs are
/// captured back - so a tweak stays a tweak instead of quietly becoming a setting.
pub(crate) fn info_ini_tweaks<'a>(app: &App, i: usize, m: &ModEntry) -> Element<'a, Message> {
    let available = eidos_instance::available_ini_tweaks(&m.path);
    if available.is_empty() {
        return Column::new()
            .spacing(6)
            .push(text("This mod ships no INI tweaks.").size(12.0))
            .push(
                text("A mod with tweaks has an 'INI Tweaks' folder of small INI fragments.")
                    .size(10.0),
            )
            .into();
    }
    let enabled: Vec<String> =
        app.created.as_ref().map(|inst| inst.mod_meta(&m.name).ini_tweaks().to_vec()).unwrap_or_default();

    let mut col = Column::new().spacing(3).push(
        text("Enabled fragments are merged into this profile's game INI at launch.").size(11.0),
    );
    for name in available {
        let on = enabled.iter().any(|e| e.eq_ignore_ascii_case(&name));
        let label = name.clone();
        col = col.push(
            checkbox(on).label(label)
                .on_toggle(move |_| Message::ToggleIniTweak(i, name.clone()))
                .size(13.0)
                .text_size(12.0),
        );
    }
    col.into()
}

/// Notes tab: an editable note persisted to the mod's meta.ini.
pub(crate) fn info_notes<'a>(app: &App) -> Element<'a, Message> {
    Column::new()
        .spacing(8)
        .push(text("Note (saved to the mod's meta.ini):").size(12.0))
        .push(
            text_input("Add a note...", &app.notes_edit)
                .on_input(Message::NotesChanged)
                .on_submit(Message::NotesSave)
                .padding(6)
                .size(12.0),
        )
        .push(tool_btn("Save note", Message::NotesSave))
        .into()
}

/// MO2's per-mod info dialog: a centered modal with General / Conflicts /
/// Filetree / Notes tabs.
pub(crate) fn mod_info_dialog<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(m) = app.mods.get(i) else {
        return Space::new().width(Length::Shrink).height(Length::Shrink).into();
    };

    let title = Row::new()
        .spacing(8)
        .push(text(m.name.clone()).size(16.0).width(Length::Fill))
        .push(
            button(text("Close").size(12.0))
                .padding([3, 10])
                .on_press(Message::CloseInfo)
                .style(button::secondary),
        );

    let tabs = Row::new()
        .spacing(4)
        .push(info_tab_btn("General", InfoTab::General, info_mix(app, InfoTab::General)))
        .push(info_tab_btn("Conflicts", InfoTab::Conflicts, info_mix(app, InfoTab::Conflicts)))
        .push(info_tab_btn("Filetree", InfoTab::Filetree, info_mix(app, InfoTab::Filetree)))
        .push(info_tab_btn("INI Tweaks", InfoTab::IniTweaks, info_mix(app, InfoTab::IniTweaks)))
        .push(info_tab_btn("Notes", InfoTab::Notes, info_mix(app, InfoTab::Notes)));

    let content = match app.info_tab {
        InfoTab::General => info_general(app, m),
        InfoTab::Conflicts => info_conflicts(app, i),
        InfoTab::Filetree => info_filetree(app, i, m),
        InfoTab::IniTweaks => info_ini_tweaks(app, i, m),
        InfoTab::Notes => info_notes(app),
    };

    let card = Column::new()
        .spacing(10)
        .push(title)
        .push(tabs)
        .push(scrollable(content).height(Length::Fill));

    container(card)
        .width(Length::Fixed(660.0))
        .height(Length::Fixed(460.0))
        .padding(16)
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xEC, 0xDF, 0xC2))),
            border: Border {
                color: Color::from_rgb8(0x6E, 0x24, 0x2E),
                width: 2.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}

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
            text_input("Filter files", &app.data_query)
                .on_input(Message::DataQueryChanged)
                .padding(5)
                .size(12.0)
                .width(Length::Fill),
        )
        .push(
            button(text("Conflicts only").size(11.0))
                .padding([3, 8])
                .style(if app.data_conflicts_only { button::primary } else { button::secondary })
                .on_press(Message::DataToggleConflictsOnly),
        )
        .push(tool_btn("Expand all", Message::DataExpandAll))
        .push(tool_btn("Collapse all", Message::DataCollapseAll));

    let header = Row::new()
        .spacing(6)
        .push(text("Name").size(11.0).width(Length::FillPortion(3)))
        .push(text("Provided by").size(11.0).width(Length::FillPortion(2)))
        .push(text("Size").size(11.0).width(Length::Fixed(C_SIZE)))
        .push(text("Modified").size(11.0).width(Length::Fixed(C_DATE)))
        .push(text("").size(11.0).width(Length::Fixed(C_ACTION)));

    // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
    let rows = data_tree_rows(app, DATA_TREE_ROWS);
    if rows.is_empty() {
        list = list.push(
            text(if app.data_query.trim().is_empty() && !app.data_conflicts_only {
                "(empty)"
            } else {
                "Nothing in the merged view matches."
            })
            .size(12.0),
        );
    }
    let truncated = rows.len() >= DATA_TREE_ROWS;
    for (idx, r) in rows.into_iter().enumerate() {
        // A folder gets a clickable disclosure triangle; a file gets a spacer of
        // the same width so names stay in one column.
        let lead: Element<'a, Message> = if r.row.is_dir {
            let glyph = if app.data_expanded.contains(&r.rel) { "\u{25BE}" } else { "\u{25B8}" };
            button(text(glyph).size(11.0))
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
            text(format!("{} *", r.row.name)).size(12.0).color(CONFLICT_WINS_FG).into()
        } else {
            text(r.row.name.clone()).size(12.0).into()
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
        let owner = app.mods.iter().position(|m| !m.is_separator() && m.name == r.row.source);
        let mut action = Row::new().spacing(3).align_y(iced::Alignment::Center);
        // Reveal works on every row: the whole reason to look at this tree is to
        // find out WHICH copy of a file the game gets, and the next question is
        // always "where is it".
        action = action.push(
            button(text("Reveal").size(10.0))
                .padding([1, 5])
                .on_press(Message::DataReveal(r.row.real.clone()))
                .style(button::secondary),
        );
        if !r.row.is_dir {
            action = action.push(
                button(text("View").size(10.0))
                    .padding([1, 5])
                    .on_press(Message::PreviewFile(r.row.real.clone()))
                    .style(button::text),
            );
        }
        if let Some(i) = owner {
            action = action.push(
                button(text("Hide").size(10.0))
                    .padding([1, 5])
                    .on_press(Message::ToggleFileHidden(i, r.rel.clone()))
                    .style(button::secondary),
            );
        }

        let row = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(container(name).width(Length::FillPortion(3)))
            .push(text(r.row.source.clone()).size(12.0).width(Length::FillPortion(2)))
            .push(
                text(r.row.size.map(format_size).unwrap_or_default())
                    .size(11.0)
                    .width(Length::Fixed(C_SIZE))
                    .align_x(iced::alignment::Horizontal::Right),
            )
            .push(
                text(r.row.mtime.map(format_when).unwrap_or_default())
                    .size(11.0)
                    .width(Length::Fixed(C_DATE)),
            )
            .push(container(action).width(Length::Fixed(C_ACTION)));
        list = list.push(striped(row.into(), idx % 2 == 0));
    }
    if truncated {
        list = list.push(
            text(format!("Showing the first {DATA_TREE_ROWS} entries - collapse a folder to see more."))
                .size(11.0),
        );
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
    let secs = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // The backups list already owns this arithmetic (Howard Hinnant's
    // civil_from_days); it renders `YYYY-MM-DD HH:MM` and the date is the half
    // a file column has room for.
    eidos_instance::format_stamp(secs).split(' ').next().unwrap_or_default().to_string()
}

pub(crate) fn overwrite_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text("No instance open.").into();
    };
    let dir = inst.overwrite_dir();
    let actions = Row::new()
        .spacing(6)
        .push(
            text("Everything the game writes (configs, new saves, generated files) lands here.")
                .size(12.0)
                .width(Length::Fill),
        )
        // MO2's central Overwrite workflow: turn what the game/tools generated into
        // a real, orderable mod instead of only being able to delete it.
        .push(tool_btn("Create mod...", Message::OverwriteToModStart))
        // MO2's "Sync to Mods": send each file back to the mod that already
        // provides that path, instead of bundling everything into a new one.
        .push(
            button(
                text(if app.confirm_sync { "Confirm send back?" } else { "Send back to mods" })
                    .size(12.0),
            )
            .padding(5)
            .on_press(Message::OverwriteSyncToMods)
            .style(if app.confirm_sync { button::danger } else { button::secondary }),
        )
        .push(tool_btn("Open folder", Message::OpenFolder(dir.clone())))
        .push(
            button(text(if app.confirm_clear { "Confirm clear?" } else { "Clear" }).size(12.0))
                .padding(5)
                .on_press(Message::ClearOverwrite)
                .style(if app.confirm_clear { button::danger } else { button::secondary }),
        );

    // The inline name prompt, shown while "Create mod..." is armed. Typing an
    // existing mod's name merges into it (MO2's "move content to mod").
    let prompt: Option<Element<'a, Message>> = app.overwrite_to_mod.as_ref().map(|name| {
        let exists = inst.mods_dir().join(name.trim()).exists();
        let hint = if exists {
            "merges into that existing mod"
        } else {
            "creates a new mod at the top of the priority order"
        };
        Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text("Mod name").size(12.0))
            .push(
                text_input("Mod name", name)
                    .on_input(Message::OverwriteToModName)
                    .on_submit(Message::OverwriteToModCommit)
                    .padding(5)
                    .size(12.0)
                    .width(Length::Fixed(260.0)),
            )
            .push(text(hint).size(10.0).width(Length::Fill))
            .push(
                button(text("Create").size(12.0))
                    .padding([4, 12])
                    .on_press(Message::OverwriteToModCommit)
                    .style(button::primary),
            )
            .push(tool_btn("Cancel", Message::OverwriteToModCancel))
            .into()
    });

    // A tree, not 4902 full paths one under the other. Same grammar as the Data
    // tab - triangle, indent, name - because they are the same gesture.
    let entries = cached_entries(app, &dir);
    let mut c = Column::new().spacing(2);
    if entries.is_empty() {
        c = c.push(text("(empty)").size(12.0));
    } else {
        c = c.push(text(format!("{} file(s):", entries.len())).size(11.0));
    }
    let rows = overwrite_tree_rows(app, &entries, DATA_TREE_ROWS);
    let truncated = rows.len() >= DATA_TREE_ROWS;
    for r in rows {
        let lead: Element<'a, Message> = match r.files {
            Some(_) => {
                let glyph =
                    if app.overwrite_expanded.contains(&r.rel) { "\u{25BE}" } else { "\u{25B8}" };
                button(text(glyph).size(11.0))
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
            .push(text(r.name).size(11.5));
        if let Some(n) = r.files {
            // How much is under a folder, so a closed one still says something.
            row = row.push(text(format!("  {n}")).size(10.0).color(FOMOD_INK_FAINT));
        }
        c = c.push(row);
    }
    if truncated {
        c = c.push(
            text(format!("Showing the first {DATA_TREE_ROWS} rows - collapse a folder to see more."))
                .size(11.0),
        );
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

/// The Saves tab: the active profile's save files (name / date / size) plus
/// open-folder + per-save delete. MO2's savegame list.
pub(crate) fn saves_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text("No instance open.").into();
    };
    let dir = inst.active().saves_dir();

    let header = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text("Save").size(13.0))
        .push(Space::new().width(Length::Fill))
        .push(button(text("Open folder").size(11.0)).padding(4).on_press(Message::OpenFolder(dir.clone())))
        .push(button(text("Refresh").size(11.0)).padding(4).on_press(Message::RefreshSaves));

    let col_header = Row::new()
        .spacing(8)
        .push(text("Name").size(11.0).width(Length::Fill))
        .push(text("Date").size(11.0).width(Length::Fixed(130.0)))
        .push(text("Size").size(11.0).width(Length::Fixed(80.0)))
        .push(Space::new().width(Length::Fixed(80.0)));

    let mut rows = Column::new().spacing(2);
    if app.saves.is_empty() {
        rows = rows.push(
            text("(no saves yet) Saves your game writes for this profile appear here.")
                .size(12.0),
        );
    }
    for (i, save) in app.saves.iter().take(SAVES_LIST_CAP).enumerate() {
        let armed = app.confirm_delete_save == Some(i);
        let del = button(text(if armed { "Confirm?" } else { "Delete" }).size(11.0))
            .padding(4)
            .on_press(if armed { Message::ConfirmDeleteSave(i) } else { Message::DeleteSave(i) })
            .style(if armed { button::danger } else { button::secondary });
        // The name is the click target for the details pane; the row's other
        // controls keep working (a Delete click must not also select).
        let name = button(text(save.filename.clone()).size(12.0))
            .padding(0)
            .width(Length::Fill)
            .on_press(Message::SelectSave(i))
            .style(button::text);
        // A tick box rather than Ctrl+click: the batch here is a deliberate
        // "these ones", built over several clicks, and a modifier that silently
        // clears the set on a plain click is the wrong shape for that.
        let picked = app.selected_saves.contains(&i);
        let tick = button(text(if picked { "[x]" } else { "[ ]" }).size(11.0).font(iced::Font::MONOSPACE))
            .padding([0, 4])
            .on_press(Message::SaveToggleSelect(i))
            .style(button::text);
        let row = Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .push(tick)
            .push(name)
            .push(text(format_mtime(save.mtime)).size(11.0).width(Length::Fixed(130.0)))
            .push(text(format_size(save.size)).size(11.0).width(Length::Fixed(80.0)))
            .push(container(del).width(Length::Fixed(80.0)));
        rows = rows.push(list_row(
            container(row).padding(3).into(),
            i % 2 == 0,
            app.selected_save == Some(i),
            // Saves do not fight over files, and carry no user colour.
            None,
            None,
        ));
    }

    // The batch bar, shown only when something is ticked - it has nothing to say
    // otherwise, and a permanent row of dead buttons is worse than none.
    let batch: Option<Element<'a, Message>> = (!app.selected_saves.is_empty()).then(|| {
        let n = app.selected_saves.len();
        let mut bar = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text(format!("{n} selected")).size(11.0));
        // Copy to another profile - MO2's Transfer Save Games. Only profiles
        // that are not this one: copying a save onto itself is not a thing.
        if app.created.is_some() {
            // The memoised list, not a fresh read_dir: this bar is drawn on
            // every frame while anything is selected.
            let (names, here) = cached_profiles(app);
            for name in names.into_iter().filter(|p| *p != here) {
                bar = bar.push(
                    button(text(format!("Copy to {name}")).size(11.0))
                        .padding([3, 8])
                        .style(button::secondary)
                        .on_press(Message::SavesCopyToProfile(name)),
                );
            }
        }
        bar = bar.push(Space::new().width(Length::Fill)).push(
            button(
                text(if app.confirm_saves_delete {
                    format!("Confirm - delete {n}?")
                } else {
                    format!("Delete {n}")
                })
                .size(11.0),
            )
            .padding([3, 8])
            .on_press(Message::SavesDeleteSelected)
            .style(if app.confirm_saves_delete { button::danger } else { button::secondary }),
        );
        bar.into()
    });

    let mut list = Column::new()
        .spacing(6)
        .push(header)
        .push(text(dir.display().to_string()).size(10.0));
    if let Some(bar) = batch {
        list = list.push(bar);
    }
    let list = list.push(col_header).push(scrollable(rows).height(Length::Fill));

    match app.selected_save.and_then(|i| app.saves.get(i)) {
        Some(save) => Row::new()
            .spacing(8)
            .push(container(list).width(Length::FillPortion(3)))
            .push(container(save_details(app, save)).width(Length::FillPortion(2)))
            .into(),
        None => list.into(),
    }
}

/// The details pane for the selected save: who and where, and - the reason this
/// exists - which of the plugins baked into the save are no longer active.
///
/// MO2 shows this before you load: a save carries the plugin list it was written
/// with, and loading it without those plugins is how a playthrough loses its
/// contents (or crashes on the way in).
pub(crate) fn save_details<'a>(app: &App, save: &eidos_instance::SaveEntry) -> Element<'a, Message> {
    let mut col = Column::new().spacing(4).push(text(save.filename.clone()).size(13.0));

    let info = match app.save_info.as_ref().filter(|(p, _)| *p == save.path) {
        Some((_, Ok(info))) => info,
        Some((_, Err(e))) => {
            return col
                .push(text(format!("Cannot read this save: {e}")).size(11.0))
                .push(text("The list below is unavailable; the file itself is untouched.").size(10.0))
                .into();
        }
        // Parsed on selection, so this only shows for the frame in between.
        None => return col.push(text("Reading...").size(11.0)).into(),
    };

    // The save's own screenshot, above the facts: it is what identifies a save
    // at a glance, which is the entire reason MO2 shows it.
    if let Some((_, Some(handle))) = app.save_shot.as_ref().filter(|(p, _)| *p == save.path) {
        col = col.push(
            container(iced::widget::image(handle.clone()).width(Length::Fill))
                // Its own aspect ratio is preserved by `image`; the cap stops a
                // wide shot pushing the facts off the bottom of the pane.
                .max_height(200.0),
        );
    }

    let mut facts: Vec<(&'static str, String)> = vec![
        ("Character", format!("{} (level {})", info.player_name, info.level)),
        ("Location", info.location.clone()),
        ("In-game date", info.game_date.clone()),
    ];
    if let Some((d, h, m)) = info.playtime() {
        facts.push(("Played", format!("{d}d {h}h {m}m")));
    }
    facts.push(("Save", format!("#{}", info.save_number)));
    // Fallout writes the runtime it was saved by; Skyrim does not, and leaves this
    // empty. Worth a row of its own: "this save came from another game build" is
    // the answer to a whole class of "my DLL plugins stopped loading" sessions.
    if !info.game_version.is_empty() {
        facts.push(("Game build", info.game_version.clone()));
    }
    facts.push(("Plugins", format!("{} + {} light", info.plugins.len(), info.light_plugins.len())));
    for (k, v) in facts {
        col = col.push(info_kv(k, v));
    }

    let missing = &app.save_missing;
    col = col.push(Space::new().height(Length::Fixed(6.0)));
    if missing.is_empty() {
        return col
            .push(text("Every plugin this save uses is active.").size(11.0))
            .push(
                text(if info.truncated {
                    "(the save's plugin list was truncated, so this is advisory)"
                } else {
                    ""
                })
                .size(10.0),
            )
            .into();
    }

    col = col.push(text(format!("{} plugin(s) missing:", missing.len())).size(12.0));
    for m in missing.iter().take(40) {
        let what = match m.state {
            eidos_gamefeatures::SavePluginState::Inactive => "inactive",
            eidos_gamefeatures::SavePluginState::Absent => "not installed",
        };
        let who = if m.providers.is_empty() {
            "  no mod here provides it".to_string()
        } else {
            format!("  in: {}", m.providers.join(", "))
        };
        col = col.push(text(format!("{} ({what})", m.name)).size(11.0)).push(text(who).size(10.0));
    }
    // Only offer the fix when something on disk can actually supply the plugins;
    // otherwise the button would enable nothing and look broken.
    let fixable = missing.iter().any(|m| !m.providers.is_empty());
    if fixable {
        col = col
            .push(Space::new().height(Length::Fixed(4.0)))
            .push(tool_btn("Enable the mods this save needs", Message::FixSaveMods));
    }
    if info.truncated {
        col = col.push(
            text("The save's plugin list was truncated, so treat this as advisory.").size(10.0),
        );
    }
    col.into()
}

/// A short status label for a download row (MO2's downloads state column).
pub(crate) fn download_state_label(state: DownloadState) -> &'static str {
    match state {
        DownloadState::Untracked => "-",
        DownloadState::Downloading => "Downloading",
        DownloadState::Stalled => "Stalled",
        DownloadState::Paused => "Paused",
        DownloadState::Ready => "Ready",
        DownloadState::Installed => "Installed",
        DownloadState::Uninstalled => "Uninstalled",
    }
}

/// The colour of the status word, following MO2's own scheme
/// (downloadlist.cpp:202): green for the one state that is asking to be acted
/// on, amber for a mod that was installed and then removed, and NOTHING for
/// "Installed" - a finished job should not keep waving.
pub(crate) fn download_state_color(state: DownloadState, theme: &Theme) -> Option<Color> {
    match state {
        DownloadState::Ready => Some(theme.palette().success),
        DownloadState::Uninstalled => Some(theme.palette().warning),
        // Burgundy for the one that is happening right now, amber for one that
        // stopped and is waiting to be resumed.
        DownloadState::Downloading => Some(theme.palette().primary),
        DownloadState::Stalled => Some(theme.palette().warning),
        // Paused is not a warning: nothing went wrong, the user asked for it.
        DownloadState::Paused => None,
        DownloadState::Installed | DownloadState::Untracked => None,
    }
}

// Downloads column widths, declared once so the header and the rows cannot drift
// apart. Each is sized to its widest real content and no more: every pixel they
// do not take goes to the name, which is the only column whose content has no
// bound - Nexus file names run to eighty characters.
//
// They were 80/80/90/150, which is 400px of a pane that is roughly 500 wide once
// the mod list has its half. That left about 68px for the name, so "Dynamic Armor
// Physics" came out three lines tall. The action column was the worst of it: 150
// reserved for two buttons that measure about 100.
pub(crate) const DL_C_VERSION: f32 = 56.0; // "1.0.1"
pub(crate) const DL_C_SIZE: f32 = 66.0; // "10.3 MiB"
pub(crate) const DL_C_STATUS: f32 = 66.0; // "Installed"
// Sized on the WIDEST pair the column can ever hold at once, which is not the
// resting state: "Reinstall" beside Delete armed as "Confirm?". Sizing it on
// "Install" + "Delete" would clip the two labels that only appear when something
// is at stake.
pub(crate) const DL_C_ACTIONS: f32 = 128.0;
/// Fixed height for the action cell, so a row does not change height at the one
/// moment it changes CONTENT: a finishing download swaps its progress bar for
/// two buttons, and if those measured differently the whole list below would
/// jump at exactly the instant the user was watching it.
pub(crate) const DL_ACTION_H: f32 = 24.0;
/// Fixed width for the speed/percentage readout, so the progress bar beside it
/// keeps the same geometry from one tick to the next. Fits "12.3 MiB/s".
pub(crate) const DL_READOUT_W: f32 = 54.0;

pub(crate) fn downloads_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text("No instance open.").into();
    };
    let dir = inst.downloads_dir();

    let header = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text("Downloads").size(13.0))
        .push(Space::new().width(Length::Fill))
        .push(button(text("Open folder").size(11.0)).padding(4).on_press(Message::OpenFolder(dir.clone())))
        .push(button(text("Refresh").size(11.0)).padding(4).on_press(Message::RefreshDownloads));

    // MO2 treats this tab as an archive library rather than a transfer queue,
    // and a library needs to be searchable and orderable to be worth keeping.
    let installed_here =
        app.downloads.iter().filter(|r| r.state == DownloadState::Installed).count();
    let mut tools = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(
            text_input("Filter by name...", &app.dl_filter)
                .on_input(Message::DownloadFilterChanged)
                .padding(4)
                .size(11.0)
                .width(Length::Fill),
        )
        .push(
            pick_list(DownloadSort::ALL.to_vec(), Some(app.dl_sort), Message::DownloadSortChanged)
                .text_size(11.0)
                .padding(4),
        )
        .push(
            button(text(if app.dl_show_hidden { "Hide hidden" } else { "Show hidden" }).size(11.0))
                .padding(4)
                .style(button::secondary)
                .on_press(Message::ToggleShowHiddenDownloads),
        );
    // Offered only when it would do something, and it names the count rather
    // than saying "installed" - a bulk delete has to say how many.
    if installed_here > 0 {
        let armed = app.confirm_purge_installed;
        tools = tools.push(
            button(
                text(if armed {
                    format!("Delete {installed_here}?")
                } else {
                    format!("Remove {installed_here} installed")
                })
                .size(11.0),
            )
            .padding(4)
            .style(if armed { button::danger } else { button::secondary })
            .on_press(if armed {
                Message::ConfirmPurgeInstalled
            } else {
                Message::PurgeInstalledDownloads
            }),
        );
    }

    let col_header = Row::new()
        .spacing(8)
        .push(text("Name").size(11.0).width(Length::Fill))
        .push(text("Version").size(11.0).width(Length::Fixed(DL_C_VERSION)))
        .push(text("Size").size(11.0).width(Length::Fixed(DL_C_SIZE)))
        .push(text("Status").size(11.0).width(Length::Fixed(DL_C_STATUS)))
        .push(Space::new().width(Length::Fixed(DL_C_ACTIONS)));

    let mut rows = Column::new().spacing(2);
    if app.downloads.is_empty() {
        rows = rows.push(
            text("No downloads yet. On Nexus, use \"Mod Manager Download\" once the handler is registered (eidos nxm --register), or drop archives here.")
                .size(11.0),
        );
    }
    for (i, row) in app.downloads.iter().enumerate() {
        let armed = app.confirm_delete_download.as_deref() == Some(row.name.as_str());
        let identifying = app.identifying_download.as_deref() == Some(row.name.as_str());
        // Two action buttons: Install (re-run the installer) and Delete.
        // MO2 keeps Install available on an already-installed archive
        // (downloadlistview.cpp:230, `state >= STATE_READY`) because re-running a
        // FOMOD with different answers is a real thing to want. What it does NOT
        // do is present it as the next step: its Install lives in a context menu,
        // and it colours the STATUS rather than the action.
        //
        // So keep the action, drop the shouting. Burgundy means "this is what to
        // do here"; on a row that is already installed, that was a lie, and the
        // label said "Install" for something that would install it a second time.
        let arriving = matches!(
            row.state,
            DownloadState::Downloading | DownloadState::Stalled | DownloadState::Paused
        );
        let installed = row.state == DownloadState::Installed;
        // Nothing can be installed out of a partial file, so while one is
        // arriving the action column carries the progress instead of two buttons
        // that would either lie or refuse. It is also the widest column, which is
        // what a bar wants.
        let actions: Element<'a, Message> = if arriving {
            // Stopped either way - the difference is only WHY, which the status
            // column already says. Both offer the same two things to do.
            let stalled = matches!(row.state, DownloadState::Stalled | DownloadState::Paused);
            let frac = if row.total > 0 {
                (row.downloaded as f32 / row.total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let label = if stalled {
                match row.total {
                    0 => format_size(row.downloaded),
                    _ => format!("{:.0}%", frac * 100.0),
                }
            } else {
                match (row.total, row.speed) {
                // No total: an older sidecar, from before the size was recorded.
                // Say how much has arrived rather than invent a percentage.
                (0, _) => format_size(row.downloaded),
                (_, Some(bps)) => format!("{}/s", format_size(bps as u64)),
                // Between the first sighting and the next tick there is no rate
                // yet. A "0 B/s" here would read as stopped, which it is not.
                (_, None) => format!("{:.0}%", frac * 100.0),
                }
            };
            // FIXED width for the readout. The bar takes what is left, so a
            // label that measures differently every tick - "9.8 MiB/s" then
            // "12.3 MiB/s" then "985 KiB/s" - would resize the bar under a
            // monotonic value, and the fill would visibly step BACKWARDS while
            // the download went forwards.
            let readout = text(label)
                .size(9.5)
                .color(FOMOD_INK_SOFT)
                .width(Length::Fixed(DL_READOUT_W))
                .align_x(iced::alignment::Horizontal::Right);
            let cell = Row::new()
                .spacing(5)
                .align_y(iced::Alignment::Center)
                .width(Length::Fixed(DL_C_ACTIONS))
                .height(Length::Fixed(DL_ACTION_H));
            // A live transfer offers nothing: there is nothing to do but wait,
            // and Install on a partial file would be a lie. A STALLED one has to
            // be removable, or an abandoned download becomes a row that can never
            // be got rid of - and its partial is invisible in a file manager too,
            // having no archive extension.
            //
            // The button and the bar do not share the cell: squeezed beside it a
            // bar would be twenty pixels wide, which says less than the number
            // next to it already does. Stalled gets the number and the button.
            if stalled {
                cell.push(readout)
                    .push(
                        button(text("Resume").size(10.0))
                            .padding(3)
                            .on_press(Message::ResumeDownload(row.name.clone()))
                            .style(button::primary),
                    )
                    .push(
                        button(text(if armed { "Confirm?" } else { "Delete" }).size(10.0))
                            .padding(3)
                            .on_press(if armed {
                                Message::ConfirmDeleteDownload(row.name.clone())
                            } else {
                                Message::DeleteDownload(row.name.clone())
                            })
                            .style(if armed { button::danger } else { button::secondary }),
                    )
                    .into()
            } else {
                cell.push(
                    // iced 0.14 names a bar's axes `length` (along) and `girth`
                    // (across), not width/height - it can be vertical.
                    iced::widget::progress_bar(0.0..=1.0, frac)
                        .length(Length::Fill)
                        .girth(Length::Fixed(7.0)),
                )
                .push(readout)
                // A live transfer gets exactly one control: stop it. Delete is
                // deliberately absent - a partial cannot be installed, and the
                // safe order is always pause first, then decide.
                .push(
                    button(text("Pause").size(10.0))
                        .padding(3)
                        .on_press(Message::PauseDownload(row.name.clone()))
                        .style(button::secondary),
                )
                .into()
            }
        } else {
            let install = button(text(if installed { "Reinstall" } else { "Install" }).size(11.0))
                .padding(4)
                .on_press(Message::ModPicked(Some(row.path.clone())))
                .style(if installed { button::secondary } else { button::primary });
            let del = button(text(if armed { "Confirm?" } else { "Delete" }).size(11.0))
                .padding(4)
                .on_press(if armed {
                    Message::ConfirmDeleteDownload(row.name.clone())
                } else {
                    Message::DeleteDownload(row.name.clone())
                })
                .style(if armed { button::danger } else { button::secondary });
            let mut actions = Row::new()
                .spacing(4)
                .align_y(iced::Alignment::Center)
                .width(Length::Fixed(DL_C_ACTIONS))
                .height(Length::Fixed(DL_ACTION_H));
            // An archive with no sidecar has no mod id, so no version, no update
            // check and no Nexus page - and every archive copied in by hand
            // arrives that way. Nexus can identify it from its MD5, which is the
            // only route back (MO2 calls this Query Metadata).
            // On the ID, not on the STATE: hiding a hand-copied archive writes a
            // sidecar, which stops it being Untracked - and took the only route
            // back to a mod id with it. What Identify is for is an archive
            // nobody has identified, which is exactly "no mod id".
            if row.mod_id.is_none() {
                actions = actions.push(
                    button(text(if identifying { "..." } else { "Identify" }).size(11.0))
                        .padding(4)
                        .style(button::secondary)
                        .on_press_maybe(
                            (!identifying).then(|| Message::IdentifyDownload(row.name.clone())),
                        ),
                );
            }
            // The mod's page, straight from the row. The sidecar has known the id
            // since the download started; nothing offered it.
            if let (Some(id), Some(domain)) =
                (row.mod_id, selected_game(app).map(|g| g.def.nexus_game).filter(|d| !d.is_empty()))
            {
                actions = actions.push(
                    button(text("Nexus").size(11.0))
                        .padding(4)
                        .style(button::text)
                        .on_press(Message::OpenUrl(format!(
                            "https://www.nexusmods.com/{domain}/mods/{id}"
                        ))),
                );
            }
            // Hiding is not deleting, which is why both are offered: an archive
            // somebody is done with but does not want to lose is the common case
            // this list had no answer for.
            let hide = button(text(if row.hidden { "Unhide" } else { "Hide" }).size(11.0))
                .padding(4)
                .style(button::text)
                .on_press(Message::HideDownload(row.name.clone()));
            actions.push(hide).push(install).push(del).into()
        };
        // Prefer the friendly Nexus mod name when present, else the file name.
        let display = row.mod_name.clone().unwrap_or_else(|| row.name.clone());
        let r = Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .push(text(display).size(12.0).width(Length::Fill))
            .push(text(row.version.clone()).size(11.0).width(Length::Fixed(DL_C_VERSION)))
            .push(text(format_size(row.size)).size(11.0).width(Length::Fixed(DL_C_SIZE)))
            .push({
                let st = row.state;
                let label = match (st, row.total) {
                    (DownloadState::Downloading, t) if t > 0 => {
                        format!("{:.0}%", (row.downloaded as f64 / t as f64) * 100.0)
                    }
                    _ => download_state_label(st).to_string(),
                };
                text(label)
                    .size(11.0)
                    .width(Length::Fixed(DL_C_STATUS))
                    .style(move |t: &Theme| iced::widget::text::Style {
                        color: download_state_color(st, t),
                    })
            })
            .push(actions);
        // Pressing the row arms a drag onto the mod list, so a download can be
        // installed AT a priority instead of always landing at the end. The
        // Install / Delete buttons swallow their own presses, exactly as the mod
        // rows do, so this never steals a click meant for them. A partial has
        // nothing to install, and the handler refuses it.
        let r: Element<'a, Message> = if arriving {
            container(r).padding(3).into()
        } else {
            mouse_area(container(r).padding(3)).on_press(Message::DownloadDragStart(i)).into()
        };
        rows = rows.push(striped(r, i % 2 == 0));
    }

    Column::new()
        .spacing(6)
        .push(header)
        .push(text(dir.display().to_string()).size(10.0))
        .push(tools)
        .push(col_header)
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

/// How serious a diagnostic is: `Problem` needs action (it will break or lose
/// something), `Advice` is worth knowing, `Ok` is a passing check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagLevel {
    Problem,
    Advice,
    Ok,
}

/// One health check: what it found, and what to do about it.
#[derive(Clone)]
pub(crate) struct Diagnostic {
    pub(crate) level: DiagLevel,
    pub(crate) title: String,
    pub(crate) detail: String,
    /// One-click remedies, rendered as buttons on the card. Most checks only
    /// inform; the ones that can FIX what they found carry the fix with them, so
    /// recovery is not a file-manager expedition. More than one when the finding
    /// has two honest outcomes (restore vs accept).
    pub(crate) actions: Vec<(&'static str, Message)>,
}

/// Run every health check for the current setup - MO2's problems panel, plus the
/// Linux-specific ones MO2 never needed (the launch capability above all, which
/// silently disables FUSE passthrough after each rebuild).
pub(crate) fn diagnostics(app: &App) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();

    // Eidos's own machinery first, before anything about the user's mods. When
    // the prefix is unusable nothing below is actionable: a tool that cannot
    // find the game will not be fixed by tidying an archive, and a user reading
    // top to bottom should meet the cause before the symptoms.
    if let Some(f) = crate::state::selected_game(app).and_then(crate::health::prefix_facts) {
        out.extend(crate::health::prefix_checks(&f));
    }
    out.extend(crate::health::nexus_checks(&crate::health::NexusFacts {
        signed_in: app.nexus_account.is_some(),
        last_error: app.nexus_error.clone(),
    }));

    // What the user's own `diagnose` extensions reported, FIRST and attributed by
    // name. First because someone who wrote a check wants to see it; attributed
    // because a row that reads like one of Eidos's own would put its own
    // authority behind something Eidos did not check.
    for (who, f) in &app.addon_findings {
        out.push(Diagnostic {
            level: match f.level {
                eidos_addons::FindingLevel::Problem => DiagLevel::Problem,
                eidos_addons::FindingLevel::Advice => DiagLevel::Advice,
                eidos_addons::FindingLevel::Ok => DiagLevel::Ok,
            },
            title: format!("{} - {}", who, f.title),
            detail: f.detail.clone(),
            actions: Vec::new(),
        });
    }

    // First, because while it is showing nothing else in this tab is trustworthy:
    // the mod list Eidos is working from does not match what is on disk, so the
    // conflict map, the load order and the layer stack are all built from a
    // partial picture. Saving is refused for as long as this is true.
    if let Some(why) = app.created.as_ref().and_then(|i| i.modlist_checked().1.reason().map(str::to_string)) {
        out.push(Diagnostic {
            level: DiagLevel::Problem,
            title: "The mod list does not match the mods folder".to_string(),
            detail: format!(
                "{why} Eidos will not save the mod list until this is resolved, so the order and \
                 enabled state on disk are safe. If a drive holds your mods, mount it and press F5."
            ),
            actions: Vec::new(),
        });
    }

    // The launch capability is optional: it only gates FUSE passthrough, which is
    // off by default because it stops the game opening its own archives and
    // plugins. So this is only worth a Problem when passthrough was asked for.
    if passthrough_requested() {
        out.push(if app.cap_missing {
            Diagnostic {
                level: DiagLevel::Problem,
                title: "Passthrough requested but unavailable (launch capability missing)"
                    .to_string(),
                detail: format!(
                    "EIDOS_FUSE_PASSTHROUGH is set, but the launch binary has no CAP_SYS_ADMIN, so reads go through the daemon anyway. Run:  sudo setcap cap_sys_admin+ep {}  then press F5. Every rebuild of that binary wipes it.",
                    find_eidos_binary().display()
                ),
                actions: Vec::new(),
            }
        } else {
            Diagnostic {
                level: DiagLevel::Advice,
                title: "FUSE passthrough is ON (opt-in)".to_string(),
                detail: "Reads go straight to the backing file. Measured on Skyrim SE, this makes the game fail to open its archives and plugins, so mods do not load. Unset EIDOS_FUSE_PASSTHROUGH if content goes missing in-game.".to_string(),
                actions: Vec::new(),
            }
        });
    } else {
        out.push(Diagnostic {
            level: DiagLevel::Ok,
            title: "FUSE passthrough is off".to_string(),
            detail: "The daemon serves reads itself, which is what lets the game open its archives and plugins. The launch capability is not needed for this.".to_string(),
            actions: Vec::new(),
        });
    }

    // Dropping a file from the file manager is inert on a native Wayland
    // session, and there is no moment of failure to hang a message on: the event
    // never fires, so the window cannot know a drop was attempted. It has to be
    // said somewhere findable instead, and this tab is where "why did that do
    // nothing" is meant to be answered.
    if on_wayland() {
        out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: "Dropping files from your file manager will not work here".to_string(),
            detail: "This is a Wayland session, and winit - the windowing layer Eidos is built on \
                     - implements file drops for X11 only. Nothing reaches Eidos when you drop an \
                     archive on it, and it cannot tell that you tried. Two paths do work \
                     everywhere: Install Mod, and dragging a row from the Downloads tab onto the \
                     position in the mod list where you want it installed."
                .to_string(),
            actions: Vec::new(),
        });
    }

    // Missing masters: the single most reliable crash predictor.
    //
    // `app.plugins` is a CACHE - dropped on every mod-list change and only
    // rebuilt while the Plugins tab is open - so most of the time it is `None`.
    // This check used to skip itself and print "load order not computed yet",
    // which reads as reassurance and is not: it says nothing was looked at, on
    // the one check most likely to predict a crash. If the cache is cold,
    // compute the answer. `diagnostics` only runs when something changed, not
    // per frame, so it can afford to.
    let computed;
    let plugins = match app.plugins.as_ref() {
        Some(list) => Some(list),
        None => {
            computed = compute_plugins(app);
            computed.as_ref()
        }
    };
    match plugins {
        Some(list) => {
            let missing = list.missing_masters();
            if missing.is_empty() {
                out.push(Diagnostic {
                    level: DiagLevel::Ok,
                    title: "No missing masters".to_string(),
                    detail: format!("All {} plugins have their masters enabled.", list.plugins.len()),
                    actions: Vec::new(),
                });
            } else {
                let mut detail = missing
                    .iter()
                    .take(8)
                    .map(|(p, m)| format!("{p} needs {m}"))
                    .collect::<Vec<_>>()
                    .join("; ");
                if missing.len() > 8 {
                    detail.push_str(&format!("; and {} more", missing.len() - 8));
                }
                out.push(Diagnostic {
                    level: DiagLevel::Problem,
                    title: format!("{} plugin(s) are missing a master", missing.len()),
                    detail: format!("{detail}. The game will crash on load - enable or install them."),
                    actions: Vec::new(),
                });
            }
        }
        // Reachable when there is no game yet, not when nobody has opened a tab.
        // Say which, rather than asking the user to go and do the program's job.
        //
        // NOT reachable for a game that simply has no plugins: telling a Stellar
        // Blade user that their load order is unavailable describes a thing that
        // does not exist for their game, which reads as something being broken.
        None if game_has_plugins(app) => out.push(Diagnostic {
            level: DiagLevel::Advice,
            title: "Load order unavailable".to_string(),
            detail: "No game is selected, or this game has no plugin load order \
                     for Eidos to analyse."
                .to_string(),
            actions: Vec::new(),
        }),
        None => {}
    }

    // ENB + Community Shaders both injecting into D3D11.
    if let (Some(game), Some(inst)) = (selected_game(app), app.created.as_ref()) {
        let cs_roots: Vec<PathBuf> = inst
            .modlist()
            .into_iter()
            .filter(|m| m.is_active())
            .map(|m| m.path)
            .collect();
        if eidos_gamefeatures::enb_cs_conflict(&game.install_path, &cs_roots) {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: "ENB and Community Shaders are both active".to_string(),
                detail: "They can run together, but if visuals look wrong disable one in its INI."
                    .to_string(),
                actions: Vec::new(),
            });
        }
    }

    // A non-empty Overwrite is generated content sitting outside any mod.
    if let Some(inst) = app.created.as_ref() {
        if !inst.overwrite_is_empty() {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: "The Overwrite holds generated files".to_string(),
                detail: "Tool output (xEdit, DynDOLOD, Nemesis) is sitting outside any mod. Turn it into one from the Overwrite tab so it can be ordered and disabled.".to_string(),
                actions: Vec::new(),
            });
        }
        // Debris from an interrupted install.
        let debris: Vec<String> = fs::read_dir(inst.mods_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.starts_with(".eidos-install"))
            .collect();
        if !debris.is_empty() {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: format!("{} leftover extraction folder(s)", debris.len()),
                detail: format!(
                    "An install was interrupted. Eidos ignores them - they are hidden and are \
                     never mounted - but they hold whatever had been unpacked before it stopped, \
                     which for a body or texture mod can be gigabytes. In {}.",
                    inst.mods_dir().display()
                ),
                // A check that knows the answer should not end by naming a
                // directory and wishing the user luck. This one can only ever
                // remove `mods/.eidos-install*`, which Eidos created itself and
                // which nothing else reads.
                actions: vec![("Delete them", Message::CleanInstallDebris)],
            });
        }
    }

    // The last session wrecked the active set (a crash artifact written straight
    // into the bound profile dir): the pre-session snapshot noticed, and the fix
    // is one click. Also fires when the user deliberately disabled most plugins
    // in-game - they dismiss it by playing on; restoring is never automatic.
    if let (Some(inst), Some(spec)) = (
        app.created.as_ref(),
        selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)),
    ) {
        let prof = inst.active();
        if let Some(reason) = prof.plugin_loss_since_snapshot(&spec) {
            out.push(Diagnostic {
                level: DiagLevel::Problem,
                title: "The last session damaged the plugin active set".to_string(),
                detail: format!(
                    "Compared to launch, plugins.txt now {reason}. If the game crashed, restore \
                     the pre-session order below; if you disabled those plugins on purpose, \
                     ignore this."
                ),
                actions: vec![
                    ("Restore the pre-session order", Message::RestorePreSessionPlugins),
                    ("Keep the current set", Message::AcceptPluginState),
                ],
            });
        }
    }

    // The game rewrites its own load order; a profile that never captured one is
    // still riding on the prefix's copy.
    if let Some(inst) = app.created.as_ref().filter(|_| game_manages_plugins(app)) {
        let prof = inst.active();
        if !prof.has_plugin_state() {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: format!("Profile '{}' has no load order of its own yet", prof.name),
                detail: "It will adopt the current one on the next launch, after which switching profiles switches load orders.".to_string(),
                actions: Vec::new(),
            });
        }
    }

    // LOOT coverage for this game.
    if let Some(game) = selected_game(app) {
        // Only worth saying for a game that HAS plugins. LOOT sorts Bethesda
        // plugin files and nothing else, so "LOOT cannot sort Stellar Blade" is
        // true of every game ever made that is not Bethesda's - and it pointed at
        // a Plugins tab that this game does not even show.
        if game_has_plugins(app) && !eidos_loot::is_supported(game.def.id) {
            out.push(Diagnostic {
                level: DiagLevel::Advice,
                title: format!("LOOT cannot sort {}", game.def.name),
                detail: "This game orders plugins by file timestamp; sort it by hand in the Plugins tab.".to_string(),
                actions: Vec::new(),
            });
        }
        // A Flatpak-Steam Proton runs from the host here, which can fail to resolve
        // its sandbox libraries. Eidos will not re-launch through `flatpak run`:
        // that would put the game in Flatpak's sandbox, blind to the FUSE union in
        // our private namespace, and it would silently play vanilla.
        if let Some(cd) = game.compatdata.as_ref() {
            let flatpak = eidos_games::proton_command(
                &home(),
                game.def.steam_app_id,
                cd,
                &game.install_path,
            )
            .is_some_and(|r| r.flatpak);
            if flatpak {
                out.push(Diagnostic {
                    level: DiagLevel::Problem,
                    title: "Proton comes from the Flatpak Steam install".to_string(),
                    detail: "It ships its runtime and steamclient libraries inside the sandbox, so running it from the host may fail. Install a Proton in ~/.steam/root/compatibilitytools.d/ and select it for this game.".to_string(),
                    actions: Vec::new(),
                });
            }
        }
        if game.compatdata.is_none() {
            out.push(Diagnostic {
                level: DiagLevel::Problem,
                title: "No Proton prefix found".to_string(),
                detail: "Launch the game once through Steam so its prefix exists; until then the load order and INIs cannot be deployed.".to_string(),
                actions: Vec::new(),
            });
        }
        out.extend(orphan_archive_diagnostics(app, game.def.id));
        out.extend(script_extender_diagnostic(game));
    }

    out
}

/// What the script extender itself recorded about each of its plugin DLLs on the
/// last run.
///
/// The passthrough check above says whether DLL loading is *likely* to work. This
/// says what happened. The distinction matters because the two failure modes look
/// identical from outside: a plugin refused for an incompatible runtime version
/// and one the manager failed to expose both end with the feature simply absent
/// in game.
pub(crate) fn script_extender_diagnostic(game: &DetectedGame) -> Option<Diagnostic> {
    let spec = GameSpec::for_id(game.def.id)?;
    let prefix = game.compatdata.as_ref()?.join("pfx");
    let docs = eidos_plugins::documents_my_games_dir(&prefix, &spec);
    let path = eidos_gamefeatures::se_log_path(game.def.id, &docs, &game.install_path)?;

    let Ok(raw) = fs::read(&path) else {
        return Some(Diagnostic {
            level: DiagLevel::Advice,
            title: "No script-extender log yet".to_string(),
            detail: format!(
                "Launch the game once through Eidos and this will report whether each SKSE-style plugin DLL loaded. Expected at {}.",
                path.display()
            ),
            actions: Vec::new(),
        });
    };
    // The extender writes cp1252, so a plugin name with an accent is not valid
    // UTF-8; lossy keeps the rest of the line readable rather than dropping it.
    let plugins = eidos_gamefeatures::parse_se_log(&String::from_utf8_lossy(&raw));
    if plugins.is_empty() {
        return None;
    }
    // The log is from the LAST run, which may predate the current load order, so
    // stamp it - an old log claiming success is the confusing case.
    let when = fs::metadata(&path).and_then(|m| m.modified()).map(format_mtime).unwrap_or_default();
    let failed: Vec<&eidos_gamefeatures::SePluginLoad> =
        plugins.iter().filter(|p| !p.loaded).collect();
    if failed.is_empty() {
        return Some(Diagnostic {
            level: DiagLevel::Ok,
            title: format!("All {} script-extender plugins loaded", plugins.len()),
            detail: format!("From the extender's own log, last written {when}."),
            actions: Vec::new(),
        });
    }
    let lines: Vec<String> =
        failed.iter().take(10).map(|p| format!("{}: {}", p.dll, p.status)).collect();
    let more = failed.len().saturating_sub(lines.len());
    let tail = if more > 0 { format!("  (and {more} more)") } else { String::new() };
    Some(Diagnostic {
        level: DiagLevel::Problem,
        title: format!(
            "{} of {} script-extender plugins did not load",
            failed.len(),
            plugins.len()
        ),
        detail: format!("{}{tail}  -  from the extender's own log, last written {when}.", lines.join("   ")),
        actions: Vec::new(),
    })
}

/// Archives (BSA/BA2) an enabled mod ships that nothing will load: the engine
/// only reads an archive whose name matches an ACTIVE plugin, or that the INI
/// registers by hand. An orphan is silent - the mod looks installed and simply
/// has no effect - which is exactly the class of problem a diagnostic is for.
///
/// Advice, never a problem: a mod can ship an archive deliberately for a plugin
/// the user has not enabled yet.
/// Which plugins the next launch will treat as ACTIVE.
///
/// `app.plugins` is a CACHE - dropped whenever the mod list changes, only rebuilt
/// while the Plugins tab is open - so most of the time it is `None`. Treating
/// that as "nothing is active" is the bug this exists to avoid: it makes every
/// archive an orphan. An absent list is not an empty one, so the profile's own
/// plugins.txt is read instead, and if even that is unreadable the answer is
/// `None` - "we do not know", which is not evidence of anything.
pub(crate) fn active_plugin_names(app: &App, game_id: &str) -> Option<Vec<String>> {
    let inst = app.created.as_ref()?;
    let names: Vec<String> = match app.plugins.as_ref() {
        Some(l) => l.plugins.iter().filter(|p| p.enabled).map(|p| p.name.clone()).collect(),
        None => {
            let spec = GameSpec::for_id(game_id)?;
            let prof = inst.active();
            let dir = if prof.has_plugin_state() {
                prof.plugins_state_dir()
            } else {
                plugins_txt_dir(&selected_game(app)?.compatdata.clone()?.join("pfx"), &spec)
            };
            PluginList::read_active(&dir, &spec)
                .into_iter()
                .filter(|(_, on)| *on)
                .map(|(n, _)| n)
                .collect()
        }
    };
    (!names.is_empty()).then_some(names)
}

/// One row of the Archives tab: an archive a mod ships, and why it will or will
/// not be loaded.
#[derive(Clone)]
pub(crate) struct ArchiveRow {
    pub(crate) mod_name: String,
    pub(crate) archive: String,
    /// The plugin whose name pulls it in, if any.
    pub(crate) by_plugin: Option<String>,
    /// Whether the profile's INI registers it explicitly.
    pub(crate) registered: bool,
}

impl ArchiveRow {
    /// Whether the engine will load it at all.
    pub(crate) fn loaded(&self) -> bool {
        self.registered || self.by_plugin.is_some()
    }
}

/// Every archive the enabled mods ship, with the reason each one loads.
///
/// `None` when the active plugin set is unknown - the same distinction the
/// diagnostic makes, for the same reason: without it, "we have not looked" would
/// render as a tab full of red.
pub(crate) fn cached_archive_rows(app: &App, game_id: &str) -> Option<Vec<ArchiveRow>> {
    let gen = app.view_generation.get();
    if let Some((at, rows)) = app.archives_cache.borrow().as_ref() {
        if *at == gen {
            return rows.clone();
        }
    }
    let rows = archive_rows(app, game_id);
    *app.archives_cache.borrow_mut() = Some((gen, rows.clone()));
    rows
}

pub(crate) fn archive_rows(app: &App, game_id: &str) -> Option<Vec<ArchiveRow>> {
    let inst = app.created.as_ref()?;
    let mods: Vec<(String, PathBuf)> = app
        .mods
        .iter()
        .filter(|m| m.is_active())
        .map(|m| (m.name.clone(), m.path.clone()))
        .collect();
    let archives = eidos_gamefeatures::mod_archives(&mods);
    let active = active_plugin_names(app, game_id)?;
    // The profile's own INI copy is the one that gets deployed, so it is what
    // the next launch will actually register.
    let registered = eidos_gamefeatures::registered_archives_in(&inst.active().dir(), game_id);
    let orphans = eidos_gamefeatures::orphan_archives(&archives, &active, &registered);

    Some(
        archives
            .into_iter()
            .map(|(mod_name, archive)| {
                let is_registered =
                    registered.iter().any(|r| r.trim().eq_ignore_ascii_case(&archive));
                // An orphan is exactly "no active plugin names it and the INI does
                // not register it", so anything NOT an orphan is loaded - and the
                // plugin responsible is the one whose base name matches. Asked of
                // the same function the diagnostic uses, so the tab and the health
                // check can never disagree about what loads.
                let orphan = orphans.iter().any(|(m, a)| *m == mod_name && *a == archive);
                let by_plugin = (!orphan && !is_registered)
                    .then(|| {
                        let stem = archive
                            .rsplit_once('.')
                            .map_or(archive.as_str(), |(s, _)| s)
                            .to_ascii_lowercase();
                        active.iter().find(|p| {
                            let base = p
                                .rsplit_once('.')
                                .map_or(p.as_str(), |(s, _)| s)
                                .to_ascii_lowercase();
                            stem == base
                                || stem.strip_prefix(base.as_str()).is_some_and(|r| r.starts_with(" - "))
                        })
                    })
                    .flatten()
                    .cloned();
                ArchiveRow { mod_name, archive, by_plugin, registered: is_registered }
            })
            .collect(),
    )
}

pub(crate) fn orphan_archive_diagnostics(app: &App, game_id: &str) -> Vec<Diagnostic> {
    let Some(inst) = app.created.as_ref() else { return Vec::new() };
    let mods: Vec<(String, PathBuf)> = app
        .mods
        .iter()
        .filter(|m| m.is_active())
        .map(|m| (m.name.clone(), m.path.clone()))
        .collect();
    let archives = eidos_gamefeatures::mod_archives(&mods);
    if archives.is_empty() {
        return Vec::new();
    }
    // `app.plugins` is a CACHE. It is dropped whenever the mod list changes and
    // only rebuilt while the Plugins tab is open, so most of the time it is
    // `None` - and `unwrap_or_default()` turned "we have not looked" into
    // "nothing is active", which makes EVERY archive an orphan. The tab then
    // announced that eleven archives would not load, naming mods whose plugins
    // were all active, and sent the user hunting a problem that did not exist.
    //
    // An absent list is not an empty one. Read the profile's own plugins.txt
    // instead - one small file, on a diagnostic that already walks every enabled
    // mod for archives - and if even that is unreadable, say nothing at all.
    let active: Vec<String> = match app.plugins.as_ref() {
        Some(l) => l.plugins.iter().filter(|p| p.enabled).map(|p| p.name.clone()).collect(),
        None => {
            let Some(spec) = GameSpec::for_id(game_id) else { return Vec::new() };
            let prof = inst.active();
            let dir = if prof.has_plugin_state() {
                prof.plugins_state_dir()
            } else {
                match selected_game(app).and_then(|g| g.compatdata.clone()) {
                    Some(cd) => plugins_txt_dir(&cd.join("pfx"), &spec),
                    None => return Vec::new(),
                }
            };
            PluginList::read_active(&dir, &spec)
                .into_iter()
                .filter(|(_, on)| *on)
                .map(|(n, _)| n)
                .collect()
        }
    };
    // Still nothing? Then the load order is unknown, and an unknown load order
    // cannot be evidence that an archive is unloaded.
    if active.is_empty() {
        return Vec::new();
    }
    // The profile's own INI copy is the one that gets deployed, so it is what the
    // next launch will actually register.
    let registered =
        eidos_gamefeatures::registered_archives_in(&inst.active().dir(), game_id);

    let orphans = eidos_gamefeatures::orphan_archives(&archives, &active, &registered);
    if orphans.is_empty() {
        return Vec::new();
    }
    let listed: Vec<String> = orphans.iter().take(8).map(|(m, a)| format!("{a} ({m})")).collect();
    let more = orphans.len().saturating_sub(listed.len());
    let tail = if more > 0 { format!(", and {more} more") } else { String::new() };
    vec![Diagnostic {
        level: DiagLevel::Advice,
        title: format!("{} archive(s) no active plugin loads", orphans.len()),
        detail: format!(
            "{}{tail}. An engine only loads an archive named after an ACTIVE plugin \
             (<plugin>.bsa or \"<plugin> - Textures.bsa\"), or one the INI registers. \
             Enable the matching plugin, or the mod's assets will not appear.",
            listed.join(", ")
        ),
        actions: Vec::new(),
    }]
}

/// The Diagnostics tab label, carrying the count of things needing attention.
pub(crate) fn diagnostics_tab_label(app: &App) -> String {
    let n = app.diag.iter().filter(|d| d.level == DiagLevel::Problem).count();
    if n > 0 {
        format!("Diagnostics ({n})")
    } else {
        "Diagnostics".to_string()
    }
}

pub(crate) fn diagnostics_panel<'a>(app: &App) -> Element<'a, Message> {
    // The same cache the tab label reads, so the count on the tab and the cards
    // in the panel can never tell two different stories.
    let checks = app.diag.clone();
    let problems = checks.iter().filter(|d| d.level == DiagLevel::Problem).count();
    let summary = if problems == 0 {
        "No problems found.".to_string()
    } else {
        format!("{problems} problem(s) need attention.")
    };
    let mut col = Column::new()
        .spacing(8)
        .push(text("Diagnostics").size(13.0))
        .push(text(summary).size(12.0));
    for d in checks {
        let (tag, color) = match d.level {
            DiagLevel::Problem => ("PROBLEM", Color::from_rgb8(0x8A, 0x2A, 0x2A)),
            DiagLevel::Advice => ("ADVICE", Color::from_rgb8(0xB0, 0x6A, 0x10)),
            DiagLevel::Ok => ("OK", Color::from_rgb8(0x3E, 0x73, 0x50)),
        };
        let mut card = Column::new()
            .spacing(2)
            .push(
                Row::new()
                    .spacing(6)
                    .align_y(iced::Alignment::Center)
                    .push(text(tag).size(9.0).color(color).width(Length::Fixed(58.0)))
                    .push(text(d.title).size(12.0).width(Length::Fill)),
            )
            .push(text(d.detail).size(10.5).color(Color::from_rgb8(0x6A, 0x5A, 0x40)));
        if !d.actions.is_empty() {
            let mut row = Row::new().spacing(6);
            for (label, msg) in d.actions {
                row = row.push(tool_btn(label, msg));
            }
            card = card.push(row);
        }
        col = col.push(container(card).padding([4, 6]).width(Length::Fill).style(card_style));
    }
    scrollable(col).height(Length::Fill).into()
}

/// How selected a main-strip tab should look right now.
///
/// Split out so the eight call sites below say which tab they are and nothing
/// else - threading the phase, the previous tab and the motion preference
/// through each of them is how one of the eight ends up different.
fn main_mix(app: &App, this: Tab) -> f32 {
    crate::anim::tab_mix(
        crate::anim::at(app, &app.tab_anim),
        &app.tab,
        app.tab_prev.as_ref(),
        &this,
    )
}

/// The same for the mod-information strip.
fn info_mix(app: &App, this: InfoTab) -> f32 {
    crate::anim::tab_mix(
        crate::anim::at(app, &app.info_anim),
        &app.info_tab,
        app.info_prev.as_ref(),
        &this,
    )
}

/// One tab of the main strip.
///
/// `mix` is how selected it should LOOK, 0.0 to 1.0, rather than whether it is:
/// mid-transition the arriving tab and the one being left behind are both
/// somewhere in between. A window with motion off only ever passes 0.0 or 1.0,
/// and then this draws exactly what it drew before animation existed.
pub(crate) fn tab_btn<'a>(label: String, t: Tab, mix: f32) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .padding(6)
        .on_press(Message::SelectTab(t))
        // Both ends are iced's own styles, resolved against the live theme, so
        // no colour is named here and the blend follows the palette.
        .style(move |theme, status| {
            crate::anim::mix_button(
                button::secondary(theme, status),
                button::primary(theme, status),
                mix,
            )
        })
        .into()
}

/// Compute the ESP/ESM load order for the Plugins tab: discover from the selected
/// game's Data plus the enabled mods, preserve any existing prefix order, and
/// validate. `None` if there is no game with a plugin system.
pub(crate) fn compute_plugins(app: &App) -> Option<PluginList> {
    let game = selected_game(app)?;
    let spec = GameSpec::for_id(game.def.id)?;
    let mut sources: Vec<(String, PathBuf)> = vec![(String::new(), game.data_path.clone())];
    // app.mods is MO2 display order (lowest priority first) = the ascending order
    // plugin discovery wants, so feed it through as-is.
    let enabled = app.mods.iter().filter(|m| m.is_active());
    sources.extend(enabled.map(|m| (m.name.clone(), m.path.clone())));
    // The Overwrite layer is a plugin source too (a cleaned/generated .esp lands
    // there) - the launch path includes it, so the GUI must agree.
    if let Some(inst) = app.created.as_ref() {
        sources.push(("overwrite".to_string(), inst.overwrite_dir()));
    }

    let mut list = PluginList::discover(&sources, &spec);
    // The load order is per-profile: read the active profile's own copy once it
    // has one, and otherwise the prefix's (which the profile adopts on first
    // launch). Same primitive as the launch path, so for PlainList games this also
    // keeps "in loadorder.txt but not plugins.txt" DISABLED instead of silently
    // re-enabling plugins the user turned off.
    let profile_state = app
        .created
        .as_ref()
        .map(|i| i.active())
        .filter(|p| p.has_plugin_state())
        .map(|p| p.plugins_state_dir());
    match profile_state {
        Some(dir) => list.apply_prefix_state(&dir, &spec),
        None => {
            if let Some(cd) = game.compatdata.as_ref() {
                let dir = plugins_txt_dir(&cd.join("pfx"), &spec);
                list.apply_prefix_state(&dir, &spec);
            }
        }
    }
    // The pins are the user's, so they load from the profile and outlive any
    // rediscovery of the plugins themselves.
    if let Some(inst) = app.created.as_ref() {
        list.locked = inst.active().read_locked_order();
    }
    list.refresh(&spec);
    Some(list)
}

/// Persist the plugin load order: into the active profile's plugins dir (the
/// single source of truth, bind-mounted over the prefix at launch) AND a shadow
/// copy into the prefix for external tools reading it outside Eidos.
/// The trees LOOT must look at besides the game's own `Data`, highest priority
/// first with Overwrite ahead of all - the union's own precedence.
///
/// Two filters, both of which cost a sort when they are missing:
///
/// UNMANAGED rows are the game's DLC and Creation Club content. `app.mods`
/// carries them so the list can show them, but their `path` is a single `.esm`
/// FILE inside the game's Data directory, not a directory. Offered to libloot as
/// data paths, eighty files it is asked to scan as folders, every sort died with
/// "libloot: an I/O error occurred" and no hint as to which path was at fault.
/// They also need no offering: they are already in the Data dir LOOT reads.
///
/// And anything no longer on disk - a mod folder deleted since the list was
/// read - for the same reason: libloot reports the failure without naming it, so
/// one stale row would take the whole sort down.
pub(crate) fn loot_data_paths(app: &App) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(inst) = app.created.as_ref() {
        dirs.push(inst.overwrite_dir());
    }
    dirs.extend(
        app.mods
            .iter()
            .rev()
            .filter(|m| m.is_active() && !m.is_unmanaged())
            .map(|m| m.path.clone()),
    );
    dirs.retain(|p| p.is_dir());
    dirs
}

/// Why a plugin will not move, in the terms a modder already thinks in.
///
/// A plugin must load after every one of its masters and before anything that
/// declares IT as a master, so a plugin caught between the two has exactly one
/// legal slot. Naming both sides is the difference between "the drag is broken"
/// and "of course, EX2 needs EX1".
pub(crate) fn pinned_by(range: &MovableRange) -> String {
    match (&range.after, &range.before) {
        (Some(a), Some(b)) => format!(
            "Held in place: it must load after {a} (one of its masters) and before {b}, which lists it as a master."
        ),
        (Some(a), None) => {
            format!("Held in place: it must load after {a}, which is one of its masters.")
        }
        (None, Some(b)) => {
            format!("Held in place: it must load before {b}, which lists it as a master.")
        }
        (None, None) => {
            "Held in place: the game loads this plugin itself, at a fixed position.".to_string()
        }
    }
}

/// Persist the load order after a user-driven change, and say so if disk refused.
///
/// A refused write means the in-memory order never landed. Keeping it would let a
/// LATER successful write commit this stale list over whatever a running session
/// wrote meanwhile, so the list is re-read instead: disk is the truth.
pub(crate) fn commit_plugin_order(app: &mut App, spec: &GameSpec) {
    plugin_state_changed(app);
    let written = app.plugins.as_ref().map(|list| write_plugin_state(app, list, spec)).transpose();
    if let Err(e) = written {
        app.status = Some(format!("Could not write the load order: {e}"));
        app.plugins = compute_plugins(app);
    }
}

pub(crate) fn write_plugin_state(app: &App, list: &PluginList, spec: &GameSpec) -> std::io::Result<()> {
    // Cross-process lock: a running session owns these files (the plugins dir is
    // bind-mounted into it); a mid-game reorder must refuse, not corrupt.
    let _lock = app.created.as_ref().map(|inst| inst.try_lock("the Eidos window")).transpose()?;
    if let Some(inst) = app.created.as_ref() {
        let prof = inst.active();
        // A deliberate GUI edit is the user speaking: it must not trip the
        // "session damaged the active set" card, so the snapshot follows it -
        // EXCEPT while damage is currently flagged, where refreshing would
        // destroy the only copy that can still restore the pre-damage state.
        let damage_flagged = prof.plugin_loss_since_snapshot(spec).is_some();
        list.write_load_order(&prof.plugins_state_dir(), spec)?;
        prof.write_locked_order(&list.locked)?;
        if !damage_flagged {
            let _ = prof.snapshot_plugin_state();
        }
    }
    if let Some(cd) = selected_game(app).and_then(|g| g.compatdata.as_ref()) {
        list.write_load_order(&plugins_txt_dir(&cd.join("pfx"), spec), spec)?;
    }
    Ok(())
}

pub(crate) fn plugins_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(list) = &app.plugins else {
        return Column::new()
            .spacing(4)
            .push(text("Plugins (ESP / ESM / ESL load order)").size(13.0))
            .push(text("Open a game instance to compute the plugin load order.").size(12.0))
            .into();
    };

    let active = list.plugins.iter().filter(|p| p.enabled).count();
    let missing = list.missing_masters();

    // Top row: the plugin count plus a "Sort with LOOT" action (MO2's Sort button),
    // shown only for games LOOT can sort.
    let loot_ok = selected_game(app).map(|g| eidos_loot::is_supported(g.def.id)).unwrap_or(false);
    let mut top = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text(format!("{} plugins - {active} active", list.plugins.len())).size(12.0));
    if loot_ok {
        // No `on_press` while a sort runs, nor while a run is tracked: the button
        // greys itself. That is the only sign of work a multi-second masterlist
        // download otherwise gives, and the only sign that a session still holds
        // the load-order files - the handler refused both cases already, but a
        // live-looking button that answers with a status line reads as broken.
        let busy = app.sorting || app.running.is_some();
        let label = if app.sorting {
            "Sorting..."
        } else if app.running.is_some() {
            "Sort with LOOT (game running)"
        } else {
            "Sort with LOOT"
        };
        let mut b = button(text(label).size(11.0)).padding([3, 8]).style(button::secondary);
        if !busy {
            b = b.on_press(Message::SortPlugins);
        }
        top = top.push(b);
    }
    // Batch enable/disable, shown only once a selection exists so the toolbar
    // does not offer an action with no subject.
    let picked = if app.selected_plugins.len() > 1 {
        app.selected_plugins.len()
    } else {
        usize::from(app.selected_plugin.is_some())
    };
    if picked > 0 {
        top = top
            .push(text(format!("{picked} selected")).size(11.0))
            .push(
                button(text("Enable").size(11.0))
                    .padding([3, 8])
                    .on_press(Message::SetSelectedPluginsEnabled(true))
                    .style(button::secondary),
            )
            .push(
                button(text("Disable").size(11.0))
                    .padding([3, 8])
                    .on_press(Message::SetSelectedPluginsEnabled(false))
                    .style(button::secondary),
            );
    }
    let mut head = Column::new().spacing(2).push(top);
    if !missing.is_empty() {
        head = head.push(
            text(format!("! {} missing master(s) - the game would crash", missing.len())).size(12.0),
        );
    }

    // A pin the engine had to overrule. Silence here would leave the user
    // believing a slot is held when it is not, so it is said out loud.
    let violated = list.violated_locks();
    if !violated.is_empty() {
        let names: Vec<&str> = violated.iter().map(|(n, _, _)| n.as_str()).take(3).collect();
        let more = violated.len().saturating_sub(names.len());
        let tail = if more > 0 { format!(" (+{more} more)") } else { String::new() };
        head = head.push(
            text(format!(
                "{} pinned position(s) could not be kept - a plugin must load after its masters: {}{tail}",
                violated.len(),
                names.join(", ")
            ))
            .size(11.0),
        );
    }

    let header = Row::new()
        .spacing(6)
        .push(text("Index").size(11.0).width(Length::Fixed(52.0)))
        .push(text("On").size(11.0).width(Length::Fixed(28.0)))
        .push(text("Plugin").size(11.0).width(Length::Fill))
        .push(text("Type").size(11.0).width(Length::Fixed(36.0)))
        .push(text("LOOT").size(11.0).width(Length::Fixed(40.0)))
        .push(text("Pin").size(11.0).width(Length::Fixed(26.0)));

    // Base-game masters are implicit/always-on; show them as forced, not togglable.
    let spec = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id));
    // Computed once for the whole list, not per row: the mod selection does not
    // change while the panel is being drawn, and lowercasing 215 folder names
    // per plugin would be work proportional to mods x plugins.
    let selected_origins = selected_mod_origins(app);
    // No spacing: the insertion strips are the spacing, exactly as in the mod
    // list, so the layout does not shift the instant a drag begins.
    let mut rows = Column::new();
    // One entry per drawn row, feeding the strip over the scrollbar: a highlight
    // is useless if the plugin it marks is forty rows out of sight.
    let mut marks: Vec<Option<Color>> = Vec::new();
    let total = list.plugins.len();
    let drag = app.plugin_drag.as_ref();
    // The line draws iff releasing here would move the block - the exact
    // refusal move_plugins_to applies: only a CONTIGUOUS block treats its own
    // interior gaps as "already here". A non-contiguous selection is compacted
    // by ANY drop, its own rows' gaps included, and hiding the line there let a
    // release commit a move no indicator ever announced. `aimed` keeps a plain
    // click from flashing a line at the grabbed row's own gap.
    let live_gap = drag
        .filter(|d| {
            let (lo, hi) = (
                d.block.first().copied().unwrap_or(d.from),
                d.block.last().copied().unwrap_or(d.from),
            );
            let contiguous = hi - lo + 1 == d.block.len();
            d.aimed && !(contiguous && d.gap >= lo && d.gap <= hi + 1)
        })
        .map(|d| d.gap);
    // A strip is a target only inside the range the engine allows this plugin,
    // so an illegal slot cannot be aimed at in the first place. That is strictly
    // better than MO2, which accepts the drop and clamps it afterwards
    // (pluginlist.cpp:1940-2016) - the user there has no way to know why the row
    // did not go where they put it.
    let legal = |gap: usize| {
        drag.is_some_and(|d| {
            gap >= d.range.lo && gap <= d.range.hi && !d.range.blocked.contains(&gap)
        })
    };
    // Said once, above the list, while the drag is live: the boundary is visible
    // as a place the line stops, and this explains what is stopping it.
    if let Some(d) = drag {
        let msg = if d.range.is_stuck(d.block.first().copied().unwrap_or(d.from)) {
            pinned_by(&d.range)
        } else {
            match (&d.range.after, &d.range.before) {
                (Some(a), Some(b)) => format!("Can move between {a} and {b} - both are master ties."),
                (Some(a), None) => format!("Must stay after {a}, one of its masters."),
                (None, Some(b)) => format!("Must stay before {b}, which lists it as a master."),
                (None, None) => "Free to move anywhere in its section.".to_string(),
            }
        };
        head = head.push(text(msg).size(11.0));
    }
    let dragging = drag.is_some();
    for (i, p) in list.plugins.iter().enumerate() {
        let idx = p.index.clone().unwrap_or_else(|| "--".to_string());
        let kind = if p.is_light {
            "ESL"
        } else if p.loads_as_master() {
            "ESM"
        } else {
            "esp"
        };
        let is_primary = spec
            .as_ref()
            .map(|s| s.primary_plugins.iter().any(|pp| pp.eq_ignore_ascii_case(&p.name)))
            .unwrap_or(false);
        // Creation Club content is loaded by the engine from the .ccc file, so
        // it is as immovable and as un-togglable as a base-game master - and has
        // to look it, or the row invites clicks that can do nothing.
        let engine_owned = is_primary || list.implicit.contains(&p.name.to_ascii_lowercase());
        // MO2-style checkbox. A checkbox with no `on_toggle` renders disabled/greyed,
        // which is exactly the look for the non-togglable cases.
        let toggle: Element<'a, Message> = if engine_owned {
            // A forced game master: always on, never togglable (checked + greyed).
            checkbox(true).size(15).into()
        } else if p.force_disabled {
            // An .esl on a no-light engine: can never load (unchecked + greyed).
            checkbox(false).size(15).into()
        } else {
            checkbox(p.enabled).on_toggle(move |_| Message::TogglePlugin(i)).size(15).into()
        };
        // Manual reorder (MO2 lets the load order be moved by hand, not only
        // The pin (MO2's locked order). A primary master is already nailed to the
        // top by the engine, so offering to pin it would be theatre.
        let locked = list.is_locked(i);
        let pin: Element<'a, Message> = if engine_owned {
            text("").width(Length::Fixed(26.0)).into()
        } else {
            button(text(if locked { "[*]" } else { "[ ]" }).size(10.0))
                .padding([0, 3])
                .style(button::text)
                .on_press(Message::TogglePluginLock(i))
                .into()
        };
        // MO2 puts exactly this behind a hover (pluginlist.cpp tooltipData:
        // Origin, Masters, Missing Masters). It is the information that explains
        // why a plugin will not move, and it is far too wide to be a column -
        // these plugins carry five to nine masters each.
        let mut tip = if p.origin_mod.is_empty() {
            "Origin: the game's own Data".to_string()
        } else {
            format!("Origin: {}", p.origin_mod)
        };
        if engine_owned {
            tip.push_str("\nThe game loads this plugin itself: it cannot be moved or disabled.");
        }
        if !p.masters.is_empty() {
            let present: Vec<&str> = p
                .masters
                .iter()
                .filter(|m| list.plugins.iter().any(|q| q.name.eq_ignore_ascii_case(m)))
                .map(|m| m.as_str())
                .collect();
            let absent: Vec<&str> = p
                .masters
                .iter()
                .filter(|m| !list.plugins.iter().any(|q| q.name.eq_ignore_ascii_case(m)))
                .map(|m| m.as_str())
                .collect();
            if !present.is_empty() {
                tip.push_str(&format!("\nMasters: {}", present.join(", ")));
            }
            if !absent.is_empty() {
                tip.push_str(&format!("\nMISSING masters: {}", absent.join(", ")));
            }
            tip.push_str("\nThis plugin must load after all of them.");
        }
        // The LOOT verdicts from the last sort - MO2's plugin flags: a compact
        // badge in its own column, the detail folded into the hover the row
        // already carries. Absent until a sort has run, exactly like MO2.
        let bundle =
            app.loot_meta.as_ref().and_then(|m| m.get(&p.name.to_ascii_lowercase()));
        if let Some(b) = bundle {
            for msg in b.messages.iter().take(3) {
                tip.push_str(&format!(
                    "\nLOOT {}: {}",
                    loot_severity_label(msg.kind),
                    msg.text
                ));
            }
            if b.messages.len() > 3 {
                tip.push_str(&format!(
                    "\n... {} more LOOT message(s) - see the sort report",
                    b.messages.len() - 3
                ));
            }
            for d in &b.dirty_info {
                tip.push_str(&format!(
                    "\nLOOT: needs cleaning with {} ({} ITM, {} deleted reference(s))",
                    d.cleaning_utility, d.itm_count, d.deleted_reference_count
                ));
            }
            if !b.bash_tags.is_empty() {
                tip.push_str(&format!("\nBash Tags: {}", b.bash_tags.join(", ")));
            }
        }
        // Worst thing first: a dirty plugin needs an action (xEdit QAC), which
        // outranks reading a message.
        let loot_badge = match bundle {
            Some(b) if b.needs_cleaning() => "dirty",
            Some(b) => match b.highest_severity() {
                Some(eidos_loot::MessageType::Error) => "error",
                Some(eidos_loot::MessageType::Warn) => "warn",
                Some(eidos_loot::MessageType::Say) => "note",
                None if !b.bash_tags.is_empty() => "tags",
                None => "",
            },
            None => "",
        };
        let name_cell = tooltip(
            text(p.name.clone()).size(12.0).width(Length::Fill),
            container(text(tip).size(11.0))
                .padding(6)
                .style(|t: &Theme| container::Style {
                    background: Some(Background::Color(t.extended_palette().background.weak.color)),
                    ..Default::default()
                }),
            tooltip::Position::FollowCursor,
        )
        .gap(4);
        let row = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(text(idx).size(11.0).width(Length::Fixed(52.0)))
            .push(container(toggle).width(Length::Fixed(28.0)))
            .push(container(name_cell).width(Length::Fill))
            .push(text(kind).size(10.0).width(Length::Fixed(36.0)))
            .push(text(loot_badge).size(10.0).width(Length::Fixed(40.0)))
            .push(container(pin).width(Length::Fixed(26.0)));
        // Grabbing the row arms the drag AND selects it, the same press doing
        // both exactly as in the mod list; hovering it during a drag means
        // "insert above me".
        let selected = app.selected_plugin == Some(i) || app.selected_plugins.contains(&i);
        // Highlight the plugins the SELECTED MOD ships, as MO2 does: with two
        // hundred rows, "which of these came from the mod I just clicked" is
        // otherwise a tooltip-by-tooltip hunt. The plugin's own selection still
        // outranks it - that row is where the user's focus is, and losing its
        // marker to a colour describing a relationship would be a step back.
        let from_selected_mod = plugin_from_selected_mod(&selected_origins, &p.origin_mod);
        // Same padding as `striped`, or a selected row would be a different
        // height from its neighbours and the list would twitch as focus moves.
        let painted: Element<'a, Message> = if selected || from_selected_mod {
            let bg = if selected { SEL_BG } else { ORIGIN_BG };
            container(row)
                .width(Length::Fill)
                .padding(2)
                .style(move |_t: &Theme| container::Style {
                    background: Some(Background::Color(bg)),
                    ..Default::default()
                })
                .into()
        } else {
            striped(row.into(), i % 2 == 0)
        };
        let grab = mouse_area(painted)
            .on_right_press(Message::OpenPluginMenu(i))
            .on_press(Message::SelectPlugin(i))
            .on_enter(Message::PluginDragOverGap(i))
            .on_release(Message::PluginDragDrop);
        rows = rows.push(drop_gap(
            i,
            live_gap == Some(i),
            dragging && legal(i),
            Message::PluginDragOverGap,
            Message::PluginDragDrop,
        ));
        rows = rows.push(grab);
        marks.push(from_selected_mod.then_some(ORIGIN_BG));
    }
    // The trailing strip: hovering a row always means "above it", so this is the
    // only way to aim at the end of the load order.
    if total > 0 {
        rows = rows.push(drop_gap(
            total,
            live_gap == Some(total),
            dragging && legal(total),
            Message::PluginDragOverGap,
            Message::PluginDragDrop,
        ));
    }

    // Same as the mod list: the global release listener decides, and nothing
    // here second-guesses it. `on_exit` cancelled a drag that merely left the
    // bounds - the gesture that reaches an earlier row - and `on_release` raced
    // the listener to cancel what it was about to drop.
    let list_area = mouse_area(scrollable(rows).id(plugin_scroll_id()).height(Length::Fill));
    // Stacked over the scrollbar rather than beside it: in the flow, a strip
    // shifts the whole list sideways for what is meant to be a hint. Nothing in
    // it handles events, so the scrollbar keeps the pointer. Always drawn, like
    // the mod list's: the strip takes no width when there is nothing to mark, so
    // there was never anything for a switch to save.
    let list_area = Stack::new().push(list_area).push(
        Row::new().push(Space::new().width(Length::Fill)).push(scroll_marks(&marks)),
    );

    Column::new().spacing(6).push(head).push(header).push(list_area).into()
}

/// Analyse file conflicts across the enabled mods (+ the game data) for the
/// Conflicts tab and the mod-row flags. Highest-priority mod first; game data
/// last as origin 0. `None` if there is no game.
pub(crate) fn compute_conflicts(app: &App) -> Option<ConflictMap> {
    let game = selected_game(app)?;
    // app.mods is MO2 display order (lowest priority first); the conflict crate wants
    // layers highest-priority first, so reverse. The origin stays the app.mods index
    // + 1 (NOT the layer position), so conflicts_panel's `origin = i + 1` lookup over
    // app.mods still maps to the same mod.
    let mut layers: Vec<Layer> = app
        .mods
        .iter()
        .enumerate()
        .filter(|(_, m)| m.is_active())
        .map(|(i, m)| Layer {
            origin: (i + 1) as u32,
            name: m.name.clone(),
            root: m.path.clone(),
        })
        .rev()
        .collect();
    // MO2's Overwrite is an always-active, top-priority pseudo-mod (xEdit / Bashed
    // Patch output lands there); include it at the front so the mods it shadows get
    // the overwritten emblem. Its whiteout markers are skipped by collect_files, and
    // its reserved origin (u32::MAX) keeps it distinct from BASE_ORIGIN (0).
    if let Some(inst) = app.created.as_ref() {
        let ow = inst.overwrite_dir();
        if ow.is_dir() {
            layers.insert(0, Layer { origin: u32::MAX, name: "Overwrite".to_string(), root: ow });
        }
    }
    layers.push(Layer {
        origin: 0,
        name: format!("[{}]", game.def.id),
        root: game.data_path.clone(),
    });
    Some(build_conflicts_cached(app, layers))
}

/// Build the conflict map from cached per-layer file walks: only layers missing
/// from the cache touch the filesystem, so a toggle/reorder (same set of mods)
/// re-derives winners entirely in memory. The cache is keyed by layer name
/// (mod folder names are unique; the game/Overwrite pseudo-layers use their
/// bracketed display names).
pub(crate) fn build_conflicts_cached(app: &App, layers: Vec<Layer>) -> ConflictMap {
    let mut cache = app.files_cache.borrow_mut();
    let parts: Vec<(Layer, (Vec<String>, bool))> = layers
        .into_iter()
        .map(|l| {
            let files = cache
                .entry(l.name.clone())
                .or_insert_with(|| eidos_conflicts::collect_files(&l.root))
                .clone();
            (l, files)
        })
        .collect();
    ConflictMap::build_from(&parts)
}

pub(crate) fn conflicts_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(map) = &app.conflicts else {
        return Column::new()
            .spacing(4)
            .push(text("Conflicts").size(13.0))
            .push(text("Open a game instance to analyse file conflicts across your mods.").size(12.0))
            .into();
    };

    let mut counts = (0usize, 0usize, 0usize, 0usize); // overwrites, overwritten, mixed, redundant
    let mut rows = Column::new().spacing(1);
    for (i, m) in app.mods.iter().enumerate().filter(|(_, m)| m.is_active()) {
        let origin = (i + 1) as u32;
        let tag = match map.state(origin) {
            ConflictState::Overwrites => {
                counts.0 += 1;
                "overwrites others"
            }
            ConflictState::Overwritten => {
                counts.1 += 1;
                "overwritten"
            }
            ConflictState::Mixed => {
                counts.2 += 1;
                "mixed"
            }
            ConflictState::Redundant => {
                counts.3 += 1;
                "redundant - wins nothing"
            }
            ConflictState::None => continue,
        };
        let detail = map
            .mods
            .get(&origin)
            .map(|c| format!("{}/{} won", c.won, c.total))
            .unwrap_or_default();
        let row = Row::new()
            .spacing(6)
            .push(text(m.name.clone()).size(12.0).width(Length::Fill))
            .push(text(tag).size(11.0).width(Length::Fixed(160.0)))
            .push(text(detail).size(10.0).width(Length::Fixed(80.0)));
        rows = rows.push(striped(row.into(), i % 2 == 0));
    }

    let summary = format!(
        "{} overwrite - {} overwritten - {} mixed - {} redundant",
        counts.0, counts.1, counts.2, counts.3
    );
    Column::new()
        .spacing(6)
        .push(text(format!("Conflicts: {summary}")).size(12.0))
        .push(text("(only conflicting mods shown; flags also appear in the mod list)").size(10.0))
        .push(scrollable(rows).height(Length::FillPortion(2)))
        .push(conflicting_files(app, map))
        .into()
}

/// The FILES the selected mod is fighting over, and who wins each.
///
/// The list above says a mod won "1/2" and stops there, which is where every
/// real question starts: WHICH file, and to whom. Answering it meant reading the
/// mod folders by hand - the flag could be raised by a texture the user cares
/// about or by a stale `.log` the author happened to zip up, and the panel gave
/// no way to tell those apart.
///
/// Capped, because a texture pack can contest thousands of paths and a list that
/// long is not an answer either; the count says what was left out.
pub(crate) fn conflicting_files<'a>(app: &App, map: &ConflictMap) -> Element<'a, Message> {
    const SHOWN: usize = 40;
    let Some(focus) = app.selected_mod else {
        return text("Select a mod to see which files it contests.").size(11.0).into();
    };
    let origin = (focus + 1) as u32;
    let name = app.mods.get(focus).map(|m| m.display_name().to_string()).unwrap_or_default();

    let mut rows = Column::new().spacing(1);
    let mut n = 0usize;
    for node in map.files.values() {
        // No Vec: this loop visits every file in the conflict map on every
        // redraw, and materialising a heap-allocated provider list per node made
        // each pointer event allocate and free once per file in the instance -
        // several hundred thousand times for a real load order. Two comparisons
        // answer the same question.
        let contested = node.is_conflicted()
            && (node.winner == origin || node.alternatives.contains(&origin));
        if !contested {
            continue;
        }
        n += 1;
        if n > SHOWN {
            continue;
        }
        let wins = node.winner == origin;
        // Who this file actually comes from, when it is not us.
        let verdict = if wins {
            "wins it".to_string()
        } else {
            format!("loses to {}", map.name(node.winner))
        };
        let row = Row::new()
            .spacing(6)
            .push(text(node.display_path.clone()).size(11.0).width(Length::Fill))
            .push(
                text(verdict)
                    .size(11.0)
                    .width(Length::Fixed(260.0))
                    .color(if wins { CONFLICT_WINS_FG } else { CONFLICT_LOSES_FG }),
            );
        rows = rows.push(striped(row.into(), n.is_multiple_of(2)));
    }

    let head = if n == 0 {
        format!("{name} contests no files.")
    } else if n > SHOWN {
        format!("{name}: {n} contested file(s), showing the first {SHOWN}")
    } else {
        format!("{name}: {n} contested file(s)")
    };
    Column::new()
        .spacing(4)
        .height(Length::FillPortion(3))
        .push(text(head).size(12.0))
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

pub(crate) fn right_pane<'a>(app: &App) -> Element<'a, Message> {
    // Run-target picker (MO2's executables combo): the game, or any tool run
    // through the same merged view. The game's launcher/binary + script extender are
    // auto-detected as tools, so they show up here alongside the user's tools.
    // Pinned first, hidden not at all. A game's defaults include tools somebody
    // may never use, and a picker listing eight entries to reach the second is a
    // picker nobody reads. The order within each half is left alone - that is
    // the order tools.ini has, which the user controls.
    let mut listed: Vec<&Tool> = app.tools.iter().filter(|t| !t.hidden).collect();
    listed.sort_by_key(|t| !t.pinned);
    let run_options: Vec<String> = std::iter::once(RUN_GAME.to_string())
        .chain(listed.iter().map(|t| t.title.clone()))
        .collect();
    let run_choice = app.tool_choice.clone().unwrap_or_else(|| RUN_GAME.to_string());

    let top = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text("Run:").size(13.0))
        .push(pick_list(run_options, Some(run_choice), Message::ToolPicked).text_size(13.0).padding(8))
        .push(Space::new().width(Length::Fill))
        .push(
            button(Row::new().spacing(6).push(icon(IC_RUN, 18.0)).push(text("Run").size(15.0)))
                .padding(10)
                .on_press(Message::Run)
                .style(button::primary),
        );

    // The tab in force, which is not always `app.tab`: see `effective_tab`.
    let tab = effective_tab(app);
    let mut tabs = Row::new()
        .spacing(4)
        .push(tab_btn("Data".to_string(), Tab::Data, main_mix(app, Tab::Data)));
    // Only for a game whose plugins Eidos actually manages. Stellar Blade is the
    // first game with no plugin system at all, and every other pane keys off the
    // same `GameSpec::for_id` - so without this the tab is there, opens, and
    // shows an empty list for a game that will never have one.
    if game_manages_plugins(app) {
        tabs = tabs.push(tab_btn("Plugins".to_string(), Tab::Plugins, main_mix(app, Tab::Plugins)));
    }
    let tabs = tabs
        .push(tab_btn("Conflicts".to_string(), Tab::Conflicts, main_mix(app, Tab::Conflicts)))
        .push(tab_btn("Overwrite".to_string(), Tab::Overwrite, main_mix(app, Tab::Overwrite)))
        .push(tab_btn("Archives".to_string(), Tab::Archives, main_mix(app, Tab::Archives)))
        .push(tab_btn("Saves".to_string(), Tab::Saves, main_mix(app, Tab::Saves)))
        .push(tab_btn("Downloads".to_string(), Tab::Downloads, main_mix(app, Tab::Downloads)))
        .push(tab_btn(diagnostics_tab_label(app), Tab::Diagnostics, main_mix(app, Tab::Diagnostics)));

    let content = match tab {
        Tab::Data => data_panel(app),
        Tab::Plugins => plugins_panel(app),
        Tab::Conflicts => conflicts_panel(app),
        Tab::Overwrite => overwrite_panel(app),
        Tab::Archives => archives_panel(app),
        Tab::Saves => saves_panel(app),
        Tab::Downloads => downloads_panel(app),
        Tab::Diagnostics => diagnostics_panel(app),
    };

    let inner = Column::new().spacing(8).push(top).push(tabs).push(content);
    let portion = 1000_u16.saturating_sub((app.split * 1000.0) as u16);
    container(inner).width(Length::FillPortion(portion)).height(Length::Fill).padding(8).style(panel_style).into()
}

pub(crate) fn status_bar<'a>(app: &App) -> Element<'a, Message> {
    let kind = match app.kind {
        InstanceKind::Global => "Global",
        InstanceKind::Portable => "Portable",
    };
    let game = selected_game(app).map(|g| g.def.name).unwrap_or("Instance");
    // A live multi-selection count takes the left slot (MO2's "N selected"), unless a
    // transient status message is showing; otherwise the instance summary.
    let showing_status = app.status.is_some();
    let left = if let Some(s) = app.status.clone() {
        s
    } else if app.selected_mods.len() > 1 {
        format!("{} mods selected", app.selected_mods.len())
    } else {
        let profile = app
            .created
            .as_ref()
            .map(|i| i.active().name)
            .unwrap_or_else(|| "Default".to_string());
        format!("{game} - {kind} - {profile}")
    };
    // The Nexus account, if connected this session (MO2's status-bar login state).
    // The tier is always spelled out: showing "(Premium)" only when premium made
    // a free account look like an account whose tier had not been checked yet -
    // and the difference is not cosmetic, since a free account cannot fetch a
    // download link without the key/expires pair from a fresh nxm:// link.
    let account = match &app.nexus_account {
        Some(a) => format!("Nexus: {} ({})", a.name, if a.is_premium { "Premium" } else { "free" }),
        None => "not logged in".to_string(),
    };
    // The left slot fades in when its message changes, so a status that
    // replaces another is visibly a NEW one rather than a word that quietly
    // became a different word. Colour only - the row keeps its height whatever
    // the fade is doing, so the status bar never nudges the panes above it.
    let fg = crate::theme::palette().text;
    let ink = crate::anim::mix(
        Color { a: 0.0, ..fg },
        fg,
        crate::anim::at(app, &app.status_anim),
    );
    let mut row = Row::new()
        .align_y(iced::Alignment::Center)
        .push(text(left).size(11.0).color(ink).width(Length::Fill));
    if showing_status {
        // A tiny dismiss so a stale message stops masking the selection count and
        // instance summary.
        row = row.push(
            button(text("x").size(10.0))
                .padding([0, 6])
                .on_press(Message::ClearStatus)
                .style(button::text),
        );
    }
    row = row.push(text(account).size(11.0));
    container(row).width(Length::Fill).padding(4).style(bar_style).into()
}

pub(crate) fn main_screen(app: &App) -> Element<'_, Message> {
    // The name is the way into Settings, as Colony's is. It was decoration
    // before, and the only route in was the toolbar button - which is a long way
    // to travel for the thing a window's own title usually opens.
    let header = Row::new()
        .spacing(10)
        .push(
            button(text("Eidos").size(20.0))
                .padding([2, 6])
                .on_press(Message::OpenSettings)
                .style(button::text),
        )
        .push(Space::new().width(Length::Fill))
        .push(tool_btn("New instance", Message::Restart));

    // A grabbable divider rather than a gap. The panes were a fixed 3:2, which is
    // a guess about somebody else's screen: on a narrow window the right pane's
    // tab strip runs out of room and clips "Diagnostics", with nothing to do
    // about it. Six pixels wide because a hairline is hard to hit and a bar is
    // furniture; it lights up while held so the drag is visibly in progress.
    let held = app.split_drag;
    let divider = mouse_area(
        container(Space::new().width(Length::Fixed(6.0)).height(Length::Fill))
            .height(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(if held {
                    crate::theme::DIVIDER_HELD
                } else {
                    crate::theme::DIVIDER
                })),
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
    )
    .on_press(Message::SplitGrab);

    let body = Row::new()
        .spacing(4)
        .height(Length::Fill)
        .push(modlist_pane(app))
        .push(divider)
        .push(right_pane(app));

    let mut base = Column::new().spacing(4).padding(4).push(header).push(menu_bar());
    if app.ui_toolbar_visible {
        base = base.push(toolbar(app));
    }
    // Persistent warning while the eidos binary lacks CAP_SYS_ADMIN: launches
    // still work but FUSE passthrough is off and SKSE plugin DLLs may fail to
    // load. Every rebuild wipes the capability, so this fires often enough that
    // silence cost real debugging time.
    if app.cap_missing && passthrough_requested() {
        base = base.push(cap_warning_banner());
    }
    base = base.push(body);
    if app.ui_statusbar_visible {
        base = base.push(status_bar(app));
    }

    let mut layers = Stack::new().push(base);

    // The right-click action menu floats over the window (MO2's context menu).
    // A full-window catcher behind it dismisses on a click outside the card.
    if let Some(i) = app.menu_mod {
        if i < app.mods.len() {
            let catcher =
                mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseMenu);
            let at = app.menu_at.unwrap_or(app.cursor);
            let card = floating_at(mod_menu_card(app, i), at, app.window);
            layers = layers.push(catcher).push(card);
        }
    }

    // The per-mod info dialog is a centered modal (MO2's modinfodialog).
    if let Some(i) = app.info_mod {
        if i < app.mods.len() {
            let scrim =
                mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseInfo);
            let dialog = container(mod_info_dialog(app, i)).center(Length::Fill);
            layers = layers.push(scrim).push(dialog);
        }
    }

    // The manual / BAIN picker (MO2's InstallDialog and BainComplexInstallerDialog).
    // Below the collision chooser in the stack: a collision raised BY the picker
    // has to be the thing you can click.
    if let Some(p) = &app.picker {
        let scrim =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::PickerCancel);
        layers = layers.push(scrim).push(container(install_picker_dialog(p)).center(Length::Fill));
    }

    // The install-collision chooser is a centered modal (MO2's QueryOverwriteDialog).
    if let Some(c) = &app.collision {
        let scrim =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CollisionCancel);
        let dialog = container(collision_dialog(c)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Preferences modal (MO2's Settings dialog).
    if app.settings_open {
        let scrim =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseSettings);
        let dialog = container(settings_dialog(app)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Executables editor (MO2's Modify Executables dialog).
    if let Some(state) = &app.executables {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseExecutablesDialog);
        let dialog = container(executables_dialog(app, state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The filter pane, hanging under its button.
    if app.filters_open {
        let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::ToggleFilterPane)
            .on_right_press(Message::ToggleFilterPane);
        // Hangs under the Filters button rather than floating at the cursor:
        // it is a panel belonging to a control, not a context menu.
        let card = container(filter_pane(app))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding { top: 84.0, right: 0.0, bottom: 0.0, left: 320.0 })
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top);
        layers = layers.push(catcher).push(card);
    }

    // The plugin context menu, floating where it was summoned from.
    // Bounds-checked: a stale row would otherwise render an invisible
    // full-window click catcher over an empty card.
    let plugin_menu_row = app
        .menu_plugin
        .filter(|&i| app.plugins.as_ref().is_some_and(|l| i < l.plugins.len()));
    if let Some(i) = plugin_menu_row {
        let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::ClosePluginMenu)
            .on_right_press(Message::ClosePluginMenu);
        // From the frozen anchor, not the live cursor.
        let at = app.menu_at.unwrap_or(app.cursor);
        let card = floating_at(plugin_menu_card(app, i), at, app.window);
        layers = layers.push(catcher).push(card);
    }

    // The Backups dialog (MO2's Create Backup / Restore Backup).
    if let Some(state) = &app.backups {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseBackupsDialog);
        let dialog = container(backups_dialog(state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Categories dialog (MO2's Change Categories).
    if let Some(state) = &app.categories_dialog {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseCategoriesDialog);
        let dialog = container(categories_dialog(state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Extensions list.
    if app.addons_open {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseAddons);
        let dialog = container(addons_dialog(app)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The log pane (MO2's dockable log view).
    if let Some(state) = &app.log_pane {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseLogPane);
        let dialog = container(log_pane_dialog(state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The INI editor (MO2's bundled tool plugin).
    if let Some(state) = &app.ini_editor {
        // No scrim dismiss: the card holds an editable buffer, and a stray click
        // outside it must not be how unsaved edits are thrown away. The catcher
        // is still there to swallow the click.
        let catcher = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::Noop)
            .on_right_press(Message::Noop);
        let dialog = container(ini_editor_dialog(app, state)).center(Length::Fill);
        layers = layers.push(catcher).push(dialog);
    }

    // The About box (Help menu).
    if app.about_open {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseAbout);
        let dialog = container(about_dialog()).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // A file is hovering over the window: say the drop will be taken. The flag
    // was already tracked and read by nothing, so on X11 the gesture had no
    // feedback at all - the user could not tell the window from any other.
    if app.files_hovering {
        let banner = container(
            container(text("Drop to install").size(15.0))
                .padding([10, 20])
                .style(|t: &Theme| container::Style {
                    background: Some(Background::Color(t.extended_palette().primary.base.color)),
                    text_color: Some(t.extended_palette().primary.base.text),
                    border: Border { width: 0.0, radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill);
        // No catcher: a hover is not a click, and swallowing presses here would
        // freeze the window for as long as the pointer carried a file.
        layers = layers.push(banner);
    }

    // The preview pane.
    if let Some(p) = &app.preview {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::ClosePreview);
        let dialog = container(preview_dialog(p)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The collection browser.
    if let Some(state) = &app.collection {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseCollection);
        let dialog = container(collection_dialog(state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The instance manager (MO2's Manage Instances).
    if app.instances_open {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseInstanceManager);
        let dialog = container(instances_dialog(app)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The Export dialog (MO2's Export to csv).
    if let Some(state) = &app.export {
        let scrim = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseExportDialog);
        let dialog = container(export_dialog(app, state)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The File dropdown floats just under the menu bar, at the File item.
    if app.file_menu_open {
        let catcher =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseFileMenu);
        let card = container(file_menu_card(app))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding { top: 44.0, right: 0.0, bottom: 0.0, left: 4.0 })
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top);
        layers = layers.push(catcher).push(card);
    }

    // The View dropdown floats just under the menu bar, near the View item.
    if app.view_menu_open {
        let catcher =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseViewMenu);
        let card = container(view_menu_card(app))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding { top: 44.0, right: 0.0, bottom: 0.0, left: 44.0 })
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top);
        layers = layers.push(catcher).push(card);
    }

    // The per-profile menu (rename / copy / delete), opened by right-clicking a
    // profile chip. A catcher behind it dismisses on an outside click.
    if let Some(name) = app.profile_menu.clone() {
        let catcher =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::ProfileCloseMenu);
        let at = app.menu_at.unwrap_or(app.cursor);
        let card = floating_at(profile_menu_card(app, &name), at, app.window);
        layers = layers.push(catcher).push(card);
    }

    // The LOOT report (MO2's post-sort dialog): a centered modal listing general
    // messages + per-plugin missing masters / messages / dirty advice.
    if let Some(report) = &app.loot_report {
        let scrim =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CloseLootReport);
        // The card swallows its own presses. A `container` does not take mouse
        // events, so without this a click anywhere ON the report fell through to
        // the scrim behind it and dismissed the thing the user was reading.
        let dialog = container(mouse_area(loot_report_dialog(report)).on_press(Message::Noop))
            .center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    // The run lock (MO2's "lock GUI while the application runs"): a full-window
    // overlay that blocks everything beneath it until the game exits or the user
    // clicks Unlock. Added last so it sits on top of every other layer. A tracked
    // run with `lock` off (setting disabled, or force-unlocked) shows no overlay.
    if let Some(run) = app.running.as_ref().filter(|r| r.lock) {
        // A backdrop that swallows EVERY pointer event (press/release/right/scroll)
        // so nothing beneath it is reachable - clicks, context menus and the modlist
        // scroll wheel are all inert while locked. `interaction` also tells the Stack
        // to mark lower layers unavailable for scroll.
        let scrim = mouse_area(
            container(Space::new().width(Length::Fill).height(Length::Fill)).style(|_| container::Style {
                background: Some(iced::Color { a: 0.55, ..iced::Color::BLACK }.into()),
                ..Default::default()
            }),
        )
        .on_press(Message::Noop)
        .on_release(Message::Noop)
        .on_right_press(Message::Noop)
        .on_scroll(|_| Message::Noop)
        .interaction(iced::mouse::Interaction::NotAllowed);
        let dialog = container(running_lock_card(run)).center(Length::Fill);
        layers = layers.push(scrim).push(dialog);
    }

    layers.into()
}

/// MO2's targeted "Send to" actions, below the blunt top/bottom pair.
///
/// The two conflict-relative moves are the ones people actually reach for -
/// "put this just above the mod it is overriding" - and both are gated on the
/// relevant set being non-empty, so the menu never offers a move that would do
/// nothing. Priority and separator open an inline editor rather than a modal,
/// matching how rename already works in this menu.
/// The separators offered as a destination when sending row `i` (and whatever
/// else is selected with it) into a group.
///
/// The moved rows are excluded. MO2 does not bother, because in a flat list
/// sending a separator into its own group is merely nonsensical rather than
/// unsound - it lands at the tail of the group it used to head. Offering the
/// user a choice whose only outcome is confusion is not parity worth having.
pub(crate) fn separator_choices(app: &App, i: usize) -> Vec<usize> {
    let moving = selection_or(app, i);
    (0..app.mods.len())
        .filter(|&idx| app.mods[idx].is_separator())
        .filter(|idx| !moving.contains(idx))
        .collect()
}

pub(crate) fn send_to_targets<'a>(app: &App, i: usize) -> Element<'a, Message> {
    // Same origin convention as the emblems: index + 1, with the game (0) and the
    // Overwrite pseudo-layer (u32::MAX) excluded because they are not rows.
    let real = |set: &std::collections::BTreeSet<u32>| {
        set.iter().any(|&o| o != 0 && o != u32::MAX)
    };
    let mc = app.conflicts.as_ref().and_then(|m| m.mods.get(&((i + 1) as u32)));
    let mut col = Column::new().spacing(1);

    if let Some((row, text)) = app.send_priority.as_ref().filter(|(r, _)| *r == i) {
        let _ = row;
        col = col.push(
            text_input("Priority", text)
                .on_input(Message::SendToPriorityChanged)
                .on_submit(Message::SendToPriorityCommit)
                .padding(5)
                .size(12.0),
        );
        return col.into();
    }
    if app.send_separator == Some(i) {
        // An inline chooser of the separators, scrollable because a big load
        // order has plenty of them.
        // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
        for idx in separator_choices(app, i) {
            // Owned, so the Element does not borrow from `app`.
            let label = app.mods[idx].display_name().to_string();
            list = list.push(menu_item_owned(label, Message::SendToSeparatorPick(idx)));
        }
        col = col
            .push(text("Move into group:").size(11.0))
            .push(scrollable(list).height(Length::Fixed(160.0)))
            .push(menu_item("Cancel", Message::SendToTargetCancel));
        return col.into();
    }

    if mc.is_some_and(|m| real(&m.overwrites)) {
        col = col.push(menu_item("Send above first conflict", Message::SendToFirstConflict(i)));
    }
    if mc.is_some_and(|m| real(&m.overwritten_by)) {
        col = col.push(menu_item("Send below last conflict", Message::SendToLastConflict(i)));
    }
    col = col
        .push(menu_item("Send to priority...", Message::SendToPriorityStart(i)))
        .push(menu_item("Send to separator...", Message::SendToSeparatorStart(i)));
    col.into()
}

/// The per-profile context menu (MO2's profile manager actions), opened by
/// right-clicking a profile chip: rename, copy-to-new, delete (two-click confirm).
pub(crate) fn profile_menu_card<'a>(app: &App, name: &str) -> Element<'a, Message> {
    let title = Row::new()
        .spacing(6)
        .push(text(format!("Profile: {name}")).size(13.0).width(Length::Fill))
        .push(
            button(text("x").size(13.0))
                .padding([1, 6])
                .on_press(Message::ProfileCloseMenu)
                .style(button::text),
        );
    let mut col = Column::new().spacing(1).push(title).push(menu_sep());

    // Rename: an inline editor when armed, else a menu item that arms it.
    match &app.profile_rename {
        Some((orig, edited)) if orig == name => {
            col = col.push(
                text_input("New name", edited)
                    .on_input(Message::ProfileRenameChanged)
                    .on_submit(Message::ProfileRenameCommit)
                    .padding(5)
                    .size(12.0),
            );
        }
        _ => col = col.push(menu_item("Rename", Message::ProfileRenameStart(name.to_string()))),
    }

    // Copy to a new profile: an inline editor when armed, else a menu item.
    match &app.profile_copy {
        Some((src, edited)) if src == name => {
            col = col.push(
                text_input("Copy name", edited)
                    .on_input(Message::ProfileCopyChanged)
                    .on_submit(Message::ProfileCopyCommit)
                    .padding(5)
                    .size(12.0),
            );
        }
        _ => col = col.push(menu_item("Copy to new...", Message::ProfileCopyStart(name.to_string()))),
    }

    col = col.push(menu_sep());
    // Take over an existing MO2 profile's mod order + load order, so a migrating
    // user does not re-tick dozens of mods and plugins by hand.
    col = col.push(menu_item("Import from MO2...", Message::ImportMo2Pick));

    col = col.push(menu_sep());
    // Delete: two-click confirm (backend refuses the active / last profile).
    let delete: Element<'a, Message> = if app.profile_delete_confirm.as_deref() == Some(name) {
        button(text("Click again to delete").size(12.0))
            .padding([2, 6])
            .width(Length::Fill)
            .on_press(Message::ProfileDeleteCommit(name.to_string()))
            .style(button::danger)
            .into()
    } else {
        menu_item("Delete", Message::ProfileDeleteConfirm(name.to_string()))
    };
    col = col.push(delete);

    container(col).max_width(240.0).padding(8).style(card_style).into()
}

/// Suggest a free profile name near `base` (`base`, `base 2`, `base 3`, ...) so the
/// copy editor never starts on a name that already collides.
pub(crate) fn suggest_free_profile_name(inst: &Instance, base: &str) -> String {
    if !inst.profile(base).dir().exists() {
        return base.to_string();
    }
    (2..1000)
        .map(|n| format!("{base} {n}"))
        .find(|cand| !inst.profile(cand).dir().exists())
        .unwrap_or_else(|| base.to_string())
}

/// Suggest a free mod-folder name near `name` (`name (2)`, `name (3)`, ...) for the
/// Rename option, so the prefilled value doesn't immediately collide again.
pub(crate) fn suggest_free_name(mods_dir: &std::path::Path, name: &str) -> String {
    if !mods_dir.join(name).exists() {
        return name.to_string();
    }
    (2..1000)
        .map(|n| format!("{name} ({n})"))
        .find(|cand| !mods_dir.join(cand).exists())
        .unwrap_or_else(|| name.to_string())
}

/// Retry the pending collision install under `policy`. Reuses the same discovery as
/// a normal install (rebuilds the FOMOD context in case the archive turns out to be
/// a FOMOD). A Rename that collides again re-opens the prompt.
/// The extracted archive parsed once for the picker: the extraction does not
/// change while the dialog is on screen, and re-walking it on every redraw would
/// stutter a large pack - which is also why the picker keeps the TREE and not
/// just its rows: the Manual mode's validity label needs it too.
pub(crate) fn parsed_tree(tree: &eidos_install::ExtractedTree) -> eidos_install::ArchiveTree {
    eidos_install::ArchiveTree::from_dir(tree.path()).unwrap_or_default()
}

/// Install what the manual / BAIN picker currently has selected.
///
/// A name collision hands off to the existing Merge / Replace / Rename prompt,
/// carrying the picks so resolving it does not re-ask them. On any other failure
/// the picker stays open with the reason, so a bad data root can just be
/// re-picked instead of re-extracting the archive.
pub(crate) fn run_picker_install(app: &mut App) {
    let Some(p) = app.picker.as_ref() else { return };
    let Some(mods_dir) = app.created.as_ref().map(|i| i.mods_dir()) else {
        app.status = Some("Open a game instance first.".to_string());
        return;
    };
    let name = p.name.trim().to_string();
    if name.is_empty() {
        app.status = Some("Give the mod a name first.".to_string());
        return;
    }
    let choice = match &p.mode {
        PickerMode::Bain { subpackages, picked, .. } => {
            let chosen: Vec<String> = subpackages
                .iter()
                .zip(picked)
                .filter(|(_, &on)| on)
                .map(|(s, _)| s.clone())
                .collect();
            if chosen.is_empty() {
                app.status = Some("Tick at least one sub-package.".to_string());
                return;
            }
            PickerChoice::Bain(chosen)
        }
        PickerMode::Manual { root } => PickerChoice::Manual(root.clone()),
    };
    let result = install_with_choice(
        &p.tree,
        &choice,
        &p.archive,
        &mods_dir,
        &name,
        &p.game_id,
        eidos_install::OverwritePolicy::Fail,
    );
    match result {
        Ok(r) => {
            let archive = p.archive.clone();
            // A successful install may have consumed the tree (a lone source is
            // moved, not copied), so the picker must go before anything else.
            app.picker = None;
            remember_bain_options(app, &r.name, &choice);
            after_install(app, &r.name, r.dest, r.fomod, Some(&archive));
        }
        Err(eidos_install::InstallError::Exists(_)) => {
            let Some(p) = app.picker.take() else { return };
            let rename_to = suggest_free_name(&mods_dir, &name);
            app.collision = Some(CollisionPrompt {
                archive: p.archive,
                name: name.clone(),
                game_id: p.game_id,
                rename_to,
                fomod: false,
                tree: Some(p.tree),
                pick: Some(choice),
            });
            app.status = Some(format!("'{name}' already exists - choose how to install."));
        }
        Err(e) => app.status = Some(format!("Install failed: {e}")),
    }
}

/// Dispatch one picker choice to the matching installer.
pub(crate) fn install_with_choice(
    tree: &eidos_install::ExtractedTree,
    choice: &PickerChoice,
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: eidos_install::OverwritePolicy,
) -> Result<eidos_install::InstallReport, eidos_install::InstallError> {
    match choice {
        PickerChoice::Bain(subs) => {
            eidos_install::install_bain(tree, subs, archive, mods_dir, name, game_id, policy)
        }
        PickerChoice::Manual(root) => {
            eidos_install::install_manual(tree, root, archive, mods_dir, name, game_id, policy)
        }
    }
}

/// Record a BAIN selection in the installed mod's `meta.ini`, so reinstalling it
/// later opens the picker with the same sub-packages already ticked (MO2's
/// `onInstallationEnd`). Best-effort: failing to remember a preference must not
/// look like a failed install.
pub(crate) fn remember_bain_options(app: &App, mod_name: &str, choice: &PickerChoice) {
    let (PickerChoice::Bain(subs), Some(inst)) = (choice, app.created.as_ref()) else { return };
    let mut meta = inst.mod_meta(mod_name);
    meta.set_bain_options(subs);
    let _ = meta.write(&inst.meta_path(mod_name));
}

pub(crate) fn run_collision_install(app: &mut App, policy: eidos_install::OverwritePolicy) {
    let Some(c) = app.collision.take() else { return };
    // A FOMOD reinstall: the wizard (with the user's choices) is still open in
    // app.fomod - resolve through finish_fomod, never by re-extracting with
    // default selections.
    if c.fomod {
        let Some(mods_dir) = app.created.as_ref().map(|i| i.mods_dir()) else { return };
        // A Rename onto another existing mod re-opens the prompt BEFORE the
        // session is consumed (its drop would delete the extracted tree).
        if let eidos_install::OverwritePolicy::Rename(new) = &policy {
            if eidos_install::collision_name(&mods_dir, new).is_some() {
                app.status = Some("That name also exists - pick another.".to_string());
                app.collision = Some(c);
                return;
            }
        }
        let Some(w) = app.fomod.take() else { return };
        let archive = w.archive.clone();
        match eidos_install::finish_fomod(w.session, &w.selection, &mods_dir, &w.game_id, &w.ctx, policy)
        {
            Ok(r) => after_install(app, &r.name, r.dest, true, Some(&archive)),
            Err(e) => app.status = Some(format!("Install failed: {e}")),
        }
        return;
    }
    let (Some(inst), Some(game)) = (app.created.as_ref(), selected_game(app)) else {
        app.status = Some("Open a game instance first.".to_string());
        return;
    };
    let mods_dir = inst.mods_dir();
    let enabled_roots: Vec<std::path::PathBuf> =
        app.mods.iter().filter(|m| m.is_active()).map(|m| m.path.clone()).collect();
    let disabled_roots: Vec<std::path::PathBuf> =
        app.mods.iter().filter(|m| !m.is_active() && !m.is_separator()).map(|m| m.path.clone()).collect();
    let ctx = eidos_install::fomod_context(&game.data_path, &enabled_roots, &disabled_roots);
    let archive = c.archive.clone();
    // A collision raised by the manual / BAIN picker: replay the SAME picks. The
    // tree alone does not say which sub-packages were ticked, so re-running the
    // plain installer here would quietly install something else.
    if let (Some(choice), Some(tree)) = (c.pick.as_ref(), c.tree.as_ref()) {
        match install_with_choice(
            tree,
            choice,
            &c.archive,
            &mods_dir,
            &c.name,
            &c.game_id,
            policy,
        ) {
            Ok(r) => {
                remember_bain_options(app, &r.name, choice);
                after_install(app, &r.name, r.dest, r.fomod, Some(&archive));
            }
            Err(eidos_install::InstallError::Exists(_)) => {
                app.status = Some("That name also exists - pick another.".to_string());
                app.collision = Some(c);
            }
            Err(e) => app.status = Some(format!("Install failed: {e}")),
        }
        return;
    }
    // Reuse the tree extracted when the collision was raised; only fall back to a
    // fresh extraction if it is gone.
    let result = match c.tree.as_ref() {
        Some(tree) => eidos_install::install_extracted(
            tree,
            &c.archive,
            &mods_dir,
            &c.name,
            &c.game_id,
            policy,
            &ctx,
        ),
        None => eidos_install::install_archive_with_policy(
            &c.archive,
            &mods_dir,
            &c.name,
            &c.game_id,
            policy,
            &ctx,
        ),
    };
    match result {
        Ok(r) => after_install(app, &r.name, r.dest, r.fomod, Some(&archive)),
        Err(eidos_install::InstallError::Exists(_)) => {
            // A Rename target that also exists: keep the prompt open for another try.
            app.status = Some("That name also exists - pick another.".to_string());
            app.collision = Some(c);
        }
        Err(e) => app.status = Some(format!("Install failed: {e}")),
    }
}

/// Cap on the rows the Saves / Downloads panels render (matches the 500-entry
/// cap on the Data / Overwrite listings).
pub(crate) const SAVES_LIST_CAP: usize = 500;

/// Re-scan the active profile's save directory into `app.saves`.
pub(crate) fn load_saves(app: &mut App) {
    app.saves = match &app.created {
        Some(inst) => inst.savegames(),
        None => Vec::new(),
    };
    app.confirm_delete_save = None;
    // Indices just moved; a selection kept across the reload could point at a
    // different save (or past the end).
    app.selected_saves.clear();
    app.confirm_saves_delete = false;
    app.saves_fingerprint = saves_fingerprint(app);
    clear_save_selection(app);
}

/// A cheap picture of the saves directory: how many entries, and the newest
/// mtime among them.
///
/// This is what the tab's tick compares. Re-listing into the model on every tick
/// would rebuild `app.saves` twice a second, dropping the selection and the
/// details pane under the user's hands; comparing a fingerprint costs one
/// `read_dir` and changes nothing unless something really moved.
pub(crate) fn saves_fingerprint(app: &App) -> Option<(usize, std::time::SystemTime)> {
    let dir = app.created.as_ref()?.active().saves_dir();
    let mut n = 0usize;
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    for e in fs::read_dir(dir).ok()?.flatten() {
        n += 1;
        if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
            newest = newest.max(t);
        }
    }
    Some((n, newest))
}

/// Close the save details pane and drop what it derived.
pub(crate) fn clear_save_selection(app: &mut App) {
    app.selected_save = None;
    app.save_info = None;
    app.save_missing = Vec::new();
}

/// Parse the selected save's header and diff its plugin list against the profile's
/// current one. Runs on selection only - a save header means decompressing part of
/// the file, which is not something to do per redraw.
pub(crate) fn load_save_details(app: &mut App) {
    let Some(save) = app.selected_save.and_then(|i| app.saves.get(i)) else {
        clear_save_selection(app);
        return;
    };
    let path = save.path.clone();
    let parsed = eidos_gamefeatures::parse_save(&path).map_err(|e| e.to_string());
    // The screenshot, for the ONE selected save. Decoded here rather than in
    // `view`, which runs every frame, and never for the list - reading every
    // save's image would turn a cheap header scan into hundreds of megabytes.
    app.save_shot = Some((
        path.clone(),
        parsed.as_ref().ok().and_then(|info| {
            eidos_gamefeatures::read_screenshot_with(&path, info).ok().map(|shot| {
                iced::widget::image::Handle::from_rgba(shot.width, shot.height, shot.rgba)
            })
        }),
    ));
    app.save_missing = match (&parsed, app.plugins.as_ref()) {
        (Ok(info), Some(list)) => {
            let known: Vec<eidos_gamefeatures::KnownPlugin> = list
                .plugins
                .iter()
                .map(|p| eidos_gamefeatures::KnownPlugin {
                    name: &p.name,
                    enabled: p.enabled,
                    origin_mod: &p.origin_mod,
                })
                .collect();
            // Every mod, disabled ones included: a disabled mod holding the plugin
            // is precisely the case the "enable what this save needs" fix exists
            // for. Overwrite counts as a provider too (a cleaned .esp lands there).
            let overwrite = app.created.as_ref().map(|i| i.overwrite_dir());
            let mut mods: Vec<eidos_gamefeatures::ModFolder> = app
                .mods
                .iter()
                .filter(|m| !m.is_separator())
                .map(|m| eidos_gamefeatures::ModFolder { name: &m.name, path: &m.path })
                .collect();
            if let Some(o) = overwrite.as_deref() {
                mods.push(eidos_gamefeatures::ModFolder { name: "Overwrite", path: o });
            }
            let data = selected_game(app).map(|g| g.data_path.clone());
            if let Some(d) = data.as_deref() {
                mods.push(eidos_gamefeatures::ModFolder { name: "(game data)", path: d });
            }
            eidos_gamefeatures::missing_plugins(info, &known, &mods, data.as_deref())
        }
        _ => Vec::new(),
    };
    app.save_info = Some((path, parsed));
}

/// Re-scan the downloads directory into `app.downloads`, reading each archive's
/// `.meta` sidecar for its version + install status. Newest first.
pub(crate) fn load_downloads(app: &mut App) {
    let Some(inst) = &app.created else {
        app.downloads = Vec::new();
        app.confirm_delete_download = None;
        app.identifying_download = None;
        return;
    };
    let dir = inst.downloads_dir();
    let entries: Vec<(DownloadRow, std::time::SystemTime)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let raw = p.file_name()?.to_string_lossy().into_owned();
            let lower = raw.to_ascii_lowercase();

            // Two kinds of entry are a download. A finished archive, and a
            // `<archive>.unfinished` partial, which is one ARRIVING - it is
            // written by the separate `eidos nxm` process, and noticing it is
            // the whole reason a running download can be shown at all.
            let partial = lower.ends_with(".unfinished");
            let name = if partial {
                raw.get(..raw.len() - ".unfinished".len())?.to_string()
            } else {
                let is_archive =
                    lower.ends_with(".7z") || lower.ends_with(".zip") || lower.ends_with(".rar");
                if !is_archive {
                    return None;
                }
                raw
            };
            // The row always points at where the archive WILL be, so Install
            // works the instant it lands without the row being rebuilt around it.
            let dest = p.with_file_name(&name);
            let md = e.metadata().ok()?;
            let modified = md.modified().ok()?;

            // Version + install status from the MO2-format `.meta` sidecar.
            let meta_path = PathBuf::from(format!("{}.meta", dest.display()));
            let meta = eidos_instance::ModMeta::read(&meta_path);
            let has_meta = std::fs::metadata(&meta_path).is_ok();
            let total = meta.total_size().unwrap_or(0);
            let state = if partial {
                // Growing or abandoned? The writer is another process, so there
                // is nobody to ask - but a file being appended to has a fresh
                // mtime. A generous window, because a slow mirror can go quiet
                // for a few seconds without being dead.
                //
                // The partial's mtime ALONE is not enough, and getting that wrong
                // loses a download. `eidos nxm` writes the sidecar before the
                // first byte, then makes three API calls and waits on the CDN
                // before it opens the partial - and a RESUMED download appends,
                // which does not touch the mtime until the first byte lands. So
                // for the whole API-plus-latency window a live retry carries the
                // DEAD attempt's mtime, read as "Stalled", which offered a Delete
                // that unlinked the file the running process was writing to: it
                // kept filling an unlinked inode, its rename failed, and the
                // transfer vanished with its sidecar and no message anywhere.
                //
                // The sidecar is the missing signal. It is rewritten on every
                // attempt, so the NEWER of the two mtimes is when something last
                // happened. A genuinely dead download has an equally old sidecar,
                // so this does not weaken the true case.
                let touched = std::fs::metadata(&meta_path)
                    .and_then(|m| m.modified())
                    .map_or(modified, |t| t.max(modified));
                let quiet = touched.elapsed().map(|d| d > STALLED_AFTER).unwrap_or(false);
                // Paused BEATS the mtime heuristic, in both directions. A user
                // who paused a second ago has a fresh mtime and would otherwise
                // still read as "Downloading" for the whole quiet window; and
                // one paused an hour ago would read as "Stalled", which says
                // something went wrong when nothing did.
                if eidos_nexus::download_meta_key(&dest, "paused").as_deref() == Some("true") {
                    DownloadState::Paused
                } else if quiet {
                    DownloadState::Stalled
                } else {
                    DownloadState::Downloading
                }
            } else if !has_meta {
                DownloadState::Untracked
            } else if meta.uninstalled() {
                DownloadState::Uninstalled
            } else if meta.installed() {
                DownloadState::Installed
            } else {
                DownloadState::Ready
            };
            let row = DownloadRow {
                name,
                path: dest,
                // Show the eventual size while it is arriving, so the number does
                // not creep upward in the Size column while the bar already says
                // how far along it is.
                size: if partial && total != 0 { total } else { md.len() },
                version: meta.version().unwrap_or_default(),
                mod_name: meta.mod_name(),
                mod_id: meta.mod_id(),
                state,
                downloaded: md.len(),
                total,
                speed: None,
                hidden: meta.removed(),
                modified,
            };
            Some((row, modified))
        })
        .collect();
    let mut rows: Vec<DownloadRow> = entries.into_iter().map(|(r, _)| r).collect();
    // Hidden rows are dropped BEFORE the cap, so hiding a hundred stale archives
    // actually reveals the ones underneath rather than shortening the list.
    if !app.dl_show_hidden {
        rows.retain(|r| !r.hidden);
    }
    let q = app.dl_filter.trim().to_lowercase();
    if !q.is_empty() {
        // The friendly mod name too, not just the file name: an archive called
        // `SkyUI_5_2_SE-12604-5-2SE.7z` is found by typing "skyui" only if the
        // sidecar's modName is searched as well.
        rows.retain(|r| {
            r.name.to_lowercase().contains(&q)
                || r.mod_name.as_deref().is_some_and(|m| m.to_lowercase().contains(&q))
        });
    }
    match app.dl_sort {
        DownloadSort::Newest => rows.sort_by_key(|r| std::cmp::Reverse(r.modified)),
        // By what the ROW SHOWS, which is the friendly mod name when there is
        // one: sorting by the archive file name puts `SkyUI_5_2_SE-12604.7z`
        // under S while the row reads "SkyUI", and a list ordered by something
        // invisible reads as unordered.
        DownloadSort::Name => rows
            .sort_by_key(|r| r.mod_name.as_deref().unwrap_or(r.name.as_str()).to_lowercase()),
        DownloadSort::Size => rows.sort_by_key(|r| std::cmp::Reverse(r.size)),
        // Within a state, newest first - so the secondary order is the one the
        // list has always had rather than whatever read_dir happened to return.
        DownloadSort::State => {
            rows.sort_by_key(|r| (r.state as u8, std::cmp::Reverse(r.modified)))
        }
    }
    rows.truncate(SAVES_LIST_CAP);

    // Speed is a derivative: compare each in-flight row against the previous
    // sample. A first sighting has no rate yet and says so rather than showing a
    // zero, which would read as "stopped".
    let now = std::time::Instant::now();
    let mut samples = HashMap::new();
    for r in rows.iter_mut().filter(|r| r.state == DownloadState::Downloading) {
        if let Some((then, bytes)) = app.download_samples.get(&r.name) {
            let secs = now.duration_since(*then).as_secs_f64();
            // Guard both ends: a tick that arrives too close carries no signal,
            // and a partial that SHRANK means the transfer restarted from zero
            // (a server that ignored our Range), which is not a negative speed.
            if secs > 0.05 && r.downloaded >= *bytes {
                r.speed = Some((r.downloaded - *bytes) as f64 / secs);
            }
        }
        samples.insert(r.name.clone(), (now, r.downloaded));
    }
    // Only in-flight rows are kept, so the map cannot grow without bound.
    app.download_samples = samples;
    app.downloads = rows;
    // A collection member turns from "missing" to "downloaded" when its archive
    // lands, and "Try to fetch missing" produces exactly that - without this the
    // pane keeps saying "missing" for files that are already on disk, and the
    // asked-set means clicking again does nothing.
    if app.collection.is_some() {
        crate::update::recompute_collection_states(app);
    }
}

/// Recursively copy the CONTENTS of `src` into `dst` (creating `dst`), MO2's
/// "Install from folder": the new mod folder mirrors the chosen directory's root
/// rather than nesting the directory inside itself.
pub(crate) fn copy_dir_contents(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_contents(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Shared post-install step: give the new mod the highest priority (wins conflicts
/// by default, like MO2), reload the list, and invalidate the plugin + conflict
/// caches. modlist() is lowest-priority-first, so highest = the END of the list.
pub(crate) fn after_install(app: &mut App, name: &str, dest: PathBuf, fomod: bool, archive: Option<&Path>) {
    if let Some(inst) = &app.created {
        // Same lock as save_mods: the modlist must not be rewritten under a
        // running session. A refusal is not a lost install - the files are on
        // disk, reconciliation lists the folder on the next reload, and only the
        // auto-enable is skipped.
        match inst.try_lock("the Eidos window") {
            Ok(_lock) => {
                let mut ml = inst.modlist();
                ml.retain(|m| m.name != name);
                ml.push(ModEntry {
                    name: name.to_string(),
                    enabled: true,
                    path: dest,
                    unmanaged: false,
                });
                let _ = inst.save_modlist(&ml);
            }
            Err(e) => {
                app.status = Some(format!(
                    "Installed '{name}', but could not enable it now: {e}. Enable it once the \
                     game closes."
                ));
            }
        }
    }
    reload_mods(app);
    // Flip the source archive's `.meta` status to installed (MO2 marks the
    // download), so the Downloads manager shows it as installed. Best-effort: a
    // manually dropped archive with no sidecar is a no-op.
    if let Some(a) = archive {
        let _ = eidos_nexus::mark_installed(a);
    }
    // The installed mod's tree changed (and a FOMOD may have replaced it wholesale).
    drop_files_cache(app, Some(name));
    invalidate_plugins(app);
    app.conflicts = compute_conflicts(app);
    // The reinstalled mod's meta.ini was just rewritten, and
    // `refresh_meta_cache` only computes rows it does not already hold - so
    // without dropping this one the list keeps showing the OLD version, colour
    // and note for a mod that has just changed underneath it.
    invalidate_meta(app, name);
    refresh_meta_cache(app);
    // Refresh the cached downloads only if they were already loaded, so the
    // status column reflects the new install without a full re-scan otherwise.
    if !app.downloads.is_empty() {
        load_downloads(app);
    }
    let mut where_to = String::new();
    // A drop aimed at a gap says WHERE, not just whether. Consumed here, after
    // `reload_mods`, because that is when the new row exists to be moved - and
    // this is MO2's own ordering too (install first, reposition after).
    // Only when this install is the one that was aimed. An aim left behind by an
    // install that failed or was cancelled must never move an unrelated mod.
    let aimed = app
        .install_at
        .as_ref()
        .filter(|(_, want)| archive.is_some_and(|a| a == want.as_path()))
        .map(|(gap, _)| *gap);
    if aimed.is_some() {
        app.install_at = None;
    }
    if let Some(dest) = aimed {
        if let Some(at) = app.mods.iter().position(|m| m.name == name) {
            let dest = dest.min(app.mods.len());
            let hidden = hidden_by_folds(app);
            let landed = move_block(&mut app.mods, &[at], dest);
            settle_folds_after_move(app, landed, 1, &hidden);
            mods_changed(app);
            app.selected_mod = Some(landed);
            where_to = format!(" at priority {landed}");
        }
    }
    // A note the drop left for the installer to deliver - it could not say it
    // itself, because this line would have overwritten it.
    let note = app.pending_note.take().map(|n| format!(" ({n})")).unwrap_or_default();
    app.status = Some(if fomod {
        format!("Installed '{name}' via FOMOD{where_to}{note}.")
    } else {
        format!("Installed '{name}'{where_to}{note}.")
    });
}


/// The install-collision chooser card (MO2's QueryOverwriteDialog): Merge / Replace
/// / Rename / Cancel for an already-existing `mods/<name>/`.
pub(crate) fn collision_dialog<'a>(c: &CollisionPrompt) -> Element<'a, Message> {
    let buttons = Row::new()
        .spacing(8)
        .push(
            button(text("Merge").size(12.0))
                .padding([4, 10])
                .on_press(Message::CollisionMerge)
                .style(button::secondary),
        )
        .push(
            button(text("Replace").size(12.0))
                .padding([4, 10])
                .on_press(Message::CollisionReplace)
                .style(button::danger),
        );
    let rename = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text("Rename:").size(12.0))
        .push(
            text_input("new name", &c.rename_to)
                .on_input(Message::CollisionRenameChanged)
                .on_submit(Message::CollisionRenameCommit)
                .padding(5)
                .size(12.0)
                .width(Length::Fill),
        )
        .push(
            button(text("Install").size(12.0))
                .padding([4, 10])
                .on_press(Message::CollisionRenameCommit)
                .style(button::primary),
        );
    let card = Column::new()
        .spacing(10)
        .push(text(format!("\"{}\" already exists", c.name)).size(15.0))
        .push(text("A mod with this name is already installed. Choose how to install it:").size(12.0))
        .push(buttons)
        .push(
            text("Merge installs over the existing files. Replace wipes the mod and reinstalls (your endorsement and category are kept).")
                .size(10.0),
        )
        .push(rename)
        .push(
            button(text("Cancel").size(12.0))
                .padding([4, 10])
                .on_press(Message::CollisionCancel)
                .style(button::text),
        );
    container(card).max_width(460.0).padding(16).style(card_style).into()
}

/// How many tree rows the manual picker draws. An archive with more entries than
/// this is one whose data root is a top-level folder anyway.
pub(crate) const PICKER_TREE_ROWS: usize = 1500;

/// The manual / BAIN install picker: MO2's `InstallDialog` (point at the data
/// root) and `BainComplexInstallerDialog` (tick sub-packages), which share an
/// When nothing in an archive looks like a mod, whether the archive is really a
/// bundle of OTHER archives - the variant packs that ship two `.zip` options and
/// a folder of screenshots.
///
/// Neither level of such an archive can ever look valid, so the dialog otherwise
/// just repeats "does NOT look valid" wherever the user clicks, with no hint that
/// the answer is to open one of the inner files instead.
pub(crate) fn nested_archive_hint(rows: &[eidos_install::TreeRow]) -> Option<String> {
    let n = rows
        .iter()
        .filter(|r| !r.is_dir)
        .filter(|r| {
            r.name
                .rsplit_once('.')
                .is_some_and(|(_, e)| matches!(e.to_ascii_lowercase().as_str(), "7z" | "zip" | "rar"))
        })
        .count();
    match n {
        0 => None,
        1 => Some("This archive contains another archive - the mod is probably inside it.".to_string()),
        n => Some(format!(
            "This archive contains {n} archives - it is a set of variants, and the mod is inside \
             the one you want."
        )),
    }
}

/// archive tree and a name field.
pub(crate) fn install_picker_dialog<'a>(p: &InstallPicker) -> Element<'a, Message> {
    let name_row = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(text("Install as:").size(12.0))
        .push(
            text_input("mod name", &p.name)
                .on_input(Message::PickerNameChanged)
                .on_submit(Message::PickerInstall)
                .padding(5)
                .size(12.0)
                .width(Length::Fill),
        );

    let (title, body): (String, Element<'a, Message>) = match &p.mode {
        // MO2 asks before assuming: an archive whose top level mixes sub-packages
        // with other folders is as likely to be a plain mod with extras.
        PickerMode::Bain { asking: true, subpackages, .. } => (
            "May be a BAIN installer".to_string(),
            Column::new()
                .spacing(10)
                .push(
                    text(format!(
                        "This archive has {} folder(s) that look like Wrye Bash sub-packages, \
                         and others that do not. Install it as a BAIN package?",
                        subpackages.len()
                    ))
                    .size(12.0),
                )
                .push(
                    Row::new()
                        .spacing(8)
                        .push(
                            button(text("Yes, pick sub-packages").size(12.0))
                                .padding([4, 10])
                                .on_press(Message::PickerBainConfirm(true))
                                .style(button::primary),
                        )
                        .push(
                            button(text("No, choose the data folder").size(12.0))
                                .padding([4, 10])
                                .on_press(Message::PickerBainConfirm(false))
                                .style(button::secondary),
                        ),
                )
                .into(),
        ),
        PickerMode::Bain { subpackages, picked, .. } => {
            // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
            for (i, (name, &on)) in subpackages.iter().zip(picked).enumerate() {
                list = list.push(
                    checkbox(on).label(name.clone())
                        .on_toggle(move |_| Message::PickerBainToggle(i))
                        .size(13.0)
                        .text_size(12.0),
                );
            }
            (
                "Choose sub-packages".to_string(),
                Column::new()
                    .spacing(8)
                    .push(
                        text("Ticked sub-packages are merged top to bottom, so a later one wins.")
                            .size(11.0),
                    )
                    .push(scrollable(list).height(Length::Fixed(240.0)))
                    .into(),
            )
        }
        PickerMode::Manual { root } => {
            // Like MO2's live green/red label - but against the tree parsed at
            // construction, never a re-walk of the extraction inside view().
            let rules = eidos_install::LayoutRules::for_game(&p.game_id);
            let valid = p.archive_tree.root_looks_valid(root, rules);
            let chosen = if root.is_empty() { "<archive root>" } else { root.as_str() };

            let mut list = Column::new().spacing(1).push(
                button(text("<archive root>").size(12.0))
                    .padding([1, 4])
                    .on_press(Message::PickerSetRoot(String::new()))
                    .style(if root.is_empty() { button::primary } else { button::text }),
            );
            for r in p.rows.iter().filter(|r| r.is_dir).take(PICKER_TREE_ROWS) {
                let selected = *root == r.path;
                let label = format!("{}{}", "    ".repeat(r.depth + 1), r.name);
                list = list.push(
                    button(text(label).size(12.0))
                        .padding([1, 4])
                        .on_press(Message::PickerSetRoot(r.path.clone()))
                        .style(if selected { button::primary } else { button::text }),
                );
            }
            (
                "Choose the data folder".to_string(),
                Column::new()
                    .spacing(6)
                    .push(scrollable(list).height(Length::Fixed(220.0)))
                    .push(
                        text(if valid {
                            format!("The content of {chosen} looks valid.")
                        } else {
                            format!("The content of {chosen} does NOT look valid.")
                        })
                        .size(11.0)
                        .color(if valid {
                            Color::from_rgb8(0x2E, 0x6E, 0x31)
                        } else {
                            Color::from_rgb8(0x8E, 0x2A, 0x2A)
                        }),
                    )
                    // MO2 warns but still lets you through: the checker only knows
                    // what the game itself calls mod content, and plenty of valid
                    // mods (SKSE plugins, tool configs) match none of it.
                    //
                    // "folder names" alone was already half the story - extensions
                    // have always counted too - and became actively misleading once
                    // the vocabulary went per-game: a Stellar Blade mod is
                    // recognised ONLY by its `.pak`, never by a folder.
                    .push(
                        text(
                            "You can install anyway - the check only recognises this game's \
                             own folder names and file types.",
                        )
                        .size(10.0),
                    )
                    // The case that sends people round in circles: an archive whose
                    // real content is another archive. Nothing here can install it,
                    // and without saying so the dialog just repeats that nothing
                    // looks valid at every level the user tries.
                    .push(match nested_archive_hint(&p.rows) {
                        Some(h) => Element::from(text(h).size(10.0)),
                        None => Space::new().into(),
                    })
                    .into(),
            )
        }
    };

    let mut card = Column::new().spacing(10).push(text(title).size(15.0)).push(name_row).push(body);

    // No Install button while the BAIN question is open: the answer decides which
    // installer would even run.
    if !matches!(p.mode, PickerMode::Bain { asking: true, .. }) {
        card = card.push(
            Row::new()
                .spacing(8)
                .push(
                    button(text("Install").size(12.0))
                        .padding([4, 10])
                        .on_press(Message::PickerInstall)
                        .style(button::primary),
                )
                .push(
                    button(text("Cancel").size(12.0))
                        .padding([4, 10])
                        .on_press(Message::PickerCancel)
                        .style(button::text),
                ),
        );
    }
    container(card).max_width(520.0).padding(16).style(card_style).into()
}


/// Whether this process is talking Wayland rather than X11.
///
/// The same test winit itself makes when it picks a backend: it uses Wayland when
/// `WAYLAND_DISPLAY` names a socket it can reach, and X11 otherwise. There is no
/// API to ask it afterwards, and the env var is what decided the answer - so
/// reading it here gives the same verdict rather than a guess about it.
///
/// It matters because winit 0.30 has no `wl_data_device` at all: on Wayland, the
/// file-drop events exist in the type system and never fire.
pub(crate) fn on_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
}


/// The Archives tab: every BSA/BA2 the enabled mods ship, and whether the engine
/// will actually load it.
///
/// MO2 lists archives and leaves you to work out which are dead weight. Eidos
/// already computes that for the Health tab; this shows the same answer per mod,
/// with the reason attached - loaded because a plugin names it, loaded because
/// the INI registers it, or not loaded at all.
pub(crate) fn archives_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(game) = selected_game(app) else {
        return text("Open a game instance to see its archives.").into();
    };
    let Some(rows) = cached_archive_rows(app, game.def.id) else {
        return Column::new()
            .spacing(6)
            .push(text("Archives").size(13.0))
            .push(
                text(
                    "The active plugin list is not known yet, and without it Eidos cannot say \
                     which archives load - every one would look dead. Open the Plugins tab once, \
                     or launch the game, and come back.",
                )
                .size(12.0),
            )
            .into();
    };

    let orphans = rows.iter().filter(|r| !r.loaded()).count();
    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(text(format!("{} archive(s) across your enabled mods", rows.len())).size(13.0))
        .push(Space::new().width(Length::Fill))
        .push(
            text(if orphans == 0 {
                "all of them load".to_string()
            } else {
                format!("{orphans} will not load")
            })
            .size(11.0)
            .color(if orphans == 0 { CONFLICT_WINS_FG } else { CONFLICT_LOSES_FG }),
        );

    let col_header = Row::new()
        .spacing(8)
        .push(text("Archive").size(11.0).width(Length::FillPortion(3)))
        .push(text("From mod").size(11.0).width(Length::FillPortion(2)))
        .push(text("Loaded because").size(11.0).width(Length::FillPortion(3)));

    let mut list = Column::new();
    if rows.is_empty() {
        list = list.push(text("None of your enabled mods ships a BSA or BA2.").size(12.0));
    }
    for (i, r) in rows.iter().enumerate() {
        let why: Element<'a, Message> = match (&r.by_plugin, r.registered) {
            (_, true) => text("the INI registers it").size(11.0).into(),
            (Some(p), _) => text(format!("{p} is active")).size(11.0).into(),
            (None, false) => text("nothing loads it - it is dead weight")
                .size(11.0)
                .color(CONFLICT_LOSES_FG)
                .into(),
        };
        let row = Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .push(text(r.archive.clone()).size(12.0).width(Length::FillPortion(3)))
            .push(text(r.mod_name.clone()).size(11.0).width(Length::FillPortion(2)))
            .push(container(why).width(Length::FillPortion(3)));
        list = list.push(list_row(container(row).padding(3).into(), i % 2 == 0, false, None, None));
    }

    Column::new()
        .spacing(6)
        .push(header)
        .push(col_header)
        .push(scrollable(list).height(Length::Fill))
        .push(
            text(
                "An engine loads an archive named after an ACTIVE plugin (<plugin>.bsa, or \
                 \"<plugin> - Textures.bsa\"), or one the INI registers explicitly. Anything else \
                 is on disk and never read.",
            )
            .size(10.0),
        )
        .into()
}
