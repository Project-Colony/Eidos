//! The bisect panel (A37): find the mod behind a reproducible problem with a
//! series of test runs, each on half of the suspects. The search lives in
//! `<instance>/bisect.json`, so it survives the window closing between steps.

use crate::theme::{card_style, scaled};
use crate::*;
use eidos_instance::{Answer, Bisect, BisectResult, TestGroup, TestRunWrites, TestState};
use iced::widget::text::Wrapping;

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

/// What the step's test run says before the user answers. Either one pre-selects Bad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BisectHint {
    /// The newest crash log the run wrote.
    Crash(PathBuf),
    /// A non-zero exit code that is not a Stop.
    Exit(i32),
}

/// The first press of an action that throws files away or ends the search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BisectArm {
    Answer(Answer),
    Repeat,
    Stop,
}

/// What `app.test_state`'s run says about the problem. Not for a view: it
/// lists the crash log folders.
pub(crate) fn bisect_hint(app: &App) -> Option<BisectHint> {
    let (inst, (run, _)) = (app.created.as_ref()?, app.test_state.as_ref()?);
    if let Some(log) = selected_game(app).and_then(|g| test_run_crash_log(g, inst, run)) {
        return Some(BisectHint::Crash(log));
    }
    // Stop leaves 143 or 137: `exit_code` maps a signal to 128 + signal.
    run.exit
        .filter(|c| ![0, 137, 143].contains(c))
        .map(BisectHint::Exit)
}

/// The files a test run wrote, the Overwrite deletions it recorded included.
pub(crate) fn files_written(w: &TestRunWrites) -> usize {
    TestGroup::ALL.into_iter().map(|g| w.files(g)).sum()
}

/// The hint and the file count of an ended bisect step, for `app.bisect_step`.
/// Not for a view: it walks the test run's folders.
pub(crate) fn read_bisect_step(app: &App) -> (Option<BisectHint>, usize) {
    match (&app.test_state, &app.created) {
        (Some((run, TestState::Ended)), Some(inst)) if run.bisect => {
            (bisect_hint(app), files_written(&inst.test_run_writes()))
        }
        _ => (None, 0),
    }
}

/// The Result stage's lines; `t` picks the language, so the log can be English.
pub(crate) fn bisect_result_lines(
    r: &BisectResult,
    t: impl Fn(&str) -> &'static str,
) -> Vec<String> {
    match r {
        BisectResult::NoneLeft => vec![t("bisect_result_none").to_string()],
        BisectResult::Found { mods, confirmed } => {
            let mut lines = vec![t("bisect_result").replace("{names}", &mods.join(", "))];
            if !confirmed {
                lines.push(t("bisect_result_unconfirmed").to_string());
            }
            if mods.len() > 1 {
                lines.push(t("bisect_result_set").to_string());
            }
            lines
        }
    }
}

/// The text `end_bisect` keeps: one line per step, then the result, or where
/// the search stood when it was stopped.
pub(crate) fn bisect_log(b: &Bisect, result: Option<&BisectResult>) -> String {
    let mut log = String::new();
    for (i, s) in b.steps.iter().enumerate() {
        let answer = s
            .answer
            .map_or("no answer".to_string(), |a| format!("{a:?}"));
        log += &format!("step {}: {} -> {answer}\n", i + 1, s.tested.join(", "));
    }
    log += &match result {
        Some(r) => {
            bisect_result_lines(r, |k| crate::i18n::t_in(colony_ui::i18n::Locale::En, k)).join(" ")
        }
        None => format!(
            "Stopped with {} suspects left: {}",
            b.candidates.len(),
            b.candidates.join(", ")
        ),
    };
    log + "\n"
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
/// The status slot under the stage area: three lines, for the French Stop line
/// at the largest text size.
pub(crate) const BISECT_SLOT_H: f32 = 48.0;
/// The hint's one-line slot.
const LINE_H: f32 = 18.0;
/// The files line: three lines, for the French one beside its button at the
/// largest text size.
pub(crate) const BISECT_FILES_H: f32 = 48.0;

/// A tool's run would never take the request, so a step waits for the game.
fn game_picked(app: &App) -> bool {
    app.tool_choice
        .as_deref()
        .is_none_or(|t| picks_the_game(app, t))
}

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
                    game_picked(app).then_some(Message::BisectRunStep),
                ))
        }
        (BisectStage::Running, Some(b)) => {
            stage.push(line(tf("bisect_running", &[("n", &b.steps.len())])))
        }
        (BisectStage::OtherProfile, Some(b)) => {
            stage.push(line(tf("bisect_other_profile", &[("profile", &b.profile)])))
        }
        (BisectStage::Answer, Some(b)) => {
            let (hint, files) = &app.bisect_step;
            let hint_line = match hint {
                Some(BisectHint::Crash(log)) => tf("bisect_hint_crash", &[("log", &log.display())]),
                Some(BisectHint::Exit(code)) => tf("bisect_hint_exit", &[("code", code)]),
                None => String::new(),
            };
            let pick = |key: &str, msg: Option<Message>, arm: BisectArm, hinted: bool| {
                button(text(t(key)).size(sz(13)))
                    .padding([6, 14])
                    .on_press_maybe(msg)
                    .style(if app.bisect_armed == Some(arm) {
                        button::danger
                    } else if hinted {
                        button::primary
                    } else {
                        button::secondary
                    })
            };
            let answer = |a: Answer, key: &str| {
                let hinted = a == Answer::Bad && hint.is_some();
                pick(
                    key,
                    Some(Message::BisectAnswer(a)),
                    BisectArm::Answer(a),
                    hinted,
                )
            };
            let repeat = game_picked(app).then_some(Message::BisectRepeat);
            let mut written = Row::new()
                .spacing(8)
                .align_y(iced::alignment::Vertical::Center);
            if *files > 0 {
                written = written
                    .push(
                        text(tf("bisect_writes", &[("n", files)]))
                            .size(sz(12))
                            .width(Length::Fill),
                    )
                    .push(
                        button(text(t("bisect_review")).size(sz(12)))
                            .padding([4, 12])
                            .on_press(Message::BisectReview)
                            .style(button::secondary),
                    );
            }
            stage
                .push(line(tf("bisect_ask", &[("n", &b.steps.len())])))
                .push(
                    container(text(hint_line).size(sz(12)).wrapping(Wrapping::None))
                        .width(Length::Fill)
                        .height(Length::Fixed(scaled(LINE_H)))
                        .clip(true),
                )
                .push(
                    Row::new()
                        .spacing(8)
                        .push(answer(Answer::Good, "bisect_good"))
                        .push(answer(Answer::Bad, "bisect_bad"))
                        .push(answer(Answer::Unsure, "bisect_unsure"))
                        .push(pick("bisect_repeat", repeat, BisectArm::Repeat, false))
                        .wrap()
                        .vertical_spacing(8),
                )
                .push(
                    container(written)
                        .width(Length::Fill)
                        .height(Length::Fixed(scaled(BISECT_FILES_H)))
                        .clip(true),
                )
        }
        (BisectStage::Result, Some(b)) => match b.result(universe, plugins) {
            Some(r) => bisect_result_lines(&r, t)
                .into_iter()
                .fold(stage, |stage, l| stage.push(line(l))),
            None => stage,
        },
        _ => stage,
    };
    let answered = app
        .bisect
        .as_ref()
        .map_or(0, |b| b.steps.iter().filter(|s| s.answer.is_some()).count());
    let slot = match app.bisect_armed {
        Some(BisectArm::Answer(_)) => tf("bisect_confirm_answer", &[("n", &app.bisect_step.1)]),
        Some(BisectArm::Repeat) => tf("review_armed_line", &[("n", &app.bisect_step.1)]),
        Some(BisectArm::Stop) => tf("bisect_stop_line", &[("n", &answered)]),
        None => app.bisect_line.clone().unwrap_or_default(),
    };
    // Hide first: Stop's label grows when armed and moves nothing.
    let mut bottom = Row::new().spacing(10).push(
        button(text(t("bisect_hide")).size(sz(13)))
            .padding([6, 22])
            .on_press(Message::BisectHide)
            .style(button::secondary),
    );
    match bisect_stage(app) {
        BisectStage::Setup => {}
        BisectStage::Result => {
            bottom = bottom.push(primary(
                t("bisect_close").to_string(),
                Some(Message::BisectClose),
            ))
        }
        _ => {
            let armed = app.bisect_armed == Some(BisectArm::Stop);
            bottom = bottom.push(
                button(
                    text(t(if armed {
                        "bisect_stop_armed"
                    } else {
                        "bisect_stop"
                    }))
                    .size(sz(13)),
                )
                .padding([6, 22])
                .on_press(Message::BisectStop)
                .style(if armed {
                    button::danger
                } else {
                    button::secondary
                }),
            );
        }
    }
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
            container(text(slot).size(sz(12)))
                .width(Length::Fill)
                .height(Length::Fixed(scaled(BISECT_SLOT_H)))
                .clip(true),
        )
        .push(bottom);
    container(card)
        .max_width(560.0)
        .padding(20)
        .style(card_style)
        .into()
}
