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

use colony_ui::preferences::Change;

#[test]
fn picking_a_theme_takes_effect_at_once_and_is_written_down() {
    let _look = look_lock();
    let mut app = nav_app(&[]);
    assert_eq!(app.ui_prefs.standard.selected_theme, "parchment");

    let _ = update(
        &mut app,
        Message::Preference(Change::Theme {
            family: "nord",
            variant: "dark",
        }),
    );
    assert_eq!(app.ui_prefs.standard.selected_theme, "nord");
    // The palette is a global that every style closure reads: changing the
    // preference alone would leave the window drawing the old theme.
    assert_eq!(theme::pal(), colony_ui::resolve("nord", "dark"));
    assert_eq!(app.toast, Some(colony_ui::i18n::t("theme_applied")));
    let _ = update(&mut app, Message::DismissToast);
    assert_eq!(app.toast, None);
}

#[test]
fn an_accent_can_be_picked_and_given_back() {
    let _look = look_lock();
    let mut app = nav_app(&[]);
    let own = theme::accent();

    let _ = update(&mut app, Message::Preference(Change::Accent("green")));
    assert_eq!(app.ui_prefs.standard.selected_accent.as_deref(), Some("green"));
    assert_ne!(theme::accent(), own, "the override did not reach the window");

    // Auto accent hands the accent back to the theme.
    let _ = update(&mut app, Message::Preference(Change::AutoAccent(true)));
    assert_eq!(theme::accent(), own);
}

#[test]
fn high_contrast_reaches_the_window() {
    let _look = look_lock();
    let mut app = nav_app(&[]);
    let plain = theme::pal().text_primary;
    let _ = update(&mut app, Message::Preference(Change::HighContrast(true)));
    assert!(app.ui_prefs.standard.high_contrast);
    assert_ne!(theme::pal().text_primary, plain, "the boost never took effect");
}

#[test]
fn the_text_size_reaches_every_sz_call() {
    let _look = look_lock();
    let mut app = nav_app(&[]);
    assert_eq!(colony_ui::sz(10), 10.0);
    let _ = update(
        &mut app,
        Message::Preference(Change::TextSize(colony_ui::TextSize::XLarge)),
    );
    assert_eq!(colony_ui::sz(10), 14.0);
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

/// The name and Close are one toggle: two ways out, one message.
#[test]
fn the_identity_button_and_close_send_the_same_toggle() {
    let mut app = nav_app(&[]);
    let _ = update(&mut app, Message::TogglePreferences);
    assert!(app.settings_open);
    let _ = update(&mut app, Message::TogglePreferences);
    assert!(!app.settings_open);
}

/// Every visit starts on General with every section collapsed; a section's
/// state is remembered while the page stays open.
#[test]
fn the_page_opens_on_general_with_every_section_shut() {
    let mut app = nav_app(&[]);
    let _ = update(&mut app, Message::TogglePreferences);
    assert_eq!(app.settings_tab, SettingsTab::General);
    assert!(app.settings_expanded.is_empty());

    let _ = update(&mut app, Message::SettingsToggleSection("startup"));
    let _ = update(&mut app, Message::SettingsTabSelected(SettingsTab::Nexus));
    assert!(app.settings_expanded.contains("startup"), "forgotten while open");

    let _ = update(&mut app, Message::TogglePreferences);
    let _ = update(&mut app, Message::TogglePreferences);
    assert_eq!(app.settings_tab, SettingsTab::General);
    assert!(app.settings_expanded.is_empty(), "a section arrived open");
}

#[test]
fn a_section_header_toggles_rather_than_only_opening() {
    let mut app = nav_app(&[]);
    let _ = update(&mut app, Message::SettingsToggleSection("startup"));
    assert!(app.settings_expanded.contains("startup"), "it did not open");
    let _ = update(&mut app, Message::SettingsToggleSection("startup"));
    assert!(!app.settings_expanded.contains("startup"), "it did not close");
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
fn the_server_field_is_stored_as_it_is_typed() {
    let mut app = app_for_game("skyrimse");
    let _ = update_inner(
        &mut app,
        Message::PreferredServersChanged("  Paris , ,Nexus CDN,  ".to_string()),
    );
    assert_eq!(
        app.prefs.preferred_servers,
        vec!["Paris", "Nexus CDN"],
        "trimmed, blanks gone"
    );
    // The field keeps what was typed: rewriting it mid-word would eat the comma
    // the user is about to follow with a name.
    assert_eq!(app.servers_edit, "  Paris , ,Nexus CDN,  ");
}

#[test]
fn the_tools_folder_is_stored_as_it_is_typed() {
    let mut app = app_for_game("skyrimse");
    let _ = update_inner(&mut app, Message::ToolsDirChanged(" /mnt/Tools ".to_string()));
    assert_eq!(app.prefs.tools_dir.as_deref(), Some("/mnt/Tools"));
    let _ = update_inner(&mut app, Message::ToolsDirChanged("  ".to_string()));
    assert_eq!(app.prefs.tools_dir, None, "an empty box clears it");
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
    assert_eq!(app.ui_prefs, crate::preferences::UiPreferences::default());
}
