//! The Test run dialog (A36): write the request, then Run as usual. The game
//! run that starts next, from here or from its store, consumes it. Once it
//! ends, the review keeps or throws away what it wrote.

use super::*;
use eidos_instance::{TestGroup, TestState};

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Read the instance's test run into `app.test_state`, which the view reads
/// instead of the lock. A run that died is closed as unfinished first. With
/// `open`, an ended run opens its review (a bisect step's is the bisect
/// panel's), and a waiting request says so when the status bar is empty.
pub(crate) fn load_test_state(app: &mut App, open: bool) {
    let Some(inst) = app.created.as_ref() else {
        app.test_state = None;
        return;
    };
    let mut state = inst.test_run_state();
    if matches!(state, Some((_, TestState::Interrupted))) {
        if let Err(e) = inst.close_interrupted_test_run(now()) {
            app.status = Some(e.to_string());
        }
        state = inst.test_run_state();
    }
    app.test_state = state;
    if !open {
        return;
    }
    match &app.test_state {
        Some((run, TestState::Ended)) if !run.bisect => open_review(app),
        Some((run, TestState::Requested)) if app.status.is_none() => {
            app.status = Some(tf("test_waiting", &[("profile", &run.profile)]));
        }
        _ => {}
    }
}

/// Open the review of the test run `app.test_state` holds.
fn open_review(app: &mut App) {
    let (Some(inst), Some((run, _))) = (app.created.as_ref(), app.test_state.as_ref()) else {
        return;
    };
    let crash = selected_game(app).and_then(|g| test_run_crash_log(g, inst, run));
    app.test_review = Some(TestReview::new(inst.test_run_writes(), run.clone(), crash));
}

/// Apply: every group with files decided, then a second press when it
/// deletes files, then move the kept groups into place.
fn review_apply(app: &mut App) {
    let (Some(inst), Some(review)) = (app.created.as_ref(), app.test_review.as_mut()) else {
        return;
    };
    if review.undecided() {
        review.line = Some(t("review_choose_all").to_string());
        return;
    }
    let gone = review.thrown_away();
    if gone > 0 && !review.armed {
        review.armed = true;
        review.line = Some(tf("review_armed_line", &[("n", &gone)]));
        return;
    }
    let kept = review.kept();
    if let Err(e) = inst.finish_test_run(&kept) {
        review.line = Some(tf("review_failed", &[("error", &e)]));
        return;
    }
    app.test_review = None;
    load_saves(app);
    drop_files_cache(app, Some("Overwrite"));
    if kept.contains(&TestGroup::Output) {
        app.conflicts = compute_conflicts(app);
    }
    if kept.contains(&TestGroup::Plugins) {
        invalidate_plugins(app);
    }
    app.status = Some(t("status_test_closed").to_string());
    load_test_state(app, false);
}

pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::OpenTestRun => {
            // The run may have happened in the window or CLI the store started.
            load_test_state(app, false);
            if matches!(app.test_state, Some((_, TestState::Ended))) {
                open_review(app);
            } else {
                app.test_dialog = Some(TestChoice::Profile);
            }
        }
        Message::TestChoose(choice) => app.test_dialog = Some(choice),
        Message::CloseTestDialog => app.test_dialog = None,
        Message::TestStart => {
            let (Some(inst), Some(game), Some(choice)) =
                (app.created.as_ref(), selected_game(app), app.test_dialog)
            else {
                return Task::none();
            };
            let run = eidos_instance::TestRun {
                profile: inst.active_profile(),
                game: game.def.id.to_string(),
                mods: test_mods(app, choice),
                bisect: false,
                requested: now(),
                ..Default::default()
            };
            if let Err(e) = inst.request_test_run(&run) {
                app.status = Some(tf("status_test_failed", &[("error", &e)]));
                return Task::none();
            }
            app.test_dialog = None;
            // Cleared so the prefix lands on what Run says, never on an older line.
            app.status = None;
            let task = launch::handle(app, Message::Run);
            launch::prepend_status(app, Some(t("status_test_prefix").to_string()));
            load_test_state(app, false);
            return task;
        }
        Message::TestCancelRequest => {
            app.status = app
                .created
                .as_ref()
                .map(|inst| match inst.cancel_test_request() {
                    Ok(()) => t("status_test_cancelled").to_string(),
                    Err(e) => e.to_string(),
                });
            load_test_state(app, false);
        }
        Message::ReviewChoose(g, keep) => {
            if let Some(r) = app.test_review.as_mut() {
                r.keep[g as usize] = Some(keep);
            }
        }
        Message::ReviewApply => review_apply(app),
        Message::ReviewLater => app.test_review = None,
        _ => {}
    }
    Task::none()
}
