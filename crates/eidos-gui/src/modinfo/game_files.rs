//! The Game files card at the top of Diagnostics: which game files the next
//! launch runs, and the one action that switches them.

use crate::theme::*;
use crate::widgets::*;
use crate::*;
use eidos::stock_game::GameFilesFacts;
use iced::widget::text::Wrapping;

/// What the card's one action slot holds.
#[derive(Debug, Clone)]
pub(crate) enum GameFilesSlot {
    Actions(Vec<(String, Message)>),
    /// Why no switch is offered.
    Hint(String),
}

/// The card's one detail line and its slot.
#[derive(Debug, Clone)]
pub(crate) struct GameFilesCard {
    pub(crate) detail: String,
    pub(crate) slot: GameFilesSlot,
}

/// "version 1.6.1170.0", or "version unknown".
pub(crate) fn game_files_version(v: Option<&str>) -> String {
    match v {
        Some(v) => tf("game_files_version", &[("v", &v)]),
        None => t("game_files_version_unknown").to_string(),
    }
}

/// The Game files card for `f`, the facts of the game named `game`.
pub(crate) fn game_files_card(f: &GameFilesFacts, game: &str) -> GameFilesCard {
    use GameFilesAction::*;
    use GameFilesSlot::*;
    let act = |label: &str, a: GameFilesAction| (label.to_string(), Message::GameFiles(a));
    let store_version = game_files_version(f.store_version.as_deref());
    let detail = match &f.files.copy {
        None => tf(
            "game_files_detail_store",
            &[
                ("version", &store_version),
                ("path", &f.files.root.display()),
            ],
        ),
        Some(folder) => tf(
            "game_files_detail_copy",
            &[
                ("folder", folder),
                ("version", &game_files_version(f.copy_version.as_deref())),
                ("store_version", &store_version),
            ],
        ),
    };
    let slot = if f.files.copy.is_some() {
        Actions(vec![act(t("game_files_use_store"), Store)])
    } else if !f.supported {
        Hint(tf("game_files_hint_unsupported", &[("game", &game)]))
    } else if f.unfinished {
        Actions(vec![act(t("game_files_delete_unfinished"), Delete)])
    } else if let Some(folder) = &f.frozen {
        Actions(vec![
            act(t("game_files_use_frozen"), Use(folder.clone())),
            act(t("game_files_delete_copy"), Delete),
        ])
    } else if let Some(folder) = &f.list_folder {
        let label = tf("game_files_use_folder", &[("folder", folder)]);
        Actions(vec![act(&label, Use(folder.clone()))])
    } else {
        Actions(vec![act(t("game_files_freeze"), Freeze)])
    };
    GameFilesCard { detail, slot }
}

/// The card: the detail is one clipped line and the slot one fixed-height row,
/// so the card keeps its height whatever it says (the #120 pattern).
pub(crate) fn game_files_card_view<'a>(card: &GameFilesCard) -> Element<'a, Message> {
    let line = |s: String| text(s).size(sz(10)).wrapping(Wrapping::None);
    let slot: Element<'a, Message> = match &card.slot {
        GameFilesSlot::Actions(actions) => actions
            .iter()
            .cloned()
            .fold(Row::new().spacing(6), |row, (label, msg)| {
                row.push(tool_btn(label, msg))
            })
            .into(),
        GameFilesSlot::Hint(hint) => line(hint.clone()).into(),
    };
    let foot = Column::new()
        .spacing(2)
        .push(
            container(line(card.detail.clone()).color(text_muted()))
                .width(Length::Fill)
                // One line of size-10 text: iced's default line height is 1.3 em.
                .height(Length::Fixed(sz(10) * 1.3))
                .clip(true),
        )
        .push(
            container(slot)
                .height(Length::Fixed(scaled(DIRTY_SLOT_H)))
                .clip(true),
        );
    diag_card(
        DiagLevel::Ok,
        t("game_files_title").to_string(),
        String::new(),
        Some(foot.into()),
    )
}
