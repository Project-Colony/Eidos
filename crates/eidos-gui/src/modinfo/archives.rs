//! The Archives tab: every BSA/BA2 the enabled mods ship, and whether it loads.

use super::*;

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
        Some(l) => l
            .plugins
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.name.clone())
            .collect(),
        None => {
            let spec = selected_game(app)
                .filter(|g| g.def.id == game_id)?
                .plugin_spec()?;
            let prof = inst.active();
            let dir = if prof.has_plugin_state() {
                prof.plugins_state_dir()
            } else {
                selected_game(app)?.plugin_state_dir()?
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
    pub(crate) overridden_by: Option<String>,
}

impl ArchiveRow {
    /// Whether the engine will load it at all.
    pub(crate) fn loaded(&self) -> bool {
        self.overridden_by.is_none() && (self.registered || self.by_plugin.is_some())
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
    let mods: Vec<_> = app
        .mods
        .iter()
        .filter(|m| m.is_active())
        .map(|m| (m.name.clone(), m.path.clone()))
        .collect();
    let archives = eidos_gamefeatures::mod_archives(&mods);
    let fallback;
    let plan = if app.archive_completed_epoch == Some(app.archive_epoch.get()) {
        app.archive_plan.as_ref()?
    } else {
        let active = active_plugin_names(app, game_id)?;
        let game = selected_game(app)?;
        let (texts, _) = archive_ini_texts(game, inst);
        fallback = eidos_gamefeatures::archives::archive_plan(
            game_id,
            &archives.iter().map(|(_, a)| a.clone()).collect::<Vec<_>>(),
            &active,
            &texts,
            "en",
        );
        &fallback
    };
    Some(
        archives
            .into_iter()
            .map(|(mod_name, archive)| {
                let activation = plan
                    .archives
                    .iter()
                    .find(|a| a.name.eq_ignore_ascii_case(&archive));
                let overridden_by = app
                    .conflicts
                    .as_ref()
                    .and_then(|map| {
                        map.files
                            .get(&archive.to_ascii_lowercase())
                            .map(|n| (map, n))
                    })
                    .filter(|(map, n)| map.name(n.winner) != mod_name)
                    .map(|(map, n)| map.name(n.winner).to_string());
                ArchiveRow {
                    mod_name,
                    archive,
                    by_plugin: activation.and_then(|a| a.plugin.clone()),
                    registered: activation.is_some_and(|a| a.plugin.is_none()),
                    overridden_by,
                }
            })
            .collect(),
    )
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
        return text(t("archives_open_instance")).into();
    };
    let Some(rows) = cached_archive_rows(app, game.def.id) else {
        return Column::new()
            .spacing(6)
            .push(text(t("tab_archives")).size(sz(13)))
            .push(
                text(t("archives_plugins_unknown"))
                .size(sz(12)),
            )
            .into();
    };

    let orphans = rows
        .iter()
        .filter(|r| !r.loaded() && r.overridden_by.is_none())
        .count();
    let header = Row::new()
        .align_y(iced::Alignment::Center)
        .spacing(8)
        .push(
            text(tf("archives_count", &[("n", &rows.len())]))
            .size(sz(13)),
        )
        .push(Space::new().width(Length::Fill))
        .push(
            text(if orphans == 0 {
                t("archives_all_load").to_string()
            } else {
                tf("archives_n_will_not_load", &[("n", &orphans)])
            })
            .size(sz(11))
            .color(if orphans == 0 {
                conflict_wins_fg()
            } else {
                conflict_loses_fg()
            }),
        );

    let col_header = Row::new()
        .spacing(8)
        .push(text(t("archives_col_archive")).size(sz(11)).width(Length::FillPortion(3)))
        .push(text(t("archives_col_from_mod")).size(sz(11)).width(Length::FillPortion(2)))
        .push(
            text(t("archives_col_loaded_because"))
                .size(sz(11))
                .width(Length::FillPortion(3)),
        );

    let mut list = Column::new();
    if rows.is_empty() {
        list = list.push(text(t("archives_none")).size(sz(12)));
    }
    for (i, r) in rows.iter().enumerate() {
        let why: Element<'a, Message> = if let Some(winner) = &r.overridden_by {
            text(tf("archives_overridden", &[("winner", &winner)]))
                .size(sz(11))
                .into()
        } else {
            match (&r.by_plugin, r.registered) {
                (_, true) => text(t("archives_ini_registers")).size(sz(11)).into(),
                (Some(p), _) => text(tf("archives_plugin_active", &[("plugin", &p)])).size(sz(11)).into(),
                (None, false) => text(t("archives_dead_weight"))
                    .size(sz(11))
                    .color(conflict_loses_fg())
                    .into(),
            }
        };
        let row = Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .push(
                text(r.archive.clone())
                    .size(sz(12))
                    .width(Length::FillPortion(3)),
            )
            .push(
                text(r.mod_name.clone())
                    .size(sz(11))
                    .width(Length::FillPortion(2)),
            )
            .push(container(why).width(Length::FillPortion(3)));
        list = list.push(list_row(
            container(row).padding(3).into(),
            i % 2 == 0,
            false,
            None,
            None,
        ));
    }

    Column::new()
        .spacing(6)
        .push(header)
        .push(col_header)
        .push(scrollable(list).height(Length::Fill))
        .push(
            text(t("archives_explainer"))
            .size(sz(10)),
        )
        .into()
}
