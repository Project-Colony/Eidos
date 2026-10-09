//! Preferences: the categories the page is divided into, and the page itself.
//!
//! The categories and what each one says about itself are kept apart from the
//! page that draws them, because the Colony convention fixes their order and
//! wording and the tests hold them to it.

mod page;

pub(crate) use page::preferences_page;

/// Sections of the Settings screen, in sidebar order.
///
/// A vertical rail rather than a row of tabs, matching Colony: five entries do
/// not fit across a dialog, and the rail leaves room to add a sixth without
/// re-laying anything out.
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

    /// The sections open the first time Preferences is shown - one per category,
    /// so every page says something without a click.
    pub(crate) const DEFAULT_OPEN: [&'static str; 6] =
        ["startup", "theme", "motion", "dragging", "account", "paths"];

    pub(crate) fn label(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Appearance => "Appearance",
            SettingsTab::Accessibility => "Accessibility",
            SettingsTab::ModList => "Mod list",
            SettingsTab::Nexus => "Nexus",
            SettingsTab::About => "About",
        }
    }

    /// The line under the category's own heading: what it changes, not its name
    /// said again.
    pub(crate) fn description(self) -> &'static str {
        match self {
            // This sentence is imposed by the convention, near enough word for
            // word, because it carries the contract that matters most on this
            // page: there is no Save button, and its absence has to be explained
            // somewhere rather than left to be discovered.
            SettingsTab::General => "Preferences are saved automatically.",
            SettingsTab::Appearance => "Applies immediately - nothing here needs a restart.",
            SettingsTab::Accessibility => "Changes how the window behaves, on any theme.",
            SettingsTab::ModList => "How the list of mods behaves under the pointer.",
            SettingsTab::Nexus => "The account downloads are fetched with, and what is cached.",
            SettingsTab::About => "Version, licence and where this instance lives.",
        }
    }
}

#[cfg(test)]
mod tests;
