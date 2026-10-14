//! The window's strings, in English and French.
//!
//! [`t`] looks a key up in the active locale, then in English, then in the
//! shared Colony table (`colony_ui::i18n`, which owns the Preferences kit's own
//! labels, the theme names and the accents), and last returns the key itself.
//! The locale is colony-ui's, so the kit and this window can never disagree.
//!
//! English is canonical: a key is named after its English string and written in
//! `en.rs` first. Both files carry the same keys; `fr_and_en_have_identical_key_sets`
//! fails otherwise. The CLI does not come through here and stays English.

mod en;
mod fr;

use std::collections::HashMap;
use std::sync::OnceLock;

use colony_ui::i18n::Locale;

/// The Language preference: follow the system, or one of the two locales.
pub(crate) const LANGUAGES: [&str; 3] = ["system", "en", "fr"];

fn table(locale: Locale) -> &'static HashMap<&'static str, &'static str> {
    static EN: OnceLock<HashMap<&str, &str>> = OnceLock::new();
    static FR: OnceLock<HashMap<&str, &str>> = OnceLock::new();
    match locale {
        Locale::Fr => FR.get_or_init(|| fr::STRINGS.iter().copied().collect()),
        Locale::En => EN.get_or_init(|| en::STRINGS.iter().copied().collect()),
    }
}

/// The string for `key` in the active locale.
pub(crate) fn t(key: &str) -> &'static str {
    t_in(colony_ui::i18n::locale(), key)
}

/// The string for `key` in `locale`.
pub(crate) fn t_in(locale: Locale, key: &str) -> &'static str {
    table(locale)
        .get(key)
        .or_else(|| table(Locale::En).get(key))
        .copied()
        .unwrap_or_else(|| colony_ui::i18n::t_in(locale, key))
}

/// [`t`] with each `{name}` replaced by its value.
pub(crate) fn tf(key: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut out = t(key).to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), &value.to_string());
    }
    out
}

/// The locale a Language preference means. `system` reads `LC_ALL`, then
/// `LC_MESSAGES`, then `LANG`, as gettext does.
pub(crate) fn resolve(language: &str, env: impl Fn(&str) -> Option<String>) -> Locale {
    match language {
        "en" | "fr" => Locale::from_tag(language),
        _ => ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .filter_map(|var| env(var).filter(|v| !v.is_empty()))
            .map(|tag| Locale::from_tag(&tag))
            .next()
            .unwrap_or(Locale::En),
    }
}

/// Put a Language preference into force for the whole window.
///
/// Not under test: the harness runs tests on parallel threads of one process,
/// and a test that switched the locale would translate the strings another test
/// is asserting on. [`resolve`] is what the tests hold to.
pub(crate) fn apply(language: &str) {
    if cfg!(test) {
        return;
    }
    colony_ui::i18n::set_locale(resolve(language, |var| std::env::var(var).ok()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn keys(strings: &[(&'static str, &'static str)]) -> BTreeSet<&'static str> {
        strings.iter().map(|(k, _)| *k).collect()
    }

    #[test]
    fn fr_and_en_have_identical_key_sets() {
        let (en, fr) = (keys(en::STRINGS), keys(fr::STRINGS));
        let only_en: Vec<_> = en.difference(&fr).collect();
        let only_fr: Vec<_> = fr.difference(&en).collect();
        assert!(only_en.is_empty(), "missing in fr.rs: {only_en:?}");
        assert!(only_fr.is_empty(), "missing in en.rs: {only_fr:?}");
        assert_eq!(en.len(), en::STRINGS.len(), "a key appears twice in en.rs");
        assert_eq!(fr.len(), fr::STRINGS.len(), "a key appears twice in fr.rs");
    }

    /// Theme and accent names are the shared table's, never redefined here.
    #[test]
    fn no_shared_label_is_redefined() {
        for (key, _) in en::STRINGS {
            assert!(
                !key.starts_with("settings_theme_") && !key.starts_with("settings_accent_"),
                "{key} belongs to colony_ui::i18n"
            );
            assert_eq!(
                colony_ui::i18n::t_in(Locale::En, key),
                *key,
                "{key} shadows a shared label"
            );
        }
    }

    /// Every literal key handed to `t` or `tf` in the sources exists, here or in
    /// the shared table, so a typo cannot reach the screen as a raw key.
    #[test]
    fn every_key_used_in_the_sources_exists() {
        fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(std::fs::read_to_string(&path).unwrap());
                }
            }
        }
        let mut sources = Vec::new();
        walk(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut sources,
        );
        let mut missing = BTreeSet::new();
        let mut used = 0;
        for body in &sources {
            for call in ["t(\"", "tf(\""] {
                for (i, _) in body.match_indices(call) {
                    let before = body[..i].chars().next_back();
                    if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                        continue;
                    }
                    let rest = &body[i + call.len()..];
                    let Some(end) = rest.find('"') else { continue };
                    let key = &rest[..end];
                    used += 1;
                    if t_in(Locale::En, key) == key {
                        missing.insert(key.to_string());
                    }
                }
            }
        }
        assert!(used > 0, "the scan found no keys at all");
        assert!(missing.is_empty(), "keys with no string: {missing:?}");
    }

    /// The CLI is linked into this binary (`--eidos-cli`) and dispatched before
    /// the window sets a locale; it has no strings of its own here, so its
    /// output stays English whatever the window's language is.
    #[test]
    fn the_cli_stays_english() {
        let cli = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../eidos/src");
        for entry in std::fs::read_dir(&cli).unwrap().flatten() {
            let body = std::fs::read_to_string(entry.path()).unwrap_or_default();
            assert!(
                !body.contains("i18n") && !body.contains("set_locale"),
                "{} translates CLI output",
                entry.path().display()
            );
        }
    }

    #[test]
    fn french_falls_back_to_english_then_to_the_key() {
        assert_eq!(t_in(Locale::Fr, "settings_title"), "Préférences");
        assert_eq!(t_in(Locale::En, "no_such_key"), "no_such_key");
    }

    #[test]
    fn the_language_follows_the_system_unless_one_is_picked() {
        let env = |lang: &'static str| move |var: &str| (var == "LANG").then(|| lang.to_string());
        assert_eq!(resolve("system", env("fr_FR.UTF-8")), Locale::Fr);
        assert_eq!(resolve("system", env("de_DE.UTF-8")), Locale::En);
        assert_eq!(resolve("system", |_| None), Locale::En);
        assert_eq!(resolve("en", env("fr_FR.UTF-8")), Locale::En);
        assert_eq!(resolve("fr", |_| None), Locale::Fr);
        // LC_ALL outranks LANG.
        let both = |var: &str| match var {
            "LC_ALL" => Some("fr_FR.UTF-8".to_string()),
            "LANG" => Some("en_US.UTF-8".to_string()),
            _ => None,
        };
        assert_eq!(resolve("system", both), Locale::Fr);
    }

    #[test]
    fn placeholders_are_filled() {
        assert_eq!(
            tf(
                "settings_signed_in_as",
                &[
                    ("name", &"Lin"),
                    ("tier", &t_in(Locale::En, "settings_tier_free"))
                ]
            ),
            format!(
                "Signed in as Lin ({}).",
                t_in(Locale::En, "settings_tier_free")
            )
        );
    }
}
