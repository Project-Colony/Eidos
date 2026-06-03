//! Eidos GUI (iced) - first window.
//!
//! Shows the supported games detected on this system (left) and the selected
//! game's instance details + launch option (right), in the Colony parchment /
//! burgundy look. This is the visual front for the same `eidos-games` detection
//! the CLI uses; instance editing and the mod list come next.
//!
//! Run it yourself with: `cargo run -p eidos-gui`

use iced::widget::{button, container, scrollable, text, Column, Row};
use iced::{Background, Border, Color, Element, Length, Task, Theme};

use eidos_games::{detect, home, DetectedGame};

#[derive(Debug, Clone)]
enum Message {
    Select(usize),
}

struct App {
    games: Vec<DetectedGame>,
    selected: Option<usize>,
}

fn new() -> (App, Task<Message>) {
    (
        App {
            games: detect(&home()),
            selected: None,
        },
        Task::none(),
    )
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Select(i) => app.selected = Some(i),
    }
    Task::none()
}

/// The Colony parchment + burgundy palette.
fn palette() -> iced::theme::Palette {
    iced::theme::Palette {
        background: Color::from_rgb8(0xEC, 0xDF, 0xC2), // parchment
        text: Color::from_rgb8(0x2B, 0x20, 0x18),       // ink
        primary: Color::from_rgb8(0x7A, 0x1F, 0x2B),    // burgundy
        success: Color::from_rgb8(0x4A, 0x6B, 0x3A),
        danger: Color::from_rgb8(0x8A, 0x2A, 0x2A),
    }
}

fn theme(_app: &App) -> Theme {
    Theme::custom("Eidos".to_string(), palette())
}

/// A bordered parchment "card" for the detail panel.
fn card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
        border: Border {
            color: Color::from_rgb8(0x7A, 0x1F, 0x2B),
            width: 1.5,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn view(app: &App) -> Element<'_, Message> {
    // Left: the detected-games list.
    let mut list = Column::new()
        .spacing(6)
        .push(text("Games").size(20.0));
    if app.games.is_empty() {
        list = list.push(
            text("No supported games detected.\nInstall a supported game via Steam.").size(13.0),
        );
    } else {
        for (i, game) in app.games.iter().enumerate() {
            let row = button(text(game.def.name).size(15.0))
                .width(Length::Fill)
                .on_press(Message::Select(i))
                .style(if Some(i) == app.selected {
                    button::primary
                } else {
                    button::secondary
                });
            list = list.push(row);
        }
    }
    let left = container(scrollable(list))
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .padding(8);

    // Right: details for the selected game.
    let detail = match app.selected.and_then(|i| app.games.get(i)) {
        Some(game) => {
            let proton = match &game.compatdata {
                Some(p) => format!("Proton prefix: {}", p.display()),
                None => "Proton prefix: (none yet)".to_string(),
            };
            Column::new()
                .spacing(8)
                .push(text(game.def.name).size(24.0))
                .push(text(format!("Steam: {}", game.steam_name)).size(13.0))
                .push(text(format!("Install:  {}", game.install_path.display())).size(12.0))
                .push(text(format!("Data dir: {}", game.data_path.display())).size(12.0))
                .push(text(proton).size(12.0))
                .push(
                    text(format!(
                        "Steam launch option:\n    eidos play {} -- %command%",
                        game.def.id
                    ))
                    .size(13.0),
                )
        }
        None => Column::new().push(text("Select a game on the left.").size(16.0)),
    };
    let right = container(detail)
        .width(Length::FillPortion(3))
        .height(Length::Fill)
        .padding(16)
        .style(card_style);

    let header = Column::new()
        .spacing(2)
        .push(text("Eidos").size(30.0))
        .push(text("native Linux mod manager").size(13.0));

    let body = Row::new().spacing(12).push(left).push(right);

    container(Column::new().spacing(12).padding(16).push(header).push(body))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn main() -> iced::Result {
    iced::application("Eidos", update, view)
        .theme(theme)
        .run_with(new)
}
