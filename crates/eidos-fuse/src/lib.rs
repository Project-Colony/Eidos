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
use std::ffi::OsStr;
use std::fs::Metadata;
use std::os::unix::fs::{MetadataExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::UNIX_EPOCH;

use eidos_core::LayerStack;
use fuser::{
    BackgroundSession, FileAttr, FileType,
    Generation, INodeNo, ReplyEntry,
};

mod config;
mod ops;
mod inodes;
mod stats;
mod sys;
#[cfg(test)]
mod tests;

use config::*;
use inodes::*;
use stats::*;
use sys::*;

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

impl<'a> Timed<'a> {
    fn start(total: &'a AtomicU64) -> Option<Timed<'a>> {
        STATS_ON.then(|| Timed { total, start: std::time::Instant::now() })
    }
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
        fuser::mount(self, mountpoint, &mount_config())
    }

    /// Mount at `mountpoint` on a background thread, returning a session handle.
    /// Dropping the handle unmounts.
    pub fn spawn(self, mountpoint: &Path) -> std::io::Result<BackgroundSession> {
        // Build the session by hand rather than via spawn_mount, so the kernel
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
