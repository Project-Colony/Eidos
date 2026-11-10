//! The bisect panel (A37): start a search, then run each step as a test run
//! (A36) on half of the suspects, their masters and the always-on mods.

use super::test_run::{now, open_review};
use super::*;
use eidos_instance::{Bisect, TestRun, TestState};

/// Show the panel with what its view reads filled in.
pub(super) fn open_panel(app: &mut App) {
    app.bisect_universe = bisect_universe(app);
    app.bisect_pinned = bisect_pinned(app);
    app.bisect_open = true;
}

/// Request the next step's test run, and record the step only once the request
/// is on disk: a refused request leaves no step waiting for an answer.
fn run_step(app: &mut App) -> Task<Message> {
    // The step may have run, or a test run ended, in another window.
    load_test_state(app, false);
    if let Some(r) = &app.running {
        app.status = Some(tf("status_still_running", &[("title", &r.title)]));
        return Task::none();
    }
    if matches!(&app.test_state, Some((run, TestState::Ended)) if !run.bisect) {
        app.status = Some(t("status_bisect_review_first").to_string());
        open_review(app);
        return Task::none();
    }
    if bisect_stage(app) != BisectStage::Ready {
        return Task::none();
    }
    let (Some(inst), Some(game), Some(mut b)) = (
        app.created.clone(),
        selected_game(app).map(|g| g.def.id.to_string()),
        app.bisect.clone(),
    ) else {
        return Task::none();
    };
    let universe = bisect_universe(app);
    let Some((on, tested)) = b.next_run(&universe.0, &universe.1) else {
        return Task::none();
    };
    app.bisect_universe = universe;
    let run = TestRun {
        profile: b.profile.clone(),
        game,
        mods: Some(on),
        bisect: true,
        requested: now(),
        ..Default::default()
    };
    if let Err(e) = inst.request_test_run(&run) {
        app.status = Some(tf("status_test_failed", &[("error", &e)]));
        return Task::none();
    }
    b.begin_step(tested);
    if let Err(e) = inst.write_bisect(&b) {
        // The step is not recorded, so its run must not start either.
        let _ = inst.cancel_test_request();
        app.status = Some(tf("status_test_failed", &[("error", &e)]));
        return Task::none();
    }
    app.bisect = Some(b);
    // Cleared so the prefix lands on what Run says, as Test run does.
    app.status = None;
    let task = launch::handle(app, Message::Run);
    launch::prepend_status(app, Some(t("status_test_prefix").to_string()));
    load_test_state(app, false);
    task
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
        _ => {}
    }
    Task::none()
}
