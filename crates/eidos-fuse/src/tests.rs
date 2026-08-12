use std::sync::atomic::Ordering;

use super::*;

#[test]
fn one_file_gets_one_inode_whatever_the_casing() {
    // Windows games mix casing freely; the resolver folds it, so the inode
    // table must too or stat() reports two different files.
    let mut inodes = Inodes::new();
    let a = inodes.intern("Textures/Armor.DDS");
    let b = inodes.intern("textures/armor.dds");
    assert_eq!(a, b, "casing must not split inode identity");
    // The display path keeps the casing it was first interned with.
    assert_eq!(inodes.path(a).as_deref(), Some("Textures/Armor.DDS"));
}

#[test]
fn rename_over_an_interned_destination_keeps_the_survivor_mapped() {
    // The atomic INI/save replacement pattern: write tmp, rename over target.
    let mut inodes = Inodes::new();
    let victim = inodes.lookup("Skyrim.ini");
    let src = inodes.lookup("Skyrim.ini.tmp");
    inodes.rename("Skyrim.ini.tmp", "Skyrim.ini");

    assert_eq!(inodes.intern("Skyrim.ini"), src, "the renamed inode owns the path");
    // Forgetting the clobbered inode must not unmap the survivor.
    inodes.forget(victim, 1);
    assert_eq!(inodes.intern("Skyrim.ini"), src, "forget() unmapped a live inode");
}

#[test]
fn renaming_a_directory_rebinds_its_children() {
    let mut inodes = Inodes::new();
    let child = inodes.lookup("tools/xedit.exe");
    let grandchild = inodes.lookup("tools/sub/deep.txt");
    inodes.rename("tools", "tools_bak");

    assert_eq!(inodes.path(child).as_deref(), Some("tools_bak/xedit.exe"));
    assert_eq!(inodes.path(grandchild).as_deref(), Some("tools_bak/sub/deep.txt"));
    // And they resolve from the new path without minting fresh inodes.
    assert_eq!(inodes.intern("tools_bak/xedit.exe"), child);
    assert_eq!(inodes.intern("tools_bak/sub/deep.txt"), grandchild);
}

#[test]
fn intern_is_stable_and_uncounted() {
    let mut inodes = Inodes::new();
    let a = inodes.intern("foo/bar");
    let b = inodes.intern("foo/bar");
    assert_eq!(a, b); // same path -> same inode
    // readdir interns without taking a kernel reference.
    assert!(!inodes.counts.contains_key(&a));
    assert_eq!(inodes.path(a).as_deref(), Some("foo/bar"));
}

#[test]
fn lookup_counts_and_forget_frees_at_zero() {
    let mut inodes = Inodes::new();
    let ino = inodes.lookup("a.esp");
    let _ = inodes.lookup("a.esp"); // a second kernel reference, same inode
    assert_eq!(inodes.counts[&ino], 2);

    inodes.forget(ino, 1);
    assert!(inodes.path(ino).is_some()); // still referenced once
    inodes.forget(ino, 1);
    assert!(inodes.path(ino).is_none()); // freed at zero
    assert!(!inodes.by_path.contains_key("a.esp"));
}

#[test]
fn inode_numbers_are_never_reused() {
    let mut inodes = Inodes::new();
    let first = inodes.lookup("x");
    inodes.forget(first, 1); // freed
    let second = inodes.lookup("x"); // same path, fresh lookup
    // Monotonic allocation: a freed number is never handed out again, so the
    // kernel (ino, generation) pair stays unambiguous with generation == 0.
    assert_ne!(first, second);
}

#[test]
fn forget_more_than_held_does_not_underflow() {
    let mut inodes = Inodes::new();
    let ino = inodes.lookup("y");
    inodes.forget(ino, 1_000); // kernel over-forgets at unmount race
    assert!(inodes.path(ino).is_none());
}

#[test]
fn root_is_never_forgotten() {
    let mut inodes = Inodes::new();
    inodes.forget(ROOT_INO, 1_000);
    assert_eq!(inodes.path(ROOT_INO).as_deref(), Some(""));
}

#[test]
fn rename_rebinds_inode_keeping_number() {
    let mut inodes = Inodes::new();
    let ino = inodes.lookup("save.tmp");
    inodes.rename("save.tmp", "save.ess");
    assert_eq!(inodes.path(ino).as_deref(), Some("save.ess"));
    assert!(!inodes.by_path.contains_key("save.tmp"));
    assert_eq!(inodes.by_path.get("save.ess"), Some(&ino));
}

#[test]
fn timings_are_absent_until_something_is_measured() {
    // A run without EIDOS_FUSE_STATS records no time, and the report must
    // then say nothing rather than print a row of confident zeroes - which
    // would read as "the filesystem cost nothing", a claim the run did not
    // make.
    let s = Stats::default();
    assert_eq!(s.timings(&eidos_core::ResolveStats::default()), "");
}

#[test]
fn timings_report_milliseconds_and_a_total() {
    let s = Stats::default();
    s.ns_read.fetch_add(240_000_000, Ordering::Relaxed); // 240 ms
    s.ns_lookup.fetch_add(60_000_000, Ordering::Relaxed); // 60 ms
    let out = s.timings(&eidos_core::ResolveStats::default());
    assert!(out.contains("300 ms"), "{out}");
    // The wording is load-bearing. Both totals are sums over concurrent
    // worker threads, and saying so is what stops the next reader treating
    // them as elapsed time - or dividing one by the other, which is how a
    // "960%" got printed.
    assert!(out.contains("summed across threads"), "{out}");
    assert!(!out.contains('%'), "no ratio between two things that do not nest: {out}");
    assert!(out.contains("read 240"), "{out}");
    assert!(out.contains("lookup 60"), "{out}");
    // Handlers that never ran are still listed, at zero: their absence is
    // information too.
    assert!(out.contains("write 0"), "{out}");
}

#[test]
fn the_timer_charges_the_early_returns_too() {
    // Several handlers bail before doing the work - ENOENT, a cache hit, a
    // refused write. A stopwatch stopped only on the success path would
    // report exactly the cheap cases as free and skew the total the other
    // way from the truth.
    let total = AtomicU64::new(0);
    {
        let _t = Timed { total: &total, start: std::time::Instant::now() };
        std::thread::sleep(std::time::Duration::from_millis(2));
        // falls out of scope on an early return, exactly like a handler
    }
    assert!(total.load(Ordering::Relaxed) >= 1_000_000, "at least 1ms was charged");
}


#[test]
fn this_crate_never_makes_a_name_appear_or_vanish_by_itself() {
    // The invariant step 1 exists to establish: `LayerStack` owns every name
    // in the overwrite, so there is exactly ONE place that has to be told
    // when one appears - which is what makes a cache or an index of that
    // layer tractable at all.
    //
    // Three handlers used to breach it with their own `OpenOptions`, and
    // nothing would have noticed a fourth. This test notices. It reads this
    // crate's own source, which is blunt but is the only thing that actually
    // enforces an architectural rule rather than describing one.
    //
    // The needles are assembled from fragments so this test does not match
    // itself.
    let src = include_str!("lib.rs");
    let needles = [
        concat!("fs::remove_", "file"),
        concat!("fs::remove_", "dir"),
        concat!("fs::", "rename"),
        concat!("fs::create_", "dir"),
        concat!("File::", "create"),
        concat!("unix::fs::", "symlink"),
        concat!("create", "(true)"),
        concat!("fs::hard_", "link"),
    ];
    let mut found = Vec::new();
    for (n, line) in src.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        for needle in needles {
            if code.contains(needle) {
                found.push(format!("line {}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a name-creating filesystem call has appeared in eidos-fuse. It belongs \
         on LayerStack, beside create_truncated / create_symlink / set_len, so \
         that one layer knows about every name in the overwrite:\n  {}",
        found.join("\n  ")
    );
}
