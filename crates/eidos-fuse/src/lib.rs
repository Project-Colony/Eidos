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
use std::sync::atomic::{AtomicU64, Ordering};
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

/// Caching is off when `EIDOS_FUSE_NO_CACHE` is set to anything but `0`. The
/// escape hatch ships WITH the caching so that "the game sees stale data" can be
/// tested against caching as the suspect in one run, instead of being chased
/// through the code.
fn caching_disabled() -> bool {
    std::env::var("EIDOS_FUSE_NO_CACHE").is_ok_and(|v| v != "0")
}

static TTL: std::sync::LazyLock<Duration> = std::sync::LazyLock::new(|| {
    Duration::from_secs(if caching_disabled() { 0 } else { TTL_SECS })
});
static NEG_TTL: std::sync::LazyLock<Duration> = std::sync::LazyLock::new(|| {
    Duration::from_secs(if caching_disabled() { 0 } else { NEG_TTL_SECS })
});

/// A directory entry in an `opendir` snapshot: `(inode, kind, name)`.
type DirEntry = (u64, FileType, String);

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
    readdir: AtomicU64,
    open: AtomicU64,
    read: AtomicU64,
    write: AtomicU64,
}

impl Stats {
    fn bump(c: &AtomicU64) {
        c.fetch_add(1, Ordering::Relaxed);
    }

    fn enabled() -> bool {
        std::env::var("EIDOS_FUSE_STATS").is_ok_and(|v| v != "0")
    }

    fn report(&self) -> String {
        let g = |c: &AtomicU64| c.load(Ordering::Relaxed);
        let (hit, miss) = (g(&self.lookup_hit), g(&self.lookup_miss));
        let total = hit + miss;
        let miss_pct = if total == 0 { 0.0 } else { miss as f64 * 100.0 / total as f64 };
        format!(
            "eidos-fuse stats: lookup {total} ({miss} missing, {miss_pct:.1}%), \
             getattr {}, readdir {}, open {}, read {}, write {}",
            g(&self.getattr),
            g(&self.readdir),
            g(&self.open),
            g(&self.read),
            g(&self.write),
        )
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
    fn rename(&mut self, from: &str, to: &str) {
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
        if let Some(ino) = self.by_path.remove(&from_key) {
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
    }
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
    /// Per-handle directory snapshots taken at `opendir`, for stable `readdir`.
    open_dirs: Mutex<HashMap<u64, Vec<DirEntry>>>,
    next_fh: AtomicU64,
    stats: Stats,
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
    let ft = meta.file_type();
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
    config
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
            next_fh: AtomicU64::new(1),
            stats: Stats::default(),
        }
    }

    /// Mount at `mountpoint`, blocking until the filesystem is unmounted.
    pub fn mount(self, mountpoint: &Path) -> std::io::Result<()> {
        fuser::mount2(self, mountpoint, &mount_config())
    }

    /// Mount at `mountpoint` on a background thread, returning a session handle.
    /// Dropping the handle unmounts.
    pub fn spawn(self, mountpoint: &Path) -> std::io::Result<BackgroundSession> {
        fuser::spawn_mount2(self, mountpoint, &mount_config())
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
    fn reply_negative(&self, reply: ReplyEntry) {
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

        // Merge + stat each child before taking the inode lock.
        let children: Vec<(String, FileType)> = self
            .stack
            .list_dir(vpath)
            .into_iter()
            .map(|(name, real)| {
                let kind = fs::symlink_metadata(&real).map_or(FileType::RegularFile, |m| kind_of(&m));
                (name, kind)
            })
            .collect();

        let mut entries = Vec::with_capacity(children.len() + 2);
        entries.push((ino, FileType::Directory, ".".to_string()));
        entries.push((parent_ino, FileType::Directory, "..".to_string()));
        let mut inodes = self.inodes.lock_recover();
        for (name, kind) in children {
            let child_ino = inodes.intern(&join(vpath, &name));
            entries.push((child_ino, kind, name));
        }
        entries
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
        // This is what lets Windows DLLs (SKSE plugins) image-map natively, which a
        // userspace daemon cannot serve reliably (demand-paged image pages get
        // corrupted, so relocation-heavy plugins crash on load). Registering a
        // backing fd needs CAP_SYS_ADMIN in the *initial* user namespace, so it
        // engages only when Eidos runs privileged (`setcap cap_sys_admin+ep` plus a
        // plain mount namespace, no userns). Rootless, `open_backing` returns EPERM
        // and we fall back to serving reads/writes ourselves (DLLs may then fail).
        let _ = config.add_capabilities(InitFlags::FUSE_PASSTHROUGH);
        let _ = config.set_max_stack_depth(1);

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
                reply.error(e.into());
                return;
            }
        };

        // Cache the open fd under a fresh handle; try to register it for kernel
        // passthrough (no-op fallback when rootless, where it returns EPERM).
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        let backing = reply.open_backing(file.as_fd()).ok();
        let mut files = self.open_files.lock_recover();
        files.insert(
            fh,
            OpenFile { _real: real, file: Arc::new(file), _backing: backing },
        );
        match files.get(&fh).unwrap()._backing.as_ref() {
            Some(b) => reply.opened_passthrough(FileHandle(fh), FopenFlags::empty(), b),
            None => reply.opened(FileHandle(fh), FopenFlags::empty()),
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
        // Resolve + stat without the inode lock held.
        let Some(real) = self.stack.resolve_read(&vpath) else {
            Stats::bump(&self.stats.lookup_miss);
            self.reply_negative(reply);
            return;
        };
        match fs::symlink_metadata(&real) {
            Ok(meta) => {
                Stats::bump(&self.stats.lookup_hit);
                let ino = self.inodes.lock_recover().lookup(&vpath);
                reply.entry(&TTL, &self.attr(ino, &meta), Generation(0));
            }
            Err(_) => {
                Stats::bump(&self.stats.lookup_miss);
                self.reply_negative(reply);
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
        Stats::bump(&self.stats.readdir);
        // Snapshot the listing now so offsets stay valid even if the directory
        // changes before releasedir (the conformant readdir pattern).
        let Some(vpath) = self.inodes.lock_recover().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let entries = self.dir_snapshot(ino.0, &vpath);
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        self.open_dirs.lock_recover().insert(fh, entries);
        // CACHE_DIR lets the kernel keep the directory listing and serve repeat
        // enumerations itself. The Creation Engine's loose-file indexer and Wine's
        // directory probing re-walk the same directories relentlessly, and each
        // uncached readdir costs a merged multi-layer scan in `dir_snapshot`. Safe
        // for the same reason the long entry TTL is: mod layers are immutable for
        // the life of the mount, and anything written through the mount goes
        // through our own handlers.
        let flags = if caching_disabled() {
            FopenFlags::empty()
        } else {
            FopenFlags::FOPEN_CACHE_DIR | FopenFlags::FOPEN_KEEP_CACHE
        };
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
        // Serve from the opendir snapshot; stop the moment the kernel buffer is
        // full (the offset we pass is the resume point: index of the next entry).
        {
            let dirs = self.open_dirs.lock_recover();
            if let Some(entries) = dirs.get(&fh.0) {
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

        // Fallback: the kernel issued readdir without an opendir snapshot.
        let Some(vpath) = self.inodes.lock_recover().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let entries = self.dir_snapshot(ino.0, &vpath);
        for (i, (e_ino, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(INodeNo(e_ino), (i + 1) as u64, kind, name) {
                break;
            }
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
        self.open_dirs.lock_recover().remove(&fh.0);
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

        let ino = self.inodes.lock_recover().lookup(&vpath);
        let attr = self.attr(ino, &meta);
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        let backing = reply.open_backing(file.as_fd()).ok();
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
            Ok(()) => reply.ok(),
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
            Ok(()) => reply.ok(),
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
                self.inodes.lock_recover().rename(&from, &to);
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
