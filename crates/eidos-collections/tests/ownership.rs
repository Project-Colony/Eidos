use eidos_collections::{
    driver::RealHooks,
    install::{Hooks, Installed},
    manifest::{Mod, Source},
    state::{key_for, InstallState, Status},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "eidos-collection-ownership-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn archive(root: &Path) -> PathBuf {
    let payload = root.join("payload");
    fs::create_dir_all(payload.join("scripts")).unwrap();
    fs::write(payload.join("scripts/new.pex"), b"collection payload").unwrap();
    let archive = root.join("member.zip");
    let status =
        Command::new(eidos_sevenzip::find_7z().expect("7-Zip is required for installer tests"))
            .current_dir(&payload)
            .args(["a", "-tzip"])
            .arg(&archive)
            .arg("scripts")
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
    assert!(status.success());
    archive
}

#[test]
fn a_previously_unavailable_member_does_not_own_an_existing_personal_mod() {
    let temp = Fixture::new();
    let inst = eidos_instance::Instance::portable(temp.0.join("instance"));
    inst.create().unwrap();
    let personal = inst.mods_dir().join("Popular Mod");
    fs::create_dir_all(personal.join("scripts")).unwrap();
    fs::write(personal.join("scripts/personal.pex"), b"personal data").unwrap();
    let def = eidos_games::catalog()
        .iter()
        .find(|g| g.id == "skyrimse")
        .unwrap();
    let game = eidos_games::DetectedGame {
        source: Default::default(),
        def,
        install_path: temp.0.join("game"),
        data_path: temp.0.join("game/Data"),
        compatdata: None,
        steam_name: "test".into(),
    };
    fs::create_dir_all(&game.data_path).unwrap();
    let member = Mod {
        name: "Popular Mod".into(),
        source: Source {
            file_id: Some(1),
            mod_id: Some(1),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut state = InstallState::default();
    state.set(
        &key_for(&member, "skyrimspecialedition"),
        Status::Unavailable("manual download needed".into()),
    );
    let nexus = eidos_nexus::Nexus::with_bearer("synthetic-no-network");
    let mut say = |_: String| {};
    let mut hooks = RealHooks {
        payload_root: std::path::PathBuf::new(),
        allow_runtime_mismatch: false,
        nexus: &nexus,
        inst: &inst,
        game: &game,
        game_id: "skyrimse".into(),
        say: &mut say,
        collection_domain: "skyrimspecialedition".into(),
        owner: "test:1".into(),
        renamed: vec![],
    };
    let folder = hooks
        .reserve(
            &member,
            state
                .folders
                .get(&key_for(&member, "skyrimspecialedition"))
                .map(String::as_str),
        )
        .unwrap();
    let installed = hooks.install(&member, &archive(&temp.0), &folder);
    assert!(matches!(installed, Installed::Ok(_)), "{installed:?}");
    assert_eq!(
        fs::read(personal.join("scripts/personal.pex")).unwrap(),
        b"personal data"
    );
    assert!(!personal.join("scripts/new.pex").exists());
    assert!(inst
        .mods_dir()
        .join("Popular Mod (2)/scripts/new.pex")
        .is_file());
    // A retry must reuse its actual renamed destination, not the original personal mod.
    assert_eq!(hooks.reserve(&member, Some(&folder)).unwrap(), folder);
    assert!(hooks.reserve(&member, Some("../outside")).is_err());
    assert!(hooks.reserve(&member, Some("Popular Mod")).is_err());
    assert!(hooks.verify_installed(&member, &folder).unwrap());
    assert!(hooks.verify_installed(&member, "Popular Mod").is_err());
    assert!(!hooks.verify_installed(&member, "Gone").unwrap());
    eidos_install::install_archive_with_policy(
        &temp.0.join("member.zip"),
        &inst.mods_dir(),
        &folder,
        "skyrimse",
        eidos_install::OverwritePolicy::Replace,
        &Default::default(),
    )
    .unwrap();
    assert!(
        hooks.verify_installed(&member, &folder).is_err(),
        "a manual replacement revokes collection authority"
    );
    // Modifying or removing the marker revokes replacement authority.
    fs::write(
        inst.mods_dir().join(&folder).join("meta.ini"),
        "[General]\nmodid=1\n",
    )
    .unwrap();
    assert!(hooks.reserve(&member, Some(&folder)).is_err());
}

#[test]
fn collection_ini_output_reserves_its_own_folder_and_stops_on_checkpoint_failure() {
    use eidos_collections::{driver::apply_ini_tweaks, manifest::Collection, report::Report};
    let temp = Fixture::new();
    let inst = eidos_instance::Instance::portable(temp.0.join("instance"));
    inst.create().unwrap();
    let mut collection = Collection::default();
    collection.info.name = "My Collection".into();
    let personal = inst
        .mods_dir()
        .join("My Collection - INI Tweaks/Ini Tweaks/settings.ini");
    fs::create_dir_all(personal.parent().unwrap()).unwrap();
    fs::write(&personal, b"personal").unwrap();
    let source = temp.0.join("collection");
    fs::create_dir_all(source.join("INI Tweaks")).unwrap();
    fs::write(source.join("INI Tweaks/settings.ini"), b"collection").unwrap();
    let mut state = InstallState {
        slug: "one".into(),
        revision: 1,
        game_domain: "skyrimspecialedition".into(),
        ..Default::default()
    };
    let mut report = Report::default();
    apply_ini_tweaks(
        &inst,
        &source,
        &collection,
        &mut state,
        &mut |_| Err("disk full".into()),
        &mut report,
    );
    assert!(report.aborted);
    assert_eq!(fs::read(&personal).unwrap(), b"personal");
    let folder = state.folders["aux:ini-tweaks"].clone();
    let output = inst
        .mods_dir()
        .join(&folder)
        .join("Ini Tweaks/settings.ini");
    assert!(!output.exists());
    let mut report = Report::default();
    let state_path = temp.0.join("state.json");
    apply_ini_tweaks(
        &inst,
        &source,
        &collection,
        &mut state,
        &mut |s| s.save(&state_path).map_err(|e| e.to_string()),
        &mut report,
    );
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    assert_eq!(fs::read(output).unwrap(), b"collection");
    assert_eq!(fs::read(personal).unwrap(), b"personal");
    assert_eq!(state.folders["aux:ini-tweaks"], folder);
}

#[test]
fn a_stale_download_sidecar_does_not_hide_a_later_complete_archive() {
    let temp = Fixture::new();
    let inst = eidos_instance::Instance::portable(temp.0.join("instance"));
    inst.create().unwrap();
    fs::create_dir_all(inst.downloads_dir()).unwrap();
    let def = eidos_games::catalog()
        .iter()
        .find(|g| g.id == "skyrimse")
        .unwrap();
    let game = eidos_games::DetectedGame {
        source: Default::default(),
        def,
        install_path: temp.0.join("game"),
        data_path: temp.0.join("game/Data"),
        compatdata: None,
        steam_name: "test".into(),
    };
    for name in ["first.zip.meta", "second.zip.meta"] {
        fs::write(
            inst.downloads_dir().join(name),
            "[General]\ngameName=SkyrimSE\nmodID=1\nfileID=1\n",
        )
        .unwrap();
    }
    let sidecars: Vec<_> = fs::read_dir(inst.downloads_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let complete = sidecars[1].with_extension("");
    fs::write(&complete, b"archive").unwrap();
    let member = Mod {
        name: "Member".into(),
        source: Source {
            kind: eidos_collections::manifest::SourceType::Browse,
            file_id: Some(1),
            mod_id: Some(1),
            ..Default::default()
        },
        ..Default::default()
    };
    let nexus = eidos_nexus::Nexus::with_bearer("synthetic-no-network");
    let mut say = |_: String| {};
    let mut hooks = RealHooks {
        payload_root: std::path::PathBuf::new(),
        allow_runtime_mismatch: false,
        nexus: &nexus,
        inst: &inst,
        game: &game,
        game_id: "skyrimse".into(),
        say: &mut say,
        collection_domain: "skyrimspecialedition".into(),
        owner: "test:1".into(),
        renamed: vec![],
    };
    assert_eq!(
        hooks.obtain(&member),
        eidos_collections::install::Obtained::Ready(complete.clone())
    );
    let mut wrong = member.clone();
    wrong.source.file_size = Some(999);
    assert!(
        matches!(hooks.obtain(&wrong), eidos_collections::install::Obtained::Failed(error) if error.contains("size mismatch"))
    );
    wrong.source.file_size = None;
    wrong.source.md5 = "00000000000000000000000000000000".into();
    assert!(
        matches!(hooks.obtain(&wrong), eidos_collections::install::Obtained::Failed(error) if error.contains("MD5 mismatch"))
    );
    assert_eq!(fs::read(complete).unwrap(), b"archive");
}

#[test]
fn bundle_clone_patch_pipeline_resumes_and_failed_replacement_retains_owned_and_personal_bytes() {
    use eidos_collections::{install::run, Collection, FileHash, SourceType};
    let temp = Fixture::new();
    let inst = eidos_instance::Instance::portable(temp.0.join("instance"));
    inst.create().unwrap();
    let personal = inst.mods_dir().join("Member");
    fs::create_dir(&personal).unwrap();
    fs::write(personal.join("personal.txt"), b"personal").unwrap();
    let payload = temp.0.join("payload");
    fs::create_dir_all(payload.join("bundled/Literal Folder/options")).unwrap();
    fs::write(
        payload.join("bundled/Literal Folder/options/blue.dds"),
        b"abc",
    )
    .unwrap();
    fs::write(
        payload.join("bundled/Literal Folder/unselected.txt"),
        b"skip",
    )
    .unwrap();
    fs::create_dir_all(payload.join("patches/Member/textures")).unwrap();
    fs::write(
        payload.join("patches/Member/textures/a.dds.diff"),
        include_bytes!("fixtures/tiny.bsdiff"),
    )
    .unwrap();
    let mut member = Mod {
        name: "Member".into(),
        source: Source {
            kind: SourceType::Bundle,
            file_expression: "Literal Folder".into(),
            file_size: Some(7),
            ..Default::default()
        },
        hashes: vec![FileHash {
            path: "textures/a.dds".into(),
            md5: "900150983cd24fb0d6963f7d28e17f72".into(),
        }],
        file_overrides: vec!["textures/a.dds".into()],
        ..Default::default()
    };
    member
        .patches
        .insert("textures/a.dds".into(), "352441C2".into());
    let mut c = Collection {
        mods: vec![member],
        ..Default::default()
    };
    c.info.domain_name = "skyrimspecialedition".into();
    let def = eidos_games::catalog()
        .iter()
        .find(|g| g.id == "skyrimse")
        .unwrap();
    let game = eidos_games::DetectedGame {
        source: Default::default(),
        def,
        install_path: temp.0.join("game"),
        data_path: temp.0.join("game/Data"),
        compatdata: None,
        steam_name: "fixture".into(),
    };
    fs::create_dir_all(&game.data_path).unwrap();
    let nexus = eidos_nexus::Nexus::with_bearer("synthetic-unused");
    let mut say = |_: String| {};
    let mut hooks = RealHooks {
        payload_root: payload,
        allow_runtime_mismatch: false,
        nexus: &nexus,
        inst: &inst,
        game: &game,
        game_id: def.id.into(),
        say: &mut say,
        collection_domain: c.info.domain_name.clone(),
        owner: "synthetic:1".into(),
        renamed: vec![],
    };
    let mut state = InstallState::default();
    let mut durable = state.clone();
    let report = run(&c, &mut state, &mut hooks, &mut |s| {
        if s.members
            .values()
            .any(|status| matches!(status, Status::Installed(_)))
        {
            Err("injected post-publication checkpoint failure".into())
        } else {
            durable = s.clone();
            Ok(())
        }
    });
    assert!(report.aborted);
    let key = key_for(&c.mods[0], &c.info.domain_name);
    let folder = state.folders[&key].clone();
    assert_eq!(folder, "Member (2)");
    let installed = inst.mods_dir().join(&folder);
    assert_eq!(
        fs::read(installed.join("textures/a.dds.mohidden")).unwrap(),
        b"axcd!"
    );
    assert!(!installed.join("unselected.txt").exists());
    use std::os::unix::fs::MetadataExt;
    let published_inode = fs::metadata(&installed).unwrap().ino();
    state = durable;
    let report = run(&c, &mut state, &mut hooks, &mut |_| Ok(()));
    assert!(report.failed.is_empty(), "{report:?}");
    assert!(hooks.verify_installed(&c.mods[0], &folder).unwrap());
    assert_eq!(
        fs::metadata(&installed).unwrap().ino(),
        published_inode,
        "a verified published recipe must recover without installing and patching again"
    );
    let previous_meta = fs::read(installed.join("meta.ini")).unwrap();
    let previous_receipt = fs::read(installed.join(".eidos-collection-recipe.json")).unwrap();
    let report = run(&c, &mut state, &mut hooks, &mut |_| {
        panic!("a verified resume requires no checkpoint writes")
    });
    assert_eq!(report.installed, ["Member"]);
    c.mods[0]
        .patches
        .insert("textures/a.dds".into(), "DEADBEEF".into());
    let report = run(&c, &mut state, &mut hooks, &mut |_| Ok(()));
    assert_eq!(report.failed.len(), 1);
    assert_eq!(
        fs::read(installed.join("textures/a.dds.mohidden")).unwrap(),
        b"axcd!"
    );
    assert_eq!(fs::read(installed.join("meta.ini")).unwrap(), previous_meta);
    assert_eq!(
        fs::read(installed.join(".eidos-collection-recipe.json")).unwrap(),
        previous_receipt
    );
    assert_eq!(
        fs::read(personal.join("personal.txt")).unwrap(),
        b"personal"
    );
    assert!(!fs::read_dir(inst.mods_dir()).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".eidos-install-stage")
    }));
}

#[test]
fn omod_recipes_patch_before_publication_and_hashes_use_decoded_sources() {
    use eidos_collections::FileHash;
    let temp = Fixture::new();
    let inst = eidos_instance::Instance::portable(temp.0.join("instance"));
    inst.create().unwrap();
    let archive = temp.0.join("fixture.omod");
    // Synthetic ZIP32 OMODs: textures/a.dds contains exactly "abc".
    fs::write(&archive, include_bytes!("fixtures/unscripted.omod")).unwrap();
    let payload = temp.0.join("recipe");
    for name in ["Plain", "Hashed"] {
        fs::create_dir_all(payload.join(format!("patches/{name}/textures"))).unwrap();
        fs::write(
            payload.join(format!("patches/{name}/textures/a.dds.diff")),
            include_bytes!("fixtures/tiny.bsdiff"),
        )
        .unwrap();
    }
    let def = eidos_games::catalog()
        .iter()
        .find(|g| g.id == "oblivion")
        .unwrap();
    let game = eidos_games::DetectedGame {
        source: Default::default(),
        def,
        install_path: temp.0.join("game"),
        data_path: temp.0.join("game/Data"),
        compatdata: None,
        steam_name: "fixture".into(),
    };
    fs::create_dir_all(&game.data_path).unwrap();
    let nexus = eidos_nexus::Nexus::with_bearer("synthetic-unused");
    let mut say = |_: String| {};
    let mut hooks = RealHooks {
        payload_root: payload,
        allow_runtime_mismatch: false,
        nexus: &nexus,
        inst: &inst,
        game: &game,
        game_id: "oblivion".into(),
        say: &mut say,
        collection_domain: "oblivion".into(),
        owner: "omod:1".into(),
        renamed: vec![],
    };
    for (name, hashes) in [
        ("Plain", vec![]),
        (
            "Hashed",
            vec![FileHash {
                path: "textures/a.dds".into(),
                md5: "900150983cd24fb0d6963f7d28e17f72".into(),
            }],
        ),
    ] {
        let mut member = Mod {
            name: name.into(),
            hashes,
            file_overrides: vec!["textures/a.dds".into()],
            ..Default::default()
        };
        member
            .patches
            .insert("textures/a.dds".into(), "352441C2".into());
        assert_eq!(hooks.reserve(&member, None).unwrap(), name);
        let result = hooks.install(&member, &archive, name);
        assert!(matches!(result, Installed::Ok(_)), "{result:?}");
        let dest = inst.mods_dir().join(name);
        assert_eq!(
            fs::read(dest.join("textures/a.dds.mohidden")).unwrap(),
            b"axcd!"
        );
        assert!(!dest.join("data").exists());
        let meta = fs::read(dest.join("meta.ini")).unwrap();
        let receipt = fs::read(dest.join(".eidos-collection-recipe.json")).unwrap();
        assert!(hooks.verify_installed(&member, name).unwrap());
        member
            .patches
            .insert("textures/a.dds".into(), "DEADBEEF".into());
        assert!(matches!(
            hooks.install(&member, &archive, name),
            Installed::Failed(_)
        ));
        assert_eq!(
            fs::read(dest.join("textures/a.dds.mohidden")).unwrap(),
            b"axcd!"
        );
        assert_eq!(fs::read(dest.join("meta.ini")).unwrap(), meta);
        assert_eq!(
            fs::read(dest.join(".eidos-collection-recipe.json")).unwrap(),
            receipt
        );
        let scripted = temp.0.join("scripted.omod");
        fs::write(&scripted, include_bytes!("fixtures/scripted.omod")).unwrap();
        // Scripted OMODs now retain an exact prompt instead of rejecting all scripts.
        // No default file set or failing patch is published before acknowledgment.
        let pending = hooks.install(&member, &scripted, name);
        assert!(matches!(pending, Installed::NeedsUser(_)), "{pending:?}");
        assert_eq!(fs::read(dest.join("meta.ini")).unwrap(), meta);
    }
    assert!(fs::read_dir(inst.mods_dir()).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".eidos-install")));
}
