//! Preferences: the categories the page is divided into, and the page itself.
//!
//! The categories and what each one says about itself are kept apart from the
//! page that draws them, because the Colony convention fixes their order and
//! wording and the tests hold them to it.

mod page;
mod store;

pub(crate) use page::preferences_page;
pub(crate) use store::UiPreferences;

/// Categories of the Preferences page, in rail order.
///
/// The first three are the Colony standard ones, drawn by the shared kit; the
/// rest are Eidos's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    General,
    Appearance,
    Accessibility,
    ModList,
    Nexus,
    About,
}

impl SettingsTab {
    /// Every category, in the order the rail lists them.
    ///
    /// The first three are imposed by the Colony convention and must stay in
    /// this order: they are what somebody hunting for a setting scans first, and
    /// they hold the same things in every program in the ecosystem. After them
    /// come Eidos's own, and About last. `the_first_three_categories_are_the_imposed_ones`
    /// fails if that is disturbed.
    pub(crate) const ALL: [SettingsTab; 6] = [
        SettingsTab::General,
        SettingsTab::Appearance,
        SettingsTab::Accessibility,
        SettingsTab::ModList,
        SettingsTab::Nexus,
        SettingsTab::About,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            SettingsTab::General => colony_ui::i18n::t("settings_cat_general"),
            SettingsTab::Appearance => colony_ui::i18n::t("settings_cat_appearance"),
            SettingsTab::Accessibility => colony_ui::i18n::t("settings_cat_accessibility"),
            SettingsTab::ModList => crate::t("settings_cat_mod_list"),
            SettingsTab::Nexus => crate::t("settings_cat_nexus"),
            SettingsTab::About => colony_ui::i18n::t(colony_ui::widgets::ABOUT_CATEGORY),
        }
    }

    /// The line under the category's own heading: what it changes, not its name
    /// said again.
    pub(crate) fn description(self) -> &'static str {
        match self {
            // General's line carries the contract that matters most on this
            // page: there is no Save button, and its absence has to be
            // explained somewhere rather than left to be discovered.
            SettingsTab::General => colony_ui::i18n::t("settings_general_desc"),
            SettingsTab::Appearance => colony_ui::i18n::t("settings_appearance_desc"),
            SettingsTab::Accessibility => colony_ui::i18n::t("settings_accessibility_desc"),
            SettingsTab::ModList => crate::t("settings_mod_list_desc"),
            SettingsTab::Nexus => crate::t("settings_nexus_desc"),
            SettingsTab::About => crate::t("settings_about_desc"),
        }
    }
}

#[cfg(test)]
mod tests;
