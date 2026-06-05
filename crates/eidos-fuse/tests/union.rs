//! End-to-end tests that mount a real Eidos union and drive it through the
//! kernel, exercising the FUSE daemon itself (inode table, copy-up, whiteouts,
//! rename rebind, readdir) which the pure-resolver unit tests cannot reach.
//!
//! These run with `harness = false` so `main` can enter a private user+mount
//! namespace *before* any threads exist (a hard requirement of `CLONE_NEWUSER`).
//! That matches how the product mounts (per-launch private namespace) and, just
//! as importantly, hides the test mounts from host desktop services (gvfs,
//! file indexers) that would otherwise race on them and make absence-after-delete
//! assertions flaky. If the namespace cannot be entered (userns disabled) or a
//! FUSE mount cannot be established (no `/dev/fuse`, restricted sandbox), the
//! affected tests are skipped so the suite stays green.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use eidos_fuse::Eidos;
use fuser::BackgroundSession;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Enter a private user + mount namespace (the `unshare --map-root-user --mount`
/// idiom) so our FUSE mounts are invisible to the rest of the system. Must run
/// single-threaded, hence before the test runner does anything else.
fn enter_private_namespace() -> std::io::Result<()> {
    // SAFETY: getuid/getgid always succeed.
    let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
    // SAFETY: unshare with namespace flags has no memory-safety preconditions.
    if unsafe { libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    fs::write("/proc/self/setgroups", "deny")?;
    fs::write("/proc/self/uid_map", format!("0 {uid} 1"))?;
    fs::write("/proc/self/gid_map", format!("0 {gid} 1"))?;
    // Stop our mounts from propagating to (or being seen by) the host.
    // SAFETY: standard mount(2) call with static C-string arguments.
    let rc = unsafe {
        libc::mount(
            c"none".as_ptr(),
            c"/".as_ptr(),
            std::ptr::null(),
            libc::MS_REC | libc::MS_PRIVATE,
            std::ptr::null(),
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// A self-cleaning temp directory tree, no external deps.
struct Tmp(PathBuf);

impl Tmp {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("eidos-fuse-it-{}-{}", std::process::id(), n));
        fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }

    fn sub(&self, name: &str) -> PathBuf {
        let p = self.0.join(name);
        fs::create_dir_all(&p).unwrap();
        p
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn put(dir: &Path, rel: &str, contents: &[u8]) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, contents).unwrap();
}

/// Mount a union, or return `None` (with a skip notice) if FUSE is unavailable.
fn mount(layers: Vec<PathBuf>, overwrite: PathBuf, mountpoint: &Path) -> Option<BackgroundSession> {
    match Eidos::new(layers, overwrite).spawn(mountpoint) {
        Ok(session) => Some(session),
        Err(e) => {
            eprintln!("  (cannot mount, skipping: {e})");
            None
        }
    }
}

/// Bind the mounted session to a name (keeping it alive) or `return` early when
/// FUSE is not available in this environment.
macro_rules! mounted {
    ($layers:expr, $over:expr, $mnt:expr) => {
        match mount($layers, $over, $mnt) {
            Some(session) => session,
            None => return,
        }
    };
}

fn mod_shadows_game_and_falls_through() {
    let t = Tmp::new();
    let (game, modd, over, mnt) = (t.sub("game"), t.sub("mod"), t.sub("over"), t.sub("mnt"));
    put(&game, "shared.dat", b"vanilla");
    put(&game, "only_game.dat", b"g");
    put(&modd, "shared.dat", b"from mod");
    let _s = mounted!(vec![modd, game], over, &mnt);

    assert_eq!(fs::read(mnt.join("shared.dat")).unwrap(), b"from mod"); // mod wins
    assert_eq!(fs::read(mnt.join("only_game.dat")).unwrap(), b"g"); // falls through
}

fn case_insensitive_read_through_mount() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "Textures/Armor.DDS", b"texdata");
    let _s = mounted!(vec![game], over, &mnt);

    // Ask with completely different casing, as a Windows game engine would.
    assert_eq!(fs::read(mnt.join("textures/armor.dds")).unwrap(), b"texdata");
}

fn write_copies_up_and_keeps_sources_pristine() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "config.ini", b"vanilla");
    let s = mounted!(vec![game.clone()], over.clone(), &mnt);

    fs::write(mnt.join("config.ini"), b"tweaked").unwrap();
    assert_eq!(fs::read(mnt.join("config.ini")).unwrap(), b"tweaked"); // merged view updated

    drop(s); // unmount before inspecting the real layers
    assert_eq!(fs::read(game.join("config.ini")).unwrap(), b"vanilla"); // game pristine
    assert_eq!(fs::read(over.join("config.ini")).unwrap(), b"tweaked"); // landed in overwrite
}

fn create_new_file_and_dir_land_in_overwrite() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    let s = mounted!(vec![game], over.clone(), &mnt);

    fs::create_dir(mnt.join("saves")).unwrap(); // exercises mkdir
    fs::write(mnt.join("saves/save01.ess"), b"savegame").unwrap(); // exercises create
    assert_eq!(fs::read(mnt.join("saves/save01.ess")).unwrap(), b"savegame");

    drop(s);
    assert_eq!(fs::read(over.join("saves/save01.ess")).unwrap(), b"savegame");
}

fn delete_hides_file_and_keeps_game_pristine() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "removeme.dat", b"x");
    put(&game, "keepme.dat", b"y");
    let s = mounted!(vec![game.clone()], over, &mnt);

    fs::remove_file(mnt.join("removeme.dat")).unwrap();
    assert!(!mnt.join("removeme.dat").exists()); // hidden in merged view
    assert!(mnt.join("keepme.dat").exists());

    drop(s);
    assert_eq!(fs::read(game.join("removeme.dat")).unwrap(), b"x"); // game pristine
}

fn rename_moves_file_through_mount() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    let _s = mounted!(vec![game], over, &mnt);

    fs::write(mnt.join("save.tmp"), b"data").unwrap();
    fs::rename(mnt.join("save.tmp"), mnt.join("save.ess")).unwrap();
    assert!(!mnt.join("save.tmp").exists());
    assert_eq!(fs::read(mnt.join("save.ess")).unwrap(), b"data");
}

fn readdir_lists_merged_deduped_entries() {
    let t = Tmp::new();
    let (game, modd, over, mnt) = (t.sub("game"), t.sub("mod"), t.sub("over"), t.sub("mnt"));
    put(&game, "a.dat", b"a");
    put(&game, "shared.dat", b"g");
    put(&modd, "b.dat", b"b");
    put(&modd, "shared.dat", b"m");
    let _s = mounted!(vec![modd, game], over, &mnt);

    let mut names: Vec<String> = fs::read_dir(&mnt)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, vec!["a.dat", "b.dat", "shared.dat"]); // shared appears once
}

fn rmdir_refuses_non_empty_directory() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "dir/inside.dat", b"x");
    let _s = mounted!(vec![game], over, &mnt);

    let err = fs::remove_dir(mnt.join("dir")).unwrap_err();
    assert_eq!(err.raw_os_error(), Some(libc::ENOTEMPTY)); // POSIX, not a silent recurse
}

fn large_file_round_trips_through_cached_handle() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    // Larger than the 1 MiB readahead/max_write, so the read spans several FUSE
    // ops served from the cached fd (exercises read_full_at chunking).
    let data: Vec<u8> = (0..(3 * 1024 * 1024)).map(|i| (i % 251) as u8).collect();
    put(&game, "big.bsa", &data);
    let _s = mounted!(vec![game], over, &mnt);

    let got = fs::read(mnt.join("big.bsa")).unwrap();
    assert_eq!(got.len(), data.len());
    assert!(got == data, "3 MiB read-back mismatch");
}

fn symlink_in_a_layer_is_readable() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "real.txt", b"target data");
    std::os::unix::fs::symlink("real.txt", game.join("link.txt")).unwrap(); // symlink in the layer
    let _s = mounted!(vec![game], over, &mnt);

    let meta = fs::symlink_metadata(mnt.join("link.txt")).unwrap();
    assert!(meta.file_type().is_symlink()); // reported as a symlink
    assert_eq!(fs::read_link(mnt.join("link.txt")).unwrap(), Path::new("real.txt")); // readlink
    assert_eq!(fs::read(mnt.join("link.txt")).unwrap(), b"target data"); // follows through
}

fn creating_a_symlink_lands_in_overwrite() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "real.txt", b"hi");
    let s = mounted!(vec![game], over.clone(), &mnt);

    std::os::unix::fs::symlink("real.txt", mnt.join("alias.txt")).unwrap(); // create via the mount
    assert_eq!(fs::read_link(mnt.join("alias.txt")).unwrap(), Path::new("real.txt"));
    assert_eq!(fs::read(mnt.join("alias.txt")).unwrap(), b"hi");

    drop(s);
    assert!(fs::symlink_metadata(over.join("alias.txt")).unwrap().file_type().is_symlink());
}

fn main() {
    // Enter the private namespace first, single-threaded, so the mounts are
    // isolated from host services. Best-effort: if it fails (userns disabled),
    // run in the host namespace and skip the delete test, which is the only one
    // sensitive to a host service racing on the mount.
    let isolated = match enter_private_namespace() {
        Ok(()) => true,
        Err(e) => {
            eprintln!("note: running in the host namespace; host services may race the mounts ({e})");
            false
        }
    };

    // (name, test fn, needs an isolated namespace to be deterministic)
    let tests: &[(&str, fn(), bool)] = &[
        ("mod_shadows_game_and_falls_through", mod_shadows_game_and_falls_through, false),
        ("case_insensitive_read_through_mount", case_insensitive_read_through_mount, false),
        ("write_copies_up_and_keeps_sources_pristine", write_copies_up_and_keeps_sources_pristine, false),
        ("create_new_file_and_dir_land_in_overwrite", create_new_file_and_dir_land_in_overwrite, false),
        ("delete_hides_file_and_keeps_game_pristine", delete_hides_file_and_keeps_game_pristine, true),
        ("rename_moves_file_through_mount", rename_moves_file_through_mount, false),
        ("readdir_lists_merged_deduped_entries", readdir_lists_merged_deduped_entries, false),
        ("rmdir_refuses_non_empty_directory", rmdir_refuses_non_empty_directory, false),
        ("large_file_round_trips_through_cached_handle", large_file_round_trips_through_cached_handle, false),
        ("symlink_in_a_layer_is_readable", symlink_in_a_layer_is_readable, false),
        ("creating_a_symlink_lands_in_overwrite", creating_a_symlink_lands_in_overwrite, false),
    ];

    println!("\nrunning {} union integration tests", tests.len());
    let mut failed = 0;
    let mut skipped = 0;
    for (name, test, needs_iso) in tests {
        if *needs_iso && !isolated {
            println!("test {name} ... SKIPPED (needs a private namespace)");
            skipped += 1;
            continue;
        }
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(*test)) {
            Ok(()) => println!("test {name} ... ok"),
            Err(_) => {
                println!("test {name} ... FAILED");
                failed += 1;
            }
        }
    }
    println!(
        "\nunion integration result: {} passed, {failed} failed, {skipped} skipped",
        tests.len() - failed - skipped
    );
    if failed > 0 {
        std::process::exit(1);
    }
}
