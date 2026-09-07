//! The whole thing, once, against a real 7-Zip: pack an instance that carries
//! every shape that has ever broken a list file, unpack it somewhere else, and
//! insist the result is the instance again.
//!
//! The unit tests cover the pieces; this covers the seams between them - the two
//! passes into one archive, the empty directories that only exist because the
//! list file names them, the scrubbed copy that has to replace the original
//! rather than join it, and the relocation that only makes sense once the tree
//! is somewhere new.
//!
//! Skipped, loudly, where 7-Zip is not installed. A test that silently passes
//! because the thing it tests could not run is a test that lies.

use std::fs;
use std::path::{Path, PathBuf};

use eidos_instance::Instance;
use eidos_transfer::{plan, Options, Transfer};

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "eidos-round-trip-{}-{name}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn write(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

/// An instance with, deliberately, one of everything awkward.
fn build_instance(root: &Path) {
    write(
        &root.join("eidos-instance.ini"),
        "[eidos]\nschema_version=1\ngame_id=skyrimse\nkind=portable\nselected_profile=Default\n",
    );
    // Names that have broken a list file before: brackets, a 7-Zip wildcard,
    // non-ASCII in two scripts, spaces.
    write(
        &root.join("mods/[Rudolph] Dark Souls/meshes/a.nif"),
        "nif",
    );
    write(&root.join("mods/Weapons * Armour/textures/w.dds"), "dds");
    write(&root.join("mods/스크린아처메뉴/x.esp"), "esp");
    // A leading space in a mod folder name. Windows-sourced archives carry
    // them, and 7-Zip trims every list-file line, so an unquoted list drops
    // this whole mod with a warning that reads like nothing.
    write(&root.join("mods/ Leading Space/x.dds"), "dds");
    // Empty directories: invisible to a list file of files only, and a mod that
    // comes back without one is a mod that has changed.
    fs::create_dir_all(root.join("mods/[Rudolph] Dark Souls/textures/Новая папка")).unwrap();
    fs::create_dir_all(root.join("mods/스크린아처메뉴/empty-one")).unwrap();
    write(&root.join("profiles/Default/modlist.txt"), "+A\n");
    write(&root.join("overwrite/SKSE/Plugins/settings.json"), "{}");

    // A path INTO the instance, which only the relocation pass can repair.
    write(
        &root.join("mods/[Rudolph] Dark Souls/meta.ini"),
        &format!(
            "[General]\r\nmodid=1\r\ninstallationFile={}/downloads/R.7z\r\n",
            root.display()
        ),
    );
    write(
        &root.join("tools.ini"),
        &format!(
            "[Tool/BodySlide]\nexe={}/mods/[Rudolph] Dark Souls/BodySlide.exe\n\
             arg0=-D:{}/overwrite\n\n[Tool/Elsewhere]\nexe=/opt/nowhere/xEdit.exe\n",
            root.display(),
            root.display()
        ),
    );

    // A signed download URL, which must not travel.
    write(
        &root.join("downloads/Mod.7z.meta"),
        "[General]\r\nmodID=18780\r\nurl=\"https://cdn/x?user_id=42&h=SIG\"\r\nversion=2.5\r\n",
    );
    write(&root.join("downloads/Mod.7z"), "ARCHIVE");

    // Everything that must be left behind.
    write(&root.join(".eidos.lock"), "a running game session (pid 1)");
    write(&root.join("prereqs.done"), "d3dx9_43");
    write(&root.join("prereqs.log"), "log");
    write(&root.join("logs/session.log"), "l");
    write(&root.join("loot/masterlist.yaml"), "m");
    write(&root.join("downloads/Other.7z.unfinished"), "partial");
    write(&root.join("profiles/Default/plugins.eidos-tmp"), "tmp");
    fs::create_dir_all(root.join(".base")).unwrap();
    fs::create_dir_all(root.join(".base-root")).unwrap();
    std::os::unix::fs::symlink("/etc", root.join("mods/danger-link")).unwrap();
}

/// Every path under `root`, instance-relative and sorted.
fn tree(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            out.push(
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                stack.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn an_instance_survives_being_packed_and_put_back_somewhere_else() {
    if eidos_sevenzip::find_7z().is_none() {
        eprintln!("SKIPPED: no 7-Zip on this machine, so there is nothing to test against.");
        return;
    }
    let base = tmp("case");
    let src = base.join("source");
    fs::create_dir_all(&src).unwrap();
    build_instance(&src);
    let inst = Instance::portable(src.clone());

    let opt = Options::default();
    let p = plan(&inst, &opt);
    assert_eq!(p.files, 11, "{:?}", p.entries);
    assert_eq!(p.empty_dirs, 2);
    assert_eq!(p.scrub, vec!["downloads/Mod.7z.meta".to_string()]);
    assert_eq!(p.wildcards, vec!["mods/Weapons * Armour/textures/w.dds"]);
    assert_eq!(
        p.tools_outside
            .iter()
            .map(|t| t.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Elsewhere"]
    );

    let archive = base.join("backup.eidos");
    let transfer = Transfer::new(opt).expect("7-Zip was found a moment ago");
    let mut seen: Vec<u8> = Vec::new();
    let report = transfer
        .pack(&inst, &p, &archive, &mut |pc| seen.push(pc))
        .expect("pack");
    assert!(archive.is_file());
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(
        seen.last().copied(),
        Some(100),
        "the bar must reach 100 when the archive is written"
    );
    // The `.part` name is a working name, and it does not survive.
    assert!(!base.join("backup.eidos.part").exists());

    let dest = base.join("elsewhere");
    let out = transfer
        .unpack(&archive, &dest, &mut |_| {})
        .expect("unpack");

    // The tree comes back, minus exactly what was left out, plus the manifest.
    let mut expected = tree(&src);
    expected.retain(|p| {
        !p.starts_with(".base")
            && !p.starts_with("logs")
            && !p.starts_with("loot")
            && p != ".eidos.lock"
            && p != "prereqs.done"
            && p != "prereqs.log"
            && p != "mods/danger-link"
            && !p.ends_with(".unfinished")
            && !p.ends_with(".eidos-tmp")
    });
    expected.push("eidos-backup.ini".to_string());
    expected.sort();
    assert_eq!(tree(&dest), expected);

    // Bytes, not just names.
    assert_eq!(
        fs::read_to_string(dest.join("mods/스크린아처메뉴/x.esp")).unwrap(),
        "esp"
    );
    assert_eq!(
        fs::read_to_string(dest.join("mods/Weapons * Armour/textures/w.dds")).unwrap(),
        "dds"
    );
    assert_eq!(
        fs::read_to_string(dest.join("mods/ Leading Space/x.dds")).unwrap(),
        "dds",
        "a name 7-Zip would have trimmed"
    );

    // The signed URL did not travel; the rest of the record did.
    let meta = fs::read_to_string(dest.join("downloads/Mod.7z.meta")).unwrap();
    assert!(!meta.contains("user_id"), "{meta}");
    assert!(meta.contains("modID=18780") && meta.contains("version=2.5"), "{meta}");
    assert!(meta.contains("\r\n"), "CRLF must survive: {meta:?}");

    // The instance now points at where it is, not where it was.
    assert_eq!(out.relocated.values, 3);
    let tools = fs::read_to_string(dest.join("tools.ini")).unwrap();
    assert!(
        tools.contains(&format!("exe={}/mods/[Rudolph] Dark Souls/BodySlide.exe", dest.display())),
        "{tools}"
    );
    assert!(
        tools.contains(&format!("arg0=-D:{}/overwrite", dest.display())),
        "{tools}"
    );
    assert!(tools.contains("exe=/opt/nowhere/xEdit.exe"), "{tools}");
    let mod_meta = fs::read_to_string(dest.join("mods/[Rudolph] Dark Souls/meta.ini")).unwrap();
    assert!(
        mod_meta.contains(&format!("installationFile={}/downloads/R.7z", dest.display())),
        "{mod_meta}"
    );

    // And it says which tool the new machine is missing.
    assert_eq!(
        out.missing_tools
            .iter()
            .map(|t| t.exe.as_str())
            .collect::<Vec<_>>(),
        vec!["/opt/nowhere/xEdit.exe"]
    );

    // The manifest inside the archive answers "what is not in here".
    assert_eq!(out.manifest.game_id, "skyrimse");
    assert_eq!(out.manifest.source_root, src.to_string_lossy());
    assert!(out.manifest.portable);
    assert!(out
        .manifest
        .left_out
        .iter()
        .any(|(p, w)| p == "mods/danger-link" && w.contains("symbolic")));

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn a_second_unpack_into_the_same_folder_is_refused_until_it_is_asked_for() {
    if eidos_sevenzip::find_7z().is_none() {
        eprintln!("SKIPPED: no 7-Zip on this machine.");
        return;
    }
    let base = tmp("twice");
    let src = base.join("source");
    fs::create_dir_all(&src).unwrap();
    build_instance(&src);
    let inst = Instance::portable(src.clone());
    let archive = base.join("b.eidos");
    let opt = Options::default();
    let p = plan(&inst, &opt);
    Transfer::new(opt)
        .unwrap()
        .pack(&inst, &p, &archive, &mut |_| {})
        .unwrap();

    let dest = base.join("dest");
    let t = Transfer::new(opt).unwrap();
    t.unpack(&archive, &dest, &mut |_| {}).unwrap();
    // Unpacking over a folder with an instance in it would merge two setups into
    // one, which is never what somebody meant and is not undoable.
    let err = t.unpack(&archive, &dest, &mut |_| {}).unwrap_err();
    assert!(err.to_string().contains("not empty"), "{err}");

    let forced = Transfer::new(Options {
        force: true,
        ..Options::default()
    })
    .unwrap();
    forced.unpack(&archive, &dest, &mut |_| {}).unwrap();
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn packing_an_instance_into_itself_is_refused_before_anything_is_written() {
    if eidos_sevenzip::find_7z().is_none() {
        eprintln!("SKIPPED: no 7-Zip on this machine.");
        return;
    }
    let base = tmp("selfpack");
    let src = base.join("source");
    fs::create_dir_all(&src).unwrap();
    build_instance(&src);
    let inst = Instance::portable(src.clone());
    let opt = Options::default();
    let p = plan(&inst, &opt);
    // The archive would grow while 7-Zip read the folder it is being written to.
    let inside = src.join("backup.eidos");
    let err = Transfer::new(opt)
        .unwrap()
        .pack(&inst, &p, &inside, &mut |_| {})
        .unwrap_err();
    assert!(err.to_string().contains("inside the instance"), "{err}");
    assert!(!inside.exists());
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn a_file_that_vanishes_under_the_pack_costs_that_file_and_not_the_backup() {
    if eidos_sevenzip::find_7z().is_none() {
        eprintln!("SKIPPED: no 7-Zip on this machine.");
        return;
    }
    // The real race: an instance is read in a second and packed over twenty
    // minutes, so something CAN disappear in between. 7-Zip exits 1, having
    // written a complete archive of everything else. Failing on that threw away
    // the whole backup for one file.
    let base = tmp("vanish");
    let src = base.join("source");
    fs::create_dir_all(&src).unwrap();
    build_instance(&src);
    let inst = Instance::portable(src.clone());
    let opt = Options::default();
    let p = plan(&inst, &opt);
    fs::remove_file(src.join("mods/스크린아처메뉴/x.esp")).unwrap();

    let archive = base.join("b.eidos");
    let t = Transfer::new(opt).unwrap();
    let report = t.pack(&inst, &p, &archive, &mut |_| {}).expect("pack");
    assert!(archive.is_file(), "the archive is real and worth keeping");
    assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
    assert!(
        report.warnings[0].contains("missing some files"),
        "{:?}",
        report.warnings
    );

    // And everything that did not vanish came through.
    let dest = base.join("dest");
    t.unpack(&archive, &dest, &mut |_| {}).unwrap();
    assert!(dest.join("mods/Weapons * Armour/textures/w.dds").is_file());
    assert!(!dest.join("mods/스크린아처메뉴/x.esp").exists());
    // The folder the vanished file lived in is still there: it held nothing else
    // to pack, so the walk listed the directory itself.
    assert!(dest.join("mods/스크린아처메뉴/empty-one").is_dir());
    let _ = fs::remove_dir_all(&base);
}
