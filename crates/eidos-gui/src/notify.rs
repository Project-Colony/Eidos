//! Desktop notifications for a long job that ends while the window is in the
//! background (Preferences > Running, off by default).

use crate::{App, DownloadState};

/// Tell the desktop that a long job ended, when the user asked for it and the
/// window does not have the focus.
pub(crate) fn when_away(app: &App, summary: &str, body: &str) {
    if app.prefs.notify_when_away && !app.focused {
        send(summary.to_string(), body.to_string());
    }
}

/// Whether the last running download has just ended: `before` had one
/// arriving, `after` has none, and none of those stalled or was paused.
pub(crate) fn downloads_finished(
    before: &[(String, DownloadState)],
    after: &[(String, DownloadState)],
) -> bool {
    let was_arriving = |s: &DownloadState| *s == DownloadState::Downloading;
    before.iter().any(|(_, s)| was_arriving(s))
        && !after.iter().any(|(_, s)| was_arriving(s))
        && !after.iter().any(|(n, s)| {
            matches!(s, DownloadState::Stalled | DownloadState::Paused)
                && before.iter().any(|(b, bs)| b == n && was_arriving(bs))
        })
}

/// One `org.freedesktop.Notifications.Notify` call on its own thread. No
/// notification daemon means no notification, and nothing else.
#[cfg(not(test))]
fn send(summary: String, body: String) {
    std::thread::spawn(move || {
        let _ = smol::block_on(async {
            let conn = zbus::Connection::session().await?;
            conn.call_method(
                Some("org.freedesktop.Notifications"),
                "/org/freedesktop/Notifications",
                Some("org.freedesktop.Notifications"),
                "Notify",
                &(
                    "Eidos",
                    0u32,
                    "eidos",
                    summary,
                    body,
                    Vec::<&str>::new(),
                    std::collections::HashMap::<&str, zbus::zvariant::Value>::new(),
                    -1i32,
                ),
            )
            .await?;
            Ok::<_, zbus::Error>(())
        });
    });
}

// What tests would have sent, so no test reaches the session bus. Per thread:
// each test runs on its own, so two tests that notify cannot drain each other.
#[cfg(test)]
thread_local! {
    static SENT: std::cell::RefCell<Vec<(String, String)>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn send(summary: String, body: String) {
    SENT.with_borrow_mut(|s| s.push((summary, body)));
}

/// Drain the notifications this thread sent so far.
#[cfg(test)]
pub(crate) fn take_sent() -> Vec<(String, String)> {
    SENT.take()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn many_downloads_finishing_give_one_notification() {
        use DownloadState::*;
        let rows = |v: &[(&str, DownloadState)]| {
            v.iter()
                .map(|(n, s)| (n.to_string(), *s))
                .collect::<Vec<_>>()
        };
        let forty: Vec<_> = (0..40).map(|i| (format!("m{i}.7z"), Downloading)).collect();
        let mut one_left = forty.clone();
        for r in one_left.iter_mut().skip(1) {
            r.1 = Ready;
        }
        let all_done: Vec<_> = forty.iter().map(|(n, _)| (n.clone(), Ready)).collect();
        let ticks = [
            (&forty, &one_left),
            (&one_left, &all_done),
            (&all_done, &all_done),
        ];
        assert_eq!(
            ticks
                .iter()
                .filter(|(b, a)| downloads_finished(b, a))
                .count(),
            1
        );
        let before = rows(&[("a.7z", Downloading), ("b.7z", Downloading)]);
        assert!(
            !downloads_finished(&before, &rows(&[("a.7z", Ready), ("b.7z", Stalled)])),
            "a stall is not done"
        );
        assert!(
            !downloads_finished(&before, &rows(&[("a.7z", Ready), ("b.7z", Paused)])),
            "a pause is not done"
        );
    }
}
