use std::{
    fs,
    process::Command,
    thread,
    time::{Duration, Instant},
};

#[test]
fn a_fresh_registry_loads_custom_games_and_inherits_builtin_tools() {
    const CHILD: &str = "EIDOS_GAMEDEF_REGISTRY_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let custom = eidos_gamedef::GameDef::for_id("custom").unwrap();
        assert_eq!(custom.name, "Custom Game");
        let skyrim = eidos_gamedef::GameDef::for_id("skyrimse").unwrap();
        assert_eq!(skyrim.name, "Custom Skyrim");
        assert!(!skyrim.known_tools.is_empty());
        assert_eq!(
            eidos_gamedef::all()
                .iter()
                .filter(|g| g.id == "skyrimse")
                .count(),
            1
        );
        return;
    }

    // A child starts with an empty OnceLock without changing this test process's
    // environment or depending on the order in which other tests use it.
    let dir = std::env::temp_dir().join(format!("eidos-gamedef-registry-{}", std::process::id()));
    let games = dir.join("Colony/Eidos/games");
    fs::create_dir_all(&games).unwrap();
    for (id, name) in [("custom", "Custom Game"), ("skyrimse", "Custom Skyrim")] {
        fs::write(
            games.join(format!("{id}.toml")),
            format!("id = '{id}'\nname = '{name}'\nsteam_app_id = 1\ndata_dir = 'Data'\n"),
        )
        .unwrap();
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "a_fresh_registry_loads_custom_games_and_inherits_builtin_tools",
        ])
        .env(CHILD, "1")
        .env("XDG_CONFIG_HOME", &dir)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            break None;
        }
        thread::sleep(Duration::from_millis(10));
    };
    fs::remove_dir_all(&dir).unwrap();
    assert!(
        status.is_some_and(|s| s.success()),
        "loading user games hung or failed"
    );
}
