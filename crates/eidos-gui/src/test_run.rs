//! The Test run dialog (A36): which mods the next game run uses, while its
//! writes wait in the instance's `test-run/` folder; and the review of what
//! the run wrote, group by group, once it ends.

use crate::theme::{card_style, scaled, text_muted};
use crate::*;
use eidos_instance::{TestGroup, TestRun, TestRunWrites, TestState};
use iced::widget::text::Wrapping;

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
    card = match &app.test_state {
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

/// The review of an ended test run: what it wrote, and the choice per group.
pub(crate) struct TestReview {
    pub writes: TestRunWrites,
    pub run: TestRun,
    /// Keep (true) or throw away (false), indexed like `TestGroup::ALL`.
    pub keep: [Option<bool>; 5],
    /// The first Apply press of one that deletes files.
    pub armed: bool,
    /// The status slot's line.
    pub line: Option<String>,
    pub crash: Option<PathBuf>,
}

impl TestReview {
    pub(crate) fn new(writes: TestRunWrites, run: TestRun, crash: Option<PathBuf>) -> TestReview {
        TestReview {
            writes,
            run,
            keep: [None; 5],
            armed: false,
            line: None,
            crash,
        }
    }

    /// `g`'s slot in `keep`: `TestGroup::ALL` lists the variants in declaration order.
    pub(crate) fn choice(&self, g: TestGroup) -> Option<bool> {
        self.keep[g as usize]
    }

    /// A group with files and no choice yet.
    pub(crate) fn undecided(&self) -> bool {
        TestGroup::ALL
            .into_iter()
            .any(|g| self.writes.files(g) > 0 && self.choice(g).is_none())
    }

    /// The files Apply removes for good: a thrown-away group's files, and the
    /// Overwrite files a kept output deletes. Throwing away the output never
    /// applies its deletions.
    pub(crate) fn thrown_away(&self) -> usize {
        let w = &self.writes;
        TestGroup::ALL
            .into_iter()
            .map(|g| match (g, self.choice(g)) {
                (TestGroup::Output, Some(true)) => w.output_deleted,
                (TestGroup::Output, Some(false)) => w.output.len(),
                (_, Some(false)) => w.files(g),
                _ => 0,
            })
            .sum()
    }

    pub(crate) fn kept(&self) -> Vec<TestGroup> {
        TestGroup::ALL
            .into_iter()
            .filter(|g| self.choice(*g) == Some(true))
            .collect()
    }
}

/// The newest crash log the test run wrote, in its My Games or Data upper.
pub(crate) fn test_run_crash_log(
    game: &DetectedGame,
    inst: &Instance,
    run: &TestRun,
) -> Option<PathBuf> {
    eidos_gamefeatures::crash_log::newest_crash_log(
        game.def.id,
        &inst.test_docs_upper(),
        &inst.test_data_upper(),
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(run.started?),
    )
}

/// The review's header slot and each group row: two lines, since the French
/// "did not finish" and "not offered" lines wrap.
pub(crate) const REVIEW_HEADER_H: f32 = 32.0;
/// A one-line slot: the crash line, the status line.
const REVIEW_LINE_H: f32 = 18.0;

fn group_label(w: &TestRunWrites, g: TestGroup) -> String {
    match g {
        TestGroup::Saves => tf("review_saves", &[("n", &w.saves.len())]),
        TestGroup::Inis => tf("review_inis", &[("n", &w.inis.len())]),
        TestGroup::Plugins if !w.plugins_offered => t("review_plugins_not_offered").to_string(),
        TestGroup::Plugins => tf("review_plugins", &[("n", &w.plugins.len())]),
        TestGroup::Output => tf("review_output", &[("n", &w.output.len())]),
        TestGroup::Logs => tf("review_logs", &[("n", &w.logs.len())]),
    }
}

fn group_files(w: &TestRunWrites, g: TestGroup) -> &[String] {
    match g {
        TestGroup::Saves => &w.saves,
        TestGroup::Inis => &w.inis,
        TestGroup::Plugins => &w.plugins,
        TestGroup::Output => &w.output,
        TestGroup::Logs => &w.logs,
    }
}

/// What the test run wrote and a Keep or Throw away per group. Every part is
/// always there at a fixed height, so the dialog never changes size.
pub(crate) fn review_dialog<'a>(review: &TestReview) -> Element<'a, Message> {
    let w = &review.writes;
    let one_line = |s: String| {
        container(text(s).size(sz(12)).wrapping(Wrapping::None))
            .width(Length::Fill)
            .height(Length::Fixed(scaled(REVIEW_LINE_H)))
            .clip(true)
    };
    let header = match review.run.exit {
        Some(code) => tf("review_ended", &[("code", &code)]),
        None => t("review_interrupted").to_string(),
    };
    let crash = review.crash.as_ref().map_or(String::new(), |p| {
        tf("review_crashed", &[("log", &p.display())])
    });
    let mut card = Column::new()
        .spacing(10)
        .push(text(t("review_title")).size(sz(18)))
        .push(
            container(text(header).size(sz(12)))
                .height(Length::Fixed(scaled(REVIEW_HEADER_H)))
                .clip(true),
        )
        .push(one_line(crash));
    for g in TestGroup::ALL {
        let enabled = w.files(g) > 0;
        let pick = |keep: bool, label: &str| {
            button(text(t(label)).size(sz(12)))
                .padding([4, 12])
                .on_press_maybe(enabled.then_some(Message::ReviewChoose(g, keep)))
                .style(if review.choice(g) == Some(keep) {
                    button::primary
                } else {
                    button::secondary
                })
        };
        card = card.push(
            Row::new()
                .spacing(8)
                .align_y(iced::alignment::Vertical::Center)
                .height(Length::Fixed(scaled(REVIEW_HEADER_H)))
                .push(text(group_label(w, g)).size(sz(12)).width(Length::Fill))
                .push(pick(true, "review_keep"))
                .push(pick(false, "review_drop")),
        );
    }
    let mut list = Column::new().spacing(2);
    for g in TestGroup::ALL {
        let files = group_files(w, g);
        if files.is_empty() {
            continue;
        }
        list = list.push(text(group_label(w, g)).size(sz(12)));
        for f in files {
            list = list
                .push(container(text(f.clone()).size(sz(11)).color(text_muted())).padding([0, 12]));
        }
    }
    let saves = !w.saves.is_empty();
    let notes = [
        (w.saves_replacing > 0).then(|| tf("review_note_replacing", &[("n", &w.saves_replacing)])),
        (w.saves_deleted > 0).then(|| tf("review_note_deleted", &[("n", &w.saves_deleted)])),
        (review.run.mods.is_some() && saves).then(|| t("review_note_subset").to_string()),
        (review.run.cloud && saves).then(|| t("review_note_cloud").to_string()),
        (w.output_deleted > 0)
            .then(|| tf("review_note_output_deletes", &[("n", &w.output_deleted)])),
    ];
    for note in notes.into_iter().flatten() {
        list = list.push(text(note).size(sz(11)));
    }
    let apply = if review.armed {
        t("review_apply_armed")
    } else {
        t("review_apply")
    };
    card = card
        .push(
            scrollable(list.width(Length::Fill))
                .spacing(4)
                .width(Length::Fill)
                .height(Length::Fixed(scaled(160.0))),
        )
        .push(one_line(review.line.clone().unwrap_or_default()))
        // Decide later first: Apply's label grows when armed and moves nothing.
        .push(
            Row::new()
                .spacing(10)
                .push(
                    button(text(t("review_later")).size(sz(13)))
                        .padding([6, 22])
                        .on_press(Message::ReviewLater)
                        .style(button::secondary),
                )
                .push(
                    button(text(apply).size(sz(13)))
                        .padding([6, 22])
                        .on_press(Message::ReviewApply)
                        .style(if review.armed {
                            button::danger
                        } else {
                            button::primary
                        }),
                ),
        );
    container(card)
        .max_width(560.0)
        .padding(20)
        .style(card_style)
        .into()
}
