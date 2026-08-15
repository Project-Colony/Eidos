//! The request dispatcher (`ops`, because `fs` would shadow `std::fs`):
//! `impl Filesystem for Eidos`, one handler per FUSE
//! opcode, each delegating path decisions to `eidos-core` through the state in
//! `lib.rs`.

use std::ffi::OsStr;
use std::fs::{self, File, Metadata, OpenOptions, Permissions};
use std::os::fd::AsFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::SystemTime;

use fuser::{
    BsdFileFlags, Errno, FileHandle,
    Filesystem, FopenFlags, Generation, INodeNo, InitFlags, KernelConfig, LockOwner, OpenFlags, RenameFlags, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
    WriteFlags,
};

use crate::*;

/// The marker CIOPFS leaves in every directory it serves, and the only way Wine
/// can tell that a filesystem folds case (`get_dir_case_sensitivity` in
/// `dlls/ntdll/unix/file.c` stats it). Eidos answers it in `lookup` so Wine skips
/// its brute-force directory rescan on every mis-cased path - see the comment
/// there for what that rescan costs.
pub(crate) const CIOPFS_MARKER: &[u8] = b".ciopfs";

impl Filesystem for Eidos {
    fn destroy(&mut self) {
        // Unmount is the natural place to report: the run is over and the numbers
        // are final. Silent unless EIDOS_FUSE_STATS is set.
        if Stats::enabled() {
            // Bound separately so the closure borrows the inode table while
            // `report` borrows the counters - two disjoint fields of `self`.
            let (stats, inodes) = (&self.stats, &self.inodes);
            let path_of = |ino: u64| inodes.lock_recover().path(ino);
            eprintln!("{}", stats.report(self.stack.resolve_stats(), &path_of));
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
        let freed = self.inodes.lock_recover().forget(ino.0, nlookup);
        // And the tables keyed by that inode go with it. FORGET means the kernel
        // has dropped every reference, so there is no dentry left for an alias or
        // a negative entry to invalidate - they were provably dead and were kept
        // anyway, for the life of a mount, growing with every distinct path the
        // game ever touched rather than with the working set. Two uncontended
        // locks on an op that is rare and off the latency path.
        if freed {
            self.aliases.lock_recover().remove(&ino.0);
            self.negatives.lock_recover().remove(&ino.0);
        }
    }

    fn open(&self, _req: &Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        Stats::bump(&self.stats.open);
        let _t = Timed::start(&self.stats.ns_open);
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

        // O_TRUNC makes the name appear, so it goes through the stack like every
        // other creation; the remaining opens here only ever touch a name that
        // already exists.
        if truncating {
            match self.stack.create_truncated(&vpath) {
                Ok(_) => {}
                Err(e) => {
                    reply.error(e.into());
                    return;
                }
            }
        }
        let mut opts = OpenOptions::new();
        if want_write {
            opts.read(true).write(true);
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
        let _t = Timed::start(&self.stats.ns_lookup);
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
        let _t = Timed::start(&self.stats.ns_getattr);
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
        req: &Request,
        ino: INodeNo,
        fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        Stats::bump(&self.stats.read);
        // Times the read and, when stats are on, records its shape: the request
        // size, which file it hit, which thread asked, and where it lands on the
        // session timeline. `req.pid()` is the CALLER's thread id, which is what
        // says whether a game is blocking one thread on I/O or streaming across
        // a pool. Off, this is one atomic load.
        let _t = TimedRead::start(&self.stats, ino.0, size, req.pid());
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
        let _t = Timed::start(&self.stats.ns_readdir);
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
            let (dest, file) = self.stack.create_truncated(&vpath)?;
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
        let _t = Timed::start(&self.stats.ns_write);
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
            let dest = self.stack.create_symlink(&vpath, target)?;
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
                    self.stack.set_len(&vpath, sz)?;
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
