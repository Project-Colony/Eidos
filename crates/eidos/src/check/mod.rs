//! Health decisions with no GUI in them: facts in, `Finding`s out. The GUI
//! renders a finding through `diag_<code>` / `diag_<code>_detail` strings; the
//! CLI prints the code and its args.

pub mod crash;
pub mod extender;
pub mod graphics;
pub mod launch_option;

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
    "launch_option_missing",
    "launch_option_no_eidos",
    "launch_option_binary_missing",
    "launch_option_no_command",
    "launch_option_other_copy",
    "nvapi_missing",
    "fullscreen_blocks_framegen",
    "enb_linux_version",
    "game_crashed",
    "game_crashed_unattributed",
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
    home: &Path,
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
    let this_exe = std::env::current_exe().ok();
    let mut options = None;
    if game.is_steam() && game.compatdata.is_some() {
        options = eidos_games::steam_launch_options(home, game.def.steam_app_id);
        let path: Vec<PathBuf> = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        if let Some(this_exe) = &this_exe {
            out.extend(launch_option::launch_option_findings(
                options.as_deref(),
                this_exe,
                home,
                &path,
                game.def,
            ));
        }
    }
    if game.prefix().is_some() {
        let this_exe = this_exe.unwrap_or_else(|| PathBuf::from("eidos"));
        out.extend(graphics::graphics_findings(&graphics::graphics_facts(
            game,
            inst,
            mods,
            options.as_deref(),
            &this_exe,
            home,
        )));
    }
    out.extend(crash::crash_findings(game, inst, mods));
    out
}
