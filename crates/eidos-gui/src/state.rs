//! Everything that reads or mutates `App` without drawing or dispatching: the
//! caches, the selection and fold arithmetic, keyboard navigation, and the
//! save/reload paths.
//!
//! Split out of `main.rs` unchanged. `update` calls into here; `view` reads from
//! it. main.rs is left with the types they all share and the iced wiring.

use crate::*;
use std::time::UNIX_EPOCH;

/// Build the per-mod metadata cache for the open instance's mod list.
/// Bring `app.meta_cache` in step with `app.mods`, computing ONLY the rows it does
/// not already hold and dropping the ones whose mod is gone.
///
/// Each row costs a `meta.ini` read plus `classify_content_dir`, which is a
/// `read_dir` on the mod plus two more on `meshes/` and `textures/`. Rebuilding
/// the whole map cost 100 meta reads and 100-300 `read_dir` on a 100-mod setup -
/// and it ran on EVERY checkbox click and EVERY arrow click, for 150-500 ms of
/// dead window each time. None of those actions change a single byte on disk.
///
/// So the map is only appended to. The three places that genuinely rewrite a
/// `meta.ini` drop what they changed first (see [`invalidate_meta`]), and Refresh
/// clears the lot - that is what Refresh is for.
pub(crate) fn refresh_meta_cache(app: &mut App) {
    let wanted: HashSet<String> = app.mods.iter().map(|m| m.name.clone()).collect();
    app.meta_cache.retain(|name, _| wanted.contains(name));
    let Some(inst) = app.created.clone() else {
        app.meta_cache.clear();
        return;
    };
    // Only built when there is actually something to compute: the catalog parses
    // the category files, which is pure waste on the common no-op refresh.
    let missing: Vec<(String, PathBuf)> = app
        .mods
        .iter()
        .filter(|m| !app.meta_cache.contains_key(&m.name))
        .map(|m| (m.name.clone(), m.path.clone()))
        .collect();
    if missing.is_empty() {
        return;
    }
    let cats = inst.category_factory();
    // The two state flags need the game's layout rules and its short name.
    // Resolved once for the whole refresh rather than per mod: both are constant
    // for the instance, and this loop already reads every mod's directory.
    let game = selected_game(app).map(|g| g.def);
    let rules =
        game.map(|g| eidos_install::LayoutRules::for_game(g.id)).unwrap_or_default();
    let short = game.map(|g| g.short_name).unwrap_or_default();
    for (name, path) in missing {
        let meta = inst.mod_meta(&name);
        let category_id = meta.category().as_deref().and_then(eidos_instance::parse_primary);
        let category_name = category_id.and_then(|id| cats.name_for_id(id)).map(str::to_string);
        app.meta_cache.insert(
            name,
            RowMeta {
                version: meta.version(),
                mod_id: meta.mod_id(),
                category_id,
                category_name,
                content_tags: eidos_install::classify_content_dir(&path).tags(),
                update: meta.update_available(),
                color: meta.color(),
                notes: meta.notes(),
                nexus_gone: meta.nexus_available() == Some(false),
                author: meta.author().or_else(|| meta.uploader()),
                game_name: meta.game_name().filter(|g| !g.is_empty()),
                installed_at: std::fs::metadata(&path).and_then(|m| m.modified()).ok(),
                // A mod the user vouched for is silent under both rules. That
                // is what MO2's `validated=` is for, and honouring it means a
                // mod already vouched for in MO2 arrives here quiet.
                invalid_data: !meta.validated()
                    && !eidos_install::folder_looks_valid(&path, rules),
                other_game: meta
                    .game_name()
                    .filter(|_| !meta.validated())
                    .filter(|g| !g.is_empty() && !short.is_empty())
                    .filter(|g| !g.eq_ignore_ascii_case(short)),
            },
        );
    }
}

/// Drop one mod's cached row, for the paths that rewrite its `meta.ini`. The next
/// [`refresh_meta_cache`] recomputes exactly that row.
pub(crate) fn invalidate_meta(app: &mut App, name: &str) {
    app.meta_cache.remove(name);
}


/// The run-target picker entry meaning "the game itself".
pub(crate) const RUN_GAME: &str = "Game (Steam command)";

/// An instance the welcome screen can offer to OPEN: something that exists on
/// disk and whose game is detected on this machine. Built from the registry
/// (portable roots + last used) and the detected games' global paths.
#[derive(Debug, Clone)]
pub(crate) struct KnownInstance {
    /// What the button says: game name + where the instance lives.
    pub label: String,
    pub inst: Instance,
    /// Index into `app.games`.
    pub game_index: usize,
    /// Portable instances live at a path the user chose and are listed from the
    /// registry; a global one is derived from the game id and cannot be renamed
    /// or forgotten.
    pub portable: bool,
}

/// Every existing instance worth offering on the welcome screen, most recently
/// used first. Portable roots come from the registry; the global path of each
/// detected game is appended. Roots that no longer exist are skipped, not
/// forgotten - an unmounted drive is not a deleted instance.
pub(crate) fn known_instances(games: &[DetectedGame]) -> Vec<KnownInstance> {
    // NEVER under test: the real registry lives in the user's config dir, and
    // a test whose assertions depend on which instances THIS machine happens
    // to know is not a test (same rule as the real-instance guard in `new`).
    // The building logic is `known_instances_from`, which tests feed directly.
    if cfg!(test) {
        return Vec::new();
    }
    known_instances_from(&eidos_instance::Registry::load(), games)
}

/// The construction behind [`known_instances`], with the registry injected.
pub(crate) fn known_instances_from(
    reg: &eidos_instance::Registry,
    games: &[DetectedGame],
) -> Vec<KnownInstance> {
    let mut out: Vec<KnownInstance> = Vec::new();
    let mut push = |inst: Instance, game_index: usize, portable: bool, games: &[DetectedGame]| {
        if !inst.exists() || out.iter().any(|k| k.inst.root == inst.root) {
            return;
        }
        let name = games[game_index].def.name;
        let label = if portable {
            format!("{name}  -  {}", inst.root.display())
        } else {
            format!("{name}  -  global")
        };
        out.push(KnownInstance { label, inst, game_index, portable });
    };
    // The last-used instance first: it is what the user means by "my setup".
    if let Some(last) = reg.last.clone() {
        let inst = last.instance();
        let gid = match &last {
            eidos_instance::InstanceRef::Global(id) => Some(id.clone()),
            eidos_instance::InstanceRef::Portable(_) => inst.read_manifest().map(|m| m.game_id),
        };
        if let Some(i) = gid.and_then(|gid| games.iter().position(|g| g.def.id == gid)) {
            let portable = matches!(last, eidos_instance::InstanceRef::Portable(_));
            push(inst, i, portable, games);
        }
    }
    for root in &reg.portables {
        let inst = Instance::portable(root.clone());
        let Some(m) = inst.read_manifest() else { continue };
        if let Some(i) = games.iter().position(|g| g.def.id == m.game_id) {
            push(inst, i, true, games);
        }
    }
    for (i, g) in games.iter().enumerate() {
        push(Instance::global(g.def.id), i, false, games);
    }
    out
}

/// Record that an instance was just opened, so the next start - and the
/// browser-spawned `eidos nxm` handler - land here rather than on the
/// XDG-global default. Registry trouble must never block the open itself.
pub(crate) fn remember_open(inst: &Instance, game_id: &str) {
    // NEVER under test: this writes the USER'S registry file, and a test open
    // must not rewire which instance their next real session lands on.
    if cfg!(test) {
        return;
    }
    let mut reg = eidos_instance::Registry::load();
    let r = if inst.root == Instance::global(game_id).root {
        eidos_instance::InstanceRef::Global(game_id.to_string())
    } else {
        eidos_instance::InstanceRef::Portable(inst.root.clone())
    };
    reg.set_last(r);
    let _ = reg.save();
}

/// How a spawned `eidos` child should NAME the open instance: the game id for
/// the global one, the folder for a portable one. The id alone sends the child
/// to the XDG-global root no matter which instance is open here - which is
/// exactly how a portable user's Run button used to mount the wrong mods.
pub(crate) fn instance_arg(app: &App) -> Option<String> {
    let id = selected_game(app)?.def.id;
    Some(match &app.created {
        Some(inst) if inst.root != Instance::global(id).root => inst.root.display().to_string(),
        _ => id.to_string(),
    })
}

pub(crate) fn new(launch_command: Vec<String>) -> (App, Task<Message>) {
    // Kept whole for the flag scan below; `identify_game` reads the same list
    // and ignores anything that is not a Proton command.
    let launch_args = launch_command.clone();
    let games = detect(&home());
    // If Steam launched us with the game's command (`eidos-gui %command%`),
    // identify the game and open straight to its instance, like MO2 does.
    let auto = identify_game(&games, &launch_command);
    let mut app = App {
        screen: Screen::Welcome,
        games,
        kind: InstanceKind::Global,
        portable_path: String::new(),
        selected: None,
        name: String::new(),
        created: None,
        error: None,
        mods: Vec::new(),
        plugins: None,
        conflicts: None,
        tab: Tab::Data,
        status: None,
        confirm_clear: false,
        overwrite_to_mod: None,
        send_priority: None,
        send_separator: None,
        launch_command,
        fomod: None,
        collision: None,
        picker: None,
        tools: Vec::new(),
        tool_choice: None,
        search: String::new(),
        selected_mod: None,
        menu_mod: None,
        menu_plugin: None,
        rename: None,
        meta_cache: HashMap::new(),
        confirm_remove: None,
        info_mod: None,
        info_tab: InfoTab::General,
        notes_edit: String::new(),
        collapsed: HashSet::new(),
        category_filter: None,
        filters: ModFilters::default(),
        filters_open: false,
        settings_open: false,
        settings_tab: SettingsTab::General,
        // Open on first sight: a settings page whose sections are all shut asks
        // the user to click five times before it says anything.
        settings_expanded: SettingsTab::DEFAULT_OPEN.iter().copied().collect(),
        // Prefill the key field from the shared store (the same key `eidos nexus
        // key` writes), so it survives across sessions without a network round trip.
        nexus_account: None,
        nexus_signing_in: false,
        nexus_error: None,
        // NOT `Settings::load()` under test, for two reasons that both bit.
        // Loading binds the real file, so a test dispatching "toggle lock"
        // wrote the developer's own preferences - the symptom, options
        // reverting after a build, looked like anything but a test. And
        // reading it makes every assertion about a default depend on what
        // happens to be in that person's file. A default has no path, so
        // saving it is a no-op.
        prefs: if cfg!(test) { Settings::default() } else { Settings::load() },
        executables: None,
        backups: None,
        dropped: Vec::new(),
        files_hovering: false,
        download_drag: None,
        install_at: None,
        install_gap: None,
        confirm_set_all: None,
        file_menu_open: false,
        plugin_send_priority: None,
        export: None,
        collection: None,
        instances_open: false,
        registry_path: eidos_instance::registry_path(),
        servers_edit: String::new(),
        tree_rename: None,
        tree_rename_text: String::new(),
        tree_delete_armed: None,
        tree_new_folder: None,
        confirm_restore: None,
        dl_filter: String::new(),
        dl_sort: DownloadSort::default(),
        dl_show_hidden: false,
        confirm_purge_installed: false,
        // Derived from prefs, but computed HERE rather than after the cfg(test)
        // early return below - a test App whose columns differ from a real one
        // is a test that cannot see a column bug.
        mod_columns: columns_from_settings(&if cfg!(test) {
            Settings::default()
        } else {
            Settings::load()
        }),
        mod_sort: None,
        group_by: None,
        groups_collapsed: std::collections::HashSet::new(),
        instance_rename: None,
        confirm_forget: None,
        confirm_sync: false,
        url_edit: String::new(),
        selected_saves: std::collections::BTreeSet::new(),
        confirm_saves_delete: false,
        saves_fingerprint: None,
        save_shot: None,
        pending_note: None,
        categories_dialog: None,
        ini_editor: None,
        log_pane: None,
        addons: eidos_addons::load_addons(),
        addon_failed: HashMap::new(),
        addon_rejected: eidos_addons::rejected_manifests(),
        addons_open: false,
        addon_findings: Vec::new(),
        endorsing: None,
        endorsed_count: 0,
        updated_count: 0,
        nexus_hourly_left: None,
        nexus_daily_left: None,
        update_in_progress: false,
        sorting: false,
        ui_toolbar_visible: true,
        ui_statusbar_visible: true,
        view_menu_open: false,
        about_open: false,
        saves: Vec::new(),
        confirm_delete_save: None,
        selected_save: None,
        save_info: None,
        save_missing: Vec::new(),
        downloads: Vec::new(),
        resuming: None,
        confirm_delete_download: None,
        identifying_download: None,
        download_samples: HashMap::new(),
        selected_mods: HashSet::new(),
        sel_anchor: None,
        confirm_batch_remove: false,
        modifiers: iced::keyboard::Modifiers::default(),
        drag_state: None,
        drag_scroll: None,
        drag_hover_group: None,
        drag_scroll_depth: 0.5,
        selected_plugin: None,
        selected_plugins: HashSet::new(),
        plugin_anchor: None,
        focus: Pane::Mods,
        categories: None,
        cursor: iced::Point::ORIGIN,
        window: iced::Size::new(1280.0, 800.0),
        menu_at: None,
        typing: false,
        plugin_drag: None,
        profile_menu: None,
        profile_rename: None,
        profile_copy: None,
        profile_delete_confirm: None,
        running: None,
        cap_missing: !eidos_launch::binary_has_cap_sys_admin(&find_eidos_binary()),
        files_cache: std::cell::RefCell::new(HashMap::new()),
        view_generation: std::cell::Cell::new(0),
        diag: Vec::new(),
        diag_dirty: true,
        diag_stale: std::cell::Cell::new(true),
        data_listing: std::cell::RefCell::new(HashMap::new()),
        profiles_cache: std::cell::RefCell::new(None),
        archives_cache: std::cell::RefCell::new(None),
        data_stack: std::cell::RefCell::new(None),
        data_query: String::new(),
        data_conflicts_only: false,
        data_expanded: HashSet::new(),
        overwrite_expanded: HashSet::new(),
        listing_cache: std::cell::RefCell::new(HashMap::new()),
        loot_report: None,
        loot_meta: None,
        known: Vec::new(),
    };
    // NEVER under test. This opens the REAL instance in the user's home and,
    // through ensure_manifest/ensure_profiles, writes to it - so any test that
    // built an App through `new` was one `mods_changed` away from saving its
    // fixture over a live mod list. That is not hypothetical: a keyboard test
    // whose list was ["a","b","c","d"] wrote exactly that into a real
    // modlist.txt, and the only reason it was noticed is that the user restarted
    // and saw four mods. A test needs an App, not a machine's data.
    if cfg!(test) {
        return (app, Task::none());
    }
    // The welcome screen's "open an existing instance" list (also the in-app
    // switcher, via Restart). Built before the auto-open below so it is ready
    // whichever screen the app lands on.
    app.known = known_instances(&app.games);
    // Which existing instance of a game to open: the registry decides
    // (last-used first, then known portables, then the global path). Before
    // the registry, both branches below probed ONLY `Instance::global`, so a
    // portable instance could never be rediscovered after a restart.
    let reg = eidos_instance::Registry::load();
    // `EIDOS_INSTANCE` pins the instance outright - the same variable the CLI
    // honors, so one Steam launch option works for both faces of Eidos. One
    // sanity rule: under a Steam launch the folder must belong to the game
    // Steam is launching, else the variable is ignored and the normal
    // resolution runs (a stale pin must not mount one game's mods over
    // another).
    let pinned: Option<(usize, Instance)> = std::env::var_os("EIDOS_INSTANCE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .and_then(|root| {
            let inst = Instance::portable(root);
            let m = inst.read_manifest()?;
            let i = app.games.iter().position(|g| g.def.id == m.game_id)?;
            if auto.is_some_and(|auto_i| auto_i != i) {
                return None;
            }
            inst.exists().then_some((i, inst))
        });
    let open_existing = |app: &mut App, i: usize, inst: Instance| {
        let id = app.games[i].def.id;
        // No-op when a manifest exists (every registry-found portable has
        // one); stamps the legacy manifest-less global otherwise.
        let _ = inst.ensure_manifest(id, InstanceKind::Global);
        let _ = inst.ensure_profiles();
        remember_open(&inst, id);
        app.selected = Some(i);
        app.mods = modlist_with_unmanaged(&inst, app.games.get(i));
        app.categories = Some(inst.category_factory());
        app.created = Some(inst);
        app.screen = Screen::Main;
    };
    if let Some((i, inst)) = pinned {
        open_existing(&mut app, i, inst);
        if auto.is_some() {
            app.status =
                Some("Launched from Steam. Click Run to start the game through Eidos.".to_string());
        }
    } else if let Some(i) = auto {
        app.selected = Some(i);
        let found = reg.candidates_for(app.games[i].def.id).into_iter().find(|c| c.exists());
        if let Some(inst) = found {
            open_existing(&mut app, i, inst);
            app.status =
                Some("Launched from Steam. Click Run to start the game through Eidos.".to_string());
        }
    } else if let Some(k) = app.known.first() {
        // Standalone: open the last-used instance if it still exists, else the
        // first known one - `known_instances` puts last-used first and lists
        // only existing roots of detected games, which is exactly the reopen
        // rule. Nothing known lands on the wizard, as before.
        let (i, inst) = (k.game_index, k.inst.clone());
        open_existing(&mut app, i, inst);
    }
    load_tools(&mut app);
    // Conflicts feed the mod-list emblems, so compute them as soon as the
    // instance opens instead of waiting for the Conflicts tab.
    app.conflicts = compute_conflicts(&app);
    refresh_meta_cache(&mut app);
    app.collapsed = load_collapsed(&app);
    // The field shows what is stored, so opening Settings does not present an
    // empty box beside a preference that is actually set.
    app.servers_edit = app.prefs.preferred_servers.join(", ");

    recompute_counts(&mut app);
    // A stored session means the user IS signed in: validate it in the background
    // so the status bar shows the account instead of "not logged in" every session.
    let startup = if eidos_nexus::Nexus::have_credentials() {
        Task::perform(
            async move { eidos_nexus::Nexus::connect().and_then(|n| n.validate()) },
            Message::NexusSignInResult,
        )
    } else {
        Task::none()
    };
    // `eidos-gui --collection <nxm://...>`: the nxm handler hands a collection
    // link here rather than trying to install one itself, because reading it
    // needs the mod list this window holds.
    let collection = launch_args
        .iter()
        .position(|a| a == "--collection")
        .and_then(|i| launch_args.get(i + 1))
        .cloned();
    let startup = match collection {
        Some(link) => Task::batch([startup, Task::done(Message::ShowCollection(link))]),
        None => startup,
    };
    (app, startup)
}

/// Reload the tool list for the open instance (user `tools.ini` + per-game
/// defaults), keeping the current pick when it still exists.
/// The auto-detectable executables for a game (launcher, binary, script extender),
/// from its `GameDef` - fed to `default_tools` for MO2-style file-existence detection.
pub(crate) fn game_executables(g: &eidos_games::DetectedGame) -> eidos_instance::GameExecutables<'_> {
    eidos_instance::GameExecutables {
        game_name: g.def.name,
        launcher: g.def.script_extender.as_ref().map(|se| se.launcher),
        binary: Some(g.def.game_binary),
        script_extender: g.def.script_extender.as_ref().map(|se| se.loader),
    }
}

pub(crate) fn load_tools(app: &mut App) {
    let merged = match (selected_game(app), &app.created) {
        (Some(g), Some(inst)) => eidos_instance::merge_tools(
            inst.tools(),
            eidos_instance::default_tools_in(
                game_executables(g),
                &g.install_path,
                &app.created.as_ref().map(|i| i.root_layers()).unwrap_or_default(),
            ),
        ),
        _ => Vec::new(),
    };
    if let Some(t) = &app.tool_choice {
        if !merged.iter().any(|x| x.title.eq_ignore_ascii_case(t)) {
            app.tool_choice = None;
        }
    }
    app.tools = merged;
}

/// Run the whole Nexus OAuth sign-in, blocking: build the PKCE challenge, hand
/// the browser the authorize URL, wait on the loopback listener for the code,
/// exchange it, and store the session.
///
/// Personal API keys are deliberately absent - Nexus requires them gone from a
/// distributed client - so this is the ONLY way Eidos authenticates. It signs
/// in under Eidos's own registered `client_id`; `EIDOS_NEXUS_CLIENT_ID`
/// overrides it for testing against another registration.
pub(crate) fn nexus_sign_in() -> Result<eidos_nexus::Account, String> {
    use eidos_nexus::oauth;
    let cfg = oauth::Config::from_env().ok_or_else(|| {
        "EIDOS_NEXUS_CLIENT_ID is set but empty: unset it to use Eidos's own \
         registration, or give it the client id you want to sign in with"
            .to_string()
    })?;
    let pkce = oauth::Pkce::new().map_err(|e| e.to_string())?;
    let state = oauth::random_token(32).map_err(|e| e.to_string())?;
    let url = oauth::authorize_url(&cfg, &pkce, &state);
    // Hand off to the browser BEFORE listening, so a failure to open is reported
    // as itself rather than as a listener timeout two minutes later.
    std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map_err(|e| format!("could not open your browser: {e}"))?;
    let code = oauth::wait_for_code(cfg.redirect_port, &state, std::time::Duration::from_secs(300))?;
    let tokens = oauth::exchange_code(&cfg, &code, &pkce)?;
    let mut creds = eidos_instance::settings::load_nexus_creds();
    creds.access_token = Some(tokens.access_token.clone());
    creds.refresh_token = Some(tokens.refresh_token);
    creds.expires_at = tokens.expires_at;
    eidos_instance::settings::save_nexus_creds(&creds)
        .map_err(|e| format!("signed in, but could not store the session: {e}"))?;
    // The account comes from the token, NOT from `users/validate`. That endpoint
    // belongs to the personal-API-key era and answers 401 to a Bearer, so calling
    // it here failed every OAuth sign-in at the last step - after the browser had
    // said "connected" and the session had been stored, which is exactly the
    // confusing shape the bug had. Everything Eidos shows about the user is in
    // the token's own claims, signed by Nexus and verified before use.
    let claims = eidos_nexus::oauth::claims(&tokens.access_token)?;
    Ok(eidos_nexus::Account {
        name: claims.username,
        user_id: claims.user_id,
        is_premium: claims.is_premium,
    })
}

/// Build the Executables editor state for the open instance: the user's tools.ini
/// entries (editable) followed by the per-game defaults (read-only). Recomputed
/// every open so a game switch picks up the right script-extender defaults; `None`
/// when no instance is open.
/// The mod-list row of the mod that ships the plugin at `row`, if any.
///
/// The plugin already carries its origin as a NAME, and the mod list is what
/// every mod action is indexed by, so this is the translation between the two.
/// `None` means the game's own Data - which is a real answer, not a failure:
/// vanilla content belongs to no mod and has no folder to open.
pub(crate) fn plugin_origin_row(app: &App, row: usize) -> Option<usize> {
    let list = app.plugins.as_ref()?;
    let origin = list.plugins.get(row).map(|p| p.origin_mod.as_str())?;
    if origin.is_empty() {
        return None;
    }
    app.mods.iter().position(|m| m.name.eq_ignore_ascii_case(origin))
}

/// Read both lists' restore points for the Backups dialog, newest first.
pub(crate) fn load_backups(app: &App) -> Option<BackupsDialogState> {
    let prof = app.created.as_ref()?.active();
    Some(BackupsDialogState {
        mods: prof.backups(eidos_instance::BackupKind::ModList),
        order: prof.backups(eidos_instance::BackupKind::LoadOrder),
    })
}

pub(crate) fn open_executables_dialog(app: &App) -> Option<ExecutablesDialogState> {
    let (game, inst) = (selected_game(app)?, app.created.as_ref()?);
    let user = inst.tools();
    let user_len = user.len();
    // Widened to enabled mods' Root/ dirs, so a script extender installed as a
    // mod is detected; the root union puts it on the game root at launch.
    let roots = app.created.as_ref().map(|i| i.root_layers()).unwrap_or_default();
    let defaults =
        eidos_instance::default_tools_in(game_executables(game), &game.install_path, &roots);
    let merged = eidos_instance::merge_tools(user, defaults);
    let mut state = ExecutablesDialogState {
        merged,
        user_len,
        selected: None,
        title: String::new(),
        exe: String::new(),
        workdir: String::new(),
        args: String::new(),
        prereqs: String::new(),
        output_mod: String::new(),
        // Separators head groups and unmanaged rows are the game's own content -
        // neither is a folder output can be written into. MO2 filters the same
        // set out of this combo.
        mod_names: inst
            .modlist()
            .into_iter()
            .filter(|m| !m.is_separator() && !m.unmanaged)
            .map(|m| m.name)
            .collect(),
    };
    // Select the first user tool, if any, so the editor opens with something.
    if user_len > 0 {
        state.selected = Some(0);
        state.load_buffers();
    }
    Some(state)
}

/// The `eidos tool <instance> run <title>` command: the CLI resolves the tool +
/// Proton and runs it through the merged view (same single-process requirement
/// as `play`). Returned unspawned so `start_run` can route its output to a log.
/// `inst_arg` is [`instance_arg`]'s output: the id or the portable folder.
pub(crate) fn tool_command(inst_arg: &str, title: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(find_eidos_binary());
    cmd.arg("tool").arg(inst_arg).arg("run").arg(title);
    cmd
}

/// Install the tools' runtime prerequisites into the prefix (`eidos prereqs <id>
/// --install`). The Tier-2 winetricks step downloads from Microsoft and can take a
/// while; its output is redirected to `log` (the GUI has no terminal when launched
/// from Steam) so the user can follow progress and read any error.
pub(crate) fn run_prereqs_setup(inst_arg: &str, log: &Path) -> std::io::Result<()> {
    let out = std::fs::File::create(log)?;
    let err = out.try_clone()?;
    std::process::Command::new(find_eidos_binary())
        .arg("prereqs")
        .arg(inst_arg)
        .arg("--install")
        .stdout(std::process::Stdio::from(out))
        .stderr(std::process::Stdio::from(err))
        .spawn()
        .map(|_| ())
}

/// Identify which detected game a Steam `%command%` is launching, by matching
/// each game's install directory against the command's arguments.
pub(crate) fn identify_game(games: &[DetectedGame], command: &[String]) -> Option<usize> {
    for arg in command {
        for (i, g) in games.iter().enumerate() {
            if let Some(dir) = g.data_path.parent() {
                if arg.contains(&*dir.to_string_lossy()) {
                    return Some(i);
                }
            }
        }
    }
    None
}



/// The mod list as the user should see it: the profile's rows, with the game's
/// own content (DLCs, Creation Club) reconciled into them.
///
/// A row the profile already places KEEPS ITS POSITION. That is the whole
/// difference from prepending everything: a user who drags the DLC block under a
/// separator, or puts one above it, has said where it goes, and re-pinning it to
/// the top on the next refresh would throw that away silently.
///
/// Content the profile has never seen is prepended, because the display runs
/// lowest-priority-first and the engine loads its own content before anything
/// anyone installed. Content the profile lists but the game no longer ships is
/// dropped - a DLC can be uninstalled, and a row pointing at nothing helps no one.
pub(crate) fn modlist_with_unmanaged(inst: &Instance, game: Option<&DetectedGame>) -> Vec<ModEntry> {
    let listed = inst.modlist();
    let Some(game) = game else { return strip_unmanaged(listed) };
    let Some(spec) = GameSpec::for_id(game.def.id) else { return strip_unmanaged(listed) };
    // The order the engine imposes on its own content: the primary masters, then
    // whatever the `.ccc` lists. Anything else falls in after, alphabetically.
    let mut engine_order: Vec<String> = spec.primary_plugins.clone();
    engine_order.extend(eidos_plugins::implicit_plugins(&game.install_path));
    let managed: Vec<ModEntry> = listed.iter().filter(|m| !m.unmanaged).cloned().collect();
    let real = inst.unmanaged_mods(&game.data_path, &engine_order, &managed);

    // What the game actually ships, by name, so a listed row can be matched to it
    // and given the path this layer alone knows.
    let mut by_name: std::collections::HashMap<String, ModEntry> =
        real.into_iter().map(|m| (m.name.to_ascii_lowercase(), m)).collect();

    let mut out: Vec<ModEntry> = Vec::with_capacity(listed.len() + by_name.len());
    let mut placed: Vec<ModEntry> = Vec::new();
    for m in listed {
        if !m.unmanaged {
            placed.push(m);
            continue;
        }
        // `remove` both fills in the real path and marks the row as accounted for,
        // so what is left in the map afterwards is exactly the new content.
        if let Some(found) = by_name.remove(&m.name.to_ascii_lowercase()) {
            placed.push(found);
        }
        // Otherwise the game no longer ships it: drop the row.
    }
    // Whatever the profile never mentioned, in engine order, ahead of everything.
    let mut fresh: Vec<ModEntry> = by_name.into_values().collect();
    fresh.sort_by_key(|m| m.name.to_ascii_lowercase());
    out.extend(fresh);
    out.extend(placed);
    out
}

/// The list without the game's content, for when there is no game to reconcile
/// against. A `*` row whose files cannot be located is not a mod and must not be
/// shown as one - least of all with an empty path, which every consumer would
/// then have to defend against.
pub(crate) fn strip_unmanaged(mods: Vec<ModEntry>) -> Vec<ModEntry> {
    mods.into_iter().filter(|m| !m.unmanaged).collect()
}


/// Refresh `app.mods` from disk, unmanaged content included. Clones the instance
/// and the game first so the immutable borrows end before `app.mods` is assigned.
pub(crate) fn reload_mods(app: &mut App) {
    let Some(inst) = app.created.clone() else { return };
    let game = selected_game(app).cloned();
    // This replaces the list the selection indexes into, so it is carried
    // across by name; anything that disappeared is dropped rather than silently
    // re-pointed at whatever took its place.
    let held = hold_mod_selection(app);
    app.mods = modlist_with_unmanaged(&inst, game.as_ref());
    put_mod_selection(app, held);
    // Same moment the list is rebuilt: a category could have been added by an
    // install, and this is the only place that would notice.
    app.categories = Some(inst.category_factory());
    // An open collection pane joins its members against this list, so it goes
    // stale the moment a member is installed. Recomputed here rather than in the
    // view, which runs every frame and this reads every mod's meta.ini.
    crate::update::recompute_collection_states(app);
}

/// Find the `eidos` CLI that drives the namespaced launch. The GUI is
/// multi-threaded, so it cannot enter a user namespace itself; the single-process
/// `eidos` binary can. Prefer a sibling of this binary, then `~/.cargo/bin`, then
/// `PATH`.
pub(crate) fn find_eidos_binary() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let sib = exe.with_file_name("eidos");
        if sib.is_file() {
            return sib;
        }
    }
    let cargo = home().join(".cargo").join("bin").join("eidos");
    if cargo.is_file() {
        return cargo;
    }
    PathBuf::from("eidos")
}

/// Launch the game through Eidos: spawn `eidos play <instance> -- <command>`
/// (with the script-extender swap applied), which mounts the merged mods over
/// the game's Data dir in a private namespace and runs the command through it.
/// Build the `eidos play` command, swapping the vanilla launcher for the script
/// extender's loader - but only if the loader actually exists on disk (a swap to
/// a missing skse64_loader.exe would just make Proton fail cryptically). Returns
/// the command plus a warning to surface when the extender is not installed.
///
/// `game_id` drives the script-extender swap (a real game id, always);
/// `inst_arg` is what the child is told to open ([`instance_arg`]: the id, or
/// the portable folder). Conflating the two sent portable users' launches to
/// the XDG-global instance.
pub(crate) fn play_command(
    game_id: &str,
    inst_arg: &str,
    command: &[String],
) -> (std::process::Command, Option<String>) {
    let mut swapped: Vec<String> = command.to_vec();
    let mut warning = None;
    if let Some((from, prefer)) = launch_targets(game_id) {
        for a in swapped.iter_mut() {
            if !a.contains(from) {
                continue;
            }
            // First target that is actually on disk wins.
            let picked = prefer.iter().find_map(|to| {
                let candidate = a.replace(from, to);
                Path::new(&candidate).is_file().then_some((*to, candidate))
            });
            match picked {
                Some((to, candidate)) => {
                    // Falling back past the script extender is worth saying out
                    // loud: the game will start, and every SKSE mod will be inert.
                    if Some(to) != prefer.first().copied() {
                        warning = Some(format!(
                            "{} is not installed - launching {to} directly, so script-extender mods will not load.",
                            prefer[0]
                        ));
                    }
                    *a = candidate;
                }
                None => {
                    warning = Some(format!(
                        "Neither {} nor the game binary was found next to {from}; launching it unchanged.",
                        prefer.join(" nor ")
                    ));
                }
            }
        }
    }
    let mut cmd = std::process::Command::new(find_eidos_binary());
    cmd.arg("play").arg(inst_arg).arg("--").args(&swapped);
    (cmd, warning)
}

/// What to run INSTEAD of the vanilla Bethesda launcher, best first: the script
/// extender's loader, then the game binary. Returns `(launcher name, preferences)`.
///
/// Steam's `%command%` for these games often points at `<Game>Launcher.exe`, and
/// running that through a mod manager is never what the user wants. It is a
/// separate settings app that re-scans Data and rewrites `plugins.txt`, undoing
/// the load order Eidos just deployed - MO2 runs the game binary or the extender
/// and never the launcher, which is also why Eidos already writes
/// `bEnableFileSelection` to stop the launcher resetting the plugin selection.
/// On top of that the launcher is simply fragile under Proton, where the game
/// itself runs fine.
pub(crate) fn launch_targets(game_id: &str) -> Option<(&'static str, Vec<&'static str>)> {
    let def = eidos_games::GameDef::for_id(game_id)?;
    let se = def.script_extender?;
    Some((se.launcher, vec![se.loader, def.game_binary]))
}

/// Spawn a launch and start tracking it: the child's stdout+stderr go to a
/// per-run log under the instance (the GUI has no terminal when started from
/// Steam), a detached thread `wait()`s it (reaping it, so no zombie) and records
/// its exit status, and the poll subscription refreshes on exit. When `lock_gui`
/// is set the lock overlay also comes up; otherwise the run is tracked without
/// blocking the window.
/// A command as a single copy-pasteable line, quoting only the arguments that
/// need it. Written into the run log so a failing launch can be reproduced by
/// hand in a terminal, where the error the GUI swallows is visible.
pub(crate) fn render_command(cmd: &std::process::Command) -> String {
    let quote = |s: &str| {
        if s.is_empty() || s.contains([' ', '"', '\'', '\\', '$', '`']) {
            format!("'{}'", s.replace('\'', r"'\''"))
        } else {
            s.to_string()
        }
    };
    let mut out = quote(&cmd.get_program().to_string_lossy());
    for a in cmd.get_args() {
        out.push(' ');
        out.push_str(&quote(&a.to_string_lossy()));
    }
    out
}

pub(crate) fn start_run(app: &mut App, title: String, mut cmd: std::process::Command) {
    use std::sync::atomic::Ordering;
    let log = app.created.as_ref().and_then(|inst| {
        let dir = inst.root.join("logs");
        std::fs::create_dir_all(&dir).ok()?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Some(dir.join(format!("run-{stamp}.log")))
    });
    if let Some(p) = &log {
        if let Ok(mut f) = std::fs::File::create(p) {
            use std::io::Write;
            // The command itself, before a byte of its output. Without it a log is
            // only evidence that SOMETHING ran: which executable Eidos picked after
            // the launcher swap is exactly the question these logs get read to
            // answer, and it was the one thing they never recorded.
            let _ = writeln!(f, "# eidos: running {title}");
            let _ = writeln!(f, "# command: {}", render_command(&cmd));
            let _ = writeln!(f, "#");
            if let Ok(out) = f.try_clone() {
                cmd.stdout(std::process::Stdio::from(out));
            }
            cmd.stderr(std::process::Stdio::from(f));
        }
    }
    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            app.status = Some(format!("Launch failed: {e}"));
            return;
        }
    };
    let pid = child.id();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let outcome = std::sync::Arc::new(std::sync::Mutex::new(None));
    let (signal, slot) = (done.clone(), outcome.clone());
    std::thread::spawn(move || {
        let mut child = child;
        let status = child.wait().ok();
        if let Ok(mut s) = slot.lock() {
            *s = status;
        }
        signal.store(true, Ordering::SeqCst);
    });
    let lock = app.prefs.lock_gui;
    app.running = Some(RunningState { title: title.clone(), pid, done, outcome, log, lock });
    app.status = Some(if lock {
        format!("Running {title} - Eidos is locked until it exits (or click Unlock).")
    } else {
        format!("Running {title}...")
    });
}

/// Clear the run lock and refresh the way MO2's `afterRun` does: the game may have
/// rewritten plugins.txt / loadorder.txt while playing, so re-read the mod list,
/// load order and conflicts. Called from the exit poll once the child exits.
/// A non-zero exit is reported with the run log's path so failures are diagnosable.
pub(crate) fn finish_run(app: &mut App) {
    let run = app.running.take();
    reload_mods(app);
    if app.created.is_some() {
        // The session wrote into the Overwrite (and tools may have edited mods).
        drop_files_cache(app, None);
        invalidate_plugins(app);
        app.conflicts = compute_conflicts(app);
        refresh_meta_cache(app);
        recompute_counts(app);
        app.selected_mods.clear();
        app.drag_state = None;
        app.download_drag = None;
        // The run just wrote the script extender's log, which is one of the
        // health checks - and reading it is exactly what the cache defers.
        app.diag_dirty = true;
    }
    if app.created.is_some() {
        // The session may have written new saves; the Saves tab must not go stale
        // exactly when they appear.
        load_saves(app);
    }
    // A rebuild may have wiped the launch capability while we played; re-check so
    // the warning banner is current for the next run.
    app.cap_missing = !eidos_launch::binary_has_cap_sys_admin(&find_eidos_binary());
    let Some(run) = run else {
        app.status = Some("Application exited. Refreshed plugins and load order.".to_string());
        return;
    };
    let status = run.outcome.lock().ok().and_then(|s| *s);
    let failed = status.map(|st| !st.success()).unwrap_or(false);
    // Record how it ended in the log itself. The status bar says it too, but the
    // status bar is gone by the time anyone reads the log - and "exited with 0
    // after one second" versus "killed by SIGSEGV" are completely different
    // problems that looked identical in these files.
    if let Some(p) = &run.log {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(p) {
            let _ = match status {
                Some(st) => match st.code() {
                    Some(c) => writeln!(f, "\n# eidos: {} exited with code {c}", run.title),
                    // No code means a signal killed it; on Unix that is the
                    // interesting case and `ExitStatus` prints which one.
                    None => writeln!(f, "\n# eidos: {} was killed ({st})", run.title),
                },
                None => writeln!(f, "\n# eidos: {} - could not read the exit status", run.title),
            };
        }
    }
    app.status = Some(if failed {
        match &run.log {
            Some(p) => format!(
                "{} exited with an error - see the log: {}",
                run.title,
                p.display()
            ),
            None => format!("{} exited with an error.", run.title),
        }
    } else {
        format!("{} exited. Refreshed plugins and load order.", run.title)
    });
}

pub(crate) fn selected_game(app: &App) -> Option<&DetectedGame> {
    app.selected.and_then(|i| app.games.get(i))
}

/// The mod folder names currently selected in the mod list, lowercased for
/// comparison against a plugin's `origin_mod`.
///
/// MO2 highlights, in the plugin list, the plugins that come from the mod you
/// just clicked - the answer to "which of these 215 rows did THIS mod bring?",
/// which is otherwise a tooltip-by-tooltip hunt. Multi-selection is included
/// because the mod list supports it and a highlight that covered only the
/// anchor row would contradict what the user sees selected.
///
/// Separators contribute nothing: they are dividers, never the origin of a
/// plugin, so a selected separator highlights nothing rather than matching
/// every plugin whose origin happens to be empty.
pub(crate) fn selected_mod_origins(app: &App) -> HashSet<String> {
    app.selected_mods
        .iter()
        .copied()
        .chain(app.selected_mod)
        .filter_map(|i| app.mods.get(i))
        .filter(|m| !m.is_separator())
        .map(|m| m.name.to_ascii_lowercase())
        .collect()
}

/// Whether a plugin was shipped by one of the mods selected in the mod list.
///
/// Case-insensitive because the origin is a folder name and the comparison must
/// not depend on how the archive happened to be cased. An empty origin means
/// the game's own Data, which belongs to no mod and so never highlights.
pub(crate) fn plugin_from_selected_mod(origins: &HashSet<String>, origin_mod: &str) -> bool {
    !origin_mod.is_empty() && origins.contains(&origin_mod.to_ascii_lowercase())
}

pub(crate) fn planned_instance(app: &App) -> Option<Instance> {
    let game = selected_game(app)?;
    Some(match app.kind {
        InstanceKind::Global => Instance::global(game.def.id),
        InstanceKind::Portable => {
            // `~` expanded and relative paths anchored at home, BEFORE anything
            // is created. The wizard's own placeholder suggests "~/Eidos/...",
            // and taken literally that made a directory named `~` wherever the
            // GUI's CWD happened to be (the game folder, under Steam) - with a
            // summary screen echoing the tilde back so it looked right.
            let typed = app.portable_path.trim();
            let root = if typed.is_empty() {
                home().join("Eidos").join(game.def.id)
            } else if typed == "~" {
                home()
            } else if let Some(rest) = typed.strip_prefix("~/") {
                home().join(rest)
            } else {
                let p = PathBuf::from(typed);
                if p.is_absolute() {
                    p
                } else {
                    home().join(p)
                }
            };
            Instance::portable(root)
        }
    })
}

/// Which rows the mod list draws, given the filter and the folded groups.
///
/// Filtering SUSPENDS folding. A search is a question - "which of my mods are
/// called this?" - and a folded group is a display convenience; letting the
/// second silently amputate the answer to the first means the list can show
/// nothing, or worse print "no mods match", while the match sits two rows away
/// inside a group the user folded last week and has forgotten about. That is
/// not a slow answer, it is a wrong one, so a matching mod shows whatever its
/// group is doing.
///
/// A separator then draws only when a mod under it survived the filter, so
/// suspending the fold does not leave a wall of empty headers; with no filter
/// running it always draws, since it is the handle the group folds by.
///
/// `matches` is asked only about real mods - a separator carries no version,
/// no category and no content, and is never a filter subject itself.
pub(crate) fn visible_rows(
    mods: &[ModEntry],
    collapsed: &HashSet<String>,
    filtering: bool,
    matches: impl Fn(usize, &ModEntry) -> bool,
) -> Vec<bool> {
    let mut vis = vec![false; mods.len()];
    let mut folded = false;
    for (i, m) in mods.iter().enumerate() {
        if m.is_separator() {
            folded = !filtering && collapsed.contains(m.display_name());
            vis[i] = !filtering;
            continue;
        }
        vis[i] = !folded && matches(i, m);
    }
    if filtering {
        // Walk back so each separator sees the group it heads, which is every
        // row after it up to the next separator.
        let mut group_has_match = false;
        for i in (0..mods.len()).rev() {
            if mods[i].is_separator() {
                vis[i] = group_has_match;
                group_has_match = false;
            } else if vis[i] {
                group_has_match = true;
            }
        }
    }
    vis
}

/// Ask whether the instance is free, WITHOUT still holding it afterwards.
///
/// `if let Err(e) = inst.try_lock(..)` reads like a test but is not one: the
/// `InstanceLock` it produces is a temporary that lives to the end of the whole
/// `if let` statement, the `else` block included. The rename path then called
/// `switch_to_profile`, which takes the same flock again from a second
/// descriptor - refused, because `LOCK_NB` does not care that the caller is the
/// same process. The profile was renamed and the window kept pointing at a name
/// that no longer existed, saying "Cannot switch profiles".
///
/// Dropping the lock before returning narrows the check to what it always
/// actually was: a courtesy probe, since every write underneath takes its own.
pub(crate) fn probe_lock(inst: &Instance) -> std::io::Result<()> {
    inst.try_lock("the Eidos window").map(drop)
}

/// Whether releasing the mod drag `d` right now would change nothing.
///
/// ONE predicate for two consumers that must never disagree: the view draws
/// the insertion line iff this is false, and `DragDrop` commits iff this is
/// false. Derived separately, they DID disagree - the view hid the line on the
/// grabbed row's own edges for every drag, while the commit only treated those
/// gaps as a no-op for a single row, so a multi-row drag released back on its
/// own row moved (compacted) the block with no line on screen. `aimed` is what
/// keeps a plain click - which arrives as a drop - from ever counting.
pub(crate) fn mod_drop_is_noop(app: &App, d: &DragState) -> bool {
    !d.aimed
        || (selection_or(app, d.from).len() == 1
            && (d.gap == d.from || d.gap == d.from + 1))
}

/// The rows a row-targeted action should act on: the whole multi-selection when
/// the clicked row belongs to it, otherwise just that row.
pub(crate) fn selection_or(app: &App, row: usize) -> Vec<usize> {
    let mut v: Vec<usize> = if app.selected_mods.contains(&row) && app.selected_mods.len() > 1 {
        app.selected_mods.iter().copied().collect()
    } else {
        vec![row]
    };
    // Separators are IN. They used to be filtered out here, on the theory that a
    // separator defines a group rather than sitting in one - which made every
    // reorder gesture a no-op on a separator, since the callers all bail on an
    // empty block. MO2 does the opposite: `ModList::flags` marks a separator
    // `ItemIsDragEnabled` like any other row (modlist.cpp:630), and
    // `dropMimeData` hands the dragged rows to `changeModPriority` untouched
    // (modlist.cpp:1159). Group membership is positional and recomputed after
    // every move, so a separator that moves alone has not abandoned its mods -
    // it now heads whatever follows it, and they belong to the header above them.
    //
    // Actions a separator cannot answer are refused where MO2 refuses them: at
    // the menu entry, on the grounds of the thing being missing (no conflict
    // flags, no checkbox), never on the grounds of being a separator.
    v.retain(|&i| i < app.mods.len());
    v.sort_unstable();
    v
}

/// The rows a separator heads: everything after it up to the next separator.
///
/// Adjacency IS the group - the same rule `visible_rows` walks to decide what a
/// fold hides. There is no parent pointer anywhere, in Eidos or in MO2.
/// The mods a bulk enable/disable would actually touch: what the list is
/// DRAWING, minus separators (no toggle) and the game's own content (not ours to
/// switch off).
///
/// Shared by the menu label and the handler on purpose. Computed separately they
/// would drift, and the drift is the worst kind here - a button that says it will
/// touch 12 mods and touches 400.
pub(crate) fn mods_visible_for_bulk(app: &App) -> Vec<usize> {
    let vis = mod_row_visibility(app, app.categories.as_ref());
    app.mods
        .iter()
        .enumerate()
        .filter(|(i, m)| {
            vis.get(*i).copied().unwrap_or(false) && !m.is_separator() && !m.is_unmanaged()
        })
        .map(|(i, _)| i)
        .collect()
}

pub(crate) fn group_children(mods: &[ModEntry], sep: usize) -> std::ops::Range<usize> {
    let end = mods
        .iter()
        .enumerate()
        .skip(sep + 1)
        .find(|(_, m)| m.is_separator())
        .map(|(i, _)| i)
        .unwrap_or(mods.len());
    (sep + 1).min(end)..end
}

/// The mods a fold is currently hiding, by name, so a move can be compared
/// against what it swallowed.
pub(crate) fn hidden_by_folds(app: &App) -> HashSet<String> {
    let vis = visible_rows(&app.mods, &app.collapsed, false, |_, _| true);
    app.mods
        .iter()
        .zip(&vis)
        .filter(|(m, &shown)| !shown && !m.is_separator())
        .map(|(m, _)| m.name.clone())
        .collect()
}

/// Reconcile the fold state with a move that just happened.
///
/// Two things, both about rows going invisible without being asked to:
///
/// A separator that moved is unfolded if it heads anything at its new position.
/// MO2 does exactly this after a priority change (`ModListView::onModPrioritiesChanged`,
/// modlistview.cpp:449), and it is what makes "a separator moves alone"
/// survivable: a folded header dropped somewhere new would otherwise go on
/// hiding rows that were never inside it, which reads as mods having been deleted.
///
/// The mirror case has no MO2 answer, because MO2's tree at least draws the
/// swallowed rows under a parent: lift a separator out from between a folded
/// group and its own mods, and those mods join the folded group and vanish. The
/// fold is the user's, so it is not overridden - but the disappearance is named,
/// because a row leaving the screen unbidden and unremarked is the failure mode
/// this list is most often accused of.
pub(crate) fn settle_folds_after_move(app: &mut App, at: usize, len: usize, hidden_before: &HashSet<String>) {
    let opened: Vec<String> = (at..(at + len).min(app.mods.len()))
        .filter(|&i| app.mods[i].is_separator())
        .filter(|&i| !group_children(&app.mods, i).is_empty())
        .map(|i| app.mods[i].display_name().to_string())
        .collect();
    let mut changed = false;
    for name in opened {
        changed |= app.collapsed.remove(&name);
    }
    if changed {
        save_collapsed(app);
    }
    let swallowed = hidden_by_folds(app);
    let n = swallowed.difference(hidden_before).count();
    if n > 0 {
        app.status =
            Some(format!("{n} mod(s) are now inside a folded group. Unfold it to see them."));
    }
}

/// The scrollables the keyboard has to move, named so `snap_to` can reach them.
///
/// `scrollable::Id` became the shared `widget::Id` in iced 0.14 - the same type
/// every operation addresses a widget by.
/// The order the mod list is DRAWN in, as indices into `app.mods`.
///
/// In load order this is simply `0..len`, which is what keeps the drag machinery
/// valid: the insertion strips address gaps in the real list, and they only mean
/// anything while the two orders agree.
///
/// Under any other sort, separators come out of the list entirely rather than
/// floating to one end. A separator is a HEADING for the rows beneath it; ordered
/// by author or by date it heads nothing, and leaving one in the middle of a
/// sorted list is a label attached to whatever happened to land under it.
pub(crate) fn display_entries(app: &App) -> Vec<ListEntry> {
    let Some(_) = app.group_by else {
        return display_order(app).into_iter().map(ListEntry::Row).collect();
    };
    // Grouping and the user's separators cannot both be true at once: a
    // separator heads the rows that FOLLOW it in load order, and a grouped list
    // has moved them. Sorted within a group by whatever the sort says, or by
    // load order when it says nothing - which keeps priority readable inside a
    // group, the one place it still means something.
    let order = display_order(app);
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for i in order.into_iter().filter(|&i| !app.mods[i].is_separator()) {
        let label = group_label(app, i);
        match groups.iter_mut().find(|(l, _)| *l == label) {
            Some((_, rows)) => rows.push(i),
            None => groups.push((label, vec![i])),
        }
    }
    // Headers in name order, always - a grouped list whose groups move around
    // as mods are installed is not a grouping anybody can navigate. The
    // catch-all sinks to the bottom whatever it is called.
    groups.sort_by(|a, b| {
        let sink = |l: &str| l == "Uncategorised" || l == "Installed by hand";
        sink(&a.0).cmp(&sink(&b.0)).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
    });
    let mut out = Vec::new();
    for (label, rows) in groups {
        let folded = app.groups_collapsed.contains(&label);
        out.push(ListEntry::Group(label, rows.len()));
        if !folded {
            out.extend(rows.into_iter().map(ListEntry::Row));
        }
    }
    out
}

pub(crate) fn display_order(app: &App) -> Vec<usize> {
    let Some(sort) = app.mod_sort else { return (0..app.mods.len()).collect() };
    let mut rows: Vec<usize> =
        (0..app.mods.len()).filter(|&i| !app.mods[i].is_separator()).collect();
    let meta = |i: usize| app.meta_cache.get(&app.mods[i].name);
    match sort.by {
        SortKey::Name => rows.sort_by_key(|&i| app.mods[i].display_name().to_lowercase()),
        SortKey::Column(ModColumn::Category) => rows.sort_by_key(|&i| {
            meta(i).and_then(|r| r.category_name.clone()).unwrap_or_default().to_lowercase()
        }),
        SortKey::Column(ModColumn::Content) => {
            rows.sort_by_key(|&i| meta(i).map(|r| r.content_tags.clone()).unwrap_or_default())
        }
        SortKey::Column(ModColumn::Version) => {
            rows.sort_by_key(|&i| meta(i).and_then(|r| r.version.clone()).unwrap_or_default())
        }
        SortKey::Column(ModColumn::Author) => rows.sort_by_key(|&i| {
            meta(i).and_then(|r| r.author.clone()).unwrap_or_default().to_lowercase()
        }),
        SortKey::Column(ModColumn::Installed) => rows
            .sort_by_key(|&i| meta(i).and_then(|r| r.installed_at).unwrap_or(UNIX_EPOCH)),
        SortKey::Column(ModColumn::ModId) => {
            rows.sort_by_key(|&i| meta(i).and_then(|r| r.mod_id).unwrap_or(0))
        }
        SortKey::Column(ModColumn::Game) => rows.sort_by_key(|&i| {
            meta(i).and_then(|r| r.game_name.clone()).unwrap_or_default().to_lowercase()
        }),
        // Not offered as a sort - the header does not make it clickable - but
        // the match has to be total, and load order is the honest answer.
        SortKey::Column(ModColumn::Flags) => {}
    }
    if !sort.ascending {
        rows.reverse();
    }
    rows
}

/// The label a mod falls under when the list is grouped.
///
/// Never empty: a row with no category still has to land somewhere, and
/// "Uncategorised" is a group people actually want to find - it is the pile
/// that needs sorting out.
pub(crate) fn group_label(app: &App, i: usize) -> String {
    let meta = app.meta_cache.get(&app.mods[i].name);
    match app.group_by {
        Some(GroupBy::Category) => meta
            .and_then(|r| r.category_name.clone())
            .filter(|c| !c.trim().is_empty())
            .unwrap_or_else(|| "Uncategorised".to_string()),
        Some(GroupBy::Source) => match meta.and_then(|r| r.mod_id) {
            Some(_) => "From Nexus".to_string(),
            None => "Installed by hand".to_string(),
        },
        None => String::new(),
    }
}

/// One entry of the drawn list: a real row, or a header the grouping invented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ListEntry {
    /// An index into `app.mods`.
    Row(usize),
    /// A synthetic group header: its label, and how many rows it heads. It has
    /// no index because there is nothing in `mods` behind it - which is also
    /// why it can be neither dragged, renamed, coloured nor removed.
    Group(String, usize),
}

/// Which columns to draw, from what the user saved.
///
/// The default - never chosen - is the four MO2 shows out of the box, not all
/// eight: a list with every column on has no room left for the name, which is
/// the column somebody is actually reading.
pub(crate) fn columns_from_settings(prefs: &Settings) -> Vec<ModColumn> {
    match &prefs.mod_columns {
        Some(keys) => {
            // Drawn in ModColumn::ALL order regardless of how the file lists
            // them, so a hand-edited settings file cannot produce a header that
            // disagrees with the rows.
            ModColumn::ALL
                .into_iter()
                .filter(|c| keys.iter().any(|k| k == c.key()))
                .collect()
        }
        None => vec![
            ModColumn::Category,
            ModColumn::Content,
            ModColumn::Version,
            ModColumn::Flags,
        ],
    }
}

/// The filter box, so Ctrl+F can put the caret in it.
pub(crate) fn filter_input_id() -> widget::Id {
    widget::Id::new("mod-filter")
}
pub(crate) fn mod_scroll_id() -> widget::Id {
    widget::Id::new("mod-list")
}
pub(crate) fn plugin_scroll_id() -> widget::Id {
    widget::Id::new("plugin-list")
}

/// Bring the row at visible position `pos` of `total` into view.
///
/// Without this the arrow keys move a highlight the user cannot see: past the
/// bottom of a hundred-row list the focus is real, the selection is real, and
/// nothing on screen changes. iced has no "scroll this row into view", so the
/// list is scrolled proportionally - the focused row ends up roughly a third
/// down the viewport, which keeps its neighbours visible in both directions.
pub(crate) fn scroll_focus_into_view(id: widget::Id, pos: usize, total: usize) -> Task<Message> {
    if total <= 1 {
        return Task::none();
    }
    let frac = (pos as f32 / (total - 1) as f32).clamp(0.0, 1.0);
    // The offset is per-axis optional in 0.14, so `x: None` says "leave the
    // horizontal scroll where the user put it" instead of yanking it back to 0
    // on every arrow key - which is what passing 0.0 used to do.
    operation::snap_to(id, operation::RelativeOffset { x: None, y: Some(frac) })
}

/// One criterion of the mod-list filter, and how it is currently set.
///
/// MO2's filter pane is what makes a five-hundred-mod list navigable: it slices
/// by STATE, not just by name. Each row cycles through three settings rather
/// than two, because "only conflicted mods" and "everything except conflicted
/// mods" are both questions people ask, and a plain checkbox can only ask one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Criterion {
    /// Not filtering on this at all.
    #[default]
    Off,
    /// Keep only the rows that match.
    Require,
    /// Keep only the rows that do NOT match.
    Exclude,
}

impl Criterion {
    /// The next setting when the row is clicked: off -> require -> exclude.
    pub(crate) fn next(self) -> Criterion {
        match self {
            Criterion::Off => Criterion::Require,
            Criterion::Require => Criterion::Exclude,
            Criterion::Exclude => Criterion::Off,
        }
    }

    /// Whether a row whose answer to this criterion is `has` survives it.
    pub(crate) fn keeps(self, has: bool) -> bool {
        match self {
            Criterion::Off => true,
            Criterion::Require => has,
            Criterion::Exclude => !has,
        }
    }

    pub(crate) fn mark(self) -> &'static str {
        match self {
            Criterion::Off => "[ ]",
            Criterion::Require => "[+]",
            Criterion::Exclude => "[-]",
        }
    }
}

/// Which state filters the mod list is applying. All of them combine with AND,
/// and with the name box and the category dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ModFilters {
    pub active: Criterion,
    pub conflicted: Criterion,
    pub update: Criterion,
    pub plugins: Criterion,
    pub uncategorised: Criterion,
}

impl ModFilters {
    /// Every criterion, as (label, current setting, the message that cycles it).
    /// One list so the panel and the count cannot drift apart.
    pub(crate) fn rows(self) -> [(&'static str, Criterion, FilterField); 5] {
        [
            ("Active", self.active, FilterField::Active),
            ("Conflicted", self.conflicted, FilterField::Conflicted),
            ("Update available", self.update, FilterField::Update),
            ("Has plugins", self.plugins, FilterField::Plugins),
            ("No category", self.uncategorised, FilterField::Uncategorised),
        ]
    }

    pub(crate) fn get(self, f: FilterField) -> Criterion {
        match f {
            FilterField::Active => self.active,
            FilterField::Conflicted => self.conflicted,
            FilterField::Update => self.update,
            FilterField::Plugins => self.plugins,
            FilterField::Uncategorised => self.uncategorised,
        }
    }

    pub(crate) fn set(&mut self, f: FilterField, c: Criterion) {
        match f {
            FilterField::Active => self.active = c,
            FilterField::Conflicted => self.conflicted = c,
            FilterField::Update => self.update = c,
            FilterField::Plugins => self.plugins = c,
            FilterField::Uncategorised => self.uncategorised = c,
        }
    }

    /// How many criteria are actually filtering, for the button's badge.
    pub(crate) fn active_count(self) -> usize {
        self.rows().iter().filter(|(_, c, _)| *c != Criterion::Off).count()
    }

    pub(crate) fn any(self) -> bool {
        self.active_count() > 0
    }
}

/// Whether ANY narrowing is in force: the name box, the category dropdown, or a
/// filter criterion.
///
/// One definition, because it decides two things that must agree: whether
/// `visible_rows` suspends folding, and whether the list may say "no mods
/// match". It was computed twice with two different definitions - the copy in
/// `modlist_pane` never learned about the criteria - so a state filter left
/// folding on (hiding rows the filter had chosen to show) and emptied the list
/// in silence.
pub(crate) fn is_filtering(app: &App) -> bool {
    !app.search.trim().is_empty() || app.category_filter.is_some() || app.filters.any()
}

pub(crate) fn mod_row_visibility(app: &App, cats: Option<&eidos_instance::CategoryFactory>) -> Vec<bool> {
    let query = app.search.trim().to_lowercase();
    visible_rows(&app.mods, &app.collapsed, is_filtering(app), |i, m| {
        if !query.is_empty() && !m.display_name().to_lowercase().contains(&query) {
            return false;
        }
        let meta = app.meta_cache.get(&m.name);
        let by_category = match app.category_filter {
            None => true,
            Some(fid) => meta
                .and_then(|r| r.category_id)
                .zip(cats)
                .is_some_and(|(cid, cf)| cf.is_descendant_of(cid, fid)),
        };
        if !by_category {
            return false;
        }
        let f = app.filters;
        // Each answer is read from what the list already computes - the whole
        // point of the pane is that none of this needs new bookkeeping.
        f.active.keeps(m.enabled)
            && f.conflicted.keeps(mod_has_conflict(app, i))
            && f.update.keeps(meta.is_some_and(|r| r.update))
            && f.plugins.keeps(meta.is_some_and(|r| r.content_tags.contains('P')))
            && f.uncategorised.keeps(meta.is_none_or(|r| r.category_id.is_none()))
    })
}

/// Whether this mod contests a file with any other mod.
///
/// Not the same question the row TINTS answer: those are relative to the
/// selected mod ("does it beat the focused one"), while a filter has to hold
/// with nothing selected at all.
pub(crate) fn mod_has_conflict(app: &App, row: usize) -> bool {
    let Some(map) = app.conflicts.as_ref() else { return false };
    // Origins are `index + 1`; 0 is the game's own data.
    map.mods
        .get(&((row + 1) as u32))
        .is_some_and(|c| c.state != eidos_conflicts::ConflictState::None)
}

/// Which list the keyboard actually drives right now.
///
/// `App::focus` remembers the last list the user touched, but the plugin list is
/// only on screen while its tab is - so a focus left there after switching tabs
/// would send the arrow keys somewhere invisible.
/// Whether Eidos manages this game's plugin load order at all.
///
/// The condition every plugin path already bails on, named once: a game with no
/// `GameSpec` has no `plugins.txt` that Eidos writes, so there is no list to show
/// and no tab worth offering. Stellar Blade is the first such game.
pub(crate) fn game_manages_plugins(app: &App) -> bool {
    selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)).is_some()
}

/// Whether this game has plugins AT ALL, managed by Eidos or not.
///
/// Wider than [`game_manages_plugins`]: Oblivion and Morrowind have `.esp` files
/// and a load order, Eidos just does not write it for them. Advice about plugins
/// is worth giving there and meaningless for a game with no plugin system, which
/// is the distinction these two predicates exist to keep apart.
pub(crate) fn game_has_plugins(app: &App) -> bool {
    selected_game(app).is_some_and(|g| g.def.load_order != eidos_games::LoadOrder::None)
}

/// The tab actually in force.
///
/// `app.tab` is remembered across a game switch, so it can name a tab the
/// current game does not have. A tab that is not on screen must not be the one
/// deciding which panel gets drawn.
pub(crate) fn effective_tab(app: &App) -> Tab {
    match app.tab {
        Tab::Plugins if !game_manages_plugins(app) => Tab::Data,
        t => t,
    }
}

pub(crate) fn effective_focus(app: &App) -> Pane {
    match app.focus {
        Pane::Plugins if app.tab == Tab::Plugins && app.plugins.is_some() => Pane::Plugins,
        _ => Pane::Mods,
    }
}

/// Move the focused mod (or the whole selection) beside `neighbour`.
///
/// `neighbour` is the row the user can see next to this one, which under a
/// filter is not the adjacent index - landing one raw place away would look like
/// nothing happened.
pub(crate) fn move_mod_rows(app: &mut App, from: usize, neighbour: usize, up: bool) -> Task<Message> {
    let block = selection_or(app, from);
    if block.is_empty() {
        return Task::none();
    }
    // No floor. This used to clamp every move to below the game's own content,
    // for a reason that was true at the time - those rows were not in modlist.txt,
    // so a mod dropped among them vanished on the next save. They are written now
    // (MO2's `*`), which is what makes a separator above the DLC block possible,
    // and a collapsed block is the only way to put that noise away.
    let dest = if up { neighbour } else { neighbour + 1 };
    let held = hold_mod_selection(app);
    let hidden = hidden_by_folds(app);
    let at = move_block(&mut app.mods, &block, dest);
    put_mod_selection(app, held);
    app.selected_mod = Some(at);
    settle_folds_after_move(app, at, block.len(), &hidden);
    mods_changed(app);
    Task::none()
}

/// The plugin twin. The engine's ordering rules decide whether it happens at
/// all, and say why when they refuse - the same answer a drag gets.
pub(crate) fn move_plugin_rows(app: &mut App, from: usize, neighbour: usize, up: bool) -> Task<Message> {
    let Some(spec) = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id)) else {
        return Task::none();
    };
    let block = plugin_selection_or(app, from);
    if block.is_empty() {
        return Task::none();
    }
    let gap = if up { neighbour } else { neighbour + 1 };
    let held = hold_plugin_selection(app);
    let mut moved = false;
    if let Some(list) = app.plugins.as_mut() {
        moved = list.move_plugins_to(&block, gap, &spec);
        if moved {
            list.refresh(&spec);
        }
    }
    put_plugin_selection(app, held);
    if !moved {
        // Refused by the engine's rules, not by a mis-aimed gesture; say which
        // plugin is in the way rather than looking like a dead key.
        if let Some(r) = app
            .plugins
            .as_ref()
            .and_then(|l| l.block_movable_range(&block, &spec))
            .filter(|r| r.is_stuck(block[0]))
        {
            app.status = Some(pinned_by(&r));
        }
        return Task::none();
    }
    commit_plugin_order(app, &spec);
    Task::none()
}

/// Move the focused row, or act on it. One place, so the two lists cannot drift
/// into answering the same key differently.
pub(crate) fn key_nav(app: &mut App, nav: Nav) -> Task<Message> {
    const PAGE: usize = 10;
    let pane = effective_focus(app);
    // The rows the keyboard may land on: what the list is actually DRAWING.
    // Walking the raw vector would move the focus into rows hidden by the filter
    // or folded into a collapsed group, where the highlight cannot be seen and
    // Space toggles something nobody is looking at.
    let rows: Vec<usize> = match pane {
        Pane::Mods => {
            let vis = mod_row_visibility(app, app.categories.as_ref());
            (0..app.mods.len()).filter(|&i| vis.get(i).copied().unwrap_or(false)).collect()
        }
        Pane::Plugins => (0..app.plugins.as_ref().map(|l| l.plugins.len()).unwrap_or(0)).collect(),
    };
    if rows.is_empty() {
        return Task::none();
    }
    let cur = match pane {
        Pane::Mods => app.selected_mod,
        Pane::Plugins => app.selected_plugin,
    };

    // The actions first: they act on the row, not on where the row is.
    match nav {
        Nav::Toggle => {
            return match (pane, cur) {
                // The batch path already handles "just the focused row" via
                // plugin_selection_or, so one message covers both cases.
                (Pane::Plugins, Some(_)) => {
                    let on = app
                        .selected_plugin
                        .and_then(|i| app.plugins.as_ref()?.plugins.get(i).map(|p| !p.enabled))
                        .unwrap_or(true);
                    update(app, Message::SetSelectedPluginsEnabled(on))
                }
                (Pane::Mods, Some(i)) => update(app, Message::ToggleMod(i)),
                _ => Task::none(),
            };
        }
        Nav::Activate => {
            return match (pane, cur) {
                (Pane::Mods, Some(i)) => update(app, Message::ShowModInfo(i)),
                _ => Task::none(),
            };
        }
        Nav::Remove => {
            // The same two-step guard the row menu uses - and the SECOND press
            // has to be able to finish it, or the promise in the status line is
            // a lie and the key does nothing but light up a button elsewhere.
            return match (pane, cur) {
                (Pane::Mods, Some(i))
                    if app
                        .mods
                        .get(i)
                        .is_some_and(|m| !m.is_unmanaged() && !m.is_separator()) =>
                {
                    if app.confirm_remove == Some(i) {
                        return update(app, Message::ModRemove(i));
                    }
                    let name = app.mods[i].display_name().to_string();
                    app.confirm_remove = Some(i);
                    app.status =
                        Some(format!("Press Delete again to remove '{name}', Escape to cancel."));
                    Task::none()
                }
                _ => Task::none(),
            };
        }
        Nav::ShiftUp | Nav::ShiftDown => {
            let up = matches!(nav, Nav::ShiftUp);
            let Some(i) = cur else { return Task::none() };
            // Land beside the neighbour the user can SEE, not one raw index
            // away: under a filter those differ, and a move whose effect is
            // invisible reads as a key that did nothing.
            let Some(here) = rows.iter().position(|&r| r == i) else { return Task::none() };
            let neighbour = if up {
                if here == 0 {
                    return Task::none();
                }
                rows[here - 1]
            } else {
                match rows.get(here + 1) {
                    Some(&r) => r,
                    None => return Task::none(),
                }
            };
            return match pane {
                Pane::Mods => move_mod_rows(app, i, neighbour, up),
                Pane::Plugins => move_plugin_rows(app, i, neighbour, up),
            };
        }
        _ => {}
    }

    // Movement, in VISIBLE positions rather than raw indices, so a step is one
    // row on screen however many are filtered out between them. With nothing
    // focused yet the first key lands on an end rather than doing nothing, so
    // the list is reachable without ever touching the mouse.
    let last = rows.len() - 1;
    // Where the current focus sits among the visible rows. A focus that is no
    // longer drawn (the filter moved under it) is treated as "before the list",
    // so the next key brings it back onto something visible.
    let at = cur.and_then(|i| rows.iter().position(|&r| r == i));
    let pos = match (at, nav) {
        (None, Nav::Up | Nav::PageUp | Nav::Last) => last,
        (None, _) => 0,
        (Some(p), Nav::Up) => p.saturating_sub(1),
        (Some(p), Nav::Down) => (p + 1).min(last),
        (Some(p), Nav::PageUp) => p.saturating_sub(PAGE),
        (Some(p), Nav::PageDown) => (p + PAGE).min(last),
        (_, Nav::First) => 0,
        (_, Nav::Last) => last,
        (Some(p), _) => p,
    };
    let next = rows[pos];

    // Shift extends from the anchor, exactly as Shift-click does, so the two
    // ways of building a selection agree.
    let extend = app.modifiers.shift();
    match pane {
        Pane::Mods => {
            if extend {
                let t = update(app, Message::SelectModExtend(next));
                return Task::batch([t, scroll_focus_into_view(mod_scroll_id(), pos, rows.len())]);
            }
            app.selected_mod = Some(next);
            app.sel_anchor = Some(next);
            app.selected_mods.clear();
            app.menu_mod = None;
            app.confirm_remove = None;
            scroll_focus_into_view(mod_scroll_id(), pos, rows.len())
        }
        Pane::Plugins => {
            if extend {
                let t = update(app, Message::SelectPluginExtend(next));
                return Task::batch([t, scroll_focus_into_view(plugin_scroll_id(), pos, rows.len())]);
            }
            app.selected_plugin = Some(next);
            app.plugin_anchor = Some(next);
            app.selected_plugins.clear();
            scroll_focus_into_view(plugin_scroll_id(), pos, rows.len())
        }
    }
}

/// The plugin rows an action should act on: the whole selection when the given
/// row belongs to it, otherwise just that row.
///
/// The twin of [`selection_or`], and deliberately the same shape: a batch action
/// and a single-row action must not disagree about what "the rows I am acting
/// on" means, or a right-click would do something different from the menu it
/// opened.
pub(crate) fn plugin_selection_or(app: &App, row: usize) -> Vec<usize> {
    let len = app.plugins.as_ref().map(|l| l.plugins.len()).unwrap_or(0);
    let mut v: Vec<usize> = if app.selected_plugins.contains(&row) && app.selected_plugins.len() > 1
    {
        app.selected_plugins.iter().copied().collect()
    } else {
        vec![row]
    };
    v.retain(|&i| i < len);
    v.sort_unstable();
    v
}

/// Move `targets` (indices into `mods`) so the block lands at `dest`, preserving
/// their relative order. Returns the destination index of the first moved row.
///
/// Removing the sources shifts everything after them down, so a downward move has
/// to compensate; getting this wrong is the classic off-by-one that lands a
/// dragged mod one slot short. Every reorder - drag-drop, send to top/bottom, and
/// the targeted sends - goes through here so the correction exists in one place.
pub(crate) fn move_block(mods: &mut Vec<ModEntry>, targets: &[usize], dest: usize) -> usize {
    let mut idx: Vec<usize> = targets.iter().copied().filter(|&i| i < mods.len()).collect();
    idx.sort_unstable();
    idx.dedup();
    if idx.is_empty() {
        return dest.min(mods.len());
    }
    // How many of the moved rows sat before the destination: the block lands that
    // much earlier once they are lifted out.
    let before = idx.iter().filter(|&&i| i < dest).count();
    let block: Vec<ModEntry> = idx.iter().rev().map(|&i| mods.remove(i)).collect();
    let at = dest.saturating_sub(before).min(mods.len());
    // `block` came out highest-index-first, so re-insert in reverse to restore order.
    for m in block {
        mods.insert(at, m);
    }
    at
}

/// Persist the mod list, surfacing a failure instead of losing it silently (a
/// full disk or permission problem would otherwise revert the user's changes on
/// the next restart with no warning). Returns the error text, if any.
pub(crate) fn save_mods(app: &App) -> Option<String> {
    let inst = app.created.as_ref()?;
    // The cross-process lock: a running `eidos play` holds it for the whole
    // session, so a mid-game edit is refused HERE with a readable reason instead
    // of writing into files the live session owns.
    let _lock = match inst.try_lock("the Eidos window") {
        Ok(l) => l,
        Err(e) => return Some(format!("Not saved: {e}.")),
    };
    inst.save_modlist(&app.mods).err().map(|e| format!("Could not save the mod list: {e}"))
}

/// Invalidate every memoised view listing. Cheap: the listings rebuild lazily on
/// the next redraw that needs them. The stored entries are dropped rather than
/// left to accumulate one stale copy per directory ever viewed.
pub(crate) fn bump_views(app: &App) {
    app.view_generation.set(app.view_generation.get().wrapping_add(1));
    app.data_listing.borrow_mut().clear();
    app.data_stack.borrow_mut().take();
    app.archives_cache.borrow_mut().take();
    app.profiles_cache.borrow_mut().take();
    app.listing_cache.borrow_mut().clear();
    app.diag_stale.set(true);
}

/// Recompute the cached health checks if anything flagged them stale. Called once
/// at the end of `update()`, so a message that changes ten things still pays for
/// one scan - and a message that changes nothing pays for none.
pub(crate) fn refresh_diagnostics(app: &mut App) {
    if !app.diag_stale.get() && !app.diag_dirty {
        return;
    }
    app.diag_stale.set(false);
    app.diag_dirty = false;
    run_diagnose_addons(app);
    app.diag = diagnostics(app);
}

/// The values an add-on's `{placeholders}` are filled from.
///
/// A snapshot, deliberately: an add-on gets values and cannot call back into the
/// window, so it cannot leave the application in a state the application did not
/// choose. Every key is absolute, because an add-on's working directory is its
/// own business.
pub(crate) fn addon_context(app: &App) -> eidos_addons::Context {
    let mut values = std::collections::BTreeMap::new();
    if let Some(inst) = &app.created {
        values.insert("instance".to_string(), inst.root.display().to_string());
        values.insert("mods".to_string(), inst.mods_dir().display().to_string());
        values.insert("downloads".to_string(), inst.downloads_dir().display().to_string());
        values.insert("overwrite".to_string(), inst.overwrite_dir().display().to_string());
        let prof = inst.active();
        values.insert("profile".to_string(), prof.name.clone());
        values.insert("profile_dir".to_string(), prof.dir().display().to_string());
    }
    if let Some(g) = selected_game(app) {
        values.insert("game".to_string(), g.def.id.to_string());
        values.insert("game_name".to_string(), g.def.name.to_string());
        values.insert("install".to_string(), g.install_path.display().to_string());
        values.insert("data".to_string(), g.data_path.display().to_string());
    }
    eidos_addons::Context { values }
}

/// How long a `diagnose` add-on is given before it is killed.
///
/// It runs on the diagnostics refresh, which happens after every message that
/// changes anything, so one that blocks would freeze the window on every click.
const ADDON_DIAGNOSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// Run every applicable `diagnose` add-on and collect what they reported.
fn run_diagnose_addons(app: &mut App) {
    app.addon_findings.clear();
    let Some(game_id) = selected_game(app).map(|g| g.def.id.to_string()) else { return };
    let ctx = addon_context(app);
    let addons: Vec<eidos_addons::Addon> = app
        .addons
        .iter()
        .filter(|a| a.kind == eidos_addons::AddonKind::Diagnose && a.applies_to(&game_id))
        .cloned()
        .collect();
    for a in addons {
        if a.unavailable().is_some() {
            continue;
        }
        // A check that already failed is NOT retried. This runs on the
        // diagnostics refresh, which follows every message that changes
        // anything, so one bad manifest would otherwise cost three seconds of
        // frozen window per click - and several would serialise. Reload clears
        // the memory, which is the button someone reaches for after fixing one.
        if let Some(why) = app.addon_failed.get(&a.id) {
            app.addon_findings.push((
                a.name.clone(),
                eidos_addons::Finding {
                    level: eidos_addons::FindingLevel::Problem,
                    title: format!("The '{}' extension did not run", a.name),
                    detail: format!("{why} It will not be retried until you press Reload."),
                },
            ));
            continue;
        }
        match run_addon_capture(&a, &ctx) {
            Ok(stdout) => {
                for f in eidos_addons::parse_findings(&stdout) {
                    app.addon_findings.push((a.name.clone(), f));
                }
            }
            Err(e) => {
                app.addon_failed.insert(a.id.clone(), e.clone());
                app.addon_findings.push((
                    a.name.clone(),
                    eidos_addons::Finding {
                        level: eidos_addons::FindingLevel::Problem,
                        title: format!("The '{}' extension did not run", a.name),
                        detail: e,
                    },
                ));
            }
        }
    }
}

/// Run an add-on and return its stdout, killing it if it overruns.
///
/// Three things this has to get right, and the obvious implementation gets none
/// of them.
///
/// The pipes must be DRAINED while the child runs, not after it exits: a check
/// that writes more than the 64 KiB pipe buffer blocks in `write` forever, and
/// polling `try_wait` in a loop would never see it finish. It would be killed at
/// the timeout and reported as slow, having actually produced its findings.
///
/// The wait must be on the CHILD, not on the pipes: `wait_with_output` reads to
/// EOF, and a child that spawns its own children hands them the same pipe, so EOF
/// only arrives when the last grandchild exits. A three-second timeout would
/// become unbounded for a shell script that backgrounds anything.
///
/// And the reading must happen off this thread, because this runs on the
/// diagnostics refresh that follows every message.
fn run_addon_capture_inner(
    a: &eidos_addons::Addon,
    ctx: &eidos_addons::Context,
    timeout: std::time::Duration,
) -> Result<String, String> {
    use std::io::Read;

    let mut cmd = std::process::Command::new(&a.exec);
    for arg in &a.args {
        cmd.arg(ctx.expand(arg));
    }
    if !a.workdir.is_empty() {
        cmd.current_dir(ctx.expand(&a.workdir));
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    // One reader thread per pipe, so neither can wedge the other and neither can
    // wedge us. They end when their pipe closes, which happens when the last
    // holder of the write end exits - possibly after we have already returned,
    // which is why the handles are detached rather than joined on the slow path.
    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let stdout_buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let stderr_buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let drain = |pipe: Option<std::process::ChildStdout>, into: std::sync::Arc<std::sync::Mutex<Vec<u8>>>| {
        std::thread::spawn(move || {
            if let Some(mut p) = pipe {
                let mut buf = Vec::new();
                let _ = p.read_to_end(&mut buf);
                if let Ok(mut slot) = into.lock() {
                    *slot = buf;
                }
            }
        })
    };
    let out_thread = drain(out_pipe.take(), stdout_buf.clone());
    // Same shape for stderr; the types differ, so it cannot share the closure.
    let err_slot = stderr_buf.clone();
    let err_thread = std::thread::spawn(move || {
        if let Some(mut p) = err_pipe.take() {
            let mut buf = Vec::new();
            let _ = p.read_to_end(&mut buf);
            if let Ok(mut slot) = err_slot.lock() {
                *slot = buf;
            }
        }
    });

    // Wait on the CHILD only.
    let deadline = std::time::Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("it took longer than {timeout:?} and was stopped"));
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
            Err(e) => return Err(e.to_string()),
        }
    };
    // The child is gone; its own pipe ends are closed, so the readers finish -
    // unless a grandchild still holds them, which is exactly the case the wait
    // above refuses to block on. Bounded join, then take what arrived.
    let joined = out_thread.join().is_ok() && err_thread.join().is_ok();
    let stdout = stdout_buf.lock().map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    let stderr = stderr_buf.lock().map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    let _ = joined;

    if !status.success() && stdout.is_empty() {
        let why = stderr.lines().next().unwrap_or("no output").trim();
        return Err(format!("it exited with {status} ({why})"));
    }
    Ok(stdout)
}

/// [`run_addon_capture_inner`] with the standard timeout.
pub(crate) fn run_addon_capture(
    a: &eidos_addons::Addon,
    ctx: &eidos_addons::Context,
) -> Result<String, String> {
    run_addon_capture_inner(a, ctx, ADDON_DIAGNOSE_TIMEOUT)
}

/// Drop cached per-layer file walks: one layer by name (a mod whose contents
/// just changed), or every layer (`None`) when anything might have moved. Also
/// invalidates the memoised view listings, which derive from the same trees.
pub(crate) fn drop_files_cache(app: &App, layer: Option<&str>) {
    let mut cache = app.files_cache.borrow_mut();
    match layer {
        Some(name) => {
            cache.remove(name);
        }
        None => cache.clear(),
    }
    drop(cache);
    bump_views(app);
}

/// Drop the plugin-order cache - and, when the Plugins tab is open, recompute it
/// immediately so the pane updates in place instead of blanking to the
/// placeholder until the user leaves and re-enters the tab.
/// Everything derived from WHICH PLUGINS ARE ACTIVE.
///
/// The Archives tab answers "will the engine load this BSA", which is a question
/// about the active plugin set; so does the orphan-archive health check. Neither
/// is keyed on the view generation, because toggling a plugin does not move it -
/// and switching tabs does not either. One function, called from every path that
/// can change that set, so a new path cannot silently forget one of them.
pub(crate) fn plugin_state_changed(app: &App) {
    app.archives_cache.borrow_mut().take();
    // The orphan-archive check reads the same set; leaving it would have the
    // Health tab and the Archives tab disagreeing about the same file.
    app.diag_stale.set(true);
}

pub(crate) fn invalidate_plugins(app: &mut App) {
    // The menu holds a raw ROW, and the rebuild below renumbers them: acting on
    // a stale index would hit whichever plugin now sits there. The selection
    // survives because it is carried by name; an open menu cannot be, so it
    // closes.
    app.menu_plugin = None;
    plugin_state_changed(app);
    app.profiles_cache.borrow_mut().take();
    let held = hold_plugin_selection(app);
    app.plugins = None;
    if app.tab == Tab::Plugins && app.created.is_some() {
        app.plugins = compute_plugins(app);
    }
    put_plugin_selection(app, held);
}

/// A selection captured BY NAME so it can survive its list being rebuilt or
/// reordered.
///
/// A selection is a set of indices, and almost everything moves them: a LOOT
/// sort, a drag, an arrow-button move, a refresh, a mod enabled. Left alone the
/// numbers stay in range and simply mean different rows - which is worse than
/// going out of range, because nothing errors: the highlight paints strangers
/// and a batch action writes them to disk.
///
/// The ANCHOR is in here too. It is the one index a Shift extension counts from,
/// so a stale one silently turns a three-row gesture into a twenty-row one.
#[derive(Debug, Clone, Default)]
pub(crate) struct HeldSelection {
    focus: Option<String>,
    anchor: Option<String>,
    set: Vec<String>,
}

/// Capture the plugin selection by name. Pair every call with
/// [`put_plugin_selection`] around whatever moves the rows.
pub(crate) fn hold_plugin_selection(app: &App) -> HeldSelection {
    let Some(list) = app.plugins.as_ref() else { return HeldSelection::default() };
    let name = |i: &usize| list.plugins.get(*i).map(|p| p.name.clone());
    HeldSelection {
        focus: app.selected_plugin.as_ref().and_then(name),
        anchor: app.plugin_anchor.as_ref().and_then(name),
        set: app.selected_plugins.iter().filter_map(name).collect(),
    }
}

/// Put it back on the current list, dropping whatever is no longer there.
pub(crate) fn put_plugin_selection(app: &mut App, held: HeldSelection) {
    let Some(list) = app.plugins.as_ref() else {
        app.selected_plugin = None;
        app.plugin_anchor = None;
        app.selected_plugins.clear();
        return;
    };
    let at = |n: &String| list.plugins.iter().position(|p| p.name.eq_ignore_ascii_case(n));
    app.selected_plugin = held.focus.as_ref().and_then(at);
    app.plugin_anchor = held.anchor.as_ref().and_then(at);
    app.selected_plugins = held.set.iter().filter_map(at).collect();
}

/// The mod-list twin of [`hold_plugin_selection`].
pub(crate) fn hold_mod_selection(app: &App) -> HeldSelection {
    let name = |i: &usize| app.mods.get(*i).map(|m| m.name.clone());
    HeldSelection {
        focus: app.selected_mod.as_ref().and_then(name),
        anchor: app.sel_anchor.as_ref().and_then(name),
        set: app.selected_mods.iter().filter_map(name).collect(),
    }
}

/// The mod-list twin of [`put_plugin_selection`]. Also disarms a pending
/// removal: that guard names its target by index, and confirming it after the
/// list moved would delete whatever slid into the slot.
pub(crate) fn put_mod_selection(app: &mut App, held: HeldSelection) {
    let at = |n: &String| app.mods.iter().position(|m| &m.name == n);
    app.selected_mod = held.focus.as_ref().and_then(at);
    app.sel_anchor = held.anchor.as_ref().and_then(at);
    app.selected_mods = held.set.iter().filter_map(at).collect();
    app.confirm_remove = None;
}

/// Persist the mod list and invalidate everything derived from it (plugin order,
/// conflict emblems, the per-mod metadata cache).
pub(crate) fn mods_changed(app: &mut App) {
    if let Some(err) = save_mods(app) {
        app.status = Some(err);
        // The write was refused (another process owns the instance): the
        // in-memory edit will never reach disk, and leaving it displayed shows
        // the user a state that silently evaporates when they close the window.
        // Disk is the truth; resync the view to it.
        reload_mods(app);
    }
    // The merged view depends on which mods are enabled and in what order, not
    // just on their contents.
    bump_views(app);
    invalidate_plugins(app);
    app.conflicts = compute_conflicts(app);
    refresh_meta_cache(app);
    recompute_counts(app);
}

/// Refresh everything that a hide or unhide inside `mod_name` invalidates: the
/// mod's cached file walk (and with it the Data tree and the hidden-files glyph),
/// the conflict map, and - when a plugin came or went - the load order, since a
/// hidden `.esp` is one the game no longer sees.
pub(crate) fn after_hidden_change(app: &mut App, mod_name: &str, rel: &str) {
    drop_files_cache(app, Some(mod_name));
    app.conflicts = compute_conflicts(app);
    let lower = rel.to_ascii_lowercase();
    if [".esp", ".esm", ".esl"].iter().any(|e| lower.trim_end_matches(".mohidden").ends_with(e)) {
        invalidate_plugins(app);
    }
}

/// Make `name` the active profile and reload all per-profile view state (mod list,
/// plugin/conflict caches, collapsed groups, saves), clearing any transient
/// selection / menu / drag. Shared by the profile switch, copy, rename, and delete
/// flows so they can never drift apart.
/// Returns whether the switch actually happened - callers gate their success
/// toasts on it, or a refused switch got its refusal message overwritten by
/// "Created ..." a millisecond later.
pub(crate) fn switch_to_profile(app: &mut App, name: &str) -> bool {
    if let Some(inst) = &app.created {
        // Same lock as every other mutation: a switch during a run would point
        // the run's post-exit steps at the wrong profile. The flock also covers
        // sessions this window did not start (CLI, Steam direct).
        match inst.try_lock("the Eidos window") {
            Ok(_lock) => {
                let _ = inst.set_active_profile(name);
            }
            Err(e) => {
                app.status = Some(format!("Cannot switch profiles: {e}."));
                return false;
            }
        }
    }
    reload_mods(app);
    invalidate_plugins(app);
    app.conflicts = compute_conflicts(app);
    refresh_meta_cache(app);
    app.collapsed = load_collapsed(app);
    recompute_counts(app);
    app.selected_mod = None;
    app.selected_mods.clear();
    app.drag_state = None;
    app.download_drag = None;
    app.menu_mod = None;
    // The new profile has a different mod list, so a different merged view: the
    // Data tab's layer stack and its per-directory memos both belong to the
    // profile that was just left.
    bump_views(app);
    // Saves are per-profile; drop the cache so the Saves tab reloads.
    app.saves = Vec::new();
    app.confirm_delete_save = None;
    // The LOOT badges were computed against the OLD profile's load order.
    app.loot_meta = None;
    clear_save_selection(app);
    true
}

/// Recompute the profile-row Endorsed / Updated counts (MO2 surfaces these). Only
/// real, enabled mods count; separators and disabled mods do not.
pub(crate) fn recompute_counts(app: &mut App) {
    let mut endorsed = 0usize;
    let mut updated = 0usize;
    if let Some(inst) = &app.created {
        for m in &app.mods {
            if !m.enabled || m.is_separator() {
                continue;
            }
            let meta = inst.mod_meta(&m.name);
            if meta.endorsed() {
                endorsed += 1;
            }
            if meta.update_available() {
                updated += 1;
            }
        }
    }
    app.endorsed_count = endorsed;
    app.updated_count = updated;
}

/// The active profile's collapsed-separators file (MO2 keeps this per-profile, out
/// of `modlist.txt`/`meta.ini` so the load order stays clean).
pub(crate) fn collapsed_path(app: &App) -> Option<PathBuf> {
    app.created.as_ref().map(|inst| inst.active().dir().join("collapsed_separators.txt"))
}

/// Load the collapsed-separator set for the active profile.
pub(crate) fn load_collapsed(app: &App) -> HashSet<String> {
    collapsed_path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}

/// Persist the collapsed-separator set (one display name per line).
pub(crate) fn save_collapsed(app: &App) {
    if let Some(p) = collapsed_path(app) {
        let body: String = app.collapsed.iter().map(|n| format!("{n}\n")).collect();
        let _ = fs::write(p, body);
    }
}
