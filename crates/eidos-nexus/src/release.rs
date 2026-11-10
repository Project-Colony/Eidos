//! Whether a newer Eidos release is out: the daily GitHub check and the pure
//! decisions around it. Nothing here downloads or replaces Eidos.

use std::path::Path;
use std::time::Duration;

use eidos_instance::Settings;

/// The GitHub repository Eidos is released from.
pub const REPO: &str = "Project-Colony/Eidos";
/// The least time between two release requests, in seconds.
pub const DAY: u64 = 86_400;

/// The GitHub page of a release, with its changelog.
pub fn notes_url(version: &str) -> String {
    format!("https://github.com/{REPO}/releases/tag/v{version}")
}

/// A copy under `/usr` came from a package manager, which updates it.
/// `/usr/local` is where `install.sh --system` puts a user's own install.
pub fn package_managed(exe: &Path) -> bool {
    exe.starts_with("/usr") && !exe.starts_with("/usr/local")
}

/// Whether to ask GitHub now: the check is on, Eidos is online, the copy is
/// the user's own, and the last request is a day old or in the future (a clock
/// set back).
pub fn check_due(prefs: &Settings, package_managed: bool, now: u64) -> bool {
    prefs.release_check
        && !prefs.offline
        && !package_managed
        && (now < prefs.release_checked_at || now - prefs.release_checked_at >= DAY)
}

/// The cached newer version to link, if the check is on for this copy.
/// Offline mode only stops the request, so a cached notice still shows.
pub fn notice(prefs: &Settings, package_managed: bool, current: &str) -> Option<String> {
    if !prefs.release_check || package_managed {
        return None;
    }
    prefs
        .release_latest
        .clone()
        .filter(|v| is_newer(current, v))
}

/// `latest` is a higher dotted version than `current`. Anything that is not
/// dotted numbers is never newer.
pub fn is_newer(current: &str, latest: &str) -> bool {
    fn parts(v: &str) -> Option<Vec<u64>> {
        let mut n = v
            .trim_start_matches(['v', 'V'])
            .split('.')
            .map(|p| p.parse::<u64>().ok())
            .collect::<Option<Vec<_>>>()?;
        while n.last() == Some(&0) {
            n.pop();
        }
        Some(n)
    }
    matches!((parts(current), parts(latest)), (Some(c), Some(l)) if l > c)
}

/// The version of a published, final release, from GitHub's release JSON.
/// The trust boundary: the value is written to `settings.ini` and drawn in the
/// menu bar, so only 1 to 32 digits and inner dots get through.
pub(crate) fn tag_version(body: &serde_json::Value) -> Option<String> {
    let flag = |k| body.get(k).and_then(serde_json::Value::as_bool) == Some(true);
    if flag("draft") || flag("prerelease") {
        return None;
    }
    let tag = body.get("tag_name")?.as_str()?;
    let v = tag.strip_prefix(['v', 'V']).unwrap_or(tag);
    let plain = (1..=32).contains(&v.len())
        && v.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && !v.starts_with('.')
        && !v.ends_with('.');
    plain.then(|| v.to_string())
}

/// The newest Eidos release, from one GET of GitHub's `releases/latest`. It
/// reads the tag only and downloads nothing else. Never called from a test.
pub fn latest_version() -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .user_agent(format!("Eidos/{}", env!("CARGO_PKG_VERSION")))
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_body(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let mut resp = agent
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("GitHub answered HTTP {}", resp.status().as_u16()));
    }
    let body: serde_json::Value = resp.body_mut().read_json().map_err(|e| e.to_string())?;
    tag_version(&body).ok_or_else(|| "GitHub's latest release has no plain version tag".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_versions_compare_by_number() {
        assert!(is_newer("1.23.0", "1.24.0"));
        assert!(is_newer("1.23.0", "v1.23.1"));
        assert!(is_newer("1.23.0", "1.100.0"));
        assert!(!is_newer("1.23.0", "1.23.0"));
        assert!(!is_newer("1.23.0", "v1.23"));
        assert!(!is_newer("1.23.0", "1.9.9"));
        assert!(!is_newer("1.23.0", "garbage"));
        assert!(!is_newer("1.23.0", ""));
    }

    #[test]
    fn a_release_tag_that_is_not_a_plain_version_is_refused() {
        let tag = |t: &str| {
            tag_version(&serde_json::json!({"tag_name": t, "draft": false, "prerelease": false}))
        };
        assert_eq!(tag("v1.24.0").as_deref(), Some("1.24.0"));
        assert_eq!(tag("1.24.0").as_deref(), Some("1.24.0"));
        for bad in [
            "v1.24.0\nlock_gui=false",
            "../1.24",
            "v",
            ".1.2",
            "1.2.",
            "1.2.0-beta",
            &"9".repeat(200),
        ] {
            assert_eq!(tag(bad), None, "{bad:?} was accepted");
        }
        assert_eq!(
            tag_version(&serde_json::json!({"tag_name": "v2.0.0", "draft": true})),
            None
        );
        assert_eq!(
            tag_version(&serde_json::json!({"tag_name": "v2.0.0", "prerelease": true})),
            None
        );
        assert_eq!(tag_version(&serde_json::json!({"name": "v2.0.0"})), None);
    }

    #[test]
    fn only_a_package_manager_install_skips_the_release_check() {
        assert!(package_managed(Path::new("/usr/bin/eidos-gui")));
        assert!(!package_managed(Path::new("/usr/local/bin/eidos-gui")));
        assert!(!package_managed(Path::new(
            "/home/someone/.local/bin/eidos-gui"
        )));
        assert!(!package_managed(Path::new("/usrx/bin/eidos-gui")));
    }

    /// Default settings changed by `f`. A struct literal cannot fill
    /// `Settings`'s private `path` from this crate.
    fn settings(f: impl FnOnce(&mut Settings)) -> Settings {
        let mut s = Settings::default();
        f(&mut s);
        s
    }

    #[test]
    fn the_release_check_runs_at_most_once_a_day() {
        let now = 10 * DAY;
        let prefs = |checked_at| settings(|s| s.release_checked_at = checked_at);
        assert!(check_due(&prefs(0), false, now));
        assert!(!check_due(&prefs(now - 3600), false, now));
        assert!(check_due(&prefs(now - DAY), false, now));
        assert!(
            check_due(&prefs(now + DAY), false, now),
            "a clock set back must not stop the check for good"
        );
        assert!(!check_due(&prefs(0), true, now));
        assert!(!check_due(&settings(|s| s.offline = true), false, now));
        assert!(!check_due(
            &settings(|s| s.release_check = false),
            false,
            now
        ));
    }

    #[test]
    fn the_notice_needs_the_check_on_and_a_newer_cached_release() {
        let cached = settings(|s| s.release_latest = Some("1.24.0".into()));
        assert_eq!(notice(&cached, false, "1.23.0").as_deref(), Some("1.24.0"));
        assert_eq!(notice(&cached, false, "1.24.0"), None);
        assert_eq!(notice(&cached, true, "1.23.0"), None);
        let mut off = cached.clone();
        off.release_check = false;
        assert_eq!(notice(&off, false, "1.23.0"), None);
        assert_eq!(notice(&Settings::default(), false, "1.23.0"), None);
    }

    #[test]
    fn release_notes_link_the_tag_page() {
        assert_eq!(
            notes_url("1.24.0"),
            "https://github.com/Project-Colony/Eidos/releases/tag/v1.24.0"
        );
    }
}
