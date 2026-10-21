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
use eidos_transfer::{plan, Options, Phase, Transfer};

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
    write(&root.join("mods/[Rudolph] Dark Souls/meshes/a.nif"), "nif");
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
            out.push(p.strip_prefix(root).unwrap().to_string_lossy().into_owned());
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
    let mut seen: Vec<(Phase, u8)> = Vec::new();
    let report = transfer
        .pack(&inst, &p, &archive, &mut |ph, pc| seen.push((ph, pc)))
        .expect("pack");
    assert!(archive.is_file());
    // The one loss in this instance is the symlink, and it makes the backup
    // incomplete rather than quietly smaller.
    assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
    assert!(
        report.warnings[0].contains("mods/danger-link - a symbolic link"),
        "{:?}",
        report.warnings
    );
    let last = |phase| seen.iter().rev().find(|(ph, _)| *ph == phase).map(|s| s.1);
    assert_eq!(
        last(Phase::Archive),
        Some(100),
        "the bar must reach 100 when the archive is written"
    );
    // And it was read back before it got its name, with a bar of its own.
    assert_eq!(last(Phase::Check), Some(100), "{seen:?}");
    // The `.part` name is a working name, and it does not survive.
    assert!(!base.join("backup.eidos.part").exists());

    let dest = base.join("elsewhere");
    let out = transfer
        .unpack(&archive, &dest, &mut |_, _| {})
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
    assert!(
        meta.contains("modID=18780") && meta.contains("version=2.5"),
        "{meta}"
    );
    assert!(meta.contains("\r\n"), "CRLF must survive: {meta:?}");

    // The instance now points at where it is, not where it was.
    assert_eq!(out.relocated.values, 3, "{out:?}");
    let tools = fs::read_to_string(dest.join("tools.ini")).unwrap();
    assert!(
        tools.contains(&format!(
            "exe={}/mods/[Rudolph] Dark Souls/BodySlide.exe",
            dest.display()
        )),
        "{tools}"
    );
    assert!(
        tools.contains(&format!("arg0=-D:{}/overwrite", dest.display())),
        "{tools}"
    );
    assert!(tools.contains("exe=/opt/nowhere/xEdit.exe"), "{tools}");
    let mod_meta = fs::read_to_string(dest.join("mods/[Rudolph] Dark Souls/meta.ini")).unwrap();
    assert!(
        mod_meta.contains(&format!(
            "installationFile={}/downloads/R.7z",
            dest.display()
        )),
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
        .pack(&inst, &p, &archive, &mut |_, _| {})
        .unwrap();

    let dest = base.join("dest");
    let t = Transfer::new(opt).unwrap();
    t.unpack(&archive, &dest, &mut |_, _| {}).unwrap();
    // Unpacking over a folder with an instance in it would merge two setups into
    // one, which is never what somebody meant and is not undoable.
    let err = t.unpack(&archive, &dest, &mut |_, _| {}).unwrap_err();
    assert!(err.to_string().contains("not empty"), "{err}");

    let forced = Transfer::new(Options {
        force: true,
        ..Options::default()
    })
    .unwrap();
    forced.unpack(&archive, &dest, &mut |_, _| {}).unwrap();
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
        .pack(&inst, &p, &inside, &mut |_, _| {})
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
    let report = t.pack(&inst, &p, &archive, &mut |_, _| {}).expect("pack");
    assert!(archive.is_file(), "the archive is real and worth keeping");
    // Two: the fixture's symlink, and the vanished file.
    assert_eq!(report.warnings.len(), 2, "{:?}", report.warnings);
    assert!(
        report.warnings[1].contains("missing some files"),
        "{:?}",
        report.warnings
    );

    // And everything that did not vanish came through.
    let dest = base.join("dest");
    t.unpack(&archive, &dest, &mut |_, _| {}).unwrap();
    assert!(dest.join("mods/Weapons * Armour/textures/w.dds").is_file());
    assert!(!dest.join("mods/스크린아처메뉴/x.esp").exists());
    // The folder the vanished file lived in is still there: it held nothing else
    // to pack, so the walk listed the directory itself.
    assert!(dest.join("mods/스크린아처메뉴/empty-one").is_dir());
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn excluded_only_directories_are_packed_as_empty_placeholders() {
    if eidos_sevenzip::find_7z().is_none() {
        eprintln!("SKIPPED: no 7-Zip on this machine.");
        return;
    }
    let base = tmp("excluded-only");
    let src = base.join("source");
    build_instance(&src);
    let excluded = src.join("mods/ExcludedOnly");
    fs::create_dir_all(&excluded).unwrap();
    fs::write(excluded.join("private.unfinished"), b"excluded").unwrap();
    let outside = base.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("sentinel.txt"), b"personal").unwrap();
    std::os::unix::fs::symlink(&outside, excluded.join("link")).unwrap();
    let instance = Instance::portable(src);
    let options = Options::default();
    let plan = plan(&instance, &options);
    assert!(plan
        .left
        .iter()
        .any(|item| item.path.ends_with("private.unfinished")));
    let transfer = Transfer::new(options).unwrap();
    let archive = base.join("backup.eidos");
    transfer
        .pack(&instance, &plan, &archive, &mut |_, _| {})
        .unwrap();
    let restored = base.join("restored");
    transfer
        .unpack(&archive, &restored, &mut |_, _| {})
        .unwrap();
    let empty = restored.join("mods/ExcludedOnly");
    assert!(empty.is_dir());
    assert_eq!(fs::read_dir(empty).unwrap().count(), 0);
    assert_eq!(fs::read(outside.join("sentinel.txt")).unwrap(), b"personal");
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn forced_restore_refuses_symlink_ancestors_before_writing_any_payload() {
    if eidos_sevenzip::find_7z().is_none() {
        eprintln!("SKIPPED: no 7-Zip on this machine.");
        return;
    }
    let base = tmp("restore-symlink");
    let src = base.join("source");
    build_instance(&src);
    let instance = Instance::portable(src);
    let options = Options::default();
    let transfer = Transfer::new(options).unwrap();
    let archive = base.join("backup.eidos");
    transfer
        .pack(
            &instance,
            &plan(&instance, &options),
            &archive,
            &mut |_, _| {},
        )
        .unwrap();
    let forced = Transfer::new(Options {
        force: true,
        ..options
    })
    .unwrap();
    for linked in [
        "mods",
        "mods/Weapons * Armour/textures",
        "mods/Weapons * Armour/textures/w.dds",
    ] {
        let dest = base.join("restored");
        let outside = base.join("outside");
        fs::create_dir_all(&outside).unwrap();
        let sentinel = if linked.ends_with(".dds") {
            outside.join("sentinel")
        } else {
            outside.join("w.dds")
        };
        fs::write(&sentinel, b"personal").unwrap();
        let target = if linked.ends_with(".dds") {
            &sentinel
        } else {
            &outside
        };
        let link = dest.join(linked);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(target, &link).unwrap();
        let result = forced.unpack(&archive, &dest, &mut |_, _| {});
        assert!(result.is_err());
        assert_eq!(fs::read(&sentinel).unwrap(), b"personal", "{linked}");
        assert!(
            !dest.join("eidos-instance.ini").exists(),
            "must refuse before extraction"
        );
        fs::remove_dir_all(dest).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_newline_in_a_filename_cannot_add_outside_files_to_the_backup() {
    if eidos_sevenzip::find_7z().is_none() {
        eprintln!("SKIPPED: no 7-Zip on this machine.");
        return;
    }
    let base = tmp("list-injection");
    let src = base.join("source");
    build_instance(&src);
    let original = src.join("mods/A/x\"\n\"../outside/sentinel");
    write(&original, "innocent payload");
    let outside = base.join("outside/sentinel");
    write(&outside, "private outside bytes");
    let instance = Instance::portable(src);
    let options = Options::default();
    let transfer = Transfer::new(options).unwrap();
    let archive = base.join("backup.eidos");
    let plan = plan(&instance, &options);
    transfer
        .pack(&instance, &plan, &archive, &mut |_, _| {})
        .unwrap();
    let restored = base.join("restored");
    transfer
        .unpack(&archive, &restored, &mut |_, _| {})
        .unwrap();
    assert!(
        !restored.join("sentinel").exists(),
        "outside data entered the backup"
    );
    assert!(plan.left.iter().any(|item| item.path.contains('\n')));
    assert_eq!(fs::read(&original).unwrap(), b"innocent payload");
    assert_eq!(fs::read(&outside).unwrap(), b"private outside bytes");
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn level_zero_stores_and_still_comes_back() {
    let Some(seven) = eidos_sevenzip::find_7z() else {
        eprintln!("SKIPPED: no 7-Zip on this machine.");
        return;
    };
    let base = tmp("stored");
    let src = base.join("source");
    fs::create_dir_all(&src).unwrap();
    build_instance(&src);
    let inst = Instance::portable(src.clone());
    let opt = Options {
        level: 0,
        ..Options::default()
    };
    let archive = base.join("stored.eidos");
    let t = Transfer::new(opt).unwrap();
    t.pack(&inst, &plan(&inst, &opt), &archive, &mut |_, _| {})
        .unwrap();
    // The dialog promises that level 0 stores; an explicit LZMA2 method made
    // -mx0 compress with a 64 KiB dictionary instead.
    let listing = std::process::Command::new(seven)
        .args(["l", "-slt"])
        .arg(&archive)
        .output()
        .unwrap();
    let listing = String::from_utf8_lossy(&listing.stdout);
    let methods: Vec<&str> = listing
        .lines()
        .filter_map(|line| line.strip_prefix("Method = "))
        // Directories and empty files carry no method.
        .filter(|method| !method.is_empty())
        .collect();
    assert!(!methods.is_empty(), "{listing}");
    assert!(methods.iter().all(|m| *m == "Copy"), "{methods:?}");
    let dest = base.join("dest");
    t.unpack(&archive, &dest, &mut |_, _| {}).unwrap();
    assert_eq!(
        fs::read_to_string(dest.join("mods/스크린아처메뉴/x.esp")).unwrap(),
        "esp"
    );
    let _ = fs::remove_dir_all(&base);
}

/// A file of `len` bytes that does not compress, so damage to the middle of an
/// archive lands in its bytes rather than in a run LZMA2 shrank to nothing.
fn noise(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}

/// The instance the PAR2 tests pack: the usual one, plus 256 kB that does not
/// compress, so the archive is mostly payload and has room to be damaged.
fn par2_case(name: &str) -> (PathBuf, Instance) {
    let base = tmp(name);
    let src = base.join("source");
    fs::create_dir_all(&src).unwrap();
    build_instance(&src);
    write_bytes(
        &src.join("mods/Big/textures/big.dds"),
        &noise(256 * 1024, 7),
    );
    (base, Instance::portable(src))
}

fn write_bytes(path: &Path, body: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn flip_bytes(path: &Path, from: usize, count: usize) {
    let mut b = fs::read(path).unwrap();
    for x in &mut b[from..from + count] {
        *x ^= 0xA5;
    }
    fs::write(path, b).unwrap();
}

#[test]
fn recovery_data_repairs_a_flipped_byte_before_the_unpack() {
    if eidos_sevenzip::find_7z().is_none() || !eidos_transfer::par2_installed() {
        eprintln!("SKIPPED: needs both 7-Zip and par2 (package par2cmdline) on this machine.");
        return;
    }
    let (base, inst) = par2_case("par2-repair");
    let opt = Options::default();
    let t = Transfer::new(opt).unwrap();
    assert!(t.writes_recovery());
    let archive = base.join("b.eidos");
    let mut phases = Vec::new();
    let report = t
        .pack(&inst, &plan(&inst, &opt), &archive, &mut |ph, _| {
            phases.push(ph)
        })
        .unwrap();
    assert!(report.recovery_warning.is_none(), "{report:?}");
    assert!(base.join("b.eidos.par2").is_file());
    assert!(
        report.recovery.len() >= 2,
        "an index and a volume: {:?}",
        report.recovery
    );
    assert!(phases.contains(&Phase::Recovery), "{phases:?}");

    let len = fs::metadata(&archive).unwrap().len() as usize;
    flip_bytes(&archive, len / 2, 1);
    let dest = base.join("restored");
    let mut phases = Vec::new();
    let out = t
        .unpack(&archive, &dest, &mut |ph, _| phases.push(ph))
        .expect("repaired, then unpacked");
    assert!(
        phases.contains(&Phase::Verify) && phases.contains(&Phase::Repair),
        "{phases:?}"
    );
    assert!(
        out.warnings
            .iter()
            .any(|w| w.contains("repaired from its recovery data")),
        "{:?}",
        out.warnings
    );
    assert_eq!(
        fs::read(dest.join("mods/Big/textures/big.dds")).unwrap(),
        noise(256 * 1024, 7)
    );
    // par2 sets the damaged file aside rather than deleting it.
    assert!(base.join("b.eidos.1").is_file());
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn damage_beyond_the_recovery_data_stops_before_anything_is_extracted() {
    if eidos_sevenzip::find_7z().is_none() || !eidos_transfer::par2_installed() {
        eprintln!("SKIPPED: needs both 7-Zip and par2 (package par2cmdline) on this machine.");
        return;
    }
    let (base, inst) = par2_case("par2-beyond");
    let opt = Options::default();
    let t = Transfer::new(opt).unwrap();
    let archive = base.join("b.eidos");
    t.pack(&inst, &plan(&inst, &opt), &archive, &mut |_, _| {})
        .unwrap();
    // A fifth of the archive, from a quarter in: four times what 5% covers,
    // and clear of the header 7-Zip keeps at the end.
    let len = fs::metadata(&archive).unwrap().len() as usize;
    flip_bytes(&archive, len / 4, len / 5);
    let dest = base.join("restored");
    let err = t
        .unpack(&archive, &dest, &mut |_, _| {})
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("beyond what its recovery data can repair"),
        "{err}"
    );
    assert!(
        !dest.exists(),
        "nothing may be extracted from an unrepaired archive"
    );
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn recovery_files_without_par2_installed_warn_and_still_unpack() {
    if eidos_sevenzip::find_7z().is_none() {
        eprintln!("SKIPPED: no 7-Zip on this machine.");
        return;
    }
    let (base, inst) = par2_case("par2-missing");
    let opt = Options::default();
    // A machine without par2, whatever this one has.
    let t = Transfer::with_par2(opt, "/nonexistent/eidos-test/par2").unwrap();
    assert!(
        !t.writes_recovery(),
        "recovery is on, so par2 was not found"
    );
    let archive = base.join("b.eidos");
    // Recovery files of the archive this pack replaces: they describe another
    // file, and must not survive next to the new one.
    fs::write(base.join("b.eidos.par2"), b"stale").unwrap();
    fs::write(base.join("b.eidos.vol00+01.par2"), b"stale").unwrap();
    let report = t
        .pack(&inst, &plan(&inst, &opt), &archive, &mut |_, _| {})
        .unwrap();
    assert!(report.recovery.is_empty() && report.recovery_warning.is_none());
    assert!(!base.join("b.eidos.par2").exists());
    assert!(!base.join("b.eidos.vol00+01.par2").exists());

    // As if the set had been carried over from a machine that has par2.
    fs::write(base.join("b.eidos.par2"), b"from elsewhere").unwrap();
    let dest = base.join("restored");
    let out = t.unpack(&archive, &dest, &mut |_, _| {}).unwrap();
    assert!(
        out.warnings
            .iter()
            .any(|w| w.contains("par2 is not installed")),
        "{:?}",
        out.warnings
    );
    assert!(dest.join("mods/Big/textures/big.dds").is_file());
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn recovery_off_writes_no_par2_files() {
    if eidos_sevenzip::find_7z().is_none() || !eidos_transfer::par2_installed() {
        eprintln!("SKIPPED: needs both 7-Zip and par2 (package par2cmdline) on this machine.");
        return;
    }
    let (base, inst) = par2_case("par2-off");
    let opt = Options {
        recovery: false,
        ..Options::default()
    };
    let t = Transfer::new(opt).unwrap();
    assert!(!t.writes_recovery());
    let archive = base.join("b.eidos");
    let report = t
        .pack(&inst, &plan(&inst, &opt), &archive, &mut |_, _| {})
        .unwrap();
    assert!(report.recovery.is_empty());
    assert!(!base.join("b.eidos.par2").exists());
    let _ = fs::remove_dir_all(&base);
}
