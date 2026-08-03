//! Persisted GLOBAL app settings, shared by the CLI and the GUI.
//!
//! Two files under the XDG dirs, both our own minimal `key=value` INI dialect
//! (not MO2's, distinct from the per-mod `meta.ini`):
//!
//! - `$XDG_CONFIG_HOME/eidos/nexus.ini` holds only the personal Nexus API key.
//!   It is kept separate, and at the path the CLI already uses, so the key the
//!   user stored with `eidos nexus key` is the same one the GUI sees - the key
//!   survives across sessions and across tools.
//! - `$XDG_CONFIG_HOME/eidos/settings.ini` holds the rest of the app-global
//!   preferences (theme, default game id, last window size), none of which is
//!   secret.
//!
//! Everything here is process-global, not per-instance: per-instance state lives
//! in the instance manifest (`manifest.rs`) instead.

use std::fs;
use std::io;
use std::path::PathBuf;

/// `$XDG_CONFIG_HOME`, or `$HOME/.config`.
pub fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
            home.join(".config")
        })
}

/// `$XDG_CONFIG_HOME/eidos/nexus.ini`, holding the Nexus OAuth session. Same
/// path the CLI uses, so a sign-in is shared between the CLI and the GUI.
pub fn nexus_key_path() -> PathBuf {
    config_home().join("eidos").join("nexus.ini")
}

/// Everything `nexus.ini` can hold: the OAuth session, and nothing else.
///
/// It used to carry a personal API key beside the tokens. Nexus's API team
/// requires personal keys removed from a distributed client entirely - not
/// merely unused - so the field is gone rather than ignored, and an `api_key=`
/// line left over from an older version is passed through untouched but never
/// read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NexusCreds {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    /// Unix seconds. Absolute, so it survives being written and read back.
    pub expires_at: u64,
}

impl NexusCreds {
    /// Whether a stored sign-in is present at all. Says nothing about whether
    /// the access token is still fresh - that is `expires_at`'s job.
    pub fn has_oauth(&self) -> bool {
        self.access_token.as_ref().is_some_and(|t| !t.is_empty())
    }
}

/// Read every credential in `nexus.ini`. A missing or unreadable file reads as
/// "nothing stored", which is the same thing from the caller's point of view.
pub fn load_nexus_creds() -> NexusCreds {
    let text = fs::read_to_string(nexus_key_path()).unwrap_or_default();
    parse_nexus_creds(&text)
}

/// Write the credentials back, preserving any key we do not know about so a
/// newer Eidos writing a field an older one drops does not lose it.
pub fn save_nexus_creds(creds: &NexusCreds) -> io::Result<()> {
    let path = nexus_key_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = fs::read_to_string(&path).unwrap_or_default();
    fs::write(&path, render_nexus_creds(&existing, creds))?;
    // These are credentials: keep them out of other users' reach (0600), the
    // same as ssh does. Applied after the write so a pre-existing 0644 file from
    // an older Eidos gets tightened too.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Forget the OAuth sign-in, keeping any API key. What "Sign out" does.
pub fn clear_nexus_tokens() -> io::Result<()> {
    let mut creds = load_nexus_creds();
    creds.access_token = None;
    creds.refresh_token = None;
    creds.expires_at = 0;
    save_nexus_creds(&creds)
}

/// The three fields this module owns; anything else in the file is passed
/// through untouched by [`render_nexus_creds`] - including an `api_key=` line
/// from a version that still had one, which is neither read nor deleted.
const OWNED_KEYS: [&str; 3] = ["access_token", "refresh_token", "expires_at"];

fn parse_nexus_creds(text: &str) -> NexusCreds {
    let val = |want: &str| {
        text.lines()
            .filter_map(|l| l.trim().split_once('='))
            .find(|(k, _)| k.trim() == want)
            .map(|(_, v)| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    NexusCreds {
        access_token: val("access_token"),
        refresh_token: val("refresh_token"),
        expires_at: val("expires_at").and_then(|v| v.parse().ok()).unwrap_or(0),
    }
}

/// Serialise `creds` into a `[Nexus]` body, carrying over any line from
/// `existing` whose key is not one of ours. Split out so the merge is testable
/// without touching the filesystem.
fn render_nexus_creds(existing: &str, creds: &NexusCreds) -> String {
    let mut out = String::from("[Nexus]\n");
    let mut push = |k: &str, v: &str| {
        if !v.is_empty() {
            out.push_str(k);
            out.push('=');
            out.push_str(v);
            out.push('\n');
        }
    };
    push("access_token", creds.access_token.as_deref().unwrap_or_default().trim());
    push("refresh_token", creds.refresh_token.as_deref().unwrap_or_default().trim());
    if creds.expires_at != 0 {
        push("expires_at", &creds.expires_at.to_string());
    }
    for line in existing.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('[') || t.starts_with('#') {
            continue;
        }
        if let Some((k, _)) = t.split_once('=') {
            if !OWNED_KEYS.contains(&k.trim()) {
                out.push_str(t);
                out.push('\n');
            }
        }
    }
    out
}

/// `$XDG_CONFIG_HOME/eidos/settings.ini`, holding the non-secret app-global
/// preferences (theme, default game, last window size).
pub fn settings_path() -> PathBuf {
    config_home().join("eidos").join("settings.ini")
}

/// Which color theme the app renders in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Follow the platform, or fall back to dark.
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    /// The on-disk token (lowercase, stable across releases).
    fn as_str(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    /// Parse a stored token; unknown values fall back to the default.
    fn parse(s: &str) -> Theme {
        match s.trim().to_ascii_lowercase().as_str() {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => Theme::System,
        }
    }
}

/// The app-global preferences that are not secret. The Nexus API key is stored
/// separately (see `load_nexus_key`/`save_nexus_key`) so it never lands in this
/// file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The color theme to render in.
    pub theme: Theme,
    /// The game id to open by default when none is given (e.g. `skyrimse`).
    pub default_game: Option<String>,
    /// The last window size in logical pixels, `(width, height)`.
    pub window_size: Option<(u32, u32)>,
    /// Lock the GUI while a launched game/tool runs (MO2's `[Settings] lock_gui`,
    /// on by default): the main window is blocked behind an overlay until the
    /// process exits, with an Unlock escape hatch.
    pub lock_gui: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::default(),
            default_game: None,
            window_size: None,
            // MO2 defaults `lock_gui` to true, and an absent key means "on".
            lock_gui: true,
        }
    }
}

impl Settings {
    /// Load the persisted settings, or the defaults if the file is absent or
    /// unreadable (so the app always has something to start from).
    pub fn load() -> Settings {
        match fs::read_to_string(settings_path()) {
            Ok(text) => Settings::parse(&text),
            Err(_) => Settings::default(),
        }
    }

    /// Persist these settings, creating the parent directory if needed.
    pub fn save(&self) -> io::Result<()> {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, self.to_ini())
    }

    /// Parse a `settings.ini` body. Unknown keys are ignored; missing or invalid
    /// values fall back to the field default. Split out for unit tests.
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('[') || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let v = v.trim();
            match k.trim() {
                "theme" => s.theme = Theme::parse(v),
                "default_game" if !v.is_empty() => s.default_game = Some(v.to_string()),
                "window_width" => width = v.parse().ok(),
                "window_height" => height = v.parse().ok(),
                // Any value other than an explicit off keeps locking on (default).
                "lock_gui" => {
                    s.lock_gui = !matches!(v.to_ascii_lowercase().as_str(), "false" | "0" | "no" | "off")
                }
                _ => {}
            }
        }
        // Only a complete, non-zero pair is a usable window size.
        if let (Some(w), Some(h)) = (width, height) {
            if w > 0 && h > 0 {
                s.window_size = Some((w, h));
            }
        }
        s
    }

    /// Render these settings as a `settings.ini` body. Split out for unit tests.
    pub fn to_ini(&self) -> String {
        let mut out = format!("[eidos]\ntheme={}\nlock_gui={}\n", self.theme.as_str(), self.lock_gui);
        if let Some(game) = &self.default_game {
            out.push_str("default_game=");
            out.push_str(game);
            out.push('\n');
        }
        if let Some((w, h)) = self.window_size {
            out.push_str(&format!("window_width={w}\nwindow_height={h}\n"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    fn tmp(label: &str) -> PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("eidos-settings-{}-{}-{}.ini", label, std::process::id(), n))
    }

    #[test]
    fn an_old_api_key_line_is_neither_read_nor_destroyed() {
        // `nexus.ini` used to hold a personal API key beside the tokens. Nexus
        // requires personal keys gone from the client, so the field is no longer
        // parsed - but a user upgrading has that line on disk, and silently
        // rewriting their file to drop it is not this module's call.
        let existing = "[Nexus]\napi_key=old\naccess_token=at\nrefresh_token=rt\nexpires_at=999\n";
        let creds = parse_nexus_creds(existing);
        let out = render_nexus_creds(existing, &creds);
        assert!(out.contains("api_key=old"), "the line was destroyed: {out}");
        let back = parse_nexus_creds(&out);
        assert_eq!(back.access_token.as_deref(), Some("at"));
        assert_eq!(back.refresh_token.as_deref(), Some("rt"));
        assert_eq!(back.expires_at, 999);
    }

    #[test]
    fn signing_out_clears_the_session_and_nothing_else() {
        let existing = "[Nexus]\naccess_token=at\nrefresh_token=rt\nexpires_at=5\nkeep=me\n";
        let mut creds = parse_nexus_creds(existing);
        creds.access_token = None;
        creds.refresh_token = None;
        creds.expires_at = 0;
        let out = render_nexus_creds(existing, &creds);
        let back = parse_nexus_creds(&out);
        assert!(!back.has_oauth());
        assert_eq!(back.expires_at, 0);
        assert!(out.contains("keep=me"), "an unrelated line was dropped: {out}");
    }

    #[test]
    fn unknown_keys_survive_a_rewrite() {
        // A newer Eidos may store a field this build knows nothing about;
        // dropping it on every save would corrupt their config by downgrade.
        let existing = "[Nexus]\naccess_token=at\nsomething_new=keep-me\n";
        let out = render_nexus_creds(existing, &parse_nexus_creds(existing));
        assert!(out.contains("something_new=keep-me"), "{out}");
        assert_eq!(out.matches("[Nexus]").count(), 1, "the header was duplicated: {out}");
    }

    #[test]
    fn an_empty_file_reads_as_nothing_stored() {
        let c = parse_nexus_creds("");
        assert_eq!(c, NexusCreds::default());
        assert!(!c.has_oauth());
    }

    #[test]
    fn the_session_survives_a_disk_round_trip() {
        let path = tmp("nexus");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&path, "[Nexus]\naccess_token=persisted\nexpires_at=42\n").unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let c = parse_nexus_creds(&text);
        assert_eq!(c.access_token.as_deref(), Some("persisted"));
        assert_eq!(c.expires_at, 42);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn theme_round_trips_each_variant() {
        for theme in [Theme::System, Theme::Light, Theme::Dark] {
            assert_eq!(Theme::parse(theme.as_str()), theme);
        }
    }

    #[test]
    fn theme_parse_is_case_insensitive_and_falls_back() {
        assert_eq!(Theme::parse("DARK"), Theme::Dark);
        assert_eq!(Theme::parse("Light"), Theme::Light);
        assert_eq!(Theme::parse("nonsense"), Theme::System);
        assert_eq!(Theme::parse(""), Theme::System);
    }

    #[test]
    fn defaults_are_empty() {
        let s = Settings::default();
        assert_eq!(s.theme, Theme::System);
        assert_eq!(s.default_game, None);
        assert_eq!(s.window_size, None);
        assert!(s.lock_gui, "locking the GUI during a run is on by default (MO2 parity)");
    }

    #[test]
    fn settings_round_trip_full() {
        let s = Settings {
            theme: Theme::Dark,
            default_game: Some("skyrimse".to_string()),
            window_size: Some((1280, 720)),
            lock_gui: false,
        };
        let parsed = Settings::parse(&s.to_ini());
        assert_eq!(parsed, s);
    }

    #[test]
    fn settings_round_trip_minimal() {
        let s = Settings { theme: Theme::Light, default_game: None, window_size: None, lock_gui: true };
        let parsed = Settings::parse(&s.to_ini());
        assert_eq!(parsed, s);
    }

    #[test]
    fn lock_gui_defaults_on_and_parses_off() {
        // Absent key -> on (MO2 default).
        assert!(Settings::parse("[eidos]\ntheme=dark\n").lock_gui);
        // Explicit off forms.
        for off in ["false", "0", "no", "off", "OFF"] {
            assert!(!Settings::parse(&format!("lock_gui={off}\n")).lock_gui, "{off} should disable");
        }
        // Anything else stays on.
        assert!(Settings::parse("lock_gui=true\n").lock_gui);
    }

    #[test]
    fn parse_ignores_unknown_and_malformed() {
        let text = "[eidos]\ntheme=dark\nbogus_key=whatever\nno-equals-sign\ndefault_game=fallout4\n";
        let s = Settings::parse(text);
        assert_eq!(s.theme, Theme::Dark);
        assert_eq!(s.default_game.as_deref(), Some("fallout4"));
    }

    #[test]
    fn parse_rejects_partial_or_zero_window_size() {
        // width only -> no size
        assert_eq!(Settings::parse("[eidos]\nwindow_width=1024\n").window_size, None);
        // a zero dimension -> no size
        assert_eq!(Settings::parse("window_width=0\nwindow_height=600\n").window_size, None);
        // non-numeric -> no size
        assert_eq!(Settings::parse("window_width=wide\nwindow_height=600\n").window_size, None);
    }

    #[test]
    fn empty_default_game_is_none() {
        let s = Settings::parse("[eidos]\ndefault_game=\n");
        assert_eq!(s.default_game, None);
    }

    #[test]
    fn settings_survive_a_disk_round_trip() {
        let path = tmp("settings");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let s = Settings {
            theme: Theme::Dark,
            default_game: Some("starfield".to_string()),
            window_size: Some((1600, 900)),
            lock_gui: false,
        };
        fs::write(&path, s.to_ini()).unwrap();
        let loaded = Settings::parse(&fs::read_to_string(&path).unwrap());
        assert_eq!(loaded, s);
        let _ = fs::remove_file(&path);
    }
}
