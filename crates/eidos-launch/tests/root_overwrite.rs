//! Real isolated launch regression: root Overwrite without an enabled Root mod.
use eidos_launch::{launch, LaunchSpec};
use std::{fs, path::Path};

fn put(root: &Path, name: &str, bytes: &[u8]) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn detach(path: &Path) {
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: only synthetic mountpoints in this test's private namespace.
    unsafe {
        libc::umount2(name.as_ptr(), libc::MNT_DETACH);
    }
}

fn mount_boundaries(root: &Path) {
    for mode in [
        "absent-root",
        "empty-root",
        "marker-root",
        "directory-root",
        "empty-root-mod",
        "nested-absent",
        "nested-empty",
    ] {
        let fixture = root.join(mode);
        let game = fixture.join("game");
        let overwrite = fixture.join("overwrite");
        let root_upper = fixture.join("root-upper");
        let data_stash = fixture.join("data-stash");
        let root_stash = fixture.join("root-stash");
        let data_layer = fixture.join("data-layer");
        let root_layer = fixture.join("root-layer");
        let nested = mode.starts_with("nested-");
        let data_relative = if nested { "Content/DaTa" } else { "Data" };
        let data = game.join(data_relative);
        put(&game, "hidden.exe", b"original");
        put(&game, "visible.exe", b"original");
        put(&data_layer, "mod.esm", b"mod data");
        fs::create_dir_all(&overwrite).unwrap();
        if mode != "absent-root" {
            fs::create_dir_all(&root_upper).unwrap();
        }
        if mode != "nested-absent" {
            fs::create_dir_all(&data).unwrap();
        }
        if mode == "marker-root" {
            put(&root_upper, ".eidoswh.hidden.exe", b"");
        }
        if mode == "directory-root" {
            fs::create_dir_all(root_upper.join("empty")).unwrap();
        }
        let root_layers = if mode == "empty-root-mod" || nested {
            fs::create_dir_all(&root_layer).unwrap();
            vec![root_layer]
        } else {
            vec![]
        };
        let root_mounted = !matches!(mode, "absent-root" | "empty-root");
        let start = std::time::Instant::now();
        let status = launch(LaunchSpec {
        root_readonly_overwrite: None,
            plugin_timestamps: None, layers: vec![data_layer], overwrite: overwrite.clone(),
            mountpoint: data.clone(),
            command: vec!["python3".into(), "-c".into(), r#"
import os,sys,time
game,data,data_stash,root_stash,mode,mounted = sys.argv[1:]
mounted = mounted == 'true'
mounts = [line.split() for line in open('/proc/self/mountinfo')]
at = lambda p: [m for m in mounts if m[4] == p]
assert len(at(game)) == int(mounted), (mode, at(game))
assert len(at(data)) == 1, (mode, at(data))
assert open(os.path.join(data,'mod.esm')).read() == 'mod data'
assert open('visible.exe').read() == 'original'
assert os.path.exists('hidden.exe') == (mode != 'marker-root')
if mode != 'nested-absent':
    assert os.stat(data_stash).st_dev != os.stat(data).st_dev
    if mounted:
        assert os.stat(data_stash).st_dev == os.stat(root_stash).st_dev
        assert os.stat(root_stash).st_dev != os.stat(game).st_dev
open('new-root-output.log','w').write(mode)
open(os.path.join(data,'new-data-output.log'),'w').write(mode)
start=time.monotonic_ns()
for _ in range(2000):
    assert os.stat(os.path.join(data,'mod.esm')).st_size == 8
print('mounted metadata loop',mode,':',round((time.monotonic_ns()-start)/1e6,3),'ms / 2000 warm Data stats')
"#.into(), game.display().to_string(), data.display().to_string(), data_stash.display().to_string(), root_stash.display().to_string(), mode.into(), root_mounted.to_string()],
            env: vec![], base_bind: (mode != "nested-absent").then(|| (data.clone(), data_stash.clone())),
            binds: vec![], cwd: Some(game.clone()), root_layers, root_overwrite: Some(root_upper.clone()), root_base_bind: Some((game.clone(), root_stash.clone())),
        }).unwrap();
        assert!(status.success(), "{mode}");
        println!(
            "synthetic launch {mode}: {:.3} ms including child and teardown",
            start.elapsed().as_secs_f64() * 1000.0
        );
        assert_eq!(
            fs::read(overwrite.join("new-data-output.log")).unwrap(),
            mode.as_bytes()
        );
        assert!(!data.join("new-data-output.log").exists());
        let output_root = if root_mounted { &root_upper } else { &game };
        assert_eq!(
            fs::read(output_root.join("new-root-output.log")).unwrap(),
            mode.as_bytes()
        );
        if root_mounted {
            assert!(!game.join("new-root-output.log").exists());
        }
        if mode == "nested-absent" {
            assert!(!game.join("Content").exists());
            assert!(root_upper.join(data_relative).is_dir());
        }
        assert_eq!(fs::read(game.join("hidden.exe")).unwrap(), b"original");
        detach(&data_stash);
        detach(&root_stash);
    }
}

fn main() {
    let root = std::env::temp_dir().join(format!("eidos-root-overwrite-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    mount_boundaries(&root);
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
        root_readonly_overwrite: None,
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
    let high_root = root.join("high-root-mod");
    let low_root = root.join("low-root-mod");
    for (dir, bytes) in [
        (&game, b"vanilla".as_slice()),
        (&low_root, b"low"),
        (&high_root, b"high"),
        (&root_overwrite, b"shared"),
    ] {
        put(dir, "upper-wins.dll", bytes);
    }
    for (dir, bytes) in [
        (&game, b"vanilla".as_slice()),
        (&low_root, b"low"),
        (&high_root, b"high"),
    ] {
        put(dir, "mod-wins.dll", bytes);
    }
    put(&game, "lower-wins.dll", b"vanilla");
    put(&low_root, "lower-wins.dll", b"low");
    put(&high_root, "shared-wins.dll", b"high");
    put(&root_overwrite, "shared-wins.dll", b"shared");
    put(&game, "vanilla-only.dll", b"vanilla");
    put(&high_root, "Data/base.esm", b"shadowed root Data");
    put(&game, "Cache/base.dll", b"hidden by shared opacity");
    put(&root_overwrite, "Cache/.eidoswh_opaque", b"");
    put(&root_overwrite, "Cache/shared.dll", b"shared cache");
    put(&game, "Recreated/game.bin", b"game child");
    put(&root_overwrite, "Recreated/shared.bin", b"shared child");
    let source_time = fs::metadata(game.join("Data/base.esm"))
        .unwrap()
        .modified()
        .unwrap();
    let mut additional_stashes = Vec::new();
    for name in ["profile-a", "profile-b"] {
        let private = root.join(name).join("runtime-root");
        put(&private, "Morrowind.ini", name.as_bytes());
        put(&private, "upper-wins.dll", b"private");
        put(&private, "Cache/private.dll", b"private cache");
        let data_view = root.join(format!("{name}-data"));
        let root_view = root.join(format!("{name}-root"));
        let receipt = root.join(name).join("plugin-times.pending");
        let projected = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1234);
        let status = launch(LaunchSpec {
            root_readonly_overwrite: Some(root_overwrite.clone()),
            plugin_timestamps: Some(eidos_launch::PluginTimestamps {
                times: std::collections::BTreeMap::from([("base.esm".into(), projected)]),
                state_path: receipt.clone(),
            }),
            layers: vec![],
            overwrite: overwrite.clone(),
            mountpoint: game.join("Data"),
            command: vec![
                "python3".into(),
                "-c".into(),
                r#"
import os,sys,shutil
assert open('Morrowind.ini').read() == sys.argv[1]
assert not os.path.exists('hidden.exe')
for path,value in [('skse64_loader.exe','loader'), ('upper-wins.dll','private'),
                   ('shared-wins.dll','shared'), ('mod-wins.dll','high'),
                   ('lower-wins.dll','low'), ('vanilla-only.dll','vanilla')]:
    assert open(path).read() == value
assert not os.path.exists('Cache/base.dll')
assert sorted(os.listdir('Cache')) == ['private.dll','shared.dll']
assert open('Cache/shared.dll').read() == 'shared cache'
with open('Cache/shared.dll','r+') as changed:
    changed.write('private edit')
os.rename('Cache/shared.dll','Cache/moved.dll')
assert not os.path.exists('Cache/shared.dll')
assert open('Cache/moved.dll').read() == 'private edit'
shutil.rmtree('Recreated')
os.mkdir('Recreated')
assert os.listdir('Recreated') == []
open('Recreated/new.bin','w').write('new child')
assert os.listdir('Recreated') == ['new.bin']
open('hidden.exe','w').write('private recreation')
assert open('hidden.exe').read() == 'private recreation'
os.unlink('hidden.exe')
assert not os.path.exists('hidden.exe')
p='Data/base.esm'
f=open(p,'rb')
assert f.read() == b'master'
assert os.stat(p).st_mtime_ns == 1234000000000
assert os.fstat(f.fileno()).st_mtime_ns == 1234000000000
os.utime(p, ns=(1500000000000,1500000000000))
assert os.fstat(f.fileno()).st_mtime_ns == 1500000000000
open('Morrowind.new','w').write(sys.argv[1]+' captured')
os.replace('Morrowind.new','Morrowind.ini')
open('private-output.log','w').write(sys.argv[1])
"#
                .into(),
                name.into(),
            ],
            env: vec![],
            base_bind: Some((game.join("Data"), data_view.clone())),
            binds: vec![],
            cwd: Some(game.clone()),
            root_layers: vec![high_root.clone(), low_root.clone()],
            root_overwrite: Some(private.clone()),
            root_base_bind: Some((game.clone(), root_view.clone())),
        })
        .unwrap();
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
        assert!(!private.join("Cache/.eidoswh_opaque").exists());
        assert!(!private.join("Cache/.eidoswh.base.dll").exists());
        assert_eq!(
            fs::read(root_overwrite.join("Cache/shared.dll")).unwrap(),
            b"shared cache"
        );
        assert!(root_overwrite.join("Cache/.eidoswh_opaque").is_file());
        assert!(private.join("Recreated/.eidoswh_opaque").is_file());
        assert_eq!(
            fs::read(root_overwrite.join("Recreated/shared.bin")).unwrap(),
            b"shared child"
        );
        assert_eq!(
            fs::read(game.join("Recreated/game.bin")).unwrap(),
            b"game child"
        );
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
        detach(stash);
    }
    fs::remove_dir_all(root).unwrap();
    println!("root Overwrite, two private profile INIs, atomic replacement, timestamps and write isolation: passed");
}
