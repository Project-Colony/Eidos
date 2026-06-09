//! Eidos GUI (iced) - MO2-style wizard + two-pane main window.
//!
//!   Welcome -> Instance type (portable/global) -> Game -> Name/location
//!           -> Summary -> [create] -> Main (MO2-style mod manager)
//!
//! The main window mirrors Mod Organizer 2: menu bar + toolbar + profile row,
//! left = the mod list (enable, priority, reorder) with an Overwrite entry,
//! right = Run + Data/Saves/Downloads tabs, plus a status bar. Colony parchment
//! / burgundy palette. Run with: `cargo run -p eidos-gui`

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use iced::widget::{button, container, image, scrollable, text, text_input, Column, Row, Space};
use iced::{Background, Border, Color, Element, Length, Task, Theme};

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::{Instance, InstanceKind, ModEntry};

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
    Overwrite,
    Downloads,
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
    ToggleMod(usize),
    MoveUp(usize),
    MoveDown(usize),
    SelectTab(Tab),
    Run,
    Refresh,
    OpenFolder(PathBuf),
    ClearOverwrite,
    Noop,
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
    tab: Tab,
    status: Option<String>,
    /// Two-click guard for the destructive "Clear Overwrite" action.
    confirm_clear: bool,
    /// The Proton command Steam passed via `%command%` (empty if launched
    /// standalone). The Run button launches the game through this.
    launch_command: Vec<String>,
}

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
        tab: Tab::Data,
        status: None,
        confirm_clear: false,
        launch_command,
    };
    if let Some(i) = auto {
        app.selected = Some(i);
        let inst = Instance::global(app.games[i].def.id);
        if inst.exists() {
            let _ = inst.ensure_manifest(app.games[i].def.id, InstanceKind::Global);
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
                app.selected = Some(i);
                app.mods = inst.modlist();
                app.created = Some(inst);
                app.screen = Screen::Main;
                break;
            }
        }
    }
    (app, Task::none())
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
    match game_id {
        "skyrimse" => Some(("SkyrimSELauncher.exe", "skse64_loader.exe")),
        "skyrim" => Some(("SkyrimLauncher.exe", "skse_loader.exe")),
        "fallout4" => Some(("Fallout4Launcher.exe", "f4se_loader.exe")),
        "falloutnv" => Some(("FalloutNVLauncher.exe", "nvse_loader.exe")),
        "fallout3" => Some(("FalloutLauncher.exe", "fose_loader.exe")),
        "oblivion" => Some(("OblivionLauncher.exe", "obse_loader.exe")),
        _ => None,
    }
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
            app.screen = Screen::Welcome;
        }
        Message::ToggleMod(i) => {
            if let Some(m) = app.mods.get_mut(i) {
                m.enabled = !m.enabled;
            }
            save_mods(app);
        }
        Message::MoveUp(i) => {
            if i > 0 && i < app.mods.len() {
                app.mods.swap(i - 1, i);
                save_mods(app);
            }
        }
        Message::MoveDown(i) => {
            if i + 1 < app.mods.len() {
                app.mods.swap(i, i + 1);
                save_mods(app);
            }
        }
        Message::SelectTab(t) => app.tab = t,
        Message::Run => {
            if app.launch_command.is_empty() {
                // Standalone: we don't have Steam's Proton command, so we cannot
                // build the launch environment. Point the user at the option.
                app.status = Some(
                    "To launch from here, set the game's Steam launch option to:  eidos-gui %command%  then press Play in Steam (Eidos opens, then click Run)."
                        .to_string(),
                );
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
                app.status = Some("Refreshed mod list.".to_string());
            }
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
        .push(icon_text_btn(IC_INSTALL, "Install Mod", Message::Noop))
        .push(icon_text_btn(IC_NEXUS, "Nexus", Message::Noop))
        .push(icon_text_btn(IC_CHANGE_GAME, "Change Game", Message::Noop))
        .push(icon_text_btn(IC_REFRESH, "Refresh", Message::Refresh))
        .push(icon_text_btn(IC_EXECUTABLES, "Executables", Message::Noop))
        .push(icon_text_btn(IC_TOOLS, "Tools", Message::Noop))
        .push(icon_text_btn(IC_SETTINGS, "Settings", Message::Noop))
        .push(Space::with_width(Length::Fill))
        .push(icon_btn(IC_ENDORSE, 20.0, Some(Message::Noop)))
        .push(icon_btn(IC_UPDATE, 20.0, Some(Message::Noop)))
        .push(icon_btn(IC_HELP, 20.0, Some(Message::Noop)));
    container(row).width(Length::Fill).padding(2).style(bar_style).into()
}

fn mod_row<'a>(i: usize, m: &ModEntry, len: usize) -> Element<'a, Message> {
    let up = icon_btn(IC_UP, 14.0, (i > 0).then_some(Message::MoveUp(i)));
    let dn = icon_btn(IC_DOWN, 14.0, (i + 1 < len).then_some(Message::MoveDown(i)));
    let toggle = button(text(if m.enabled { "[x]" } else { "[ ]" }).size(12.0))
        .padding(3)
        .on_press(Message::ToggleMod(i))
        .style(button::secondary);

    Row::new()
        .spacing(6)
        .push(container(toggle).width(C_CHECK))
        .push(text(format!("{:>2}", i + 1)).size(12.0).width(C_PRIO))
        .push(text(m.name.clone()).size(13.0).width(Length::Fill))
        .push(text(if m.enabled { "" } else { "off" }).size(11.0).width(C_FLAGS))
        .push(Row::new().spacing(2).push(up).push(dn).width(C_MOVE))
        .into()
}

fn modlist_pane<'a>(app: &App) -> Element<'a, Message> {
    let active = app.mods.iter().filter(|m| m.enabled).count();
    let profile = Row::new()
        .spacing(8)
        .push(text("Profile:").size(12.0))
        .push(combo("Default".to_string(), Message::Noop))
        .push(tool_btn("Save", Message::Noop))
        .push(Space::with_width(Length::Fill))
        .push(text(format!("Active: {active}")).size(12.0));

    let header = Row::new()
        .spacing(6)
        .push(text("").width(C_CHECK))
        .push(text("#").size(11.0).width(C_PRIO))
        .push(text("Mod Name").size(11.0).width(Length::Fill))
        .push(text("Flags").size(11.0).width(C_FLAGS))
        .push(text("").width(C_MOVE));

    let len = app.mods.len();
    let mut list = Column::new().spacing(1);
    if app.mods.is_empty() {
        list = list.push(text("No mods yet. Drop mod folders into the instance's mods/ dir.").size(12.0));
    }
    for (i, m) in app.mods.iter().enumerate() {
        list = list.push(striped(mod_row(i, m, len), i % 2 == 0));
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
        .push(header)
        .push(scrollable(list).height(Length::Fill))
        .push(overwrite);

    container(inner).width(Length::FillPortion(3)).height(Length::Fill).padding(8).style(panel_style).into()
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

fn downloads_panel<'a>() -> Element<'a, Message> {
    Column::new()
        .spacing(4)
        .push(text("Downloads").size(13.0))
        .push(text("Nexus integration comes later. For now, extract mods into mods/.").size(12.0))
        .into()
}

fn tab_btn<'a>(label: &'a str, t: Tab, selected: bool) -> Element<'a, Message> {
    button(text(label).size(12.0))
        .padding(6)
        .on_press(Message::SelectTab(t))
        .style(if selected { button::primary } else { button::secondary })
        .into()
}

fn right_pane<'a>(app: &App) -> Element<'a, Message> {
    let game_name = selected_game(app).map(|g| g.def.name).unwrap_or("Instance");
    let top = Row::new()
        .spacing(8)
        .push(combo(game_name.to_string(), Message::Noop))
        .push(Space::with_width(Length::Fill))
        .push(
            button(Row::new().spacing(6).push(icon(IC_RUN, 18.0)).push(text("Run").size(15.0)))
                .padding(10)
                .on_press(Message::Run)
                .style(button::primary),
        );

    let tabs = Row::new()
        .spacing(4)
        .push(tab_btn("Data", Tab::Data, app.tab == Tab::Data))
        .push(tab_btn("Overwrite", Tab::Overwrite, app.tab == Tab::Overwrite))
        .push(tab_btn("Downloads", Tab::Downloads, app.tab == Tab::Downloads));

    let content = match app.tab {
        Tab::Data => data_panel(app),
        Tab::Overwrite => overwrite_panel(app),
        Tab::Downloads => downloads_panel(),
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

    Column::new()
        .spacing(4)
        .padding(4)
        .push(header)
        .push(menu_bar())
        .push(toolbar())
        .push(body)
        .push(status_bar(app))
        .into()
}

fn view(app: &App) -> Element<'_, Message> {
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
