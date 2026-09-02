//! Time-based animation.
//!
//! Three rules hold everything here together, and they are why this costs
//! nothing when the window is idle.
//!
//! **Nothing animated changes layout.** Every animation in this window
//! interpolates a *colour*. A frame in flight therefore never re-measures a
//! row, never reflows the mod list, and never touches the file tree - which
//! matters, because a fully expanded Skyrim Data tree is six figures of rows and
//! re-laying it out sixty times a second would be a stutter, not a flourish.
//!
//! **The clock only runs while something is moving.** [`App::animating`] gates
//! the frame subscription, so an idle window subscribes to no timer at all. The
//! cost at rest is not small, it is zero.
//!
//! **A phase ends by itself.** [`Phase`] holds the instant a transition began
//! and derives everything from elapsed time, so nothing has to remember to
//! switch it off - and a missed tick shows up as a jump forward rather than as
//! an animation stuck half way.
//!
//! [`App::animating`]: crate::App::animating

use std::time::Instant;

use iced::Color;

/// How long every transition in this window lasts.
///
/// One value, not one per animation: the Colony convention fixes the sidebar
/// slide at 200 ms, and two durations in one window read as one of them being
/// wrong. Long enough to be seen, short enough that it never sits between the
/// user and the thing they clicked.
pub(crate) const MS: f32 = 200.0;

/// Fast at the start, settling at the end.
///
/// The same curve Colony uses. Linear interpolation reads as mechanical because
/// nothing physical starts and stops at a constant rate.
pub(crate) fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// One transition: when it started, or `None` if it never has.
///
/// Copy, small, and derived entirely from a single `Instant` - so a struct that
/// holds several of these stays cheap to clone, which `App` relies on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Phase {
    started: Option<Instant>,
}

impl Phase {
    /// (Re)start the transition from zero.
    ///
    /// Restarting mid-flight is deliberate rather than merely allowed: clicking
    /// a third tab while the second is still fading should run the new
    /// transition, not queue it behind the old one.
    pub(crate) fn start(&mut self) {
        self.started = Some(Instant::now());
    }

    /// How far along, 0.0 to 1.0, eased.
    ///
    /// A phase that has never started reads as **finished**, not as pending: a
    /// window opening on its default tab must draw that tab selected, not fade
    /// it in from nothing on the first frame.
    pub(crate) fn eased(&self) -> f32 {
        ease_out_cubic(self.linear())
    }

    /// The raw fraction of `MS` elapsed, before easing. Split out so the tests
    /// can talk about time without also talking about the curve.
    pub(crate) fn linear(&self) -> f32 {
        match self.started {
            None => 1.0,
            Some(t) => (t.elapsed().as_secs_f32() * 1000.0 / MS).clamp(0.0, 1.0),
        }
    }

    /// Whether this phase still needs frames.
    pub(crate) fn running(&self) -> bool {
        self.linear() < 1.0
    }
}

/// Blend two colours, including their alpha.
///
/// `t` is clamped, so a caller that hands over an unclamped fraction gets the
/// endpoint rather than a colour outside the range - which would render as
/// something that belongs to no theme.
pub(crate) fn mix(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

/// The same blend for an optional background, treating `None` as transparent.
///
/// iced's button styles use `None` to mean "draw no background at all", which is
/// not the same as a transparent one for the purposes of blending: fading from
/// `None` to a colour has to start from that colour at zero alpha, or the first
/// frame flashes the wrong hue.
pub(crate) fn mix_bg(
    from: Option<iced::Background>,
    to: Option<iced::Background>,
    t: f32,
) -> Option<iced::Background> {
    let colour = |b: Option<iced::Background>, other: Option<iced::Background>| match b {
        Some(iced::Background::Color(c)) => c,
        // Not `Color::TRANSPARENT`: black at zero alpha darkens the blend on
        // every intermediate frame. Start from the OTHER end's hue instead.
        _ => match other {
            Some(iced::Background::Color(c)) => Color { a: 0.0, ..c },
            _ => Color::TRANSPARENT,
        },
    };
    let a = colour(from, to);
    let b = colour(to, from);
    Some(iced::Background::Color(mix(a, b, t)))
}

/// Whether anything is moving right now.
///
/// This is what keeps an idle window free: `subscription` asks, and when the
/// answer is no it does not subscribe to the frame timer at all - there is no
/// timer running and being ignored, there is no timer.
///
/// `motion` short-circuits it, so a user who turned animation off never gets a
/// tick even for a transition that was just started. The phases still advance
/// in wall-clock time; they are simply drawn at their destination, because
/// every reader goes through [`at`].
pub(crate) fn animating(app: &crate::App) -> bool {
    app.motion && (app.tab_anim.running() || app.info_anim.running() || app.status_anim.running())
}

/// How far a phase should be DRAWN, honouring the motion preference.
///
/// Every view-side reader goes through here rather than calling `eased`
/// directly. With motion off the answer is always 1.0 - the end state, drawn
/// immediately - which is what "reduced motion" has to mean: not a quicker
/// animation, none at all.
pub(crate) fn at(app: &crate::App, phase: &Phase) -> f32 {
    if app.motion {
        phase.eased()
    } else {
        1.0
    }
}

/// How selected a tab should be DRAWN, from 0.0 (unselected) to 1.0 (selected).
///
/// Three cases, and the middle one is the whole point: the tab being left
/// behind runs the same transition backwards, so the strip crossfades instead
/// of one end snapping while the other fades.
///
/// Generic because the window has two of these strips - the main one and the
/// mod-information one - and they must not drift apart.
pub(crate) fn tab_mix<T: PartialEq>(t: f32, current: &T, previous: Option<&T>, this: &T) -> f32 {
    if this == current {
        t
    } else if previous == Some(this) {
        1.0 - t
    } else {
        0.0
    }
}

/// Blend two button styles.
///
/// Both ends come from iced's own `button::primary` / `button::secondary`,
/// evaluated against the live theme, so this never names a colour: it stays
/// correct on the dark palette and on any palette added later. The endpoints
/// short-circuit, which is also what makes an un-animated window pixel-identical
/// to what it drew before this existed.
pub(crate) fn mix_button(
    off: iced::widget::button::Style,
    on: iced::widget::button::Style,
    t: f32,
) -> iced::widget::button::Style {
    if t <= 0.0 {
        return off;
    }
    if t >= 1.0 {
        return on;
    }
    iced::widget::button::Style {
        background: mix_bg(off.background, on.background, t),
        text_color: mix(off.text_color, on.text_color, t),
        border: iced::Border {
            color: mix(off.border.color, on.border.color, t),
            width: off.border.width + (on.border.width - off.border.width) * t,
            radius: on.border.radius,
        },
        ..on
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curve_starts_at_zero_ends_at_one_and_never_leaves_the_range() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        // Out-of-range input is clamped rather than extrapolated: a colour blend
        // past either endpoint is a colour from no theme.
        assert_eq!(ease_out_cubic(-5.0), 0.0);
        assert_eq!(ease_out_cubic(9.0), 1.0);

        let mut prev = 0.0;
        for i in 0..=100 {
            let v = ease_out_cubic(i as f32 / 100.0);
            assert!((0.0..=1.0).contains(&v));
            assert!(v >= prev, "the curve went backwards at {i}");
            prev = v;
        }
    }

    #[test]
    fn the_curve_is_front_loaded() {
        // Ease-OUT: more than half the distance is covered in the first half of
        // the time. A linear ramp would fail this.
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    /// A phase nobody started is finished, not pending - the window's first
    /// frame draws its default tab selected rather than fading it in.
    #[test]
    fn an_unstarted_phase_is_already_finished() {
        let p = Phase::default();
        assert_eq!(p.linear(), 1.0);
        assert_eq!(p.eased(), 1.0);
        assert!(!p.running());
    }

    #[test]
    fn a_started_phase_runs_from_the_beginning() {
        let mut p = Phase::default();
        p.start();
        // Just started: near zero, and asking for frames.
        assert!(p.linear() < 0.5, "linear was {}", p.linear());
        assert!(p.running());
    }

    #[test]
    fn a_phase_finishes_by_itself_without_being_told() {
        let mut p = Phase::default();
        p.start();
        std::thread::sleep(std::time::Duration::from_millis((MS as u64) + 40));
        assert_eq!(p.linear(), 1.0);
        assert_eq!(p.eased(), 1.0);
        // And stops asking for frames, which is what lets the subscription drop.
        assert!(!p.running());
    }

    #[test]
    fn restarting_mid_flight_runs_the_new_transition_from_zero() {
        let mut p = Phase::default();
        p.start();
        std::thread::sleep(std::time::Duration::from_millis(120));
        let mid = p.linear();
        assert!(mid > 0.0 && mid < 1.0, "expected mid-flight, got {mid}");

        p.start();
        assert!(p.linear() < mid, "the restart did not go back to the start");
    }

    #[test]
    fn a_blend_hits_both_endpoints_and_moves_between_them() {
        let a = Color::from_rgb(0.0, 0.0, 0.0);
        let b = Color::from_rgb(1.0, 0.5, 0.25);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);

        let half = mix(a, b, 0.5);
        assert!((half.r - 0.5).abs() < 1e-6);
        assert!((half.g - 0.25).abs() < 1e-6);

        // Clamped, not extrapolated.
        assert_eq!(mix(a, b, 2.0), b);
        assert_eq!(mix(a, b, -1.0), a);
    }

    #[test]
    fn a_blend_carries_alpha_too() {
        let a = Color {
            a: 0.0,
            ..Color::WHITE
        };
        let b = Color::WHITE;
        assert_eq!(mix(a, b, 0.0).a, 0.0);
        assert_eq!(mix(a, b, 1.0).a, 1.0);
        assert!((mix(a, b, 0.5).a - 0.5).abs() < 1e-6);
    }

    /// Fading in from "no background" must start at the DESTINATION hue with no
    /// alpha. Starting from transparent black darkens every frame in between,
    /// which on the parchment palette reads as a grey flash.
    #[test]
    fn fading_in_from_no_background_does_not_go_through_black() {
        let to = iced::Background::Color(Color::from_rgb(0.9, 0.2, 0.2));
        let half = mix_bg(None, Some(to), 0.5).unwrap();
        let iced::Background::Color(c) = half else {
            panic!("not a colour")
        };

        assert!((c.a - 0.5).abs() < 1e-6, "alpha should be half way");
        // The hue is the destination's, not a blend with black.
        assert!((c.r - 0.9).abs() < 1e-6, "r drifted to {}", c.r);
        assert!((c.g - 0.2).abs() < 1e-6, "g drifted to {}", c.g);
    }

    #[test]
    fn the_strip_crossfades_rather_than_one_end_snapping() {
        // Mid-transition from Data to Saves.
        let t = 0.25;
        let arriving = tab_mix(t, &"saves", Some(&"data"), &"saves");
        let leaving = tab_mix(t, &"saves", Some(&"data"), &"data");
        let bystander = tab_mix(t, &"saves", Some(&"data"), &"plugins");

        assert_eq!(arriving, 0.25);
        assert_eq!(leaving, 0.75, "the tab being left must fade out, not snap");
        assert_eq!(bystander, 0.0);
        // The two ends always sum to one: what one gains the other gives up.
        assert!((arriving + leaving - 1.0).abs() < 1e-6);
    }

    #[test]
    fn with_no_transition_the_selected_tab_is_simply_selected() {
        // t = 1.0 is what a finished (or never-started) phase reports.
        assert_eq!(tab_mix(1.0, &"saves", None, &"saves"), 1.0);
        assert_eq!(tab_mix(1.0, &"saves", None, &"data"), 0.0);
        // Even with a previous tab recorded, a finished phase leaves it at zero.
        assert_eq!(tab_mix(1.0, &"saves", Some(&"data"), &"data"), 0.0);
    }

    #[test]
    fn the_endpoints_of_a_button_blend_are_the_untouched_originals() {
        let off = iced::widget::button::Style {
            text_color: Color::BLACK,
            ..Default::default()
        };
        let on = iced::widget::button::Style {
            text_color: Color::WHITE,
            ..Default::default()
        };
        // Pixel-identical to no animation at all at both ends - which is what
        // makes an idle window look exactly as it did before.
        assert_eq!(mix_button(off, on, 0.0).text_color, Color::BLACK);
        assert_eq!(mix_button(off, on, 1.0).text_color, Color::WHITE);
        let half = mix_button(off, on, 0.5).text_color;
        assert!((half.r - 0.5).abs() < 1e-6);
    }

    #[test]
    fn fading_out_to_no_background_is_the_mirror_image() {
        let from = iced::Background::Color(Color::from_rgb(0.9, 0.2, 0.2));
        let half = mix_bg(Some(from), None, 0.5).unwrap();
        let iced::Background::Color(c) = half else {
            panic!("not a colour")
        };
        assert!((c.a - 0.5).abs() < 1e-6);
        assert!((c.r - 0.9).abs() < 1e-6);
    }
}
