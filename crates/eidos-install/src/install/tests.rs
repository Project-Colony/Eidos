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
        let d = std::env::temp_dir()
            .join(format!("eidos-install-test-{}-{}-{}", tag, std::process::id(), n));
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
    let f = fs::OpenOptions::new().write(true).open(root.join(rel)).unwrap();
    let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs);
    f.set_times(std::fs::FileTimes::new().set_modified(t)).unwrap();
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
    assert_eq!(fs::read(mods.join("MyMod/textures/a.dds")).unwrap(), b"precious");
    assert_eq!(fs::read(mods.join("MyMod/meta.ini")).unwrap(), b"[General]\nendorsed=1\n");
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
    assert_eq!(rel_paths(t.path()), vec!["meshes/".to_string(), "meshes/armor.nif".to_string()]);
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
        vec!["meshes/".to_string(), "meshes/a.nif".to_string(), "meshes/b.nif".to_string()]
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
    assert!(t.path().join("textures").is_file(), "file wins the canonical name");
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
    assert!(t.path().join("textures").is_file(), "file wins the canonical name");
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
    assert_eq!(fs::read_link(t.path().join("current")).unwrap().to_str(), Some("some_target"));
    assert!(t.path().join("current_dir/f.txt").is_file(), "colliding dir moved aside");
}

#[test]
fn case_collision_non_utf8_skipped() {
    use std::os::unix::ffi::OsStrExt;
    let t = TempDir::new("ccnonutf8");
    // A genuine collision that MUST still resolve.
    write_at(t.path(), "Foo/a.txt", b"a");
    write_at(t.path(), "foo/b.txt", b"b");
    // A non-UTF8 sibling that must be left untouched (no panic).
    let bad = t.path().join(std::ffi::OsStr::from_bytes(b"weird\xff\xfename"));
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
    assert!(t.path().join("meshes/empty_subdir").is_dir(), "empty dir survives the merge");
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
    assert_eq!(before, rel_paths(t.path()), "non-colliding names are never touched");
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
    assert!(landed.is_file(), "file should land at <dest>/subdir/real.esp");
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
    assert!(matches!(r, Err(InstallError::Fomod(_))), "traversal must be refused");
    assert!(!outside.exists(), "nothing must be written outside the mod folder");

    // An absolute destination is refused too.
    let plan2 = vec![file_item("evil.esp", "/tmp/eidos-traversal-abs")];
    assert!(matches!(apply_plan(root.path(), &plan2, dest.path()), Err(InstallError::Fomod(_))));
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
    assert_eq!(ctx.file_states.get("skyrim.esm").map(String::as_str), Some("Active"));
    assert_eq!(ctx.file_states.get("skyui.esp").map(String::as_str), Some("Active"));
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
    assert_eq!(ctx.file_states.get("active.esp").map(String::as_str), Some("Active"));
    assert_eq!(ctx.file_states.get("disabled.esp").map(String::as_str), Some("Inactive"));
    assert_eq!(ctx.file_states.get("shared.esp").map(String::as_str), Some("Active"));
    assert_eq!(ctx.file_states.get("absent.esp"), None); // -> Missing
}

#[test]
fn reapply_user_meta_restores_endorsement_and_category() {
    let dir = TempDir::new("meta");
    let old_path = dir.path().join("old.ini");
    fs::write(&old_path, "[General]\nendorsed=1\ncategory=\"42,\"\ntracked=1\n").unwrap();
    let old = ModMeta::read(&old_path);

    let new_path = dir.path().join("new.ini");
    fs::write(&new_path, "[General]\nendorsed=0\ncategory=\"-1,\"\ntracked=0\n").unwrap();
    reapply_user_meta(&old, &new_path);

    let s = fs::read_to_string(&new_path).unwrap();
    assert!(s.contains("endorsed=1"));
    assert!(s.contains("tracked=1"));
    assert!(s.contains("category=\"42,\""));
}

#[test]
fn apply_plan_reports_missing_sources() {
    let root = TempDir::new("root");
    let dest = TempDir::new("dest");
    fs::write(root.path().join("present.esp"), b"x").unwrap();
    let plan = vec![file_item("present.esp", "present.esp"), file_item("absent.esp", "absent.esp")];
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
    ExtractedTree { tmp: dir.to_path_buf() }
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
    let r = install_bain(&tree, &picks, &archive, &mods, "Pack", "skyrimse", OverwritePolicy::Fail)
        .expect("bain install");

    // Both chosen sub-packages are merged; the unticked one is not installed.
    assert!(r.dest.join("MyMod.esp").is_file());
    assert!(r.dest.join("textures/extra.dds").is_file());
    assert!(!r.dest.join("meshes").exists());
    // The BAIN contract: a later sub-package overwrites an earlier one's file.
    assert_eq!(fs::read(r.dest.join("textures/shared.dds")).unwrap(), b"OPTIONAL");
    assert!(r.dest.join("meta.ini").is_file(), "a BAIN install writes meta.ini like any other");
    assert!(!r.fomod);
}

#[test]
fn bain_merge_order_is_the_callers_not_the_archives() {
    // Same pack, reversed ticks: now the core file survives. This is why the API
    // takes an ordered list instead of a set.
    let (_t, mods, tmp, archive) = bain_pack("bainorder");
    let tree = extracted(&tmp);
    let picks = vec!["01 Optional".to_string(), "00 Core".to_string()];
    let r = install_bain(&tree, &picks, &archive, &mods, "Pack", "skyrimse", OverwritePolicy::Fail)
        .expect("bain install (reversed)");
    assert_eq!(fs::read(r.dest.join("textures/shared.dds")).unwrap(), b"CORE");
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
        vec![],                                  // nothing ticked
        vec!["99 Nope".to_string()],             // not in the archive
        vec!["../outside".to_string()],          // traversal
        vec!["00 Core/textures".to_string()],    // not a top-level sub-package
    ] {
        let r = install_bain(&tree, &bad, &archive, &mods, "Pack", "skyrimse", OverwritePolicy::Fail);
        assert!(matches!(r, Err(InstallError::BadSelection(_))), "must refuse {bad:?}");
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
    assert_eq!(fs::read(mods.join("Pack/textures/a.dds")).unwrap(), b"precious");
    assert_eq!(fs::read(mods.join("Pack/meta.ini")).unwrap(), b"[General]\nendorsed=1\n");
}

#[test]
fn failed_bain_install_cleans_up_its_fresh_destination() {
    // A late failure (here: a sub-package shipping a DIRECTORY named meta.ini, so
    // writing the real one fails) must not leave debris the mod list would show as
    // an installed mod.
    let (t, mods, tmp) = bain_layout("bainclean");
    write_at(&tmp, "00 Core/MyMod.esp", b"x");
    write_at(&tmp, "01 Extras/meta.ini/oops.txt", b"a directory where a file goes");
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
    assert!(!mods.join("Pack").exists(), "the fresh destination must be cleaned up");
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
    assert!(!r.dest.join("Root").join("SSE Engine Fixes - Install Instructions.txt").exists());
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
    assert!(dest.join("PRECIOUS.esp").is_file(), "the existing mod must be untouched");
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
    assert!(dest.join("PRECIOUS.esp").is_file(), "the existing mod must be untouched");
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
    assert!(dest.join("PRECIOUS.esp").is_file(), "the existing mod must be untouched");
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
    let r =
        install_extracted(&extracted(&tmp), &archive, &mods, "SKSE", "skyrimse", OverwritePolicy::Fail, &ctx)
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
    assert!(r.dest.join("meta.ini").is_file(), "a completed install always has its meta");
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
    write_at(&tmp, "SB/Content/Paks/~mods/CNS/Cosmetics/Outfits.dekcns.json", b"{}");
    write_at(&tmp, "SB/Content/Paks/LogicMods/DekCNS_P.pak", b"pak");
    write_at(&tmp, "SB/Binaries/Win64/ue4ss/Mods/DekCNS/enabled.txt", b"1");

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
    assert!(r.dest.join("Root/SB/Content/Paks/LogicMods/DekCNS_P.pak").is_file());
    assert!(
        r.dest.join("Root/SB/Binaries/Win64/ue4ss/Mods/DekCNS/enabled.txt").is_file(),
        "the ue4ss tree keeps the path the loader expects"
    );
    // And nothing was flattened or duplicated on the way.
    assert!(!r.dest.join("Root/Binaries").exists(), "the game directory is not flattened");
    assert!(!r.dest.join("Root/LogicMods").exists(), "nor is the paks directory");
    assert!(!r.dest.join("~mods").exists(), "the deploy root is not nested inside itself");
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
    let r =
        install_extracted(&tree, &archive, &mods, "Mod", "skyrimse", OverwritePolicy::Fail, &ctx)
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

    let r =
        install_manual(&tree, "Package/Data", &archive, &mods, "Odd", "skyrimse", OverwritePolicy::Fail)
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
    let r =
        install_manual(&tree, "package/data", &archive, &mods, "Odd", "skyrimse", OverwritePolicy::Fail)
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
    let r = install_manual(&tree, "", &archive, &mods, "Odd", "skyrimse", OverwritePolicy::Fail)
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
        let r =
            install_manual(&tree, bad, &archive, &mods, "Odd", "skyrimse", OverwritePolicy::Fail);
        assert!(matches!(r, Err(InstallError::BadSelection(_))), "must refuse root '{bad}'");
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
    assert_eq!(fs::read(&outside).unwrap(), b"untouched", "must not write through the link");
    assert_eq!(fs::read(dest.join("victim")).unwrap(), b"payload");
    assert!(!fs::symlink_metadata(dest.join("victim")).unwrap().file_type().is_symlink());
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
    assert_eq!(fs::read_link(dest.join("link")).unwrap().to_str(), Some("nowhere"));
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
    assert_eq!(layout.simple_archive_base(rules()), None, "not a simple archive");
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
    assert!(all.iter().any(|e| e.path.ends_with("a.esp")), "real files still described");
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
