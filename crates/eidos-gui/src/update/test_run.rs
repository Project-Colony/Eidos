//! The Test run dialog (A36): write the request, then Run as usual. The game
//! run that starts next, from here or from its store, consumes it.

use super::*;

pub(super) fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::OpenTestRun => {
            app.test_state = app.created.as_ref().and_then(Instance::test_run_state);
            app.test_dialog = Some(TestChoice::Profile);
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
                requested: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs()),
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
            app.test_state = app.created.as_ref().and_then(Instance::test_run_state);
        }
        _ => {}
    }
    Task::none()
}
