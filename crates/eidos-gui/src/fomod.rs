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
                let locked = matches!(ptype, PluginType::Required)
                    || matches!(group.group_type, GroupType::SelectAll);
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

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    if let Some(w) = &app.fomod {
        let base = fomod_wizard_view(w);
        // A reinstall collision raised from inside the wizard must be able to
        // show over it (the wizard replaces the whole view).
        if let Some(c) = &app.collision {
            let scrim =
                mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::CollisionCancel);
            let dialog = container(collision_dialog(c)).center(Length::Fill);
            return Stack::new().push(base).push(scrim).push(dialog).into();
        }
        return base;
    }
    if app.screen == Screen::Main {
        return main_screen(app);
    }
    let inner = match app.screen {
        Screen::Welcome => welcome(),
        Screen::Kind => kind_screen(app),
        Screen::Game => game_screen(app),
        Screen::NameLoc => nameloc_screen(app),
        Screen::Summary => summary_screen(app),
        Screen::Main => welcome(),
    };
    container(inner).width(Length::Fill).height(Length::Fill).padding(20).into()
}

/// Keyboard subscription: surface global shortcuts and keep the live modifier state
/// in sync so a plain mod-row click can branch to Ctrl-toggle / Shift-extend.
///
/// Shortcuts only fire on the main screen, and only when no modal / inline editor is
/// stealing input, so they never clobber typing into a text field. Mirrors MO2's
/// global accelerators: F5 (Refresh) and Ctrl+R (Run).
/// How long a `.unfinished` partial may go without growing before it is called
/// stalled rather than downloading. Generous: a slow mirror can go quiet for a
/// few seconds, and calling a live download dead is worse than the reverse.
pub(crate) const STALLED_AFTER: std::time::Duration = std::time::Duration::from_secs(20);

/// How often the downloads directory is re-scanned while something is arriving.
/// Fast enough that a progress bar moves rather than jumps, slow enough that it
/// is a rounding error next to the transfer itself.
pub(crate) const DOWNLOAD_TICK: std::time::Duration = std::time::Duration::from_millis(500);

/// The same, when nothing is in flight: something has to notice that a download
/// STARTED, and that something cannot be the download itself - it runs in
/// another process, launched by the browser, with no way to reach this one.
pub(crate) const DOWNLOAD_IDLE_TICK: std::time::Duration = std::time::Duration::from_secs(2);

pub(crate) fn subscription(app: &App) -> iced::Subscription<Message> {
    use iced::keyboard::{self, key::Named, Key};

    // Track held modifiers from every key press AND release (a release with no
    // remaining keys still carries the updated modifier set).
    // One stream now: `listen` yields every keyboard event and all three variants
    // carry the modifier set, so press and release no longer need separate
    // subscriptions. ModifiersChanged also reaches us for the first time - no
    // widget captures it - which means the held set no longer goes stale while a
    // text field has the caret.
    let track = keyboard::listen().map(|event| match event {
        keyboard::Event::KeyPressed { modifiers, .. }
        | keyboard::Event::KeyReleased { modifiers, .. }
        | keyboard::Event::ModifiersChanged(modifiers) => Message::ModifiersChanged(modifiers),
    });

    // Where the pointer is, and how big the window is. Needed because iced's
    // right-press carries no coordinates, so a context menu cannot otherwise be
    // placed where it was summoned from. The handlers do nothing but store.
    let pointer = iced::event::listen_with(|event, _status, _window| match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::PointerAt(position))
        }
        iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::WindowResized(size))
        }
        _ => None,
    });

    // App shortcuts. `on_key_press` takes a plain `fn`, so it cannot read `app`;
    // the handlers themselves no-op off the main screen / while a modal is open.
    let shortcuts = keyboard::listen().filter_map(|event| {
        let keyboard::Event::KeyPressed { key, modifiers: mods, .. } = event else {
            return None;
        };
        match key.as_ref() {
        Key::Named(Named::F5) => Some(Message::Refresh),
        // Ctrl+R launches the current run target (MO2's Run accelerator).
        Key::Character("r") if mods.control() => Some(Message::Run),
        Key::Named(Named::Escape) => Some(Message::ClearSelection),
        Key::Character("a") if mods.control() || mods.command() => {
            Some(Message::SelectAllInFocus)
        }
        // Ctrl+C over the LOOT report copies it whole. `update` no-ops when the
        // report is not open, since this closure cannot see the app.
        Key::Character("c") if mods.control() || mods.command() => {
            Some(Message::CopyLootReport)
        }
        // Navigation. Which list answers is decided in `update` - this closure
        // is a plain `fn` and cannot see the app.
        Key::Named(Named::Tab) => Some(Message::CycleFocus),
        // Ctrl moves the ROW; plain moves the focus. Checked first, or the
        // plain arms below would swallow it.
        Key::Named(Named::ArrowUp) if mods.control() || mods.command() => {
            Some(Message::KeyNav(Nav::ShiftUp))
        }
        Key::Named(Named::ArrowDown) if mods.control() || mods.command() => {
            Some(Message::KeyNav(Nav::ShiftDown))
        }
        Key::Named(Named::ArrowUp) => Some(Message::KeyNav(Nav::Up)),
        Key::Named(Named::ArrowDown) => Some(Message::KeyNav(Nav::Down)),
        Key::Named(Named::PageUp) => Some(Message::KeyNav(Nav::PageUp)),
        Key::Named(Named::PageDown) => Some(Message::KeyNav(Nav::PageDown)),
        Key::Named(Named::Home) => Some(Message::KeyNav(Nav::First)),
        Key::Named(Named::End) => Some(Message::KeyNav(Nav::Last)),
        Key::Named(Named::Space) => Some(Message::KeyNav(Nav::Toggle)),
        Key::Named(Named::Enter) => Some(Message::KeyNav(Nav::Activate)),
        Key::Named(Named::Delete) => Some(Message::KeyNav(Nav::Remove)),
        _ => None,
        }
    });

    // The shortcut stream is gated on the main screen (the wizard/FOMOD views have
    // their own focus); modifier tracking always runs so the set is never stale.
    // Navigation keys are suppressed while a field has the caret; the always-safe
    // ones (F5, Ctrl+R, Escape, Ctrl+A) keep working, and Escape is what gets the
    // keyboard back out of a field.
    let typing = app.typing;
    let shortcuts = shortcuts.with(typing).map(|(typing, m)| match m {
        Message::KeyNav(_) | Message::CycleFocus if typing => Message::Noop,
        other => other,
    });

    let mut subs = vec![track, pointer];
    if app.screen == Screen::Main
        && app.fomod.is_none()
        && app.rename.is_none()
        && !app.settings_open
        && app.executables.is_none()
        && app.collision.is_none()
        && app.info_mod.is_none()
        // Don't fire shortcuts (especially Ctrl+R) while the GUI is locked behind a
        // running game or a LOOT report is open. An unlocked tracked run keeps them.
        && app.running.as_ref().is_none_or(|r| !r.lock)
        && app.loot_report.is_none()
        // Every other overlay that owns the screen. A navigation key reaching
        // the mod list from behind one of these moves a selection the user
        // cannot see, and Space would toggle a mod they are not looking at.
        && !app.about_open
        && !app.view_menu_open
        && app.picker.is_none()
        && app.profile_menu.is_none()
        && app.profile_rename.is_none()
        && app.profile_copy.is_none()
        && app.profile_delete_confirm.is_none()
        && app.send_priority.is_none()
        && app.overwrite_to_mod.is_none()
        && app.menu_mod.is_none()
    {
        subs.push(shortcuts);
    }
    // Watch the downloads directory while its tab is open. Polling is not a
    // shortcut taken for want of something better: the transfer runs in a
    // separate `eidos nxm` process spawned by the BROWSER, so there is no handle
    // to await and no channel to listen on. The filesystem is the interface, and
    // a directory of a few dozen entries is cheap to read twice a second.
    //
    // Faster while something is arriving, so a bar moves instead of jumping;
    // slower otherwise, because the idle case only has to notice that a download
    // has begun.
    if app.tab == Tab::Downloads {
        let arriving =
            app.downloads.iter().any(|d| d.state == DownloadState::Downloading);
        let period = if arriving { DOWNLOAD_TICK } else { DOWNLOAD_IDLE_TICK };
        subs.push(iced::time::every(period).map(|_| Message::DownloadTick));
    }
    // While waiting on a launched game/tool, poll for its exit so we can unlock.
    if app.running.is_some() {
        subs.push(
            iced::time::every(std::time::Duration::from_millis(600)).map(|_| Message::PollRunning),
        );
    }
    iced::Subscription::batch(subs)
}

pub(crate) fn main() -> iced::Result {
    // Steam passes the Proton command as our arguments via `eidos-gui %command%`.
    let launch_command: Vec<String> = std::env::args().skip(1).collect();
    // The title moved out of `application` and onto a builder; the first argument
    // is now the boot function that `run_with` used to take. It must be `Fn`, not
    // `FnOnce` - which is why the `.clone()` stays: without it the closure would
    // consume the Vec and only be callable once.
    iced::application(move || new(launch_command.clone()), update, view)
        .title("Eidos")
        .theme(theme)
        .subscription(subscription)
        .window(window_settings())
        .run()
}

/// The desktop identity of the window. MUST equal the basename of the installed
/// `eidos.desktop`, because that pairing is the only thing tying the two
/// together.
pub const APP_ID: &str = "eidos";

/// How the window introduces itself to the desktop.
///
/// Without `application_id` a Wayland surface announces an EMPTY app id, so the
/// compositor has nothing to match against a desktop entry and a taskbar shows a
/// placeholder tile no matter how many icons are installed. That was the actual
/// symptom; the icon files were never the missing part.
///
/// The embedded icon covers X11 and XWayland, where the icon travels with the
/// window instead of being looked up from a desktop file - so the binary is
/// self-sufficient even with nothing installed. It is the dark-ground tile
/// rather than the transparent mark: the mark is pale ink, which disappears
/// against a light panel. A decode failure costs the icon, never the launch.
pub(crate) fn window_settings() -> iced::window::Settings {
    iced::window::Settings {
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: APP_ID.to_string(),
            ..Default::default()
        },
        icon: iced::window::icon::from_file_data(
            include_bytes!("../../../assets/brand/png/eidos-icon-256-on-dark.png"),
            None,
        )
        .ok(),
        ..Default::default()
    }
}
