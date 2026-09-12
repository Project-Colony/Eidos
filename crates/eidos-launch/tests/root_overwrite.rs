//! Real isolated launch regression: root Overwrite without an enabled Root mod.
use eidos_launch::{launch, LaunchSpec};
use std::{fs, path::Path};

fn put(root: &Path, name: &str, bytes: &[u8]) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn main() {
    let root = std::env::temp_dir().join(format!("eidos-root-overwrite-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let game = root.join("game");
    let overwrite = root.join("overwrite");
    let root_overwrite = overwrite.join("Root");
    let data_stash = root.join("data-stash");
    let root_stash = root.join("root-stash");
    put(&game, "Data/base.esm", b"master");
    put(&game, "hidden.exe", b"hidden");
    put(&root_overwrite, "skse64_loader.exe", b"loader");
    put(&root_overwrite, ".eidoswh.hidden.exe", b"");
    let status = launch(LaunchSpec {
        plugin_timestamps: None,
        layers: vec![],
        overwrite: overwrite.clone(),
        mountpoint: game.join("Data"),
        command: vec!["/bin/sh".into(), "-ec".into(),
            "test \"$(cat skse64_loader.exe)\" = loader; test ! -e hidden.exe; test \"$(cat Data/base.esm)\" = master; printf root-output > output.log; printf data-output > Data/output.txt".into()],
        env: vec![],
        base_bind: Some((game.join("Data"), data_stash.clone())),
        binds: vec![],
        cwd: Some(game.clone()),
        root_layers: vec![],
        root_overwrite: Some(root_overwrite.clone()),
        root_base_bind: Some((game.clone(), root_stash.clone())),
    }).expect("isolated synthetic launch");
    assert!(
        status.success(),
        "root Overwrite must be mounted even without Root mods"
    );
    assert_eq!(
        fs::read(root_overwrite.join("output.log")).unwrap(),
        b"root-output"
    );
    assert_eq!(
        fs::read(overwrite.join("output.txt")).unwrap(),
        b"data-output"
    );
    assert!(!game.join("output.log").exists());
    assert!(game.join("hidden.exe").exists());
    assert!(!game.join("Data/output.txt").exists());
    // Each profile's private upper wins over shared Root INIs, supports atomic
    // replacement, and captures only writes. Existing shared payloads stay lower.
    put(&game, "Morrowind.ini", b"real settings");
    put(&root_overwrite, "Morrowind.ini", b"stale shared settings");
    let source_time = fs::metadata(game.join("Data/base.esm"))
        .unwrap()
        .modified()
        .unwrap();
    let mut additional_stashes = Vec::new();
    for name in ["profile-a", "profile-b"] {
        let private = root.join(name).join("runtime-root");
        put(&private, "Morrowind.ini", name.as_bytes());
        let data_view = root.join(format!("{name}-data"));
        let root_view = root.join(format!("{name}-root"));
        let receipt = root.join(name).join("plugin-times.pending");
        let projected = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1234);
        let status=launch(LaunchSpec {
            plugin_timestamps:Some(eidos_launch::PluginTimestamps {times:std::collections::BTreeMap::from([("base.esm".into(),projected)]),state_path:receipt.clone()}),
            layers:vec![],overwrite:overwrite.clone(),mountpoint:game.join("Data"),
            command:vec!["python3".into(),"-c".into(),"import os,sys; assert open('Morrowind.ini').read()==sys.argv[1]; assert open('skse64_loader.exe').read()=='loader'; p='Data/base.esm'; f=open(p,'rb'); assert os.stat(p).st_mtime_ns==1234000000000; assert os.fstat(f.fileno()).st_mtime_ns==1234000000000; os.utime(p, ns=(1500000000000,1500000000000)); assert os.fstat(f.fileno()).st_mtime_ns==1500000000000; open('Morrowind.new','w').write(sys.argv[1]+' captured'); os.replace('Morrowind.new','Morrowind.ini'); open('private-output.log','w').write(sys.argv[1])".into(),name.into()],
            env:vec![],base_bind:Some((game.join("Data"),data_view.clone())),binds:vec![],cwd:Some(game.clone()),root_layers:vec![root_overwrite.clone()],root_overwrite:Some(private.clone()),root_base_bind:Some((game.clone(),root_view.clone())),
        }).unwrap();
        assert!(status.success());
        assert_eq!(
            fs::read_to_string(private.join("Morrowind.ini")).unwrap(),
            format!("{name} captured")
        );
        assert_eq!(
            fs::read_to_string(private.join("private-output.log")).unwrap(),
            name
        );
        assert_eq!(
            eidos_launch::read_plugin_mtimes(&receipt).unwrap()["base.esm"],
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(1500)
        );
        assert!(!overwrite.join("base.esm").exists());
        assert!(!private.join("skse64_loader.exe").exists());
        assert_eq!(
            fs::read(game.join("Morrowind.ini")).unwrap(),
            b"real settings"
        );
        assert_eq!(
            fs::read(root_overwrite.join("Morrowind.ini")).unwrap(),
            b"stale shared settings"
        );
        assert_eq!(
            fs::metadata(game.join("Data/base.esm"))
                .unwrap()
                .modified()
                .unwrap(),
            source_time
        );
        additional_stashes.extend([data_view, root_view]);
    }
    for stash in [&data_stash, &root_stash]
        .into_iter()
        .chain(additional_stashes.iter())
    {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(stash.as_os_str().as_bytes()).unwrap();
        // SAFETY: private-namespace mountpoints created only by this test.
        unsafe {
            libc::umount2(name.as_ptr(), libc::MNT_DETACH);
        }
    }
    fs::remove_dir_all(root).unwrap();
    println!("root Overwrite, two private profile INIs, atomic replacement, timestamps and write isolation: passed");
}
