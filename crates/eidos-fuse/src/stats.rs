//! Per-operation counters and timings, printed at unmount when
//! `EIDOS_FUSE_STATS=1`. Pure observability: nothing here changes behaviour.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;



/// Per-mount operation counters, for answering "where did the time go" with data
/// instead of a guess.
///
/// A metadata storm is invisible from outside: the process looks idle (almost no
/// bytes read) while the kernel and the game trade millions of `lookup`/`getattr`
/// round-trips. These counters make the shape of a run legible - in particular
/// the ratio of misses to hits, which is what the negative-dentry cache exists to
/// collapse. Relaxed atomics on a handful of counters cost nothing next to the
/// syscalls each op already performs. Dumped at unmount when `EIDOS_FUSE_STATS`
/// is set.
#[derive(Default)]
pub(crate) struct Stats {
    pub(crate) lookup_hit: AtomicU64,
    pub(crate) lookup_miss: AtomicU64,
    pub(crate) getattr: AtomicU64,
    /// Directory OPENS. Counted separately from `readdir` since a measured session
    /// reported 516301 of these against 26999 reads - and the counter used to live
    /// in `opendir` while being NAMED `readdir`, which made the number unreadable:
    /// it was taken for half a million enumerations when almost none of them
    /// enumerated anything. See `probe`.
    pub(crate) opendir: AtomicU64,
    pub(crate) releasedir: AtomicU64,
    /// Directory handles closed without a single `readdir` - opened purely to look
    /// at the directory inode, never to list it. Wine does this on every failed
    /// path resolution to ask whether the directory folds case. This counter is the
    /// only honest measure of how much of `opendir` is probing rather than
    /// enumerating; ratios against `lookup` cannot tell them apart, because the
    /// negative-dentry cache absorbs the lookups while every directory open still
    /// reaches us (the kernel has no cache for those).
    pub(crate) probe: AtomicU64,
    /// Enumeration requests, at last counted in `readdir` itself.
    pub(crate) readdir: AtomicU64,
    /// Merged listings actually built - a multi-layer walk plus an NTFS-collation
    /// sort. Against `dir_hit` this gives the by-path cache's hit rate.
    pub(crate) snapshot: AtomicU64,
    pub(crate) dir_hit: AtomicU64,
    /// `.ciopfs` marker lookups: how often Wine asks whether a directory folds
    /// case. Expected to sit orders of magnitude BELOW `opendir`, because the
    /// answer is dentry-cached while the open that precedes it is not.
    pub(crate) marker: AtomicU64,
    pub(crate) open: AtomicU64,
    pub(crate) read: AtomicU64,
    /// Nanoseconds spent INSIDE each handler, summed. Counters alone cannot
    /// answer the only question that matters when a load feels slow - "how much
    /// of it is us" - because an operation count is not a duration, and the two
    /// do not even rank the same: `read` outnumbers `readdir` fifty to one and
    /// may still cost less, a merged listing being a multi-layer walk plus a
    /// sort while a read is a `pread` on an already-open handle.
    ///
    /// Measured only while `EIDOS_FUSE_STATS` is set. Two clock reads per
    /// operation is roughly 40ns against handlers measured at 2.8-7.3µs, so
    /// about 1% - visible in these numbers, absent from every normal run.
    pub(crate) ns_lookup: AtomicU64,
    pub(crate) ns_getattr: AtomicU64,
    pub(crate) ns_open: AtomicU64,
    pub(crate) ns_read: AtomicU64,
    pub(crate) ns_readdir: AtomicU64,
    pub(crate) ns_write: AtomicU64,
    pub(crate) write: AtomicU64,
    /// How many times each directory was opened, by path.
    ///
    /// The totals say a session opened directories half a million times. They
    /// cannot say whether that is five hundred directories opened a thousand
    /// times each or half a million opened once - and those two shapes call for
    /// OPPOSITE fixes. A few hot directories means the caller is asking the same
    /// question over and over and the answer is to stop it asking; a long flat
    /// tail means the work is real and the answer is to serve each one faster.
    /// Guessing wrong costs weeks, so this measures it.
    ///
    /// Only populated when `EIDOS_FUSE_STATS` is set: on the hot path this is
    /// one relaxed atomic load, and the map is never touched otherwise.
    pub(crate) dirs: Mutex<HashMap<String, u64>>,
    /// Opens whose path did not fit under [`DIR_HISTOGRAM_CAP`], so the top-N
    /// stays honest about what it is not showing.
    pub(crate) dirs_overflow: AtomicU64,
}

/// Distinct directories the histogram will hold before it stops learning new
/// ones. A pathological session must not turn a diagnostic into an OOM; at this
/// size the map is a few tens of MB and the shape is already unambiguous.
pub(crate) const DIR_HISTOGRAM_CAP: usize = 200_000;

/// Whether the per-directory histogram is being collected. Read once, so the
/// cost on a path taken half a million times a session is an atomic load and
/// not a `getenv`.
pub(crate) static STATS_ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(Stats::enabled);

/// Adds the time a handler took to `total`, on the way out.
///
/// A guard rather than a pair of calls because several handlers return early -
/// on ENOENT, on a cache hit, on a refused write - and a stopwatch that only
/// stops on the success path would report the fast cases as free.
pub(crate) struct Timed<'a> {
    pub(crate) total: &'a AtomicU64,
    pub(crate) start: std::time::Instant,
}

impl Drop for Timed<'_> {
    fn drop(&mut self) {
        self.total.fetch_add(self.start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }
}

impl Stats {
    pub(crate) fn bump(c: &AtomicU64) {
        c.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn enabled() -> bool {
        std::env::var("EIDOS_FUSE_STATS").is_ok_and(|v| v != "0")
    }

    /// Note one directory open against its path.
    pub(crate) fn note_dir(&self, path: &str) {
        if !*STATS_ON {
            return;
        }
        let mut m = match self.dirs.lock() {
            Ok(m) => m,
            Err(p) => p.into_inner(),
        };
        if let Some(c) = m.get_mut(path) {
            *c += 1;
        } else if m.len() < DIR_HISTOGRAM_CAP {
            m.insert(path.to_string(), 1);
        } else {
            Stats::bump(&self.dirs_overflow);
        }
    }

    /// The shape of the directory opens: how many distinct directories, how
    /// concentrated they are, and which ones dominate.
    pub(crate) fn dir_shape(&self) -> String {
        if !*STATS_ON {
            return String::new();
        }
        let m = match self.dirs.lock() {
            Ok(m) => m,
            Err(p) => p.into_inner(),
        };
        if m.is_empty() {
            return String::new();
        }
        let mut v: Vec<(&String, u64)> = m.iter().map(|(k, &c)| (k, c)).collect();
        v.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let total: u64 = v.iter().map(|(_, c)| c).sum();
        // Where the opens actually live. If a handful of directories carry most
        // of them, the caller is repeating itself and the fix is upstream.
        let share = |n: usize| -> f64 {
            let s: u64 = v.iter().take(n).map(|(_, c)| c).sum();
            if total == 0 {
                0.0
            } else {
                s as f64 * 100.0 / total as f64
            }
        };
        let top: Vec<String> =
            v.iter().take(15).map(|(p, c)| format!("  {c:>9}  /{p}")).collect();
        let dropped = self.dirs_overflow.load(Ordering::Relaxed);
        let note = if dropped > 0 {
            format!(
                "\n  (histogram capped at {DIR_HISTOGRAM_CAP} directories; {dropped} opens of \
                 further directories were counted in the total but not listed)"
            )
        } else {
            String::new()
        };
        format!(
            "\neidos-fuse directory opens by path: {} distinct, {total} opens\n  \
             top 1 = {:.1}%, top 10 = {:.1}%, top 100 = {:.1}%, top 1000 = {:.1}%\n{}{}",
            v.len(),
            share(1),
            share(10),
            share(100),
            share(1000),
            top.join("\n"),
            note,
        )
    }

    pub(crate) fn report(&self, r: &eidos_core::ResolveStats) -> String {
        let g = |c: &AtomicU64| c.load(Ordering::Relaxed);
        let (hit, miss) = (g(&self.lookup_hit), g(&self.lookup_miss));
        let total = hit + miss;
        let miss_pct = if total == 0 { 0.0 } else { miss as f64 * 100.0 / total as f64 };
        let (od, probe) = (g(&self.opendir), g(&self.probe));
        let probe_pct = if od == 0 { 0.0 } else { probe as f64 * 100.0 / od as f64 };
        let (snap, dhit) = (g(&self.snapshot), g(&self.dir_hit));
        let builds = snap + dhit;
        let hit_pct = if builds == 0 { 0.0 } else { dhit as f64 * 100.0 / builds as f64 };
        format!(
            "eidos-fuse stats: lookup {total} ({miss} missing, {miss_pct:.1}%), \
             getattr {}, opendir {od} ({probe} probe-only, {probe_pct:.1}%), releasedir {}, \
             readdir {}, listing {builds} ({dhit} cached, {hit_pct:.1}%), marker {}, \
             open {}, read {}, write {}",
            g(&self.getattr),
            g(&self.releasedir),
            g(&self.readdir),
            g(&self.marker),
            g(&self.open),
            g(&self.read),
            g(&self.write),
        ) + &self.timings(r)
            + &self.dir_shape()
    }
}

impl Stats {
    /// Where the time actually went, in milliseconds.
    ///
    /// The point of the whole counter set: an operation count says how often the
    /// kernel came to us, never how long it waited. Empty when the run predates
    /// timing or when every handler was too fast to register, which is itself an
    /// answer - it means the filesystem is not where the seconds are.
    pub(crate) fn timings(&self, r: &eidos_core::ResolveStats) -> String {
        let g = |c: &AtomicU64| c.load(Ordering::Relaxed);
        let ms = |ns: u64| ns as f64 / 1_000_000.0;
        let parts = [
            ("lookup", g(&self.ns_lookup)),
            ("getattr", g(&self.ns_getattr)),
            ("open", g(&self.ns_open)),
            ("read", g(&self.ns_read)),
            ("readdir", g(&self.ns_readdir)),
            ("write", g(&self.ns_write)),
        ];
        let total: u64 = parts.iter().map(|(_, n)| n).sum();
        if total == 0 {
            return String::new();
        }
        let each: Vec<String> =
            parts.iter().map(|(n, ns)| format!("{n} {:.0}", ms(*ns))).collect();
        // The counts lead, because they are the only exact figures here. Both
        // millisecond totals are sums over concurrent FUSE worker threads, so
        // neither is wall-clock, and resolution is NOT a subset of the handlers
        // above - it also runs while listings are built. An earlier version
        // divided one by the other and printed "960%", which is what a ratio
        // between two things that do not nest looks like.
        format!(
            "\n  time in handlers: {:.0} ms, summed across threads ({})\
             \n  path resolution: {} probes, {} directory scans, {:.0} ms (also summed)",
            ms(total),
            each.join(", "),
            r.probes.load(Ordering::Relaxed),
            r.scans.load(Ordering::Relaxed),
            ms(r.ns.load(Ordering::Relaxed)),
        )
    }
}
