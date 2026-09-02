//! Staging a profile into the prefix before a launch: plugins.txt, INIs,
//! saves - and capturing what the game wrote back, afterwards.

use std::path::PathBuf;

use eidos_games::DetectedGame;
use eidos_instance::{Instance, ModEntry};

/// The plugin-discovery sources in ASCENDING priority (later wins same-name
/// shadowing), as fed to [`eidos_plugins::PluginList::discover`]: the game's own
/// Data (lowest), then each enabled mod from lowest to highest priority, then the
/// Overwrite layer LAST (highest). Overwrite is the always-on writable top layer
/// the launcher mounts, mirroring MO2's always-active top-priority Overwrite
/// pseudo-mod - plugins a tool wrote there (xEdit / Bashed Patch output) must be
/// discovered, not dropped from plugins.txt.
pub(crate) fn plugin_sources(
    game_data: &std::path::Path,
    enabled_lowest_first: &[ModEntry],
    overwrite: &std::path::Path,
) -> Vec<(String, PathBuf)> {
    let mut sources: Vec<(String, PathBuf)> = vec![(String::new(), game_data.to_path_buf())];
    // `modlist()` is already in ascending-priority (MO2 display) order, so feed the
    // mods through as-is: lowest priority first, highest last.
    sources.extend(
        enabled_lowest_first
            .iter()
            .map(|m| (m.name.clone(), m.path.clone())),
    );
    sources.push(("overwrite".to_string(), overwrite.to_path_buf()));
    sources
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

    // Sources in ascending plugin priority: the game's own Data (lowest), each
    // enabled mod, then the Overwrite layer last (highest) so plugins a tool wrote
    // into Overwrite are discovered and win same-name shadowing.
    let enabled: Vec<ModEntry> = inst
        .modlist()
        .into_iter()
        .filter(|m| m.is_active())
        .collect();
    let sources = plugin_sources(&game.data_path, &enabled, &inst.overwrite_dir());

    let mut list = eidos_plugins::PluginList::discover(&sources, &spec);

    // Preserve the user's saved order + enabled state, FROM THE PROFILE - the
    // single source of truth. loadorder.txt is the order authority; plugins.txt
    // supplies the flags (it deliberately omits the primaries and Creations, so
    // it cannot order them).
    list.apply_prefix_state(&state_dir, &spec);
    // The pinned positions are part of that saved state. Loading them only in
    // the GUI made a pin a window decoration: this pass rewrites the order right
    // before the game starts, so an unpinned launch silently handed the engine a
    // load order the user had explicitly nailed down.
    list.locked = prof.read_locked_order();
    list.refresh(&spec);

    for (p, m) in list.missing_masters() {
        eidos_log::warn!("eidos play: WARNING - {p} is missing master {m} (likely a crash)");
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

    let mut n = 0;
    for (src, src_mtime) in files {
        let Some(name) = src.file_name() else {
            continue;
        };
        let dst = prefix_saves.join(name);
        match std::fs::metadata(&dst).and_then(|m| m.modified()) {
            Err(_) => {} // missing: plain copy below
            Ok(d) if src_mtime > d => {
                // The prefix copy is older, but it may be a save the profile has
                // NEVER seen: a failed-bind session once wrote saves straight
                // into the prefix, and Skyrim reuses fixed names (quicksave.ess)
                // - overwriting would destroy the only copy of that session.
                // UNLESS this sync put it there itself: without the provenance
                // check, every quicksave rotation "rescued" our own previous
                // copy, minting an orphan file per session, forever.
                if !manifest.contains(&manifest_key(name, meta_of(&dst))) {
                    preserve_diverged_save(&dst, d, prof_saves);
                }
            }
            Ok(_) => continue, // prefix copy is newer: leave it alone
        }
        std::fs::copy(&src, &dst)?;
        // Stamp the copy with the SOURCE mtime, then record it: that pair is how
        // the next sync recognises its own work.
        if let Ok(f) = std::fs::File::options().write(true).open(&dst) {
            let _ = f.set_modified(src_mtime);
        }
        new_entries.push(manifest_key(name, meta_of(&dst)));
        n += 1;
    }
    if !new_entries.is_empty() {
        manifest.extend(new_entries);
        let body: String = manifest.iter().map(|e| format!("{e}\n")).collect();
        let _ = std::fs::write(&manifest_path, body);
    }
    Ok(n)
}

/// The provenance line for a synced file: name, size and mtime seconds - enough
/// to recognise our own copy later, cheap enough to record for every sync.
pub(crate) fn manifest_key(name: &std::ffi::OsStr, meta: Option<(u64, u64)>) -> String {
    let (len, secs) = meta.unwrap_or((0, 0));
    format!("{}\t{len}\t{secs}", name.to_string_lossy())
}

pub(crate) fn meta_of(p: &std::path::Path) -> Option<(u64, u64)> {
    let m = std::fs::metadata(p).ok()?;
    let secs = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((m.len(), secs))
}

/// Rescue a prefix save the profile has no copy of, before the cloud sync
/// overwrites it (see the caller). "No copy" = no profile file with the same
/// size and mtime; the rescue keeps the `.ess` extension so the game (and the
/// Saves tab) can still load it.
pub(crate) fn preserve_diverged_save(
    dst: &std::path::Path,
    dst_mtime: std::time::SystemTime,
    prof_saves: &std::path::Path,
) {
    let (Ok(meta), Some(name)) = (std::fs::metadata(dst), dst.file_name()) else {
        return;
    };
    // "Known" comes in two shapes: an exact (size, mtime) twin anywhere in the
    // profile, or the profile's SAME-NAME file with the same size - the latter
    // because the original seeding used a copy that did not preserve mtimes, so
    // an adopted save's profile twin carries a fresher timestamp. Without the
    // second test, the first sync after adoption "rescued" duplicates of saves
    // the profile already owned.
    let same_name_same_size = std::fs::metadata(prof_saves.join(name))
        .is_ok_and(|m| m.len() == meta.len())
        // Same length is a hint, not proof: settle it on the bytes. Saves are a
        // few MB and this runs once per divergence, not per session.
        && std::fs::read(dst).ok() == std::fs::read(prof_saves.join(name)).ok();
    let known = same_name_same_size
        || std::fs::read_dir(prof_saves)
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| {
                e.metadata()
                    .is_ok_and(|m| m.len() == meta.len() && m.modified().ok() == Some(dst_mtime))
            });
    if known {
        return;
    }
    let secs = dst_mtime
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let orphan = prof_saves.join(format!("orphan-{secs}-{}", name.to_string_lossy()));
    if !orphan.exists() && std::fs::copy(dst, &orphan).is_ok() {
        eidos_log::info!(
            "eidos: rescued a save the profile had never seen into {}",
            orphan.display()
        );
    }
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
