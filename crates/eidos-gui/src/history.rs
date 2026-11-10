//! Undo and redo for the mod list and the load order (Ctrl+Z, Ctrl+Shift+Z).
//!
//! Each save that landed pushes the state it replaced, so one step is what one
//! click wrote. The history lives only in memory and is reset whenever the list
//! is read from disk again, which is when it stops describing what is there.

use eidos_instance::ModEntry;
use eidos_plugins::PluginList;
use std::collections::BTreeMap;

/// Steps kept per window; the oldest is dropped past this.
pub(crate) const HISTORY_CAP: usize = 50;

/// The mod list as one save wrote it: names in priority order, enabled flags.
pub(crate) type ModsState = Vec<(String, bool)>;

pub(crate) fn mods_state(mods: &[ModEntry]) -> ModsState {
    mods.iter().map(|m| (m.name.clone(), m.enabled)).collect()
}

/// The load order as one save wrote it: plugins in order with their enabled
/// flags, and the pinned positions.
pub(crate) type PluginsState = (Vec<(String, bool)>, BTreeMap<String, usize>);

pub(crate) fn plugins_state(list: &PluginList) -> PluginsState {
    (
        list.plugins
            .iter()
            .map(|p| (p.name.clone(), p.enabled))
            .collect(),
        list.locked.clone(),
    )
}

/// A state to put back.
pub(crate) enum Step {
    Mods(ModsState),
    Plugins(PluginsState),
}

#[derive(Default)]
pub(crate) struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// What the last save (or read) left on disk; `None` until either happened.
    mods: Option<ModsState>,
    /// Same for the load order.
    plugins: Option<PluginsState>,
}

impl History {
    /// The list was just read from disk: nothing before this can be undone.
    pub(crate) fn reset(&mut self, mods: ModsState) {
        *self = Self {
            mods: Some(mods),
            ..Self::default()
        };
    }

    /// A save of the mod list landed with `now`.
    pub(crate) fn saved_mods(&mut self, now: ModsState) {
        if self.mods.as_ref() == Some(&now) {
            return;
        }
        if let Some(before) = self.mods.replace(now) {
            self.push(Step::Mods(before));
        }
    }

    /// The load order was just read from disk: the baseline, nothing pushed.
    pub(crate) fn seen_plugins(&mut self, now: PluginsState) {
        self.plugins = Some(now);
    }

    /// A save of the load order landed with `now`.
    pub(crate) fn saved_plugins(&mut self, now: PluginsState) {
        if self.plugins.as_ref() == Some(&now) {
            return;
        }
        if let Some(before) = self.plugins.replace(now) {
            self.push(Step::Plugins(before));
        }
    }

    /// Forget the steps; the baselines stay, so the next edit is still recorded.
    pub(crate) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    fn push(&mut self, step: Step) {
        self.undo.push(step);
        if self.undo.len() > HISTORY_CAP {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub(crate) fn undo(&mut self) -> Option<Step> {
        Self::step(
            &mut self.undo,
            &mut self.redo,
            &mut self.mods,
            &mut self.plugins,
        )
    }

    pub(crate) fn redo(&mut self) -> Option<Step> {
        Self::step(
            &mut self.redo,
            &mut self.undo,
            &mut self.mods,
            &mut self.plugins,
        )
    }

    /// Take the newest step of `from`, and push the state it replaces onto `to`
    /// so the opposite command can bring it back.
    fn step(
        from: &mut Vec<Step>,
        to: &mut Vec<Step>,
        mods: &mut Option<ModsState>,
        plugins: &mut Option<PluginsState>,
    ) -> Option<Step> {
        Some(match from.pop()? {
            Step::Mods(state) => {
                if let Some(current) = mods.replace(state.clone()) {
                    to.push(Step::Mods(current));
                }
                Step::Mods(state)
            }
            Step::Plugins(state) => {
                if let Some(current) = plugins.replace(state.clone()) {
                    to.push(Step::Plugins(current));
                }
                Step::Plugins(state)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_and_plugin_steps_undo_in_the_order_they_were_saved() {
        let mut h = History::default();
        h.reset(vec![("A".into(), true)]);
        h.seen_plugins((vec![("a.esp".into(), true)], Default::default()));
        h.saved_plugins((vec![("a.esp".into(), false)], Default::default()));
        h.saved_mods(vec![("A".into(), false)]);
        assert!(matches!(h.undo(), Some(Step::Mods(_))));
        assert!(matches!(h.undo(), Some(Step::Plugins((p, _))) if p[0].1));
    }

    #[test]
    fn undo_and_redo_walk_saved_states_and_a_new_save_drops_redo() {
        let s = |on: bool| vec![("A".to_string(), on)];
        let mut h = History::default();
        h.reset(s(true));
        h.saved_mods(s(false));
        assert!(matches!(h.undo(), Some(Step::Mods(m)) if m == s(true)));
        h.saved_mods(s(true)); // the apply's own save: equal to the baseline, no push
        assert!(matches!(h.redo(), Some(Step::Mods(m)) if m == s(false)));
        h.saved_mods(s(false));
        h.saved_mods(s(true));
        assert!(h.redo().is_none());
        for _ in 0..60 {
            h.saved_mods(s(false));
            h.saved_mods(s(true));
        }
        assert_eq!(std::iter::from_fn(|| h.undo()).count(), HISTORY_CAP);
    }
}
