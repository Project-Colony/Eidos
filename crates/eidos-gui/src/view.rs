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
/// The flags cell's width. The other columns carry their own (see
/// [`ModColumn::width`]); this one is built before the loop that lays them out,
/// so it needs the number here too - and the two must agree.
/// The ground behind a synthetic group header - a wash rather than the
/// separator's full-strength bar, because it is a fact about the list rather
/// than a row in it.
pub(crate) const GROUP_HEADER_BG: Color = Color::from_rgb(0.86, 0.84, 0.79);

pub(crate) const C_FLAGS: Length = Length::Fixed(46.0);

/// Every file in the Overwrite as `/`-joined paths relative to it (recursive).
/// [`overwrite_entries`] memoised against the view generation: the Overwrite tab
/// and the mod-info file tree re-render constantly, and each render used to walk
/// the whole tree again. Rebuilds only after something changes on disk.
///
/// Handed out as an `Rc`, because the memoisation used to be half of one: the
/// walk was cached but every HIT still cloned the whole Vec - against the real
/// 4902-file Overwrite this file documents, that was ~5k String allocations and
/// ~300 KB of memcpy per redraw, per pointer event, with the tab open. A cache
/// whose hit path allocates proportionally to the data is a cache in name only.
pub(crate) fn cached_entries(app: &App, dir: &Path) -> std::rc::Rc<Vec<String>> {
    let gen = app.view_generation.get();
    if let Some((at, entries)) = app.listing_cache.borrow().get(dir) {
        if *at == gen {
            return entries.clone(); // Rc bump, not a Vec copy
        }
    }
    let entries = std::rc::Rc::new(overwrite_entries(dir));
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

/// Every file under `dir`, relative and sorted.
///
/// Capped at the same depth as its neighbours in this file, and it does not
/// follow symlinks. `restore_hidden_files` and `data_tree_rows` both already
/// guard this way - the latter saying "a symlink loop inside a mod would
/// otherwise recurse until the stack gives out" - and this walk was the one that
/// did not, in the crate that must never crash: it runs on the Mod Info dialog's
/// General tab, so merely SELECTING a mod containing `link -> ..` was enough to
/// take the whole GUI down. The install path can genuinely put a symlink into
/// `mods/`, since `overlay_dir` recreates them.
pub(crate) fn overwrite_entries(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>, depth: usize) {
        if depth > 32 {
            return;
        }
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let is_real_dir = e.file_type().map(|t| t.is_dir() && !t.is_symlink()).unwrap_or(false);
            if is_real_dir {
                walk(root, &p, out, depth + 1);
            } else if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out, 0);
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

/// The union the Data tab reads, built once per view generation.
///
/// The same `LayerStack` `eidos-launch` mounts: same layers, same order, same
/// overwrite. Building it walks every enabled mod once, which is why it is
/// cached against the view generation rather than rebuilt per directory.
pub(crate) fn data_stack(app: &App) -> Option<std::rc::Rc<eidos_core::LayerStack>> {
    let gen = app.view_generation.get();
    if let Some((at, stack)) = app.data_stack.borrow().as_ref() {
        if *at == gen {
            return Some(stack.clone());
        }
    }
    let inst = app.created.as_ref()?;
    let game = selected_game(app)?;
    // `load_order` is already highest-priority-first and already drops disabled
    // mods, separators and unmanaged rows - exactly what the mount is handed.
    let mut layers = inst.load_order();
    layers.push(game.data_path.clone());
    let stack = std::rc::Rc::new(eidos_core::LayerStack::new(layers, inst.overwrite_dir()));
    *app.data_stack.borrow_mut() = Some((gen, stack.clone()));
    Some(stack)
}

/// The entries of ONE directory of the merged view (`dir` relative to `Data`,
/// `""` for the root): each name, the source providing it, whether it is a
/// folder, and the real path behind it.
///
/// Answered by the SAME `LayerStack` the mount serves from. It used to be a
/// third, independent reimplementation of the merge, and it had drifted: it
/// showed `.eidoswh.<name>` whiteout markers as ordinary rows, and showed the
/// lower-layer files those markers DELETE as winners - so the tab claimed the
/// game would see files the mount hides.
///
/// One level at a time, so expanding a node costs one directory read per layer
/// that has it rather than a full recursive walk of every enabled mod.
pub(crate) fn merged_listing(app: &App, dir: &str) -> Vec<DataRow> {
    let Some(stack) = data_stack(app) else { return Vec::new() };
    // Where each real path came from, so a winner can be named. Built once per
    // call rather than per row: a deep tree asks this thousands of times.
    let mut sources: Vec<(PathBuf, String)> = Vec::new();
    if let Some(inst) = app.created.as_ref() {
        sources.push((inst.overwrite_dir(), "[Overwrite]".to_string()));
    }
    // Unmanaged rows are EXCLUDED. Their `path` is a single plugin file inside
    // the game's own Data directory, and a longest-prefix match against that
    // would attribute every vanilla file to whichever DLC row sorted first - and
    // put a Hide button on the pristine game install.
    for m in app.mods.iter().filter(|m| m.is_active() && !m.is_unmanaged()) {
        sources.push((m.path.clone(), m.name.clone()));
    }
    if let Some(g) = selected_game(app) {
        sources.push((g.data_path.clone(), format!("[{}]", g.def.id)));
    }
    let conflicts = app.conflicts.as_ref();

    let mut out: Vec<DataRow> = stack
        .list_dir_typed(dir)
        .into_iter()
        .map(|(name, real, ftype)| {
            // Longest match wins: a mod nested under another root would
            // otherwise be attributed to whichever prefix came first.
            let source = sources
                .iter()
                .filter(|(root, _)| real.starts_with(root))
                .max_by_key(|(root, _)| root.as_os_str().len())
                .map(|(_, label)| label.clone())
                .unwrap_or_default();
            let md = fs::symlink_metadata(&real).ok();
            let is_dir = ftype.map(|t| t.is_dir()).unwrap_or_else(|| real.is_dir());
            // The conflict map is keyed by lowercased relative path and is
            // already computed for the mod list, so this is a lookup, not a walk.
            // ASCII-only case folding, because that is how `eidos-conflicts`
            // keys the map. Unicode `to_lowercase` disagrees with it for any
            // path containing a non-ASCII capital, and the lookup would then
            // silently never match - a contested file quietly unflagged.
            let rel = if dir.is_empty() {
                name.to_ascii_lowercase()
            } else {
                format!("{}/{}", dir.to_ascii_lowercase(), name.to_ascii_lowercase())
            };
            let conflicted = !is_dir
                && conflicts.is_some_and(|c| {
                    c.files.get(&rel).is_some_and(eidos_conflicts::FileNode::is_conflicted)
                });
            DataRow {
                name,
                source,
                is_dir,
                real,
                size: md.as_ref().filter(|m| m.is_file()).map(|m| m.len()),
                mtime: md.as_ref().and_then(|m| m.modified().ok()),
                conflicted,
            }
        })
        .collect();
    // Folders first, then files, each alphabetically - the ordering every file
    // browser uses, and the one that makes a deep tree navigable.
    out.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
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
    found.sort_by_key(|f| std::cmp::Reverse(f.0));
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
    pub(crate) row: DataRow,
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
        // A filter reaches INTO folders: a name typed in the box is somewhere in
        // the tree, not necessarily on the level currently expanded, so a
        // directory is walked even when its own name does not match, and then
        // dropped if nothing under it survived.
        let query = app.data_query.trim().to_lowercase();
        let filtering = !query.is_empty() || app.data_conflicts_only;
        for row in cached_merged_listing(app, dir) {
            // Checked against the KEPT count, and re-checked after each subtree.
            // Filtering removes rows again, so `out.len()` alone stopped being a
            // bound the moment a filter was typed: the walk then stat'd the
            // entire merged tree on every redraw, inside `view()`.
            if out.len() >= limit {
                return;
            }
            let rel =
                if dir.is_empty() { row.name.clone() } else { format!("{dir}/{}", row.name) };
            let expanded = row.is_dir && (app.data_expanded.contains(&rel) || filtering);
            let keeps = !filtering
                || (row.name.to_lowercase().contains(&query)
                    && (!app.data_conflicts_only || row.conflicted));
            let at = out.len();
            let is_dir = row.is_dir;
            out.push(TreeRow { depth, rel: rel.clone(), row });
            if expanded {
                walk(app, &rel, depth + 1, limit, out);
            }
            // A folder earns its row by what is under it - unless the budget ran
            // out inside it, in which case it is KEPT: it may well contain a
            // match nobody got to look for, and dropping it would both hide that
            // and shorten the list below the budget, suppressing the "showing
            // the first N" notice that explains why.
            let budget_spent = out.len() >= limit;
            if filtering && !keeps && !budget_spent && (!is_dir || out.len() == at + 1) {
                out.remove(at);
            }
        }
    }
    let mut out = Vec::new();
    walk(app, "", 0, limit, &mut out);
    out
}

/// The menu bar. iced 0.13 has no native dropdown widget, so the top-level items
/// that carry ONE useful action fire it directly (Tools -> Executables, Run,
/// Refresh, Help -> About); File and View open small floating menus, because they
/// each host several things.
pub(crate) fn menu_bar<'a>() -> Element<'a, Message> {
    let row = Row::new()
        .spacing(0)
        .push(flat_btn("File", Message::OpenFileMenu))
        .push(flat_btn("View", Message::OpenViewMenu))
        .push(flat_btn("Tools", Message::ShowExecutablesDialog))
        // Shortcut hints inline, MO2-style (the keys are wired in `subscription`).
        .push(flat_btn("Run (Ctrl+R)", Message::Run))
        .push(flat_btn("Refresh (F5)", Message::Refresh))
        .push(flat_btn("Help", Message::ShowAbout));
    container(row).width(Length::Fill).padding(1).style(bar_style).into()
}

/// The File dropdown: every folder that matters, in one place.
///
/// Worth more on Linux than the same menu is on Windows. The paths a modder
/// actually needs - the game's INI directory, the prefix's My Games - live at
/// `steamapps/compatdata/<appid>/pfx/drive_c/users/steamuser/Documents/My Games/…`,
/// which nobody retypes and no file manager bookmarks by accident. Eidos already
/// resolves every one of them; they were just never offered.
///
/// An entry whose path cannot be resolved right now (no instance open, no Proton
/// prefix yet) is drawn inert rather than hidden, so the menu does not change
/// shape underneath the user and the absence is legible.
pub(crate) fn file_menu_card<'a>(app: &App) -> Element<'a, Message> {
    /// `owned` = a directory Eidos creates on demand (downloads before the first
    /// download, overwrite before the first run). Those stay live and are created
    /// when opened; a path outside Eidos - the game, the prefix - is only offered
    /// when it is really there.
    fn entry<'a>(label: &'a str, path: Option<PathBuf>, owned: bool) -> Element<'a, Message> {
        match path.filter(|p| owned || p.exists()) {
            Some(p) => menu_item_owned(label.to_string(), Message::OpenFolder(p)),
            None => container(text(label).size(12.0))
                .width(Length::Fill)
                .padding([4, 8])
                .style(|t: &Theme| container::Style {
                    text_color: Some(t.extended_palette().background.weak.color),
                    ..Default::default()
                })
                .into(),
        }
    }

    let inst = app.created.as_ref();
    let game = selected_game(app);
    // The profile's INIs are the ones Eidos owns; the prefix copy is what the
    // game reads. Both are worth reaching, and only one of them is guessable.
    let prefix_inis = game.and_then(|g| {
        let spec = GameSpec::for_id(g.def.id)?;
        let prefix = g.compatdata.as_ref()?.join("pfx");
        Some(eidos_plugins::documents_my_games_dir(&prefix, &spec))
    });

    let col = Column::new()
        .spacing(1)
        .push(entry("Instance folder", inst.map(|i| i.root.clone()), true))
        .push(entry("Mods", inst.map(|i| i.mods_dir()), true))
        .push(entry("Downloads", inst.map(|i| i.downloads_dir()), true))
        .push(entry("Overwrite", inst.map(|i| i.overwrite_dir()), true))
        .push(entry("Active profile", inst.map(|i| i.active().dir()), true))
        .push(menu_sep())
        .push(entry("Game install", game.map(|g| g.install_path.clone()), false))
        .push(entry("Game Data", game.map(|g| g.data_path.clone()), false))
        .push(entry("Game INIs (in the Proton prefix)", prefix_inis, false))
        .push(menu_sep())
        .push(menu_item_owned("Open a Nexus collection...".to_string(), Message::ShowCollection(String::new())))
        .push(menu_item("Instances...", Message::ShowInstanceManager))
        .push(menu_item("Export the mod list...", Message::ShowExportDialog))
        .push(menu_sep())
        .push(entry("Eidos logs", Some(eidos_log::log_dir()), true))
        .push(entry("Extensions", Some(eidos_addons::user_addons_dir()), true));
    menu_frame(col.into())
}

/// The View dropdown's contents (floats over the window via the Stack, dismissed
/// by a click outside). Hosts the toolbar/status-bar toggles + collapse/expand-all.
pub(crate) fn view_menu_card<'a>(app: &App) -> Element<'a, Message> {
    let toolbar_label = if app.ui_toolbar_visible { "Hide toolbar" } else { "Show toolbar" };
    let status_label = if app.ui_statusbar_visible { "Hide status bar" } else { "Show status bar" };
    let mut col = Column::new()
        .spacing(1)
        .push(menu_item(toolbar_label, Message::ToggleToolbar))
        .push(menu_item(status_label, Message::ToggleStatusBar))
        .push(menu_sep())
        .push(menu_item("INI editor...", Message::ShowIniEditor))
        .push(menu_item("Log...", Message::ShowLogPane))
        .push(menu_item("Extensions...", Message::ShowAddons))
        .push(menu_sep());
    // The columns, each a tick. Not a submenu: eight items is a short list, and
    // a submenu here would hide the one thing somebody opens this menu to find.
    for c in ModColumn::ALL {
        let on = app.mod_columns.contains(&c);
        col = col.push(menu_item_owned(
            format!("{} {}", if on { "\u{2713}" } else { "\u{2007}" }, c.title()),
            Message::ToggleModColumn(c),
        ));
    }
    col = col.push(menu_sep());
    for g in GroupBy::ALL {
        let on = app.group_by == Some(g);
        col = col.push(menu_item_owned(
            format!("{} {}", if on { "\u{2713}" } else { "\u{2007}" }, g.label()),
            Message::SetGroupBy((!on).then_some(g)),
        ));
    }
    if app.mod_sort.is_some() || app.group_by.is_some() {
        // Say it, and offer the way out. A sorted or grouped list refuses drags,
        // and a user who does not know why cannot fix it.
        col = col.push(menu_sep()).push(menu_item_owned(
            "Back to load order (drag needs it)".to_string(),
            if app.group_by.is_some() {
                Message::SetGroupBy(None)
            } else {
                Message::CycleModSort(SortKey::Name)
            },
        ));
    }
    let col = col
        .push(menu_sep())
        .push(menu_item("Collapse all groups", Message::CollapseAllGroups))
        .push(menu_item("Expand all groups", Message::ExpandAllGroups))
        .push(menu_sep())
        .push(set_all_item(app, true))
        .push(set_all_item(app, false));
    menu_frame(col.into())
}

/// "Enable all" / "Disable all", armed on the first click.
///
/// The label carries the COUNT and, under a filter, the word "shown" - because
/// this deliberately touches only the rows on screen, and a user who forgot a
/// filter was running would otherwise read "Disable all" as meaning everything.
/// Two clicks rather than a modal, matching the batch remove in the same menus.
fn set_all_item<'a>(app: &App, enable: bool) -> Element<'a, Message> {
    let n = mods_visible_for_bulk(app).len();
    let armed = app.confirm_set_all == Some(enable);
    let verb = if enable { "Enable" } else { "Disable" };
    // A FOLD narrows this as much as a filter does - a mod inside a collapsed
    // group is not drawn, so it is not touched - and saying so only for filters
    // left the commonest case silent. The count already tells the truth; the
    // word is what stops it being read as "everything".
    let narrowed = is_filtering(app) || n < app.mods.iter().filter(|m| !m.is_separator() && !m.is_unmanaged()).count();
    let scope = if narrowed { " shown" } else { "" };
    let label = if armed {
        format!("Confirm - {} {n}{scope} mod(s)?", verb.to_lowercase())
    } else {
        format!("{verb} all{scope} ({n})")
    };
    button(text(label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(Message::SetAllModsEnabled(enable))
        .style(if armed { button::danger } else { button::text })
        .into()
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
        .push(text_btn("Backups", Message::ShowBackupsDialog))
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
    columns: &[ModColumn],
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
    // A note rides here rather than taking a column of its own. Notes are long
    // and read on demand; a column would cost width off the NAME on every row to
    // show the first six characters of something most rows do not have.
    // The mod's Nexus page is gone. It rides here rather than in the Version
    // column because an unavailable mod will never show an update - it would
    // otherwise pass through every check invisibly and only be noticed by
    // someone going looking for the page.
    if meta.is_some_and(|r| r.nexus_gone) {
        flags = flags.push(
            tooltip(
                text("\u{2298}").size(12.0).color(CONFLICT_LOSES_FG),
                container(
                    text("Nexus no longer serves this mod's page. Keep your archive: you will not be able to download it again.")
                        .size(11.0),
                )
                .padding(6)
                .style(card_style),
                tooltip::Position::Left,
            )
            .gap(4),
        );
    }
    // MO2's two state flags. Both are advisory - the mod still loads, and Eidos
    // still deploys it - so they are a glyph with an explanation on hover rather
    // than anything that blocks. "Mark as valid" on the row menu silences either.
    if meta.is_some_and(|r| r.invalid_data) {
        flags = flags.push(
            tooltip(
                text("\u{26A0}").size(12.0).color(CONFLICT_LOSES_FG),
                container(
                    text(
                        "Nothing at the top of this mod looks like data this game loads. It \
                         may need its folders moved up a level, or it may simply not be a mod \
                         for this game. Right-click and Mark as valid to stop asking.",
                    )
                    .size(11.0),
                )
                .padding(6)
                .style(card_style),
                tooltip::Position::Left,
            )
            .gap(4),
        );
    }
    if let Some(other) = meta.and_then(|r| r.other_game.clone()) {
        flags = flags.push(
            tooltip(
                text("\u{25C6}").size(12.0).color(CONFLICT_LOSES_FG),
                container(
                    text(format!(
                        "Downloaded for {other}, not for this game. It may still work - many \
                         mods do - but nothing here checked. Right-click and Mark as valid to \
                         stop asking."
                    ))
                    .size(11.0),
                )
                .padding(6)
                .style(card_style),
                tooltip::Position::Left,
            )
            .gap(4),
        );
    }
    if let Some(note) = meta.and_then(|r| r.notes.clone()).filter(|n| !n.trim().is_empty()) {
        flags = flags.push(
            tooltip(
                text("\u{270E}").size(12.0),
                container(text(note).size(11.0)).padding(6).style(card_style),
                tooltip::Position::Left,
            )
            .gap(4),
        );
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

    // A backup contributes nothing to the game, so its checkbox does nothing -
    // a tick that deployed two copies of one mod over each other would be worse
    // than no tick at all. Drawn inert, like unmanaged content.
    let toggle: Element<'a, Message> =
        if m.is_backup() { checkbox(false).size(16).into() } else { toggle.into() };
    let mut row = Row::new()
        .spacing(6)
        .height(Length::Fixed(MOD_ROW_H))
        .align_y(iced::Alignment::Center)
        .push(container(toggle).width(C_CHECK))
        .push(text(format!("{:>2}", i + 1)).size(12.0).width(C_PRIO))
        .push(name_cell(m.name.clone(), bg));
    let mut flag_cell = Some(flag_cell);
    for col in columns {
        let w = Length::Fixed(col.width());
        row = match col {
            ModColumn::Category => row.push(
                text(if m.unmanaged { "Game content".to_string() } else { category.clone() })
                    .size(11.0)
                    .width(w),
            ),
            ModColumn::Content => row.push(text(content.clone()).size(10.0).width(w)),
            ModColumn::Version => row.push(text(version.clone()).size(11.0).width(w)),
            ModColumn::Author => row.push(
                text(meta.and_then(|r| r.author.clone()).unwrap_or_default()).size(11.0).width(w),
            ),
            ModColumn::Installed => row.push(
                text(meta.and_then(|r| r.installed_at).map(fmt_day).unwrap_or_default())
                    .size(11.0)
                    .width(w),
            ),
            ModColumn::ModId => row.push(
                text(meta.and_then(|r| r.mod_id).map(|n| n.to_string()).unwrap_or_default())
                    .size(11.0)
                    .width(w),
            ),
            ModColumn::Game => row.push(
                text(meta.and_then(|r| r.game_name.clone()).unwrap_or_default())
                    .size(11.0)
                    .width(w),
            ),
            // Taken rather than cloned: the flags cell is built once above and
            // there is exactly one of it.
            ModColumn::Flags => match flag_cell.take() {
                Some(cell) => row.push(cell),
                None => row,
            },
        };
    }

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
        // MO2's three: plain opens Information, Ctrl opens the folder, Shift
        // opens the Nexus page. A double-click still delivers both presses, so
        // this arrives AFTER a drag has been armed and dropped on itself - which
        // is a no-op move, and why it can be layered on without a mode.
        .on_double_click(Message::ModDoubleClick(i))
        .on_right_press(Message::OpenModMenu(i))
        .into()
}

/// A group header the grouping invented, as opposed to a separator the user
/// wrote. Deliberately quieter than a separator: it is a fact about the list
/// rather than something in it, and dressing it up like a separator would
/// invite the right-click that a separator answers and this cannot.
fn group_header_row<'a>(label: String, count: usize, folded: bool) -> Element<'a, Message> {
    let msg = Message::ToggleGroupFold(label.clone());
    let row = Row::new()
        .spacing(6)
        .height(Length::Fixed(MOD_ROW_H))
        .align_y(iced::Alignment::Center)
        .push(text(if folded { "[+]" } else { "[-]" }).size(11.0).width(C_CHECK))
        .push(text(label).size(12.0).width(Length::Fill))
        .push(text(format!("{count}")).size(11.0).width(C_PRIO));
    mouse_area(container(row).padding([0, 4]).style(|_: &Theme| container::Style {
        background: Some(Background::Color(GROUP_HEADER_BG)),
        ..Default::default()
    }))
    .on_press(msg)
    .into()
}

/// A date with no time: an install date is read as "which week was this", and a
/// clock on every row is width spent on a precision nobody wants.
fn fmt_day(t: std::time::SystemTime) -> String {
    let secs = t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (y, m, d, ..) = eidos_log::civil_from_unix(secs);
    format!("{y:04}-{m:02}-{d:02}")
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
    let active = app.mods.iter().filter(|m| m.is_active()).count();
    let (names, active_name) = cached_profiles(app);
    let mut profile = Row::new().spacing(6).push(text("Profile:").size(12.0));
    if app.created.is_some() {
        for name in names {
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
                "Active: {active}  |  Endorsed: {}  |  Updates: {}{}",
                app.endorsed_count,
                app.updated_count,
                nexus_budget_suffix(app)
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
                .id(filter_input_id())
                .on_input(Message::SearchChanged)
                .padding(5)
                .size(12.0),
        )
        .push(
            pick_list(choices, selected, |c: CategoryChoice| Message::CategoryFilterChanged(c.id))
                .text_size(12.0)
                .padding(5),
        )
        .push(filter_button(app))
        .push(tool_btn("+ Separator", Message::AddSeparator(0)))
        .push(tool_btn("+ Empty mod", Message::CreateEmptyMod))
        .push(tool_btn("Install folder", Message::InstallFromFolder));

    // A heading that can be clicked to sort by it. The arrow says which way, and
    // its absence says "load order", which is the state that matters most: it is
    // the only one where dragging works.
    let head = |label: &str, key: Option<SortKey>, width: Length| -> Element<'a, Message> {
        let Some(key) = key else {
            return text(label.to_string()).size(11.0).width(width).into();
        };
        let arrow = match app.mod_sort {
            Some(s) if s.by == key && s.ascending => " \u{25B2}",
            Some(s) if s.by == key => " \u{25BC}",
            _ => "",
        };
        button(text(format!("{label}{arrow}")).size(11.0))
            .padding(0)
            .width(width)
            .style(button::text)
            .on_press(Message::CycleModSort(key))
            .into()
    };
    let mut header = Row::new()
        .spacing(6)
        .push(text("").width(C_CHECK))
        .push(head("#", None, C_PRIO))
        .push(head("Mod Name", Some(SortKey::Name), Length::Fill));
    for col in &app.mod_columns {
        // Flags is a row of glyphs with no order anybody would want; every other
        // column sorts.
        let key = (*col != ModColumn::Flags).then_some(SortKey::Column(*col));
        header = header.push(head(col.title(), key, Length::Fixed(col.width())));
    }

    let query = app.search.trim().to_lowercase();
    // No spacing: the insertion strips below provide the separation, and they
    // must be part of the flow so the layout is identical with and without a drag.
    let mut list = Column::new();
    // One entry per row actually drawn, feeding the conflict strip beside the
    // scrollbar. Filled in the same order the rows are pushed.
    let mut tints: Vec<Option<Color>> = Vec::new();
    let mut shown = 0usize;
    if app.mods.is_empty() {
        list = list.push(text("No mods yet. Drop mod folders into the instance's mods/ dir.").size(12.0));
    }
    // Decided up front, because whether a separator draws depends on whether any
    // mod BELOW it survives the filter - which the single downward pass this used
    // to be could not know when it reached the header.
    let filtering = is_filtering(app);
    let vis = mod_row_visibility(app, cats);
    // The live drag's insertion point, if any, so exactly one gap draws the line.
    // Drawn iff releasing HERE would move the block - the SAME predicate the
    // DragDrop commit uses, so the line and the drop cannot disagree. The old
    // filter hid the line on the grabbed row's own edges unconditionally, while
    // the commit only treats those gaps as a no-op for a SINGLE row: a
    // multi-row drag released there moved the block with no line on screen.
    // (`aimed` inside the predicate is what keeps a plain click from flashing.)
    let live_gap = app
        .drag_state
        .filter(|d| !mod_drop_is_noop(app, d))
        .map(|d| d.gap)
        // A download being dropped in has no row in the list yet, so it has no
        // "own edge" to suppress: every aimed gap really would install there.
        .or_else(|| app.download_drag.as_ref().filter(|d| d.aimed).map(|d| d.gap));
    let dragging = app.drag_state.is_some() || app.download_drag.is_some();
    // Which drag the strips answer to. Only one can be live - a press on a
    // download row cancels a mod drag through the same release ladder - so this
    // is a choice, not a merge.
    let (over_gap, drop_msg): (fn(usize) -> Message, Message) = if app.download_drag.is_some() {
        (Message::DownloadDragOverGap, Message::DownloadDragDrop)
    } else {
        (Message::DragOverGap, Message::DragDrop)
    };
    // Load order is `0..len`, so the common path is exactly what it was. Any
    // other order disables the insertion strips: a drop in a sorted list has no
    // meaning to give the row it lands on, which is why MO2 disables it too.
    let entries = display_entries(app);
    let reorderable = app.mod_sort.is_none() && app.group_by.is_none();
    let dragging = dragging && reorderable;
    for entry in &entries {
        // A header the grouping invented. It heads rows that are not adjacent in
        // the real list, so it has none of a separator's actions - there is
        // nothing in `mods` behind it to rename, colour, move or remove.
        let i = match entry {
            ListEntry::Group(label, n) => {
                let folded = app.groups_collapsed.contains(label);
                list = list.push(group_header_row(label.clone(), *n, folded));
                tints.push(None);
                continue;
            }
            ListEntry::Row(i) => *i,
        };
        let m = &app.mods[i];
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
            if reorderable {
                list = list
                    .push(drop_gap(i, live_gap == Some(i), dragging, over_gap, drop_msg.clone()));
            }
            list = list.push(separator_row(i, m, color, collapsed, selected));
            tints.push(None); // a separator has no conflict of its own
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
        if reorderable {
            list =
                list.push(drop_gap(i, live_gap == Some(i), dragging, over_gap, drop_msg.clone()));
        }
        // Computed once and handed to both: the row paints this colour, and the
        // name cell fades into it.
        let conflict = conflict_tint(app, i);
        tints.push(conflict);
        // The mod's own colour, washed down to a row background. MO2 colours any
        // mod through the Notes column; Eidos already stored the colour per mod
        // and only ever offered the picker on separators.
        let tint = meta.and_then(|r| r.color).map(|rgb| mod_tint(rgb, i % 2 == 0));
        let bg = row_background(i % 2 == 0, selected, conflict, tint);
        list = list.push(list_row(
            mod_row(i, m, meta, flag_icon, hidden_icon, bg, &app.mod_columns),
            i % 2 == 0,
            selected,
            conflict,
            tint,
        ));
    }
    // The trailing strip: the only way to aim at the end of the list, since
    // hovering a row always means "above it". Drawn even for an EMPTY list, or a
    // download dragged into a fresh instance could never become aimed and the
    // release would install nothing, silently - the one case where there is
    // nothing else on screen to aim at.
    let end = app.mods.len();
    if reorderable {
        list =
            list.push(drop_gap(end, live_gap == Some(end), dragging, over_gap, drop_msg.clone()));
    }
    // `shown` counts mods only, so this cannot fire on a list that is all folded
    // groups - and it only speaks when something was actually asked.
    if !app.mods.is_empty() && shown == 0 && filtering {
        // Say which of the three narrowings is responsible, or the user is left
        // staring at an empty list with the guilty control off screen.
        let mut by: Vec<String> = Vec::new();
        if !query.is_empty() {
            by.push(format!("named \"{}\"", app.search.trim()));
        }
        if app.category_filter.is_some() {
            by.push("in this category".to_string());
        }
        if app.filters.any() {
            by.push(format!("matching the {} active filter(s)", app.filters.active_count()));
        }
        list = list.push(text(format!("No mods {}.", by.join(", "))).size(12.0));
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

    // `on_release` is the catch-all: a row or a strip that handles the release
    // captures it and this never fires, but a release landing anywhere else in
    // the list - a header, a gap the layout moved, empty space below the last
    // row - disarms instead of leaving a drag live for the next click to commit.
    //
    // A release OUTSIDE the list is caught globally instead of by `on_exit`, and
    // that is the whole difference: `mouse_area` cannot tell "left while
    // dragging" from "let go out there", so cancelling on exit meant dragging
    // upward past the header dropped the mod every time.
    // No release handler here. Every release is decided in ONE place -
    // `Message::PointerReleased`, from the global listener - which drops at the
    // aimed gap or disarms if none was aimed.
    //
    // There used to be an `on_release(DragCancel)` as a catch-all, from before
    // that listener existed. With both, a release that did not land exactly on a
    // drop strip raced: this one cancelled the drag before the global one could
    // commit it. Releasing over a row lost the drop, and once the auto-scroll
    // bands existed - which are never drop strips - dragging any distance made
    // it certain.
    let list_area = mouse_area(
        scrollable(list).id(mod_scroll_id()).width(Length::Fill).height(Length::Fill),
    );
    // The conflict marks go ON the scrollbar, at the same fraction of the list,
    // so the mod a tint refers to can be found without reading every row on the
    // way. Stacked rather than placed beside it: a strip in the flow pushed the
    // whole list sideways to make room, which is a lot of shifted UI for a hint.
    // Nothing in the strip handles events, so the scrollbar keeps the pointer.
    let mut layers = Stack::new().push(list_area).push(
        Row::new().push(Space::new().width(Length::Fill)).push(scroll_marks(&tints)),
    );
    // Auto-scroll bands, and only once the drag is REALLY under way. `DragStart`
    // fires on press, so keying these off "a drag exists" put them under the
    // pointer on every click - and `mouse_area` publishes `on_enter` the first
    // time it is laid out beneath a stationary cursor. `aimed` means the pointer
    // has crossed an insertion point, which no plain click does.
    if app.drag_state.is_some_and(|d| d.aimed)
        || app.download_drag.as_ref().is_some_and(|d| d.aimed)
    {
        // `on_move` gives the pointer's position INSIDE the band, so depth is
        // just its height normalised - 1.0 hard against the edge of the list.
        let band = |edge: ScrollEdge| {
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_enter(Message::DragScrollEdge(Some(edge)))
                .on_move(move |p| {
                    let t = (p.y / DRAG_SCROLL_BAND).clamp(0.0, 1.0);
                    Message::DragScrollDepth(match edge {
                        ScrollEdge::Up => 1.0 - t,
                        ScrollEdge::Down => t,
                    })
                })
                .on_exit(Message::DragScrollEdge(None))
        };
        layers = layers.push(
            Column::new()
                .push(container(band(ScrollEdge::Up)).height(Length::Fixed(DRAG_SCROLL_BAND)))
                .push(Space::new().height(Length::Fill))
                .push(container(band(ScrollEdge::Down)).height(Length::Fixed(DRAG_SCROLL_BAND))),
        );
    }
    let list_area = layers;

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

/// MO2's right-click plugin menu: jump to the mod that ships this plugin, send
/// the selection to either end of the load order, or set every plugin at once.
///
/// "Which mod does this ESP come from" is asked dozens of times while debugging
/// a load order, and until now the only answer was the row's hover tooltip.
pub(crate) fn plugin_menu_card<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(list) = app.plugins.as_ref() else {
        return Space::new().width(Length::Shrink).height(Length::Shrink).into();
    };
    let Some(p) = list.plugins.get(i) else {
        return Space::new().width(Length::Shrink).height(Length::Shrink).into();
    };
    let picked = app.selected_plugins.len().max(1);
    let mut col = Column::new()
        .spacing(1)
        .push(text(p.name.clone()).size(13.0))
        .push(menu_sep());

    // The origin actions are only offered when there IS an origin: vanilla
    // content belongs to no mod, and a greyed row that never explains itself is
    // worse than no row.
    match plugin_origin_row(app, i) {
        Some(row) => {
            let name = app.mods.get(row).map(|m| m.display_name().to_string()).unwrap_or_default();
            col = col
                .push(menu_item_owned(format!("Open mod folder  ({name})"), Message::OpenPluginOrigin(i)))
                .push(menu_item("Mod info", Message::ShowPluginOriginInfo(i)));
        }
        None => {
            col = col.push(container(text("From the game's own Data").size(11.0)).padding([4, 8]));
        }
    }

    col = col
        .push(menu_sep())
        .push(menu_item_owned(
            if picked > 1 { format!("Send {picked} to top") } else { "Send to top".to_string() },
            Message::PluginsSendTop,
        ))
        .push(menu_item_owned(
            if picked > 1 {
                format!("Send {picked} to bottom")
            } else {
                "Send to bottom".to_string()
            },
            Message::PluginsSendBottom,
        ))
        .push(send_to_plugin_priority(app, i))
        .push(menu_sep())
        .push(menu_item("Activate all", Message::PluginsSetAll(true)))
        .push(menu_item("Deactivate all", Message::PluginsSetAll(false)));

    container(col).width(Length::Fixed(240.0)).padding(6).style(card_style).into()
}

/// The profile names and the active one, memoised against the view generation.
///
/// Read straight from disk this cost a `read_dir` plus a stat per profile and a
/// file read for the active name, on every frame the main screen was drawn -
/// which is every pointer move. Every path that creates, renames, deletes or
/// switches a profile bumps the view generation, so the memo cannot go stale.
pub(crate) fn cached_profiles(app: &App) -> (Vec<String>, String) {
    let gen = app.view_generation.get();
    if let Some((at, names, active)) = app.profiles_cache.borrow().as_ref() {
        if *at == gen {
            return (names.clone(), active.clone());
        }
    }
    let (names, active) = match app.created.as_ref() {
        Some(inst) => (inst.profiles(), inst.active_profile()),
        None => (Vec::new(), String::new()),
    };
    *app.profiles_cache.borrow_mut() = Some((gen, names.clone(), active.clone()));
    (names, active)
}

/// The host of a URL, for a menu label: `https://www.loverslab.com/x` ->
/// `loverslab.com`. Falls back to the whole string when it does not parse,
/// which is better than an entry reading "Visit ".
pub(crate) fn url_host(url: &str) -> String {
    url.split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
        .trim_start_matches("www.")
        .to_string()
}

/// What is left of the Nexus request budget, appended to the counters line.
///
/// Empty until something has actually asked Nexus a question: an invented "1000
/// left" before the first call would be a guess, and the whole value of the
/// number is that it is the one the server last reported.
///
/// The hourly figure is the one shown, because it is the one that runs out - the
/// daily budget is large enough that it is only interesting once the hourly one
/// has stopped mattering, so it appears only when it is the smaller of the two.
pub(crate) fn nexus_budget_suffix(app: &App) -> String {
    match (app.nexus_hourly_left, app.nexus_daily_left) {
        (None, None) => String::new(),
        (h, d) => {
            let n = match (h, d) {
                (Some(h), Some(d)) => h.min(d),
                (Some(h), None) => h,
                (None, Some(d)) => d,
                (None, None) => return String::new(),
            };
            format!("  |  Nexus: {n} req. left")
        }
    }
}

/// MO2's "Send to priority..." for plugins: an inline field that takes a load
/// index, opened by the menu row above it.
///
/// The field replaces the row rather than opening a dialog, exactly as the mod
/// list's does - a modal for one number is a lot of ceremony, and the menu is
/// already floating where the user is looking.
fn send_to_plugin_priority<'a>(app: &App, i: usize) -> Element<'a, Message> {
    match app.plugin_send_priority.as_ref().filter(|(row, _)| *row == i) {
        Some((_, typed)) => text_input("Row number (1 = first)", typed)
            .on_input(Message::PluginSendToPriorityChanged)
            .on_submit(Message::PluginSendToPriorityCommit)
            .padding(5)
            .size(12.0)
            .into(),
        None => menu_item("Send to priority...", Message::PluginSendToPriorityStart),
    }
}

/// The Filters button, badged with how many criteria are currently narrowing
/// the list - so a list that looks short always says why.
pub(crate) fn filter_button<'a>(app: &App) -> Element<'a, Message> {
    let n = app.filters.active_count();
    let label = if n > 0 { format!("Filters ({n})") } else { "Filters".to_string() };
    button(text(label).size(12.0))
        .padding(5)
        .on_press(Message::ToggleFilterPane)
        .style(if n > 0 { button::primary } else { button::text })
        .into()
}

/// MO2's filter pane: one row per criterion, each cycling off -> only -> except.
///
/// Three settings rather than a checkbox because "only conflicted mods" and
/// "everything except conflicted mods" are both real questions, and every
/// criterion here is read from data the list already computes.
pub(crate) fn filter_pane<'a>(app: &App) -> Element<'a, Message> {
    let mut col = Column::new().spacing(2).push(
        Row::new()
            .align_y(iced::Alignment::Center)
            .push(text("Show mods that are...").size(11.0).width(Length::Fill))
            .push(
                button(text("Clear").size(11.0))
                    .padding([2, 8])
                    .style(button::text)
                    .on_press(Message::ClearFilters),
            ),
    );
    for (label, state, field) in app.filters.rows() {
        col = col.push(
            button(
                Row::new()
                    .spacing(6)
                    .push(text(state.mark()).size(11.0).font(iced::Font::MONOSPACE))
                    .push(text(label).size(12.0).width(Length::Fill))
                    .push(
                        text(match state {
                            Criterion::Off => "",
                            Criterion::Require => "only",
                            Criterion::Exclude => "except",
                        })
                        .size(10.0),
                    ),
            )
            .width(Length::Fill)
            .padding([3, 6])
            .style(button::text)
            .on_press(Message::CycleFilter(field)),
        );
    }
    // Wrapped in a mouse_area that SWALLOWS the press. iced's Stack dispatches
    // top-down and stops only when a widget captures the event; a bare container
    // never captures, so every click landing on the card's own padding, on the
    // header text, or in the gap between two rows fell straight through to the
    // full-window catcher behind it and closed the pane. `on_right_press` is set
    // for the same reason: mouse_area captures a right press only when it has a
    // handler, so without it a right-click reached the mod list THROUGH the open
    // pane and opened a context menu underneath it.
    mouse_area(container(col).width(Length::Fixed(260.0)).padding(6).style(card_style))
        .on_press(Message::Noop)
        .on_right_press(Message::Noop)
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
            .push(menu_item_owned(
                "Collapse others".to_string(),
                Message::CollapseOthers(m.display_name().to_string()),
            ))
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
            .push(menu_sep())
            // On a separator, "inside" is the only placement that reads
            // naturally, and it means the END of the group it heads - the same
            // range the fold machinery uses.
            .push(menu_item("Install mod inside", Message::InstallAt(group_children(&app.mods, i).end)))
            .push(menu_item("New empty mod inside", Message::CreateEmptyModAt(group_children(&app.mods, i).end)))
            .push(menu_item("Add separator above", Message::AddSeparator(i)))
            .push(menu_sep())
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
    // Every page this mod has, not just the Nexus one. A mod can have both - a
    // Nexus listing and a GitHub the author actually updates - and a single
    // entry could only ever offer one of them.
    if let Some(url) = meta.as_ref().and_then(|mm| mm.url()) {
        col = col.push(menu_item_owned(
            format!("Visit {}", url_host(&url)),
            Message::OpenUrl(url),
        ));
    }
    // The person who published it, not just the page. Offered only when Nexus
    // actually gave a profile URL - it is a real account there, unlike the
    // free-text Author field, which is why nothing links to that one.
    if let Some((who, url)) = meta
        .as_ref()
        .and_then(|mm| mm.uploader().zip(mm.uploader_url()))
    {
        col = col.push(menu_item_owned(format!("Visit {who}'s profile"), Message::OpenUrl(url)));
    }
    if has_nexus {
        col = col.push(menu_item("Visit on Nexus", Message::ModVisitNexus(i)));
        let endorsed = meta.as_ref().is_some_and(|mm| mm.endorsed());
        let endorse_label = if endorsed { "Abstain (un-endorse)" } else { "Endorse" };
        col = col.push(menu_item(endorse_label, Message::ModEndorse(i)));
        let tracked = meta.as_ref().is_some_and(|mm| mm.tracked());
        let track_label = if tracked { "Untrack" } else { "Track" };
        col = col.push(menu_item(track_label, Message::ModTrack(i)));
    }
    // A backup's menu is short: it is not a mod, so enabling, reinstalling,
    // endorsing and the rest have nothing to act on. Restoring is what it is for.
    if m.is_backup() {
        let armed = app.confirm_restore.as_deref() == Some(m.name.as_str());
        col = col.push(menu_item_owned(
            if armed {
                "Click again: this replaces the mod".to_string()
            } else {
                "Restore this backup over the mod".to_string()
            },
            if armed {
                Message::ConfirmModRestoreBackup(m.name.clone())
            } else {
                Message::ModRestoreBackup(i)
            },
        ));
    } else if !m.is_separator() && !m.unmanaged {
        col = col.push(menu_item("Back up this mod", Message::ModBackup(i)));
    }
    // Offered only when there is actually a warning to silence, so the menu does
    // not carry a line whose effect nobody can see.
    let flagged = app
        .meta_cache
        .get(&m.name)
        .is_some_and(|r| r.invalid_data || r.other_game.is_some());
    if flagged {
        col = col.push(menu_item("Mark as valid", Message::ModMarkValid(i)));
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
        .push(menu_item("Categories...", Message::ShowCategoriesDialog(i)))
        .push(separator_swatches(i, app.meta_cache.get(&m.name).and_then(|r| r.color)))
        .push(menu_sep())
        // The gap is an INSERTION index: `i` is above this row, `i + 1` below.
        .push(menu_item("Install mod above", Message::InstallAt(i)))
        .push(menu_item("Install mod below", Message::InstallAt(i + 1)))
        .push(menu_item("New empty mod above", Message::CreateEmptyModAt(i)))
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
        // Anchored on the first REAL target: `real_selection` drops separators
        // and unmanaged rows, so a selection of nothing but those is empty here
        // and indexing it would panic.
        .push(match targets.first() {
            Some(&first) => menu_item("Categories...", Message::ShowCategoriesDialog(first)),
            None => Space::new().width(Length::Shrink).height(Length::Shrink).into(),
        })
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

/// A small palette of colour swatches for a mod or a separator (iced has no
/// native colour dialog), plus an "x" to clear back to the default.
///
/// On a separator the colour paints the whole header bar; on an ordinary mod it
/// is washed down into the row background - see `mod_tint`.
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
