//! TEMPORARY audit probe - delete after use.
use std::fs;
use std::path::{Path, PathBuf};

use eidos_fuse::Eidos;
use fuser::BackgroundSession;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("eidos-audit-{}-{name}", std::process::id()));
    fs::create_dir_all(&d).unwrap();
    d
}

fn mount(layers: Vec<PathBuf>, over: PathBuf, mnt: &Path) -> Option<BackgroundSession> {
    Eidos::new(layers, over).spawn(mnt).ok()
}

#[test]
fn case_fold_hazard() {
    let (game, over, mnt) = (tmp("g1"), tmp("o1"), tmp("m1"));
    let Some(_s) = mount(vec![game], over, &mnt) else {
        eprintln!("cannot mount, skip");
        return;
    };

    // Probe with one casing (seeds a kernel negative dentry keyed on those bytes).
    assert!(!mnt.join("MISSING.ESP").exists(), "precondition");
    // Create with a different casing THROUGH THE MOUNT.
    fs::write(mnt.join("missing.esp"), b"here").unwrap();
    // Eidos resolves case-insensitively, so this SHOULD be readable.
    let r = fs::read(mnt.join("MISSING.ESP"));
    println!("CASEFOLD read of MISSING.ESP after creating missing.esp => {r:?}");
    assert!(r.is_ok(), "case-fold hazard reproduced: negative dentry outlived the create");
}

#[test]
fn transient_stat_error_is_cached_as_absence() {
    // resolve_read succeeds but symlink_metadata fails -> negative dentry with TTL.
    // Simulated by racing: hard to do deterministically, so just document reachability.
    let (game, over, mnt) = (tmp("g2"), tmp("o2"), tmp("m2"));
    fs::create_dir_all(game.join("sub")).unwrap();
    fs::write(game.join("sub/a.esp"), b"x").unwrap();
    let Some(_s) = mount(vec![game.clone()], over, &mnt) else { return };
    assert_eq!(fs::read(mnt.join("sub/a.esp")).unwrap(), b"x");
    // Remove out of band, probe (negative cached), restore out of band, probe again.
    fs::remove_file(game.join("sub/a.esp")).unwrap();
    assert!(!mnt.join("sub/a.esp").exists());
    fs::write(game.join("sub/a.esp"), b"y").unwrap();
    let r = fs::read(mnt.join("sub/a.esp"));
    println!("OUT-OF-BAND restore visible again? => {r:?}");
}

#[test]
fn negative_lookup_inode_growth() {
    let (game, over, mnt) = (tmp("g3"), tmp("o3"), tmp("m3"));
    fs::write(game.join("real.esp"), b"x").unwrap();
    let Some(_s) = mount(vec![game], over, &mnt) else { return };
    let base = fs::metadata(mnt.join("real.esp")).unwrap();
    println!("ino of real.esp before probes: {}", std::os::unix::fs::MetadataExt::ino(&base));
    for i in 0..64 {
        assert!(!mnt.join(format!("missing{i}.dll")).exists());
    }
    fs::write(mnt.join("after.esp"), b"z").unwrap();
    let after = fs::metadata(mnt.join("after.esp")).unwrap();
    println!("ino of after.esp (created post-64-probes): {}", std::os::unix::fs::MetadataExt::ino(&after));
}
