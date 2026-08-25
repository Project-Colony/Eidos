//! The Colony parchment palette and the container styles built on it.
//!
//! Split out of `main.rs` unchanged. These are leaves: called from everywhere,
//! calling nothing back, which is what made them the first thing worth moving
//! out of a 13k-line file.

use iced::widget::container;
use iced::{Background, Border, Color, Element, Length, Theme};

use crate::{App, Message};

pub(crate) fn palette() -> iced::theme::Palette {
    iced::theme::Palette {
        background: Color::from_rgb8(0xEC, 0xDF, 0xC2),
        text: Color::from_rgb8(0x2B, 0x20, 0x18),
        primary: Color::from_rgb8(0x7A, 0x1F, 0x2B),
        success: Color::from_rgb8(0x4A, 0x6B, 0x3A),
        // New in iced 0.14, and it has to sit between the green of success and
        // the deep red of danger without reading as either: a burnt amber that
        // belongs to the same parchment family.
        warning: Color::from_rgb8(0xB0, 0x6A, 0x1E),
        danger: Color::from_rgb8(0x8A, 0x2A, 0x2A),
    }
}

pub(crate) fn theme(_app: &App) -> Theme {
    Theme::custom("Eidos".to_string(), palette())
}

pub(crate) fn card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
        border: Border { color: Color::from_rgb8(0x7A, 0x1F, 0x2B), width: 1.5, radius: 8.0.into() },
        ..Default::default()
    }
}

pub(crate) fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
        border: Border { color: Color::from_rgb8(0x7A, 0x1F, 0x2B), width: 1.0, radius: 3.0.into() },
        ..Default::default()
    }
}

pub(crate) fn bar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xE3, 0xD6, 0xB6))),
        border: Border { color: Color::from_rgb8(0xC9, 0xB8, 0x90), width: 1.0, radius: 0.0.into() },
        ..Default::default()
    }
}

/// The bar between the two panes at rest: the same muted line the toolbars use,
/// so it reads as furniture rather than as content.
pub(crate) const DIVIDER: Color = Color::from_rgb8(0xC9, 0xB8, 0x90);

/// The same bar while it is being dragged - the panel border's burgundy, which is
/// the strongest colour in this palette and the one already used for "this is the
/// edge of something".
pub(crate) const DIVIDER_HELD: Color = Color::from_rgb8(0x7A, 0x1F, 0x2B);

/// Secondary text: a description under a title, a caption, a hint.
///
/// The convention asks for descriptions in `text_muted` rather than in the body
/// colour, so that a page of settings reads as titles with explanations under
/// them instead of as two columns of equally loud text. This is the parchment
/// palette's version of it - the same brown as the body ink, lightened until it
/// recedes without becoming hard to read.
pub(crate) const TEXT_MUTED: Color = Color::from_rgb8(0x6A, 0x5A, 0x40);

pub(crate) fn row_bg(even: bool) -> Color {
    if even {
        Color::from_rgb8(0xF3, 0xEA, 0xD3)
    } else {
        Color::from_rgb8(0xEA, 0xDD, 0xBF)
    }
}

/// Wrap a row with an alternating background (MO2-style row striping).
pub(crate) fn striped<'a>(content: Element<'a, Message>, even: bool) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .padding(2)
        .style(move |_t: &Theme| container::Style {
            background: Some(Background::Color(row_bg(even))),
            ..Default::default()
        })
        .into()
}

/// The highlight behind the selected mod row.
pub(crate) const SEL_BG: Color = Color::from_rgb(0.812, 0.722, 0.525); // tan, distinct from the stripes

/// A plugin that comes FROM the mod selected in the mod list (MO2 highlights
/// the same relationship). Blue on purpose: it must not be mistaken for the
/// selection tan, nor for the green/red of the conflict tints, because it
/// answers a different question - not "who wins", but "who ships this".
pub(crate) const ORIGIN_BG: Color = Color::from_rgb(0.796, 0.851, 0.898);

/// A mod the focused one OVERWRITES: it sits lower in the list and wins the
/// files they share. Green - the focused mod is on top of these.
pub(crate) const CONFLICT_WINS_BG: Color = Color::from_rgb(0.784, 0.855, 0.706);
/// A mod that overwrites the focused one: it sits lower and takes those files
/// away. Red - the focused mod is losing to these.
pub(crate) const CONFLICT_LOSES_BG: Color = Color::from_rgb(0.921, 0.769, 0.741);
/// The same two meanings as text, dark enough to read on parchment.
pub(crate) const CONFLICT_WINS_FG: Color = Color::from_rgb(0.13, 0.42, 0.16);
pub(crate) const CONFLICT_LOSES_FG: Color = Color::from_rgb(0.60, 0.16, 0.16);
