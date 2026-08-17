//! The FOMOD scripted-installer wizard: the step-by-step option UI, its own
//! palette of parchment tones, and the preview pane.
//!
//! Split out of `main.rs` unchanged.

use crate::theme::*;
use crate::*;

pub(crate) fn group_type_label(t: eidos_fomod::GroupType) -> &'static str {
    use eidos_fomod::GroupType::*;
    match t {
        SelectExactlyOne => "choose one",
        SelectAtMostOne => "choose at most one",
        SelectAtLeastOne => "choose at least one",
        SelectAny => "choose any",
        SelectAll => "all included",
    }
}

/// Whether the wizard's current step satisfies its group constraints: a "choose
/// one" group needs exactly one selected, a "choose at least one" needs >= 1.
pub(crate) fn step_valid(w: &FomodWizard) -> bool {
    use eidos_fomod::GroupType::*;
    let Some(step) = w.session.config.steps.get(w.step) else {
        return true;
    };
    let Some(sel) = w.selection.get(w.step) else {
        return true;
    };
    for (gi, group) in step.groups.iter().enumerate() {
        let count = sel.get(gi).map(|g| g.iter().filter(|&&x| x).count()).unwrap_or(0);
        let ok = match group.group_type {
            SelectExactlyOne => count == 1,
            SelectAtLeastOne => count >= 1,
            _ => true,
        };
        if !ok {
            return false;
        }
    }
    true
}

/// The FOMOD installer wizard: the current step's groups as selectable options,
/// with Back / Cancel / Next / Install.
/// Width of the option column. FIXED, and narrow: the options are short labels,
/// and every pixel it does not take is a pixel the preview image gets. A long
/// option name is clipped rather than allowed to widen the column, because the
/// image is what the user is actually comparing.
pub(crate) const FOMOD_OPTIONS_W: f32 = 260.0;
/// Height of the preview box. Also fixed, and it stays even when an option has
/// no image: FOMOD art is wildly inconsistent - CBBE ships portrait body shots
/// next to letterbox eyebrow strips - so a box that resized to its content would
/// make the whole dialog jump on every hover.
pub(crate) const FOMOD_PREVIEW_H: f32 = 420.0;

// The parchment family, spelled out once. These are the same inks the rest of the
// window uses; the wizard only ever looked out of place because it was drawn with
// iced's stock `button::secondary` and an ASCII `[x]`, not because it was missing
// anything iced cannot do.
pub(crate) const FOMOD_RULE: Color = Color::from_rgb(0.81, 0.75, 0.63); // hairlines and dividers
pub(crate) const FOMOD_ROW_BG: Color = Color::from_rgb(0.89, 0.84, 0.72); // an unselected option
pub(crate) const FOMOD_ROW_HOVER: Color = Color::from_rgb(0.93, 0.88, 0.78);
// Both inks are measured against the page (0xECDFC2): SOFT reaches 6.2:1 and FAINT
// 4.5:1, the WCAG floor for text this small. The first pass had FAINT at 2.9:1,
// which is a decorative grey, not a legible one - and it was carrying "required"
// and "recommended", the only guidance the mod author gives. Below ~10px there is
// no room for a genuinely faint tier, so the hierarchy lives in size and weight.
pub(crate) const FOMOD_INK_SOFT: Color = Color::from_rgb(0.36, 0.30, 0.23); // descriptions, tags
pub(crate) const FOMOD_INK_FAINT: Color = Color::from_rgb(0.44, 0.38, 0.30); // group metadata
pub(crate) const FOMOD_PARCHMENT: Color = Color::from_rgb(0.95, 0.92, 0.83); // ink on burgundy

/// The circle or square in front of an option, drawn rather than written.
///
/// Eidos ships no icon font, so `[x]`/`[ ]` was standing in for a control - and it
/// was the single loudest thing separating this dialog from MO2's. Two nested
/// containers cost nothing and give the real shape, which also carries meaning MO2
/// itself carries: a ROUND marker is a group you pick one of, a SQUARE one is a
/// group you pick any number of. The user learns the rule from the shape instead of
/// reading "choose at most one" every time.
pub(crate) fn fomod_marker<'a>(on: bool, usable: bool, radio: bool) -> Element<'a, Message> {
    // `on` is tested FIRST because the row fill and the row label both test it
    // first: a selected row is burgundy whatever else is true of it. Testing
    // `!usable` first painted the marker in dark ink on that burgundy, so an option
    // that was both ticked and forbidden showed a tick you could not see.
    let ink = if on {
        FOMOD_PARCHMENT
    } else if !usable {
        FOMOD_INK_FAINT
    } else {
        FOMOD_INK_SOFT
    };
    // The dot fills what the 4px inset leaves it, rather than being a fixed size
    // that gets centred. Centring a 7px dot in a 14px ring asks the renderer for a
    // 3.5px offset on each side, and half a pixel has to land somewhere: it went
    // up and left, so every ticked option looked knocked off its axis. An inset is
    // exact arithmetic - 14 - 4 - 4 = 6 - and cannot drift.
    let inner: Element<'_, Message> = if on {
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_t: &Theme| container::Style {
                background: Some(Background::Color(ink)),
                border: Border {
                    radius: (if radio { 3.0 } else { 1.0 }).into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    } else {
        Space::new().into()
    };
    container(inner)
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .padding(4)
        .style(move |_t: &Theme| container::Style {
            border: Border {
                color: ink,
                width: 1.5,
                radius: (if radio { 7.0 } else { 3.0 }).into(),
            },
            ..Default::default()
        })
        .into()
}

/// A hairline. Used to seat the header and the footer instead of leaving three
/// blocks of content floating in one undifferentiated field of parchment.
pub(crate) fn fomod_rule<'a>(vertical: bool) -> Element<'a, Message> {
    let (w, h) = if vertical {
        (Length::Fixed(1.0), Length::Fill)
    } else {
        (Length::Fill, Length::Fixed(1.0))
    };
    container(Space::new())
        .width(w)
        .height(h)
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(FOMOD_RULE)),
            ..Default::default()
        })
        .into()
}

/// The FOMOD installer, laid out the way MO2 lays it out: the options on one
/// side, and ONE description and ONE image for whichever option is current.
///
/// The previous version rendered every option's description and full-size image
/// inline, one after another, so a step of CBBE was several thousand pixels tall
/// and two body shapes could never be compared without scrolling between them.
/// MO2 avoids that by filtering HoverEnter on each option and filling two fixed
/// panes (fomodinstallerdialog.cpp:628); this does the same, and additionally
/// falls back to the SELECTED option when nothing is hovered, so the pane is
/// never blank and the dialog is usable without a mouse.
pub(crate) fn fomod_wizard_view(w: &FomodWizard) -> Element<'_, Message> {
    use eidos_fomod::PluginType;
    let config = &w.session.config;
    let types = eidos_fomod::step_types(config, &w.selection, &w.ctx, w.step);
    let step = config.steps.get(w.step);

    // Steps whose `<visible>` condition is false are skipped by Next/Back and
    // ignored by build_plan, so counting raw indices made the header lie: a run
    // that shows three panels announced "Step 1 of 5", then jumped to "Step 3".
    // Number by position among the steps that will actually be shown.
    let vis = eidos_fomod::visible_steps(config, &w.selection, &w.ctx);
    let total = vis.iter().filter(|v| **v).count().max(1);
    let shown_no = (0..=w.step).filter(|&i| vis.get(i).copied().unwrap_or(false)).count().max(1);

    // What the preview is about: the hovered option, else the first selected one,
    // else the first option of the step.
    let current = w.hover.filter(|&(gi, pi)| {
        step.is_some_and(|s| s.groups.get(gi).is_some_and(|g| pi < g.plugins.len()))
    });
    let current = current.or_else(|| {
        let s = step?;
        let sel = w.selection.get(w.step)?;
        s.groups.iter().enumerate().find_map(|(gi, g)| {
            (0..g.plugins.len())
                .find(|&pi| sel.get(gi).and_then(|gg| gg.get(pi)).copied().unwrap_or(false))
                .map(|pi| (gi, pi))
        })
    });
    let current = current.or_else(|| step.and_then(|s| (!s.groups.is_empty()).then_some((0, 0))));

    // ---- header: which mod, which step ----
    let bold = iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::DEFAULT };
    // The name takes Length::Fill and the chip stays Shrink. iced's flex measures
    // the Shrink children first and hands what is left to the Fill ones, so the
    // chip is always laid out at its full size; with the name left as Shrink it was
    // measured first, ate the whole row, and squeezed the chip to zero width.
    let mut title = Row::new()
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .push(text(config.module_name.clone()).size(17.0).font(bold).width(Length::Fill));
    if let Some(s) = step {
        title = title.push(text(s.name.clone()).size(12.0).color(FOMOD_INK_FAINT));
    }
    let head = title.push(
        // The step counter as a chip, so it reads as status rather than as one
        // more sentence competing with the mod's name.
        container(
            text(format!("Step {shown_no} of {total}")).size(11.0).color(FOMOD_PARCHMENT),
        )
            .padding([3, 9])
            .style(|t: &Theme| container::Style {
                background: Some(Background::Color(t.palette().primary)),
                border: Border { radius: 9.0.into(), ..Default::default() },
                ..Default::default()
            }),
    );

    // ---- left: the options, compact ----
    let mut opts = Column::new().spacing(3);
    if let Some(s) = step {
        for (gi, group) in s.groups.iter().enumerate() {
            use eidos_fomod::GroupType;
            // Round marker = pick one of these; square = pick as many as you like.
            let radio = matches!(
                group.group_type,
                GroupType::SelectExactlyOne | GroupType::SelectAtMostOne
            );
            opts = opts.push(
                container(
                    Row::new()
                        .spacing(5)
                        // Fill on the name for the same reason as the header: the
                        // separator and the "choose one" label are Shrink and get
                        // measured first, so they can no longer be starved to zero
                        // width by a long group name and pushed out of the column.
                        .push(
                            text(group.name.clone())
                                .size(11.0)
                                .color(FOMOD_INK_SOFT)
                                .width(Length::Fill),
                        )
                        .push(text("·").size(11.0).color(FOMOD_INK_FAINT))
                        .push(
                            text(group_type_label(group.group_type))
                                .size(11.0)
                                .color(FOMOD_INK_FAINT),
                        ),
                )
                .padding([9, 4]),
            );
            for (pi, plugin) in group.plugins.iter().enumerate() {
                let on = w
                    .selection
                    .get(w.step)
                    .and_then(|sl| sl.get(gi))
                    .and_then(|g| g.get(pi))
                    .copied()
                    .unwrap_or(false);
                let ptype =
                    types.get(gi).and_then(|g| g.get(pi)).copied().unwrap_or(PluginType::Optional);
                let usable = ptype != PluginType::NotUsable;
                // Two ways an option can be present but not yours to change, and
                // neither may offer a click that does nothing or, worse, one that
                // quietly breaks the install:
                //   - Required: the engine pre-ticks it, but every branch of the
                //     toggle handler would happily tick it back OFF, and build_plan
                //     only installs the files of options still marked selected. One
                //     click on a required option silently dropped its files.
                //   - SelectAll: the handler's `SelectAll => {}` arm is a no-op, so
                //     the row lit up on hover and answered nothing.
                //   - and in a RADIO group holding a Required option, every OTHER
                //     row: the radio arms clear the whole group before setting the
                //     clicked one, so a click on an Optional sibling silently
                //     unticked the Required row and dropped its files from the
                //     plan - the exact failure the Required lock exists for, one
                //     row over. The engine says the choice is forced; the view
                //     has to say it too.
                let group_forced = radio
                    && types.get(gi).is_some_and(|g| {
                        g.iter().any(|t| matches!(t, PluginType::Required))
                    });
                let locked = matches!(ptype, PluginType::Required)
                    || matches!(group.group_type, GroupType::SelectAll)
                    || group_forced;
                let tag = match ptype {
                    PluginType::Required => "required",
                    PluginType::Recommended => "recommended",
                    PluginType::NotUsable => "not usable",
                    _ => "",
                };
                let label = if on {
                    FOMOD_PARCHMENT
                } else if usable {
                    palette().text
                } else {
                    FOMOD_INK_FAINT
                };
                let row = Row::new()
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .push(fomod_marker(on, usable, radio))
                    .push(text(plugin.name.clone()).size(12.5).color(label).width(Length::Fill))
                    // 10px, not 9.5, and in the darker ink: this string carries the
                    // author's own guidance and was being rendered at 2.7:1.
                    .push(text(tag).size(10.0).color(if on {
                        FOMOD_PARCHMENT
                    } else {
                        FOMOD_INK_SOFT
                    }));
                let mut b = button(row)
                    .padding([7, 9])
                    .width(Length::Fill)
                    .style(move |t: &Theme, s: button::Status| {
                        // A locked row must not light up on hover: the highlight is
                        // a promise that a click will do something.
                        let hovered = matches!(s, button::Status::Hovered) && !locked;
                        let bg = if on {
                            t.palette().primary
                        } else if !usable {
                            Color { a: 0.35, ..FOMOD_ROW_BG }
                        } else if hovered {
                            FOMOD_ROW_HOVER
                        } else {
                            FOMOD_ROW_BG
                        };
                        button::Style {
                            background: Some(Background::Color(bg)),
                            text_color: label,
                            border: Border {
                                color: if on { t.palette().primary } else { FOMOD_RULE },
                                width: 1.0,
                                radius: 5.0.into(),
                            },
                            ..Default::default()
                        }
                    });
                if usable && !locked {
                    b = b.on_press(Message::FomodToggle(gi, pi));
                }
                // Hover drives the preview; leaving falls back to the selection
                // rather than blanking the pane.
                opts = opts.push(
                    mouse_area(b)
                        .on_enter(Message::FomodHover(Some((gi, pi))))
                        .on_exit(Message::FomodUnhover(gi, pi)),
                );
            }
        }
    }

    // ---- right: one image, one description ----
    let shown = current.and_then(|(gi, pi)| step?.groups.get(gi)?.plugins.get(pi));
    let art = shown
        .and_then(|p| p.image.as_ref())
        .and_then(|p| w.session.resolve(p))
        .or_else(|| config.module_image.as_ref().and_then(|p| w.session.resolve(p)));
    let preview: Element<'_, Message> = match art {
        // `contain` keeps the aspect ratio inside the fixed box, so a portrait
        // body shot and a letterbox eyebrow sheet both sit still.
        Some(path) => image(image::Handle::from_path(path))
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(iced::ContentFit::Contain)
            .into(),
        // INK_SOFT, not FAINT: this sits on the preview fill, which is darker than
        // the page, so the faint ink fell to 2.4:1 and the box just read as blank.
        None => container(text("No preview for this option.").size(12.0).color(FOMOD_INK_SOFT))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
    };
    let mut right = Column::new().spacing(10).push(
        container(preview)
            .width(Length::Fill)
            .height(Length::Fixed(FOMOD_PREVIEW_H))
            .padding(8)
            .style(|_t: &Theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(0xD9, 0xC9, 0xA8))),
                border: Border {
                    color: FOMOD_RULE,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            }),
    );
    if let Some(p) = shown {
        // Name on its own line, description under it. A Row would have kept the
        // name from ever wrapping with the paragraph, and FOMOD descriptions are
        // paragraphs - CBBE's run to several lines.
        let mut d = Column::new()
            .spacing(4)
            .push(text(p.name.clone()).size(13.0).font(bold).width(Length::Fill));
        if !p.description.is_empty() {
            d = d.push(
                text(p.description.clone()).size(12.0).color(FOMOD_INK_SOFT).width(Length::Fill),
            );
        }
        // Scrollable, because the preview box above it is a hard 420px and the pane
        // is whatever the window leaves. iced does not clip text to its node, so a
        // long description did not truncate cleanly - it painted straight over the
        // footer, and its tail was unreachable. FOMOD descriptions carry "do NOT
        // pick this if..." warnings, so losing the tail is losing the warning.
        right = right.push(scrollable(d).height(Length::Fill));
    }

    // ---- footer ----
    let vis = eidos_fomod::visible_steps(config, &w.selection, &w.ctx);
    let has_prev = (0..w.step).any(|i| vis.get(i).copied().unwrap_or(false));
    let has_next = (w.step + 1..vis.len()).any(|i| vis[i]);
    let valid = step_valid(w);

    let mut nav = Row::new().spacing(8).align_y(iced::Alignment::Center);
    if !valid {
        nav = nav.push(
            text("Select the required option(s) to continue.").size(11.0).color(FOMOD_INK_FAINT),
        );
    }
    nav = nav.push(Space::new().width(Length::Fill));
    nav = nav.push(fomod_btn("Cancel", Some(Message::FomodCancel), false));
    if has_prev {
        nav = nav.push(fomod_btn("Back", Some(Message::FomodBack), false));
    }
    let (label, msg) =
        if has_next { ("Next", Message::FomodNext) } else { ("Install", Message::FomodInstall) };
    // The one button that carries the flow gets the burgundy. When the step is
    // unsatisfied it keeps its place and its size and simply stops responding,
    // rather than vanishing and shifting the whole row.
    nav = nav.push(fomod_btn(label, valid.then_some(msg), true));

    let panes = Row::new()
        .spacing(14)
        .height(Length::Fill)
        .push(
            // The right inset goes INSIDE the scrollable, not around it: iced draws
            // the scrollbar over the content, so without it the bar sat on the rows'
            // right border and crowded the "recommended" tag.
            container(
                scrollable(container(opts).padding(iced::Padding {
                    top: 0.0,
                    right: 13.0,
                    bottom: 0.0,
                    left: 3.0,
                }))
                .height(Length::Fill),
            )
                .width(Length::Fixed(FOMOD_OPTIONS_W)),
        )
        .push(fomod_rule(true))
        .push(container(right).width(Length::Fill));

    Column::new()
        .spacing(12)
        .padding(16)
        .push(head)
        .push(fomod_rule(false))
        .push(panes)
        .push(fomod_rule(false))
        .push(nav)
        .into()
}

/// A footer button. `msg == None` means present but inert: the disabled Next has
/// to hold its width or the footer jumps the moment a step becomes satisfiable.
///
/// The primary one is also given a FIXED width, because its label is not stable:
/// ticking an option can reveal or hide a later step, which flips "Next" to
/// "Install" and back. The row is right-aligned behind a Fill spacer, so an 8px
/// change in that one label slid Cancel and Back sideways under the pointer.
pub(crate) fn fomod_btn<'a>(label: &'a str, msg: Option<Message>, primary: bool) -> Element<'a, Message> {
    let live = msg.is_some();
    let mut b = button(text(label).size(12.5).width(Length::Fill).center())
        .padding([7, 16])
        .width(if primary { Length::Fixed(104.0) } else { Length::Shrink })
        .style(move |t: &Theme, s: button::Status| {
            let hovered = matches!(s, button::Status::Hovered);
            let p = t.palette().primary;
            // The disabled state used to be translucent burgundy under translucent
            // parchment, which composited to a 1.3:1 label - not dim, gone. The one
            // moment the user most needs to read a button is when it will not let
            // them past. Opaque fill, dark ink: readable, and unmistakably inert.
            let (bg, fg) = match (primary, live, hovered) {
                (true, true, false) => (p, FOMOD_PARCHMENT),
                (true, true, true) => (Color { a: 0.85, ..p }, FOMOD_PARCHMENT),
                (true, false, _) => (FOMOD_ROW_BG, FOMOD_INK_SOFT),
                (false, _, true) => (FOMOD_ROW_HOVER, t.palette().text),
                (false, _, false) => (FOMOD_ROW_BG, t.palette().text),
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: fg,
                border: Border {
                    color: if primary && live { Color { a: 0.0, ..p } } else { FOMOD_RULE },
                    width: 1.0,
                    radius: 5.0.into(),
                },
                ..Default::default()
            }
        });
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    b.into()
}
