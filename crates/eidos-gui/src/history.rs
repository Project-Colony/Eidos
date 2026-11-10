//! Undo and redo for the mod list (Ctrl+Z, Ctrl+Shift+Z).
//!
//! Each save that landed pushes the state it replaced, so one step is what one
//! click wrote. The history lives only in memory and is reset whenever the list
//! is read from disk again, which is when it stops describing what is there.

use eidos_instance::ModEntry;

/// Steps kept per window; the oldest is dropped past this.
pub(crate) const HISTORY_CAP: usize = 50;

/// The mod list as one save wrote it: names in priority order, enabled flags.
pub(crate) type ModsState = Vec<(String, bool)>;

pub(crate) fn mods_state(mods: &[ModEntry]) -> ModsState {
    mods.iter().map(|m| (m.name.clone(), m.enabled)).collect()
}

/// A state to put back.
pub(crate) enum Step {
    Mods(ModsState),
}

#[derive(Default)]
pub(crate) struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
    /// What the last save (or read) left on disk; `None` until either happened.
    mods: Option<ModsState>,
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
            self.undo.push(Step::Mods(before));
            if self.undo.len() > HISTORY_CAP {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }

    pub(crate) fn undo(&mut self) -> Option<Step> {
        Self::step(&mut self.undo, &mut self.redo, &mut self.mods)
    }

    pub(crate) fn redo(&mut self) -> Option<Step> {
        Self::step(&mut self.redo, &mut self.undo, &mut self.mods)
    }

    /// Take the newest step of `from`, and push the state it replaces onto `to`
    /// so the opposite command can bring it back.
    fn step(
        from: &mut Vec<Step>,
        to: &mut Vec<Step>,
        mods: &mut Option<ModsState>,
    ) -> Option<Step> {
        let Step::Mods(state) = from.pop()?;
        if let Some(current) = mods.replace(state.clone()) {
            to.push(Step::Mods(current));
        }
        Some(Step::Mods(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
