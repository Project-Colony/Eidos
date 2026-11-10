//! The Saves tab: the list, the details of one save and loading both.

use super::*;

/// The Saves tab: the active profile's save files (name / date / size) plus
/// open-folder + per-save delete. MO2's savegame list.
pub(crate) fn saves_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text(t("panel_no_instance")).into();
    };
    let dir = inst.active().saves_dir();

    let header = Row::new()
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .push(text(t("saves_heading")).size(sz(13)))
        .push(Space::new().width(Length::Fill))
        .push(
            button(text(t("common_open_folder")).size(sz(11)))
                .padding(4)
                .on_press(Message::OpenFolder(dir.clone())),
        )
        .push(
            button(text(t("toolbar_refresh")).size(sz(11)))
                .padding(4)
                .on_press(Message::RefreshSaves),
        );

    let col_header = Row::new()
        .spacing(8)
        .push(text(t("info_name")).size(sz(11)).width(Length::Fill))
        .push(
            text(t("saves_col_date"))
                .size(sz(11))
                .width(Length::Fixed(scaled(130.0))),
        )
        .push(
            text(t("col_size"))
                .size(sz(11))
                .width(Length::Fixed(scaled(80.0))),
        )
        .push(Space::new().width(Length::Fixed(scaled(80.0))));

    let mut rows = Column::new().spacing(2);
    if app.saves.is_empty() {
        rows = rows.push(text(t("saves_none_yet")).size(sz(12)));
    }
    for (i, save) in app.saves.iter().take(SAVES_LIST_CAP).enumerate() {
        let armed = app.confirm_delete_save == Some(i);
        let del = button(
            text(if armed {
                t("common_confirm")
            } else {
                t("common_delete")
            })
            .size(sz(11)),
        )
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
        let name = button(text(save.filename.clone()).size(sz(12)))
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
                .size(sz(11))
                .font(colony_ui::ui_font()),
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
                    .size(sz(11))
                    .width(Length::Fixed(scaled(130.0))),
            )
            .push(
                text(format_size(save.size))
                    .size(sz(11))
                    .width(Length::Fixed(scaled(80.0))),
            )
            .push(container(del).width(Length::Fixed(scaled(80.0))));
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
            .push(text(tf("plugins_n_selected", &[("n", &n)])).size(sz(11)));
        // Copy to another profile - MO2's Transfer Save Games. Only profiles
        // that are not this one: copying a save onto itself is not a thing.
        if app.created.is_some() {
            // The memoised list, not a fresh read_dir: this bar is drawn on
            // every frame while anything is selected.
            let (names, here) = cached_profiles(app);
            for name in names.into_iter().filter(|p| *p != here) {
                bar = bar.push(
                    button(text(tf("saves_copy_to", &[("profile", &name)])).size(sz(11)))
                        .padding([3, 8])
                        .style(button::secondary)
                        .on_press(Message::SavesCopyToProfile(name)),
                );
            }
        }
        bar = bar.push(Space::new().width(Length::Fill)).push(
            button(
                text(if app.confirm_saves_delete {
                    tf("saves_confirm_delete_n", &[("n", &n)])
                } else {
                    tf("saves_delete_n", &[("n", &n)])
                })
                .size(sz(11)),
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
        .push(text(dir.display().to_string()).size(sz(10)));
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
        .push(text(save.filename.clone()).size(sz(13)))
        .push(
            button(text(t("saves_extension_details")).size(sz(11))).on_press(
                Message::RunFileExtension(
                    eidos_addons::protocol::Operation::SaveInfo,
                    save.path.clone(),
                ),
            ),
        );

    let info = match app.save_info.as_ref().filter(|(p, _)| *p == save.path) {
        Some((_, Ok(info))) => info,
        Some((_, Err(e))) => {
            return col
                .push(text(tf("saves_unreadable", &[("error", &e)])).size(sz(11)))
                .push(text(t("saves_unreadable_note")).size(sz(10)))
                .into();
        }
        // Parsed on selection, so this only shows for the frame in between.
        None => return col.push(text(t("saves_reading")).size(sz(11))).into(),
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
            t("saves_character"),
            tf(
                "saves_character_level",
                &[("name", &info.player_name), ("level", &info.level)],
            ),
        ),
        (t("saves_location"), info.location.clone()),
        (t("saves_game_date"), info.game_date.clone()),
    ];
    if let Some((d, h, m)) = info.playtime() {
        facts.push((
            t("saves_played"),
            tf("saves_playtime", &[("d", &d), ("h", &h), ("m", &m)]),
        ));
    }
    facts.push((t("saves_heading"), format!("#{}", info.save_number)));
    // Fallout writes the runtime it was saved by; Skyrim does not, and leaves this
    // empty. Worth a row of its own: "this save came from another game build" is
    // the answer to a whole class of "my DLL plugins stopped loading" sessions.
    if !info.game_version.is_empty() {
        facts.push((t("saves_game_build"), info.game_version.clone()));
    }
    facts.push((
        t("tab_plugins"),
        tf(
            "saves_plugin_counts",
            &[
                ("n", &info.plugins.len()),
                ("light", &info.light_plugins.len()),
            ],
        ),
    ));
    for (k, v) in facts {
        col = col.push(info_kv(k, v));
    }

    let missing = &app.save_missing;
    col = col.push(Space::new().height(Length::Fixed(6.0)));
    if missing.is_empty() {
        return col
            .push(text(t("saves_all_plugins_active")).size(sz(11)))
            .push(
                text(if info.truncated {
                    t("saves_truncated_advisory")
                } else {
                    ""
                })
                .size(sz(10)),
            )
            .into();
    }

    col = col.push(text(tf("saves_n_missing", &[("n", &missing.len())])).size(sz(12)));
    for m in missing.iter().take(40) {
        let what = match m.state {
            eidos_gamefeatures::SavePluginState::Inactive => t("saves_plugin_inactive"),
            eidos_gamefeatures::SavePluginState::Absent => t("saves_plugin_absent"),
        };
        let who = if m.providers.is_empty() {
            format!("  {}", t("saves_no_provider"))
        } else {
            format!(
                "  {}",
                tf("saves_provided_in", &[("mods", &m.providers.join(", "))])
            )
        };
        col = col
            .push(text(format!("{} ({what})", m.name)).size(sz(11)))
            .push(text(who).size(sz(10)));
    }
    // Only offer the fix when something on disk can actually supply the plugins;
    // otherwise the button would enable nothing and look broken.
    let fixable = missing.iter().any(|m| !m.providers.is_empty());
    if fixable {
        col = col
            .push(Space::new().height(Length::Fixed(4.0)))
            .push(tool_btn(t("saves_enable_needed"), Message::FixSaveMods));
    }
    if info.truncated {
        col = col.push(text(t("saves_truncated_note")).size(sz(10)));
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

/// The active profile's three newest saves Eidos can read, newest first. Empty
/// without an instance or for a game whose saves Eidos does not parse.
pub(crate) fn newest_profile_saves(app: &App) -> Vec<(PathBuf, eidos_gamefeatures::SaveInfo)> {
    let Some(inst) = &app.created else {
        return Vec::new();
    };
    eidos_gamefeatures::newest_saves(inst.savegames().into_iter().map(|s| s.path), 3)
}

/// The first of `saves` that loads `plugin` as a full plugin (its full table,
/// not the light one), matched ASCII case-insensitively.
pub(crate) fn full_in_saves<'a>(
    plugin: &str,
    saves: &'a [(PathBuf, eidos_gamefeatures::SaveInfo)],
) -> Option<&'a Path> {
    saves
        .iter()
        .find(|(_, s)| s.plugins.iter().any(|p| p.eq_ignore_ascii_case(plugin)))
        .map(|(path, _)| path.as_path())
}

/// The plugins of the `targets` mods that one of `saves` uses: each enabled
/// mod's top-level plugin files, minus those another mod currently wins,
/// matched ASCII case-insensitively. File spelling, sorted, deduplicated.
pub(crate) fn plugins_saves_use(
    app: &App,
    targets: &[usize],
    saves: &[(PathBuf, eidos_gamefeatures::SaveInfo)],
) -> Vec<String> {
    if saves.is_empty() {
        return Vec::new();
    }
    let computed;
    let list = match app.plugins.as_ref() {
        Some(list) => Some(list),
        None => {
            computed = compute_plugins(app);
            computed.as_ref()
        }
    };
    let used = |name: &str| {
        saves
            .iter()
            .any(|(_, s)| s.all_plugins().any(|p| p.eq_ignore_ascii_case(name)))
    };
    // Disabled targets count too: removing one deletes the only copy of a
    // plugin the saves still list. arm_disable leaves them out itself.
    let mut out: Vec<String> = Vec::new();
    for m in targets.iter().filter_map(|&i| app.mods.get(i)) {
        for name in fs::read_dir(&m.path)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
        {
            let another_wins = list.is_some_and(|l| {
                l.plugins
                    .iter()
                    .any(|p| p.name.eq_ignore_ascii_case(&name) && p.origin_mod != m.name)
            });
            if eidos_plugins::is_plugin(&name) && !another_wins && used(&name) {
                out.push(name);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The warning naming the plugins a disable or remove would take from the
/// newest saves: five names, then a count of the rest.
pub(crate) fn saves_warning(plugins: &[String]) -> String {
    let mut names = plugins
        .iter()
        .take(5)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    if plugins.len() > 5 {
        names.push_str(&tf("diag_and_n_more", &[("n", &(plugins.len() - 5))]));
    }
    tf("warn_saves_use", &[("plugins", &names)])
}

/// [`saves_warning`] and a space, to lead a Remove's click-again message, or
/// nothing when the newest saves use none of the `targets` plugins.
pub(crate) fn remove_warning(app: &App, targets: &[usize]) -> String {
    let plugins = plugins_saves_use(app, targets, &newest_profile_saves(app));
    if plugins.is_empty() {
        String::new()
    } else {
        saves_warning(&plugins) + " "
    }
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

/// The plugins in `names` that `list` will not load, each with the mods that hold
/// a file of that name. Every mod is searched, disabled ones included: a disabled
/// mod holding the plugin is the case the "enable what this needs" fixes exist for.
pub(crate) fn provider_scan<'a>(
    app: &App,
    list: &PluginList,
    names: impl IntoIterator<Item = &'a str>,
) -> Vec<eidos_gamefeatures::MissingPlugin> {
    let known: Vec<eidos_gamefeatures::KnownPlugin> = list
        .plugins
        .iter()
        .map(|p| eidos_gamefeatures::KnownPlugin {
            name: &p.name,
            enabled: p.enabled,
            origin_mod: &p.origin_mod,
        })
        .collect();
    // Overwrite counts as a provider too (a cleaned .esp lands there).
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
            name: t("saves_game_data"),
            path: d,
        });
    }
    eidos_gamefeatures::missing_from(names, &known, &mods, data.as_deref())
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
        (Ok(info), Some(list)) => provider_scan(app, list, info.all_plugins()),
        _ => Vec::new(),
    };
    app.save_info = Some((path, parsed));
}
