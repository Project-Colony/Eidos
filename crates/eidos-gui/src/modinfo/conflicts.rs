//! The Conflicts tab: the conflict map and its panel.

use super::*;

/// Analyse file conflicts across the enabled mods (+ the game data) for the
/// Conflicts tab and the mod-row flags. Highest-priority mod first; game data
/// last as origin 0. `None` if there is no game.
pub(crate) fn compute_conflicts(app: &App) -> Option<ConflictMap> {
    Some(build_conflicts_cached(app, conflict_layers(app)?))
}

pub(crate) fn conflict_layers(app: &App) -> Option<Vec<Layer>> {
    let game = selected_game(app)?;
    // app.mods is MO2 display order (lowest priority first); the conflict crate wants
    // layers highest-priority first, so reverse. The origin stays the app.mods index
    // + 1 (NOT the layer position), so conflicts_panel's `origin = i + 1` lookup over
    // app.mods still maps to the same mod.
    let mut layers: Vec<Layer> = app
        .mods
        .iter()
        .enumerate()
        .filter(|(_, m)| m.is_active() && !m.is_unmanaged())
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
            layers.insert(
                0,
                Layer {
                    origin: u32::MAX,
                    name: "Overwrite".to_string(),
                    root: ow,
                },
            );
        }
    }
    layers.push(Layer {
        origin: 0,
        name: format!("[{}]", game.def.id),
        root: game.data_path.clone(),
    });
    Some(layers)
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
            .push(text("Conflicts").size(sz(13)))
            .push(
                text("Open a game instance to analyse file conflicts across your mods.").size(sz(12)),
            )
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
            .mod_conflicts(origin)
            .map(|c| format!("{}/{} won", c.won, c.total))
            .unwrap_or_default();
        let row = Row::new()
            .spacing(6)
            .push(text(m.name.clone()).size(sz(12)).width(Length::Fill))
            .push(text(tag).size(sz(11)).width(Length::Fixed(160.0)))
            .push(text(detail).size(sz(10)).width(Length::Fixed(scaled(80.0))));
        rows = rows.push(striped(row.into(), i % 2 == 0));
    }

    let summary = format!(
        "{} overwrite - {} overwritten - {} mixed - {} redundant",
        counts.0, counts.1, counts.2, counts.3
    );
    Column::new()
        .spacing(6)
        .push(
            text(format!(
                "Conflicts: {summary}{}",
                if app.archive_job.is_some() {
                    " — archive analysis running"
                } else {
                    ""
                }
            ))
            .size(sz(12)),
        )
        .push(text(app.archive_warnings.join("\n")).size(sz(11)))
        .push(text("(only conflicting mods shown; flags also appear in the mod list)").size(sz(10)))
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
        return text("Select a mod to see which files it contests.")
            .size(sz(11))
            .into();
    };
    let origin = (focus + 1) as u32;
    let name = app
        .mods
        .get(focus)
        .map(|m| m.display_name().to_string())
        .unwrap_or_default();

    let mut rows = Column::new().spacing(1);
    let mut n = 0usize;
    for node in map.files.values() {
        // No Vec: this loop visits every file in the conflict map on every
        // redraw, and materialising a heap-allocated provider list per node made
        // each pointer event allocate and free once per file in the instance -
        // several hundred thousand times for a real load order. Two comparisons
        // answer the same question.
        let contested =
            node.is_conflicted() && (node.winner == origin || node.alternatives.contains(&origin));
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
            .push(
                text(node.display_path.clone())
                    .size(sz(11))
                    .width(Length::Fill),
            )
            .push(
                text(verdict)
                    .size(sz(11))
                    .width(Length::Fixed(260.0))
                    .color(if wins {
                        conflict_wins_fg()
                    } else {
                        conflict_loses_fg()
                    }),
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
        .push(text(head).size(sz(12)))
        .push(scrollable(rows).height(Length::Fill))
        .push(archive_member_rows(app, map, origin, 100))
        .into()
}
