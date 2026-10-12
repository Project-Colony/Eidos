//! The Saves tab: the list, the details of one save and loading both.

use super::*;

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
        .push(
            button(text("Open folder").size(11.0))
                .padding(4)
                .on_press(Message::OpenFolder(dir.clone())),
        )
        .push(
            button(text("Refresh").size(11.0))
                .padding(4)
                .on_press(Message::RefreshSaves),
        );

    let col_header = Row::new()
        .spacing(8)
        .push(text("Name").size(11.0).width(Length::Fill))
        .push(text("Date").size(11.0).width(Length::Fixed(130.0)))
        .push(text("Size").size(11.0).width(Length::Fixed(80.0)))
        .push(Space::new().width(Length::Fixed(80.0)));

    let mut rows = Column::new().spacing(2);
    if app.saves.is_empty() {
        rows = rows.push(
            text("(no saves yet) Saves your game writes for this profile appear here.").size(12.0),
        );
    }
    for (i, save) in app.saves.iter().take(SAVES_LIST_CAP).enumerate() {
        let armed = app.confirm_delete_save == Some(i);
        let del = button(text(if armed { "Confirm?" } else { "Delete" }).size(11.0))
            .padding(4)
            .on_press(if armed {
                Message::ConfirmDeleteSave(i)
            } else {
                Message::DeleteSave(i)
            })
            .style(if armed {
                button::danger
            } else {
                button::secondary
            });
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
        let tick = button(
            text(if picked { "[x]" } else { "[ ]" })
                .size(11.0)
                .font(iced::Font::MONOSPACE),
        )
        .padding([0, 4])
        .on_press(Message::SaveToggleSelect(i))
        .style(button::text);
        let row = Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .push(tick)
            .push(name)
            .push(
                text(format_mtime(save.mtime))
                    .size(11.0)
                    .width(Length::Fixed(130.0)),
            )
            .push(
                text(format_size(save.size))
                    .size(11.0)
                    .width(Length::Fixed(80.0)),
            )
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
            .style(if app.confirm_saves_delete {
                button::danger
            } else {
                button::secondary
            }),
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
    let list = list
        .push(col_header)
        .push(scrollable(rows).height(Length::Fill));

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
pub(crate) fn save_details<'a>(
    app: &App,
    save: &eidos_instance::SaveEntry,
) -> Element<'a, Message> {
    let mut col = Column::new()
        .spacing(4)
        .push(text(save.filename.clone()).size(13.0))
        .push(
            button(text("Extension details").size(11.0)).on_press(Message::RunFileExtension(
                eidos_addons::protocol::Operation::SaveInfo,
                save.path.clone(),
            )),
        );

    let info = match app.save_info.as_ref().filter(|(p, _)| *p == save.path) {
        Some((_, Ok(info))) => info,
        Some((_, Err(e))) => {
            return col
                .push(text(format!("Cannot read this save: {e}")).size(11.0))
                .push(
                    text("The list below is unavailable; the file itself is untouched.").size(10.0),
                )
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
        (
            "Character",
            format!("{} (level {})", info.player_name, info.level),
        ),
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
    facts.push((
        "Plugins",
        format!(
            "{} + {} light",
            info.plugins.len(),
            info.light_plugins.len()
        ),
    ));
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
        col = col
            .push(text(format!("{} ({what})", m.name)).size(11.0))
            .push(text(who).size(10.0));
    }
    // Only offer the fix when something on disk can actually supply the plugins;
    // otherwise the button would enable nothing and look broken.
    let fixable = missing.iter().any(|m| !m.providers.is_empty());
    if fixable {
        col = col
            .push(Space::new().height(Length::Fixed(4.0)))
            .push(tool_btn(
                "Enable the mods this save needs",
                Message::FixSaveMods,
            ));
    }
    if info.truncated {
        col = col.push(
            text("The save's plugin list was truncated, so treat this as advisory.").size(10.0),
        );
    }
    col.into()
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
            eidos_gamefeatures::read_screenshot_with(&path, info)
                .ok()
                .map(|shot| {
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
                .map(|m| eidos_gamefeatures::ModFolder {
                    name: &m.name,
                    path: &m.path,
                })
                .collect();
            if let Some(o) = overwrite.as_deref() {
                mods.push(eidos_gamefeatures::ModFolder {
                    name: "Overwrite",
                    path: o,
                });
            }
            let data = selected_game(app).map(|g| g.data_path.clone());
            if let Some(d) = data.as_deref() {
                mods.push(eidos_gamefeatures::ModFolder {
                    name: "(game data)",
                    path: d,
                });
            }
            eidos_gamefeatures::missing_plugins(info, &known, &mods, data.as_deref())
        }
        _ => Vec::new(),
    };
    app.save_info = Some((path, parsed));
}
