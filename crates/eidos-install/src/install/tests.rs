use std::sync::atomic::Ordering;
use std::time::SystemTime;

use eidos_instance::ModMeta;

use super::*;
use eidos_fomod::FileItem;

/// The Gamebryo vocabulary, which is what these cases are written against.
fn rules() -> LayoutRules {
    LayoutRules::default()
}

/// A unique temp directory removed on drop (the crate has no `tempfile` dep).
struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!(
            "eidos-install-test-{}-{}-{}",
            tag,
            std::process::id(),
            n
        ));
        fs::create_dir_all(&d).unwrap();
        TempDir(d)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn file_item(source: &str, destination: &str) -> FileItem {
    FileItem {
        source: source.to_string(),
        destination: destination.to_string(),
        priority: 0,
        is_folder: false,
        always_install: false,
        install_if_usable: false,
        sequence: 0,
    }
}

// ---- case-collision normalisation -------------------------------------

/// Write `content` to `root/rel`, creating parent dirs.
fn write_at(root: &Path, rel: &str, content: &[u8]) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

/// Force a file's mtime so the "oldest wins" rule is testable deterministically
/// (writing content stamps mtime to ~now, so set it afterwards). std-only.
fn set_mtime(root: &Path, rel: &str, secs: u64) {
    let f = fs::OpenOptions::new()
        .write(true)
        .open(root.join(rel))
        .unwrap();
    let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs);
    f.set_times(std::fs::FileTimes::new().set_modified(t))
        .unwrap();
}

/// Sorted recursive listing of `root` (dirs end `/`, symlinks end `@`), for
/// structural assertions and the idempotency snapshot.
fn rel_paths(root: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let mut entries: Vec<_> = fs::read_dir(dir).unwrap().flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
            let ft = fs::symlink_metadata(&p).unwrap().file_type();
            if ft.is_symlink() {
                out.push(format!("{rel}@"));
            } else if ft.is_dir() {
                out.push(format!("{rel}/"));
                walk(base, &p, out);
            } else {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn replace_with_bad_archive_keeps_the_old_mod() {
    // The Replace wipe must come AFTER extraction succeeds: a garbage archive
    // (or a missing 7z) must leave the existing mod untouched.
    let t = TempDir::new("replsafe");
    let mods = t.path().join("mods");
    write_at(&mods, "MyMod/textures/a.dds", b"precious");
    write_at(&mods, "MyMod/meta.ini", b"[General]\nendorsed=1\n");
    let bogus = t.path().join("not-an-archive.7z");
    fs::write(&bogus, b"this is not a 7z file").unwrap();

    let r = install_archive_with_policy(
        &bogus,
        &mods,
        "MyMod",
        "Skyrim Special Edition",
        OverwritePolicy::Replace,
        &eidos_fomod::Context::default(),
    );
    assert!(r.is_err(), "a garbage archive must not install");
    // Whatever the failure (No7z on a bare system, extraction failure with 7z
    // present), the old mod must still be fully intact.
    assert_eq!(
        fs::read(mods.join("MyMod/textures/a.dds")).unwrap(),
        b"precious"
    );
    assert_eq!(
        fs::read(mods.join("MyMod/meta.ini")).unwrap(),
        b"[General]\nendorsed=1\n"
    );
}

#[test]
fn meta_ini_records_the_game_the_way_mo2_spells_it() {
    // MO2 writes `gameName=SkyrimSE`, its short name. Eidos was writing its
    // own lowercase id, so MO2 opening a mod Eidos installed did not
    // recognise the game. Nothing reads this back for behaviour; it is purely
    // what the other manager sees.
    let t = TempDir::new("gamename");
    let archive = t.path().join("MyMod-1234.7z");
    fs::write(&archive, b"x").unwrap();
    let dest = t.path().join("MyMod");
    fs::create_dir_all(&dest).unwrap();

    write_meta(&archive, &dest, "skyrimse", None).unwrap();
    assert_eq!(
        ModMeta::read(&dest.join("meta.ini")).game_name().as_deref(),
        Some("SkyrimSE")
    );

    // A game outside the catalog falls back to the id it was given rather
    // than inventing a spelling.
    write_meta(&archive, &dest, "not-a-game", None).unwrap();
    assert_eq!(
        ModMeta::read(&dest.join("meta.ini")).game_name().as_deref(),
        Some("not-a-game")
    );

    // And a sidecar always wins: it is what the download itself declared.
    fs::write(
        t.path().join("MyMod-1234.7z.meta"),
        b"[General]\ngameName=Fallout4\n",
    )
    .unwrap();
    write_meta(&archive, &dest, "skyrimse", None).unwrap();
    assert_eq!(
        ModMeta::read(&dest.join("meta.ini")).game_name().as_deref(),
        Some("Fallout4")
    );
}

#[test]
fn case_collision_file_same_dir() {
    let t = TempDir::new("ccfsd");
    write_at(t.path(), "meshes/armor.nif", b"a");
    write_at(t.path(), "Meshes/armor.nif", b"b");
    normalize_case_collisions(t.path()).unwrap();
    // One canonical lower-case dir, one file.
    assert_eq!(
        rel_paths(t.path()),
        vec!["meshes/".to_string(), "meshes/armor.nif".to_string()]
    );
}

#[test]
fn case_collision_file_keeps_oldest_mtime() {
    let t = TempDir::new("ccmtime");
    write_at(t.path(), "meshes/armor.nif", b"OLD");
    write_at(t.path(), "Meshes/armor.nif", b"NEW");
    set_mtime(t.path(), "meshes/armor.nif", 100);
    set_mtime(t.path(), "Meshes/armor.nif", 200);
    normalize_case_collisions(t.path()).unwrap();
    assert_eq!(fs::read(t.path().join("meshes/armor.nif")).unwrap(), b"OLD");
}

#[test]
fn case_collision_dir_merge() {
    let t = TempDir::new("ccdm");
    write_at(t.path(), "meshes/a.nif", b"a");
    write_at(t.path(), "Meshes/b.nif", b"b");
    normalize_case_collisions(t.path()).unwrap();
    assert_eq!(
        rel_paths(t.path()),
        vec![
            "meshes/".to_string(),
            "meshes/a.nif".to_string(),
            "meshes/b.nif".to_string()
        ]
    );
}

#[test]
fn case_collision_nested_dir() {
    let t = TempDir::new("ccnd");
    write_at(t.path(), "data/meshes/a.nif", b"a");
    write_at(t.path(), "Data/Meshes/b.nif", b"b");
    normalize_case_collisions(t.path()).unwrap();
    assert_eq!(
        rel_paths(t.path()),
        vec![
            "data/".to_string(),
            "data/meshes/".to_string(),
            "data/meshes/a.nif".to_string(),
            "data/meshes/b.nif".to_string(),
        ]
    );
}

#[test]
fn case_collision_file_vs_dir() {
    let t = TempDir::new("ccfvd");
    fs::write(t.path().join("textures"), b"file").unwrap();
    write_at(t.path(), "Textures/inside.txt", b"in");
    normalize_case_collisions(t.path()).unwrap();
    assert!(
        t.path().join("textures").is_file(),
        "file wins the canonical name"
    );
    assert!(
        t.path().join("textures_dir/inside.txt").is_file(),
        "dir is moved aside, contents preserved"
    );
}

#[test]
fn case_collision_file_vs_dir_aside_name_taken() {
    // Regression: `textures` (file) + `Textures/` (dir) move the dir aside to
    // `textures_dir`, but a pre-existing FILE already holds that name. The merge
    // must NOT read_dir a file (ENOTDIR aborts the whole install); both the
    // pre-existing file and the rescued dir contents must survive.
    let t = TempDir::new("ccfvd_taken");
    fs::write(t.path().join("textures"), b"file").unwrap();
    write_at(t.path(), "Textures/inside.txt", b"in");
    fs::write(t.path().join("textures_dir"), b"unrelated").unwrap();
    normalize_case_collisions(t.path()).unwrap();
    assert!(
        t.path().join("textures").is_file(),
        "file wins the canonical name"
    );
    assert_eq!(
        fs::read(t.path().join("textures_dir")).unwrap(),
        b"unrelated",
        "pre-existing file at the aside name is untouched"
    );
    assert!(
        t.path().join("textures_dir_1/inside.txt").is_file(),
        "moved-aside dir lands at the first free suffixed name, nothing lost"
    );
}

#[test]
fn case_collision_symlink_preserved() {
    use std::os::unix::fs::symlink;
    let t = TempDir::new("ccsym");
    symlink("some_target", t.path().join("current")).unwrap();
    write_at(t.path(), "Current/f.txt", b"x");
    normalize_case_collisions(t.path()).unwrap();
    // The symlink (opaque) wins the canonical name and is NOT dereferenced.
    let meta = fs::symlink_metadata(t.path().join("current")).unwrap();
    assert!(meta.file_type().is_symlink(), "symlink preserved as-is");
    assert_eq!(
        fs::read_link(t.path().join("current")).unwrap().to_str(),
        Some("some_target")
    );
    assert!(
        t.path().join("current_dir/f.txt").is_file(),
        "colliding dir moved aside"
    );
}

#[test]
fn case_collision_non_utf8_skipped() {
    use std::os::unix::ffi::OsStrExt;
    let t = TempDir::new("ccnonutf8");
    // A genuine collision that MUST still resolve.
    write_at(t.path(), "Foo/a.txt", b"a");
    write_at(t.path(), "foo/b.txt", b"b");
    // A non-UTF8 sibling that must be left untouched (no panic).
    let bad = t
        .path()
        .join(std::ffi::OsStr::from_bytes(b"weird\xff\xfename"));
    fs::write(&bad, b"keep").unwrap();
    normalize_case_collisions(t.path()).unwrap();
    assert!(t.path().join("foo/a.txt").is_file() && t.path().join("foo/b.txt").is_file());
    assert!(bad.exists(), "non-UTF8 entry is skipped, not lost");
}

#[test]
fn case_collision_idempotent() {
    let t = TempDir::new("ccidem");
    write_at(t.path(), "meshes/a.nif", b"a");
    write_at(t.path(), "Meshes/b.nif", b"b");
    write_at(t.path(), "data/Meshes/c.nif", b"c");
    write_at(t.path(), "Data/meshes/d.nif", b"d");
    normalize_case_collisions(t.path()).unwrap();
    let after_first = rel_paths(t.path());
    normalize_case_collisions(t.path()).unwrap();
    assert_eq!(after_first, rel_paths(t.path()), "second pass is a no-op");
}

#[test]
fn case_collision_empty_dir_preserved() {
    let t = TempDir::new("ccempty");
    write_at(t.path(), "Meshes/file.nif", b"x");
    fs::create_dir_all(t.path().join("meshes/empty_subdir")).unwrap();
    normalize_case_collisions(t.path()).unwrap();
    assert!(t.path().join("meshes/file.nif").is_file());
    assert!(
        t.path().join("meshes/empty_subdir").is_dir(),
        "empty dir survives the merge"
    );
}

#[test]
fn case_collision_deep_three_way() {
    let t = TempDir::new("ccdeep");
    write_at(t.path(), "A/B/C/file.txt", b"1");
    write_at(t.path(), "a/b/c/file.txt", b"2");
    write_at(t.path(), "A/b/C/file.txt", b"3");
    normalize_case_collisions(t.path()).unwrap();
    // Everything collapses to a single all-lower-case chain with one file.
    assert_eq!(
        rel_paths(t.path()),
        vec![
            "a/".to_string(),
            "a/b/".to_string(),
            "a/b/c/".to_string(),
            "a/b/c/file.txt".to_string(),
        ]
    );
}

#[test]
fn case_collision_no_change_without_collision() {
    let t = TempDir::new("ccnone");
    // Mixed casing but NO sibling collides: every name must keep its casing.
    write_at(t.path(), "Meshes/Armor.nif", b"a");
    write_at(t.path(), "Meshes/Sub/Armor.nif", b"b");
    let before = rel_paths(t.path());
    normalize_case_collisions(t.path()).unwrap();
    assert_eq!(
        before,
        rel_paths(t.path()),
        "non-colliding names are never touched"
    );
}

// MO2 parity (copyLeaf): a file with an empty destination lands at
// <dest>/<source-filename>, not on the mod root dir (which fails EISDIR).
#[test]
fn empty_destination_uses_source_filename() {
    let root = TempDir::new("root");
    let dest = TempDir::new("dest");
    fs::write(root.path().join("real.esp"), b"data").unwrap();

    let plan = vec![file_item("real.esp", "")];
    apply_plan(root.path(), &plan, dest.path()).expect("apply_plan");

    let landed = dest.path().join("real.esp");
    assert!(landed.is_file(), "file should land at <dest>/real.esp");
    assert_eq!(fs::read(&landed).unwrap(), b"data");
}

// A trailing-slash destination means "into this directory": append the file name.
#[test]
fn trailing_slash_destination_uses_source_filename() {
    let root = TempDir::new("root");
    let dest = TempDir::new("dest");
    fs::create_dir_all(root.path().join("Core")).unwrap();
    fs::write(root.path().join("Core").join("real.esp"), b"data").unwrap();

    let plan = vec![file_item("Core/real.esp", "subdir/")];
    apply_plan(root.path(), &plan, dest.path()).expect("apply_plan");

    let landed = dest.path().join("subdir").join("real.esp");
    assert!(
        landed.is_file(),
        "file should land at <dest>/subdir/real.esp"
    );
    assert_eq!(fs::read(&landed).unwrap(), b"data");
}

// A normal explicit destination is untouched (guards the new branch is gated).
#[test]
fn explicit_destination_is_preserved() {
    let root = TempDir::new("root");
    let dest = TempDir::new("dest");
    fs::write(root.path().join("real.esp"), b"data").unwrap();

    let plan = vec![file_item("real.esp", "renamed.esp")];
    apply_plan(root.path(), &plan, dest.path()).expect("apply_plan");

    assert!(dest.path().join("renamed.esp").is_file());
    assert!(!dest.path().join("real.esp").exists());
}

#[test]
fn apply_plan_refuses_path_traversal_destination() {
    // A malicious FOMOD destination that escapes the mod folder must be refused,
    // not written outside it.
    let root = TempDir::new("root");
    let dest = TempDir::new("dest");
    fs::write(root.path().join("evil.esp"), b"data").unwrap();
    // A sentinel that must NOT be overwritten.
    let outside = dest.path().parent().unwrap().join("eidos-traversal-victim");
    let _ = fs::remove_file(&outside);

    let plan = vec![file_item("evil.esp", "../eidos-traversal-victim")];
    let r = apply_plan(root.path(), &plan, dest.path());
    assert!(
        matches!(r, Err(InstallError::Fomod(_))),
        "traversal must be refused"
    );
    assert!(
        !outside.exists(),
        "nothing must be written outside the mod folder"
    );

    // An absolute destination is refused too.
    let plan2 = vec![file_item("evil.esp", "/tmp/eidos-traversal-abs")];
    assert!(matches!(
        apply_plan(root.path(), &plan2, dest.path()),
        Err(InstallError::Fomod(_))
    ));
}

#[test]
fn resolve_ci_refuses_dotdot_source() {
    // A `..` in an attacker-controlled FOMOD source must not read outside root.
    let root = TempDir::new("root");
    fs::create_dir_all(root.path().join("sub")).unwrap();
    assert!(resolve_ci(root.path(), "sub").is_some());
    assert!(resolve_ci(root.path(), "../root").is_none());
    assert!(resolve_ci(root.path(), "sub/../../etc").is_none());
}

#[test]
fn escapes_root_detects_traversal() {
    assert!(escapes_root("../x"));
    assert!(escapes_root("a/../../b"));
    assert!(escapes_root("/abs/path"));
    assert!(!escapes_root("a/b/c.esp"));
    assert!(!escapes_root("textures/foo.dds"));
}

#[test]
fn fail_policy_errors_when_dest_exists() {
    let mods = TempDir::new("mods");
    fs::create_dir_all(mods.path().join("ExistingMod")).unwrap();
    fs::write(mods.path().join("ExistingMod/a.esp"), b"x").unwrap();
    // The archive is never read - Fail returns before extraction (no 7-Zip needed).
    let archive = mods.path().join("whatever.7z");
    let r = install_archive_with_policy(
        &archive,
        mods.path(),
        "ExistingMod",
        "skyrimse",
        OverwritePolicy::Fail,
        &eidos_fomod::Context::default(),
    );
    assert!(matches!(r, Err(InstallError::Exists(_))));
}

#[test]
fn fomod_context_marks_present_plugins_active() {
    let game = TempDir::new("game");
    let modd = TempDir::new("mod");
    fs::write(game.path().join("Skyrim.esm"), b"").unwrap();
    fs::write(modd.path().join("SkyUI.esp"), b"").unwrap();
    let ctx = fomod_context(game.path(), &[modd.path().to_path_buf()], &[]);
    // A present plugin reads Active (so fileDependency state="Active" holds); an
    // absent one is left out, which eval treats as Missing.
    assert_eq!(
        ctx.file_states.get("skyrim.esm").map(String::as_str),
        Some("Active")
    );
    assert_eq!(
        ctx.file_states.get("skyui.esp").map(String::as_str),
        Some("Active")
    );
    assert_eq!(ctx.file_states.get("absent.esp"), None);
}

#[test]
fn fomod_context_distinguishes_inactive_from_missing() {
    let game = TempDir::new("game");
    let en = TempDir::new("enabled");
    let dis = TempDir::new("disabled");
    fs::write(en.path().join("Active.esp"), b"").unwrap();
    fs::write(dis.path().join("Disabled.esp"), b"").unwrap();
    // A plugin shipped by BOTH an enabled and a disabled mod must read Active.
    fs::write(en.path().join("Shared.esp"), b"").unwrap();
    fs::write(dis.path().join("Shared.esp"), b"").unwrap();
    let ctx = fomod_context(
        game.path(),
        &[en.path().to_path_buf()],
        &[dis.path().to_path_buf()],
    );
    assert_eq!(
        ctx.file_states.get("active.esp").map(String::as_str),
        Some("Active")
    );
    assert_eq!(
        ctx.file_states.get("disabled.esp").map(String::as_str),
        Some("Inactive")
    );
    assert_eq!(
        ctx.file_states.get("shared.esp").map(String::as_str),
        Some("Active")
    );
    assert_eq!(ctx.file_states.get("absent.esp"), None); // -> Missing
}

#[test]
fn replacement_preserves_user_metadata_and_updates_archive_facts() {
    let t = TempDir::new("preserved-meta");
    let mods = t.path().join("mods");
    let dest = mods.join("Test");
    write_at(&dest, "meta.ini", b"[General]\r\nmodid=7\r\nversion=1\r\ncategory=\"42,\"\r\neidosCollectionOwner=reservation-token\r\n\r\n[Other]\r\nraw=keep=verbatim\r\n");
    let meta_path = dest.join("meta.ini");
    let mut old = ModMeta::read(&meta_path);
    old.set_endorsed(true);
    old.set_tracked(true);
    old.set_notes("needs the AE patch");
    old.set_color(Some([0x2e, 0x5e, 0x8b]));
    old.set_url("https://github.com/me/mod");
    old.set_ignore_update(true);
    old.set_author("Arthmoor");
    old.set_uploader("Arthmoor", "https://www.nexusmods.com/users/1234");
    old.write(&meta_path).unwrap();
    let archive = t.path().join("archive.7z");
    fs::write(
        t.path().join("archive.7z.meta"),
        "[General]\nmodID=7\nfileID=123\nversion=2\n",
    )
    .unwrap();
    let src = t.path().join("source");
    write_at(&src, "test.esp", b"updated");
    install_extracted(
        &extracted(&src),
        &archive,
        &mods,
        "Test",
        "skyrimse",
        OverwritePolicy::Replace,
        &Default::default(),
    )
    .unwrap();
    let back = ModMeta::read(&meta_path);
    assert!(back.endorsed() && back.tracked() && back.ignore_update());
    assert_eq!(back.category().as_deref(), Some("42,"));
    assert_eq!(back.notes().as_deref(), Some("needs the AE patch"));
    assert_eq!(back.color(), Some([0x2e, 0x5e, 0x8b]));
    assert_eq!(back.url().as_deref(), Some("https://github.com/me/mod"));
    assert_eq!(back.author().as_deref(), Some("Arthmoor"));
    assert_eq!(back.uploader().as_deref(), Some("Arthmoor"));
    assert_eq!(
        back.uploader_url().as_deref(),
        Some("https://www.nexusmods.com/users/1234")
    );
    assert_eq!(back.version().as_deref(), Some("2"));
    let raw = fs::read_to_string(meta_path).unwrap();
    assert!(!raw.contains("eidosCollectionOwner=reservation-token\r\n"));
    assert!(raw.contains("[Other]\r\nraw=keep=verbatim\r\n"));
}

#[test]
fn a_freshly_installed_mod_already_knows_who_made_it() {
    // The hole this closes: author and uploader were only ever learned by an
    // update check, so a mod just downloaded and installed had no "Visit X's
    // profile" entry until the user happened to run one - missing on exactly
    // the mods somebody is most likely to want it on.
    let dir = TempDir::new("seed");
    let archive = dir.path().join("Mod-1234-1-0.7z");
    fs::write(&archive, b"x").unwrap();
    fs::write(
        PathBuf::from(format!("{}.meta", archive.display())),
        "[General]\nmodID=1234\nversion=1.0\nauthor=\"Arthmoor\"\n\
         uploader=\"Arthmoor\"\nuploaderUrl=\"https://www.nexusmods.com/users/1234\"\n",
    )
    .unwrap();
    let dest = TempDir::new("dest");
    write_meta(&archive, dest.path(), "skyrimse", None).unwrap();

    let m = ModMeta::read(&dest.path().join("meta.ini"));
    assert_eq!(m.author().as_deref(), Some("Arthmoor"));
    assert_eq!(m.uploader().as_deref(), Some("Arthmoor"));
    assert_eq!(
        m.uploader_url().as_deref(),
        Some("https://www.nexusmods.com/users/1234")
    );
}

#[test]
fn apply_plan_reports_missing_sources() {
    let root = TempDir::new("root");
    let dest = TempDir::new("dest");
    fs::write(root.path().join("present.esp"), b"x").unwrap();
    let plan = vec![
        file_item("present.esp", "present.esp"),
        file_item("absent.esp", "absent.esp"),
    ];
    let missing = apply_plan(root.path(), &plan, dest.path()).unwrap();
    // The source the archive didn't contain is reported, the present one installed.
    assert_eq!(missing, vec!["absent.esp".to_string()]);
    assert!(dest.path().join("present.esp").is_file());
}

#[test]
fn mod_name_for_prefers_sidecar_then_sanitizes() {
    let dir = TempDir::new("name");
    let archive = dir.path().join("Beyond Skyrim Bruma-1234-1-0.7z");
    // No sidecar -> the filename guess (clean here).
    assert_eq!(mod_name_for(&archive), "Beyond Skyrim Bruma");
    // A sidecar modName wins, and its ':' is sanitized out of the folder name.
    fs::write(
        PathBuf::from(format!("{}.meta", archive.display())),
        "[General]\nmodName=Beyond Skyrim: Bruma\nname=file name\n",
    )
    .unwrap();
    assert_eq!(mod_name_for(&archive), "Beyond Skyrim Bruma");
}

// ---- BAIN sub-packages + manual data root --------------------------------

/// An [`ExtractedTree`] over a directory laid out by hand: the tests need the
/// post-extraction state without paying for a real 7-Zip run. Dropping it removes
/// the directory, exactly as a real extraction's would.
fn extracted(dir: &Path) -> ExtractedTree {
    ExtractedTree::owned(dir.to_path_buf())
}

/// A `mods/` dir plus an extraction temp inside it (where a real install puts it).
fn bain_layout(tag: &str) -> (TempDir, PathBuf, PathBuf) {
    let t = TempDir::new(tag);
    let mods = t.path().join("mods");
    let tmp = mods.join(".extract");
    fs::create_dir_all(&tmp).unwrap();
    (t, mods, tmp)
}

/// A three-sub-package BAIN pack whose `00 Core` and `01 Optional` both ship
/// `textures/shared.dds`, so the merge order is observable.
fn bain_pack(tag: &str) -> (TempDir, PathBuf, PathBuf, PathBuf) {
    let (t, mods, tmp) = bain_layout(tag);
    write_at(&tmp, "00 Core/MyMod.esp", b"core");
    write_at(&tmp, "00 Core/textures/shared.dds", b"CORE");
    write_at(&tmp, "01 Optional/textures/shared.dds", b"OPTIONAL");
    write_at(&tmp, "01 Optional/textures/extra.dds", b"extra");
    write_at(&tmp, "02 Unwanted/meshes/no.nif", b"no");
    let archive = t.path().join("Pack-1234-1-0.7z");
    fs::write(&archive, b"x").unwrap();
    (t, mods, tmp, archive)
}

#[test]
fn bain_merges_chosen_subpackages_later_wins() {
    let (_t, mods, tmp, archive) = bain_pack("bainmerge");
    let tree = extracted(&tmp);
    let picks = vec!["00 Core".to_string(), "01 Optional".to_string()];
    let r = install_bain(
        &tree,
        &picks,
        &archive,
        &mods,
        "Pack",
        "skyrimse",
        OverwritePolicy::Fail,
    )
    .expect("bain install");

    // Both chosen sub-packages are merged; the unticked one is not installed.
    assert!(r.dest.join("MyMod.esp").is_file());
    assert!(r.dest.join("textures/extra.dds").is_file());
    assert!(!r.dest.join("meshes").exists());
    // The BAIN contract: a later sub-package overwrites an earlier one's file.
    assert_eq!(
        fs::read(r.dest.join("textures/shared.dds")).unwrap(),
        b"OPTIONAL"
    );
    assert!(
        r.dest.join("meta.ini").is_file(),
        "a BAIN install writes meta.ini like any other"
    );
    assert!(!r.fomod);
}

#[test]
fn bain_merge_order_is_the_callers_not_the_archives() {
    // Same pack, reversed ticks: now the core file survives. This is why the API
    // takes an ordered list instead of a set.
    let (_t, mods, tmp, archive) = bain_pack("bainorder");
    let tree = extracted(&tmp);
    let picks = vec!["01 Optional".to_string(), "00 Core".to_string()];
    let r = install_bain(
        &tree,
        &picks,
        &archive,
        &mods,
        "Pack",
        "skyrimse",
        OverwritePolicy::Fail,
    )
    .expect("bain install (reversed)");
    assert_eq!(
        fs::read(r.dest.join("textures/shared.dds")).unwrap(),
        b"CORE"
    );
}

#[test]
fn bain_matches_subpackage_names_case_insensitively() {
    // The extraction may have case-folded a colliding folder name, so a pick that
    // no longer matches byte-for-byte must still resolve.
    let (t, mods, tmp) = bain_layout("bainci");
    write_at(&tmp, "00 core/MyMod.esp", b"x");
    let archive = t.path().join("Pack.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);
    let r = install_bain(
        &tree,
        &["00 Core".to_string()],
        &archive,
        &mods,
        "Pack",
        "skyrimse",
        OverwritePolicy::Fail,
    )
    .expect("bain install");
    assert!(r.dest.join("MyMod.esp").is_file());
}

#[test]
fn bain_refuses_a_stale_or_malformed_selection() {
    let (t, mods, tmp) = bain_layout("bainbad");
    write_at(&tmp, "00 Core/MyMod.esp", b"x");
    let archive = t.path().join("Pack.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);

    for bad in [
        vec![],                               // nothing ticked
        vec!["99 Nope".to_string()],          // not in the archive
        vec!["../outside".to_string()],       // traversal
        vec!["00 Core/textures".to_string()], // not a top-level sub-package
    ] {
        let r = install_bain(
            &tree,
            &bad,
            &archive,
            &mods,
            "Pack",
            "skyrimse",
            OverwritePolicy::Fail,
        );
        assert!(
            matches!(r, Err(InstallError::BadSelection(_))),
            "must refuse {bad:?}"
        );
    }
    // A refused selection must not leave a half-made mod folder behind.
    assert!(!mods.join("Pack").exists());
}

#[test]
fn bain_replace_keeps_the_old_mod_when_the_selection_is_stale() {
    // Destructive-step-last, the discipline the crate already keeps: a selection
    // that no longer resolves must be caught BEFORE the Replace wipe.
    let (t, mods, tmp) = bain_layout("bainrepl");
    write_at(&mods, "Pack/textures/a.dds", b"precious");
    write_at(&mods, "Pack/meta.ini", b"[General]\nendorsed=1\n");
    write_at(&tmp, "00 Core/MyMod.esp", b"x");
    write_at(&tmp, "01 Extras/meshes/a.nif", b"y");
    let archive = t.path().join("Pack.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);

    let r = install_bain(
        &tree,
        &["00 Core".to_string(), "99 Gone".to_string()],
        &archive,
        &mods,
        "Pack",
        "skyrimse",
        OverwritePolicy::Replace,
    );
    assert!(matches!(r, Err(InstallError::BadSelection(_))));
    assert_eq!(
        fs::read(mods.join("Pack/textures/a.dds")).unwrap(),
        b"precious"
    );
    assert_eq!(
        fs::read(mods.join("Pack/meta.ini")).unwrap(),
        b"[General]\nendorsed=1\n"
    );
}

#[test]
fn failed_bain_install_cleans_up_its_fresh_destination() {
    // A late failure (here: a sub-package shipping a DIRECTORY named meta.ini, so
    // writing the real one fails) must not leave debris the mod list would show as
    // an installed mod.
    let (t, mods, tmp) = bain_layout("bainclean");
    write_at(&tmp, "00 Core/MyMod.esp", b"x");
    write_at(
        &tmp,
        "01 Extras/meta.ini/oops.txt",
        b"a directory where a file goes",
    );
    write_at(&tmp, "01 Extras/meshes/a.nif", b"y");
    let archive = t.path().join("Pack.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);

    let r = install_bain(
        &tree,
        &["00 Core".to_string(), "01 Extras".to_string()],
        &archive,
        &mods,
        "Pack",
        "skyrimse",
        OverwritePolicy::Fail,
    );
    assert!(r.is_err(), "writing meta.ini over a directory must fail");
    assert!(
        !mods.join("Pack").exists(),
        "the fresh destination must be cleaned up"
    );
}

#[test]
fn a_root_builder_archive_installs_both_halves() {
    let (t, mods, tmp) = bain_layout("rootbuilder");
    // SSE Engine Fixes' All-In-One shape.
    write_at(&tmp, "data/skse/plugins/EngineFixes.dll", b"plugin");
    write_at(&tmp, "data/skse/plugins/EngineFixes.toml", b"cfg");
    write_at(&tmp, "d3dx9_42.dll", b"preloader");
    write_at(&tmp, "SSE Engine Fixes - Install Instructions.txt", b"docs");
    let archive = t.path().join("Engine Fixes-17230-7-0-19.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);

    let ctx = eidos_fomod::Context::default();
    let r = install_extracted(
        &tree,
        &archive,
        &mods,
        "Engine Fixes",
        "skyrimse",
        OverwritePolicy::Fail,
        &ctx,
    )
    .expect("root builder install");

    // The Data half is the mod itself, Data-relative.
    assert!(r.dest.join("skse/plugins/EngineFixes.dll").is_file());
    assert!(r.dest.join("skse/plugins/EngineFixes.toml").is_file());
    // The root half lands where the launcher projects it onto the game root.
    assert!(r.dest.join("Root/d3dx9_42.dll").is_file());
    // The Data folder itself is not nested inside the mod, and docs are dropped.
    assert!(!r.dest.join("data").exists());
    assert!(!r.dest.join("Root/data").exists());
    assert!(!r
        .dest
        .join("Root")
        .join("SSE Engine Fixes - Install Instructions.txt")
        .exists());
    assert_eq!(r.stripped, "data/");
    assert!(r.dest.join("meta.ini").is_file());
}

/// The invariant this file is built on, for the root-split path: a layout that
/// cannot be resolved must be refused BEFORE the Replace wipe, leaving the
/// existing mod exactly as it was.
///
/// `Data` shipped as a symlink is the concrete way to get there: `from_dir`
/// classifies with `Path::is_dir`, which follows the link, so the tree says
/// directory while `is_real_dir` says otherwise. This used to fall back to the
/// whole extraction temp, wipe the mod, and then die on ENOENT part-way through.
#[test]
fn an_unresolvable_data_half_is_refused_before_the_wipe() {
    let (t, mods, tmp) = bain_layout("rootsym");
    write_at(&tmp, "payload/meshes/a.nif", b"mesh");
    write_at(&tmp, "d3dx9_42.dll", b"preloader");
    std::os::unix::fs::symlink("payload", tmp.join("Data")).unwrap();

    // An existing mod that must survive the refusal untouched.
    let dest = mods.join("MyMod");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("PRECIOUS.esp"), b"irreplaceable").unwrap();

    let archive = t.path().join("Sym-1-0.7z");
    fs::write(&archive, b"x").unwrap();
    let ctx = eidos_fomod::Context::default();
    let r = install_extracted(
        &extracted(&tmp),
        &archive,
        &mods,
        "MyMod",
        "skyrimse",
        OverwritePolicy::Replace,
        &ctx,
    );

    assert!(r.is_err(), "an unresolvable layout must not install");
    assert!(
        dest.join("PRECIOUS.esp").is_file(),
        "the existing mod must be untouched"
    );
}

/// Same invariant, the empty case: an archive that resolves to nothing must not
/// wipe a mod and then report success over the crater.
#[test]
fn an_archive_with_nothing_to_install_is_refused_before_the_wipe() {
    let (t, mods, tmp) = bain_layout("rootempty");
    fs::create_dir_all(tmp.join("Root")).unwrap();

    let dest = mods.join("MyMod");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("PRECIOUS.esp"), b"irreplaceable").unwrap();

    let archive = t.path().join("Empty-1-0.7z");
    fs::write(&archive, b"x").unwrap();
    let ctx = eidos_fomod::Context::default();
    let r = install_extracted(
        &extracted(&tmp),
        &archive,
        &mods,
        "MyMod",
        "skyrimse",
        OverwritePolicy::Replace,
        &ctx,
    );

    assert!(r.is_err(), "an empty archive must not report success");
    assert!(
        dest.join("PRECIOUS.esp").is_file(),
        "the existing mod must be untouched"
    );
}

/// Two sources claiming the same name in `Root/`: a loose `notes` file beside the
/// archive's own `Root/notes/`. One would clobber the other, or the type mismatch
/// would abort the install after the wipe. Refuse instead, before the wipe.
#[test]
fn two_sources_claiming_one_root_name_are_refused() {
    let (t, mods, tmp) = bain_layout("rootdup");
    write_at(&tmp, "Data/MyMod.esp", b"esp");
    write_at(&tmp, "Root/notes/thing.cfg", b"cfg");
    write_at(&tmp, "notes", b"a loose file with the same name");

    let dest = mods.join("MyMod");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("PRECIOUS.esp"), b"irreplaceable").unwrap();

    let archive = t.path().join("Dup-1-0.7z");
    fs::write(&archive, b"x").unwrap();
    let ctx = eidos_fomod::Context::default();
    let r = install_extracted(
        &extracted(&tmp),
        &archive,
        &mods,
        "MyMod",
        "skyrimse",
        OverwritePolicy::Replace,
        &ctx,
    );

    assert!(r.is_err(), "an ambiguous Root/ layout must not install");
    assert!(
        dest.join("PRECIOUS.esp").is_file(),
        "the existing mod must be untouched"
    );
}

/// `Root/Data/` is a legitimate Root Builder layout - Root Builder maps `Root/`
/// onto the game folder, whose child IS `Data` - and it is how a repackaged
/// script extender ships. Left inside `Root/` it would be served at
/// `<game>/Data`, which the Data union is mounted over, so it would be visible
/// to nobody: not the game, not the plugin list. It has to join the Data half.
#[test]
fn root_data_joins_the_data_half_instead_of_being_shadowed() {
    let (t, mods, tmp) = bain_layout("rootdata");
    write_at(&tmp, "Root/skse64_loader.exe", b"loader");
    write_at(&tmp, "Root/Data/foo.esp", b"esp");
    write_at(&tmp, "Root/Data/SKSE/Plugins/x.dll", b"plugin");
    let archive = t.path().join("SKSE-1-0.7z");
    fs::write(&archive, b"x").unwrap();

    let ctx = eidos_fomod::Context::default();
    let r = install_extracted(
        &extracted(&tmp),
        &archive,
        &mods,
        "SKSE",
        "skyrimse",
        OverwritePolicy::Fail,
        &ctx,
    )
    .expect("root/data install");

    // Served by the Data union, where the game and the plugin list can see it.
    assert!(r.dest.join("foo.esp").is_file());
    assert!(r.dest.join("SKSE/Plugins/x.dll").is_file());
    // Served by the root union, next to the game exe.
    assert!(r.dest.join("Root/skse64_loader.exe").is_file());
    // And NOT left where nothing would ever read it.
    assert!(!r.dest.join("Root/Data").exists());
}

/// The last fallible step after the wipe: creating `<mod>/Root` when the Data
/// half just planted a FILE of that name. `create_dir_all` is EEXIST there, and
/// the mod is already gone by then.
#[test]
fn a_file_named_root_in_the_data_half_does_not_abort_the_install() {
    let (t, mods, tmp) = bain_layout("rootfile");
    write_at(&tmp, "Data/meshes/a.nif", b"mesh");
    write_at(&tmp, "Data/Root", b"a plain file called Root");
    write_at(&tmp, "d3dx9_42.dll", b"preloader");

    let dest = mods.join("MyMod");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("PRECIOUS.esp"), b"irreplaceable").unwrap();

    let archive = t.path().join("Odd-1-0.7z");
    fs::write(&archive, b"x").unwrap();
    let ctx = eidos_fomod::Context::default();
    let r = install_extracted(
        &extracted(&tmp),
        &archive,
        &mods,
        "MyMod",
        "skyrimse",
        OverwritePolicy::Replace,
        &ctx,
    )
    .expect("must not abort half-way through a Replace");

    assert!(r.dest.join("meshes/a.nif").is_file());
    assert!(r.dest.join("Root/d3dx9_42.dll").is_file());
    assert!(
        r.dest.join("meta.ini").is_file(),
        "a completed install always has its meta"
    );
}

/// Same shape one level down: a directory from the archive's `Root/` landing on
/// a name already occupied by a file. `overlay_dir` cannot clear its OWN
/// destination - its first act is `create_dir_all` - so the caller must.
#[test]
fn a_ue4ss_mod_lands_in_all_three_places_it_shipped_for() {
    // The real shape of CustomNanosuitSystem, which addresses THREE different
    // places from one archive, all relative to the game INSTALL root:
    //
    //   ~mods/…           the config JSONs, which is the deploy root
    //   LogicMods/…       a blueprint pak, a SIBLING of the deploy root
    //   SB/Binaries/…     the UE4SS Lua mod
    //
    // Only the first is Data-relative. The other two have to travel through
    // Root/ at their own paths, or they land somewhere nothing reads.
    let (t, mods, tmp) = bain_layout("ue4ss");
    write_at(
        &tmp,
        "SB/Content/Paks/~mods/CNS/Cosmetics/Outfits.dekcns.json",
        b"{}",
    );
    write_at(&tmp, "SB/Content/Paks/LogicMods/DekCNS_P.pak", b"pak");
    write_at(
        &tmp,
        "SB/Binaries/Win64/ue4ss/Mods/DekCNS/enabled.txt",
        b"1",
    );

    let archive = t.path().join("CustomNanosuitSystem-1496.zip");
    fs::write(&archive, b"x").unwrap();
    let r = install_extracted(
        &extracted(&tmp),
        &archive,
        &mods,
        "CNS",
        "stellarblade",
        OverwritePolicy::Fail,
        &eidos_fomod::Context::default(),
    )
    .expect("a three-way install-root archive installs");

    // The Data half is the mod root, so it deploys INTO ~mods - which is where
    // CNS roots its recursive config scan. One level higher and the mod loads
    // its pak and adds nothing, which is what a real install did.
    assert!(r.dest.join("CNS/Cosmetics/Outfits.dekcns.json").is_file());
    // The other two keep their archive paths under Root/.
    assert!(r
        .dest
        .join("Root/SB/Content/Paks/LogicMods/DekCNS_P.pak")
        .is_file());
    assert!(
        r.dest
            .join("Root/SB/Binaries/Win64/ue4ss/Mods/DekCNS/enabled.txt")
            .is_file(),
        "the ue4ss tree keeps the path the loader expects"
    );
    // And nothing was flattened or duplicated on the way.
    assert!(
        !r.dest.join("Root/Binaries").exists(),
        "the game directory is not flattened"
    );
    assert!(
        !r.dest.join("Root/LogicMods").exists(),
        "nor is the paks directory"
    );
    assert!(
        !r.dest.join("~mods").exists(),
        "the deploy root is not nested inside itself"
    );
}

#[test]
fn a_merge_survives_a_type_conflict_with_the_existing_mod() {
    // A mod update replaces a directory with a file of the same name (or the
    // reverse). Merge used to copy with `copy_dir_all`, which aborted with
    // EISDIR mid-way - leaving the mod half old, half new, with no cleanup
    // because a merge target is not "fresh". The archive must win the name in
    // both directions, like the BAIN and root merge paths already do.
    let (t, mods, tmp) = bain_layout("mergetype");
    write_at(&tmp, "MyMod.esp", b"v2");
    write_at(&tmp, "docs", b"now a file"); // was a directory in v1
    write_at(&tmp, "SKSE/Plugins/foo.dll/keep.txt", b"now a dir"); // was a file in v1

    let dest = mods.join("MyMod");
    fs::create_dir_all(dest.join("docs")).unwrap();
    fs::write(dest.join("docs/readme.txt"), b"old dir content").unwrap();
    fs::create_dir_all(dest.join("SKSE/Plugins")).unwrap();
    fs::write(dest.join("SKSE/Plugins/foo.dll"), b"was a file").unwrap();
    fs::write(dest.join("untouched.esp"), b"stays").unwrap();

    let archive = t.path().join("MyMod-2-0.7z");
    fs::write(&archive, b"x").unwrap();
    let r = install_extracted(
        &extracted(&tmp),
        &archive,
        &mods,
        "MyMod",
        "skyrimse",
        OverwritePolicy::Merge,
        &eidos_fomod::Context::default(),
    )
    .expect("a type conflict must not abort a merge");

    // Both directions: the archive's file replaced the dir, its dir replaced the file.
    assert_eq!(fs::read(r.dest.join("docs")).unwrap(), b"now a file");
    assert!(r.dest.join("SKSE/Plugins/foo.dll/keep.txt").is_file());
    // And merge semantics are intact: files the archive does not ship survive.
    assert_eq!(fs::read(r.dest.join("untouched.esp")).unwrap(), b"stays");
    assert_eq!(fs::read(r.dest.join("MyMod.esp")).unwrap(), b"v2");
}

#[test]
fn a_root_entry_can_land_on_a_file_of_the_same_name() {
    let (t, mods, tmp) = bain_layout("rootocc");
    write_at(&tmp, "Data/MyMod.esp", b"esp");
    write_at(&tmp, "Root/tools/patch.exe", b"tool");

    // A previous install left `Root/tools` as a FILE.
    let dest = mods.join("MyMod");
    fs::create_dir_all(dest.join("Root")).unwrap();
    fs::write(dest.join("Root/tools"), b"stale file").unwrap();

    let archive = t.path().join("Occ-1-0.7z");
    fs::write(&archive, b"x").unwrap();
    let ctx = eidos_fomod::Context::default();
    let r = install_extracted(
        &extracted(&tmp),
        &archive,
        &mods,
        "MyMod",
        "skyrimse",
        OverwritePolicy::Merge,
        &ctx,
    )
    .expect("the stale file must lose, not abort the install");

    assert!(r.dest.join("Root/tools/patch.exe").is_file());
}

/// Engine Fixes' second half on its own: no Data at all, just the preloader.
/// It has to become a mod whose whole content is `Root/`, or it installs as a
/// mod that looks fine and never loads.
#[test]
fn a_bare_preloader_installs_entirely_into_root() {
    let (t, mods, tmp) = bain_layout("pureroot");
    write_at(&tmp, "d3dx9_42.dll", b"preloader");
    write_at(&tmp, "vortex_override_instructions.json", b"{}");
    let archive = t.path().join("Preloader-17230-7.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);

    let ctx = eidos_fomod::Context::default();
    let r = install_extracted(
        &tree,
        &archive,
        &mods,
        "Preloader",
        "skyrimse",
        OverwritePolicy::Fail,
        &ctx,
    )
    .expect("pure root install");

    assert!(r.dest.join("Root/d3dx9_42.dll").is_file());
    // Nothing leaks into the Data-relative half.
    assert!(!r.dest.join("d3dx9_42.dll").exists());
    assert_eq!(r.stripped, "");
    assert!(r.dest.join("meta.ini").is_file());
}

/// The same install seen from the launcher's side: what it will mount over the
/// game root is exactly the preloader, and nothing of the Data half.
#[test]
fn an_installed_root_mod_is_what_the_launcher_projects() {
    let (t, mods, tmp) = bain_layout("rootproject");
    write_at(&tmp, "Data/MyMod.esp", b"esp");
    write_at(&tmp, "Root/d3dx9_42.dll", b"preloader");
    write_at(&tmp, "Root/tools/patch.exe", b"tool");
    let archive = t.path().join("Mod-1-0.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);

    let ctx = eidos_fomod::Context::default();
    let r = install_extracted(
        &tree,
        &archive,
        &mods,
        "Mod",
        "skyrimse",
        OverwritePolicy::Fail,
        &ctx,
    )
    .expect("root builder install");

    assert!(r.dest.join("MyMod.esp").is_file());
    // One level, not two: `Root/Root/` would put the DLL a directory away from
    // where the Windows loader looks, which is the whole point of this path.
    assert!(r.dest.join("Root/d3dx9_42.dll").is_file());
    assert!(r.dest.join("Root/tools/patch.exe").is_file());
    assert!(!r.dest.join("Root/Root").exists());
}

#[test]
fn manual_installs_from_the_chosen_root() {
    let (t, mods, tmp) = bain_layout("manualroot");
    write_at(&tmp, "Package/Data/meshes/a.nif", b"mesh");
    write_at(&tmp, "Package/Data/MyMod.esp", b"esp");
    write_at(&tmp, "Package/src/build.sh", b"tool");
    write_at(&tmp, "readme.txt", b"docs");
    let archive = t.path().join("Odd-1234-1-0.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);

    let r = install_manual(
        &tree,
        "Package/Data",
        &archive,
        &mods,
        "Odd",
        "skyrimse",
        OverwritePolicy::Fail,
    )
    .expect("manual install");
    // Everything under the chosen root becomes the mod; everything beside it is
    // dropped, exactly like the wrapper strip on the simple path.
    assert!(r.dest.join("meshes/a.nif").is_file());
    assert!(r.dest.join("MyMod.esp").is_file());
    assert!(!r.dest.join("src").exists());
    assert!(!r.dest.join("readme.txt").exists());
    assert_eq!(r.stripped, "Package/Data/");
    assert!(r.dest.join("meta.ini").is_file());
}

#[test]
fn manual_root_is_matched_case_insensitively() {
    let (t, mods, tmp) = bain_layout("manualci");
    write_at(&tmp, "Package/Data/MyMod.esp", b"esp");
    let archive = t.path().join("Odd.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);
    let r = install_manual(
        &tree,
        "package/data",
        &archive,
        &mods,
        "Odd",
        "skyrimse",
        OverwritePolicy::Fail,
    )
    .expect("manual install");
    assert!(r.dest.join("MyMod.esp").is_file());
}

#[test]
fn manual_empty_root_installs_the_archive_as_is() {
    // The user's "this IS already the data dir" - nothing is stripped.
    let (t, mods, tmp) = bain_layout("manualasis");
    write_at(&tmp, "Package/Data/MyMod.esp", b"esp");
    let archive = t.path().join("Odd.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);
    let r = install_manual(
        &tree,
        "",
        &archive,
        &mods,
        "Odd",
        "skyrimse",
        OverwritePolicy::Fail,
    )
    .expect("manual install at root");
    assert!(r.dest.join("Package/Data/MyMod.esp").is_file());
    assert_eq!(r.stripped, "");
}

#[test]
fn manual_refuses_a_root_outside_the_archive() {
    let (t, mods, tmp) = bain_layout("manualesc");
    write_at(&tmp, "Package/MyMod.esp", b"esp");
    let archive = t.path().join("Odd.7z");
    fs::write(&archive, b"x").unwrap();
    let tree = extracted(&tmp);
    for bad in ["../..", "Package/../../mods", "/etc", "nope"] {
        let r = install_manual(
            &tree,
            bad,
            &archive,
            &mods,
            "Odd",
            "skyrimse",
            OverwritePolicy::Fail,
        );
        assert!(
            matches!(r, Err(InstallError::BadSelection(_))),
            "must refuse root '{bad}'"
        );
    }
    assert!(!mods.join("Odd").exists());
}

#[test]
fn overlay_replaces_a_conflicting_entry_type() {
    // Two sub-packages disagreeing on whether a name is a file or a directory: the
    // later one wins outright, rather than aborting the install with EISDIR.
    let t = TempDir::new("overlaytype");
    write_at(t.path(), "a/conflict/inner.txt", b"dir");
    write_at(t.path(), "b/conflict", b"file");
    write_at(t.path(), "b/other/x.txt", b"x");
    let dest = t.path().join("dest");
    overlay_dir(&t.path().join("a"), &dest).unwrap();
    overlay_dir(&t.path().join("b"), &dest).unwrap();
    assert_eq!(fs::read(dest.join("conflict")).unwrap(), b"file");
    assert!(dest.join("other/x.txt").is_file());
    // ...and the reverse, a directory landing on a file.
    let dest2 = t.path().join("dest2");
    overlay_dir(&t.path().join("b"), &dest2).unwrap();
    overlay_dir(&t.path().join("a"), &dest2).unwrap();
    assert!(dest2.join("conflict/inner.txt").is_file());
}

#[test]
fn overlay_never_writes_through_a_planted_symlink() {
    // Security: an earlier sub-package ships `victim -> <outside>`, a later one
    // ships a real file of the same name. The copy must replace the LINK, never
    // follow it and write outside the mod folder.
    use std::os::unix::fs::symlink;
    let t = TempDir::new("overlayescape");
    let outside = t.path().join("outside.txt");
    fs::write(&outside, b"untouched").unwrap();
    fs::create_dir_all(t.path().join("a")).unwrap();
    symlink(&outside, t.path().join("a/victim")).unwrap();
    write_at(t.path(), "b/victim", b"payload");

    let dest = t.path().join("dest");
    overlay_dir(&t.path().join("a"), &dest).unwrap();
    overlay_dir(&t.path().join("b"), &dest).unwrap();
    assert_eq!(
        fs::read(&outside).unwrap(),
        b"untouched",
        "must not write through the link"
    );
    assert_eq!(fs::read(dest.join("victim")).unwrap(), b"payload");
    assert!(!fs::symlink_metadata(dest.join("victim"))
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn overlay_preserves_a_dangling_symlink() {
    // 7-Zip does extract symlinks; copying THROUGH a dangling one would fail the
    // whole install over an entry the simple (rename) path installs fine.
    use std::os::unix::fs::symlink;
    let t = TempDir::new("overlaysym");
    fs::create_dir_all(t.path().join("a")).unwrap();
    symlink("nowhere", t.path().join("a/link")).unwrap();
    let dest = t.path().join("dest");
    overlay_dir(&t.path().join("a"), &dest).unwrap();
    let meta = fs::symlink_metadata(dest.join("link")).unwrap();
    assert!(meta.file_type().is_symlink());
    assert_eq!(
        fs::read_link(dest.join("link")).unwrap().to_str(),
        Some("nowhere")
    );
}

#[test]
fn from_dir_feeds_bain_detection_with_real_names() {
    // End-to-end over an actual directory: the disk layout an extraction leaves
    // must classify as BAIN, with the folder names as they exist on disk.
    let (_t, _mods, tmp) = bain_layout("bainfromdir");
    write_at(&tmp, "00 Core/MyMod.esp", b"x");
    write_at(&tmp, "01 Optional Textures/textures/a.dds", b"y");
    write_at(&tmp, "--03 Disabled/meshes/a.nif", b"z");
    write_at(&tmp, "Docs/readme.txt", b"d");
    let layout = ArchiveTree::from_dir(&tmp).unwrap();
    assert_eq!(
        layout.simple_archive_base(rules()),
        None,
        "not a simple archive"
    );
    let (subs, invalid) = layout.bain_subpackages(rules());
    assert_eq!(subs, vec!["00 Core", "01 Optional Textures"]);
    assert_eq!(invalid, 0);
}

#[test]
fn civil_from_unix_is_correct() {
    assert_eq!(civil_from_unix(1_700_000_000), (2023, 11, 14)); // 2023-11-14 UTC
    assert_eq!(civil_from_unix(0), (1970, 1, 1));
}

#[test]
fn a_symlink_loop_in_an_archive_does_not_blow_the_stack() {
    // An archive is untrusted input. `link -> ..` used to make this walk recurse
    // until the process died, which for the GUI meant a SIGSEGV mid-install.
    let dir = std::env::temp_dir().join(format!("eidos-loop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Data/textures")).unwrap();
    std::fs::write(dir.join("Data/a.esp"), b"x").unwrap();
    std::os::unix::fs::symlink("..", dir.join("Data/textures/up")).unwrap();

    let tree = ArchiveTree::from_dir(&dir).expect("walk must return, not recurse forever");
    let all = tree.flatten();
    assert!(
        all.iter().any(|e| e.path.ends_with("a.esp")),
        "real files still described"
    );
    // The loop is BOUNDED, not banned: symlinked directories are followed - an
    // archive may legitimately ship `Data` as a symlink, and a first version of
    // this guard that refused to descend reclassified such archives as Manual -
    // so the defence is the depth cap, and the proof is that this returned.
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_archive_shipping_data_as_a_symlink_still_classifies_by_its_contents() {
    // The regression a symlink-refusing loop guard introduced: `Data -> payload`
    // made simple_archive_base() blind to the whole payload, so a perfectly
    // ordinary archive fell through to the manual picker.
    let dir = std::env::temp_dir().join(format!("eidos-symdata-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("payload/textures")).unwrap();
    std::fs::write(dir.join("payload/mod.esp"), b"x").unwrap();
    std::fs::write(dir.join("payload/textures/a.dds"), b"x").unwrap();
    std::os::unix::fs::symlink(dir.join("payload"), dir.join("Data")).unwrap();

    let tree = ArchiveTree::from_dir(&dir).unwrap();
    let all = tree.flatten();
    assert!(
        all.iter().any(|e| e.path == "Data" && e.is_dir),
        "the symlinked Data must read as a directory: {all:?}"
    );
    assert!(
        all.iter().any(|e| e.path == "Data/mod.esp"),
        "its contents must be described: {all:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn failed_replacements_preserve_payload_and_raw_metadata_in_every_flow() {
    for flow in ["simple", "manual", "bain", "fomod", "default-fomod"] {
        let t = TempDir::new(flow);
        let mods = t.path().join("mods");
        let src = t.path().join("source");
        let archive = t.path().join("Update.7z");
        let original = b"[General]\r\nnotes=precious\r\n\r\n[Unknown]\r\nraw=unchanged\r\n";
        write_at(&mods, "Old/precious.esp", b"original payload");
        write_at(&mods, "Old/meta.ini", original);
        let root = if flow == "bain" { "00 Core/" } else { "" };
        write_at(&src, &format!("{root}new.esp"), b"replacement");
        // Metadata publication must fail after payload placement has succeeded.
        write_at(&src, &format!("{root}meta.ini/blocker"), b"directory");
        let xml = "<config><moduleName>Test</moduleName><requiredInstallFiles><file source=\"new.esp\" destination=\"new.esp\"/><file source=\"new.esp\" destination=\"../escape.esp\"/></requiredInstallFiles></config>";
        if flow.contains("fomod") {
            write_at(&src, "fomod/ModuleConfig.xml", xml.as_bytes());
        }
        let tree = extracted(&src);
        let ctx = eidos_fomod::Context::default();
        let result = match flow {
            "manual" => install_manual(
                &tree,
                "",
                &archive,
                &mods,
                "Old",
                "skyrimse",
                OverwritePolicy::Replace,
            ),
            "bain" => install_bain(
                &tree,
                &["00 Core".into()],
                &archive,
                &mods,
                "Old",
                "skyrimse",
                OverwritePolicy::Replace,
            ),
            "fomod" => {
                let config = parse_fomod_at(&src).unwrap();
                let selection = eidos_fomod::default_selection(&config, &ctx);
                finish_fomod(
                    FomodSession {
                        config,
                        root: src,
                        tree,
                        name: "Old".into(),
                        archive,
                    },
                    &selection,
                    &mods,
                    "skyrimse",
                    &ctx,
                    OverwritePolicy::Replace,
                )
            }
            _ => install_extracted(
                &tree,
                &archive,
                &mods,
                "Old",
                "skyrimse",
                OverwritePolicy::Replace,
                &ctx,
            ),
        };
        assert!(
            result.is_err(),
            "{flow} must reject the incomplete replacement"
        );
        assert_eq!(
            fs::read(mods.join("Old/precious.esp")).ok().as_deref(),
            Some(b"original payload".as_slice()),
            "{flow}"
        );
        assert_eq!(
            fs::read(mods.join("Old/meta.ini")).ok().as_deref(),
            Some(original.as_slice()),
            "{flow}"
        );
    }
}

#[test]
fn default_fomod_wins_over_loose_plugins_and_reports_missing_sources() {
    let t = TempDir::new("default-fomod-priority");
    let src = t.path().join("source");
    write_at(&src, "one.esp", b"selected");
    write_at(&src, "two.esp", b"unselected");
    write_at(&src, "fomod/ModuleConfig.xml", br#"<config><moduleName>Test</moduleName><requiredInstallFiles><file source="one.esp" destination="chosen.esp"/><file source="missing.esp" destination="missing.esp"/></requiredInstallFiles></config>"#);
    let report = install_extracted(
        &extracted(&src),
        &t.path().join("archive.7z"),
        &t.path().join("mods"),
        "Test",
        "skyrimse",
        OverwritePolicy::Fail,
        &Default::default(),
    )
    .unwrap();
    assert!(report.fomod);
    assert_eq!(report.missing, ["missing.esp"]);
    assert!(eidos_instance::ModMeta::read(&report.dest.join("meta.ini"))
        .install_warning()
        .unwrap()
        .contains("missing.esp"));
    assert_eq!(
        fs::read(report.dest.join("chosen.esp")).unwrap(),
        b"selected"
    );
    assert!(!report.dest.join("two.esp").exists());
}

#[test]
fn wrapped_root_archives_keep_both_data_and_sibling_payload() {
    for prefix in ["Wrapper/", "Outer/Inner/"] {
        let t = TempDir::new("wrapped-root");
        let src = t.path().join("source");
        write_at(&src, &format!("{prefix}Data/scripts/test.pex"), b"script");
        write_at(&src, &format!("{prefix}loader.dll"), b"loader");
        let report = install_extracted(
            &extracted(&src),
            &t.path().join("archive.7z"),
            &t.path().join("mods"),
            "Test",
            "skyrimse",
            OverwritePolicy::Fail,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(
            fs::read(report.dest.join("scripts/test.pex")).unwrap(),
            b"script"
        );
        assert_eq!(
            fs::read(report.dest.join("Root/loader.dll")).unwrap(),
            b"loader"
        );
    }
}

#[test]
fn merge_retains_distinct_file_sources_and_replace_discards_old_sources() {
    let t = TempDir::new("installed-sources");
    let mods = t.path().join("mods");
    for (id, policy) in [
        (101, OverwritePolicy::Fail),
        (102, OverwritePolicy::Merge),
        (103, OverwritePolicy::Replace),
    ] {
        let archive = t.path().join(format!("archive-{id}.7z"));
        fs::write(
            PathBuf::from(format!("{}.meta", archive.display())),
            format!("[General]\nmodID=42\nfileID={id}\nversion=1\n"),
        )
        .unwrap();
        let src = t.path().join(format!("source-{id}"));
        write_at(&src, "test.esp", b"plugin");
        let report = install_extracted(
            &extracted(&src),
            &archive,
            &mods,
            "Test",
            "skyrimse",
            policy,
            &Default::default(),
        )
        .unwrap();
        let meta_path = report.dest.join("meta.ini");
        let text = fs::read_to_string(&meta_path).unwrap();
        assert!(text.contains(&format!("\\fileid={id}\n")), "{text}");
        if id == 101 {
            fs::write(&meta_path, format!("{text}\n[Custom]\nraw=keep=verbatim\n")).unwrap();
        } else {
            assert!(text.contains("[Custom]\nraw=keep=verbatim\n"), "{text}");
            assert_eq!(text.contains("\\fileid=101\n"), id == 102, "{text}");
        }
    }
}

#[test]
fn fomod_context_uses_plugin_activation_and_overwrite_presence() {
    let t = TempDir::new("plugin-context");
    let game = t.path().join("game");
    let enabled = t.path().join("enabled");
    let disabled = t.path().join("disabled");
    let overwrite = t.path().join("overwrite");
    write_at(&game, "Skyrim.esm", b"master");
    write_at(&enabled, "DisabledInProfile.esp", b"plugin");
    write_at(&disabled, "Unavailable.esp", b"plugin");
    write_at(&overwrite, "Generated.esp", b"plugin");
    write_at(&overwrite, "Unselected.esp", b"plugin");
    let ctx = fomod_context_with_plugins(
        &game,
        &[enabled],
        &[disabled],
        &overwrite,
        [
            ("SKYRIM.ESM", true),
            ("DisabledInProfile.esp", false),
            ("Generated.esp", true),
            ("Absent.esp", true),
        ],
    );
    for (name, state) in [
        ("skyrim.esm", "Active"),
        ("disabledinprofile.esp", "Inactive"),
        ("unavailable.esp", "Inactive"),
        ("generated.esp", "Active"),
        ("unselected.esp", "Inactive"),
    ] {
        assert_eq!(
            ctx.file_states.get(name).map(String::as_str),
            Some(state),
            "{name}"
        );
    }
    assert!(!ctx.file_states.contains_key("absent.esp"));
}

#[test]
fn failed_publication_restores_the_original_mod() {
    let t = TempDir::new("publication-rollback");
    let dest = t.path().join("Old");
    write_at(&dest, "precious.esp", b"original");
    write_at(&dest, "meta.ini", b"raw metadata");
    let stage = t.path().join("stage");
    fs::create_dir(&stage).unwrap();
    assert!(publish_install(extracted(&stage), &stage.join("missing-payload"), &dest).is_err());
    assert_eq!(fs::read(dest.join("precious.esp")).unwrap(), b"original");
    assert_eq!(fs::read(dest.join("meta.ini")).unwrap(), b"raw metadata");
    assert!(!stage.exists());
}

#[test]
fn wrappers_with_docs_keep_explicit_root_data_and_refuse_sibling_payload() {
    let t = TempDir::new("wrapper-docs");
    let src = t.path().join("source");
    write_at(&src, "Readme.txt", b"documentation");
    write_at(&src, "Wrapper/Root/Data/scripts/test.pex", b"script");
    write_at(&src, "Wrapper/Root/loader.dll", b"loader");
    let layout = ArchiveTree::from_dir(&src).unwrap();
    assert!(layout.root_builder_split(rules()).is_some());
    let report = install_extracted(
        &extracted(&src),
        &t.path().join("archive.7z"),
        &t.path().join("mods"),
        "Test",
        "skyrimse",
        OverwritePolicy::Fail,
        &Default::default(),
    )
    .unwrap();
    assert_eq!(
        fs::read(report.dest.join("scripts/test.pex")).unwrap(),
        b"script"
    );
    assert_eq!(
        fs::read(report.dest.join("Root/loader.dll")).unwrap(),
        b"loader"
    );
    let ambiguous = t.path().join("ambiguous");
    write_at(&ambiguous, "Wrapper/Data/scripts/test.pex", b"script");
    write_at(&ambiguous, "Wrapper/loader.dll", b"loader");
    write_at(&ambiguous, "outside.dll", b"must not be dropped");
    assert!(ArchiveTree::from_dir(&ambiguous)
        .unwrap()
        .root_builder_split(rules())
        .is_none());
}

#[test]
fn merging_the_same_source_deduplicates_and_an_unknown_replacement_clears_identity() {
    let t = TempDir::new("source-reset");
    let dest = t.path().join("mods/Test");
    write_at(&dest, "meta.ini", b"[General]\nmodid=42\nfileID=101\neidosCollectionOwner=owned\n\n[installedFiles]\n1\\modid=42\n1\\fileid=101\nsize=1\n");
    let archive = t.path().join("archive.7z");
    fs::write(
        t.path().join("archive.7z.meta"),
        "[General]\nmodID=42\nfileID=101\n",
    )
    .unwrap();
    for policy in [OverwritePolicy::Merge, OverwritePolicy::Replace] {
        let src = t.path().join("source");
        write_at(&src, "test.esp", b"plugin");
        install_extracted(
            &extracted(&src),
            &archive,
            &t.path().join("mods"),
            "Test",
            "skyrimse",
            policy.clone(),
            &Default::default(),
        )
        .unwrap();
        let meta = ModMeta::read(&dest.join("meta.ini"));
        if policy == OverwritePolicy::Merge {
            assert_eq!(meta.installed_files(), [(42, 101)]);
            fs::remove_file(t.path().join("archive.7z.meta")).unwrap();
        } else {
            assert!(meta.installed_files().is_empty());
            assert_eq!(meta.file_id(), None);
            assert_eq!(meta.mod_id(), None);
        }
        assert_ne!(
            meta.collection_owner(),
            Some("owned"),
            "manual edits revoke collection replacement authority"
        );
    }
}

#[test]
fn public_archive_entry_points_apply_the_same_fomod_plan() {
    let Some(bin) = eidos_sevenzip::find_7z() else {
        return;
    };
    let t = TempDir::new("public-fomod");
    let src = t.path().join("source");
    write_at(&src, "one.esp", b"selected");
    write_at(&src, "two.esp", b"unselected");
    write_at(&src, "fomod/ModuleConfig.xml", br#"<config><moduleName>Test</moduleName><requiredInstallFiles><file source="one.esp" destination="chosen.esp"/><file source="missing.esp" destination="missing.esp"/></requiredInstallFiles></config>"#);
    let archive = t.path().join("archive.zip");
    let output = std::process::Command::new(bin)
        .args(["a", "-tzip"])
        .arg(&archive)
        .arg(".")
        .current_dir(&src)
        .output()
        .unwrap();
    assert!(output.status.success());
    let mods = t.path().join("mods");
    let ctx = Default::default();
    let automatic = install_archive_with_policy(
        &archive,
        &mods,
        "Automatic",
        "skyrimse",
        OverwritePolicy::Fail,
        &ctx,
    )
    .unwrap();
    let Opened::Fomod(session) = open_archive(&archive, &mods, "Interactive", "skyrimse").unwrap()
    else {
        panic!("FOMOD must take priority")
    };
    let selection = eidos_fomod::default_selection(&session.config, &ctx);
    let interactive = finish_fomod(
        *session,
        &selection,
        &mods,
        "skyrimse",
        &ctx,
        OverwritePolicy::Fail,
    )
    .unwrap();
    for report in [automatic, interactive] {
        assert!(report.fomod);
        assert_eq!(report.missing, ["missing.esp"]);
        assert!(eidos_instance::ModMeta::read(&report.dest.join("meta.ini"))
            .install_warning()
            .unwrap()
            .contains("missing.esp"));
        assert_eq!(
            fs::read(report.dest.join("chosen.esp")).unwrap(),
            b"selected"
        );
        assert!(!report.dest.join("two.esp").exists());
    }
}

#[test]
fn persisted_fomod_context_uses_profile_activation_and_whiteouts() {
    let t = TempDir::new("persisted-fomod-context");
    let inst = eidos_instance::Instance::portable(t.path().join("instance"));
    inst.create().unwrap();
    let source = inst.create_empty_mod("Source").unwrap();
    inst.save_modlist(std::slice::from_ref(&source)).unwrap();
    let game = t.path().join("game/Data");
    fs::create_dir_all(&game).unwrap();
    let mut header = [0u8; 24];
    header[..4].copy_from_slice(b"TES4");
    header[20..22].copy_from_slice(&44u16.to_le_bytes());
    for name in ["Enabled.esp", "Disabled.esp", "Hidden.esp"] {
        write_at(&source.path, name, &header);
    }
    write_at(&inst.overwrite_dir(), "Generated.esp", &header);
    fs::write(
        inst.active().plugins_state_dir().join("plugins.txt"),
        "*Enabled.esp\nDisabled.esp\n*Hidden.esp\n*Generated.esp\n",
    )
    .unwrap();
    let stack = eidos_core::LayerStack::new(vec![source.path], inst.overwrite_dir());
    stack.remove("Hidden.esp").unwrap();
    let context = fomod_context_for_instance(&inst, &game, "skyrimse", None);
    assert_eq!(
        context.file_states.get("enabled.esp").map(String::as_str),
        Some("Active")
    );
    assert_eq!(
        context.file_states.get("disabled.esp").map(String::as_str),
        Some("Inactive")
    );
    assert_eq!(
        context.file_states.get("generated.esp").map(String::as_str),
        Some("Active")
    );
    assert!(!context.file_states.contains_key("hidden.esp"));
}

#[test]
fn unreadable_metadata_aborts_replacement_before_old_payload_is_touched() {
    let t = TempDir::new("unreadable-replacement-meta");
    let mods = t.path().join("mods");
    write_at(&mods.join("Old"), "keep.esp", b"old");
    write_at(&mods.join("Old"), "meta.ini", &[0xff]);
    let src = t.path().join("source");
    write_at(&src, "new.esp", b"new");
    assert!(install_extracted(
        &extracted(&src),
        &t.path().join("archive.7z"),
        &mods,
        "Old",
        "skyrimse",
        OverwritePolicy::Replace,
        &Default::default()
    )
    .is_err());
    assert_eq!(fs::read(mods.join("Old/meta.ini")).unwrap(), [0xff]);
    assert_eq!(fs::read(mods.join("Old/keep.esp")).unwrap(), b"old");
    assert!(!mods.join("Old/new.esp").exists());
}

#[test]
fn replacement_reuses_existing_case_and_does_not_resurrect_empty_source_arrays() {
    let t = TempDir::new("case-source-replacement");
    let mods = t.path().join("mods");
    write_at(
        &mods.join("Mixed Case"),
        "meta.ini",
        b"[General]\nmodid=9\nfileID=10\n[installedFiles]\nsize=0\n",
    );
    write_at(&mods.join("Mixed Case"), "old.esp", b"old");
    let src = t.path().join("source");
    write_at(&src, "new.esp", b"new");
    assert_eq!(
        collision_name(&mods, "mixed case").as_deref(),
        Some("Mixed Case")
    );
    let report = install_extracted(
        &extracted(&src),
        &t.path().join("archive.7z"),
        &mods,
        "mixed case",
        "skyrimse",
        OverwritePolicy::Merge,
        &Default::default(),
    )
    .unwrap();
    assert_eq!(report.name, "Mixed Case");
    assert!(!mods.join("mixed case").exists());
    assert!(ModMeta::read(&report.dest.join("meta.ini"))
        .installed_files()
        .is_empty());
}

#[test]
fn failed_merge_revokes_exact_identity_and_marks_partial_content() {
    let mods = TempDir::new("merge-checkpoint");
    let dest = mods.path().join("Owned");
    fs::create_dir(&dest).unwrap();
    let mut original = ModMeta::default();
    original.set("eidosCollectionOwner", "collection:member");
    original.set("modid", "1");
    original.set("fileID", "2");
    original.set_installed_files(&[(1, 2)]);
    original.write(&dest.join("meta.ini")).unwrap();
    // Inject at the live publication seam, after native staging validation.
    let result = install_destination_ready(
        Path::new("Update.7z"),
        mods.path(),
        "Owned",
        "skyrimse",
        OverwritePolicy::Merge,
        &[],
        |target, _| {
            fs::write(target.join("changed.esp"), b"partial")?;
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "injected mid-merge copy failure",
            )
            .into())
        },
    );
    assert!(result.is_err());
    assert!(dest.join("changed.esp").is_file());
    let meta = ModMeta::read(&dest.join("meta.ini"));
    assert_ne!(meta.collection_owner(), Some("collection:member"));
    assert!(meta.has_installed_files());
    assert!(meta.installed_files().is_empty());
    assert!(meta.file_id().is_none());
    assert!(meta
        .install_warning()
        .unwrap()
        .contains("Merge did not complete"));
}

#[test]
fn fomod_copy_refuses_destination_links_at_any_depth() {
    use std::os::unix::fs::symlink;
    for (folder, link_path, destination) in [
        (false, "scripts", "scripts/personal.txt"),
        (false, "scripts/personal.txt", "scripts/personal.txt"),
        (true, "scripts", "scripts"),
        (true, "scripts/nested", "scripts"),
        (true, "scripts/nested/personal.txt", "scripts"),
    ] {
        let root = TempDir::new("fomod-safe-source");
        let dest = TempDir::new("fomod-safe-dest");
        let outside = TempDir::new("fomod-safe-outside");
        write_at(root.path(), "payload/nested/personal.txt", b"incoming");
        write_at(outside.path(), "personal.txt", b"precious");
        let link = dest.path().join(link_path);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let target = if link_path.ends_with(".txt") {
            outside.path().join("personal.txt")
        } else {
            outside.path().to_path_buf()
        };
        symlink(target, link).unwrap();
        let mut item = file_item(
            if folder {
                "payload"
            } else {
                "payload/nested/personal.txt"
            },
            destination,
        );
        item.is_folder = folder;
        assert!(
            apply_plan(root.path(), &[item], dest.path()).is_err(),
            "{link_path}"
        );
        assert_eq!(
            fs::read(outside.path().join("personal.txt")).unwrap(),
            b"precious"
        );
        assert!(!outside.path().join("nested").exists());
    }
}

#[test]
fn fomod_copy_refuses_sources_outside_archive() {
    use std::os::unix::fs::symlink;
    for folder in [false, true] {
        let root = TempDir::new("fomod-source-link");
        let dest = TempDir::new("fomod-source-link-dest");
        let outside = TempDir::new("fomod-source-link-outside");
        write_at(outside.path(), "personal.txt", b"private");
        symlink(outside.path(), root.path().join("escape")).unwrap();
        let mut item = file_item(
            if folder {
                "escape"
            } else {
                "escape/personal.txt"
            },
            "copied",
        );
        item.is_folder = folder;
        assert!(apply_plan(root.path(), &[item], dest.path()).is_err());
        assert!(!dest.path().join("copied").exists());
    }
}

#[test]
fn fomod_folder_copy_allows_internal_links_but_refuses_cycles() {
    let root = TempDir::new("fomod-source-cycle");
    let dest = TempDir::new("fomod-source-cycle-dest");
    write_at(root.path(), "payload/personal.txt", b"included");
    std::os::unix::fs::symlink("payload", root.path().join("alias")).unwrap();
    let mut item = file_item("alias", "copied");
    item.is_folder = true;
    apply_plan(root.path(), &[item.clone()], dest.path()).unwrap();
    assert_eq!(
        fs::read(dest.path().join("copied/personal.txt")).unwrap(),
        b"included"
    );
    std::os::unix::fs::symlink(".", root.path().join("payload/cycle")).unwrap();
    assert!(apply_plan(root.path(), &[item], dest.path()).is_err());
}

#[test]
fn merge_metadata_failure_keeps_incomplete_checkpoint_after_payload() {
    let mods = TempDir::new("merge-metadata-failure");
    let archive = mods.path().join("Update.7z");
    fs::create_dir(archive.with_extension("7z.meta")).unwrap();
    let result = install_destination_inner(
        &archive,
        mods.path(),
        "Mod",
        "skyrimse",
        OverwritePolicy::Merge,
        false,
        |target, _| {
            fs::write(target.join("changed.esp"), b"partial")?;
            fs::write(
                target.join("meta.ini"),
                b"[General]\neidosCollectionOwner=forged\n",
            )?;
            Ok((String::new(), false, Vec::new()))
        },
    );
    assert!(result.is_err());
    let meta = ModMeta::read(&mods.path().join("Mod/meta.ini"));
    assert_ne!(meta.collection_owner(), Some("forged"));
    assert!(meta
        .install_warning()
        .unwrap()
        .contains("Merge did not complete"));
}

#[test]
fn root_split_merge_refuses_nested_destination_symlink() {
    let fixture = TempDir::new("root-merge-link");
    let dest = fixture.path().join("Mod");
    let outside = fixture.path().join("outside");
    write_at(&outside, "Binaries/personal.txt", b"precious");
    write_at(fixture.path(), "source/personal.txt", b"incoming");
    fs::create_dir_all(dest.join("Root")).unwrap();
    std::os::unix::fs::symlink(&outside, dest.join("Root/SB")).unwrap();
    let sources = RootSources {
        data: None,
        data_extra: Vec::new(),
        root: vec![("SB/Binaries".into(), fixture.path().join("source"))],
    };
    assert!(place_root_split(&sources, &dest, true).is_err());
    assert_eq!(
        fs::read(outside.join("Binaries/personal.txt")).unwrap(),
        b"precious"
    );
}

#[test]
fn manual_and_root_selection_refuse_intermediate_source_escape() {
    let root = TempDir::new("selection-source-link");
    let outside = TempDir::new("selection-source-outside");
    write_at(outside.path(), "Data/personal.txt", b"precious");
    std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
    assert!(resolve_manual_root(root.path(), "escape/Data").is_err());
    let split = crate::RootSplit {
        wrapper_prefix: String::new(),
        data_prefix: Some("escape/Data".into()),
        root_dir: None,
        root_entries: vec![],
    };
    assert!(resolve_root_split(root.path(), &split).is_err());
    assert!(outside.path().join("Data/personal.txt").exists());
}

#[test]
fn merge_updates_the_case_insensitive_winner_for_plain_and_fomod_content() {
    for fomod in [false, true] {
        let fixture = TempDir::new("merge-ci-winner");
        let source = fixture.path().join("source");
        let dest = fixture.path().join("Mod");
        write_at(&source, "textures/a.dds", b"new");
        write_at(&dest, "Textures/A.dds", b"old");
        if fomod {
            apply_plan(
                &source,
                &[file_item("textures/a.dds", "textures/a.dds")],
                &dest,
            )
            .unwrap();
        } else {
            overlay_dir(&source, &dest).unwrap();
        }
        let stack =
            eidos_core::LayerStack::new(vec![dest.clone()], fixture.path().join("Overwrite"));
        assert_eq!(
            fs::read(stack.resolve_read("textures/a.dds").unwrap()).unwrap(),
            b"new"
        );
        assert!(!dest.join("textures").exists());
    }
}

#[test]
fn retained_backups_cover_every_installer_route_and_preserve_the_old_tree() {
    use std::os::unix::fs::MetadataExt;
    for flow in ["simple", "manual", "bain", "fomod", "default-fomod", "root"] {
        for policy in [
            OverwritePolicy::ReplaceWithBackup,
            OverwritePolicy::MergeWithBackup,
            OverwritePolicy::Merge,
        ] {
            let t = TempDir::new("retained-routes");
            let mods = t.path().join("mods");
            let src = t.path().join("source");
            let archive = t.path().join("Update.7z");
            let old = mods.join("Old");
            write_at(&old, "precious.esp", b"original");
            write_at(&old, ".hidden/state", b"hidden");
            write_at(
                &old,
                "meta.ini",
                b"[General]\nnotes=precious\n[Unknown]\nraw=keep\n",
            );
            let old_inode = fs::metadata(&old).unwrap().ino();
            let old_meta = fs::read(old.join("meta.ini")).unwrap();
            let prefix = match flow {
                "bain" => "00 Core/",
                "root" => "Data/",
                _ => "",
            };
            write_at(&src, &format!("{prefix}new.esp"), b"replacement");
            if flow == "bain" {
                write_at(&src, "10 Optional/omitted.esp", b"not selected");
            }
            if flow == "root" {
                write_at(&src, "d3dx9_42.dll", b"root payload");
            }
            if flow.contains("fomod") {
                write_at(&src, "omitted.esp", b"not selected");
                write_at(&src, "fomod/ModuleConfig.xml", b"<config><moduleName>Test</moduleName><requiredInstallFiles><file source=\"new.esp\" destination=\"new.esp\"/></requiredInstallFiles></config>");
            }
            let tree = extracted(&src);
            let ctx = eidos_fomod::Context::default();
            let result = match flow {
                "manual" => install_manual(
                    &tree,
                    "",
                    &archive,
                    &mods,
                    "old",
                    "skyrimse",
                    policy.clone(),
                ),
                "bain" => install_bain(
                    &tree,
                    &["00 Core".into()],
                    &archive,
                    &mods,
                    "old",
                    "skyrimse",
                    policy.clone(),
                ),
                "fomod" => {
                    let config = parse_fomod_at(&src).unwrap();
                    let selection = eidos_fomod::default_selection(&config, &ctx);
                    finish_fomod(
                        FomodSession {
                            config,
                            root: src,
                            tree,
                            name: "old".into(),
                            archive,
                        },
                        &selection,
                        &mods,
                        "skyrimse",
                        &ctx,
                        policy.clone(),
                    )
                }
                _ => install_extracted(
                    &tree,
                    &archive,
                    &mods,
                    "old",
                    "skyrimse",
                    policy.clone(),
                    &ctx,
                ),
            }
            .unwrap();
            if let Some(backup) = result.backup {
                assert_ne!(policy, OverwritePolicy::Merge);
                assert_eq!(backup, mods.join("Old_backup"), "{flow}");
                assert_eq!(fs::read(backup.join("precious.esp")).unwrap(), b"original");
                assert_eq!(fs::read(backup.join(".hidden/state")).unwrap(), b"hidden");
                assert_eq!(fs::read(backup.join("meta.ini")).unwrap(), old_meta);
                assert!(!backup.join("new.esp").exists());
                if policy == OverwritePolicy::ReplaceWithBackup {
                    assert_eq!(fs::metadata(backup).unwrap().ino(), old_inode);
                }
            } else {
                assert_eq!(policy, OverwritePolicy::Merge);
                assert!(!mods.join("Old_backup").exists());
            }
            assert_eq!(
                fs::read(result.dest.join("new.esp")).unwrap(),
                b"replacement"
            );
            assert!(!result.dest.join("omitted.esp").exists());
        }
    }
}

#[test]
fn failed_merge_backup_prevents_even_the_metadata_checkpoint() {
    let t = TempDir::new("merge-backup-failed");
    let old = t.path().join("Old");
    write_at(&old, "keep.esp", b"precious");
    write_at(
        &old,
        "meta.ini",
        b"[General]\neidosCollectionOwner=original\n",
    );
    let original = fs::read(old.join("meta.ini")).unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(old.join("unsupported.socket")).unwrap();
    // Inject at the live publication seam, after native staging validation.
    let result = install_destination_ready(
        Path::new("Update.7z"),
        t.path(),
        "Old",
        "skyrimse",
        OverwritePolicy::MergeWithBackup,
        &[],
        |_, _| panic!("backup failure must prevent placement"),
    );
    assert!(result.is_err());
    assert_eq!(fs::read(old.join("meta.ini")).unwrap(), original);
    assert_eq!(fs::read(old.join("keep.esp")).unwrap(), b"precious");
    assert!(!t.path().join("Old_backup").exists());
}

#[test]
fn failed_merge_retains_and_reports_backup_with_incomplete_warning() {
    let t = TempDir::new("merge-backup-partial");
    let old = t.path().join("Old");
    write_at(&old, "keep.esp", b"precious");
    // Inject at the live publication seam, after native staging validation.
    let error = install_destination_ready(
        Path::new("Update.7z"),
        t.path(),
        "Old",
        "skyrimse",
        OverwritePolicy::MergeWithBackup,
        &[],
        |target, _| {
            fs::write(target.join("keep.esp"), b"partial")?;
            Err(InstallError::BadSelection(
                "injected partial overlay".into(),
            ))
        },
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains(&t.path().join("Old_backup").display().to_string()));
    assert_eq!(
        fs::read(t.path().join("Old_backup/keep.esp")).unwrap(),
        b"precious"
    );
    assert_eq!(fs::read(old.join("keep.esp")).unwrap(), b"partial");
    assert!(ModMeta::read(&old.join("meta.ini"))
        .install_warning()
        .unwrap()
        .contains("Merge did not complete"));
}

#[test]
fn default_overwrite_policies_do_not_retain_backups() {
    for policy in [
        OverwritePolicy::Replace,
        OverwritePolicy::Merge,
        OverwritePolicy::Rename("New".into()),
    ] {
        let t = TempDir::new("no-retained-backup");
        write_at(&t.path().join("Old"), "keep.esp", b"precious");
        let src = t.path().join("source");
        write_at(&src, "new.esp", b"new");
        let report = install_extracted(
            &extracted(&src),
            Path::new("Update.7z"),
            t.path(),
            "Old",
            "skyrimse",
            policy,
            &Default::default(),
        )
        .unwrap();
        assert!(report.backup.is_none());
        assert!(!t.path().join("Old_backup").exists());
    }
}

#[test]
fn extracted_finish_runs_in_staging_and_its_failure_prevents_replacement() {
    for fail in [false, true] {
        let t = TempDir::new("extracted-finish");
        let src = t.path().join("source");
        let mods = t.path().join("mods");
        let old = mods.join("Old");
        write_at(&old, "keep.esp", b"precious");
        write_at(&src, "new.esp", b"unpatched");
        let result = install_extracted_with_finish(
            &extracted(&src),
            Path::new("Update.7z"),
            &mods,
            "Old",
            "skyrimse",
            OverwritePolicy::ReplaceWithBackup,
            &Default::default(),
            |target| {
                assert_ne!(target, old);
                assert_eq!(fs::read(old.join("keep.esp")).unwrap(), b"precious");
                fs::write(target.join("new.esp"), b"patched")?;
                if fail {
                    Err(InstallError::BadSelection(
                        "injected transform failure".into(),
                    ))
                } else {
                    Ok(())
                }
            },
        );
        if fail {
            assert!(result.is_err());
            assert_eq!(fs::read(old.join("keep.esp")).unwrap(), b"precious");
            assert!(!old.join("new.esp").exists());
            assert!(!mods.join("Old_backup").exists());
        } else {
            assert_eq!(
                fs::read(result.unwrap().dest.join("new.esp")).unwrap(),
                b"patched"
            );
        }
    }
}

fn assert_transform_merge_rejected(flow: &str, policy: OverwritePolicy) {
    for game in ["skyrimse", "stardewvalley", "7daystodie"] {
        for existing in [true, false] {
            let t = TempDir::new("transform-merge-rejected");
            let src = t.path().join("source");
            let mods = t.path().join("mods");
            let old = mods.join("Old");
            fs::create_dir(&mods).unwrap();
            let original_meta = b"; retain raw formatting\r\n[General]\r\neidosCollectionOwner=original\r\nnotes=precious\r\n[Unknown]\r\nraw=keep\r\n";
            if existing {
                write_at(&old, "precious.esp", b"original");
                write_at(&old, ".hidden/state", b"hidden");
                write_at(&old, "meta.ini", original_meta);
                set_mtime(&old, "precious.esp", 100);
                set_mtime(&old, "meta.ini", 100);
            }
            let paths_before = rel_paths(&mods);
            let metadata_before: Vec<_> = paths_before
                .iter()
                .map(|path| {
                    let meta = fs::metadata(mods.join(path)).unwrap();
                    (meta.permissions(), meta.modified().unwrap())
                })
                .collect();
            write_at(&src, "new.esp", b"new");
            if flow == "fomod" {
                write_at(&src, "fomod/ModuleConfig.xml", br#"<config><moduleName>Test</moduleName><requiredInstallFiles><file source="new.esp" destination="new.esp"/></requiredInstallFiles></config>"#);
            }
            let tree = extracted(&src);
            let mut called = false;
            let mut live_seen = false;
            let finish = |target: &Path| {
                called = true;
                live_seen = target == old;
                fs::write(target.join("precious.esp"), b"failed transform")?;
                Err(InstallError::BadSelection(
                    "transform rejected payload".into(),
                ))
            };
            let result = match flow {
                "extracted" => install_extracted_with_finish(
                    &tree,
                    Path::new("Update.7z"),
                    &mods,
                    "old",
                    game,
                    policy.clone(),
                    &Default::default(),
                    finish,
                ),
                "fomod" => finish_fomod_with_finish(
                    FomodSession {
                        config: parse_fomod_at(&src).unwrap(),
                        root: src,
                        tree,
                        name: "old".into(),
                        archive: "Update.7z".into(),
                    },
                    &Vec::new(),
                    &mods,
                    game,
                    &Default::default(),
                    policy.clone(),
                    finish,
                ),
                "destination" => install_destination(
                    Path::new("Update.7z"),
                    &mods,
                    "old",
                    game,
                    policy.clone(),
                    |target, _| {
                        let mut finish = finish;
                        finish(target)?;
                        Ok((String::new(), false, Vec::new()))
                    },
                ),
                _ => unreachable!(),
            };
            assert!(
                !live_seen,
                "{flow} {policy:?}: callback received the live directory"
            );
            assert!(!called, "Merge must be rejected before callback activity");
            assert!(
                matches!(result, Err(InstallError::BadSelection(_))),
                "{result:?}"
            );
            assert_eq!(
                rel_paths(&mods),
                paths_before,
                "no payload, backup, or stage may be created"
            );
            for (path, (permissions, modified)) in paths_before.iter().zip(metadata_before) {
                let after = fs::metadata(mods.join(path)).unwrap();
                assert_eq!(after.permissions(), permissions, "{path}");
                assert_eq!(after.modified().unwrap(), modified, "{path}");
            }
            if existing {
                assert_eq!(fs::read(old.join("precious.esp")).unwrap(), b"original");
                assert_eq!(fs::read(old.join(".hidden/state")).unwrap(), b"hidden");
                assert_eq!(fs::read(old.join("meta.ini")).unwrap(), original_meta);
            }
        }
    }
}

#[test]
fn extracted_transform_rejects_merge() {
    assert_transform_merge_rejected("extracted", OverwritePolicy::Merge);
}

#[test]
fn extracted_transform_rejects_merge_with_backup() {
    assert_transform_merge_rejected("extracted", OverwritePolicy::MergeWithBackup);
}

#[test]
fn fomod_transform_rejects_merge() {
    assert_transform_merge_rejected("fomod", OverwritePolicy::Merge);
}

#[test]
fn fomod_transform_rejects_merge_with_backup() {
    assert_transform_merge_rejected("fomod", OverwritePolicy::MergeWithBackup);
}

#[test]
fn destination_transform_rejects_merge() {
    assert_transform_merge_rejected("destination", OverwritePolicy::Merge);
}

#[test]
fn destination_transform_rejects_merge_with_backup() {
    assert_transform_merge_rejected("destination", OverwritePolicy::MergeWithBackup);
}

#[test]
fn failed_backup_reservation_prevents_replace_publication() {
    let t = TempDir::new("backup-reservation-failed");
    // The original name fits the filesystem; appending `_backup` cannot fit.
    let name = "a".repeat(250);
    let old = t.path().join(&name);
    write_at(&old, "keep.esp", b"precious");
    let result = install_destination(
        Path::new("Update.7z"),
        t.path(),
        &name,
        "skyrimse",
        OverwritePolicy::ReplaceWithBackup,
        |target, _| {
            fs::write(target.join("new.esp"), b"new")?;
            Ok((String::new(), false, Vec::new()))
        },
    );
    assert!(result.is_err());
    assert_eq!(fs::read(old.join("keep.esp")).unwrap(), b"precious");
    assert!(!old.join("new.esp").exists());
    assert_eq!(fs::read_dir(t.path()).unwrap().count(), 1);
}

#[test]
fn owned_backup_policy_requires_the_exact_reservation_and_preserves_it() {
    let t = TempDir::new("owned-backup");
    let old = t.path().join("Owned");
    write_at(
        &old,
        "meta.ini",
        b"[General]\neidosCollectionOwner=collection:member\n",
    );
    write_at(&old, "keep.esp", b"precious");
    assert!(install_destination(
        Path::new("Update.7z"),
        t.path(),
        "Owned",
        "skyrimse",
        OverwritePolicy::ReplaceOwnedWithBackup("changed-owner".into()),
        |_, _| panic!("invalid ownership must prevent placement")
    )
    .is_err());
    assert!(!t.path().join("Owned_backup").exists());
    let report = install_destination(
        Path::new("Update.7z"),
        t.path(),
        "Owned",
        "skyrimse",
        OverwritePolicy::ReplaceOwnedWithBackup("collection:member".into()),
        |target, _| {
            fs::write(target.join("new.esp"), b"new")?;
            Ok((String::new(), false, Vec::new()))
        },
    )
    .unwrap();
    assert_eq!(report.backup, Some(t.path().join("Owned_backup")));
    assert_eq!(
        ModMeta::read(&report.dest.join("meta.ini")).collection_owner(),
        Some("collection:member")
    );
}

#[test]
fn fomod_finish_preserves_exact_answers_and_aborts_on_transform_failure() {
    for fail in [false, true] {
        let t = TempDir::new("fomod-finish");
        let src = t.path().join("source");
        let mods = t.path().join("mods");
        let old = mods.join("Old");
        write_at(&old, "keep.esp", b"precious");
        write_at(&src, "default.esp", b"default");
        write_at(&src, "chosen.esp", b"selected");
        write_at(&src, "fomod/ModuleConfig.xml", br#"<config><moduleName>Test</moduleName><installSteps order="Explicit"><installStep name="Options"><optionalFileGroups order="Explicit"><group name="Choice" type="SelectExactlyOne"><plugins order="Explicit"><plugin name="Default"><files><file source="default.esp" destination="default.esp"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin><plugin name="Chosen"><files><file source="chosen.esp" destination="chosen.esp"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin></plugins></group></optionalFileGroups></installStep></installSteps></config>"#);
        let config = parse_fomod_at(&src).unwrap();
        let session = FomodSession {
            config,
            root: src.clone(),
            tree: extracted(&src),
            name: "Old".into(),
            archive: t.path().join("Update.7z"),
        };
        let result = finish_fomod_with_finish(
            session,
            &vec![vec![vec![false, true]]],
            &mods,
            "skyrimse",
            &Default::default(),
            OverwritePolicy::ReplaceWithBackup,
            |target| {
                assert_ne!(target, old);
                assert!(!target.join("default.esp").exists());
                assert_eq!(fs::read(target.join("chosen.esp")).unwrap(), b"selected");
                assert_eq!(fs::read(old.join("keep.esp")).unwrap(), b"precious");
                fs::write(target.join("chosen.esp"), b"patched selection")?;
                if fail {
                    Err(InstallError::BadSelection("injected recipe failure".into()))
                } else {
                    Ok(())
                }
            },
        );
        if fail {
            assert!(result.is_err());
            assert_eq!(fs::read(old.join("keep.esp")).unwrap(), b"precious");
            assert!(!old.join("chosen.esp").exists());
        } else {
            let report = result.unwrap();
            assert_eq!(
                fs::read(report.dest.join("chosen.esp")).unwrap(),
                b"patched selection"
            );
            assert!(!report.dest.join("default.esp").exists());
            assert_eq!(
                fs::read(report.backup.unwrap().join("keep.esp")).unwrap(),
                b"precious"
            );
        }
    }
}

#[test]
fn folder_mods_preserve_units_across_simple_bain_manual_and_fomod() {
    for (game, marker) in [
        ("stardewvalley", "manifest.json"),
        ("7daystodie", "ModInfo.xml"),
    ] {
        for flow in [
            "simple",
            "bain",
            "manual-parent",
            "manual-unit",
            "fomod",
            "fomod-default",
        ] {
            let (t, mods, src) = bain_layout("folder-units");
            let prefix = if flow == "bain" {
                "00 Core/"
            } else {
                "Download-123/"
            };
            write_at(&src, &format!("{prefix}ContentPatcher/{marker}"), b"marker");
            write_at(
                &src,
                &format!("{prefix}ContentPatcher/content.json"),
                b"content",
            );
            let archive = t.path().join("Pack.7z");
            let tree = extracted(&src);
            if flow.starts_with("fomod") {
                write_at(&src, "fomod/ModuleConfig.xml", br#"<config><moduleName>Test</moduleName><requiredInstallFiles><folder source="Download-123/ContentPatcher" destination=""/></requiredInstallFiles></config>"#);
            }
            let result = match flow {
                "bain" => install_bain(
                    &tree,
                    &["00 Core".into()],
                    &archive,
                    &mods,
                    "Pack",
                    game,
                    OverwritePolicy::Fail,
                ),
                "manual-parent" => install_manual(
                    &tree,
                    "Download-123",
                    &archive,
                    &mods,
                    "Pack",
                    game,
                    OverwritePolicy::Fail,
                ),
                "manual-unit" => install_manual(
                    &tree,
                    "download-123/contentpatcher",
                    &archive,
                    &mods,
                    "Pack",
                    game,
                    OverwritePolicy::Fail,
                ),
                "fomod" => finish_fomod(
                    FomodSession {
                        config: parse_fomod_at(&src).unwrap(),
                        root: src.clone(),
                        tree,
                        name: "Pack".into(),
                        archive,
                    },
                    &Vec::new(),
                    &mods,
                    game,
                    &Default::default(),
                    OverwritePolicy::Fail,
                ),
                _ => install_extracted(
                    &tree,
                    &archive,
                    &mods,
                    "Pack",
                    game,
                    OverwritePolicy::Fail,
                    &Default::default(),
                ),
            };
            let report = result.unwrap_or_else(|e| panic!("{game} {flow}: {e}"));
            assert_eq!(
                fs::read(report.dest.join(format!("ContentPatcher/{marker}"))).unwrap(),
                b"marker",
                "{game} {flow}"
            );
            let mut expected = vec![
                "ContentPatcher/".to_string(),
                format!("ContentPatcher/{marker}"),
                "ContentPatcher/content.json".into(),
                "meta.ini".into(),
            ];
            expected.sort();
            assert_eq!(rel_paths(&report.dest), expected, "{game} {flow}");
        }
    }
}

#[test]
fn folder_mods_reject_bare_markers_before_live_merge_changes() {
    for game in ["stardewvalley", "7daystodie"] {
        let marker = if game == "stardewvalley" {
            "manifest.json"
        } else {
            "ModInfo.xml"
        };
        for flow in ["simple", "manual", "fomod"] {
            for policy in [
                OverwritePolicy::Merge,
                OverwritePolicy::MergeWithBackup,
                OverwritePolicy::ReplaceWithBackup,
            ] {
                let (t, mods, src) = bain_layout("folder-bare");
                write_at(&mods, "Pack/Existing/state", b"precious");
                write_at(&mods, "Pack/meta.ini", b"[General]\nnotes=original\n");
                write_at(&src, marker, b"marker");
                if flow == "fomod" {
                    write_at(&src, "fomod/ModuleConfig.xml", format!("<config><moduleName>Test</moduleName><requiredInstallFiles><file source=\"{marker}\" destination=\"{marker}\"/></requiredInstallFiles></config>").as_bytes());
                }
                let tree = extracted(&src);
                let archive = t.path().join("Pack.7z");
                let result = match flow {
                    "manual" => install_manual(&tree, "", &archive, &mods, "Pack", game, policy),
                    "fomod" => finish_fomod(
                        FomodSession {
                            config: parse_fomod_at(&src).unwrap(),
                            root: src.clone(),
                            tree,
                            name: "Pack".into(),
                            archive,
                        },
                        &Vec::new(),
                        &mods,
                        game,
                        &Default::default(),
                        policy,
                    ),
                    _ => install_extracted(
                        &tree,
                        &archive,
                        &mods,
                        "Pack",
                        game,
                        policy,
                        &Default::default(),
                    ),
                };
                assert!(result.is_err(), "{game} {flow} must reject a bare marker");
                assert_eq!(
                    fs::read(mods.join("Pack/meta.ini")).unwrap(),
                    b"[General]\nnotes=original\n"
                );
                assert_eq!(
                    fs::read(mods.join("Pack/Existing/state")).unwrap(),
                    b"precious"
                );
                assert_eq!(
                    rel_paths(&mods.join("Pack")),
                    ["Existing/", "Existing/state", "meta.ini"]
                );
                assert!(!mods.join("Pack_backup").exists());
            }
        }
    }
}

#[test]
fn folder_mods_check_links_traversal_and_case_before_publication() {
    for bad in ["dangling", "escape", "cycle", "traversal"] {
        for flow in ["simple", "bain", "manual", "fomod"] {
            let (t, mods, src) = bain_layout("folder-safety");
            write_at(&mods, "Pack/ContentPatcher/manifest.json", b"old marker");
            write_at(&mods, "Pack/meta.ini", b"[General]\nnotes=original\n");
            write_at(t.path(), "outside", b"precious");
            write_at(&src, "00 Core/ContentPatcher/manifest.json", b"new marker");
            match bad {
                "dangling" => {
                    std::os::unix::fs::symlink("missing", src.join("00 Core/ContentPatcher/link"))
                        .unwrap()
                }
                "escape" => std::os::unix::fs::symlink(
                    t.path().join("outside"),
                    src.join("00 Core/ContentPatcher/link"),
                )
                .unwrap(),
                "cycle" => std::os::unix::fs::symlink(".", src.join("00 Core/ContentPatcher/link"))
                    .unwrap(),
                _ => {}
            }
            if flow == "fomod" {
                let destination = if bad == "traversal" {
                    "../../outside"
                } else {
                    ""
                };
                write_at(&src, "fomod/ModuleConfig.xml", format!("<config><moduleName>Test</moduleName><requiredInstallFiles><folder source=\"00 Core/ContentPatcher\" destination=\"{destination}\"/></requiredInstallFiles></config>").as_bytes());
            }
            let tree = extracted(&src);
            let archive = t.path().join("Pack.7z");
            let result = match flow {
                "bain" => install_bain(
                    &tree,
                    &[if bad == "traversal" {
                        "../outside"
                    } else {
                        "00 Core"
                    }
                    .into()],
                    &archive,
                    &mods,
                    "Pack",
                    "stardewvalley",
                    OverwritePolicy::MergeWithBackup,
                ),
                "manual" => install_manual(
                    &tree,
                    if bad == "traversal" {
                        "../outside"
                    } else {
                        "00 Core/ContentPatcher"
                    },
                    &archive,
                    &mods,
                    "Pack",
                    "stardewvalley",
                    OverwritePolicy::MergeWithBackup,
                ),
                "fomod" => finish_fomod(
                    FomodSession {
                        config: parse_fomod_at(&src).unwrap(),
                        root: src.clone(),
                        tree,
                        name: "Pack".into(),
                        archive,
                    },
                    &Vec::new(),
                    &mods,
                    "stardewvalley",
                    &Default::default(),
                    OverwritePolicy::MergeWithBackup,
                ),
                _ if bad == "traversal" => continue,
                _ => install_extracted(
                    &tree,
                    &archive,
                    &mods,
                    "Pack",
                    "stardewvalley",
                    OverwritePolicy::MergeWithBackup,
                    &Default::default(),
                ),
            };
            assert!(result.is_err(), "{bad} {flow}");
            assert_eq!(
                fs::read(mods.join("Pack/meta.ini")).unwrap(),
                b"[General]\nnotes=original\n"
            );
            assert_eq!(
                fs::read(mods.join("Pack/ContentPatcher/manifest.json")).unwrap(),
                b"old marker"
            );
            assert_eq!(fs::read(t.path().join("outside")).unwrap(), b"precious");
            assert!(!mods.join("Pack_backup").exists());
        }
    }

    let (t, mods, src) = bain_layout("folder-case");
    write_at(&mods, "Pack/ContentPatcher/manifest.json", b"old marker");
    write_at(&mods, "Pack/ContentPatcher/Config.json", b"old config");
    write_at(&src, "contentpatcher/MANIFEST.JSON", b"new marker");
    write_at(&src, "contentpatcher/config.JSON", b"new config");
    write_at(&src, "SecondMod/manifest.json", b"second marker");
    let report = install_extracted(
        &extracted(&src),
        &t.path().join("Pack.7z"),
        &mods,
        "pack",
        "stardewvalley",
        OverwritePolicy::MergeWithBackup,
        &Default::default(),
    )
    .unwrap();
    assert_eq!(report.dest, mods.join("Pack"));
    assert_eq!(
        fs::read(report.dest.join("ContentPatcher/manifest.json")).unwrap(),
        b"new marker"
    );
    assert_eq!(
        fs::read(report.dest.join("ContentPatcher/Config.json")).unwrap(),
        b"new config"
    );
    assert!(report.dest.join("SecondMod/manifest.json").is_file());
    assert!(!report.dest.join("contentpatcher").exists());
    let backup = report.backup.unwrap();
    assert_eq!(
        fs::read(backup.join("ContentPatcher/manifest.json")).unwrap(),
        b"old marker"
    );
    assert_eq!(
        fs::read(backup.join("ContentPatcher/Config.json")).unwrap(),
        b"old config"
    );
}

#[test]
fn folder_mods_open_real_archives_and_install_selected_units() {
    let Some(bin) = eidos_sevenzip::find_7z() else {
        eprintln!("7-Zip unavailable: folder archive dispatch fixture skipped");
        return;
    };
    for (game, marker) in [
        ("stardewvalley", "manifest.json"),
        ("7daystodie", "ModInfo.xml"),
    ] {
        for flow in ["simple", "bain", "fomod"] {
            let t = TempDir::new("folder-archive");
            let src = t.path().join("source");
            let prefix = if flow == "bain" { "00 Core" } else { "Mods" };
            write_at(&src, &format!("{prefix}/ContentPatcher/{marker}"), b"core");
            if flow == "bain" {
                write_at(
                    &src,
                    &format!("01 Optional/OptionalUnit/{marker}"),
                    b"optional",
                );
            } else if flow == "fomod" {
                write_at(&src, "fomod/ModuleConfig.xml", br#"<config><moduleName>Test</moduleName><requiredInstallFiles><folder source="Mods/ContentPatcher" destination="ContentPatcher"/></requiredInstallFiles></config>"#);
            } else {
                write_at(&src, "README.txt", b"documentation");
            }
            let archive = t.path().join("Pack.zip");
            let output = std::process::Command::new(bin)
                .args(["a", "-tzip"])
                .arg(&archive)
                .arg(".")
                .current_dir(&src)
                .output()
                .unwrap();
            assert!(output.status.success());
            let before = fs::read(&archive).unwrap();
            let mods = t.path().join("mods");
            let opened = open_archive(&archive, &mods, "Pack", game).unwrap();
            let report = match opened {
                Opened::Simple(tree) if flow == "simple" => install_extracted(
                    &tree,
                    &archive,
                    &mods,
                    "Pack",
                    game,
                    OverwritePolicy::Fail,
                    &Default::default(),
                ),
                Opened::Bain {
                    tree, subpackages, ..
                } if flow == "bain" => {
                    assert_eq!(subpackages, ["00 Core", "01 Optional"]);
                    install_bain(
                        &tree,
                        &subpackages,
                        &archive,
                        &mods,
                        "Pack",
                        game,
                        OverwritePolicy::Fail,
                    )
                }
                Opened::Fomod(session) if flow == "fomod" => finish_fomod(
                    *session,
                    &Vec::new(),
                    &mods,
                    game,
                    &Default::default(),
                    OverwritePolicy::Fail,
                ),
                _ => panic!("wrong classifier for {game} {flow}"),
            }
            .unwrap();
            assert_eq!(
                fs::read(report.dest.join(format!("ContentPatcher/{marker}"))).unwrap(),
                b"core"
            );
            assert!(!report.dest.join(marker).exists());
            assert!(!report.dest.join("Mods").exists());
            if flow == "bain" {
                assert_eq!(
                    fs::read(report.dest.join(format!("OptionalUnit/{marker}"))).unwrap(),
                    b"optional"
                );
            }
            assert_eq!(fs::read(&archive).unwrap(), before);
        }
    }
}

pub(super) fn omod_string(out: &mut Vec<u8>, value: &str) {
    let mut len = value.len();
    while len >= 128 {
        out.push((len as u8) | 128);
        len >>= 7;
    }
    out.push(len as u8);
    out.extend_from_slice(value.as_bytes());
}

/// A real ZIP with stored regular entries; no external fixture assets.
pub(super) fn omod_zip(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut directory = Vec::new();
    for (name, content) in entries {
        let offset = bytes.len() as u32;
        let crc = crc32fast::hash(content);
        bytes.extend_from_slice(&0x04034b50u32.to_le_bytes());
        bytes.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        bytes.extend_from_slice(&crc.to_le_bytes());
        bytes.extend_from_slice(&(content.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(content.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(content);
        directory.extend_from_slice(&0x02014b50u32.to_le_bytes());
        directory.extend_from_slice(&[20, 3, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&crc.to_le_bytes());
        directory.extend_from_slice(&(content.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(content.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
        directory.extend_from_slice(&[0; 8]);
        directory.extend_from_slice(&(0o100644u32 << 16).to_le_bytes());
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name.as_bytes());
    }
    let offset = bytes.len() as u32;
    let size = directory.len() as u32;
    bytes.extend(directory);
    bytes.extend_from_slice(&0x06054b50u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}

pub(super) fn omod_config(version: u8, compression: u8) -> Vec<u8> {
    let mut config = vec![version];
    omod_string(&mut config, "Fixture Mod");
    config.extend_from_slice(&1i32.to_le_bytes());
    config.extend_from_slice(&2i32.to_le_bytes());
    for s in [
        "Fixture Author",
        "author@example.test",
        "https://example.test/mod",
        "Synthetic description",
    ] {
        omod_string(&mut config, s);
    }
    if version >= 2 {
        config.extend_from_slice(&638000000000000000i64.to_le_bytes());
    } else {
        omod_string(&mut config, "12/09/2006 12:30");
    }
    config.push(compression);
    if version >= 1 {
        config.extend_from_slice(&3i32.to_le_bytes());
    }
    config
}

pub(super) fn omod_crc(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut records = Vec::new();
    for (name, bytes) in files {
        omod_string(&mut records, name);
        records.extend_from_slice(&crc32fast::hash(bytes).to_le_bytes());
        records.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    }
    records
}

#[test]
fn omod_unscripted_data_and_plugins_install_as_payload_not_container_blobs() {
    if eidos_sevenzip::find_7z().is_none() {
        return;
    }
    let t = TempDir::new("omod-basic");
    let archive = t.path().join("Example.omod");
    let data = [
        ("Textures\\Demo.dds", b"texture".as_slice()),
        ("empty.txt", b"".as_slice()),
    ];
    let plugins = [("Demo.esp", b"plugin".as_slice())];
    let mut readme = Vec::new();
    omod_string(&mut readme, "Synthetic readme");
    fs::write(
        &archive,
        omod_zip(&[
            ("config", omod_config(3, 1)),
            ("readme", readme),
            ("data.crc", omod_crc(&data)),
            ("data", omod_zip(&[("a", b"texture".to_vec())])),
            ("plugins.crc", omod_crc(&plugins)),
            ("plugins", omod_zip(&[("a", b"plugin".to_vec())])),
        ]),
    )
    .unwrap();
    let report = install_archive(&archive, &t.path().join("mods"), "Pack", "oblivion").unwrap();
    assert_eq!(
        fs::read(report.dest.join("Textures/Demo.dds")).unwrap(),
        b"texture"
    );
    assert_eq!(fs::read(report.dest.join("Demo.esp")).unwrap(), b"plugin");
    assert_eq!(fs::read(report.dest.join("empty.txt")).unwrap(), b"");
    for blob in ["data", "plugins", "config", "data.crc", "plugins.crc"] {
        assert!(!report.dest.join(blob).exists());
    }
    let meta = ModMeta::read(&report.dest.join("meta.ini"));
    assert_eq!(meta.author().as_deref(), Some("Fixture Author"));
    assert_eq!(meta.version().as_deref(), Some("1.2.3"));
}

#[test]
fn omod_finish_is_private_and_merge_refused_before_writes() {
    let t = TempDir::new("omod-finish");
    let archive = t.path().join("pack.omod");
    fs::write(
        &archive,
        omod_zip(&[
            ("config", omod_config(4, 1)),
            ("data.crc", omod_crc(&[("Demo.esp", b"new")])),
            ("data", omod_zip(&[("a", b"new".to_vec())])),
        ]),
    )
    .unwrap();
    let session = open_omod_with(
        &archive,
        &t.path().join("work"),
        &std::sync::atomic::AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    let mods = t.path().join("mods");
    let live = mods.join("Pack");
    write_at(&live, "Demo.esp", b"old");
    write_at(&live, "meta.ini", b"[General]\nnotes=precious\n");
    let before = rel_paths(&mods);
    for policy in [
        OverwritePolicy::Merge,
        OverwritePolicy::MergeWithBackup,
        OverwritePolicy::Replace,
    ] {
        let mut called = false;
        let result = install_omod_with_finish(
            &session,
            &mods,
            "Pack",
            "oblivion",
            policy.clone(),
            |stage| {
                called = true;
                assert_ne!(stage, live);
                assert_eq!(fs::read(stage.join("Demo.esp")).unwrap(), b"new");
                fs::write(stage.join("Demo.esp"), b"broken")?;
                Err(InstallError::BadSelection(
                    "fixture transform failed".into(),
                ))
            },
        );
        assert!(result.is_err());
        assert_eq!(called, matches!(policy, OverwritePolicy::Replace));
        assert_eq!(fs::read(live.join("Demo.esp")).unwrap(), b"old");
        assert_eq!(
            fs::read(live.join("meta.ini")).unwrap(),
            b"[General]\nnotes=precious\n"
        );
        assert_eq!(rel_paths(&mods), before);
    }
    let report = install_omod_with_finish(
        &session,
        &mods,
        "Pack",
        "oblivion",
        OverwritePolicy::Replace,
        |stage| {
            fs::write(stage.join("receipt.txt"), b"complete")?;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        fs::read(report.dest.join("receipt.txt")).unwrap(),
        b"complete"
    );
    assert_eq!(fs::read(report.dest.join("Demo.esp")).unwrap(), b"new");
}

fn write_omod(
    root: &Path,
    name: &str,
    config: Vec<u8>,
    crc: Vec<u8>,
    data: Vec<u8>,
    script: Option<&str>,
) -> PathBuf {
    let path = root.join(name);
    let mut entries = vec![("config", config), ("data.crc", crc), ("data", data)];
    if let Some(source) = script {
        let mut bytes = Vec::new();
        omod_string(&mut bytes, source);
        entries.push(("script", bytes));
    }
    fs::write(&path, omod_zip(&entries)).unwrap();
    path
}

#[test]
fn omod_raw_lzma_config_versions_and_exact_member_boundaries() {
    let t = TempDir::new("omod-lzma");
    // Python lzma.FORMAT_ALONE, 1 MiB dictionary; omit its eight-byte size.
    let raw = vec![
        93, 0, 0, 16, 0, 0, 48, 152, 136, 152, 62, 209, 181, 112, 63, 255, 251, 115, 224, 0,
    ];
    for version in 0..=4 {
        let archive = write_omod(
            t.path(),
            &format!("v{version}.omod"),
            omod_config(version, 0),
            omod_crc(&[
                ("empty.txt", b""),
                ("Textures/a.dds", b"abc"),
                ("Demo.esp", b"de"),
                ("last.txt", b""),
            ]),
            raw.clone(),
            None,
        );
        let session = open_omod_with(
            &archive,
            &t.path().join("work"),
            &std::sync::atomic::AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
        assert_eq!(session.metadata.format_version, version);
        assert_eq!(
            session.metadata.version(),
            if version == 0 { "1.2" } else { "1.2.3" }
        );
        assert_eq!(session.members.len(), 4);
        assert_eq!(
            fs::read(session.data_root().join("Textures/a.dds")).unwrap(),
            b"abc"
        );
        assert_eq!(
            fs::read(session.data_root().join("Demo.esp")).unwrap(),
            b"de"
        );
        assert_eq!(fs::read(session.data_root().join("last.txt")).unwrap(), b"");
    }
}

#[test]
fn omod_scripts_classify_exact_bytes_and_never_default_install() {
    let t = TempDir::new("omod-scripts");
    for (i, source, kind, body) in [
        (
            0,
            "Message legacy\r\n",
            OmodScriptKind::Obmm,
            "Message legacy\r\n",
        ),
        (
            1,
            "\0Message typed\r\n",
            OmodScriptKind::Obmm,
            "Message typed\r\n",
        ),
        (
            2,
            "\x01print('hello')",
            OmodScriptKind::Python,
            "print('hello')",
        ),
        (3, "\x02// C#", OmodScriptKind::CSharp, "// C#"),
        (
            4,
            "\x03' Visual Basic",
            OmodScriptKind::VisualBasic,
            "' Visual Basic",
        ),
        (5, "", OmodScriptKind::Obmm, ""),
    ] {
        let archive = write_omod(
            t.path(),
            &format!("script{i}.zip"),
            omod_config(4, 1),
            omod_crc(&[("Demo.esp", b"abc")]),
            omod_zip(&[("a", b"abc".to_vec())]),
            Some(source),
        );
        let mods = t.path().join(format!("mods{i}"));
        let Opened::Omod(session) =
            open_archive(&archive, &t.path().join("work"), "Pack", "oblivion").unwrap()
        else {
            panic!("OMOD content in ZIP was not recognized")
        };
        let script = session.script.as_ref().unwrap();
        assert_eq!(script.kind, kind);
        assert_eq!(script.bytes, source.as_bytes());
        assert_eq!(script.source(), body);
        let result = install_omod(
            &session,
            &mods,
            "Pack",
            "oblivion",
            OverwritePolicy::Replace,
        );
        assert!(matches!(result, Err(InstallError::BadSelection(e)) if e.contains("script")));
        assert!(!mods.exists());
        assert!(install_archive(&archive, &mods, "Pack", "oblivion").is_err());
        assert!(fs::read_dir(&mods).unwrap().next().is_none());
    }
}

#[test]
fn omod_corruption_is_rejected_and_decoding_staging_is_removed() {
    let t = TempDir::new("omod-corruption");
    let crc = omod_crc(&[("Demo.esp", b"abc")]);
    let raw = vec![
        93, 0, 0, 16, 0, 0, 48, 152, 136, 164, 74, 142, 159, 255, 246, 99, 128, 0,
    ];
    let mut bad_crc = crc.clone();
    let n = bad_crc.len();
    bad_crc[n - 12] ^= 1;
    let mut negative = crc.clone();
    negative[n - 8..].copy_from_slice(&u64::MAX.to_le_bytes());
    let mut bad_props = raw.clone();
    bad_props[0] = 225;
    let mut big_dict = raw.clone();
    big_dict[1..5].copy_from_slice(&(128u32 * 1024 * 1024).to_le_bytes());
    let mut truncated_crc = crc.clone();
    truncated_crc.pop();
    let cases = vec![
        (
            "checksum",
            omod_config(4, 1),
            bad_crc,
            omod_zip(&[("a", b"abc".to_vec())]),
        ),
        (
            "negative",
            omod_config(4, 1),
            negative,
            omod_zip(&[("a", b"abc".to_vec())]),
        ),
        (
            "crc-truncated",
            omod_config(4, 1),
            truncated_crc,
            omod_zip(&[("a", b"abc".to_vec())]),
        ),
        (
            "too-long",
            omod_config(4, 1),
            crc.clone(),
            omod_zip(&[("a", b"abcd".to_vec())]),
        ),
        (
            "too-short",
            omod_config(4, 1),
            crc.clone(),
            omod_zip(&[("a", b"ab".to_vec())]),
        ),
        (
            "wrong-member",
            omod_config(4, 1),
            crc.clone(),
            omod_zip(&[("b", b"abc".to_vec())]),
        ),
        (
            "extra-member",
            omod_config(4, 1),
            crc.clone(),
            omod_zip(&[("a", b"abc".to_vec()), ("b", vec![])]),
        ),
        ("lzma-properties", omod_config(4, 0), crc.clone(), bad_props),
        ("lzma-dictionary", omod_config(4, 0), crc.clone(), big_dict),
        (
            "lzma-truncated",
            omod_config(4, 0),
            crc.clone(),
            raw[..9].to_vec(),
        ),
        (
            "lzma-too-long",
            omod_config(4, 0),
            omod_crc(&[("Demo.esp", b"ab")]),
            raw.clone(),
        ),
        (
            "lzma-too-short",
            omod_config(4, 0),
            omod_crc(&[("Demo.esp", b"abcd")]),
            raw,
        ),
        (
            "config-version",
            omod_config(5, 1),
            crc,
            omod_zip(&[("a", b"abc".to_vec())]),
        ),
    ];
    let mods = t.path().join("mods");
    write_at(&mods.join("Pack"), "precious.esp", b"keep");
    write_at(
        &mods.join("Pack"),
        "meta.ini",
        b"[General]\nnotes=original\n",
    );
    let before = rel_paths(&mods);
    for (name, config, crc, data) in cases {
        let archive = write_omod(t.path(), &format!("{name}.omod"), config, crc, data, None);
        let result = install_archive_with_policy(
            &archive,
            &mods,
            "Pack",
            "oblivion",
            OverwritePolicy::ReplaceWithBackup,
            &Default::default(),
        );
        assert!(result.is_err(), "accepted {name}");
        assert_eq!(rel_paths(&mods), before, "left staging behind for {name}");
        assert_eq!(fs::read(mods.join("Pack/precious.esp")).unwrap(), b"keep");
        assert_eq!(
            fs::read(mods.join("Pack/meta.ini")).unwrap(),
            b"[General]\nnotes=original\n"
        );
    }
}

#[test]
fn omod_rejects_unsafe_duplicate_and_cross_group_paths() {
    let t = TempDir::new("omod-paths");
    for (index, names) in [
        vec!["../escape.esp"],
        vec!["/absolute.esp"],
        vec!["C:\\drive.esp"],
        vec!["a/../escape"],
        vec![".eidos-collection.json"],
        vec!["meta.ini"],
        vec!["Meta.INI/child"],
        vec!["root/a.dll"],
        vec!["omod-readme.txt"],
        vec!["a//b"],
        vec!["a. /b"],
        vec!["a/./b"],
        vec!["a\nb.esp"],
        vec!["A.esp", "a.esp"],
        vec!["a", "a/b"],
        vec!["a/b", "a"],
    ]
    .into_iter()
    .enumerate()
    {
        let files: Vec<_> = names.iter().map(|s| (*s, b"".as_slice())).collect();
        let archive = write_omod(
            t.path(),
            &format!("bad{index}.omod"),
            omod_config(4, 1),
            omod_crc(&files),
            omod_zip(&[("a", vec![])]),
            None,
        );
        assert!(
            open_omod_with(
                &archive,
                &t.path().join("work"),
                &std::sync::atomic::AtomicBool::new(false),
                |_| {}
            )
            .is_err(),
            "accepted {names:?}"
        );
        assert!(fs::read_dir(t.path().join("work"))
            .unwrap()
            .next()
            .is_none());
    }
    let archive = t.path().join("cross.omod");
    fs::write(
        &archive,
        omod_zip(&[
            ("config", omod_config(4, 1)),
            ("data.crc", omod_crc(&[("Demo.esp", b"a")])),
            ("data", omod_zip(&[("a", b"a".to_vec())])),
            ("plugins.crc", omod_crc(&[("demo.ESP", b"b")])),
            ("plugins", omod_zip(&[("a", b"b".to_vec())])),
        ]),
    )
    .unwrap();
    assert!(open_archive(&archive, &t.path().join("work"), "Pack", "oblivion").is_err());
}

#[test]
fn omod_source_changes_and_links_fail_before_live_merge() {
    let t = TempDir::new("omod-source-change");
    let archive = write_omod(
        t.path(),
        "pack.omod",
        omod_config(4, 1),
        omod_crc(&[("Demo.esp", b"abc")]),
        omod_zip(&[("a", b"abc".to_vec())]),
        None,
    );
    let session = open_omod_with(
        &archive,
        &t.path().join("work"),
        &std::sync::atomic::AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    let mods = t.path().join("mods");
    write_at(&mods.join("Pack"), "Demo.esp", b"keep");
    write_at(&mods.join("Pack"), "meta.ini", b"original");
    let before = rel_paths(&mods);
    for content in [b"ab".as_slice(), b"xyz", b"abcd"] {
        fs::write(session.data_root().join("Demo.esp"), content).unwrap();
        assert!(install_omod(
            &session,
            &mods,
            "Pack",
            "oblivion",
            OverwritePolicy::MergeWithBackup
        )
        .is_err());
        assert_eq!(rel_paths(&mods), before);
        assert_eq!(fs::read(mods.join("Pack/Demo.esp")).unwrap(), b"keep");
        assert_eq!(fs::read(mods.join("Pack/meta.ini")).unwrap(), b"original");
    }
    fs::remove_file(session.data_root().join("Demo.esp")).unwrap();
    std::os::unix::fs::symlink(
        t.path().join("missing"),
        session.data_root().join("Demo.esp"),
    )
    .unwrap();
    assert!(install_omod(&session, &mods, "Pack", "oblivion", OverwritePolicy::Merge).is_err());
    assert_eq!(rel_paths(&mods), before);
    let external = t.path().join("external");
    write_at(&external, "data/Demo.esp", b"abc");
    fs::remove_dir_all(session.payload_root()).unwrap();
    std::os::unix::fs::symlink(&external, session.payload_root()).unwrap();
    assert!(install_omod(&session, &mods, "Pack", "oblivion", OverwritePolicy::Merge).is_err());
    assert_eq!(fs::read(mods.join("Pack/Demo.esp")).unwrap(), b"keep");
    assert_eq!(fs::read(external.join("data/Demo.esp")).unwrap(), b"abc");
    assert_eq!(rel_paths(&mods), before);
}

#[test]
#[ignore = "requires the separately downloaded OMODFramework reference archive"]
fn omod_upstream_reference_decodes_all_members() {
    let path =
        PathBuf::from(std::env::var_os("EIDOS_OMOD_REFERENCE").expect("set EIDOS_OMOD_REFERENCE"));
    let t = TempDir::new("omod-upstream");
    let session = open_omod_with(
        &path,
        t.path(),
        &std::sync::atomic::AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    assert_eq!(session.members.len(), 157);
    assert_eq!(
        session.members.iter().map(|m| m.size).sum::<u64>(),
        8_356_025
    );
    assert!(session.script.is_some());
    for m in &session.members {
        let root = if m.kind == OmodFileKind::Data {
            session.data_root()
        } else {
            session.plugins_root()
        };
        let bytes = fs::read(root.join(&m.path)).unwrap();
        assert_eq!(bytes.len() as u64, m.size);
        assert_eq!(crc32fast::hash(&bytes), m.crc32);
    }
}

#[test]
fn omod_case_alias_directories_keep_first_spelling_through_publication() {
    let t = TempDir::new("omod-case");
    let archive = write_omod(
        t.path(),
        "pack.omod",
        omod_config(4, 1),
        omod_crc(&[("Textures/A.dds", b"abc"), ("textures/B.dds", b"def")]),
        omod_zip(&[("a", b"abcdef".to_vec())]),
        None,
    );
    let report = install_archive(&archive, &t.path().join("mods"), "Pack", "oblivion").unwrap();
    assert_eq!(
        fs::read(report.dest.join("Textures/A.dds")).unwrap(),
        b"abc"
    );
    assert_eq!(
        fs::read(report.dest.join("Textures/B.dds")).unwrap(),
        b"def"
    );
    assert!(!report.dest.join("textures").exists());
}

#[test]
fn malformed_omod_container_is_not_reclassified_after_renaming_to_zip() {
    let t = TempDir::new("omod-classify");
    let archive = t.path().join("pack.zip");
    fs::write(
        &archive,
        omod_zip(&[
            ("config", omod_config(4, 1)),
            ("data.crc", omod_crc(&[("Demo.esp", b"abc")])),
            ("data", omod_zip(&[("a", b"abc".to_vec())])),
            ("unexpected", b"bad".to_vec()),
        ]),
    )
    .unwrap();
    assert!(try_open_omod(&archive, &t.path().join("work"), |_| {}).is_err());
}

#[test]
fn omod_progress_does_not_restart_between_data_and_plugins() {
    let t = TempDir::new("omod-progress");
    let archive = t.path().join("pack.omod");
    let bytes = vec![1; 131072];
    fs::write(
        &archive,
        omod_zip(&[
            ("config", omod_config(4, 1)),
            ("data.crc", omod_crc(&[("Textures/a.dds", &bytes)])),
            ("data", omod_zip(&[("a", bytes.clone())])),
            ("plugins.crc", omod_crc(&[("Demo.esp", &bytes)])),
            ("plugins", omod_zip(&[("a", bytes)])),
        ]),
    )
    .unwrap();
    let mut progress = Vec::new();
    let session = open_omod_with(
        &archive,
        &t.path().join("work"),
        &std::sync::atomic::AtomicBool::new(false),
        |p| progress.push(p),
    )
    .unwrap();
    assert_eq!(session.members.len(), 2);
    assert_eq!(progress.last(), Some(&100));
    assert!(progress.windows(2).all(|p| p[0] <= p[1]), "{progress:?}");
}

#[test]
fn omod_ordinary_merge_and_replace_retain_existing_backup_contract() {
    for policy in [
        OverwritePolicy::MergeWithBackup,
        OverwritePolicy::ReplaceWithBackup,
    ] {
        let t = TempDir::new("omod-backup");
        let archive = write_omod(
            t.path(),
            "pack.omod",
            omod_config(4, 1),
            omod_crc(&[("Demo.esp", b"new")]),
            omod_zip(&[("a", b"new".to_vec())]),
            None,
        );
        let mods = t.path().join("mods");
        let live = mods.join("Pack");
        write_at(&live, "demo.ESP", b"old");
        write_at(&live, ".hidden/state", b"precious");
        write_at(&live, "meta.ini", b"[General]\nnotes=retain\n");
        let report = install_archive_with_policy(
            &archive,
            &mods,
            "pack",
            "oblivion",
            policy.clone(),
            &Default::default(),
        )
        .unwrap();
        let backup = report.backup.unwrap();
        assert_eq!(fs::read(backup.join("demo.ESP")).unwrap(), b"old");
        assert_eq!(fs::read(backup.join(".hidden/state")).unwrap(), b"precious");
        assert_eq!(
            fs::read(backup.join("meta.ini")).unwrap(),
            b"[General]\nnotes=retain\n"
        );
        let merging = matches!(policy, OverwritePolicy::MergeWithBackup);
        assert_eq!(
            fs::read(
                report
                    .dest
                    .join(if merging { "demo.ESP" } else { "Demo.esp" })
            )
            .unwrap(),
            b"new"
        );
        assert_eq!(report.dest.join(".hidden/state").exists(), merging);
        assert_eq!(
            ModMeta::read(&report.dest.join("meta.ini"))
                .notes()
                .as_deref(),
            Some("retain")
        );
    }
}
