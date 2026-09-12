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
    for stash in [&data_stash, &root_stash] {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(stash.as_os_str().as_bytes()).unwrap();
        // SAFETY: private-namespace mountpoints created only by this test.
        unsafe {
            libc::umount2(name.as_ptr(), libc::MNT_DETACH);
        }
    }
    fs::remove_dir_all(root).unwrap();
    println!("root Overwrite-only launch, whiteout, nested Data and write isolation: passed");
}
