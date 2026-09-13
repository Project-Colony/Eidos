use super::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
static COUNTER: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
    instance: eidos_instance::Instance,
    game: ScriptedGame,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "eidos-scripted-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let instance = eidos_instance::Instance::portable(root.join("instance"));
        instance.create().unwrap();
        let install_path = root.join("game");
        let data_path = install_path.join("Data");
        fs::create_dir_all(&data_path).unwrap();
        fs::write(data_path.join("base.txt"), "vanilla").unwrap();
        fs::write(
            instance.active().ini_path("Oblivion.ini"),
            "[General]\nName=Original\n",
        )
        .unwrap();
        Self {
            root,
            instance,
            game: ScriptedGame {
                game_id: "oblivion".into(),
                install_path,
                data_path,
                prefix: None,
                observed_versions: BTreeMap::from([("Oblivion".into(), "1.2.416.0".into())]),
            },
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn scripted_context_rejects_other_games_before_reading_paths() {
    let instance = eidos_instance::Instance::portable(PathBuf::from("/not-an-instance"));
    let game = ScriptedGame {
        game_id: "skyrimse".into(),
        install_path: "/not-a-game".into(),
        data_path: "/not-a-game/Data".into(),
        prefix: None,
        observed_versions: BTreeMap::new(),
    };
    let error = ScriptedContext::capture(&instance, &game, &AtomicBool::new(false)).unwrap_err();
    assert!(error.to_string().contains("Oblivion"));
}

#[test]
fn context_uses_virtual_winners_whiteouts_and_profile_ini_state() {
    let f = Fixture::new();
    let layer = f.instance.mods_dir().join("Layer");
    fs::create_dir(&layer).unwrap();
    fs::write(layer.join("base.txt"), "modded").unwrap();
    fs::write(layer.join("mod-only.txt"), "mod").unwrap();
    f.instance.register_installed_mod("Layer").unwrap();
    fs::write(f.instance.overwrite_dir().join(".eidoswh.base.txt"), []).unwrap();
    fs::write(f.instance.overwrite_dir().join("generated.txt"), "output").unwrap();
    let context = ScriptedContext::capture(&f.instance, &f.game, &AtomicBool::new(false)).unwrap();
    assert!(!context.interpreter.files.contains("base.txt"));
    assert!(context.interpreter.files.contains("mod-only.txt"));
    assert!(context.interpreter.files.contains("generated.txt"));
    assert_eq!(context.interpreter.ini["general"]["name"], "Original");
    assert!(context.interpreter.active_mods.contains("Layer"));
    assert_eq!(context.origin.profile, f.instance.active_profile());
    assert_eq!(context.origin.game, f.game);
    let again = ScriptedContext::capture(&f.instance, &f.game, &AtomicBool::new(false)).unwrap();
    assert_eq!(context.origin.context_sha256, again.origin.context_sha256);
    fs::write(f.instance.overwrite_dir().join("generated.txt"), "changed").unwrap();
    let changed = ScriptedContext::capture(&f.instance, &f.game, &AtomicBool::new(false)).unwrap();
    assert_ne!(context.origin.context_sha256, changed.origin.context_sha256);
    assert_eq!(
        fs::read(f.game.data_path.join("base.txt")).unwrap(),
        b"vanilla"
    );
}

impl Fixture {
    fn session(
        &self,
        script: &str,
        files: &[(&str, &[u8], super::super::OmodFileKind)],
    ) -> OmodSession {
        use super::super::*;
        let tmp = self.root.join(format!(
            "session-{}",
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(tmp.join("payload/data")).unwrap();
        fs::create_dir_all(tmp.join("payload/plugins")).unwrap();
        let mut members = Vec::new();
        for (name, bytes, kind) in files {
            let group = if *kind == OmodFileKind::Data {
                "data"
            } else {
                "plugins"
            };
            let path = tmp.join("payload").join(group).join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
            members.push(OmodMember {
                path: name.to_string(),
                size: bytes.len() as u64,
                crc32: crc32fast::hash(bytes),
                kind: *kind,
            });
        }
        let archive = self.root.join(format!(
            "test-{}.omod",
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&archive, script).unwrap();
        OmodSession {
            metadata: OmodMetadata {
                format_version: 4,
                name: "Test".into(),
                major: 1,
                minor: 0,
                build: 0,
                author: "Fixture".into(),
                email: String::new(),
                website: String::new(),
                description: "Synthetic OMOD".into(),
                created: OmodCreationTime::DotNetBinary(0),
                compression: OmodCompression::Zip,
            },
            script: Some(OmodScript {
                kind: OmodScriptKind::Obmm,
                bytes: script.as_bytes().to_vec(),
                body_start: 0,
            }),
            readme: None,
            image: None,
            members,
            origin: super::super::ArchiveOrigin::capture(&archive, &AtomicBool::new(false))
                .unwrap(),
            archive,
            tree: super::super::super::ExtractedTree::owned(tmp),
        }
    }
    fn review(&self, session: &OmodSession) -> (ScriptedContext, Box<ScriptedReview>) {
        let c =
            ScriptedContext::capture(&self.instance, &self.game, &AtomicBool::new(false)).unwrap();
        let r = new_omod_receipt(session, &c, &AtomicBool::new(false)).unwrap();
        let ScriptedEvaluation::Review(review) =
            evaluate_scripted_omod(session, &c, &r, &AtomicBool::new(false)).unwrap()
        else {
            panic!("unexpected prompt")
        };
        (c, review)
    }
}
#[test]
fn archive_origin_binds_decoded_session_before_first_receipt() {
    let f = Fixture::new();
    let session = f.session("Return", &[]);
    let cancel = AtomicBool::new(false);
    let context = ScriptedContext::capture(&f.instance, &f.game, &cancel).unwrap();
    fs::write(&session.archive, "unrelated replacement archive").unwrap();
    assert!(new_omod_receipt(&session, &context, &cancel).is_err());
    let mut relabeled = f.session("Return", &[]);
    let other = f.root.join("same-bytes.omod");
    fs::copy(&relabeled.archive, &other).unwrap();
    relabeled.archive = other;
    assert!(new_omod_receipt(&relabeled, &context, &cancel).is_err());
    assert!(!f.instance.mods_dir().join("Pack").exists());
}

#[test]
fn receipt_binds_exact_script_archive_context_and_full_answers() {
    let f = Fixture::new();
    let s = f.session("Message \"Review this\"", &[]);
    let stop = AtomicBool::new(false);
    let c = ScriptedContext::capture(&f.instance, &f.game, &stop).unwrap();
    let mut receipt = new_omod_receipt(&s, &c, &stop).unwrap();
    let ScriptedEvaluation::NeedPrompt(prompt) =
        evaluate_scripted_omod(&s, &c, &receipt, &stop).unwrap()
    else {
        panic!()
    };
    receipt.answers.push(ObmmRecordedAnswer {
        prompt,
        answer: ObmmAnswer::Acknowledge,
    });
    assert!(matches!(
        evaluate_scripted_omod(&s, &c, &receipt, &stop).unwrap(),
        ScriptedEvaluation::Review(_)
    ));
    let restored: OmodReplayReceipt =
        serde_json::from_slice(&serde_json::to_vec(&receipt).unwrap()).unwrap();
    assert_eq!(receipt, restored);
    let mut stale = receipt.clone();
    stale.origin.profile = "Other".into();
    assert!(evaluate_scripted_omod(&s, &c, &stale, &stop)
        .unwrap_err()
        .to_string()
        .contains("receipt"));
    let mut stale = receipt.clone();
    stale.answers[0].prompt.ordinal += 1;
    assert!(evaluate_scripted_omod(&s, &c, &stale, &stop).is_err());
    fs::write(&s.archive, "changed archive").unwrap();
    assert!(evaluate_scripted_omod(&s, &c, &receipt, &stop).is_err());
}
#[test]
fn review_classifies_profile_and_unsupported_effects_without_applying() {
    use super::super::OmodFileKind::Plugin;
    let f = Fixture::new();
    let s = f.session(
        "EditINI General Name Changed\nSetGMST sample.esp test 1\nUncheckESP sample.esp",
        &[("sample.esp", b"TES4fixture", Plugin)],
    );
    let (_, r) = f.review(&s);
    assert!(r.profile_effects.iter().any(|e| e.command == "EditINI"));
    assert!(r.unsupported.iter().any(|e| e.contains("SetGMST")));
    assert_eq!(
        fs::read_to_string(f.instance.active().ini_path("Oblivion.ini")).unwrap(),
        "[General]\nName=Original\n"
    );
}

#[test]
fn stage_applies_xml_binary_and_patch_then_commits_profile_effects() {
    use super::super::OmodFileKind::{Data, Plugin};
    let f = Fixture::new();
    let script="EditXMLReplace menus/a.xml old new\nSetPluginInt sample.esp 4 12345\nPatchDataFile patch.txt base.txt\nEditINI General Name Changed";
    let s = f.session(
        script,
        &[
            ("menus/a.xml", b"<x>old</x>\n", Data),
            ("sample.esp", b"TES4\0\0\0\0fixture", Plugin),
            ("patch.txt", b"patched", Data),
        ],
    );
    let (c, r) = f.review(&s);
    let report = install_scripted_omod(
        &s,
        &c,
        &r,
        "Installed",
        OverwritePolicy::Fail,
        ScriptedApproval {
            apply_profile_effects: true,
            allow_incomplete: false,
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(!report.pending_effects);
    assert_eq!(
        fs::read(report.install.dest.join("menus/a.xml")).unwrap(),
        b"<x>new</x>\n"
    );
    assert_eq!(
        &fs::read(report.install.dest.join("sample.esp")).unwrap()[4..8],
        &12345i32.to_le_bytes()
    );
    assert_eq!(
        fs::read(report.install.dest.join("base.txt")).unwrap(),
        b"patched"
    );
    assert!(
        fs::read_to_string(f.instance.active().ini_path("Oblivion.ini"))
            .unwrap()
            .contains("Name=Changed")
    );
    assert_eq!(
        fs::read(f.game.data_path.join("base.txt")).unwrap(),
        b"vanilla"
    );
    assert_eq!(
        fs::read(s.data_root().join("menus/a.xml")).unwrap(),
        b"<x>old</x>\n"
    );
    assert!(report.receipt_path.is_file());
    let stack = eidos_core::LayerStack::new(vec![report.install.dest], f.instance.overwrite_dir());
    assert!(!stack
        .list_dir("")
        .iter()
        .any(|(n, _)| n.starts_with(".eidos-omod")));
}
#[test]
fn merge_cancel_stale_crc_and_finish_errors_precede_publication() {
    use super::super::OmodFileKind::Data;
    let f = Fixture::new();
    let s = f.session("", &[("a.txt", b"original", Data)]);
    let (c, r) = f.review(&s);
    let called = std::cell::Cell::new(false);
    let result = install_scripted_omod_with_finish(
        &s,
        &c,
        &r,
        "NoMod",
        OverwritePolicy::MergeWithBackup,
        ScriptedApproval::default(),
        &AtomicBool::new(false),
        |_| {
            called.set(true);
            Ok(())
        },
    );
    assert!(result.is_err());
    assert!(!called.get());
    assert!(!f.instance.mods_dir().join("NoMod").exists());
    assert!(install_scripted_omod(
        &s,
        &c,
        &r,
        "NoMod",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(true)
    )
    .is_err());
    let error = install_scripted_omod_with_finish(
        &s,
        &c,
        &r,
        "NoMod",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false),
        |_| Err(bad("fixture finish failure")),
    )
    .unwrap_err();
    assert!(error.to_string().contains("fixture finish failure"));
    assert!(!f.instance.mods_dir().join("NoMod").exists());
    fs::write(s.data_root().join("a.txt"), b"tampered").unwrap();
    assert!(install_scripted_omod(
        &s,
        &c,
        &r,
        "NoMod",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(!f.instance.mods_dir().join("NoMod").exists());
    fs::write(s.data_root().join("a.txt"), b"original").unwrap();
    fs::write(f.instance.overwrite_dir().join("new.txt"), b"new context").unwrap();
    assert!(install_scripted_omod(
        &s,
        &c,
        &r,
        "NoMod",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false)
    )
    .is_err());
}
#[test]
fn unsupported_and_unapproved_profile_requests_require_explicit_incomplete_review() {
    use super::super::OmodFileKind::Plugin;
    let f = Fixture::new();
    let s = f.session(
        "EditINI General Name Changed\nSetGMST sample.esp test 1",
        &[("sample.esp", b"TES4fixture", Plugin)],
    );
    let (c, r) = f.review(&s);
    assert!(install_scripted_omod(
        &s,
        &c,
        &r,
        "Mod",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false)
    )
    .is_err());
    let result = install_scripted_omod(
        &s,
        &c,
        &r,
        "Mod",
        OverwritePolicy::Fail,
        ScriptedApproval {
            allow_incomplete: true,
            apply_profile_effects: false,
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(result.warnings.iter().any(|s| s.contains("SetGMST")));
    assert!(result.warnings.iter().any(|s| s.contains("EditINI")));
    assert!(
        fs::read_to_string(f.instance.active().ini_path("Oblivion.ini"))
            .unwrap()
            .contains("Original")
    );
}

#[test]
fn shader_requests_normalize_packages_and_stage_virtual_edits_without_source_writes() {
    use super::super::OmodFileKind::Data;
    let f = Fixture::new();
    fs::write(f.game.data_path.join("virtual.xml"), "first\nsecond\n").unwrap();
    let s=f.session("EditXMLLine virtual.xml 1 changed\nEditShader 01 lighting.PSO replacement.bin\nDontInstallDataFile replacement.bin",&[("replacement.bin",b"shader",Data)]);
    let (c, r) = f.review(&s);
    let result = install_scripted_omod(
        &s,
        &c,
        &r,
        "Shaders",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        fs::read(result.install.dest.join("Shaders/OMOD/1/LIGHTING.pso")).unwrap(),
        b"shader"
    );
    assert_eq!(
        fs::read(result.install.dest.join("virtual.xml")).unwrap(),
        b"first\nchanged\n"
    );
    assert_eq!(
        fs::read(f.game.data_path.join("virtual.xml")).unwrap(),
        b"first\nsecond\n"
    );
}
#[test]
fn invalid_shader_or_linked_callback_output_never_publishes() {
    use super::super::OmodFileKind::Data;
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let s = f.session(
        "EditShader 1 lighting.exe replacement.bin",
        &[("replacement.bin", b"shader", Data)],
    );
    let (c, r) = f.review(&s);
    assert!(install_scripted_omod(
        &s,
        &c,
        &r,
        "Shaders",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false)
    )
    .unwrap_err()
    .to_string()
    .contains("shader name"));
    assert!(!f.instance.mods_dir().join("Shaders").exists());
    let s = f.session("", &[("a.txt", b"owned", Data)]);
    let (c, r) = f.review(&s);
    let result = install_scripted_omod_with_finish(
        &s,
        &c,
        &r,
        "Linked",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false),
        |stage| {
            symlink(&f.game.data_path, stage.join("redirect"))?;
            Ok(())
        },
    );
    assert!(result.is_err());
    assert!(!f.instance.mods_dir().join("Linked").exists());
}
#[test]
fn root_mode_ini_fallback_and_extender_presence_use_selected_layers() {
    let f = Fixture::new();
    fs::remove_file(f.instance.active().ini_path("Oblivion.ini")).unwrap();
    fs::write(
        f.game.install_path.join("Oblivion.ini"),
        "[General]\nbUseMyGamesDirectory=0\nName=Root\n",
    )
    .unwrap();
    fs::create_dir_all(f.instance.root_overwrite_dir()).unwrap();
    fs::write(
        f.instance.root_overwrite_dir().join("obse_loader.exe"),
        b"mod loader",
    )
    .unwrap();
    let context = ScriptedContext::capture(&f.instance, &f.game, &AtomicBool::new(false)).unwrap();
    assert_eq!(context.interpreter.ini["general"]["name"], "Root");
    assert!(context.interpreter.script_extender_present);
    assert_eq!(context.interpreter.versions["OBMM"], "1.1.12");
    assert!(!context.interpreter.versions.contains_key("OBSE"));
    let private = f.instance.active().dir().join("runtime-root");
    fs::create_dir_all(&private).unwrap();
    fs::write(private.join(".eidoswh.obse_loader.exe"), []).unwrap();
    let hidden = ScriptedContext::capture(&f.instance, &f.game, &AtomicBool::new(false)).unwrap();
    assert!(!hidden.interpreter.script_extender_present);
}
#[test]
fn read_installed_receipt_is_bounded_shared_and_rejects_links() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    assert!(
        read_installed_receipt(&f.instance.mods_dir().join("Absent"))
            .unwrap()
            .is_none()
    );
    let s = f.session("", &[]);
    let (c, r) = f.review(&s);
    let installed = install_scripted_omod(
        &s,
        &c,
        &r,
        "Receipt",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        read_installed_receipt(&installed.install.dest)
            .unwrap()
            .unwrap(),
        r.receipt
    );
    fs::remove_file(&installed.receipt_path).unwrap();
    symlink(&s.archive, &installed.receipt_path).unwrap();
    assert!(read_installed_receipt(&installed.install.dest).is_err());
}

impl Fixture {
    fn pending_effects(&self, name: &str) -> PathBuf {
        use super::super::OmodFileKind::Plugin;
        let session = self.session(
            "EditINI General Name Changed",
            &[("sample.esp", b"TES4\0\0\0\0fixture", Plugin)],
        );
        let (c, r) = self.review(&session);
        let stage = self.instance.mods_dir().join(name);
        fs::create_dir(&stage).unwrap();
        stage::materialize(&session, &c, &r, &stage, &AtomicBool::new(false)).unwrap();
        effects::prepare(
            &c,
            &r,
            &stage,
            ScriptedApproval {
                apply_profile_effects: true,
                allow_incomplete: false,
            },
            Vec::new(),
            &AtomicBool::new(false),
        )
        .unwrap();
        stage
    }
}
#[test]
fn profile_retry_recovers_interrupted_write_and_retains_backup_images() {
    let f = Fixture::new();
    let installed = f.pending_effects("Interrupted");
    let record_path = effects::receipt_path(&installed);
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
    let first = &record["writes"][0];
    let image = installed
        .join(effects::RECEIPT_DIR)
        .join(format!("after-{}", first["image"]));
    let target = f
        .instance
        .active()
        .dir()
        .join(first["target"].as_str().unwrap());
    // Simulate a process stopping after its first atomic profile publication,
    // before it marks the installation receipt complete.
    fs::write(&target, fs::read(image).unwrap()).unwrap();
    assert!(!record["complete"].as_bool().unwrap());
    assert!(retry_omod_effects(&f.instance, "Interrupted", &AtomicBool::new(false)).unwrap());
    let active =
        fs::read_to_string(f.instance.active().plugins_state_dir().join("plugins.txt")).unwrap();
    assert!(active.contains("sample.esp"));
    assert!(fs::read_to_string(&target).unwrap().contains("Changed"));
    let backup = installed.join(effects::RECEIPT_DIR).join("before-0");
    assert!(fs::read_to_string(backup).unwrap().contains("Original"));
    let complete = fs::read(&record_path).unwrap();
    assert!(retry_omod_effects(&f.instance, "Interrupted", &AtomicBool::new(false)).unwrap());
    assert_eq!(fs::read(record_path).unwrap(), complete);
}
#[test]
fn retry_refuses_changed_profile_and_tampered_or_linked_effect_images() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let installed = f.pending_effects("Pending");
    let ini = f.instance.active().ini_path("Oblivion.ini");
    fs::write(&ini, "[General]\nName=UserEdited\n").unwrap();
    let error = retry_omod_effects(&f.instance, "Pending", &AtomicBool::new(false)).unwrap_err();
    assert!(error.to_string().contains("profile file changed"));
    assert!(!f
        .instance
        .active()
        .plugins_state_dir()
        .join("plugins.txt")
        .exists());
    assert!(fs::read_to_string(&ini).unwrap().contains("UserEdited"));
    fs::write(&ini, "[General]\nName=Original\n").unwrap();
    let image = installed.join(effects::RECEIPT_DIR).join("after-0");
    let good = fs::read(&image).unwrap();
    fs::write(&image, b"corrupt").unwrap();
    assert!(
        retry_omod_effects(&f.instance, "Pending", &AtomicBool::new(false))
            .unwrap_err()
            .to_string()
            .contains("checksum")
    );
    fs::remove_file(&image).unwrap();
    symlink(&ini, &image).unwrap();
    assert!(retry_omod_effects(&f.instance, "Pending", &AtomicBool::new(false)).is_err());
    fs::remove_file(&image).unwrap();
    fs::write(&image, good).unwrap();
    assert!(retry_omod_effects(&f.instance, "Pending", &AtomicBool::new(false)).unwrap());
}
#[test]
fn retry_rejects_ambiguous_profile_case_variants_before_any_change() {
    let f = Fixture::new();
    f.pending_effects("Collision");
    fs::write(
        f.instance.active().dir().join("oblivion.INI"),
        b"other case",
    )
    .unwrap();
    assert!(
        retry_omod_effects(&f.instance, "Collision", &AtomicBool::new(false))
            .unwrap_err()
            .to_string()
            .contains("ambiguous")
    );
    assert!(
        fs::read_to_string(f.instance.active().ini_path("Oblivion.ini"))
            .unwrap()
            .contains("Original")
    );
    assert!(!f
        .instance
        .active()
        .plugins_state_dir()
        .join("plugins.txt")
        .exists());
}

#[test]
fn known_handler_bracketed_ini_sections_normalize_exactly_once() {
    let f = Fixture::new();
    let session = f.session("", &[]);
    let (c, mut r) = f.review(&session);
    r.profile_effects.push(ObmmEffect {
        line: 927,
        command: "EditINI".into(),
        arguments: vec!["[Display]".into(), "bLocalMapShader".into(), "0".into()],
    });
    let stage = f.instance.mods_dir().join("NativeUI");
    fs::create_dir(&stage).unwrap();
    effects::prepare(
        &c,
        &r,
        &stage,
        ScriptedApproval {
            apply_profile_effects: true,
            allow_incomplete: false,
        },
        Vec::new(),
        &AtomicBool::new(false),
    )
    .unwrap();
    retry_omod_effects(&f.instance, "NativeUI", &AtomicBool::new(false)).unwrap();
    let text = fs::read_to_string(f.instance.active().ini_path("Oblivion.ini")).unwrap();
    assert_eq!(
        eidos_ini::get_key(&text, "Display", "bLocalMapShader"),
        Some("0")
    );
    assert!(!text.contains("[[Display]]"));
}

#[test]
fn archive_change_during_finish_refuses_publication() {
    use super::super::OmodFileKind::Data;
    let f = Fixture::new();
    let s = f.session("", &[("a.txt", b"owned", Data)]);
    let (c, r) = f.review(&s);
    let result = install_scripted_omod_with_finish(
        &s,
        &c,
        &r,
        "ChangedArchive",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false),
        |_| {
            fs::write(&s.archive, b"changed after review")?;
            Ok(())
        },
    );
    assert!(result.is_err());
    assert!(!f.instance.mods_dir().join("ChangedArchive").exists());
}

#[test]
fn semantically_unchanged_modlist_save_keeps_receipt_but_activation_invalidates() {
    let f = Fixture::new();
    let reserved = f.instance.mods_dir().join("Reserved");
    fs::create_dir(&reserved).unwrap();
    fs::write(reserved.join("a.txt"), b"reserved").unwrap();
    let stop = AtomicBool::new(false);
    let before = ScriptedContext::capture(&f.instance, &f.game, &stop).unwrap();
    f.instance.save_modlist(&f.instance.modlist()).unwrap();
    let saved = ScriptedContext::capture(&f.instance, &f.game, &stop).unwrap();
    assert_eq!(before.origin.context_sha256, saved.origin.context_sha256);
    let mut mods = f.instance.modlist();
    mods.iter_mut()
        .find(|m| m.name == "Reserved")
        .unwrap()
        .enabled = true;
    f.instance.save_modlist(&mods).unwrap();
    let active = ScriptedContext::capture(&f.instance, &f.game, &stop).unwrap();
    assert_ne!(before.origin.context_sha256, active.origin.context_sha256);
}

#[test]
fn missing_or_changed_backup_prevents_pending_profile_publication() {
    let f = Fixture::new();
    let installed = f.pending_effects("Backup");
    let backup = installed.join(effects::RECEIPT_DIR).join("before-0");
    fs::write(&backup, b"corrupt backup").unwrap();
    assert!(retry_omod_effects(&f.instance, "Backup", &AtomicBool::new(false)).is_err());
    assert!(
        fs::read_to_string(f.instance.active().ini_path("Oblivion.ini"))
            .unwrap()
            .contains("Original")
    );
    assert!(!f
        .instance
        .active()
        .plugins_state_dir()
        .join("plugins.txt")
        .exists());
}

#[test]
fn cancelled_omod_probe_stops_before_directory_inspection_or_staging() {
    let f = Fixture::new();
    let result = super::super::try_open_omod_with(
        &f.root.join("missing.zip"),
        &f.root.join("must-not-exist"),
        &AtomicBool::new(true),
        |_| {},
    );
    assert!(result
        .err()
        .is_some_and(|e| e.to_string().contains("cancel")));
    assert!(!f.root.join("must-not-exist").exists());
}

#[test]
fn modified_generated_file_review_is_rejected_before_publication() {
    let f = Fixture::new();
    let session = f.session("", &[]);
    let (c, mut review) = f.review(&session);
    review.generated_files.push("unexpected.bsa".into());
    assert!(install_scripted_omod(
        &session,
        &c,
        &review,
        "ChangedPlan",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(!f.instance.mods_dir().join("ChangedPlan").exists());
}

#[test]
fn mixed_case_virtual_xml_edits_share_one_staged_destination() {
    let f = Fixture::new();
    fs::create_dir(f.game.data_path.join("menus")).unwrap();
    fs::write(f.game.data_path.join("menus/a.xml"), b"old").unwrap();
    let s = f.session(
        "EditXMLReplace menus/a.xml old middle\nEditXMLReplace MENUS/A.XML middle final",
        &[],
    );
    let (c, r) = f.review(&s);
    let result = install_scripted_omod(
        &s,
        &c,
        &r,
        "CaseEdits",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        fs::read(result.install.dest.join("menus/a.xml")).unwrap(),
        b"final"
    );
    assert!(!result.install.dest.join("MENUS").exists());
    assert_eq!(
        fs::read(f.game.data_path.join("menus/a.xml")).unwrap(),
        b"old"
    );
}
#[test]
fn shader_larger_than_launch_limit_is_rejected_before_publication() {
    use super::super::OmodFileKind::Data;
    let f = Fixture::new();
    let bytes = vec![0x41; 8 * 1024 * 1024 + 1];
    let s = f.session(
        "EditShader 1 test.pso replacement.bin",
        &[("replacement.bin", &bytes, Data)],
    );
    let (c, r) = f.review(&s);
    assert!(install_scripted_omod(
        &s,
        &c,
        &r,
        "OversizedShader",
        OverwritePolicy::Fail,
        ScriptedApproval::default(),
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(!f.instance.mods_dir().join("OversizedShader").exists());
}
