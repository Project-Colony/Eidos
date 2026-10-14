//! The per-mod information dialog, Eidos's answer to MO2's `modinfodialog`: the
//! tabs for a single mod's files, conflicts, categories, notes and Nexus data.
//!
//! The main window's tab panels live beside it, one file per tab (`plugins`,
//! `conflicts`, `downloads`, ...), re-exported here so callers name them as before.

use crate::fomod::{fomod_ink_faint, fomod_ink_soft};
use crate::theme::*;
use crate::widgets::*;
use crate::*;

mod archives;
mod conflicts;
mod diagnostics;
mod downloads;
mod files;
mod install;
mod main_screen;
mod plugins;
mod saves;

pub(crate) use archives::*;
pub(crate) use conflicts::*;
pub(crate) use diagnostics::*;
pub(crate) use downloads::*;
pub(crate) use files::*;
pub(crate) use install::*;
pub(crate) use main_screen::*;
pub(crate) use plugins::*;
pub(crate) use saves::*;

/// One tab of the mod-information strip. Same contract as [`tab_btn`].
pub(crate) fn info_tab_btn<'a>(label: &'a str, tab: InfoTab, mix: f32) -> Element<'a, Message> {
    button(text(label).size(sz(12)))
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
        .push(text(k).size(sz(12)).width(Length::Fixed(scaled(120.0))))
        .push(text(v).size(sz(12)).width(Length::Fill))
        .into()
}

/// General tab: name/version/category/Nexus id/source/endorsed/tracked + counts.
pub(crate) fn info_general<'a>(app: &App, m: &ModEntry) -> Element<'a, Message> {
    let meta = app.created.as_ref().map(|inst| inst.mod_meta(&m.name));
    let files = cached_entries(app, &m.path).len();
    let mut col = Column::new()
        .spacing(4)
        .push(info_kv(t("info_name"), m.name.clone()));
    if let Some(meta) = &meta {
        if let Some(v) = meta.version() {
            col = col.push(info_kv(t("info_version"), v));
        }
        if let Some(nv) = meta.newest_version() {
            col = col.push(info_kv(t("info_newest"), nv));
        }
        if let Some(c) = app
            .meta_cache
            .get(&m.name)
            .and_then(|r| r.category_name.clone())
        {
            col = col.push(info_kv(t("info_category"), c));
        }
        if let Some(id) = meta.mod_id() {
            col = col.push(info_kv(t("info_nexus_id"), id.to_string()));
        }
        let sources = meta.installed_files();
        if !sources.is_empty() {
            col = col.push(info_kv(
                t("info_nexus_files"),
                sources
                    .iter()
                    .map(|(m, f)| tf("info_nexus_file", &[("mod", m), ("file", f)]))
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        if let Some(warning) = meta.install_warning() {
            col = col.push(info_kv(t("info_install_warning"), warning));
        }
        if let Some(a) = meta.author() {
            col = col.push(info_kv(t("info_author"), a));
        }
        // The uploader is shown only when it says something the Author line does
        // not: on most mods they are the same person, and a second identical row
        // is noise pretending to be information.
        if let Some(u) = meta
            .uploader()
            .filter(|u| Some(u) != meta.author().as_ref())
        {
            col = col.push(info_kv(t("info_uploaded_by"), u));
        }
        if let Some(src) = meta.installation_file() {
            col = col.push(info_kv(t("info_installed_from"), src));
        }
        col = col
            .push(info_kv(
                t("info_endorsed"),
                if meta.endorsed() {
                    t("common_yes").into()
                } else {
                    t("common_no").into()
                },
            ))
            .push(info_kv(
                t("info_tracked"),
                if meta.tracked() {
                    t("common_yes").into()
                } else {
                    t("common_no").into()
                },
            ));
    }
    if !app.info_provenance.is_empty() {
        col = col.push(info_kv(
            t("info_generated_output"),
            app.info_provenance.clone(),
        ));
    }
    // A page for mods that are not on Nexus. Without it every such mod is a dead
    // end in the interface: "Visit on Nexus" only appears when a Nexus id
    // exists, so a LoversLab or GitHub mod has nowhere to go from here.
    let url_row = Row::new()
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .push(
            text(t("info_page"))
                .size(sz(11))
                .width(Length::Fixed(scaled(90.0))),
        )
        .push(
            text_input("https://...", &app.url_edit)
                .on_input(Message::ModUrlChanged)
                .on_submit(Message::ModUrlSave)
                .padding(4)
                .size(sz(12)),
        )
        .push(
            button(text(t("common_save")).size(sz(11)))
                .padding([3, 8])
                .style(button::secondary)
                .on_press(Message::ModUrlSave),
        );

    col.push(info_kv(
        t("info_enabled"),
        if m.enabled {
            t("common_yes").into()
        } else {
            t("common_no").into()
        },
    ))
    .push(info_kv(t("info_files"), files.to_string()))
    .push(info_kv(t("common_folder"), m.path.display().to_string()))
    .push(url_row)
    .into()
}

/// Build the receipt text on dialog-open, never by reading the journal every frame.
pub(crate) fn generated_summary(app: &App, name: &str) -> String {
    let Some(inst) = app.created.as_ref() else {
        return String::new();
    };
    let outputs = match inst.generated_output(Some(name)) {
        Ok(outputs) => outputs,
        Err(error) => return tf("info_receipts_failed", &[("error", &error)]),
    };
    if outputs.is_empty() {
        return String::new();
    }
    let plugins = app.plugins.clone().or_else(|| compute_plugins(app));
    let inputs: Vec<_> = plugins
        .as_ref()
        .into_iter()
        .flat_map(|list| list.plugins.iter().map(|p| (p.name.clone(), p.enabled)))
        .collect();
    outputs
        .iter()
        .map(|output| {
            let receipt = &output.receipt;
            let state = if inst.generated_input_drift(receipt, &inputs) {
                t("info_inputs_changed")
            } else {
                t("info_inputs_unchanged")
            };
            let versions = receipt
                .inputs
                .iter()
                .map(|input| {
                    let sources = input
                        .installed_files
                        .iter()
                        .map(|(m, f)| format!("{m}/{f}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "{}: {}{}",
                        input.name,
                        input
                            .version
                            .as_deref()
                            .unwrap_or(t("info_version_unknown")),
                        if sources.is_empty() {
                            String::new()
                        } else {
                            format!(" [{sources}]")
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            let fingerprint = receipt
                .executable
                .as_ref()
                .map(|exe| {
                    tf(
                        "info_fingerprint",
                        &[
                            ("path", &exe.path.display()),
                            ("size", &exe.size),
                            ("modified", &format_when(exe.modified)),
                        ],
                    )
                })
                .unwrap_or_else(|| t("info_unavailable").into());
            tf(
                "info_receipt",
                &[
                    ("tool", &receipt.tool),
                    ("when", &eidos_instance::format_stamp(receipt.started_at)),
                    ("profile", &receipt.profile),
                    ("n", &output.paths.len()),
                    ("state", &state),
                    ("inputs", &versions),
                    ("command", &receipt.command.join(" ")),
                    ("fingerprint", &fingerprint),
                ],
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Conflicts tab: which files this mod overrides, and which it loses, by mod name.
pub(crate) fn info_conflicts<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(cmap) = &app.conflicts else {
        return text(t("info_conflicts_not_computed")).size(sz(12)).into();
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
                let losers: Vec<&str> = node
                    .alternatives
                    .iter()
                    .filter(|&&a| a != 0)
                    .map(|&a| cmap.name(a))
                    .collect();
                wins.push((node.display_path.clone(), losers.join(", ")));
            }
        } else if node.winner != origin && node.winner != 0 && node.alternatives.contains(&origin) {
            loses_n += 1;
            if loses.len() < SHOWN {
                loses.push((
                    node.display_path.clone(),
                    cmap.name(node.winner).to_string(),
                ));
            }
        }
    }
    let mut col = Column::new().spacing(2);
    col = col.push(text(tf("info_overrides", &[("n", &wins_n)])).size(sz(13)));
    if wins.is_empty() {
        col = col.push(text(format!("  {}", t("info_none"))).size(sz(11)));
    }
    for (p, who) in &wins {
        col = col.push(text(format!("  {p}   >   {who}")).size(sz(11)));
    }
    col = col
        .push(Space::new().height(Length::Fixed(8.0)))
        .push(text(tf("info_overridden_by", &[("n", &loses_n)])).size(sz(13)));
    if loses.is_empty() {
        col = col.push(text(format!("  {}", t("info_none"))).size(sz(11)));
    }
    for (p, who) in &loses {
        col = col.push(text(format!("  {p}   <   {who}")).size(sz(11)));
    }
    col.push(archive_member_rows(app, cmap, origin, 100)).into()
}

/// Filetree tab: every file the mod ships, relative to its root, each with a
/// Hide / Unhide toggle. Unlike the Data tab this is the mod's REAL contents, so
/// hidden files are listed (with their suffix) and are the only place to unhide
/// one individually.
pub(crate) fn info_filetree<'a>(app: &App, i: usize, m: &ModEntry) -> Element<'a, Message> {
    let entries = cached_entries(app, &m.path);
    let hidden = entries.iter().filter(|e| path_is_hidden(e)).count();
    let summary = if hidden == 0 {
        tf("info_n_files", &[("n", &entries.len())])
    } else {
        tf(
            "info_n_files_hidden",
            &[("n", &entries.len()), ("hidden", &hidden)],
        )
    };
    let mut col = Column::new().spacing(1).push(text(summary).size(sz(12)));
    let mut tools = Row::new().spacing(6).align_y(iced::Alignment::Center);
    if hidden > 0 {
        tools = tools.push(tool_btn(
            t("info_unhide_all"),
            Message::RestoreHiddenFiles(i),
        ));
    }
    tools = tools.push(tool_btn(
        t("info_new_folder"),
        Message::FiletreeNewFolderStart(i),
    ));
    col = col.push(tools);
    if let Some((_, name)) = app.tree_new_folder.as_ref().filter(|(n, _)| *n == m.name) {
        col = col.push(
            Row::new()
                .spacing(6)
                .align_y(iced::Alignment::Center)
                .push(
                    text_input(t("info_folder_name"), name)
                        .on_input(Message::FiletreeNewFolderChanged)
                        .on_submit(Message::FiletreeNewFolderCommit)
                        .padding(4)
                        .size(sz(11)),
                )
                .push(
                    button(text(t("info_create")).size(sz(10)))
                        .padding([1, 5])
                        .style(button::primary)
                        .on_press(Message::FiletreeNewFolderCommit),
                ),
        );
    }
    for e in entries.iter().take(2000) {
        let is_hidden = path_is_hidden(e);
        let label = if is_hidden {
            t("info_unhide")
        } else {
            t("info_hide")
        };
        // Renaming replaces the last component only, so the box holds the NAME.
        let renaming = app
            .tree_rename
            .as_ref()
            .is_some_and(|(mn, r)| *mn == m.name && r == e);
        let armed = app
            .tree_delete_armed
            .as_ref()
            .is_some_and(|(mn, r)| *mn == m.name && r == e);
        let name_cell: Element<'a, Message> = if renaming {
            text_input(t("info_new_name"), &app.tree_rename_text)
                .on_input(Message::FiletreeRenameChanged)
                .on_submit(Message::FiletreeRenameCommit)
                .padding(2)
                .size(sz(11))
                .width(Length::Fill)
                .into()
        } else {
            text(e.clone()).size(sz(11)).width(Length::Fill).into()
        };
        let mut row = Row::new()
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .push(name_cell);
        if renaming {
            row = row
                .push(
                    button(text(t("common_rename")).size(sz(10)))
                        .padding([1, 5])
                        .style(button::primary)
                        .on_press(Message::FiletreeRenameCommit),
                )
                .push(
                    button(text(t("common_cancel")).size(sz(10)))
                        .padding([1, 5])
                        .style(button::secondary)
                        .on_press(Message::FiletreeRenameCancel),
                );
        } else {
            row = row
                .push(
                    button(text(t("info_view")).size(sz(10)))
                        .padding([1, 5])
                        .style(button::text)
                        .on_press_maybe(resolve_in_mod(&m.path, e).map(Message::PreviewFile)),
                )
                .push(
                    button(text(t("common_open")).size(sz(10)))
                        .padding([1, 5])
                        .style(button::text)
                        .on_press(Message::FiletreeOpen(i, e.clone())),
                )
                .push(
                    button(text(t("common_rename")).size(sz(10)))
                        .padding([1, 5])
                        .style(button::text)
                        .on_press(Message::FiletreeRenameStart(i, e.clone())),
                )
                .push(
                    button(text(label).size(sz(10)))
                        .padding([1, 5])
                        .on_press(Message::ToggleFileHidden(i, e.clone()))
                        .style(if is_hidden {
                            button::primary
                        } else {
                            button::secondary
                        }),
                )
                // Deleting a mod's own file is the one action here that cannot
                // be undone by clicking again - hiding renames, and renaming
                // renames back. Two clicks, like everything else that removes.
                .push(
                    button(
                        text(if armed {
                            t("info_delete_confirm")
                        } else {
                            t("common_delete")
                        })
                        .size(sz(10)),
                    )
                    .padding([1, 5])
                    .style(if armed {
                        button::danger
                    } else {
                        button::secondary
                    })
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
        col = col.push(text(tf("info_tree_truncated", &[("n", &entries.len())])).size(sz(11)));
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
        if n > 0
            && out
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_symlink())
        {
            return None;
        }
        out.push(part);
    }
    // The LAST component too. Without this the walk checked every ancestor and
    // then handed back a path whose final element is a link - which a preview
    // reads through, and a delete unlinks (harmlessly) while a rename moves.
    if out
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
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
            .push(text(t("info_no_tweaks")).size(sz(12)))
            .push(text(t("info_tweaks_hint")).size(sz(10)))
            .into();
    }
    let enabled: Vec<String> = app
        .created
        .as_ref()
        .map(|inst| inst.mod_meta(&m.name).ini_tweaks().to_vec())
        .unwrap_or_default();

    let mut col = Column::new()
        .spacing(3)
        .push(text(t("info_tweaks_note")).size(sz(11)));
    for name in available {
        let on = enabled.iter().any(|e| e.eq_ignore_ascii_case(&name));
        let label = name.clone();
        col = col.push(
            checkbox(on)
                .label(label)
                .on_toggle(move |_| Message::ToggleIniTweak(i, name.clone()))
                .size(sz(13))
                .text_size(sz(12)),
        );
    }
    col.into()
}

/// Notes tab: an editable note persisted to the mod's meta.ini.
pub(crate) fn info_notes<'a>(app: &App) -> Element<'a, Message> {
    Column::new()
        .spacing(8)
        .push(text(t("info_note_label")).size(sz(12)))
        .push(
            text_input(t("info_note_placeholder"), &app.notes_edit)
                .on_input(Message::NotesChanged)
                .on_submit(Message::NotesSave)
                .padding(6)
                .size(sz(12)),
        )
        .push(tool_btn(t("info_save_note"), Message::NotesSave))
        .into()
}

/// MO2's per-mod info dialog: a centered modal with General / Conflicts /
/// Filetree / Notes tabs.
pub(crate) fn mod_info_dialog<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(m) = app.mods.get(i) else {
        return Space::new()
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into();
    };

    let title = Row::new()
        .spacing(8)
        .push(text(m.name.clone()).size(sz(16)).width(Length::Fill))
        .push(
            button(text(t("common_close")).size(sz(12)))
                .padding([3, 10])
                .on_press(Message::CloseInfo)
                .style(button::secondary),
        );

    let tabs = Row::new()
        .spacing(4)
        .push(info_tab_btn(
            t("info_tab_general"),
            InfoTab::General,
            info_mix(app, InfoTab::General),
        ))
        .push(info_tab_btn(
            t("info_tab_conflicts"),
            InfoTab::Conflicts,
            info_mix(app, InfoTab::Conflicts),
        ))
        .push(info_tab_btn(
            t("info_tab_filetree"),
            InfoTab::Filetree,
            info_mix(app, InfoTab::Filetree),
        ))
        .push(info_tab_btn(
            t("info_tab_ini_tweaks"),
            InfoTab::IniTweaks,
            info_mix(app, InfoTab::IniTweaks),
        ))
        .push(info_tab_btn(
            t("info_tab_notes"),
            InfoTab::Notes,
            info_mix(app, InfoTab::Notes),
        ));

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
            background: Some(Background::Color(pal().bg_primary)),
            border: Border {
                color: accent(),
                width: 2.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
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

#[cfg(test)]
mod tests;
