//! Eidos GUI (iced) - MO2-style wizard + two-pane main window.
//!
//!   Welcome -> Instance type (portable/global) -> Game -> Name/location
//!           -> Summary -> [create] -> Main (MO2-style mod manager)
//!
//! The main window mirrors Mod Organizer 2: menu bar + toolbar + profile row,
//! left = the mod list (enable, priority, reorder) with an Overwrite entry,
//! right = Run + Data/Saves/Downloads tabs, plus a status bar. Colony parchment
//! / burgundy palette. Run with: `cargo run -p eidos-gui`

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use iced::widget::{
    button, container, image, mouse_area, pick_list, scrollable, text, text_input, Column, Row,
    Space, Stack,
};
use iced::{Background, Border, Color, Element, Length, Task, Theme};

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::{Instance, InstanceKind, ModEntry};
use eidos_plugins::{plugins_txt_dir, GameSpec, PluginList};
use eidos_conflicts::{ConflictMap, ConflictState, Layer};

// MO2's own toolbar icons (GPL-3.0, from ModOrganizer2/modorganizer src/resources).
const IC_INSTALL: &[u8] = include_bytes!("../assets/icons/system-installer.png");
const IC_NEXUS: &[u8] = include_bytes!("../assets/icons/internet-web-browser.png");
const IC_CHANGE_GAME: &[u8] = include_bytes!("../assets/icons/switch-instance-icon.png");
const IC_REFRESH: &[u8] = include_bytes!("../assets/icons/view-refresh.png");
const IC_EXECUTABLES: &[u8] = include_bytes!("../assets/icons/function.png");
const IC_TOOLS: &[u8] = include_bytes!("../assets/icons/plugins.png");
const IC_SETTINGS: &[u8] = include_bytes!("../assets/icons/preferences-system.png");
const IC_ENDORSE: &[u8] = include_bytes!("../assets/icons/icon-favorite.png");
const IC_UPDATE: &[u8] = include_bytes!("../assets/icons/system-software-update.png");
const IC_HELP: &[u8] = include_bytes!("../assets/icons/help-browser_32.png");
// MO2's real conflict emblems (modconflicticondelegate: emblem_conflict_*).
const IC_CONFLICT_OVERWRITE: &[u8] = include_bytes!("../assets/icons/conflict-overwrite.png");
const IC_CONFLICT_OVERWRITTEN: &[u8] = include_bytes!("../assets/icons/conflict-overwritten.png");
const IC_CONFLICT_MIXED: &[u8] = include_bytes!("../assets/icons/conflict-mixed.png");
const IC_CONFLICT_REDUNDANT: &[u8] = include_bytes!("../assets/icons/conflict-redundant.png");
const IC_CONFLICT_HIDDEN: &[u8] = include_bytes!("../assets/icons/conflict-hidden.png");
const IC_RUN: &[u8] = include_bytes!("../assets/icons/media-playback-start.png");
const IC_UP: &[u8] = include_bytes!("../assets/icons/go-up.png");
const IC_DOWN: &[u8] = include_bytes!("../assets/icons/go-down.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    Kind,
    Game,
    NameLoc,
    Summary,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Data,
    Plugins,
    Conflicts,
    Overwrite,
    Downloads,
}

/// Tabs of the per-mod information dialog (MO2's modinfodialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InfoTab {
    General,
    Conflicts,
    Filetree,
    Notes,
}

#[derive(Debug, Clone)]
enum Message {
    Next,
    Back,
    PickKind(InstanceKind),
    PickGame(usize),
    NameChanged(String),
    PortableChanged(String),
    Finish,
    Restart,
    ToolPicked(String),
    ToggleMod(usize),
    MoveUp(usize),
    MoveDown(usize),
    SelectTab(Tab),
    SwitchProfile(String),
    NewProfile,
    InstallMod,
    ModPicked(Option<PathBuf>),
    FomodToggle(usize, usize),
    FomodNext,
    FomodBack,
    FomodInstall,
    FomodCancel,
    Run,
    Refresh,
    OpenFolder(PathBuf),
    ClearOverwrite,
    // ---- mod-list interactivity (MO2 right-click + filter) ----
    /// Filter box above the mod list (case-insensitive substring on the name).
    SearchChanged(String),
    /// Left-click a mod row: select it and close any open menu.
    SelectMod(usize),
    /// Right-click a mod row: select it and open its action menu.
    OpenModMenu(usize),
    /// Dismiss the open action menu / rename editor.
    CloseMenu,
    /// Reorder shortcuts (MO2 sendModsToTop / sendModsToBottom).
    ModSendTop(usize),
    ModSendBottom(usize),
    /// Open the mod's folder in the file manager (MO2 openExplorer).
    ModOpenFolder(usize),
    /// Open the mod's Nexus page in the browser (MO2 visitOnNexus).
    ModVisitNexus(usize),
    /// Re-run the installer for this mod (MO2 reinstallMod).
    ModReinstall(usize),
    /// Delete the mod from disk (MO2 removeMods); two-click confirm.
    ModRemove(usize),
    /// Begin renaming a mod (MO2 renameMod); opens an inline editor.
    RenameStart(usize),
    RenameChanged(String),
    RenameCommit,
    /// Enable/disable an ESP/ESM in the Plugins tab, persisting plugins.txt.
    TogglePlugin(usize),
    // ---- per-mod information dialog (MO2 modinfodialog) ----
    ShowModInfo(usize),
    CloseInfo,
    InfoSelectTab(InfoTab),
    NotesChanged(String),
    NotesSave,
    // ---- toolbar ----
    /// Re-open the game picker to switch the managed game (MO2 switch-instance).
    ChangeGame,
    /// Open the current game's Nexus page in the browser.
    OpenNexusGame,
    /// Open the instance's root folder in the file manager.
    OpenInstanceFolder,
    /// Install the modding tools' runtime prerequisites into the prefix
    /// (`eidos prereqs <id> --install`); the Tier-2 verbs download from Microsoft.
    SetupPrereqs,
    Noop,
}

/// An in-progress FOMOD installer: the extracted+parsed archive, the current step,
/// and the user's selection so far.
struct FomodWizard {
    session: eidos_install::FomodSession,
    step: usize,
    selection: eidos_fomod::Selection,
    game_id: String,
    /// Current plugin states, so fileDependency/gameDependency conditions evaluate
    /// against the real setup instead of always reading Missing.
    ctx: eidos_fomod::Context,
}

struct App {
    screen: Screen,
    games: Vec<DetectedGame>,
    kind: InstanceKind,
    portable_path: String,
    selected: Option<usize>,
    name: String,
    created: Option<Instance>,
    error: Option<String>,
    mods: Vec<ModEntry>,
    /// Cached ESP/ESM load order for the Plugins tab (recomputed on demand).
    plugins: Option<PluginList>,
    /// Cached per-file conflict analysis for the Conflicts tab + mod-row flags.
    conflicts: Option<ConflictMap>,
    tab: Tab,
    status: Option<String>,
    /// Two-click guard for the destructive "Clear Overwrite" action.
    confirm_clear: bool,
    /// The Proton command Steam passed via `%command%` (empty if launched
    /// standalone). The Run button launches the game through this.
    launch_command: Vec<String>,
    /// An open FOMOD installer wizard, if the user is mid-install.
    fomod: Option<FomodWizard>,
    /// Tools runnable through the merged view (user tools.ini + per-game
    /// defaults), shown in the run-target picker next to Run.
    tools: Vec<eidos_instance::Tool>,
    /// The picked run target: `None` = the game, `Some(title)` = that tool.
    tool_choice: Option<String>,
    /// Mod-list filter query (case-insensitive substring on the mod name).
    search: String,
    /// The highlighted mod row, if any.
    selected_mod: Option<usize>,
    /// The mod whose right-click action menu is open (None = closed).
    menu_mod: Option<usize>,
    /// In-progress rename: `(mod index, edited name)`.
    rename: Option<(usize, String)>,
    /// Per-mod metadata for the extra columns + context menu, keyed by folder name.
    meta_cache: HashMap<String, RowMeta>,
    /// Two-click guard for the destructive per-mod "Remove" action.
    confirm_remove: Option<usize>,
    /// The mod whose info dialog is open (None = closed), its active tab, and the
    /// note text being edited.
    info_mod: Option<usize>,
    info_tab: InfoTab,
    notes_edit: String,
}

/// The slice of a mod's `meta.ini` the main window shows (extra columns + the
/// Nexus action). Cached so a search keystroke doesn't re-read every file.
#[derive(Debug, Clone, Default)]
struct RowMeta {
    version: Option<String>,
    mod_id: Option<u64>,
    category: Option<String>,
    update: bool,
}

/// Build the per-mod metadata cache for the open instance's mod list.
fn build_meta_cache(app: &App) -> HashMap<String, RowMeta> {
    let mut out = HashMap::new();
    if let Some(inst) = &app.created {
        for m in &app.mods {
            let meta = inst.mod_meta(&m.name);
            out.insert(
                m.name.clone(),
                RowMeta {
                    version: meta.version(),
                    mod_id: meta.mod_id(),
                    category: meta.category(),
                    update: meta.update_available(),
                },
            );
        }
    }
    out
}

/// The run-target picker entry meaning "the game itself".
const RUN_GAME: &str = "Game (Steam command)";

fn new(launch_command: Vec<String>) -> (App, Task<Message>) {
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
        launch_command,
        fomod: None,
        tools: Vec::new(),
        tool_choice: None,
        search: String::new(),
        selected_mod: None,
        menu_mod: None,
        rename: None,
        meta_cache: HashMap::new(),
        confirm_remove: None,
        info_mod: None,
        info_tab: InfoTab::General,
        notes_edit: String::new(),
    };
    if let Some(i) = auto {
        app.selected = Some(i);
        let inst = Instance::global(app.games[i].def.id);
        if inst.exists() {
            let _ = inst.ensure_manifest(app.games[i].def.id, InstanceKind::Global);
            let _ = inst.ensure_profiles();
            app.mods = inst.modlist();
            app.created = Some(inst);
            app.screen = Screen::Main;
            app.status =
                Some("Launched from Steam. Click Run to start the game through Eidos.".to_string());
        }
    } else {
        // Standalone: open the first detected game that already has an instance,
        // so `eidos-gui` lands on your existing setup instead of the wizard.
        for (i, g) in app.games.iter().enumerate() {
            let inst = Instance::global(g.def.id);
            if inst.exists() {
                let _ = inst.ensure_manifest(g.def.id, InstanceKind::Global);
                let _ = inst.ensure_profiles();
                app.selected = Some(i);
                app.mods = inst.modlist();
                app.created = Some(inst);
                app.screen = Screen::Main;
                break;
            }
        }
    }
    load_tools(&mut app);
    // Conflicts feed the mod-list emblems, so compute them as soon as the
    // instance opens instead of waiting for the Conflicts tab.
    app.conflicts = compute_conflicts(&app);
    app.meta_cache = build_meta_cache(&app);
    (app, Task::none())
}

/// Reload the tool list for the open instance (user `tools.ini` + per-game
/// defaults), keeping the current pick when it still exists.
fn load_tools(app: &mut App) {
    let merged = match (selected_game(app), &app.created) {
        (Some(g), Some(inst)) => eidos_instance::merge_tools(
            inst.tools(),
            eidos_instance::default_tools(
                g.def.script_extender.as_ref().map(|se| se.loader),
                &g.install_path,
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

/// Spawn `eidos tool <id> run <title>`: the CLI resolves the tool + Proton and
/// runs it through the merged view (same single-process requirement as `play`).
fn run_tool_through_eidos(game_id: &str, title: &str) -> std::io::Result<()> {
    std::process::Command::new(find_eidos_binary())
        .arg("tool")
        .arg(game_id)
        .arg("run")
        .arg(title)
        .spawn()
        .map(|_| ())
}

/// Install the tools' runtime prerequisites into the prefix (`eidos prereqs <id>
/// --install`). The Tier-2 winetricks step downloads from Microsoft and can take a
/// while; its output is redirected to `log` (the GUI has no terminal when launched
/// from Steam) so the user can follow progress and read any error.
fn run_prereqs_setup(game_id: &str, log: &Path) -> std::io::Result<()> {
    let out = std::fs::File::create(log)?;
    let err = out.try_clone()?;
    std::process::Command::new(find_eidos_binary())
        .arg("prereqs")
        .arg(game_id)
        .arg("--install")
        .stdout(std::process::Stdio::from(out))
        .stderr(std::process::Stdio::from(err))
        .spawn()
        .map(|_| ())
}

/// Identify which detected game a Steam `%command%` is launching, by matching
/// each game's install directory against the command's arguments.
fn identify_game(games: &[DetectedGame], command: &[String]) -> Option<usize> {
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

/// The (launcher exe, script-extender loader) swap for a game: launching through
/// Eidos runs the script extender (SKSE/F4SE/...) instead of the vanilla
/// launcher, matching how a modded game is actually played.
fn script_extender_swap(game_id: &str) -> Option<(&'static str, &'static str)> {
    eidos_games::GameDef::for_id(game_id)
        .and_then(|g| g.script_extender)
        .map(|se| (se.launcher, se.loader))
}

/// Find the `eidos` CLI that drives the namespaced launch. The GUI is
/// multi-threaded, so it cannot enter a user namespace itself; the single-process
/// `eidos` binary can. Prefer a sibling of this binary, then `~/.cargo/bin`, then
/// `PATH`.
fn find_eidos_binary() -> PathBuf {
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

/// Launch the game through Eidos: spawn `eidos play <id> -- <command>` (with the
/// script-extender swap applied), which mounts the merged mods over the game's
/// Data dir in a private namespace and runs the command through it.
fn launch_through_eidos(game_id: &str, command: &[String]) -> std::io::Result<()> {
    let mut cmd: Vec<String> = command.to_vec();
    if let Some((from, to)) = script_extender_swap(game_id) {
        for a in cmd.iter_mut() {
            if a.contains(from) {
                *a = a.replace(from, to);
            }
        }
    }
    std::process::Command::new(find_eidos_binary())
        .arg("play")
        .arg(game_id)
        .arg("--")
        .args(&cmd)
        .spawn()?;
    Ok(())
}

fn selected_game(app: &App) -> Option<&DetectedGame> {
    app.selected.and_then(|i| app.games.get(i))
}

fn planned_instance(app: &App) -> Option<Instance> {
    let game = selected_game(app)?;
    Some(match app.kind {
        InstanceKind::Global => Instance::global(game.def.id),
        InstanceKind::Portable => {
            let root = if app.portable_path.trim().is_empty() {
                home().join("Eidos").join(game.def.id)
            } else {
                PathBuf::from(app.portable_path.trim())
            };
            Instance::portable(root)
        }
    })
}

fn save_mods(app: &App) {
    if let Some(inst) = &app.created {
        let _ = inst.save_modlist(&app.mods);
    }
}

/// Persist the mod list and invalidate everything derived from it (plugin order,
/// conflict emblems, the per-mod metadata cache).
fn mods_changed(app: &mut App) {
    save_mods(app);
    app.plugins = None;
    app.conflicts = compute_conflicts(app);
    app.meta_cache = build_meta_cache(app);
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    // Any action other than a second Clear click cancels the clear confirmation.
    if !matches!(message, Message::ClearOverwrite) {
        app.confirm_clear = false;
    }
    match message {
        Message::Next => {
            app.screen = match app.screen {
                Screen::Welcome => Screen::Kind,
                Screen::Kind => Screen::Game,
                Screen::Game => Screen::NameLoc,
                Screen::NameLoc => Screen::Summary,
                other => other,
            };
        }
        Message::Back => {
            app.screen = match app.screen {
                Screen::Kind => Screen::Welcome,
                Screen::Game => Screen::Kind,
                Screen::NameLoc => Screen::Game,
                Screen::Summary => Screen::NameLoc,
                other => other,
            };
        }
        Message::PickKind(k) => app.kind = k,
        Message::PickGame(i) => {
            app.selected = Some(i);
            if app.name.trim().is_empty() {
                if let Some(g) = app.games.get(i) {
                    app.name = g.def.name.to_string();
                }
            }
        }
        Message::NameChanged(s) => app.name = s,
        Message::PortableChanged(s) => app.portable_path = s,
        Message::Finish => {
            if let Some(inst) = planned_instance(app) {
                let game_id = selected_game(app).map(|g| g.def.id.to_string());
                let kind = app.kind;
                match inst.create() {
                    Ok(()) => {
                        if let Some(id) = &game_id {
                            let _ = inst.ensure_manifest(id, kind);
                        }
                        app.mods = inst.modlist();
                        app.created = Some(inst);
                        app.tab = Tab::Data;
                        app.error = None;
                        app.screen = Screen::Main;
                        load_tools(app);
                        app.conflicts = compute_conflicts(app);
                        app.meta_cache = build_meta_cache(app);
                    }
                    Err(e) => app.error = Some(e.to_string()),
                }
            }
        }
        Message::Restart => {
            app.selected = None;
            app.name.clear();
            app.portable_path.clear();
            app.created = None;
            app.error = None;
            app.mods.clear();
            app.status = None;
            app.kind = InstanceKind::Global;
            app.fomod = None;
            app.screen = Screen::Welcome;
        }
        Message::ToggleMod(i) => {
            if let Some(m) = app.mods.get_mut(i) {
                m.enabled = !m.enabled;
            }
            mods_changed(app);
        }
        Message::MoveUp(i) => {
            if i > 0 && i < app.mods.len() {
                app.mods.swap(i - 1, i);
                if app.selected_mod == Some(i) {
                    app.selected_mod = Some(i - 1);
                }
                mods_changed(app);
            }
        }
        Message::MoveDown(i) => {
            if i + 1 < app.mods.len() {
                app.mods.swap(i, i + 1);
                if app.selected_mod == Some(i) {
                    app.selected_mod = Some(i + 1);
                }
                mods_changed(app);
            }
        }
        Message::SelectTab(t) => {
            app.tab = t;
            if t == Tab::Plugins && app.plugins.is_none() {
                app.plugins = compute_plugins(app);
            }
            if t == Tab::Conflicts && app.conflicts.is_none() {
                app.conflicts = compute_conflicts(app);
            }
        }
        Message::SwitchProfile(name) => {
            if let Some(inst) = &app.created {
                let _ = inst.set_active_profile(&name);
                app.mods = inst.profile(&name).modlist();
                app.plugins = None;
                app.conflicts = compute_conflicts(app);
                app.meta_cache = build_meta_cache(app);
                app.selected_mod = None;
                app.menu_mod = None;
                app.status = Some(format!("Switched to profile '{name}'."));
            }
        }
        Message::NewProfile => {
            if let Some(inst) = &app.created {
                let existing = inst.profiles();
                let mut n = existing.len() + 1;
                let mut name = format!("Profile {n}");
                while existing.contains(&name) {
                    n += 1;
                    name = format!("Profile {n}");
                }
                let src = inst.active();
                let dest = inst.profile(&name);
                if dest.create_from(&src).is_ok() {
                    let _ = inst.set_active_profile(&name);
                    app.mods = dest.modlist();
                    app.plugins = None;
                    app.conflicts = compute_conflicts(app);
                    app.meta_cache = build_meta_cache(app);
                    app.selected_mod = None;
                    app.menu_mod = None;
                    app.status = Some(format!("Created '{name}' (copy of '{}').", src.name));
                }
            }
        }
        Message::InstallMod => {
            // Open a native file picker off-thread; the result comes back as ModPicked.
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .add_filter("Mod archives", &["7z", "zip", "rar"])
                    .set_title("Select a mod archive to install")
                    .pick_file(),
                |handle| Message::ModPicked(handle.map(|h| h.path().to_path_buf())),
            );
        }
        Message::ModPicked(picked) => {
            let Some(path) = picked else { return Task::none() };
            let game_id = selected_game(app).map(|g| g.def.id.to_string());
            let mods_dir = app.created.as_ref().map(|i| i.mods_dir());
            let (Some(gid), Some(mods_dir)) = (game_id, mods_dir) else {
                return Task::none();
            };
            let name = eidos_install::mod_name_for(&path);
            match eidos_install::open_fomod(&path, &mods_dir, &name) {
                Ok(Some(session)) => {
                    let enabled_roots: Vec<std::path::PathBuf> =
                        app.mods.iter().filter(|m| m.enabled).map(|m| m.path.clone()).collect();
                    let ctx = match selected_game(app) {
                        Some(g) => eidos_install::fomod_context(&g.data_path, &enabled_roots),
                        None => eidos_fomod::Context::default(),
                    };
                    let selection = eidos_fomod::default_selection(&session.config, &ctx);
                    app.fomod = Some(FomodWizard { session, step: 0, selection, game_id: gid, ctx });
                    app.status = Some("FOMOD installer: choose your options, then Install.".to_string());
                }
                Ok(None) => match eidos_install::install_archive(&path, &mods_dir, &name, &gid) {
                    Ok(r) => after_install(app, &r.name, r.dest, r.fomod),
                    Err(e) => app.status = Some(format!("Install failed: {e}")),
                },
                Err(e) => app.status = Some(format!("Install failed: {e}")),
            }
        }
        Message::FomodToggle(gi, pi) => {
            if let Some(w) = &mut app.fomod {
                let si = w.step;
                let gtype =
                    w.session.config.steps.get(si).and_then(|s| s.groups.get(gi)).map(|g| g.group_type);
                if let (Some(gtype), Some(g)) =
                    (gtype, w.selection.get_mut(si).and_then(|s| s.get_mut(gi)))
                {
                    use eidos_fomod::GroupType::*;
                    match gtype {
                        SelectAll => {}
                        SelectExactlyOne => {
                            g.iter_mut().for_each(|x| *x = false);
                            if let Some(s) = g.get_mut(pi) {
                                *s = true;
                            }
                        }
                        SelectAtMostOne => {
                            let was = g.get(pi).copied().unwrap_or(false);
                            g.iter_mut().for_each(|x| *x = false);
                            if let Some(s) = g.get_mut(pi) {
                                *s = !was;
                            }
                        }
                        _ => {
                            if let Some(s) = g.get_mut(pi) {
                                *s = !*s;
                            }
                        }
                    }
                }
            }
        }
        Message::FomodNext => {
            if let Some(w) = &mut app.fomod {
                let vis = eidos_fomod::visible_steps(
                    &w.session.config,
                    &w.selection,
                    &w.ctx,
                );
                let mut s = w.step + 1;
                while s < vis.len() && !vis[s] {
                    s += 1;
                }
                if s < vis.len() {
                    w.step = s;
                }
            }
        }
        Message::FomodBack => {
            if let Some(w) = &mut app.fomod {
                let vis = eidos_fomod::visible_steps(
                    &w.session.config,
                    &w.selection,
                    &w.ctx,
                );
                let mut s = w.step;
                while s > 0 {
                    s -= 1;
                    if vis.get(s).copied().unwrap_or(true) {
                        w.step = s;
                        break;
                    }
                }
            }
        }
        Message::FomodInstall => {
            if let Some(w) = app.fomod.take() {
                if let Some(mods_dir) = app.created.as_ref().map(|i| i.mods_dir()) {
                    match eidos_install::finish_fomod(w.session, &w.selection, &mods_dir, &w.game_id, &w.ctx) {
                        Ok(r) => after_install(app, &r.name, r.dest, true),
                        Err(e) => app.status = Some(format!("Install failed: {e}")),
                    }
                }
            }
        }
        Message::FomodCancel => {
            app.fomod = None;
            app.status = Some("FOMOD install cancelled.".to_string());
        }
        Message::ToolPicked(choice) => {
            app.tool_choice = (choice != RUN_GAME).then_some(choice);
        }
        Message::Run => {
            if let Some(title) = app.tool_choice.clone() {
                // A tool: the CLI resolves Proton itself, no Steam command needed.
                if let Some(game) = selected_game(app) {
                    let id = game.def.id;
                    match run_tool_through_eidos(id, &title) {
                        Ok(()) => app.status = Some(format!("Launching {title} through the merged view...")),
                        Err(e) => app.status = Some(format!("Tool launch failed: {e}")),
                    }
                } else {
                    app.status = Some("Create or open an instance first.".to_string());
                }
            } else if app.launch_command.is_empty() {
                // Standalone: we don't have Steam's Proton command, so we cannot
                // build the launch environment. Point the user at the option, with
                // this binary's absolute path (Steam's launch options don't see
                // ~/.cargo/bin on PATH) and native d3dcompiler forced so the game's
                // shader compilation works under Proton. Eidos merges that with any
                // mod-shipped DLL overrides at launch.
                let exe = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.to_str().map(str::to_string))
                    .unwrap_or_else(|| "eidos-gui".to_string());
                app.status = Some(format!(
                    "Set the game's Steam launch option to:  WINEDLLOVERRIDES=\"d3dcompiler_47=n\" {exe} %command%  then press Play in Steam (Eidos opens, then click Run)."
                ));
            } else if let (Some(game), Some(_)) = (selected_game(app), &app.created) {
                let id = game.def.id;
                match launch_through_eidos(id, &app.launch_command) {
                    Ok(()) => app.status = Some(format!("Launching {} through Eidos...", game.def.name)),
                    Err(e) => app.status = Some(format!("Launch failed: {e}")),
                }
            } else {
                app.status = Some("Create or open an instance first.".to_string());
            }
        }
        Message::Refresh => {
            if let Some(inst) = &app.created {
                app.mods = inst.modlist();
                app.plugins = None;
                app.conflicts = compute_conflicts(app);
                app.meta_cache = build_meta_cache(app);
                app.status = Some("Refreshed mod list.".to_string());
            }
            load_tools(app);
        }
        Message::OpenFolder(p) => {
            let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
            app.status = Some(format!("Opened {} in your file manager.", p.display()));
        }
        Message::ClearOverwrite => {
            if let Some(inst) = &app.created {
                let dir = inst.overwrite_dir();
                if app.confirm_clear {
                    app.confirm_clear = false;
                    app.status = Some(match clear_dir_contents(&dir) {
                        Ok(()) => "Overwrite cleared.".to_string(),
                        Err(e) => format!("Clear failed: {e}"),
                    });
                } else {
                    app.confirm_clear = true;
                    app.status = Some(
                        "Click Clear again to confirm - this permanently deletes everything the game wrote to the Overwrite (configs, new saves, generated files)."
                            .to_string(),
                    );
                }
            }
        }
        Message::SearchChanged(q) => {
            app.search = q;
            // A filter change can hide the menu's target row; keep it simple.
            app.menu_mod = None;
            app.rename = None;
        }
        Message::SelectMod(i) => {
            app.selected_mod = Some(i);
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
        }
        Message::OpenModMenu(i) => {
            app.selected_mod = Some(i);
            app.menu_mod = Some(i);
            app.rename = None;
            app.confirm_remove = None;
        }
        Message::CloseMenu => {
            app.menu_mod = None;
            app.rename = None;
            app.confirm_remove = None;
        }
        Message::ModSendTop(i) => {
            if i < app.mods.len() {
                let m = app.mods.remove(i);
                app.mods.insert(0, m);
                app.selected_mod = Some(0);
                mods_changed(app);
            }
            app.menu_mod = None;
        }
        Message::ModSendBottom(i) => {
            if i < app.mods.len() {
                let m = app.mods.remove(i);
                app.selected_mod = Some(app.mods.len());
                app.mods.push(m);
                mods_changed(app);
            }
            app.menu_mod = None;
        }
        Message::ModOpenFolder(i) => {
            app.menu_mod = None;
            if let Some(m) = app.mods.get(i) {
                let _ = std::process::Command::new("xdg-open").arg(&m.path).spawn();
                app.status = Some(format!("Opened '{}' in your file manager.", m.name));
            }
        }
        Message::ModVisitNexus(i) => {
            app.menu_mod = None;
            let domain = selected_game(app).map(|g| g.def.nexus_game).filter(|s| !s.is_empty());
            let mod_id = app.mods.get(i).and_then(|m| app.meta_cache.get(&m.name)).and_then(|r| r.mod_id);
            match (domain, mod_id) {
                (Some(domain), Some(id)) => {
                    let url = format!("https://www.nexusmods.com/{domain}/mods/{id}");
                    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                    app.status = Some(format!("Opening {url}"));
                }
                _ => {
                    app.status =
                        Some("No Nexus mod id on record for this mod (install it from Nexus to link it).".to_string());
                }
            }
        }
        Message::ModReinstall(i) => {
            app.menu_mod = None;
            if let Some(m) = app.mods.get(i) {
                app.status = Some(format!(
                    "Reinstalling '{}': pick the archive to install over it.",
                    m.name
                ));
            }
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .add_filter("Mod archives", &["7z", "zip", "rar"])
                    .set_title("Select the archive to reinstall")
                    .pick_file(),
                |handle| Message::ModPicked(handle.map(|h| h.path().to_path_buf())),
            );
        }
        Message::ModRemove(i) => {
            if app.confirm_remove == Some(i) {
                app.confirm_remove = None;
                app.menu_mod = None;
                if let Some(m) = app.mods.get(i).cloned() {
                    match fs::remove_dir_all(&m.path) {
                        Ok(()) => {
                            app.mods.remove(i);
                            app.selected_mod = None;
                            mods_changed(app);
                            app.status = Some(format!("Removed '{}'.", m.name));
                        }
                        Err(e) => app.status = Some(format!("Remove failed: {e}")),
                    }
                }
            } else {
                app.confirm_remove = Some(i);
                if let Some(m) = app.mods.get(i) {
                    app.status =
                        Some(format!("Click Remove again to permanently delete '{}' from disk.", m.name));
                }
            }
        }
        Message::RenameStart(i) => {
            if let Some(m) = app.mods.get(i) {
                app.rename = Some((i, m.name.clone()));
                app.menu_mod = Some(i);
                app.confirm_remove = None;
            }
        }
        Message::RenameChanged(s) => {
            if let Some((_, name)) = &mut app.rename {
                *name = s;
            }
        }
        Message::RenameCommit => {
            if let Some((i, new_name)) = app.rename.take() {
                app.menu_mod = None;
                let new_name = new_name.trim().to_string();
                let old = app.mods.get(i).cloned();
                if let Some(old) = old {
                    if new_name.is_empty() || new_name.contains('/') || new_name.contains('\\') {
                        app.status = Some("Invalid mod name.".to_string());
                    } else if new_name == old.name {
                        // no-op
                    } else if let Some(mods_dir) = app.created.as_ref().map(|inst| inst.mods_dir()) {
                        let dest = mods_dir.join(&new_name);
                        if dest.exists() {
                            app.status = Some(format!("A mod named '{new_name}' already exists."));
                        } else {
                            match fs::rename(&old.path, &dest) {
                                Ok(()) => {
                                    if let Some(m) = app.mods.get_mut(i) {
                                        m.name = new_name.clone();
                                        m.path = dest;
                                    }
                                    mods_changed(app);
                                    app.status = Some(format!("Renamed to '{new_name}'."));
                                }
                                Err(e) => app.status = Some(format!("Rename failed: {e}")),
                            }
                        }
                    }
                }
            }
        }
        Message::TogglePlugin(i) => {
            // Compute the spec + prefix dir up front (immutable borrows of `app`)
            // before mutating `app.plugins`.
            let spec = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id));
            let prefix = selected_game(app).and_then(|g| g.compatdata.as_ref().map(|cd| cd.join("pfx")));
            let name = app.plugins.as_ref().and_then(|l| l.plugins.get(i)).map(|p| p.name.clone());
            if let (Some(spec), Some(name)) = (spec, name) {
                // Base-game masters are implicit and always loaded; refuse to toggle.
                if spec.primary_plugins.iter().any(|p| p.eq_ignore_ascii_case(&name)) {
                    app.status = Some(format!("{name} is a base-game master and is always loaded."));
                } else if let Some(list) = app.plugins.as_mut() {
                    let now = list.plugins.get(i).map(|p| p.enabled).unwrap_or(false);
                    list.set_enabled(&name, !now);
                    list.refresh(&spec);
                    match prefix.map(|pfx| plugins_txt_dir(&pfx, &spec)) {
                        Some(dir) => match list.write_load_order(&dir, &spec) {
                            Ok(()) => {
                                app.status =
                                    Some(format!("{} {name}.", if now { "Disabled" } else { "Enabled" }));
                            }
                            Err(e) => app.status = Some(format!("Could not write plugins.txt: {e}")),
                        },
                        None => {
                            app.status = Some(
                                "Toggled; it will persist once the game's Proton prefix exists (launch it once)."
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        }
        Message::ChangeGame => {
            // Re-open the game picker; keep detection and any selection.
            app.menu_mod = None;
            app.info_mod = None;
            app.error = None;
            app.screen = Screen::Game;
        }
        Message::OpenNexusGame => {
            let domain = selected_game(app).map(|g| g.def.nexus_game).filter(|s| !s.is_empty());
            let url = match domain {
                Some(d) => format!("https://www.nexusmods.com/{d}"),
                None => "https://www.nexusmods.com".to_string(),
            };
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            app.status = Some(format!("Opening {url}"));
        }
        Message::OpenInstanceFolder => {
            if let Some(inst) = &app.created {
                let _ = std::process::Command::new("xdg-open").arg(&inst.root).spawn();
                app.status = Some(format!("Opened {}", inst.root.display()));
            }
        }
        Message::SetupPrereqs => {
            let id = selected_game(app).map(|g| g.def.id);
            let has_prefix = selected_game(app).and_then(|g| g.compatdata.as_ref()).is_some();
            let log = app.created.as_ref().map(|i| i.root.join("prereqs.log"));
            match (id, log) {
                (Some(_), _) if !has_prefix => {
                    app.status = Some(
                        "Launch the game once through Steam first so its Proton prefix exists, then run Tool Setup."
                            .to_string(),
                    );
                }
                (Some(id), Some(log)) => match run_prereqs_setup(id, &log) {
                    Ok(()) => {
                        app.status = Some(format!(
                            "Installing tool prerequisites: bundled DLLs copy now; .NET/vcrun download via winetricks. Progress + errors -> {}",
                            log.display()
                        ));
                    }
                    Err(e) => app.status = Some(format!("Could not start prereq setup: {e}")),
                },
                _ => app.status = Some("Open a game instance first.".to_string()),
            }
        }
        Message::ShowModInfo(i) => {
            app.menu_mod = None;
            let notes = match (app.created.as_ref(), app.mods.get(i)) {
                (Some(inst), Some(m)) => Some(inst.mod_meta(&m.name).notes().unwrap_or_default()),
                _ => None,
            };
            if let Some(notes) = notes {
                app.notes_edit = notes;
                app.info_mod = Some(i);
                app.info_tab = InfoTab::General;
            }
        }
        Message::CloseInfo => app.info_mod = None,
        Message::InfoSelectTab(t) => app.info_tab = t,
        Message::NotesChanged(s) => app.notes_edit = s,
        Message::NotesSave => {
            let result = match (app.info_mod, app.created.as_ref()) {
                (Some(i), Some(inst)) => app.mods.get(i).map(|m| {
                    let mut meta = inst.mod_meta(&m.name);
                    meta.set_notes(&app.notes_edit);
                    (m.name.clone(), meta.write(&inst.meta_path(&m.name)))
                }),
                _ => None,
            };
            if let Some((name, r)) = result {
                app.status = Some(match r {
                    Ok(()) => format!("Saved notes for '{name}'."),
                    Err(e) => format!("Could not save notes: {e}"),
                });
            }
        }
        Message::Noop => {}
    }
    Task::none()
}

// ---- theme -------------------------------------------------------------------

fn palette() -> iced::theme::Palette {
    iced::theme::Palette {
        background: Color::from_rgb8(0xEC, 0xDF, 0xC2),
        text: Color::from_rgb8(0x2B, 0x20, 0x18),
        primary: Color::from_rgb8(0x7A, 0x1F, 0x2B),
        success: Color::from_rgb8(0x4A, 0x6B, 0x3A),
        danger: Color::from_rgb8(0x8A, 0x2A, 0x2A),
    }
}

fn theme(_app: &App) -> Theme {
    Theme::custom("Eidos".to_string(), palette())
}

fn card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
        border: Border { color: Color::from_rgb8(0x7A, 0x1F, 0x2B), width: 1.5, radius: 8.0.into() },
        ..Default::default()
    }
}

fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
        border: Border { color: Color::from_rgb8(0x7A, 0x1F, 0x2B), width: 1.0, radius: 3.0.into() },
        ..Default::default()
    }
}

fn bar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xE3, 0xD6, 0xB6))),
        border: Border { color: Color::from_rgb8(0xC9, 0xB8, 0x90), width: 1.0, radius: 0.0.into() },
        ..Default::default()
    }
}

/// A flat, combo-box-looking button (bordered light field), for dropdowns.
fn combo_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::from_rgb8(0xF7, 0xF0, 0xDE))),
        text_color: Color::from_rgb8(0x2B, 0x20, 0x18),
        border: Border { color: Color::from_rgb8(0xB8, 0xA5, 0x80), width: 1.0, radius: 3.0.into() },
        shadow: Default::default(),
    }
}

fn row_bg(even: bool) -> Color {
    if even {
        Color::from_rgb8(0xF3, 0xEA, 0xD3)
    } else {
        Color::from_rgb8(0xEA, 0xDD, 0xBF)
    }
}

/// Wrap a row with an alternating background (MO2-style row striping).
fn striped<'a>(content: Element<'a, Message>, even: bool) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .padding(2)
        .style(move |_t: &Theme| container::Style {
            background: Some(Background::Color(row_bg(even))),
            ..Default::default()
        })
        .into()
}

/// The highlight behind the selected mod row.
const SEL_BG: Color = Color::from_rgb(0.812, 0.722, 0.525); // tan, distinct from the stripes

/// A mod-list row background that also reflects selection (MO2's blue highlight,
/// here a parchment-tan so it reads on the burgundy theme).
fn list_row<'a>(content: Element<'a, Message>, even: bool, selected: bool) -> Element<'a, Message> {
    let bg = if selected { SEL_BG } else { row_bg(even) };
    container(content)
        .width(Length::Fill)
        .padding(2)
        .style(move |_t: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            ..Default::default()
        })
        .into()
}

// ---- shared widgets ----------------------------------------------------------

fn nav<'a>(label: &'a str, msg: Option<Message>, primary: bool) -> Element<'a, Message> {
    let mut b = button(text(label).size(13.0)).padding(8);
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    if primary {
        b.style(button::primary).into()
    } else {
        b.style(button::secondary).into()
    }
}

fn tool_btn<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12.0)).padding(6).on_press(msg).style(button::secondary).into()
}

/// A flat, menu/toolbar-style button (no chrome until hovered).
fn flat_btn<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(13.0)).padding(6).on_press(msg).style(button::text).into()
}

/// A combo-box-looking button with a dropdown caret.
fn combo<'a>(label: String, msg: Message) -> Element<'a, Message> {
    button(text(format!("{label}   v")).size(12.0)).padding(6).on_press(msg).style(combo_style).into()
}

fn icon<'a>(bytes: &'static [u8], size: f32) -> Element<'a, Message> {
    image(image::Handle::from_bytes(bytes.to_vec()))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

/// A flat toolbar button: icon + label (MO2's icons-and-text mode).
fn icon_text_btn<'a>(bytes: &'static [u8], label: &'a str, msg: Message) -> Element<'a, Message> {
    let content = Row::new()
        .spacing(5)
        .push(icon(bytes, 16.0))
        .push(text(label).size(12.0));
    button(content).padding(5).on_press(msg).style(button::text).into()
}

/// A flat icon-only button (toolbar right group, row arrows).
fn icon_btn<'a>(bytes: &'static [u8], size: f32, msg: Option<Message>) -> Element<'a, Message> {
    let mut b = button(icon(bytes, size)).padding(3).style(button::text);
    if let Some(m) = msg {
        b = b.on_press(m);
    }
    b.into()
}

// ---- wizard ------------------------------------------------------------------

fn frame<'a>(
    step: &'a str,
    title: &'a str,
    content: Element<'a, Message>,
    back: Option<Message>,
    next_label: &'a str,
    next_msg: Option<Message>,
) -> Element<'a, Message> {
    let header = Column::new()
        .spacing(2)
        .push(text("Eidos").size(26.0))
        .push(text(step).size(12.0));

    let card = container(content).width(Length::Fill).padding(18).style(card_style);

    let footer = Row::new()
        .push(nav("Back", back, false))
        .push(Space::with_width(Length::Fill))
        .push(nav(next_label, next_msg, true));

    Column::new()
        .spacing(16)
        .push(header)
        .push(text(title).size(20.0))
        .push(card)
        .push(Space::with_height(Length::Fill))
        .push(footer)
        .into()
}

fn welcome<'a>() -> Element<'a, Message> {
    let content = Column::new()
        .spacing(10)
        .push(text("Eidos creates an isolated modding setup for your game,").size(15.0))
        .push(text("mounting your mods over the game without touching its files.").size(15.0))
        .push(text("Let's set up an instance.").size(13.0));
    frame("Step 1 of 5", "Welcome", content.into(), None, "Next", Some(Message::Next))
}

fn kind_card<'a>(label: &'a str, desc: &'a str, selected: bool, msg: Message) -> Element<'a, Message> {
    let inner = Column::new()
        .spacing(4)
        .push(text(label).size(16.0))
        .push(text(desc).size(12.0));
    button(inner)
        .width(Length::Fill)
        .padding(12)
        .on_press(msg)
        .style(if selected { button::primary } else { button::secondary })
        .into()
}

fn kind_screen<'a>(app: &App) -> Element<'a, Message> {
    let content = Column::new()
        .spacing(10)
        .push(kind_card(
            "Global",
            "Stored centrally in ~/.local/share/eidos, managed by Eidos. Recommended.",
            app.kind == InstanceKind::Global,
            Message::PickKind(InstanceKind::Global),
        ))
        .push(kind_card(
            "Portable",
            "A self-contained folder you choose. Movable and isolated.",
            app.kind == InstanceKind::Portable,
            Message::PickKind(InstanceKind::Portable),
        ));
    frame("Step 2 of 5", "Instance type", content.into(), Some(Message::Back), "Next", Some(Message::Next))
}

fn game_screen<'a>(app: &App) -> Element<'a, Message> {
    let content: Element<Message> = if app.games.is_empty() {
        Column::new()
            .push(text("No supported games detected.").size(15.0))
            .push(text("Install a supported game via Steam, then restart Eidos.").size(12.0))
            .into()
    } else {
        let mut list = Column::new().spacing(6);
        for (i, g) in app.games.iter().enumerate() {
            list = list.push(
                button(text(format!("{}  ({})", g.def.name, g.steam_name)).size(14.0))
                    .width(Length::Fill)
                    .padding(10)
                    .on_press(Message::PickGame(i))
                    .style(if app.selected == Some(i) { button::primary } else { button::secondary }),
            );
        }
        scrollable(list).height(Length::Fixed(240.0)).into()
    };
    let next = app.selected.map(|_| Message::Next);
    frame("Step 3 of 5", "Choose the game to mod", content, Some(Message::Back), "Next", next)
}

fn nameloc_screen<'a>(app: &App) -> Element<'a, Message> {
    let mut content = Column::new()
        .spacing(8)
        .push(text("Instance name").size(13.0))
        .push(text_input("My Skyrim setup", &app.name).on_input(Message::NameChanged).padding(8));
    if app.kind == InstanceKind::Portable {
        content = content.push(text("Portable folder").size(13.0)).push(
            text_input("~/Eidos/skyrimse", &app.portable_path).on_input(Message::PortableChanged).padding(8),
        );
    }
    let next = (!app.name.trim().is_empty()).then_some(Message::Next);
    frame("Step 4 of 5", "Name and location", content.into(), Some(Message::Back), "Next", next)
}

fn summary_screen<'a>(app: &App) -> Element<'a, Message> {
    let kind = match app.kind {
        InstanceKind::Global => "Global",
        InstanceKind::Portable => "Portable",
    };
    let game = selected_game(app);
    let location = planned_instance(app).map(|i| i.root.display().to_string()).unwrap_or_default();

    let mut content = Column::new()
        .spacing(8)
        .push(text(format!("Name:     {}", app.name)).size(14.0))
        .push(text(format!("Type:     {kind}")).size(14.0))
        .push(text(format!("Game:     {}", game.map(|g| g.def.name).unwrap_or("(none)"))).size(14.0))
        .push(text(format!("Location: {location}")).size(13.0));
    if let Some(g) = game {
        content = content.push(text(format!("Game data: {}", g.data_path.display())).size(12.0));
    }
    if let Some(err) = &app.error {
        content = content.push(text(format!("Error: {err}")).size(13.0));
    }
    frame("Step 5 of 5", "Review and create", content.into(), Some(Message::Back), "Create instance", Some(Message::Finish))
}

// ---- main window (MO2 layout) ------------------------------------------------

const C_CHECK: Length = Length::Fixed(36.0);
const C_PRIO: Length = Length::Fixed(26.0);
const C_FLAGS: Length = Length::Fixed(46.0);
const C_VERSION: Length = Length::Fixed(64.0);
const C_MOVE: Length = Length::Fixed(70.0);

/// Every file in the Overwrite as `/`-joined paths relative to it (recursive).
fn overwrite_entries(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(root, &p, out);
            } else if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Delete everything inside a directory, keeping the directory itself.
fn clear_dir_contents(dir: &Path) -> std::io::Result<()> {
    for e in fs::read_dir(dir)?.flatten() {
        let p = e.path();
        if p.is_dir() {
            fs::remove_dir_all(&p)?;
        } else {
            fs::remove_file(&p)?;
        }
    }
    Ok(())
}

/// Top-level entries of the merged view: each name, the source providing it
/// (highest-priority enabled mod, or the game data), and whether it's a folder.
fn merged_listing(app: &App) -> Vec<(String, String, bool)> {
    let mut seen = HashSet::new();
    let mut out: Vec<(String, String, bool)> = Vec::new();
    for m in app.mods.iter().filter(|m| m.enabled) {
        if let Ok(rd) = fs::read_dir(&m.path) {
            for e in rd.flatten() {
                if let Ok(name) = e.file_name().into_string() {
                    if seen.insert(name.clone()) {
                        out.push((name, m.name.clone(), e.path().is_dir()));
                    }
                }
            }
        }
    }
    if let Some(g) = selected_game(app) {
        if let Ok(rd) = fs::read_dir(&g.data_path) {
            for e in rd.flatten() {
                if let Ok(name) = e.file_name().into_string() {
                    if seen.insert(name.clone()) {
                        out.push((name, format!("[{}]", g.def.id), e.path().is_dir()));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out
}

fn menu_bar<'a>() -> Element<'a, Message> {
    let row = Row::new()
        .spacing(0)
        .push(flat_btn("File", Message::Noop))
        .push(flat_btn("View", Message::Noop))
        .push(flat_btn("Tools", Message::Noop))
        .push(flat_btn("Run", Message::Noop))
        .push(flat_btn("Help", Message::Noop));
    container(row).width(Length::Fill).padding(1).style(bar_style).into()
}

fn toolbar<'a>() -> Element<'a, Message> {
    let row = Row::new()
        .spacing(2)
        .push(icon_text_btn(IC_INSTALL, "Install Mod", Message::InstallMod))
        .push(icon_text_btn(IC_NEXUS, "Nexus", Message::OpenNexusGame))
        .push(icon_text_btn(IC_CHANGE_GAME, "Change Game", Message::ChangeGame))
        .push(icon_text_btn(IC_REFRESH, "Refresh", Message::Refresh))
        .push(icon_text_btn(IC_EXECUTABLES, "Executables", Message::Noop))
        .push(icon_text_btn(IC_TOOLS, "Tool Setup", Message::SetupPrereqs))
        .push(icon_text_btn(IC_SETTINGS, "Settings", Message::OpenInstanceFolder))
        .push(Space::with_width(Length::Fill))
        .push(icon_btn(IC_ENDORSE, 20.0, Some(Message::Noop)))
        .push(icon_btn(IC_UPDATE, 20.0, Some(Message::Noop)))
        .push(icon_btn(IC_HELP, 20.0, Some(Message::Noop)));
    container(row).width(Length::Fill).padding(2).style(bar_style).into()
}

#[allow(clippy::too_many_arguments)]
fn mod_row<'a>(
    i: usize,
    m: &ModEntry,
    len: usize,
    meta: Option<&RowMeta>,
    flag_icon: Option<&'static [u8]>,
    hidden_icon: Option<&'static [u8]>,
) -> Element<'a, Message> {
    let up = icon_btn(IC_UP, 14.0, (i > 0).then_some(Message::MoveUp(i)));
    let dn = icon_btn(IC_DOWN, 14.0, (i + 1 < len).then_some(Message::MoveDown(i)));
    let toggle = button(text(if m.enabled { "[x]" } else { "[ ]" }).size(12.0))
        .padding(3)
        .on_press(Message::ToggleMod(i))
        .style(button::secondary);

    // MO2's conflict emblem plus an optional hidden-files glyph (a mod can be both).
    let mut flags = Row::new().spacing(2);
    if let Some(bytes) = flag_icon {
        flags = flags.push(icon(bytes, 14.0));
    }
    if let Some(bytes) = hidden_icon {
        flags = flags.push(icon(bytes, 14.0));
    }
    let flag_cell: Element<'a, Message> = container(flags).width(C_FLAGS).into();

    // MO2's Version column; an update marker prefixes it when Nexus has a newer one.
    let version = meta.and_then(|r| r.version.clone()).unwrap_or_default();
    let version = match meta {
        Some(r) if r.update => format!("^ {version}"),
        _ => version,
    };

    let row = Row::new()
        .spacing(6)
        .push(container(toggle).width(C_CHECK))
        .push(text(format!("{:>2}", i + 1)).size(12.0).width(C_PRIO))
        .push(text(m.name.clone()).size(13.0).width(Length::Fill))
        .push(text(version).size(11.0).width(C_VERSION))
        .push(flag_cell)
        .push(Row::new().spacing(2).push(up).push(dn).width(C_MOVE));

    // Left-click selects, right-click opens the action menu (MO2's context menu).
    // Inner buttons still get their own clicks; the mouse_area catches the rest.
    mouse_area(row)
        .on_press(Message::SelectMod(i))
        .on_right_press(Message::OpenModMenu(i))
        .into()
}

fn modlist_pane<'a>(app: &App) -> Element<'a, Message> {
    let active = app.mods.iter().filter(|m| m.enabled).count();
    let active_name = app.created.as_ref().map(|i| i.active_profile()).unwrap_or_default();
    let mut profile = Row::new().spacing(6).push(text("Profile:").size(12.0));
    if let Some(inst) = &app.created {
        for name in inst.profiles() {
            let selected = name == active_name;
            profile = profile.push(
                button(text(name.clone()).size(12.0))
                    .padding(4)
                    .on_press(Message::SwitchProfile(name.clone()))
                    .style(if selected { button::primary } else { button::secondary }),
            );
        }
    }
    let profile = profile
        .push(tool_btn("+ New", Message::NewProfile))
        .push(Space::with_width(Length::Fill))
        .push(text(format!("Active: {active}")).size(12.0));

    // MO2's mod-list filter box.
    let search = text_input("Filter mods by name...", &app.search)
        .on_input(Message::SearchChanged)
        .padding(5)
        .size(12.0);

    let header = Row::new()
        .spacing(6)
        .push(text("").width(C_CHECK))
        .push(text("#").size(11.0).width(C_PRIO))
        .push(text("Mod Name").size(11.0).width(Length::Fill))
        .push(text("Version").size(11.0).width(C_VERSION))
        .push(text("Flags").size(11.0).width(C_FLAGS))
        .push(text("").width(C_MOVE));

    let len = app.mods.len();
    let query = app.search.trim().to_lowercase();
    let mut list = Column::new().spacing(1);
    let mut shown = 0usize;
    if app.mods.is_empty() {
        list = list.push(text("No mods yet. Drop mod folders into the instance's mods/ dir.").size(12.0));
    }
    for (i, m) in app.mods.iter().enumerate() {
        if !query.is_empty() && !m.name.to_lowercase().contains(&query) {
            continue;
        }
        shown += 1;
        // MO2's conflict emblems; a disabled mod shows none (the checkbox says it).
        let flag_icon = if !m.enabled {
            None
        } else if let Some(c) = &app.conflicts {
            match c.state((i + 1) as u32) {
                ConflictState::Overwrites => Some(IC_CONFLICT_OVERWRITE),
                ConflictState::Overwritten => Some(IC_CONFLICT_OVERWRITTEN),
                ConflictState::Mixed => Some(IC_CONFLICT_MIXED),
                ConflictState::Redundant => Some(IC_CONFLICT_REDUNDANT),
                ConflictState::None => None,
            }
        } else {
            None
        };
        // A separate hidden-files glyph (MO2's FLAG_HIDDEN_FILES), shown alongside.
        let hidden_icon = if m.enabled {
            app.conflicts
                .as_ref()
                .and_then(|c| c.mods.get(&((i + 1) as u32)))
                .filter(|mc| mc.has_hidden)
                .map(|_| IC_CONFLICT_HIDDEN)
        } else {
            None
        };
        let meta = app.meta_cache.get(&m.name);
        let selected = app.selected_mod == Some(i);
        list = list.push(list_row(
            mod_row(i, m, len, meta, flag_icon, hidden_icon),
            i % 2 == 0,
            selected,
        ));
    }
    if !app.mods.is_empty() && shown == 0 {
        list = list.push(text(format!("No mods match \"{}\".", app.search.trim())).size(12.0));
    }

    let overwrite = button(
        Row::new()
            .spacing(6)
            .push(text("").width(C_CHECK))
            .push(text("").width(C_PRIO))
            .push(text("Overwrite").size(13.0).width(Length::Fill)),
    )
    .padding(2)
    .on_press(Message::SelectTab(Tab::Overwrite))
    .style(button::text);

    let inner = Column::new()
        .spacing(6)
        .push(profile)
        .push(search)
        .push(header)
        .push(scrollable(list).height(Length::Fill))
        .push(overwrite);

    container(inner).width(Length::FillPortion(3)).height(Length::Fill).padding(8).style(panel_style).into()
}

/// A single left-aligned action in the mod context menu.
fn menu_item<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(msg)
        .style(button::text)
        .into()
}

/// A small separator line inside the context menu.
fn menu_sep<'a>() -> Element<'a, Message> {
    container(Space::new(Length::Fill, Length::Fixed(1.0)))
        .padding([2, 6])
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xC8, 0xB8, 0x95))),
            ..Default::default()
        })
        .into()
}

/// MO2's right-click mod menu, rendered as a floating card (the action set from
/// modlistviewactions.cpp: enable/disable, send-to-top/bottom, explorer, Nexus,
/// reinstall, rename, remove). Shows the rename editor when a rename is in flight.
fn mod_menu_card<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(m) = app.mods.get(i) else {
        return Space::new(Length::Shrink, Length::Shrink).into();
    };

    let title = Row::new()
        .spacing(6)
        .push(text(m.name.clone()).size(13.0).width(Length::Fill))
        .push(
            button(text("x").size(13.0)).padding([1, 6]).on_press(Message::CloseMenu).style(button::text),
        );

    let mut col = Column::new().spacing(1).push(title);

    // A read-only info line (MO2 surfaces version/category/Nexus id on the row).
    if let Some(r) = app.meta_cache.get(&m.name) {
        let mut bits: Vec<String> = Vec::new();
        if let Some(v) = &r.version {
            bits.push(format!("v{v}"));
        }
        if let Some(c) = r.category.as_deref().map(str::trim).filter(|c| !c.is_empty() && *c != "-1,") {
            bits.push(format!("cat {}", c.trim_end_matches(',')));
        }
        if let Some(id) = r.mod_id {
            bits.push(format!("Nexus #{id}"));
        }
        if !bits.is_empty() {
            col = col.push(text(bits.join("  ·  ")).size(10.0));
        }
    }

    col = col.push(menu_sep());

    // Inline rename editor (MO2 renameMod) takes over the card while active.
    if let Some((ri, name)) = &app.rename {
        if *ri == i {
            let editor = text_input("New name", name)
                .on_input(Message::RenameChanged)
                .on_submit(Message::RenameCommit)
                .padding(5)
                .size(12.0);
            let actions = Row::new()
                .spacing(6)
                .push(tool_btn("Save", Message::RenameCommit))
                .push(tool_btn("Cancel", Message::CloseMenu));
            col = col.push(editor).push(actions);
            return menu_frame(col.into());
        }
    }

    col = col
        .push(menu_item("Information...", Message::ShowModInfo(i)))
        .push(menu_sep())
        .push(menu_item(if m.enabled { "Disable" } else { "Enable" }, Message::ToggleMod(i)))
        .push(menu_sep())
        .push(menu_item("Send to Top", Message::ModSendTop(i)))
        .push(menu_item("Send to Bottom", Message::ModSendBottom(i)))
        .push(menu_sep())
        .push(menu_item("Open in Explorer", Message::ModOpenFolder(i)));

    // Visit on Nexus only when we actually have a mod id to link to.
    let has_nexus = app.meta_cache.get(&m.name).and_then(|r| r.mod_id).is_some();
    if has_nexus {
        col = col.push(menu_item("Visit on Nexus", Message::ModVisitNexus(i)));
    }

    col = col
        .push(menu_item("Reinstall Mod", Message::ModReinstall(i)))
        .push(menu_item("Rename", Message::RenameStart(i)))
        .push(menu_sep());

    let remove_label = if app.confirm_remove == Some(i) { "Confirm remove?" } else { "Remove" };
    let remove = button(text(remove_label).size(12.0))
        .width(Length::Fill)
        .padding([4, 8])
        .on_press(Message::ModRemove(i))
        .style(if app.confirm_remove == Some(i) { button::danger } else { button::text });
    col = col.push(remove);

    menu_frame(col.into())
}

/// The bordered card chrome around the context menu's contents.
fn menu_frame<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fixed(210.0))
        .padding(6)
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xF3, 0xEA, 0xD3))),
            border: Border {
                color: Color::from_rgb8(0x6E, 0x24, 0x2E),
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// ---- per-mod information dialog (MO2 modinfodialog) -------------------------

fn info_tab_btn<'a>(label: &'a str, tab: InfoTab, active: bool) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .padding([4, 10])
        .on_press(Message::InfoSelectTab(tab))
        .style(if active { button::primary } else { button::secondary })
        .into()
}

fn info_kv<'a>(k: &'a str, v: String) -> Element<'a, Message> {
    Row::new()
        .spacing(8)
        .push(text(k).size(12.0).width(Length::Fixed(120.0)))
        .push(text(v).size(12.0).width(Length::Fill))
        .into()
}

/// General tab: name/version/category/Nexus id/source/endorsed/tracked + counts.
fn info_general<'a>(app: &App, m: &ModEntry) -> Element<'a, Message> {
    let meta = app.created.as_ref().map(|inst| inst.mod_meta(&m.name));
    let files = overwrite_entries(&m.path).len();
    let mut col = Column::new().spacing(4).push(info_kv("Name", m.name.clone()));
    if let Some(meta) = &meta {
        if let Some(v) = meta.version() {
            col = col.push(info_kv("Version", v));
        }
        if let Some(nv) = meta.newest_version() {
            col = col.push(info_kv("Newest", nv));
        }
        if let Some(c) = meta
            .category()
            .map(|c| c.trim_end_matches(',').trim().to_string())
            .filter(|c| !c.is_empty() && c != "-1")
        {
            col = col.push(info_kv("Category", c));
        }
        if let Some(id) = meta.mod_id() {
            col = col.push(info_kv("Nexus id", id.to_string()));
        }
        if let Some(src) = meta.installation_file() {
            col = col.push(info_kv("Installed from", src));
        }
        col = col
            .push(info_kv("Endorsed", if meta.endorsed() { "yes".into() } else { "no".into() }))
            .push(info_kv("Tracked", if meta.tracked() { "yes".into() } else { "no".into() }));
    }
    col.push(info_kv("Enabled", if m.enabled { "yes".into() } else { "no".into() }))
        .push(info_kv("Files", files.to_string()))
        .push(info_kv("Folder", m.path.display().to_string()))
        .into()
}

/// Conflicts tab: which files this mod overrides, and which it loses, by mod name.
fn info_conflicts<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(cmap) = &app.conflicts else {
        return text("Conflicts not computed yet.").size(12.0).into();
    };
    let origin = (i + 1) as u32;
    let mut wins: Vec<(String, String)> = Vec::new();
    let mut loses: Vec<(String, String)> = Vec::new();
    for node in cmap.files.values() {
        if node.winner == origin && node.is_conflicted() {
            let losers: Vec<&str> =
                node.alternatives.iter().filter(|&&a| a != 0).map(|&a| cmap.name(a)).collect();
            wins.push((node.display_path.clone(), losers.join(", ")));
        } else if node.winner != origin && node.winner != 0 && node.alternatives.contains(&origin) {
            loses.push((node.display_path.clone(), cmap.name(node.winner).to_string()));
        }
    }
    let mut col = Column::new().spacing(2);
    col = col.push(text(format!("Overrides ({}):", wins.len())).size(13.0));
    if wins.is_empty() {
        col = col.push(text("  (none)").size(11.0));
    }
    for (p, who) in wins.iter().take(300) {
        col = col.push(text(format!("  {p}   >   {who}")).size(11.0));
    }
    col = col
        .push(Space::with_height(Length::Fixed(8.0)))
        .push(text(format!("Overridden by ({}):", loses.len())).size(13.0));
    if loses.is_empty() {
        col = col.push(text("  (none)").size(11.0));
    }
    for (p, who) in loses.iter().take(300) {
        col = col.push(text(format!("  {p}   <   {who}")).size(11.0));
    }
    col.into()
}

/// Filetree tab: every file the mod ships, relative to its root.
fn info_filetree<'a>(m: &ModEntry) -> Element<'a, Message> {
    let entries = overwrite_entries(&m.path);
    let mut col = Column::new().spacing(1).push(text(format!("{} file(s):", entries.len())).size(12.0));
    for e in entries.into_iter().take(2000) {
        col = col.push(text(e).size(11.0));
    }
    col.into()
}

/// Notes tab: an editable note persisted to the mod's meta.ini.
fn info_notes<'a>(app: &App) -> Element<'a, Message> {
    Column::new()
        .spacing(8)
        .push(text("Note (saved to the mod's meta.ini):").size(12.0))
        .push(
            text_input("Add a note...", &app.notes_edit)
                .on_input(Message::NotesChanged)
                .on_submit(Message::NotesSave)
                .padding(6)
                .size(12.0),
        )
        .push(tool_btn("Save note", Message::NotesSave))
        .into()
}

/// MO2's per-mod info dialog: a centered modal with General / Conflicts /
/// Filetree / Notes tabs.
fn mod_info_dialog<'a>(app: &App, i: usize) -> Element<'a, Message> {
    let Some(m) = app.mods.get(i) else {
        return Space::new(Length::Shrink, Length::Shrink).into();
    };

    let title = Row::new()
        .spacing(8)
        .push(text(m.name.clone()).size(16.0).width(Length::Fill))
        .push(
            button(text("Close").size(12.0))
                .padding([3, 10])
                .on_press(Message::CloseInfo)
                .style(button::secondary),
        );

    let tabs = Row::new()
        .spacing(4)
        .push(info_tab_btn("General", InfoTab::General, app.info_tab == InfoTab::General))
        .push(info_tab_btn("Conflicts", InfoTab::Conflicts, app.info_tab == InfoTab::Conflicts))
        .push(info_tab_btn("Filetree", InfoTab::Filetree, app.info_tab == InfoTab::Filetree))
        .push(info_tab_btn("Notes", InfoTab::Notes, app.info_tab == InfoTab::Notes));

    let content = match app.info_tab {
        InfoTab::General => info_general(app, m),
        InfoTab::Conflicts => info_conflicts(app, i),
        InfoTab::Filetree => info_filetree(m),
        InfoTab::Notes => info_notes(app),
    };

    let card = Column::new()
        .spacing(10)
        .push(title)
        .push(tabs)
        .push(scrollable(content).height(Length::Fill));

    container(card)
        .width(Length::Fixed(660.0))
        .height(Length::Fixed(460.0))
        .padding(16)
        .style(|_t: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(0xEC, 0xDF, 0xC2))),
            border: Border {
                color: Color::from_rgb8(0x6E, 0x24, 0x2E),
                width: 2.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn data_panel<'a>(app: &App) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(6)
        .push(text("Name").size(11.0).width(Length::FillPortion(3)))
        .push(text("Mod").size(11.0).width(Length::FillPortion(2)))
        .push(text("Type").size(11.0).width(Length::Fixed(70.0)));

    let mut list = Column::new().spacing(1);
    let entries = merged_listing(app);
    if entries.is_empty() {
        list = list.push(text("(empty)").size(12.0));
    }
    for (idx, (name, source, is_dir)) in entries.into_iter().take(500).enumerate() {
        let row = Row::new()
            .spacing(6)
            .push(text(name).size(12.0).width(Length::FillPortion(3)))
            .push(text(source).size(12.0).width(Length::FillPortion(2)))
            .push(text(if is_dir { "Folder" } else { "File" }).size(12.0).width(Length::Fixed(70.0)));
        list = list.push(striped(row.into(), idx % 2 == 0));
    }
    Column::new().spacing(4).push(header).push(scrollable(list).height(Length::Fill)).into()
}

fn overwrite_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text("No instance open.").into();
    };
    let dir = inst.overwrite_dir();
    let actions = Row::new()
        .spacing(6)
        .push(
            text("Everything the game writes (configs, new saves, generated files) lands here.")
                .size(12.0)
                .width(Length::Fill),
        )
        .push(tool_btn("Open folder", Message::OpenFolder(dir.clone())))
        .push(
            button(text(if app.confirm_clear { "Confirm clear?" } else { "Clear" }).size(12.0))
                .padding(5)
                .on_press(Message::ClearOverwrite)
                .style(if app.confirm_clear { button::danger } else { button::secondary }),
        );

    let entries = overwrite_entries(&dir);
    let mut c = Column::new().spacing(2);
    if entries.is_empty() {
        c = c.push(text("(empty)").size(12.0));
    } else {
        c = c.push(text(format!("{} file(s):", entries.len())).size(11.0));
    }
    for e in entries.into_iter().take(500) {
        c = c.push(text(e).size(11.0));
    }

    Column::new()
        .spacing(8)
        .push(actions)
        .push(scrollable(c).height(Length::Fill))
        .into()
}

fn downloads_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(inst) = &app.created else {
        return text("No instance open.").into();
    };
    let dir = inst.downloads_dir();

    // Downloaded archives (skip .meta/.unfinished sidecars), newest first.
    let mut entries: Vec<(String, std::path::PathBuf, u64, std::time::SystemTime)> =
        std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                let name = p.file_name()?.to_string_lossy().into_owned();
                let lower = name.to_ascii_lowercase();
                let is_archive = lower.ends_with(".7z")
                    || lower.ends_with(".zip")
                    || lower.ends_with(".rar");
                if !is_archive {
                    return None;
                }
                let md = e.metadata().ok()?;
                Some((name, p, md.len(), md.modified().ok()?))
            })
            .collect();
    entries.sort_by(|a, b| b.3.cmp(&a.3));

    let mut rows = Column::new().spacing(2);
    if entries.is_empty() {
        rows = rows.push(
            text("No downloads yet. On Nexus, use \"Mod Manager Download\" once the handler is registered (eidos nxm --register), or drop archives here.")
                .size(11.0),
        );
    }
    for (i, (name, path, size, _)) in entries.into_iter().enumerate() {
        // Version from the MO2-format .meta sidecar, when present.
        let meta = eidos_instance::ModMeta::read(&std::path::PathBuf::from(format!(
            "{}.meta",
            path.display()
        )));
        let version = meta.version().unwrap_or_default();
        let row = Row::new()
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .push(text(name).size(12.0).width(Length::Fill))
            .push(text(version).size(11.0).width(Length::Fixed(90.0)))
            .push(text(format!("{:.1} MiB", size as f64 / (1024.0 * 1024.0))).size(11.0).width(Length::Fixed(80.0)))
            .push(
                button(text("Install").size(11.0))
                    .padding(4)
                    .on_press(Message::ModPicked(Some(path))),
            );
        rows = rows.push(striped(container(row).padding(3).into(), i % 2 == 0));
    }

    Column::new()
        .spacing(6)
        .push(
            Row::new()
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .push(text("Downloads").size(13.0))
                .push(Space::with_width(Length::Fill))
                .push(button(text("Open folder").size(11.0)).padding(4).on_press(Message::OpenFolder(dir.clone())))
                .push(button(text("Refresh").size(11.0)).padding(4).on_press(Message::Refresh)),
        )
        .push(text(dir.display().to_string()).size(10.0))
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

fn tab_btn<'a>(label: &'a str, t: Tab, selected: bool) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .padding(6)
        .on_press(Message::SelectTab(t))
        .style(if selected { button::primary } else { button::secondary })
        .into()
}

/// Compute the ESP/ESM load order for the Plugins tab: discover from the selected
/// game's Data plus the enabled mods, preserve any existing prefix order, and
/// validate. `None` if there is no game with a plugin system.
fn compute_plugins(app: &App) -> Option<PluginList> {
    let game = selected_game(app)?;
    let spec = GameSpec::for_id(game.def.id)?;
    let mut sources: Vec<(String, PathBuf)> = vec![(String::new(), game.data_path.clone())];
    let mut enabled: Vec<ModEntry> = app.mods.iter().filter(|m| m.enabled).cloned().collect();
    enabled.reverse(); // modlist is highest-first; plugins discover low-to-high
    sources.extend(enabled.into_iter().map(|m| (m.name, m.path)));

    let mut list = PluginList::discover(&sources, &spec);
    if let Some(cd) = game.compatdata.as_ref() {
        let dir = plugins_txt_dir(&cd.join("pfx"), &spec);
        let existing = PluginList::read_active(&dir, &spec);
        if !existing.is_empty() {
            list.apply_active(&existing);
        }
    }
    list.refresh(&spec);
    Some(list)
}

fn plugins_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(list) = &app.plugins else {
        return Column::new()
            .spacing(4)
            .push(text("Plugins (ESP / ESM / ESL load order)").size(13.0))
            .push(text("Open a game instance to compute the plugin load order.").size(12.0))
            .into();
    };

    let active = list.plugins.iter().filter(|p| p.enabled).count();
    let missing = list.missing_masters();
    let mut head = Column::new()
        .spacing(2)
        .push(text(format!("{} plugins - {active} active", list.plugins.len())).size(12.0));
    if !missing.is_empty() {
        head = head.push(
            text(format!("! {} missing master(s) - the game would crash", missing.len())).size(12.0),
        );
    }

    let header = Row::new()
        .spacing(6)
        .push(text("Index").size(11.0).width(Length::Fixed(52.0)))
        .push(text("On").size(11.0).width(Length::Fixed(28.0)))
        .push(text("Plugin").size(11.0).width(Length::Fill))
        .push(text("Type").size(11.0).width(Length::Fixed(36.0)));

    // Base-game masters are implicit/always-on; show them as forced, not togglable.
    let spec = selected_game(app).and_then(|g| GameSpec::for_id(g.def.id));
    let mut rows = Column::new().spacing(1);
    for (i, p) in list.plugins.iter().enumerate() {
        let idx = p.index.clone().unwrap_or_else(|| "--".to_string());
        let kind = if p.is_light {
            "ESL"
        } else if p.loads_as_master() {
            "ESM"
        } else {
            "esp"
        };
        let is_primary = spec
            .as_ref()
            .map(|s| s.primary_plugins.iter().any(|pp| pp.eq_ignore_ascii_case(&p.name)))
            .unwrap_or(false);
        let toggle: Element<'a, Message> = if is_primary {
            text("[x]").size(11.0).into()
        } else {
            button(text(if p.enabled { "[x]" } else { "[ ]" }).size(11.0))
                .padding(1)
                .on_press(Message::TogglePlugin(i))
                .style(button::text)
                .into()
        };
        let row = Row::new()
            .spacing(6)
            .push(text(idx).size(11.0).width(Length::Fixed(52.0)))
            .push(container(toggle).width(Length::Fixed(28.0)))
            .push(text(p.name.clone()).size(12.0).width(Length::Fill))
            .push(text(kind).size(10.0).width(Length::Fixed(36.0)));
        rows = rows.push(striped(row.into(), i % 2 == 0));
    }

    Column::new()
        .spacing(6)
        .push(head)
        .push(header)
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

/// Analyse file conflicts across the enabled mods (+ the game data) for the
/// Conflicts tab and the mod-row flags. Highest-priority mod first; game data
/// last as origin 0. `None` if there is no game.
fn compute_conflicts(app: &App) -> Option<ConflictMap> {
    let game = selected_game(app)?;
    let mut layers: Vec<Layer> = app
        .mods
        .iter()
        .enumerate()
        .filter(|(_, m)| m.enabled)
        .map(|(i, m)| Layer {
            origin: (i + 1) as u32,
            name: m.name.clone(),
            root: m.path.clone(),
        })
        .collect();
    // MO2's Overwrite is an always-active, top-priority pseudo-mod (xEdit / Bashed
    // Patch output lands there); include it at the front so the mods it shadows get
    // the overwritten emblem. Its whiteout markers are skipped by collect_files, and
    // its reserved origin (u32::MAX) keeps it distinct from BASE_ORIGIN (0).
    if let Some(inst) = app.created.as_ref() {
        let ow = inst.overwrite_dir();
        if ow.is_dir() {
            layers.insert(0, Layer { origin: u32::MAX, name: "Overwrite".to_string(), root: ow });
        }
    }
    layers.push(Layer {
        origin: 0,
        name: format!("[{}]", game.def.id),
        root: game.data_path.clone(),
    });
    Some(ConflictMap::build(&layers))
}

fn conflicts_panel<'a>(app: &App) -> Element<'a, Message> {
    let Some(map) = &app.conflicts else {
        return Column::new()
            .spacing(4)
            .push(text("Conflicts").size(13.0))
            .push(text("Open a game instance to analyse file conflicts across your mods.").size(12.0))
            .into();
    };

    let mut counts = (0usize, 0usize, 0usize, 0usize); // overwrites, overwritten, mixed, redundant
    let mut rows = Column::new().spacing(1);
    for (i, m) in app.mods.iter().enumerate().filter(|(_, m)| m.enabled) {
        let origin = (i + 1) as u32;
        let tag = match map.state(origin) {
            ConflictState::Overwrites => {
                counts.0 += 1;
                "overwrites others"
            }
            ConflictState::Overwritten => {
                counts.1 += 1;
                "overwritten"
            }
            ConflictState::Mixed => {
                counts.2 += 1;
                "mixed"
            }
            ConflictState::Redundant => {
                counts.3 += 1;
                "redundant - wins nothing"
            }
            ConflictState::None => continue,
        };
        let detail = map
            .mods
            .get(&origin)
            .map(|c| format!("{}/{} won", c.won, c.total))
            .unwrap_or_default();
        let row = Row::new()
            .spacing(6)
            .push(text(m.name.clone()).size(12.0).width(Length::Fill))
            .push(text(tag).size(11.0).width(Length::Fixed(160.0)))
            .push(text(detail).size(10.0).width(Length::Fixed(80.0)));
        rows = rows.push(striped(row.into(), i % 2 == 0));
    }

    let summary = format!(
        "{} overwrite - {} overwritten - {} mixed - {} redundant",
        counts.0, counts.1, counts.2, counts.3
    );
    Column::new()
        .spacing(6)
        .push(text(format!("Conflicts: {summary}")).size(12.0))
        .push(text("(only conflicting mods shown; flags also appear in the mod list)").size(10.0))
        .push(scrollable(rows).height(Length::Fill))
        .into()
}

fn right_pane<'a>(app: &App) -> Element<'a, Message> {
    let game_name = selected_game(app).map(|g| g.def.name).unwrap_or("Instance");
    // Run-target picker (MO2's executables combo): the game, or any tool run
    // through the same merged view.
    let run_options: Vec<String> = std::iter::once(RUN_GAME.to_string())
        .chain(app.tools.iter().map(|t| t.title.clone()))
        .collect();
    let run_choice = app.tool_choice.clone().unwrap_or_else(|| RUN_GAME.to_string());

    let top = Row::new()
        .spacing(8)
        .push(combo(game_name.to_string(), Message::Noop))
        .push(Space::with_width(Length::Fill))
        .push(pick_list(run_options, Some(run_choice), Message::ToolPicked).text_size(12.0).padding(8))
        .push(
            button(Row::new().spacing(6).push(icon(IC_RUN, 18.0)).push(text("Run").size(15.0)))
                .padding(10)
                .on_press(Message::Run)
                .style(button::primary),
        );

    let tabs = Row::new()
        .spacing(4)
        .push(tab_btn("Data", Tab::Data, app.tab == Tab::Data))
        .push(tab_btn("Plugins", Tab::Plugins, app.tab == Tab::Plugins))
        .push(tab_btn("Conflicts", Tab::Conflicts, app.tab == Tab::Conflicts))
        .push(tab_btn("Overwrite", Tab::Overwrite, app.tab == Tab::Overwrite))
        .push(tab_btn("Downloads", Tab::Downloads, app.tab == Tab::Downloads));

    let content = match app.tab {
        Tab::Data => data_panel(app),
        Tab::Plugins => plugins_panel(app),
        Tab::Conflicts => conflicts_panel(app),
        Tab::Overwrite => overwrite_panel(app),
        Tab::Downloads => downloads_panel(app),
    };

    let inner = Column::new().spacing(8).push(top).push(tabs).push(content);
    container(inner).width(Length::FillPortion(2)).height(Length::Fill).padding(8).style(panel_style).into()
}

fn status_bar<'a>(app: &App) -> Element<'a, Message> {
    let kind = match app.kind {
        InstanceKind::Global => "Global",
        InstanceKind::Portable => "Portable",
    };
    let game = selected_game(app).map(|g| g.def.name).unwrap_or("Instance");
    let left = app.status.clone().unwrap_or_else(|| format!("{game} - {kind} - Default"));
    let row = Row::new()
        .push(text(left).size(11.0).width(Length::Fill))
        .push(text("not logged in").size(11.0));
    container(row).width(Length::Fill).padding(4).style(bar_style).into()
}

fn main_screen<'a>(app: &App) -> Element<'a, Message> {
    let header = Row::new()
        .spacing(10)
        .push(text("Eidos").size(20.0))
        .push(Space::with_width(Length::Fill))
        .push(tool_btn("New instance", Message::Restart));

    let body = Row::new()
        .spacing(8)
        .height(Length::Fill)
        .push(modlist_pane(app))
        .push(right_pane(app));

    let base = Column::new()
        .spacing(4)
        .padding(4)
        .push(header)
        .push(menu_bar())
        .push(toolbar())
        .push(body)
        .push(status_bar(app));

    let mut layers = Stack::new().push(base);

    // The right-click action menu floats over the window (MO2's context menu).
    // A full-window catcher behind it dismisses on a click outside the card.
    if let Some(i) = app.menu_mod {
        if i < app.mods.len() {
            let catcher =
                mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::CloseMenu);
            let card = container(mod_menu_card(app, i))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(iced::Padding { top: 150.0, right: 0.0, bottom: 0.0, left: 40.0 })
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(iced::alignment::Vertical::Top);
            layers = layers.push(catcher).push(card);
        }
    }

    // The per-mod info dialog is a centered modal (MO2's modinfodialog).
    if let Some(i) = app.info_mod {
        if i < app.mods.len() {
            let scrim =
                mouse_area(Space::new(Length::Fill, Length::Fill)).on_press(Message::CloseInfo);
            let dialog = container(mod_info_dialog(app, i)).center(Length::Fill);
            layers = layers.push(scrim).push(dialog);
        }
    }

    layers.into()
}

/// Shared post-install step: activate the new mod at the top of the load order,
/// reload the list, and invalidate the plugin + conflict caches.
fn after_install(app: &mut App, name: &str, dest: PathBuf, fomod: bool) {
    if let Some(inst) = &app.created {
        let mut ml = inst.modlist();
        ml.retain(|m| m.name != name);
        ml.insert(0, ModEntry { name: name.to_string(), enabled: true, path: dest });
        let _ = inst.save_modlist(&ml);
        app.mods = inst.modlist();
    }
    app.plugins = None;
    app.conflicts = compute_conflicts(app);
    app.meta_cache = build_meta_cache(app);
    app.status = Some(if fomod {
        format!("Installed '{name}' via FOMOD.")
    } else {
        format!("Installed '{name}'.")
    });
}

fn group_type_label(t: eidos_fomod::GroupType) -> &'static str {
    use eidos_fomod::GroupType::*;
    match t {
        SelectExactlyOne => "choose one",
        SelectAtMostOne => "choose at most one",
        SelectAtLeastOne => "choose at least one",
        SelectAny => "choose any",
        SelectAll => "all included",
    }
}

/// Whether the wizard's current step satisfies its group constraints: a "choose
/// one" group needs exactly one selected, a "choose at least one" needs >= 1.
fn step_valid(w: &FomodWizard) -> bool {
    use eidos_fomod::GroupType::*;
    let Some(step) = w.session.config.steps.get(w.step) else {
        return true;
    };
    let Some(sel) = w.selection.get(w.step) else {
        return true;
    };
    for (gi, group) in step.groups.iter().enumerate() {
        let count = sel.get(gi).map(|g| g.iter().filter(|&&x| x).count()).unwrap_or(0);
        let ok = match group.group_type {
            SelectExactlyOne => count == 1,
            SelectAtLeastOne => count >= 1,
            _ => true,
        };
        if !ok {
            return false;
        }
    }
    true
}

/// The FOMOD installer wizard: the current step's groups as selectable options,
/// with Back / Cancel / Next / Install.
fn fomod_wizard_view(w: &FomodWizard) -> Element<'_, Message> {
    use eidos_fomod::PluginType;
    let config = &w.session.config;
    let total = config.steps.len();
    // Effective option types for this step (re-evaluated against the choices so far).
    let types = eidos_fomod::step_types(config, &w.selection, &w.ctx, w.step);

    let mut col = Column::new().spacing(8).padding(12);
    col = col.push(text(format!("{}  -  FOMOD installer", config.module_name)).size(20.0));
    if let Some(banner) = config.module_image.as_ref().and_then(|p| w.session.resolve(p)) {
        col = col.push(image(image::Handle::from_path(banner)).width(Length::Fixed(360.0)));
    }
    if let Some(step) = config.steps.get(w.step) {
        col = col.push(text(format!("Step {}/{}: {}", w.step + 1, total, step.name)).size(14.0));
        for (gi, group) in step.groups.iter().enumerate() {
            col = col.push(
                text(format!("{}  ({})", group.name, group_type_label(group.group_type))).size(13.0),
            );
            for (pi, plugin) in group.plugins.iter().enumerate() {
                let on = w
                    .selection
                    .get(w.step)
                    .and_then(|s| s.get(gi))
                    .and_then(|g| g.get(pi))
                    .copied()
                    .unwrap_or(false);
                let ptype = types.get(gi).and_then(|g| g.get(pi)).copied().unwrap_or(PluginType::Optional);
                let usable = ptype != PluginType::NotUsable;
                let mark = if on {
                    "[x]  "
                } else if usable {
                    "[  ]  "
                } else {
                    "[-]  "
                };
                let tag = match ptype {
                    PluginType::Required => "   - required",
                    PluginType::Recommended => "   - recommended",
                    PluginType::NotUsable => "   - not usable",
                    _ => "",
                };
                let mut b = button(text(format!("{mark}{}{tag}", plugin.name)).size(13.0))
                    .padding(4)
                    .width(Length::Fill)
                    .style(if on { button::primary } else { button::secondary });
                if usable {
                    b = b.on_press(Message::FomodToggle(gi, pi));
                }
                col = col.push(b);
                if !plugin.description.is_empty() {
                    col = col.push(text(plugin.description.clone()).size(11.0));
                }
                if let Some(img) = plugin.image.as_ref().and_then(|p| w.session.resolve(p)) {
                    col = col.push(image(image::Handle::from_path(img)).width(Length::Fixed(220.0)));
                }
            }
        }
    }
    let vis = eidos_fomod::visible_steps(config, &w.selection, &w.ctx);
    let has_prev = (0..w.step).any(|i| vis.get(i).copied().unwrap_or(false));
    let has_next = (w.step + 1..vis.len()).any(|i| vis[i]);
    let valid = step_valid(w);

    let mut nav = Row::new().spacing(8);
    if has_prev {
        nav = nav.push(tool_btn("Back", Message::FomodBack));
    }
    nav = nav.push(tool_btn("Cancel", Message::FomodCancel));
    nav = nav.push(Space::with_width(Length::Fill));
    let (label, msg) = if has_next {
        ("Next", Message::FomodNext)
    } else {
        ("Install", Message::FomodInstall)
    };
    if valid {
        nav = nav.push(tool_btn(label, msg));
    } else {
        // A constraint is unmet (e.g. a "choose one" group with nothing picked).
        nav = nav.push(button(text(label).size(13.0)).padding(6).style(button::secondary));
    }

    let mut bottom = Column::new().spacing(4);
    if !valid {
        bottom = bottom.push(text("Select the required option(s) to continue.").size(11.0));
    }
    bottom = bottom.push(nav);

    Column::new()
        .spacing(8)
        .padding(8)
        .push(scrollable(col).height(Length::Fill))
        .push(bottom)
        .into()
}

fn view(app: &App) -> Element<'_, Message> {
    if let Some(w) = &app.fomod {
        return fomod_wizard_view(w);
    }
    if app.screen == Screen::Main {
        return main_screen(app);
    }
    let inner = match app.screen {
        Screen::Welcome => welcome(),
        Screen::Kind => kind_screen(app),
        Screen::Game => game_screen(app),
        Screen::NameLoc => nameloc_screen(app),
        Screen::Summary => summary_screen(app),
        Screen::Main => welcome(),
    };
    container(inner).width(Length::Fill).height(Length::Fill).padding(20).into()
}

fn main() -> iced::Result {
    // Steam passes the Proton command as our arguments via `eidos-gui %command%`.
    let launch_command: Vec<String> = std::env::args().skip(1).collect();
    iced::application("Eidos", update, view)
        .theme(theme)
        .run_with(move || new(launch_command.clone()))
}
