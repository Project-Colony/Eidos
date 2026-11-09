//! Health decisions with no GUI in them: facts in, `Finding`s out. The GUI
//! renders a finding through `diag_<code>` / `diag_<code>_detail` strings; the
//! CLI prints the code and its args.

pub mod extender;

use std::path::{Path, PathBuf};

use eidos_games::DetectedGame;
use eidos_instance::Instance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Problem,
    Advice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub code: &'static str,
    pub level: Level,
    pub args: Vec<(&'static str, String)>,
}

impl Finding {
    pub fn arg(&self, key: &str) -> Option<&str> {
        self.args
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
    }

    /// `key=value; key=value`
    pub fn args_line(&self) -> String {
        self.args
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Every code `host_findings` and `prefix_findings` can emit; the GUI has en/fr strings for each.
pub const CODES: &[&str] = &[
    "extender_missing",
    "extender_runtime_missing",
    "game_updated",
];

/// The instance's enabled mods, highest priority first.
pub fn active_mods(inst: &Instance) -> Vec<(String, PathBuf)> {
    inst.modlist()
        .into_iter()
        .rev()
        .filter(|m| m.is_active())
        .map(|m| (m.name, m.path))
        .collect()
}

pub fn host_findings(
    _home: &Path,
    game: &DetectedGame,
    inst: &Instance,
    mods: &[(String, PathBuf)],
) -> Vec<Finding> {
    let overwrite = inst.overwrite_dir();
    let mut out = eidos_gamefeatures::preflight::extender_facts(
        game.def,
        &game.data_path,
        &game.install_path,
        mods,
        &overwrite,
    )
    .map(|f| extender::extender_findings(&f))
    .unwrap_or_default();
    out.extend(extender::game_update_findings(
        game.def.name,
        inst.launch_record().last_good.as_ref(),
        &extender::current_build(game, mods, &overwrite),
    ));
    out
}
