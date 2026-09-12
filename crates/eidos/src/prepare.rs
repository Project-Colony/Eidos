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
    let spec = eidos_plugins::GameSpec::for_id(id)?;
    let Some(compatdata) = game.compatdata.as_ref() else {
        eidos_log::info!("eidos play: no Proton prefix found, skipping plugins.txt");
        return None;
    };
    let prefix = compatdata.join("pfx");
    let prefix_dir = eidos_plugins::plugins_txt_dir(&prefix, &spec);

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
    let list = inst.plugin_list(&game.data_path, id, Some(&state_dir))?;

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
    let _ = list.write_load_order(&prefix_dir, &spec);

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

/// Before launch: give the active profile its own INIs in the prefix. Seed the
/// profile from the prefix on first run (adopting an existing setup, losing
/// nothing), deploy the profile's INIs into the prefix Documents, then enable BSA
/// invalidation on the deployed copy. Returns the prefix Documents dir + the
/// game's INI set, so the caller can capture in-game changes back afterwards.
pub(crate) fn prepare_inis(
    id: &str,
    game: &DetectedGame,
    inst: &Instance,
    prof: &eidos_instance::Profile,
) -> Option<PreparedInis> {
    let spec = eidos_plugins::GameSpec::for_id(id)?;
    let compatdata = game.compatdata.as_ref()?;
    let ini_files = eidos_gamefeatures::ini_files_for(id);
    if ini_files.is_empty() {
        return None;
    }
    let docs = if id == "morrowind" {
        // Morrowind keeps Morrowind.ini in the install dir (MO2 manages it there),
        // not My Games, so the per-profile INI cycle is pointed at the game dir.
        game.install_path.clone()
    } else {
        eidos_plugins::documents_my_games_dir(&compatdata.join("pfx"), &spec)
    };

    match prof.seed_inis(&docs, ini_files) {
        Ok(n) if n > 0 => {
            eidos_log::info!(
                "eidos play: seeded {n} INI(s) into profile '{}' from the prefix",
                prof.name
            )
        }
        Ok(_) => {}
        Err(e) => eidos_log::warn!("eidos play: WARNING - could not seed profile INIs: {e}"),
    }
    // If the deploy fails, the prefix keeps its OLD INIs; capturing those back
    // after the run would clobber the profile's copies with stale content. So a
    // failed deploy disables this run's capture (see the return below).
    let deploy_ok = match prof.deploy_inis(&docs, ini_files) {
        Ok(n) => {
            if n > 0 {
                eidos_log::info!("eidos play: deployed {n} profile INI(s) into the prefix");
            }
            true
        }
        Err(e) => {
            eidos_log::warn!(
                "eidos play: WARNING - could not deploy profile INIs into the prefix ({e}); \
                 the game runs with the prefix's own INIs and they will NOT be captured back"
            );
            false
        }
    };
    // Mod-shipped INI Tweaks, merged into the DEPLOYED copies in priority order
    // (lowest first, so a higher-priority mod's fragment wins), with the profile's
    // own tweak file last. What each write displaced comes back so the capture can
    // undo it - otherwise a tweak becomes indistinguishable from a setting the
    // user chose, and disabling the fragment would change nothing.
    let mut tweaked: Vec<(String, Vec<eidos_instance::TweakedKey>)> = Vec::new();
    if deploy_ok {
        let fragments = inst.enabled_ini_tweaks(&inst.modlist());
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

    // Loose files must win over the vanilla BSAs, and the Bethesda launcher must not
    // reset the plugin selection (both written into the deployed profile INIs).
    match eidos_gamefeatures::enable_bsa_invalidation(&docs, &inst.overwrite_dir(), id) {
        Ok(()) => eidos_log::warn!("eidos play: BSA invalidation on"),
        Err(e) => eidos_log::warn!("eidos play: could not enable BSA invalidation: {e}"),
    }
    if id == "morrowind" {
        // Morrowind only loads a BSA listed in its numbered [Archives] section;
        // register every enabled mod's top-level .bsa so BSA-shipping mods work.
        let mod_bsas: Vec<String> = inst
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
    // No capture cycle when the deploy failed: the prefix INIs are not this
    // profile's state and must not overwrite it after the run.
    deploy_ok.then_some(PreparedInis {
        docs,
        ini_files,
        tweaked,
    })
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
    /// The prefix directory the INIs were deployed into.
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
    id: &str,
    game: &DetectedGame,
    prof: &eidos_instance::Profile,
) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let spec = eidos_plugins::GameSpec::for_id(id)?;
    let compatdata = game.compatdata.as_ref()?;
    let docs = eidos_plugins::documents_my_games_dir(&compatdata.join("pfx"), &spec);
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
