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

/// A38 ruling: until crash detection lands, the last good run is the newest game
/// run that exited with code 0.
pub(crate) fn default_target(
    records: &[(PathBuf, SessionRecord)],
) -> Option<&(PathBuf, SessionRecord)> {
    records
        .iter()
        .filter(|(_, r)| r.tool.is_none() && r.exit == Some(0))
        .max_by_key(|(_, r)| (r.started, r.pid))
}

fn stem(path: &std::path::Path) -> String {
    path.file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into())
}

fn list_line(path: &std::path::Path, r: &SessionRecord) -> String {
    format!(
        "{}  {} UTC  {}  {}  exit {}",
        stem(path),
        eidos_instance::format_stamp(r.started),
        r.profile,
        r.tool.as_deref().unwrap_or("game"),
        r.exit.map_or("?".into(), |c| c.to_string())
    )
}

pub(crate) fn cmd_sessions(args: &[String]) {
    let (arg, picked) = match args {
        [arg] => (arg, None),
        [arg, d] if d == "diff" => (arg, Some(None)),
        [arg, d, id] if d == "diff" => (arg, Some(Some(id.as_str()))),
        _ => {
            eidos_log::info!("usage: eidos sessions <instance> [diff [<id>]]");
            std::process::exit(2);
        }
    };
    let target = match crate::resolve::try_resolve(arg) {
        Ok(t) => t,
        Err(e) => {
            eidos_log::info!("eidos: {e}");
            std::process::exit(2);
        }
    };
    let records = target.inst.sessions();
    let Some(picked) = picked else {
        if records.is_empty() {
            eidos_log::info!("No runs are recorded yet.");
        }
        for (path, r) in &records {
            println!("{}", list_line(path, r));
        }
        return;
    };
    let chosen = match picked {
        Some(id) => records.iter().find(|(p, _)| stem(p) == id),
        None => default_target(&records),
    };
    let Some((path, old)) = chosen else {
        match picked {
            Some(id) => {
                eidos_log::info!("No recorded run is named {id}. Run `eidos sessions {arg}`.")
            }
            None => eidos_log::info!("No game run that exited with code 0 is recorded yet."),
        }
        std::process::exit(1);
    };
    let Some(game) = crate::games::find_instance_game(&target) else {
        eidos_log::info!(
            "Game '{}' is not detected. Run `eidos games`.",
            target.game_id
        );
        std::process::exit(2);
    };
    let now = capture(&game, &target.inst, &target.inst.active(), None);
    println!("Changes since {}", list_line(path, old));
    let lines = old.diff(&now);
    if lines.is_empty() {
        println!("No differences.");
    }
    for l in lines {
        println!("{l}");
    }
}
