//! The bisect panel (A37): start a search, run each step as a test run (A36)
//! on half of the suspects, their masters and the always-on mods, answer it,
//! and end the search with a log.

use super::test_run::{now, open_review};
use super::*;
use eidos_instance::{Answer, Bisect, TestRun, TestState};

/// Show the panel with what its view reads filled in.
pub(super) fn open_panel(app: &mut App) {
    app.bisect_universe = bisect_universe(app);
    app.bisect_pinned = bisect_pinned(app);
    app.bisect_open = true;
}

/// Read the test state again (the step may have run, or a test run ended, in
/// another window), then refuse while a run goes on or a normal test run waits
/// for its review: its files are the review's to decide, never a step's.
fn refused(app: &mut App) -> bool {
    load_test_state(app, false);
    if let Some(r) = &app.running {
        app.status = Some(tf("status_still_running", &[("title", &r.title)]));
        return true;
    }
    if matches!(&app.test_state, Some((run, TestState::Ended)) if !run.bisect) {
        app.status = Some(t("status_bisect_review_first").to_string());
        open_review(app);
        return true;
    }
    false
}

/// Write a bisect test run request with `mods` on, then Run as Test run does.
/// `record` is the search with its new step, saved only once the request is on
/// disk; when saving it fails the request goes too, so no step runs unrecorded.
fn request_step(app: &mut App, mods: Vec<String>, record: Option<Bisect>) -> Task<Message> {
    let (Some(inst), Some(game), Some(profile)) = (
        app.created.clone(),
        selected_game(app).map(|g| g.def.id.to_string()),
        app.bisect.as_ref().map(|b| b.profile.clone()),
    ) else {
        return Task::none();
    };
    let run = TestRun {
        profile,
        game,
        mods: Some(mods),
        bisect: true,
        requested: now(),
        ..Default::default()
    };
    if let Err(e) = inst.request_test_run(&run) {
        app.status = Some(tf("status_test_failed", &[("error", &e)]));
        return Task::none();
    }
    if let Some(b) = record {
        if let Err(e) = inst.write_bisect(&b) {
            let _ = inst.cancel_test_request();
            app.status = Some(tf("status_test_failed", &[("error", &e)]));
            return Task::none();
        }
        app.bisect = Some(b);
    }
    // Cleared so the prefix lands on what Run says, as Test run does.
    app.status = None;
    let task = launch::handle(app, Message::Run);
    launch::prepend_status(app, Some(t("status_test_prefix").to_string()));
    load_test_state(app, false);
    task
}

/// Request the next step's test run, and record the step only once the request
/// is on disk: a refused request leaves no step waiting for an answer.
fn run_step(app: &mut App) -> Task<Message> {
    if refused(app) || bisect_stage(app) != BisectStage::Ready {
        return Task::none();
    }
    let Some(mut b) = app.bisect.clone() else {
        return Task::none();
    };
    let universe = bisect_universe(app);
    let Some((on, tested)) = b.next_run(&universe.0, &universe.1) else {
        return Task::none();
    };
    app.bisect_universe = universe;
    b.begin_step(tested);
    request_step(app, on, Some(b))
}

/// Throw away the step's test run. When it wrote files the first press of
/// `arm` only arms. True once no test run is left.
fn throw_step_away(app: &mut App, arm: BisectArm) -> bool {
    let Some(inst) = app.created.clone() else {
        return false;
    };
    if inst.test_run().is_none() {
        return true;
    }
    if files_written(&inst.test_run_writes()) > 0 && app.bisect_armed != Some(arm) {
        app.bisect_armed = Some(arm);
        return false;
    }
    app.bisect_armed = None;
    if let Err(e) = inst.finish_test_run(&[]) {
        app.bisect_line = Some(tf("review_failed", &[("error", &e)]));
        return false;
    }
    load_test_state(app, false);
    true
}

fn answer(app: &mut App, a: Answer) {
    if refused(app) || bisect_stage(app) != BisectStage::Answer {
        return;
    }
    if !throw_step_away(app, BisectArm::Answer(a)) {
        return;
    }
    let (Some(inst), Some(mut b)) = (app.created.clone(), app.bisect.clone()) else {
        return;
    };
    b.answer(a);
    if let Err(e) = inst.write_bisect(&b) {
        app.bisect_line = Some(e.to_string());
        return;
    }
    app.bisect = Some(b);
    app.bisect_universe = bisect_universe(app);
}

/// Run the unanswered step again on the same mods.
fn repeat(app: &mut App) -> Task<Message> {
    if refused(app) || bisect_stage(app) != BisectStage::Answer {
        return Task::none();
    }
    let ran = app
        .test_state
        .as_ref()
        .and_then(|(run, _)| run.mods.clone());
    if !throw_step_away(app, BisectArm::Repeat) {
        return Task::none();
    }
    // With the run already reviewed away the split is worked out again: the
    // candidates have not changed since the step began.
    let (universe, plugins) = &app.bisect_universe;
    let Some(mods) = ran.or_else(|| {
        app.bisect
            .as_ref()
            .and_then(|b| b.next_run(universe, plugins))
            .map(|(on, _)| on)
    }) else {
        return Task::none();
    };
    request_step(app, mods, None)
}

/// Write the search's log, drop `bisect.json` and close the panel. A step's
/// test run that waits gets its review.
fn end_search(app: &mut App, result: Option<eidos_instance::BisectResult>) {
    // Disarmed first, so a failure line shows instead of the Stop line.
    app.bisect_armed = None;
    let (Some(inst), Some(b)) = (app.created.clone(), app.bisect.as_ref()) else {
        return;
    };
    if let Err(e) = inst.end_bisect(&bisect_log(b, result.as_ref()), now()) {
        app.bisect_line = Some(e.to_string());
        return;
    }
    app.bisect = None;
    app.bisect_open = false;
    load_test_state(app, true);
}

pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::OpenBisect => {
            // The step may have run in the window the store opened.
            load_test_state(app, false);
            open_panel(app);
        }
        Message::BisectStart => {
            let Some(inst) = app.created.clone().filter(|_| app.bisect.is_none()) else {
                return Task::none();
            };
            let universe = bisect_universe(app);
            let b = Bisect::new(
                &inst.active_profile(),
                app.bisect_pinned.clone(),
                universe.0.clone(),
                now(),
            );
            match inst.write_bisect(&b) {
                Ok(()) => {
                    app.bisect = Some(b);
                    app.bisect_universe = universe;
                }
                Err(e) => app.status = Some(e.to_string()),
            }
        }
        Message::BisectRunStep => return run_step(app),
        Message::BisectHide => app.bisect_open = false,
        Message::BisectAnswer(a) => answer(app, a),
        Message::BisectRepeat => return repeat(app),
        Message::BisectReview => {
            load_test_state(app, false);
            if matches!(app.test_state, Some((_, TestState::Ended))) {
                open_review(app);
            }
        }
        Message::BisectStop => {
            if matches!(bisect_stage(app), BisectStage::Setup | BisectStage::Result) {
                return Task::none();
            }
            if app.bisect_armed == Some(BisectArm::Stop) {
                end_search(app, None);
            } else {
                app.bisect_armed = Some(BisectArm::Stop);
            }
        }
        Message::BisectClose if bisect_stage(app) == BisectStage::Result => {
            let (universe, plugins) = &app.bisect_universe;
            let result = app
                .bisect
                .as_ref()
                .and_then(|b| b.result(universe, plugins));
            end_search(app, result);
        }
        _ => {}
    }
    Task::none()
}
