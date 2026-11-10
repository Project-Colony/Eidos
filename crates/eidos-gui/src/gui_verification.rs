//! Synthetic offscreen widget verification. No native window or user profile is opened.
use crate::*;
use iced::{mouse, Event, Font, Pixels, Point, Rectangle, Size};
use iced_runtime::{
    core,
    user_interface::{Cache, UserInterface},
};
use std::time::Instant;

const SIZE: Size = Size::new(1280.0, 800.0);

fn renderer() -> iced::Renderer {
    iced::Renderer::Secondary(iced_tiny_skia::Renderer::new(Font::DEFAULT, Pixels(14.0)))
}

fn frame<'a>(
    element: Element<'a, Message>,
    renderer: &mut iced::Renderer,
    cache: Cache,
    events: &[Event],
    cursor: Point,
    name: Option<&str>,
) -> (Cache, Vec<Message>) {
    let mut ui = UserInterface::build(element, SIZE, cache, renderer);
    let cursor = mouse::Cursor::Available(cursor);
    let mut messages = Vec::new();
    ui.update(
        events,
        cursor,
        renderer,
        &mut core::clipboard::Null,
        &mut messages,
    );
    ui.update(
        &[Event::Window(iced::window::Event::RedrawRequested(
            Instant::now(),
        ))],
        cursor,
        renderer,
        &mut core::clipboard::Null,
        &mut messages,
    );
    ui.draw(
        renderer,
        &iced::Theme::custom("Eidos verification", crate::theme::palette()),
        &core::renderer::Style {
            text_color: crate::theme::pal().text_primary,
        },
        cursor,
    );
    if let Some(dir) = name.and_then(|name| {
        std::env::var_os("EIDOS_GUI_VISUAL_DIR").map(|dir| (PathBuf::from(dir), name))
    }) {
        fs::create_dir_all(&dir.0).unwrap();
        let iced::Renderer::Secondary(cpu) = renderer else {
            panic!("CPU renderer required")
        };
        // The compositor performs its native BGRA-to-RGBA conversion too.
        let pixels = iced_tiny_skia::window::compositor::screenshot(
            cpu,
            &iced_tiny_skia::graphics::Viewport::with_physical_size(
                Size::new(SIZE.width as u32, SIZE.height as u32),
                1.0,
            ),
            crate::theme::pal().bg_primary,
        );
        image_decoder::save_buffer(
            dir.0.join(format!("{}.png", dir.1)),
            &pixels,
            SIZE.width as u32,
            SIZE.height as u32,
            image_decoder::ColorType::Rgba8,
        )
        .unwrap();
    }
    (ui.into_cache(), messages)
}

fn click() -> [Event; 2] {
    [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]
}

#[derive(Default)]
struct Positions {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<Rectangle>,
}
impl core::widget::Operation for Positions {
    fn traverse(&mut self, f: &mut dyn FnMut(&mut dyn core::widget::Operation)) {
        f(self);
    }
    fn text(&mut self, _: Option<&core::widget::Id>, bounds: Rectangle, text: &str) {
        self.labels.push((text.into(), bounds));
    }
    fn text_input(
        &mut self,
        _: Option<&core::widget::Id>,
        bounds: Rectangle,
        _: &mut dyn core::widget::operation::TextInput,
    ) {
        self.inputs.push(bounds);
    }
}
fn positions(element: Element<'_, Message>, renderer: &mut iced::Renderer) -> Positions {
    let mut ui = UserInterface::build(element, SIZE, Cache::new(), renderer);
    let mut out = Positions::default();
    ui.operate(renderer, &mut out);
    out
}
fn label(
    element: Element<'_, Message>,
    renderer: &mut iced::Renderer,
    name: &str,
    index: usize,
) -> Point {
    let positions = positions(element, renderer);
    positions
        .labels
        .iter()
        .filter(|(text, _)| text == name)
        .nth(index)
        .unwrap_or_else(|| panic!("Missing label {name}: {:?}", positions.labels))
        .1
        .center()
}
fn apply(app: &mut App, messages: Vec<Message>) {
    assert!(!messages.is_empty(), "widget did not emit an action");
    for message in messages {
        let task = crate::update::update_inner(app, message);
        installers::tests::drive(app, task);
    }
}
fn key(
    key: iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
    text: Option<&str>,
) -> Event {
    Event::Keyboard(iced::keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        location: iced::keyboard::Location::Standard,
        modifiers,
        text: text.map(Into::into),
        repeat: false,
    })
}

/// Preferences open from the first-run screens too: the language, the text size
/// and the dyslexia-friendly font matter most before there is any instance.
#[test]
fn the_name_opens_preferences_from_the_welcome_screen() {
    let _look = crate::test_support::look_lock();
    let mut app = crate::test_support::nav_app(&[]);
    app.screen = Screen::Welcome;
    let mut renderer = renderer();
    let point = label(crate::view(&app), &mut renderer, "Eidos", 0);
    let (_, messages) = frame(
        crate::view(&app),
        &mut renderer,
        Cache::new(),
        &click(),
        point,
        None,
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::TogglePreferences)),
        "{messages:?}"
    );
    apply(&mut app, messages);
    assert!(app.settings_open);

    let title = |locale| colony_ui::i18n::t_in(locale, "settings_title");
    let drawn = positions(crate::view(&app), &mut renderer).labels;
    assert!(
        drawn.iter().any(|(text, _)| {
            text == title(colony_ui::i18n::Locale::En) || text == title(colony_ui::i18n::Locale::Fr)
        }),
        "the page was not drawn: {drawn:?}"
    );
}

#[test]
fn installer_widgets_emit_pointer_and_keyboard_events_and_publish_exact_choice() {
    let _look = crate::test_support::look_lock();
    let (_dir, mut app, archive) = installers::tests::fixture();
    installers::tests::open(&mut app, &archive);
    let mut renderer = renderer();
    let (mut cache, _) = frame(
        crate::view(&app),
        &mut renderer,
        Cache::new(),
        &[],
        Point::ORIGIN,
        Some("installer-options"),
    );
    let point = label(crate::view(&app), &mut renderer, "Two", 0);
    let (next, messages) = frame(
        crate::view(&app),
        &mut renderer,
        cache,
        &click(),
        point,
        None,
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::Installer(installers::Action::Toggle(1)))),
        "{messages:?}"
    );
    cache = next;
    apply(&mut app, messages);
    let point = label(crate::view(&app), &mut renderer, "Continue", 0);
    let (next, messages) = frame(
        crate::view(&app),
        &mut renderer,
        cache,
        &click(),
        point,
        None,
    );
    cache = next;
    apply(&mut app, messages);
    let point = positions(crate::view(&app), &mut renderer).inputs[0].center();
    let (next, _) = frame(
        crate::view(&app),
        &mut renderer,
        cache,
        &click(),
        point,
        None,
    );
    let events = [
        Event::Keyboard(iced::keyboard::Event::ModifiersChanged(
            iced::keyboard::Modifiers::CTRL,
        )),
        key(
            iced::keyboard::Key::Character("a".into()),
            iced::keyboard::Modifiers::CTRL,
            None,
        ),
        Event::Keyboard(iced::keyboard::Event::ModifiersChanged(
            iced::keyboard::Modifiers::SHIFT,
        )),
        key(
            iced::keyboard::Key::Character("W".into()),
            iced::keyboard::Modifiers::SHIFT,
            Some("W"),
        ),
    ];
    let (next, messages) = frame(crate::view(&app), &mut renderer, next, &events, point, None);
    assert!(
        messages.iter().any(
            |m| matches!(m, Message::Installer(installers::Action::Name(name)) if name == "W")
        ),
        "{messages:?}"
    );
    cache = next;
    apply(&mut app, messages);
    let (next, _) = frame(
        crate::view(&app),
        &mut renderer,
        cache,
        &[],
        Point::ORIGIN,
        Some("installer-review"),
    );
    let point = label(
        crate::view(&app),
        &mut renderer,
        "Install reviewed files",
        0,
    );
    let (_, messages) = frame(
        crate::view(&app),
        &mut renderer,
        next,
        &click(),
        point,
        None,
    );
    apply(&mut app, messages);
    assert!(app.installer.is_none());
    assert_eq!(
        fs::read(
            app.created
                .as_ref()
                .unwrap()
                .mods_dir()
                .join("W/selected.txt")
        )
        .unwrap(),
        b"second"
    );
}

fn dds_fixture() -> Vec<u8> {
    let mut bytes = vec![0; 148];
    bytes[..4].copy_from_slice(b"DDS ");
    for (at, value) in [
        (4, 124u32),
        (8, 0x21007),
        (12, 256),
        (16, 256),
        (28, 2),
        (76, 32),
        (80, 4),
        (84, u32::from_le_bytes(*b"DX10")),
        (108, 0x401008),
        (128, 28),
        (132, 3),
        (140, 1),
    ] {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[40, 160, 220, 255].repeat(256 * 256));
    bytes.extend_from_slice(&[220, 100, 40, 255].repeat(128 * 128));
    bytes
}

#[test]
fn dds_nif_and_archive_controls_are_laid_out_rendered_and_clickable() {
    let _look = crate::test_support::look_lock();
    let mut renderer = renderer();
    let bytes = dds_fixture();
    let mut preview = file_preview::from_bytes(Path::new("synthetic.dds"), bytes, false);
    assert!(matches!(preview, Preview::Dds { .. }));
    let (cache, _) = frame(
        dialogs::preview_dialog(&preview),
        &mut renderer,
        Cache::new(),
        &[],
        Point::ORIGIN,
        Some("dds-mip-0"),
    );
    let point = label(dialogs::preview_dialog(&preview), &mut renderer, "+", 0);
    let (_, messages) = frame(
        dialogs::preview_dialog(&preview),
        &mut renderer,
        cache,
        &click(),
        point,
        None,
    );
    let selected = messages
        .iter()
        .find_map(|m| {
            if let Message::PreviewDdsSelection(s) = m {
                Some(*s)
            } else {
                None
            }
        })
        .expect("DDS mip button event");
    assert_eq!(selected.mip, 1);
    if let Preview::Dds {
        bytes,
        info,
        selection,
        image,
        ..
    } = &mut preview
    {
        let decoded = dds_preview::decode(bytes, selected).unwrap();
        *selection = selected;
        *info = decoded.info;
        *image = Ok(iced::widget::image::Handle::from_rgba(
            decoded.width,
            decoded.height,
            decoded.rgba,
        ));
    }
    frame(
        dialogs::preview_dialog(&preview),
        &mut renderer,
        Cache::new(),
        &[],
        Point::ORIGIN,
        Some("dds-mip-1"),
    );

    let point = label(dialogs::preview_dialog(&preview), &mut renderer, "+", 2);
    let events = [
        Event::Keyboard(iced::keyboard::Event::ModifiersChanged(
            iced::keyboard::Modifiers::CTRL,
        )),
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 },
        }),
    ];
    let (_, messages) = frame(
        dialogs::preview_dialog(&preview),
        &mut renderer,
        Cache::new(),
        &events,
        Point::new(point.x + 50.0, point.y),
        None,
    );
    assert!(messages.iter().any(|m| matches!(m, Message::PreviewDdsSelection(s) if s.channel == dds_preview::Channel::Rgb)), "DDS channel modifier-wheel event: {messages:?}");

    let mut preview = nif_preview::change_view(
        Preview::Nif {
            path: "synthetic.nif".into(),
            provenance: None,
            model: nif_preview::tests::fixture_model(),
        },
        Default::default(),
        &std::sync::atomic::AtomicBool::new(false),
    );
    let (cache, _) = frame(
        dialogs::preview_dialog(&preview),
        &mut renderer,
        Cache::new(),
        &[],
        Point::ORIGIN,
        Some("nif-solid"),
    );
    let point = label(
        dialogs::preview_dialog(&preview),
        &mut renderer,
        "Wireframe",
        0,
    );
    let (_, messages) = frame(
        dialogs::preview_dialog(&preview),
        &mut renderer,
        cache,
        &click(),
        point,
        None,
    );
    let selected = messages
        .iter()
        .find_map(|m| {
            if let Message::PreviewNifView(view) = m {
                Some(*view)
            } else {
                None
            }
        })
        .expect("NIF wireframe checkbox event");
    assert!(selected.wireframe);
    preview = nif_preview::change_view(
        preview,
        selected,
        &std::sync::atomic::AtomicBool::new(false),
    );
    let (cache, _) = frame(
        dialogs::preview_dialog(&preview),
        &mut renderer,
        Cache::new(),
        &[],
        Point::ORIGIN,
        Some("nif-wireframe"),
    );
    let orbit = label(dialogs::preview_dialog(&preview), &mut renderer, "Orbit", 0);
    let (_, messages) = frame(
        dialogs::preview_dialog(&preview),
        &mut renderer,
        cache,
        &click(),
        Point::new(orbit.x + 190.0, orbit.y),
        None,
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::PreviewNifView(view) if view.yaw != selected.yaw)),
        "{messages:?}"
    );

    let (_, messages) = frame(
        dialogs::preview_dialog(&preview),
        &mut renderer,
        Cache::new(),
        &[key(
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp),
            iced::keyboard::Modifiers::empty(),
            None,
        )],
        Point::new(orbit.x + 190.0, orbit.y),
        None,
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::PreviewNifView(view) if view.yaw > selected.yaw)),
        "NIF slider keyboard event: {messages:?}"
    );

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("synthetic.bsa");
    let folder = b"textures\0";
    let member = b"synthetic.dds\0";
    let record = 52 + 1 + folder.len();
    let names = record + 16;
    let offset = names + member.len();
    let dds = dds_fixture();
    let mut bsa = vec![0; offset];
    bsa[..4].copy_from_slice(b"BSA\0");
    for (at, value) in [
        (4, 103u32),
        (8, 36),
        (12, 3),
        (16, 1),
        (20, 1),
        (24, folder.len() as u32),
        (28, member.len() as u32),
        (44, 1),
        (48, (52 + member.len()) as u32),
        (record + 8, dds.len() as u32),
        (record + 12, offset as u32),
    ] {
        bsa[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    bsa[52] = folder.len() as u8;
    bsa[53..53 + folder.len()].copy_from_slice(folder);
    bsa[names..offset].copy_from_slice(member);
    bsa.extend_from_slice(&dds);
    fs::write(&path, bsa).unwrap();
    let decoded_member =
        eidos_conflicts::read_archive_member(&path, "textures/synthetic.dds", 1024 * 1024).unwrap();
    assert_eq!(decoded_member, dds);
    let archive = Preview::Archive {
        source: archive_conflicts::MemberSource {
            identity: eidos_conflicts::archive_identity(&path).unwrap(),
            path,
            member: "textures/synthetic.dds".into(),
        },
        content: Box::new(file_preview::from_bytes(
            Path::new("textures/synthetic.dds"),
            decoded_member,
            false,
        )),
    };
    let (cache, _) = frame(
        dialogs::preview_dialog(&archive),
        &mut renderer,
        Cache::new(),
        &[],
        Point::ORIGIN,
        Some("archive-member-dds"),
    );
    let point = label(
        dialogs::preview_dialog(&archive),
        &mut renderer,
        "Extension",
        0,
    );
    let (cache, messages) = frame(
        dialogs::preview_dialog(&archive),
        &mut renderer,
        cache,
        &click(),
        point,
        None,
    );
    assert!(
        messages.is_empty(),
        "archive member must not dispatch a file extension: {messages:?}"
    );
    let point = label(dialogs::preview_dialog(&archive), &mut renderer, "Close", 0);
    let (_, messages) = frame(
        dialogs::preview_dialog(&archive),
        &mut renderer,
        cache,
        &click(),
        point,
        None,
    );
    assert!(messages.iter().any(|m| matches!(m, Message::ClosePreview)));
}

#[test]
fn warmed_main_view_construction_and_layout_medians() {
    let _look = crate::test_support::look_lock();
    let (_dir, mut app, _archive) = installers::tests::fixture();
    let mut renderer = renderer();
    for count in [1_000, 10_000] {
        app.mods = (0..count)
            .map(|i| ModEntry {
                name: format!("Synthetic mod {i:05}"),
                enabled: i % 3 != 0,
                path: app
                    .created
                    .as_ref()
                    .unwrap()
                    .mods_dir()
                    .join(format!("mod-{i}")),
                unmanaged: false,
            })
            .collect();
        app.view_generation.set(app.view_generation.get() + 1);
        let mut cache = Cache::new();
        let mut builds = Vec::new();
        let mut layouts = Vec::new();
        for i in 0..13 {
            let start = Instant::now();
            let element = crate::view(&app);
            let constructed = Instant::now();
            let ui = UserInterface::build(element, SIZE, cache, &mut renderer);
            let laid_out = Instant::now();
            cache = ui.into_cache();
            if i >= 3 {
                builds.push(constructed.duration_since(start));
                layouts.push(laid_out.duration_since(constructed));
            }
        }
        builds.sort();
        layouts.sort();
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "optimized"
        };
        println!("GUI_BENCH rows={count} samples=10 construction_us={} layout_us={} renderer=tiny-skia profile={profile} viewport=1280x800 statistic=upper-middle", builds[5].as_micros(), layouts[5].as_micros());
        let (_, messages) = frame(
            crate::view(&app),
            &mut renderer,
            cache,
            &[],
            Point::ORIGIN,
            Some(&format!("main-{count}")),
        );
        assert!(messages.is_empty());
    }
}

/// Rows aim a drag only while one is live. Without a drag, crossing rows
/// publishes nothing (each message would be a whole-window rebuild); once a
/// press arms one, entering another row aims at it, and the row already under
/// the pointer fires no false enter. Mod rows and plugin rows both.
#[test]
fn rows_aim_only_during_a_drag() {
    let _look = crate::test_support::look_lock();
    let (_dir, mut app, _archive) = installers::tests::fixture();
    let mods_dir = app.created.as_ref().unwrap().mods_dir();
    app.mods = ["Aim A", "Aim B", "Aim C"]
        .iter()
        .map(|name| ModEntry {
            name: name.to_string(),
            enabled: true,
            path: mods_dir.join(name),
            unmanaged: false,
        })
        .collect();
    app.view_generation.set(app.view_generation.get() + 1);
    let mut renderer = renderer();
    let moved = |at: Point| [Event::Mouse(mouse::Event::CursorMoved { position: at })];
    let aimed = |messages: &[Message]| -> Vec<usize> {
        messages
            .iter()
            .filter_map(|m| match m {
                Message::DragOverGap(i) | Message::PluginDragOverGap(i) => Some(*i),
                _ => None,
            })
            .collect()
    };

    let a = label(crate::view(&app), &mut renderer, "Aim A", 0);
    let c = label(crate::view(&app), &mut renderer, "Aim C", 0);
    let (cache, messages) = frame(
        crate::view(&app),
        &mut renderer,
        Cache::new(),
        &moved(a),
        a,
        None,
    );
    assert!(messages.is_empty(), "{messages:?}");
    let (cache, messages) = frame(crate::view(&app), &mut renderer, cache, &moved(c), c, None);
    assert!(messages.is_empty(), "no drag, no message: {messages:?}");
    let (cache, _) = frame(crate::view(&app), &mut renderer, cache, &moved(a), a, None);

    let _ = crate::update::update_inner(&mut app, Message::DragStart(0));
    assert!(app.drag_state.is_some(), "the press armed a drag");
    let (cache, messages) = frame(crate::view(&app), &mut renderer, cache, &moved(a), a, None);
    assert!(aimed(&messages).is_empty(), "no false enter: {messages:?}");
    let (_, messages) = frame(crate::view(&app), &mut renderer, cache, &moved(c), c, None);
    assert_eq!(aimed(&messages), vec![2], "{messages:?}");

    let mut list = eidos_plugins::PluginList::default();
    for name in ["AimA.esp", "AimB.esp", "AimC.esp"] {
        list.plugins
            .push(crate::test_support::plugin_row(name, "Aim A"));
    }
    app.plugins = Some(list);
    let panel = || crate::modinfo::plugins_panel(&app);
    let a = label(panel(), &mut renderer, "AimA.esp", 0);
    let c = label(panel(), &mut renderer, "AimC.esp", 0);
    let (cache, _) = frame(panel(), &mut renderer, Cache::new(), &moved(a), a, None);
    let (cache, messages) = frame(panel(), &mut renderer, cache, &moved(c), c, None);
    assert!(
        aimed(&messages).is_empty(),
        "no plugin drag, no aim: {messages:?}"
    );
    let (cache, _) = frame(panel(), &mut renderer, cache, &moved(a), a, None);

    let _ = crate::update::update_inner(&mut app, Message::PluginDragStart(0));
    assert!(app.plugin_drag.is_some(), "the press armed a plugin drag");
    // The range line has a slot of its own above the list, so the press moves
    // no row: a row sliding under a still pointer aimed the drag (#120).
    let panel = || crate::modinfo::plugins_panel(&app);
    assert_eq!(
        label(panel(), &mut renderer, "AimA.esp", 0),
        a,
        "the drag hint moved the rows"
    );
    let (cache, messages) = frame(panel(), &mut renderer, cache, &moved(a), a, None);
    assert!(aimed(&messages).is_empty(), "no false enter: {messages:?}");
    let (_, messages) = frame(panel(), &mut renderer, cache, &moved(c), c, None);
    assert_eq!(aimed(&messages), vec![2], "{messages:?}");

    // Unchecking a master raises the missing-master line in that same slot, so
    // the rows stay put and a quick second click lands on the same checkbox.
    app.plugin_drag = None;
    if let Some(list) = app.plugins.as_mut() {
        list.plugins[1].masters = vec!["AimA.esp".to_string()];
        list.plugins[0].enabled = false;
        assert!(!list.missing_masters().is_empty());
    }
    assert_eq!(
        label(
            crate::modinfo::plugins_panel(&app),
            &mut renderer,
            "AimC.esp",
            0
        ),
        c,
        "the missing-master line moved the rows"
    );
}

/// The plugin filter only skips rows. A drawn row still names its full-list
/// index, and while the gaps between drawn rows are not load-order gaps no drag
/// starts or aims. Clearing the box brings back every row and the drag.
#[test]
fn the_plugin_filter_keeps_full_list_indices_and_holds_the_drag() {
    let _look = crate::test_support::look_lock();
    let (_dir, mut app, _archive) = installers::tests::fixture();
    let mut list = eidos_plugins::PluginList::default();
    for name in ["Alpha.esp", "Beta.esp", "Gamma.esp"] {
        list.plugins
            .push(crate::test_support::plugin_row(name, "Alpha"));
    }
    app.plugins = Some(list);
    let mut renderer = renderer();
    let panel = |app: &App| crate::modinfo::plugins_panel(app);
    let drawn = |app: &App, renderer: &mut iced::Renderer| -> Vec<String> {
        positions(panel(app), renderer)
            .labels
            .into_iter()
            .map(|(text, _)| text)
            .filter(|text| text.ends_with(".esp"))
            .collect()
    };
    let moved = |at: Point| [Event::Mouse(mouse::Event::CursorMoved { position: at })];
    let press = [Event::Mouse(mouse::Event::ButtonPressed(
        mouse::Button::Left,
    ))];
    let drag_message = |m: &Message| {
        matches!(
            m,
            Message::PluginDragStart(..) | Message::PluginDragOverGap(..) | Message::PluginDragDrop
        )
    };
    assert_eq!(
        drawn(&app, &mut renderer),
        ["Alpha.esp", "Beta.esp", "Gamma.esp"]
    );

    // Typing narrows the rows, ignoring case.
    let input = positions(panel(&app), &mut renderer).inputs[0].center();
    let (cache, _) = frame(
        panel(&app),
        &mut renderer,
        Cache::new(),
        &click(),
        input,
        None,
    );
    let typed = [key(
        iced::keyboard::Key::Character("g".into()),
        iced::keyboard::Modifiers::empty(),
        Some("g"),
    )];
    let (_, messages) = frame(panel(&app), &mut renderer, cache, &typed, input, None);
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::PluginSearchChanged(q) if q == "g")),
        "{messages:?}"
    );
    apply(&mut app, messages);
    assert_eq!(drawn(&app, &mut renderer), ["Gamma.esp"]);

    // Gamma is drawn first and still selects row 2, without arming a drag.
    let gamma = label(panel(&app), &mut renderer, "Gamma.esp", 0);
    let (_, messages) = frame(
        panel(&app),
        &mut renderer,
        Cache::new(),
        &click(),
        gamma,
        Some("plugins-filtered"),
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::SelectPlugin(2))),
        "{messages:?}"
    );
    assert!(!messages.iter().any(drag_message), "{messages:?}");
    apply(&mut app, messages);
    assert_eq!(app.selected_plugin, Some(2));
    assert!(app.plugin_drag.is_none(), "a filtered press armed a drag");
    let _ = crate::update::update_inner(&mut app, Message::PluginDragStart(2));
    assert!(app.plugin_drag.is_none(), "a drag started under a filter");

    // A drag armed before the box was filled aims nowhere either.
    app.plugin_search.clear();
    let _ = crate::update::update_inner(&mut app, Message::PluginDragStart(0));
    assert!(app.plugin_drag.is_some());
    app.plugin_search = "g".to_string();
    let (cache, _) = frame(
        panel(&app),
        &mut renderer,
        Cache::new(),
        &moved(input),
        input,
        None,
    );
    let (_, messages) = frame(
        panel(&app),
        &mut renderer,
        cache,
        &moved(gamma),
        gamma,
        None,
    );
    assert!(!messages.iter().any(drag_message), "{messages:?}");

    // Clearing the box restores every row and the drag.
    let _ = crate::update::update_inner(&mut app, Message::PluginSearchChanged(String::new()));
    assert!(app.plugin_drag.is_none(), "a filter change keeps no drag");
    assert_eq!(
        drawn(&app, &mut renderer),
        ["Alpha.esp", "Beta.esp", "Gamma.esp"]
    );
    let alpha = label(panel(&app), &mut renderer, "Alpha.esp", 0);
    let gamma = label(panel(&app), &mut renderer, "Gamma.esp", 0);
    let (cache, messages) = frame(
        panel(&app),
        &mut renderer,
        Cache::new(),
        &press,
        alpha,
        None,
    );
    apply(&mut app, messages);
    assert_eq!(app.selected_plugin, Some(0));
    assert!(app.plugin_drag.is_some(), "the press armed a drag again");
    let (_, messages) = frame(
        panel(&app),
        &mut renderer,
        cache,
        &moved(gamma),
        gamma,
        None,
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::PluginDragOverGap(2))),
        "{messages:?}"
    );
}

/// The #120 pattern: the dirty-plugin card's one slot holds the Clean button,
/// the add-xEdit line or the cleaning status, so a label below the card never
/// moves when a clean starts or ends under the pointer.
#[test]
fn the_dirty_plugin_card_keeps_one_height_with_xedit_without_it_and_while_cleaning() {
    let _look = crate::test_support::look_lock();
    let mut renderer = renderer();
    let names = vec!["Dirty.esp".to_string(), "Other.esp".to_string()];
    let mut below = |slot: DirtySlot| {
        let card = DirtyCard {
            names: names.clone(),
            slot,
        };
        let col = Column::new()
            .push(dirty_plugins_card(&card))
            .push(text("below the card"));
        label(col.into(), &mut renderer, "below the card", 0).y
    };
    let with_xedit = below(DirtySlot::Clean(2));
    assert_eq!(
        below(DirtySlot::AddXedit),
        with_xedit,
        "the add-xEdit line changed the card height"
    );
    let long = "A plugin name long enough to wrap onto a second line if the slot let it grow.esp"
        .to_string();
    assert_eq!(
        below(DirtySlot::Cleaning {
            plugin: long,
            left: 1
        }),
        with_xedit,
        "the cleaning status changed the card height"
    );
    // The slot is one tool_btn tall: the button is neither clipped nor padded out.
    let button = label(
        Column::new()
            .push(crate::widgets::tool_btn(
                "Clean all 2",
                Message::CleanPlugins(names.clone()),
            ))
            .push(text("below the button"))
            .into(),
        &mut renderer,
        "below the button",
        0,
    )
    .y - label(
        Column::new().push(text("below the button")).into(),
        &mut renderer,
        "below the button",
        0,
    )
    .y;
    assert!(
        (button - crate::theme::scaled(DIRTY_SLOT_H)).abs() < 1.0,
        "tool_btn is {button} px, the slot {} px",
        crate::theme::scaled(DIRTY_SLOT_H)
    );
}

/// The Nexus files tab of the 660 px mod info card: every file keeps its Open
/// button, the successor its Update button, each whole on one line and inside
/// the card. Fixed columns used to take 560 px before the buttons, which iced
/// then squeezed or cut off.
#[test]
fn nexus_file_buttons_fit_inside_the_mod_info_card() {
    let _look = crate::test_support::look_lock();
    let (_dir, mut app, _archive) = installers::tests::fixture();
    let folder = app.created.as_ref().unwrap().mods_dir().join("Sky UI");
    fs::create_dir_all(&folder).unwrap();
    fs::write(
        folder.join("meta.ini"),
        "[General]\nmodid=3863\nversion=5.1\n[installedFiles]\n1\\modid=3863\n1\\fileid=101\nsize=1\n",
    )
    .unwrap();
    app.mods = vec![ModEntry {
        name: "Sky UI".into(),
        enabled: true,
        path: folder,
        unmanaged: false,
    }];
    let file = |file_id, category_id, name: &str, version: &str| eidos_nexus::NexusFile {
        file_id,
        name: name.into(),
        file_name: format!("{name}.7z"),
        version: version.into(),
        category_id,
        size_in_bytes: 123_456_789,
        uploaded_timestamp: 1_760_000_000 + file_id,
    };
    let name = "SkyUI main file with a long descriptive name";
    app.nexus_files.insert(
        ("oblivion".into(), 3863),
        Ok(eidos_nexus::ModFiles {
            files: vec![file(101, 4, name, "5.1"), file(102, 1, name, "5.2")],
            updates: vec![(101, 102)],
        }),
    );
    app.info_mod = Some(0);
    app.info_tab = InfoTab::NexusFiles;
    let mut renderer = renderer();
    frame(
        crate::view(&app),
        &mut renderer,
        Cache::new(),
        &[],
        Point::ORIGIN,
        Some("nexus-files-tab"),
    );
    // The card alone, laid out from the left edge: 660 px wide, 16 px padding.
    let labels = positions(crate::modinfo::mod_info_dialog(&app, 0), &mut renderer).labels;
    let line = labels
        .iter()
        .find(|(s, _)| s == t("nexus_file_installed"))
        .map(|(_, b)| b.height)
        .unwrap();
    let buttons: Vec<Rectangle> = labels
        .iter()
        .filter(|(s, _)| s == t("nexus_file_open") || s == t("nexus_file_update"))
        .map(|(_, b)| *b)
        .collect();
    assert_eq!(buttons.len(), 3, "two Open and one Update: {labels:?}");
    for b in buttons {
        assert!(b.x + b.width <= 660.0 - 16.0, "past the card: {b:?}");
        assert!(b.height < line * 1.5, "wrapped: {b:?}");
    }
}

/// Collection notices arrive seconds after Check for updates, maybe while the
/// pointer is on a row. They go under the list, so no row moves when they do.
#[test]
fn collection_notices_never_move_the_mod_rows() {
    let _look = crate::test_support::look_lock();
    let (_dir, mut app, _archive) = installers::tests::fixture();
    let mods_dir = app.created.as_ref().unwrap().mods_dir();
    app.mods = ["Alpha", "Beta"]
        .into_iter()
        .map(|name| ModEntry {
            name: name.into(),
            enabled: true,
            path: mods_dir.join(name),
            unmanaged: false,
        })
        .collect();
    let mut renderer = renderer();
    let before = label(crate::view(&app), &mut renderer, "Beta", 0);
    let notice = CollectionNotice {
        slug: "abc".into(),
        domain: "oblivion".into(),
        name: "Some list".into(),
        have: 1,
        latest: 2,
        diff: None,
    };
    app.collection_notices = vec![notice.clone(), notice];
    frame(
        crate::view(&app),
        &mut renderer,
        Cache::new(),
        &[],
        Point::ORIGIN,
        Some("collection-notices"),
    );
    assert_eq!(label(crate::view(&app), &mut renderer, "Beta", 0), before);
    let drawn = positions(crate::view(&app), &mut renderer).labels;
    assert!(
        drawn.iter().any(|(s, _)| s.contains("Some list")),
        "the notice is drawn: {drawn:?}"
    );
}

/// The nxm status line is empty until the first read answers and changes after
/// every Register, so the button must not move with it.
#[test]
fn the_nxm_register_button_stays_put_when_the_status_line_changes() {
    let _look = crate::test_support::look_lock();
    let mut app = crate::test_support::nav_app(&[]);
    app.settings_open = true;
    app.settings_tab = SettingsTab::Nexus;
    app.settings_expanded.insert("downloads".into());
    let mut renderer = renderer();
    let register = t("settings_nxm_register");
    let before = label(crate::view(&app), &mut renderer, register, 0);
    app.nxm_handler = Some(eidos::nxm_handler::NxmHandler::Eidos {
        exec: "/home/someone/Applications/eidos-portable/bin/eidos-gui".into(),
    });
    assert_eq!(label(crate::view(&app), &mut renderer, register, 0), before);
}

/// The disable confirmation answers a click or Space on a row. It floats over
/// the list, so no row moves under the pointer, and a second click at the same
/// spot cancels it instead of toggling the row that slid there (#120).
#[test]
fn the_disable_confirmation_never_moves_the_mod_rows() {
    let _look = crate::test_support::look_lock();
    let (_dir, mut app, _archive) = installers::tests::fixture();
    let mods_dir = app.created.as_ref().unwrap().mods_dir();
    app.mods = ["Alpha", "Beta"]
        .into_iter()
        .map(|name| ModEntry {
            name: name.into(),
            enabled: true,
            path: mods_dir.join(name),
            unmanaged: false,
        })
        .collect();
    let mut renderer = renderer();
    let before = label(crate::view(&app), &mut renderer, "Beta", 0);
    app.disable_confirm = Some(DisableConfirm {
        targets: vec![0],
        plugins: (1..=9)
            .map(|n| format!("A long plugin name {n}.esp"))
            .collect(),
        dependents: Vec::new(),
    });
    assert_eq!(label(crate::view(&app), &mut renderer, "Beta", 0), before);
    let drawn = positions(crate::view(&app), &mut renderer).labels;
    assert!(
        drawn.iter().any(|(s, _)| s == t("disable_anyway")),
        "the confirmation is drawn: {drawn:?}"
    );
    let (_, messages) = frame(
        crate::view(&app),
        &mut renderer,
        Cache::new(),
        &click(),
        before,
        Some("disable-confirm"),
    );
    assert!(
        messages.iter().any(|m| matches!(m, Message::CancelDisable))
            && !messages.iter().any(|m| matches!(m, Message::ToggleMod(_))),
        "{messages:?}"
    );
}
