//! The first-run wizard: welcome, instance kind, game, name and location, then
//! the summary that precedes creating the instance.
//!
//! Split out of `main.rs` unchanged. Each screen is a pure function of `App`;
//! the `Message`s they emit are handled where they always were.

use iced::widget::{button, container, scrollable, text_input, Column, Row, Space};
use iced::{Element, Length};

use colony_ui::sz;

use crate::theme::{card_style, text};
use crate::widgets::nav;
use crate::{planned_instance, selected_game, App, InstanceKind, Message};

pub(crate) fn frame<'a>(
    step: &'a str,
    title: &'a str,
    content: Element<'a, Message>,
    back: Option<Message>,
    next_label: &'a str,
    next_msg: Option<Message>,
) -> Element<'a, Message> {
    let header = Column::new()
        .spacing(2)
        .push(text("Eidos").size(sz(26)))
        .push(text(step).size(sz(12)));

    let card = container(content)
        .width(Length::Fill)
        .padding(18)
        .style(card_style);

    let footer = Row::new()
        .push(nav("Back", back, false))
        .push(Space::new().width(Length::Fill))
        .push(nav(next_label, next_msg, true));

    Column::new()
        .spacing(16)
        .push(header)
        .push(text(title).size(sz(20)))
        .push(card)
        .push(Space::new().height(Length::Fill))
        .push(footer)
        .into()
}

pub(crate) fn welcome(app: &App) -> Element<'_, Message> {
    let mut content = Column::new()
        .spacing(10)
        .push(text("Eidos creates an isolated modding setup for your game,").size(sz(15)))
        .push(text("mounting your mods over the game without touching its files.").size(sz(15)))
        .push(text("Let's set up an instance.").size(sz(13)));
    // Existing instances open with one click - this doubles as the instance
    // switcher (the toolbar's switch action lands here via Restart). Portable
    // roots come from the registry; without this list they were orphaned the
    // moment the app closed.
    if !app.known.is_empty() {
        let mut list = Column::new().spacing(6);
        for (i, k) in app.known.iter().enumerate() {
            list = list.push(
                button(text(k.label.clone()).size(sz(13)))
                    .width(Length::Fill)
                    .padding(8)
                    .on_press(Message::OpenKnown(i))
                    .style(button::secondary),
            );
        }
        content = content
            .push(Space::new().height(Length::Fixed(6.0)))
            .push(text("Or open an existing instance:").size(sz(13)))
            .push(scrollable(list).height(Length::Fixed(160.0)));
    }
    // OUTSIDE the `known` block above, deliberately: a machine that has just
    // been set up has no instances at all, and that is precisely the machine
    // somebody arrives at holding a .eidos file.
    content = content
        .push(Space::new().height(Length::Fixed(6.0)))
        .push(text("Or restore one you packed on another machine:").size(sz(13)))
        .push(
            button(text("Unpack a backup...").size(sz(13)))
                .width(Length::Fill)
                .padding(8)
                .on_press(Message::ShowUnpackDialog)
                .style(button::secondary),
        );
    frame(
        "Step 1 of 5",
        "Welcome",
        content.into(),
        None,
        "Next",
        Some(Message::Next),
    )
}

pub(crate) fn kind_card<'a>(
    label: &'a str,
    desc: &'a str,
    selected: bool,
    msg: Message,
) -> Element<'a, Message> {
    let inner = Column::new()
        .spacing(4)
        .push(text(label).size(sz(16)))
        .push(text(desc).size(sz(12)));
    button(inner)
        .width(Length::Fill)
        .padding(12)
        .on_press(msg)
        .style(if selected {
            button::primary
        } else {
            button::secondary
        })
        .into()
}

pub(crate) fn kind_screen<'a>(app: &App) -> Element<'a, Message> {
    let content = Column::new()
        .spacing(10)
        .push(kind_card(
            "Global",
            "Stored centrally in ~/.local/share/Colony/Eidos/instances, managed by Eidos. Recommended.",
            app.kind == InstanceKind::Global,
            Message::PickKind(InstanceKind::Global),
        ))
        .push(kind_card(
            "Portable",
            "A self-contained folder you choose. Movable and isolated.",
            app.kind == InstanceKind::Portable,
            Message::PickKind(InstanceKind::Portable),
        ));
    frame(
        "Step 2 of 5",
        "Instance type",
        content.into(),
        Some(Message::Back),
        "Next",
        Some(Message::Next),
    )
}

pub(crate) fn game_screen<'a>(app: &App) -> Element<'a, Message> {
    let content: Element<Message> = if app.games.is_empty() {
        Column::new()
            .push(text("No supported games detected.").size(sz(15)))
            .push(
                text(
                    "Install a supported game via Steam, Heroic or Legendary, then restart Eidos.",
                )
                .size(sz(12)),
            )
            .into()
    } else {
        let mut list = Column::new().spacing(6);
        for (i, g) in app.games.iter().enumerate() {
            list = list.push(
                button(
                    text(format!(
                        "{}  ({})\n{}",
                        g.def.name,
                        g.source_name(),
                        g.install_path.display()
                    ))
                    .size(sz(14)),
                )
                .width(Length::Fill)
                .padding(10)
                .on_press(Message::PickGame(i))
                .style(if app.selected == Some(i) {
                    button::primary
                } else {
                    button::secondary
                }),
            );
        }
        scrollable(list).height(Length::Fixed(240.0)).into()
    };
    let next = app.selected.map(|_| Message::Next);
    frame(
        "Step 3 of 5",
        "Choose the game to mod",
        content,
        Some(Message::Back),
        "Next",
        next,
    )
}

pub(crate) fn nameloc_screen<'a>(app: &App) -> Element<'a, Message> {
    let mut content = Column::new()
        .spacing(8)
        .push(text("Instance name").size(sz(13)))
        .push(
            text_input("My Skyrim setup", &app.name)
                .on_input(Message::NameChanged)
                .padding(8),
        );
    if app.kind == InstanceKind::Portable {
        content = content.push(text("Portable folder").size(sz(13))).push(
            text_input("~/Eidos/skyrimse", &app.portable_path)
                .on_input(Message::PortableChanged)
                .padding(8),
        );
    }
    let next = (!app.name.trim().is_empty()).then_some(Message::Next);
    frame(
        "Step 4 of 5",
        "Name and location",
        content.into(),
        Some(Message::Back),
        "Next",
        next,
    )
}

pub(crate) fn summary_screen<'a>(app: &App) -> Element<'a, Message> {
    let kind = match app.kind {
        InstanceKind::Global => "Global",
        InstanceKind::Portable => "Portable",
    };
    let game = selected_game(app);
    let location = planned_instance(app)
        .map(|i| i.root.display().to_string())
        .unwrap_or_default();

    let mut content = Column::new()
        .spacing(8)
        .push(text(format!("Name:     {}", app.name)).size(sz(14)))
        .push(text(format!("Type:     {kind}")).size(sz(14)))
        .push(
            text(format!(
                "Game:     {}",
                game.map(|g| g.def.name).unwrap_or("(none)")
            ))
            .size(sz(14)),
        )
        .push(text(format!("Location: {location}")).size(sz(13)));
    if let Some(g) = game {
        content = content.push(text(format!("Game data: {}", g.data_path.display())).size(sz(12)));
    }
    // Say when Finish will ADOPT rather than create: the folder already
    // declares this game, and nothing in it will be overwritten. (A folder
    // declaring ANOTHER game is refused by the Finish handler, which puts the
    // reason in `app.error` below.)
    let adopts = planned_instance(app)
        .and_then(|i| i.read_manifest())
        .is_some_and(|m| game.map(|g| g.def.id) == Some(m.game_id.as_str()));
    if adopts {
        content = content.push(
            text("This folder already holds an instance for this game - it will be opened as-is.")
                .size(sz(12)),
        );
    }
    if let Some(err) = &app.error {
        content = content.push(text(format!("Error: {err}")).size(sz(13)));
    }
    let next_label = if adopts {
        "Open instance"
    } else {
        "Create instance"
    };
    frame(
        "Step 5 of 5",
        "Review and create",
        content.into(),
        Some(Message::Back),
        next_label,
        Some(Message::Finish),
    )
}
