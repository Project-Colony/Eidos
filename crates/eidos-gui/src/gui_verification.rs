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

#[test]
fn installer_widgets_emit_pointer_and_keyboard_events_and_publish_exact_choice() {
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
