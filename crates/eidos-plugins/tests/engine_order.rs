use eidos_plugins::{GameSpec, PluginList};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, UNIX_EPOCH},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "eidos-engine-order-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn plugin(root: &Path, name: &str, time: u64, game: &str, masters: &[&str]) {
    let morrowind = game == "morrowind";
    let master = name.ends_with(".esm");
    let mut body = b"HEDR".to_vec();
    if morrowind {
        body.extend(300u32.to_le_bytes());
        body.extend(1.3f32.to_le_bytes());
        body.extend(u32::from(master).to_le_bytes());
        body.extend([0; 292]);
    } else {
        body.extend(12u16.to_le_bytes());
        body.extend(1.0f32.to_le_bytes());
        body.extend([0; 8]);
    }
    for name in masters {
        body.extend(b"MAST");
        if morrowind {
            body.extend(((name.len() + 1) as u32).to_le_bytes());
        } else {
            body.extend(((name.len() + 1) as u16).to_le_bytes());
        }
        body.extend(name.as_bytes());
        body.push(0);
        body.extend(b"DATA");
        if morrowind {
            body.extend(8u32.to_le_bytes());
        } else {
            body.extend(8u16.to_le_bytes());
        }
        body.extend([0; 8]);
    }
    let mut b = vec![
        0;
        if morrowind {
            16
        } else if game == "oblivion" {
            20
        } else {
            24
        }
    ];
    b[..4].copy_from_slice(if morrowind { b"TES3" } else { b"TES4" });
    b[4..8].copy_from_slice(&(body.len() as u32).to_le_bytes());
    if !morrowind && master {
        b[8] = 1;
    }
    b.extend(body);
    fs::create_dir_all(root).unwrap();
    let path = root.join(name);
    fs::write(&path, b).unwrap();
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(time))
        .unwrap();
}
#[test]
fn timestamp_discovery_uses_engine_ties_and_preserves_saved_profile_order() {
    for id in ["morrowind", "oblivion", "fallout3", "falloutnv"] {
        let t = Temp::new();
        let spec = GameSpec::for_id(id).expect("timestamp engine spec");
        let data = t.0.join("Data");
        for (name, time) in [
            ("Z.esp", 10),
            ("A.esp", 10),
            ("Last.esp", 20),
            ("Master.esm", 30),
        ] {
            plugin(&data, name, time, id, &[]);
        }
        let mut list = PluginList::discover(&[(String::new(), data.clone())], &spec);
        list.refresh(&spec);
        assert_eq!(
            list.plugins
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["Master.esm", "Z.esp", "A.esp", "Last.esp"],
            "{id}"
        );
        let profile = t.0.join("profile");
        fs::create_dir(&profile).unwrap();
        fs::write(
            profile.join("loadorder.txt"),
            "Master.esm\nLast.esp\nA.esp\nZ.esp\n",
        )
        .unwrap();
        fs::write(
            profile.join(if id == "morrowind" {
                "Morrowind.ini"
            } else {
                "plugins.txt"
            }),
            if id == "morrowind" {
                "[Game Files]\nGameFile0=Master.esm\nGameFile1=Last.esp\n"
            } else {
                "Master.esm\nLast.esp\n"
            },
        )
        .unwrap();
        list.apply_prefix_state(&profile, &spec);
        list.refresh(&spec);
        assert_eq!(
            list.plugins
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["Master.esm", "Last.esp", "A.esp", "Z.esp"]
        );
        assert!(
            !list
                .plugins
                .iter()
                .find(|p| p.name == "A.esp")
                .unwrap()
                .enabled
        );
    }
}
#[test]
fn enderal_required_primaries_are_explicit_and_missing_requirements_are_diagnosed() {
    let t = Temp::new();
    for name in [
        "Skyrim.esm",
        "Update.esm",
        "Enderal - Forgotten Stories.esm",
        "SkyUI_SE.esp",
    ] {
        plugin(&t.0, name, 1, "skyrimse", &[]);
    }
    let spec = GameSpec::for_id("enderalse").unwrap();
    assert!(spec.primary_plugins.iter().any(|n| n == "SkyUI_SE.esp"));
    let mut list = PluginList::discover(&[(String::new(), t.0.clone())], &spec);
    list.refresh(&spec);
    let profile = t.0.join("profile");
    list.write_load_order(&profile, &spec).unwrap();
    let text = eidos_plugins::read_decoded(&profile.join("plugins.txt")).unwrap();
    assert_eq!(text.matches("*SkyUI_SE.esp").count(), 1);
    assert_eq!(text.matches("*Enderal - Forgotten Stories.esm").count(), 1);
    assert!(!text.contains("Skyrim.esm"));
    list.plugins.retain(|p| p.name != "SkyUI_SE.esp");
    assert!(list
        .diagnostics(&spec)
        .iter()
        .any(|d| d.plugin == "SkyUI_SE.esp" && d.code == "missing_primary"));
}
#[test]
fn morrowind_activation_preserves_ini_sections_and_refuses_lossy_names() {
    let t = Temp::new();
    plugin(&t.0, "Café.esp", 1, "morrowind", &[]);
    let spec = GameSpec::for_id("morrowind").unwrap();
    let list = PluginList::discover(&[(String::new(), t.0.clone())], &spec);
    let profile = t.0.join("profile");
    fs::create_dir(&profile).unwrap();
    fs::write(profile.join("Morrowind.ini"),b"; keep\r\n[General]\r\nKey=value\r\n[Game Files]\r\nGameFile0=Old.esp\r\n; comment\r\nOther=keep\r\n[Archives]\r\nArchive 0=Foo.bsa\r\n").unwrap();
    list.write_load_order(&profile, &spec).unwrap();
    let text = eidos_plugins::read_decoded(&profile.join("Morrowind.ini")).unwrap();
    for preserved in [
        "Key=value",
        "; comment",
        "Other=keep",
        "Archive 0=Foo.bsa",
        "GameFile0=Café.esp",
    ] {
        assert!(text.contains(preserved), "{text}");
    }
    assert!(!text.contains("Old.esp"));
    assert_eq!(
        PluginList::read_active(&profile, &spec),
        [("Café.esp".into(), true)]
    );
    let original = fs::read(profile.join("Morrowind.ini")).unwrap();
    let mut bad = list;
    bad.plugins[0].name = "不可.esp".into();
    assert!(bad.write_load_order(&profile, &spec).is_err());
    assert_eq!(fs::read(profile.join("Morrowind.ini")).unwrap(), original);
}
#[test]
fn oblivion_state_location_obeys_the_install_root_flag() {
    let t = Temp::new();
    let prefix = t.0.join("prefix");
    let spec = GameSpec::for_id("oblivion").unwrap();
    let ordinary = eidos_plugins::plugins_txt_dir(&prefix, &spec);
    assert_eq!(
        eidos_plugins::plugin_state_dir(&prefix, &t.0, &spec),
        ordinary
    );
    fs::write(
        t.0.join("Oblivion.ini"),
        "[General]\nbUseMyGamesDirectory=0\n",
    )
    .unwrap();
    assert_eq!(eidos_plugins::plugin_state_dir(&prefix, &t.0, &spec), t.0);
    fs::write(
        t.0.join("Oblivion.ini"),
        "[General]\nbUseMyGamesDirectory=1\n",
    )
    .unwrap();
    assert_eq!(
        eidos_plugins::plugin_state_dir(&prefix, &t.0, &spec),
        ordinary
    );
}
#[test]
fn timestamp_order_matches_libloadorder_without_changing_sources() {
    for (id, oracle_id) in [
        ("morrowind", loadorder::GameId::Morrowind),
        ("oblivion", loadorder::GameId::Oblivion),
        ("fallout3", loadorder::GameId::Fallout3),
        ("falloutnv", loadorder::GameId::FalloutNV),
    ] {
        let t = Temp::new();
        let data = t.0.join(if id == "morrowind" {
            "Data Files"
        } else {
            "Data"
        });
        let spec = GameSpec::for_id(id).unwrap();
        for (name, time) in [
            (spec.primary_plugins[0].as_str(), 5),
            ("Z.esp", 10),
            ("A.esp", 10),
            ("Master.esm", 40),
            ("Dependent.esp", 1),
        ] {
            plugin(
                &data,
                name,
                time,
                id,
                if name == "Dependent.esp" {
                    &["Master.esm"]
                } else {
                    &[]
                },
            );
        }
        let local = t.0.join("local");
        fs::create_dir(&local).unwrap();
        let active = spec.primary_plugins[0].clone() + "\nZ.esp\nDependent.esp\nMaster.esm\n";
        fs::write(local.join("Plugins.txt"), &active).unwrap();
        fs::write(
            t.0.join("Morrowind.ini"),
            format!(
                "[Game Files]\n{}",
                active
                    .lines()
                    .enumerate()
                    .map(|(i, n)| format!("GameFile{i}={n}\n"))
                    .collect::<String>()
            ),
        )
        .unwrap();
        let settings = loadorder::GameSettings::with_local_path(oracle_id, &t.0, &local).unwrap();
        let mut oracle = settings.into_load_order();
        oracle.load().unwrap();
        let mut ours = PluginList::discover(&[(String::new(), data.clone())], &spec);
        ours.apply_prefix_state(if id == "morrowind" { &t.0 } else { &local }, &spec);
        ours.refresh(&spec);
        assert_eq!(
            ours.plugins
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            oracle.plugin_names(),
            "{id}"
        );
        assert_eq!(
            ours.plugins
                .iter()
                .filter(|p| p.enabled)
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            oracle.active_plugin_names(),
            "{id}"
        );
        let before = ours
            .plugins
            .iter()
            .map(|p| {
                (
                    p.path.clone(),
                    fs::metadata(&p.path).unwrap().modified().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let times = ours.virtual_mtimes(&spec).unwrap();
        assert_eq!(times.len(), ours.plugins.len());
        assert_eq!(
            times
                .values()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            ours.plugins.len()
        );
        for (path, time) in &before {
            assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), *time);
        }
        // libloadorder save deliberately changes physical times, so run its oracle
        // only on a second synthetic game tree, never on an Eidos source path.
        let copy = t.0.join("oracle-copy");
        let copy_data = copy.join(if id == "morrowind" {
            "Data Files"
        } else {
            "Data"
        });
        fs::create_dir_all(&copy_data).unwrap();
        let copy_local = copy.join("local");
        fs::create_dir(&copy_local).unwrap();
        for (path, time) in &before {
            let dest = copy_data.join(path.file_name().unwrap());
            fs::copy(path, &dest).unwrap();
            fs::File::options()
                .write(true)
                .open(dest)
                .unwrap()
                .set_modified(*time)
                .unwrap();
        }
        fs::copy(local.join("Plugins.txt"), copy_local.join("Plugins.txt")).unwrap();
        fs::copy(t.0.join("Morrowind.ini"), copy.join("Morrowind.ini")).unwrap();
        let mut save_oracle =
            loadorder::GameSettings::with_local_path(oracle_id, &copy, &copy_local)
                .unwrap()
                .into_load_order();
        save_oracle.load().unwrap();
        save_oracle
            .set_load_order(
                &ours
                    .plugins
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        save_oracle.save().unwrap();
        for p in &ours.plugins {
            assert_eq!(
                times[&p.name.to_ascii_lowercase()],
                fs::metadata(copy_data.join(&p.name))
                    .unwrap()
                    .modified()
                    .unwrap(),
                "{id}: {}",
                p.name
            );
        }
        for (path, time) in before {
            assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), time);
        }
    }
}

#[test]
fn empty_timestamp_active_file_disables_nonprimary_plugins() {
    for id in ["morrowind", "oblivion", "fallout3", "falloutnv"] {
        let t = Temp::new();
        plugin(&t.0, "Disabled.esp", 1, id, &[]);
        let spec = GameSpec::for_id(id).unwrap();
        let mut list = PluginList::discover(&[(String::new(), t.0.clone())], &spec);
        fs::write(
            t.0.join(spec.active_file()),
            if id == "morrowind" {
                "[Game Files]\n"
            } else {
                "# no active plugins\n"
            },
        )
        .unwrap();
        list.apply_prefix_state(&t.0, &spec);
        list.refresh(&spec);
        assert!(!list.plugins[0].enabled, "{id}");
    }
}
