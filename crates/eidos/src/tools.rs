//! `eidos tool`: run a modding tool through the mounted view.

use std::path::{Path, PathBuf};
use std::process::exit;

use eidos_gamefeatures::RegistryOutcome;
use eidos_games::{home, DetectedGame};
use eidos_instance::{Instance, InstanceKind};

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

/// `eidos tool <game-id> [list | add <title> <exe> [args...] | rm <title> |
/// run <title> [--print] [-- extra...]]` - manage and run tools (xEdit, FNIS,
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

pub(crate) fn cmd_tool(args: &[String]) {
    let Some(id) = args.first() else {
        eidos_log::info!(
            "usage:\n\
             \x20 eidos tool <game-id>                       list tools\n\
             \x20 eidos tool <game-id> add <title> <exe> [args...]\n\
             \x20 eidos tool <game-id> rm <title>\n\
             \x20 eidos tool <game-id> run <title> [--print] [-- <extra args>...]"
        );
        exit(2);
    };
    let target = resolve(id);
    let Some(game) = find_game(&target.game_id) else {
        eidos_log::info!(
            "Game '{}' is not detected. Run `eidos games`.",
            target.game_id
        );
        exit(1);
    };
    let inst = target.inst;
    inst.create().ok();
    let _ = inst.ensure_manifest(&target.game_id, InstanceKind::Global);

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
        Some("run") => {
            let Some(title) = args.get(2) else {
                eidos_log::info!(
                    "usage: eidos tool {id} run <title> [--print] [-- <extra args>...]"
                );
                exit(2);
            };
            // Everything after `--` is opaque tool args, so scan for --print only
            // BEFORE the separator (a tool may itself take a --print flag).
            let sep = args.iter().position(|a| a == "--");
            let print_only = args[..sep.unwrap_or(args.len())]
                .iter()
                .any(|a| a == "--print");
            let extra: Vec<String> = match sep {
                Some(i) => args[i + 1..].to_vec(),
                None => Vec::new(),
            };

            let tools =
                eidos_instance::merge_tools(inst.tools(), default_tools_for(&game, Some(&inst)));
            let Some(tool) = tools.iter().find(|t| t.title.eq_ignore_ascii_case(title)) else {
                eidos_log::info!("No tool named '{title}'. List them: eidos tool {id}");
                exit(1);
            };
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
            let Some(compat) = game.compatdata.as_ref() else {
                eidos_log::info!(
                    "No Proton prefix for {id} - launch the game once through Steam first."
                );
                exit(1);
            };
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
            {
                let proton = run.proton.clone();
                let env = run.env.clone();
                match eidos_gamefeatures::ensure_registry(
                    compat,
                    &game.install_path,
                    game.def.registry_name,
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
                    Ok(RegistryOutcome::Imported) => {
                        eidos_log::info!("eidos: registered the game path in the Wine prefix")
                    }
                    Ok(RegistryOutcome::AlreadyCorrect) => {}
                    // The two silences that used to cost a support thread. The
                    // tool is about to name a folder that does not exist; these
                    // lines are the only thing that connects that to Eidos.
                    Ok(RegistryOutcome::PrefixUninitialised) => eidos_log::warn!(
                        "eidos: the Wine prefix has not been created yet, so the game path is not \
                         registered - launch {} once through Steam first, or '{title}' may open on \
                         an empty folder.",
                        game.def.name
                    ),
                    Ok(RegistryOutcome::PrefixBusy(busy)) => {
                        let who = busy
                            .first()
                            .map(|(pid, cmd)| format!("{} (pid {pid})", cmd.split_whitespace().next().unwrap_or(cmd)))
                            .unwrap_or_else(|| "another program".to_string());
                        eidos_log::warn!(
                            "eidos: the Wine prefix is in use by {who}, so the game path could not \
                             be registered - close it and run '{title}' again if it opens on an \
                             empty folder."
                        );
                    }
                    Err(e) => eidos_log::warn!("eidos: could not write the prefix registry ({e}); tools may ask for the game path"),
                }
            }
            let mut command = run.command(&exe, &tool.args);
            command.extend(extra);
            // MO2's default working directory for a tool is its own folder - which,
            // after the rewrite above, is the merged one. An explicit workdir gets
            // the same treatment (MO2 adjusts cwd and binary independently).
            let cwd = tool
                .workdir
                .clone()
                .map(|c| virtualize_under_data(&c, &layers, &game.data_path).unwrap_or(c))
                .or_else(|| exe.parent().map(|p| p.to_path_buf()));
            let prereqs = tool.prereqs.clone();
            let output_mod = tool.output_mod.clone();
            let app_id = tool.app_id;
            // The bundled Tier-1 DLLs get provisioned at launch; but a Tier-2 verb
            // (vcrun/dotnet) that hasn't been installed will likely crash the tool, so
            // warn with the fix - without blocking (the user may have it via Steam).
            let satisfied = satisfied_prereqs_in(&inst, game.compatdata.as_ref());
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
            // Tier 3: point the tool at any runtime it declares. Nothing is
            // installed into the prefix - the variable IS the mechanism, which is
            // why an absent runtime must contribute nothing rather than a path
            // that does not exist (the .NET host stops looking once it is set).
            let mut run = run;
            run.env
                .extend(eidos_gamefeatures::runtime_env_for(&prereqs));
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
                eidos_log::info!(
                    "eidos tool: '{title}' needs the {names} runtime, which is not downloaded yet - \
                     run `eidos prereqs {id} --install`. Without it DynDOLOD's LODGen dies at startup \
                     leaving a log with nothing in it but a version banner."
                );
            }

            // A tool that IS its own Steam app - the Creation Kit is the usual
            // one - wants that app's id, not the game's. Both variables, because
            // Proton reads one and Steam's own libraries read the other, and a
            // tool that sees them disagree is a tool that fails oddly rather
            // than clearly. Set here, on the env that is about to be handed
            // over, so `--print` shows exactly what the real run would get.
            if let Some(id) = app_id {
                run.env
                    .retain(|(k, _)| k != "SteamAppId" && k != "SteamGameId");
                run.env.push(("SteamAppId".to_string(), id.to_string()));
                run.env.push(("SteamGameId".to_string(), id.to_string()));
            }
            if print_only {
                println!(
                    "would run (through the merged view at {}):",
                    game.data_path.display()
                );
                println!("  argv : {command:?}");
                for (k, v) in &run.env {
                    println!("  env  : {k}={v}");
                }
                if let Some(c) = &cwd {
                    println!("  cwd  : {}", c.display());
                }
                if let Some(m) = &output_mod {
                    println!("  out  : mods/{m} (the run's Overwrite output is captured there)");
                }
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
            run_through_view(
                // The RESOLVED game, not the CLI argument: see the note in
                // `run_through_view`. `id` here is "/mnt/Jeux/Eidos-Fallout4".
                game.def.id,
                &game,
                &inst,
                command,
                run.env,
                cwd,
                crate::launch::ToolOpts {
                    prereqs: &prereqs,
                    output_mod: output_mod.as_deref(),
                    tool_name: Some(title),
                    executable: Some(&source_exe),
                },
            );
        }
        Some(other) => {
            eidos_log::warn!("unknown tool subcommand '{other}' (list | add | rm | run)");
            exit(2);
        }
    }
}

#[cfg(test)]
mod root_tool_tests {
    use super::*;

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
