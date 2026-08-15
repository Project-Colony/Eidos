//! `eidos play`: the whole launch pipeline - swap in the script extender,
//! mount the union over the game inside a private namespace, run, capture.

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::exit;

use eidos_games::DetectedGame;
use eidos_instance::{Instance, InstanceKind};
use eidos_launch::{launch, LaunchSpec};

use crate::*;

pub(crate) fn cmd_play(args: &[String]) {
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
pub(crate) fn warn_if_flatpak_proton(run: &eidos_games::ProtonRun) {
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
pub(crate) fn swap_script_extender(id: &str, command: &mut [String]) {
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
/// Re-root a path that lives inside an enabled mod onto the game's Data directory,
/// so a tool shipped as a mod runs from the MERGED view instead of from its own
/// folder. `None` when the path is not inside any mounted layer.
///
/// This is MO2's `adjustForVirtualized` (processrunner.cpp:13, whose comment reads
/// `mods\FNIS\path\exe => game\data\path\exe`). It matters because tools of this
/// kind read their data relative to their own executable: BodySlide ships an EMPTY
/// `SliderSets`, and every body it can build comes from CBBE and the outfit mods.
/// Launched from its own folder it finds nothing - which is not a crash, just an
/// empty list, so it looks like the tool is broken rather than misplaced.
///
/// Where MO2 matches the mods folder by string prefix and drops one path component,
/// this matches against the layers actually mounted. That is the same answer for a
/// plain mod, and a better one twice over: a mod whose real content sits in a
/// subdirectory is re-rooted from the subdirectory that gets mounted, and a DISABLED
/// mod matches nothing, so it is not silently mapped to a path that will not exist.
pub(crate) fn virtualize_under_data(path: &Path, layers: &[PathBuf], data: &Path) -> Option<PathBuf> {
    // Longest match: layers are whole mod roots, but nothing forbids one being
    // nested inside another, and the innermost is the one that provides the file.
    let layer = layers.iter().filter(|l| path.starts_with(l)).max_by_key(|l| l.as_os_str().len())?;
    let tail = path.strip_prefix(layer).ok()?;
    (!tail.as_os_str().is_empty()).then(|| data.join(tail))
}

pub(crate) fn run_through_view(
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

    // The mod list is JUDGED before anything is prepared, and a bad verdict stops
    // the launch here.
    //
    // `modlist_checked` computes exactly this verdict - "is the list safe to act
    // on" - and every caller on this path used to take `.0` and drop it. That was
    // not merely a silent vanilla launch: with `mods/` unreadable the enabled set
    // is empty, plugin discovery then finds only the game's own masters, and
    // `prepare_plugins` writes that vanilla-only set over the PROFILE's
    // plugins.txt and loadorder.txt (`plugins_state_dir` is the profile
    // directory, not the prefix) before snapshotting the result as the new
    // restore point. One launch with the mod pool unreachable therefore replaced
    // a curated load order with about ten vanilla entries and overwrote the
    // snapshot that could have undone it.
    //
    // `ListTrust::judge` only says Suspect when the directory is unreadable, or
    // when a non-empty list kept nothing, or on a loss past both its floors - so
    // this cannot fire on a deliberate mass-disable.
    let (_, trust) = prof.modlist_checked();
    if let Some(why) = trust.reason() {
        eprintln!(
            "eidos play: refusing to start - the mod list cannot be trusted: {why}\n\
             \x20 Nothing has been written. Fix the mods folder (permissions, a mount that is \
             not mounted) and try again; the profile's load order is untouched."
        );
        exit(1);
    }

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

    // The mount point has to exist before anything can be mounted over it, and for
    // some games it does not ship with the game at all: Stellar Blade deploys into
    // `SB/Content/Paks/~mods`, which is a modding convention the game never
    // creates. On a clean install the mount therefore failed with a bare
    // "No such file or directory (os error 2)" that named neither the path nor the
    // reason - and it would have failed that way for every user of such a game,
    // not just one who moved the directory.
    //
    // Creating it is the same promise Eidos already makes for its own directories,
    // and an empty directory inside the game is inert: nothing loads from it, and
    // it is what the user would have had to make by hand anyway.
    if !game.data_path.is_dir() {
        if let Err(e) = std::fs::create_dir_all(&game.data_path) {
            eprintln!(
                "eidos: cannot create the mod directory {}: {e}",
                game.data_path.display()
            );
            exit(1);
        }
        eprintln!("eidos: created {} (the game does not ship it)", game.data_path.display());
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
        // ONE Overwrite, as in MO2: game-root writes go to its `Root/` subdir.
        root_overwrite: Some(inst.root_overwrite_dir()),
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
            // Both copies: the captured PROFILE copy (so the profile keeps its
            // own values), and the deployed PREFIX copy (so the prefix does not
            // keep mod tweaks baked in - a NEW profile seeds its baseline from
            // the prefix, and used to inherit every mod tweak as if the user had
            // chosen those settings).
            for path in [prof.ini_path(file), prepared.docs.join(file)] {
                // Encoding-aware, or the pass silently no-ops on any INI holding
                // one CP1252 byte - leaving the tweaks baked in, the exact
                // failure this loop exists to prevent.
                let Some((text, cp1252)) = eidos_instance::read_text_lossy(&path) else {
                    continue;
                };
                let restored = eidos_instance::untweak_ini(&text, record);
                if restored != text {
                    if let Err(e) = eidos_instance::write_text(&path, &restored, cp1252) {
                        eprintln!(
                            "eidos: WARNING - could not un-apply INI tweaks in {}: {e}",
                            path.display()
                        );
                    }
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
        let post_loss = eidos_plugins::GameSpec::for_id(id)
            .and_then(|spec| prof.plugin_loss_since_snapshot(&spec));
        if let Some(reason) = post_loss {
            eprintln!(
                "eidos: WARNING - plugins.txt now {reason} relative to the pre-session snapshot \
                 kept at {snap}. Restore it from the GUI Diagnostics tab if this was a crash, or \
                 accept the current set there - without the GUI, restore by copying that file \
                 over plugins.txt, or accept by deleting it.",
                snap = prof.plugins_snapshot_path().display()
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
            // Name what was being mounted. `No such file or directory (os error 2)`
            // on its own gives the user nothing to act on, and it is the message
            // they get for the most likely failure here - a mount point or a game
            // directory that is not where the descriptor says.
            eprintln!("eidos: launch failed: {e}");
            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!("eidos:   mod directory: {}", game.data_path.display());
                eprintln!("eidos:   game install:  {}", game.install_path.display());
            }
            exit(1)
        }
    }
}

/// Which of `shadows` are shipped as a top-level `.dll` in any of `dirs`.
///
/// One listing per directory, no recursion: a wrapper DLL only works where the
/// loader looks for it, so it is never buried. Unreadable directories are skipped -
/// `dirs` includes `Root/` paths that most mods do not have.
pub(crate) fn shipped_shadow_stems(
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
pub(crate) fn forced_dll_overrides(
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
