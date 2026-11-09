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
//! - Kernel passthrough is off by default and needs CAP_SYS_ADMIN in the
//!   initial user namespace, so the rootless daemon serves reads and writes
//!   itself. `writeback_cache` is not negotiated (`init` says why).

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::Metadata;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, UNIX_EPOCH};

/// One parent's recorded negatives: folded name, then (exact spelling, recorded at).
type NegativeDir = HashMap<String, Vec<(String, Instant)>>;

use eidos_core::LayerStack;
use fuser::{BackgroundSession, BackingId, FileAttr, FileType, Generation, INodeNo, ReplyEntry};

mod timestamps;
pub use timestamps::{read_plugin_mtimes, recover_plugin_mtimes, PluginTimestamps};

mod config;
mod inodes;
mod ops;
mod passthrough;
mod stats;
mod sys;
#[cfg(test)]
mod tests;

use config::*;
use inodes::*;
use passthrough::*;
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
        STATS_ON.then(|| Timed {
            total,
            start: std::time::Instant::now(),
        })
    }
}

/// A slot's identity prevents invalidated scans from refilling the cache. Slots
/// are created by reads only; mutations never leave per-path tombstones behind.
#[derive(Default)]
struct DirCache {
    entries: HashMap<String, DirCacheEntry>,
}

struct DirCacheEntry {
    scan: Arc<()>,
    children: Option<Listing>,
}

impl DirCache {
    /// Return a cache hit, or share the identity of this directory's current scan.
    fn get_or_start(&mut self, vpath: &str) -> Result<Listing, Arc<()>> {
        if let Some(entry) = self.entries.get(vpath) {
            return match &entry.children {
                Some(hit) => Ok(Arc::clone(hit)),
                None => Err(Arc::clone(&entry.scan)),
            };
        }
        let scan = Arc::new(());
        self.entries.insert(
            vpath.to_string(),
            DirCacheEntry {
                scan: Arc::clone(&scan),
                children: None,
            },
        );
        Err(scan)
    }

    fn publish(&mut self, vpath: &str, scan: &Arc<()>, children: Listing) -> Option<Listing> {
        let entry = self.entries.get_mut(vpath)?;
        if !Arc::ptr_eq(&entry.scan, scan) {
            return None;
        }
        // Concurrent scans of the same slot retain the first completed listing.
        Some(Arc::clone(entry.children.get_or_insert(children)))
    }

    fn invalidate(&mut self, vpath: Option<&str>) {
        if let Some(vpath) = vpath {
            self.entries.remove(vpath);
        } else {
            self.entries.clear();
        }
    }
}

/// The Eidos union filesystem over a [`LayerStack`].
pub struct Eidos {
    stack: LayerStack,
    plugin_timestamps: Option<Mutex<timestamps::ProjectionState>>,
    plugin_atimes: Mutex<std::collections::BTreeMap<String, std::time::SystemTime>>,
    inodes: Mutex<Inodes>,
    uid: u32,
    gid: u32,
    open_files: Mutex<HashMap<u64, OpenFile>>,
    /// The backing id each inode shares among its passthrough opens, and its
    /// live opens of each kind. See `passthrough.rs`.
    passthrough: Mutex<Passthrough<BackingId>>,
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
    dir_cache: Mutex<DirCache>,
    next_fh: AtomicU64,
    stats: Stats,
    /// Negative dentries handed to the kernel: parent inode, then ASCII-folded
    /// name, then the exact spellings denied with the time each was recorded.
    ///
    /// The kernel keys its dentry cache on the EXACT name bytes while Eidos
    /// resolves case-insensitively, so a negative cached for `Foo.esp` would
    /// survive a later create of `foo.esp` and the game would be told a file it
    /// can plainly see does not exist. We remember what we denied so that a
    /// create can invalidate precisely those spellings. A record is dropped
    /// once it is older than the kernel TTL it justified ([`NEG_TTL_RECORDED`]).
    negatives: Mutex<HashMap<u64, NegativeDir>>,
    /// Positive dentries handed to the kernel, keyed by inode: the
    /// `(parent_ino, exact name)` spellings it now caches for that file.
    ///
    /// The case fold means one inode is reachable through several kernel
    /// dentries. A rename moves only the spelling it was given, so every OTHER
    /// recorded spelling would keep resolving to the moved file for the whole
    /// entry TTL and serve its contents under a name that no longer exists.
    aliases: Mutex<HashMap<u64, Vec<(u64, String)>>>,
    /// Directory inode -> whether a name in it needs Unicode case folding, which
    /// Eidos does not do, so the directory must not claim to fold case. Read from
    /// the layers on first ask; `dir_changed` sets it and nothing clears it.
    unicode_case_dirs: Mutex<HashMap<u64, bool>>,
    /// Set once the session is mounted: feeds the one thread that owns the
    /// kernel notifier and pushes these invalidations.
    notifier: Arc<Mutex<Option<std::sync::mpsc::Sender<Inval>>>>,
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

/// A kernel cache invalidation, applied by the notifier thread.
pub(crate) enum Inval {
    /// Drop the dentry `name` in directory inode `.0`.
    Entry(u64, std::ffi::OsString),
    /// Drop all cached pages of an inode.
    Inode(u64),
}

impl Eidos {
    /// Build a union over the given layers (highest priority first) and a
    /// writable overwrite layer.
    pub fn new(layers: Vec<PathBuf>, overwrite: PathBuf) -> Self {
        Self::new_with_unindexed_subtree(layers, overwrite, None)
    }

    /// Leave a subtree unindexed when another mount will cover it. Reads still
    /// fall back to the layers, including before that covering mount is ready.
    pub fn new_with_unindexed_subtree(
        layers: Vec<PathBuf>,
        overwrite: PathBuf,
        unindexed_subtree: Option<&Path>,
    ) -> Self {
        Self::new_with_readonly_overwrite(layers, overwrite, unindexed_subtree, None)
    }

    /// Place an inherited Overwrite below this session's writable Overwrite.
    pub fn new_with_readonly_overwrite(
        layers: Vec<PathBuf>,
        overwrite: PathBuf,
        unindexed_subtree: Option<&Path>,
        readonly_overwrite: Option<PathBuf>,
    ) -> Self {
        // SAFETY: getuid/getgid always succeed with no preconditions.
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
        Self {
            stack: LayerStack::new_with_readonly_overwrite(
                layers,
                overwrite,
                unindexed_subtree,
                readonly_overwrite,
            ),
            plugin_timestamps: None,
            plugin_atimes: Mutex::new(Default::default()),
            inodes: Mutex::new(Inodes::new()),
            uid,
            gid,
            open_files: Mutex::new(HashMap::new()),
            passthrough: Mutex::new(Passthrough::default()),
            open_dirs: Mutex::new(HashMap::new()),
            dir_cache: Mutex::new(DirCache::default()),
            next_fh: AtomicU64::new(1),
            no_opendir: AtomicBool::new(false),
            enumerations: Mutex::new(HashMap::new()),
            stats: Stats::default(),
            negatives: Mutex::new(HashMap::new()),
            aliases: Mutex::new(HashMap::new()),
            unicode_case_dirs: Mutex::new(HashMap::new()),
            notifier: Arc::new(Mutex::new(None)),
        }
    }

    /// Size of the lower index this filesystem was built with, if any.
    pub fn index_summary(&self) -> Option<eidos_core::IndexSummary> {
        self.stack.index_summary()
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
        let notifier = session.notifier();
        let (tx, rx) = std::sync::mpsc::channel();
        // Ends when the filesystem, and with it the sender, is dropped.
        std::thread::spawn(move || {
            for msg in rx {
                match msg {
                    Inval::Entry(parent, name) => {
                        let _ = notifier.inval_entry(INodeNo(parent), &name);
                    }
                    Inval::Inode(ino) => {
                        let _ = notifier.inval_inode(INodeNo(ino), 0, 0);
                    }
                }
            }
        });
        *notifier_slot.lock_recover() = Some(tx);
        session.spawn()
    }

    /// Queue kernel notifications for the notifier thread. Never send them from
    /// inside a request handler: the kernel may need locks the request holds, so
    /// a direct call deadlocks the mount. The queue returns at once.
    pub(crate) fn notify(&self, msgs: impl IntoIterator<Item = Inval>) {
        if let Some(tx) = self.notifier.lock_recover().as_ref() {
            for m in msgs {
                let _ = tx.send(m);
            }
        }
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
    /// A recorded miss gets an hour: a differently-cased create invalidates the
    /// recorded spellings through the notifier. A miss the per-parent cap could
    /// not record keeps the 60 s TTL, since nothing could invalidate it. The
    /// kernel keys on exact name bytes while Eidos folds case, which is why the
    /// record exists. Creation through this mount is safe regardless (the kernel
    /// re-issues a real lookup for O_CREAT and for every LOOKUP_EXCL path).
    ///
    /// `definitive` is true only for a miss the immutable index answered. A
    /// transient stat failure on a file that exists is not definitive: it is
    /// neither recorded nor given more than the short TTL, since no create
    /// would ever invalidate it.
    fn reply_negative(&self, parent: u64, name: &str, definitive: bool, reply: ReplyEntry) {
        let ttl = self.negative_ttl(parent, name, definitive);
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
        reply.entry(ttl, &blank, Generation(0));
    }

    fn negative_ttl(&self, parent: u64, name: &str, definitive: bool) -> &'static Duration {
        if definitive && self.record_negative(parent, name) {
            &NEG_TTL_RECORDED
        } else {
            &NEG_TTL
        }
    }

    /// Remember the exact spelling we denied so a create can invalidate it.
    /// Returns false when the per-parent cap stopped the record: such a miss
    /// gets the short TTL, because nothing could invalidate it.
    fn record_negative(&self, parent: u64, name: &str) -> bool {
        let now = Instant::now();
        let max_age = Duration::from_secs(NEG_TTL_RECORDED_SECS);
        let mut neg = self.negatives.lock_recover();
        let dir = neg.entry(parent).or_default();
        // A record older than the TTL it justified no longer covers anything.
        dir.retain(|_, v| {
            v.retain(|(_, at)| now.duration_since(*at) < max_age);
            !v.is_empty()
        });
        let key = name.to_ascii_lowercase();
        if let Some((_, at)) = dir
            .get_mut(&key)
            .and_then(|v| v.iter_mut().find(|(n, _)| n == name))
        {
            *at = now;
            return true;
        }
        if dir.values().map(Vec::len).sum::<usize>() >= 4096 {
            return false;
        }
        dir.entry(key).or_default().push((name.to_string(), now));
        true
    }

    /// Remember a spelling the kernel now caches for `ino`, so a later rename can
    /// drop the ones it did not move. Capped per inode: a file is normally
    /// reached through one or two spellings, and the cap bounds a pathological
    /// case rather than a real one.
    fn record_alias(&self, ino: u64, parent: u64, name: &str) {
        let mut aliases = self.aliases.lock_recover();
        // Capacity 1, not the 4 a first push picks for a 32 B element: nearly
        // every inode only ever gets one spelling, and the entry lives until
        // FORGET, which a machine with free RAM rarely sends. About 96 B per
        // looked-up inode; a second spelling grows it as before.
        let entry = aliases.entry(ino).or_insert_with(|| Vec::with_capacity(1));
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
            let Some(entry) = aliases.get_mut(&ino) else {
                return;
            };
            let (stale, keep): (Vec<_>, Vec<_>) = entry
                .drain(..)
                .partition(|(p, n)| !(*p == kept_parent && n == kept_name));
            *entry = keep;
            stale
        };
        if stale.is_empty() {
            return;
        }
        // Queued: see `notify`.
        self.notify(
            stale
                .into_iter()
                .map(|(parent, name)| Inval::Entry(parent, name.into())),
        );
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
        self.notify([Inval::Inode(ino)]);
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
        // A removal sets it too: the flag never clears, and a stale one only
        // costs Wine its own scan of this directory.
        if eidos_core::has_unicode_case(name)
            && self.unicode_case_dirs.lock_recover().insert(parent, true) == Some(false)
        {
            // The kernel keeps a `.ciopfs` answered before for the entry TTL.
            self.notify([Inval::Entry(
                parent,
                OsStr::from_bytes(ops::CIOPFS_MARKER).to_owned(),
            )]);
        }
    }

    /// Whether directory `dir` may tell Wine it folds case (`FS_IOC_GETFLAGS`
    /// and the `.ciopfs` marker): not when a name in it needs Unicode case
    /// folding, because Wine then skips its own scan, which would find it.
    fn folds_case(&self, dir: u64) -> bool {
        if let Some(&unicode) = self.unicode_case_dirs.lock_recover().get(&dir) {
            return !unicode;
        }
        let Some(vpath) = self.inodes.lock_recover().path(dir) else {
            return false;
        };
        let unicode = self.stack.dir_has_unicode_case(&vpath);
        // `or_insert`: a create racing this read may have set it already.
        !*self
            .unicode_case_dirs
            .lock_recover()
            .entry(dir)
            .or_insert(unicode)
    }

    fn invalidate_folded_negatives(&self, parent: u64, name: &str) {
        let stale: Vec<String> = {
            let mut neg = self.negatives.lock_recover();
            let Some(dir) = neg.get_mut(&parent) else {
                return;
            };
            let key = name.to_ascii_lowercase();
            let Some(names) = dir.get_mut(&key) else {
                return;
            };
            // Exact matches are handled by the kernel itself when it instantiates
            // the new dentry; only the OTHER spellings need a nudge.
            let (stale, keep): (Vec<_>, Vec<_>) = names.drain(..).partition(|(n, _)| n != name);
            if keep.is_empty() {
                dir.remove(&key);
            } else {
                *names = keep;
            }
            stale.into_iter().map(|(n, _)| n).collect()
        };
        if stale.is_empty() {
            return;
        }
        // MUST NOT run inline (deadlocks the mount, observed: the create never
        // returns); `notify` hands it to the notifier thread.
        self.notify(stale.into_iter().map(|s| Inval::Entry(parent, s.into())));
    }

    /// Build a `FileAttr` from a real file's metadata, owned by the mounting
    /// user (the game runs as us under Proton).
    fn attr(&self, ino: u64, meta: &Metadata) -> FileAttr {
        let projected = if self.plugin_timestamps.is_some() {
            self.inodes
                .lock_recover()
                .path(ino)
                .map(|p| self.projected_times(&p))
                .unwrap_or_default()
        } else {
            (None, None)
        };
        let mtime = projected
            .0
            .unwrap_or_else(|| meta.modified().unwrap_or(UNIX_EPOCH));
        let atime = projected
            .1
            .unwrap_or_else(|| meta.accessed().unwrap_or(mtime));
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
    /// Scans run outside the cache lock. Removing their slot rejects stale
    /// publication without retrying scans of unrelated directories.
    fn merged_children(&self, vpath: &str) -> Listing {
        loop {
            let scan = match self.dir_cache.lock_recover().get_or_start(vpath) {
                Ok(hit) => {
                    Stats::bump(&self.stats.dir_hit);
                    return hit;
                }
                Err(scan) => scan,
            };
            Stats::bump(&self.stats.snapshot);
            let children: Listing = Arc::new(
                self.stack
                    .list_dir_typed(vpath)
                    .into_iter()
                    .filter_map(|(name, _real, ft)| Some((name, kind_of_type(&ft?))))
                    .collect(),
            );
            if let Some(children) = self
                .dir_cache
                .lock_recover()
                .publish(vpath, &scan, children)
            {
                return children;
            }
        }
    }

    /// Drop a listing and reject scans that started before this mutation.
    fn invalidate_dir_cache(&self, vpath: &str) {
        self.dir_cache.lock_recover().invalidate(Some(vpath));
    }

    /// Drop the cached listing of `parent_ino`'s directory: the form the mutating
    /// handlers have their parent in.
    fn invalidate_dir_of(&self, parent_ino: u64) {
        let vpath = self.inodes.lock_recover().path(parent_ino);
        if let Some(v) = vpath {
            self.invalidate_dir_cache(&v);
        }
    }

    /// How big the daemon's own tables grew, next to its resident memory, for
    /// the unmount report. Two readings of the same code disagreed on whether
    /// `readdir` pins inodes the kernel never looks up; `inodes - counted` is
    /// that population, measured instead of argued. Lengths only, one lock at
    /// a time: nothing contends at unmount.
    fn table_sizes(&self) -> String {
        let (inodes, counted) = {
            let t = self.inodes.lock_recover();
            (t.by_ino.len(), t.counts.len())
        };
        let (alias_inodes, spellings) = {
            let a = self.aliases.lock_recover();
            (a.len(), a.values().map(Vec::len).sum::<usize>())
        };
        let (negative_dirs, negative_names) = {
            let n = self.negatives.lock_recover();
            (
                n.len(),
                n.values()
                    .flat_map(|d| d.values())
                    .map(Vec::len)
                    .sum::<usize>(),
            )
        };
        let (listings, children) = {
            let c = self.dir_cache.lock_recover();
            let children = c
                .entries
                .values()
                .filter_map(|e| e.children.as_ref())
                .map(|l| l.len())
                .sum::<usize>();
            (c.entries.len(), children)
        };
        let enumerations = self.enumerations.lock_recover().len();
        // Read here rather than from outside, so the sizes and the memory are
        // one snapshot of one process.
        let rss = std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find_map(|l| l.strip_prefix("VmRSS:"))
                    .map(|v| v.trim().to_string())
            })
            .unwrap_or_else(|| "unknown".into());
        format!(
            "eidos-fuse tables: {inodes} inodes ({counted} kernel-counted, {} listed only), \
             {spellings} alias spelling(s) on {alias_inodes} inode(s), {negative_names} \
             negative name(s) under {negative_dirs} dir(s), {listings} cached listing(s) \
             holding {children} entries, {enumerations} enumeration(s) in flight; VmRSS {rss}",
            // The root is in the table but never counted: it is never looked up.
            inodes.saturating_sub(counted + 1),
        )
    }
}
