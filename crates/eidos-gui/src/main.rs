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

use iced::widget::{button, container, scrollable, text, text_input, Column, Row, Space};
use iced::{Background, Border, Color, Element, Length, Task, Theme};

use eidos_games::{detect, home, DetectedGame};
use eidos_instance::{Instance, InstanceKind, ModEntry};

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
    Saves,
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
}

fn new() -> (App, Task<Message>) {
    (
        App {
            screen: Screen::Welcome,
            games: detect(&home()),
            kind: InstanceKind::Global,
            portable_path: String::new(),
            selected: None,
            name: String::new(),
            created: None,
            error: None,
            mods: Vec::new(),
            tab: Tab::Data,
            status: None,
        },
        Task::none(),
    )
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
                match inst.create() {
                    Ok(()) => {
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
            let id = selected_game(app).map(|g| g.def.id);
            if let Some(id) = id {
                app.status =
                    Some(format!("Set Steam launch option:  eidos play {id} -- %command%  then launch from Steam."));
            }
        }
        Message::Refresh => {
            if let Some(inst) = &app.created {
                app.mods = inst.modlist();
                app.status = Some("Refreshed mod list.".to_string());
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
        background: Some(Background::Color(Color::from_rgb8(0xE0, 0xD2, 0xB0))),
        border: Border { color: Color::from_rgb8(0xC9, 0xB8, 0x90), width: 1.0, radius: 0.0.into() },
        ..Default::default()
    }
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

fn list_dir_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
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
        .spacing(4)
        .push(tool_btn("File", Message::Noop))
        .push(tool_btn("View", Message::Noop))
        .push(tool_btn("Tools", Message::Noop))
        .push(tool_btn("Help", Message::Noop));
    container(row).width(Length::Fill).padding(3).style(bar_style).into()
}

fn toolbar<'a>(app: &App) -> Element<'a, Message> {
    let active = app.mods.iter().filter(|m| m.enabled).count();
    let row = Row::new()
        .spacing(8)
        .push(tool_btn("Refresh", Message::Refresh))
        .push(tool_btn("Settings", Message::Noop))
        .push(Space::with_width(Length::Fixed(16.0)))
        .push(text("Profile:").size(12.0))
        .push(tool_btn("Default", Message::Noop))
        .push(Space::with_width(Length::Fill))
        .push(text(format!("Active: {active}")).size(12.0));
    container(row).width(Length::Fill).padding(4).style(bar_style).into()
}

fn mod_row<'a>(i: usize, m: &ModEntry, len: usize) -> Element<'a, Message> {
    let mut up = button(text("Up").size(10.0)).padding(3).style(button::secondary);
    if i > 0 {
        up = up.on_press(Message::MoveUp(i));
    }
    let mut dn = button(text("Dn").size(10.0)).padding(3).style(button::secondary);
    if i + 1 < len {
        dn = dn.on_press(Message::MoveDown(i));
    }
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
    let header = Row::new()
        .spacing(6)
        .push(text("").width(C_CHECK))
        .push(text("#").size(11.0).width(C_PRIO))
        .push(text("Mod Name").size(11.0).width(Length::Fill))
        .push(text("Flags").size(11.0).width(C_FLAGS))
        .push(text("").width(C_MOVE));

    let len = app.mods.len();
    let mut list = Column::new().spacing(3);
    if app.mods.is_empty() {
        list = list.push(text("No mods yet. Drop mod folders into the instance's mods/ dir.").size(12.0));
    }
    for (i, m) in app.mods.iter().enumerate() {
        list = list.push(mod_row(i, m, len));
    }

    let overwrite = Row::new()
        .spacing(6)
        .push(text("").width(C_CHECK))
        .push(text("").width(C_PRIO))
        .push(text("Overwrite").size(13.0).width(Length::Fill));

    let inner = Column::new()
        .spacing(6)
        .push(text(format!("Mods ({len})")).size(14.0))
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

    let mut list = Column::new().spacing(2);
    let entries = merged_listing(app);
    if entries.is_empty() {
        list = list.push(text("(empty)").size(12.0));
    }
    for (name, source, is_dir) in entries.into_iter().take(500) {
        list = list.push(
            Row::new()
                .spacing(6)
                .push(text(name).size(12.0).width(Length::FillPortion(3)))
                .push(text(source).size(12.0).width(Length::FillPortion(2)))
                .push(text(if is_dir { "Folder" } else { "File" }).size(12.0).width(Length::Fixed(70.0))),
        );
    }
    Column::new().spacing(4).push(header).push(scrollable(list).height(Length::Fill)).into()
}

fn saves_panel<'a>(app: &App) -> Element<'a, Message> {
    let mut c = Column::new().spacing(3).push(text("Overwrite (writes land here)").size(13.0));
    if let Some(inst) = &app.created {
        let names = list_dir_names(&inst.overwrite_dir());
        if names.is_empty() {
            c = c.push(text("(empty - the game's saves and new files appear here)").size(12.0));
        }
        for name in names.into_iter().take(300) {
            c = c.push(text(name).size(12.0));
        }
    }
    scrollable(c).height(Length::Fill).into()
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
        .push(text(game_name).size(15.0).width(Length::Fill))
        .push(button(text("Run").size(15.0)).padding(10).on_press(Message::Run).style(button::primary));

    let tabs = Row::new()
        .spacing(4)
        .push(tab_btn("Data", Tab::Data, app.tab == Tab::Data))
        .push(tab_btn("Saves", Tab::Saves, app.tab == Tab::Saves))
        .push(tab_btn("Downloads", Tab::Downloads, app.tab == Tab::Downloads));

    let content = match app.tab {
        Tab::Data => data_panel(app),
        Tab::Saves => saves_panel(app),
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
        .spacing(6)
        .padding(6)
        .push(header)
        .push(menu_bar())
        .push(toolbar(app))
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
    iced::application("Eidos", update, view).theme(theme).run_with(new)
}
