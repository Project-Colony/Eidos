use eidos_collections::{FileHash, Mod, Source, SourceType, recipe::*};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "eidos-recipe-{}-{}",
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
fn put(root: &Path, p: &str, bytes: &[u8]) {
    let p = root.join(p);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}
fn clone_member() -> Mod {
    Mod {
        name: "Example".into(),
        hashes: vec![
            FileHash {
                path: "textures\\renamed.dds".into(),
                md5: "900150983cd24fb0d6963f7d28e17f72".into(),
            },
            FileHash {
                path: "Root/copy.txt".into(),
                md5: "900150983cd24fb0d6963f7d28e17f72".into(),
            },
        ],
        ..Default::default()
    }
}
#[test]
fn exact_archive_validation_preserves_mismatches() {
    let t = Temp::new();
    let p = t.0.join("personal.zip");
    fs::write(&p, b"abc").unwrap();
    let mut m = Mod {
        source: Source {
            md5: "900150983CD24FB0D6963F7D28E17F72".into(),
            file_size: Some(3),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(validate_archive(&m, &p).is_ok());
    m.source.file_size = Some(4);
    assert!(validate_archive(&m, &p).is_err());
    m.source.file_size = None;
    m.source.md5 = "bad".into();
    assert!(validate_archive(&m, &p).is_err());
    m.source.md5 = "00000000000000000000000000000000".into();
    assert!(validate_archive(&m, &p).is_err());
    m.source.md5.clear();
    assert!(validate_archive(&m, &p).is_ok());
    assert_eq!(fs::read(p).unwrap(), b"abc");
}
#[test]
fn clone_selects_bytes_renames_and_repeats_without_installers() {
    let t = Temp::new();
    let s = t.0.join("source");
    let d = t.0.join("dest");
    put(&s, "options/blue.dds", b"abc");
    put(&s, "options/red.dds", b"red");
    put(&s, "fomod/ModuleConfig.xml", b"not an installer");
    populate(&clone_member(), &s, &d).unwrap();
    assert_eq!(fs::read(d.join("textures/renamed.dds")).unwrap(), b"abc");
    assert_eq!(fs::read(d.join("Root/copy.txt")).unwrap(), b"abc");
    assert!(!d.join("fomod").exists());
    assert!(!d.join("options").exists());
    let mut missing = clone_member();
    missing.hashes[0].md5 = "00000000000000000000000000000000".into();
    assert!(
        populate(&missing, &s, &t.0.join("missing"))
            .unwrap_err()
            .contains("Missing")
    );
}
#[test]
fn recipe_destinations_reject_escapes_and_windows_collisions() {
    for path in [
        "../outside",
        "C:\\temp\\x",
        "\\\\host\\x",
        "/root",
        "a/../b",
        "a\0b",
        "meta.ini",
        "a:b",
        "x./y",
    ] {
        let mut m = clone_member();
        m.hashes[0].path = path.into();
        assert!(validate_member(&m).is_err(), "{path}");
    }
    let mut m = clone_member();
    m.hashes[1].path = "Textures/Renamed.dds".into();
    assert!(validate_member(&m).is_err());
    m = clone_member();
    m.patches.insert("../x".into(), "12345678".into());
    assert!(validate_member(&m).is_err());
    m = Mod::default();
    m.name = "../evil".into();
    m.patches.insert("x".into(), "12345678".into());
    assert!(validate_member(&m).is_err());
}
#[test]
fn bundles_keep_installed_tree_and_reject_missing_or_symlink_sources() {
    let t = Temp::new();
    let s = t.0.join("bundled/My literal v1");
    put(&s, "Data/config.ini", b"ini");
    put(&s, "Root/helper.txt", b"root");
    put(&s, "textures/sub/a.dds", b"tex");
    let m = Mod {
        source: Source {
            kind: SourceType::Bundle,
            file_expression: "My literal v1".into(),
            file_size: Some(10),
            ..Default::default()
        },
        ..Default::default()
    };
    let source = bundle_source(&m, &t.0).unwrap();
    let d = t.0.join("out");
    populate(&m, &source, &d).unwrap();
    assert_eq!(fs::read(d.join("Data/config.ini")).unwrap(), b"ini");
    assert_eq!(fs::read(d.join("Root/helper.txt")).unwrap(), b"root");
    let mut bad = m.clone();
    bad.source.file_size = Some(11);
    assert!(bundle_source(&bad, &t.0).is_err());
    bad.source.file_expression = "absent".into();
    assert!(bundle_source(&bad, &t.0).is_err());
    std::os::unix::fs::symlink("/etc/passwd", s.join("escape")).unwrap();
    assert!(bundle_source(&m, &t.0).is_err());
}
#[test]
fn original_crc_patch_and_exclusions_finish_before_receipt_and_resume() {
    let t = Temp::new();
    let stage = t.0.join("stage");
    put(&stage, "textures/renamed.dds", b"abc");
    put(&stage, "Root/copy.txt", b"abc");
    put(
        &t.0,
        "patches/Example/textures/renamed.dds.diff",
        include_bytes!("fixtures/tiny.bsdiff"),
    );
    let mut m = clone_member();
    m.patches
        .insert("textures/renamed.dds".into(), "352441C2".into());
    m.file_overrides.push("textures/renamed.dds".into());
    assert!(!verify_receipt(&m, &t.0, &stage, "owner").unwrap());
    finish(&m, &t.0, &stage, "owner").unwrap();
    assert!(!stage.join("textures/renamed.dds").exists());
    assert_eq!(
        fs::read(stage.join("textures/renamed.dds.mohidden")).unwrap(),
        b"axcd!"
    );
    assert_eq!(fs::read(stage.join("Root/copy.txt")).unwrap(), b"abc");
    assert!(verify_receipt(&m, &t.0, &stage, "owner").unwrap());
    put(&stage, "Root/copy.txt", b"changed");
    assert!(!verify_receipt(&m, &t.0, &stage, "owner").unwrap());
}
#[test]
fn patch_bad_crc_corruption_and_size_bounds_leave_base_unchanged() {
    let t = Temp::new();
    let p = t.0.join("base");
    fs::write(&p, b"abc").unwrap();
    let patch = include_bytes!("fixtures/tiny.bsdiff");
    assert!(patch_file(&p, patch, "00000000", 64).is_err());
    assert!(patch_file(&p, patch, "352441C2", 4).is_err());
    for bytes in [&patch[..20], &patch[..patch.len() - 5]] {
        assert!(patch_file(&p, bytes, "352441C2", 64).is_err());
    }
    let mut invalid = patch.to_vec();
    invalid[31] = 0x80;
    assert!(patch_file(&p, &invalid, "352441C2", 64).is_err());
    invalid[31] = 0x7f;
    assert!(patch_file(&p, &invalid, "352441C2", 64).is_err());
    assert_eq!(fs::read(&p).unwrap(), b"abc");
    patch_file(&p, patch, "352441C2", 64).unwrap();
    assert_eq!(fs::read(p).unwrap(), b"axcd!");
}
#[test]
fn exclusions_are_local_and_never_clobber_hidden_files_or_guess_foreign_roots() {
    let t = Temp::new();
    let high = t.0.join("high");
    let low = t.0.join("low");
    put(&high, "textures/a.dds", b"high");
    put(&low, "textures/a.dds", b"low");
    let mut m = Mod {
        file_overrides: vec!["textures/a.dds".into()],
        ..Default::default()
    };
    finish(&m, &t.0, &high, "owner").unwrap();
    assert_eq!(fs::read(low.join("textures/a.dds")).unwrap(), b"low");
    assert!(!high.join("textures/a.dds").exists());
    put(&high, "textures/a.dds", b"new");
    assert!(finish(&m, &t.0, &high, "owner").is_err());
    assert_eq!(
        fs::read(high.join("textures/a.dds.mohidden")).unwrap(),
        b"high"
    );
    m.file_overrides = vec!["C:\\Unknown\\Game\\Data\\textures\\a.dds".into()];
    assert!(validate_member(&m).is_err());
}
#[test]
fn runtime_evidence_is_exact_and_unknown_is_not_compatible() {
    assert!(matches!(
        compare_runtime(&["1.2.3.0".into()], Ok("1.2.3.0".into())),
        RuntimeCheck::Compatible { .. }
    ));
    assert!(matches!(
        compare_runtime(&["1.2.3".into()], Ok("1.2.3.0".into())),
        RuntimeCheck::Mismatch { .. }
    ));
    assert!(matches!(
        compare_runtime(&["1".into()], Err("conflicting PE versions".into())),
        RuntimeCheck::Unknown { .. }
    ));
}
#[test]
fn malformed_present_recipe_fields_cannot_disappear_during_tolerant_parsing() {
    for source in [r#""md5":123"#, r#""md5":"garbage""#, r#""fileSize":-1"#] {
        let json = format!(r#"{{"mods":[{{"name":"A","source":{{{source}}}}}]}}"#);
        assert!(
            eidos_collections::read(&json)
                .and_then(|r| {
                    for m in &r.collection.mods {
                        validate_member(m)?;
                    }
                    Ok(())
                })
                .is_err(),
            "{json}"
        );
    }
    for recipe in [
        r#""hashes":[{"path":"x","md5":7}]"#,
        r#""hashes":"oops""#,
        r#""patches":{"x":123}"#,
        r#""fileOverrides":[7]"#,
    ] {
        let json = format!(r#"{{"mods":[{{"name":"A",{recipe}}}]}}"#);
        assert!(
            eidos_collections::read(&json)
                .and_then(|r| {
                    for m in &r.collection.mods {
                        validate_member(m)?;
                    }
                    Ok(())
                })
                .is_err(),
            "{json}"
        );
    }
}
#[test]
fn known_absolute_exclusions_rebase_only_exact_data_or_game_roots() {
    let m = Mod {
        file_overrides: vec![
            "Z:\\games\\Skyrim\\Data\\Textures\\a.dds".into(),
            "Z:\\games\\Skyrim\\helper.txt".into(),
        ],
        ..Default::default()
    };
    let mapped = map_exclusions(
        &m,
        Path::new("/games/Skyrim"),
        Path::new("/games/Skyrim/Data"),
    )
    .unwrap();
    assert_eq!(mapped.file_overrides, ["Textures/a.dds", "Root/helper.txt"]);
}
#[test]
fn runtime_mismatch_blocks_every_member_then_requires_a_durable_exact_decision() {
    use eidos_collections::{
        Collection,
        install::{Hooks, Installed, Obtained, run},
        state::InstallState,
    };
    struct Runtime {
        allow: bool,
        observed: String,
        mutations: usize,
    }
    impl Hooks for Runtime {
        fn runtime_check(&mut self, c: &Collection) -> RuntimeCheck {
            compare_runtime(&c.info.game_versions, Ok(self.observed.clone()))
        }
        fn allow_runtime_mismatch(&self) -> bool {
            self.allow
        }
        fn obtain(&mut self, _: &Mod) -> Obtained {
            self.mutations += 1;
            Obtained::Ready("source".into())
        }
        fn install(&mut self, m: &Mod, _: &Path, _: &str) -> Installed {
            self.mutations += 1;
            Installed::Ok(m.name.clone())
        }
    }
    let mut c = Collection::default();
    c.info.game_versions = vec!["old".into()];
    c.mods.push(Mod {
        name: "A".into(),
        ..Default::default()
    });
    let mut hooks = Runtime {
        allow: false,
        observed: "new".into(),
        mutations: 0,
    };
    let mut state = InstallState::default();
    let original = state.clone();
    assert!(run(&c, &mut state, &mut hooks, &mut |_| Ok(())).aborted);
    assert_eq!(state, original);
    assert_eq!(hooks.mutations, 0);
    hooks.allow = true;
    assert!(run(&c, &mut state, &mut hooks, &mut |_| Err("disk full".into())).aborted);
    assert_eq!(hooks.mutations, 0);
    assert_eq!(
        state.runtime_decision, None,
        "a failed save cannot authorize a later resume"
    );
    state = InstallState::default();
    assert!(!run(&c, &mut state, &mut hooks, &mut |_| Ok(())).aborted);
    assert_eq!(hooks.mutations, 2);
    hooks.allow = false;
    assert!(!run(&c, &mut state, &mut hooks, &mut |_| Ok(())).aborted);
    hooks.observed = "different".into();
    assert!(run(&c, &mut state, &mut hooks, &mut |_| Ok(())).aborted);
}
#[test]
fn bundle_preserves_empty_directories_and_destination_symlinks_cannot_escape() {
    let t = Temp::new();
    let source = t.0.join("source");
    fs::create_dir_all(source.join("empty/nested")).unwrap();
    put(&source, "textures/a.dds", b"abc");
    let member = Mod::default();
    let dest = t.0.join("out");
    populate(&member, &source, &dest).unwrap();
    assert!(dest.join("empty/nested").is_dir());
    let outside = t.0.join("personal");
    fs::create_dir(&outside).unwrap();
    let hostile = t.0.join("hostile");
    fs::create_dir(&hostile).unwrap();
    std::os::unix::fs::symlink(&outside, hostile.join("textures")).unwrap();
    assert!(populate(&member, &source, &hostile).is_err());
    assert!(!outside.join("a.dds").exists());
}

#[test]
fn receipt_rejects_removed_empty_bundle_directories() {
    let t = Temp::new();
    let source = t.0.join("bundled/Bundle");
    fs::create_dir_all(source.join("empty/nested")).unwrap();
    put(&source, "a.txt", b"abc");
    let member = Mod {
        source: Source {
            kind: SourceType::Bundle,
            file_expression: "Bundle".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let stage = t.0.join("stage");
    populate(&member, &source, &stage).unwrap();
    finish(&member, &t.0, &stage, "owner").unwrap();
    assert!(verify_receipt(&member, &t.0, &stage, "owner").unwrap());
    fs::remove_dir(stage.join("empty/nested")).unwrap();
    assert!(!verify_receipt(&member, &t.0, &stage, "owner").unwrap());
}

fn bsdiff_fixture(control: &[u8], diff: &[u8], extra: &[u8], target: u64) -> Vec<u8> {
    use std::io::Write;
    let compress = |bytes: &[u8]| {
        let mut enc = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    };
    let control = compress(control);
    let diff = compress(diff);
    let extra = compress(extra);
    let mut patch = b"BSDIFF40".to_vec();
    patch.extend((control.len() as u64).to_le_bytes());
    patch.extend((diff.len() as u64).to_le_bytes());
    patch.extend(target.to_le_bytes());
    patch.extend(control);
    patch.extend(diff);
    patch.extend(extra);
    patch
}
#[test]
fn malformed_bsdiff_control_and_stream_lengths_never_publish() {
    let t = Temp::new();
    let path = t.0.join("base");
    fs::write(&path, b"abc").unwrap();
    let mut ctrl = Vec::new();
    for n in [3u64, 2, 0] {
        ctrl.extend(n.to_le_bytes());
    }
    let mut negative = ctrl.clone();
    negative[7] = 0x80;
    for (label, patch) in [
        (
            "partial control",
            bsdiff_fixture(&ctrl[..23], &[0, 22, 0], b"d!", 5),
        ),
        ("truncated diff", bsdiff_fixture(&ctrl, &[0, 22], b"d!", 5)),
        (
            "truncated extra",
            bsdiff_fixture(&ctrl, &[0, 22, 0], b"d", 5),
        ),
        (
            "negative control",
            bsdiff_fixture(&negative, &[0, 22, 0], b"d!", 5),
        ),
        ("short output", bsdiff_fixture(&ctrl, &[0, 22, 0], b"d!", 6)),
        ("long output", bsdiff_fixture(&ctrl, &[0, 22, 0], b"d!", 4)),
    ] {
        assert!(
            patch_file(&path, &patch, "352441C2", 64).is_err(),
            "{label}"
        );
        assert_eq!(fs::read(&path).unwrap(), b"abc");
    }
    let mut oversized = include_bytes!("fixtures/tiny.bsdiff").to_vec();
    oversized[24..32].copy_from_slice(&(512u64 * 1024 * 1024 + 1).to_le_bytes());
    assert!(
        patch_file(&path, &oversized, "352441C2", u64::MAX).is_err(),
        "callers cannot exceed the fixed memory ceiling"
    );
    let mut invalid = include_bytes!("fixtures/tiny.bsdiff").to_vec();
    let offset = 32 + u64::from_le_bytes(invalid[8..16].try_into().unwrap()) as usize;
    invalid.insert(offset, 0);
    let control_len = (offset - 31) as u64;
    invalid[8..16].copy_from_slice(&control_len.to_le_bytes());
    assert!(
        patch_file(&path, &invalid, "352441C2", 64).is_err(),
        "trailing corrupt bytes in a compressed stream"
    );
    assert_eq!(fs::read(&path).unwrap(), b"abc");
}

#[test]
fn excluded_provider_yields_in_both_profile_orders_and_keeps_other_files() {
    let t = Temp::new();
    let inst = eidos_instance::Instance::portable(t.0.join("instance"));
    inst.create().unwrap();
    let high = inst.mods_dir().join("High");
    let low = inst.mods_dir().join("Low");
    put(&high, "Shared.esp", b"high");
    put(&high, "Other.esp", b"other");
    put(&low, "Shared.esp", b"low");
    finish(
        &Mod {
            file_overrides: vec!["Shared.esp".into()],
            ..Default::default()
        },
        &t.0,
        &high,
        "owner",
    )
    .unwrap();
    inst.register_installed_mod("High").unwrap();
    inst.register_installed_mod("Low").unwrap();
    let mut mods = inst.modlist();
    for m in &mut mods {
        m.enabled = true;
    }
    let data = t.0.join("Data");
    fs::create_dir(&data).unwrap();
    for _ in 0..2 {
        inst.save_modlist(&mods).unwrap();
        let list = inst.plugin_list(&data, "skyrimse", None).unwrap();
        let shared = list
            .plugins
            .iter()
            .find(|p| p.name == "Shared.esp")
            .unwrap();
        assert_eq!(shared.origin_mod, "Low");
        assert_eq!(fs::read(&shared.path).unwrap(), b"low");
        assert_eq!(
            list.plugins
                .iter()
                .find(|p| p.name == "Other.esp")
                .unwrap()
                .origin_mod,
            "High"
        );
        mods.reverse();
    }
    assert_eq!(fs::read(high.join("Shared.esp.mohidden")).unwrap(), b"high");
    assert!(!high.join(".eidoswh.Shared.esp").exists());
    let collision = t.0.join("collision");
    put(&collision, "a.txt", b"new");
    put(&collision, "A.TXT.MOHIDDEN", b"personal hidden");
    assert!(
        finish(
            &Mod {
                file_overrides: vec!["a.txt".into()],
                ..Default::default()
            },
            &t.0,
            &collision,
            "owner"
        )
        .unwrap_err()
        .contains("collides with existing hidden")
    );
    assert_eq!(fs::read(collision.join("a.txt")).unwrap(), b"new");
    assert_eq!(
        fs::read(collision.join("A.TXT.MOHIDDEN")).unwrap(),
        b"personal hidden"
    );
}

#[test]
fn actual_pe_runtime_evidence_gates_the_real_driver_before_member_mutation() {
    use eidos_collections::{Collection, driver::RealHooks, install::run, state::InstallState};
    let t = Temp::new();
    let inst = eidos_instance::Instance::portable(t.0.join("instance"));
    inst.create().unwrap();
    let def = eidos_games::catalog()
        .iter()
        .find(|g| g.id == "skyrimse")
        .unwrap();
    let game = eidos_games::DetectedGame {
        source: Default::default(),
        def,
        install_path: t.0.join("game"),
        data_path: t.0.join("game/Data"),
        compatdata: None,
        steam_name: "fixture".into(),
    };
    fs::create_dir_all(&game.data_path).unwrap();
    let exe = game.install_path.join(def.game_binary);
    fs::write(&exe, include_bytes!("fixtures/runtime.pe")).unwrap();
    let mut c = Collection::default();
    c.info.game_versions = vec!["1.7.104.0".into()];
    assert!(matches!(
        runtime_check(&c, &game),
        RuntimeCheck::Compatible { .. }
    ));
    c.info.game_versions = vec!["1.6.640.0".into()];
    assert!(matches!(
        runtime_check(&c, &game),
        RuntimeCheck::Mismatch { .. }
    ));
    let nexus = eidos_nexus::Nexus::with_bearer("synthetic-unused");
    let mut say = |_: String| {};
    let mut hooks = RealHooks {
        payload_root: t.0.clone(),
        allow_runtime_mismatch: false,
        nexus: &nexus,
        inst: &inst,
        game: &game,
        game_id: def.id.into(),
        say: &mut say,
        collection_domain: def.nexus_game.into(),
        owner: "test:1".into(),
        renamed: vec![],
    };
    c.mods.push(Mod {
        name: "Must never be reserved".into(),
        source: Source {
            kind: SourceType::Bundle,
            file_expression: "missing".into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let mut state = InstallState::default();
    let report = run(&c, &mut state, &mut hooks, &mut |_| {
        panic!("mismatch must block before any checkpoint")
    });
    assert!(report.aborted);
    assert!(state.folders.is_empty());
    assert_eq!(fs::read_dir(inst.mods_dir()).unwrap().count(), 0);
    let mut ambiguous = include_bytes!("fixtures/runtime.pe").to_vec();
    ambiguous[0x23e..0x240].copy_from_slice(&2u16.to_le_bytes());
    for (offset, value) in [
        (0x244, 96u32),
        (0x248, 1036),
        (0x24c, 112),
        (0x260, 0x1100),
        (0x264, 92),
        (0x270, 0x1180),
        (0x274, 92),
    ] {
        ambiguous[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    ambiguous.copy_within(0x300..0x35c, 0x380);
    ambiguous[0x3b4..0x3b8].copy_from_slice(&(105u32 << 16).to_le_bytes());
    fs::write(&exe, ambiguous).unwrap();
    assert!(
        matches!(runtime_check(&c,&game),RuntimeCheck::Unknown{reason} if reason.contains("conflicting"))
    );
    fs::write(&exe, b"unreadable PE").unwrap();
    assert!(matches!(
        runtime_check(&c, &game),
        RuntimeCheck::Unknown { .. }
    ));
    let mut no_version = include_bytes!("fixtures/runtime.pe").to_vec();
    no_version[0x118..0x120].fill(0);
    fs::write(&exe, no_version).unwrap();
    assert!(matches!(
        runtime_check(&c, &game),
        RuntimeCheck::Unknown { .. }
    ));
}
