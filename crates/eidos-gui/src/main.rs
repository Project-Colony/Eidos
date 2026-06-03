//! Eidos GUI (iced) - MO2-style wizard + two-pane main window.
//!
//!   Welcome -> Instance type (portable/global) -> Game -> Name/location
//!           -> Summary -> [create] -> Main (two-pane mod manager)
//!
//! The main window mirrors MO2: left = the mod list (enable/disable, priority,
//! reorder), right = tabs (Info, Saves). Colony parchment / burgundy look.
//! Run with: `cargo run -p eidos-gui`

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
    Info,
    Saves,
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
            tab: Tab::Info,
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
                        app.tab = Tab::Info;
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
                app.status = Some(format!(
                    "Set this Steam launch option, then launch from Steam:\n    eidos play {id} -- %command%"
                ));
            }
        }
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
        border: Border {
            color: Color::from_rgb8(0x7A, 0x1F, 0x2B),
            width: 1.5,
            radius: 8.0.into(),
        },
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
                    .style(if app.selected == Some(i) {
                        button::primary
                    } else {
                        button::secondary
                    }),
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
            text_input("~/Eidos/skyrimse", &app.portable_path)
                .on_input(Message::PortableChanged)
                .padding(8),
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
    frame(
        "Step 5 of 5",
        "Review and create",
        content.into(),
        Some(Message::Back),
        "Create instance",
        Some(Message::Finish),
    )
}

// ---- main two-pane window ----------------------------------------------------

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

fn mod_row<'a>(i: usize, m: &ModEntry, len: usize) -> Element<'a, Message> {
    Row::new()
        .spacing(8)
        .push(nav(if m.enabled { "[x]" } else { "[ ]" }, Some(Message::ToggleMod(i)), false))
        .push(text(format!("{:>2}", i + 1)).size(13.0))
        .push(text(m.name.clone()).size(14.0).width(Length::Fill))
        .push(nav("Up", (i > 0).then_some(Message::MoveUp(i)), false))
        .push(nav("Dn", (i + 1 < len).then_some(Message::MoveDown(i)), false))
        .into()
}

fn info_panel<'a>(app: &App) -> Element<'a, Message> {
    let enabled = app.mods.iter().filter(|m| m.enabled).count();
    let mut c = Column::new().spacing(6).push(text("Instance").size(16.0));
    if let Some(inst) = &app.created {
        c = c.push(text(format!("Path: {}", inst.root.display())).size(12.0));
        c = c.push(text(format!("Mods: {} enabled / {} total", enabled, app.mods.len())).size(13.0));
        c = c.push(text(format!("Add mods in: {}", inst.mods_dir().display())).size(12.0));
    }
    if let Some(g) = selected_game(app) {
        c = c.push(text(format!("Game data: {}", g.data_path.display())).size(12.0));
        let run = button(text(format!("Run {}", g.def.name)).size(14.0))
            .padding(10)
            .on_press(Message::Run)
            .style(button::primary);
        c = c.push(run);
    }
    if let Some(s) = &app.status {
        c = c.push(text(s.clone()).size(12.0));
    }
    c.into()
}

fn saves_panel<'a>(app: &App) -> Element<'a, Message> {
    let mut c = Column::new().spacing(4).push(text("Overwrite (writes land here)").size(14.0));
    if let Some(inst) = &app.created {
        let names = list_dir_names(&inst.overwrite_dir());
        if names.is_empty() {
            c = c.push(text("(empty - the game's saves and new files appear here)").size(12.0));
        }
        for name in names.into_iter().take(200) {
            c = c.push(text(name).size(12.0));
        }
    }
    scrollable(c).into()
}

fn main_screen<'a>(app: &App) -> Element<'a, Message> {
    let title = selected_game(app)
        .map(|g| g.def.name.to_string())
        .unwrap_or_else(|| "Instance".to_string());
    let header = Row::new()
        .spacing(12)
        .push(text("Eidos").size(24.0))
        .push(text(title).size(15.0))
        .push(Space::with_width(Length::Fill))
        .push(nav("New instance", Some(Message::Restart), false));

    // Left: mod list.
    let mut list = Column::new()
        .spacing(4)
        .push(text(format!("Mods ({})", app.mods.len())).size(16.0));
    if app.mods.is_empty() {
        list = list.push(
            text("No mods yet. Drop mod folders into the instance's mods/ dir.").size(12.0),
        );
    }
    let len = app.mods.len();
    for (i, m) in app.mods.iter().enumerate() {
        list = list.push(mod_row(i, m, len));
    }
    let left = container(scrollable(list))
        .width(Length::FillPortion(3))
        .height(Length::Fill)
        .padding(10)
        .style(card_style);

    // Right: tabs.
    let tabs = Row::new()
        .spacing(6)
        .push(nav("Info", Some(Message::SelectTab(Tab::Info)), app.tab == Tab::Info))
        .push(nav("Saves", Some(Message::SelectTab(Tab::Saves)), app.tab == Tab::Saves));
    let tab_content = match app.tab {
        Tab::Info => info_panel(app),
        Tab::Saves => saves_panel(app),
    };
    let right = container(Column::new().spacing(10).push(tabs).push(tab_content))
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .padding(10)
        .style(card_style);

    let body = Row::new().spacing(12).push(left).push(right);

    Column::new().spacing(12).push(header).push(body).into()
}

fn view(app: &App) -> Element<'_, Message> {
    let inner = match app.screen {
        Screen::Welcome => welcome(),
        Screen::Kind => kind_screen(app),
        Screen::Game => game_screen(app),
        Screen::NameLoc => nameloc_screen(app),
        Screen::Summary => summary_screen(app),
        Screen::Main => main_screen(app),
    };
    container(inner).width(Length::Fill).height(Length::Fill).padding(20).into()
}

fn main() -> iced::Result {
    iced::application("Eidos", update, view).theme(theme).run_with(new)
}
