//! The reusable pieces the main window is drawn from: floating cards, drop gaps,
//! list rows, the icon buttons, and the conflict legend that explains the row
//! tints.
//!
//! Split out of `main.rs` unchanged. Everything here builds an `Element` and
//! hands it back; none of it decides anything.

use std::collections::HashMap;

use iced::widget::{button, container, image, mouse_area, text, Column, Row, Space};
use iced::{Background, Border, Color, Element, Length, Theme};

use crate::theme::{row_bg, CONFLICT_LOSES_BG, CONFLICT_WINS_BG, SEL_BG};
use crate::{App, Message};

/// A thin strip beside the scrollbar marking WHERE the conflicting mods are, in
/// the same green/red as the row tints.
///
/// MO2 draws these marks on the scrollbar itself. The point is the same: with a
/// few hundred mods, "this one is overwritten" is useless if finding the culprit
/// means scrolling the whole list looking for a tinted row.
///
/// `tints` is one entry per DRAWN row, in order, so a position on the strip is
/// the same fraction of the list as the scrollbar's. Runs of the same colour
/// collapse into one widget - a list is mostly untinted, so this is a handful of
/// containers rather than one per mod.
///
/// Meant to be stacked over the scrollbar, not placed next to it. Nothing here
/// handles events, so the scrollbar underneath still takes the pointer.
pub(crate) fn conflict_map<'a>(tints: &[Option<Color>]) -> Element<'a, Message> {
    // Nothing to point at: take no width at all rather than leave a dead gutter.
    if tints.is_empty() || tints.iter().all(Option::is_none) {
        return Space::new().width(Length::Fixed(0.0)).into();
    }
    let mut col = Column::new().width(Length::Fixed(CONFLICT_MAP_W)).height(Length::Fill);
    let mut i = 0;
    while i < tints.len() {
        let tint = tints[i];
        let start = i;
        while i < tints.len() && tints[i] == tint {
            i += 1;
        }
        let run = (i - start) as u16;
        col = col.push(
            container(Space::new().width(Length::Fill).height(Length::Fill))
                .height(Length::FillPortion(run))
                .style(move |_t: &Theme| container::Style {
                    background: tint.map(Background::Color),
                    ..Default::default()
                }),
        );
    }
    col.into()
}

/// Width of the conflict strip: iced's default scrollbar width, because the
/// strip is laid OVER the scrollbar rather than beside it. Beside it, the marks
/// pushed the whole list sideways to make room - visible, and ugly, for
/// something that is meant to be a hint.
pub(crate) const CONFLICT_MAP_W: f32 = 10.0;

/// Place a floating card with one corner at `at`, growing away from the nearest
/// window edge.
///
/// The card's height is not known until it is laid out, so a menu summoned near
/// the bottom cannot simply be offset downwards - it would run off the screen.
/// Anchoring the BOTTOM edge to the pointer instead, and mirroring the same
/// trick horizontally, avoids ever needing to guess the size: the container
/// aligns the card and the padding does the positioning.
pub(crate) fn floating_at<'a>(
    card: Element<'a, Message>,
    at: iced::Point,
    win: iced::Size,
) -> Element<'a, Message> {
    // Past the halfway line the menu would head towards an edge, so flip it.
    let right = at.x > win.width * 0.5;
    let below = at.y > win.height * 0.5;
    let pad = iced::Padding {
        top: if below { 0.0 } else { at.y },
        bottom: if below { (win.height - at.y).max(0.0) } else { 0.0 },
        left: if right { 0.0 } else { at.x },
        right: if right { (win.width - at.x).max(0.0) } else { 0.0 },
    };
    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(pad)
        .align_x(if right {
            iced::alignment::Horizontal::Right
        } else {
            iced::alignment::Horizontal::Left
        })
        .align_y(if below {
            iced::alignment::Vertical::Bottom
        } else {
            iced::alignment::Vertical::Top
        })
        .into()
}

/// The strip that explains the two conflict colours, and how many rows carry
/// each - `None` when the focused mod fights with nothing, or nothing is
/// focused.
pub(crate) fn conflict_legend<'a>(app: &App) -> Option<Element<'a, Message>> {
    let focus = app.selected_mod?;
    let me = app.conflicts.as_ref()?.mods.get(&((focus + 1) as u32))?;
    // Origin 0 is the game's own data and u32::MAX is the Overwrite layer;
    // neither is a row, so neither is counted here.
    let rows = |set: &std::collections::BTreeSet<u32>| {
        set.iter().filter(|&&o| o != 0 && o != u32::MAX).count()
    };
    let (over, under) = (rows(&me.overwrites), rows(&me.overwritten_by));
    if over == 0 && under == 0 {
        return None;
    }
    let swatch = |c: Color, label: String| -> Element<'a, Message> {
        Row::new()
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .push(
                container(Space::new().width(Length::Fixed(12.0)).height(Length::Fixed(12.0)))
                    .style(move |_t: &Theme| container::Style {
                        background: Some(Background::Color(c)),
                        border: Border { color: Color::from_rgb8(0x6E, 0x24, 0x2E), width: 1.0, radius: 2.0.into() },
                        ..Default::default()
                    }),
            )
            .push(text(label).size(11.0))
            .into()
    };
    let name = app.mods.get(focus).map(|m| m.display_name().to_string()).unwrap_or_default();
    let mut row = Row::new().spacing(10).align_y(iced::Alignment::Center);
    row = row.push(text(format!("{name} conflicts:")).size(11.0));
    if over > 0 {
        row = row.push(swatch(CONFLICT_WINS_BG, format!("{over} it overwrites")));
    }
    if under > 0 {
        row = row.push(swatch(CONFLICT_LOSES_BG, format!("{under} overwrite it")));
    }
    Some(row.into())
}

/// How the row at `i` relates to the focused mod, for painting.
///
/// MO2's whole conflict workflow is this: click a mod, and every mod it fights
/// with lights up so the stack can be read at a glance instead of by opening a
/// dialog per pair. Both directions are shown, because "who am I beating" and
/// "who is beating me" are different questions and the answer to the second is
/// what sends a texture pack to the bottom of the list.
///
/// `None` for the focused row itself, which already reads as selected.
pub(crate) fn conflict_tint(app: &App, i: usize) -> Option<Color> {
    let focus = app.selected_mod?;
    if focus == i {
        return None;
    }
    let map = app.conflicts.as_ref()?;
    // Origins are `index + 1`; 0 is the game's own data.
    let me = map.mods.get(&((focus + 1) as u32))?;
    let other = (i + 1) as u32;
    if me.overwrites.contains(&other) {
        Some(CONFLICT_WINS_BG)
    } else if me.overwritten_by.contains(&other) {
        Some(CONFLICT_LOSES_BG)
    } else {
        None
    }
}

/// A mod-list row background that also reflects selection (MO2's blue highlight,
/// here a parchment-tan so it reads on the burgundy theme).
/// The height of the insertion strip between two rows. Rendered ALWAYS, not only
/// during a drag, so the list does not jump when one starts - on a 100-mod list,
/// making the strips appear on grab shifted everything below by hundreds of
/// pixels and the pointer ended up over a completely different row. It replaces
/// the list's old 1px spacing, so the real cost is 3px per row, and it gives the
/// dense view the breathing room it needed anyway.
pub(crate) const GAP_H: f32 = 4.0;

/// An insertion point between two rows: the drop target for index `gap`, drawn as
/// a burgundy line while it is the live target.
///
/// This is what replaced a border around the hovered ROW. A border says "this row
/// is involved" and leaves the user guessing which side; a line in the gap says
/// exactly where the block lands, which is the whole point of aiming. MO2 draws
/// the same indicator, and its geometry is why: the strip IS the destination, so
/// there is nothing to infer.
/// `interactive` is false when no drag is in flight, and for the strips above the
/// game's own content (which nothing may be ordered above). A non-interactive
/// strip is pure spacing: no `mouse_area`, so idly moving the pointer down a
/// 100-row list does not fire a hover message per strip and rebuild the view
/// each time.
///
/// Both reorderable lists render through this, so a drag reads and aims the same
/// way in the mod list and the plugin list; only the messages differ.
pub(crate) fn drop_gap<'a>(
    gap: usize,
    active: bool,
    interactive: bool,
    over: fn(usize) -> Message,
    drop: Message,
) -> Element<'a, Message> {
    let bar = container(Space::new().width(Length::Fill).height(Length::Fixed(if active { 2.0 } else { 0.0 })))
        .width(Length::Fill)
        .style(move |_t: &Theme| container::Style {
            background: active.then(|| Background::Color(Color::from_rgb8(0x6E, 0x24, 0x2E))),
            ..Default::default()
        });
    // `center_y(len)` is `height(len) + align`, so passing Fill here silently
    // REPLACED the fixed height: every strip then demanded the whole viewport,
    // the rows were squeezed to nothing and the list rendered blank mid-drag.
    // The height is fixed once, and the alignment is set without touching it.
    let strip = container(bar)
        .width(Length::Fill)
        .height(Length::Fixed(GAP_H))
        .align_y(iced::alignment::Vertical::Center);
    if !interactive {
        return strip.into();
    }
    mouse_area(strip).on_enter(over(gap)).on_release(drop).into()
}

/// The colour behind a mod row.
///
/// Selection outranks the conflict tint: the focused row is where the user's
/// attention already is, and losing its highlight to a colour that describes
/// OTHER rows would be a step backwards.
///
/// Its own function because the NAME CELL needs the same answer - it fades its
/// overflow into this colour, and a fade into the wrong one is a smear.
pub(crate) fn row_background(even: bool, selected: bool, conflict: Option<Color>) -> Color {
    if selected {
        SEL_BG
    } else {
        conflict.unwrap_or_else(|| row_bg(even))
    }
}

/// Every mod row is exactly this tall, whatever its name.
///
/// iced wraps text by default, so a long mod name became two or three lines and
/// that row grew with it - a list of uneven rows, which is harder to scan and
/// makes a drag land somewhere other than where it looked.
pub(crate) const MOD_ROW_H: f32 = 21.0;
/// How much of the name cell the fade covers. Wide enough to read as a fade
/// rather than a hard edge, narrow enough not to dim a name that fits.
pub(crate) const NAME_FADE_W: f32 = 26.0;

pub(crate) fn list_row<'a>(
    content: Element<'a, Message>,
    even: bool,
    selected: bool,
    conflict: Option<Color>,
) -> Element<'a, Message> {
    let bg = row_background(even, selected, conflict);
    container(content)
        .width(Length::Fill)
        .padding(2)
        .style(move |_t: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            ..Default::default()
        })
        .into()
}

// ---- shared widgets ----------------------------------------------------------

pub(crate) fn nav<'a>(label: &'a str, msg: Option<Message>, primary: bool) -> Element<'a, Message> {
    let mut b = button(text(label).size(13.0)).padding(8);
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    if primary {
        b.style(button::primary).into()
    } else {
        b.style(button::secondary).into()
    }
}

pub(crate) fn tool_btn<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12.0)).padding(6).on_press(msg).style(button::secondary).into()
}

/// A flat, menu/toolbar-style button (no chrome until hovered).
pub(crate) fn flat_btn<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(13.0)).padding(6).on_press(msg).style(button::text).into()
}

/// The decoded handle for each icon, made ONCE and handed out by address.
///
/// `image::Handle::from_bytes` stamps every handle with `Id::unique()`, so
/// building one per call meant every icon was a brand-new image to the renderer
/// on every view rebuild - a fresh texture upload per icon per frame, plus a
/// `to_vec` copy of the PNG bytes to go with it. That stayed invisible while the
/// view only rebuilt on a click or a hover transition. Tracking the pointer for
/// context-menu placement made it rebuild on every mouse MOVE, and the cache
/// thrashing showed up as icons and text flickering as the pointer travelled.
///
/// Keyed by the address of the `&'static [u8]`, which is stable and unique per
/// icon constant - the bytes themselves are never copied again.
pub(crate) static ICON_HANDLES: std::sync::LazyLock<std::sync::Mutex<HashMap<usize, image::Handle>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

pub(crate) fn icon<'a>(bytes: &'static [u8], size: f32) -> Element<'a, Message> {
    let handle = {
        let mut cache = ICON_HANDLES.lock().unwrap_or_else(|p| p.into_inner());
        cache
            .entry(bytes.as_ptr() as usize)
            .or_insert_with(|| image::Handle::from_bytes(bytes))
            .clone()
    };
    image(handle).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

/// A flat toolbar button: icon + label (MO2's icons-and-text mode).
pub(crate) fn icon_text_btn<'a>(bytes: &'static [u8], label: &'a str, msg: Message) -> Element<'a, Message> {
    let content = Row::new()
        .spacing(5)
        .push(icon(bytes, 16.0))
        .push(text(label).size(12.0));
    button(content).padding(5).on_press(msg).style(button::text).into()
}

/// A flat icon-only button (toolbar right group, row arrows).
pub(crate) fn icon_btn<'a>(bytes: &'static [u8], size: f32, msg: Option<Message>) -> Element<'a, Message> {
    let mut b = button(icon(bytes, size)).padding(3).style(button::text);
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    b.into()
}
