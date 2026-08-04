use crate::ModEntry;

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static N: AtomicUsize = AtomicUsize::new(0);

fn inst_with_mods(mods: &[&str]) -> PathBuf {
    let n = N.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("eidos-prof-{}-{}", std::process::id(), n));
    for m in mods {
        fs::create_dir_all(root.join("mods").join(m)).unwrap();
    }
    root
}

fn prof(root: &Path, name: &str) -> Profile {
    Profile { instance_root: root.to_path_buf(), name: name.to_string() }
}

#[test]
fn later_fragments_win_and_the_original_value_is_what_gets_restored() {
    let mut ini = "[Display]\nfDefaultFOV=75.0\niSize W=1920\n".to_string();
    let mut rec = Vec::new();

    assert!(merge_tweak(&mut ini, "[Display]\nfDefaultFOV=90.0\n", &mut rec));
    // A second fragment overwrites the first; `before` must still be vanilla,
    // or disabling both would leave the user on the first tweak's value.
    assert!(merge_tweak(&mut ini, "[Display]\nfDefaultFOV = 110.0\n", &mut rec));
    assert_eq!(eidos_ini::get_key(&ini, "Display", "fDefaultFOV"), Some("110.0"));
    assert_eq!(rec.len(), 1);
    assert_eq!(rec[0].before.as_deref(), Some("75.0"));
    assert_eq!(rec[0].after, "110.0");

    let restored = untweak_ini(&ini, &rec);
    assert_eq!(eidos_ini::get_key(&restored, "Display", "fDefaultFOV"), Some("75.0"));
    assert_eq!(eidos_ini::get_key(&restored, "Display", "iSize W"), Some("1920"));
}

#[test]
fn a_key_the_game_changed_in_flight_keeps_its_new_value() {
    let mut ini = "[Display]\nfDefaultFOV=75.0\n".to_string();
    let mut rec = Vec::new();
    merge_tweak(&mut ini, "[Display]\nfDefaultFOV=90.0\n", &mut rec);
    // The user moved the FOV slider in-game, so the captured INI no longer
    // holds what the tweak wrote. Their choice wins over the restore.
    let captured = eidos_ini::set_key(&ini, "Display", "fDefaultFOV", "100.0");
    let restored = untweak_ini(&captured, &rec);
    assert_eq!(eidos_ini::get_key(&restored, "Display", "fDefaultFOV"), Some("100.0"));
}

#[test]
fn a_key_the_tweak_invented_is_deleted_again_not_blanked() {
    let mut ini = "[Display]\niSize W=1920\n".to_string();
    let mut rec = Vec::new();
    merge_tweak(&mut ini, "[Papyrus]\nbEnableLogging=1\n", &mut rec);
    assert_eq!(rec[0].before, None);
    let restored = untweak_ini(&ini, &rec);
    // Absent, not `bEnableLogging=`: the engines read those differently.
    assert_eq!(eidos_ini::get_key(&restored, "Papyrus", "bEnableLogging"), None);
    assert!(restored.contains("[Papyrus]"));
}

#[test]
fn a_fragment_cannot_corrupt_the_target() {
    let mut ini = "[Display]\niSize W=1920\n".to_string();
    let mut rec = Vec::new();
    let junk = concat!(
        "; a comment\n",
        "# another\n",
        "\n",
        "strayKey=1\n",             // outside any section: dropped
        "[[not a header\n",         // not a section either
        "[General]\n",
        "sTestFile1 = a=b=c\n",     // value keeps its own '='
        "=novalue\n",               // empty key: dropped
        "no equals sign at all\n",
    );
    merge_tweak(&mut ini, junk, &mut rec);
    assert_eq!(eidos_ini::get_key(&ini, "General", "sTestFile1"), Some("a=b=c"));
    assert_eq!(rec.len(), 1);
    // The pre-existing key survived untouched.
    assert_eq!(eidos_ini::get_key(&ini, "Display", "iSize W"), Some("1920"));
}

#[test]
fn the_profile_tweak_file_is_applied_after_every_mod() {
    let root = inst_with_mods(&["A"]);
    let p = prof(&root, "Default");
    fs::create_dir_all(p.dir()).unwrap();
    fs::write(p.tweaks_path(), "[Display]\nfDefaultFOV=100.0\n").unwrap();

    let frag = root.join("frag.ini");
    fs::write(&frag, "[Display]\nfDefaultFOV=90.0\n").unwrap();
    let deployed = root.join("Skyrim.ini");
    fs::write(&deployed, "[Display]\nfDefaultFOV=75.0\n").unwrap();

    let rec = p.apply_ini_tweaks(&deployed, &[frag]).unwrap();
    let text = fs::read_to_string(&deployed).unwrap();
    // The profile's own file is last, so the user beats the mod.
    assert_eq!(eidos_ini::get_key(&text, "Display", "fDefaultFOV"), Some("100.0"));
    assert_eq!(rec[0].before.as_deref(), Some("75.0"));
    assert_eq!(rec[0].after, "100.0");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_separator_above_the_games_content_round_trips() {
    // The arrangement the whole feature exists for: a header, then the game's
    // DLC block, then the mods. Only the file decides whether it survives -
    // a separator is an ordinary managed row, so it needs its folder on disk,
    // and the `*` row needs its position read back in the same place.
    let root = inst_with_mods(&["Skyrim DLCs_separator", "Real"]);
    let p = prof(&root, "Default");
    let e = |n: &str, un: bool| ModEntry {
        name: n.into(),
        enabled: true,
        path: if un { root.join("gamedata").join(n) } else { root.join("mods").join(n) },
        unmanaged: un,
    };
    p.save_modlist(&[e("Skyrim DLCs_separator", false), e("Dawnguard", true), e("Real", false)])
        .unwrap();

    let (back, _) = p.modlist_checked();
    assert_eq!(
        back.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
        ["Skyrim DLCs_separator", "Dawnguard", "Real"],
        "the header did not stay above the game's content"
    );
    assert!(back[1].unmanaged, "the DLC row is still the game's, not a mod");
    // And the header is not a mount layer either - it has no files, and a
    // group of nothing must not shadow anything.
    let mounted = p.load_order();
    assert!(!mounted.iter().any(|m| m.to_string_lossy().contains("_separator")), "{mounted:?}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn unmanaged_content_keeps_its_position_but_is_never_mounted() {
    // The game's own DLCs and Creation Club plugins belong in the list - four
    // mods beside eighty loading plugins is what makes a user ask whether
    // their DLC is there at all.
    //
    // They are written with MO2's `*`, which grants the row a POSITION without
    // claiming Eidos installed the files. Dropping them, as this once did,
    // meant they could only ever be re-discovered and pinned to the top: no
    // separator could sit above them, so the block could not be collapsed and
    // the noise could not be put away.
    let root = inst_with_mods(&["Real"]);
    let p = prof(&root, "Default");
    let e = |n: &str, un: bool| ModEntry {
        name: n.into(),
        enabled: true,
        path: if un { root.join("gamedata").join(n) } else { root.join("mods").join(n) },
        unmanaged: un,
    };
    p.save_modlist(&[e("Dawnguard", true), e("Real", false)]).unwrap();

    let written = fs::read_to_string(p.modlist_path()).unwrap();
    assert!(written.contains("+Real"), "{written}");
    assert!(written.contains("*Dawnguard"), "the game's content needs a line to have a place: {written}");

    // Read back, the row is still there, still marked as the game's.
    let (back, _) = p.modlist_checked();
    let dg = back.iter().find(|m| m.name == "Dawnguard").expect("row survived the round trip");
    assert!(dg.unmanaged, "a `*` line is the game's content, not a mod");
    assert!(dg.path.as_os_str().is_empty(), "this layer cannot know the game's data dir");
    // And the order is preserved: display runs lowest priority first, and it
    // was saved ahead of Real.
    assert_eq!(back.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["Dawnguard", "Real"]);

    // It is never a mount layer, whatever a caller hands us. This is what makes
    // writing the row safe: the `*` says "position only", and the one consumer
    // that could act on it refuses by name.
    let mounted = p.load_order();
    assert!(
        !mounted.iter().any(|m| m.to_string_lossy().contains("Dawnguard")),
        "{mounted:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn modlist_round_trips_per_profile() {
    let root = inst_with_mods(&["A", "B", "C"]);
    let p = prof(&root, "Default");
    let mods = vec![
        ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B"), unmanaged: false },
        ModEntry { name: "A".into(), enabled: false, path: root.join("mods/A"), unmanaged: false },
        ModEntry { name: "C".into(), enabled: true, path: root.join("mods/C"), unmanaged: false },
    ];
    p.save_modlist(&mods).unwrap();
    let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
    assert_eq!(read, vec![("B".into(), true), ("A".into(), false), ("C".into(), true)]);

    // The atomic write must leave no stray ".tmp" sibling behind.
    let leftover_tmp = fs::read_dir(p.dir())
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().ends_with(".tmp"));
    assert!(!leftover_tmp, "save_modlist left a leftover .tmp file in the profile dir");

    let _ = fs::remove_dir_all(&root);
}

/// The curated order must survive a destroyed/partial `modlist.txt`: because
/// the write is atomic (temp file then rename), a save never leaves an empty
/// file that [`Profile::modlist`] would rebuild as "everything enabled,
/// alphabetical". Guards FIX F1 (MO2 `SafeWriteFile`/`QSaveFile` parity).
#[test]
fn save_modlist_is_atomic_and_keeps_a_backup() {
    let root = inst_with_mods(&["A", "B", "C"]);
    let p = prof(&root, "Default");
    let v1 = vec![
        ModEntry { name: "C".into(), enabled: true, path: root.join("mods/C"), unmanaged: false },
        ModEntry { name: "B".into(), enabled: false, path: root.join("mods/B"), unmanaged: false },
        ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A"), unmanaged: false },
    ];
    p.save_modlist(&v1).unwrap();

    // A second save (a toggle/move) over an existing list: backs the old one
    // up and swaps atomically.
    let v2 = vec![
        ModEntry { name: "A".into(), enabled: false, path: root.join("mods/A"), unmanaged: false },
        ModEntry { name: "C".into(), enabled: true, path: root.join("mods/C"), unmanaged: false },
        ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B"), unmanaged: false },
    ];
    p.save_modlist(&v2).unwrap();

    // The live file reflects the latest curated order (not the alphabetical default).
    let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
    assert_eq!(read, vec![("A".into(), false), ("C".into(), true), ("B".into(), true)]);

    // The one-deep backup holds the previous list and sits in the same dir.
    // The file stores highest-priority first (reverse of the in-memory v1).
    let bak = p.dir().join("modlist.txt.bak");
    assert!(bak.is_file(), "expected a one-deep modlist.txt.bak backup");
    assert_eq!(fs::read_to_string(&bak).unwrap(), "+A\n-B\n+C\n");

    // No temp file lingers after a successful save.
    assert!(!p.dir().join("modlist.txt.tmp").exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn create_from_copies_saves_subdir() {
    let root = inst_with_mods(&["A"]);
    let src = prof(&root, "Src");
    src.create().unwrap();
    // A save in the source profile's saves/ subdir + a curated modlist.
    let saves = src.dir().join("saves");
    fs::create_dir_all(&saves).unwrap();
    fs::write(saves.join("Save1.ess"), b"x").unwrap();
    src.save_modlist(&[ModEntry { name: "A".into(), enabled: false, path: root.join("mods/A"), unmanaged: false }])
        .unwrap();

    let dst = prof(&root, "Copy");
    dst.create_from(&src).unwrap();
    // The saves/ subdir is copied recursively (MO2 parity), not skipped...
    assert!(dst.dir().join("saves/Save1.ess").is_file());
    // ...and the modlist file came across too.
    assert!(!dst.modlist().iter().find(|m| m.name == "A").unwrap().enabled);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn modlist_parses_star_and_trims_names() {
    let root = inst_with_mods(&["A", "B", "C"]);
    let p = prof(&root, "Default");
    fs::create_dir_all(p.dir()).unwrap();
    // MO2 '*' foreign line (enabled), and +/- with padding that must be trimmed.
    // The file is highest-priority first; modlist() returns it reversed (display order).
    fs::write(p.modlist_path(), "*A\n-  B\n+ C \n").unwrap();
    let got: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
    assert_eq!(got, vec![("C".into(), true), ("B".into(), false), ("A".into(), true)]);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn two_profiles_share_mods_but_keep_own_order() {
    let root = inst_with_mods(&["A", "B"]);
    prof(&root, "Default")
        .save_modlist(&[
            ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A"), unmanaged: false },
            ModEntry { name: "B".into(), enabled: false, path: root.join("mods/B"), unmanaged: false },
        ])
        .unwrap();
    prof(&root, "Test")
        .save_modlist(&[
            ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B"), unmanaged: false },
            ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A"), unmanaged: false },
        ])
        .unwrap();
    let d: Vec<_> = prof(&root, "Default").modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
    let t: Vec<_> = prof(&root, "Test").modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
    assert_eq!(d, vec![("A".into(), true), ("B".into(), false)]);
    assert_eq!(t, vec![("B".into(), true), ("A".into(), true)]);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn default_profile_falls_back_to_legacy_flat_modlist() {
    let root = inst_with_mods(&["A", "B"]);
    // A pre-profiles instance: a flat <root>/modlist.txt (highest-priority first).
    fs::write(root.join("modlist.txt"), "-A\n+B\n").unwrap();
    let p = prof(&root, "Default");
    // modlist() returns display order (reverse of the file): B (top) then A.
    let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
    assert_eq!(read, vec![("B".into(), true), ("A".into(), false)]);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_folder_nobody_listed_appears_disabled() {
    let root = inst_with_mods(&["A", "New"]);
    let p = prof(&root, "Default");
    p.save_modlist(&[ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A"), unmanaged: false }]).unwrap();
    // "New" exists on disk but not in the saved list. It appears, but DISABLED
    // (MO2 parity): nothing knows where in the conflict order it belongs, and
    // silently enabling it could overwrite half the load order's files on the
    // next launch. A mod installed THROUGH Eidos never takes this path - the
    // installer writes its own modlist entry.
    let read: Vec<_> = p.modlist().iter().map(|m| (m.name.clone(), m.enabled)).collect();
    assert_eq!(read, vec![("New".into(), false), ("A".into(), true)]);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_mod_whose_folder_is_gone_leaves_the_list_but_not_the_file() {
    let root = inst_with_mods(&["A", "B"]);
    let p = prof(&root, "Default");
    let e = |n: &str| ModEntry { name: n.into(), enabled: true, path: root.join("mods").join(n), unmanaged: false };
    p.save_modlist(&[e("A"), e("B")]).unwrap();

    fs::remove_dir_all(root.join("mods/B")).unwrap();
    let (list, trust) = p.modlist_checked();
    assert_eq!(list.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["A"]);
    // One of two gone is an ordinary edit, not an accident.
    assert!(trust.is_good(), "{trust:?}");
    // The file still says both until something saves - the drop is a view.
    assert!(fs::read_to_string(p.modlist_path()).unwrap().contains("B"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_unmounted_mods_folder_cannot_flatten_the_order() {
    // The disaster case: mods/ lives on another drive via a bind mount, and the
    // mount is not up. The directory EXISTS and is READABLE and is EMPTY, so
    // every guard that only checks for existence sails straight through.
    let root = inst_with_mods(&["A", "B", "C"]);
    let p = prof(&root, "Default");
    let e = |n: &str| ModEntry { name: n.into(), enabled: true, path: root.join("mods").join(n), unmanaged: false };
    p.save_modlist(&[e("A"), e("B"), e("C")]).unwrap();
    let before = fs::read_to_string(p.modlist_path()).unwrap();

    for m in ["A", "B", "C"] {
        fs::remove_dir_all(root.join("mods").join(m)).unwrap();
    }
    let (list, trust) = p.modlist_checked();
    assert!(list.is_empty());
    assert!(!trust.is_good(), "an empty scan against a non-empty list must not be trusted");

    // And the save is refused rather than silently flattening the order.
    let err = p.save_modlist(&[]).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert_eq!(fs::read_to_string(p.modlist_path()).unwrap(), before);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_unreadable_mods_folder_is_not_an_empty_one() {
    use std::os::unix::fs::PermissionsExt;
    let root = inst_with_mods(&["A", "B"]);
    let p = prof(&root, "Default");
    let e = |n: &str| ModEntry { name: n.into(), enabled: true, path: root.join("mods").join(n), unmanaged: false };
    p.save_modlist(&[e("A"), e("B")]).unwrap();

    let mods = root.join("mods");
    fs::set_permissions(&mods, fs::Permissions::from_mode(0o000)).unwrap();
    let (_, trust) = p.modlist_checked();
    let refused = p.save_modlist(&[]).is_err();
    // Restore before asserting, so a failure does not leave an unremovable dir.
    fs::set_permissions(&mods, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(!trust.is_good(), "a read error must not read as 'you have no mods'");
    assert!(refused);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_mod_whose_name_starts_with_a_dot_is_kept() {
    // ".NET Script Framework" is a real, near-universal Skyrim SE dependency.
    // Only Eidos's own extraction temps are hidden from the list.
    let root = inst_with_mods(&[".NET Script Framework", ".eidos-install-abc123", "A"]);
    let p = prof(&root, "Default");
    let names: Vec<String> = p.modlist().iter().map(|m| m.name.clone()).collect();
    assert!(names.iter().any(|n| n == ".NET Script Framework"), "{names:?}");
    assert!(!names.iter().any(|n| n.starts_with(".eidos-install-")), "{names:?}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_dangling_symlink_is_still_a_row() {
    // A mod symlinked to a drive that is not mounted is BROKEN, not absent: its
    // position, enabled state and intended target are the irreplaceable part,
    // and dropping the row throws them away. `path().is_dir()` follows the link
    // and cannot tell this from a deleted mod; `file_type()` can.
    let root = inst_with_mods(&["A"]);
    std::os::unix::fs::symlink(root.join("nowhere"), root.join("mods/Linked")).unwrap();
    let p = prof(&root, "Default");
    let names: Vec<String> = p.modlist().iter().map(|m| m.name.clone()).collect();
    assert!(names.iter().any(|n| n == "Linked"), "{names:?}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn inis_seed_deploy_and_capture_round_trip() {
    let root = inst_with_mods(&["A"]);
    let p = prof(&root, "Default");
    // A fake prefix Documents dir holding the user's existing INIs.
    let prefix = root.join("prefix-docs");
    fs::create_dir_all(&prefix).unwrap();
    fs::write(prefix.join("Skyrim.ini"), "[General]\nsLanguage=ENGLISH\n").unwrap();
    fs::write(prefix.join("SkyrimPrefs.ini"), "[Display]\niSize W=1920\n").unwrap();
    let inis = ["Skyrim.ini", "SkyrimPrefs.ini"];

    // Seed adopts both into the profile; seeding again copies nothing.
    assert_eq!(p.seed_inis(&prefix, &inis).unwrap(), 2);
    assert!(p.ini_path("Skyrim.ini").is_file());
    assert_eq!(p.seed_inis(&prefix, &inis).unwrap(), 0);

    // The profile is now the source of truth: edit its copy, deploy elsewhere.
    fs::write(p.ini_path("Skyrim.ini"), "[General]\nsLanguage=FRENCH\n").unwrap();
    let prefix2 = root.join("prefix2");
    assert_eq!(p.deploy_inis(&prefix2, &inis).unwrap(), 2);
    assert!(fs::read_to_string(prefix2.join("Skyrim.ini")).unwrap().contains("FRENCH"));

    // The game writes to the prefix; capture pulls the change back.
    fs::write(prefix2.join("SkyrimPrefs.ini"), "[Display]\niSize W=2560\n").unwrap();
    assert_eq!(p.capture_inis(&prefix2, &inis).unwrap(), 2);
    assert!(fs::read_to_string(p.ini_path("SkyrimPrefs.ini")).unwrap().contains("2560"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn saves_seed_adopts_existing_then_skips() {
    let root = inst_with_mods(&["A"]);
    let p = prof(&root, "Default");
    let prefix_saves = root.join("prefix-saves");
    fs::create_dir_all(&prefix_saves).unwrap();
    fs::write(prefix_saves.join("Save1.ess"), b"x").unwrap();
    fs::write(prefix_saves.join("Save2.ess"), b"y").unwrap();

    // First run adopts the existing playthrough.
    assert_eq!(p.seed_saves(&prefix_saves).unwrap(), 2);
    assert!(p.saves_dir().join("Save1.ess").is_file());
    // Profile already has saves -> never re-seed (would clobber progress).
    fs::write(prefix_saves.join("Save3.ess"), b"z").unwrap();
    assert_eq!(p.seed_saves(&prefix_saves).unwrap(), 0);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn savegames_lists_files_newest_first_and_skips_dirs() {
    let root = inst_with_mods(&["A"]);
    let p = prof(&root, "Default");
    let saves = p.saves_dir();
    fs::create_dir_all(&saves).unwrap();
    // Write `Old` first, then sleep so `New` gets a strictly later mtime.
    fs::write(saves.join("Old.ess"), b"old").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(saves.join("New.ess"), b"newer-and-bigger").unwrap();
    // A subdirectory and a dotfile are both ignored.
    fs::create_dir_all(saves.join("backup")).unwrap();
    fs::write(saves.join(".DS_Store"), b"junk").unwrap();

    let list = p.savegames();
    let names: Vec<_> = list.iter().map(|s| s.filename.clone()).collect();
    assert_eq!(names, vec!["New.ess".to_string(), "Old.ess".to_string()]);
    assert_eq!(list[0].size, "newer-and-bigger".len() as u64);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn savegames_is_empty_when_no_saves_dir() {
    let root = inst_with_mods(&["A"]);
    let p = prof(&root, "Default");
    assert!(p.savegames().is_empty());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn display_order_file_order_and_load_order_stay_consistent() {
    // The orientation contract: modlist() is MO2 display order (lowest priority
    // at the top); the file stores highest-priority first; load_order() re-reverses
    // to highest-first for the mount. A regression here silently inverts conflicts.
    let root = inst_with_mods(&["Low", "High"]);
    let p = prof(&root, "Default");
    // Display order: Low at the top (lowest priority), High at the bottom (highest).
    p.save_modlist(&[
        ModEntry { name: "Low".into(), enabled: true, path: root.join("mods/Low"), unmanaged: false },
        ModEntry { name: "High".into(), enabled: true, path: root.join("mods/High"), unmanaged: false },
    ])
    .unwrap();
    // The file is highest-priority first (MO2 on-disk convention).
    assert_eq!(fs::read_to_string(p.dir().join("modlist.txt")).unwrap(), "+High\n+Low\n");
    // modlist() round-trips the display order.
    let names: Vec<_> = p.modlist().iter().map(|m| m.name.clone()).collect();
    assert_eq!(names, vec!["Low".to_string(), "High".to_string()]);
    // load_order() mounts highest priority first, so High wins same-name conflicts.
    assert_eq!(p.load_order(), vec![root.join("mods/High"), root.join("mods/Low")]);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn separator_round_trips_keeps_position_and_is_excluded_from_load_order() {
    // A separator is a real `*_separator` folder; it must round-trip in place,
    // be recognised as a separator, and never become a mount layer.
    let root = inst_with_mods(&["A", "Sec_separator", "B"]);
    let p = prof(&root, "Default");
    let mods = vec![
        ModEntry { name: "A".into(), enabled: true, path: root.join("mods/A"), unmanaged: false },
        ModEntry { name: "Sec_separator".into(), enabled: false, path: root.join("mods/Sec_separator"), unmanaged: false },
        ModEntry { name: "B".into(), enabled: true, path: root.join("mods/B"), unmanaged: false },
    ];
    p.save_modlist(&mods).unwrap();

    // modlist.txt is byte-faithful, including the `-` prefix + `_separator` suffix,
    // and stored highest-priority first (reverse of the in-memory display order).
    assert_eq!(fs::read_to_string(p.dir().join("modlist.txt")).unwrap(), "+B\n-Sec_separator\n+A\n");

    // Read back: order + the separator flag preserved, separator at index 1.
    let read = p.modlist();
    let names: Vec<_> = read.iter().map(|m| (m.name.clone(), m.enabled)).collect();
    assert_eq!(
        names,
        vec![("A".into(), true), ("Sec_separator".into(), false), ("B".into(), true)]
    );
    assert!(read[1].is_separator());
    assert_eq!(read[1].display_name(), "Sec");
    assert!(!read[0].is_separator());

    // load_order mounts only A and B - the separator is content-less - in
    // highest-priority-first order (B is below A in the display, so it wins).
    let order = p.load_order();
    assert_eq!(order, vec![root.join("mods/B"), root.join("mods/A")]);
    let _ = fs::remove_dir_all(&root);

    // An ENABLED separator (alone) still contributes no mount layer.
    let root2 = inst_with_mods(&["Solo_separator"]);
    let p2 = prof(&root2, "Default");
    p2.save_modlist(&[ModEntry {
        name: "Solo_separator".into(),
        enabled: true,
        path: root2.join("mods/Solo_separator"), unmanaged: false }])
    .unwrap();
    assert!(p2.load_order().is_empty());
    let _ = fs::remove_dir_all(&root2);
}

#[test]
fn a_crash_mangled_session_is_flagged_and_restorable() {
    // Under the bind-mount design the game writes the profile's plugins.txt
    // DIRECTLY, so there is no capture moment to refuse. The pre-session
    // snapshot restores the choke point: a session that wrecked the active
    // set is flagged, and one call puts the pre-session state back.
    let root = inst_with_mods(&["A"]);
    let p = prof(&root, "Default");
    fs::create_dir_all(p.dir()).unwrap();

    // A real 200-plugin order, snapshotted at launch.
    let full: String = (0..200).map(|i| format!("*Mod{i}.esp\n")).collect();
    fs::write(p.plugins_txt_path(), &full).unwrap();
    p.snapshot_plugin_state().unwrap();

    // The game dies during shutdown and leaves the active set mostly cleared.
    let mangled: String = (0..200)
        .map(|i| if i < 5 { format!("*Mod{i}.esp\n") } else { format!("Mod{i}.esp\n") })
        .collect();
    fs::write(p.plugins_txt_path(), &mangled).unwrap();
    assert!(
        p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_some(),
        "a crash artefact must be flagged, or the user never learns their order died"
    );
    p.restore_plugin_snapshot().unwrap();
    assert_eq!(fs::read_to_string(p.plugins_txt_path()).unwrap(), full);
    assert!(p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_none(), "restored = healthy");

    // A legitimate edit - the user turning a handful of mods off - is not
    // flagged; sessions that edit must not cry wolf.
    let edited: String = (0..200)
        .map(|i| if i < 195 { format!("*Mod{i}.esp\n") } else { format!("Mod{i}.esp\n") })
        .collect();
    fs::write(p.plugins_txt_path(), &edited).unwrap();
    assert!(p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_none());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn small_load_order_losses_follow_the_majority_rule() {
    // The check is a warn-with-one-click-dismiss now, not a silent refusal,
    // so the trade changed: turning off a couple of plugins must stay
    // silent, but losing the MAJORITY of a small list flags - that shape is
    // also what a partial crash artifact looks like, and it used to slide
    // under the big-list floor unchallenged.
    let root = inst_with_mods(&["A"]);
    let p = prof(&root, "Default");
    fs::create_dir_all(p.dir()).unwrap();
    fs::write(p.plugins_txt_path(), "*a\n*b\n*c\n*d\n*e\n*f\n").unwrap();
    p.snapshot_plugin_state().unwrap();

    // Two of six off: routine, silent.
    fs::write(p.plugins_txt_path(), "*a\n*b\n*c\n*d\ne\nf\n").unwrap();
    assert!(p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_none());

    // Four of six off: majority loss, flagged (dismissable in one click).
    fs::write(p.plugins_txt_path(), "*a\n*b\nc\nd\ne\nf\n").unwrap();
    assert!(p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_some());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_accented_plugin_name_does_not_disarm_the_wipe_guard() {
    // plugins.txt is CP1252 on disk - the encoding Eidos itself writes - so a
    // guard that reads it as strict UTF-8 returns None on the first accented
    // name and silently stops guarding. One translated mod ("Épées de
    // Bordeciel.esp") was enough to reopen the wipe this guard exists for.
    let root = inst_with_mods(&["A"]);
    let prefix = root.join("prefix");
    fs::create_dir_all(&prefix).unwrap();
    let p = prof(&root, "Default");
    fs::create_dir_all(p.dir()).unwrap();

    // The profile's list holds an accented name, CP1252-encoded (0xC9 = 'É').
    let mut good = b"*\xC9p\xE9es de Bordeciel.esp\r\n".to_vec();
    good.extend_from_slice(b"*a.esp\r\n*b.esp\r\n*c.esp\r\n*d.esp\r\n*e.esp\r\n*f.esp\r\n");
    fs::write(p.plugins_txt_path(), &good).unwrap();
    assert!(
        std::str::from_utf8(&good).is_err(),
        "the fixture must be real CP1252, not accidentally-valid UTF-8"
    );
    p.snapshot_plugin_state().unwrap();

    // The game crashes and leaves a header-only artifact.
    fs::write(p.plugins_txt_path(), b"# ruined\r\n").unwrap();
    assert!(
        p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_some(),
        "the wipe must be flagged even when the list has accented names"
    );
    p.restore_plugin_snapshot().unwrap();
    assert_eq!(fs::read(p.plugins_txt_path()).unwrap(), good);
}

#[test]
fn a_small_load_order_cleared_to_nothing_is_still_refused() {
    // The case that actually bit: Skyrim rewrote plugins.txt with nothing but
    // its own header while a 7-plugin order was live. That is below the floor
    // the proportional rule uses, so it went through unchallenged and the
    // profile lost every active plugin - which is exactly the state a user
    // adding mods a few at a time is in.
    let root = inst_with_mods(&["A"]);
    let prefix = root.join("prefix");
    fs::create_dir_all(&prefix).unwrap();
    let p = prof(&root, "Default");
    fs::create_dir_all(p.dir()).unwrap();
    let good = "*a.esp\n*b.esp\n*c.esp\n*d.esp\n*e.esp\n*f.esp\n*g.esp\n";
    fs::write(p.plugins_txt_path(), good).unwrap();
    p.snapshot_plugin_state().unwrap();
    fs::write(
        p.plugins_txt_path(),
        "# This file is used by Skyrim to keep track of your downloaded content.\n",
    )
    .unwrap();
    assert!(p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_some());
    p.restore_plugin_snapshot().unwrap();
    assert_eq!(fs::read_to_string(p.plugins_txt_path()).unwrap(), good);

    // Turning every plugin off BY HAND still flags - the backstop cannot read
    // minds - but the names stay listed, so nothing is lost and the user just
    // dismisses the warning instead of losing their order.
    let all_off = "a.esp\nb.esp\nc.esp\nd.esp\ne.esp\nf.esp\ng.esp\n";
    fs::write(p.plugins_txt_path(), all_off).unwrap();
    assert!(
        p.plugin_loss_since_snapshot(&eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).is_some(),
        "clearing every active plugin is flagged at any size"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn plugin_state_is_seeded_and_stays_per_profile() {
    let root = inst_with_mods(&["A"]);
    let prefix = root.join("prefix");
    fs::create_dir_all(&prefix).unwrap();
    fs::write(prefix.join("plugins.txt"), b"*Alpha.esp\nBeta.esp\n").unwrap();
    fs::write(prefix.join("loadorder.txt"), b"Alpha.esp\nBeta.esp\n").unwrap();
    // The game keeps sidecar files next to them; the bind must carry those
    // too, or the bound dir shows the game less than the dir it wrote.
    fs::write(prefix.join("ContentCatalog.txt"), b"{}").unwrap();
    // A crashed write's leftover must NOT be adopted.
    fs::write(prefix.join("plugins.tmp"), b"junk").unwrap();

    // Seed: the profile adopts the prefix's existing state once.
    let a = prof(&root, "Default");
    assert!(!a.has_plugin_state());
    assert_eq!(a.seed_plugin_state(&prefix, &eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).unwrap(), 3);
    assert!(a.has_plugin_state());
    assert!(a.plugins_state_dir().join("ContentCatalog.txt").is_file());
    assert!(!a.plugins_state_dir().join("plugins.tmp").exists());
    // Seeding again must not clobber the profile's own copy.
    fs::write(a.plugins_txt_path(), b"*Alpha.esp\n").unwrap();
    assert_eq!(a.seed_plugin_state(&prefix, &eidos_plugins::GameSpec::for_id("skyrimse").unwrap()).unwrap(), 0);
    assert_eq!(fs::read(a.plugins_txt_path()).unwrap(), b"*Alpha.esp\n");

    // A second profile has its own, independent state - the bound dir swaps
    // with the profile, so nothing leaks between them.
    let b = prof(&root, "Testing");
    assert!(!b.has_plugin_state());
    fs::write(b.plugins_txt_path(), b"*Beta.esp\n").unwrap();
    assert_eq!(fs::read(a.plugins_txt_path()).unwrap(), b"*Alpha.esp\n");
    assert_eq!(fs::read(b.plugins_txt_path()).unwrap(), b"*Beta.esp\n");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_backstop_sees_a_plainlist_wipe() {
    // PlainList files have no `*` at all, so counting asterisks read every
    // healthy Fallout list as "0 active" and every wipe as no-change - the
    // backstop was stone dead for that whole family.
    let root = inst_with_mods(&["A"]);
    let p = prof(&root, "Default");
    fs::create_dir_all(p.dir()).unwrap();
    let spec = eidos_plugins::GameSpec::for_id("falloutnv").unwrap();

    fs::write(p.plugins_txt_path(), b"FalloutNV.esm\nModA.esp\nModB.esp\n").unwrap();
    p.snapshot_plugin_state().unwrap();

    // Healthy rewrite: same actives, no flag.
    fs::write(p.plugins_txt_path(), b"FalloutNV.esm\nModA.esp\nModB.esp\n").unwrap();
    assert!(p.plugin_loss_since_snapshot(&spec).is_none());

    // The wipe: header only. Must flag at any size.
    fs::write(p.plugins_txt_path(), b"# nothing\n").unwrap();
    assert!(
        p.plugin_loss_since_snapshot(&spec).is_some(),
        "a PlainList wipe must be flagged, not read as 0-vs-0"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn seeding_adopts_verbatim_for_every_mechanism() {
    // The founding rule is ADOPT VERBATIM, always. An earlier "refuse the
    // crash artifact" version was worse than the disease: the same run then
    // derived everything-ENABLED from discovery and shadow-wrote it over the
    // prefix - and its signature (names listed, no `*`) is what every
    // healthy PlainList plugins.txt looks like, so Fallout and Skyrim LE
    // setups were refused wholesale. The artifact case is a WARNING now.
    let root = inst_with_mods(&["A"]);
    let prefix = root.join("prefix");
    fs::create_dir_all(&prefix).unwrap();
    // Asterisk game, names listed, zero active: adopted anyway, byte-for-byte.
    let artifact = b"a.esp\nb.esp\nc.esp\nd.esp\n";
    fs::write(prefix.join("plugins.txt"), artifact).unwrap();
    let p = prof(&root, "Default");
    p.seed_plugin_state(&prefix, &eidos_plugins::GameSpec::for_id("skyrimse").unwrap())
        .unwrap();
    assert!(p.has_plugin_state());
    assert_eq!(fs::read(p.plugins_txt_path()).unwrap(), artifact, "verbatim, not derived");

    // PlainList game (Fallout NV): a healthy actives-without-asterisks file
    // is NORMAL and adopts silently.
    let root2 = inst_with_mods(&["A"]);
    let prefix2 = root2.join("prefix");
    fs::create_dir_all(&prefix2).unwrap();
    let healthy = b"FalloutNV.esm\nSomeMod.esp\nOtherMod.esp\n";
    fs::write(prefix2.join("plugins.txt"), healthy).unwrap();
    let p2 = prof(&root2, "Default");
    p2.seed_plugin_state(&prefix2, &eidos_plugins::GameSpec::for_id("falloutnv").unwrap())
        .unwrap();
    assert!(p2.has_plugin_state());
    assert_eq!(fs::read(p2.plugins_txt_path()).unwrap(), healthy);
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&root2);
}

#[test]
fn a_truncated_ini_is_not_captured_over_the_profile() {
    let root = inst_with_mods(&["A"]);
    let docs = root.join("docs");
    fs::create_dir_all(&docs).unwrap();
    let p = prof(&root, "Default");
    fs::create_dir_all(p.dir()).unwrap();

    let good = "[Display]\n".to_string() + &"iKey=1\n".repeat(50);
    fs::write(p.ini_path("Skyrim.ini"), &good).unwrap();

    // Empty: never captured.
    fs::write(docs.join("Skyrim.ini"), b"").unwrap();
    assert_eq!(p.capture_inis(&docs, &["Skyrim.ini"]).unwrap(), 0);
    assert_eq!(fs::read_to_string(p.ini_path("Skyrim.ini")).unwrap(), good);

    // Under half the profile's size: a wreck, not an edit.
    fs::write(docs.join("Skyrim.ini"), b"[Display]\niKey=1\n").unwrap();
    assert_eq!(p.capture_inis(&docs, &["Skyrim.ini"]).unwrap(), 0);
    assert_eq!(fs::read_to_string(p.ini_path("Skyrim.ini")).unwrap(), good);

    // A real edit (same order of size) captures.
    let edited = good.replace("iKey=1", "iKey=2");
    fs::write(docs.join("Skyrim.ini"), &edited).unwrap();
    assert_eq!(p.capture_inis(&docs, &["Skyrim.ini"]).unwrap(), 1);
    assert_eq!(fs::read_to_string(p.ini_path("Skyrim.ini")).unwrap(), edited);

    // The engine's own compact rewrite is STABLE: refused once, but the
    // same size on the next run is the real format and must be accepted -
    // refusing forever would mean in-game settings never persist again.
    let compact = "[Display]\niKey=3\n";
    fs::write(docs.join("Skyrim.ini"), compact).unwrap();
    assert_eq!(p.capture_inis(&docs, &["Skyrim.ini"]).unwrap(), 0, "first sight: refused");
    assert_eq!(p.capture_inis(&docs, &["Skyrim.ini"]).unwrap(), 1, "stable repeat: accepted");
    assert_eq!(fs::read_to_string(p.ini_path("Skyrim.ini")).unwrap(), compact);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn emptying_the_saves_dir_does_not_resurrect_prefix_saves() {
    let root = inst_with_mods(&["A"]);
    let prefix_saves = root.join("prefix_saves");
    fs::create_dir_all(&prefix_saves).unwrap();
    fs::write(prefix_saves.join("ancient.ess"), b"2024").unwrap();
    fs::write(prefix_saves.join("steam_autocloud.vdf"), b"junk").unwrap();

    let p = prof(&root, "Default");
    assert_eq!(p.seed_saves(&prefix_saves).unwrap(), 1, "junk is not a save");
    assert!(p.saves_dir().join("ancient.ess").is_file());
    assert!(!p.saves_dir().join("steam_autocloud.vdf").exists());

    // The user empties the dir on purpose. The old emptiness probe re-seeded
    // the ancient save with a fresh mtime that sorted above everything.
    fs::remove_file(p.saves_dir().join("ancient.ess")).unwrap();
    assert_eq!(p.seed_saves(&prefix_saves).unwrap(), 0, "seeding is once, ever");
    assert!(!p.saves_dir().join("ancient.ess").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_float_reserialised_by_the_engine_still_untweaks() {
    // The tweak set fShadowDistance=8000; the engine rewrote it "8000.0000".
    // Text-compare said "user changed it" and kept the tweak forever.
    let mut ini = "[Display]\nfShadowDistance=4000\n".to_string();
    let mut rec = Vec::new();
    assert!(merge_tweak(&mut ini, "[Display]\nfShadowDistance=8000\n", &mut rec));
    let engine_rewritten = ini.replace("fShadowDistance=8000", "fShadowDistance=8000.0000");
    let restored = untweak_ini(&engine_rewritten, &rec);
    assert!(
        restored.contains("fShadowDistance=4000"),
        "numerically-equal means unchanged; the original must come back: {restored}"
    );

    // A REAL user change (different number) still wins over the restore.
    let user_changed = ini.replace("fShadowDistance=8000", "fShadowDistance=6500");
    let kept = untweak_ini(&user_changed, &rec);
    assert!(kept.contains("fShadowDistance=6500"), "{kept}");
}

#[test]
fn the_legacy_top_level_plugin_files_migrate_into_the_plugins_dir() {
    // Profiles created before the bind-mount design kept plugins.txt and
    // loadorder.txt at the profile top level. First access must move them in,
    // or every existing user starts from an empty load order.
    let root = inst_with_mods(&["A"]);
    let p = prof(&root, "Default");
    fs::create_dir_all(p.dir()).unwrap();
    fs::write(p.dir().join("plugins.txt"), b"*Old.esp\n").unwrap();
    fs::write(p.dir().join("loadorder.txt"), b"Old.esp\n").unwrap();

    let dir = p.plugins_state_dir();
    assert_eq!(fs::read(dir.join("plugins.txt")).unwrap(), b"*Old.esp\n");
    assert_eq!(fs::read(dir.join("loadorder.txt")).unwrap(), b"Old.esp\n");
    assert!(!p.dir().join("plugins.txt").exists(), "the legacy copy must MOVE, not fork");
    assert!(p.has_plugin_state());
    let _ = fs::remove_dir_all(&root);
}
