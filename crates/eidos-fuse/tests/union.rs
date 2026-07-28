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
//!
//! RUN THIS SUITE TWICE. By default the daemon declines `opendir` so the kernel
//! opens and reads directories from its own cache, which is where nearly all of
//! a real session's directory cost went. `EIDOS_FUSE_OPENDIR=1` restores the
//! per-handle snapshots. The two paths reach `readdir` completely differently -
//! one through a snapshot pinned to a file handle, one through the by-path
//! listing cache - so a listing or invalidation bug can live in either alone.
//! Both must pass:
//!
//! ```sh
//! cargo test -p eidos-fuse --test union
//! EIDOS_FUSE_OPENDIR=1 cargo test -p eidos-fuse --test union
//! ```

use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
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

/// Poll `f` until it yields a value, for up to a second. Used where the daemon
/// answers a request and only then pushes a cache invalidation to the kernel.
fn settle<T>(mut f: impl FnMut() -> Option<T>) -> Option<T> {
    for _ in 0..100 {
        if let Some(v) = f() {
            return Some(v);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    None
}

/// Recursively bind-mount `src` onto `dst`, as the launcher does to capture the
/// pristine files before a union covers that same path. Safe here because the
/// test binary already entered a private mount namespace.
fn bind(src: &Path, dst: &Path) -> bool {
    let c = |p: &Path| std::ffi::CString::new(p.as_os_str().as_bytes()).unwrap();
    let (s, d) = (c(src), c(dst));
    // SAFETY: standard mount(2) bind with valid C strings.
    unsafe {
        libc::mount(s.as_ptr(), d.as_ptr(), std::ptr::null(), libc::MS_BIND | libc::MS_REC, std::ptr::null())
            == 0
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

fn recreate_after_delete_starts_empty() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    // The lower-layer file is LONGER than what we rewrite, so any resurrected
    // bytes would survive as trailing garbage after the new content.
    put(&game, "gen.json", b"OLD-CONTENT-MUCH-LONGER-THAN-NEW");
    let s = mounted!(vec![game.clone()], over.clone(), &mnt);

    // The delete-then-recreate cycle of modding tools (xEdit backups, DynDOLOD
    // regen, PapyrusUtil JSON rewrites): unlink, then open(O_CREAT) = FUSE CREATE.
    fs::remove_file(mnt.join("gen.json")).unwrap();
    fs::write(mnt.join("gen.json"), b"new").unwrap();
    assert_eq!(fs::read(mnt.join("gen.json")).unwrap(), b"new"); // no stale tail

    drop(s);
    assert_eq!(fs::read(game.join("gen.json")).unwrap(), b"OLD-CONTENT-MUCH-LONGER-THAN-NEW"); // game pristine
    assert_eq!(fs::read(over.join("gen.json")).unwrap(), b"new"); // overwrite holds only the new bytes
}

fn same_file_has_one_inode_whatever_the_casing() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "Textures/Armor.DDS", b"tex");
    let _s = mounted!(vec![game], over, &mnt);

    // The Creation Engine and Wine mix casing between the plugin header, the
    // loose-file indexer and BSA lookups. NTFS reports one identity; so must we.
    let a = fs::metadata(mnt.join("Textures/Armor.DDS")).unwrap();
    let b = fs::metadata(mnt.join("textures/armor.dds")).unwrap();
    assert_eq!(a.ino(), b.ino(), "one real file must have one inode");
}

fn xattr_on_a_deleted_file_does_not_resurrect_it() {
    use std::os::fd::AsRawFd;
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "gone.esp", b"vanilla");
    let s = mounted!(vec![game.clone()], over.clone(), &mnt);

    // Hold the file open, THEN delete it. The kernel keeps the inode alive for
    // the open fd, so an fsetxattr still reaches our handler - unlike a
    // path-based setxattr, which the kernel rejects against a negative dentry.
    // Wine holds handles open across attribute writes, so this is the real shape.
    let f = fs::File::open(mnt.join("gone.esp")).unwrap();
    fs::remove_file(mnt.join("gone.esp")).unwrap();

    // SAFETY: valid fd and NUL-terminated strings.
    let rc = unsafe {
        libc::fsetxattr(f.as_raw_fd(), c"user.DOSATTRIB".as_ptr(), c"x".as_ptr().cast(), 1, 0)
    };
    assert_eq!(rc, -1, "an xattr write against a deleted path must fail, not copy it up");
    assert!(!mnt.join("gone.esp").exists(), "the file must stay deleted");

    drop(f);
    drop(s);
    assert!(!over.join("gone.esp").exists(), "no copy-up may have happened");
    assert_eq!(fs::read(game.join("gone.esp")).unwrap(), b"vanilla"); // game pristine
}

fn creating_a_file_after_a_negative_lookup_is_visible() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    let _s = mounted!(vec![game], over, &mnt);

    // Probe an absent path first: this is what seeds the kernel's negative
    // dentry. Creating it afterwards must be immediately visible - the kernel
    // re-issues a real lookup for O_CREAT, so the cache cannot hide it.
    assert!(!mnt.join("later.esp").exists());
    fs::write(mnt.join("later.esp"), b"now here").unwrap();
    assert_eq!(fs::read(mnt.join("later.esp")).unwrap(), b"now here");

    // Same for a directory, which takes the LOOKUP_EXCL path.
    assert!(!mnt.join("newdir").exists());
    fs::create_dir(mnt.join("newdir")).unwrap();
    assert!(mnt.join("newdir").is_dir());
}

fn a_negative_lookup_does_not_mint_an_inode() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "real.esp", b"x");
    let _s = mounted!(vec![game], over, &mnt);

    // Probing a pile of absent names must not grow the inode table; a negative
    // dentry carries ino = 0 and is never interned.
    for i in 0..64 {
        assert!(!mnt.join(format!("missing{i}.dll")).exists());
    }
    // The real file still resolves normally afterwards.
    assert_eq!(fs::read(mnt.join("real.esp")).unwrap(), b"x");
}

fn a_create_clears_a_differently_cased_negative_lookup() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    let _s = mounted!(vec![game], over, &mnt);

    // The kernel caches negative dentries on the EXACT name bytes, while Eidos
    // resolves case-insensitively. Probe one spelling, create another: without an
    // explicit invalidation the game is told a file it can plainly see is absent.
    // The Creation Engine mixes casing constantly, so this is normal traffic.
    //
    // The invalidation is necessarily ASYNCHRONOUS: a notification is a message
    // to the kernel, and sending one from inside a request handler deadlocks the
    // mount, so it is handed to a thread. That leaves a sub-millisecond window,
    // hence the short retry rather than a bare assert.
    assert!(!mnt.join("MISSING.ESP").exists());
    fs::write(mnt.join("missing.esp"), b"here").unwrap();
    assert_eq!(
        settle(|| fs::read(mnt.join("MISSING.ESP")).ok()).expect("a cached negative outlived the create"),
        b"here"
    );

    // Same for a directory, which arrives through mkdir rather than create.
    assert!(!mnt.join("SCRIPTS").exists());
    fs::create_dir(mnt.join("scripts")).unwrap();
    assert!(
        settle(|| mnt.join("SCRIPTS").is_dir().then_some(())).is_some(),
        "mkdir must clear the folded negative too"
    );
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

fn renaming_a_file_drops_its_other_cached_spellings() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "Save.ess", b"first");
    let _s = mounted!(vec![game], over, &mnt);

    // Warm the kernel's dentry cache under BOTH spellings: the case fold makes
    // one inode reachable through several dentries.
    assert_eq!(fs::read(mnt.join("Save.ess")).unwrap(), b"first");
    assert_eq!(fs::read(mnt.join("save.ess")).unwrap(), b"first");

    // Rename through one spelling. The kernel moves that dentry; the other one
    // must not keep serving the moved file under a name that no longer exists.
    fs::rename(mnt.join("Save.ess"), mnt.join("Save.bak")).unwrap();
    assert_eq!(fs::read(mnt.join("Save.bak")).unwrap(), b"first");
    assert!(
        settle(|| fs::read(mnt.join("save.ess")).is_err().then_some(())).is_some(),
        "a stale case-variant dentry kept resolving to the renamed file"
    );
}

fn a_root_union_can_carry_a_data_union_inside_it() {
    // MO2's Root Builder shape: one union over the GAME ROOT (where the exe and
    // a script extender's DLLs live) with a second union over its Data/. This is
    // what lets SKSE and ENB be mods instead of files copied into the install.
    let t = Tmp::new();
    let (game, root_mod, over_root) = (t.sub("game"), t.sub("rootmod"), t.sub("over_root"));
    let (data_mod, over_data) = (t.sub("datamod"), t.sub("over_data"));
    let stash = t.sub("stash");

    // A pristine game root with its exe and a vanilla Data file.
    put(&game, "SkyrimSE.exe", b"vanilla exe");
    put(&game, "Data/Skyrim.esm", b"vanilla master");
    // A mod shipping root-level content, as SKSE does.
    put(&root_mod, "skse64_loader.exe", b"skse");
    put(&root_mod, "Data/SKSE/Plugins/x.dll", b"plugin");
    // And an ordinary Data mod.
    put(&data_mod, "Interface/thing.swf", b"ui");

    // Capture the pristine root at the stash, exactly as launch does: the union
    // is about to cover `game` itself, so the daemon needs another way to read it.
    if !bind(&game, &stash) {
        eprintln!("  (cannot bind-mount, skipping)");
        return;
    }
    let Some(_root) = mount(vec![root_mod, stash.clone()], over_root, &game) else { return };
    // Everything the game root should show: vanilla exe, mod-provided loader.
    assert_eq!(fs::read(game.join("SkyrimSE.exe")).unwrap(), b"vanilla exe");
    assert_eq!(fs::read(game.join("skse64_loader.exe")).unwrap(), b"skse");

    // The Data union mounts INSIDE the root union.
    let data_mnt = game.join("Data");
    let Some(_data) = mount(vec![data_mod], over_data, &data_mnt) else { return };
    assert_eq!(fs::read(data_mnt.join("Interface/thing.swf")).unwrap(), b"ui");
    // The root union is still readable underneath.
    assert_eq!(fs::read(game.join("skse64_loader.exe")).unwrap(), b"skse");
}

fn readdir_lists_merged_deduped_entries() {
    let t = Tmp::new();
    let (game, modd, over, mnt) = (t.sub("game"), t.sub("mod"), t.sub("over"), t.sub("mnt"));
    put(&game, "a.dat", b"a");
    put(&game, "shared.dat", b"g");
    put(&game, "_under.dat", b"u");
    put(&modd, "b.dat", b"b");
    put(&modd, "shared.dat", b"m");
    let _s = mounted!(vec![modd, game], over, &mnt);

    // Do NOT sort: readdir must already emit NTFS-collated order (the daemon sorts
    // in list_dir), so assert the emission order verbatim. `_under.dat`
    // discriminates - NTFS upcases, so `_` (0x5F) sorts AFTER letters, whereas a
    // plain ASCII-lowercase sort would place it first.
    let names: Vec<String> = fs::read_dir(&mnt)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["a.dat", "b.dat", "shared.dat", "_under.dat"]); // shared once; `_` last
}

/// The merged listing is memoised by path, so every mutation must drop it. Each
/// case here ENUMERATES FIRST - that is what fills the cache - then mutates, then
/// enumerates again. Without the drop the second listing is the first one, and the
/// game spends the rest of the mount looking at a directory that no longer exists
/// in that shape.
fn a_cached_listing_is_dropped_by_every_mutation() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "keep.dat", b"k");
    put(&game, "doomed.dat", b"d");
    put(&game, "sub/inner.dat", b"i");
    let _s = mounted!(vec![game], over, &mnt);

    let names = |p: &std::path::Path| -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(p)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    };

    // Prime the cache.
    assert_eq!(names(&mnt), vec!["doomed.dat", "keep.dat", "sub"]);

    // create
    fs::write(mnt.join("fresh.dat"), b"f").unwrap();
    assert!(names(&mnt).contains(&"fresh.dat".to_string()), "create must invalidate");

    // unlink
    fs::remove_file(mnt.join("doomed.dat")).unwrap();
    assert!(!names(&mnt).contains(&"doomed.dat".to_string()), "unlink must invalidate");

    // mkdir
    fs::create_dir(mnt.join("made")).unwrap();
    assert!(names(&mnt).contains(&"made".to_string()), "mkdir must invalidate");

    // rmdir - and the removed directory's OWN cached listing must go too, or a
    // recreated directory of the same name would come back with the old contents.
    let _ = names(&mnt.join("made")); // cache the empty listing
    fs::remove_dir(mnt.join("made")).unwrap();
    assert!(!names(&mnt).contains(&"made".to_string()), "rmdir must invalidate the parent");

    // symlink
    std::os::unix::fs::symlink("keep.dat", mnt.join("alias.dat")).unwrap();
    assert!(names(&mnt).contains(&"alias.dat".to_string()), "symlink must invalidate");

    // rename, which changes BOTH parents
    let _ = names(&mnt.join("sub"));
    fs::rename(mnt.join("keep.dat"), mnt.join("sub/moved.dat")).unwrap();
    let top = names(&mnt);
    assert!(!top.contains(&"keep.dat".to_string()), "rename must invalidate the source dir");
    assert!(
        names(&mnt.join("sub")).contains(&"moved.dat".to_string()),
        "rename must invalidate the destination dir"
    );
}

/// A directory recreated after being removed must not inherit the listing cached
/// for its previous incarnation.
fn a_recreated_directory_does_not_inherit_the_old_listing() {
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "d/old.dat", b"o");
    let _s = mounted!(vec![game], over, &mnt);

    let names = |p: &std::path::Path| -> Vec<String> {
        fs::read_dir(p)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    };

    assert_eq!(names(&mnt.join("d")), vec!["old.dat"]); // prime
    fs::remove_file(mnt.join("d/old.dat")).unwrap();
    fs::remove_dir(mnt.join("d")).unwrap();
    fs::create_dir(mnt.join("d")).unwrap();
    assert!(names(&mnt.join("d")).is_empty(), "a recreated directory starts empty");
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

fn writable_mmap_persists_and_keeps_source_pristine() {
    use std::os::fd::AsRawFd;
    let t = Tmp::new();
    let (game, over, mnt) = (t.sub("game"), t.sub("over"), t.sub("mnt"));
    put(&game, "patch.dat", b"AAAAAAAA"); // 8 bytes in the game layer
    let s = mounted!(vec![game.clone()], over.clone(), &mnt);

    // Open read+write (copies up to Overwrite), mmap MAP_SHARED, and write
    // through the mapping. msync forces the kernel to flush the dirty pages back
    // through the daemon, which only succeeds with writeback_cache negotiated.
    let file = std::fs::OpenOptions::new().read(true).write(true).open(mnt.join("patch.dat")).unwrap();
    let len = 8usize;
    // SAFETY: standard mmap/msync/munmap on a valid fd and length; we check each.
    unsafe {
        let p = libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            file.as_raw_fd(),
            0,
        );
        assert!(p != libc::MAP_FAILED, "mmap failed");
        let bytes = std::slice::from_raw_parts_mut(p as *mut u8, len);
        bytes[0] = b'Z';
        bytes[7] = b'Z';
        assert_eq!(libc::msync(p, len, libc::MS_SYNC), 0, "msync failed");
        assert_eq!(libc::munmap(p, len), 0, "munmap failed");
    }
    drop(file);

    assert_eq!(fs::read(mnt.join("patch.dat")).unwrap(), b"ZAAAAAAZ"); // visible in the view
    drop(s);
    assert_eq!(fs::read(game.join("patch.dat")).unwrap(), b"AAAAAAAA"); // game pristine (copy-up)
    assert_eq!(fs::read(over.join("patch.dat")).unwrap(), b"ZAAAAAAZ"); // change in overwrite
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
        ("recreate_after_delete_starts_empty", recreate_after_delete_starts_empty, true),
        ("same_file_has_one_inode_whatever_the_casing", same_file_has_one_inode_whatever_the_casing, false),
        ("xattr_on_a_deleted_file_does_not_resurrect_it", xattr_on_a_deleted_file_does_not_resurrect_it, true),
        ("creating_a_file_after_a_negative_lookup_is_visible", creating_a_file_after_a_negative_lookup_is_visible, false),
        ("a_negative_lookup_does_not_mint_an_inode", a_negative_lookup_does_not_mint_an_inode, false),
        ("a_create_clears_a_differently_cased_negative_lookup", a_create_clears_a_differently_cased_negative_lookup, false),
        ("renaming_a_file_drops_its_other_cached_spellings", renaming_a_file_drops_its_other_cached_spellings, true),
        ("a_root_union_can_carry_a_data_union_inside_it", a_root_union_can_carry_a_data_union_inside_it, true),
        ("rename_moves_file_through_mount", rename_moves_file_through_mount, false),
        ("readdir_lists_merged_deduped_entries", readdir_lists_merged_deduped_entries, false),
        ("rmdir_refuses_non_empty_directory", rmdir_refuses_non_empty_directory, false),
        ("a_cached_listing_is_dropped_by_every_mutation", a_cached_listing_is_dropped_by_every_mutation, true),
        ("a_recreated_directory_does_not_inherit_the_old_listing", a_recreated_directory_does_not_inherit_the_old_listing, true),
        ("large_file_round_trips_through_cached_handle", large_file_round_trips_through_cached_handle, false),
        ("symlink_in_a_layer_is_readable", symlink_in_a_layer_is_readable, false),
        ("creating_a_symlink_lands_in_overwrite", creating_a_symlink_lands_in_overwrite, false),
        ("writable_mmap_persists_and_keeps_source_pristine", writable_mmap_persists_and_keeps_source_pristine, false),
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
