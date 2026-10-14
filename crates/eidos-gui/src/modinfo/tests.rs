use super::*;
use crate::test_support::*;

#[test]
fn archive_activation_rejects_sse_arbitrary_suffix_and_merges_ini_keys() {
    let (mut app, root) = data_app(
        &[
            ("A.esp", "plugin"),
            ("A - Scripts.bsa", "archive"),
            ("A - Textures.bsa", "archive"),
            ("Base.bsa", "archive"),
        ],
        &[],
    );
    let mut plugins = PluginList::default();
    plugins.plugins = vec![plugin_row("A.esp", "AAA")];
    plugins.plugins[0].enabled = true;
    app.plugins = Some(plugins);
    let profile = app.created.as_ref().unwrap().active();
    fs::write(
        profile.ini_path("Skyrim.ini"),
        "[Archive]\nsResourceArchiveList=Base.bsa\n",
    )
    .unwrap();
    fs::write(
        profile.ini_path("SkyrimCustom.ini"),
        "[Archive]\nsResourceArchiveList=\n",
    )
    .unwrap();
    let rows = archive_rows(&app, "skyrimse").unwrap();
    assert!(!rows
        .iter()
        .find(|r| r.archive == "A - Scripts.bsa")
        .unwrap()
        .loaded());
    assert!(rows
        .iter()
        .find(|r| r.archive == "A - Textures.bsa")
        .unwrap()
        .loaded());
    assert!(
        !rows
            .iter()
            .find(|r| r.archive == "Base.bsa")
            .unwrap()
            .loaded(),
        "custom empty list replaces base list"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_bundle_of_variants_says_the_mod_is_inside_one_of_them() {
    use eidos_install::{ArchiveEntry, ArchiveTree};
    let rows = |paths: &[&str]| {
        ArchiveTree::from_entries(
            &paths
                .iter()
                .map(|p| ArchiveEntry {
                    path: p.trim_end_matches('/').to_string(),
                    is_dir: p.ends_with('/'),
                })
                .collect::<Vec<_>>(),
        )
        .flatten()
    };

    // The real shape of EVE_Sunrise_Dress.7z: screenshots, a readme, and two
    // zips that each hold one variant of the mod. No level of it can ever look
    // valid, so without this the dialog just repeats "does NOT look valid"
    // wherever the user clicks.
    let hint = nested_archive_hint(&rows(&[
        "EVE Sunrise Dress/1 (1).jpg",
        "EVE Sunrise Dress/1 (2).jpg",
        "EVE Sunrise Dress/Full replaces planet diving suit-704.zip",
        "EVE Sunrise Dress/No back accessories-704.zip",
        "EVE Sunrise Dress/readme.txt",
    ]))
    .expect("a bundle of variants is worth naming");
    assert!(hint.contains('2'), "it should say how many: {hint}");

    // One inner archive is the singular case, not "0 archives".
    let one = nested_archive_hint(&rows(&["Mod/inner.rar"])).expect("one nested archive");
    assert!(!one.contains('2'));

    // And an ordinary mod says nothing at all: the hint is for the dead end,
    // not a remark on every archive that fails the check.
    assert_eq!(
        nested_archive_hint(&rows(&["Mod/thing_P.pak", "Mod/notes.txt"])),
        None
    );
    // A directory that merely ends in an archive extension is not one.
    assert_eq!(nested_archive_hint(&rows(&["Mod/backup.zip/x.pak"])), None);
}

#[test]
fn a_game_without_plugins_gets_no_plugin_diagnostics() {
    let titles =
        |app: &App| -> Vec<String> { diagnostics(app).into_iter().map(|d| d.title).collect() };
    let sb = titles(&app_for_game("stellarblade"));
    for needle in ["LOOT", "Load order", "load order"] {
        assert!(
            !sb.iter().any(|t| t.contains(needle)),
            "Stellar Blade was given plugin advice: {sb:?}"
        );
    }
    // Morrowind's newly supported sorter must not retain obsolete warnings.
    let mw = titles(&app_for_game("morrowind"));
    assert!(
        !mw.iter().any(|t| t.contains("LOOT cannot sort")),
        "Morrowind retained obsolete sorter advice: {mw:?}"
    );
}

#[test]
fn send_to_separator_will_not_send_a_separator_into_itself() {
    let app = nav_app(&["A_separator", "a", "B_separator", "b"]);
    assert_eq!(
        separator_choices(&app, 0),
        vec![2],
        "a header is not a destination for itself"
    );
    assert_eq!(separator_choices(&app, 2), vec![0]);
    assert_eq!(
        separator_choices(&app, 1),
        vec![0, 2],
        "an ordinary mod may go anywhere"
    );
}

#[test]
fn a_plugins_tab_edit_on_a_fresh_morrowind_profile_keeps_the_real_ini() {
    let root = temp_portable("morrowind");
    let inst = Instance::portable(root.join("instance"));
    inst.create().unwrap();
    let mut app = app_for_game("morrowind");
    app.games[0].install_path = root.join("game");
    app.games[0].data_path = root.join("game/Data Files");
    let game = app.games[0].clone();
    fs::create_dir_all(&game.data_path).unwrap();
    for name in ["A.esp", "B.esp"] {
        fs::write(game.data_path.join(name), []).unwrap();
    }
    let ini = "[General]\r\nSubtitles=1\r\n[Game Files]\r\nGameFile0=A.esp\r\nGameFile1=B.esp\r\n";
    fs::write(game.install_path.join("Morrowind.ini"), ini).unwrap();
    app.created = Some(inst.clone());
    let spec = game.plugin_spec().unwrap();
    let mut list = inst
        .plugin_list(&game.data_path, "morrowind", game.plugin_state_dir().as_deref())
        .unwrap();
    assert!(list.set_enabled("B.esp", false));
    write_plugin_state(&app, &list, &spec).unwrap();
    let written =
        fs::read_to_string(inst.active().plugins_state_dir().join("Morrowind.ini")).unwrap();
    assert!(written.contains("Subtitles=1"), "{written}");
    assert!(written.contains("GameFile0=A.esp") && !written.contains("B.esp"), "{written}");
    assert_eq!(fs::read_to_string(game.install_path.join("Morrowind.ini")).unwrap(), ini);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_plugin_edit_in_a_stale_window_does_not_revert_another_process_sort() {
    let root = temp_portable("morrowind");
    let inst = Instance::portable(root.join("instance"));
    inst.create().unwrap();
    let mut app = app_for_game("morrowind");
    app.games[0].install_path = root.join("game");
    app.games[0].data_path = root.join("game/Data Files");
    let game = app.games[0].clone();
    fs::create_dir_all(&game.data_path).unwrap();
    for name in ["A.esp", "B.esp"] {
        fs::write(game.data_path.join(name), []).unwrap();
    }
    let ini = "[General]\r\n[Game Files]\r\nGameFile0=A.esp\r\nGameFile1=B.esp\r\n";
    fs::write(game.install_path.join("Morrowind.ini"), ini).unwrap();
    app.created = Some(inst.clone());
    let spec = game.plugin_spec().unwrap();
    app.plugins = compute_plugins(&app);
    let list = app.plugins.clone().unwrap();
    write_plugin_state(&app, &list, &spec).unwrap();
    // `eidos sort` in a terminal rewrites the profile's order while the
    // window keeps the list it read before.
    let state = inst.active().plugins_state_dir().join("Morrowind.ini");
    let sorted = "[General]\r\n[Game Files]\r\nGameFile0=A.esp\r\nGameFile1=B.esp\r\n";
    assert_ne!(fs::read_to_string(&state).unwrap(), sorted, "the sort must change something");
    fs::write(&state, sorted).unwrap();
    let mut stale = list.clone();
    assert!(stale.set_enabled("B.esp", false));
    assert!(write_plugin_state(&app, &stale, &spec).is_err());
    assert_eq!(fs::read_to_string(&state).unwrap(), sorted);
    // Re-read, the window's edits land again - twice, so its own write is
    // not mistaken for another process's.
    app.plugins = compute_plugins(&app);
    let mut fresh = app.plugins.clone().unwrap();
    assert!(fresh.set_enabled("B.esp", false));
    write_plugin_state(&app, &fresh, &spec).unwrap();
    assert!(fresh.set_enabled("B.esp", true));
    write_plugin_state(&app, &fresh, &spec).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_unknown_plugin_set_makes_the_archives_tab_say_so_rather_than_condemn_everything() {
    // Without this the tab renders every archive red: "we have not looked"
    // would be indistinguishable from "no plugin is active", which is the
    // exact bug the orphan diagnostic already had once.
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.screen = Screen::Main;
    assert!(app.plugins.is_none());
    assert!(archive_rows(&app, "skyrimse").is_none());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_archives_tab_says_why_each_archive_does_or_does_not_load() {
    let root = temp_portable("skyrimse");
    let inst = Instance::portable(root.clone());
    inst.create().unwrap();
    let mods = root.join("mods");
    // Named after an active plugin, the " - suffix" form, and an orphan.
    fs::create_dir_all(mods.join("Good")).unwrap();
    fs::write(mods.join("Good/Mine.bsa"), b"").unwrap();
    fs::write(mods.join("Good/Mine - Textures.bsa"), b"").unwrap();
    fs::create_dir_all(mods.join("Dead")).unwrap();
    fs::write(mods.join("Dead/Nobody.bsa"), b"").unwrap();

    let mut app = app_for_game("skyrimse");
    app.created = Some(inst);
    app.mods = vec![
        ModEntry {
            name: "Good".into(),
            enabled: true,
            path: mods.join("Good"),
            unmanaged: false,
        },
        ModEntry {
            name: "Dead".into(),
            enabled: true,
            path: mods.join("Dead"),
            unmanaged: false,
        },
    ];
    let mut list = PluginList::default();
    list.plugins.push(plugin_row("Mine.esp", "Good"));
    list.plugins[0].enabled = true;
    app.plugins = Some(list);
    app.screen = Screen::Main;

    let rows = archive_rows(&app, "skyrimse").expect("the plugin set is known");
    let by = |a: &str| rows.iter().find(|r| r.archive == a).expect(a);
    assert_eq!(by("Mine.bsa").by_plugin.as_deref(), Some("Mine.esp"));
    assert!(by("Mine.bsa").loaded());
    // The engine's " - <suffix>" rule, not MO2's looser starts-with.
    assert_eq!(
        by("Mine - Textures.bsa").by_plugin.as_deref(),
        Some("Mine.esp")
    );
    assert!(!by("Nobody.bsa").loaded(), "nothing names it");
    assert_eq!(by("Nobody.bsa").by_plugin, None);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn gui_plugin_discovery_and_diagnostics_ignore_whiteouted_plugins() {
    let (mut app, root) = list_app(&["Broken"]);
    let inst = app.created.clone().unwrap();
    fs::write(app.mods[0].path.join("Broken.esp"), b"invalid header").unwrap();
    let stack =
        eidos_core::LayerStack::new(vec![app.mods[0].path.clone()], inst.overwrite_dir());
    stack.remove("Broken.esp").unwrap();
    app.plugins = compute_plugins(&app);
    assert!(!app
        .plugins
        .as_ref()
        .unwrap()
        .plugins
        .iter()
        .any(|p| p.name == "Broken.esp"));
    let unexpected: Vec<_> = diagnostics(&app)
        .into_iter()
        .filter(|d| d.detail.contains("Broken.esp"))
        .map(|d| (d.title, d.detail))
        .collect();
    assert!(unexpected.is_empty(), "{unexpected:?}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn completed_install_surfaces_persistent_missing_payload_warning() {
    let (mut app, root) = list_app(&["Incomplete"]);
    let inst = app.created.clone().unwrap();
    let mut meta = inst.mod_meta("Incomplete");
    meta.set_install_warning("Missing archive source: required.esp");
    meta.write(&inst.meta_path("Incomplete")).unwrap();
    after_install(
        &mut app,
        "Incomplete",
        inst.mods_dir().join("Incomplete"),
        true,
        None,
    );
    assert!(app.status.as_deref().unwrap().contains("required.esp"));
    assert!(app.meta_cache["Incomplete"].install_warning.is_some());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_hidden_tool_leaves_the_picker_and_a_pinned_one_goes_to_the_top() {
    use eidos_instance::Tool;
    let mk = |title: &str, hidden: bool, pinned: bool| Tool {
        title: title.to_string(),
        exe: PathBuf::from("/x/t.exe"),
        hidden,
        pinned,
        ..Default::default()
    };
    let tools = vec![
        mk("Launcher", false, false),
        mk("Never used", true, false),
        mk("SSEEdit", false, true),
    ];
    let mut listed: Vec<&Tool> = tools.iter().filter(|t| !t.hidden).collect();
    listed.sort_by_key(|t| !t.pinned);
    let titles: Vec<&str> = listed.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(
        titles,
        vec!["SSEEdit", "Launcher"],
        "pinned first, hidden gone"
    );
}

#[test]
fn the_filetree_path_resolver_refuses_everything_that_is_not_inside_the_mod() {
    use crate::modinfo::resolve_in_mod;
    let base = PathBuf::from("/tmp/mods/Armour");
    // The ordinary case works.
    assert_eq!(
        resolve_in_mod(&base, "Meshes/armour/x.nif"),
        Some(base.join("Meshes/armour/x.nif"))
    );
    // And everything else is refused rather than normalised. A path that
    // needed normalising is not one this tab produced.
    for bad in [
        "",
        "   ",
        "..",
        "../../etc/passwd",
        "Meshes/../../..",
        "./x",
        "/etc/passwd",
        "Meshes//x",
        "C:\\Windows",
        "Meshes\\x",
    ] {
        assert_eq!(resolve_in_mod(&base, bad), None, "{bad:?} must be refused");
    }
}

#[test]
fn the_filetree_resolver_will_not_walk_through_a_symlink() {
    use crate::modinfo::resolve_in_mod;
    let root = temp_portable("skyrimse");
    let modd = root.join("mods").join("Armour");
    let outside = root.join("elsewhere");
    fs::create_dir_all(modd.join("Meshes")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret"), b"not ours").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, modd.join("Away")).unwrap();

    // A real path inside still resolves.
    assert!(resolve_in_mod(&modd, "Meshes").is_some());
    // Through the link does not - which is how a delete or a rename would
    // otherwise reach outside the mod folder entirely.
    #[cfg(unix)]
    assert_eq!(resolve_in_mod(&modd, "Away/secret"), None);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_wayland_session_is_told_the_desktop_drop_does_nothing() {
    // There is no moment of failure to hang this on - the event never fires,
    // so the window cannot know a drop was attempted. It has to be findable
    // instead, and Health is where "why did that do nothing" is answered.
    let mut app = app_for_game("skyrimse");
    app.screen = Screen::Main;
    let diag = diagnostics(&app);
    let row = diag.iter().find(|d| d.title.contains("file manager"));
    if on_wayland() {
        let row = row.expect("a Wayland session must be told");
        assert_eq!(
            row.level,
            DiagLevel::Advice,
            "nothing is broken - it is a limitation"
        );
        // And it must point at the two paths that DO work, or it is a
        // complaint rather than help.
        assert!(row.detail.contains("Install Mod"), "{}", row.detail);
        assert!(row.detail.contains("Downloads"), "{}", row.detail);
    } else {
        assert!(
            row.is_none(),
            "an X11 session must not be told its drops are broken"
        );
    }
}

/// S.L.A.C.K. prefixes its DLL with punctuation so it sorts first - SKSE
/// loads plugins alphabetically and it has to hook cosaves before anything
/// else. That is deliberate and must not be renamed, but it has no place in
/// a report: the author also declares a readable name, and that is the one a
/// person recognises in their mod list.
#[test]
fn a_refused_plugin_is_named_by_its_author_not_by_its_filename() {
    let p = eidos_gamefeatures::SePluginLoad {
        name: "Save & Load Accelerator for SKSE Cosaves (S.L.A.C.K.)".to_string(),
        dll: "!!!!!!!##$Save&LoadAcceleratorForSKSECosaves.dll".to_string(),
        version: None,
        status: "disabled, incompatible with current runtime version".to_string(),
        loaded: false,
    };
    let line = failed_plugin_line(&p);
    assert!(line.starts_with("Save & Load Accelerator"), "{line}");
    assert!(
        !line.contains("!!!"),
        "the sort-first trick is not a name: {line}"
    );
    assert!(
        line.contains("incompatible"),
        "and it still says what happened"
    );
}

/// A plugin that declared nothing has no name to fall back on but its file,
/// and saying THAT is better than saying nothing.
#[test]
fn a_plugin_that_declared_no_name_is_still_identified() {
    let p = eidos_gamefeatures::SePluginLoad {
        name: "samrim.dll".to_string(),
        dll: "samrim.dll".to_string(),
        version: None,
        status: "no version data".to_string(),
        loaded: false,
    };
    assert!(failed_plugin_line(&p).starts_with("samrim.dll"));
}

#[test]
fn an_aim_left_by_a_failed_install_never_moves_the_next_mod() {
    // An install can end without ever reaching `after_install`: an extraction
    // failure, an unrecognised layout, a dialog dismissed. The aim it left
    // behind must not be adopted by whatever is installed next.
    let mut app = nav_app(&["a", "b", "c"]);
    app.install_at = Some((0, PathBuf::from("/tmp/Aimed.7z")));
    let before: Vec<String> = app.mods.iter().map(|m| m.name.clone()).collect();

    // A DIFFERENT archive finishes installing.
    after_install(
        &mut app,
        "c",
        PathBuf::from("/tmp/x"),
        false,
        Some(Path::new("/tmp/Other.7z")),
    );
    let after: Vec<String> = app.mods.iter().map(|m| m.name.clone()).collect();
    assert_eq!(before, after, "the unrelated mod was moved by a stale aim");
    assert!(
        app.install_at.is_some(),
        "and the aim is still waiting for ITS archive"
    );
}

#[test]
fn reinstall_preserves_a_disabled_bases_priority_below_its_translation() {
    let (mut app, root) = list_app(&["Base", "Translation"]);
    app.mods[0].enabled = false;
    app.created
        .as_ref()
        .unwrap()
        .save_modlist(&app.mods)
        .unwrap();
    after_install(&mut app, "Base", root.join("mods/Base"), false, None);
    let installed = app.created.as_ref().unwrap().modlist();
    assert_eq!(
        installed
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Base", "Translation"]
    );
    assert!(!installed[0].enabled);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_download_in_flight_is_listed_before_it_finishes() {
    // The reported complaint: a mod being downloaded does not appear until
    // it is done and Refresh is pressed. The partial IS the evidence.
    let app = downloads_app(
        &[("Cool Mod.zip.unfinished", b"1234567890")],
        &[(
            "Cool Mod.zip.meta",
            "[General]\nmodName=Cool Mod\ntotalSize=100\n",
        )],
    );
    assert_eq!(app.downloads.len(), 1, "the partial must produce a row");
    let r = &app.downloads[0];
    assert_eq!(r.state, DownloadState::Downloading);
    assert_eq!(r.name, "Cool Mod.zip", "named for what it will BE");
    assert!(
        r.path.ends_with("Cool Mod.zip"),
        "Install must aim at the final path"
    );
    assert_eq!(r.downloaded, 10);
    assert_eq!(r.total, 100);
    assert_eq!(r.mod_name.as_deref(), Some("Cool Mod"));
    // Size shows the destination, so the column does not creep upward while
    // the bar is already saying how far along it is.
    assert_eq!(r.size, 100);
}

#[test]
fn a_partial_that_stopped_growing_reads_as_stalled() {
    let app = downloads_app(&[("x.zip.unfinished", b"abc")], &[]);
    assert_eq!(
        app.downloads[0].state,
        DownloadState::Downloading,
        "fresh mtime"
    );

    // Backdate it well past the window: the writing process is gone.
    let dl = app.created.as_ref().unwrap().downloads_dir();
    let old = std::time::SystemTime::now() - STALLED_AFTER - std::time::Duration::from_secs(30);
    let f = fs::File::options()
        .write(true)
        .open(dl.join("x.zip.unfinished"))
        .unwrap();
    f.set_modified(old).unwrap();
    // No sidecar exists in this case, so the partial's own mtime decides.
    let mut app = app;
    load_downloads(&mut app);
    assert_eq!(app.downloads[0].state, DownloadState::Stalled);
}

#[test]
fn a_finished_download_replaces_its_partial_and_becomes_installable() {
    // The handover: `download` renames <dest>.unfinished to <dest>. One row
    // throughout, never two, and never a gap where neither is listed.
    let mut app = downloads_app(
        &[("m.zip.unfinished", b"partial")],
        &[("m.zip.meta", "[General]\ntotalSize=7\n")],
    );
    assert_eq!(app.downloads[0].state, DownloadState::Downloading);

    let dl = app.created.as_ref().unwrap().downloads_dir();
    fs::rename(dl.join("m.zip.unfinished"), dl.join("m.zip")).unwrap();
    load_downloads(&mut app);
    assert_eq!(app.downloads.len(), 1);
    assert_eq!(app.downloads[0].state, DownloadState::Ready);
}

#[test]
fn speed_needs_two_samples_and_never_goes_backwards() {
    let mut app = downloads_app(
        &[("s.zip.unfinished", b"aaaa")],
        &[("s.zip.meta", "[General]\ntotalSize=1000\n")],
    );
    assert_eq!(app.downloads[0].speed, None, "one sighting is not a rate");

    let dl = app.created.as_ref().unwrap().downloads_dir();
    std::thread::sleep(std::time::Duration::from_millis(120));
    fs::write(dl.join("s.zip.unfinished"), vec![b'a'; 4004]).unwrap();
    load_downloads(&mut app);
    let v = app.downloads[0].speed.expect("two samples give a rate");
    assert!(v > 0.0, "grew by 4000 bytes, so the rate is positive: {v}");

    // A server that ignores our Range restarts from zero, so the partial
    // SHRINKS. That is not a negative speed.
    std::thread::sleep(std::time::Duration::from_millis(120));
    fs::write(dl.join("s.zip.unfinished"), b"a").unwrap();
    load_downloads(&mut app);
    assert_eq!(
        app.downloads[0].speed, None,
        "a shrinking partial reports no rate"
    );
}

#[test]
fn a_resumed_download_is_not_called_stalled_while_it_waits_on_the_network() {
    // The data-loss case. A previous attempt left an hours-old partial. The
    // user retries: `eidos nxm` rewrites the sidecar, then spends seconds in
    // API calls and CDN latency before the first byte lands - and a resume
    // APPENDS, so the partial's mtime stays the dead attempt's throughout.
    // Judged on the partial alone the row read "Stalled" and offered a
    // Delete that unlinked the file a live process was writing to.
    let app = downloads_app(
        &[("retry.zip.unfinished", b"leftover")],
        &[("retry.zip.meta", "[General]\ntotalSize=500\n")],
    );
    let dl = app.created.as_ref().unwrap().downloads_dir();
    let ages_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(6 * 3600);
    fs::File::options()
        .write(true)
        .open(dl.join("retry.zip.unfinished"))
        .unwrap()
        .set_modified(ages_ago)
        .unwrap();

    // The sidecar is fresh, because the retry just wrote it.
    let mut app = app;
    load_downloads(&mut app);
    assert_eq!(
        app.downloads[0].state,
        DownloadState::Downloading,
        "a fresh sidecar means an attempt is under way, whatever the partial's mtime"
    );

    // Genuinely abandoned: BOTH are old, so it really is stalled and can be
    // cleared. Age the sidecar too.
    fs::File::options()
        .write(true)
        .open(dl.join("retry.zip.meta"))
        .unwrap()
        .set_modified(ages_ago)
        .unwrap();
    load_downloads(&mut app);
    assert_eq!(app.downloads[0].state, DownloadState::Stalled);
}

#[test]
fn a_cold_plugin_cache_does_not_silence_the_missing_master_check() {
    // The check that predicts crashes must never answer "not computed yet".
    // `app.plugins` is None here, exactly as it is after any mod-list change,
    // and the diagnostic set must still contain a real verdict rather than an
    // apology.
    let mut app = nav_app(&["A"]);
    app.plugins = None;
    let out = diagnostics(&app);
    assert!(
        !out.iter().any(|d| d.title.contains("not computed")),
        "the old message is gone: {:?}",
        out.iter().map(|d| &d.title).collect::<Vec<_>>()
    );
}

#[test]
fn preflight_diagnostics_never_clear_masters_with_a_corrupt_header() {
    let (app, root) = data_app(&[("Broken.esp", "TES4")], &[]);
    let found = diagnostics(&app);
    assert!(!found.iter().any(|d| d.title == "No missing masters"));
    assert!(found
        .iter()
        .any(|d| d.title.contains("plugin_header") && d.detail.contains("Broken.esp")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn preflight_diagnostics_show_the_winning_dll_and_persistent_warnings() {
    let (mut app, root) = data_app(
        &[("SKSE/Plugins/Test.dll", "bad lower")],
        &[("SKSE/Plugins/TEST.DLL", "bad winner")],
    );
    app.games[0].install_path = root.join("game");
    let inst = app.created.as_ref().unwrap();
    let mut meta = inst.mod_meta("AAA");
    meta.set_install_warning("Missing required payload meshes/needed.nif");
    meta.write(&inst.mods_dir().join("AAA/meta.ini")).unwrap();
    app.archive_warnings = vec!["Corrupt archive Test.bsa".into()];
    let found = diagnostics(&app);
    assert!(found.iter().any(|d| d.title.contains("pe_unverified")
        && d.detail.contains("Overwrite")
        && d.detail.contains("overwrite/SKSE/Plugins/TEST.DLL")));
    assert!(found.iter().any(
        |d| d.title.contains("Incomplete installation") && d.detail.contains("needed.nif")
    ));
    assert!(found
        .iter()
        .any(|d| d.detail.contains("Corrupt archive Test.bsa")));
    fs::remove_file(root.join("overwrite/SKSE/Plugins/TEST.DLL")).unwrap();
    fs::write(root.join("overwrite/SKSE/Plugins/.eidoswh.test.dll"), []).unwrap();
    assert!(!diagnostics(&app)
        .iter()
        .any(|d| d.title.contains("pe_unverified")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn preflight_diagnostics_preserve_interrupted_runs_and_warn_when_inputs_change() {
    let (app, root) = data_app(&[("meshes/test.nif", "mesh")], &[]);
    let inst = app.created.as_ref().unwrap();
    let plugins = compute_plugins(&app)
        .unwrap()
        .plugins
        .into_iter()
        .map(|p| (p.name, p.enabled))
        .collect::<Vec<_>>();
    let run = inst
        .begin_tool_run("xEdit", None, &["xEdit.exe".into()], &plugins)
        .unwrap();
    assert!(diagnostics(&app)
        .iter()
        .any(|d| d.title.contains("unfinished tool: xEdit")));
    fs::write(inst.overwrite_dir().join("Patch.esp"), b"generated").unwrap();
    inst.finish_tool_run(run).unwrap();
    let mut meta = inst.mod_meta("AAA");
    meta.set_installed_files(&[(12, 34)]);
    meta.write(&inst.meta_path("AAA")).unwrap();
    let found = diagnostics(&app);
    assert!(!found
        .iter()
        .any(|d| d.title.contains("unfinished tool: xEdit")));
    assert!(found
        .iter()
        .any(|d| d.title.contains("Generated output inputs changed")
            && d.detail.contains("xEdit")
            && d.detail.contains("Patch.esp")));
    fs::remove_dir_all(root).unwrap();
}
