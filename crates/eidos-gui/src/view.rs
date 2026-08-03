//! The main window: menu bar, toolbar, the mod and plugin lists, the tabs on the
//! right, and the status bar. MO2's layout.
//!
//! Split out of `main.rs` unchanged. Everything here reads `App` and returns an
//! `Element`; the decisions live in `update`.

use crate::theme::*;
use crate::widgets::*;
use crate::*;

pub(crate) const C_CHECK: Length = Length::Fixed(36.0);
pub(crate) const C_PRIO: Length = Length::Fixed(26.0);
pub(crate) const C_FLAGS: Length = Length::Fixed(46.0);
pub(crate) const C_VERSION: Length = Length::Fixed(64.0);
pub(crate) const C_CATEGORY: Length = Length::Fixed(96.0);
pub(crate) const C_CONTENT: Length = Length::Fixed(78.0);

/// Every file in the Overwrite as `/`-joined paths relative to it (recursive).
/// [`overwrite_entries`] memoised against the view generation: the Overwrite tab
/// and the mod-info file tree re-render constantly, and each render used to walk
/// the whole tree again. Rebuilds only after something changes on disk.
pub(crate) fn cached_entries(app: &App, dir: &Path) -> Vec<String> {
    let gen = app.view_generation.get();
    if let Some((at, entries)) = app.listing_cache.borrow().get(dir) {
        if *at == gen {
            return entries.clone();
        }
    }
    let entries = overwrite_entries(dir);
    app.listing_cache.borrow_mut().insert(dir.to_path_buf(), (gen, entries.clone()));
    entries
}

/// One drawn line of the Overwrite tree.
pub(crate) struct OwRow {
    pub(crate) depth: usize,
    /// `/`-joined path relative to the Overwrite: the expansion key.
    pub(crate) rel: String,
    pub(crate) name: String,
    /// `Some(n)` for a folder holding `n` files (recursively), `None` for a file.
    pub(crate) files: Option<usize>,
}

/// The immediate children of `dir` inside a SORTED list of `/`-joined FILE paths.
///
/// The list is the one the tab already had and already caches, so the tree costs
/// no extra disk read - it is derived, not gathered. Sortedness is what makes it
/// cheap: every descendant of `dir` is one contiguous run, found by binary search,
/// and only a run belonging to an EXPANDED folder is ever scanned. A collapsed
/// Overwrite of 4902 files touches a few dozen strings.
///
/// Folders first, then files, each alphabetically - the order MO2 uses.
pub(crate) fn tree_children(entries: &[String], dir: &str) -> Vec<(String, Option<usize>)> {
    let prefix = if dir.is_empty() { String::new() } else { format!("{dir}/") };
    let lo = entries.partition_point(|e| e.as_str() < prefix.as_str());
    let mut dirs: Vec<(String, usize)> = Vec::new();
    let mut files: Vec<String> = Vec::new();
    for e in &entries[lo..] {
        let Some(rest) = e.strip_prefix(prefix.as_str()) else { break };
        match rest.split_once('/') {
            // A folder: count every file under it by extending the current run
            // rather than searching again.
            Some((head, _)) => match dirs.last_mut() {
                Some((n, c)) if n == head => *c += 1,
                _ => dirs.push((head.to_string(), 1)),
            },
            None => files.push(rest.to_string()),
        }
    }
    dirs.into_iter()
        .map(|(n, c)| (n, Some(c)))
        .chain(files.into_iter().map(|n| (n, None)))
        .collect()
}

/// Flatten the expanded parts of the Overwrite into the rows to draw, depth
/// first. Bounded by `limit` for the same reason the Data tree is: the point of
/// opening one level at a time is not to build the other 4900 rows.
pub(crate) fn overwrite_tree_rows(app: &App, entries: &[String], limit: usize) -> Vec<OwRow> {
    fn walk(
        app: &App,
        entries: &[String],
        dir: &str,
        depth: usize,
        limit: usize,
        out: &mut Vec<OwRow>,
    ) {
        if out.len() >= limit || depth > 32 {
            return;
        }
        for (name, files) in tree_children(entries, dir) {
            if out.len() >= limit {
                return;
            }
            let rel = if dir.is_empty() { name.clone() } else { format!("{dir}/{name}") };
            let expanded = files.is_some() && app.overwrite_expanded.contains(&rel);
            out.push(OwRow { depth, rel: rel.clone(), name, files });
            if expanded {
                walk(app, entries, &rel, depth + 1, limit, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(app, entries, "", 0, limit, &mut out);
    out
}

pub(crate) fn overwrite_entries(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(root, &p, out);
            } else if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Delete everything inside a directory, keeping the directory itself.
pub(crate) fn clear_dir_contents(dir: &Path) -> std::io::Result<()> {
    for e in fs::read_dir(dir)?.flatten() {
        let p = e.path();
        if p.is_dir() {
            fs::remove_dir_all(&p)?;
        } else {
            fs::remove_file(&p)?;
        }
    }
    Ok(())
}

/// [`merged_listing`] memoised per directory against the view generation - it
/// read every enabled mod's directory on each redraw of the Data tab.
pub(crate) fn cached_merged_listing(app: &App, dir: &str) -> Vec<DataRow> {
    let gen = app.view_generation.get();
    if let Some((at, entries)) = app.data_listing.borrow().get(dir) {
        if *at == gen {
            return entries.clone();
        }
    }
    let entries = merged_listing(app, dir);
    app.data_listing.borrow_mut().insert(dir.to_string(), (gen, entries.clone()));
    entries
}

/// The entries of ONE directory of the merged view (`dir` relative to `Data`,
/// `""` for the root): each name, the source providing it (highest-priority
/// enabled mod, or the game data), and whether it's a folder. Winner attribution
/// matches what the FUSE layer actually serves: Overwrite first, then mods from
/// HIGHEST display priority down, then the game data.
///
/// One level at a time, so expanding a node costs one directory read per layer
/// that has it rather than a full recursive walk of every enabled mod.
pub(crate) fn merged_listing(app: &App, dir: &str) -> Vec<DataRow> {
    let mut seen = HashSet::new();
    let mut out: Vec<DataRow> = Vec::new();
    let take = |root: &Path, source: &str, seen: &mut HashSet<String>, out: &mut Vec<DataRow>| {
        let base = if dir.is_empty() { root.to_path_buf() } else { root.join(dir) };
        let Ok(rd) = fs::read_dir(base) else { return };
        for e in rd.flatten() {
            let Ok(name) = e.file_name().into_string() else { continue };
            // Hidden entries are out of the virtual view (eidos-core drops them
            // from the mount too), so the Data tree must not show them as winners
            // - the point of hiding is that the layer below wins instead.
            if eidos_core::is_hidden_name(&name) {
                continue;
            }
            if seen.insert(name.to_lowercase()) {
                out.push((name, source.to_string(), e.path().is_dir()));
            }
        }
    };
    if let Some(inst) = app.created.as_ref() {
        take(&inst.overwrite_dir(), "[Overwrite]", &mut seen, &mut out);
    }
    // `app.mods` is display order = lowest priority first; the merged view's
    // winner is the highest, so walk it in reverse.
    for m in app.mods.iter().rev().filter(|m| m.enabled && !m.is_separator()) {
        take(&m.path, &m.name, &mut seen, &mut out);
    }
    if let Some(g) = selected_game(app) {
        let label = format!("[{}]", g.def.id);
        take(&g.data_path, &label, &mut seen, &mut out);
    }
    // Folders first, then files, each alphabetically - the ordering every file
    // browser uses, and the one that makes a deep tree navigable.
    out.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
    out
}

/// MO2's hide/unhide, which is a rename and never a delete (`filetree.cpp:375-391`
/// drives exactly this through a `FileRenamer` constructed with HIDE / UNHIDE):
/// hiding appends `.mohidden`, unhiding strips it. Refuses to hide something
/// already hidden or unhide something that is not, so a stale row cannot
/// double-suffix a file into `foo.dds.mohidden.mohidden`.
///
/// Works on directories too - hiding `meshes/` suppresses the whole subtree.
pub(crate) fn set_hidden(path: &Path, hide: bool) -> std::io::Result<PathBuf> {
    use std::io::{Error, ErrorKind};
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "unusable file name"))?;
    let already = eidos_core::is_hidden_name(name);
    if already == hide {
        let what = if hide { "already hidden" } else { "not hidden" };
        return Err(Error::new(ErrorKind::AlreadyExists, what));
    }
    let target = if hide {
        path.with_file_name(format!("{name}{}", eidos_core::HIDDEN_SUFFIX))
    } else {
        path.with_file_name(&name[..name.len() - eidos_core::HIDDEN_SUFFIX.len()])
    };
    // Never let a hide silently swallow an existing file: unhiding onto a name the
    // mod already carries would destroy the live copy.
    if target.symlink_metadata().is_ok() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            format!("{} already exists", target.display()),
        ));
    }
    fs::rename(path, &target)?;
    Ok(target)
}

/// Unhide everything under `root`, MO2's `restoreHiddenFiles`. Returns how many
/// entries were restored.
///
/// Deepest first, so renaming a hidden directory never invalidates the paths of
/// the hidden files collected inside it.
pub(crate) fn restore_hidden_files(root: &Path) -> std::io::Result<usize> {
    fn collect(dir: &Path, depth: usize, out: &mut Vec<(usize, PathBuf)>) {
        if depth > 32 {
            return;
        }
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(&p, depth + 1, out);
            }
            if p.file_name().and_then(|n| n.to_str()).is_some_and(eidos_core::is_hidden_name) {
                out.push((depth, p));
            }
        }
    }
    let mut found = Vec::new();
    collect(root, 0, &mut found);
    found.sort_by(|a, b| b.0.cmp(&a.0));
    let mut done = 0;
    for (_, p) in found {
        if set_hidden(&p, false).is_ok() {
            done += 1;
        }
    }
    Ok(done)
}

/// One rendered line of the Data tree: how deep it sits, its full path relative
/// to `Data` (the expansion key and what a hide acts on), and the merged-listing
/// entry itself.
pub(crate) struct TreeRow {
    pub(crate) depth: usize,
    pub(crate) rel: String,
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) is_dir: bool,
}

/// Flatten the expanded parts of the merged tree into the rows to draw, depth
/// first. Bounded by `limit`: a fully-expanded Skyrim Data tree is six figures of
/// files, and the whole point of expanding a level at a time is not to build them.
pub(crate) fn data_tree_rows(app: &App, limit: usize) -> Vec<TreeRow> {
    fn walk(app: &App, dir: &str, depth: usize, limit: usize, out: &mut Vec<TreeRow>) {
        // Guard against a pathological tree as well as the row budget: a symlink
        // loop inside a mod would otherwise recurse until the stack gives out.
        if out.len() >= limit || depth > 32 {
            return;
        }
        for (name, source, is_dir) in cached_merged_listing(app, dir) {
            if out.len() >= limit {
                return;
            }
            let rel = if dir.is_empty() { name.clone() } else { format!("{dir}/{name}") };
            let expanded = is_dir && app.data_expanded.contains(&rel);
            out.push(TreeRow { depth, rel: rel.clone(), name, source, is_dir });
            if expanded {
                walk(app, &rel, depth + 1, limit, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(app, "", 0, limit, &mut out);
    out
}

/// The menu bar. iced 0.13 has no native dropdown widget, so most top-level items
/// fire the single most useful action (MO2's most-used per menu): File -> open the
/// instance folder, Tools -> Executables, Run -> run the current target, Help ->
/// About. View opens a small floating menu (it has several toggles to host).
pub(crate) fn menu_bar<'a>() -> Element<'a, Message> {
    let row = Row::new()
        .spacing(0)
        .push(flat_btn("File", Message::OpenInstanceFolder))
        .push(flat_btn("View", Message::OpenViewMenu))
        .push(flat_btn("Tools", Message::ShowExecutablesDialog))
        // Shortcut hints inline, MO2-style (the keys are wired in `subscription`).
        .push(flat_btn("Run (Ctrl+R)", Message::Run))
        .push(flat_btn("Refresh (F5)", Message::Refresh))
        .push(flat_btn("Help", Message::ShowAbout));
    container(row).width(Length::Fill).padding(1).style(bar_style).into()
}

/// The View dropdown's contents (floats over the window via the Stack, dismissed
/// by a click outside). Hosts the toolbar/status-bar toggles + collapse/expand-all.
pub(crate) fn view_menu_card<'a>(app: &App) -> Element<'a, Message> {
    let toolbar_label = if app.ui_toolbar_visible { "Hide toolbar" } else { "Show toolbar" };
    let status_label = if app.ui_statusbar_visible { "Hide status bar" } else { "Show status bar" };
    let col = Column::new()
        .spacing(1)
        .push(menu_item(toolbar_label, Message::ToggleToolbar))
        .push(menu_item(status_label, Message::ToggleStatusBar))
        .push(menu_sep())
        .push(menu_item("Collapse all groups", Message::CollapseAllGroups))
        .push(menu_item("Expand all groups", Message::ExpandAllGroups));
    menu_frame(col.into())
}

pub(crate) fn toolbar<'a>(app: &App) -> Element<'a, Message> {
    // Greyed (no on_press) while a Nexus call for that action is in flight; the
    // first selected mod is the endorse target (MO2's toolbar endorse button); it
    // must be a real mod with a Nexus id to act on.
    let endorse_target = app.selected_mod.filter(|&i| {
        app.mods.get(i).is_some_and(|m| {
            !m.is_separator()
                && app.meta_cache.get(&m.name).and_then(|r| r.mod_id).is_some()
        })
    });
    let endorse_msg = (app.endorsing.is_none()).then(|| endorse_target.map(Message::ModEndorse)).flatten();
    let update_msg = (!app.update_in_progress).then_some(Message::CheckUpdates);
    let row = Row::new()
        .spacing(2)
        .push(icon_text_btn(IC_INSTALL, "Install Mod", Message::InstallMod))
        .push(icon_text_btn(IC_NEXUS, "Nexus", Message::OpenNexusGame))
        .push(icon_text_btn(IC_CHANGE_GAME, "Change Game", Message::ChangeGame))
        .push(icon_text_btn(IC_REFRESH, "Refresh", Message::Refresh))
        .push(icon_text_btn(IC_EXECUTABLES, "Executables", Message::ShowExecutablesDialog))
        .push(icon_text_btn(IC_TOOLS, "Tool Setup", Message::SetupPrereqs))
        .push(icon_text_btn(IC_SETTINGS, "Settings", Message::OpenSettings))
        .push(Space::new().width(Length::Fill))
        .push(icon_btn(IC_ENDORSE, 20.0, endorse_msg))
        .push(icon_btn(IC_UPDATE, 20.0, update_msg))
        .push(icon_btn(IC_HELP, 20.0, Some(Message::ShowAbout)));
    container(row).width(Length::Fill).padding(2).style(bar_style).into()
}

#[allow(clippy::too_many_arguments)]
/// A name on ONE line, cut to its column and dissolved into `bg` at the edge.
///
/// MO2 elides with an ellipsis (Qt's default `ElideRight`; it only switches to
/// `ElideLeft` for path columns, where the end is what matters). iced 0.14 has
/// no elision, and faking one means measuring a proportional font to decide
/// where to cut the string - a guess that is wrong again at every window width.
///
/// A fade needs no measurement at all. The gradient runs from transparent to the
/// row's own colour, so where the name is short it blends the background into
/// the background and cannot be seen; where the name overflows it dissolves the
/// cut. It re-answers the question on every resize without being asked.
pub(crate) fn name_cell<'a>(name: String, bg: Color) -> Element<'a, Message> {
    let label = container(
        // Without this the text wraps and the row grows; with it, and without
        // the clip below, the text would instead paint over the next column -
        // iced does not clip text to its node.
        text(name).size(13.0).wrapping(iced::widget::text::Wrapping::None),
    )
    .width(Length::Fill)
    .clip(true);

    let fade = container(Space::new())
        .width(Length::Fixed(NAME_FADE_W))
        .height(Length::Fill)
        .style(move |_t: &Theme| container::Style {
            background: Some(Background::Gradient(iced::Gradient::Linear(
                // Left to right: nothing, then the row colour.
                iced::gradient::Linear::new(std::f32::consts::FRAC_PI_2)
                    .add_stop(0.0, Color { a: 0.0, ..bg })
                    .add_stop(1.0, bg),
            ))),
            ..Default::default()
        });

    Stack::new()
        .width(Length::Fill)
        .push(label)
        // Pinned right: the Fill spacer pushes the fade to the edge of whatever
        // width the column ends up with.
        .push(Row::new().push(Space::new().width(Length::Fill)).push(fade))
        .into()
}

pub(crate) fn mod_row<'a>(
    i: usize,
    m: &ModEntry,
    meta: Option<&RowMeta>,
    flag_icon: Option<&'static [u8]>,
    hidden_icon: Option<&'static [u8]>,
    bg: Color,
) -> Element<'a, Message> {
    // Unmanaged content - the game's own DLCs and Creation Club plugins - is
    // listed so the mod list matches what will actually load, but none of it is
    // ours to move, disable or remove. MO2 renders these the same way: present,
    // greyed, inert. A checkbox with no `on_toggle` draws disabled, which is
    // exactly the look.
    let toggle = if m.unmanaged {
        checkbox(true).size(16)
    } else {
        checkbox(m.enabled).on_toggle(move |_| Message::ToggleMod(i)).size(16)
    };

    // MO2's conflict emblem plus an optional hidden-files glyph (a mod can be both).
    let mut flags = Row::new().spacing(2);
    if let Some(bytes) = flag_icon {
        flags = flags.push(icon(bytes, 14.0));
    }
    if let Some(bytes) = hidden_icon {
        flags = flags.push(icon(bytes, 14.0));
    }
    let flag_cell: Element<'a, Message> = container(flags).width(C_FLAGS).into();

    // MO2's Version column; an update marker prefixes it when Nexus has a newer one.
    let version = meta.and_then(|r| r.version.clone()).unwrap_or_default();
    let version = match meta {
        Some(r) if r.update => format!("^ {version}"),
        _ => version,
    };
    // MO2's Category column: the mod's primary category, resolved to a name.
    let category = meta.and_then(|r| r.category_name.clone()).unwrap_or_default();
    // MO2's Content column: a compact letters summary of what the mod ships.
    let content = meta.map(|r| r.content_tags.clone()).unwrap_or_default();

    let row = Row::new()
        .spacing(6)
        .height(Length::Fixed(MOD_ROW_H))
        .align_y(iced::Alignment::Center)
        .push(container(toggle).width(C_CHECK))
        .push(text(format!("{:>2}", i + 1)).size(12.0).width(C_PRIO))
        .push(name_cell(m.name.clone(), bg))
        .push(
            text(if m.unmanaged { "Game content".to_string() } else { category })
                .size(11.0)
                .width(C_CATEGORY),
        )
        .push(text(content).size(10.0).width(C_CONTENT))
        .push(text(version).size(11.0).width(C_VERSION))
        .push(flag_cell);

    // Left-press selects + arms a drag, entering during a drag retargets the drop,
    // release commits it; right-click opens the action menu (MO2's context menu).
    // Inner buttons still get their own clicks; the mouse_area catches the rest.
    if m.unmanaged {
        // No drag, no context menu: there is no action on this row that would do
        // anything, and offering one only invites the question of why it failed.
        return container(row).into();
    }
    mouse_area(row)
        .on_press(Message::DragStart(i))
        .on_enter(Message::DragOverGap(i))
        .on_release(Message::DragDrop)
        .on_right_press(Message::OpenModMenu(i))
        .into()
}

/// Default separator bar colour when its `meta.ini` carries none (a parchment tan,
/// #C8B895).
pub(crate) const SEPARATOR_ACCENT: Color = Color::from_rgb(0.784, 0.722, 0.584);

/// A separator (group divider) row, MO2-style: a full-width coloured bar with the
/// display name centred, no checkbox / version / conflict flags, but still movable.
pub(crate) fn separator_row<'a>(
    i: usize,
    m: &ModEntry,
    color: Option<[u8; 3]>,
    collapsed: bool,
    selected: bool,
) -> Element<'a, Message> {
    let bg = color.map(|[r, g, b]| Color::from_rgb8(r, g, b)).unwrap_or(SEPARATOR_ACCENT);

    // The collapse/expand toggle sits in the checkbox column (a separator has no
    // checkbox); it hides/shows the mods grouped beneath this separator.
    let collapse = button(text(if collapsed { "[+]" } else { "[-]" }).size(11.0))
        .padding([1, 4])
        .on_press(Message::ToggleCollapse(m.display_name().to_string()))
        .style(button::text);

    // One line here too, and clipped rather than faded: this name is CENTRED, so
    // a fade would have to work from both ends, and a separator that needs one is
    // a separator whose name should be shorter.
    let name = container(
        text(m.display_name().to_string()).size(13.0).wrapping(iced::widget::text::Wrapping::None),
    )
    .width(Length::Fill)
    .clip(true)
    .align_x(iced::alignment::Horizontal::Center);

    let row = Row::new()
        .spacing(6)
        .height(Length::Fixed(MOD_ROW_H))
        .align_y(iced::Alignment::Center)
        .push(container(collapse).width(C_CHECK))
        .push(text(format!("{:>2}", i + 1)).size(12.0).width(C_PRIO))
        .push(name)
        ;

    container(
        mouse_area(row)
            .on_press(Message::DragStart(i))
            .on_enter(Message::DragOverGap(i))
            .on_release(Message::DragDrop)
            .on_right_press(Message::OpenModMenu(i)),
    )
    .width(Length::Fill)
    .padding(3)
    .style(move |_t: &Theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: if selected { Color::from_rgb8(0x6E, 0x24, 0x2E) } else { bg },
            width: if selected { 2.0 } else { 0.0 },
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

pub(crate) fn modlist_pane<'a>(app: &App) -> Element<'a, Message> {
    let active = app.mods.iter().filter(|m| m.enabled && !m.is_separator()).count();
    let active_name = app.created.as_ref().map(|i| i.active_profile()).unwrap_or_default();
    let mut profile = Row::new().spacing(6).push(text("Profile:").size(12.0));
    if let Some(inst) = &app.created {
        for name in inst.profiles() {
            let selected = name == active_name;
            // Left-click switches (MO2's profile selector); right-click opens the
            // rename / copy / delete menu (MO2's Profiles dialog actions).
            let chip = button(text(name.clone()).size(12.0))
                .padding(4)
                .on_press(Message::SwitchProfile(name.clone()))
                .style(if selected { button::primary } else { button::secondary });
            profile = profile
                .push(mouse_area(chip).on_right_press(Message::ProfileMenuOpen(name.clone())));
        }
    }
    let profile = profile
        .push(tool_btn("+ New", Message::NewProfile))
        .push(Space::new().width(Length::Fill))
        .push(
            text(format!(
                "Active: {active}  |  Endorsed: {}  |  Updates: {}",
                app.endorsed_count, app.updated_count
            ))
            .size(12.0),
        );

    // The category catalog (resolves ids -> names; drives the filter + the column).
    let cats = app.categories.as_ref();

    // Category-filter dropdown: "All" + the top-level categories actually in use.
    let mut choices = vec![CategoryChoice { id: None, label: "All categories".to_string() }];
    if let Some(cf) = &cats {
        let mut used: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
        for r in app.meta_cache.values() {
            if let Some(mut cur) = r.category_id {
                for _ in 0..32 {
                    match cf.parent_id(cur) {
                        Some(p) if p != 0 && p != cur => cur = p,
                        _ => break,
                    }
                }
                used.insert(cur);
            }
        }
        for (id, name) in cf.all_top_level() {
            if used.contains(&id) {
                choices.push(CategoryChoice { id: Some(id), label: name.to_string() });
            }
        }
    }
    let selected = choices.iter().find(|c| c.id == app.category_filter).cloned();

    // MO2's mod-list filter box + a category dropdown + a button to drop a separator.
    let search = Row::new()
        .spacing(6)
        .push(
            text_input("Filter mods by name...", &app.search)
                .on_input(Message::SearchChanged)
                .padding(5)
                .size(12.0),
        )
        .push(
            pick_list(choices, selected, |c: CategoryChoice| Message::CategoryFilterChanged(c.id))
                .text_size(12.0)
                .padding(5),
        )
        .push(tool_btn("+ Separator", Message::AddSeparator(0)))
        .push(tool_btn("+ Empty mod", Message::CreateEmptyMod))
        .push(tool_btn("Install folder", Message::InstallFromFolder));

    let header = Row::new()
        .spacing(6)
        .push(text("").width(C_CHECK))
        .push(text("#").size(11.0).width(C_PRIO))
        .push(text("Mod Name").size(11.0).width(Length::Fill))
        .push(text("Category").size(11.0).width(C_CATEGORY))
        .push(text("Content").size(11.0).width(C_CONTENT))
        .push(text("Version").size(11.0).width(C_VERSION))
        .push(text("Flags").size(11.0).width(C_FLAGS))
        ;

    let query = app.search.trim().to_lowercase();
    // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
    let mut shown = 0usize;
    if app.mods.is_empty() {
        list = list.push(text("No mods yet. Drop mod folders into the instance's mods/ dir.").size(12.0));
    }
    // Decided up front, because whether a separator draws depends on whether any
    // mod BELOW it survives the filter - which the single downward pass this used
    // to be could not know when it reached the header.
    let filtering = !query.is_empty() || app.category_filter.is_some();
    let vis = mod_row_visibility(app, cats);
    // The live drag's insertion point, if any, so exactly one gap draws the line.
    // A drag that has not moved off its own row targets nothing visible: a plain
    // click must never flash an indicator.
    let live_gap = app
        .drag_state
        .filter(|d| d.gap != d.from && d.gap != d.from + 1)
        .map(|d| d.gap);
    let dragging = app.drag_state.is_some();
    for (i, m) in app.mods.iter().enumerate() {
        // A row is highlighted when it is the focus row or in the multi-selection.
        let selected = app.selected_mod == Some(i) || app.selected_mods.contains(&i);
        // A separator renders as a full-width group header - no checkbox, version,
        // conflict flags, or content (it never queries the ConflictMap). It draws
        // whatever its own group's fold state, since a folded header is exactly
        // what the user clicks to unfold; a filter hides it like any other row.
        if m.is_separator() {
            if !vis[i] {
                continue;
            }
            // Folding is suspended under a filter, so the header draws unfolded:
            // the mods it heads ARE on screen, and a [+] next to them would lie.
            let collapsed = !filtering && app.collapsed.contains(m.display_name());
            let color = app.meta_cache.get(&m.name).and_then(|r| r.color);
            // Every VISIBLE row gets a strip above it, separators included, or the
            // slot just before a group header would be unreachable.
            list = list.push(drop_gap(i, live_gap == Some(i), dragging, Message::DragOverGap, Message::DragDrop));
            list = list.push(separator_row(i, m, color, collapsed, selected));
            continue;
        }
        if !vis[i] {
            continue;
        }
        shown += 1;
        // MO2's conflict emblems; a disabled mod shows none (the checkbox says it).
        let flag_icon = if !m.enabled {
            None
        } else if let Some(c) = &app.conflicts {
            match c.state((i + 1) as u32) {
                ConflictState::Overwrites => Some(IC_CONFLICT_OVERWRITE),
                ConflictState::Overwritten => Some(IC_CONFLICT_OVERWRITTEN),
                ConflictState::Mixed => Some(IC_CONFLICT_MIXED),
                ConflictState::Redundant => Some(IC_CONFLICT_REDUNDANT),
                ConflictState::None => None,
            }
        } else {
            None
        };
        // A separate hidden-files glyph (MO2's FLAG_HIDDEN_FILES), shown alongside.
        let hidden_icon = if m.enabled {
            app.conflicts
                .as_ref()
                .and_then(|c| c.mods.get(&((i + 1) as u32)))
                .filter(|mc| mc.has_hidden)
                .map(|_| IC_CONFLICT_HIDDEN)
        } else {
            None
        };
        let meta = app.meta_cache.get(&m.name);
        // The insertion strip ABOVE this row. Always rendered (stable layout),
        // targetable during a drag. Every gap is a target, the very top included:
        // the game's own content is written to modlist.txt now, so a row landing
        // above it keeps its place.
        list = list.push(drop_gap(i, live_gap == Some(i), dragging, Message::DragOverGap, Message::DragDrop));
        // Computed once and handed to both: the row paints this colour, and the
        // name cell fades into it.
        let conflict = conflict_tint(app, i);
        let bg = row_background(i % 2 == 0, selected, conflict);
        list = list.push(list_row(
            mod_row(i, m, meta, flag_icon, hidden_icon, bg),
            i % 2 == 0,
            selected,
            conflict,
        ));
    }
    // The trailing strip: the only way to aim at the end of the list, since
    // hovering a row always means "above it".
    if !app.mods.is_empty() {
        let end = app.mods.len();
        list = list.push(drop_gap(end, live_gap == Some(end), dragging, Message::DragOverGap, Message::DragDrop));
    }
    // `shown` counts mods only, so this cannot fire on a list that is all folded
    // groups - and it only speaks when something was actually asked.
    if !app.mods.is_empty() && shown == 0 && filtering {
        let by = match (query.is_empty(), app.category_filter.is_some()) {
            (false, false) => format!("named \"{}\"", app.search.trim()),
            (true, _) => "in this category".to_string(),
            (false, true) => format!("named \"{}\" in this category", app.search.trim()),
        };
        list = list.push(text(format!("No mods {by}.")).size(12.0));
    }

    let overwrite = button(
        Row::new()
            .spacing(6)
            .push(text("").width(C_CHECK))
            .push(text("").width(C_PRIO))
            .push(text("Overwrite").size(13.0).width(Length::Fill)),
    )
    .padding(2)
    .on_press(Message::SelectTab(Tab::Overwrite))
    .style(button::text);

    // Wrap the list so the pointer leaving its bounds during a drag cancels it
    // (MO2 drops nothing when you release outside the list).
    // `on_release` here is the catch-all: a row or a strip that handles the
    // release captures it and this never fires, but a release landing anywhere
    // else in the list - a header, a gap the layout moved, empty space below the
    // last row - disarms instead of leaving a drag live for the next click to
    // commit. `on_exit` covers releasing outside the list entirely.
    let list_area = mouse_area(scrollable(list).id(mod_scroll_id()).height(Length::Fill))
        .on_exit(Message::DragCancel)
        .on_release(Message::DragCancel);

    // ALWAYS in the flow, at a fixed height, even when it has nothing to say.
    // Appearing and disappearing moved every row below it by its own height, so
    // clicking a mod scrolled the list out from under the pointer - and if the
    // button came up somewhere that was no longer a row, the armed drag was
    // never released and the next click moved the mod. The same mistake the
    // insertion strips were built to avoid, made again one panel over.
    let legend = container(conflict_legend(app).unwrap_or_else(|| Space::new().width(0).height(0).into()))
        // Tall enough for the 12px swatch and the 11pt label without clipping,
        // and identical whether or not there is anything to show.
        .height(Length::Fixed(20.0))
        .align_y(iced::alignment::Vertical::Center);

    let inner = Column::new()
        .spacing(6)
        .push(profile)
        .push(search)
        .push(legend)
        .push(header)
        .push(list_area)
        .push(overwrite);

    container(inner).width(Length::FillPortion(3)).height(Length::Fill).padding(8).style(panel_style).into()
}

/// A single left-aligned action in the mod context menu.
/// A menu row whose label is owned, so the resulting element borrows nothing.
pub(crate) fn menu_item_owned<'a>(label: String, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(msg)
        .style(button::text)
        .into()
}

pub(crate) fn menu_item<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(msg)
        .style(button::text)
        .into()
}

/// A small separator line inside the context menu.
pub(crate) fn menu_sep<'a>() -> Element<'a, Message> {
    container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
        .padding([2, 6])
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xC8, 0xB8, 0x95))),
            ..Default::default()
        })
        .into()
}

/// MO2's right-click mod menu, rendered as a floating card (the action set from
/// modlistviewactions.cpp: enable/disable, send-to-top/bottom, explorer, Nexus,
/// reinstall, rename, remove). Shows the rename editor when a rename is in flight.
pub(crate) fn mod_menu_card<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(m) = app.mods.get(i) else {
        return Space::new().width(Length::Shrink).height(Length::Shrink).into();
    };

    // When more than one mod is selected, the right-click menu becomes a batch menu
    // (MO2 swaps the per-mod actions for selection-wide ones).
    if app.selected_mods.len() > 1 {
        return batch_mod_menu_card(app);
    }

    let title = Row::new()
        .spacing(6)
        .push(text(m.display_name().to_string()).size(13.0).width(Length::Fill))
        .push(
            button(text("x").size(13.0)).padding([1, 6]).on_press(Message::CloseMenu).style(button::text),
        );

    let mut col = Column::new().spacing(1).push(title);

    // A read-only info line (MO2 surfaces version/category/Nexus id on the row).
    if let Some(r) = app.meta_cache.get(&m.name) {
        let mut bits: Vec<String> = Vec::new();
        if let Some(v) = &r.version {
            bits.push(format!("v{v}"));
        }
        if let Some(c) = &r.category_name {
            bits.push(format!("cat {c}"));
        }
        if let Some(id) = r.mod_id {
            bits.push(format!("Nexus #{id}"));
        }
        if !bits.is_empty() {
            col = col.push(text(bits.join("  ·  ")).size(10.0));
        }
    }

    col = col.push(menu_sep());

    // Inline rename editor (MO2 renameMod) takes over the card while active.
    if let Some((ri, name)) = &app.rename {
        if *ri == i {
            let editor = text_input("New name", name)
                .on_input(Message::RenameChanged)
                .on_submit(Message::RenameCommit)
                .padding(5)
                .size(12.0);
            let actions = Row::new()
                .spacing(6)
                .push(tool_btn("Save", Message::RenameCommit))
                .push(tool_btn("Cancel", Message::CloseMenu));
            col = col.push(editor).push(actions);
            return menu_frame(col.into());
        }
    }

    // A separator gets a reduced menu: rename, colour, reorder, add-above, remove
    // (no enable/disable, information, reinstall, or Nexus - MO2 parity).
    if m.is_separator() {
        let current = app.meta_cache.get(&m.name).and_then(|r| r.color);
        col = col
            .push(menu_item("Rename", Message::RenameStart(i)))
            .push(separator_swatches(i, current))
            .push(menu_sep())
            .push(menu_item("Send to Top", Message::ModSendTop(i)))
            .push(menu_item("Send to Bottom", Message::ModSendBottom(i)))
            // The same targeted moves every other row gets. MO2's separator menu
            // calls `addSendToContextMenu()` unchanged (modlistcontextmenu.cpp:395);
            // the conflict entries drop out on their own, because a separator owns
            // no files and so carries no conflict flags - which is MO2's reason
            // too, rather than a test for separator-ness.
            .push(send_to_targets(app, i))
            .push(menu_item("Add separator above", Message::AddSeparator(i)))
            .push(menu_item("Open in Explorer", Message::ModOpenFolder(i)))
            .push(menu_sep())
            .push(remove_button(app, i));
        return menu_frame(col.into());
    }

    col = col
        .push(menu_item("Information...", Message::ShowModInfo(i)))
        .push(menu_sep())
        .push(menu_item(if m.enabled { "Disable" } else { "Enable" }, Message::ToggleMod(i)))
        .push(menu_sep())
        .push(menu_item("Send to Top", Message::ModSendTop(i)))
        .push(menu_item("Send to Bottom", Message::ModSendBottom(i)))
        .push(send_to_targets(app, i))
        .push(menu_sep())
        .push(menu_item("Open in Explorer", Message::ModOpenFolder(i)));

    // Visit on Nexus + Endorse + Track only when we have a mod id to act on. The
    // Endorse / Track labels reflect the current state (MO2 toggles them).
    let meta = app.created.as_ref().map(|inst| inst.mod_meta(&m.name));
    let has_nexus = app.meta_cache.get(&m.name).and_then(|r| r.mod_id).is_some();
    if has_nexus {
        col = col.push(menu_item("Visit on Nexus", Message::ModVisitNexus(i)));
        let endorsed = meta.as_ref().is_some_and(|mm| mm.endorsed());
        let endorse_label = if endorsed { "Abstain (un-endorse)" } else { "Endorse" };
        col = col.push(menu_item(endorse_label, Message::ModEndorse(i)));
        let tracked = meta.as_ref().is_some_and(|mm| mm.tracked());
        let track_label = if tracked { "Untrack" } else { "Track" };
        col = col.push(menu_item(track_label, Message::ModTrack(i)));
    }
    // Ignore update is a local flag (MO2 shows it for any mod, Nexus id or not).
    let ignored = meta.as_ref().is_some_and(|mm| mm.ignore_update());
    let ignore_label = if ignored { "Check for updates" } else { "Ignore updates" };
    col = col.push(menu_item(ignore_label, Message::ModIgnoreUpdate(i)));

    // Bulk unhide, offered only when the mod actually has hidden files - the
    // conflict scan already tracks that (it is what drives the hidden glyph on the
    // row), so this costs no extra walk.
    let has_hidden = app
        .conflicts
        .as_ref()
        .and_then(|c| c.mods.get(&((i + 1) as u32)))
        .is_some_and(|mc| mc.has_hidden);
    if has_hidden {
        col = col.push(menu_item("Unhide all files", Message::RestoreHiddenFiles(i)));
    }

    col = col
        .push(menu_sep())
        .push(menu_item("Reinstall Mod", Message::ModReinstall(i)))
        .push(menu_item("Rename", Message::RenameStart(i)))
        .push(menu_item("Add separator above", Message::AddSeparator(i)))
        .push(menu_sep())
        .push(remove_button(app, i));

    menu_frame(col.into())
}

/// The batch context menu shown when several mods are selected at once (MO2's
/// multi-row right-click): enable/disable, send-to-top/bottom, and a two-click
/// Remove that wipes the whole selection from disk.
pub(crate) fn batch_mod_menu_card<'a>(app: &App) -> Element<'a, Message> {
    let targets = real_selection(app);
    let n = targets.len();
    // Mirror the batch toggle's decision so the label reads true ("Disable" when
    // any selected mod is on, else "Enable").
    let any_on = targets.iter().any(|&i| app.mods.get(i).is_some_and(|m| m.enabled));
    let toggle_label = if any_on { "Disable selected" } else { "Enable selected" };

    let title = Row::new()
        .spacing(6)
        .push(text(format!("{n} mods selected")).size(13.0).width(Length::Fill))
        .push(
            button(text("x").size(13.0))
                .padding([1, 6])
                .on_press(Message::CloseMenu)
                .style(button::text),
        );

    // Two-click guard: the first click arms (BatchRemoveMods), the second executes
    // (ConfirmBatchRemove). The label + danger style flip once armed.
    let (remove_label, remove_msg) = if app.confirm_batch_remove {
        ("Confirm remove?", Message::ConfirmBatchRemove)
    } else {
        ("Remove selected", Message::BatchRemoveMods)
    };
    let remove = button(text(remove_label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(remove_msg)
        .style(if app.confirm_batch_remove { button::danger } else { button::text });

    let col = Column::new()
        .spacing(1)
        .push(title)
        .push(menu_sep())
        .push(menu_item(toggle_label, Message::BatchToggleMods))
        .push(menu_sep())
        .push(menu_item("Send to Top", Message::BatchSendTop))
        .push(menu_item("Send to Bottom", Message::BatchSendBottom))
        .push(menu_sep())
        .push(remove);
    menu_frame(col.into())
}

/// The two-click Remove button shared by the mod and separator menus.
pub(crate) fn remove_button<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let label = if app.confirm_remove == Some(i) { "Confirm remove?" } else { "Remove" };
    button(text(label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(Message::ModRemove(i))
        .style(if app.confirm_remove == Some(i) { button::danger } else { button::text })
        .into()
}

/// A small palette of colour swatches for a separator (iced has no native colour
/// dialog), plus an "x" to clear back to the default.
pub(crate) fn separator_swatches<'a>(i: usize, current: Option<[u8; 3]>) -> Element<'a, Message> {
    const PALETTE: &[[u8; 3]] = &[
        [0x8b, 0x2e, 0x2e],
        [0x8b, 0x5e, 0x2e],
        [0x6e, 0x6e, 0x2e],
        [0x2e, 0x6e, 0x3e],
        [0x2e, 0x5e, 0x8b],
        [0x5e, 0x2e, 0x8b],
        [0x55, 0x55, 0x55],
    ];
    let mut row = Row::new().spacing(3).align_y(iced::Alignment::Center).push(text("Colour").size(10.0));
    for &rgb in PALETTE {
        let [r, g, b] = rgb;
        let sel = current == Some(rgb);
        let sw = button(Space::new().width(Length::Fixed(15.0)).height(Length::Fixed(13.0)))
            .padding(0)
            .on_press(Message::SetSeparatorColor(i, Some(rgb)))
            .style(move |_t: &Theme, _s: button::Status| button::Style {
                background: Some(Background::Color(Color::from_rgb8(r, g, b))),
                border: Border {
                    color: Color::from_rgb8(0x3a, 0x2a, 0x1a),
                    width: if sel { 2.0 } else { 0.5 },
                    radius: 2.0.into(),
                },
                ..Default::default()
            });
        row = row.push(sw);
    }
    row.push(
        button(text("x").size(10.0))
            .padding([1, 4])
            .on_press(Message::SetSeparatorColor(i, None))
            .style(button::text),
    )
    .into()
}

/// The bordered card chrome around the context menu's contents.
pub(crate) fn menu_frame<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fixed(210.0))
        .padding(6)
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
            border: Border {
                color: Color::from_rgb8(0x6E, 0x24, 0x2E),
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
}
