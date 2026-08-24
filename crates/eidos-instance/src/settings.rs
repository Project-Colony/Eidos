//! Persisted GLOBAL app settings, shared by the CLI and the GUI.
//!
//! Two files under the XDG dirs, both our own minimal `key=value` INI dialect
//! (not MO2's, distinct from the per-mod `meta.ini`):
//!
//! - `~/.config/Colony/Eidos/nexus.ini` holds only the personal Nexus API key.
//!   It is kept separate, and at the path the CLI already uses, so the key the
//!   user stored with `eidos nexus key` is the same one the GUI sees - the key
//!   survives across sessions and across tools.
//! - `~/.config/Colony/Eidos/settings.ini` holds the rest of the app-global
//!   preferences (theme, default game id, last window size), none of which is
//!   secret.
//!
//! Everything here is process-global, not per-instance: per-instance state lives
//! in the instance manifest (`manifest.rs`) instead.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Eidos's config directory: `~/.config/Colony/Eidos`.
///
/// One line now, because four crates used to answer this themselves and by
/// hand - see `eidos_paths`, which is where the answer lives.
pub fn config_home() -> PathBuf {
    eidos_paths::config_dir()
}

/// `~/.config/Colony/Eidos/nexus.ini`, holding the Nexus OAuth session. Same
/// path the CLI uses, so a sign-in is shared between the CLI and the GUI.
pub fn nexus_key_path() -> PathBuf {
    config_home().join("nexus.ini")
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
    /// The account's "show adult content" preference, as last read from Nexus.
    /// `None` means "not known", which is NOT the same as `Some(false)`: unknown
    /// hides adult metadata AND tells the user we could not check, where a known
    /// `false` is the user's own setting. See `eidos_nexus::AdultPolicy`.
    pub adult_ok: Option<bool>,
    /// Unix seconds when [`Self::adult_ok`] was read. A stale answer decays back
    /// to "unknown" rather than being trusted forever - the user can change the
    /// setting on the site at any time, and we would never hear about it.
    pub adult_checked_at: u64,
}

/// How long a cached adult-content preference is trusted before it decays to
/// "unknown". A judgement call, not a documented requirement: long enough that a
/// day's use costs one extra request, short enough that turning the setting off
/// on the website takes effect the next day without signing in again.
pub const ADULT_PREF_TTL: u64 = 24 * 3600;

impl NexusCreds {
    /// Whether a stored sign-in is present at all. Says nothing about whether
    /// the access token is still fresh - that is `expires_at`'s job.
    pub fn has_oauth(&self) -> bool {
        self.access_token.as_ref().is_some_and(|t| !t.is_empty())
    }

    /// The cached adult-content preference, or `None` once it has aged out.
    pub fn adult_pref(&self, now: u64) -> Option<bool> {
        let fresh = now.saturating_sub(self.adult_checked_at) < ADULT_PREF_TTL;
        self.adult_ok.filter(|_| fresh && self.adult_checked_at != 0)
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
    let body = render_nexus_creds(&existing, creds);
    // These are credentials: 0600 from the very first byte, like ssh. The old
    // write-then-chmod left a window - and, if the process died between the
    // two, a permanent state - where a FRESH nexus.ini sat at the umask default
    // with the tokens inside. Written to a sibling temp born 0600 and renamed
    // over, which also tightens a pre-existing looser file (the rename replaces
    // its inode, permissions and all) and makes the write atomic.
    // Unique per process AND per call. A fixed name is not atomic against a
    // second WRITER - two processes interleave into one temp and the last rename
    // publishes the mixture - and here it is worse than elsewhere: the comment
    // below about a leftover keeping its old permission bits describes exactly
    // that collision, and with a unique name it cannot happen at all.
    let tmp = path.with_extension(format!(
        "ini.eidos-tmp.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    {
        use std::io::Write;
        let mut opts = fs::OpenOptions::new();
        opts.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp)?;
        f.write_all(body.as_bytes())?;
    }
    // `mode` only applies when the temp is CREATED; a leftover from a crashed
    // attempt keeps its old bits, so tighten explicitly before publishing.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
    }
    match fs::rename(&tmp, &path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Forget the OAuth sign-in, keeping any API key. What "Sign out" does.
pub fn clear_nexus_tokens() -> io::Result<()> {
    let mut creds = load_nexus_creds();
    creds.access_token = None;
    creds.refresh_token = None;
    creds.expires_at = 0;
    // The adult-content preference belongs to the account that just signed out.
    // Keeping it would apply one person's setting to whoever signs in next.
    creds.adult_ok = None;
    creds.adult_checked_at = 0;
    save_nexus_creds(&creds)
}

/// The fields this module owns; anything else in the file is passed through
/// untouched by [`render_nexus_creds`] - including an `api_key=` line from a
/// version that still had one, which is neither read nor deleted.
const OWNED_KEYS: [&str; 5] =
    ["access_token", "refresh_token", "expires_at", "adult_ok", "adult_checked_at"];

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
        // Anything that is not a clean `1`/`0` reads as "unknown", which hides
        // adult content: a corrupted line must not be able to grant permission.
        adult_ok: val("adult_ok").and_then(|v| match v.as_str() {
            "1" | "true" => Some(true),
            "0" | "false" => Some(false),
            _ => None,
        }),
        adult_checked_at: val("adult_checked_at").and_then(|v| v.parse().ok()).unwrap_or(0),
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
    if let Some(ok) = creds.adult_ok {
        push("adult_ok", if ok { "1" } else { "0" });
        push("adult_checked_at", &creds.adult_checked_at.to_string());
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

/// `~/.config/Colony/Eidos/settings.ini`, holding the non-secret app-global
/// preferences (theme, default game, last window size).
pub fn settings_path() -> PathBuf {
    config_home().join("settings.ini")
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
// No `Eq`: `drag_scroll_speed` is a float, and a total equality on floats is a
// promise this type cannot keep.
#[derive(Debug, Clone, PartialEq)]
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
    /// Multiplier on how fast the mod list scrolls when a drag rests on one of
    /// its edges. 1.0 is the tuned default; the range a user can pick from is
    /// the GUI's business, this only stores what they picked.
    pub drag_scroll_speed: f32,
    /// Restore the window to its last size on launch (on by default). Off means
    /// the size is neither read nor written, so the compositor decides.
    pub remember_window: bool,
    /// Cut every Nexus request. MO2's "offline mode": nothing reaches the
    /// network, and the parts of the window that would have asked say why
    /// rather than failing with a connection error.
    pub offline: bool,
    /// CDN node names, best first, one per line in the file. Empty means
    /// "whatever Nexus offers first". Only a premium account is ever given more
    /// than one mirror to choose between.
    pub preferred_servers: Vec<String>,
    /// The file these settings were read from, and the ONLY file `save` will
    /// write. `None` for a value that was never loaded from disk - a default,
    /// or a fixture - which therefore cannot save over anybody's real
    /// preferences. Private so nothing can set it except by going through
    /// `load_from` or `with_path`.
    path: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::default(),
            default_game: None,
            window_size: None,
            // MO2 defaults `lock_gui` to true, and an absent key means "on".
            lock_gui: true,
            drag_scroll_speed: 1.0,
            remember_window: true,
            offline: false,
            preferred_servers: Vec::new(),
            // Nowhere. A default that could save itself over the real file is
            // exactly the defect this field exists to remove.
            path: None,
        }
    }
}

impl Settings {
    /// Load the persisted settings, or the defaults if the file is absent or
    /// unreadable (so the app always has something to start from).
    pub fn load() -> Settings {
        Settings::load_from(&settings_path())
    }

    /// Load from an explicit file, and REMEMBER it, so [`Settings::save`] writes
    /// back where it read from.
    ///
    /// This is what stops a test writing the user's real preferences. It is not
    /// hypothetical: with the path resolved globally inside `save`, a GUI test
    /// that dispatched "toggle lock" wrote `~/.config/.../settings.ini` on the
    /// developer's own machine, and the symptom - options reverting after a
    /// build - looked like anything but a test.
    pub fn load_from(path: &Path) -> Settings {
        let mut s = match fs::read_to_string(path) {
            Ok(text) => Settings::parse(&text),
            Err(_) => Settings::default(),
        };
        s.path = Some(path.to_path_buf());
        s
    }

    /// Persist these settings, creating the parent directory if needed.
    ///
    /// Writes to the file these settings were LOADED from. A `Settings` built
    /// with [`Settings::default`] - which is what a test gets - has no path and
    /// writing it is a no-op rather than a write to the real one.
    pub fn save(&self) -> io::Result<()> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_ini())
    }

    /// Point these settings at a file, for a test that WANTS to exercise saving.
    pub fn with_path(mut self, path: &Path) -> Settings {
        self.path = Some(path.to_path_buf());
        self
    }

    /// Where these settings would be written, if anywhere.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
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
                "drag_scroll_speed" => {
                    // Out-of-range or unparseable reads as the default rather
                    // than as a value: a bad number here is a list that either
                    // will not move or cannot be aimed.
                    if let Ok(n) = v.parse::<f32>() {
                        if (0.25..=4.0).contains(&n) {
                            s.drag_scroll_speed = n;
                        }
                    }
                }
                // Off unless explicitly on - the opposite default to lock_gui,
                // and deliberately: a key nobody wrote must not cut the network.
                "offline" => {
                    s.offline = matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on")
                }
                // Comma-separated so one line holds an ordering, which is what
                // this is - a list where position means something.
                "preferred_servers" => {
                    s.preferred_servers = v
                        .split(',')
                        .map(str::trim)
                        .filter(|x| !x.is_empty())
                        .map(str::to_string)
                        .collect();
                }
                "remember_window" => {
                    s.remember_window =
                        !matches!(v.to_ascii_lowercase().as_str(), "false" | "0" | "no" | "off")
                }
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
        let mut out = format!(
            "[eidos]\ntheme={}\nlock_gui={}\ndrag_scroll_speed={}\nremember_window={}\n",
            self.theme.as_str(),
            self.lock_gui,
            self.drag_scroll_speed,
            self.remember_window
        );
        if self.offline {
            out.push_str("offline=true\n");
        }
        if !self.preferred_servers.is_empty() {
            out.push_str(&format!("preferred_servers={}\n", self.preferred_servers.join(",")));
        }
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
    fn a_settings_file_from_before_the_marks_switch_still_loads() {
        // `conflict_marks` was a real key until the scrollbar marks became
        // unconditional. Files in the wild still carry it, and an unknown key
        // must be ignored rather than throw the whole file away - the settings
        // beside it are the user's and have to survive the upgrade.
        let s = Settings::parse(
            "[eidos]\ntheme=dark\nlock_gui=false\nconflict_marks=false\nremember_window=false\n",
        );
        assert_eq!(s.theme, Theme::Dark);
        assert!(!s.lock_gui);
        assert!(!s.remember_window);
        // And the key is not written back: nothing reads it any more.
        assert!(!s.to_ini().contains("conflict_marks"));
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
    fn the_adult_preference_survives_a_write_and_a_read() {
        let mut creds = parse_nexus_creds("[Nexus]\naccess_token=at\n");
        creds.adult_ok = Some(true);
        creds.adult_checked_at = 1_000;
        let back = parse_nexus_creds(&render_nexus_creds("", &creds));
        assert_eq!(back.adult_ok, Some(true));
        assert_eq!(back.adult_checked_at, 1_000);
        assert_eq!(back.adult_pref(1_000), Some(true));
    }

    #[test]
    fn a_cached_adult_preference_decays_to_unknown_once_it_is_stale() {
        // The user can turn the setting off on the website at any time and we
        // would never hear about it, so a cached "yes" has to expire.
        let creds = NexusCreds { adult_ok: Some(true), adult_checked_at: 1_000, ..Default::default() };
        assert_eq!(creds.adult_pref(1_000 + ADULT_PREF_TTL - 1), Some(true));
        assert_eq!(creds.adult_pref(1_000 + ADULT_PREF_TTL), None, "stale must read as unknown");
    }

    #[test]
    fn a_corrupted_adult_line_reads_as_unknown_rather_than_as_permission() {
        // Anything that is not a clean 1/0 must not be able to grant permission.
        for line in ["adult_ok=yes", "adult_ok=", "adult_ok=maybe", "adult_ok=2"] {
            let creds = parse_nexus_creds(&format!("[Nexus]\n{line}\nadult_checked_at=1000\n"));
            assert_eq!(creds.adult_ok, None, "{line}");
            assert_eq!(creds.adult_pref(1_000), None, "{line}");
        }
    }

    #[test]
    fn signing_out_forgets_the_adult_preference_with_the_session() {
        // It belongs to the account that just signed out; keeping it would apply
        // one person's setting to whoever signs in next.
        let existing = "[Nexus]\naccess_token=at\nadult_ok=1\nadult_checked_at=1000\n";
        let mut creds = parse_nexus_creds(existing);
        assert_eq!(creds.adult_ok, Some(true));
        creds.access_token = None;
        creds.adult_ok = None;
        creds.adult_checked_at = 0;
        let back = parse_nexus_creds(&render_nexus_creds(existing, &creds));
        assert_eq!(back.adult_ok, None);
        assert_eq!(back.adult_pref(1_000), None);
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
            path: None,
            offline: false,
            preferred_servers: Vec::new(),
            theme: Theme::Dark,
            default_game: Some("skyrimse".to_string()),
            window_size: Some((1280, 720)),
            lock_gui: false,
            drag_scroll_speed: 1.0,
            remember_window: false,
        };
        let parsed = Settings::parse(&s.to_ini());
        assert_eq!(parsed, s);
    }

    #[test]
    fn settings_round_trip_minimal() {
        let s = Settings { theme: Theme::Light, ..Settings::default() };
        let parsed = Settings::parse(&s.to_ini());
        assert_eq!(parsed, s);
    }

    #[test]
    fn the_drag_scroll_speed_round_trips_and_refuses_nonsense() {
        assert_eq!(Settings::default().drag_scroll_speed, 1.0);
        assert_eq!(Settings::parse("drag_scroll_speed=2.5\n").drag_scroll_speed, 2.5);
        // Out of range or unparseable falls back to the default rather than
        // being taken at face value: a bad number here is a list that either
        // will not move or cannot be aimed.
        for bad in ["0", "99", "-3", "fast", ""] {
            assert_eq!(
                Settings::parse(&format!("drag_scroll_speed={bad}\n")).drag_scroll_speed,
                1.0,
                "{bad} was accepted"
            );
        }
        let s = Settings { drag_scroll_speed: 1.75, ..Settings::default() };
        assert_eq!(Settings::parse(&s.to_ini()).drag_scroll_speed, 1.75);
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
            path: None,
            offline: false,
            preferred_servers: Vec::new(),
            theme: Theme::Dark,
            default_game: Some("starfield".to_string()),
            window_size: Some((1600, 900)),
            lock_gui: false,
            drag_scroll_speed: 1.0,
            remember_window: false,
        };
        fs::write(&path, s.to_ini()).unwrap();
        let loaded = Settings::parse(&fs::read_to_string(&path).unwrap());
        assert_eq!(loaded, s);
        let _ = fs::remove_file(&path);
    }
}
