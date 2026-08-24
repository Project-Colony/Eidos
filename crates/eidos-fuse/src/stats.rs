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
    /// The shape of the read traffic - see [`ReadShape`]. One mutex, taken once
    /// per read and only while `EIDOS_FUSE_STATS` is set.
    pub(crate) reads: Mutex<ReadShape>,
    /// When the first read arrived, which is where the timeline starts.
    ///
    /// Outside the mutex on purpose: the slot index is computed from it BEFORE
    /// the lock is taken, so waiting for the lock cannot push a read into a later
    /// slot. It could - contention rises exactly during a burst, so the timeline
    /// would have smeared the bursts it exists to find, biasing its own headline
    /// downward by the thing being measured.
    ///
    /// Anchored at the first READ rather than at daemon construction, because a
    /// launch spends 30-120 s in Proton and the main menu before the load order
    /// streams, and that would have burned a chunk of the timeline on an idle
    /// mount.
    pub(crate) read_origin: std::sync::OnceLock<std::time::Instant>,
}

/// Upper bounds of the read-size buckets, in bytes; the last is the catch-all.
///
/// THESE ARE FUSE REQUEST SIZES, NOT THE GAME'S. What arrives here is what the
/// kernel decided to ask for after readahead, coalescing and serving whatever the
/// page cache already held - so it is neither the size the game passed to
/// `read()` nor the number of calls it made. Measured on a real mount: 192 preads
/// of exactly 64 KiB arrived as 101 requests averaging 126 KiB, i.e. half the
/// calls at twice the size, plus a little more data than was asked for.
///
/// The distinction is not pedantry. It is tempting to read a bucket full of 4 KiB
/// entries as "the game issues tiny reads, so the per-round-trip tax dominates" -
/// and that conclusion does not follow, because page-cache-miss granularity
/// produces the same shape. What this histogram DOES answer is what this daemon
/// is asked to serve, which is the right input for sizing buffers and judging the
/// round-trip cost we actually pay. Seeing the game's own request sizes needs
/// strace on the game side; the daemon structurally cannot.
pub(crate) const READ_BUCKETS: [u32; 8] =
    [4 << 10, 16 << 10, 32 << 10, 64 << 10, 128 << 10, 256 << 10, 1 << 20, u32::MAX];

/// Width of one timeline slot, in milliseconds.
pub(crate) const SLOT_MS: u64 = 100;

/// Timeline slots kept: 100 ms x 36000 = ONE HOUR, and it is a prefix, not a
/// ring - reads past the end are counted separately rather than wrapping, so a
/// long session cannot quietly overwrite the peak it was run to find.
///
/// An hour rather than the ten minutes this started at, because the survey exists
/// to catch bursts at a cell transition and in a real session those happen at
/// minute 20 or 40, not minute 3 - past the end the timeline simply stopped
/// recording while the other tables kept counting, which is the worst kind of
/// silent gap. A `Slot` is 24 bytes, so an hour costs 864 KB in a daemon that
/// already caches merged listings for the whole game tree.
pub(crate) const TIMELINE_SLOTS: usize = 36_000;

/// Distinct files the per-file survey learns before it stops taking new ones.
pub(crate) const HOT_FILES_CAP: usize = 50_000;

/// Distinct calling threads the survey learns. Same reasoning as
/// [`HOT_FILES_CAP`]: Skyrim's thread count is bounded, but a caller that spawns
/// a thread per read would otherwise grow this map without limit, and a
/// diagnostic must not be able to OOM the daemon that is serving the game.
pub(crate) const TID_CAP: usize = 4_096;

/// One 100 ms slice of the session.
#[derive(Clone, Copy, Default)]
pub(crate) struct Slot {
    pub(crate) reads: u32,
    pub(crate) bytes: u64,
    /// Handler nanoseconds spent in this slice, summed across worker threads.
    pub(crate) ns: u64,
}

/// What the read traffic actually looks like, as opposed to what it averages to.
///
/// The totals already say a session spends N milliseconds in `read`. They cannot
/// say whether that was a steady trickle or a wall arriving during a cell
/// transition, and those two shapes mean opposite things: a trickle spread over
/// idle threads cannot stall a frame, while a burst that saturates every worker
/// can. An average of 170 reads a second hides both. This records the timeline,
/// the request sizes, which files carry the traffic, and which threads issue it.
pub(crate) struct ReadShape {
    /// Counts per [`READ_BUCKETS`] bucket.
    pub(crate) sizes: [u64; READ_BUCKETS.len()],
    /// The timeline, allocated on first use so an unmeasured run pays nothing.
    pub(crate) slots: Vec<Slot>,
    /// Reads that arrived after the timeline's hour was up.
    pub(crate) past_timeline: u64,
    /// Reads and bytes per inode. Keyed by inode because that is free on the hot
    /// path; the names are resolved once, at report time.
    pub(crate) per_file: HashMap<u64, (u64, u64)>,
    pub(crate) files_overflow: u64,
    /// Reads per calling thread. FUSE reports the caller's TID in the request
    /// header, so this says whether the traffic comes from one thread - a game
    /// blocking its own render loop on I/O - or from a streaming pool.
    pub(crate) per_tid: HashMap<u32, u64>,
    pub(crate) tids_overflow: u64,
    /// Bytes REQUESTED, summed. Not bytes delivered: the count is taken before
    /// the read runs, so a request truncated by EOF is charged in full. The
    /// difference only shows at file ends and never changes a ratio.
    pub(crate) bytes: u64,
}

impl Default for ReadShape {
    fn default() -> Self {
        ReadShape {
            sizes: [0; READ_BUCKETS.len()],
            slots: Vec::new(),
            past_timeline: 0,
            per_file: HashMap::new(),
            files_overflow: 0,
            per_tid: HashMap::new(),
            tids_overflow: 0,
            bytes: 0,
        }
    }
}

/// Times a read AND surveys its shape, on the way out.
///
/// Separate from [`Timed`] because the survey must not pay into the number it
/// exists to explain: the elapsed time is read FIRST, `ns_read` is credited with
/// exactly that, and only then does the bookkeeping run. Measuring a thing must
/// not be the reason the thing looks expensive.
pub(crate) struct TimedRead<'a> {
    stats: &'a Stats,
    ino: u64,
    size: u32,
    tid: u32,
    start: std::time::Instant,
}

impl<'a> TimedRead<'a> {
    /// `None` when `EIDOS_FUSE_STATS` is unset, so a normal run pays one atomic
    /// load for the whole survey and touches none of the maps.
    pub(crate) fn start(stats: &'a Stats, ino: u64, size: u32, tid: u32) -> Option<TimedRead<'a>> {
        STATS_ON.then(|| TimedRead { stats, ino, size, tid, start: std::time::Instant::now() })
    }
}

impl Drop for TimedRead<'_> {
    fn drop(&mut self) {
        let ns = self.start.elapsed().as_nanos() as u64;
        self.stats.ns_read.fetch_add(ns, Ordering::Relaxed);
        // The slot is chosen from when this read STARTED, and computed here,
        // outside the mutex. Both halves matter: charging a read to the slot it
        // ended in let a read spanning several slices dump all its time into the
        // last one, which can push a slot's total past its own wall-clock
        // capacity and print an impossible percentage - the same family as the
        // "960%" the comment in `timings` was written to prevent.
        let origin = *self.stats.read_origin.get_or_init(|| self.start);
        let since = self.start.saturating_duration_since(origin).as_nanos() as u64;
        let slot_ns = SLOT_MS * 1_000_000;
        self.stats.note_read(self.ino, self.size, self.tid, ns, (since / slot_ns) as usize, since % slot_ns);
    }
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

    /// Record one read against the survey.
    ///
    /// No `STATS_ON` check here, deliberately: the only caller is
    /// [`TimedRead::drop`], and [`TimedRead::start`] already returns `None` when
    /// stats are off, so on a normal run this function does not exist at
    /// runtime. Putting the gate at the construction site rather than here is
    /// also what lets a test drive the survey directly.
    pub(crate) fn note_read(&self, ino: u64, size: u32, tid: u32, ns: u64, slot: usize, offset_ns: u64) {
        let mut s = match self.reads.lock() {
            Ok(s) => s,
            Err(p) => p.into_inner(),
        };
        let b = READ_BUCKETS.iter().position(|&hi| size <= hi).unwrap_or(READ_BUCKETS.len() - 1);
        s.sizes[b] += 1;
        s.bytes += size as u64;

        if s.slots.is_empty() {
            s.slots = vec![Slot::default(); TIMELINE_SLOTS];
        }
        if slot >= TIMELINE_SLOTS {
            s.past_timeline += 1;
        } else {
            // The count and the bytes belong to the slot the read STARTED in.
            let start = &mut s.slots[slot];
            start.reads += 1;
            start.bytes += size as u64;

            // The TIME is deposited by REAL OVERLAP: what this read occupied of
            // the slice it began in, whole slices after that, and the remainder
            // in the last one. `offset_ns` is where inside its first slice the
            // read started, and it is what makes the sum honest.
            //
            // Two earlier attempts got this wrong in the same direction. Charging
            // the whole duration to the starting slice printed "500% of 4
            // worker-thread(s)" for four workers each blocked 500 ms - a number
            // of the same family as the "960%" the comment in `timings` warns
            // about. Spreading it evenly over `ceil(duration / slice)` slices
            // fixed that case and still ignored the offset, so two 90 ms reads per
            // worker - which really straddle two slices - printed 180%. Only
            // overlap bounds a slice by its own wall clock, which is the property
            // that makes the percentage mean "how much of the pool was busy".
            let mut left = ns;
            let mut k = 0usize;
            let mut room = SLOT_MS * 1_000_000 - offset_ns.min(SLOT_MS * 1_000_000);
            while left > 0 {
                let Some(sl) = s.slots.get_mut(slot + k) else { break }; // off the end
                let take = left.min(room);
                sl.ns += take;
                left -= take;
                k += 1;
                room = SLOT_MS * 1_000_000;
            }
        }

        let room = s.per_file.len() < HOT_FILES_CAP;
        match s.per_file.get_mut(&ino) {
            Some(e) => {
                e.0 += 1;
                e.1 += size as u64;
            }
            None if room => {
                s.per_file.insert(ino, (1, size as u64));
            }
            None => s.files_overflow += 1,
        }

        let tid_room = s.per_tid.len() < TID_CAP;
        match s.per_tid.get_mut(&tid) {
            Some(c) => *c += 1,
            None if tid_room => {
                s.per_tid.insert(tid, 1);
            }
            None => s.tids_overflow += 1,
        }
    }

    /// The shape of the read traffic: sizes, bursts, threads, and which files
    /// carry it.
    ///
    /// `path_of` resolves an inode to its virtual path. It is a closure rather
    /// than a borrow of the inode table so this module keeps knowing nothing
    /// about inodes, and it runs once per listed file at unmount - never on the
    /// hot path.
    pub(crate) fn read_shape(&self, path_of: &dyn Fn(u64) -> Option<String>) -> String {
        let s = match self.reads.lock() {
            Ok(s) => s,
            Err(p) => p.into_inner(),
        };
        // Silent when nothing was surveyed, which is also what an unmeasured run
        // looks like - so no gate on the environment is needed to stay quiet.
        let total: u64 = s.sizes.iter().sum();
        if total == 0 {
            return String::new();
        }
        let mib = |b: u64| b as f64 / (1024.0 * 1024.0);
        let pct = |n: u64| n as f64 * 100.0 / total as f64;

        let label = |i: usize| -> String {
            let hi = READ_BUCKETS[i];
            if hi == u32::MAX {
                format!(">{}K", READ_BUCKETS[i - 1] / 1024)
            } else {
                format!("<={}K", hi / 1024)
            }
        };
        let sizes: Vec<String> = (0..READ_BUCKETS.len())
            .filter(|&i| s.sizes[i] > 0)
            .map(|i| format!("{} {:.1}%", label(i), pct(s.sizes[i])))
            .collect();

        // The peak slot, and how much of the available worker time it used. THIS
        // is the burst answer: a slot at 5% cannot have stalled anything, while
        // one approaching 100% means every worker was blocked for that tenth of
        // a second and whoever was waiting on us waited too.
        let threads = crate::config::fuse_threads() as f64;
        let slot_capacity_ns = SLOT_MS as f64 * 1_000_000.0 * threads;
        let non_empty = s.slots.iter().filter(|x| x.reads > 0).count();
        // Only slots that actually SAW a read can be the peak. Ranking every slot
        // by `ns` alone put the peak at the end of the hour whenever the durations
        // were all zero, because `max_by_key` keeps the LAST maximum on a tie and
        // an empty timeline is one long tie - the report then pointed at t=+3599.9s
        // for traffic that all arrived in the first second.
        let (peak_i, peak) = s
            .slots
            .iter()
            .enumerate()
            .filter(|(_, x)| x.reads > 0)
            .max_by_key(|(_, x)| (x.ns, x.reads))
            .map(|(i, x)| (i, *x))
            .unwrap_or((0, Slot::default()));
        let busiest = s
            .slots
            .iter()
            .map(|x| x.reads)
            .max()
            .unwrap_or(0);
        let over = |n: u32| s.slots.iter().filter(|x| x.reads >= n).count();

        let mut tids: Vec<(u32, u64)> = s.per_tid.iter().map(|(&t, &c)| (t, c)).collect();
        tids.sort_unstable_by_key(|&(_, count)| std::cmp::Reverse(count));
        let top_tid = tids.first().map(|(_, c)| pct(*c)).unwrap_or(0.0);

        let mut files: Vec<(u64, u64, u64)> =
            s.per_file.iter().map(|(&i, &(c, b))| (i, c, b)).collect();
        files.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let hottest: Vec<String> = files
            .iter()
            .take(20)
            .map(|(ino, c, b)| {
                let name = path_of(*ino).unwrap_or_else(|| format!("(inode {ino}, gone)"));
                format!("  {c:>8}  {:>9.1} MiB  /{name}", mib(*b))
            })
            .collect();

        let dropped = if s.files_overflow > 0 {
            format!("\n  ({} reads of further files counted but not attributed)", s.files_overflow)
        } else {
            String::new()
        };
        // Say what the timeline did NOT see. Without this the peak line reads as
        // if it covered the whole session, and on a session longer than the
        // timeline it silently would not.
        let past = if s.past_timeline > 0 {
            format!(
                "\n  ({} reads ({:.1}%) arrived after the timeline's {} minutes and are counted \
                 everywhere EXCEPT the timeline)",
                s.past_timeline,
                pct(s.past_timeline),
                TIMELINE_SLOTS as u64 * SLOT_MS / 60_000,
            )
        } else {
            String::new()
        };
        let tid_note = if s.tids_overflow > 0 {
            format!("\n  ({} reads from further threads not attributed)", s.tids_overflow)
        } else {
            String::new()
        };

        // Only print a peak when there IS one. With every read past the end of
        // the timeline, the "peak" would be slot 0 of an empty table: 0 ms at
        // t=+0.0s, which reads as "no bursts" when the truth is "not measured".
        let timeline = if non_empty == 0 {
            "\n  timeline: no reads landed inside it".to_string()
        } else {
            // Two honesty rules in the wording. The percentage names READ time,
            // because that is all `note_read` ever feeds it: a slice saturated
            // by lookups or readdirs would print a low figure here, and an
            // unlabelled number would read as "nothing waited on the daemon" -
            // an inference this timeline cannot support on its own. And the
            // heaviest slot carries its OWN read count, because it need not be
            // the busiest-by-count slot: sharing one line let a 1000-read burst
            // at t=+0s print beside the timestamp of a different slot entirely.
            format!(
                "\n  timeline: {non_empty} non-empty {SLOT_MS}ms slots; busiest {busiest} reads; \
                 heaviest slot {:.0} ms of READ-handler time ({} read(s)) at t=+{:.1}s \
                 = {:.1}% of the {}-thread pool's capacity for that slice\
                 \n  slots with >=50 reads: {}, >=200: {}, >=500: {}",
                peak.ns as f64 / 1_000_000.0,
                peak.reads,
                peak_i as f64 * SLOT_MS as f64 / 1000.0,
                peak.ns as f64 * 100.0 / slot_capacity_ns,
                threads,
                over(50),
                over(200),
                over(500),
            )
        };

        format!(
            "\neidos-fuse read shape: {total} reads, {:.1} MiB requested, mean {:.1} KiB\
             \n  request sizes (as the kernel issued them, not as the game asked): {}{timeline}\
             \n  threads issuing reads: {} distinct, busiest holds {:.1}%\
             \n  files by read count:\n{}{}{}{}",
            mib(s.bytes),
            s.bytes as f64 / total as f64 / 1024.0,
            sizes.join(", "),
            tids.len(),
            top_tid,
            hottest.join("\n"),
            dropped,
            tid_note,
            past,
        )
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

    pub(crate) fn report(
        &self,
        r: &eidos_core::ResolveStats,
        path_of: &dyn Fn(u64) -> Option<String>,
    ) -> String {
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
            + &self.read_shape(path_of)
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
        // The thread count is printed, not implied. It is the one thing an A/B run
        // varies that the report otherwise cannot show (`EIDOS_FUSE_THREADS` is an
        // environment variable, so it leaves no trace in the recorded command
        // line), and without it two dumps from two arms are indistinguishable
        // afterwards - which is exactly how one comparison was already lost.
        format!(
            "\n  time in handlers: {:.0} ms, summed across {} thread(s) ({})\
             \n  path resolution: {} probes, {} directory scans, {:.0} ms (also summed)",
            ms(total),
            crate::config::fuse_threads(),
            each.join(", "),
            r.probes.load(Ordering::Relaxed),
            r.scans.load(Ordering::Relaxed),
            ms(r.ns.load(Ordering::Relaxed)),
        )
    }
}
