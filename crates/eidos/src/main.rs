//! `eidos`: the front end that ties detection, instances, and launching together.
//!
//!   eidos games                       list supported games installed on this system
//!   eidos init <game-id>              create a (global) modding instance
//!   eidos play <game-id>              show how to launch / what is mounted
//!   eidos play <game-id> -- <cmd...>  run <cmd> with the mods mounted over the game
//!
//! Instances (global vs portable, layout, load order) live in `eidos-instance`.
//! `play` mounts the instance's mods over the game's own Data directory (via a
//! bind-stash) inside a private namespace, then runs the command through it.

use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::exit;

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::{Instance, InstanceKind, ModEntry};
use eidos_launch::{launch, LaunchSpec};

fn find_game(id: &str) -> Option<DetectedGame> {
    detect(&home()).into_iter().find(|g| g.def.id == id)
}

/// The plugin-discovery sources in ASCENDING priority (later wins same-name
/// shadowing), as fed to [`eidos_plugins::PluginList::discover`]: the game's own
/// Data (lowest), then each enabled mod from lowest to highest priority, then the
/// Overwrite layer LAST (highest). Overwrite is the always-on writable top layer
/// the launcher mounts, mirroring MO2's always-active top-priority Overwrite
/// pseudo-mod - plugins a tool wrote there (xEdit / Bashed Patch output) must be
/// discovered, not dropped from plugins.txt.
fn plugin_sources(
    game_data: &std::path::Path,
    enabled_lowest_first: &[ModEntry],
    overwrite: &std::path::Path,
) -> Vec<(String, PathBuf)> {
    let mut sources: Vec<(String, PathBuf)> = vec![(String::new(), game_data.to_path_buf())];
    // `modlist()` is already in ascending-priority (MO2 display) order, so feed the
    // mods through as-is: lowest priority first, highest last.
    sources.extend(enabled_lowest_first.iter().map(|m| (m.name.clone(), m.path.clone())));
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
fn prepare_plugins(
    id: &str,
    game: &DetectedGame,
    inst: &Instance,
    prof: &eidos_instance::Profile,
) -> Option<(PathBuf, PathBuf)> {
    let spec = eidos_plugins::GameSpec::for_id(id)?;
    let Some(compatdata) = game.compatdata.as_ref() else {
        eprintln!("eidos play: no Proton prefix found, skipping plugins.txt");
        return None;
    };
    let prefix = compatdata.join("pfx");
    let prefix_dir = eidos_plugins::plugins_txt_dir(&prefix, &spec);

    // First run: adopt the prefix's existing state (plugins.txt, loadorder.txt,
    // and the sidecars the game keeps next to them) into the profile, so the
    // bound dir never shows the game less than the dir it wrote.
    match prof.seed_plugin_state(&prefix_dir) {
        Ok(n) if n > 0 => {
            eprintln!("eidos play: adopted {n} plugin-state file(s) into profile '{}'", prof.name)
        }
        Ok(_) => {}
        Err(e) => {
            // FAIL CLOSED. Proceeding with a half-seeded (or unwritable) profile
            // dir meant the pass below saw an empty state, fell back to discovery
            // defaults, and the prefix shadow write then OVERWROTE the user's only
            // good copies with an alphabetical everything-enabled list - the exact
            // files the seed just failed to adopt. Plugin management sits out this
            // run; the game reads its own prefix files, untouched.
            eprintln!(
                "eidos play: WARNING - could not adopt the plugin state into profile '{}' ({e});                  plugin management is OFF for this run and the prefix files are left alone",
                prof.name
            );
            return None;
        }
    }
    let state_dir = prof.plugins_state_dir();

    // Sources in ascending plugin priority: the game's own Data (lowest), each
    // enabled mod, then the Overwrite layer last (highest) so plugins a tool wrote
    // into Overwrite are discovered and win same-name shadowing.
    let enabled: Vec<ModEntry> = inst.modlist().into_iter().filter(|m| m.enabled && !m.is_separator()).collect();
    let sources = plugin_sources(&game.data_path, &enabled, &inst.overwrite_dir());

    let mut list = eidos_plugins::PluginList::discover(&sources, &spec);

    // Preserve the user's saved order + enabled state, FROM THE PROFILE - the
    // single source of truth. loadorder.txt is the order authority; plugins.txt
    // supplies the flags (it deliberately omits the primaries and Creations, so
    // it cannot order them).
    list.apply_prefix_state(&state_dir, &spec);
    list.refresh(&spec);

    for (p, m) in list.missing_masters() {
        eprintln!("eidos play: WARNING - {p} is missing master {m} (likely a crash)");
    }
    let active = list.plugins.iter().filter(|p| p.enabled).count();
    match list.write_load_order(&state_dir, &spec) {
        Ok(listed) => {
            // `listed` is what plugins.txt actually holds; `active` includes the
            // primaries and Creations the engine loads by itself, which are
            // deliberately NOT in the file. Reporting `active` as "written" once
            // pointed a whole investigation at a file that was never wrong.
            eprintln!(
                "eidos play: wrote plugins.txt ({listed} listed, {active} active incl. implicit)"
            );
        }
        Err(e) => {
            // Same fail-closed rule as the seed: if the PROFILE copy could not be
            // written, the shadow below must not run - it would push a state that
            // exists nowhere else onto the prefix, destroying the real files.
            eprintln!(
                "eidos play: WARNING - could not write the profile plugins.txt ({e});                  plugin management is OFF for this run and the prefix files are left alone"
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
    // overwritten, while the current state still looks damaged relative to it -
    // otherwise one unnoticed launch after a crash canonised the wreck and
    // destroyed the only copy that could restore it.
    if prof.plugin_loss_since_snapshot().is_some() {
        eprintln!(
            "eidos play: keeping the previous pre-session plugins.txt snapshot - the current              active set still looks damaged relative to it (the GUI Diagnostics tab offers the              restore, or accept the current set there)"
        );
    } else if let Err(e) = prof.snapshot_plugin_state() {
        eprintln!("eidos play: WARNING - could not snapshot plugins.txt: {e}");
    }
    Some((state_dir, prefix_dir))
}

/// Before launch: give the active profile its own INIs in the prefix. Seed the
/// profile from the prefix on first run (adopting an existing setup, losing
/// nothing), deploy the profile's INIs into the prefix Documents, then enable BSA
/// invalidation on the deployed copy. Returns the prefix Documents dir + the
/// game's INI set, so the caller can capture in-game changes back afterwards.
fn prepare_inis(
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
            eprintln!("eidos play: seeded {n} INI(s) into profile '{}' from the prefix", prof.name)
        }
        Ok(_) => {}
        Err(e) => eprintln!("eidos play: WARNING - could not seed profile INIs: {e}"),
    }
    // If the deploy fails, the prefix keeps its OLD INIs; capturing those back
    // after the run would clobber the profile's copies with stale content. So a
    // failed deploy disables this run's capture (see the return below).
    let deploy_ok = match prof.deploy_inis(&docs, ini_files) {
        Ok(n) => {
            if n > 0 {
                eprintln!("eidos play: deployed {n} profile INI(s) into the prefix");
            }
            true
        }
        Err(e) => {
            eprintln!(
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
            for f in ini_files {
                match prof.apply_ini_tweaks(&docs.join(f), &fragments) {
                    Ok(rec) if !rec.is_empty() => {
                        eprintln!("eidos play: applied {} INI tweak(s) to {f}", rec.len());
                        tweaked.push((f.to_string(), rec));
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("eidos play: WARNING - could not apply INI tweaks to {f}: {e}"),
                }
            }
        }
    }

    // Loose files must win over the vanilla BSAs, and the Bethesda launcher must not
    // reset the plugin selection (both written into the deployed profile INIs).
    match eidos_gamefeatures::enable_bsa_invalidation(&docs, &inst.overwrite_dir(), id) {
        Ok(()) => eprintln!("eidos play: BSA invalidation on"),
        Err(e) => eprintln!("eidos play: could not enable BSA invalidation: {e}"),
    }
    if id == "morrowind" {
        // Morrowind only loads a BSA listed in its numbered [Archives] section;
        // register every enabled mod's top-level .bsa so BSA-shipping mods work.
        let mod_bsas: Vec<String> = inst
            .modlist()
            .into_iter()
            .filter(|m| m.enabled && !m.is_separator())
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
        let _ = eidos_gamefeatures::register_morrowind_archives(&docs.join("Morrowind.ini"), &mod_bsas);
    } else if let Some(ini) = eidos_gamefeatures::ini_file_for(id) {
        if let Err(e) = eidos_gamefeatures::enable_file_selection(&docs, ini) {
            eprintln!("eidos play: could not enable launcher file selection: {e}");
        }
    }
    // No capture cycle when the deploy failed: the prefix INIs are not this
    // profile's state and must not overwrite it after the run.
    deploy_ok.then_some(PreparedInis { docs, ini_files, tweaked })
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
fn sync_saves_for_cloud(
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
            let lower = e.file_name().to_string_lossy().to_ascii_lowercase();
            if !(lower.ends_with(".ess") || lower.ends_with(".skse")) || !path.is_file() {
                return None;
            }
            // An unreadable mtime sorts oldest rather than being skipped: an
            // extra copy is cheap, a hole in the backup is not.
            let mtime =
                e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            Some((path, mtime))
        })
        .collect();
    files.sort_by(|a, b| b.1.cmp(&a.1));
    files.truncate(MAX_FILES);

    let mut n = 0;
    for (src, src_mtime) in files {
        let Some(name) = src.file_name() else { continue };
        let dst = prefix_saves.join(name);
        match std::fs::metadata(&dst).and_then(|m| m.modified()) {
            Err(_) => {} // missing: plain copy below
            Ok(d) if src_mtime > d => {
                // The prefix copy is older, but it may be a save the profile has
                // NEVER seen: a failed-bind session once wrote saves straight
                // into the prefix, and Skyrim reuses fixed names (quicksave.ess)
                // - overwriting would destroy the only copy of that session.
                // Adopt it into the profile first, then overwrite.
                preserve_diverged_save(&dst, d, prof_saves);
            }
            Ok(_) => continue, // prefix copy is newer: leave it alone
        }
        std::fs::copy(&src, &dst)?;
        n += 1;
    }
    Ok(n)
}

/// Rescue a prefix save the profile has no copy of, before the cloud sync
/// overwrites it (see the caller). "No copy" = no profile file with the same
/// size and mtime; the rescue keeps the `.ess` extension so the game (and the
/// Saves tab) can still load it.
fn preserve_diverged_save(
    dst: &std::path::Path,
    dst_mtime: std::time::SystemTime,
    prof_saves: &std::path::Path,
) {
    let (Ok(meta), Some(name)) = (std::fs::metadata(dst), dst.file_name()) else { return };
    // "Known" comes in two shapes: an exact (size, mtime) twin anywhere in the
    // profile, or the profile's SAME-NAME file with the same size - the latter
    // because the original seeding used a copy that did not preserve mtimes, so
    // an adopted save's profile twin carries a fresher timestamp. Without the
    // second test, the first sync after adoption "rescued" duplicates of saves
    // the profile already owned.
    let same_name_same_size = std::fs::metadata(prof_saves.join(name))
        .is_ok_and(|m| m.len() == meta.len());
    let known = same_name_same_size
        || std::fs::read_dir(prof_saves).into_iter().flatten().flatten().any(|e| {
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
        eprintln!(
            "eidos: rescued a save the profile had never seen into {}",
            orphan.display()
        );
    }
}

/// What `prepare_inis` leaves for the post-run capture.
struct PreparedInis {
    /// The prefix directory the INIs were deployed into.
    docs: std::path::PathBuf,
    ini_files: &'static [&'static str],
    /// Per INI file, the keys an INI tweak overwrote, so the capture can put the
    /// profile's own values back.
    tweaked: Vec<(String, Vec<eidos_instance::TweakedKey>)>,
}

/// Before launch: give the active profile its own saves. Seed the profile from
/// the prefix's existing saves on first run (adopting the playthrough), then
/// return the `(profile_saves, prefix_saves)` bind so the launcher redirects the
/// game's save dir to this profile for the run - the prefix is never modified.
fn prepare_saves(
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
            eprintln!("eidos play: adopted {n} existing save(s) into profile '{}'", prof.name);
        }
    }
    Some((prof.saves_dir(), prefix_saves))
}

fn cmd_games() {
    let games = detect(&home());
    if games.is_empty() {
        println!("No supported games detected. Make sure Steam is installed and the game is downloaded.");
        return;
    }
    println!("Supported games installed:");
    for g in &games {
        println!("  {:<10} {}  (Steam: {})", g.def.id, g.def.name, g.steam_name);
        println!("             data: {}", g.data_path.display());
    }
    println!("\nNext: `eidos init <id>` to create a modding instance.");
}

fn cmd_init(id: &str) {
    let Some(game) = find_game(id) else {
        eprintln!("Game '{id}' is not detected. Run `eidos games` to see what's available.");
        exit(1);
    };
    let inst = Instance::global(id);
    inst.create().expect("create instance");
    let _ = inst.ensure_manifest(id, InstanceKind::Global);
    let _ = std::fs::write(
        inst.mods_dir().join("README.txt"),
        "Drop each mod here as its own folder.\n\
         Load order is alphabetical unless a ../load_order.txt lists folder\n\
         names (top line = highest priority, wins file conflicts).\n",
    );
    println!("Created instance for {} ({id}).", game.def.name);
    println!("  instance : {}", inst.root.display());
    println!("  game data: {}", game.data_path.display());
    println!("  add mods : {}", inst.mods_dir().display());
    println!("\nThen: `eidos play {id} -- %command%` (as a Steam launch option).");
}

fn cmd_play(args: &[String]) {
    let Some(id) = args.first() else {
        eprintln!("usage: eidos play <game-id> [-- <command>...]");
        exit(2);
    };
    let command: Vec<String> = match args.iter().position(|a| a == "--") {
        Some(i) => args[i + 1..].to_vec(),
        None => Vec::new(),
    };

    let Some(game) = find_game(id) else {
        eprintln!("Game '{id}' is not detected. Run `eidos games`.");
        exit(1);
    };

    let inst = Instance::global(id);
    inst.create().ok();
    let _ = inst.ensure_manifest(id, InstanceKind::Global);

    if command.is_empty() {
        let layers = inst.load_order();
        println!("Instance      : {}", inst.root.display());
        println!("Mount target  : {}  (the game's Data dir)", game.data_path.display());
        println!("Mod layers ({}):", layers.len());
        for (i, l) in layers.iter().enumerate() {
            println!("  {}. {}", i + 1, l.file_name().unwrap_or_default().to_string_lossy());
        }
        if layers.is_empty() {
            println!("  (none yet - drop mods into {})", inst.mods_dir().display());
        }
        println!("\nTo launch the game through Eidos, set this Steam launch option:");
        println!("    eidos play {id} -- %command%");
        println!("\nOr run any command through the view now, e.g.:");
        println!("    eidos play {id} -- ls \"{}\"", game.data_path.display());
        return;
    }

    // The play path gets its Proton from Steam's own %command%, so there is no
    // ProtonRun to inspect - check the prefix's provenance directly. A flatpak
    // Steam cannot use this launch option at all: `eidos play` runs on the host
    // and the game would start inside the sandbox, blind to our mount.
    if let Some(cd) = game.compatdata.as_ref() {
        if eidos_games::is_flatpak_steam(cd) {
            eprintln!(
                "eidos: WARNING - this game's prefix belongs to the Flatpak Steam install.\n\
                 eidos: the `eidos play` launch option cannot work from inside the Steam sandbox:\n\
                 eidos: the game would run there and never see the mods mounted in our namespace.\n\
                 eidos: use a native Steam for a modded setup."
            );
        }
    }
    let mut command = command;
    swap_script_extender(id, &mut command);
    run_through_view(id, &game, &inst, command, Vec::new(), None, &[]);
}

/// Warn when the resolved Proton belongs to the Flatpak Steam install.
///
/// Eidos deliberately still runs it from the host: re-launching through
/// `flatpak run` would start the game in Flatpak's sandbox, which cannot see the
/// FUSE union mounted in our private namespace, so the game would silently play
/// VANILLA. A loud warning beats a silent wrong result, and the fix the user
/// actually wants is a native Proton (or native Steam).
fn warn_if_flatpak_proton(run: &eidos_games::ProtonRun) {
    if run.flatpak {
        eprintln!(
            "eidos: WARNING - this Proton belongs to the Flatpak Steam install ({}).\n\
             eidos: it ships its runtime and steamclient libraries inside the sandbox, so running\n\
             eidos: it from the host may fail to resolve them. Eidos will NOT relaunch through\n\
             eidos: `flatpak run`: the game would start in Flatpak's sandbox, which cannot see the\n\
             eidos: mods mounted in our namespace, and would silently play vanilla.\n\
             eidos: The `eidos play` Steam launch option cannot work from inside the Steam sandbox\n\
             eidos: at all. The durable fix is a NATIVE Steam (the flatpak's compat-tool list is\n\
             eidos: read from its own root, so dropping a Proton in ~/.steam/ will not appear there;\n\
             eidos: to add one to the flatpak, put it in\n\
             eidos:   ~/.var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d/",
            run.proton.display()
        );
    }
}

/// Swap the vanilla launcher for the game's script-extender loader inside a Steam
/// `%command%`, when the game has one AND the loader actually exists on disk
/// (a swap to a missing skse64_loader.exe would make Proton exit with a cryptic
/// error). Mirrors the GUI's swap, so `eidos play <id> -- %command%` behaves the
/// same whether or not the GUI is in the middle.
fn swap_script_extender(id: &str, command: &mut [String]) {
    let Some(se) = eidos_games::GameDef::for_id(id).and_then(|g| g.script_extender) else {
        return;
    };
    for a in command.iter_mut() {
        if a.contains(se.launcher) {
            let candidate = a.replace(se.launcher, se.loader);
            if std::path::Path::new(&candidate).is_file() {
                eprintln!("eidos play: running {} (script extender) instead of {}", se.loader, se.launcher);
                *a = candidate;
            } else {
                eprintln!(
                    "eidos play: {} is not installed - launching the vanilla {} \
                     (script-extender mods will not load)",
                    se.loader, se.launcher
                );
            }
        }
    }
}

/// The shared launch pipeline behind `play` and `tool`: write `plugins.txt`,
/// deploy the active profile's INIs (+ BSA invalidation), bind its saves, mount
/// the merged view over the game's Data dir, run `command` through it, then
/// capture game-modified INIs back into the profile. Never returns.
fn run_through_view(
    id: &str,
    game: &DetectedGame,
    inst: &Instance,
    command: Vec<String>,
    env: Vec<(String, String)>,
    cwd: Option<std::path::PathBuf>,
    prereqs: &[String],
) -> ! {
    // The instance lock, held for the WHOLE run: the GUI and a second `eidos`
    // are separate processes, and without this two concurrent runs interleaved
    // their prepare/capture cycles into torn profiles. Dropped (with the
    // process) at exit; flock leaves nothing stale behind on a crash.
    let _lock = {
        // A GUI write holds the lock for milliseconds; a launch colliding with
        // one should wait it out, not die. A HELD lock (another session) still
        // refuses quickly.
        let mut attempt = 0;
        loop {
            match inst.try_lock("a running game session") {
                Ok(l) => break l,
                Err(_) if attempt < 20 => {
                    attempt += 1;
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("eidos: refusing to launch: {e}");
                    exit(1);
                }
            }
        }
    };
    // The profile is resolved ONCE and threaded through every prepare and every
    // post-run step. Re-reading `inst.active()` after the game exits re-reads the
    // manifest - and a profile switched in the GUI mid-game then received the
    // PLAYED profile's captures, corrupting a profile that was never run.
    let prof = inst.active();
    let inis = prepare_inis(id, game, inst, &prof);
    let plugin_bind = prepare_plugins(id, game, inst, &prof);
    let save_bind = prepare_saves(id, game, &prof);

    // Soft advisory: an ENB (game root, outside the Data mount) and Community
    // Shaders (an enabled SKSE-plugin mod) both inject into the D3D11 pipeline.
    // They can run together, but users often don't realise both are active - so we
    // note it, never block.
    {
        let cs_roots: Vec<PathBuf> =
            inst.modlist().into_iter().filter(|m| m.enabled && !m.is_separator()).map(|m| m.path).collect();
        if eidos_gamefeatures::enb_cs_conflict(&game.install_path, &cs_roots) {
            eprintln!(
                "eidos play: note - ENB (game root) and Community Shaders (a mod) are both active. \
                 Both can run together; if visuals look wrong, disable one in its INI."
            );
        }
    }

    // Force-load any mod-provided builtin-shadowing DLLs (ENB/ReShade/.asi loaders) -
    // the Linux equivalent of usvfs forced libraries, otherwise Wine's builtin wins -
    // plus any Tier-1 DLL prerequisites a tool declares (d3dx for BodySlide etc.).
    let mut env = env;
    if let Some(kv) = forced_dll_overrides(game, inst, prereqs) {
        env.push(kv);
    }
    // Wine derives its Unix codepage from the locale; a C/POSIX one collapses to
    // CP1252 and MSVC's std::filesystem then rejects any mod path with an
    // accented, Cyrillic or CJK character. Steam's pressure-vessel can strip the
    // locale on the way in, so this covers both `eidos play` and `eidos tool`. An
    // existing UTF-8 locale is left untouched.
    env.extend(eidos_launch::utf8_locale_env_from_process());
    // Xalia is Proton's accessibility/gamepad overlay for Wine dialogs. A modded
    // Bethesda game has no use for it, and the build shipped with several current
    // Proton forks throws a fatal Mono MissingMethodException the moment it starts
    // - which lands a stack trace in every run log and buries whatever the game
    // actually said. Eidos already disables it for the unattended prereq and
    // registry runs; there is no reason the game launch should differ.
    // Not forced if the user set it deliberately.
    if std::env::var_os("PROTON_USE_XALIA").is_none() {
        env.push(("PROTON_USE_XALIA".to_string(), "0".to_string()));
    }

    let root_layers = inst.root_layers();
    if !root_layers.is_empty() {
        eprintln!("eidos: {} mod(s) provide root-level files", root_layers.len());
    }

    let spec = LaunchSpec {
        layers: inst.load_order(),
        overwrite: inst.overwrite_dir(),
        mountpoint: game.data_path.clone(),
        command,
        env,
        base_bind: Some((game.data_path.clone(), inst.base_dir())),
        // The saves bind and the plugins bind ride the same mechanism: the
        // profile's dir over the prefix's, for the life of the run only.
        binds: save_bind.clone().into_iter().chain(plugin_bind.clone()).collect(),
        cwd,
        // MO2's Root Builder: a mod's `Root/` is projected onto the GAME INSTALL
        // ROOT rather than into Data/, which is how a script extender, ENB,
        // ReShade or Engine Fixes becomes a real, orderable, per-profile mod
        // instead of files copied into the game by hand. Empty for a load order
        // that uses none, in which case no second mount happens.
        root_layers,
        root_base_bind: Some((game.install_path.clone(), inst.base_root_dir())),
    };
    let result = launch(spec);

    // The command has exited: capture any INI changes back into the profile.
    // (`prof`, not a fresh `inst.active()`: the captures belong to the profile
    // that was PLAYED, whatever the GUI switched to since.)
    if let Some(prepared) = inis {
        if let Ok(n) = prof.capture_inis(&prepared.docs, prepared.ini_files) {
            if n > 0 {
                eprintln!("eidos: captured {n} INI(s) back into profile '{}'", prof.name);
            }
        }
        // Undo the INI tweaks in the CAPTURED copy, so the profile keeps the values
        // it had rather than adopting the tweaks as its own. A key the game or the
        // user changed while running is left alone - that is a real preference
        // change, and it beats the tweak the same way the profile's own tweak file
        // beats a mod's.
        for (file, record) in &prepared.tweaked {
            let path = prof.ini_path(file);
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let restored = eidos_instance::untweak_ini(&text, record);
            if restored != text {
                if let Err(e) = std::fs::write(&path, restored) {
                    eprintln!("eidos: WARNING - could not un-apply INI tweaks in {file}: {e}");
                }
            }
        }
    }
    // The plugins dir was BOUND, so the game's own plugins.txt rewrite already
    // landed in the profile - there is nothing to capture and nothing to revert.
    // What remains is the backstop: a session that wrecked the active set (a
    // crash artifact written straight into the profile) is flagged against the
    // pre-session snapshot, loudly, with the restore one GUI click away.
    if plugin_bind.is_some() {
        if let Some(reason) = prof.plugin_loss_since_snapshot() {
            eprintln!(
                "eidos: WARNING - this session's plugins.txt {reason}. The pre-session copy is \
                 kept at {}; restore it from the GUI Diagnostics tab if this was a crash, or \
                 ignore this if you disabled those plugins on purpose.",
                prof.plugins_snapshot_path().display()
            );
        }
    }
    // Steam Cloud reads the REAL prefix Saves dir, which the bind shadowed all
    // session: without this sync the cloud backs up a save set frozen at the
    // pre-Eidos era (observed: two saves from 2024, nothing since). One-way,
    // never deleting - the prefix is a backup target, not an authority.
    if let Some((prof_saves, prefix_saves)) = &save_bind {
        match sync_saves_for_cloud(prof_saves, prefix_saves) {
            Ok(n) if n > 0 => {
                eprintln!("eidos: synced {n} save file(s) into the prefix for Steam Cloud")
            }
            Ok(_) => {}
            Err(e) => eprintln!("eidos: WARNING - could not sync saves for Steam Cloud: {e}"),
        }
    }

    match result {
        // Propagate the child's real status. On Unix `code()` is `None` when the
        // child was killed by a signal, so fall back to the shell convention
        // 128 + signal - otherwise a crashed (signal-killed) game/tool would make
        // eidos exit 0 and hide the failure from Steam or any wrapping script.
        Ok(status) => exit(status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(1))),
        Err(e) => {
            eprintln!("eidos: launch failed: {e}");
            exit(1)
        }
    }
}

/// Which of `shadows` are shipped as a top-level `.dll` in any of `dirs`.
///
/// One listing per directory, no recursion: a wrapper DLL only works where the
/// loader looks for it, so it is never buried. Unreadable directories are skipped -
/// `dirs` includes `Root/` paths that most mods do not have.
fn shipped_shadow_stems(
    dirs: &[PathBuf],
    shadows: &[&str],
) -> std::collections::BTreeSet<String> {
    let mut stems = std::collections::BTreeSet::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            if let Some(stem) = name.strip_suffix(".dll") {
                if shadows.contains(&stem) {
                    stems.insert(stem.to_string());
                }
            }
        }
    }
    stems
}

/// Compose the `WINEDLLOVERRIDES` that forces the right DLLs native-then-builtin
/// (`n,b`) so mod graphics DLLs actually load under Wine. Three cases, mirroring
/// MO2's forced libraries:
///
/// 1. A mod SHIPS a wrapper DLL that shadows a Wine builtin (ENB `d3d11`, ReShade
///    `dxgi`, `.asi` loaders, Engine Fixes' `d3dx9_42` preloader) - force the mod's
///    own native so the builtin doesn't win. Looked for at the mod's top level and
///    in its `Root/`, since a game-root wrapper lives in the latter.
/// 2. A mod (often a nested SKSE plugin) IMPORTS `d3dcompiler_47.dll` - Community
///    Shaders / ENB / ReShade need Microsoft's native HLSL compiler, which no
///    Proton flavour ships (they all link the Wine builtin, which those mods
///    reject). Detect by import table and provision the bundled native MS DLL into
///    the prefix's system32/syswow64 (best-effort - a missing/read-only prefix
///    never blocks the launch).
/// 3. A tool declares Tier-1 DLL prerequisites (`tool_prereqs`, e.g. `d3dx9_43` for
///    BodySlide's 3D preview) - provision the bundled native and force it too.
fn forced_dll_overrides(
    game: &DetectedGame,
    inst: &Instance,
    tool_prereqs: &[String],
) -> Option<(String, String)> {
    // Wrapper DLLs a mod ships at its root (d3dcompiler_47 is handled by import
    // detection below, not by the ship check - mods import it, they don't ship it).
    // `d3dx9_42` is the SKSE64 plugin preloader (SSE Engine Fixes' second half): a
    // proxy DLL the Windows loader picks up from the game root, which then preloads
    // the SKSE plugins that need to run before SKSE itself. Wine implements
    // d3dx9_42, so without the override the builtin wins and the preloader never
    // runs - silently, which is the whole problem with this class of mod.
    const SHIPPED_SHADOWS: &[&str] = &[
        "d3d8", "d3d9", "d3d10", "d3d11", "d3d12", "dxgi", "dinput", "dinput8", "winmm",
        "xinput1_3", "x3daudio1_7", "opengl32", "d3dx9_42",
    ];
    let mut roots: Vec<PathBuf> =
        inst.modlist().into_iter().filter(|m| m.enabled && !m.is_separator()).map(|m| m.path).collect();
    roots.push(inst.overwrite_dir());

    // A wrapper DLL sits at the mod's top level when the mod is Data-relative, and
    // in its `Root/` when the mod targets the game install root - which is where
    // this whole class of DLL belongs, since the Windows loader only looks beside
    // the executable. Scan both, one directory listing each: these are wrappers,
    // never buried. The Root dirs come from `root_layers()`, the same list the
    // launcher mounts over the game root, so the override and the mount cannot
    // disagree about which mods ship root content (it also matches `Root`
    // case-insensitively, which a hand-rolled join would not).
    let mut scan: Vec<PathBuf> = roots.clone();
    scan.extend(inst.root_layers());
    let mut stems = shipped_shadow_stems(&scan, SHIPPED_SHADOWS);

    // The prefix's windows dir, where bundled native DLLs get deployed.
    let win = game.compatdata.as_ref().map(|cd| cd.join("pfx").join("drive_c").join("windows"));

    // Case 2: provision the native d3dcompiler_47 if any mod DLL imports it.
    if eidos_gamefeatures::scan_imports_provisionable(&roots) {
        if let Some(win) = &win {
            match eidos_gamefeatures::ensure_d3dcompiler_47(win) {
                Ok(true) => eprintln!("eidos play: provisioned native d3dcompiler_47 into the prefix"),
                Ok(false) => {}
                Err(e) => eprintln!("eidos play: could not provision d3dcompiler_47: {e}"),
            }
        }
        stems.insert("d3dcompiler_47".to_string());
    }

    // Case 3: a tool's declared Tier-1 DLL prerequisites (the bundled DirectX
    // helpers). Tier-2 verbs (vcrun/dotnet) are skipped here - they install via
    // `eidos prereqs`, not WINEDLLOVERRIDES.
    for verb in tool_prereqs {
        if eidos_gamefeatures::is_tier1_dll(verb) {
            if let Some(win) = &win {
                match eidos_gamefeatures::ensure_native_dll(win, verb) {
                    Ok(true) => eprintln!("eidos tool: provisioned native {verb} into the prefix"),
                    Ok(false) => {}
                    Err(e) => eprintln!("eidos tool: could not provision {verb}: {e}"),
                }
            }
            stems.insert(verb.clone());
        }
    }

    if stems.is_empty() {
        return None;
    }
    let stems: Vec<String> = stems.into_iter().collect();
    let value =
        eidos_launch::wine_dll_overrides(&stems, std::env::var("WINEDLLOVERRIDES").ok().as_deref());
    Some(("WINEDLLOVERRIDES".to_string(), value))
}

/// Per-game default tools auto-detected in the game dir: the script extender, the
/// vanilla launcher, and the game binary - whichever are present.
///
/// `inst` widens the search to enabled mods' `Root/` directories, so a script
/// extender installed AS A MOD is detected too; at launch the root union puts it
/// on the game root for real.
fn default_tools_for(game: &DetectedGame, inst: Option<&Instance>) -> Vec<eidos_instance::Tool> {
    let roots = inst.map(|i| i.root_layers()).unwrap_or_default();
    eidos_instance::default_tools_in(game_executables(game), &game.install_path, &roots)
}

/// The auto-detectable executables for a game, from its `GameDef`.
fn game_executables(game: &DetectedGame) -> eidos_instance::GameExecutables<'_> {
    eidos_instance::GameExecutables {
        game_name: game.def.name,
        launcher: game.def.script_extender.as_ref().map(|se| se.launcher),
        binary: Some(game.def.game_binary),
        script_extender: game.def.script_extender.as_ref().map(|se| se.loader),
    }
}

/// `eidos tool <game-id> [list | add <title> <exe> [args...] | rm <title> |
/// run <title> [--print] [-- extra...]]` - manage and run tools (xEdit, FNIS,
/// BodySlide) through the merged view, inside the game's Proton prefix.
fn cmd_tool(args: &[String]) {
    let Some(id) = args.first() else {
        eprintln!(
            "usage:\n\
             \x20 eidos tool <game-id>                       list tools\n\
             \x20 eidos tool <game-id> add <title> <exe> [args...]\n\
             \x20 eidos tool <game-id> rm <title>\n\
             \x20 eidos tool <game-id> run <title> [--print] [-- <extra args>...]"
        );
        exit(2);
    };
    let Some(game) = find_game(id) else {
        eprintln!("Game '{id}' is not detected. Run `eidos games`.");
        exit(1);
    };
    let inst = Instance::global(id);
    inst.create().ok();
    let _ = inst.ensure_manifest(id, InstanceKind::Global);

    match args.get(1).map(String::as_str) {
        None | Some("list") => {
            let tools = eidos_instance::merge_tools(inst.tools(), default_tools_for(&game, Some(&inst)));
            if tools.is_empty() {
                println!("No tools. Add one: eidos tool {id} add <title> <exe> [args...]");
                return;
            }
            println!("Tools for {} (run: eidos tool {id} run <title>):", game.def.name);
            for t in &tools {
                let exe = if t.exe.is_absolute() { t.exe.clone() } else { game.install_path.join(&t.exe) };
                let missing = if exe.is_file() { "" } else { "  [MISSING]" };
                println!("  {:<18} {}{}", t.title, t.exe.display(), missing);
            }
        }
        Some("add") => {
            let (Some(title), Some(exe)) = (args.get(2), args.get(3)) else {
                eprintln!("usage: eidos tool {id} add <title> <exe> [args...]");
                exit(2);
            };
            // The title becomes a `[Tool/<title>]` section header, so reject what
            // cannot round-trip: empty or control characters (a newline would split
            // the header and corrupt neighbouring tools on the next read).
            let title = title.trim();
            if title.is_empty() || title.chars().any(char::is_control) {
                eprintln!("Invalid tool title: must be non-empty and free of control characters.");
                exit(2);
            }
            let mut user = inst.tools();
            user.retain(|t| !t.title.eq_ignore_ascii_case(title));
            user.push(eidos_instance::Tool {
                title: title.to_string(),
                exe: std::path::PathBuf::from(exe),
                args: args[4..].to_vec(),
                workdir: None,
                // Seed known tools' runtime prereqs (BodySlide -> d3dx, Synthesis ->
                // dotnet...); the user can edit tools.ini to override.
                prereqs: eidos_instance::default_prereqs(title),
            });
            match inst.save_tools(&user) {
                Ok(()) => println!("Added '{title}'. Run it: eidos tool {id} run {title}"),
                Err(e) => {
                    eprintln!("could not save tools.ini: {e}");
                    exit(1);
                }
            }
        }
        Some("rm") => {
            let Some(title) = args.get(2) else {
                eprintln!("usage: eidos tool {id} rm <title>");
                exit(2);
            };
            let mut user = inst.tools();
            let before = user.len();
            user.retain(|t| !t.title.eq_ignore_ascii_case(title));
            if user.len() == before {
                eprintln!("No user tool named '{title}' (defaults cannot be removed).");
                exit(1);
            }
            let _ = inst.save_tools(&user);
            println!("Removed '{title}'.");
        }
        Some("run") => {
            let Some(title) = args.get(2) else {
                eprintln!("usage: eidos tool {id} run <title> [--print] [-- <extra args>...]");
                exit(2);
            };
            // Everything after `--` is opaque tool args, so scan for --print only
            // BEFORE the separator (a tool may itself take a --print flag).
            let sep = args.iter().position(|a| a == "--");
            let print_only = args[..sep.unwrap_or(args.len())].iter().any(|a| a == "--print");
            let extra: Vec<String> = match sep {
                Some(i) => args[i + 1..].to_vec(),
                None => Vec::new(),
            };

            let tools = eidos_instance::merge_tools(inst.tools(), default_tools_for(&game, Some(&inst)));
            let Some(tool) = tools.iter().find(|t| t.title.eq_ignore_ascii_case(title)) else {
                eprintln!("No tool named '{title}'. List them: eidos tool {id}");
                exit(1);
            };
            let exe = if tool.exe.is_absolute() {
                tool.exe.clone()
            } else {
                game.install_path.join(&tool.exe)
            };
            if !exe.is_file() {
                eprintln!("Tool executable not found: {}", exe.display());
                exit(1);
            }
            let Some(compat) = game.compatdata.as_ref() else {
                eprintln!("No Proton prefix for {id} - launch the game once through Steam first.");
                exit(1);
            };
            let Some(run) = eidos_games::proton_command(
                &home(),
                game.def.steam_app_id,
                compat,
                &game.install_path,
            )
            else {
                eprintln!(
                    "Could not resolve the Proton for {id} (config.vdf CompatToolMapping / \
                     compatibilitytools.d). Is the game set up to run under Proton?"
                );
                exit(1);
            };

            warn_if_flatpak_proton(&run);
            // Windows tools find their game by reading HKLM\Software\Bethesda
            // Softworks\<game> "installed path", which the game's own installer
            // writes - and which Steam under Proton never runs. Without it xEdit,
            // Wrye Bash and DynDOLOD open on an empty path. Idempotent, additive,
            // and skipped entirely if the prefix is uninitialised or in use.
            if let Some(reg) = eidos_games::GameDef::for_id(id).map(|g| g.registry_name) {
                let proton = run.proton.clone();
                let env = run.env.clone();
                match eidos_gamefeatures::ensure_registry(
                    compat,
                    &game.install_path,
                    reg,
                    |reg_file| {
                        vec![
                            proton.to_string_lossy().into_owned(),
                            // `runinprefix`, not the game verb: this must not run
                            // Proton's game-drive setup, only a program in the
                            // existing prefix.
                            "runinprefix".to_string(),
                            "regedit".to_string(),
                            reg_file.to_string_lossy().into_owned(),
                        ]
                    },
                    &env,
                ) {
                    Ok(true) => eprintln!("eidos: registered the game path in the Wine prefix"),
                    Ok(false) => {}
                    Err(e) => eprintln!("eidos: could not write the prefix registry ({e}); tools may ask for the game path"),
                }
            }
            let mut command = run.command(&exe, &tool.args);
            command.extend(extra);
            // MO2's default working directory for a tool is its own folder.
            let cwd = tool.workdir.clone().or_else(|| exe.parent().map(|p| p.to_path_buf()));
            let prereqs = tool.prereqs.clone();
            // The bundled Tier-1 DLLs get provisioned at launch; but a Tier-2 verb
            // (vcrun/dotnet) that hasn't been installed will likely crash the tool, so
            // warn with the fix - without blocking (the user may have it via Steam).
            let satisfied = satisfied_prereqs(&inst);
            let missing2: Vec<&String> = prereqs
                .iter()
                .filter(|v| eidos_gamefeatures::is_tier2_verb(v) && !satisfied.contains(*v))
                .collect();
            if !missing2.is_empty() {
                let names = missing2.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
                eprintln!(
                    "eidos tool: '{title}' needs {names} - run `eidos prereqs {id} --install` to set it up (downloads from Microsoft)."
                );
            }

            if print_only {
                println!("would run (through the merged view at {}):", game.data_path.display());
                println!("  argv : {command:?}");
                for (k, v) in &run.env {
                    println!("  env  : {k}={v}");
                }
                if let Some(c) = &cwd {
                    println!("  cwd  : {}", c.display());
                }
                return;
            }
            run_through_view(id, &game, &inst, command, run.env, cwd, &prereqs);
        }
        Some(other) => {
            eprintln!("unknown tool subcommand '{other}' (list | add | rm | run)");
            exit(2);
        }
    }
}

/// The Tier-2 prereq verbs already installed into the prefix (the `prereqs.done`
/// sentinel in the instance dir), so a re-run is a no-op and the tool warning is quiet.
fn satisfied_prereqs(inst: &Instance) -> std::collections::BTreeSet<String> {
    std::fs::read_to_string(inst.root.join("prereqs.done"))
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}

/// `eidos prereqs <game-id> [--install]`: show, or install, the runtime
/// prerequisites the instance's tools declare. Tier-1 (bundled DirectX DLLs) copy
/// with no network; Tier-2 (vcrun/dotnet) DOWNLOAD from Microsoft via winetricks and
/// so run only on the explicit `--install`.
fn cmd_prereqs(args: &[String]) {
    let Some(id) = args.first() else {
        eprintln!("usage: eidos prereqs <game-id> [--install]");
        exit(2);
    };
    let install = args.iter().any(|a| a == "--install");
    let Some(game) = find_game(id) else {
        eprintln!("Game '{id}' is not detected. Run `eidos games`.");
        exit(1);
    };
    let inst = Instance::global(id);
    inst.create().ok();
    let _ = inst.ensure_manifest(id, InstanceKind::Global);

    // Union of every tool's declared prereqs, split by tier.
    let tools = eidos_instance::merge_tools(inst.tools(), default_tools_for(&game, Some(&inst)));
    let mut verbs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for t in &tools {
        verbs.extend(t.prereqs.iter().cloned());
    }
    let tier1: Vec<String> = verbs.iter().filter(|v| eidos_gamefeatures::is_tier1_dll(v)).cloned().collect();
    let tier2: Vec<String> = verbs.iter().filter(|v| eidos_gamefeatures::is_tier2_verb(v)).cloned().collect();
    // A verb that is neither a bundled DLL nor a known winetricks verb (a tools.ini
    // typo, or one Eidos hasn't catalogued): surface it rather than silently drop it.
    let unknown: Vec<String> = verbs
        .iter()
        .filter(|v| !eidos_gamefeatures::is_tier1_dll(v) && !eidos_gamefeatures::is_tier2_verb(v))
        .cloned()
        .collect();
    let satisfied = satisfied_prereqs(&inst);
    let pending2: Vec<String> = tier2.iter().filter(|v| !satisfied.contains(*v)).cloned().collect();

    if !install {
        println!("Tool prerequisites for {id}:");
        for t in &tools {
            if !t.prereqs.is_empty() {
                println!("  {:<16} {}", t.title, t.prereqs.join(", "));
            }
        }
        let t1 = if tier1.is_empty() { "(none)".to_string() } else { tier1.join(", ") };
        println!("\nTier 1 (bundled, applied at launch, no download): {t1}");
        let t2 = if tier2.is_empty() {
            "(none)".to_string()
        } else {
            tier2
                .iter()
                .map(|v| format!("{v} [{}]", if satisfied.contains(v) { "done" } else { "pending" }))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!("Tier 2 (winetricks, DOWNLOADS from Microsoft): {t2}");
        if !unknown.is_empty() {
            println!("Unknown verbs (ignored - typo or uncatalogued): {}", unknown.join(", "));
        }
        if !pending2.is_empty() {
            println!(
                "\nRun `eidos prereqs {id} --install` to download + install: {}",
                pending2.join(", ")
            );
        }
        return;
    }

    if !unknown.is_empty() {
        eprintln!("eidos prereqs: ignoring unknown verb(s): {}", unknown.join(", "));
    }

    // --install: Tier 1 (copy bundled DLLs) then the consented Tier 2 (winetricks).
    let Some(compat) = game.compatdata.as_ref() else {
        eprintln!("No Proton prefix for {id} - launch the game once through Steam first.");
        exit(1);
    };
    let win = compat.join("pfx").join("drive_c").join("windows");
    for v in &tier1 {
        match eidos_gamefeatures::ensure_native_dll(&win, v) {
            Ok(true) => println!("provisioned {v} (bundled)"),
            Ok(false) => {}
            Err(e) => eprintln!("could not provision {v}: {e}"),
        }
    }
    if pending2.is_empty() {
        println!("Tier 2 already satisfied (nothing to download).");
        return;
    }
    let Some(run) =
        eidos_games::proton_command(&home(), game.def.steam_app_id, compat, &game.install_path)
    else {
        eprintln!("Could not resolve Proton for {id}.");
        exit(1);
    };
    warn_if_flatpak_proton(&run);
    if !eidos_gamefeatures::cabextract_available() {
        eprintln!("warning: cabextract not on PATH - some winetricks verbs need it (e.g. `pacman -S cabextract`).");
    }
    let prefix = compat.join("pfx");
    // Refuse rather than corrupt. A game, Steam or a stale wineserver still
    // attached to this prefix holds registry and filesystem locks that a
    // winetricks run waits on forever - and the prefix may belong to a session
    // the user is deliberately keeping open, so naming the processes and stopping
    // is the right move. We do not kill anything on the user's behalf.
    let busy = eidos_gamefeatures::prefix_busy(&prefix, compat);
    if !busy.is_empty() {
        eprintln!("This prefix is still in use by {} process(es):", busy.len());
        for (pid, cmd) in busy.iter().take(10) {
            eprintln!("  pid {pid}: {}", cmd.chars().take(100).collect::<String>());
        }
        eprintln!("Close the game, its tools and Steam, then run this again.");
        exit(1);
    }
    println!(
        "Installing {} via winetricks (downloads from Microsoft).",
        pending2.join(", ")
    );
    // One verb at a time, recording each success, so a later failure does not lose
    // the earlier installs (a single batched winetricks call would).
    let mut done = satisfied;
    let mut failed: Option<(String, String)> = None;
    for v in &pending2 {
        println!("  installing {v}...");
        match eidos_gamefeatures::install_tier2_verb(&run.proton, &prefix, &run.env, v) {
            Ok(()) => {
                done.insert(v.clone());
                let body: String = done.iter().map(|x| format!("{x}\n")).collect();
                let _ = std::fs::write(inst.root.join("prereqs.done"), body);
            }
            Err(e) => {
                failed = Some((v.clone(), e.to_string()));
                break;
            }
        }
    }
    match failed {
        None => println!("Done."),
        Some((v, e)) => {
            eprintln!("winetricks failed on '{v}': {e}");
            eprintln!("(earlier verbs were recorded; re-run `eidos prereqs {id} --install` to resume.)");
            exit(1);
        }
    }
}

/// Quote a CSV string field MO2-style: always wrapped in double quotes, embedded
/// quotes doubled.
fn csv_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// A directory's modified time as `yyyy/MM/dd HH:mm:ss` (UTC; MO2 uses local time,
/// a documented divergence to stay dependency-free). Empty if unreadable.
fn fmt_mtime(path: &std::path::Path) -> String {
    let Ok(secs) = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).map_err(|_| std::io::Error::other("pre-epoch")))
        .map(|d| d.as_secs() as i64)
    else {
        return String::new();
    };
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let (h, mi, s) = ((rem / 3600) as u32, ((rem % 3600) / 60) as u32, (rem % 60) as u32);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}/{m:02}/{d:02} {h:02}:{mi:02}:{s:02}")
}

/// `eidos export <game-id> [-o <file>] [--active]`: export the active profile's mod
/// list to CSV in MO2's `exportModListCSV` format (CRLF, always-quoted strings, the
/// unquoted Nexus id, the same 13 columns). Fields Eidos doesn't track (author,
/// uploader) are emitted empty for column/parser parity.
fn cmd_export(args: &[String]) {
    let Some(id) = args.first() else {
        eprintln!("usage: eidos export <game-id> [-o <file>] [--active]");
        exit(2);
    };
    let active_only = args.iter().any(|a| a == "--active");
    let out_path = args.iter().position(|a| a == "-o").and_then(|i| args.get(i + 1)).cloned();
    let Some(game) = find_game(id) else {
        eprintln!("Game '{id}' is not detected. Run `eidos games`.");
        exit(1);
    };
    let inst = Instance::global(id);
    let _ = inst.ensure_profiles();
    let factory = inst.category_factory();
    let domain = game.def.nexus_game;

    let mut csv = String::from(
        "#Mod_Priority,#Mod_Status,#Mod_Name,#Note,#Primary_Category,#Mod_Author,\
         #Mod_Uploader,#Nexus_ID,#Mod_Nexus_URL,#Mod_Uploader_URL,#Mod_Version,\
         #Install_Date,#Download_File_Name\r\n",
    );
    let mut count = 0usize;
    for (i, m) in inst.modlist().iter().enumerate() {
        if active_only && !m.enabled {
            continue;
        }
        let meta = inst.mod_meta(&m.name);
        let note = meta.notes().unwrap_or_default().replace(',', "");
        let category = meta
            .category()
            .as_deref()
            .and_then(eidos_instance::parse_primary)
            .and_then(|cid| factory.name_for_id(cid))
            .unwrap_or_default()
            .to_string();
        let nexus_id = meta.mod_id().unwrap_or(0);
        let nexus_url = if nexus_id > 0 && !domain.is_empty() {
            format!("https://www.nexusmods.com/{domain}/mods/{nexus_id}")
        } else {
            String::new()
        };
        // (value, quoted) per MO2: every string quoted; Nexus_ID is the only bare int.
        let cells: [(String, bool); 13] = [
            (format!("{i:04}"), true),
            ((if m.enabled { "+" } else { "-" }).to_string(), true),
            (m.name.clone(), true),
            (note, true),
            (category, true),
            (String::new(), true), // author - not tracked
            (String::new(), true), // uploader - not tracked
            (nexus_id.to_string(), false),
            (nexus_url, true),
            (String::new(), true), // uploader url - not tracked
            (meta.version().unwrap_or_default(), true),
            (fmt_mtime(&m.path), true),
            (meta.installation_file().unwrap_or_default(), true),
        ];
        let row: Vec<String> =
            cells.into_iter().map(|(v, quoted)| if quoted { csv_quote(&v) } else { v }).collect();
        csv.push_str(&row.join(","));
        csv.push_str("\r\n");
        count += 1;
    }

    match out_path {
        Some(p) => match std::fs::write(&p, &csv) {
            Ok(()) => println!("Exported {count} rows to {p}"),
            Err(e) => {
                eprintln!("write failed: {e}");
                exit(1);
            }
        },
        None => print!("{csv}"),
    }
}

/// `eidos sort <game-id> [--dry-run] [--update-masterlist]` - run LOOT's real
/// graph sort (via the pure-Rust libloot) over this instance's plugins and write
/// the optimised order to plugins.txt / loadorder.txt. Mirrors `prepare_plugins`'
/// discovery so the sorted set matches exactly what a launch would deploy.
fn cmd_sort(args: &[String]) {
    let Some(id) = args.first() else {
        eprintln!("usage: eidos sort <game-id> [--dry-run] [--update-masterlist]");
        exit(2);
    };
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let update = args.iter().any(|a| a == "--update-masterlist");

    if !eidos_loot::is_supported(id) {
        eprintln!("LOOT sorting is not supported for '{id}' (timestamp-ordered or unmanaged game).");
        exit(1);
    }
    let Some(game) = find_game(id) else {
        eprintln!("Game '{id}' is not detected. Run `eidos games`.");
        exit(1);
    };
    let Some(spec) = eidos_plugins::GameSpec::for_id(id) else {
        eprintln!("No plugin support for '{id}'.");
        exit(1);
    };
    let Some(compatdata) = game.compatdata.as_ref() else {
        eprintln!("No Proton prefix found for '{id}'. Launch it once through Steam first.");
        exit(1);
    };
    let prefix = compatdata.join("pfx");
    let local_dir = eidos_plugins::plugins_txt_dir(&prefix, &spec);

    let inst = Instance::global(id);
    let _ = inst.ensure_profiles();
    // The PROFILE owns the plugin state; the prefix copy is a shadow for
    // external tools. This command once wrote only the prefix, and the next
    // launch's profile-driven pass reverted the entire sort - actives included.
    let prof = inst.active();
    let _lock = match inst.try_lock("eidos sort") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Cannot sort now: {e}.");
            exit(1);
        }
    };
    if let Err(e) = prof.seed_plugin_state(&local_dir) {
        eprintln!("Cannot sort: adopting the plugin state into the profile failed ({e}).");
        exit(1);
    }
    let state_dir = prof.plugins_state_dir();

    // Discover exactly what a launch would use, preserving the current order.
    let enabled: Vec<ModEntry> =
        inst.modlist().into_iter().filter(|m| m.enabled && !m.is_separator()).collect();
    let sources = plugin_sources(&game.data_path, &enabled, &inst.overwrite_dir());
    let mut list = eidos_plugins::PluginList::discover(&sources, &spec);
    list.apply_prefix_state(&state_dir, &spec);
    list.refresh(&spec);

    if list.plugins.is_empty() {
        eprintln!("No plugins discovered for '{id}'; nothing to sort.");
        exit(1);
    }

    // Fetch/cache the per-game masterlist + shared prelude.
    let (_game_type, repo) = eidos_loot::loot_support(id).unwrap();
    let cache = inst.root.join("loot");
    let (masterlist, prelude) = match eidos_loot::ensure_masterlist(repo, &cache, update) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Could not obtain masterlist: {e}");
            exit(1);
        }
    };
    let userlist = cache.join("userlist.yaml");

    // Hand LOOT every discovered plugin by (name, real resolved path).
    let plugins: Vec<(String, PathBuf)> =
        list.plugins.iter().map(|p| (p.name.clone(), p.path.clone())).collect();

    let sorted = match eidos_loot::sort(
        id,
        &game.install_path,
        // The PROFILE dir: it is the load-order authority now; the prefix copy is
        // a shadow that can be stale.
        &state_dir,
        &plugins,
        &masterlist,
        &prelude,
        Some(&userlist),
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("LOOT sort failed: {e}");
            exit(1);
        }
    };

    if dry_run {
        println!("LOOT-sorted order ({} plugins):", sorted.len());
        for (i, n) in sorted.iter().enumerate() {
            println!("  {i:>3}  {n}");
        }
        println!("\n(dry run - nothing written; drop --dry-run to apply)");
        return;
    }

    list.apply_sorted_order(&sorted);
    list.refresh(&spec);
    let active = list.plugins.iter().filter(|p| p.enabled).count();
    match list.write_load_order(&state_dir, &spec) {
        Ok(_) => {
            // Shadow for external tools reading the prefix; never fatal.
            let _ = list.write_load_order(&local_dir, &spec);
            println!("Sorted {} plugins ({active} active) and wrote the load order.", sorted.len())
        }
        Err(e) => {
            eprintln!("Could not write load order: {e}");
            exit(1);
        }
    }
}

fn cmd_install(args: &[String]) {
    let (Some(id), Some(archive)) = (args.first(), args.get(1)) else {
        eprintln!("usage: eidos install <game-id> <archive> [name]");
        exit(2);
    };
    let Some(game) = find_game(id) else {
        eprintln!("Game '{id}' is not detected. Run `eidos games`.");
        exit(1);
    };
    let inst = Instance::global(id);
    inst.create().ok();
    let _ = inst.ensure_manifest(id, InstanceKind::Global);

    // Optional overwrite policy; the positional name is the first non-flag arg.
    let policy = if args.iter().any(|a| a == "--replace") {
        eidos_install::OverwritePolicy::Replace
    } else if args.iter().any(|a| a == "--merge") {
        eidos_install::OverwritePolicy::Merge
    } else {
        eidos_install::OverwritePolicy::Fail
    };
    let name = args
        .iter()
        .skip(2)
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| eidos_install::mod_name_for(std::path::Path::new(archive)));
    // FOMOD condition context: plugins in enabled mods read Active, plugins in
    // disabled mods read Inactive, so a scripted installer's fileDependency /
    // gameDependency options evaluate correctly (MO2 distinguishes the two).
    let ml = inst.modlist();
    let enabled_roots: Vec<std::path::PathBuf> =
        ml.iter().filter(|m| m.enabled && !m.is_separator()).map(|m| m.path.clone()).collect();
    let disabled_roots: Vec<std::path::PathBuf> =
        ml.iter().filter(|m| !m.enabled && !m.is_separator()).map(|m| m.path.clone()).collect();
    let ctx = eidos_install::fomod_context(&game.data_path, &enabled_roots, &disabled_roots);
    match eidos_install::install_archive_with_policy(
        std::path::Path::new(archive),
        &inst.mods_dir(),
        &name,
        id,
        policy,
        &ctx,
    ) {
        Ok(r) => {
            // Give the new mod the highest priority so it wins conflicts by default,
            // like MO2. modlist() is lowest-priority-first, so highest = the END.
            let mut ml = inst.modlist();
            ml.retain(|m| m.name != r.name);
            ml.push(ModEntry { name: r.name.clone(), enabled: true, path: r.dest.clone(), unmanaged: false });
            let _ = inst.save_modlist(&ml);

            // If this archive came from a Nexus download, flag its .meta installed
            // (MO2's markInstalled); a no-op when there is no sidecar.
            let _ = eidos_nexus::mark_installed(std::path::Path::new(archive));

            print!("Installed '{}' for {}", r.name, game.def.name);
            if r.fomod {
                print!(" (via FOMOD, default options)");
            } else if !r.stripped.is_empty() {
                print!(" (stripped wrapper '{}')", r.stripped.trim_end_matches('/'));
            }
            println!();
            println!("  -> {}", r.dest.display());
            if !r.missing.is_empty() {
                eprintln!("  note: {} file(s) the installer expected were not in the archive:", r.missing.len());
                for m in &r.missing {
                    eprintln!("    - {m}");
                }
            }
            println!("  enabled at the top of the load order. `eidos play {id}` to use it.");
        }
        Err(e) => {
            eprintln!("install failed: {e}");
            if matches!(e, eidos_install::InstallError::Exists(_)) {
                eprintln!("  (re-run with --replace to reinstall it, or --merge to install over it)");
            }
            exit(1);
        }
    }
}

/// `~/.config/eidos/nexus.ini`, holding the personal Nexus API key. Delegates to
/// the shared `eidos-instance` settings store so the CLI and the GUI can never
/// disagree on the path or the file format.
fn nexus_key_path() -> std::path::PathBuf {
    eidos_instance::settings::nexus_key_path()
}

/// The stored Nexus API key, if any (the shared store the GUI also reads).
fn load_nexus_key() -> Option<String> {
    eidos_instance::settings::load_nexus_key()
}

/// A connected Nexus client, or exit with a pointer to `eidos nexus key`.
fn nexus_client() -> eidos_nexus::Nexus {
    let Some(key) = load_nexus_key() else {
        eprintln!(
            "No Nexus API key configured. Get yours at nexusmods.com -> Site settings -> \
             API keys (Personal API Key), then run:  eidos nexus key <KEY>"
        );
        exit(1);
    };
    eidos_nexus::Nexus::new(&key)
}

/// `eidos nexus key|status|update` - account + update checks.
fn cmd_nexus(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("key") => {
            let Some(key) = args.get(1) else {
                eprintln!("usage: eidos nexus key <YOUR-PERSONAL-API-KEY>");
                exit(2);
            };
            match eidos_nexus::Nexus::new(key).validate() {
                Ok(acct) => {
                    let path = nexus_key_path();
                    if let Err(e) = eidos_instance::settings::save_nexus_key(key) {
                        eprintln!("could not store the key at {}: {e}", path.display());
                        exit(1);
                    }
                    println!(
                        "Connected as {} ({}). Key stored in {}.",
                        acct.name,
                        if acct.is_premium { "premium" } else { "free" },
                        path.display()
                    );
                    println!("Next: register the browser handler:  eidos nxm --register");
                }
                Err(e) => {
                    eprintln!("key validation failed: {e}");
                    exit(1);
                }
            }
        }
        Some("status") => match nexus_client().validate() {
            Ok(acct) => println!(
                "Connected as {} ({}).",
                acct.name,
                if acct.is_premium { "premium" } else { "free" }
            ),
            Err(e) => {
                eprintln!("not connected: {e}");
                exit(1);
            }
        },
        Some("update") => {
            let Some(id) = args.get(1) else {
                eprintln!("usage: eidos nexus update <game-id>");
                exit(2);
            };
            let Some(game) = find_game(id) else {
                eprintln!("Game '{id}' is not detected. Run `eidos games`.");
                exit(1);
            };
            let inst = Instance::global(id);
            let nexus = nexus_client();

            // MO2's approach: one "updated this month" query, then only fetch
            // the mods in the intersection (stays inside the API rate limits).
            let updated = match nexus.updated_mod_ids(game.def.nexus_game, "1m") {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("update query failed: {e}");
                    exit(1);
                }
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            const MONTH: u64 = 30 * 24 * 3600;

            let mut checked = 0u32;
            let mut updates = 0u32;
            let mut individual = 0u32;
            let mut rate_limited = false;
            for m in inst.modlist() {
                let mut meta = inst.mod_meta(&m.name);
                let Some(mod_id) = meta.mod_id() else { continue };
                checked += 1;
                // MO2: the `updated?period=1m` list is only trustworthy for mods
                // checked within that window. A mod never checked - or checked over
                // a month ago - gets an individual query regardless of the
                // intersection, else an update published >1 month ago is missed
                // forever (the common first-run case on an established mod list).
                let stale = meta
                    .last_nexus_update()
                    .map(|t| now.saturating_sub(t) > MONTH)
                    .unwrap_or(true);
                if !stale && !updated.contains(&mod_id) {
                    continue;
                }
                if stale {
                    individual += 1;
                }
                match nexus.mod_info(game.def.nexus_game, mod_id) {
                    Ok(remote) => {
                        meta.set_newest_version(&remote.version);
                        meta.set_last_nexus_update(now);
                        let _ = meta.write(&inst.mods_dir().join(&m.name).join("meta.ini"));
                        if meta.update_available() {
                            updates += 1;
                            println!(
                                "  UPDATE {:<40} {} -> {}",
                                m.name,
                                meta.version().unwrap_or_default(),
                                remote.version
                            );
                        }
                    }
                    Err(e) => {
                        // MO2 stops dispatching the moment the account is exhausted.
                        if e.contains("429") {
                            rate_limited = true;
                            eprintln!("  rate limited by Nexus - stopping; remaining mods unchecked.");
                            break;
                        }
                        eprintln!("  {}: {e}", m.name);
                    }
                }
            }
            println!(
                "{updates} update(s) available ({checked} mod(s) with a Nexus id; \
                 {individual} queried individually; {} recently updated on Nexus).",
                updated.len()
            );
            let rl = nexus.rate_limits();
            if let Some(h) = rl.hourly_remaining {
                let daily = rl.daily_remaining.map(|d| format!(", {d} today")).unwrap_or_default();
                println!("Nexus budget: {h} request(s) left this hour{daily}.");
            }
            if rate_limited {
                eprintln!("Some mods were not checked (hourly limit reached). Re-run after the hour.");
            }
        }
        _ => {
            eprintln!(
                "usage:\n\
                 \x20 eidos nexus key <KEY>       connect (personal API key)\n\
                 \x20 eidos nexus status          check the stored key\n\
                 \x20 eidos nexus update <game>   check installed mods for updates"
            );
            exit(2);
        }
    }
}

/// `eidos nxm <url>` - download a "Mod Manager Download" link into the game's
/// downloads dir (with its MO2-format .meta). `--register` installs the
/// x-scheme-handler so the site's button opens Eidos.
fn cmd_nxm(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("--register") => {
            let exe = std::env::current_exe().unwrap_or_else(|_| "eidos".into());
            let apps = home().join(".local/share/applications");
            let _ = std::fs::create_dir_all(&apps);
            let desktop = apps.join("eidos-nxm.desktop");
            let body = format!(
                "[Desktop Entry]\nType=Application\nName=Eidos (Nexus nxm handler)\n\
                 Exec={} nxm %u\nMimeType=x-scheme-handler/nxm;\nNoDisplay=true\nTerminal=false\n",
                exe.display()
            );
            if let Err(e) = std::fs::write(&desktop, body) {
                eprintln!("could not write {}: {e}", desktop.display());
                exit(1);
            }
            let _ = std::process::Command::new("xdg-mime")
                .args(["default", "eidos-nxm.desktop", "x-scheme-handler/nxm"])
                .status();
            println!(
                "Registered {} for nxm:// links.\nThe site's \"Mod Manager Download\" button now downloads through Eidos.",
                desktop.display()
            );
        }
        Some(url) => {
            let nxm = match eidos_nexus::NxmUrl::parse(url) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("bad nxm link: {e}");
                    exit(1);
                }
            };
            // Which detected game does this link belong to? (VR editions share
            // their parent's Nexus; prefer the one with an existing instance.)
            let games = detect(&home());
            let mut candidates: Vec<&DetectedGame> = games
                .iter()
                .filter(|g| g.def.nexus_game.eq_ignore_ascii_case(&nxm.game))
                .collect();
            candidates.sort_by_key(|g| !Instance::global(g.def.id).exists());
            let Some(game) = candidates.first() else {
                eprintln!("No detected game matches the Nexus domain '{}'.", nxm.game);
                exit(1);
            };
            let inst = Instance::global(game.def.id);
            inst.create().ok();

            let nexus = nexus_client();
            let file = match nexus.file_info(&nxm.game, nxm.mod_id, nxm.file_id) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("file lookup failed: {e}");
                    exit(1);
                }
            };
            let remote_mod = match nexus.mod_info(&nxm.game, nxm.mod_id) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("mod lookup failed: {e}");
                    exit(1);
                }
            };
            let link = match nexus.download_link(&nxm) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("could not resolve the download: {e}");
                    exit(1);
                }
            };
            let name = eidos_nexus::file_name_from_uri(&link)
                .or_else(|| eidos_nexus::sanitize_file_name(&file.file_name))
                .unwrap_or_else(|| format!("{}-{}.archive", nxm.mod_id, nxm.file_id));
            let downloads = inst.downloads_dir();
            // Don't silently clobber an existing download. If the very same file is
            // already here (its .meta carries this fileID), stop; otherwise give the
            // new download a unique `<i>_<name>` (MO2's getDownloadFileName).
            let existing = downloads.join(&name);
            if existing.is_file() {
                let meta = std::fs::read_to_string(format!("{}.meta", existing.display())).unwrap_or_default();
                if meta.contains(&format!("fileID={}", nxm.file_id)) {
                    println!("Already downloaded: {}", existing.display());
                    println!("Install it:  eidos install {} \"{}\"", game.def.id, existing.display());
                    return;
                }
            }
            let name = eidos_nexus::unique_download_name(&downloads, &name);
            let dest = downloads.join(&name);

            println!("Downloading {} ({}) ...", file.name, name);
            match nexus.download(&link, &dest) {
                Ok(bytes) => {
                    let _ = eidos_nexus::write_download_meta(
                        &dest,
                        game.def.short_name,
                        &nxm,
                        &link,
                        &file,
                        &remote_mod,
                    );
                    println!("Downloaded {} ({:.1} MiB)", dest.display(), bytes as f64 / (1024.0 * 1024.0));
                    println!("Install it:  eidos install {} \"{}\"", game.def.id, dest.display());
                }
                Err(e) => {
                    eprintln!("download failed: {e}");
                    exit(1);
                }
            }
        }
        None => {
            eprintln!(
                "usage:\n\
                 \x20 eidos nxm <nxm://...>   download a Mod Manager Download link\n\
                 \x20 eidos nxm --register    make the browser send nxm:// links to Eidos"
            );
            exit(2);
        }
    }
}

fn usage() -> ! {
    eprintln!(
        "eidos - a native Linux mod manager\n\
         \n\
         usage:\n\
         \x20 eidos games                       list supported games installed here\n\
         \x20 eidos init <game-id>              create a modding instance\n\
         \x20 eidos play <game-id>              show what would be mounted\n\
         \x20 eidos play <game-id> -- <cmd...>  run <cmd> with mods mounted over the game\n\
         \x20 eidos install <id> <archive>      install a downloaded mod archive (.7z/.zip/.rar)\n\
         \x20 eidos tool <id> [...]             manage + run tools (xEdit/FNIS/...) through the view\n\
         \x20 eidos nexus key|status|update     connect a Nexus account / check for mod updates\n\
         \x20 eidos nxm <url> | --register      download a Nexus Mod Manager link / register the handler\n\
         \x20 eidos export <id> [-o file]       export the mod list to CSV (MO2 format; --active = enabled only)\n\
         \x20 eidos sort <id> [--dry-run]       LOOT-sort the plugin load order (--update-masterlist to refresh)\n\
         \x20 eidos import <id> <mo2-profile>   take over an MO2 profile's mod order + plugin state"
    );
    exit(2);
}

/// `eidos import <game-id> <mo2-profile-dir>`: adopt an existing Mod Organizer 2
/// profile's mod order, enabled states and load order.
fn cmd_import(args: &[String]) -> ! {
    let (Some(id), Some(dir)) = (args.first(), args.get(1)) else { usage() };
    let inst = Instance::global(id);
    if !inst.exists() {
        eprintln!("eidos import: no instance for '{id}' - run `eidos init {id}` first.");
        exit(1);
    }
    match inst.import_mo2_profile(std::path::Path::new(dir)) {
        Ok(r) => {
            println!(
                "Imported {} mod(s) from {dir} into profile '{}'.",
                r.matched,
                inst.active_profile()
            );
            if r.kept_local > 0 {
                println!("{} local mod(s) MO2 did not list were kept at the bottom.", r.kept_local);
            }
            if r.plugin_files > 0 {
                println!("Load order imported ({} file(s)).", r.plugin_files);
            }
            if !r.missing.is_empty() {
                println!("\n{} mod(s) MO2 listed are not installed here:", r.missing.len());
                for m in r.missing.iter().take(40) {
                    println!("  - {m}");
                }
                if r.missing.len() > 40 {
                    println!("  ... and {} more", r.missing.len() - 40);
                }
                println!("Install them, then run this again to place them in order.");
            }
            exit(0)
        }
        Err(e) => {
            eprintln!("eidos import: {e}");
            exit(1)
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("games") => cmd_games(),
        Some("init") => match args.get(1) {
            Some(id) => cmd_init(id),
            None => usage(),
        },
        Some("play") => cmd_play(&args[1..]),
        Some("install") => cmd_install(&args[1..]),
        Some("tool") => cmd_tool(&args[1..]),
        Some("prereqs") => cmd_prereqs(&args[1..]),
        Some("export") => cmd_export(&args[1..]),
        Some("sort") => cmd_sort(&args[1..]),
        Some("nexus") => cmd_nexus(&args[1..]),
        Some("nxm") => cmd_nxm(&args[1..]),
        Some("import") => cmd_import(&args[1..]),
        _ => usage(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A throwaway temp dir, cleaned up on drop (the same idiom the other crates
    /// use - no external dev-dependency).
    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("eidos-{}-{}", tag, std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Tmp(dir)
        }
        fn touch(&self, rel: &str) {
            let p = self.0.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, b"").unwrap();
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A wrapper DLL that belongs at the GAME ROOT lives in the mod's `Root/`, not
    /// at its top level, so scanning only the top level misses exactly the mods this
    /// override exists for. The concrete case: SSE Engine Fixes' preloader ships as
    /// `Root/d3dx9_42.dll`, Wine implements d3dx9_42, and without the override the
    /// builtin wins and the preloader never runs - with no error anywhere.
    #[test]
    fn a_wrapper_dll_is_found_in_a_mods_root_folder() {
        let t = Tmp::new("shadow");
        t.touch("mods/EngineFixesPreloader/Root/d3dx9_42.dll");
        t.touch("mods/ENB/d3d11.dll");
        // Not a wrapper, and buried: must not be picked up.
        t.touch("mods/SomeMod/SKSE/Plugins/whatever.dll");

        let shadows = ["d3d11", "d3dx9_42"];
        let dirs = vec![
            t.0.join("mods/EngineFixesPreloader"),
            t.0.join("mods/EngineFixesPreloader/Root"),
            t.0.join("mods/ENB"),
            t.0.join("mods/SomeMod"),
        ];
        let stems = shipped_shadow_stems(&dirs, &shadows);

        assert!(stems.contains("d3dx9_42"), "the Root/ preloader must be found");
        assert!(stems.contains("d3d11"), "a top-level wrapper is still found");
        assert_eq!(stems.len(), 2, "nothing else, and nothing from a nested dir");
    }

    /// The Steam Cloud sync must be idempotent (fs::copy stamps the destination
    /// with a NEWER mtime, so the second run finds nothing to do), must rescue a
    /// prefix save the profile never saw before overwriting its fixed name, and
    /// must ignore junk.
    #[test]
    fn cloud_sync_is_idempotent_and_rescues_diverged_saves() {
        let t = Tmp::new("cloudsync");
        let prof = t.0.join("prof");
        let prefix = t.0.join("prefix");
        fs::create_dir_all(&prof).unwrap();
        fs::create_dir_all(&prefix).unwrap();

        // A diverged prefix quicksave the profile has no copy of (a failed-bind
        // session wrote it), OLDER than the profile's own quicksave.
        fs::write(prefix.join("quicksave.ess"), b"orphan session").unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        let f = fs::File::options().write(true).open(prefix.join("quicksave.ess")).unwrap();
        f.set_modified(old).unwrap();
        drop(f);

        fs::write(prof.join("quicksave.ess"), b"current playthrough").unwrap();
        fs::write(prof.join("quicksave.skse"), b"cosave").unwrap();
        fs::write(prof.join("steam_autocloud.vdf"), b"junk").unwrap();

        let n = sync_saves_for_cloud(&prof, &prefix).unwrap();
        assert_eq!(n, 2, ".ess + .skse synced, junk ignored");
        assert_eq!(fs::read(prefix.join("quicksave.ess")).unwrap(), b"current playthrough");
        assert!(!prefix.join("steam_autocloud.vdf").exists());

        // The orphan was rescued into the profile before the overwrite.
        let rescued: Vec<String> = fs::read_dir(&prof)
            .unwrap()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.starts_with("orphan-") && n.ends_with("quicksave.ess"))
            .collect();
        assert_eq!(rescued.len(), 1, "the only copy of the orphan session must survive");

        // Second run: nothing to do. (The rescued orphan syncs up once, at most.)
        let again = sync_saves_for_cloud(&prof, &prefix).unwrap();
        assert!(again <= 1, "the sync must converge, not recopy everything ({again})");
        assert_eq!(sync_saves_for_cloud(&prof, &prefix).unwrap(), 0, "and then be a no-op");

        // A save the profile ADOPTED at seeding shares name + size with the
        // prefix original but not its mtime (the seed copy did not preserve
        // mtimes). It must NOT be "rescued" into a duplicate.
        fs::write(prefix.join("Save 12 - Old.ess"), b"identical bytes").unwrap();
        let f = fs::File::options().write(true).open(prefix.join("Save 12 - Old.ess")).unwrap();
        f.set_modified(old).unwrap();
        drop(f);
        fs::write(prof.join("Save 12 - Old.ess"), b"identical bytes").unwrap();
        sync_saves_for_cloud(&prof, &prefix).unwrap();
        let orphans = fs::read_dir(&prof)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains("Save 12"))
            .count();
        assert_eq!(orphans, 1, "the adopted twin must not spawn an orphan duplicate");
    }

    // Guards FIX C1: the Overwrite layer must be the LAST (highest-priority) plugin
    // source, so an ESP that lives only in Overwrite (xEdit / Bashed Patch output)
    // is discovered, and an Overwrite copy wins same-name shadowing over a mod's
    // copy - otherwise such plugins are silently dropped from plugins.txt.
    #[test]
    fn overwrite_is_the_highest_priority_plugin_source() {
        let t = Tmp::new("c1");
        let game_data = t.0.join("Data");
        fs::create_dir_all(&game_data).unwrap();

        // One enabled mod ships Patch.esp; Overwrite also has Patch.esp (a later
        // regeneration) plus a Bashed-Patch.esp that exists ONLY in Overwrite.
        let mod_dir = t.0.join("mods/AwesomeMod");
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(mod_dir.join("Patch.esp"), b"").unwrap();
        let overwrite = t.0.join("overwrite");
        t.touch("overwrite/Patch.esp");
        t.touch("overwrite/Bashed Patch.esp");

        let enabled = vec![ModEntry {
            name: "AwesomeMod".to_string(),
            enabled: true,
            path: mod_dir.clone(), unmanaged: false }];

        let sources = plugin_sources(&game_data, &enabled, &overwrite);
        // Overwrite must be the final, highest-priority source.
        assert_eq!(sources.last().unwrap().0, "overwrite");
        assert_eq!(sources.last().unwrap().1, overwrite);

        let spec = eidos_plugins::GameSpec::for_id("skyrimse").unwrap();
        let list = eidos_plugins::PluginList::discover(&sources, &spec);

        // The Overwrite-only plugin is discovered (would be dropped without C1).
        let bashed = list
            .plugins
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case("Bashed Patch.esp"))
            .expect("Overwrite-only plugin must be discovered");
        assert_eq!(bashed.origin_mod, "overwrite");

        // For the shadowed name, the Overwrite copy wins (highest priority).
        let patch = list
            .plugins
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case("Patch.esp"))
            .expect("Patch.esp must be present");
        assert_eq!(patch.origin_mod, "overwrite");
        assert!(
            patch.path.starts_with(&overwrite),
            "shadowed plugin should resolve to the Overwrite copy, got {}",
            patch.path.display()
        );
    }

    // Guards FIX C2: on Unix a signal-killed child reports `code() == None`, so the
    // exit status must fall back to 128 + signal (not 0) - otherwise a crashed game
    // would make eidos exit 0 and hide the crash. Asserts the mapping the
    // `run_through_view` exit path uses.
    #[test]
    fn signal_death_maps_to_128_plus_signal_not_zero() {
        use std::process::ExitStatus;

        // A child killed by SIGSEGV (11): code() is None, signal() is 11.
        let killed = ExitStatus::from_raw(11);
        assert_eq!(killed.code(), None, "signal death has no exit code on Unix");
        let mapped = killed.code().unwrap_or_else(|| 128 + killed.signal().unwrap_or(1));
        assert_eq!(mapped, 139, "SIGSEGV must map to 139, never 0");
        assert_ne!(mapped, 0);

        // A normal exit(3) is unaffected: code() is Some(3).
        let normal = ExitStatus::from_raw(3 << 8);
        assert_eq!(
            normal.code().unwrap_or_else(|| 128 + normal.signal().unwrap_or(1)),
            3
        );
    }
}
