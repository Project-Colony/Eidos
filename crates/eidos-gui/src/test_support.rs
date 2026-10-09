//! Fixtures the GUI's test modules share: throwaway instances on disk, an `App`
//! opened on one, and the small builders most tests start from.

use crate::*;

pub(crate) fn mods(names: &[&str]) -> Vec<ModEntry> {
    names
        .iter()
        .map(|n| ModEntry {
            name: n.to_string(),
            enabled: true,
            path: PathBuf::new(),
            unmanaged: false,
        })
        .collect()
}

/// A plugin row for the menu tests: name plus the mod that ships it.
pub(crate) fn plugin_row(name: &str, origin: &str) -> eidos_plugins::Plugin {
    eidos_plugins::Plugin {
        name: name.to_string(),
        origin_mod: origin.to_string(),
        path: PathBuf::new(),
        enabled: true,
        force_disabled: false,
        header_error: None,
        form_version: Some(44),
        is_blueprint: false,
        is_update: false,
        is_master: false,
        is_light: false,
        is_medium: false,
        masters: Vec::new(),
        priority: 0,
        index: None,
    }
}

pub(crate) fn names(v: &[ModEntry]) -> Vec<&str> {
    v.iter().map(|m| m.name.as_str()).collect()
}

/// An App with just enough filled in to drive `key_nav`.
/// An App with just enough filled in to drive `key_nav`, and NO instance.
///
/// `created` is what every save path writes through, so a test that leaves
/// it pointing at a real instance can reach the user's files. `new` refuses
/// to attach one under `cfg(test)`; this asserts it, because the guard being
/// silently lost is exactly the failure that would not be noticed until
/// somebody's mod list was four entries long.
pub(crate) fn nav_app(mod_names: &[&str]) -> App {
    let mut app = new(Vec::new()).0;
    assert!(
        app.created.is_none(),
        "a test App must never hold a real instance"
    );
    app.mods = mods(mod_names);
    app.screen = Screen::Main;
    app
}

/// An App with one game selected, built without touching the disk.
pub(crate) fn app_for_game(id: &str) -> App {
    let mut app = nav_app(&[]);
    app.games = vec![DetectedGame {
        def: eidos_games::GameDef::for_id(id).expect("a game in the catalog"),
        install_path: PathBuf::from("/nowhere"),
        data_path: PathBuf::from("/nowhere/data"),
        compatdata: None,
        source: Default::default(),
        steam_name: id.to_string(),
    }];
    app.selected = Some(0);
    app
}

/// A real portable instance in a temp dir: manifest + the minimum layout.
pub(crate) fn temp_portable(game_id: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    // Named after the test that asked for it. The harness names each test
    // thread after the test, and an instance lock refusal prints the ROOT -
    // so when one test trips over another's lock, the message says which
    // test owns it instead of leaving a bare number to correlate by hand.
    let owner = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .rsplit("::")
        .next()
        .unwrap_or("unnamed")
        .to_string();
    let root = std::env::temp_dir().join(format!(
        "eidos-portable-{}-{}-{}",
        std::process::id(),
        owner,
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("mods")).unwrap();
    eidos_instance::Manifest::new(game_id, InstanceKind::Portable)
        .write(&root.join("eidos-instance.ini"))
        .unwrap();
    root
}

/// Four mods and a separator, with an instance so meta reads work.
pub(crate) fn list_app(names: &[&str]) -> (App, PathBuf) {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    for n in names {
        fs::create_dir_all(root.join("mods").join(n)).unwrap();
    }
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.mods = names
        .iter()
        .map(|n| ModEntry {
            name: (*n).to_string(),
            enabled: true,
            path: root.join("mods").join(n),
            unmanaged: false,
        })
        .collect();
    app.screen = Screen::Main;
    refresh_meta_cache(&mut app);
    (app, root)
}

/// An App with a real portable instance, one mod, and a game Data dir - the
/// minimum the Data tab needs to build a real `LayerStack`.
pub(crate) fn data_app(
    mod_files: &[(&str, &str)],
    overwrite: &[(&str, &str)],
) -> (App, PathBuf) {
    let root = temp_portable("skyrimse");
    let game = root.join("game/Data");
    fs::create_dir_all(&game).unwrap();
    fs::write(game.join("Skyrim.esm"), b"vanilla").unwrap();
    let modroot = root.join("mods/AAA");
    for (rel, body) in mod_files {
        let p = modroot.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body.as_bytes()).unwrap();
    }
    let ow = root.join("overwrite");
    for (rel, body) in overwrite {
        let p = ow.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body.as_bytes()).unwrap();
    }
    let mut app = app_for_game("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    if let Some(g) = app.games.first_mut() {
        g.data_path = game;
    }
    app.mods = vec![ModEntry {
        name: "AAA".into(),
        enabled: true,
        path: modroot,
        unmanaged: false,
    }];
    // Written to disk, because the tab now reads the SAME layer stack the
    // mount is handed - `load_order()`, not the window's in-memory list.
    inst.save_modlist(&app.mods).unwrap();
    app.created = Some(inst);
    app.screen = Screen::Main;
    (app, root)
}

/// Build an instance whose downloads dir holds the given `(name, bytes)`
/// entries, then scan it. Real files, because the whole feature is "notice
/// what another process is writing to disk".
pub(crate) fn downloads_app(files: &[(&str, &[u8])], metas: &[(&str, &str)]) -> App {
    let mut app = nav_app(&[]);
    // A counter, not a timestamp. `cargo test` runs these on parallel
    // threads, and two of them reading the clock in the same instant got the
    // same directory - one test then saw the other's files and failed, but
    // only sometimes and never alone. A counter cannot collide.
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "eidos-dl-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let dl = root.join("downloads");
    fs::create_dir_all(&dl).unwrap();
    for (n, b) in files {
        fs::write(dl.join(n), b).unwrap();
    }
    for (n, body) in metas {
        fs::write(dl.join(n), body).unwrap();
    }
    app.created = Some(eidos_instance::Instance::portable(root));
    load_downloads(&mut app);
    app
}
