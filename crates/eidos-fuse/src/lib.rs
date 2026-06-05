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
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eidos_core::LayerStack;
use fuser::{
    BackgroundSession, BackingId, BsdFileFlags, Config, Errno, FileAttr, FileHandle, FileType,
    Filesystem, FopenFlags, Generation, INodeNo, KernelConfig, LockOwner,
    MountOption, OpenFlags, RenameFlags, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
    WriteFlags,
};

/// Attribute/entry cache lifetime handed to the kernel. Conservative for now.
const TTL: Duration = Duration::from_secs(1);
const ROOT_INO: u64 = 1;

/// A directory entry in an `opendir` snapshot: `(inode, kind, name)`.
type DirEntry = (u64, FileType, String);

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
    fn intern(&mut self, vpath: &str) -> u64 {
        if let Some(&ino) = self.by_path.get(vpath) {
            return ino;
        }
        let ino = self.next;
        self.next += 1;
        self.by_ino.insert(ino, vpath.to_string());
        self.by_path.insert(vpath.to_string(), ino);
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
                    self.by_path.remove(&path);
                }
            }
        }
    }

    /// Rebind the inode for `from` onto `to` after a rename, so the kernel's
    /// reuse of the source inode for the destination keeps resolving correctly.
    /// The inode number (and its lookup count) is preserved.
    fn rename(&mut self, from: &str, to: &str) {
        if let Some(ino) = self.by_path.remove(from) {
            self.by_path.remove(to); // drop any stale destination mapping
            self.by_ino.insert(ino, to.to_string());
            self.by_path.insert(to.to_string(), ino);
        }
    }
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
        let inodes = self.inodes.lock().unwrap();
        let parent_vpath = inodes.path(parent.0)?;
        Some(join(&parent_vpath, &name.to_string_lossy()))
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
            self.inodes.lock().unwrap().intern(&parent_vpath)
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
        let mut inodes = self.inodes.lock().unwrap();
        for (name, kind) in children {
            let child_ino = inodes.intern(&join(vpath, &name));
            entries.push((child_ino, kind, name));
        }
        entries
    }
}

impl Filesystem for Eidos {
    fn init(&mut self, _req: &Request, config: &mut KernelConfig) -> std::io::Result<()> {
        // Best-effort FUSE passthrough: with a non-zero stack depth the kernel
        // can route reads/writes straight to the backing file. NOTE: registering
        // a backing fd needs CAP_SYS_ADMIN in the initial user namespace (real
        // root); our rootless, userns-mapped daemon does not have it, so
        // `open_backing` returns EPERM and we fall back to serving reads/writes
        // ourselves. Left enabled so it engages for free if Eidos is ever run
        // privileged.
        let _ = config.set_max_stack_depth(1);

        // NOTE: FUSE_WRITEBACK_CACHE is deliberately NOT enabled. It would make
        // writable shared mmap correct and coalesce writes, but it hands the
        // kernel ownership of size/mtime, which then flushes attribute changes
        // (a setattr) on a just-unlinked inode. For a copy-up union that turns
        // any write op into a copy-up, that setattr resurrects a deleted file.
        // Read-only mmap (what games use for BSA/BA2) already works via the page
        // cache without it; writable-mmap correctness is left for a passthrough
        // or active-invalidation design (see docs/architecture.md open risks).

        // Rootless perf levers that always apply: large readahead and write
        // buffers cut the number of round-trips on big asset files. (Metadata is
        // already cached kernel-side via our entry/attr TTL.)
        let _ = config.set_max_readahead(1 << 20);
        let _ = config.set_max_write(1 << 20);
        Ok(())
    }

    fn forget(&self, _req: &Request, ino: INodeNo, nlookup: u64) {
        // The default batch_forget fans out to this, so both paths free inodes.
        self.inodes.lock().unwrap().forget(ino.0, nlookup);
    }

    fn open(&self, _req: &Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        let vpath = match self.inodes.lock().unwrap().path(ino.0) {
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
        let real = if want_write {
            match self.stack.open_for_write(&vpath) {
                Ok(p) => p,
                Err(_) => {
                    reply.error(Errno::EIO);
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
        } else {
            opts.read(true);
        }
        let file = match opts.open(&real) {
            Ok(f) => f,
            Err(_) => {
                reply.error(Errno::EIO);
                return;
            }
        };
        if want_write && flags.0 & libc::O_TRUNC != 0 {
            let _ = file.set_len(0);
        }

        // Cache the open fd under a fresh handle; try to register it for kernel
        // passthrough (no-op fallback when rootless, where it returns EPERM).
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        let backing = reply.open_backing(file.as_fd()).ok();
        let mut files = self.open_files.lock().unwrap();
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
        self.open_files.lock().unwrap().remove(&fh.0);
        reply.ok();
    }

    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let Some(parent_vpath) = self.inodes.lock().unwrap().path(parent.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let vpath = join(&parent_vpath, &name.to_string_lossy());
        // Resolve + stat without the inode lock held.
        let Some(real) = self.stack.resolve_read(&vpath) else {
            reply.error(Errno::ENOENT);
            return;
        };
        match fs::symlink_metadata(&real) {
            Ok(meta) => {
                let ino = self.inodes.lock().unwrap().lookup(&vpath);
                reply.entry(&TTL, &self.attr(ino, &meta), Generation(0));
            }
            Err(_) => reply.error(Errno::ENOENT),
        }
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        let vpath = match self.inodes.lock().unwrap().path(ino.0) {
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
        // Fast path: pread the cached fd from `open` (no re-resolve, no re-open,
        // offset-explicit so concurrent reads on one handle do not race).
        let cached = self.open_files.lock().unwrap().get(&fh.0).map(|o| o.file.clone());
        if let Some(file) = cached {
            match read_full_at(&file, offset, size as usize) {
                Ok(buf) => reply.data(&buf),
                Err(_) => reply.error(Errno::EIO),
            }
            return;
        }

        // Fallback (e.g. fh 0): resolve by inode and read once.
        let vpath = match self.inodes.lock().unwrap().path(ino.0) {
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
            Err(_) => reply.error(Errno::EIO),
        }
    }

    fn opendir(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        // Snapshot the listing now so offsets stay valid even if the directory
        // changes before releasedir (the conformant readdir pattern).
        let Some(vpath) = self.inodes.lock().unwrap().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let entries = self.dir_snapshot(ino.0, &vpath);
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        self.open_dirs.lock().unwrap().insert(fh, entries);
        reply.opened(FileHandle(fh), FopenFlags::empty());
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
            let dirs = self.open_dirs.lock().unwrap();
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
        let Some(vpath) = self.inodes.lock().unwrap().path(ino.0) else {
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
        self.open_dirs.lock().unwrap().remove(&fh.0);
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
        let Some(parent_vpath) = self.inodes.lock().unwrap().path(parent.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let vpath = join(&parent_vpath, &name.to_string_lossy());

        // Materialise the file (and any missing parents) in the Overwrite layer.
        let opened = (|| -> std::io::Result<(PathBuf, File, Metadata)> {
            let dest = self.stack.open_for_write(&vpath)?;
            let file =
                OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&dest)?;
            let meta = fs::symlink_metadata(&dest)?;
            Ok((dest, file, meta))
        })();
        let (real, file, meta) = match opened {
            Ok(t) => t,
            Err(_) => {
                reply.error(Errno::EIO);
                return;
            }
        };

        let ino = self.inodes.lock().unwrap().lookup(&vpath);
        let attr = self.attr(ino, &meta);
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        let backing = reply.open_backing(file.as_fd()).ok();
        let mut files = self.open_files.lock().unwrap();
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
        // Fast path: pwrite the cached (copied-up) fd from `open`/`create`.
        let cached = self.open_files.lock().unwrap().get(&fh.0).map(|o| o.file.clone());
        if let Some(file) = cached {
            match write_all_at(&file, data, offset) {
                Ok(()) => reply.written(data.len() as u32),
                Err(_) => reply.error(Errno::EIO),
            }
            return;
        }

        // Fallback (e.g. fh 0): copy up and write once.
        let vpath = match self.inodes.lock().unwrap().path(ino.0) {
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
            Err(_) => reply.error(Errno::EIO),
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
        let Some(parent_vpath) = self.inodes.lock().unwrap().path(parent.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let vpath = join(&parent_vpath, &name.to_string_lossy());
        let meta = match self.stack.make_dir(&vpath).and_then(fs::symlink_metadata) {
            Ok(m) => m,
            Err(_) => {
                reply.error(Errno::EIO);
                return;
            }
        };
        let ino = self.inodes.lock().unwrap().lookup(&vpath);
        reply.entry(&TTL, &self.attr(ino, &meta), Generation(0));
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
        let vpath = match self.inodes.lock().unwrap().path(ino.0) {
            Some(p) => p,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        // Any change must land in the Overwrite layer, so copy up first if the
        // path still lives only in a lower layer. We apply truncate, mode, and
        // timestamps; ownership is intentionally ignored (the game runs as us).
        if mode.is_some() || size.is_some() || atime.is_some() || mtime.is_some() {
            let r = (|| -> std::io::Result<()> {
                let dest = self.stack.open_for_write(&vpath)?;
                let f = OpenOptions::new().create(true).write(true).truncate(false).open(&dest)?;
                if let Some(sz) = size {
                    f.set_len(sz)?;
                }
                if let Some(m) = mode {
                    fs::set_permissions(&dest, Permissions::from_mode(m & 0o7777))?;
                }
                if atime.is_some() || mtime.is_some() {
                    set_times(&dest, atime, mtime)?;
                }
                Ok(())
            })();
            if r.is_err() {
                reply.error(Errno::EIO);
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
        let cached = self.open_files.lock().unwrap().get(&fh.0).map(|o| o.file.clone());
        match cached {
            Some(file) => match file.sync_all() {
                Ok(()) => reply.ok(),
                Err(_) => reply.error(Errno::EIO),
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
            Err(_) => reply.error(Errno::EIO),
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
            Err(_) => reply.error(Errno::EIO),
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
                self.inodes.lock().unwrap().rename(&from, &to);
                reply.ok();
            }
            Err(_) => reply.error(Errno::EIO),
        }
    }

    fn getxattr(&self, _req: &Request, ino: INodeNo, name: &OsStr, size: u32, reply: ReplyXattr) {
        let Some(vpath) = self.inodes.lock().unwrap().path(ino.0) else {
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
        let Some(vpath) = self.inodes.lock().unwrap().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
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
        let Some(vpath) = self.inodes.lock().unwrap().path(ino.0) else {
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
        let Some(vpath) = self.inodes.lock().unwrap().path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
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
            Err(_) => reply.error(Errno::EIO),
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
