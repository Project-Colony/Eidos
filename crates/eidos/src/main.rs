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
/// Data (lowest), then each enabled mod (the modlist is highest-priority first,
/// so reversed), then the Overwrite layer LAST (highest). Overwrite is the
/// always-on writable top layer the launcher mounts, mirroring MO2's
/// always-active top-priority Overwrite pseudo-mod - plugins a tool wrote there
/// (xEdit / Bashed Patch output) must be discovered, not dropped from plugins.txt.
fn plugin_sources(
    game_data: &std::path::Path,
    enabled_highest_first: &[ModEntry],
    overwrite: &std::path::Path,
) -> Vec<(String, PathBuf)> {
    let mut sources: Vec<(String, PathBuf)> = vec![(String::new(), game_data.to_path_buf())];
    sources.extend(
        enabled_highest_first
            .iter()
            .rev()
            .map(|m| (m.name.clone(), m.path.clone())),
    );
    sources.push(("overwrite".to_string(), overwrite.to_path_buf()));
    sources
}

/// Before launch: discover this instance's plugins, preserve any existing load
/// order from the prefix, re-validate the invariants, and write
/// `plugins.txt`/`loadorder.txt` where the game reads them. Best-effort - a game
/// with no plugin system or no Proton prefix is simply skipped.
fn prepare_plugins(id: &str, game: &DetectedGame, inst: &Instance) {
    let Some(spec) = eidos_plugins::GameSpec::for_id(id) else { return };
    let Some(compatdata) = game.compatdata.as_ref() else {
        eprintln!("eidos play: no Proton prefix found, skipping plugins.txt");
        return;
    };
    let prefix = compatdata.join("pfx");

    // Sources in ascending plugin priority: the game's own Data (lowest), each
    // enabled mod, then the Overwrite layer last (highest) so plugins a tool wrote
    // into Overwrite are discovered and win same-name shadowing.
    let enabled: Vec<ModEntry> = inst.modlist().into_iter().filter(|m| m.enabled && !m.is_separator()).collect();
    let sources = plugin_sources(&game.data_path, &enabled, &inst.overwrite_dir());

    let mut list = eidos_plugins::PluginList::discover(&sources, &spec);

    // Preserve the user's existing order + enabled state (their MO2 or prior-run
    // plugins.txt / loadorder.txt). For PlainList games this also keeps disabled
    // plugins disabled (recorded only in loadorder.txt), not just the actives.
    let dir = eidos_plugins::plugins_txt_dir(&prefix, &spec);
    list.apply_prefix_state(&dir, &spec);
    list.refresh(&spec);

    for (p, m) in list.missing_masters() {
        eprintln!("eidos play: WARNING - {p} is missing master {m} (likely a crash)");
    }
    let active = list.plugins.iter().filter(|p| p.enabled).count();
    match list.write_load_order(&dir, &spec) {
        Ok(()) => eprintln!("eidos play: wrote {active} active plugins to plugins.txt"),
        Err(e) => eprintln!("eidos play: could not write plugins.txt: {e}"),
    }
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
) -> Option<(std::path::PathBuf, &'static [&'static str])> {
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
    let prof = inst.active();

    if let Ok(n) = prof.seed_inis(&docs, ini_files) {
        if n > 0 {
            eprintln!("eidos play: seeded {n} INI(s) into profile '{}' from the prefix", prof.name);
        }
    }
    if let Ok(n) = prof.deploy_inis(&docs, ini_files) {
        if n > 0 {
            eprintln!("eidos play: deployed {n} profile INI(s) into the prefix");
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
        let _ = eidos_gamefeatures::enable_file_selection(&docs, ini);
    }
    Some((docs, ini_files))
}

/// Before launch: give the active profile its own saves. Seed the profile from
/// the prefix's existing saves on first run (adopting the playthrough), then
/// return the `(profile_saves, prefix_saves)` bind so the launcher redirects the
/// game's save dir to this profile for the run - the prefix is never modified.
fn prepare_saves(
    id: &str,
    game: &DetectedGame,
    inst: &Instance,
) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let spec = eidos_plugins::GameSpec::for_id(id)?;
    let compatdata = game.compatdata.as_ref()?;
    let docs = eidos_plugins::documents_my_games_dir(&compatdata.join("pfx"), &spec);
    let prefix_saves = docs.join("Saves");
    let prof = inst.active();
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

    run_through_view(id, &game, &inst, command, Vec::new(), None, &[]);
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
    let inis = prepare_inis(id, game, inst);
    prepare_plugins(id, game, inst);
    let save_bind = prepare_saves(id, game, inst);

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

    let spec = LaunchSpec {
        layers: inst.load_order(),
        overwrite: inst.overwrite_dir(),
        mountpoint: game.data_path.clone(),
        command,
        env,
        base_bind: Some((game.data_path.clone(), inst.base_dir())),
        binds: save_bind.into_iter().collect(),
        cwd,
    };
    let result = launch(spec);

    // The command has exited: capture any INI changes back into the profile.
    if let Some((docs, ini_files)) = inis {
        if let Ok(n) = inst.active().capture_inis(&docs, ini_files) {
            if n > 0 {
                eprintln!("eidos: captured {n} INI(s) back into profile '{}'", inst.active_profile());
            }
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

/// Compose the `WINEDLLOVERRIDES` that forces the right DLLs native-then-builtin
/// (`n,b`) so mod graphics DLLs actually load under Wine. Two cases, mirroring
/// MO2's forced libraries:
///
/// 1. A mod SHIPS a top-level DLL that shadows a Wine builtin (ENB `d3d11`,
///    ReShade `dxgi`, `.asi` loaders) - force the mod's own native so the builtin
///    doesn't win.
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
    const SHIPPED_SHADOWS: &[&str] = &[
        "d3d8", "d3d9", "d3d10", "d3d11", "d3d12", "dxgi", "dinput", "dinput8", "winmm",
        "xinput1_3", "x3daudio1_7", "opengl32",
    ];
    let mut roots: Vec<PathBuf> =
        inst.modlist().into_iter().filter(|m| m.enabled && !m.is_separator()).map(|m| m.path).collect();
    roots.push(inst.overwrite_dir());

    let mut stems: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for root in &roots {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            if let Some(stem) = name.strip_suffix(".dll") {
                if SHIPPED_SHADOWS.contains(&stem) {
                    stems.insert(stem.to_string());
                }
            }
        }
    }

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

/// Per-game default tools: the script extender, when present in the game dir.
fn default_tools(game: &DetectedGame) -> Vec<eidos_instance::Tool> {
    eidos_instance::default_tools(
        game.def.script_extender.as_ref().map(|se| se.loader),
        &game.install_path,
    )
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
            let tools = eidos_instance::merge_tools(inst.tools(), default_tools(&game));
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

            let tools = eidos_instance::merge_tools(inst.tools(), default_tools(&game));
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
    let tools = eidos_instance::merge_tools(inst.tools(), default_tools(&game));
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
    if !eidos_gamefeatures::cabextract_available() {
        eprintln!("warning: cabextract not on PATH - some winetricks verbs need it (e.g. `pacman -S cabextract`).");
    }
    let prefix = compat.join("pfx");
    println!(
        "Installing {} via winetricks (downloads from Microsoft). Close the game and all \
         tools first - installing while a session is open can corrupt the prefix.",
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

    // Discover exactly what a launch would deploy, preserving the current order.
    let enabled: Vec<ModEntry> =
        inst.modlist().into_iter().filter(|m| m.enabled && !m.is_separator()).collect();
    let sources = plugin_sources(&game.data_path, &enabled, &inst.overwrite_dir());
    let mut list = eidos_plugins::PluginList::discover(&sources, &spec);
    list.apply_prefix_state(&local_dir, &spec);
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
        &local_dir,
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
    match list.write_load_order(&local_dir, &spec) {
        Ok(()) => println!("Sorted {} plugins ({active} active) and wrote the load order.", sorted.len()),
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
    // FOMOD condition context: the plugins currently present/active, so a scripted
    // installer's fileDependency/gameDependency options evaluate correctly.
    let enabled_roots: Vec<std::path::PathBuf> =
        inst.modlist().into_iter().filter(|m| m.enabled && !m.is_separator()).map(|m| m.path).collect();
    let ctx = eidos_install::fomod_context(&game.data_path, &enabled_roots);
    match eidos_install::install_archive_with_policy(
        std::path::Path::new(archive),
        &inst.mods_dir(),
        &name,
        id,
        policy,
        &ctx,
    ) {
        Ok(r) => {
            // Activate the new mod at the top of the active profile's load order,
            // like MO2 (a freshly installed mod wins conflicts by default).
            let mut ml = inst.modlist();
            ml.retain(|m| m.name != r.name);
            ml.insert(0, ModEntry { name: r.name.clone(), enabled: true, path: r.dest.clone() });
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

/// `~/.config/eidos/nexus.ini`, holding the personal Nexus API key.
fn nexus_key_path() -> std::path::PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| home().join(".config"));
    config.join("eidos").join("nexus.ini")
}

/// The stored Nexus API key, if any.
fn load_nexus_key() -> Option<String> {
    let text = std::fs::read_to_string(nexus_key_path()).ok()?;
    text.lines()
        .filter_map(|l| l.trim().split_once('='))
        .find(|(k, _)| k.trim() == "api_key")
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
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
                    if let Some(p) = path.parent() {
                        let _ = std::fs::create_dir_all(p);
                    }
                    if let Err(e) = std::fs::write(&path, format!("[Nexus]\napi_key={key}\n")) {
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
         \x20 eidos sort <id> [--dry-run]       LOOT-sort the plugin load order (--update-masterlist to refresh)"
    );
    exit(2);
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
            path: mod_dir.clone(),
        }];

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
