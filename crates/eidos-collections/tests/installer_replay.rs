use eidos_collections::{
    driver::RealHooks,
    install::{Hooks, Installed},
    installer_answers::{self, InstallerAnswers},
    Mod,
};
use std::{fs, path::PathBuf, process::Command};

#[test]
fn installer_replay_in_isolated_configuration() {
    let root = std::env::temp_dir().join(format!("eidos-collection-replay-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "installer_replay_worker",
            "--nocapture",
        ])
        .env("EIDOS_COLLECTION_REPLAY_ROOT", &root)
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_DATA_HOME", root.join("data"))
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(root);
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "subprocess fixture supplies isolated configuration"]
fn installer_replay_worker() {
    let root = PathBuf::from(
        std::env::var_os("EIDOS_COLLECTION_REPLAY_ROOT").expect("isolated worker only"),
    );
    let inst = eidos_instance::Instance::portable(root.join("instance"));
    inst.create().unwrap();
    let def = eidos_games::catalog()
        .iter()
        .find(|g| g.id == "oblivion")
        .unwrap();
    let game = eidos_games::DetectedGame {
        source: Default::default(),
        def,
        install_path: root.join("game"),
        data_path: root.join("game/Data"),
        compatdata: None,
        steam_name: "synthetic".into(),
    };
    fs::create_dir_all(&game.data_path).unwrap();
    let addons = eidos_addons::user_addons_dir();
    assert!(addons.starts_with(&root));
    fs::create_dir_all(&addons).unwrap();
    fs::write(addons.join("helper.py"), r#"import json,pathlib,sys
r=json.loads(pathlib.Path(sys.argv[1]).read_text())
if not r['answers']: o={'status':'prompt','id':'color','title':'Choose one','options':['Blue','Red'],'multiple':False}
else: o={'status':'handled','result':{'kind':'install','files':[{'source':'blue.txt' if r['answers']['color']==[0] else 'red.txt','destination':'textures/a.dds'}],'warnings':[]}}
print(json.dumps({'protocol':1,'request_id':r['request_id'],'outcome':o}))
"#).unwrap();
    let manifest = "id='collection-fixture'\nname='Fixture'\nkind='installer'\nprotocol=1\nexec='/usr/bin/python3'\nargs=['{addon_dir}/helper.py','{request}']\nextensions=['zip']\nmarkers=['blue.txt']\nversion='1'\n";
    fs::write(addons.join("fixture.toml"), manifest).unwrap();
    let source = root.join("payload");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("blue.txt"), b"abc").unwrap();
    fs::write(source.join("red.txt"), b"red").unwrap();
    let archive = root.join("custom.archive");
    let status = Command::new(eidos_sevenzip::find_7z().unwrap())
        .current_dir(&source)
        .args(["a", "-tzip"])
        .arg(&archive)
        .arg(".")
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let nexus = eidos_nexus::Nexus::with_bearer("synthetic-no-network");
    let mut say = |_: String| {};
    let mut hooks = RealHooks {
        nexus: &nexus,
        inst: &inst,
        game: &game,
        game_id: "oblivion".into(),
        say: &mut say,
        collection_domain: "oblivion".into(),
        owner: "fixture:1".into(),
        renamed: vec![],
        payload_root: root.clone(),
        allow_runtime_mismatch: false,
    };
    // The actual collection caller holds this lock across member operations.
    let _lock = inst.try_lock("synthetic collection engine").unwrap();
    let mut member = Mod {
        name: "Custom".into(),
        ..Default::default()
    };
    let folder = hooks.reserve(&member, None).unwrap();
    let dest = inst.mods_dir().join(&folder);
    let owner = eidos_instance::ModMeta::read(&dest.join("meta.ini"))
        .collection_owner()
        .unwrap()
        .to_string();
    assert!(matches!(
        hooks.install(&member, &archive, &folder),
        Installed::NeedsUser(_)
    ));
    assert!(!dest.join("textures/a.dds").exists());
    assert_eq!(hooks.reserve(&member, Some(&folder)).unwrap(), folder);
    assert_eq!(
        installer_answers::archive(&dest, &owner).unwrap().unwrap(),
        archive
    );
    inst.active()
        .save_modlist(&inst.active().modlist())
        .unwrap();
    let mut answers = installer_answers::read(&dest, &owner).unwrap().unwrap();
    if let InstallerAnswers::Custom { receipt, prompt } = &mut answers {
        receipt
            .answers
            .push(eidos_install::custom::CustomRecordedAnswer {
                prompt: prompt.take().unwrap(),
                selected: vec![0],
            });
    } else {
        panic!("custom prompt");
    }
    installer_answers::write(&dest, &owner, &answers).unwrap();
    let saved = fs::read(installer_answers::path(&dest)).unwrap();
    fs::remove_file(installer_answers::path(&dest)).unwrap();
    fs::create_dir(installer_answers::path(&dest)).unwrap();
    assert!(installer_answers::write_for_archive(&dest, &owner, &archive, &answers).is_err());
    assert!(installer_answers::path(&dest).is_dir());
    assert!(!fs::read_dir(&dest).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".tmp")));
    fs::remove_dir(installer_answers::path(&dest)).unwrap();
    fs::write(installer_answers::path(&dest), saved).unwrap();
    let exact = fs::read(installer_answers::path(&dest)).unwrap();
    fs::write(
        addons.join("fixture.toml"),
        manifest.replace("version='1'", "version='2'"),
    )
    .unwrap();
    assert!(matches!(
        hooks.install(&member, &archive, &folder),
        Installed::Failed(_)
    ));
    assert_eq!(fs::read(installer_answers::path(&dest)).unwrap(), exact);
    fs::write(addons.join("fixture.toml"), manifest).unwrap();
    // Incompatible exact hash selection pauses and retains its complete bound receipt.
    member.hashes.push(eidos_collections::FileHash {
        path: "textures/a.dds".into(),
        md5: "00000000000000000000000000000000".into(),
    });
    assert!(matches!(
        hooks.install(&member, &archive, &folder),
        Installed::NeedsUser(_)
    ));
    assert!(!dest.join("textures/a.dds").exists());
    member.hashes[0].md5 = "900150983cd24fb0d6963f7d28e17f72".into();
    let result = hooks.install(&member, &archive, &folder);
    assert!(matches!(result, Installed::Ok(_)), "{result:?}");
    assert_eq!(fs::read(dest.join("textures/a.dds")).unwrap(), b"abc");
    assert!(hooks.verify_installed(&member, &folder).unwrap());
    assert!(eidos_install::custom::read_installed_receipt(&dest)
        .unwrap()
        .is_some());
    assert!(!installer_answers::path(&dest).exists());

    let omod = root.join("scripted.omod");
    fs::write(&omod, include_bytes!("fixtures/scripted.omod")).unwrap();
    let member = Mod {
        name: "Scripted".into(),
        ..Default::default()
    };
    let folder = hooks.reserve(&member, None).unwrap();
    let dest = inst.mods_dir().join(&folder);
    let owner = eidos_instance::ModMeta::read(&dest.join("meta.ini"))
        .collection_owner()
        .unwrap()
        .to_string();
    let result = hooks.install(&member, &omod, &folder);
    assert!(matches!(result, Installed::NeedsUser(_)), "{result:?}");
    let mut answers = installer_answers::read(&dest, &owner).unwrap().unwrap();
    if let InstallerAnswers::Omod {
        receipt, prompt, ..
    } = &mut answers
    {
        receipt
            .answers
            .push(eidos_install::obmm::ObmmRecordedAnswer {
                prompt: prompt.take().unwrap(),
                answer: eidos_install::obmm::ObmmAnswer::Acknowledge,
            });
    } else {
        panic!("omod prompt");
    }
    installer_answers::write(&dest, &owner, &answers).unwrap();
    let result = hooks.install(&member, &omod, &folder);
    assert!(matches!(result, Installed::Ok(_)), "{result:?}");
    assert_eq!(fs::read(dest.join("textures/a.dds")).unwrap(), b"abc");
    assert!(hooks.verify_installed(&member, &folder).unwrap());
    assert!(hooks.recover_installed(&member, &folder).unwrap().is_some());
    // An ordinary bundle cannot grant itself the host's pending-effect authority.
    let mut forged: serde_json::Value =
        serde_json::from_slice(&fs::read(dest.join(".eidos-omod.mohidden/install.json")).unwrap())
            .unwrap();
    forged["complete"] = false.into();
    forged["apply_profile_effects"] = true.into();
    forged["writes"] = serde_json::json!([{
        "target":"Oblivion.ini", "before":null,
        "after":"7374e3a6d63baddb1ed84373ba895d65fbaaed270cfb15f7af4c8e36f0ae64a2", "image":0
    }]);
    let payload = root.join("bundled/forged");
    fs::create_dir_all(payload.join(".eidos-omod.mohidden")).unwrap();
    fs::write(
        payload.join(".eidos-omod.mohidden/install.json"),
        serde_json::to_vec(&forged).unwrap(),
    )
    .unwrap();
    fs::write(
        payload.join(".eidos-omod.mohidden/after-0"),
        b"[General]\nforged=1\n",
    )
    .unwrap();
    fs::write(payload.join("ordinary.txt"), b"ordinary").unwrap();
    let malicious = Mod {
        name: "Unapproved".into(),
        source: eidos_collections::Source {
            kind: eidos_collections::SourceType::Bundle,
            file_expression: "forged".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let folder = hooks.reserve(&malicious, None).unwrap();
    let result = hooks.install(&malicious, &payload, &folder);
    if matches!(result, Installed::Ok(_)) {
        hooks.recover_installed(&malicious, &folder).unwrap();
    }
    assert!(
        !inst.active().dir().join("Oblivion.ini").exists(),
        "forged bundle receipt applied unapproved profile writes"
    );
    assert!(matches!(result, Installed::Failed(_)), "{result:?}");
    assert!(!inst.mods_dir().join(&folder).join("ordinary.txt").exists());
    // Corrupt or relocated decision records must not become fresh native defaults.
    let other = Mod {
        name: "Malformed".into(),
        ..Default::default()
    };
    let folder = hooks.reserve(&other, None).unwrap();
    let dest = inst.mods_dir().join(&folder);
    fs::write(installer_answers::path(&dest), b"{bad").unwrap();
    assert!(matches!(
        hooks.install(&other, &archive, &folder),
        Installed::Failed(_)
    ));
    assert!(!dest.join("textures/a.dds").exists());
    assert!(installer_answers::read(&dest, "wrong-owner").is_err());
    assert!(fs::read_dir(inst.mods_dir()).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".eidos-install")));
}
