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
