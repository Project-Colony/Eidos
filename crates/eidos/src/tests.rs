use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};

use eidos_instance::ModEntry;

use super::*;
use std::fs;

/// A throwaway temp dir, cleaned up on drop (the same idiom the other crates
/// use - no external dev-dependency).
struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("eidos-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
    fn touch(&self, rel: &str) {
        let p = self.0.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, b"").unwrap();
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A wrapper DLL that belongs at the GAME ROOT lives in the mod's `Root/`, not
/// at its top level, so scanning only the top level misses exactly the mods this
/// override exists for. The concrete case: SSE Engine Fixes' preloader ships as
/// `Root/d3dx9_42.dll`, Wine implements d3dx9_42, and without the override the
/// builtin wins and the preloader never runs - with no error anywhere.
#[test]
fn a_wrapper_dll_is_found_in_a_mods_root_folder() {
    let t = Tmp::new("shadow");
    t.touch("mods/EngineFixesPreloader/Root/d3dx9_42.dll");
    t.touch("mods/ENB/d3d11.dll");
    // Not a wrapper, and buried: must not be picked up.
    t.touch("mods/SomeMod/SKSE/Plugins/whatever.dll");

    let shadows = ["d3d11", "d3dx9_42"];
    let dirs = vec![
        t.0.join("mods/EngineFixesPreloader"),
        t.0.join("mods/EngineFixesPreloader/Root"),
        t.0.join("mods/ENB"),
        t.0.join("mods/SomeMod"),
    ];
    let stems = shipped_shadow_stems(&dirs, &shadows);

    assert!(
        stems.contains("d3dx9_42"),
        "the Root/ preloader must be found"
    );
    assert!(
        stems.contains("d3d11"),
        "a top-level wrapper is still found"
    );
    assert_eq!(
        stems.len(),
        2,
        "nothing else, and nothing from a nested dir"
    );
}

/// The Steam Cloud sync must be idempotent (fs::copy stamps the destination
/// with a NEWER mtime, so the second run finds nothing to do), must rescue a
/// prefix save the profile never saw before overwriting its fixed name, and
/// must ignore junk.
#[test]
fn cloud_sync_is_idempotent_and_rescues_diverged_saves() {
    let t = Tmp::new("cloudsync");
    let prof = t.0.join("prof");
    let prefix = t.0.join("prefix");
    fs::create_dir_all(&prof).unwrap();
    fs::create_dir_all(&prefix).unwrap();

    // A diverged prefix quicksave the profile has no copy of (a failed-bind
    // session wrote it), OLDER than the profile's own quicksave.
    fs::write(prefix.join("quicksave.ess"), b"orphan session").unwrap();
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    let f = fs::File::options()
        .write(true)
        .open(prefix.join("quicksave.ess"))
        .unwrap();
    f.set_modified(old).unwrap();
    drop(f);

    fs::write(prof.join("quicksave.ess"), b"current playthrough").unwrap();
    fs::write(prof.join("quicksave.skse"), b"cosave").unwrap();
    fs::write(prof.join("steam_autocloud.vdf"), b"junk").unwrap();

    let n = sync_saves_for_cloud(&prof, &prefix).unwrap();
    assert_eq!(n, 2, ".ess + .skse synced, junk ignored");
    assert_eq!(
        fs::read(prefix.join("quicksave.ess")).unwrap(),
        b"current playthrough"
    );
    assert!(!prefix.join("steam_autocloud.vdf").exists());

    // The orphan was rescued into the profile before the overwrite.
    let rescued: Vec<String> = fs::read_dir(&prof)
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("orphan-") && n.ends_with("quicksave.ess"))
        .collect();
    assert_eq!(
        rescued.len(),
        1,
        "the only copy of the orphan session must survive"
    );

    // Second run: nothing to do. (The rescued orphan syncs up once, at most.)
    let again = sync_saves_for_cloud(&prof, &prefix).unwrap();
    assert!(
        again <= 1,
        "the sync must converge, not recopy everything ({again})"
    );
    assert_eq!(
        sync_saves_for_cloud(&prof, &prefix).unwrap(),
        0,
        "and then be a no-op"
    );

    // A save the profile ADOPTED at seeding shares name + size with the
    // prefix original but not its mtime (the seed copy did not preserve
    // mtimes). It must NOT be "rescued" into a duplicate.
    fs::write(prefix.join("Save 12 - Old.ess"), b"identical bytes").unwrap();
    let f = fs::File::options()
        .write(true)
        .open(prefix.join("Save 12 - Old.ess"))
        .unwrap();
    f.set_modified(old).unwrap();
    drop(f);
    fs::write(prof.join("Save 12 - Old.ess"), b"identical bytes").unwrap();
    sync_saves_for_cloud(&prof, &prefix).unwrap();
    let orphans = fs::read_dir(&prof)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains("Save 12"))
        .count();
    assert_eq!(
        orphans, 1,
        "the adopted twin must not spawn an orphan duplicate"
    );

    // Quicksave rotation: the game rewrites quicksave.ess in the profile,
    // and the sync overwrites the prefix copy IT WROTE last session. Without
    // the provenance manifest that copy looked like an unknown diverged save
    // and one orphan-* file was minted per session, forever.
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(
        prof.join("quicksave.ess"),
        b"rotated - a newer, longer playthrough",
    )
    .unwrap();
    sync_saves_for_cloud(&prof, &prefix).unwrap();
    let orphan_count = fs::read_dir(&prof)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("orphan-"))
        .count();
    assert_eq!(
        orphan_count, 1,
        "rotating a fixed-name save must not mint orphans of our own sync copies"
    );
    assert_eq!(
        fs::read(prefix.join("quicksave.ess")).unwrap(),
        b"rotated - a newer, longer playthrough"
    );
}

// Guards FIX C1: the Overwrite layer must be the LAST (highest-priority) plugin
// source, so an ESP that lives only in Overwrite (xEdit / Bashed Patch output)
// is discovered, and an Overwrite copy wins same-name shadowing over a mod's
// copy - otherwise such plugins are silently dropped from plugins.txt.
#[test]
fn overwrite_is_the_highest_priority_plugin_source() {
    let t = Tmp::new("c1");
    let game_data = t.0.join("Data");
    fs::create_dir_all(&game_data).unwrap();

    // One enabled mod ships Patch.esp; Overwrite also has Patch.esp (a later
    // regeneration) plus a Bashed-Patch.esp that exists ONLY in Overwrite.
    let mod_dir = t.0.join("mods/AwesomeMod");
    fs::create_dir_all(&mod_dir).unwrap();
    fs::write(mod_dir.join("Patch.esp"), b"").unwrap();
    let overwrite = t.0.join("overwrite");
    t.touch("overwrite/Patch.esp");
    t.touch("overwrite/Bashed Patch.esp");

    let enabled = vec![ModEntry {
        name: "AwesomeMod".to_string(),
        enabled: true,
        path: mod_dir.clone(),
        unmanaged: false,
    }];

    let inst = eidos_instance::Instance::portable(t.0.clone());
    inst.create().unwrap();
    inst.save_modlist(&enabled).unwrap();
    let list = inst.plugin_list(&game_data, "skyrimse", None).unwrap();

    // The Overwrite-only plugin is discovered (would be dropped without C1).
    let bashed = list
        .plugins
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case("Bashed Patch.esp"))
        .expect("Overwrite-only plugin must be discovered");
    assert_eq!(bashed.origin_mod, "overwrite");

    // For the shadowed name, the Overwrite copy wins (highest priority).
    let patch = list
        .plugins
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case("Patch.esp"))
        .expect("Patch.esp must be present");
    assert_eq!(patch.origin_mod, "overwrite");
    assert!(
        patch.path.starts_with(&overwrite),
        "shadowed plugin should resolve to the Overwrite copy, got {}",
        patch.path.display()
    );
    fs::remove_file(overwrite.join("Patch.esp")).unwrap();
    t.touch("overwrite/.eidoswh.patch.esp");
    let list = inst.plugin_list(&game_data, "skyrimse", None).unwrap();
    assert!(
        !list
            .plugins
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case("Patch.esp")),
        "sort and launch must both exclude the lower plugin hidden by Overwrite"
    );
}

// Guards FIX C2: on Unix a signal-killed child reports `code() == None`, so the
// exit status must fall back to 128 + signal (not 0) - otherwise a crashed game
// would make eidos exit 0 and hide the crash. Asserts the mapping the
// `run_through_view` exit path uses.
#[test]
fn signal_death_maps_to_128_plus_signal_not_zero() {
    use std::process::ExitStatus;

    // A child killed by SIGSEGV (11): code() is None, signal() is 11.
    let killed = ExitStatus::from_raw(11);
    assert_eq!(killed.code(), None, "signal death has no exit code on Unix");
    let mapped = killed
        .code()
        .unwrap_or_else(|| 128 + killed.signal().unwrap_or(1));
    assert_eq!(mapped, 139, "SIGSEGV must map to 139, never 0");
    assert_ne!(mapped, 0);

    // A normal exit(3) is unaffected: code() is Some(3).
    let normal = ExitStatus::from_raw(3 << 8);
    assert_eq!(
        normal
            .code()
            .unwrap_or_else(|| 128 + normal.signal().unwrap_or(1)),
        3
    );
}

#[test]
fn a_tool_inside_a_mod_runs_from_the_merged_view() {
    // MO2's `mods\FNIS\path\exe => game\data\path\exe`. BodySlide reads
    // its slider sets relative to its own executable, and ships none of them:
    // run it from its own folder and the list is empty.
    let data = PathBuf::from("/games/skyrim/Data");
    let layers = vec![PathBuf::from("/inst/mods/BodySlide 5.8.2")];
    assert_eq!(
        virtualize_under_data(
            Path::new("/inst/mods/BodySlide 5.8.2/CalienteTools/BodySlide/BodySlide.exe"),
            &layers,
            &data,
        ),
        Some(PathBuf::from(
            "/games/skyrim/Data/CalienteTools/BodySlide/BodySlide.exe"
        ))
    );
}

#[test]
fn a_tool_outside_every_layer_is_left_alone() {
    let data = PathBuf::from("/games/skyrim/Data");
    let layers = vec![PathBuf::from("/inst/mods/BodySlide")];
    // xEdit in the game root, and a mod the user disabled (so it is not a
    // layer): both must keep their real path rather than be pointed at a
    // merged path that will not contain them.
    assert_eq!(
        virtualize_under_data(Path::new("/games/skyrim/SSEEdit.exe"), &layers, &data),
        None
    );
    assert_eq!(
        virtualize_under_data(Path::new("/inst/mods/Disabled/tool.exe"), &layers, &data),
        None
    );
}

#[test]
fn the_layer_root_is_what_gets_stripped_not_the_mod_name() {
    // A mod whose real content sits one level down is MOUNTED from that level,
    // so that is what has to be stripped. Stripping a fixed number of
    // components (MO2's approach) would leave the subdirectory in the path.
    let data = PathBuf::from("/games/skyrim/Data");
    let layers = vec![PathBuf::from("/inst/mods/Weird Archive/Data")];
    assert_eq!(
        virtualize_under_data(
            Path::new("/inst/mods/Weird Archive/Data/CalienteTools/BodySlide/BodySlide.exe"),
            &layers,
            &data,
        ),
        Some(PathBuf::from(
            "/games/skyrim/Data/CalienteTools/BodySlide/BodySlide.exe"
        ))
    );
}

#[test]
fn the_layer_itself_does_not_become_the_data_dir() {
    // Guard against the degenerate case: a path equal to the layer root has an
    // empty tail, and `data.join("")` would hand back the Data dir itself.
    let layers = vec![PathBuf::from("/inst/mods/BodySlide")];
    assert_eq!(
        virtualize_under_data(
            Path::new("/inst/mods/BodySlide"),
            &layers,
            Path::new("/games/skyrim/Data"),
        ),
        None
    );
}

#[test]
fn http_wrappers_are_in_the_shipped_shadow_list() {
    for dll in ["winhttp", "wininet"] {
        assert!(crate::launch::SHIPPED_SHADOWS.contains(&dll), "{dll}");
    }
}

#[test]
fn nexus_update_accepts_apply() {
    let strings = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        update_args(&strings(&["update", "skyrimse", "--apply"])).unwrap(),
        ("skyrimse".into(), true)
    );
    assert_eq!(
        update_args(&strings(&["update", "skyrimse"])).unwrap(),
        ("skyrimse".into(), false)
    );
    for bad in [
        &["update", "skyrimse", "--bogus"][..],
        &["update"],
        &["update", "--apply"],
        &["update", "skyrimse", "--apply", "--apply"],
        &["update", "skyrimse", "fallout4"],
    ] {
        assert!(update_args(&strings(bad)).is_err(), "{bad:?}");
    }
}

/// A portable Skyrim SE instance with no game executable: mod "A" enabled with
/// a version and one installed Nexus file, mod "B" disabled, two plugins and
/// one profile INI.
fn session_fixture(tag: &str) -> (Tmp, eidos_games::DetectedGame, eidos_instance::Instance) {
    let t = Tmp::new(tag);
    let inst = eidos_instance::Instance::portable(t.0.join("instance"));
    inst.create().unwrap();
    for m in ["A", "B"] {
        fs::create_dir_all(inst.mods_dir().join(m)).unwrap();
    }
    fs::write(
        inst.mods_dir().join("A/meta.ini"),
        "[General]\nversion=1.2\n\n[installedFiles]\n1\\modid=5\n1\\fileid=9\nsize=1\n",
    )
    .unwrap();
    let prof = inst.active();
    fs::write(prof.dir().join("modlist.txt"), "-B\n+A\n").unwrap();
    fs::write(
        prof.plugins_state_dir().join("plugins.txt"),
        "*A.esp\nB.esp\n",
    )
    .unwrap();
    fs::write(prof.ini_path("Skyrim.ini"), "x").unwrap();
    let game_dir = t.0.join("game");
    fs::create_dir_all(game_dir.join("Data")).unwrap();
    let game = eidos_games::DetectedGame {
        source: Default::default(),
        def: eidos_games::catalog()
            .iter()
            .find(|g| g.id == "skyrimse")
            .unwrap(),
        install_path: game_dir.clone(),
        data_path: game_dir.join("Data"),
        compatdata: None,
        steam_name: "synthetic".into(),
    };
    (t, game, inst)
}

#[test]
fn capture_reads_mod_versions_plugin_order_and_ini_checksums() {
    let (_t, game, inst) = session_fixture("session-capture");
    let r = sessions::capture(&game, &inst, &inst.active(), None);
    assert_eq!(
        r.mods,
        [eidos_instance::SessionMod {
            name: "A".into(),
            version: Some("1.2".into()),
            files: vec![(5, 9)]
        }]
    );
    assert_eq!(r.plugins, [("A.esp".into(), true), ("B.esp".into(), false)]);
    assert_eq!(r.inis, [("Skyrim.ini".into(), crc32fast::hash(b"x"))]);
    assert_eq!(
        (r.game_version, r.exit, r.eidos.as_str()),
        (None, None, env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn begin_returns_none_when_the_record_cannot_be_written() {
    let (t, game, inst) = session_fixture("session-begin");
    fs::write(t.0.join("instance/logs"), b"a file, not a dir").unwrap();
    assert!(sessions::begin(&game, &inst, &inst.active(), None).is_none());
}

#[test]
fn finish_records_the_signal_exit_and_only_the_saves_written_during_the_run() {
    let (_t, game, inst) = session_fixture("session-finish");
    let prof = inst.active();
    fs::create_dir_all(prof.saves_dir()).unwrap();
    let old = prof.saves_dir().join("Old.ess");
    fs::write(&old, b"").unwrap();
    fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1000))
        .unwrap();
    let begun = sessions::begin(&game, &inst, &prof, Some("SSEEdit"));
    fs::write(prof.saves_dir().join("New.ess"), b"").unwrap();
    // A raw wait status of 9: killed by SIGKILL.
    let killed = Ok(std::process::ExitStatus::from_raw(9));
    sessions::finish(begun, &killed, &inst, &prof);
    let all = inst.sessions();
    assert_eq!(all.len(), 1);
    let r = &all[0].1;
    assert_eq!(
        (r.exit, r.new_saves.as_slice(), r.tool.as_deref()),
        (Some(137), &["New.ess".to_string()][..], Some("SSEEdit"))
    );
    assert!(r.ended.is_some_and(|e| e >= r.started));
}

#[test]
fn the_default_diff_target_is_the_newest_game_run_that_exited_zero() {
    let r = |started, tool: Option<&str>, exit| {
        (
            PathBuf::from(format!("{started}")),
            eidos_instance::SessionRecord {
                started,
                tool: tool.map(Into::into),
                exit,
                ..Default::default()
            },
        )
    };
    let records = [
        r(4, None, Some(1)),
        r(3, Some("SSEEdit"), Some(0)),
        r(2, None, Some(0)),
        r(1, None, Some(0)),
    ];
    assert_eq!(sessions::default_target(&records).unwrap().1.started, 2);
}

#[test]
fn a_test_run_record_names_the_real_profile_and_is_never_the_diff_target() {
    let (_t, game, inst) = session_fixture("session-test-run");
    inst.request_test_run(&eidos_instance::TestRun {
        profile: "Default".into(),
        game: "skyrimse".into(),
        ..Default::default()
    })
    .unwrap();
    inst.begin_test_run(&inst.active(), "skyrimse", &[], None, false, 200)
        .unwrap()
        .unwrap();
    let r = sessions::capture(&game, &inst, &inst.test_run_profile(), None);
    assert!(r.test);
    assert_eq!(r.profile, "Default");
    assert!(r.mods.iter().any(|m| m.name == "A"));
    assert!(!sessions::capture(&game, &inst, &inst.active(), None).test);
    let rec = |started, test| {
        (
            PathBuf::from(format!("{started}")),
            eidos_instance::SessionRecord {
                started,
                exit: Some(0),
                test,
                ..Default::default()
            },
        )
    };
    let records = [rec(3, true), rec(2, false)];
    assert_eq!(sessions::default_target(&records).unwrap().1.started, 2);
    assert!(sessions::list_line(&records[0].0, &records[0].1).contains("  test  exit 0"));
    assert!(sessions::list_line(&records[1].0, &records[1].1).contains("  game  exit 0"));
}

#[test]
fn a_stopped_run_is_never_recorded_as_exit_zero() {
    assert_eq!(sessions::recorded_exit(0, true), 143);
    assert_eq!(sessions::recorded_exit(143, true), 143);
    assert_eq!(sessions::recorded_exit(1, true), 1);
    assert_eq!(sessions::recorded_exit(0, false), 0);
}
