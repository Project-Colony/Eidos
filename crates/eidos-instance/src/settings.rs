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

/// `$XDG_CONFIG_HOME/eidos/nexus.ini`, holding the personal Nexus API key. Same
/// path the CLI uses, so the key is shared between the CLI and the GUI.
pub fn nexus_key_path() -> PathBuf {
    config_home().join("eidos").join("nexus.ini")
}

/// The stored Nexus API key, if any. Reads the `[Nexus]` `api_key=` line; an
/// empty value reads as `None`.
pub fn load_nexus_key() -> Option<String> {
    let text = fs::read_to_string(nexus_key_path()).ok()?;
    parse_nexus_key(&text)
}

/// Persist the Nexus API key so it survives across sessions. Creates the parent
/// directory if needed and writes the `[Nexus]` section the CLI also reads.
pub fn save_nexus_key(key: &str) -> io::Result<()> {
    let path = nexus_key_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, format!("[Nexus]\napi_key={}\n", key.trim()))
}

/// The `[Nexus]` `api_key=` value from a `nexus.ini` body, if non-empty. Split
/// out so it can be unit-tested without touching the filesystem.
fn parse_nexus_key(text: &str) -> Option<String> {
    text.lines()
        .filter_map(|l| l.trim().split_once('='))
        .find(|(k, _)| k.trim() == "api_key")
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Settings {
    /// The color theme to render in.
    pub theme: Theme,
    /// The game id to open by default when none is given (e.g. `skyrimse`).
    pub default_game: Option<String>,
    /// The last window size in logical pixels, `(width, height)`.
    pub window_size: Option<(u32, u32)>,
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
        let mut out = format!("[eidos]\ntheme={}\n", self.theme.as_str());
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
    fn nexus_key_round_trips_via_parser() {
        let text = "[Nexus]\napi_key=abc123\n";
        assert_eq!(parse_nexus_key(text).as_deref(), Some("abc123"));
    }

    #[test]
    fn nexus_key_trims_whitespace() {
        let text = "[Nexus]\napi_key =   spaced-key   \n";
        assert_eq!(parse_nexus_key(text).as_deref(), Some("spaced-key"));
    }

    #[test]
    fn empty_nexus_key_is_none() {
        assert_eq!(parse_nexus_key("[Nexus]\napi_key=\n"), None);
        assert_eq!(parse_nexus_key("[Nexus]\napi_key=   \n"), None);
        assert_eq!(parse_nexus_key("nothing here"), None);
    }

    #[test]
    fn nexus_key_survives_a_disk_round_trip() {
        // Save then load from a real file, proving the key persists.
        let path = tmp("nexus");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&path, format!("[Nexus]\napi_key={}\n", "persisted-key")).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(parse_nexus_key(&text).as_deref(), Some("persisted-key"));
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
    }

    #[test]
    fn settings_round_trip_full() {
        let s = Settings {
            theme: Theme::Dark,
            default_game: Some("skyrimse".to_string()),
            window_size: Some((1280, 720)),
        };
        let parsed = Settings::parse(&s.to_ini());
        assert_eq!(parsed, s);
    }

    #[test]
    fn settings_round_trip_minimal() {
        let s = Settings { theme: Theme::Light, default_game: None, window_size: None };
        let parsed = Settings::parse(&s.to_ini());
        assert_eq!(parsed, s);
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
        };
        fs::write(&path, s.to_ini()).unwrap();
        let loaded = Settings::parse(&fs::read_to_string(&path).unwrap());
        assert_eq!(loaded, s);
        let _ = fs::remove_file(&path);
    }
}
