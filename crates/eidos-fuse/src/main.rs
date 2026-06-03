//! Eidos FUSE union daemon.
//!
//! A read-only first cut: it mounts a merged view of mod layers over game data
//! by delegating every path decision to the unit-tested `eidos-core` resolver.
//! Reads merge with mod priority and case-insensitive lookup; copy-on-write
//! writes through the Overwrite layer are the next milestone.
//!
//! Usage:
//!   eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>
//!
//! The first `--layer` has the highest priority; the last is typically the
//! pristine game data directory. Unmount with `fusermount3 -u <mountpoint>`.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::exit;
use std::sync::Mutex;
use std::time::{Duration, UNIX_EPOCH};

use eidos_core::LayerStack;
use fuser::{
    Config, Errno, FileAttr, FileHandle, FileType, Filesystem, Generation, INodeNo, LockOwner,
    MountOption, OpenFlags, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
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
}

struct Eidos {
    stack: LayerStack,
    inodes: Mutex<Inodes>,
    uid: u32,
    gid: u32,
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

impl Eidos {
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

        // "." and ".." first, then the merged directory contents.
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
            // The offset handed back is the index of the *next* entry to emit.
            if reply.add(INodeNo(e_ino), (i + 1) as u64, kind, name) {
                break; // kernel buffer full
            }
        }
        reply.ok();
    }
}

fn usage() -> ! {
    eprintln!(
        "usage: eidos-fuse --layer <dir> [--layer <dir> ...] [--overwrite <dir>] <mountpoint>\n\
         \n\
         The first --layer has the highest priority; the last is typically the\n\
         pristine game data directory. Unmount with: fusermount3 -u <mountpoint>"
    );
    exit(2);
}

fn main() -> std::io::Result<()> {
    let mut layers: Vec<PathBuf> = Vec::new();
    let mut overwrite: Option<PathBuf> = None;
    let mut mountpoint: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--layer" => layers.push(PathBuf::from(args.next().unwrap_or_else(|| usage()))),
            "--overwrite" => overwrite = Some(PathBuf::from(args.next().unwrap_or_else(|| usage()))),
            "-h" | "--help" => usage(),
            _ => mountpoint = Some(PathBuf::from(arg)),
        }
    }

    let (Some(mountpoint), false) = (mountpoint, layers.is_empty()) else {
        usage();
    };

    // A read-only mount still needs an overwrite root for the resolver to probe;
    // default to a fresh empty directory so it simply shadows nothing.
    let overwrite = overwrite.unwrap_or_else(|| {
        let p = std::env::temp_dir().join(format!("eidos-overwrite-{}", std::process::id()));
        let _ = fs::create_dir_all(&p);
        p
    });

    // SAFETY: getuid/getgid are always-succeeding syscalls with no preconditions.
    let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };

    let fs = Eidos {
        stack: LayerStack::new(layers, overwrite),
        inodes: Mutex::new(Inodes::new()),
        uid,
        gid,
    };

    // Config is #[non_exhaustive]; build via Default and set the public field.
    let mut config = Config::default();
    config.mount_options = vec![MountOption::RO, MountOption::FSName("eidos".to_string())];

    eprintln!("eidos-fuse: mounting at {}", mountpoint.display());
    fuser::mount2(fs, &mountpoint, &config)
}
