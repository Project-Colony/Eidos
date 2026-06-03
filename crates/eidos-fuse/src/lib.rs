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

use std::collections::HashMap;
use std::ffi::{CString, OsStr};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eidos_core::LayerStack;
use fuser::{
    BackgroundSession, BackingId, BsdFileFlags, Config, Errno, FileAttr, FileHandle, FileType,
    Filesystem, FopenFlags, Generation, INodeNo, KernelConfig, LockOwner, MountOption, OpenFlags,
    RenameFlags, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyOpen, ReplyStatfs, ReplyWrite, Request, TimeOrNow, WriteFlags,
};

/// Attribute/entry cache lifetime handed to the kernel. Conservative for now.
const TTL: Duration = Duration::from_secs(1);
const ROOT_INO: u64 = 1;

/// Maps inode numbers to virtual paths (relative, no leading slash; "" = root)
/// and back, minting a stable inode the first time a path is seen.
struct Inodes {
    by_ino: HashMap<u64, String>,
    by_path: HashMap<String, u64>,
    next: u64,
}

impl Inodes {
    fn new() -> Self {
        let mut s = Self {
            by_ino: HashMap::new(),
            by_path: HashMap::new(),
            next: ROOT_INO + 1,
        };
        s.by_ino.insert(ROOT_INO, String::new());
        s.by_path.insert(String::new(), ROOT_INO);
        s
    }

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

    fn path(&self, ino: u64) -> Option<String> {
        self.by_ino.get(&ino).cloned()
    }

    /// Rebind the inode for `from` onto `to` after a rename, so the kernel's
    /// reuse of the source inode for the destination keeps resolving correctly.
    fn rename(&mut self, from: &str, to: &str) {
        if let Some(ino) = self.by_path.remove(from) {
            self.by_path.remove(to); // drop any stale destination mapping
            self.by_ino.insert(ino, to.to_string());
            self.by_path.insert(to.to_string(), ino);
        }
    }
}

/// A passthrough-open file: the real backing fd and its kernel registration,
/// both kept alive until `release`.
struct OpenFile {
    _file: File,
    _backing: BackingId,
}

/// The Eidos union filesystem over a [`LayerStack`].
pub struct Eidos {
    stack: LayerStack,
    inodes: Mutex<Inodes>,
    uid: u32,
    gid: u32,
    open_files: Mutex<HashMap<u64, OpenFile>>,
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
}

impl Filesystem for Eidos {
    fn init(&mut self, _req: &Request, config: &mut KernelConfig) -> std::io::Result<()> {
        // Best-effort FUSE passthrough: with a non-zero stack depth the kernel
        // can route reads/writes straight to the backing file. NOTE: registering
        // a backing fd needs CAP_SYS_ADMIN in the initial user namespace (real
        // root); our rootless, userns-mapped daemon does not have it, so
        // `open_backing` returns EPERM and we fall back to serving reads/writes
        // ourselves. It is left enabled so it engages for free if Eidos is ever
        // run privileged, or on a kernel that allows userns passthrough.
        let _ = config.set_max_stack_depth(1);

        // Rootless perf levers that always apply: large readahead and write
        // buffers cut the number of round-trips on big asset files. (Metadata is
        // already cached kernel-side via our entry/attr TTL.)
        let _ = config.set_max_readahead(1 << 20);
        let _ = config.set_max_write(1 << 20);
        Ok(())
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

        // Register the backing fd for kernel passthrough; on failure (typically
        // EPERM when rootless) fall back to a plain handle so the kernel calls
        // our own read/write.
        let backing = match reply.open_backing(file.as_fd()) {
            Ok(b) => b,
            Err(_) => {
                reply.opened(FileHandle(0), FopenFlags::empty());
                return;
            }
        };
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        let mut files = self.open_files.lock().unwrap();
        files.insert(fh, OpenFile { _file: file, _backing: backing });
        let backing_ref = &files.get(&fh).expect("just inserted")._backing;
        reply.opened_passthrough(FileHandle(fh), FopenFlags::empty(), backing_ref);
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
        // Drops the backing registration and the real fd (no-op for fh 0).
        self.open_files.lock().unwrap().remove(&fh.0);
        reply.ok();
    }

    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let mut inodes = self.inodes.lock().unwrap();
        let Some(parent_vpath) = inodes.path(parent.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let vpath = join(&parent_vpath, &name.to_string_lossy());
        let Some(real) = self.stack.resolve_read(&vpath) else {
            reply.error(Errno::ENOENT);
            return;
        };
        match fs::symlink_metadata(&real) {
            Ok(meta) => {
                let ino = inodes.intern(&vpath);
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
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
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
        let read = (|| -> std::io::Result<Vec<u8>> {
            let mut f = File::open(&real)?;
            f.seek(SeekFrom::Start(offset))?;
            let mut buf = Vec::with_capacity(size as usize);
            f.take(size as u64).read_to_end(&mut buf)?;
            Ok(buf)
        })();
        match read {
            Ok(buf) => reply.data(&buf),
            Err(_) => reply.error(Errno::EIO),
        }
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        let mut inodes = self.inodes.lock().unwrap();
        let Some(vpath) = inodes.path(ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };

        let parent_ino = if vpath.is_empty() {
            ROOT_INO
        } else {
            let parent_vpath = vpath.rsplit_once('/').map_or("", |(p, _)| p).to_string();
            inodes.intern(&parent_vpath)
        };

        let mut entries: Vec<(u64, FileType, String)> = vec![
            (ino.0, FileType::Directory, ".".to_string()),
            (parent_ino, FileType::Directory, "..".to_string()),
        ];
        for (name, real) in self.stack.list_dir(&vpath) {
            let child_ino = inodes.intern(&join(&vpath, &name));
            let kind = fs::symlink_metadata(&real).map_or(FileType::RegularFile, |m| kind_of(&m));
            entries.push((child_ino, kind, name));
        }
        drop(inodes);

        for (i, (e_ino, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(INodeNo(e_ino), (i + 1) as u64, kind, name) {
                break; // kernel buffer full
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
        let mut inodes = self.inodes.lock().unwrap();
        let Some(parent_vpath) = inodes.path(parent.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let vpath = join(&parent_vpath, &name.to_string_lossy());
        let created = self
            .stack
            .open_for_write(&vpath)
            .and_then(|dest| OpenOptions::new().create(true).write(true).open(&dest))
            .and_then(|_| {
                let real = self.stack.resolve_read(&vpath).ok_or(std::io::ErrorKind::Other)?;
                fs::symlink_metadata(real)
            });
        match created {
            Ok(meta) => {
                let ino = inodes.intern(&vpath);
                reply.created(
                    &TTL,
                    &self.attr(ino, &meta),
                    Generation(0),
                    FileHandle(0),
                    FopenFlags::empty(),
                );
            }
            Err(_) => reply.error(Errno::EIO),
        }
    }

    fn write(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        data: &[u8],
        _write_flags: WriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyWrite,
    ) {
        let vpath = match self.inodes.lock().unwrap().path(ino.0) {
            Some(p) => p,
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        let written = (|| -> std::io::Result<u32> {
            let dest = self.stack.open_for_write(&vpath)?; // copy-up if needed (idempotent)
            let mut f = OpenOptions::new().write(true).open(&dest)?;
            f.seek(SeekFrom::Start(offset))?;
            f.write_all(data)?;
            Ok(data.len() as u32)
        })();
        match written {
            Ok(n) => reply.written(n),
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
        let mut inodes = self.inodes.lock().unwrap();
        let Some(parent_vpath) = inodes.path(parent.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let vpath = join(&parent_vpath, &name.to_string_lossy());
        match self.stack.make_dir(&vpath).and_then(fs::symlink_metadata) {
            Ok(meta) => {
                let ino = inodes.intern(&vpath);
                reply.entry(&TTL, &self.attr(ino, &meta), Generation(0));
            }
            Err(_) => reply.error(Errno::EIO),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn setattr(
        &self,
        _req: &Request,
        ino: INodeNo,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
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
        if let Some(sz) = size {
            let r = (|| -> std::io::Result<()> {
                let dest = self.stack.open_for_write(&vpath)?;
                OpenOptions::new().create(true).write(true).open(&dest)?.set_len(sz)
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

    fn fsync(&self, _req: &Request, _ino: INodeNo, _fh: FileHandle, _datasync: bool, reply: ReplyEmpty) {
        reply.ok();
    }

    fn unlink(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        match self.child(parent, name) {
            Some(vpath) => match self.stack.remove(&vpath) {
                Ok(()) => reply.ok(),
                Err(_) => reply.error(Errno::EIO),
            },
            None => reply.error(Errno::ENOENT),
        }
    }

    fn rmdir(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        match self.child(parent, name) {
            Some(vpath) => match self.stack.remove(&vpath) {
                Ok(()) => reply.ok(),
                Err(_) => reply.error(Errno::EIO),
            },
            None => reply.error(Errno::ENOENT),
        }
    }

    fn rename(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        newparent: INodeNo,
        newname: &OsStr,
        _flags: RenameFlags,
        reply: ReplyEmpty,
    ) {
        let (Some(from), Some(to)) = (self.child(parent, name), self.child(newparent, newname)) else {
            reply.error(Errno::ENOENT);
            return;
        };
        match self.stack.rename(&from, &to) {
            Ok(()) => {
                self.inodes.lock().unwrap().rename(&from, &to);
                reply.ok();
            }
            Err(_) => reply.error(Errno::EIO),
        }
    }

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

/// `statvfs(2)` of a path, for reporting real free space to the game.
fn statvfs_of(path: &Path) -> std::io::Result<libc::statvfs> {
    let c = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains NUL"))?;
    // SAFETY: valid C path and a zeroed statvfs out-param; we check the return.
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(st)
}
