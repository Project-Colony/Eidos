use super::*;
use crate::*;
use crate::test_support::*;

/// The rank each category is required to hold, written out separately from
/// `ALL`. The match is exhaustive, so adding a category without giving it a
/// rank does not compile - which is the point, because `ALL` is a
/// hand-written array and nothing else would force the question.
fn expected_rank(tab: SettingsTab) -> usize {
    match tab {
        SettingsTab::General => 0,
        SettingsTab::Appearance => 1,
        SettingsTab::Accessibility => 2,
        SettingsTab::ModList => 3,
        SettingsTab::Nexus => 4,
        SettingsTab::About => 5,
    }
}

/// "Do not reorder the first three. They are what a user hunting for a
/// setting scans first" - the ecosystem convention is explicit, and this is
/// exactly the kind of constraint a reshuffle breaks without noticing.
#[test]
fn picking_a_theme_takes_effect_at_once_and_is_written_down() {
    let mut app = nav_app(&[]);
    // Everyone starts on the parchment - an upgrade repaints nobody.
    assert_eq!(app.prefs.theme_family, "eidos");
    assert_eq!(theme::pal().bg_primary, theme::PARCHMENT.bg_primary);

    let _ = update(
        &mut app,
        Message::ThemeChanged("nord".to_string(), "dark".to_string()),
    );
    assert_eq!(app.prefs.theme_family, "nord");
    // The palette is a global that every style closure reads: changing the
    // preference alone would leave the window drawing the old theme.
    assert_ne!(theme::pal().bg_primary, theme::PARCHMENT.bg_primary);
    assert_eq!(
        theme::pal().bg_primary,
        colony_ui::resolve("nord", "dark").bg_primary
    );

    // And it survives the file.
    assert_eq!(
        eidos_instance::Settings::parse(&app.prefs.to_ini()).theme_family,
        "nord"
    );

    let _ = update(
        &mut app,
        Message::ThemeChanged(
            theme::OWN_FAMILY.to_string(),
            theme::OWN_VARIANT.to_string(),
        ),
    );
    assert_eq!(
        theme::pal().bg_primary,
        theme::PARCHMENT.bg_primary,
        "no way back"
    );
}

#[test]
fn an_accent_can_be_picked_and_given_back() {
    let mut app = nav_app(&[]);
    let own = theme::accent();

    let _ = update(&mut app, Message::AccentChanged(Some("green".to_string())));
    assert_eq!(app.prefs.accent.as_deref(), Some("green"));
    assert_ne!(
        theme::accent(),
        own,
        "the override did not reach the window"
    );

    // "Auto" is the absence of an override, not a ninth colour.
    let _ = update(&mut app, Message::AccentChanged(None));
    assert_eq!(app.prefs.accent, None);
    assert_eq!(theme::accent(), own);
    assert!(!app.prefs.to_ini().contains("accent="));
}

#[test]
fn high_contrast_reaches_the_window_and_is_saved() {
    let mut app = nav_app(&[]);
    let plain = theme::pal().text_primary;

    let _ = update(&mut app, Message::ToggleHighContrast(true));
    assert!(app.prefs.high_contrast);
    assert_ne!(
        theme::pal().text_primary,
        plain,
        "the boost never took effect"
    );
    assert!(eidos_instance::Settings::parse(&app.prefs.to_ini()).high_contrast);

    let _ = update(&mut app, Message::ToggleHighContrast(false));
    assert_eq!(theme::pal().text_primary, plain);
}

/// The catalogue is data, not code: the picker draws whatever it holds, so a
/// family added upstream needs no arm here. This states the size it has, so
/// a bump that silently loses half of it is visible.
#[test]
fn the_shared_catalogue_carries_what_it_should() {
    let families = colony_ui::THEME_FAMILIES.len();
    let variants: usize = colony_ui::THEME_FAMILIES
        .iter()
        .map(|f| f.variants.len())
        .sum();
    // Lower bounds, not exact counts. The point of the catalogue is that it
    // grows without a consumer changing, so an equality here would have to
    // be edited every time one is added - and would be the only thing
    // standing in the way. What must never happen is losing what is there.
    assert!(families >= 25, "theme families shrank to {families}");
    assert!(variants >= 57, "theme palettes shrank to {variants}");
    assert_eq!(colony_ui::ACCENT_OVERRIDES.len(), 8, "accent overrides");

    // Every one of them resolves to a palette that is actually filled in.
    for f in colony_ui::THEME_FAMILIES {
        for v in f.variants {
            let p = colony_ui::resolve(f.key, v.key);
            assert_eq!(p.bg_primary.a, 1.0, "{}/{} has no background", f.key, v.key);
        }
    }
}

#[test]
fn the_first_three_categories_are_the_imposed_ones_in_order() {
    assert_eq!(
        &SettingsTab::ALL[..3],
        &[
            SettingsTab::General,
            SettingsTab::Appearance,
            SettingsTab::Accessibility
        ],
    );
}

/// "About last where it exists."
#[test]
fn about_comes_last() {
    assert_eq!(SettingsTab::ALL.last(), Some(&SettingsTab::About));
    for (rank, tab) in SettingsTab::ALL.into_iter().enumerate() {
        assert_eq!(rank, expected_rank(tab), "{tab:?} is in the wrong place");
    }
}

/// The contract the page lives by: no Save button, so the sentence that
/// explains its absence has to be somewhere the user will read.
#[test]
fn general_carries_the_no_save_button_contract() {
    assert!(
        SettingsTab::General
            .description()
            .contains("saved automatically"),
        "General must say preferences save themselves; it says {:?}",
        SettingsTab::General.description(),
    );
}

/// A description that repeats the title teaches nothing. The convention asks
/// it to say the CONSEQUENCE of the category, so no two may be the same and
/// none may echo its own label.
#[test]
fn each_category_explains_itself_and_says_something_different() {
    let mut seen: Vec<&str> = SettingsTab::ALL
        .into_iter()
        .map(|t| t.description())
        .collect();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(before, seen.len(), "two categories share a description");

    for tab in SettingsTab::ALL {
        assert_ne!(
            tab.label(),
            tab.description(),
            "{tab:?} repeats its own name"
        );
        assert!(!tab.description().is_empty());
    }
}

/// Reduced motion belongs to Accessibility, not to Appearance. Appearance is
/// what the window looks like; Accessibility is what it does to somebody who
/// needs it to do less.
#[test]
fn motion_is_filed_under_accessibility() {
    let rank = expected_rank(SettingsTab::Accessibility);
    assert_eq!(
        SettingsTab::DEFAULT_OPEN[rank],
        "motion",
        "Accessibility must open on its motion section",
    );
}

#[test]
fn every_settings_category_opens_with_something_to_read() {
    // A page whose sections are all shut asks the user to click before it
    // says anything, which is the failure mode of a sectioned settings
    // screen. Each category ships one section open.
    let app = nav_app(&[]);
    assert_eq!(SettingsTab::DEFAULT_OPEN.len(), SettingsTab::ALL.len());
    for key in SettingsTab::DEFAULT_OPEN {
        assert!(
            app.settings_expanded.contains(key),
            "{key} did not start open"
        );
    }
}

#[test]
fn a_section_header_toggles_rather_than_only_opening() {
    let mut app = nav_app(&[]);
    assert!(app.settings_expanded.contains("startup"));
    let _ = update(&mut app, Message::SettingsToggleSection("startup"));
    assert!(
        !app.settings_expanded.contains("startup"),
        "it did not close"
    );
    let _ = update(&mut app, Message::SettingsToggleSection("startup"));
    assert!(
        app.settings_expanded.contains("startup"),
        "it did not reopen"
    );
}

#[test]
fn every_settings_toggle_persists_what_it_flipped() {
    // Each of these writes settings.ini, so a flip that only changed the
    // in-memory copy would look right and be gone next launch.
    let mut app = nav_app(&[]);
    let before = (app.prefs.remember_window, app.prefs.lock_gui);
    let _ = update(&mut app, Message::ToggleRememberWindow(!before.0));
    let _ = update(&mut app, Message::ToggleLockGui(!before.1));
    assert_eq!(app.prefs.remember_window, !before.0);
    assert_eq!(app.prefs.lock_gui, !before.1);
}

#[test]
fn offline_mode_is_off_unless_it_was_explicitly_turned_on() {
    // The opposite default to lock_gui, deliberately: a settings file
    // written by an older Eidos has no `offline` key, and reading a missing
    // key as "on" would cut the network for everybody who upgrades.
    let s = eidos_instance::settings::Settings::parse("[eidos]\ntheme=dark\n");
    assert!(!s.offline);
    let s = eidos_instance::settings::Settings::parse("[eidos]\noffline=true\n");
    assert!(s.offline);
    // And it round-trips, which is the whole point of a setting.
    assert!(eidos_instance::settings::Settings::parse(&s.to_ini()).offline);
}

#[test]
fn the_server_field_stores_an_order_and_echoes_back_what_it_kept() {
    let mut app = app_for_game("skyrimse");
    let _ = update_inner(
        &mut app,
        Message::PreferredServersChanged("  Paris , ,Nexus CDN,  ".to_string()),
    );
    let _ = update_inner(&mut app, Message::PreferredServersSave);

    assert_eq!(
        app.prefs.preferred_servers,
        vec!["Paris", "Nexus CDN"],
        "trimmed, blanks gone"
    );
    // Echoed back as stored, so a trailing comma does not sit in the field
    // looking like it means something.
    assert_eq!(app.servers_edit, "Paris, Nexus CDN");
}

#[test]
fn no_test_can_write_the_developers_own_preferences() {
    // The defect this exists for cost real time to find: `Settings::save`
    // resolved its path globally, and `App::new` loaded from the real one,
    // so a GUI test dispatching a toggle rewrote ~/.config/.../settings.ini
    // on the machine running the suite. The symptom - options reverting
    // after every rebuild - looks like anything but a test.
    let mut app = app_for_game("skyrimse");
    assert!(
        app.prefs.path().is_none(),
        "a test's Settings must not be bound to a file, let alone the real one"
    );

    let before = app.prefs.lock_gui;
    let _ = update_inner(&mut app, Message::ToggleLockGui(!before));
    assert_eq!(
        app.prefs.lock_gui, !before,
        "the toggle still works in memory"
    );
    // And saving it is a no-op rather than a write, so no arm in update.rs
    // can reach the real file by accident either.
    assert!(app.prefs.save().is_ok());
    assert!(app.prefs.path().is_none());
}

#[test]
fn the_motion_preference_survives_a_round_trip_through_the_file() {
    let mut app = nav_app(&[]);
    assert!(app.prefs.motion, "on by default");
    let _ = update(&mut app, Message::ToggleMotion(false));
    let reloaded = eidos_instance::Settings::parse(&app.prefs.to_ini());
    assert!(!reloaded.motion);
}
