//! The reusable pieces the main window is drawn from: floating cards, drop gaps,
//! list rows, the icon buttons, and the conflict legend that explains the row
//! tints.
//!
//! Split out of `main.rs` unchanged. Everything here builds an `Element` and
//! hands it back; none of it decides anything.

use std::collections::HashMap;

use iced::widget::{button, container, image, mouse_area, text, Column, Row, Space};
use iced::{Background, Border, Color, Element, Length, Theme};

use crate::theme::{accent, conflict_loses_bg, conflict_wins_bg, pal, row_bg, sel_bg};
use crate::{App, Message};

/// A collapsible settings section: a title row that toggles, and its body when
/// open. Colony's `view_collapsible_section`, in Eidos's palette.
///
/// Sections rather than one long column because a settings page is read by
/// hunting: everything visible at once is everything to read past.
pub(crate) fn settings_section<'a>(
    key: &'static str,
    title: &'a str,
    expanded: bool,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    let chevron = if expanded { "\u{25BE}" } else { "\u{25B8}" };
    let head = button(
        Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            // The convention says 15 for a section title. That number assumes
            // Colony's type scale, whose body text is 13; Eidos's is 12, and 15
            // here would leave a section title as loud as the category heading
            // above it. What the convention is actually specifying is the
            // HIERARCHY - page, category, section, body - and that is what this
            // keeps: 22 / 16 / 14 / 12.
            .push(text(title).size(14.0).width(Length::Fill))
            .push(text(chevron).size(10.0)),
    )
    .padding([7, 4])
    .width(Length::Fill)
    .on_press(Message::SettingsToggleSection(key))
    .style(button::text);

    if !expanded {
        return head.into();
    }
    let rule = container(Space::new().width(Length::Fill).height(Length::Fixed(1.0))).style(
        |_t: &Theme| container::Style {
            background: Some(Background::Color(pal().border_subtle)),
            ..Default::default()
        },
    );
    Column::new()
        .push(head)
        .push(rule)
        .push(container(content).padding([10, 4]).width(Length::Fill))
        .into()
}

/// A labelled switch with a line of explanation under it, the whole row
/// clickable. Colony's `view_functional_toggle`.
///
/// The description is not decoration: a settings page that only names its
/// options makes the user guess what each one costs.
pub(crate) fn settings_toggle<'a>(
    title: &'a str,
    desc: &'a str,
    on: bool,
    msg: Message,
) -> Element<'a, Message> {
    let knob = container(
        Space::new()
            .width(Length::Fixed(13.0))
            .height(Length::Fixed(13.0)),
    )
    .style(|_t: &Theme| container::Style {
        background: Some(Background::Color(pal().bg_card)),
        border: Border {
            radius: 7.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let track = container(
        Row::new()
            .push(Space::new().width(Length::Fixed(if on { 16.0 } else { 2.0 })))
            .push(knob),
    )
    .width(Length::Fixed(33.0))
    .height(Length::Fixed(17.0))
    .align_y(iced::alignment::Vertical::Center)
    .style(move |_t: &Theme| container::Style {
        background: Some(Background::Color(if on {
            accent()
        } else {
            pal().border_subtle
        })),
        border: Border {
            radius: 9.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    button(
        Row::new()
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .push(
                Column::new()
                    .spacing(1)
                    .width(Length::Fill)
                    .push(text(title).size(12.0))
                    .push(text(desc).size(10.0)),
            )
            .push(track),
    )
    .padding([5, 4])
    .width(Length::Fill)
    .on_press(msg)
    .style(button::text)
    .into()
}

/// A named control with its own line of explanation - the pick-list and slider
/// equivalent of [`settings_toggle`], so a section reads the same either way.
pub(crate) fn settings_row<'a>(
    title: &'a str,
    desc: &'a str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    Row::new()
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .padding([5, 4])
        .push(
            Column::new()
                .spacing(1)
                .width(Length::Fill)
                .push(text(title).size(12.0))
                .push(text(desc).size(10.0)),
        )
        .push(control)
        .into()
}

/// A read-only fact: a fixed-width label and its value. Colony's `info_row`.
pub(crate) fn settings_info<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    Row::new()
        .spacing(8)
        .padding([2, 4])
        .push(text(label).size(11.0).width(Length::Fixed(96.0)))
        .push(text(value).size(11.0))
        .into()
}

/// A thin strip beside the scrollbar marking WHERE the tinted rows are, in the
/// same colours the rows themselves use - the conflict green/red in the mod
/// list, the origin blue in the plugin list.
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
pub(crate) fn scroll_marks<'a>(tints: &[Option<Color>]) -> Element<'a, Message> {
    // Nothing to point at: take no width at all rather than leave a dead gutter.
    if tints.is_empty() || tints.iter().all(Option::is_none) {
        return Space::new().width(Length::Fixed(0.0)).into();
    }
    let mut col = Column::new()
        .width(Length::Fixed(SCROLL_MARKS_W))
        .height(Length::Fill);
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

/// Width of the marks strip: iced's default scrollbar width, because the
/// strip is laid OVER the scrollbar rather than beside it. Beside it, the marks
/// pushed the whole list sideways to make room - visible, and ugly, for
/// something that is meant to be a hint.
pub(crate) const SCROLL_MARKS_W: f32 = 10.0;

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
        bottom: if below {
            (win.height - at.y).max(0.0)
        } else {
            0.0
        },
        left: if right { 0.0 } else { at.x },
        right: if right {
            (win.width - at.x).max(0.0)
        } else {
            0.0
        },
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
    let me = app.conflicts.as_ref()?.mod_conflicts((focus + 1) as u32)?;
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
                container(
                    Space::new()
                        .width(Length::Fixed(12.0))
                        .height(Length::Fixed(12.0)),
                )
                .style(move |_t: &Theme| container::Style {
                    background: Some(Background::Color(c)),
                    border: Border {
                        color: accent(),
                        width: 1.0,
                        radius: 2.0.into(),
                    },
                    ..Default::default()
                }),
            )
            .push(text(label).size(11.0))
            .into()
    };
    let name = app
        .mods
        .get(focus)
        .map(|m| m.display_name().to_string())
        .unwrap_or_default();
    let mut row = Row::new().spacing(10).align_y(iced::Alignment::Center);
    row = row.push(text(format!("{name} conflicts:")).size(11.0));
    if over > 0 {
        row = row.push(swatch(conflict_wins_bg(), format!("{over} it overwrites")));
    }
    if under > 0 {
        row = row.push(swatch(conflict_loses_bg(), format!("{under} overwrite it")));
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
    let me = map.mod_conflicts((focus + 1) as u32)?;
    let other = (i + 1) as u32;
    if me.overwrites.contains(&other) {
        Some(conflict_wins_bg())
    } else if me.overwritten_by.contains(&other) {
        Some(conflict_loses_bg())
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
    let bar = container(
        Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(if active { 2.0 } else { 0.0 })),
    )
    .width(Length::Fill)
    .style(move |_t: &Theme| container::Style {
        background: active.then(|| Background::Color(accent())),
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
    mouse_area(strip)
        .on_enter(over(gap))
        .on_release(drop)
        .into()
}

/// The colour behind a mod row.
///
/// Selection outranks the conflict tint: the focused row is where the user's
/// attention already is, and losing its highlight to a colour that describes
/// OTHER rows would be a step backwards.
///
/// Its own function because the NAME CELL needs the same answer - it fades its
/// overflow into this colour, and a fade into the wrong one is a smear.
pub(crate) fn row_background(
    even: bool,
    selected: bool,
    conflict: Option<Color>,
    tint: Option<Color>,
) -> Color {
    // Precedence, and each step earns its place over the one below it.
    //
    // Selection wins outright: it is the answer to "where am I", asked
    // constantly. The conflict tint comes next because it is TRANSIENT - it only
    // exists relative to the currently selected mod, so it is a question being
    // actively asked, and it disappears the moment the answer stops mattering.
    // The user's own colour is last of the three because it is permanent: it can
    // afford to be covered for a moment, and it comes back on its own.
    if selected {
        sel_bg()
    } else {
        conflict.or(tint).unwrap_or_else(|| row_bg(even))
    }
}

/// A mod's chosen colour as a row background: a WASH over the stripe, not the
/// colour itself.
///
/// The palette is sized for a separator, which is a full-width bar of solid
/// colour a few times per list. Painted at that strength behind every row of a
/// four-hundred-mod list it stops being a marker and becomes the page. Mixed
/// down, the row still reads as "one of the blue ones" while the name on top of
/// it stays as legible as on any other row.
pub(crate) fn mod_tint(rgb: [u8; 3], even: bool) -> Color {
    const WASH: f32 = 0.22;
    let base = row_bg(even);
    let [r, g, b] = rgb;
    let mix = |a: f32, b8: u8| a + (f32::from(b8) / 255.0 - a) * WASH;
    Color {
        r: mix(base.r, r),
        g: mix(base.g, g),
        b: mix(base.b, b),
        a: base.a,
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
    tint: Option<Color>,
) -> Element<'a, Message> {
    let bg = row_background(even, selected, conflict, tint);
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
    button(text(label).size(12.0))
        .padding(6)
        .on_press(msg)
        .style(button::secondary)
        .into()
}

/// A flat, menu/toolbar-style button (no chrome until hovered).
pub(crate) fn flat_btn<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(13.0))
        .padding(6)
        .on_press(msg)
        .style(button::text)
        .into()
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
pub(crate) static ICON_HANDLES: std::sync::LazyLock<
    std::sync::Mutex<HashMap<usize, image::Handle>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

pub(crate) fn icon<'a>(bytes: &'static [u8], size: f32) -> Element<'a, Message> {
    let handle = {
        let mut cache = ICON_HANDLES.lock().unwrap_or_else(|p| p.into_inner());
        cache
            .entry(bytes.as_ptr() as usize)
            .or_insert_with(|| image::Handle::from_bytes(bytes))
            .clone()
    };
    image(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

/// A flat toolbar button: icon + label (MO2's icons-and-text mode).
pub(crate) fn icon_text_btn<'a>(
    bytes: &'static [u8],
    label: &'a str,
    msg: Message,
) -> Element<'a, Message> {
    let content = Row::new()
        .spacing(5)
        .push(icon(bytes, 16.0))
        .push(text(label).size(12.0));
    button(content)
        .padding(5)
        .on_press(msg)
        .style(button::text)
        .into()
}

/// A flat text-only toolbar button, for an action with no icon in the MO2 set
/// Eidos borrows. Same padding and style as [`icon_text_btn`] so the row does
/// not visibly change height where one appears.
pub(crate) fn text_btn<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .padding(5)
        .on_press(msg)
        .style(button::text)
        .into()
}

/// A flat icon-only button (toolbar right group, row arrows).
pub(crate) fn icon_btn<'a>(
    bytes: &'static [u8],
    size: f32,
    msg: Option<Message>,
) -> Element<'a, Message> {
    let mut b = button(icon(bytes, size)).padding(3).style(button::text);
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    b.into()
}

/// Where a dropdown hangs from before iced has answered where its button is,
/// and if it never does.
///
/// The y is the bottom edge of the menu bar, and unlike the two literals this
/// replaces it is DERIVED rather than measured off a screenshot: the main
/// screen's Column is `padding(4).spacing(4)`, its header row is 30.0 tall
/// (a 20.0 text at iced's fixed 1.3 line height, plus `[2, 6]` button padding),
/// and the menu bar is 30.9 (a 13.0 text, plus 6 of button padding each side,
/// plus 1 of container padding each side). 4 + 30.0 + 4 + 30.9 = 68.9. Nothing
/// that can be hidden sits above it, so the number is the same in every state
/// of the window - the old `44.0` was simply 25 px short, which put both cards
/// on top of the menu bar they were supposed to hang from.
///
/// The x is exact for the File item: 4 of Column padding plus 1 of the bar's
/// own. It is only a starting point for the others, which is why they are
/// measured rather than assumed.
pub(crate) const FALLBACK_MENU_ANCHOR: iced::Rectangle = iced::Rectangle {
    x: 5.0,
    y: 68.9,
    width: 0.0,
    height: 0.0,
};

/// The id of the menu-bar item a dropdown hangs from.
///
/// A `container` rather than the button itself, because `Button::operate`
/// passes `None` for the id (iced_widget button.rs) while `Container::operate`
/// passes its own through with the laid-out rectangle. Wrapping the button is
/// the whole trick.
pub(crate) const fn menu_anchor_id(name: &'static str) -> iced::advanced::widget::Id {
    iced::advanced::widget::Id::new(name)
}

/// Where iced actually laid out the container carrying `id`.
///
/// This is what replaces the hand-measured coordinates the two menu-bar
/// dropdowns used to be pinned at. A button's width is the width of its LABEL
/// in the resolved font, so no literal can be right on every machine, in every
/// theme, at every scale factor - and the two literals in the tree were both
/// wrong: the File menu by 1 px horizontally and 25 px vertically, the View menu
/// by 6 px and 25 px, and the filter pane by roughly 90 px in both axes at the
/// default window size, growing with every resize.
///
/// One frame of latency, spent deliberately: the caller opens the menu ON the
/// answer rather than before it, so the card is never drawn at a wrong place
/// first and then corrected.
pub(crate) fn measure(id: iced::advanced::widget::Id) -> iced::Task<Option<iced::Rectangle>> {
    use iced::advanced::widget::{operation::Outcome, Id, Operation};

    struct Find {
        target: Id,
        found: Option<iced::Rectangle>,
    }

    impl Operation<Option<iced::Rectangle>> for Find {
        fn traverse(
            &mut self,
            operate: &mut dyn FnMut(&mut dyn Operation<Option<iced::Rectangle>>),
        ) {
            operate(self);
        }

        fn container(&mut self, id: Option<&Id>, bounds: iced::Rectangle) {
            if id == Some(&self.target) {
                self.found = Some(bounds);
            }
        }

        fn finish(&self) -> Outcome<Option<iced::Rectangle>> {
            Outcome::Some(self.found)
        }
    }

    iced::advanced::widget::operate(Find {
        target: id,
        found: None,
    })
}

/// Hang `card` under `anchor`, kept inside the window.
///
/// Two things the padding-based placement it replaces could not do.
///
/// It positions with `pin`, whose layout calls `move_to` unconditionally.
/// Position expressed as container padding goes through `layout::positioned`,
/// which runs `Padding::fit` and SHRINKS the very padding that is doing the
/// positioning as soon as the card does not fit - so a tall menu did not
/// overflow where it was put, it slid back towards the window's top-left corner.
/// That is the "half of it sticks out at the top" this fixes.
///
/// And it clamps with `float`, whose closure is handed the card's real laid-out
/// rectangle and the viewport - the only place in this codebase where the size
/// of a floating card is actually known, rather than guessed at from which half
/// of the window its anchor sits in.
pub(crate) fn dropdown_under<'a>(
    card: Element<'a, Message>,
    anchor: iced::Rectangle,
    win: iced::Size,
) -> Element<'a, Message> {
    // `Pin` caps its content to what is left below and to the right of the
    // position, so a menu taller than the space under the bar is squeezed
    // rather than allowed to run off - and the scrollable gives that squeeze
    // somewhere to go instead of clipping the last items away.
    let room = (win.height - anchor.y - anchor.height - 8.0).max(80.0);
    let body: Element<'a, Message> = iced::widget::scrollable(card).height(Length::Shrink).into();
    let pinned = iced::widget::pin(body)
        .x(anchor.x)
        .y(anchor.y + anchor.height);
    iced::widget::float(container(pinned).max_height(room))
        .translate(|bounds: iced::Rectangle, viewport: iced::Rectangle| {
            // Only ever back INTO the window: `min(0.0)` leaves a card that
            // already fits exactly where it was put.
            iced::Vector::new(
                (viewport.x + viewport.width - bounds.x - bounds.width).min(0.0),
                (viewport.y + viewport.height - bounds.y - bounds.height).min(0.0),
            )
        })
        .into()
}
