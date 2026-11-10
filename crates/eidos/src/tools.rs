//! `eidos tool`: run a modding tool through the mounted view.

use std::path::{Path, PathBuf};
use std::process::exit;

use eidos_gamefeatures::RegistryOutcome;
use eidos_games::{home, DetectedGame};
use eidos_instance::{Instance, InstanceKind};

use crate::check::launch_option::{launch_handoff, Handoff};

use crate::*;

/// Per-game default tools auto-detected in the game dir: the script extender, the
/// vanilla launcher, and the game binary - whichever are present.
///
/// `inst` widens the search to enabled mods' `Root/` directories, so a script
/// extender installed AS A MOD is detected too; at launch the root union puts it
/// on the game root for real.
pub(crate) fn default_tools_for(
    game: &DetectedGame,
    inst: Option<&Instance>,
) -> Vec<eidos_instance::Tool> {
    let roots = inst.map(|i| i.root_layers()).unwrap_or_default();
    let prefs = eidos_instance::settings::Settings::load();
    let extra = eidos_instance::tool_search_roots(
        inst.map(|i| i.mods_dir()).as_deref(),
        prefs.tools_dir.as_deref(),
    );
    eidos_instance::default_tools_in_view(
        game_executables(game),
        &game.install_path,
        &roots,
        &extra,
        inst.map(|i| i.root_overwrite_dir()).as_deref(),
    )
}

/// The auto-detectable executables for a game, from its `GameDef`.
pub(crate) fn game_executables(game: &DetectedGame) -> eidos_instance::GameExecutables<'_> {
    eidos_instance::GameExecutables {
        game_name: game.def.name,
        launcher: game.def.script_extender.as_ref().map(|se| se.launcher),
        binary: Some(game.def.game_binary),
        script_extender: game.def.script_extender.as_ref().map(|se| se.loader),
        known_tools: game.def.known_tools,
    }
}

/// Variables a store sets on the process it starts. Steam exports the first two.
/// Heroic exports the `HEROIC_*` pair for every wrapper command, Legendary games
/// included (it launches them for Heroic, source `epic`). The bare `legendary`
/// CLI exports no marker of its own, so a window it opened sets
/// [`STORE_STARTED_VAR`] on the `eidos tool` child instead.
const STORE_MARKERS: [&str; 5] = [
    "SteamGameId",
    "SteamAppId",
    "HEROIC_APP_NAME",
    "HEROIC_APP_RUNNER",
    STORE_STARTED_VAR,
];

/// True when any variable name in `keys` is a store marker.
fn store_started_in(keys: impl IntoIterator<Item = String>) -> bool {
    keys.into_iter()
        .any(|k| STORE_MARKERS.contains(&k.as_str()))
}

/// A game entry run from a plain shell (a desktop shortcut made with 1.22, say)
/// goes to the store that owns the copy, as Run does in a window no store
/// opened. Inside a store-started process it runs as before.
fn hands_off_to_store(runs_game: bool, store_started: bool) -> bool {
    runs_game && !store_started
}

/// What `tool run` does with [`launch_handoff`]'s answer: `Ok` holds the store
/// command to spawn and the line to print, `Err` the line saying what to set.
fn game_entry_handoff(
    handoff: Handoff,
    game: &str,
    source: &str,
) -> Result<(Vec<String>, String), String> {
    match handoff {
        Handoff::Start { store, argv } => Ok((
            argv,
            format!(
                "eidos tool: {store} is starting {game} through its Eidos setting. If an \
                 Eidos window opens, press Run there."
            ),
        )),
        Handoff::SetSteamOption { line } => Err(format!(
            "eidos tool: this entry starts {game}, which starts through Steam. Set the game's \
             Steam launch option to:  {line}"
        )),
        Handoff::SetHeroicWrapper { exe, args } => Err(format!(
            "eidos tool: Heroic does not start {game} through Eidos. In the game's settings in \
             Heroic, add a wrapper with the executable {exe} and the arguments \"{args}\"."
        )),
        Handoff::SetLegendaryWrapper { file, app, wrapper } => Err(format!(
            "eidos tool: Legendary does not start {game} through Eidos. Add this line under \
             [{app}] in {}:  wrapper = {wrapper}",
            file.display()
        )),
        Handoff::NoLoader { loader } => Err(format!(
            "eidos tool: {loader} is not next to the game's executable in this instance, so \
             {source} is not asked to start {game}. Install the script extender first."
        )),
        Handoff::ThroughStore => Err(format!(
            "eidos tool: this entry starts {game}. Start it from {source}: Eidos cannot ask \
             {source} to start this copy."
        )),
    }
}

/// `eidos tool <game-id> [list | add <title> <exe> [args...] | rm <title> |
/// run <title> [--print] [--output-mod <mod>] [-- extra...]]` - manage and run tools (xEdit, FNIS,
/// BodySlide) through the merged view, inside the game's Proton prefix.
/// A command uses the projected path, while existence and provenance use its
/// real backing file. Root-only tools do not exist in the game directory yet.
fn tool_executable(
    inst: &Instance,
    install: &Path,
    data: &Path,
    executable: &Path,
) -> Option<(PathBuf, PathBuf)> {
    let projected = if executable.is_absolute() {
        executable.to_path_buf()
    } else {
        install.join(executable)
    };
    let layers = inst.load_order();
    let data_command = virtualize_under_data(&projected, &layers, data)
        .or_else(|| {
            projected
                .strip_prefix(inst.overwrite_dir())
                .ok()
                .map(|p| data.join(p))
        })
        .unwrap_or_else(|| projected.clone());
    if let Ok(relative) = data_command.strip_prefix(data) {
        let source = eidos_instance::root_executable_source(
            data,
            &layers,
            &inst.overwrite_dir(),
            relative.to_str()?,
        )?;
        return Some((source, data_command));
    }
    if let Ok(relative) = projected.strip_prefix(install) {
        let source = eidos_instance::root_executable_source(
            install,
            &inst.root_layers(),
            &inst.root_overwrite_dir(),
            relative.to_str()?,
        )?;
        let command_path = if source.starts_with(install) {
            source.clone()
        } else {
            projected
        };
        Some((source, command_path))
    } else {
        projected.is_file().then(|| (projected.clone(), projected))
    }
}

/// `run` flags (`args` starts at `run`): (print, output mod, extra args). Flags count only BEFORE the
/// first `--`; everything after it is opaque tool args (a tool may take its own
/// `--print` or `--output-mod`).
fn run_flags(args: &[String]) -> Result<(bool, Option<String>, Vec<String>), String> {
    let sep = args.iter().position(|a| a == "--").unwrap_or(args.len());
    let extra = args.get(sep + 1..).unwrap_or_default().to_vec();
    let mut print = false;
    let mut output = None;
    let mut it = args[..sep].iter().skip(2);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--print" => print = true,
            "--output-mod" => {
                let m = it.next().ok_or("--output-mod needs a mod name")?;
                if !eidos_instance::is_mod_folder_name(m) {
                    return Err(format!("'{m}' is not a valid mod folder name"));
                }
                output = Some(m.clone());
            }
            _ => {}
        }
    }
    Ok((print, output, extra))
}

/// `args` starts at `run`: the path of `run --exe <path>`, `None` for a run by title,
/// `Err` when `--exe` has no path.
fn exe_target(args: &[String]) -> Option<Result<PathBuf, String>> {
    (args.get(1).map(String::as_str) == Some("--exe")).then(|| {
        args.get(2)
            .filter(|p| !p.starts_with("--"))
            .map(PathBuf::from)
            .ok_or_else(|| "--exe needs the path of an executable".to_string())
    })
}

pub(crate) fn cmd_tool(args: &[String]) {
    // `--help` is a request for this text, not a game id to look up.
    let asked_help = args.first().is_some_and(|a| a == "--help" || a == "-h");
    let Some(id) = args.first().filter(|_| !asked_help) else {
        eidos_log::info!(
            "usage:\n\
             \x20 eidos tool <game-id>                       list tools\n\
             \x20 eidos tool <game-id> add <title> <exe> [args...]\n\
             \x20 eidos tool <game-id> rm <title>\n\
             \x20 eidos tool <game-id> import-mo2 [--write]\n\
             \x20 eidos tool <game-id> run <title>|--exe <path> [--print] [--output-mod <mod>] [-- <extra args>...]"
        );
        exit(if asked_help { 0 } else { 2 });
    };
    let target = resolve(id);
    let Some(game) = find_instance_game(&target) else {
        eidos_log::info!(
            "Game '{}' is not detected. Run `eidos games`.",
            target.game_id
        );
        exit(1);
    };
    let inst = target.inst;
    if let Err(error) = inst
        .create()
        .and_then(|()| inst.ensure_manifest(&target.game_id, InstanceKind::Global))
    {
        eidos_log::warn!("Cannot prepare this instance: {error}");
        exit(1);
    }

    match args.get(1).map(String::as_str) {
        None | Some("list") => {
            let tools =
                eidos_instance::merge_tools(inst.tools(), default_tools_for(&game, Some(&inst)));
            if tools.is_empty() {
                println!("No tools. Add one: eidos tool {id} add <title> <exe> [args...]");
                return;
            }
            println!(
                "Tools for {} (run: eidos tool {id} run <title>):",
                game.def.name
            );
            for t in &tools {
                let exe = if t.exe.is_absolute() {
                    t.exe.clone()
                } else {
                    game.install_path.join(&t.exe)
                };
                let missing = if exe.is_file() { "" } else { "  [MISSING]" };
                println!("  {:<18} {}{}", t.title, t.exe.display(), missing);
            }
        }
        Some("add") => {
            let (Some(title), Some(exe)) = (args.get(2), args.get(3)) else {
                eidos_log::info!("usage: eidos tool {id} add <title> <exe> [args...]");
                exit(2);
            };
            // The title becomes a `[Tool/<title>]` section header, so reject what
            // cannot round-trip: empty or control characters (a newline would split
            // the header and corrupt neighbouring tools on the next read).
            let title = title.trim();
            if title.is_empty() || title.chars().any(char::is_control) {
                eidos_log::warn!(
                    "Invalid tool title: must be non-empty and free of control characters."
                );
                exit(2);
            }
            let mut user = inst.tools();
            user.retain(|t| !t.title.eq_ignore_ascii_case(title));
            user.push(eidos_instance::Tool {
                title: title.to_string(),
                exe: std::path::PathBuf::from(exe),
                // What the user typed wins. Seeded only when they typed nothing,
                // so a known tool that needs an unguessable flag (PGPatcher's
                // `--ignore-mo2vfscheck`) still gets it from the CLI.
                args: if args.len() > 4 {
                    args[4..].to_vec()
                } else {
                    eidos_instance::default_args(title)
                },
                workdir: None,
                // Seed known tools' runtime prereqs (BodySlide -> d3dx, Synthesis ->
                // dotnet...); the user can edit tools.ini to override.
                prereqs: eidos_instance::default_prereqs(title),
                output_mod: eidos_instance::default_output_mod(title),
                // Set through the GUI's Executables editor, or by hand.
                ..Default::default()
            });
            match inst.save_tools(&user) {
                Ok(()) => println!("Added '{title}'. Run it: eidos tool {id} run {title}"),
                Err(e) => {
                    eidos_log::warn!("could not save tools.ini: {e}");
                    exit(1);
                }
            }
        }
        Some("rm") => {
            let Some(title) = args.get(2) else {
                eidos_log::info!("usage: eidos tool {id} rm <title>");
                exit(2);
            };
            let mut user = inst.tools();
            let before = user.len();
            user.retain(|t| !t.title.eq_ignore_ascii_case(title));
            if user.len() == before {
                eidos_log::warn!("No user tool named '{title}' (defaults cannot be removed).");
                exit(1);
            }
            let _ = inst.save_tools(&user);
            println!("Removed '{title}'.");
        }
        Some("import-mo2") => {
            let Some(path) = inst.foreign_mo2_ini() else {
                eidos_log::warn!(
                    "No ModOrganizer.ini from Mod Organizer 2 in {}.",
                    inst.root.display()
                );
                exit(1);
            };
            let ini = match std::fs::read(&path) {
                Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                Err(e) => {
                    eidos_log::warn!("could not read {}: {e}", path.display());
                    exit(1);
                }
            };
            let found = eidos_instance::mo2_executables(&ini, &inst.root, game_executables(&game));
            let user = inst.tools();
            let (added, report) = mo2_import_plan(&found, &user);
            print!("{report}");
            if !args[2..].iter().any(|a| a == "--write") {
                println!("Nothing written. Add them: eidos tool {id} import-mo2 --write");
                return;
            }
            let n = added.len();
            match inst.save_tools(&[user, added].concat()) {
                Ok(()) => println!("Added {n} tool(s) to tools.ini."),
                Err(e) => {
                    eidos_log::warn!("could not save tools.ini: {e}");
                    exit(1);
                }
            }
        }
        Some("run") => {
            let by_exe = exe_target(&args[1..]);
            if let Some(Err(e)) = &by_exe {
                eidos_log::warn!("eidos tool: {e}");
                exit(2);
            }
            let Some(title) = args.get(2) else {
                eidos_log::info!(
                    "usage: eidos tool {id} run <title>|--exe <path> [--print] [--output-mod <mod>] [-- <extra args>...]"
                );
                exit(2);
            };
            let flag_args = if by_exe.is_some() {
                &args[2..]
            } else {
                &args[1..]
            };
            let (print_only, output_flag, extra) = match run_flags(flag_args) {
                Ok(v) => v,
                Err(e) => {
                    eidos_log::warn!("eidos tool: {e}");
                    exit(2);
                }
            };

            let tool = if let Some(Ok(path)) = by_exe {
                // Never a registered tool nor a utility: the file is run as given.
                eidos_instance::tool_for_exe(&path, game.def.known_tools)
            } else {
                // A user tool wins `merge_tools` over a default with the same title,
                // so look there first: the defaults are detected by walking the game,
                // the Root layers and the whole mods pool, wasted for a user tool.
                let user = inst.tools();
                let named = |t: &eidos_instance::Tool| t.title.eq_ignore_ascii_case(title);
                let found = match user.iter().find(|t| named(t)) {
                    Some(tool) => Some(tool.clone()),
                    None => {
                        eidos_instance::merge_tools(user, default_tools_for(&game, Some(&inst)))
                            .into_iter()
                            .find(|t| named(t))
                    }
                };
                let Some(tool) = found else {
                    eidos_log::info!("No tool named '{title}'. List them: eidos tool {id}");
                    exit(1);
                };
                tool
            };
            let title = &tool.title;
            let Some((source_exe, exe)) =
                tool_executable(&inst, &game.install_path, &game.data_path, &tool.exe)
            else {
                eidos_log::warn!(
                    "Tool executable not found in the active view: {}",
                    tool.exe.display()
                );
                exit(1);
            };
            // `load_order`, NOT `root_layers`: the latter is the Root Builder list
            // (mods with a `root/` subdir, destined for the game's install folder)
            // and would have matched almost nothing.
            let layers = inst.load_order();
            let exe = virtualize_under_data(&exe, &layers, &game.data_path).unwrap_or_else(|| {
                // Not inside an enabled mod: either a game-root tool (xEdit, the
                // script extender) which is already where it should be, or a mod
                // the user has disabled, in which case its own files are not in the
                // view either and running it from its folder is the only option.
                if exe.starts_with(inst.mods_dir()) {
                    eidos_log::info!(
                        "eidos tool: '{title}' lives in a mod that is not enabled - it will run \
                         from its own folder and will not see other mods' files."
                    );
                }
                exe.clone()
            });
            let runs_game = game_executables(&game).launches_the_game(&tool.exe);
            let store_started =
                store_started_in(std::env::vars_os().filter_map(|(k, _)| k.into_string().ok()));
            if hands_off_to_store(runs_game, store_started) {
                let loader_found = game.def.script_extender.as_ref().is_none_or(|se| {
                    eidos_instance::root_executable(
                        &game.install_path,
                        &inst.root_layers(),
                        &inst.root_overwrite_dir(),
                        se.loader,
                    )
                    .is_some()
                });
                let search_path: Vec<PathBuf> = std::env::var_os("PATH")
                    .map(|p| std::env::split_paths(&p).collect())
                    .unwrap_or_default();
                let handoff = launch_handoff(
                    &game,
                    loader_found,
                    &crate::this_exe().0,
                    &home(),
                    &search_path,
                );
                let source = game.source_name();
                match game_entry_handoff(handoff, game.def.name, source) {
                    Ok((argv, _)) if print_only => println!("{}", argv.join(" ")),
                    Ok((argv, line)) => {
                        if let Err(e) = std::process::Command::new(&argv[0])
                            .args(&argv[1..])
                            .spawn()
                        {
                            eidos_log::warn!(
                                "eidos tool: could not run {} ({e}). Start {} from {source} \
                                 yourself.",
                                argv[0],
                                game.def.name
                            );
                            exit(1);
                        }
                        println!("{line}");
                    }
                    Err(hint) => {
                        eidos_log::info!("{hint}");
                        exit(1);
                    }
                }
                return;
            }
            // `runner=native` runs the executable itself: no Proton, no Steam
            // prefix, and none of the Proton-only steps below.
            let native = tool.runner == eidos_instance::Runner::Native;
            if native && tool.slr {
                eidos_log::info!(
                    "eidos tool: '{title}' has runner=native, so its slr= line is ignored: the \
                     Steam Linux Runtime container only wraps a Proton run."
                );
            }
            let compat = game.compatdata.as_deref();
            let mut run = match (compat, game.heroic_run()) {
                _ if native => None,
                // A Heroic copy: the Wine or Proton and prefix its settings name.
                (_, Some(run)) => Some(run.clone()),
                (None, None) => {
                    eidos_log::info!(
                        "No Steam Proton prefix for {id}. For a Heroic/Legendary copy, set \
                         runner=native on the tool in tools.ini, with exe= the copy's own runner \
                         (see Tools in the guide), or launch the game once through Heroic so its \
                         prefix exists."
                    );
                    exit(1);
                }
                (Some(compat), None) => {
                    let Some(run) = eidos_games::proton_command(
                        &home(),
                        game.def.steam_app_id,
                        compat,
                        &game.install_path,
                    ) else {
                        eidos_log::warn!(
                            "Could not resolve the Proton for {id} (config.vdf CompatToolMapping / \
                             compatibilitytools.d). Is the game set up to run under Proton?"
                        );
                        exit(1);
                    };
                    warn_if_flatpak_proton(&run);
                    Some(run)
                }
            };
            // `slr=1`: the tool and the registry import below start in the
            // container Steam uses for the game. `eidos prereqs` keeps running
            // winetricks on the host.
            if let Some(run) = run.as_mut().filter(|_| tool.slr) {
                match run.in_steam_runtime(&home()) {
                    Ok(()) => {}
                    Err(None) => {
                        eidos_log::warn!(
                            "'{title}' has slr=1, but {} names no Steam Linux Runtime (no \
                             require_tool_appid in its toolmanifest.vdf). Remove slr=1 to run \
                             the tool on the host.",
                            run.proton.display()
                        );
                        exit(1);
                    }
                    Err(Some(app)) => {
                        eidos_log::warn!(
                            "'{title}' has slr=1, but the Steam Linux Runtime its Proton needs, \
                             Steam app {app}, is not installed in any Steam library. Install it \
                             with `steam steam://install/{app}`, or remove slr=1 to run the tool \
                             on the host."
                        );
                        exit(1);
                    }
                }
            }
            // Windows tools find their game by reading HKLM\Software\Bethesda
            // Softworks\<game> "installed path", which the game's own installer
            // writes - and which Steam under Proton never runs. Without it xEdit,
            // Wrye Bash and DynDOLOD open on an empty path. Idempotent, additive,
            // and skipped when the prefix is uninitialised or in use.
            //
            // `game.def`, NOT a second lookup keyed on `id`: `id` is whatever the
            // caller named the instance with, and the GUI names it by PATH
            // (`eidos tool /mnt/Jeux/Eidos-Fallout4 run ...`). `GameDef::for_id`
            // returned None for every such path, so this whole block was skipped
            // for every run started from the GUI - the one way anyone starts one.
            // `game` is the already-resolved target a dozen lines above; there is
            // no second lookup left to get wrong.
            if let (Some(run), Some(prefix)) = (&run, game.prefix()) {
                // A tool on the .NET SDK also needs NuGet's signing roots in
                // the prefix, or every package restore fails (NU3018).
                let roots = eidos_gamefeatures::nuget_roots_pem(&tool.prereqs);
                let and_roots = " and the .NET SDK's NuGet signing roots";
                match eidos_gamefeatures::ensure_registry(
                    &prefix,
                    compat,
                    &game.install_path,
                    game.def.registry_name,
                    &roots,
                    // `runinprefix`, not the game verb: this must not run
                    // Proton's game-drive setup, only a program in the
                    // existing prefix.
                    |reg_file| {
                        run.run_in_prefix(
                            Path::new("regedit"),
                            &[reg_file.to_string_lossy().into_owned()],
                        )
                    },
                    &run.env,
                ) {
                    Ok(RegistryOutcome::Imported(added)) => {
                        let and_roots = if added { and_roots } else { "" };
                        eidos_log::info!("eidos: registered the game path{and_roots} in the Wine prefix")
                    }
                    Ok(RegistryOutcome::AlreadyCorrect) => {}
                    // The two silences that used to cost a support thread. The
                    // tool is about to name a folder that does not exist; these
                    // lines are the only thing that connects that to Eidos.
                    Ok(RegistryOutcome::PrefixUninitialised) => eidos_log::warn!(
                        "eidos: the Wine prefix has not been created yet, so the game path is not \
                         registered - launch {} once through {} first, or '{title}' may open on \
                         an empty folder.",
                        game.def.name,
                        if game.is_steam() { "Steam" } else { "Heroic" }
                    ),
                    Ok(RegistryOutcome::PrefixBusy(busy, missing)) => {
                        let (and_roots, or_restore) = match missing {
                            true => (and_roots, " or its package restore fails with NU3018"),
                            false => ("", ""),
                        };
                        let who = busy
                            .first()
                            .map(|(pid, cmd)| format!("{} (pid {pid})", cmd.split_whitespace().next().unwrap_or(cmd)))
                            .unwrap_or_else(|| "another program".to_string());
                        eidos_log::warn!(
                            "eidos: the Wine prefix is in use by {who}, so the game path{and_roots} \
                             could not be registered - close it and run '{title}' again if it opens \
                             on an empty folder{or_restore}."
                        );
                    }
                    Err(e) => eidos_log::warn!("eidos: could not write the prefix registry ({e}); tools may ask for the game path"),
                }
            }
            let command = tool_command(run.as_ref(), &exe, &tool.args, extra);
            // MO2's default working directory for a tool is its own folder - which,
            // after the rewrite above, is the merged one. An explicit workdir gets
            // the same treatment (MO2 adjusts cwd and binary independently).
            let cwd = tool
                .workdir
                .clone()
                .map(|c| virtualize_under_data(&c, &layers, &game.data_path).unwrap_or(c))
                .or_else(|| exe.parent().map(|p| p.to_path_buf()));
            let prereqs = tool.prereqs.clone();
            let output_mod = output_flag.or_else(|| tool.output_mod.clone());
            let app_id = tool.app_id;
            // The bundled Tier-1 DLLs get provisioned at launch; but a Tier-2 verb
            // (vcrun/dotnet) that hasn't been installed will likely crash the tool, so
            // warn with the fix - without blocking (the user may have it via Steam).
            let satisfied = satisfied_prereqs_in(&inst, game.prefix().as_deref());
            let missing2: Vec<&String> = prereqs
                .iter()
                .filter(|v| eidos_gamefeatures::is_tier2_verb(v) && !satisfied.contains(*v))
                .collect();
            if !missing2.is_empty() {
                let names = missing2
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                eidos_log::info!(
                    "eidos tool: '{title}' needs {names} - run `eidos prereqs {id} --install` to set it up (downloads from Microsoft)."
                );
            }
            let wineserver_wait = wineserver_wait(run.as_ref(), &exe, &tool.env);
            // Tier 3: point the tool at any runtime it declares. Nothing is
            // installed into the prefix - the variable IS the mechanism, which is
            // why an absent runtime must contribute nothing rather than a path
            // that does not exist (the .NET host stops looking once it is set).
            let mut env = run.map(|r| r.env).unwrap_or_default();
            env.extend(eidos_gamefeatures::runtime_env_for(&prereqs));
            let missing3: Vec<&String> = prereqs
                .iter()
                .filter(|v| {
                    eidos_gamefeatures::runtime(v)
                        .is_some_and(|r| !eidos_gamefeatures::runtime_is_installed(r))
                })
                .collect();
            if !missing3.is_empty() {
                let names = missing3
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let effects: String = missing3
                    .iter()
                    .filter_map(|v| match eidos_gamefeatures::runtime(v)?.verb {
                        "dotnet10" => Some(
                            " Without dotnet10, a .NET 10 tool such as Pandora or DynDOLOD's \
                             LODGen does not start.",
                        ),
                        "dotnetsdk10" => {
                            Some(" Without dotnetsdk10, Synthesis cannot build its patchers.")
                        }
                        _ => None,
                    })
                    .collect();
                eidos_log::info!(
                    "eidos tool: '{title}' needs the {names} runtime, which is not downloaded yet - \
                     run `eidos prereqs {id} --install`.{effects}"
                );
            }

            // Host-environment facts for a tool run, applied last so `--print`
            // shows them. The Creation Kit override is a different Steam app, so
            // the game's shader cache would be the wrong one for it. Not for a
            // native tool: it may be a Linux .NET program, so the host's .NET
            // roots stay as they are. A runtime the tool declares still set its
            // Z: DOTNET_ROOT above, the form a native Wine runner needs.
            if !native {
                apply_tool_env_facts(
                    &mut env,
                    compat,
                    game.def.steam_app_id,
                    app_id.is_some(),
                    |k| std::env::var(k).ok(),
                );
            }

            // A tool that IS its own Steam app - the Creation Kit is the usual
            // one - wants that app's id, not the game's. Both variables, because
            // Proton reads one and Steam's own libraries read the other, and a
            // tool that sees them disagree is a tool that fails oddly rather
            // than clearly. Set here, on the env that is about to be handed
            // over, so `--print` shows exactly what the real run would get.
            if let Some(id) = app_id {
                env.retain(|(k, _)| k != "SteamAppId" && k != "SteamGameId");
                env.push(("SteamAppId".to_string(), id.to_string()));
                env.push(("SteamGameId".to_string(), id.to_string()));
            }
            // `tool_name: None` makes `run_through_view` treat this as a game run: it
            // records the launch start, the clean launch and crash detection.
            let opts = crate::launch::ToolOpts {
                prereqs: &prereqs,
                output_mod: output_mod.as_deref(),
                // A game start is session output, not generated output: no receipt.
                tool_name: (!runs_game).then_some(title),
                executable: (!runs_game).then_some(&source_exe),
                env: &tool.env,
            };
            if print_only {
                let env = crate::launch::run_env(&game, &inst, opts, env, false);
                print!(
                    "{}",
                    print_plan(
                        &game.data_path,
                        &command,
                        &env,
                        cwd.as_deref(),
                        opts.output_mod
                    )
                );
                return;
            }
            // A capture target the user cannot SEE is a target that quietly does
            // nothing on the next launch: the mod exists and holds the output,
            // but a disabled mod contributes nothing to the merged view, so the
            // tool would regenerate the same files every run.
            if let Some(m) = &output_mod {
                let listed = inst.modlist();
                match listed.iter().find(|e| e.name.eq_ignore_ascii_case(m)) {
                    Some(e) if !e.enabled => eidos_log::info!(
                        "eidos tool: '{title}' captures into mods/{m}, which is DISABLED - \
                         the output will be written but the game will not see it."
                    ),
                    _ => {}
                }
            }
            // Outside the view, before the tool starts: see `wineserver_wait`.
            if let Some(mut wait) = wineserver_wait {
                let prefix = wait
                    .get_envs()
                    .find(|(k, _)| *k == "WINEPREFIX")
                    .and_then(|(_, v)| v.map(PathBuf::from))
                    .or_else(|| std::env::var_os("WINEPREFIX").map(PathBuf::from))
                    // Wine's own default when no WINEPREFIX is set.
                    .unwrap_or_else(|| home().join(".wine"));
                eidos_log::info!(
                    "eidos: waiting for the Wine programs in {} to exit (the game, or build \
                     servers left by a previous run) before '{title}' starts",
                    prefix.display()
                );
                if let Err(e) = wait.status() {
                    eidos_log::warn!(
                        "eidos: could not wait for the prefix's wineserver ({e}); if '{title}' \
                         does not see the mods, close the game and run it again."
                    );
                }
            }
            run_through_view(
                // The RESOLVED game, not the CLI argument: see the note in
                // `run_through_view`. `id` here is "/mnt/Jeux/Eidos-Fallout4".
                game.def.id,
                &game,
                &inst,
                command,
                env,
                cwd,
                opts,
            );
        }
        Some(other) => {
            eidos_log::warn!(
                "unknown tool subcommand '{other}' (list | add | rm | run | import-mo2)"
            );
            exit(2);
        }
    }
}

/// The tools an MO2 import adds, and the report that lists each entry. A title
/// the user already has, or one added earlier in the same import, is skipped
/// (ignoring case): an import never replaces a tool.
fn mo2_import_plan(
    found: &[eidos_instance::Mo2Executable],
    user: &[eidos_instance::Tool],
) -> (Vec<eidos_instance::Tool>, String) {
    use eidos_instance::Mo2Skip;
    let mut added: Vec<eidos_instance::Tool> = Vec::new();
    let mut report = String::new();
    for entry in found {
        let title = &entry.title;
        let line = match &entry.result {
            Err(Mo2Skip::Unplaced(binary)) => {
                format!("  skip  {title}  {binary} is not under this list's folder")
            }
            Err(Mo2Skip::NotFound(binary)) => format!("  skip  {title}  {binary} was not found"),
            Ok(tool)
                if user
                    .iter()
                    .chain(&added)
                    .any(|t| t.title.eq_ignore_ascii_case(&tool.title)) =>
            {
                format!("  skip  {title}  a tool named {title} already exists")
            }
            Ok(tool) => {
                let command = std::iter::once(tool.exe.display().to_string())
                    .chain(tool.args.iter().cloned())
                    .collect::<Vec<_>>()
                    .join(" ");
                added.push(tool.clone());
                format!("  add   {title}  {command}")
            }
        };
        report.push_str(&line);
        report.push('\n');
    }
    (added, report)
}

/// What `eidos tool run --print` shows: the command, and every variable the run
/// sets on top of the inherited environment.
fn print_plan(
    data: &Path,
    command: &[String],
    env: &[(String, String)],
    cwd: Option<&Path>,
    output_mod: Option<&str>,
) -> String {
    let mut out = format!(
        "would run (through the merged view at {}):\n  argv : {command:?}\n",
        data.display()
    );
    for (k, v) in env {
        out.push_str(&format!("  env  : {k}={v}\n"));
    }
    if let Some(c) = cwd {
        out.push_str(&format!("  cwd  : {}\n", c.display()));
    }
    if let Some(m) = output_mod {
        out.push_str(&format!(
            "  out  : mods/{m} (the run's Overwrite output is captured there)\n"
        ));
    }
    out
}

/// The argv for a tool run: `exe` and `args` through the run's Proton or Wine,
/// or as they are for a `runner=native` tool, then the `--` extra arguments.
fn tool_command(
    run: Option<&eidos_games::ProtonRun>,
    exe: &Path,
    args: &[String],
    extra: Vec<String>,
) -> Vec<String> {
    let mut command = match run {
        Some(run) => run.command(exe, args),
        None => std::iter::once(exe.to_string_lossy().into_owned())
            .chain(args.iter().cloned())
            .collect(),
    };
    command.extend(extra);
    command
}

/// The `wineserver -w` to run before a tool, outside the view: a plain Wine
/// run's, or for a `runner=native` tool whose executable is a `wine`, the
/// server next to it with the tool's own `env=` (its `WINEPREFIX`).
fn wineserver_wait(
    run: Option<&eidos_games::ProtonRun>,
    exe: &Path,
    tool_env: &[(String, String)],
) -> Option<std::process::Command> {
    match run {
        Some(run) => run.wineserver_wait(),
        None => exe
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("wine"))
            .then(|| eidos_games::wineserver_wait_for(exe, tool_env)),
    }
}

/// Env a Windows tool needs from the host, not from the instance.
///
/// Steam's shader cache for the game (only for a Steam copy and when its
/// directory exists - Eidos never creates directories there - and never over a
/// variable the user exported),
/// empty .NET roots so a host SDK cannot hijack a framework-dependent tool, one
/// `WINEPATH` that still holds the user's own, and no Steam overlay in
/// `LD_PRELOAD`. `host` reads the inherited environment.
fn apply_tool_env_facts(
    env: &mut Vec<(String, String)>,
    compat: Option<&Path>,
    game_app_id: u32,
    app_id_override: bool,
    host: impl Fn(&str) -> Option<String>,
) {
    let mut push = |k: &str, v: String| env.push((k.to_string(), v));
    if let Some(compat) = compat.filter(|_| !app_id_override) {
        let cache = compat.join(format!("../../shadercache/{game_app_id}"));
        if cache.is_dir() {
            let c = |sub: &str| cache.join(sub).to_string_lossy().into_owned();
            let mut vars = vec![
                ("STEAM_COMPAT_SHADER_PATH", c("")),
                (
                    "__GL_SHADER_DISK_CACHE_APP_NAME",
                    "steamapp_shader_cache".into(),
                ),
                ("__GL_SHADER_DISK_CACHE_PATH", c("nvidiav1")),
                (
                    "__GL_SHADER_DISK_CACHE_READ_ONLY_APP_NAME",
                    "steam_shader_cache;steamapp_merged_shader_cache".into(),
                ),
                ("__GL_SHADER_DISK_CACHE_SKIP_CLEANUP", "1".into()),
                (
                    "MESA_DISK_CACHE_READ_ONLY_FOZ_DBS",
                    "steam_cache,steam_precompiled".into(),
                ),
                ("MESA_DISK_CACHE_SINGLE_FILE", "1".into()),
                ("MESA_SHADER_CACHE_DIR", c("")),
                ("MESA_SHADER_CACHE_MAX_SIZE", "5G".into()),
            ];
            // Only DXVK builds that still read it (sarek); Steam creates the dir.
            if cache.join("DXVK_state_cache").is_dir() {
                vars.push(("DXVK_STATE_CACHE_PATH", c("DXVK_state_cache")));
            }
            for (k, v) in vars {
                if host(k).is_none() {
                    push(k, v);
                }
            }
        }
    }
    // An empty value reads as unset for the .NET host, and still shows in --print.
    if !env.iter().any(|(k, _)| k == "DOTNET_ROOT") {
        env.push(("DOTNET_ROOT".into(), String::new()));
    }
    env.push(("DOTNET_ROOT_X64".into(), String::new()));
    // Wine searches WINEPATH, a `;` list, before the prefix PATH. Fold every
    // entry into one, the latest first since it is the one that would have won
    // alone, and keep a WINEPATH the user exported after them.
    let mut paths: Vec<String> = env
        .iter()
        .rev()
        .filter(|(k, _)| k == "WINEPATH")
        .map(|(_, v)| v.clone())
        .collect();
    if !paths.is_empty() {
        paths.extend(host("WINEPATH").filter(|h| !h.is_empty()));
        env.retain(|(k, _)| k != "WINEPATH");
        env.push(("WINEPATH".into(), paths.join(";")));
    }
    if let Some(pre) = host("LD_PRELOAD").filter(|p| p.contains("gameoverlayrenderer.so")) {
        let kept: Vec<&str> = pre
            .split([' ', ':'])
            .filter(|e| !e.is_empty() && !e.contains("gameoverlayrenderer.so"))
            .collect();
        env.push(("LD_PRELOAD".into(), kept.join(":")));
    }
}

#[cfg(test)]
mod root_tool_tests {
    use super::*;

    #[test]
    fn a_run_by_exe_takes_the_path_then_its_flags() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let args = v(&[
            "run",
            "--exe",
            "/m/FNIS/GenerateFNISforUsers.exe",
            "--print",
            "--",
            "-v",
        ]);
        assert_eq!(
            exe_target(&args),
            Some(Ok(PathBuf::from("/m/FNIS/GenerateFNISforUsers.exe")))
        );
        // Tuple form: QW2's `RunFlags` is not in this branch.
        assert_eq!(run_flags(&args[1..]), Ok((true, None, v(&["-v"]))));
        assert!(matches!(exe_target(&v(&["run", "--exe"])), Some(Err(_))));
        assert_eq!(exe_target(&v(&["run", "FNIS"])), None);
    }

    #[test]
    fn run_flags_read_print_and_output_mod_before_the_separator() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            run_flags(&v(&[
                "run",
                "T",
                "--output-mod",
                "Cleaned plugins",
                "--print",
                "--",
                "--output-mod",
                "x"
            ])),
            Ok((
                true,
                Some("Cleaned plugins".into()),
                v(&["--output-mod", "x"])
            ))
        );
        assert!(run_flags(&v(&["run", "T", "--output-mod"])).is_err());
        assert!(run_flags(&v(&["run", "T", "--output-mod", "a/b"])).is_err());
    }

    #[test]
    fn the_game_and_its_loader_are_not_generators() {
        let execs = eidos_instance::GameExecutables {
            game_name: "Skyrim Special Edition",
            launcher: Some("SkyrimSELauncher.exe"),
            binary: Some("SkyrimSE.exe"),
            script_extender: Some("skse64_loader.exe"),
            known_tools: &[],
        };
        assert!(execs.launches_the_game(Path::new("skse64_loader.exe")));
        assert!(execs.launches_the_game(Path::new("/g/SKSE64_LOADER.EXE")));
        assert!(execs.launches_the_game(Path::new("SkyrimSE.exe")));
        assert!(execs.launches_the_game(Path::new("SkyrimSELauncher.exe")));
        assert!(!execs.launches_the_game(Path::new("/t/SSEEdit.exe")));
        assert!(!execs.launches_the_game(Path::new("/t/SkyrimSE.exe.bak")));
    }

    #[test]
    fn store_markers_cover_steam_and_heroic() {
        let k = |a: &[&str]| store_started_in(a.iter().map(|s| s.to_string()));
        assert!(k(&["PATH", "SteamGameId"]));
        assert!(k(&["SteamAppId"]));
        assert!(k(&["HEROIC_APP_NAME", "HEROIC_APP_RUNNER"]));
        assert!(k(&["HEROIC_APP_RUNNER"]));
        // A window bare legendary opened: no store variable, only the window's own.
        assert!(k(&["PATH", "EIDOS_STORE_STARTED"]));
        assert!(!k(&["PATH", "HOME", "STEAM_COMPAT_DATA_PATH"]));
    }

    #[test]
    fn a_game_entry_from_a_plain_shell_goes_through_the_store() {
        assert!(hands_off_to_store(true, false));
        assert!(
            !hands_off_to_store(true, true),
            "the %command% path runs as before"
        );
        assert!(
            !hands_off_to_store(false, false),
            "a real tool is not handed off"
        );
        let argv = vec!["steam".to_string(), "-applaunch".into(), "489830".into()];
        let start = Handoff::Start {
            store: "Steam",
            argv: argv.clone(),
        };
        let (spawn, line) = game_entry_handoff(start, "Skyrim Special Edition", "Steam").unwrap();
        assert_eq!(spawn, argv);
        assert!(
            line.contains("Steam is starting Skyrim Special Edition"),
            "{line}"
        );
        let set = Handoff::SetSteamOption {
            line: "/opt/eidos play skyrimse -- %command%".into(),
        };
        let hint = game_entry_handoff(set, "Skyrim Special Edition", "Steam").unwrap_err();
        assert!(
            hint.ends_with(":  /opt/eidos play skyrimse -- %command%"),
            "{hint}"
        );
        let others = [
            Handoff::SetHeroicWrapper {
                exe: "/opt/eidos".into(),
                args: "play skyrimse --".into(),
            },
            Handoff::SetLegendaryWrapper {
                file: "/h/.config/legendary/config.ini".into(),
                app: "ac82db5035584c7f8a2c548d98c86b2c".into(),
                wrapper: "/opt/eidos play skyrimse --".into(),
            },
            Handoff::NoLoader {
                loader: "skse64_loader.exe",
            },
            Handoff::ThroughStore,
        ];
        for h in others {
            let hint = game_entry_handoff(h, "Skyrim Special Edition", "GOG").unwrap_err();
            assert!(!hint.contains('\n'), "one line: {hint}");
        }
    }

    #[test]
    fn data_tools_resolve_the_winning_source_before_fingerprinting() {
        let root = std::env::temp_dir().join(format!("eidos-data-tool-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("game");
        let data = game.join("Data");
        std::fs::create_dir_all(&data).unwrap();
        let instance = Instance::portable(root.join("instance"));
        instance.create().unwrap();
        let low = instance.create_empty_mod("Low").unwrap();
        let high = instance.create_empty_mod("High").unwrap();
        let relative = "Tools/generator.exe";
        for entry in [&low, &high] {
            std::fs::create_dir_all(entry.path.join("Tools")).unwrap();
            std::fs::write(entry.path.join(relative), entry.name.as_bytes()).unwrap();
        }
        instance.save_modlist(&[low.clone(), high.clone()]).unwrap();
        let projected = data.join(relative);
        let expected = Some((high.path.join(relative), projected.clone()));
        assert_eq!(
            tool_executable(
                &instance,
                &game,
                &game.join("Data"),
                &low.path.join(relative)
            ),
            expected
        );
        assert_eq!(
            tool_executable(&instance, &game, &game.join("Data"), &projected),
            expected
        );
        let overwrite = instance.overwrite_dir().join(relative);
        std::fs::create_dir_all(overwrite.parent().unwrap()).unwrap();
        std::fs::write(&overwrite, b"overwrite").unwrap();
        assert_eq!(
            tool_executable(&instance, &game, &game.join("Data"), &projected),
            Some((overwrite, projected.clone()))
        );
        std::fs::remove_file(instance.overwrite_dir().join(relative)).unwrap();
        std::fs::write(
            instance
                .overwrite_dir()
                .join("Tools/.eidoswh.generator.exe"),
            b"",
        )
        .unwrap();
        assert!(tool_executable(
            &instance,
            &game,
            &game.join("Data"),
            &low.path.join(relative)
        )
        .is_none());
        assert!(tool_executable(&instance, &game, &game.join("Data"), &projected).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_root_only_tool_has_a_backing_fingerprint_and_a_projected_command() {
        let root = std::env::temp_dir().join(format!("eidos-root-tool-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("game");
        std::fs::create_dir_all(&game).unwrap();
        let instance = Instance::portable(root.join("instance"));
        instance.create().unwrap();
        let mut entry = instance.create_empty_mod("SKSE").unwrap();
        let source = entry.path.join("Root/skse64_loader.exe");
        std::fs::create_dir(source.parent().unwrap()).unwrap();
        std::fs::write(&source, b"loader").unwrap();
        instance.save_modlist(&[entry.clone()]).unwrap();
        assert_eq!(
            tool_executable(
                &instance,
                &game,
                &game.join("Data"),
                Path::new("skse64_loader.exe")
            ),
            Some((source, game.join("skse64_loader.exe")))
        );
        entry.enabled = false;
        instance.save_modlist(&[entry]).unwrap();
        assert!(tool_executable(
            &instance,
            &game,
            &game.join("Data"),
            Path::new("skse64_loader.exe")
        )
        .is_none());
        let source = instance.root_overwrite_dir().join("skse64_loader.exe");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, b"overwrite loader").unwrap();
        assert_eq!(
            tool_executable(
                &instance,
                &game,
                &game.join("Data"),
                Path::new("skse64_loader.exe")
            ),
            Some((source, game.join("skse64_loader.exe")))
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod env_fact_tests {
    use super::*;

    fn get<'a>(env: &'a [(String, String)], k: &str) -> Option<&'a str> {
        env.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
    }

    fn library(with_cache: bool) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "eidos-envfacts-{}-{with_cache}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let compat = root.join("steamapps/compatdata/489830");
        std::fs::create_dir_all(&compat).unwrap();
        if with_cache {
            std::fs::create_dir_all(root.join("steamapps/shadercache/489830")).unwrap();
        }
        (root, compat)
    }

    #[test]
    fn a_native_wine_tool_waits_for_the_server_next_to_it() {
        use std::ffi::OsStr;
        let env = vec![("WINEPREFIX".to_string(), "/h/Prefixes/Skyrim".to_string())];
        let wait = wineserver_wait(None, Path::new("/opt/wine-ge/bin/wine"), &env)
            .expect("a native wine waits");
        assert_eq!(wait.get_program(), "/opt/wine-ge/bin/wineserver");
        assert_eq!(wait.get_args().collect::<Vec<_>>(), ["-w"]);
        assert_eq!(
            wait.get_envs().collect::<Vec<_>>(),
            [(
                OsStr::new("WINEPREFIX"),
                Some(OsStr::new("/h/Prefixes/Skyrim"))
            )]
        );
        assert!(wineserver_wait(None, Path::new("/usr/bin/Synthesis"), &env).is_none());
    }

    #[test]
    fn a_native_tool_runs_its_exe_then_its_args_then_the_extra_ones() {
        let args = ["C:\\Tools\\xEdit.exe".to_string(), "-sse".to_string()];
        assert_eq!(
            tool_command(
                None,
                Path::new("/opt/wine-ge/bin/wine"),
                &args,
                vec!["-quickautoclean".to_string()]
            ),
            [
                "/opt/wine-ge/bin/wine",
                "C:\\Tools\\xEdit.exe",
                "-sse",
                "-quickautoclean"
            ]
        );
    }

    #[test]
    fn shader_cache_vars_follow_the_cache_dir_and_never_override_the_user() {
        let (root, compat) = library(true);
        let mut env = vec![];
        apply_tool_env_facts(&mut env, Some(&compat), 489830, false, |k| {
            (k == "MESA_SHADER_CACHE_MAX_SIZE").then(|| "1G".to_string())
        });
        assert!(get(&env, "STEAM_COMPAT_SHADER_PATH")
            .unwrap()
            .contains("shadercache/489830"));
        assert!(get(&env, "__GL_SHADER_DISK_CACHE_PATH")
            .unwrap()
            .ends_with("nvidiav1"));
        assert_eq!(get(&env, "MESA_SHADER_CACHE_MAX_SIZE"), None);
        assert_eq!(get(&env, "DXVK_STATE_CACHE_PATH"), None);
        assert_eq!(get(&env, "MESA_GLSL_CACHE_DIR"), None);
        std::fs::create_dir(root.join("steamapps/shadercache/489830/DXVK_state_cache")).unwrap();
        let mut env = vec![];
        apply_tool_env_facts(&mut env, Some(&compat), 489830, false, |_| None);
        assert!(get(&env, "DXVK_STATE_CACHE_PATH").is_some());
        let mut env = vec![];
        apply_tool_env_facts(&mut env, Some(&compat), 489830, true, |_| None);
        assert_eq!(get(&env, "STEAM_COMPAT_SHADER_PATH"), None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn no_shader_vars_and_no_dir_created_when_steam_has_no_cache() {
        let (root, compat) = library(false);
        let mut env = vec![];
        apply_tool_env_facts(&mut env, Some(&compat), 489830, false, |_| None);
        assert_eq!(get(&env, "STEAM_COMPAT_SHADER_PATH"), None);
        assert!(!root.join("steamapps/shadercache").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn dotnet_roots_are_blanked_unless_the_tool_sets_one() {
        let mut env = vec![];
        apply_tool_env_facts(&mut env, None, 1, false, |_| None);
        assert_eq!(get(&env, "DOTNET_ROOT"), Some(""));
        assert_eq!(get(&env, "DOTNET_ROOT_X64"), Some(""));
        let mut env = vec![("DOTNET_ROOT".to_string(), "/rt".to_string())];
        apply_tool_env_facts(&mut env, None, 1, false, |_| None);
        assert_eq!(get(&env, "DOTNET_ROOT"), Some("/rt"));
        assert_eq!(env.iter().filter(|(k, _)| k == "DOTNET_ROOT").count(), 1);
    }

    #[test]
    fn winepath_entries_fold_into_one_and_keep_the_users() {
        // One already in the run's env, then the SDK runtime's, then the user's export.
        let mut env = vec![
            ("WINEPATH".to_string(), "C:\\run".to_string()),
            ("WINEPATH".to_string(), "Z:\\sdk".to_string()),
        ];
        let mine = |k: &str| (k == "WINEPATH").then(|| "C:\\mine".to_string());
        apply_tool_env_facts(&mut env, None, 1, false, mine);
        assert_eq!(get(&env, "WINEPATH"), Some("Z:\\sdk;C:\\run;C:\\mine"));
        assert_eq!(env.iter().filter(|(k, _)| k == "WINEPATH").count(), 1);
        // Nothing from Eidos: the inherited value reaches the tool untouched.
        let mut env = vec![];
        apply_tool_env_facts(&mut env, None, 1, false, mine);
        assert_eq!(get(&env, "WINEPATH"), None);
    }

    #[test]
    fn print_shows_a_tool_env_line_in_the_final_env() {
        let root = std::env::temp_dir().join(format!("eidos-tool-print-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let inst = Instance::portable(root.join("instance"));
        inst.create().unwrap();
        let game = DetectedGame {
            def: eidos_games::GameDef::for_id("skyrimse").unwrap(),
            install_path: root.join("game"),
            data_path: root.join("game/Data"),
            compatdata: None,
            source: Default::default(),
            steam_name: String::new(),
        };
        let tool_env = [("PROTON_LOG".to_string(), "1".to_string())];
        let opts = crate::launch::ToolOpts {
            env: &tool_env,
            ..Default::default()
        };
        let env = crate::launch::run_env(&game, &inst, opts, Vec::new(), false);
        let shown = print_plan(&game.data_path, &["/x/t".into()], &env, None, None);
        assert!(shown.contains("  env  : PROTON_LOG=1\n"), "{shown}");
        if std::env::var_os("PROTON_USE_XALIA").is_none() {
            assert!(shown.contains("PROTON_USE_XALIA=0"), "{shown}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn steam_overlay_leaves_ld_preload_but_other_entries_stay() {
        let pre = "/a/gameoverlayrenderer.so:/b/libx.so /c/gameoverlayrenderer.so";
        let mut env = vec![];
        apply_tool_env_facts(&mut env, None, 1, false, |k| {
            (k == "LD_PRELOAD").then(|| pre.to_string())
        });
        assert_eq!(get(&env, "LD_PRELOAD"), Some("/b/libx.so"));
        let mut env = vec![];
        apply_tool_env_facts(&mut env, None, 1, false, |k| {
            (k == "LD_PRELOAD").then(|| "/b/libx.so".to_string())
        });
        assert_eq!(get(&env, "LD_PRELOAD"), None);
    }
}

#[cfg(test)]
mod import_mo2_tests {
    use super::*;
    use eidos_instance::{Mo2Executable, Tool};

    #[test]
    fn an_import_never_replaces_a_tool_title() {
        let tool = |t: &str| Tool {
            title: t.into(),
            exe: format!("/t/{t}.exe").into(),
            ..Default::default()
        };
        let ok = |t: &str| Mo2Executable {
            title: t.into(),
            binary: format!("Z:/t/{t}.exe"),
            arguments: String::new(),
            result: Ok(tool(t)),
        };
        let (added, report) = mo2_import_plan(
            &[ok("SSEEdit"), ok("Other"), ok("Other")],
            &[tool("SSEEdit")],
        );
        assert_eq!(
            added.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
            ["Other"]
        );
        assert!(
            report.contains("a tool named SSEEdit already exists"),
            "{report}"
        );
    }
}
