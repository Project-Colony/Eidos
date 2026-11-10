//! A38: every run started through `run_through_view` leaves a record in
//! `<instance>/logs/sessions/`, written before the launch and again when it ends.

use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eidos_games::DetectedGame;
use eidos_instance::{Instance, Profile, SessionMod, SessionRecord};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// What this run starts with. Reads small files only, plus the game executable's
/// version resource, so the launch is not held up.
pub(crate) fn capture(
    game: &DetectedGame,
    inst: &Instance,
    prof: &Profile,
    tool: Option<&str>,
) -> SessionRecord {
    let mods = prof
        .modlist()
        .into_iter()
        .filter(|m| m.is_active() && !m.is_unmanaged())
        .map(|m| {
            let meta = inst.mod_meta(&m.name);
            SessionMod {
                version: meta.version(),
                files: meta.installed_files(),
                name: m.name,
            }
        })
        .collect();
    let plugins = game
        .plugin_spec()
        .map(|spec| eidos_plugins::PluginList::read_active(&prof.plugins_state_dir(), &spec))
        .unwrap_or_default();
    let inis = eidos_gamefeatures::ini_files_for(game.def.id)
        .iter()
        .filter_map(|ini| {
            let bytes = std::fs::read(prof.ini_path(ini)).ok()?;
            Some((ini.to_string(), crc32fast::hash(&bytes)))
        })
        .collect();
    // The winning `Root/` executable, the same value `logs/launch.ini` records.
    let game_version = crate::check::extender::current_build(
        game,
        &crate::check::active_mods(inst),
        &inst.overwrite_dir(),
    )
    .runtime;
    SessionRecord {
        started: now(),
        pid: std::process::id(),
        eidos: env!("CARGO_PKG_VERSION").into(),
        profile: prof.name.clone(),
        tool: tool.map(Into::into),
        game_version,
        mods,
        plugins,
        inis,
        ..Default::default()
    }
}

/// Captures and writes the record; a failed write only warns, the launch goes on.
pub(crate) fn begin(
    game: &DetectedGame,
    inst: &Instance,
    prof: &Profile,
    tool: Option<&str>,
) -> Option<(PathBuf, SessionRecord)> {
    let record = capture(game, inst, prof, tool);
    match inst.write_session(&record) {
        Ok(path) => Some((path, record)),
        Err(e) => {
            eidos_log::warn!(
                "eidos: could not record this run in {}: {e}",
                inst.sessions_dir().display()
            );
            None
        }
    }
}

/// Adds the end time, the exit code and the saves written during the run.
pub(crate) fn finish(
    begun: Option<(PathBuf, SessionRecord)>,
    result: &io::Result<ExitStatus>,
    inst: &Instance,
    prof: &Profile,
) {
    let Some((_, mut record)) = begun else {
        return;
    };
    record.ended = Some(now());
    record.exit = result.as_ref().ok().map(crate::launch::exit_code);
    let since = UNIX_EPOCH + Duration::from_secs(record.started);
    record.new_saves = prof
        .savegames()
        .into_iter()
        .filter(|s| s.mtime >= since)
        .map(|s| s.filename)
        .collect();
    if let Err(e) = inst.write_session(&record) {
        eidos_log::warn!("eidos: could not record how this run ended: {e}");
    }
}
