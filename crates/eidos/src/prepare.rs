//! Staging a profile into the prefix before a launch: plugins.txt, INIs,
//! saves - and capturing what the game wrote back, afterwards.

use std::path::PathBuf;

use eidos_games::DetectedGame;
use eidos_instance::Instance;
#[cfg(test)]
use eidos_instance::ModEntry;

fn runtime_diagnostics(
    game: &DetectedGame,
    inst: &Instance,
) -> Vec<eidos_gamefeatures::preflight::PreflightDiagnostic> {
    let mods = inst
        .modlist()
        .into_iter()
        .rev()
        .filter(|m| m.is_active())
        .map(|m| (m.name, m.path))
        .collect::<Vec<_>>();
    eidos_gamefeatures::preflight::scan_skse(
        game.def,
        &game.data_path,
        &game.install_path,
        &mods,
        &inst.overwrite_dir(),
    )
}

fn plugin_diagnostic_messages(
    list: &eidos_plugins::PluginList,
    spec: &eidos_plugins::GameSpec,
) -> Vec<String> {
    let mut messages = Vec::new();
    let mut unverified = 0;
    for d in list.diagnostics(spec) {
        if d.code == "plugin_records_unverified" {
            unverified += 1;
            continue;
        }
        let origin = if d.origin_mod.is_empty() {
            "Game"
        } else {
            &d.origin_mod
        };
        messages.push(format!(
            "{:?} [{}] {} ({origin}): {}",
            d.severity, d.code, d.plugin, d.detail
        ));
    }
    if unverified > 0 {
        messages.push(format!("Unverified record limits for {unverified} light/medium/update plugin(s): launch preparation reads headers only. Run LOOT to validate full records."));
    }
    messages.extend(
        list.missing_masters()
            .into_iter()
            .map(|(p, m)| format!("Error: {p} is missing master {m} (likely a crash)")),
    );
    messages
}

/// Before launch: give the active profile its own plugin state and hand back the
/// `(profile_plugins_dir, prefix_appdata_dir)` bind pair, exactly like
/// [`prepare_saves`]. The profile's `plugins/` dir is bind-mounted over the
/// game's AppData plugin dir for the run, so the game's own `plugins.txt`
/// rewrite lands IN the profile - one copy of the truth, no post-run capture to
/// revert it, no deploy for a crash to skip. This is MO2's usvfs virtualization
/// of the plugin files, done with the mount namespace the saves already use.
///
/// Best-effort - a game with no plugin system or no Proton prefix is simply
/// skipped (`None`).
pub(crate) fn prepare_plugins(
    id: &str,
    game: &DetectedGame,
    inst: &Instance,
    prof: &eidos_instance::Profile,
) -> Option<(PathBuf, PathBuf)> {
    for d in runtime_diagnostics(game, inst) {
        eidos_log::warn!(
            "eidos play: {:?} [{}] {} ({}): {}",
            d.severity,
            d.code,
            d.path.display(),
            d.origin_mod,
            d.detail
        );
    }
    let spec = game.plugin_spec()?;
    let Some(prefix) = game.prefix() else {
        eidos_log::info!("eidos play: no Proton prefix found, skipping plugins.txt");
        return None;
    };
    let prefix_dir = eidos_plugins::plugin_state_dir(&prefix, &game.install_path, &spec);

    // First run: adopt the prefix's existing state (plugins.txt, loadorder.txt,
    // and the sidecars the game keeps next to them) into the profile, so the
    // bound dir never shows the game less than the dir it wrote.
    match prof.seed_plugin_state(&prefix_dir, &spec) {
        Ok(n) if n > 0 => {
            eidos_log::info!(
                "eidos play: adopted {n} plugin-state file(s) into profile '{}'",
                prof.name
            )
        }
        Ok(_) => {}
        Err(e) => {
            // FAIL CLOSED. Proceeding with a half-seeded (or unwritable) profile
            // dir meant the pass below saw an empty state, fell back to discovery
            // defaults, and the prefix shadow write then OVERWROTE the user's only
            // good copies with an alphabetical everything-enabled list - the exact
            // files the seed just failed to adopt. Plugin management sits out this
            // run; the game reads its own prefix files, untouched.
            eidos_log::warn!(
                "eidos play: WARNING - could not adopt the plugin state into profile '{}' ({e}); \
                 plugin management is OFF for this run, the prefix files are left alone, and \
                 plugin changes made in-game this session will NOT persist to the profile",
                prof.name
            );
            return None;
        }
    }
    let state_dir = prof.plugins_state_dir();

    // Judge the state the LAST session left, BEFORE this launch rewrites
    // anything - the snapshot-keeping decision below depends on it, and taking
    // the measurement after our own write poisoned it in both directions.
    let session_damage =
        eidos_plugins::GameSpec::for_id(id).and_then(|spec| prof.plugin_loss_since_snapshot(&spec));

    // The shared merged view preserves profile order, enabled state and pins,
    // and removes plugins hidden by the launcher's whiteouts.
    let list = inst.plugin_list_for_profile(&game.data_path, id, Some(&state_dir), prof)?;

    for message in plugin_diagnostic_messages(&list, &spec) {
        eidos_log::warn!("eidos play: {message}");
    }
    let active = list.plugins.iter().filter(|p| p.enabled).count();
    match list.write_load_order(&state_dir, &spec) {
        Ok(listed) => {
            // `listed` is what plugins.txt actually holds; `active` includes the
            // primaries and Creations the engine loads by itself, which are
            // deliberately NOT in the file. Reporting `active` as "written" once
            // pointed a whole investigation at a file that was never wrong.
            eidos_log::info!(
                "eidos play: wrote plugins.txt ({listed} listed, {active} active incl. implicit)"
            );
        }
        Err(e) => {
            // Same fail-closed rule as the seed: if the PROFILE copy could not be
            // written, the shadow below must not run - it would push a state that
            // exists nowhere else onto the prefix, destroying the real files.
            eidos_log::warn!(
                "eidos play: WARNING - could not write the profile plugins.txt ({e}); plugin \
                 management is OFF for this run and the prefix files are left alone"
            );
            return None;
        }
    }
    // Shadow copy into the real prefix dir - only after the profile write above
    // succeeded, so the shadow is always a copy of durable state, never the sole
    // copy of anything. External tools (LOOT, xEdit run outside Eidos) read the
    // prefix, and if the bind ever fails the game reads exactly what a pre-bind
    // session would have. Never fatal.
    if spec.mechanism != eidos_plugins::LoadOrderMechanism::Timestamp {
        let _ = list.write_load_order(&prefix_dir, &spec);
    }

    // Pre-session snapshot: with the game writing the profile file directly,
    // this is the reference the post-run loss check compares against. KEPT, not
    // overwritten, while the LAST session's damage is still unresolved - judged
    // BEFORE our own write above, because judging after broke it both ways: a
    // header-only crash artifact was replaced by discovery defaults and then
    // judged healthy (laundering the wipe and destroying the pinned restore
    // copy), while our own legitimate prune after a Mods-tab disable was judged
    // as damage and flamed a false alarm on every launch.
    if session_damage.is_some() {
        eidos_log::info!(
            "eidos play: keeping the previous pre-session plugins.txt snapshot - the last \
             session's damage is unresolved (the GUI Diagnostics tab offers the restore, or \
             accept the current set there)"
        );
    } else if let Err(e) = prof.snapshot_plugin_state() {
        eidos_log::warn!("eidos play: WARNING - could not snapshot plugins.txt: {e}");
    }
    Some((state_dir, prefix_dir))
}

/// Prepare a root-mode activation file without letting a previous case variant
/// or deletion marker mask the selected profile's state.
pub(crate) fn stage_root_activation(
    prof: &eidos_instance::Profile,
    runtime: &std::path::Path,
    spec: &eidos_plugins::GameSpec,
) -> std::io::Result<()> {
    let active = spec.active_file();
    let source = eidos_plugins::newest_variant(&prof.plugins_state_dir(), active)
        .ok_or_else(|| std::io::Error::other("Missing root activation state"))?;
    let bytes = std::fs::read(source)?;
    for entry in std::fs::read_dir(runtime)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name == active.to_ascii_lowercase()
            || name == format!(".eidoswh.{}", active.to_ascii_lowercase())
        {
            std::fs::remove_file(entry.path())?;
        }
    }
    eidos_instance::write_atomic(&runtime.join(active), &bytes)
}

/// Apply a tool's last virtual timestamps to durable profile order, then consume
/// the receipt. Keeping it until the write succeeds also covers a killed launcher.
pub(crate) fn recover_timestamp_order(
    id: &str,
    game: &DetectedGame,
    inst: &Instance,
    prof: &eidos_instance::Profile,
) -> std::io::Result<()> {
    let receipt = prof.dir().join("plugin-times.pending");
    let times = match eidos_launch::read_plugin_mtimes(&receipt) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let spec = game
        .plugin_spec()
        .ok_or_else(|| std::io::Error::other("No plugin specification for timestamp capture"))?;
    if spec.mechanism != eidos_plugins::LoadOrderMechanism::Timestamp {
        return Err(std::io::Error::other(
            "Unexpected timestamp receipt for this game",
        ));
    }
    let mut list = inst
        .plugin_list_for_profile(&game.data_path, id, None, prof)
        .ok_or_else(|| std::io::Error::other("No plugin list for timestamp capture"))?;
    list.apply_mtime_order(&times, &spec);
    list.write_load_order(&prof.plugins_state_dir(), &spec)?;
    std::fs::remove_file(receipt)
}

pub(crate) fn prepare_plugin_timestamps(
    id: &str,
    game: &DetectedGame,
    inst: &Instance,
    prof: &eidos_instance::Profile,
) -> std::io::Result<Option<eidos_launch::PluginTimestamps>> {
    let Some(spec) = game.plugin_spec() else {
        return Ok(None);
    };
    if spec.mechanism != eidos_plugins::LoadOrderMechanism::Timestamp {
        return Ok(None);
    }
    let list = inst
        .plugin_list_for_profile(&game.data_path, id, None, prof)
        .ok_or_else(|| std::io::Error::other("No plugin list for timestamp projection"))?;
    Ok(Some(eidos_launch::PluginTimestamps {
        times: list.virtual_mtimes(&spec)?,
        state_path: prof.dir().join("plugin-times.pending"),
    }))
}

/// Seed profile INIs once, then prepare the runtime copy. Install-root INIs use
/// the profile's private root upper; ordinary Documents INIs keep their prefix
/// path. Activation is prepared first so Morrowind uses one INI throughout.
pub(crate) fn prepare_inis(
    id: &str,
    game: &DetectedGame,
    inst: &Instance,
    prof: &eidos_instance::Profile,
) -> std::io::Result<Option<PreparedInis>> {
    let Some(spec) = game.plugin_spec() else {
        return Ok(None);
    };
    let Some(prefix) = game.prefix() else {
        return Ok(None);
    };
    let ini_files = eidos_gamefeatures::ini_files_for(id);
    if ini_files.is_empty() {
        return Ok(None);
    }
    let root_mode =
        eidos_plugins::plugin_state_dir(&prefix, &game.install_path, &spec) == game.install_path;
    let source = if root_mode {
        game.install_path.clone()
    } else {
        eidos_plugins::documents_my_games_dir(&prefix, &spec)
    };
    let docs = if root_mode {
        prof.dir().join("runtime-root")
    } else {
        source.clone()
    };
    prof.seed_inis(&source, ini_files)?;
    if root_mode {
        std::fs::create_dir_all(&docs)?;
        // A prior launcher may have atomically replaced or removed a case variant.
        // Preparation happens only after the pending capture was recovered.
        for entry in std::fs::read_dir(&docs)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if ini_files.iter().any(|f| {
                name == f.to_ascii_lowercase()
                    || name == format!(".eidoswh.{}", f.to_ascii_lowercase())
            }) {
                std::fs::remove_file(entry.path())?;
            }
        }
    }
    prof.deploy_inis(&docs, ini_files)?;

    // Mod-shipped INI Tweaks, merged into the DEPLOYED copies in priority order
    // (lowest first, so a higher-priority mod's fragment wins), with the profile's
    // own tweak file last. What each write displaced comes back so the capture can
    // undo it - otherwise a tweak becomes indistinguishable from a setting the
    // user chose, and disabling the fragment would change nothing.
    let mut tweaked: Vec<(String, Vec<eidos_instance::TweakedKey>)> = Vec::new();
    {
        let fragments = inst.enabled_ini_tweaks(&prof.modlist());
        // The profile's own file counts even when no mod contributes one.
        if !fragments.is_empty() || prof.tweaks_path().is_file() {
            // First run against a fresh prefix: nothing seeded, nothing owned,
            // so every tweak is skipped below. Self-heals one session later (the
            // game writes its INIs, the capture adopts them) - but silently
            // skipping the user's own initweaks.ini deserves a line of honesty.
            if !ini_files.iter().any(|f| prof.ini_path(f).is_file()) {
                eidos_log::info!(
                    "eidos play: INI tweaks skipped this run - profile '{}' owns no INIs yet \
                     (they are adopted after the first session)",
                    prof.name
                );
            }
            for f in ini_files {
                // Only files the profile OWNS (and therefore deployed above).
                // Applying fragments to every name in the game's INI set
                // materialised files the profile never had - and a later capture
                // then adopted the invention as the user's own config.
                if !prof.ini_path(f).is_file() {
                    continue;
                }
                match prof.apply_ini_tweaks(&docs.join(f), &fragments) {
                    Ok(rec) if !rec.is_empty() => {
                        eidos_log::info!("eidos play: applied {} INI tweak(s) to {f}", rec.len());
                        tweaked.push((f.to_string(), rec));
                    }
                    Ok(_) => {}
                    Err(e) => eidos_log::warn!(
                        "eidos play: WARNING - could not apply INI tweaks to {f}: {e}"
                    ),
                }
            }
        }
    }

    if root_mode && id == "oblivion" {
        // The install-root selector decides this session's paths. A profile INI
        // adopted in ordinary mode must not silently switch them back at runtime.
        eidos_gamefeatures::set_ini_key(
            &docs.join("Oblivion.ini"),
            "General",
            "bUseMyGamesDirectory",
            "0",
        )?;
    }

    // Loose files must win over the vanilla BSAs, and the Bethesda launcher must not
    // reset the plugin selection (both written into the deployed profile INIs).
    match eidos_gamefeatures::enable_bsa_invalidation(&docs, &inst.overwrite_dir(), id) {
        Ok(()) => eidos_log::warn!("eidos play: BSA invalidation on"),
        Err(e) => eidos_log::warn!("eidos play: could not enable BSA invalidation: {e}"),
    }
    if id == "morrowind" {
        // Morrowind only loads a BSA listed in its numbered [Archives] section;
        // register every enabled mod's top-level .bsa so BSA-shipping mods work.
        let mod_bsas: Vec<String> = prof
            .modlist()
            .into_iter()
            .filter(|m| m.is_active())
            .flat_map(|m| {
                std::fs::read_dir(&m.path)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter_map(|e| {
                        let n = e.file_name().to_string_lossy().into_owned();
                        n.to_ascii_lowercase().ends_with(".bsa").then_some(n)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let _ =
            eidos_gamefeatures::register_morrowind_archives(&docs.join("Morrowind.ini"), &mod_bsas);
    } else if let Some(ini) = eidos_gamefeatures::ini_file_for(id) {
        if let Err(e) = eidos_gamefeatures::enable_file_selection(&docs, ini) {
            eidos_log::warn!("eidos play: could not enable launcher file selection: {e}");
        }
    }
    Ok(Some(PreparedInis {
        docs,
        ini_files,
        tweaked,
        root_mode,
    }))
}

/// One-way sync of the profile's save files into the REAL prefix Saves dir,
/// after the run (the bind is gone; the prefix dir is reachable again).
///
/// Steam Cloud only ever reads the prefix path - it knows nothing of the bind -
/// so without this the cloud backs up whatever the prefix held before Eidos
/// existed, forever (observed: two saves from 2024 while the profile carried the
/// whole 2026 playthrough). Copies `.ess`/`.skse` files that are missing or
/// newer; never deletes, and never touches anything else - the prefix is a
/// backup target here, not an authority. Returns how many files were copied.
pub(crate) fn sync_saves_for_cloud(
    prof_saves: &std::path::Path,
    prefix_saves: &std::path::Path,
) -> std::io::Result<u32> {
    // The cloud is a recent-history backup, not an archive: cap what one sync
    // pushes, or the first run after adopting a long playthrough shoves the
    // entire save history at Steam's per-game quota in one go. Newest first;
    // the factor of 2 leaves room for each save's .skse co-save.
    const MAX_FILES: usize = 60;
    let Ok(rd) = std::fs::read_dir(prof_saves) else {
        return Ok(0); // no profile saves yet: nothing to back up
    };
    std::fs::create_dir_all(prefix_saves)?;
    // Real save data only: the .ess save and its .skse co-save. The junk a
    // Saves dir accumulates (steam_autocloud.vdf, .bak files) must not bounce
    // between the two dirs forever.
    let mut files: Vec<(std::path::PathBuf, std::time::SystemTime)> = rd
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            // The shared predicate: hard-coding .ess/.skse here killed the cloud
            // backup for the Fallout and Starfield families.
            if !eidos_instance::is_save_data(&e.file_name().to_string_lossy()) || !path.is_file() {
                return None;
            }
            // An unreadable mtime sorts oldest rather than being skipped: an
            // extra copy is cheap, a hole in the backup is not.
            let mtime = e
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            Some((path, mtime))
        })
        .collect();
    files.sort_by_key(|f| std::cmp::Reverse(f.1));
    files.truncate(MAX_FILES);

    // What THIS sync has written to the prefix over its lifetime, so a later run
    // can tell its own copies from saves some session wrote there directly.
    // Lives on the profile side: the prefix belongs to the game and to Steam.
    let manifest_path = prof_saves.join(".cloud-sync-manifest");
    let mut manifest: std::collections::HashSet<String> = std::fs::read_to_string(&manifest_path)
        .map(|t| t.lines().map(String::from).collect())
        .unwrap_or_default();
    let mut new_entries: Vec<String> = Vec::new();

    // Rescue the whole batch before replacing any file, so a failed co-save
    // rescue also leaves its prefix save untouched.
    let mut copies = Vec::new();
    for (src, src_mtime) in files {
        let Some(name) = src.file_name() else {
            continue;
        };
        let dst = prefix_saves.join(name);
        match std::fs::metadata(&dst).and_then(|m| m.modified()) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
            Ok(d) if src_mtime > d => {
                if !manifest.contains(&manifest_key(&dst)?) {
                    preserve_diverged_save(&dst, d, prof_saves)?;
                }
            }
            Ok(_) => continue,
        }
        copies.push((src, dst, src_mtime));
    }
    let mut n = 0;
    for (src, dst, src_mtime) in copies {
        std::fs::copy(&src, &dst)?;
        if let Ok(f) = std::fs::File::options().write(true).open(&dst) {
            let _ = f.set_modified(src_mtime);
        }
        new_entries.push(manifest_key(&dst)?);
        n += 1;
    }
    if !new_entries.is_empty() {
        manifest.extend(new_entries);
        let body: String = manifest.iter().map(|e| format!("{e}\n")).collect();
        let _ = std::fs::write(&manifest_path, body);
    }
    Ok(n)
}

/// Content provenance: matching sizes and second-resolution mtimes cannot prove
/// the prefix still holds our previous copy. A stdlib hash change merely causes
/// a conservative rescue on the first sync after an upgrade.
fn manifest_key(path: &std::path::Path) -> std::io::Result<String> {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    std::fs::read(path)?.hash(&mut hash);
    Ok(format!(
        "{}\t{:016x}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        hash.finish()
    ))
}

/// Preserve an unknown prefix save and all same-stem save data before replacing
/// either half. Existing rescue names must match bytes; a collision is an error.
pub(crate) fn preserve_diverged_save(
    dst: &std::path::Path,
    dst_mtime: std::time::SystemTime,
    prof_saves: &std::path::Path,
) -> std::io::Result<()> {
    use std::io::Write;
    let parent = dst
        .parent()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let stem = dst.file_stem().unwrap_or_default().to_string_lossy();
    let mut group = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let path = entry?.path();
        if path
            .file_stem()
            .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(&stem))
            && eidos_instance::is_save_data(&path.file_name().unwrap_or_default().to_string_lossy())
        {
            group.push((path.clone(), std::fs::read(&path)?));
        }
    }
    // Anchor the rescue name on the save, even when the co-save was newer and
    // therefore reached the sync first. Their mtimes need not be identical.
    let anchor = group
        .iter()
        .find(|(p, _)| eidos_instance::is_save_listing(&p.to_string_lossy()))
        .or_else(|| group.first())
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
    for entry in std::fs::read_dir(prof_saves)? {
        let candidate = entry?.path();
        if candidate.extension() == anchor.0.extension()
            && group.iter().all(|(path, bytes)| {
                std::fs::read(candidate.with_extension(path.extension().unwrap_or_default()))
                    .is_ok_and(|known| known == *bytes)
            })
        {
            return Ok(());
        }
    }
    let secs = std::fs::metadata(&anchor.0)?
        .modified()
        .unwrap_or(dst_mtime)
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    for (path, bytes) in group {
        let orphan = prof_saves.join(format!(
            "orphan-{secs}-{}",
            path.file_name().unwrap().to_string_lossy()
        ));
        match std::fs::File::options()
            .write(true)
            .create_new(true)
            .open(&orphan)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
                    let _ = std::fs::remove_file(&orphan);
                    return Err(error);
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::AlreadyExists
                    && std::fs::read(&orphan).is_ok_and(|known| known == bytes) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// What `prepare_inis` leaves for the post-run capture.
pub(crate) struct PreparedInis {
    pub(crate) root_mode: bool,
    /// The runtime directory the INIs were deployed into.
    pub(crate) docs: std::path::PathBuf,
    pub(crate) ini_files: &'static [&'static str],
    /// Per INI file, the keys an INI tweak overwrote, so the capture can put the
    /// profile's own values back.
    pub(crate) tweaked: Vec<(String, Vec<eidos_instance::TweakedKey>)>,
}

/// Before launch: give the active profile its own saves. Seed the profile from
/// the prefix's existing saves on first run (adopting the playthrough), then
/// return the `(profile_saves, prefix_saves)` bind so the launcher redirects the
/// game's save dir to this profile for the run - the prefix is never modified.
pub(crate) fn prepare_saves(
    game: &DetectedGame,
    prof: &eidos_instance::Profile,
) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let spec = game.plugin_spec()?;
    let prefix = game.prefix()?;
    let docs = eidos_plugins::documents_my_games_dir(&prefix, &spec);
    let prefix_saves = docs.join("Saves");
    if let Ok(n) = prof.seed_saves(&prefix_saves) {
        if n > 0 {
            eidos_log::info!(
                "eidos play: adopted {n} existing save(s) into profile '{}'",
                prof.name
            );
        }
    }
    Some((prof.saves_dir(), prefix_saves))
}

#[cfg(test)]
mod rescue_tests {
    use super::*;
    use std::{
        fs,
        time::{Duration, UNIX_EPOCH},
    };

    #[test]
    fn launch_preparation_reports_corrupt_plugins_and_winning_dlls() {
        let root =
            std::env::temp_dir().join(format!("eidos-launch-preflight-{}", std::process::id()));
        let inst = Instance::portable(root.clone());
        inst.create().unwrap();
        let path = inst.mods_dir().join("Broken");
        fs::create_dir_all(path.join("SKSE/Plugins")).unwrap();
        fs::write(path.join("Broken.esp"), b"TES4").unwrap();
        fs::write(path.join("SKSE/Plugins/Crash.dll"), b"invalid PE").unwrap();
        inst.save_modlist(&[ModEntry {
            name: "Broken".into(),
            enabled: true,
            path: path.clone(),
            unmanaged: false,
        }])
        .unwrap();
        let spec = eidos_plugins::GameSpec::for_id("skyrimse").unwrap();
        let list = eidos_plugins::PluginList::discover(&[("Broken".into(), path)], &spec);
        assert!(plugin_diagnostic_messages(&list, &spec)
            .iter()
            .any(|m| m.contains("plugin_header") && m.contains("Broken.esp")));
        let def = eidos_games::catalog()
            .iter()
            .find(|g| g.id == "skyrimse")
            .unwrap();
        let game = DetectedGame {
            def,
            install_path: root.join("game"),
            data_path: root.join("game/Data"),
            compatdata: None,
            source: Default::default(),
            steam_name: String::new(),
        };
        assert!(runtime_diagnostics(&game, &inst)
            .iter()
            .any(|d| d.code == "pe_unverified" && d.origin_mod == "Broken"));
        fs::create_dir_all(inst.overwrite_dir().join("SKSE/Plugins")).unwrap();
        fs::write(
            inst.overwrite_dir().join("SKSE/Plugins/.eidoswh.crash.dll"),
            [],
        )
        .unwrap();
        assert!(runtime_diagnostics(&game, &inst).is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    fn engine_plugin(path: &std::path::Path, morrowind: bool) {
        let mut body = b"HEDR".to_vec();
        if morrowind {
            body.extend(300u32.to_le_bytes());
            body.extend(1.3f32.to_le_bytes());
            body.extend(1u32.to_le_bytes());
            body.extend([0; 292]);
        } else {
            body.extend(12u16.to_le_bytes());
            body.extend(1.0f32.to_le_bytes());
            body.extend([0; 8]);
        }
        let mut header = vec![0; if morrowind { 16 } else { 20 }];
        header[..4].copy_from_slice(if morrowind { b"TES3" } else { b"TES4" });
        header[4..8].copy_from_slice(&(body.len() as u32).to_le_bytes());
        if !morrowind {
            header[8] = 1;
        }
        header.extend(body);
        fs::write(path, header).unwrap();
    }

    #[test]
    fn timestamp_root_preparation_is_private_and_recovers_profile_settings() {
        let root = std::env::temp_dir().join(format!("eidos-root-prepare-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let inst = Instance::portable(root.join("instance"));
        inst.create().unwrap();
        let def = eidos_games::catalog()
            .iter()
            .find(|g| g.id == "morrowind")
            .unwrap();
        let game = DetectedGame {
            def,
            install_path: root.join("game"),
            data_path: root.join("game/Data Files"),
            compatdata: Some(root.join("compatdata")),
            source: Default::default(),
            steam_name: String::new(),
        };
        fs::create_dir_all(&game.data_path).unwrap();
        fs::create_dir_all(game.compatdata.as_ref().unwrap().join("pfx")).unwrap();
        let original=b"[General]\r\nLanguage=Fran\xe7ais\r\nValue=1\r\n[Archives]\r\nArchive 0=Morrowind.bsa\r\n[Game Files]\r\nGameFile0=Morrowind.esm\r\n";
        fs::write(game.install_path.join("Morrowind.ini"), original).unwrap();
        fs::write(game.install_path.join("Morrowind.exe"), b"game").unwrap();
        engine_plugin(&game.data_path.join("Morrowind.esm"), true);
        let moddir = inst.mods_dir().join("Archive");
        fs::create_dir_all(&moddir).unwrap();
        fs::write(moddir.join("Extra.bsa"), b"archive").unwrap();
        inst.save_modlist(&[ModEntry {
            name: "Archive".into(),
            path: moddir,
            enabled: true,
            unmanaged: false,
        }])
        .unwrap();
        let prof = inst.active();
        fs::write(prof.tweaks_path(), "[General]\nValue=2\n").unwrap();
        let binding = prepare_plugins("morrowind", &game, &inst, &prof).unwrap();
        assert_eq!(binding.1, game.install_path);
        let prepared = prepare_inis("morrowind", &game, &inst, &prof)
            .unwrap()
            .unwrap();
        assert_eq!(prepared.docs, prof.dir().join("runtime-root"));
        let text = eidos_plugins::read_decoded(&prepared.docs.join("Morrowind.ini")).unwrap();
        assert!(text.contains("Extra.bsa") && text.contains("GameFile0=Morrowind.esm"));
        assert!(text.contains("Français"));
        assert!(text.contains("Value=2"));
        assert_eq!(
            prof.ini_path("Morrowind.ini"),
            prof.plugins_state_dir().join("Morrowind.ini")
        );
        assert_eq!(
            fs::read(game.install_path.join("Morrowind.ini")).unwrap(),
            original
        );
        prof.record_ini_session(&prepared.docs, prepared.ini_files, &prepared.tweaked)
            .unwrap();
        fs::write(prepared.docs.join("generated.log"), b"output").unwrap();
        fs::create_dir_all(inst.root_overwrite_dir()).unwrap();
        fs::write(
            inst.root_overwrite_dir().join("Morrowind.ini"),
            b"stale shared ini",
        )
        .unwrap();
        prof.recover_ini_session().unwrap();
        prof.merge_runtime_root_outputs(&inst.root_overwrite_dir(), &["Morrowind.ini"])
            .unwrap();
        let captured = eidos_plugins::read_decoded(&prof.ini_path("Morrowind.ini")).unwrap();
        assert!(captured.contains("Value=1") && captured.contains("Extra.bsa"));
        assert_eq!(
            fs::read(inst.root_overwrite_dir().join("generated.log")).unwrap(),
            b"output"
        );
        assert!(!prepared.docs.join("generated.log").exists());
        assert_eq!(
            fs::read(inst.root_overwrite_dir().join("Morrowind.ini")).unwrap(),
            b"stale shared ini"
        );
        assert_eq!(
            fs::read(game.install_path.join("Morrowind.ini")).unwrap(),
            original
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oblivion_root_and_appdata_preparation_never_write_activation_to_game() {
        let root =
            std::env::temp_dir().join(format!("eidos-oblivion-prepare-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let inst = Instance::portable(root.join("instance"));
        inst.create().unwrap();
        let def = eidos_games::catalog()
            .iter()
            .find(|g| g.id == "oblivion")
            .unwrap();
        let game = DetectedGame {
            def,
            install_path: root.join("game"),
            data_path: root.join("game/Data"),
            compatdata: Some(root.join("compatdata")),
            source: Default::default(),
            steam_name: String::new(),
        };
        fs::create_dir_all(&game.data_path).unwrap();
        fs::create_dir_all(game.compatdata.as_ref().unwrap().join("pfx")).unwrap();
        engine_plugin(&game.data_path.join("Oblivion.esm"), false);
        let spec = game.plugin_spec().unwrap();
        let prefix = game.prefix().unwrap();
        let local = eidos_plugins::plugins_txt_dir(&prefix, &spec);
        fs::create_dir_all(&local).unwrap();
        fs::write(local.join("Plugins.txt"), "Oblivion.esm\n").unwrap();
        for root_mode in [false, true] {
            let ini = format!(
                "[General]\nbUseMyGamesDirectory={}\n[Display]\nValue=1\n",
                if root_mode { 0 } else { 1 }
            );
            fs::write(game.install_path.join("Oblivion.ini"), &ini).unwrap();
            fs::write(
                game.install_path.join("Plugins.txt"),
                "# root original\nOblivion.esm\n",
            )
            .unwrap();
            let profile = inst.profile(if root_mode { "Root" } else { "Documents" });
            fs::create_dir_all(profile.dir()).unwrap();
            let bind = prepare_plugins("oblivion", &game, &inst, &profile).unwrap();
            assert_eq!(
                bind.1,
                if root_mode {
                    game.install_path.clone()
                } else {
                    local.clone()
                }
            );
            let prepared = prepare_inis("oblivion", &game, &inst, &profile)
                .unwrap()
                .unwrap();
            assert_eq!(prepared.root_mode, root_mode);
            assert_ne!(prepared.docs, game.install_path);
            if root_mode {
                fs::write(prepared.docs.join("Plugins.txt"), b"stale case variant").unwrap();
                fs::write(prepared.docs.join(".eidoswh.plugins.txt"), []).unwrap();
                stage_root_activation(&profile, &prepared.docs, &spec).unwrap();
                assert!(!prepared.docs.join("Plugins.txt").exists());
                assert!(!prepared.docs.join(".eidoswh.plugins.txt").exists());
                assert_eq!(
                    fs::read(prepared.docs.join("plugins.txt")).unwrap(),
                    fs::read(profile.plugins_txt_path()).unwrap()
                );
            }
            assert_eq!(
                fs::read_to_string(game.install_path.join("Oblivion.ini")).unwrap(),
                ini
            );
            assert_eq!(
                fs::read_to_string(game.install_path.join("Plugins.txt")).unwrap(),
                "# root original\nOblivion.esm\n"
            );
            assert_eq!(
                fs::read_to_string(local.join("Plugins.txt")).unwrap(),
                "Oblivion.esm\n"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn timestamp_capture_persists_the_selected_profile_and_consumes_only_good_receipts() {
        let root =
            std::env::temp_dir().join(format!("eidos-timestamp-capture-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let inst = Instance::portable(root.join("instance"));
        inst.create().unwrap();
        let def = eidos_games::catalog()
            .iter()
            .find(|g| g.id == "oblivion")
            .unwrap();
        let game = DetectedGame {
            def,
            install_path: root.join("game"),
            data_path: root.join("game/Data"),
            compatdata: None,
            source: Default::default(),
            steam_name: String::new(),
        };
        fs::create_dir_all(&game.data_path).unwrap();
        for name in ["Oblivion.esm", "A.esp", "B.esp"] {
            engine_plugin(&game.data_path.join(name), false);
        }
        let a = inst.active();
        let b = inst.profile("Other");
        fs::create_dir_all(b.plugins_state_dir()).unwrap();
        fs::write(b.plugins_txt_path(), b"Oblivion.esm\nA.esp\n").unwrap();
        fs::write(a.plugins_txt_path(), b"Oblivion.esm\nB.esp\n").unwrap();
        fs::write(a.loadorder_txt_path(), b"Oblivion.esm\nA.esp\nB.esp\n").unwrap();
        let other_before = fs::read(b.plugins_txt_path()).unwrap();
        let sources_before = fs::read_dir(&game.data_path)
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                (e.path(), e.metadata().unwrap().modified().unwrap())
            })
            .collect::<Vec<_>>();
        let pending = a.dir().join("plugin-times.pending");
        fs::write(&pending,"Eidos plugin timestamps v1\n1000000000\toblivion.esm\n9000000000\ta.esp\n2000000000\tb.esp\n").unwrap();
        recover_timestamp_order("oblivion", &game, &inst, &a).unwrap();
        assert!(!pending.exists());
        let list = inst
            .plugin_list_for_profile(&game.data_path, "oblivion", None, &a)
            .unwrap();
        assert_eq!(
            list.plugins
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["Oblivion.esm", "B.esp", "A.esp"]
        );
        assert!(
            !list
                .plugins
                .iter()
                .find(|p| p.name == "A.esp")
                .unwrap()
                .enabled
        );
        assert_eq!(fs::read(b.plugins_txt_path()).unwrap(), other_before);
        let projected = prepare_plugin_timestamps("oblivion", &game, &inst, &a)
            .unwrap()
            .unwrap();
        assert!(projected.times["b.esp"] < projected.times["a.esp"]);
        fs::write(&pending, "broken receipt").unwrap();
        assert!(recover_timestamp_order("oblivion", &game, &inst, &a).is_err());
        assert!(pending.exists());
        for (path, time) in sources_before {
            assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), time);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cloud_sync_refuses_unverified_rescue_collisions_before_overwriting() {
        let root =
            std::env::temp_dir().join(format!("eidos-rescue-collision-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let profile = root.join("profile");
        let prefix = root.join("prefix");
        fs::create_dir_all(&profile).unwrap();
        fs::create_dir_all(&prefix).unwrap();
        let old = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        for ext in ["ess", "skse"] {
            let name = format!("quicksave.{ext}");
            fs::write(profile.join(&name), b"new session").unwrap();
            fs::write(prefix.join(&name), b"old session").unwrap();
            fs::File::options()
                .write(true)
                .open(prefix.join(&name))
                .unwrap()
                .set_modified(old)
                .unwrap();
            fs::write(
                profile.join(format!("orphan-1700000000-{name}")),
                b"collision!!",
            )
            .unwrap();
        }
        assert!(sync_saves_for_cloud(&profile, &prefix).is_err());
        for ext in ["ess", "skse"] {
            assert_eq!(
                fs::read(prefix.join(format!("quicksave.{ext}"))).unwrap(),
                b"old session"
            );
        }
        // A size/mtime twin with different bytes is not a verified backup.
        fs::write(profile.join("other.ess"), b"not session").unwrap();
        fs::File::options()
            .write(true)
            .open(profile.join("other.ess"))
            .unwrap()
            .set_modified(old)
            .unwrap();
        for ext in ["ess", "skse"] {
            fs::remove_file(profile.join(format!("orphan-1700000000-quicksave.{ext}"))).unwrap();
        }
        fs::File::options()
            .write(true)
            .open(prefix.join("quicksave.skse"))
            .unwrap()
            .set_modified(old + Duration::from_secs(2))
            .unwrap();
        sync_saves_for_cloud(&profile, &prefix).unwrap();
        for ext in ["ess", "skse"] {
            assert_eq!(
                fs::read(profile.join(format!("orphan-1700000000-quicksave.{ext}"))).unwrap(),
                b"old session"
            );
            assert_eq!(
                fs::read(prefix.join(format!("quicksave.{ext}"))).unwrap(),
                b"new session"
            );
        }
        // A prefix rewrite with the same metadata must not impersonate our last sync.
        let modified = fs::metadata(prefix.join("quicksave.ess"))
            .unwrap()
            .modified()
            .unwrap();
        fs::write(prefix.join("quicksave.ess"), b"bad session").unwrap();
        fs::File::options()
            .write(true)
            .open(prefix.join("quicksave.ess"))
            .unwrap()
            .set_modified(modified)
            .unwrap();
        fs::File::options()
            .write(true)
            .open(profile.join("quicksave.ess"))
            .unwrap()
            .set_modified(modified + Duration::from_secs(2))
            .unwrap();
        sync_saves_for_cloud(&profile, &prefix).unwrap();
        assert!(fs::read_dir(&profile).unwrap().flatten().any(|entry| {
            entry.file_name().to_string_lossy().starts_with("orphan-")
                && fs::read(entry.path()).is_ok_and(|bytes| bytes == b"bad session")
        }));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_rescue_keeps_both_prefix_files_unchanged() {
        use std::os::unix::fs::PermissionsExt;
        let root =
            std::env::temp_dir().join(format!("eidos-rescue-write-error-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let profile = root.join("profile");
        let prefix = root.join("prefix");
        fs::create_dir_all(&profile).unwrap();
        fs::create_dir_all(&prefix).unwrap();
        for ext in ["ess", "skse"] {
            fs::write(profile.join(format!("quicksave.{ext}")), b"new session").unwrap();
            let destination = prefix.join(format!("quicksave.{ext}"));
            fs::write(&destination, b"old session").unwrap();
            fs::File::options()
                .write(true)
                .open(destination)
                .unwrap()
                .set_modified(UNIX_EPOCH + Duration::from_secs(1_700_000_000))
                .unwrap();
        }
        fs::set_permissions(&profile, fs::Permissions::from_mode(0o555)).unwrap();
        let result = sync_saves_for_cloud(&profile, &prefix);
        fs::set_permissions(&profile, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err());
        for ext in ["ess", "skse"] {
            assert_eq!(
                fs::read(prefix.join(format!("quicksave.{ext}"))).unwrap(),
                b"old session"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}
