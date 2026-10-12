//! The palette, and the container styles built on it.
//!
//! Every colour in this window comes from the active [`ThemePalette`] of the
//! shared Colony catalogue: 25 families, 57 palettes, generated from the design
//! tokens in Project-Colony-Resources and set through `colony_ui`
//! ([`crate::preferences::UiPreferences`] applies the user's choice). Eidos's
//! parchment is the catalogue's `parchment/light`.
//!
//! **A literal hex outside the catalogue is a bug.** It will be right on one
//! palette and wrong on the other fifty-six.

use colony_ui::ThemePalette;
use iced::widget::container;
use iced::{Background, Border, Color, Element, Length, Theme};

use crate::{App, Message};

/// The palette to draw with, high contrast already applied.
pub(crate) fn pal() -> ThemePalette {
    colony_ui::active_palette()
}

/// The accent: the user's override if they picked one, else the palette's own.
pub(crate) fn accent() -> Color {
    colony_ui::effective_accent()
}

pub(crate) fn palette() -> iced::theme::Palette {
    let p = pal();
    iced::theme::Palette {
        background: p.bg_primary,
        text: p.text_primary,
        primary: accent(),
        success: p.success,
        // Sits between success and danger without reading as either.
        warning: p.warning,
        danger: p.error,
    }
}

pub(crate) fn theme(_app: &App) -> Theme {
    Theme::custom("Eidos".to_string(), palette())
}

pub(crate) fn card_style(_theme: &Theme) -> container::Style {
    let p = pal();
    container::Style {
        background: Some(Background::Color(p.bg_card)),
        border: Border {
            color: accent(),
            width: 1.5,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn panel_style(_theme: &Theme) -> container::Style {
    let p = pal();
    container::Style {
        background: Some(Background::Color(p.bg_card)),
        border: Border {
            color: accent(),
            width: 1.0,
            radius: 3.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn bar_style(_theme: &Theme) -> container::Style {
    let p = pal();
    container::Style {
        background: Some(Background::Color(p.bg_sidebar)),
        border: Border {
            color: p.border_subtle,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

/// The bar between the two panes at rest: the same muted line the toolbars use,
/// so it reads as furniture rather than as content.
pub(crate) fn divider() -> Color {
    pal().divider
}

/// The same bar while it is being dragged - the accent, which is the strongest
/// colour in any palette and already means "this is the edge of something".
pub(crate) fn divider_held() -> Color {
    accent()
}

/// Secondary text: a description under a title, a caption, a hint.
pub(crate) fn text_muted() -> Color {
    pal().text_muted
}

/// The stripe behind a list row. The odd rows take a faint wash of the ink
/// rather than `bg_card_hover`, which is what a hovered row looks like.
pub(crate) fn row_bg(even: bool) -> Color {
    let p = pal();
    if even {
        p.bg_card
    } else {
        mix(p.bg_card, p.text_primary, 0.05)
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

/// The highlight behind a selected row: the accent, as in every Colony list.
pub(crate) fn sel_bg() -> Color {
    accent()
}

/// Text on [`sel_bg`]: `text_primary` is illegible on an accent fill on most
/// palettes.
pub(crate) fn sel_fg() -> Color {
    colony_ui::contrast_on(sel_bg())
}

/// The darkest colour the palette has, the ink on a light theme and the ground
/// on a dark one, at `alpha`: what a shadow or a scrim under a card is made of.
pub(crate) fn shade(alpha: f32) -> Color {
    let p = pal();
    let weight = |c: Color| c.r + c.g + c.b;
    let ink = if weight(p.text_primary) < weight(p.bg_primary) {
        p.text_primary
    } else {
        p.bg_primary
    };
    Color { a: alpha, ..ink }
}

/// The background of a hovered row or menu entry.
pub(crate) fn hover_bg() -> Color {
    pal().bg_card_hover
}

/// A plugin that comes FROM the mod selected in the mod list (MO2 highlights the
/// same relationship). It answers a different question from the conflict tints -
/// not "who wins", but "who ships this" - so it must not be mistaken for either
/// of them, nor for the selection.
///
/// It used to be a fixed pale blue, which cannot survive 57 palettes. It is now
/// the card tinted towards the accent - so it belongs to the theme rather than
/// sitting on top of it.
///
/// The strength of the tint is CHOSEN, not fixed. On several palettes the
/// selection is itself an accent-tinted card, and a fixed ratio landed on top of
/// it - on `catppuccin/frappe` the two were 0.02 apart, which is to say
/// identical. So five strengths are measured against the three tints this must
/// never be confused with, and the one that stays furthest from all of them
/// wins. Fifteen subtractions per call, and it makes the guarantee hold on every
/// palette instead of on most of them.
pub(crate) fn origin_bg() -> Color {
    let p = pal();
    let rivals = [sel_bg(), p.success_bg, p.error_bg];
    let gap = |a: Color, b: Color| (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs();

    let mut best = mix(p.bg_card, p.accent_icon, 0.28);
    let mut best_gap = -1.0;
    for r in [0.28, 0.42, 0.56, 0.70, 0.84] {
        let c = mix(p.bg_card, p.accent_icon, r);
        let worst = rivals.iter().fold(f32::MAX, |m, v| m.min(gap(c, *v)));
        if worst > best_gap {
            best_gap = worst;
            best = c;
        }
    }
    best
}

/// A mod the focused one OVERWRITES: it sits lower in the list and wins the
/// files they share. The palette's success tint - the focused mod is on top.
pub(crate) fn conflict_wins_bg() -> Color {
    pal().success_bg
}
/// A mod that overwrites the focused one: it takes those files away.
pub(crate) fn conflict_loses_bg() -> Color {
    pal().error_bg
}
/// The same two meanings as text.
pub(crate) fn conflict_wins_fg() -> Color {
    pal().success
}
pub(crate) fn conflict_loses_fg() -> Color {
    pal().error
}

/// Blend two colours. Shared with `anim`, which needs the same operation for a
/// different reason.
fn mix(from: Color, to: Color, t: f32) -> Color {
    crate::anim::mix(from, to, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A family this build has never heard of - a file from a later version, or
    /// one removed upstream - degrades to the catalogue's fallback instead of
    /// failing the start.
    #[test]
    fn an_unknown_theme_degrades_to_the_fallback_palette() {
        let _look = crate::test_support::look_lock();
        for (family, variant) in [("no-such-family", "dark"), ("gruvbox", "no-such-variant"), ("", "")] {
            colony_ui::set_active_theme(family, variant);
            assert_eq!(pal(), colony_ui::FALLBACK_PALETTE, "{family}/{variant}");
        }
    }

    #[test]
    fn the_parchment_is_in_the_catalogue() {
        let _look = crate::test_support::look_lock();
        let prefs = crate::preferences::UiPreferences::default().standard;
        let (family, variant) = (prefs.selected_theme, prefs.selected_variant);
        colony_ui::set_active_theme(&family, &variant);
        assert_ne!(pal(), colony_ui::FALLBACK_PALETTE, "{family}/{variant} is not in the catalogue");
    }

    /// The list tints answer different questions and must never be confusable,
    /// on ANY palette, which is what a fixed hex could not promise.
    #[test]
    fn the_list_tints_stay_distinguishable_on_every_palette() {
        let _look = crate::test_support::look_lock();
        let mut checked = 0;
        for family in colony_ui::THEME_FAMILIES {
            for variant in family.variants {
                colony_ui::set_active_theme(family.key, variant.key);
                let tints = [
                    ("selection", sel_bg()),
                    ("origin", origin_bg()),
                    ("wins", conflict_wins_bg()),
                    ("loses", conflict_loses_bg()),
                ];
                for (i, (an, a)) in tints.iter().enumerate() {
                    for (bn, b) in tints.iter().skip(i + 1) {
                        let d = (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs();
                        assert!(
                            d > 0.04,
                            "{}/{}: {an} and {bn} are indistinguishable ({d})",
                            family.key,
                            variant.key
                        );
                    }
                }
                checked += 1;
            }
        }
        // The real check is the loop above; this only guards against the
        // catalogue silently becoming empty.
        assert!(checked >= 57, "the catalogue shrank to {checked} palettes");
    }
}
