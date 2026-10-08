use std::fs;

#[test]
fn a_whiteouted_plugin_is_missing_from_collection_activation() {
    let root = std::env::temp_dir().join(format!("eidos-collection-plugin-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let inst = eidos_instance::Instance::portable(root.join("instance"));
    inst.create().unwrap();
    let def = eidos_games::catalog()
        .iter()
        .find(|g| g.id == "skyrimse")
        .unwrap();
    let game = eidos_games::DetectedGame {
        source: Default::default(),
        def,
        install_path: root.join("game"),
        data_path: root.join("game/Data"),
        compatdata: Some(root.join("compatdata")),
        steam_name: "synthetic".into(),
    };
    fs::create_dir_all(&game.data_path).unwrap();
    fs::write(game.data_path.join("Hidden.esp"), []).unwrap();
    fs::write(game.data_path.join("Visible.esp"), []).unwrap();
    fs::create_dir_all(inst.overwrite_dir()).unwrap();
    fs::write(inst.overwrite_dir().join(".eidoswh.Hidden.esp"), []).unwrap();
    let collection = eidos_collections::manifest::Collection {
        plugins: vec![
            eidos_collections::manifest::Plugin {
                name: "Hidden.esp".into(),
                enabled: true,
            },
            eidos_collections::manifest::Plugin {
                name: "Visible.esp".into(),
                enabled: false,
            },
        ],
        ..Default::default()
    };
    let mut report = eidos_collections::report::Report::default();
    eidos_collections::driver::apply_plugin_states(&inst, &game, &collection, &mut report);
    assert!(
        report
            .loot_notes
            .iter()
            .any(|note| note.subject == "Hidden.esp" && note.detail.contains("does not have it")),
        "{:?}",
        report.loot_notes
    );
    let list = inst.plugin_list(&game.data_path, def.id, None).unwrap();
    assert!(!list.plugins.iter().any(|p| p.name == "Hidden.esp"));
    assert!(
        !list
            .plugins
            .iter()
            .find(|p| p.name == "Visible.esp")
            .unwrap()
            .enabled
    );
    assert!(game.data_path.join("Hidden.esp").is_file());
    assert!(inst.overwrite_dir().join(".eidoswh.Hidden.esp").is_file());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_store_plugin_activation_uses_actual_wine_prefix() {
    use eidos_games::{GameSource, Store};
    let root =
        std::env::temp_dir().join(format!("eidos-collection-external-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let inst = eidos_instance::Instance::portable(root.join("instance"));
    inst.create().unwrap();
    let def = eidos_games::catalog()
        .iter()
        .find(|g| g.id == "skyrimse")
        .unwrap();
    let game = eidos_games::DetectedGame {
        source: GameSource::External {
            store: Store::Gog,
            app_id: "fixture".into(),
            prefix: Some(root.join("wine")),
            heroic: true,
        },
        def,
        install_path: root.join("game"),
        data_path: root.join("game/Data"),
        compatdata: None,
        steam_name: "fixture".into(),
    };
    fs::create_dir_all(&game.data_path).unwrap();
    fs::write(game.data_path.join("Visible.esp"), []).unwrap();
    let collection = eidos_collections::Collection {
        plugins: vec![eidos_collections::manifest::Plugin {
            name: "Visible.esp".into(),
            enabled: false,
        }],
        ..Default::default()
    };
    let mut report = Default::default();
    eidos_collections::driver::apply_plugin_states(&inst, &game, &collection, &mut report);
    assert!(report.loot_notes.is_empty(), "{report:?}");
    assert!(
        !inst
            .plugin_list(&game.data_path, def.id, None)
            .unwrap()
            .plugins
            .iter()
            .find(|p| p.name == "Visible.esp")
            .unwrap()
            .enabled
    );
    fs::remove_dir_all(root).unwrap();
}

/// A root-mode game on a never-launched profile, with `A.esp` active and
/// `B.esp` deliberately off in the install-root state, and a collection that
/// switches `A.esp` off.
fn root_mode_fixture(
    tag: &str,
    id: &str,
    data: &str,
    files: &[(&str, &str)],
) -> (std::path::PathBuf, eidos_instance::Instance, eidos_games::DetectedGame) {
    let root = std::env::temp_dir().join(format!("eidos-collection-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let inst = eidos_instance::Instance::portable(root.join("instance"));
    inst.create().unwrap();
    let def = eidos_games::catalog().iter().find(|g| g.id == id).unwrap();
    let game = eidos_games::DetectedGame {
        source: Default::default(),
        def,
        install_path: root.join("game"),
        data_path: root.join("game").join(data),
        compatdata: Some(root.join("compatdata")),
        steam_name: "synthetic".into(),
    };
    fs::create_dir_all(&game.data_path).unwrap();
    for (name, body) in files {
        fs::write(game.install_path.join(name), body).unwrap();
    }
    let collection = eidos_collections::Collection {
        plugins: vec![eidos_collections::manifest::Plugin {
            name: "A.esp".into(),
            enabled: false,
        }],
        ..Default::default()
    };
    let mut report = Default::default();
    for name in ["A.esp", "B.esp"] {
        fs::write(game.data_path.join(name), []).unwrap();
    }
    eidos_collections::driver::apply_plugin_states(&inst, &game, &collection, &mut report);
    (root, inst, game)
}

#[test]
fn morrowind_collection_seeds_the_install_ini_instead_of_writing_a_stub() {
    let ini = "[General]\r\nSubtitles=1\r\n[Archives]\r\nArchive 0=Tribunal.bsa\r\n\
               [Game Files]\r\nGameFile0=A.esp\r\n";
    let (root, inst, game) =
        root_mode_fixture("morrowind", "morrowind", "Data Files", &[("Morrowind.ini", ini)]);
    let profile_ini =
        fs::read_to_string(inst.active().plugins_state_dir().join("Morrowind.ini")).unwrap();
    // The real INI's other sections survive, and B.esp stays off as the user left it.
    assert!(profile_ini.contains("Archive 0=Tribunal.bsa"), "{profile_ini}");
    assert!(profile_ini.contains("[General]"), "{profile_ini}");
    assert!(!profile_ini.contains("A.esp") && !profile_ini.contains("B.esp"), "{profile_ini}");
    // The install root is the state dir here; the prefix shadow must not touch it.
    assert_eq!(fs::read_to_string(game.install_path.join("Morrowind.ini")).unwrap(), ini);
    assert!(!game.install_path.join("loadorder.txt").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn oblivion_root_mode_collection_seeds_the_install_root_plugins_txt() {
    let ini = "[General]\r\nbUseMyGamesDirectory=0\r\n";
    let (root, inst, game) = root_mode_fixture(
        "oblivion-root",
        "oblivion",
        "Data",
        &[("Oblivion.ini", ini), ("plugins.txt", "A.esp\r\n")],
    );
    let list = inst.plugin_list(&game.data_path, game.def.id, None).unwrap();
    // B.esp was off in the install-root plugins.txt; founding on the empty
    // AppData dir turned it on.
    assert!(list.plugins.iter().all(|p| !p.enabled), "{:?}", list.plugins);
    assert_eq!(fs::read_to_string(game.install_path.join("plugins.txt")).unwrap(), "A.esp\r\n");
    fs::remove_dir_all(root).unwrap();
}
