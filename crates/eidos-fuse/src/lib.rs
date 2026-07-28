//! Eidos FUSE union filesystem.
//!
//! A read-write union: it presents a merged view of mod layers over game data
//! by delegating every path decision to the unit-tested `eidos-core` resolver.
//! Reads merge with mod priority and case-insensitive lookup; writes copy up
//! into the Overwrite layer, leaving the game install and mod sources pristine.
//!
//! The filesystem is exposed two ways:
//! - [`Eidos::mount`] blocks, serving until unmounted (used by the `eidos-fuse`
//!   CLI).
//! - [`Eidos::spawn`] mounts on a background thread and returns a session handle
//!   whose drop unmounts (used by `eidos-launch` to mount inside a namespace and
//!   then run the game).
//!
//! Design notes worth keeping in mind (validated against MO2's `usvfs` and the
//! kernel FUSE docs):
//! - Each open file caches its resolved backing path and fd, so reads/writes hit
//!   the real file directly (`pread`/`pwrite`) without re-walking the layer stack
//!   per syscall.
//! - Inodes are reference-counted against the kernel lookup count and freed on
//!   `forget`, so the path<->inode table tracks the working set rather than every
//!   path ever seen.
//! - `readdir` is served from a snapshot taken at `opendir`, so offsets stay
//!   stable even if the directory changes mid-enumeration.
//! - Kernel passthrough is requested best-effort but needs real root, so the
//!   rootless daemon serves reads/writes itself; `writeback_cache` is negotiated
//!   instead to make writable shared mmap correct and coalesce writes.

use std::collections::HashMap;
use std::ffi::{CString, OsStr};
use std::fs::{self, File, Metadata, OpenOptions, Permissions};
use std::io::ErrorKind;
use std::os::fd::AsFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eidos_core::LayerStack;
use fuser::{
    BackgroundSession, BackingId, BsdFileFlags, Config, Errno, FileAttr, FileHandle, FileType,
    Filesystem, FopenFlags, Generation, INodeNo, InitFlags, KernelConfig, LockOwner,
    MountOption, OpenFlags, RenameFlags, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
    WriteFlags,
};

/// Lock a mutex, recovering the guard even if a previous holder panicked while
/// holding it. A poisoned lock would otherwise make every later handler panic on
/// `.unwrap()` and take the whole mount down; recovering keeps the daemon alive
/// (the protected state is plain maps and counters, safe to reuse).
trait LockExt<T> {
    fn lock_recover(&self) -> MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_recover(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Attribute/entry cache lifetime handed to the kernel for entries that EXIST.
/// A mod's files are immutable for the lifetime of a mount, and every mutation
/// goes through this daemon's own handlers, so the kernel can hold on to them.
/// Set `EIDOS_FUSE_NO_CACHE=1` to zero this (and [`NEG_TTL`]) when a stale-data
/// bug is the suspect.
const TTL_SECS: u64 = 3600;
/// Lifetime of a NEGATIVE dentry - see [`Eidos::reply_negative`]. Much shorter
/// than the positive TTL because the kernel matches negative entries on exact
/// name bytes while Eidos resolves case-insensitively.
const NEG_TTL_SECS: u64 = 60;
const ROOT_INO: u64 = 1;

/// The marker CIOPFS leaves in every directory it serves, and the only way Wine
/// can tell that a filesystem folds case (`get_dir_case_sensitivity` in
/// `dlls/ntdll/unix/file.c` stats it). Eidos answers it in `lookup` so Wine skips
/// its brute-force directory rescan on every mis-cased path - see the comment
/// there for what that rescan costs.
const CIOPFS_MARKER: &[u8] = b".ciopfs";


/// One kernel-side cache, individually switchable.
///
/// `EIDOS_FUSE_NO_CACHE=1` turns all four off together, which answers "is it the
/// caching?" but not "which one". `EIDOS_FUSE_NO_CACHE=attr,keep` names them, so
/// a stale-data bug can be bisected in four runs instead of by rebuilding with
/// lines commented out. The names are the kernel's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cache {
    /// Positive attribute/entry TTL.
    Attr,
    /// Negative-dentry TTL.
    Neg,
    /// `FOPEN_KEEP_CACHE`: keep the page cache across opens of a file.
    Keep,
    /// `FOPEN_CACHE_DIR`: let the kernel cache directory listings.
    Dir,
}

impl Cache {
    fn name(self) -> &'static str {
        match self {
            Cache::Attr => "attr",
            Cache::Neg => "neg",
            Cache::Keep => "keep",
            Cache::Dir => "dir",
        }
    }

    /// Whether this particular cache is switched off.
    fn is_off(self) -> bool {
        let Ok(v) = std::env::var("EIDOS_FUSE_NO_CACHE") else { return false };
        let v = v.trim();
        if v.is_empty() || v == "0" {
            return false;
        }
        // A bare truthy value means all of them; a list names the ones to drop.
        if !v.contains(',') && !["attr", "neg", "keep", "dir"].contains(&v) {
            return true;
        }
        v.split(',').any(|part| part.trim().eq_ignore_ascii_case(self.name()))
    }
}

static TTL: std::sync::LazyLock<Duration> = std::sync::LazyLock::new(|| {
    Duration::from_secs(if Cache::Attr.is_off() { 0 } else { TTL_SECS })
});
static NEG_TTL: std::sync::LazyLock<Duration> = std::sync::LazyLock::new(|| {
    Duration::from_secs(if Cache::Neg.is_off() { 0 } else { NEG_TTL_SECS })
});

/// A directory entry in an `opendir` snapshot: `(inode, kind, name)`.
type DirEntry = (u64, FileType, String);

/// A merged directory listing without inodes: the layers collapsed and NTFS-collated.
/// Shared, because it is cached by path and handed to every enumeration of it.
type Listing = Arc<Vec<(String, FileType)>>;

/// One open directory handle. `entries` is filled by the first `readdir`.
struct OpenDir {
    ino: u64,
    vpath: String,
    entries: Option<Vec<DirEntry>>,
}

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
struct Stats {
    lookup_hit: AtomicU64,
    lookup_miss: AtomicU64,
    getattr: AtomicU64,
    /// Directory OPENS. Counted separately from `readdir` since a measured session
    /// reported 516301 of these against 26999 reads - and the counter used to live
    /// in `opendir` while being NAMED `readdir`, which made the number unreadable:
    /// it was taken for half a million enumerations when almost none of them
    /// enumerated anything. See `probe`.
    opendir: AtomicU64,
    releasedir: AtomicU64,
    /// Directory handles closed without a single `readdir` - opened purely to look
    /// at the directory inode, never to list it. Wine does this on every failed
    /// path resolution to ask whether the directory folds case. This counter is the
    /// only honest measure of how much of `opendir` is probing rather than
    /// enumerating; ratios against `lookup` cannot tell them apart, because the
    /// negative-dentry cache absorbs the lookups while every directory open still
    /// reaches us (the kernel has no cache for those).
    probe: AtomicU64,
    /// Enumeration requests, at last counted in `readdir` itself.
    readdir: AtomicU64,
    /// Merged listings actually built - a multi-layer walk plus an NTFS-collation
    /// sort. Against `dir_hit` this gives the by-path cache's hit rate.
    snapshot: AtomicU64,
    dir_hit: AtomicU64,
    /// `.ciopfs` marker lookups: how often Wine asks whether a directory folds
    /// case. Expected to sit orders of magnitude BELOW `opendir`, because the
    /// answer is dentry-cached while the open that precedes it is not.
    marker: AtomicU64,
    open: AtomicU64,
    read: AtomicU64,
    write: AtomicU64,
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
    dirs: Mutex<HashMap<String, u64>>,
    /// Opens whose path did not fit under [`DIR_HISTOGRAM_CAP`], so the top-N
    /// stays honest about what it is not showing.
    dirs_overflow: AtomicU64,
}

/// Distinct directories the histogram will hold before it stops learning new
/// ones. A pathological session must not turn a diagnostic into an OOM; at this
/// size the map is a few tens of MB and the shape is already unambiguous.
const DIR_HISTOGRAM_CAP: usize = 200_000;

/// Whether the per-directory histogram is being collected. Read once, so the
/// cost on a path taken half a million times a session is an atomic load and
/// not a `getenv`.
static STATS_ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(Stats::enabled);

impl Stats {
    fn bump(c: &AtomicU64) {
        c.fetch_add(1, Ordering::Relaxed);
    }

    fn enabled() -> bool {
        std::env::var("EIDOS_FUSE_STATS").is_ok_and(|v| v != "0")
    }

    /// Note one directory open against its path.
    fn note_dir(&self, path: &str) {
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
    fn dir_shape(&self) -> String {
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

    fn report(&self) -> String {
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
        ) + &self.dir_shape()
    }
}

/// Maps inode numbers to virtual paths (relative, no leading slash; "" = root)
/// and back, minting a stable inode the first time a path is seen.
///
/// Inodes are reference-counted against the kernel's per-inode lookup count:
/// [`Inodes::lookup`] (used by lookup/create/mkdir, which return an entry the
/// kernel pins) bumps the count; [`Inodes::forget`] drops it and frees the entry
/// at zero. Plain `readdir` takes no reference, so it uses [`Inodes::intern`],
/// which mints a stable inode without counting. Inode numbers are never reused
/// (the allocator is monotonic), so the kernel `(ino, generation)` pair is always
/// unambiguous and `generation` can stay 0.
struct Inodes {
    by_ino: HashMap<u64, String>,
    by_path: HashMap<String, u64>,
    counts: HashMap<u64, u64>,
    next: u64,
}

impl Inodes {
    fn new() -> Self {
        let mut s = Self {
            by_ino: HashMap::new(),
            by_path: HashMap::new(),
            counts: HashMap::new(),
            next: ROOT_INO + 1,
        };
        // The root is never looked up or forgotten; keep it un-counted and pinned.
        s.by_ino.insert(ROOT_INO, String::new());
        s.by_path.insert(String::new(), ROOT_INO);
        s
    }

    /// Get or mint the inode for `vpath` without touching its lookup count.
    ///
    /// `by_path` is keyed case-INSENSITIVELY (see [`ikey`]) because the layer
    /// resolver folds case at every data-layer decision: `Skyrim.esm` and
    /// `skyrim.esm` are one file, so they must be one inode. `by_ino` keeps the
    /// caller's original casing, which is what `resolve_read`'s exact-path fast
    /// path wants - folding it there would degrade every resolution into a
    /// directory scan per component.
    fn intern(&mut self, vpath: &str) -> u64 {
        if let Some(&ino) = self.by_path.get(&ikey(vpath)) {
            return ino;
        }
        let ino = self.next;
        self.next += 1;
        self.by_ino.insert(ino, vpath.to_string());
        self.by_path.insert(ikey(vpath), ino);
        ino
    }

    /// Intern `vpath` and take a kernel reference on it (increment the lookup
    /// count). Use for replies that pin an inode kernel-side: lookup/create/mkdir.
    fn lookup(&mut self, vpath: &str) -> u64 {
        let ino = self.intern(vpath);
        *self.counts.entry(ino).or_insert(0) += 1;
        ino
    }

    fn path(&self, ino: u64) -> Option<String> {
        self.by_ino.get(&ino).cloned()
    }

    /// Drop `nlookup` kernel references from `ino`; free the mapping at zero.
    /// The root is pinned for the mount's lifetime and never freed.
    fn forget(&mut self, ino: u64, nlookup: u64) {
        if ino == ROOT_INO {
            return;
        }
        if let Some(count) = self.counts.get_mut(&ino) {
            *count = count.saturating_sub(nlookup);
            if *count == 0 {
                self.counts.remove(&ino);
                if let Some(path) = self.by_ino.remove(&ino) {
                    // Only drop the reverse entry if it still points at US. After a
                    // rename clobbered a destination, the surviving inode owns that
                    // key; removing it blindly would unmap a live inode and let a
                    // later lookup mint a second one for the same path.
                    let key = ikey(&path);
                    if self.by_path.get(&key) == Some(&ino) {
                        self.by_path.remove(&key);
                    }
                }
            }
        }
    }

    /// Rebind the inode for `from` onto `to` after a rename, so the kernel's
    /// reuse of the source inode for the destination keeps resolving correctly.
    /// The inode number (and its lookup count) is preserved.
    ///
    /// Also rebinds every DESCENDANT: `LayerStack::rename` supports directory
    /// renames, and the kernel keeps dentries for children it has already looked
    /// up. Leaving those mapped under the old prefix would make the next
    /// `getattr`/`read` on a kernel-held child resolve to a path that the rename
    /// just whited out, i.e. a spurious ENOENT on a file that is right there.
    /// Returns the inode that was moved, if the kernel had one for `from`.
    fn rename(&mut self, from: &str, to: &str) -> Option<u64> {
        let (from_key, to_key) = (ikey(from), ikey(to));

        // Drop an inode that the rename has made unreachable, so a later `forget`
        // on it cannot unmap whoever owns its path now.
        fn discard(this: &mut Inodes, ino: u64, survivor: u64) {
            if ino != survivor {
                this.by_ino.remove(&ino);
                this.counts.remove(&ino);
            }
        }

        // Rebind the entry itself, if the kernel ever looked it up. A directory
        // rename can arrive with the directory un-interned (only its children were
        // resolved), so this is not a precondition for the subtree pass below.
        let moved_ino = self.by_path.remove(&from_key);
        if let Some(ino) = moved_ino {
            // The destination may already be interned - renaming over an existing
            // file is the standard atomic-replace pattern for INIs and saves.
            if let Some(clobbered) = self.by_path.remove(&to_key) {
                discard(self, clobbered, ino);
            }
            self.by_ino.insert(ino, to.to_string());
            self.by_path.insert(to_key, ino);
        }

        // Re-key the subtree. Collect first: mutating `by_path` while iterating it
        // is not allowed, and a child may itself clobber an existing destination.
        let prefix = format!("{from_key}/");
        let moved: Vec<(String, u64)> = self
            .by_path
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, &i)| (k.clone(), i))
            .collect();
        for (old_key, child_ino) in moved {
            self.by_path.remove(&old_key);
            let new_key = format!("{}/{}", ikey(to), &old_key[prefix.len()..]);
            if let Some(clobbered) = self.by_path.insert(new_key, child_ino) {
                discard(self, clobbered, child_ino);
            }
            // Keep the display path in step, preserving the child's own casing.
            if let Some(old_display) = self.by_ino.get(&child_ino).cloned() {
                if old_display.len() >= from.len() {
                    self.by_ino.insert(child_ino, format!("{to}{}", &old_display[from.len()..]));
                }
            }
        }
        moved_ino
    }
}

/// Flags for a FILE open reply.
///
/// `FOPEN_KEEP_CACHE` tells the kernel to keep the page cache it already holds
/// for this inode instead of dropping it on every open. That is a large win for
/// the read-only bulk of a load order - BSAs, meshes, textures the game opens
/// over and over - and it is why it is here.
///
/// It is only sound for a file whose BYTES CANNOT CHANGE UNDER THE INODE, which
/// is why `immutable` is a parameter rather than assumed. An earlier version set
/// it unconditionally, reasoning that "the layers are immutable and every write
/// goes through this daemon". Both halves are true and the conclusion is still
/// wrong, twice over:
///
///   * The overwrite layer is not immutable. The game writes its shader cache,
///     its INIs and its saves there and reads them back within the session.
///   * A lower-layer file does not keep its identity. The first write COPIES IT
///     UP, so the same virtual path - and the same FUSE inode - is suddenly
///     backed by a different file on disk, while the kernel still holds pages it
///     read from the old one. With `FUSE_PASSTHROUGH` the kernel serves those
///     reads without ever asking the daemon, so nothing downstream can notice.
///
/// SO IT IS OFF BY DEFAULT, and stays off until someone can explain the crash
/// below rather than argue it away.
///
/// Measured on Skyrim SE 1.6.1170 under proton-cachyos 11.0, with ZERO mods
/// installed: the game reaches its main menu and then dies on a null dereference
/// (`0xc0000005`, address `0x48`) on a worker thread, deterministically, at the
/// same instruction every run. Turning this one flag off fixes it while
/// `FOPEN_CACHE_DIR`, the 1-hour attribute TTL and the negative-dentry cache all
/// stay on - each was bisected out individually and only this one mattered.
///
/// Restricting it to files outside the overwrite layer was tried and did NOT
/// help, which rules out the obvious copy-up explanation: the game crashes while
/// reading files that were never written to. The interaction with
/// `FUSE_PASSTHROUGH` - where the kernel serves reads from the backing file
/// without consulting this daemon at all - was the open question; passthrough is
/// now off by default too (see [`passthrough_enabled`]), so the two are no longer
/// entangled and this one can be re-tested on its own.
///
/// `EIDOS_FUSE_KEEP_CACHE=1` turns it back on for whoever picks that up.
fn file_open_flags() -> FopenFlags {
    if std::env::var("EIDOS_FUSE_KEEP_CACHE").is_ok_and(|v| v != "0") && !Cache::Keep.is_off() {
        FopenFlags::FOPEN_KEEP_CACHE
    } else {
        FopenFlags::empty()
    }
}

/// Whether to hand the kernel a backing file so it can serve reads without us.
///
/// OFF BY DEFAULT, because with it on Skyrim SE cannot open its own content.
///
/// Measured A/B on Skyrim SE 1.6.1170 under proton-cachyos 11.0, same 82-plugin
/// load order, `WINEDEBUG=+file` both runs, the only variable being whether the
/// binary carried `cap_sys_admin` (without it `open_backing` returns EPERM and
/// the daemon serves reads itself):
///
/// | passthrough | `NtCreateFile` failures with `STATUS_ACCESS_VIOLATION` |
/// |-------------|--------------------------------------------------------|
/// | on          | 152 - 75 `.bsa`, 65 `.esl`, 10 `.esm`, 2 `.esp`        |
/// | off         | 0                                                      |
///
/// With it on, the game loads no mod content at all: every plugin and archive it
/// wants for the whole session fails to open, which surfaces in-game as mods that
/// are simply not there. With it off the same load order reaches gameplay with
/// its plugins, archives and Papyrus scripts live.
///
/// The failure is INVISIBLE from here, which is why it took so long to find: our
/// own `open` succeeds every time and `open_backing` is never refused (verified
/// with `EIDOS_FUSE_TRACE=open` - zero `open FAILED`, zero `passthrough refused`
/// across a full failing session). The kernel produces the error after we reply
/// `opened_passthrough`, so no amount of daemon-side logging shows it. Whatever
/// the mechanism, it is not extension-specific: it hits archives and plugins
/// alike, i.e. exactly the files the game holds open for its whole run.
///
/// The cost of leaving it off is throughput, not correctness: reads take a
/// userspace round-trip instead of going straight to the backing file. That is
/// already what every rootless install does, since registering a backing fd needs
/// CAP_SYS_ADMIN in the initial user namespace.
///
/// `EIDOS_FUSE_PASSTHROUGH=1` turns it back on, for measuring the throughput it
/// buys or for re-testing once someone can explain the failure above.
fn passthrough_enabled() -> bool {
    static ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::env::var("EIDOS_FUSE_PASSTHROUGH").is_ok_and(|v| {
            let v = v.trim();
            !v.is_empty() && v != "0"
        })
    });
    *ON
}

/// Whether `EIDOS_FUSE_TRACE` names this trace channel (comma-separated, or `1`
/// for all of them). Diagnostic only, and off unless asked for: the op counters
/// say HOW MANY, this says WHICH, which is the difference between knowing a
/// directory is enumerated 50000 times and knowing which directory it is.
fn trace_enabled(channel: &str) -> bool {
    let Ok(v) = std::env::var("EIDOS_FUSE_TRACE") else { return false };
    let v = v.trim();
    !v.is_empty() && (v == "1" || v.split(',').any(|c| c.trim().eq_ignore_ascii_case(channel)))
}

/// Raise this process's open-file soft limit to its hard limit. Best-effort:
/// every failure path leaves us exactly where we started.
fn raise_fd_limit() {
    // SAFETY: getrlimit/setrlimit with a valid, fully-initialised rlimit struct.
    unsafe {
        let mut lim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim) != 0 || lim.rlim_cur >= lim.rlim_max {
            return;
        }
        lim.rlim_cur = lim.rlim_max;
        let _ = libc::setrlimit(libc::RLIMIT_NOFILE, &lim);
    }
}

/// The case-folded key under which a virtual path is interned. Eidos resolves
/// paths case-insensitively (Windows games mix casing freely between the plugin
/// header, the loose-file indexer and BSA lookups), so one real file must map to
/// exactly one inode however the caller spelled it. ASCII-only, matching the
/// documented fold in `eidos-core`.
fn ikey(vpath: &str) -> String {
    vpath.to_ascii_lowercase()
}

/// An open file: its resolved backing path plus the real fd, kept open so
/// reads/writes use `pread`/`pwrite` without re-resolving. `backing` holds the
/// kernel passthrough registration when it could be set up (privileged only).
struct OpenFile {
    _real: PathBuf,
    file: Arc<File>,
    _backing: Option<BackingId>,
}

/// The Eidos union filesystem over a [`LayerStack`].
pub struct Eidos {
    stack: LayerStack,
    inodes: Mutex<Inodes>,
    uid: u32,
    gid: u32,
    open_files: Mutex<HashMap<u64, OpenFile>>,
    /// Per-handle directory state. The snapshot is `None` until the first
    /// `readdir` asks for it - see `opendir` for why building it eagerly was a
    /// disaster - and once taken it stays fixed for the handle's life so offsets
    /// remain valid even if the directory changes underneath.
    open_dirs: Mutex<HashMap<u64, OpenDir>>,
    /// Merged directory listings by vpath: the child `(name, kind)` pairs after the
    /// layers are collapsed and NTFS-collated. That merge is the expensive half of
    /// an enumeration - a `read_dir` per layer plus a sort - and mod layers are
    /// immutable for the life of the mount, so repeat enumerations of the same
    /// directory can reuse it. Anything written through the mount goes through our
    /// own handlers, which drop the affected parent below.
    ///
    /// Inode numbers are deliberately NOT cached with it. `forget` drops an inode
    /// when the kernel releases its last reference, and a later `intern` of the same
    /// path mints a FRESH number - so a cached entry list would hand out inodes the
    /// daemon no longer knows. Interning is a hashmap hit against real disk I/O, so
    /// re-interning per enumeration costs almost nothing and is always correct.
    dir_cache: Mutex<HashMap<String, Listing>>,
    next_fh: AtomicU64,
    stats: Stats,
    /// Negative dentries handed to the kernel, as `(parent_ino, exact name)`.
    ///
    /// The kernel keys its dentry cache on the EXACT name bytes while Eidos
    /// resolves case-insensitively, so a negative cached for `Foo.esp` would
    /// survive a later create of `foo.esp` and the game would be told a file it
    /// can plainly see does not exist. We remember what we denied so that a
    /// create can invalidate precisely those spellings.
    negatives: Mutex<HashMap<u64, Vec<String>>>,
    /// Positive dentries handed to the kernel, keyed by inode: the
    /// `(parent_ino, exact name)` spellings it now caches for that file.
    ///
    /// The case fold means one inode is reachable through several kernel
    /// dentries. A rename moves only the spelling it was given, so every OTHER
    /// recorded spelling would keep resolving to the moved file for the whole
    /// entry TTL and serve its contents under a name that no longer exists.
    aliases: Mutex<HashMap<u64, Vec<(u64, String)>>>,
    /// Set once the session is mounted; used to push those invalidations.
    notifier: Arc<Mutex<Option<fuser::Notifier>>>,
    /// The kernel advertised `FUSE_NO_OPENDIR_SUPPORT`, so it can open and read
    /// directories without asking us. Negotiated in `init`; see [`Eidos::opendir`].
    no_opendir: AtomicBool,
    /// The listing an in-flight enumeration is being served from, by inode.
    ///
    /// A `readdir` offset is an INDEX into the listing the previous chunk came
    /// from, so that listing has to survive until the enumeration ends. With
    /// handles, `opendir` gave that for free - the snapshot hung off the handle.
    /// Declining `opendir` took the handle away, and the by-path cache underneath
    /// is dropped by any mutation, so a directory written to between two chunks
    /// would have the second chunk resume into a DIFFERENT list at the same index
    /// and silently skip or repeat entries. The Creation Engine's loose-file
    /// indexer answers an inconsistent enumeration by restarting it, which is the
    /// endless loop `dir_snapshot` already warns about.
    ///
    /// So the guarantee moves here: pinned on the first chunk, served unchanged
    /// for the rest, released when the enumeration finishes. Deliberately NOT
    /// cleared by `dir_changed` - a walk in progress must finish against what it
    /// started with; it is the NEXT one that must see the change.
    enumerations: Mutex<HashMap<u64, Arc<Vec<DirEntry>>>>,
}

/// Join a parent virtual path and a child name into a virtual path.
fn join(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

fn kind_of(meta: &Metadata) -> FileType {
    kind_of_type(&meta.file_type())
}

/// Map a `std::fs::FileType` to the FUSE one. Symlink-aware, so it must be fed a
/// type obtained WITHOUT following links (`symlink_metadata`, or a `DirEntry`'s
/// own `file_type`, which is `lstat`-shaped too).
fn kind_of_type(ft: &std::fs::FileType) -> FileType {
    if ft.is_dir() {
        FileType::Directory
    } else if ft.is_symlink() {
        FileType::Symlink
    } else {
        FileType::RegularFile
    }
}

fn mount_config() -> Config {
    // Config is #[non_exhaustive]; build via Default and set the public field.
    let mut config = Config::default();
    config.mount_options = vec![MountOption::FSName("eidos".to_string())];

    // Serve requests from several event loops. A game's loading screen issues
    // metadata requests far faster than one thread can answer them while each
    // answer costs real directory I/O, so a single loop serialises the whole
    // startup behind the slowest resolution. `clone_fd` gives each worker its own
    // /dev/fuse fd (Linux 4.5+), which is what makes the extra threads actually
    // parallel rather than contending on one channel.
    //
    // The handlers are already safe for this: every piece of shared state is
    // behind a mutex, and `Filesystem` takes `&self`. Bounded at 4 because the
    // work is I/O-bound; more threads buy nothing and cost context switches.
    config.n_threads = Some(fuse_threads());
    config.clone_fd = true;
    config
}

/// Event-loop threads to run. Defaults to 4, capped by the machine's parallelism,
/// and overridable with `EIDOS_FUSE_THREADS` (1 makes the daemon single-threaded
/// again, which is the first thing to try when diagnosing a concurrency bug).
fn fuse_threads() -> usize {
    if let Some(n) = std::env::var("EIDOS_FUSE_THREADS").ok().and_then(|v| v.parse::<usize>().ok()) {
        return n.max(1);
    }
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    cpus.clamp(1, 4)
}

impl Eidos {
    /// Build a union over the given layers (highest priority first) and a
    /// writable overwrite layer.
    pub fn new(layers: Vec<PathBuf>, overwrite: PathBuf) -> Self {
        // SAFETY: getuid/getgid always succeed with no preconditions.
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
        Self {
            stack: LayerStack::new(layers, overwrite),
            inodes: Mutex::new(Inodes::new()),
            uid,
            gid,
            open_files: Mutex::new(HashMap::new()),
            open_dirs: Mutex::new(HashMap::new()),
            dir_cache: Mutex::new(HashMap::new()),
            next_fh: AtomicU64::new(1),
            no_opendir: AtomicBool::new(false),
            enumerations: Mutex::new(HashMap::new()),
            stats: Stats::default(),
            negatives: Mutex::new(HashMap::new()),
            aliases: Mutex::new(HashMap::new()),
            notifier: Arc::new(Mutex::new(None)),
        }
    }

    /// Mount at `mountpoint`, blocking until the filesystem is unmounted.
    pub fn mount(self, mountpoint: &Path) -> std::io::Result<()> {
        fuser::mount2(self, mountpoint, &mount_config())
    }

    /// Mount at `mountpoint` on a background thread, returning a session handle.
    /// Dropping the handle unmounts.
    pub fn spawn(self, mountpoint: &Path) -> std::io::Result<BackgroundSession> {
        // Build the session by hand rather than via spawn_mount2, so the kernel
        // notifier can be handed back INTO the filesystem: it is what lets a
        // create invalidate a stale, differently-cased negative dentry.
        let notifier_slot = Arc::clone(&self.notifier);
        let session = fuser::Session::new(self, mountpoint, &mount_config())?;
        *notifier_slot.lock_recover() = Some(session.notifier());
        session.spawn()
    }

    /// The virtual path of `name` inside the directory inode `parent`.
    fn child(&self, parent: INodeNo, name: &OsStr) -> Option<String> {
        let inodes = self.inodes.lock_recover();
        let parent_vpath = inodes.path(parent.0)?;
        Some(join(&parent_vpath, &name.to_string_lossy()))
    }

    /// Answer a failed lookup with a NEGATIVE DENTRY (`ino = 0`) rather than
    /// ENOENT, so the kernel caches the absence and stops asking.
    ///
    /// This is the single biggest metadata lever. Wine probes enormous numbers of
    /// paths that do not exist - every DLL search-order walk, every `.ini`/`.txt`
    /// sidecar the engine looks for beside a resource, every script-extender
    /// plugin's config probe - and a bare ENOENT is not cached by the kernel, so
    /// each repeat costs a full `resolve_read`: a case-folding directory scan per
    /// layer, plus a whiteout check per ancestor. With 200 mod layers that is
    /// hundreds of `read_dir` calls for one absent file, and it is exactly the
    /// metadata storm that stalls a Bethesda game's startup for minutes.
    ///
    /// The TTL is deliberately short. The kernel keys its negative dentries on the
    /// exact name bytes while Eidos folds case, so a negative cached for `Foo.esp`
    /// would survive a create of `foo.esp` through the mount until it expires.
    /// Creation through this mount is safe regardless (the kernel re-issues a real
    /// lookup for O_CREAT and for every LOOKUP_EXCL path), so the window only
    /// concerns a differently-cased create, which self-heals in seconds.
    fn reply_negative(&self, parent: u64, name: &str, reply: ReplyEntry) {
        // Remember the exact spelling we denied, capped so a probe storm cannot
        // grow this without bound (the kernel forgets on its own TTL anyway).
        {
            let mut neg = self.negatives.lock_recover();
            let names = neg.entry(parent).or_default();
            if names.len() < 4096 && !names.iter().any(|n| n == name) {
                names.push(name.to_string());
            }
        }
        // ino = 0 IS the negative dentry; no inode is minted or refcounted for a
        // path that does not exist.
        let blank = FileAttr {
            ino: INodeNo(0),
            size: 0,
            blocks: 0,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: FileType::RegularFile,
            perm: 0,
            nlink: 0,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        };
        reply.entry(&NEG_TTL, &blank, Generation(0));
    }

    /// Remember a spelling the kernel now caches for `ino`, so a later rename can
    /// drop the ones it did not move. Capped per inode: a file is normally
    /// reached through one or two spellings, and the cap bounds a pathological
    /// case rather than a real one.
    fn record_alias(&self, ino: u64, parent: u64, name: &str) {
        let mut aliases = self.aliases.lock_recover();
        let entry = aliases.entry(ino).or_default();
        if entry.len() < 16 && !entry.iter().any(|(p, n)| *p == parent && n == name) {
            entry.push((parent, name.to_string()));
        }
    }

    /// After a rename, drop every kernel dentry that still points at the moved
    /// inode under its OLD name. The kernel moves the one dentry it was given;
    /// the case-variant spellings we handed out earlier are now lies.
    fn invalidate_stale_aliases(&self, ino: u64, kept_parent: u64, kept_name: &str) {
        let stale: Vec<(u64, String)> = {
            let mut aliases = self.aliases.lock_recover();
            let Some(entry) = aliases.get_mut(&ino) else { return };
            let (stale, keep): (Vec<_>, Vec<_>) =
                entry.drain(..).partition(|(p, n)| !(*p == kept_parent && n == kept_name));
            *entry = keep;
            stale
        };
        if stale.is_empty() {
            return;
        }
        // Off the handler thread: see `invalidate_folded_negatives` - notifying
        // the kernel from inside a request deadlocks the mount.
        let Some(notifier) = self.notifier.lock_recover().clone() else { return };
        std::thread::spawn(move || {
            for (parent, name) in stale {
                let _ = notifier.inval_entry(INodeNo(parent), OsStr::new(&name));
            }
        });
    }

    /// Drop any negative dentry the kernel holds for a name that case-folds to
    /// `name` in `parent`. Called whenever an entry is created, so a probe for
    /// `Foo.esp` cannot keep hiding a freshly created `foo.esp`.
    /// Drop the kernel's cached pages for one inode.
    ///
    /// Needed when a copy-up swaps the file backing a virtual path: the inode
    /// number is stable by design (it is keyed on the path), so the kernel has no
    /// way to notice that the bytes behind it now come from somewhere else. With
    /// `FOPEN_KEEP_CACHE` it would go on serving the old ones, and under
    /// `FUSE_PASSTHROUGH` it would do so without the daemon ever being asked.
    ///
    /// `(0, 0)` means the whole inode.
    fn invalidate_page_cache(&self, ino: u64) {
        let Some(notifier) = self.notifier.lock_recover().clone() else { return };
        // Detached, for the same reason as `invalidate_folded_negatives`: a
        // notification sent from inside a request handler can deadlock the mount.
        std::thread::spawn(move || {
            let _ = notifier.inval_inode(INodeNo(ino), 0, 0);
        });
    }

    /// A name just appeared in or vanished from `parent`. Drop that directory's
    /// cached listing, and nudge the kernel about any differently-cased negative
    /// dentry it may still hold for the name.
    ///
    /// One call per mutating handler, so `grep dir_changed` is the audit: a handler
    /// that adds or removes a name without it leaves a stale listing behind, and
    /// the game would enumerate a directory that no longer looks like that. The two
    /// halves are deliberately NOT folded into `invalidate_folded_negatives` - that
    /// one returns early when the parent has no negatives recorded, which is the
    /// common case, so the cache drop would be skipped exactly when it matters.
    fn dir_changed(&self, parent: u64, name: &str) {
        self.invalidate_dir_of(parent);
        self.invalidate_folded_negatives(parent, name);
    }

    fn invalidate_folded_negatives(&self, parent: u64, name: &str) {
        let stale: Vec<String> = {
            let mut neg = self.negatives.lock_recover();
            let Some(names) = neg.get_mut(&parent) else { return };
            // Exact matches are handled by the kernel itself when it instantiates
            // the new dentry; only the OTHER spellings need a nudge.
            let (stale, keep): (Vec<String>, Vec<String>) =
                names.drain(..).partition(|n| n.eq_ignore_ascii_case(name) && n != name);
            *names = keep;
            stale
        };
        if stale.is_empty() {
            return;
        }
        // MUST NOT run inline. A notification is a message TO the kernel, and the
        // kernel may need locks that the request we are still answering holds, so
        // calling inval_entry from inside a handler deadlocks the mount (observed:
        // the create never returns). Hand it to a detached thread; the kernel
        // applies it as soon as this request completes.
        let Some(notifier) = self.notifier.lock_recover().clone() else { return };
        std::thread::spawn(move || {
            for s in stale {
                let _ = notifier.inval_entry(INodeNo(parent), OsStr::new(&s));
            }
        });
    }

    /// Build a `FileAttr` from a real file's metadata, owned by the mounting
    /// user (the game runs as us under Proton).
    fn attr(&self, ino: u64, meta: &Metadata) -> FileAttr {
        let mtime = meta.modified().unwrap_or(UNIX_EPOCH);
        let atime = meta.accessed().unwrap_or(mtime);
        FileAttr {
            ino: INodeNo(ino),
            size: meta.len(),
            blocks: meta.blocks(),
            atime,
            mtime,
            ctime: mtime,
            crtime: mtime,
            kind: kind_of(meta),
            perm: (meta.mode() & 0o7777) as u16,
            nlink: 1,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }

    /// Attributes for the synthetic `.ciopfs` marker: an empty, read-only regular
    /// file that exists nowhere on disk. Its only job is to be `stat`-able, which
    /// is all Wine's case-sensitivity probe does with it.
    fn marker_attr(&self, ino: u64) -> FileAttr {
        FileAttr {
            ino: INodeNo(ino),
            size: 0,
            blocks: 0,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: FileType::RegularFile,
            perm: 0o444,
            nlink: 1,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }

    /// Snapshot a directory's merged listing into the `(ino, kind, name)` form
    /// `readdir` serves, including `.` and `..`. Disk I/O (the layer merge and
    /// the per-entry stat) runs without the inode lock held.
    fn dir_snapshot(&self, ino: u64, vpath: &str) -> Vec<DirEntry> {
        let parent_ino = if vpath.is_empty() {
            ROOT_INO
        } else {
            let parent_vpath = vpath.rsplit_once('/').map_or("", |(p, _)| p).to_string();
            self.inodes.lock_recover().intern(&parent_vpath)
        };

        // Merge the layers, taking each child's type from the directory entry the
        // kernel already handed us, before taking the inode lock.
        //
        // This used to `symlink_metadata` every child - one `statx` syscall per
        // entry, per enumeration - and then, when that failed, ANNOUNCE THE ENTRY
        // AS A REGULAR FILE. Both halves were wrong. The syscalls are pure waste:
        // `readdir` already carries the type in `d_type`. And a directory reported
        // as a regular file is a lie the caller cannot recover from - it will not
        // descend into it, and the Creation Engine's loose-file indexer answers
        // that by restarting its enumeration, which is an endless loop with us
        // burning a core to serve it.
        //
        // An entry whose type cannot be determined even by the `lstat` fallback is
        // one being deleted underneath us. It is dropped rather than guessed at:
        // omitting a file that is going away is a transient inaccuracy, while
        // naming a directory a file is a wrong answer the caller will act on.
        let children = self.merged_children(vpath);

        let mut entries = Vec::with_capacity(children.len() + 2);
        entries.push((ino, FileType::Directory, ".".to_string()));
        entries.push((parent_ino, FileType::Directory, "..".to_string()));
        let mut inodes = self.inodes.lock_recover();
        for (name, kind) in children.iter() {
            let child_ino = inodes.intern(&join(vpath, name));
            entries.push((child_ino, *kind, name.clone()));
        }
        entries
    }

    /// The merged child list for `vpath`, from [`Self::dir_cache`] when it is
    /// already there.
    ///
    /// The merge itself runs WITHOUT the cache locked: it does real disk I/O across
    /// every layer, and holding the lock across it would serialise every other
    /// directory in the daemon. Two threads racing the same path both do the work
    /// and the first one to finish wins - wasteful once, never wrong.
    fn merged_children(&self, vpath: &str) -> Listing {
        if let Some(hit) = self.dir_cache.lock_recover().get(vpath) {
            Stats::bump(&self.stats.dir_hit);
            return Arc::clone(hit);
        }
        Stats::bump(&self.stats.snapshot);
        let children: Listing = Arc::new(
            self.stack
                .list_dir_typed(vpath)
                .into_iter()
                .filter_map(|(name, _real, ft)| Some((name, kind_of_type(&ft?))))
                .collect(),
        );
        let mut cache = self.dir_cache.lock_recover();
        Arc::clone(cache.entry(vpath.to_string()).or_insert(children))
    }

    /// Drop the cached listing for a directory, by vpath. Called from every handler
    /// that adds or removes a name.
    fn invalidate_dir_cache(&self, vpath: &str) {
        self.dir_cache.lock_recover().remove(vpath);
    }

    /// Drop the cached listing of `parent_ino`'s directory: the form the mutating
    /// handlers have their parent in.
    fn invalidate_dir_of(&self, parent_ino: u64) {
        let vpath = self.inodes.lock_recover().path(parent_ino);
        if let Some(v) = vpath {
            self.invalidate_dir_cache(&v);
        }
    }
}

impl Filesystem for Eidos {
    fn destroy(&mut self) {
        // Unmount is the natural place to report: the run is over and the numbers
        // are final. Silent unless EIDOS_FUSE_STATS is set.
        if Stats::enabled() {
            eprintln!("{}", self.stats.report());
        }
    }

    fn init(&mut self, _req: &Request, config: &mut KernelConfig) -> std::io::Result<()> {
        // FUSE passthrough: negotiate the capability and a non-zero stack depth so
        // the kernel routes reads/writes/mmap straight to the real backing file.
        // OFF unless asked for - see `passthrough_enabled` for the measurement that
        // put it there: with it on, Skyrim SE fails to open every archive and plugin
        // it needs. Don't negotiate what we won't use, so a run with it off is a
        // clean baseline rather than the capability sitting there unused.
        if passthrough_enabled() {
            let _ = config.add_capabilities(InitFlags::FUSE_PASSTHROUGH);
            let _ = config.set_max_stack_depth(1);
        }

        // Zero-message opendir. Recorded here rather than probed later because
        // this is the ONLY place the negotiated capability is visible, and
        // answering ENOSYS to a kernel that did not advertise it would not make
        // directory opens cheap - it would make them FAIL.
        //
        // `EIDOS_FUSE_OPENDIR=1` keeps the handles, for bisecting a directory
        // bug against the old path.
        let forced_off = std::env::var("EIDOS_FUSE_OPENDIR").is_ok_and(|v| v != "0");
        let cap = config.capabilities().contains(InitFlags::FUSE_NO_OPENDIR_SUPPORT);
        self.no_opendir.store(cap && !forced_off, Ordering::Relaxed);

        // FUSE_WRITEBACK_CACHE is deliberately NOT enabled. It makes writable
        // shared mmap work, but it breaks loading Windows DLLs from the mount
        // under Wine/Proton: an image loader dirties MAP_PRIVATE copy-on-write
        // pages while applying relocations and binding the import table, and with
        // writeback_cache the kernel mishandles those over a FUSE backing, so the
        // DLL fails to map (observed: the SKSE plugins that dynamically link the
        // VC++ runtime, i.e. have heavy relocation/import work, all failed while
        // statically-linked ones loaded). Loading DLLs is essential for a modded
        // game and outranks writable shared mmap. Read-only and copy-on-write
        // image mmap (i.e. DLL loading) work fine without it.

        // Rootless perf levers that always apply: large readahead and write
        // buffers cut the number of round-trips on big asset files. (Metadata is
        // already cached kernel-side via our entry/attr TTL.)
        // Without this the kernel takes a per-directory-inode lock around LOOKUP
        // and READDIR (fuse_lock_inode), which serialises exactly the metadata
        // traffic the extra event loops were added to absorb - measured: the
        // threads buy nothing until this is negotiated.
        let _ = config.add_capabilities(InitFlags::FUSE_PARALLEL_DIROPS);
        let _ = config.set_max_readahead(1 << 20);
        let _ = config.set_max_write(1 << 20);

        // A modded load order streams hundreds of BSA/BA2 archives plus loose
        // assets, and Wine opens files it never reads (metadata probes). Each
        // `open` retains a real fd (plus a passthrough registration), so the
        // default 1024 soft limit is reachable - and EMFILE surfaces in-game as an
        // asset that simply is not there, with no error text anywhere. Raise the
        // soft limit to the hard limit; best-effort, since a failure here only
        // returns us to the status quo.
        raise_fd_limit();
        Ok(())
    }

    fn forget(&self, _req: &Request, ino: INodeNo, nlookup: u64) {
        // The default batch_forget fans out to this, so both paths free inodes.
        self.inodes.lock_recover().forget(ino.0, nlookup);
    }

    fn open(&self, _req: &Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        Stats::bump(&self.stats.open);
        let vpath = match self.inodes.lock_recover().path(ino.0) {
            Some(p) => p,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };

        // Writes must copy up first so the backing file is the Overwrite copy,
        // never the read-only mod or game file.
        let accmode = flags.0 & libc::O_ACCMODE;
        let want_write = accmode == libc::O_WRONLY || accmode == libc::O_RDWR;
        let truncating = want_write && flags.0 & libc::O_TRUNC != 0;
        let real = if want_write {
            // O_TRUNC discards the existing content, so don't pay to copy the lower
            // file up first - just prepare the overwrite path (parents + clear
            // whiteout) and create it empty below. Otherwise copy-up so the backing
            // file is the Overwrite copy, never the read-only mod/game file.
            let prepared = if truncating {
                self.stack.prepare_overwrite(&vpath)
            } else {
                self.stack.open_for_write(&vpath)
            };
            match prepared {
                Ok(p) => p,
                Err(e) => {
                    reply.error(e.into());
                    return;
                }
            }
        } else {
            match self.stack.resolve_read(&vpath) {
                Some(p) => p,
                None => {
                    reply.error(Errno::ENOENT);
                    return;
                }
            }
        };

        if real.is_dir() {
            reply.opened(FileHandle(0), FopenFlags::empty());
            return;
        }

        let mut opts = OpenOptions::new();
        if want_write {
            opts.read(true).write(true);
            // For O_TRUNC the overwrite file may not exist yet (the copy-up was
            // skipped): create it and truncate, instead of the old open-then-set_len
            // which failed (EIO) when the path had no overwrite copy.
            if truncating {
                opts.create(true).truncate(true);
            }
        } else {
            opts.read(true);
        }
        let file = match opts.open(&real) {
            Ok(f) => f,
            Err(e) => {
                if trace_enabled("open") {
                    eprintln!(
                        "eidos-fuse: open FAILED /{vpath} -> {} : {e} (errno {:?})",
                        real.display(),
                        e.raw_os_error()
                    );
                }
                reply.error(e.into());
                return;
            }
        };

        // A write may have just copied this path up from a read-only layer. The
        // inode is unchanged but its backing file is now a different file on disk,
        // and the kernel can still be holding pages it read from the old one.
        if want_write {
            self.invalidate_page_cache(ino.0);
        }

        // Cache the open fd under a fresh handle; try to register it for kernel
        // passthrough (no-op fallback when rootless, where it returns EPERM).
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        let backing = if !passthrough_enabled() {
            None
        } else {
            match reply.open_backing(file.as_fd()) {
                Ok(b) => Some(b),
                Err(e) => {
                    // Passthrough is an optimisation, never a requirement: the
                    // daemon serves the reads itself when the kernel will not take
                    // a backing file. Worth SAYING, though - a silent fallback here
                    // turned into hours of looking elsewhere.
                    if trace_enabled("open") {
                        eprintln!("eidos-fuse: passthrough refused for /{vpath}: {e}");
                    }
                    None
                }
            }
        };
        let mut files = self.open_files.lock_recover();
        files.insert(
            fh,
            OpenFile { _real: real, file: Arc::new(file), _backing: backing },
        );
        let flags = file_open_flags();
        match files.get(&fh).unwrap()._backing.as_ref() {
            Some(b) => reply.opened_passthrough(FileHandle(fh), flags, b),
            None => reply.opened(FileHandle(fh), flags),
        }
    }

    fn release(
        &self,
        _req: &Request,
        _ino: INodeNo,
        fh: FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        // Drops the cached fd and any passthrough registration (no-op for fh 0).
        self.open_files.lock_recover().remove(&fh.0);
        reply.ok();
    }

    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let Some(parent_vpath) = self.inodes.lock_recover().path(parent.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let vpath = join(&parent_vpath, &name.to_string_lossy());

        // Tell Wine this directory is case-insensitive, so it stops proving it the
        // hard way.
        //
        // Wine has no way to ask a filesystem whether it folds case, so
        // `get_dir_case_sensitivity` sniffs for the marker CIOPFS leaves in every
        // directory it serves. Without it Wine assumes case-SENSITIVE, and every
        // lookup whose spelling does not match byte-for-byte falls back to reading
        // the WHOLE directory to search for a case-insensitive match. Bethesda
        // games ask for `data/ccbgssse001-fish.bsa` while the file is
        // `ccBGSSSE001-Fish.bsa`, so that fallback fires on nearly every asset.
        //
        // Measured on Skyrim SE through this mount: 4471 `.ciopfs` probes and 2236
        // full directory re-reads in EIGHT SECONDS, 195796 `opendir`s of Data in
        // ninety - the daemon pinned at 92% of a core and the game never reaching
        // its main menu. Eidos folds case in `resolve_read` already; the whole cost
        // was Wine not being told.
        //
        // Answered on lookup only, and deliberately absent from `readdir`: it is a
        // signal to Wine, not a file the game should ever enumerate or open.
        if name.as_bytes() == CIOPFS_MARKER {
            Stats::bump(&self.stats.marker);
            let ino = self.inodes.lock_recover().lookup(&vpath);
            reply.entry(&TTL, &self.marker_attr(ino), Generation(0));
            return;
        }

        // Resolve + stat without the inode lock held.
        let Some(real) = self.stack.resolve_read(&vpath) else {
            Stats::bump(&self.stats.lookup_miss);
            self.reply_negative(parent.0, &name.to_string_lossy(), reply);
            return;
        };
        match fs::symlink_metadata(&real) {
            Ok(meta) => {
                Stats::bump(&self.stats.lookup_hit);
                let ino = self.inodes.lock_recover().lookup(&vpath);
                self.record_alias(ino, parent.0, &name.to_string_lossy());
                reply.entry(&TTL, &self.attr(ino, &meta), Generation(0));
            }
            Err(_) => {
                Stats::bump(&self.stats.lookup_miss);
                self.reply_negative(parent.0, &name.to_string_lossy(), reply);
            }
        }
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        Stats::bump(&self.stats.getattr);
        let vpath = match self.inodes.lock_recover().path(ino.0) {
            Some(p) => p,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        match self.stack.resolve_read(&vpath).and_then(|r| fs::symlink_metadata(r).ok()) {
            Some(meta) => reply.attr(&TTL, &self.attr(ino.0, &meta)),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn readlink(&self, _req: &Request, ino: INodeNo, reply: ReplyData) {
        let vpath = match self.inodes.lock_recover().path(ino.0) {
            Some(p) => p,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        // Return the raw link target; the kernel resolves it within the mount,
        // so a relative symlink shipped by a mod points back into the merged view.
        match self.stack.resolve_read(&vpath).and_then(|r| fs::read_link(r).ok()) {
            Some(target) => reply.data(target.as_os_str().as_bytes()),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        Stats::bump(&self.stats.read);
        // Fast path: pread the cached fd from `open` (no re-resolve, no re-open,
        // offset-explicit so concurrent reads on one handle do not race).
        let cached = self.open_files.lock_recover().get(&fh.0).map(|o| o.file.clone());
        if let Some(file) = cached {
            match read_full_at(&file, offset, size as usize) {
                Ok(buf) => reply.data(&buf),
                Err(e) => reply.error(e.into()),
            }
            return;
        }

        // Fallback (e.g. fh 0): resolve by inode and read once.
        let vpath = match self.inodes.lock_recover().path(ino.0) {
            Some(p) => p,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        let Some(real) = self.stack.resolve_read(&vpath) else {
            reply.error(Errno::ENOENT);
            return;
        };
        match File::open(&real).and_then(|f| read_full_at(&f, offset, size as usize)) {
            Ok(buf) => reply.data(&buf),
            Err(e) => reply.error(e.into()),
        }
    }

    fn opendir(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        Stats::bump(&self.stats.opendir);
        // `EIDOS_FUSE_TRACE=opendir` names every directory the caller enumerates.
        // A game that re-walks one directory without end is invisible in the op
        // counters (they only total) and obvious here.
        if trace_enabled("opendir") {
            let p = self.inodes.lock_recover().path(ino.0).unwrap_or_default();
            eprintln!("eidos-fuse: opendir /{p}");
        }
        // Record the handle and do NOTHING else. The snapshot is built by the
        // first `readdir`, because an `opendir` is not a promise to enumerate:
        // Wine opens a directory just to `stat` the `.ciopfs` marker inside it,
        // on essentially every path lookup. Building the merged listing here
        // meant a full multi-layer scan plus an NTFS-collation sort per probe -
        // measured at 220000 of them in ninety seconds of Skyrim startup, for
        // listings nobody ever read. Offsets stay stable because the snapshot is
        // still taken exactly once per handle, just later.
        // Declining ONCE is the whole optimisation: the kernel sets `no_opendir`
        // on the connection and never sends another OPENDIR, opening and reading
        // directories from its own cache instead.
        //
        // This is the dominant cost in a real session. Measured on Skyrim SE:
        // 273214 directory opens, 94.4% of which never enumerated anything -
        // Wine issues an unconditional `openat` on the parent directory for
        // every lookup of a file that does not exist (ntdll's
        // get_dir_case_sensitivity_stat), and its cache for that answer is
        // compiled in on macOS only. The kernel has no cache for an open, so
        // every one of those reached this daemon. On a synthetic mount the
        // count falls from 23026 to 1, a directory open from 13.9 us to 2.8 us
        // and an enumeration from 102 us to 7.3 us - the kernel serving both
        // from the page cache instead of round-tripping to us.
        //
        // `readdir` already handles this: its fallback resolves the path from
        // the inode and reads through the by-path listing cache, so offsets stay
        // stable across calls without a per-handle snapshot.
        if self.no_opendir.load(Ordering::Relaxed) {
            reply.error(Errno::ENOSYS);
            return;
        }
        let Some(vpath) = self.inodes.lock_recover().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        self.stats.note_dir(&vpath);
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        self.open_dirs.lock_recover().insert(fh, OpenDir { ino: ino.0, vpath, entries: None });
        // CACHE_DIR lets the kernel keep the directory listing and serve repeat
        // enumerations itself. The Creation Engine's loose-file indexer and Wine's
        // directory probing re-walk the same directories relentlessly, and each
        // uncached readdir costs a merged multi-layer scan in `dir_snapshot`. Safe
        // for the same reason the long entry TTL is: mod layers are immutable for
        // the life of the mount, and anything written through the mount goes
        // through our own handlers.
        // FOPEN_CACHE_DIR only: see `file_open_flags` for why FOPEN_KEEP_CACHE is
        // not paired with it here any more.
        let mut flags = FopenFlags::empty();
        if !Cache::Dir.is_off() {
            flags |= FopenFlags::FOPEN_CACHE_DIR;
        }
        flags |= file_open_flags();
        reply.opened(FileHandle(fh), flags);
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        Stats::bump(&self.stats.readdir);
        // Take the snapshot on the first readdir of this handle, then serve every
        // later call from it; stop the moment the kernel buffer is full (the
        // offset we pass is the resume point: index of the next entry).
        //
        // The listing is built WITHOUT the map locked - it does real disk I/O -
        // and only then stored, so a slow enumeration of one directory cannot
        // block every other handle in the daemon.
        let known = {
            let dirs = self.open_dirs.lock_recover();
            dirs.get(&fh.0).map(|d| (d.ino, d.vpath.clone(), d.entries.is_some()))
        };
        if let Some((d_ino, d_vpath, ready)) = known {
            if !ready {
                let built = self.dir_snapshot(d_ino, &d_vpath);
                if let Some(d) = self.open_dirs.lock_recover().get_mut(&fh.0) {
                    // Another thread may have won the race; keep whichever
                    // snapshot landed first so offsets stay consistent.
                    d.entries.get_or_insert(built);
                }
            }
            let dirs = self.open_dirs.lock_recover();
            if let Some(entries) = dirs.get(&fh.0).and_then(|d| d.entries.as_ref()) {
                for (i, (e_ino, kind, name)) in entries.iter().enumerate().skip(offset as usize) {
                    if reply.add(INodeNo(*e_ino), (i + 1) as u64, *kind, name) {
                        break;
                    }
                }
                drop(dirs);
                reply.ok();
                return;
            }
        }

        // No handle: the kernel opens directories itself (see `opendir`). The
        // listing is pinned per inode for the length of one enumeration instead,
        // so the offsets a resume request carries keep meaning what they meant.
        let Some(vpath) = self.inodes.lock_recover().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let entries = {
            let mut live = self.enumerations.lock_recover();
            match (offset, live.get(&ino.0)) {
                // Resuming a walk we are already serving: same list, always.
                (o, Some(e)) if o > 0 => Arc::clone(e),
                // Starting one (offset 0), or resuming one whose pin was lost.
                // Build outside the lock: this does disk I/O and must not block
                // every other directory in the daemon.
                _ => {
                    drop(live);
                    let built = Arc::new(self.dir_snapshot(ino.0, &vpath));
                    live = self.enumerations.lock_recover();
                    Arc::clone(live.entry(ino.0).insert_entry(Arc::clone(&built)).get())
                }
            }
        };
        let mut sent = 0usize;
        let mut full = false;
        for (i, (e_ino, kind, name)) in entries.iter().enumerate().skip(offset as usize) {
            if reply.add(INodeNo(*e_ino), (i + 1) as u64, *kind, name) {
                full = true;
                break;
            }
            sent += 1;
        }
        // The buffer was not filled, so this chunk carried the tail: the walk is
        // over and the pin can go. A caller that abandons one mid-way leaves an
        // entry behind, which the next `offset == 0` on that inode replaces - the
        // map is bounded by the number of directories, not by walks.
        if !full {
            let _ = sent;
            self.enumerations.lock_recover().remove(&ino.0);
        }
        reply.ok();
    }

    fn releasedir(
        &self,
        _req: &Request,
        _ino: INodeNo,
        fh: FileHandle,
        _flags: OpenFlags,
        reply: ReplyEmpty,
    ) {
        Stats::bump(&self.stats.releasedir);
        // A handle whose snapshot was never taken was never enumerated: the caller
        // opened the directory to look at the inode, not to list it. That is what
        // `probe` counts, by construction and without reference to any caller.
        if let Some(d) = self.open_dirs.lock_recover().remove(&fh.0) {
            if d.entries.is_none() {
                Stats::bump(&self.stats.probe);
            }
        }
        reply.ok();
    }

    fn create(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        let Some(parent_vpath) = self.inodes.lock_recover().path(parent.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let vpath = join(&parent_vpath, &name.to_string_lossy());

        // Materialise the file (and any missing parents) in the Overwrite layer.
        // CREATE means "this file did not exist" (the kernel only sends it after a
        // negative lookup), so the new file must start EMPTY: prepare_overwrite
        // clears any whiteout WITHOUT copying a deleted lower-layer file up, and
        // truncate(true) guarantees zero length. Using open_for_write here would
        // resurrect the old bytes of a deleted mod/game file into the "new" file.
        let opened = (|| -> std::io::Result<(PathBuf, File, Metadata)> {
            let dest = self.stack.prepare_overwrite(&vpath)?;
            let file =
                OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&dest)?;
            let meta = fs::symlink_metadata(&dest)?;
            Ok((dest, file, meta))
        })();
        let (real, file, meta) = match opened {
            Ok(t) => t,
            Err(e) => {
                reply.error(e.into());
                return;
            }
        };

        self.dir_changed(parent.0, &name.to_string_lossy());
        let ino = self.inodes.lock_recover().lookup(&vpath);
        let attr = self.attr(ino, &meta);
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        let backing =
            if passthrough_enabled() { reply.open_backing(file.as_fd()).ok() } else { None };
        let mut files = self.open_files.lock_recover();
        files.insert(
            fh,
            OpenFile { _real: real, file: Arc::new(file), _backing: backing },
        );
        match files.get(&fh).unwrap()._backing.as_ref() {
            Some(b) => reply.created_passthrough(
                &TTL,
                &attr,
                Generation(0),
                FileHandle(fh),
                FopenFlags::empty(),
                b,
            ),
            None => reply.created(&TTL, &attr, Generation(0), FileHandle(fh), FopenFlags::empty()),
        }
    }

    fn write(
        &self,
        _req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        offset: u64,
        data: &[u8],
        _write_flags: WriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyWrite,
    ) {
        Stats::bump(&self.stats.write);
        // Fast path: pwrite the cached (copied-up) fd from `open`/`create`.
        let cached = self.open_files.lock_recover().get(&fh.0).map(|o| o.file.clone());
        if let Some(file) = cached {
            match write_all_at(&file, data, offset) {
                Ok(()) => reply.written(data.len() as u32),
                Err(e) => reply.error(e.into()),
            }
            return;
        }

        // Fallback (e.g. fh 0): copy up and write once.
        let vpath = match self.inodes.lock_recover().path(ino.0) {
            Some(p) => p,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        let written = (|| -> std::io::Result<()> {
            let dest = self.stack.open_for_write(&vpath)?;
            let f = OpenOptions::new().write(true).open(&dest)?;
            write_all_at(&f, data, offset)
        })();
        match written {
            Ok(()) => reply.written(data.len() as u32),
            Err(e) => reply.error(e.into()),
        }
    }

    fn mkdir(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let Some(parent_vpath) = self.inodes.lock_recover().path(parent.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let vpath = join(&parent_vpath, &name.to_string_lossy());
        let meta = match self.stack.make_dir(&vpath).and_then(fs::symlink_metadata) {
            Ok(m) => m,
            Err(e) => {
                reply.error(e.into());
                return;
            }
        };
        self.dir_changed(parent.0, &name.to_string_lossy());
        let ino = self.inodes.lock_recover().lookup(&vpath);
        reply.entry(&TTL, &self.attr(ino, &meta), Generation(0));
    }

    fn symlink(
        &self,
        _req: &Request,
        parent: INodeNo,
        link_name: &OsStr,
        target: &Path,
        reply: ReplyEntry,
    ) {
        let Some(parent_vpath) = self.inodes.lock_recover().path(parent.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let vpath = join(&parent_vpath, &link_name.to_string_lossy());
        // Create the symlink in the Overwrite layer; it shadows any lower entry.
        let made = (|| -> std::io::Result<Metadata> {
            let dest = self.stack.prepare_overwrite(&vpath)?;
            let _ = fs::remove_file(&dest); // replace any existing overwrite entry
            std::os::unix::fs::symlink(target, &dest)?;
            fs::symlink_metadata(&dest)
        })();
        match made {
            Ok(meta) => {
                self.dir_changed(parent.0, &link_name.to_string_lossy());
                let ino = self.inodes.lock_recover().lookup(&vpath);
                reply.entry(&TTL, &self.attr(ino, &meta), Generation(0));
            }
            Err(e) => reply.error(e.into()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn setattr(
        &self,
        _req: &Request,
        ino: INodeNo,
        mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<FileHandle>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<BsdFileFlags>,
        reply: ReplyAttr,
    ) {
        let vpath = match self.inodes.lock_recover().path(ino.0) {
            Some(p) => p,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        // A setattr on a path that no longer resolves (notably the kernel
        // flushing attributes on a just-unlinked inode under writeback_cache)
        // must not copy it back up and resurrect a deleted file. POSIX-correct
        // anyway: you cannot change attributes of something that is not there.
        if self.stack.resolve_read(&vpath).is_none() {
            reply.error(Errno::ENOENT);
            return;
        }
        // Any change must land in the Overwrite layer, so copy up first if the
        // path still lives only in a lower layer. We apply truncate, mode, and
        // timestamps; ownership is intentionally ignored (the game runs as us).
        if mode.is_some() || size.is_some() || atime.is_some() || mtime.is_some() {
            let r = (|| -> std::io::Result<()> {
                let dest = self.stack.open_for_write(&vpath)?;
                // Only a size change needs the file open, and the open must come
                // AFTER any mode change: Wine clearing FILE_ATTRIBUTE_READONLY
                // arrives as a mode-only setattr, and opening read-write first
                // would fail EACCES on the very file we are about to make writable.
                if let Some(m) = mode {
                    fs::set_permissions(&dest, Permissions::from_mode(m & 0o7777))?;
                }
                if let Some(sz) = size {
                    let f =
                        OpenOptions::new().create(true).write(true).truncate(false).open(&dest)?;
                    f.set_len(sz)?;
                }
                if atime.is_some() || mtime.is_some() {
                    set_times(&dest, atime, mtime)?;
                }
                Ok(())
            })();
            if let Err(e) = r {
                reply.error(e.into());
                return;
            }
        }
        match self.stack.resolve_read(&vpath).and_then(|r| fs::symlink_metadata(r).ok()) {
            Some(meta) => reply.attr(&TTL, &self.attr(ino.0, &meta)),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn flush(&self, _req: &Request, _ino: INodeNo, _fh: FileHandle, _lock: LockOwner, reply: ReplyEmpty) {
        reply.ok();
    }

    fn fsync(&self, _req: &Request, _ino: INodeNo, fh: FileHandle, _datasync: bool, reply: ReplyEmpty) {
        // Make saves durable: flush the backing fd if this handle has one.
        let cached = self.open_files.lock_recover().get(&fh.0).map(|o| o.file.clone());
        match cached {
            Some(file) => match file.sync_all() {
                Ok(()) => reply.ok(),
                Err(e) => reply.error(e.into()),
            },
            None => reply.ok(),
        }
    }

    fn unlink(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(vpath) = self.child(parent, name) else {
            reply.error(Errno::ENOENT);
            return;
        };
        if self.stack.resolve_read(&vpath).is_none() {
            reply.error(Errno::ENOENT);
            return;
        }
        match self.stack.remove(&vpath) {
            Ok(()) => {
                self.dir_changed(parent.0, &name.to_string_lossy());
                reply.ok()
            }
            Err(e) => reply.error(e.into()),
        }
    }

    fn rmdir(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(vpath) = self.child(parent, name) else {
            reply.error(Errno::ENOENT);
            return;
        };
        if self.stack.resolve_read(&vpath).is_none() {
            reply.error(Errno::ENOENT);
            return;
        }
        // POSIX: rmdir of a non-empty directory must fail rather than recurse.
        if !self.stack.list_dir(&vpath).is_empty() {
            reply.error(Errno::from_i32(libc::ENOTEMPTY));
            return;
        }
        match self.stack.remove(&vpath) {
            Ok(()) => {
                // The directory itself is gone, so its own cached listing must go
                // too, not just its parent's.
                self.invalidate_dir_cache(&vpath);
                self.dir_changed(parent.0, &name.to_string_lossy());
                reply.ok()
            }
            Err(e) => reply.error(e.into()),
        }
    }

    fn rename(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        newparent: INodeNo,
        newname: &OsStr,
        flags: RenameFlags,
        reply: ReplyEmpty,
    ) {
        let (Some(from), Some(to)) = (self.child(parent, name), self.child(newparent, newname)) else {
            reply.error(Errno::ENOENT);
            return;
        };
        if self.stack.resolve_read(&from).is_none() {
            reply.error(Errno::ENOENT);
            return;
        }
        let dest_exists = self.stack.resolve_read(&to).is_some();
        if flags.contains(RenameFlags::RENAME_NOREPLACE) && dest_exists {
            reply.error(Errno::from_i32(libc::EEXIST));
            return;
        }
        if flags.contains(RenameFlags::RENAME_EXCHANGE) {
            // The resolver cannot swap two paths atomically yet; refuse rather
            // than silently doing a one-way move that would lose the destination.
            reply.error(Errno::from_i32(libc::ENOSYS));
            return;
        }
        match self.stack.rename(&from, &to) {
            Ok(()) => {
                let moved = self.inodes.lock_recover().rename(&from, &to);
                if let Some(ino) = moved {
                    self.invalidate_stale_aliases(ino, newparent.0, &newname.to_string_lossy());
                    self.record_alias(ino, newparent.0, &newname.to_string_lossy());
                }
                // The destination name may have been probed and cached absent.
                self.dir_changed(newparent.0, &newname.to_string_lossy());
                // The SOURCE directory lost a name too, and it is a different
                // directory whenever the rename crosses parents.
                self.dir_changed(parent.0, &name.to_string_lossy());
                // A renamed DIRECTORY rebinds every path beneath it, so every
                // cached listing keyed on an old descendant path is now wrong.
                // Cheap and correct beats clever here: renames are rare, a rebuilt
                // listing costs one merge, and a stale one is a directory the game
                // sees wrongly for the life of the mount.
                if self.stack.resolve_read(&to).is_some_and(|p| p.is_dir()) {
                    self.dir_cache.lock_recover().clear();
                }
                reply.ok();
            }
            Err(e) => reply.error(e.into()),
        }
    }

    fn getxattr(&self, _req: &Request, ino: INodeNo, name: &OsStr, size: u32, reply: ReplyXattr) {
        let Some(vpath) = self.inodes.lock_recover().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let Some(real) = self.stack.resolve_read(&vpath) else {
            reply.error(Errno::ENOENT);
            return;
        };
        match xattr_get(&real, name) {
            // Two-phase: size 0 = probe the length; otherwise return the bytes.
            Ok(val) if size == 0 => reply.size(val.len() as u32),
            Ok(val) if val.len() as u32 <= size => reply.data(&val),
            Ok(_) => reply.error(Errno::from_i32(libc::ERANGE)),
            Err(e) => reply.error(to_errno(&e)),
        }
    }

    fn setxattr(
        &self,
        _req: &Request,
        ino: INodeNo,
        name: &OsStr,
        value: &[u8],
        flags: i32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        // Wine stores Windows attributes (hidden/system/readonly) in
        // user.DOSATTRIB; proxy the write to the copied-up backing file.
        let Some(vpath) = self.inodes.lock_recover().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        // Same guard as setattr: `open_for_write` clears the whiteout and copies
        // the lower file up, so an xattr write against a path that no longer
        // resolves would RESURRECT a deleted file. Wine issues these constantly,
        // so this is expected traffic, not an edge case.
        if self.stack.resolve_read(&vpath).is_none() {
            reply.error(Errno::ENOENT);
            return;
        }
        let r = (|| -> std::io::Result<()> {
            let dest = self.stack.open_for_write(&vpath)?;
            xattr_set(&dest, name, value, flags)
        })();
        match r {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(to_errno(&e)),
        }
    }

    fn listxattr(&self, _req: &Request, ino: INodeNo, size: u32, reply: ReplyXattr) {
        let Some(vpath) = self.inodes.lock_recover().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let Some(real) = self.stack.resolve_read(&vpath) else {
            reply.error(Errno::ENOENT);
            return;
        };
        match xattr_list(&real) {
            Ok(list) if size == 0 => reply.size(list.len() as u32),
            Ok(list) if list.len() as u32 <= size => reply.data(&list),
            Ok(_) => reply.error(Errno::from_i32(libc::ERANGE)),
            Err(e) => reply.error(to_errno(&e)),
        }
    }

    fn removexattr(&self, _req: &Request, ino: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(vpath) = self.inodes.lock_recover().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        // See `setxattr`: without this, removing an attribute from a deleted path
        // copies the lower file back up and un-deletes it.
        if self.stack.resolve_read(&vpath).is_none() {
            reply.error(Errno::ENOENT);
            return;
        }
        let r = (|| -> std::io::Result<()> {
            let dest = self.stack.open_for_write(&vpath)?;
            xattr_remove(&dest, name)
        })();
        match r {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(to_errno(&e)),
        }
    }

    // statvfs fields are c_ulong (u64 on 64-bit, u32 on 32-bit); the `as u64`
    // casts are no-ops here but keep the conversion correct on 32-bit targets.
    #[allow(clippy::unnecessary_cast)]
    fn statfs(&self, _req: &Request, _ino: INodeNo, reply: ReplyStatfs) {
        match statvfs_of(self.stack.overwrite_root()) {
            Ok(s) => reply.statfs(
                s.f_blocks as u64,
                s.f_bfree as u64,
                s.f_bavail as u64,
                s.f_files as u64,
                s.f_ffree as u64,
                s.f_bsize as u32,
                s.f_namemax as u32,
                s.f_frsize as u32,
            ),
            Err(e) => reply.error(e.into()),
        }
    }
}

/// Read up to `size` bytes at `offset` via `pread`, looping over short reads and
/// stopping at EOF. `pread` does not disturb a shared file offset, so concurrent
/// reads on one handle are safe.
fn read_full_at(file: &File, mut offset: u64, size: usize) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; size];
    let mut filled = 0;
    while filled < size {
        match file.read_at(&mut buf[filled..], offset) {
            Ok(0) => break, // EOF
            Ok(n) => {
                filled += n;
                offset += n as u64;
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    buf.truncate(filled);
    Ok(buf)
}

/// Write all of `data` at `offset` via `pwrite`, looping over short writes.
fn write_all_at(file: &File, mut data: &[u8], mut offset: u64) -> std::io::Result<()> {
    while !data.is_empty() {
        match file.write_at(data, offset) {
            Ok(0) => return Err(std::io::Error::new(ErrorKind::WriteZero, "write returned 0")),
            Ok(n) => {
                data = &data[n..];
                offset += n as u64;
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Map a `TimeOrNow` (or its absence) onto a `timespec` for `utimensat`.
fn to_timespec(t: Option<TimeOrNow>) -> libc::timespec {
    match t {
        None => libc::timespec { tv_sec: 0, tv_nsec: libc::UTIME_OMIT },
        Some(TimeOrNow::Now) => libc::timespec { tv_sec: 0, tv_nsec: libc::UTIME_NOW },
        Some(TimeOrNow::SpecificTime(st)) => {
            let d = st.duration_since(UNIX_EPOCH).unwrap_or_default();
            libc::timespec {
                tv_sec: d.as_secs() as libc::time_t,
                tv_nsec: d.subsec_nanos() as _,
            }
        }
    }
}

/// Set a file's access/modification times (each optional) via `utimensat`.
fn set_times(path: &Path, atime: Option<TimeOrNow>, mtime: Option<TimeOrNow>) -> std::io::Result<()> {
    let times = [to_timespec(atime), to_timespec(mtime)];
    let c = cpath(path)?;
    // SAFETY: valid C path and a 2-element timespec array, per utimensat(2).
    let rc = unsafe { libc::utimensat(libc::AT_FDCWD, c.as_ptr(), times.as_ptr(), 0) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// A path as a C string, rejecting embedded NULs.
fn cpath(path: &Path) -> std::io::Result<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "path contains NUL"))
}

/// Map an `io::Error` to a FUSE errno, defaulting to EIO when there is no raw
/// OS code (so xattr ENODATA/ENOTSUP/ERANGE round-trip to the game unchanged).
fn to_errno(e: &std::io::Error) -> Errno {
    Errno::from_i32(e.raw_os_error().unwrap_or(libc::EIO))
}

/// Read extended attribute `name` from `path`.
fn xattr_get(path: &Path, name: &OsStr) -> std::io::Result<Vec<u8>> {
    let p = cpath(path)?;
    let n = CString::new(name.as_bytes())
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "xattr name contains NUL"))?;
    // SAFETY: valid C strings; first call sizes the buffer, second fills it.
    let len = unsafe { libc::getxattr(p.as_ptr(), n.as_ptr(), std::ptr::null_mut(), 0) };
    if len < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let mut buf = vec![0u8; len as usize];
    let got = unsafe {
        libc::getxattr(p.as_ptr(), n.as_ptr(), buf.as_mut_ptr() as *mut libc::c_void, buf.len())
    };
    if got < 0 {
        return Err(std::io::Error::last_os_error());
    }
    buf.truncate(got as usize);
    Ok(buf)
}

/// Set extended attribute `name` on `path`.
fn xattr_set(path: &Path, name: &OsStr, value: &[u8], flags: i32) -> std::io::Result<()> {
    let p = cpath(path)?;
    let n = CString::new(name.as_bytes())
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "xattr name contains NUL"))?;
    // SAFETY: valid C strings and a sized value buffer, per setxattr(2).
    let rc = unsafe {
        libc::setxattr(
            p.as_ptr(),
            n.as_ptr(),
            value.as_ptr() as *const libc::c_void,
            value.len(),
            flags,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// List extended attribute names of `path` (NUL-separated, as the kernel returns).
fn xattr_list(path: &Path) -> std::io::Result<Vec<u8>> {
    let p = cpath(path)?;
    // SAFETY: valid C path; first call sizes the buffer, second fills it.
    let len = unsafe { libc::listxattr(p.as_ptr(), std::ptr::null_mut(), 0) };
    if len < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let mut buf = vec![0u8; len as usize];
    let got =
        unsafe { libc::listxattr(p.as_ptr(), buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if got < 0 {
        return Err(std::io::Error::last_os_error());
    }
    buf.truncate(got as usize);
    Ok(buf)
}

/// Remove extended attribute `name` from `path`.
fn xattr_remove(path: &Path, name: &OsStr) -> std::io::Result<()> {
    let p = cpath(path)?;
    let n = CString::new(name.as_bytes())
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidInput, "xattr name contains NUL"))?;
    // SAFETY: valid C strings, per removexattr(2).
    let rc = unsafe { libc::removexattr(p.as_ptr(), n.as_ptr()) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// `statvfs(2)` of a path, for reporting real free space to the game.
fn statvfs_of(path: &Path) -> std::io::Result<libc::statvfs> {
    let c = cpath(path)?;
    // SAFETY: valid C path and a zeroed statvfs out-param; we check the return.
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(st)
}

#[cfg(test)]
mod tests {
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
}
