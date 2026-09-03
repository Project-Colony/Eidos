//! The palette, and the container styles built on it.
//!
//! Every colour in this window comes from one [`ThemePalette`] - the Colony
//! ecosystem's 38-field palette shape, from `colony-ui`. Two things can fill it:
//!
//! * [`PARCHMENT`], the look Eidos has always worn, written out here as those
//!   same 38 fields rather than as scattered literals; and
//! * any of the **57 palettes** in the shared catalogue, 25 families generated
//!   from the design tokens in Project-Colony-Resources.
//!
//! Before this, the parchment was hard-coded in about seventy places across the
//! GUI and the theme setting did nothing at all: `theme(_app)` ignored its
//! argument, so the Light / Dark / System picker had never changed a pixel.
//!
//! **A literal hex outside this file is a bug.** It will be right on one palette
//! and wrong on the other fifty-seven.


use colony_ui::{hex, ThemePalette};
use iced::widget::container;
use iced::{Background, Border, Color, Element, Length, Theme};

use crate::{App, Message};

/// The key that means "Eidos's own", as stored in `settings.ini`.
///
/// Not a family in the shared catalogue: the catalogue is the ecosystem's, and
/// this parchment is this program's. Kept as the default so an upgrade changes
/// nobody's window - what a user sees today is what they keep.
pub(crate) const OWN_FAMILY: &str = "eidos";
pub(crate) const OWN_VARIANT: &str = "parchment";
pub(crate) const OWN_LABEL: &str = "Eidos";
pub(crate) const OWN_VARIANT_LABEL: &str = "Parchment";

/// Eidos's parchment, as the 38 fields every other palette also fills.
///
/// The values are the ones this window already used - the background, the card,
/// the burgundy, the muted brown, the two conflict tints - with the fields that
/// had no literal filled in from the same family so nothing reads as borrowed
/// from another theme.
pub(crate) const PARCHMENT: ThemePalette = ThemePalette {
    bg_primary: hex(0xECDFC2),
    bg_sidebar: hex(0xE3D6B6),
    bg_card: hex(0xF3EAD3),
    bg_card_hover: hex(0xEADDBF),
    bg_card_pressed: hex(0xE0D2B2),
    bg_selected: hex(0xCFB886),
    bg_input: hex(0xF7F0DE),
    bg_progress: hex(0xD8C9A6),

    text_primary: hex(0x2B2018),
    text_secondary: hex(0x4A3B2C),
    text_muted: hex(0x6A5A40),
    text_dim: hex(0x7C6C52),
    text_dimmer: hex(0x8E7E64),
    text_dimmest: hex(0xA09076),
    text_placeholder: hex(0xA89A80),

    accent_blue: hex(0x7A1F2B),
    accent_icon: hex(0x7A1F2B),
    accent_progress: hex(0x7A1F2B),

    btn_default: hex(0xE3D6B6),
    btn_hover: hex(0xEADDBF),
    btn_pressed: hex(0xD8C9A6),

    success: hex(0x216B29),
    success_bg: hex(0xC8DAB4),
    btn_success: hex(0x4A6B3A),
    btn_success_hover: hex(0x577E45),
    btn_success_pressed: hex(0x3D5930),

    warning: hex(0xB06A1E),
    warning_bg: hex(0xEDD9B4),

    error: hex(0x8A2A2A),
    error_light: hex(0x992929),
    error_bg: hex(0xEBC4BD),
    btn_danger_bg: hex(0x8A2A2A),
    btn_danger_hover: hex(0x9C3232),
    btn_trash_hover: hex(0x9C3232),
    btn_trash_pressed: hex(0x742222),

    bg_modal_section: hex(0xEFE5CC),
    border_subtle: hex(0xC9B890),
    divider: hex(0xC9B890),
};

/// The palette in force, before high contrast. Written at boot and whenever the
/// user picks, read on every style call.
///
/// A global for the same reason `colony-ui` uses one: a style closure inside
/// `iced` cannot reach `App`, and threading a palette through some seventy of
/// them is how one of the seventy ends up different.
/// One value process-wide.
#[cfg(not(test))]
mod store {
    use super::{ThemePalette, PARCHMENT};
    use iced::Color;
    use std::sync::RwLock;

    static ACTIVE: RwLock<ThemePalette> = RwLock::new(PARCHMENT);
    /// The user's accent override, or `None` for the palette's own.
    static ACCENT: RwLock<Option<Color>> = RwLock::new(None);
    static CONTRAST: RwLock<bool> = RwLock::new(false);

    pub(super) fn set(palette: ThemePalette, accent: Option<Color>, high_contrast: bool) {
        *ACTIVE.write().unwrap() = palette;
        *ACCENT.write().unwrap() = accent;
        *CONTRAST.write().unwrap() = high_contrast;
    }
    pub(super) fn active() -> ThemePalette {
        *ACTIVE.read().unwrap()
    }
    pub(super) fn accent() -> Option<Color> {
        *ACCENT.read().unwrap()
    }
    pub(super) fn contrast() -> bool {
        *CONTRAST.read().unwrap()
    }
}

/// One value per THREAD, under test only.
///
/// The harness runs tests in parallel threads inside a single process, so a
/// process-wide palette means any test that picks a theme repaints every other
/// test that is mid-assertion. It did: roughly one run in three failed, on a
/// different theme test each time, and the message was always about the colour
/// being wrong rather than about interference - which is what made it read as
/// several unrelated flaky tests instead of one shared cell.
///
/// Per-thread is not a weaker guarantee here, it is the accurate one: the
/// program has exactly one thread that draws, and the tests have one each.
#[cfg(test)]
mod store {
    use super::{ThemePalette, PARCHMENT};
    use iced::Color;
    use std::cell::Cell;

    thread_local! {
        static ACTIVE: Cell<ThemePalette> = const { Cell::new(PARCHMENT) };
        static ACCENT: Cell<Option<Color>> = const { Cell::new(None) };
        static CONTRAST: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn set(palette: ThemePalette, accent: Option<Color>, high_contrast: bool) {
        ACTIVE.with(|c| c.set(palette));
        ACCENT.with(|c| c.set(accent));
        CONTRAST.with(|c| c.set(high_contrast));
    }
    pub(super) fn active() -> ThemePalette {
        ACTIVE.with(Cell::get)
    }
    pub(super) fn accent() -> Option<Color> {
        ACCENT.with(Cell::get)
    }
    pub(super) fn contrast() -> bool {
        CONTRAST.with(Cell::get)
    }
}

/// Point the window at a theme.
///
/// An unknown family or variant resolves to the parchment rather than failing,
/// so a `settings.ini` written by a later version - or naming a family that has
/// since been removed upstream - degrades instead of stopping the program.
pub(crate) fn apply(family: &str, variant: &str, accent: Option<&str>, high_contrast: bool) {
    let resolved = if family == OWN_FAMILY {
        PARCHMENT
    } else {
        // `resolve` never fails: an unknown pair gives the catalogue's own
        // fallback. Checking membership first is what keeps an unknown family on
        // EIDOS's default rather than on somebody else's.
        if colony_ui::THEME_FAMILIES
            .iter()
            .any(|f| f.key == family && f.variants.iter().any(|v| v.key == variant))
        {
            colony_ui::resolve(family, variant)
        } else {
            PARCHMENT
        }
    };
    store::set(
        resolved,
        accent.and_then(colony_ui::accent_key_to_color),
        high_contrast,
    );

    // The shared catalogue's own globals, kept in step so anything drawn from
    // `colony_ui` agrees with what this file draws.
    colony_ui::set_high_contrast(high_contrast);
}

/// The palette to draw with, high contrast already applied.
pub(crate) fn pal() -> ThemePalette {
    let base = store::active();
    if store::contrast() {
        // Derived rather than shipped: no theme carries a high-contrast twin, so
        // the boost works on the parchment and on all 57 alike.
        base.with_high_contrast()
    } else {
        base
    }
}

/// The accent: the user's override if they picked one, else the palette's own.
pub(crate) fn accent() -> Color {
    store::accent().unwrap_or_else(|| pal().accent_blue)
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

pub(crate) fn row_bg(even: bool) -> Color {
    let p = pal();
    if even {
        p.bg_card
    } else {
        p.bg_card_hover
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
pub(crate) fn sel_bg() -> Color {
    pal().bg_selected
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
    let rivals = [p.bg_selected, p.success_bg, p.error_bg];
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

    /// The parchment must fill every field. A hole would read as transparent
    /// black on whatever it was used for, and only on the default theme.
    #[test]
    fn the_parchment_names_a_colour_for_every_field() {
        let p = PARCHMENT;
        for (name, c) in [
            ("bg_primary", p.bg_primary),
            ("bg_card", p.bg_card),
            ("bg_selected", p.bg_selected),
            ("text_primary", p.text_primary),
            ("text_muted", p.text_muted),
            ("accent_blue", p.accent_blue),
            ("success", p.success),
            ("success_bg", p.success_bg),
            ("error", p.error),
            ("error_bg", p.error_bg),
            ("warning", p.warning),
            ("divider", p.divider),
            ("border_subtle", p.border_subtle),
        ] {
            assert_eq!(c.a, 1.0, "{name} is not opaque");
        }
    }

    /// An upgrade must not repaint anybody's window: with nothing chosen, the
    /// parchment is what is drawn.
    #[test]
    fn the_default_is_the_parchment_this_program_has_always_worn() {
        apply(OWN_FAMILY, OWN_VARIANT, None, false);
        assert_eq!(pal().bg_primary, hex(0xECDFC2));
        assert_eq!(pal().bg_card, hex(0xF3EAD3));
        assert_eq!(accent(), hex(0x7A1F2B));
    }

    #[test]
    fn a_catalogue_theme_really_replaces_the_palette() {
        apply("gruvbox", "dark", None, false);
        assert_eq!(pal().bg_primary, hex(0x282828), "gruvbox dark did not take");
        assert_ne!(pal().bg_primary, PARCHMENT.bg_primary);

        // And back.
        apply(OWN_FAMILY, OWN_VARIANT, None, false);
        assert_eq!(pal().bg_primary, PARCHMENT.bg_primary);
    }

    /// A family this build has never heard of - a config from a later version,
    /// or one removed upstream - must land on EIDOS's default, not on the
    /// catalogue's, which would repaint the window for a typo.
    #[test]
    fn an_unknown_theme_degrades_to_the_parchment() {
        for (family, variant) in [
            ("no-such-family", "dark"),
            ("gruvbox", "no-such-variant"),
            ("", ""),
        ] {
            apply(family, variant, None, false);
            assert_eq!(
                pal().bg_primary,
                PARCHMENT.bg_primary,
                "{family}/{variant} did not degrade to the parchment"
            );
        }
    }

    #[test]
    fn an_accent_override_wins_over_the_palettes_own() {
        apply(OWN_FAMILY, OWN_VARIANT, Some("green"), false);
        assert_ne!(accent(), PARCHMENT.accent_blue, "the override was ignored");
        // An unknown accent key is not an accent: fall back to the theme's.
        apply(OWN_FAMILY, OWN_VARIANT, Some("chartreuse"), false);
        assert_eq!(accent(), PARCHMENT.accent_blue);
        // And none means the theme's own.
        apply(OWN_FAMILY, OWN_VARIANT, None, false);
        assert_eq!(accent(), PARCHMENT.accent_blue);
    }

    /// Derived from the active palette, not shipped as a twin - so it works on
    /// the parchment and on all 57 alike.
    #[test]
    fn high_contrast_moves_the_palette_and_is_reversible() {
        apply(OWN_FAMILY, OWN_VARIANT, None, false);
        let plain = pal();
        apply(OWN_FAMILY, OWN_VARIANT, None, true);
        let boosted = pal();

        // It moves the INK and the lines, not the grounds: a high-contrast mode
        // that repainted the backgrounds would be a different theme rather than
        // the same one read more easily.
        assert_ne!(
            plain.text_primary, boosted.text_primary,
            "the ink did not move"
        );
        assert_ne!(plain.divider, boosted.divider, "the lines did not move");
        assert_eq!(
            plain.bg_primary, boosted.bg_primary,
            "the ground must not move"
        );

        // On a light palette the ink gets darker, not lighter.
        assert!(boosted.text_primary.r < plain.text_primary.r);

        apply(OWN_FAMILY, OWN_VARIANT, None, false);
        assert_eq!(pal().text_primary, plain.text_primary);
    }

    /// The three list tints answer three different questions and must never be
    /// confusable - on ANY palette, which is what a fixed hex could not promise.
    #[test]
    fn the_list_tints_stay_distinguishable_on_every_palette() {
        let mut checked = 0;
        for family in colony_ui::THEME_FAMILIES {
            for variant in family.variants {
                apply(family.key, variant.key, None, false);
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
        // The real check is the distinctness loop above, which runs over
        // whatever the catalogue holds; this only guards against it silently
        // becoming empty.
        assert!(checked >= 57, "the catalogue shrank to {checked} palettes");
        apply(OWN_FAMILY, OWN_VARIANT, None, false);
    }

    /// The harness runs tests in parallel threads inside ONE process. With the
    /// palette in a process-wide global, any test that picked a theme repainted
    /// every other test mid-assertion: roughly one run in three failed, on a
    /// different theme test each time, which reads as "the boost never took
    /// effect" rather than as interference.
    #[test]
    fn two_threads_picking_themes_do_not_repaint_each_other() {
        use std::sync::{Arc, Barrier};
        // Both threads apply, THEN both read: with shared state the second
        // write lands before the first read and one of them sees the other's.
        let gate = Arc::new(Barrier::new(2));
        let mine = gate.clone();
        let other = std::thread::spawn(move || {
            apply("catppuccin", "frappe", None, false);
            mine.wait();
            pal()
        });
        apply(OWN_FAMILY, OWN_VARIANT, None, false);
        gate.wait();
        let here = pal();
        let there = other.join().unwrap();

        assert_eq!(here, PARCHMENT, "this thread kept the theme it picked");
        assert_ne!(there, PARCHMENT, "and the other kept its own");
    }
}
