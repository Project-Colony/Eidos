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

use std::process::exit;

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::{Instance, InstanceKind, ModEntry};
use eidos_launch::{launch, LaunchSpec};

fn find_game(id: &str) -> Option<DetectedGame> {
    detect(&home()).into_iter().find(|g| g.def.id == id)
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

    // Sources: the game's own Data first (the base masters), then each enabled mod
    // in ascending plugin priority (the modlist is highest-first, so reverse it).
    let mut sources: Vec<(String, std::path::PathBuf)> =
        vec![(String::new(), game.data_path.clone())];
    let mut enabled: Vec<_> = inst.modlist().into_iter().filter(|m| m.enabled).collect();
    enabled.reverse();
    sources.extend(enabled.into_iter().map(|m| (m.name, m.path)));

    let mut list = eidos_plugins::PluginList::discover(&sources, &spec);

    // Preserve the user's existing order (their MO2 or prior-run plugins.txt).
    let dir = eidos_plugins::plugins_txt_dir(&prefix, &spec);
    let existing = eidos_plugins::PluginList::read_active(&dir, &spec);
    if !existing.is_empty() {
        list.apply_active(&existing);
    }
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
    let docs = eidos_plugins::documents_my_games_dir(&compatdata.join("pfx"), &spec);
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
    // Loose files must win over the vanilla BSAs (writes into the deployed INI).
    if let Some(ini) = eidos_gamefeatures::ini_file_for(id) {
        match eidos_gamefeatures::enable_bsa_invalidation(&docs, ini) {
            Ok(()) => eprintln!("eidos play: BSA invalidation on ([Archive] bInvalidateOlderFiles=1 in {ini})"),
            Err(e) => eprintln!("eidos play: could not enable BSA invalidation: {e}"),
        }
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

    run_through_view(id, &game, &inst, command, Vec::new(), None);
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
) -> ! {
    let inis = prepare_inis(id, game, inst);
    prepare_plugins(id, game, inst);
    let save_bind = prepare_saves(id, game, inst);

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
        Ok(status) => exit(status.code().unwrap_or(0)),
        Err(e) => {
            eprintln!("eidos: launch failed: {e}");
            exit(1)
        }
    }
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
            let mut user = inst.tools();
            user.retain(|t| !t.title.eq_ignore_ascii_case(title));
            user.push(eidos_instance::Tool {
                title: title.clone(),
                exe: std::path::PathBuf::from(exe),
                args: args[4..].to_vec(),
                workdir: None,
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
            let print_only = args.iter().any(|a| a == "--print");
            let extra: Vec<String> = match args.iter().position(|a| a == "--") {
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
            let Some(run) = eidos_games::proton_command(&home(), game.def.steam_app_id, compat)
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
            run_through_view(id, &game, &inst, command, run.env, cwd);
        }
        Some(other) => {
            eprintln!("unknown tool subcommand '{other}' (list | add | rm | run)");
            exit(2);
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

    let name = args.get(2).cloned().unwrap_or_else(|| eidos_install::guess_mod_name(archive));
    match eidos_install::install_archive(std::path::Path::new(archive), &inst.mods_dir(), &name, id) {
        Ok(r) => {
            // Activate the new mod at the top of the active profile's load order,
            // like MO2 (a freshly installed mod wins conflicts by default).
            let mut ml = inst.modlist();
            ml.retain(|m| m.name != r.name);
            ml.insert(0, ModEntry { name: r.name.clone(), enabled: true, path: r.dest.clone() });
            let _ = inst.save_modlist(&ml);

            print!("Installed '{}' for {}", r.name, game.def.name);
            if r.fomod {
                print!(" (via FOMOD, default options)");
            } else if !r.stripped.is_empty() {
                print!(" (stripped wrapper '{}')", r.stripped.trim_end_matches('/'));
            }
            println!();
            println!("  -> {}", r.dest.display());
            println!("  enabled at the top of the load order. `eidos play {id}` to use it.");
        }
        Err(e) => {
            eprintln!("install failed: {e}");
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
            let mut checked = 0u32;
            let mut updates = 0u32;
            for m in inst.modlist() {
                let mut meta = inst.mod_meta(&m.name);
                let Some(mod_id) = meta.mod_id() else { continue };
                checked += 1;
                if !updated.contains(&mod_id) {
                    continue;
                }
                match nexus.mod_info(game.def.nexus_game, mod_id) {
                    Ok(remote) => {
                        meta.set_newest_version(&remote.version);
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
                    Err(e) => eprintln!("  {}: {e}", m.name),
                }
            }
            println!(
                "{updates} update(s) available ({checked} mod(s) with a Nexus id, \
                 {} recently updated on Nexus).",
                updated.len()
            );
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
                .or_else(|| (!file.file_name.is_empty()).then(|| file.file_name.clone()))
                .unwrap_or_else(|| format!("{}-{}.archive", nxm.mod_id, nxm.file_id));
            let dest = inst.downloads_dir().join(&name);

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
         \x20 eidos nxm <url> | --register      download a Nexus Mod Manager link / register the handler"
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
        Some("nexus") => cmd_nexus(&args[1..]),
        Some("nxm") => cmd_nxm(&args[1..]),
        _ => usage(),
    }
}
