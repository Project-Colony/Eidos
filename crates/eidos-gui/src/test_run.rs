//! The Test run dialog (A36): which mods the next game run uses, while its
//! writes wait in the instance's `test-run/` folder.

use crate::theme::{card_style, scaled};
use crate::*;
use eidos_instance::TestState;

/// The mods a test run is asked with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TestChoice {
    Profile,
    Vanilla,
    Selected,
}

/// The plugin rows, from the Plugins tab when it is showing and computed
/// otherwise (`app.plugins` is None on every other tab). Not for a view: it
/// can read every enabled mod's plugin headers.
pub(crate) fn plugin_rows(app: &App) -> Vec<eidos_plugins::Plugin> {
    app.plugins
        .clone()
        .or_else(|| compute_plugins(app))
        .map(|l| l.plugins)
        .unwrap_or_default()
}

/// `TestRun.mods` for a choice: None runs the profile's own set.
pub(crate) fn test_mods(app: &App, choice: TestChoice) -> Option<Vec<String>> {
    let name = |i: usize| app.mods[i].name.clone();
    match choice {
        TestChoice::Profile => None,
        TestChoice::Vanilla => Some(Vec::new()),
        TestChoice::Selected => {
            let chosen: Vec<String> = real_selection(app).into_iter().map(name).collect();
            let universe: Vec<String> = app
                .mods
                .iter()
                .filter(|m| m.is_active() && !m.is_unmanaged())
                .map(|m| m.name.clone())
                .collect();
            Some(eidos_instance::with_masters(
                &chosen,
                &universe,
                &plugin_rows(app),
            ))
        }
    }
}

pub(crate) fn test_dialog<'a>(app: &App, choice: TestChoice) -> Element<'a, Message> {
    let line = |s: String| text(s).size(sz(12));
    let mut card = Column::new()
        .spacing(10)
        .push(text(t("main_test_run")).size(sz(18)))
        .push(line(t("test_dialog_body").to_string()));
    card = match app.created.as_ref().and_then(Instance::test_run_state) {
        Some((run, TestState::Requested)) => card
            .push(line(tf("test_waiting", &[("profile", &run.profile)])))
            .push(
                button(text(t("test_cancel_request")).size(sz(13)))
                    .padding([6, 22])
                    .on_press(Message::TestCancelRequest)
                    .style(button::secondary),
            ),
        Some((_, TestState::Running)) => card.push(line(t("test_in_progress").to_string())),
        _ => {
            let n = real_selection(app).len();
            let pick = |label: String, this: TestChoice, enabled: bool| {
                button(text(label).size(sz(13)))
                    .width(Length::Fill)
                    .padding([6, 12])
                    .on_press_maybe(enabled.then_some(Message::TestChoose(this)))
                    .style(if this == choice {
                        button::primary
                    } else {
                        button::secondary
                    })
            };
            let cloud =
                selected_game(app).is_some_and(|g| g.is_steam() && g.saves_path().is_some());
            // Two lines tall whether or not the note shows: the French one wraps.
            let note = container(text(if cloud { t("test_cloud_note") } else { "" }).size(sz(11)))
                .height(Length::Fixed(scaled(30.0)))
                .clip(true);
            let ready = choice != TestChoice::Selected || n > 0;
            card.push(pick(
                t("test_with_profile").into(),
                TestChoice::Profile,
                true,
            ))
            .push(pick(t("test_vanilla").into(), TestChoice::Vanilla, true))
            .push(pick(
                tf("test_only_selected", &[("n", &n)]),
                TestChoice::Selected,
                n > 0,
            ))
            .push(note)
            .push(
                Row::new()
                    .spacing(10)
                    .push(
                        button(text(t("test_start")).size(sz(13)))
                            .padding([6, 22])
                            .on_press_maybe(ready.then_some(Message::TestStart))
                            .style(button::primary),
                    )
                    .push(
                        button(text(t("common_cancel")).size(sz(13)))
                            .padding([6, 22])
                            .on_press(Message::CloseTestDialog)
                            .style(button::secondary),
                    ),
            )
        }
    };
    container(card)
        .max_width(560.0)
        .padding(20)
        .style(card_style)
        .into()
}
