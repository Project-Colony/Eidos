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
    let (_, clobbered) = inodes.rename("Skyrim.ini.tmp", "Skyrim.ini");

    assert_eq!(
        inodes.intern("Skyrim.ini"),
        src,
        "the renamed inode owns the path"
    );
    // The victim is REPORTED, because its FORGET can no longer do the reporting:
    // discard removed its count, so forget() finds nothing and frees nothing,
    // and the side tables keyed by it (aliases, negatives) would have been
    // retained for the life of the mount - on the very pattern every INI and
    // save write uses.
    assert_eq!(
        clobbered,
        vec![victim],
        "the clobbered inode must be handed back for pruning"
    );
    // Forgetting the clobbered inode must not unmap the survivor.
    inodes.forget(victim, 1);
    assert_eq!(
        inodes.intern("Skyrim.ini"),
        src,
        "forget() unmapped a live inode"
    );
}

#[test]
fn renaming_a_directory_rebinds_its_children() {
    let mut inodes = Inodes::new();
    let child = inodes.lookup("tools/xedit.exe");
    let grandchild = inodes.lookup("tools/sub/deep.txt");
    inodes.rename("tools", "tools_bak");

    assert_eq!(inodes.path(child).as_deref(), Some("tools_bak/xedit.exe"));
    assert_eq!(
        inodes.path(grandchild).as_deref(),
        Some("tools_bak/sub/deep.txt")
    );
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
    // "960%" got printed. The COUNT is there because it is what an A/B run
    // varies and what the report could not otherwise show: EIDOS_FUSE_THREADS
    // is an environment variable, so it leaves no trace in the recorded
    // command line, and two dumps from two arms used to be indistinguishable.
    assert!(out.contains("summed across"), "{out}");
    assert!(
        out.contains(&format!(
            "across {} thread(s)",
            crate::config::fuse_threads()
        )),
        "the report must name the thread count it summed over: {out}"
    );
    assert!(
        !out.contains('%'),
        "no ratio between two things that do not nest: {out}"
    );
    assert!(out.contains("read 240"), "{out}");
    assert!(out.contains("lookup 60"), "{out}");
    // Handlers that never ran are still listed, at zero: their absence is
    // information too.
    assert!(out.contains("write 0"), "{out}");
}

// ---- The read-shape survey -------------------------------------------------
//
// The survey exists to answer four questions the totals cannot: what sizes the
// game asks for, whether the traffic arrives in bursts, which files carry it,
// and which threads issue it. These pin each answer.

/// Names for the inodes the survey tests use.
fn names(ino: u64) -> Option<String> {
    match ino {
        1 => Some("Skyrim - Textures0.bsa".into()),
        2 => Some("textures/armor/steel.dds".into()),
        _ => None,
    }
}

#[test]
fn read_sizes_land_in_the_bucket_they_belong_to() {
    let s = Stats::default();
    // Exactly on a bound belongs to that bucket, not the next one up.
    s.note_read(1, 4 * 1024, 10, 0, 0, 0);
    s.note_read(1, 4 * 1024 + 1, 10, 0, 0, 0);
    s.note_read(1, 128 * 1024, 10, 0, 0, 0);
    s.note_read(1, 4 * 1024 * 1024, 10, 0, 0, 0);
    let out = s.read_shape(&names);
    assert!(out.contains("<=4K 25.0%"), "{out}");
    assert!(out.contains("<=16K 25.0%"), "{out}");
    assert!(out.contains("<=128K 25.0%"), "{out}");
    assert!(out.contains(">1024K 25.0%"), "{out}");
    // Empty buckets are omitted rather than printed as zeroes: the shape is the
    // point, and a row of "0.0%" hides it.
    assert!(!out.contains("<=32K"), "{out}");
}

#[test]
fn the_timeline_reports_the_peak_slot_and_how_full_it_was() {
    let s = Stats::default();
    // One slot carrying real handler time is the signature of a burst. The
    // percentage is against the worker pool, because that is what decides
    // whether anyone actually waited on us.
    for _ in 0..300 {
        s.note_read(1, 128 * 1024, 10, 100_000, 0, 0); // 0.1 ms each, 30 ms total
    }
    let out = s.read_shape(&names);
    assert!(out.contains("busiest 300 reads"), "{out}");
    assert!(out.contains("heaviest slot 30 ms"), "{out}");
    assert!(out.contains("slots with >=50 reads: 1"), "{out}");
    // 30 ms of handler time inside a 100 ms slot, over N worker threads.
    let expect = 30.0 * 100.0 / (100.0 * crate::config::fuse_threads() as f64);
    assert!(out.contains(&format!("= {expect:.1}% of")), "{out}");
}

#[test]
fn reads_are_attributed_to_files_and_named_only_at_report_time() {
    let s = Stats::default();
    // Keyed by inode on the hot path - resolving a name per read would cost an
    // allocation and a lock on the very path being measured.
    for _ in 0..10 {
        s.note_read(1, 64 * 1024, 10, 0, 0, 0);
    }
    s.note_read(2, 8 * 1024, 10, 0, 0, 0);
    let out = s.read_shape(&names);
    let hot = out
        .find("Skyrim - Textures0.bsa")
        .expect("named at report time");
    let cold = out
        .find("textures/armor/steel.dds")
        .expect("second file listed");
    assert!(hot < cold, "busiest file must lead: {out}");
    assert!(out.contains("0.6 MiB"), "bytes per file reported: {out}");
    // An inode that has since been forgotten is still counted, and says so
    // rather than silently dropping its reads from the total.
    s.note_read(99, 1024, 10, 0, 0, 0);
    assert!(s.read_shape(&names).contains("(inode 99, gone)"));
}

#[test]
fn the_survey_says_which_threads_issued_the_reads() {
    let s = Stats::default();
    // The question this answers: is a game blocking ONE thread on I/O - the
    // shape that can stall a frame - or streaming across a pool?
    for _ in 0..9 {
        s.note_read(1, 4096, 777, 0, 0, 0);
    }
    s.note_read(1, 4096, 888, 0, 0, 0);
    let out = s.read_shape(&names);
    assert!(out.contains("2 distinct"), "{out}");
    assert!(out.contains("busiest holds 90.0%"), "{out}");
}

#[test]
fn an_unmeasured_run_reports_no_read_shape_at_all() {
    // The guard is at TimedRead::start, so with stats off nothing is ever
    // recorded and the report must stay silent rather than print empty tables.
    let s = Stats::default();
    assert_eq!(s.read_shape(&names), "");
    assert!(TimedRead::start(&s, 1, 4096, 10).is_none() || *crate::stats::STATS_ON);
}

#[test]
fn surveying_a_read_does_not_inflate_the_time_it_reports() {
    // The trap this locks out: if the bookkeeping ran before the stopwatch was
    // read, the survey would show up inside ns_read and make reads look more
    // expensive the moment you started measuring them.
    let s = Stats::default();
    s.note_read(1, 4096, 10, 5_000_000, 0, 0); // 5 ms
    assert_eq!(
        s.ns_read.load(Ordering::Relaxed),
        0,
        "note_read must not touch the clock"
    );
}

#[test]
fn the_timer_charges_the_early_returns_too() {
    // Several handlers bail before doing the work - ENOENT, a cache hit, a
    // refused write. A stopwatch stopped only on the success path would
    // report exactly the cheap cases as free and skew the total the other
    // way from the truth.
    let total = AtomicU64::new(0);
    {
        let _t = Timed {
            total: &total,
            start: std::time::Instant::now(),
        };
        std::thread::sleep(std::time::Duration::from_millis(2));
        // falls out of scope on an early return, exactly like a handler
    }
    assert!(
        total.load(Ordering::Relaxed) >= 1_000_000,
        "at least 1ms was charged"
    );
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

#[test]
fn a_slot_can_never_report_more_time_than_its_workers_could_spend() {
    // The shapes that broke the two earlier attempts, all in one test.
    //
    // A read is charged to a slice by real OVERLAP. Charging its whole duration
    // to the slice it started in printed 500% for four workers blocked 500 ms;
    // spreading it evenly over the slices it spans fixed that and still printed
    // 180% for two 90 ms reads per worker, because those straddle a boundary the
    // even split cannot see. Each case below is checked at its own start offset.
    let pct = |out: &str| -> f64 {
        out.split("= ")
            .nth(1)
            .and_then(|x| x.split('%').next())
            .and_then(|x| x.parse().ok())
            .unwrap_or(f64::NAN)
    };
    let threads = crate::config::fuse_threads() as u64;

    // 1. Long reads, aligned: four workers blocked for five whole slices.
    let s = Stats::default();
    for tid in 0..threads as u32 {
        s.note_read(1, 128 * 1024, tid, 500_000_000, 0, 0);
    }
    let p = pct(&s.read_shape(&names));
    assert!(p <= 100.5, "aligned long reads exceeded capacity: {p}");
    assert!(p > 95.0, "blocked workers ARE saturation, got {p}");

    // 2. Sub-slice reads starting mid-slice: they straddle, and the straddle is
    //    the whole point - 50 ms into the slice, two 90 ms reads each.
    let s = Stats::default();
    for tid in 0..threads as u32 {
        s.note_read(1, 4096, tid, 90_000_000, 0, 50_000_000);
        s.note_read(1, 4096, tid, 90_000_000, 0, 50_000_000);
    }
    let p = pct(&s.read_shape(&names));
    assert!(p <= 100.5, "straddling reads exceeded capacity: {p}");

    // 3. A duration just over one slice, from the slice boundary.
    let s = Stats::default();
    for tid in 0..threads as u32 {
        s.note_read(1, 4096, tid, 100_999_999, 0, 0);
    }
    let p = pct(&s.read_shape(&names));
    assert!(
        p <= 100.5,
        "a barely-over-one-slice read exceeded capacity: {p}"
    );
}

#[test]
fn the_peak_points_at_a_slot_that_saw_reads() {
    // With every duration zero, ranking slots by time alone is one long tie, and
    // `max_by_key` keeps the LAST - so the report pointed at the end of the hour
    // for traffic that all arrived in the first second.
    let s = Stats::default();
    for _ in 0..1000 {
        s.note_read(1, 4096, 10, 0, 3, 0); // t = +0.3 s
    }
    let out = s.read_shape(&names);
    assert!(
        out.contains("at t=+0.3s"),
        "peak must sit where the reads are: {out}"
    );
    assert!(out.contains("busiest 1000 reads"), "{out}");
}
