//! The Plugins tab: building the list, LOOT data, writing the load order and
//! the panel.

use super::*;

/// Compute the ESP/ESM load order for the Plugins tab: discover from the selected
/// game's Data plus the enabled mods, preserve any existing prefix order, and
/// validate. `None` if there is no game with a plugin system.
pub(crate) fn compute_plugins(app: &App) -> Option<PluginList> {
    let game = selected_game(app)?;
    let spec = game.plugin_spec()?;
    let mut sources: Vec<(String, PathBuf)> = vec![(String::new(), game.data_path.clone())];
    // app.mods is MO2 display order (lowest priority first) = the ascending order
    // plugin discovery wants, so feed it through as-is. Unmanaged rows (DLC and
    // Creation Club) are not layers: their `path` is one `.esm` FILE inside Data,
    // which is already the first source, and the mount drops them the same way.
    let enabled = app.mods.iter().filter(|m| m.is_active() && !m.is_unmanaged());
    sources.extend(enabled.map(|m| (m.name.clone(), m.path.clone())));
    // The Overwrite layer is a plugin source too (a cleaned/generated .esp lands
    // there) - the launch path includes it, so the GUI must agree.
    if let Some(inst) = app.created.as_ref() {
        sources.push(("overwrite".to_string(), inst.overwrite_dir()));
    }

    let mut list = PluginList::discover(&sources, &spec);
    if let Some(inst) = app.created.as_ref() {
        // Plugins only live at the root, so ONE merged root listing answers for
        // all of them: a read of each layer's top directory, whiteouts and hidden
        // names applied. This runs on every mod toggle (via `diagnostics`), so the
        // stack is unindexed - an index walks every file of every enabled mod
        // first, hundreds of milliseconds of frozen window on a large list.
        let stack = eidos_core::LayerStack::new_unindexed(
            sources
                .iter()
                .rev()
                .skip(1)
                .map(|(_, path)| path.clone())
                .collect(),
            inst.overwrite_dir(),
        );
        let root: HashMap<String, PathBuf> = stack
            .list_dir("")
            .into_iter()
            .map(|(name, path)| (name.to_ascii_lowercase(), path))
            .collect();
        list.plugins.retain(|plugin| {
            root.get(&plugin.name.to_ascii_lowercase())
                .is_some_and(|path| path.is_file())
        });
    }

    // Recorded BEFORE the state is read, for the same reason as `reload_mods`:
    // a write in between may refuse the next edit, never let a stale list over.
    if let Some(inst) = app.created.as_ref() {
        app.plugins_seen
            .set(Some(plugin_state_fingerprint(&inst.active(), &spec)));
    }
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
            if let Some(dir) = game.plugin_state_dir() {
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
    let written = app
        .plugins
        .as_ref()
        .map(|list| write_plugin_state(app, list, spec))
        .transpose();
    if let Err(e) = written {
        app.status = Some(format!("Could not write the load order: {e}"));
        app.plugins = compute_plugins(app);
    }
}

/// A hash of the profile's own plugin state - the files `compute_plugins` reads
/// and `write_plugin_state` rewrites. Not the prefix shadow: other tools write
/// that, and it is never read while the profile owns a state.
fn plugin_state_fingerprint(prof: &eidos_instance::Profile, spec: &GameSpec) -> u64 {
    let dir = prof.plugins_state_dir();
    let read = |path: Option<PathBuf>| path.and_then(|p| std::fs::read(p).ok());
    content_hash(&[
        read(eidos_plugins::newest_variant(&dir, spec.active_file())),
        read(eidos_plugins::newest_variant(&dir, "loadorder.txt")),
        read(Some(prof.locked_order_path())),
    ])
}

pub(crate) fn write_plugin_state(
    app: &App,
    list: &PluginList,
    spec: &GameSpec,
) -> std::io::Result<()> {
    let selected_spec = selected_game(app).and_then(|game| game.plugin_spec());
    let spec = selected_spec.as_ref().unwrap_or(spec);
    // Cross-process lock: a running session owns these files (the plugins dir is
    // bind-mounted into it); a mid-game reorder must refuse, not corrupt.
    let _lock = app
        .created
        .as_ref()
        .map(|inst| inst.try_lock("the Eidos window"))
        .transpose()?;
    if let Some(inst) = app.created.as_ref() {
        let prof = inst.active();
        // `list` was read when the tab was opened, perhaps long ago. Since then
        // `eidos sort`, `eidos collection` or a session another window started
        // may have rewritten the profile's order, and writing `list` over it
        // would revert that silently - then snapshot the revert, so not even the
        // damage card could tell. Both callers re-read the list on Err. Checked
        // before the seed below, which would itself change the files.
        let unchanged = app
            .plugins_seen
            .get()
            .is_none_or(|seen| seen == plugin_state_fingerprint(&prof, spec));
        if !unchanged {
            return Err(std::io::Error::other(
                "another Eidos process changed the load order; it has been reloaded - redo the change",
            ));
        }
        // Adopt the game's state first, as launch and sort do, failing closed.
        // Writing into an unseeded profile built Morrowind.ini from nothing: a
        // `[Game Files]`-only stub that every later launch deployed in place of
        // the real INI, since the seed never replaces a file the profile owns.
        if let Some(dir) = selected_game(app).and_then(|g| g.plugin_state_dir()) {
            prof.seed_plugin_state(&dir, spec)?;
        }
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
        app.plugins_seen
            .set(Some(plugin_state_fingerprint(&prof, spec)));
    }
    if spec.mechanism != eidos_plugins::LoadOrderMechanism::Timestamp {
        if let Some(dir) = selected_game(app).and_then(|g| g.plugin_state_dir()) {
            list.write_load_order(&dir, spec)?;
        }
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
    let loot_ok = selected_game(app)
        .map(|g| eidos_loot::is_supported(g.def.id))
        .unwrap_or(false);
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
        let mut b = button(text(label).size(11.0))
            .padding([3, 8])
            .style(button::secondary);
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
            text(format!(
                "! {} missing master(s) - the game would crash",
                missing.len()
            ))
            .size(12.0),
        );
    }

    // A pin the engine had to overrule. Silence here would leave the user
    // believing a slot is held when it is not, so it is said out loud.
    let violated = list.violated_locks();
    if !violated.is_empty() {
        let names: Vec<&str> = violated
            .iter()
            .map(|(n, _, _)| n.as_str())
            .take(3)
            .collect();
        let more = violated.len().saturating_sub(names.len());
        let tail = if more > 0 {
            format!(" (+{more} more)")
        } else {
            String::new()
        };
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
                (Some(a), Some(b)) => {
                    format!("Can move between {a} and {b} - both are master ties.")
                }
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
            .map(|s| {
                s.primary_plugins
                    .iter()
                    .any(|pp| pp.eq_ignore_ascii_case(&p.name))
            })
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
            checkbox(p.enabled)
                .on_toggle(move |_| Message::TogglePlugin(i))
                .size(15)
                .into()
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
        let bundle = app
            .loot_meta
            .as_ref()
            .and_then(|m| m.get(&p.name.to_ascii_lowercase()));
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
                    background: Some(Background::Color(
                        t.extended_palette().background.weak.color,
                    )),
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
            let bg = if selected { sel_bg() } else { origin_bg() };
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
        marks.push(from_selected_mod.then_some(origin_bg()));
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
        Row::new()
            .push(Space::new().width(Length::Fill))
            .push(scroll_marks(&marks)),
    );

    Column::new()
        .spacing(6)
        .push(head)
        .push(header)
        .push(list_area)
        .into()
}
