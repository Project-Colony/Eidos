//! `EIDOS_GUI_TIMING=1`: log the GUI steps that take longer than a frame.
//!
//! The daemon has `EIDOS_FUSE_STATS`; the window had no way to time its own
//! work on the binary a user actually runs. The shipped build is stripped, so a
//! profiler shows addresses, and iced's `debug` feature (comet) needs a dev
//! build. This writes to the GUI's own log instead, and splits out the
//! Eidos-specific steps that run after every message. Layout and drawing happen
//! inside iced and are not covered.
//!
//! Off, it costs one read of a static per message and per frame.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Read once: the switch cannot change under a running window.
static ON: LazyLock<bool> = LazyLock::new(|| {
    std::env::var_os("EIDOS_GUI_TIMING").is_some_and(|v| !v.is_empty() && v != "0")
});

/// A step slower than this missed a 60 Hz frame.
const SLOW: Duration = Duration::from_millis(16);
/// Slow-step lines per second at most. An animation ticks every 16 ms, and a
/// tick that is slow every time would otherwise write sixty lines a second.
const LINES_PER_SECOND: u32 = 8;

pub(crate) fn on() -> bool {
    *ON
}

/// The variant name of `message`, and nothing of its payload.
///
/// Payloads carry account names, typed text and paths, and some are large
/// enough that formatting them would itself take time. The derived `Debug`
/// writes the name first and then the fields; the writer refuses the first
/// character after the name, which stops the formatter before any field is
/// formatted.
pub(crate) fn variant(message: &impl std::fmt::Debug) -> String {
    struct NameOnly(String);
    impl std::fmt::Write for NameOnly {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            for c in s.chars() {
                if !(c.is_alphanumeric() || c == '_') || self.0.len() >= 64 {
                    return Err(std::fmt::Error);
                }
                self.0.push(c);
            }
            Ok(())
        }
    }
    let mut name = NameOnly(String::new());
    let _ = write!(name, "{message:?}");
    name.0
}

#[derive(Default)]
struct Window {
    started: Option<Instant>,
    lines: u32,
    dropped: u32,
    views: u32,
    slowest_view: Duration,
}

thread_local! {
    // The update and view callbacks both run on the UI thread.
    static WINDOW: RefCell<Window> = RefCell::new(Window::default());
}

/// Close the current one-second window if it is over, logging its summary.
fn roll(w: &mut Window, now: Instant) {
    let start = *w.started.get_or_insert(now);
    if now.duration_since(start) < Duration::from_secs(1) {
        return;
    }
    if w.views > 0 || w.dropped > 0 {
        eidos_log::info!(
            "gui timing: {} views in {:.1} s, slowest {} ms, {} slow steps not logged",
            w.views,
            now.duration_since(start).as_secs_f64(),
            w.slowest_view.as_millis(),
            w.dropped
        );
    }
    *w = Window {
        started: Some(now),
        ..Window::default()
    };
}

/// Report `step` (for message `what`) if it ran past a frame since `since`.
pub(crate) fn step(step: &str, what: &str, since: Instant) {
    let now = Instant::now();
    let took = now.duration_since(since);
    WINDOW.with_borrow_mut(|w| {
        roll(w, now);
        if took < SLOW {
            return;
        }
        if w.lines < LINES_PER_SECOND {
            w.lines += 1;
            eidos_log::info!("gui timing: {step} for {what} took {} ms", took.as_millis());
        } else {
            w.dropped += 1;
        }
    });
}

/// Run `f` as `step` of message `what`, timed when the switch is on (`what`
/// is `None` when it is off).
pub(crate) fn time<R>(step_name: &str, what: Option<&str>, f: impl FnOnce() -> R) -> R {
    let Some(what) = what else {
        return f();
    };
    let since = Instant::now();
    let out = f();
    step(step_name, what, since);
    out
}

/// The iced view callback: [`crate::view`], counted and timed when the switch
/// is on. Building the widget tree only; iced lays it out and draws it after.
pub(crate) fn timed_view(app: &crate::App) -> crate::Element<'_, crate::Message> {
    if !on() {
        return crate::view(app);
    }
    let since = Instant::now();
    let tree = crate::view(app);
    view_done(since);
    tree
}

/// Count one `view()` call that started at `since`, slow or not.
fn view_done(since: Instant) {
    let now = Instant::now();
    let took = now.duration_since(since);
    WINDOW.with_borrow_mut(|w| {
        roll(w, now);
        w.views += 1;
        w.slowest_view = w.slowest_view.max(took);
    });
    step("view", "frame", since);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    #[allow(dead_code)]
    enum Sample {
        Unit,
        Tuple(String),
        Fields { account: String },
    }

    #[test]
    fn only_the_variant_name_reaches_the_log() {
        assert_eq!(variant(&Sample::Unit), "Unit");
        let secret = "alice /home/alice/mods".to_string();
        assert_eq!(variant(&Sample::Tuple(secret.clone())), "Tuple");
        assert_eq!(variant(&Sample::Fields { account: secret }), "Fields");
        // And on the real enum, whose payloads are what must stay out.
        assert_eq!(
            variant(&crate::Message::NameChanged("alice".into())),
            "NameChanged"
        );
    }
}
