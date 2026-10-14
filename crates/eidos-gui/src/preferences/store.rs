//! The window's look, kept in `preferences.json` beside `settings.ini`.
//!
//! What it holds is the Colony standard set - theme, accent, both text sizes,
//! the dyslexia font, high contrast, effects and reduced motion - as
//! [`StandardPreferences`], under the field names every Colony program writes.
//! `settings.ini` keeps what the CLI shares with the window (default game,
//! tools folder, Nexus servers); none of this is the CLI's business.

use std::io;
use std::path::{Path, PathBuf};

use colony_ui::preferences::StandardPreferences;
use serde::{Deserialize, Serialize};

/// What a fresh install wears: the catalogue's parchment, which is the look
/// Eidos has always had.
pub(crate) const DEFAULT_THEME: (&str, &str) = ("parchment", "light");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct UiPreferences {
    #[serde(flatten)]
    pub(crate) standard: StandardPreferences,
    /// `system`, `en` or `fr`: see [`crate::i18n::resolve`].
    #[serde(default = "system_language")]
    pub(crate) language: String,
    /// The file these were read from and the only one [`UiPreferences::save`]
    /// writes. `None` for defaults, which is what a test gets, so no test can
    /// write the user's real file.
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Default for UiPreferences {
    fn default() -> Self {
        UiPreferences {
            standard: StandardPreferences {
                selected_theme: DEFAULT_THEME.0.to_string(),
                selected_variant: DEFAULT_THEME.1.to_string(),
                ..StandardPreferences::default()
            },
            language: system_language(),
            path: None,
        }
    }
}

fn system_language() -> String {
    "system".to_string()
}

impl UiPreferences {
    /// `~/.config/Colony/Eidos/preferences.json`.
    pub(crate) fn default_path() -> PathBuf {
        eidos_instance::settings::config_home().join("preferences.json")
    }

    pub(crate) fn load() -> UiPreferences {
        UiPreferences::load_from(
            &UiPreferences::default_path(),
            &eidos_instance::settings::settings_path(),
        )
    }

    /// Read `path`, or, the first time, carry over the look an older Eidos kept
    /// in `settings.ini` and write it out at once: the next save of
    /// `settings.ini` drops those keys, so waiting would lose them.
    pub(crate) fn load_from(path: &Path, legacy_ini: &Path) -> UiPreferences {
        let mut prefs = match std::fs::read_to_string(path) {
            // Lenient per field: a bad value falls back to its default, so only
            // a file that is not JSON at all loses the rest.
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eidos_log::warn!("{} is not valid JSON ({e}); using defaults", path.display());
                UiPreferences::default()
            }),
            Err(_) => {
                let mut migrated = std::fs::read_to_string(legacy_ini)
                    .map(|text| UiPreferences::from_legacy_ini(&text))
                    .unwrap_or_default();
                migrated.path = Some(path.to_path_buf());
                if let Err(e) = migrated.save() {
                    eidos_log::warn!("could not write {}: {e}", path.display());
                }
                migrated
            }
        };
        prefs.path = Some(path.to_path_buf());
        prefs
    }

    /// The look as Eidos 1.18 and older stored it in `settings.ini`. Its own
    /// `eidos/parchment` theme is the catalogue's `parchment/light` now, and
    /// `motion=off` is what reduced motion means.
    pub(crate) fn from_legacy_ini(text: &str) -> UiPreferences {
        let mut prefs = UiPreferences::default();
        let s = &mut prefs.standard;
        let (mut family, mut variant) = (None, None);
        let on = |v: &str| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on");
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            match k.trim() {
                "theme_family" if !v.is_empty() => family = Some(v),
                "theme_variant" if !v.is_empty() => variant = Some(v),
                "accent" if !v.is_empty() => s.selected_accent = Some(v.to_string()),
                "high_contrast" => s.high_contrast = on(v),
                "motion" => {
                    s.reduce_motion = matches!(
                        v.to_ascii_lowercase().as_str(),
                        "false" | "0" | "no" | "off"
                    )
                }
                _ => {}
            }
        }
        if let (Some(f), Some(v)) = (family, variant) {
            if f != "eidos" {
                s.selected_theme = f.to_string();
                s.selected_variant = v.to_string();
            }
        }
        prefs
    }

    pub(crate) fn save(&self) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        std::fs::write(path, body + "\n")
    }

    /// Whether the window animates: effects on and reduced motion off. Read from
    /// these preferences rather than from colony-ui's process-wide switch, so
    /// two windows (or two tests) cannot disagree about one of them.
    pub(crate) fn motion(&self) -> bool {
        self.standard.animations && !self.standard.reduce_motion
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use colony_ui::typography::{FontSize, TextSize};

    fn tmp(label: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eidos-ui-prefs-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_fresh_install_wears_the_parchment() {
        let p = UiPreferences::default();
        assert_eq!(
            (
                p.standard.selected_theme.as_str(),
                p.standard.selected_variant.as_str()
            ),
            DEFAULT_THEME
        );
        assert!(p.motion());
    }

    #[test]
    fn the_old_settings_ini_look_is_carried_over() {
        let p = UiPreferences::from_legacy_ini(
            "[eidos]\ntheme_family=nord\ntheme_variant=dark\naccent=green\nhigh_contrast=true\nmotion=off\n",
        );
        assert_eq!(p.standard.selected_theme, "nord");
        assert_eq!(p.standard.selected_variant, "dark");
        assert_eq!(p.standard.selected_accent.as_deref(), Some("green"));
        assert!(p.standard.high_contrast);
        assert!(p.standard.reduce_motion);
        assert!(!p.motion());

        // Eidos's own parchment is the catalogue's now.
        let own = UiPreferences::from_legacy_ini("theme_family=eidos\ntheme_variant=parchment\n");
        assert_eq!(own, UiPreferences::default());
        assert_eq!(UiPreferences::from_legacy_ini(""), UiPreferences::default());
    }

    #[test]
    fn the_migration_is_written_once_and_the_file_wins_after() {
        let d = tmp("migrate");
        let (json, ini) = (d.join("preferences.json"), d.join("settings.ini"));
        std::fs::write(&ini, "theme_family=gruvbox\ntheme_variant=dark\n").unwrap();
        let first = UiPreferences::load_from(&json, &ini);
        assert_eq!(first.standard.selected_theme, "gruvbox");
        assert!(json.is_file(), "the migration was not written down");

        // settings.ini loses the keys on its next save; the JSON keeps them.
        std::fs::write(&ini, "lock_gui=true\n").unwrap();
        assert_eq!(
            UiPreferences::load_from(&json, &ini)
                .standard
                .selected_theme,
            "gruvbox"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn every_standard_preference_round_trips_through_the_file() {
        let d = tmp("roundtrip");
        let json = d.join("preferences.json");
        let mut p = UiPreferences::load_from(&json, &d.join("none.ini"));
        p.standard.selected_theme = "kanagawa".to_string();
        p.standard.selected_variant = "journal".to_string();
        p.standard.selected_accent = Some("violet".to_string());
        p.standard.font_size = FontSize::Large;
        p.standard.text_size_a11y = TextSize::XLarge;
        p.standard.dyslexia_font = true;
        p.standard.animations = false;
        p.language = "fr".to_string();
        p.save().unwrap();
        assert_eq!(UiPreferences::load_from(&json, &d.join("none.ini")), p);

        // A value from a newer build degrades that field only.
        std::fs::write(&json, r#"{"selected_theme":"nord","font_size":"enormous"}"#).unwrap();
        let back = UiPreferences::load_from(&json, &d.join("none.ini"));
        assert_eq!(back.standard.selected_theme, "nord");
        assert_eq!(back.standard.font_size, FontSize::Default);
        assert_eq!(back.language, "system", "no language means the system's");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn defaults_are_never_written_anywhere() {
        assert!(UiPreferences::default().save().is_ok());
        assert!(UiPreferences::default().path.is_none());
    }
}
