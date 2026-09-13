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
        readonly_data_binds: vec![],
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

fn readonly_data_projection(root: &Path) {
    for mode in ["valid", "missing", "escape", "symlink", "duplicate"] {
        let fixture = root.join(format!("shader-{mode}"));
        let game = fixture.join("game");
        let data = game.join("Data");
        let overwrite = fixture.join("overwrite");
        let stash = fixture.join("data-stash");
        let stage = fixture.join("stage.sdp");
        put(&data, "Shaders/shaderpackage001.sdp", b"vanilla");
        put(&overwrite, "Shaders/shaderpackage001.sdp", b"overwrite");
        fs::write(&stage, b"projected").unwrap();
        let relative = std::path::PathBuf::from("Shaders/shaderpackage001.sdp");
        let mut mappings = vec![(stage.clone(), relative.clone())];
        match mode {
            "missing" => mappings.push((stage.clone(), "Shaders/missing.sdp".into())),
            "escape" => mappings.push((stage.clone(), "../outside.sdp".into())),
            "symlink" => {
                std::os::unix::fs::symlink(&stage, fixture.join("link.sdp")).unwrap();
                mappings.push((fixture.join("link.sdp"), relative.clone()));
            }
            "duplicate" => mappings.push((stage.clone(), "shaders/SHADERPACKAGE001.SDP".into())),
            _ => {}
        }
        let marker = fixture.join("ran");
        let result = launch(LaunchSpec {
            readonly_data_binds: mappings,
            root_readonly_overwrite: None,
            plugin_timestamps: None,
            layers: vec![],
            overwrite: overwrite.clone(),
            mountpoint: data.clone(),
            command: vec![
                "python3".into(),
                "-c".into(),
                r#"
import errno,pathlib,sys
p=pathlib.Path('Data/Shaders/shaderpackage001.sdp')
assert p.read_bytes() == b'projected'
try:
    p.write_bytes(b'bad')
except OSError as e:
    assert e.errno == errno.EROFS, e
else:
    raise AssertionError('projection is writable')
pathlib.Path('Data/ordinary.txt').write_text('output')
pathlib.Path(sys.argv[1]).write_text('ran')
"#
                .into(),
                marker.display().to_string(),
            ],
            env: vec![],
            base_bind: Some((data.clone(), stash.clone())),
            binds: vec![],
            cwd: Some(game),
            root_layers: vec![],
            root_overwrite: None,
            root_base_bind: None,
        });
        if mode == "valid" {
            assert!(result.unwrap().success());
            assert!(marker.exists());
            assert_eq!(fs::read(overwrite.join("ordinary.txt")).unwrap(), b"output");
        } else {
            assert!(result.is_err(), "{mode}");
            assert!(!marker.exists(), "invalid projection ran the game: {mode}");
        }
        assert_eq!(fs::read(data.join(&relative)).unwrap(), b"vanilla");
        assert_eq!(fs::read(overwrite.join(&relative)).unwrap(), b"overwrite");
        assert_eq!(fs::read(&stage).unwrap(), b"projected");
        detach(&stash);
    }
}

fn morrowind_root_saves(root: &Path) {
    for mode in [
        "missing",
        "existing",
        "higher-root",
        "failure",
        "symlink",
        "duplicate",
        "escape",
    ] {
        let fixture = root.join(format!("morrowind-saves-{mode}"));
        let game = fixture.join("game");
        let data = game.join("Data Files");
        let overwrite = fixture.join("overwrite");
        let root_upper = fixture.join("root-upper");
        let profile = fixture.join("profile/saves");
        let root_mod = fixture.join("root-mod");
        let data_stash = fixture.join("data-stash");
        let root_stash = fixture.join("root-stash");
        put(&data, "base.esm", b"master");
        put(&profile, "Slot.ess", b"profile");
        put(&profile, "Slot.mwse", b"cosave");
        if mode == "symlink" {
            put(&fixture, "outside/Slot.ess", b"original");
            std::os::unix::fs::symlink(fixture.join("outside"), game.join("Saves")).unwrap();
        } else if mode != "missing" {
            put(&game, "Saves/Slot.ess", b"original");
        }
        if mode == "higher-root" {
            put(&root_mod, "Saves/Slot.ess", b"root mod");
        }
        let marker = fixture.join("ran");
        let mut binds = vec![(profile.clone(), game.join("Saves"))];
        if mode == "failure" {
            put(&fixture, "not-a-directory", b"obstruction");
            binds.push((fixture.join("not-a-directory"), game.join("Other")));
        }
        if mode == "duplicate" {
            binds.push((profile.clone(), game.join("sAvEs")));
        }
        if mode == "escape" {
            binds.push((profile.clone(), game.join("../escape")));
        }
        let result = launch(LaunchSpec {
            readonly_data_binds: vec![],
            root_readonly_overwrite: None,
            plugin_timestamps: None,
            layers: vec![],
            overwrite: overwrite.clone(),
            mountpoint: data.clone(),
            command: vec![
                "python3".into(),
                "-c".into(),
                r#"
import pathlib,sys
s=pathlib.Path('Saves')
assert (s/'Slot.ess').read_bytes()==b'profile'
assert (s/'Slot.mwse').read_bytes()==b'cosave'
(s/'next.tmp').write_bytes(b'new save')
(s/'next.tmp').replace(s/'Slot.ess')
(s/'Slot.mwse').write_bytes(b'new cosave')
pathlib.Path(sys.argv[1]).write_text('ran')
"#
                .into(),
                marker.display().to_string(),
            ],
            env: vec![],
            base_bind: Some((data.clone(), data_stash.clone())),
            binds,
            cwd: Some(game.clone()),
            root_layers: if mode == "higher-root" {
                vec![root_mod]
            } else {
                vec![]
            },
            root_overwrite: Some(root_upper.clone()),
            root_base_bind: Some((game.clone(), root_stash.clone())),
        });
        if ["failure", "symlink", "duplicate", "escape"].contains(&mode) {
            assert!(result.is_err(), "{mode}");
            assert!(!marker.exists());
        } else {
            assert!(result.unwrap().success(), "{mode}");
            assert_eq!(fs::read(profile.join("Slot.ess")).unwrap(), b"new save");
            assert_eq!(fs::read(profile.join("Slot.mwse")).unwrap(), b"new cosave");
        }
        if mode == "missing" {
            assert!(
                !game.join("Saves").exists(),
                "must not create original Saves"
            );
        } else {
            assert_eq!(fs::read(game.join("Saves/Slot.ess")).unwrap(), b"original");
        }
        assert!(!root_upper.join("Saves/Slot.ess").exists());
        assert!(!overwrite.join("Saves/Slot.ess").exists());
        assert!(!fs::read_to_string("/proc/self/mountinfo")
            .unwrap()
            .lines()
            .any(
                |line| line.split_whitespace().nth(4) == Some(game.join("Saves").to_str().unwrap())
            ));
        detach(&data_stash);
        detach(&root_stash);
    }
}

fn composed_shader_sessions(root: &Path) {
    use std::sync::atomic::AtomicBool;
    let fixture = root.join("composed-shaders");
    let game = fixture.join("game");
    let data = game.join("Data");
    let high = fixture.join("high");
    let low = fixture.join("low");
    let overwrite = fixture.join("overwrite");
    let stash = fixture.join("stash");
    let sdp = |a: &[u8], b: &[u8]| {
        let mut bytes = Vec::from(100u32.to_le_bytes());
        bytes.extend(2u32.to_le_bytes());
        bytes.extend(((520 + a.len() + b.len()) as u32).to_le_bytes());
        for (name, data) in [("A.pso", a), ("B.vso", b)] {
            let mut field = [0; 256];
            field[..name.len()].copy_from_slice(name.as_bytes());
            bytes.extend(field);
            bytes.extend((data.len() as u32).to_le_bytes());
            bytes.extend(data);
        }
        bytes
    };
    let original = sdp(b"base", b"base b");
    let upper = sdp(b"overwrite", b"upper b");
    put(&data, "Shaders/shaderpackage001.sdp", &original);
    put(&overwrite, "Shaders/shaderpackage001.sdp", &upper);
    put(&low, "Shaders/OMOD/1/A.pso", b"low");
    put(&high, "shaders/omod/1/a.PSO", b"high");
    for (profile, selected, expected) in [
        ("profile-a", vec![high.clone(), low.clone()], "high"),
        ("profile-b", vec![low.clone()], "low"),
    ] {
        let parent = fixture.join(profile);
        fs::create_dir(&parent).unwrap();
        let mut layers = selected.clone();
        layers.push(data.clone());
        let session = eidos_gamefeatures::omod_shaders::prepare(
            layers,
            overwrite.clone(),
            &parent,
            &AtomicBool::new(false),
        )
        .unwrap()
        .unwrap();
        session.validate_sources(&AtomicBool::new(false)).unwrap();
        let generated = session.mappings()[0].0.clone();
        let result = launch(LaunchSpec {
            readonly_data_binds: session.mappings().to_vec(),
            root_readonly_overwrite: None,
            plugin_timestamps: None,
            layers: selected,
            overwrite: overwrite.clone(),
            mountpoint: data.clone(),
            command: vec![
                "python3".into(),
                "-c".into(),
                r#"
import pathlib,struct,sys,errno
p=pathlib.Path('Data/Shaders/shaderpackage001.sdp'); data=p.read_bytes()
magic,count,size=struct.unpack_from('<III',data); assert magic==100 and size==len(data)-12
pos=12; records={}
for _ in range(count):
    name=data[pos:pos+256].split(b'\0',1)[0]; n=struct.unpack_from('<I',data,pos+256)[0]
    pos+=260; records[name]=data[pos:pos+n]; pos+=n
assert pos==len(data) and records=={b'A.pso':sys.argv[1].encode(), b'B.vso':b'upper b'}
try: p.write_bytes(b'bad')
except OSError as e: assert e.errno==errno.EROFS
else: raise AssertionError('shader projection was writable')
"#
                .into(),
                expected.into(),
            ],
            env: vec![],
            base_bind: Some((data.clone(), stash.clone())),
            binds: vec![],
            cwd: Some(game.clone()),
            root_layers: vec![],
            root_overwrite: None,
            root_base_bind: None,
        })
        .unwrap();
        assert!(result.success());
        session.cleanup().unwrap();
        assert!(!generated.exists());
        assert_eq!(fs::read_dir(&parent).unwrap().count(), 0);
        assert_eq!(
            fs::read(data.join("Shaders/shaderpackage001.sdp")).unwrap(),
            original
        );
        assert_eq!(
            fs::read(overwrite.join("Shaders/shaderpackage001.sdp")).unwrap(),
            upper
        );
        detach(&stash);
    }
    let parent = fixture.join("cancelled");
    fs::create_dir(&parent).unwrap();
    assert!(eidos_gamefeatures::omod_shaders::prepare(
        vec![high, low, data],
        overwrite,
        &parent,
        &AtomicBool::new(true)
    )
    .is_err());
    assert_eq!(fs::read_dir(parent).unwrap().count(), 0);
}

fn userdata_mods_leave_install_mods_visible(root: &Path) {
    let fixture = root.join("userdata-mods");
    let game = fixture.join("game");
    let data = fixture.join("user data/Mods");
    let low = fixture.join("low");
    let high = fixture.join("high");
    let overwrite = fixture.join("overwrite");
    let stash = fixture.join("stash");
    put(&game, "Mods/0_TFP_Harmony/ModInfo.xml", b"shipped harmony");
    put(&data, "Existing/ModInfo.xml", b"existing user mod");
    put(&low, "Chosen/ModInfo.xml", b"low");
    put(&high, "Chosen/ModInfo.xml", b"high");
    let result = launch(LaunchSpec {
        readonly_data_binds: vec![],
        root_readonly_overwrite: None,
        plugin_timestamps: None,
        layers: vec![high, low],
        overwrite: overwrite.clone(),
        mountpoint: data.clone(),
        command: vec![
            "python3".into(),
            "-c".into(),
            r#"
import pathlib,sys
mods=pathlib.Path(sys.argv[1])
assert pathlib.Path('Mods/0_TFP_Harmony/ModInfo.xml').read_bytes()==b'shipped harmony'
assert (mods/'Existing/ModInfo.xml').read_bytes()==b'existing user mod'
assert (mods/'Chosen/ModInfo.xml').read_bytes()==b'high'
(mods/'output.txt').write_bytes(b'owned output')
"#
            .into(),
            data.display().to_string(),
        ],
        env: vec![],
        base_bind: Some((data.clone(), stash.clone())),
        binds: vec![],
        cwd: Some(game.clone()),
        root_layers: vec![],
        root_overwrite: None,
        root_base_bind: None,
    })
    .unwrap();
    assert!(result.success());
    assert_eq!(
        fs::read(overwrite.join("output.txt")).unwrap(),
        b"owned output"
    );
    assert!(!data.join("output.txt").exists());
    assert!(!data.join("Chosen").exists());
    assert_eq!(
        fs::read(game.join("Mods/0_TFP_Harmony/ModInfo.xml")).unwrap(),
        b"shipped harmony"
    );
    detach(&stash);
}

fn main() {
    let root = std::env::temp_dir().join(format!("eidos-root-overwrite-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    userdata_mods_leave_install_mods_visible(&root);
    composed_shader_sessions(&root);
    morrowind_root_saves(&root);
    mount_boundaries(&root);
    readonly_data_projection(&root);
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
        readonly_data_binds: vec![],
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
            readonly_data_binds: vec![],
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
