//! The bisect panel (A37): find the mod behind a reproducible problem with a
//! series of test runs, each on half of the suspects. The search lives in
//! `<instance>/bisect.json`, so it survives the window closing between steps.

use crate::theme::{card_style, scaled};
use crate::*;
use eidos_instance::{Bisect, TestState};

/// Where the search stands, which decides what the panel shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BisectStage {
    /// No search yet.
    Setup,
    /// The next step can run.
    Ready,
    /// The last step's test run waits or runs.
    Running,
    /// The last step's test run ended: did the problem happen?
    Answer,
    Result,
    /// The search belongs to a profile that is not the active one.
    OtherProfile,
}

/// Reads the cached `app.bisect_universe`, so a view can call it.
pub(crate) fn bisect_stage(app: &App) -> BisectStage {
    let Some(b) = &app.bisect else {
        return BisectStage::Setup;
    };
    if app
        .created
        .as_ref()
        .is_some_and(|i| i.active_profile() != b.profile)
    {
        return BisectStage::OtherProfile;
    }
    let (universe, plugins) = &app.bisect_universe;
    if b.result(universe, plugins).is_some() {
        return BisectStage::Result;
    }
    if b.steps.last().is_some_and(|s| s.answer.is_none()) {
        return match app.test_state.as_ref().map(|s| s.1) {
            Some(TestState::Requested | TestState::Running) => BisectStage::Running,
            _ => BisectStage::Answer,
        };
    }
    BisectStage::Ready
}

/// The enabled managed mods in display order, and the plugin rows that give
/// their masters. Not for a view: it can read every enabled mod's plugin headers.
pub(crate) fn bisect_universe(app: &App) -> (Vec<String>, Vec<eidos_plugins::Plugin>) {
    let mods = app
        .mods
        .iter()
        .filter(|m| m.is_active() && !m.is_unmanaged())
        .map(|m| m.name.clone())
        .collect();
    (mods, plugin_rows(app))
}

/// The mods a new search keeps on in every step: the selection in display
/// order, then the mod that ships the script extender's loader. Not for a view.
pub(crate) fn bisect_pinned(app: &App) -> Vec<String> {
    let mut picked = real_selection(app);
    picked.sort_unstable();
    let mut pinned: Vec<String> = picked
        .into_iter()
        .map(|i| app.mods[i].name.clone())
        .collect();
    let extender = selected_game(app)
        .and_then(|g| g.def.script_extender.as_ref())
        .zip(app.created.as_ref())
        .and_then(|(se, inst)| inst.active().mod_with_root_file(se.loader));
    if let Some(m) = extender.filter(|m| !pinned.contains(m)) {
        pinned.push(m);
    }
    pinned
}

/// The stage area's height: every stage fits in it, so the bottom row stays put.
const BISECT_STAGE_H: f32 = 180.0;
/// The always-on line: two lines, clipped when the list is longer.
const PINNED_H: f32 = 32.0;

pub(crate) fn bisect_panel<'a>(app: &App) -> Element<'a, Message> {
    let line = |s: String| text(s).size(sz(12));
    let primary = |label: String, msg: Option<Message>| {
        button(text(label).size(sz(13)))
            .padding([6, 22])
            .on_press_maybe(msg)
            .style(button::primary)
    };
    let (universe, plugins) = &app.bisect_universe;
    let stage = Column::new().spacing(10).width(Length::Fill);
    let stage = match (bisect_stage(app), &app.bisect) {
        (BisectStage::Setup, _) => {
            let probe = Bisect::new("", app.bisect_pinned.clone(), universe.clone(), 0);
            let pinned = if app.bisect_pinned.is_empty() {
                t("bisect_pinned_none").to_string()
            } else {
                tf("bisect_pinned", &[("names", &app.bisect_pinned.join(", "))])
            };
            stage
                .push(line(tf(
                    "bisect_intro",
                    &[("n", &probe.candidates.len()), ("runs", &probe.runs_left())],
                )))
                .push(
                    container(line(pinned))
                        .width(Length::Fill)
                        .height(Length::Fixed(scaled(PINNED_H)))
                        .clip(true),
                )
                .push(primary(
                    t("bisect_start").to_string(),
                    Some(Message::BisectStart),
                ))
        }
        (BisectStage::Ready, Some(b)) => {
            let n = b.steps.len() + 1;
            let tested = b
                .next_run(universe, plugins)
                .map_or(0, |(_, tested)| tested.len());
            // A tool's run would never take the request, so the step waits for the game.
            let game_picked = app
                .tool_choice
                .as_deref()
                .is_none_or(|t| picks_the_game(app, t));
            stage
                .push(line(tf(
                    "bisect_progress",
                    &[("left", &b.candidates.len()), ("runs", &b.runs_left())],
                )))
                .push(line(tf(
                    "bisect_step_detail",
                    &[("n", &n), ("tested", &tested)],
                )))
                .push(primary(
                    tf("bisect_run_step", &[("n", &n)]),
                    game_picked.then_some(Message::BisectRunStep),
                ))
        }
        (BisectStage::Running, Some(b)) => {
            stage.push(line(tf("bisect_running", &[("n", &b.steps.len())])))
        }
        (BisectStage::OtherProfile, Some(b)) => {
            stage.push(line(tf("bisect_other_profile", &[("profile", &b.profile)])))
        }
        // shortcut: Answer and Result show an empty line until Task 89 fills them.
        _ => stage.push(line(String::new())),
    };
    let card = Column::new()
        .spacing(10)
        .push(text(t("bisect_title")).size(sz(18)))
        .push(
            container(stage)
                .width(Length::Fill)
                .height(Length::Fixed(scaled(BISECT_STAGE_H)))
                .clip(true),
        )
        .push(
            Row::new().spacing(10).push(
                button(text(t("bisect_hide")).size(sz(13)))
                    .padding([6, 22])
                    .on_press(Message::BisectHide)
                    .style(button::secondary),
            ),
        );
    container(card)
        .max_width(560.0)
        .padding(20)
        .style(card_style)
        .into()
}
