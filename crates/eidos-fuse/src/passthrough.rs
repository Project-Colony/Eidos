//! Kernel passthrough state per inode: the one backing id every passthrough
//! open of an inode shares, and the rule that an inode's live opens are either
//! all passthrough or all normal.
//!
//! The kernel enforces both rules (`fs/fuse/iomode.c`). A second backing id for
//! an inode that already has one fails the open with EIO ("mismatched backing").
//! A normal open of an inode that has a backing fails with EIO ("FOPEN_PASSTHROUGH
//! expected"), and so does a passthrough open while normal opens are live
//! ("conflicting caching mode"). Wine reports that EIO as
//! `STATUS_ACCESS_VIOLATION`. Registering a new id on every open caused the 152
//! failed opens recorded at `passthrough_enabled`.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use fuser::FopenFlags;

use crate::LockExt;

/// Smallest file that goes through passthrough for its size alone. Small files
/// are the INI and JSON files that games and tools rewrite, and a writer is
/// refused while passthrough opens of its file are live.
const MIN_SIZE: u64 = 1 << 20;

/// Executable images go through passthrough at any size, since the loader maps
/// them as images.
const MAPPED_EXTENSIONS: [&str; 3] = ["dll", "exe", "asi"];

/// Whether a read-only open of `real` goes through passthrough when the inode
/// has no normal opens. Never for a file in the Overwrite (`overwrite`), the
/// layer this session writes to.
pub(crate) fn passthrough_eligible(real: &Path, overwrite: &Path, file: &File) -> bool {
    if real.starts_with(overwrite) {
        return false;
    }
    let mapped = real
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| MAPPED_EXTENSIONS.iter().any(|m| e.eq_ignore_ascii_case(m)));
    mapped || file.metadata().is_ok_and(|m| m.len() >= MIN_SIZE)
}

/// The open flags the kernel accepts beside `FOPEN_PASSTHROUGH`
/// (`FOPEN_PASSTHROUGH_MASK`). Any other flag, `FOPEN_KEEP_CACHE` included,
/// fails the open with EIO ("conflicting open flags").
pub(crate) fn passthrough_flags(flags: FopenFlags) -> FopenFlags {
    flags
        & (FopenFlags::FOPEN_DIRECT_IO
            | FopenFlags::FOPEN_PARALLEL_DIRECT_WRITES
            | FopenFlags::FOPEN_NOFLUSH)
}

/// The live opens of one inode. `backing` is set exactly while `pt_refs > 0`,
/// and `pt_refs` and `normal_refs` are never both above zero.
struct PtInode<B> {
    backing: Option<Arc<B>>,
    pt_refs: u32,
    normal_refs: u32,
}

impl<B> Default for PtInode<B> {
    fn default() -> Self {
        Self {
            backing: None,
            pt_refs: 0,
            normal_refs: 0,
        }
    }
}

/// How a read-only open is served.
pub(crate) enum ReadOpen<B> {
    /// Through a backing id registered for this open.
    New(Arc<B>),
    /// Through the backing id a live open of the inode already uses.
    Reused(Arc<B>),
    /// Normally although eligible, because normal opens of the inode are live.
    ForcedNormal,
    /// Normally, because the file is not eligible.
    Normal,
    /// Normally, because the kernel refused the backing file (EPERM without
    /// `CAP_SYS_ADMIN` in the initial user namespace).
    Unregistered(std::io::Error),
}

/// Passthrough state by inode number, for the inodes with live opens.
///
/// Generic over the backing id so the tests can run without a kernel; the
/// daemon uses `fuser::BackingId`.
pub(crate) struct Passthrough<B> {
    inodes: HashMap<u64, PtInode<B>>,
}

impl<B> Default for Passthrough<B> {
    fn default() -> Self {
        Self {
            inodes: HashMap::new(),
        }
    }
}

impl<B> Passthrough<B> {
    /// Count a read-only open of `ino` and say how to serve it. `register` is
    /// called only when the inode needs a new backing id, under the caller's
    /// lock, so two opens cannot both register one.
    pub(crate) fn open_read(
        &mut self,
        ino: u64,
        eligible: bool,
        register: impl FnOnce() -> std::io::Result<B>,
    ) -> ReadOpen<B> {
        let st = self.inodes.entry(ino).or_default();
        // The kernel wants FOPEN_PASSTHROUGH on every open of an inode that has
        // a backing, eligible or not.
        if let Some(backing) = &st.backing {
            st.pt_refs += 1;
            return ReadOpen::Reused(Arc::clone(backing));
        }
        if st.normal_refs > 0 || !eligible {
            st.normal_refs += 1;
            return if eligible {
                ReadOpen::ForcedNormal
            } else {
                ReadOpen::Normal
            };
        }
        match register() {
            Ok(backing) => {
                let backing = Arc::new(backing);
                st.backing = Some(Arc::clone(&backing));
                st.pt_refs = 1;
                ReadOpen::New(backing)
            }
            Err(e) => {
                st.normal_refs += 1;
                ReadOpen::Unregistered(e)
            }
        }
    }

    /// Count a writer's open of `ino`, or return false while passthrough opens
    /// of it are live.
    fn open_write(&mut self, ino: u64) -> bool {
        let st = self.inodes.entry(ino).or_default();
        if st.pt_refs > 0 {
            return false;
        }
        st.normal_refs += 1;
        true
    }

    /// One open of `ino` closed; `passthrough` says which kind it was. The
    /// inode drops its backing id with its last passthrough open.
    pub(crate) fn release(&mut self, ino: u64, passthrough: bool) {
        let Some(st) = self.inodes.get_mut(&ino) else {
            return;
        };
        if passthrough {
            st.pt_refs = st.pt_refs.saturating_sub(1);
            if st.pt_refs == 0 {
                st.backing = None;
            }
        } else {
            st.normal_refs = st.normal_refs.saturating_sub(1);
        }
        if st.pt_refs == 0 && st.normal_refs == 0 {
            self.inodes.remove(&ino);
        }
    }

    /// The kernel forgot `ino`, so none of its opens is left.
    pub(crate) fn forget(&mut self, ino: u64) {
        self.inodes.remove(&ino);
    }
}

/// A writer's place among the normal opens of its inode, taken before the copy-up
/// or truncation of its open so that no passthrough open can start in between.
/// Dropping it gives the place back, which covers every error path; an open that
/// succeeds calls [`WriterSlot::keep`] and its release gives it back instead.
pub(crate) struct WriterSlot<'a, B> {
    table: &'a Mutex<Passthrough<B>>,
    ino: u64,
}

impl<'a, B> WriterSlot<'a, B> {
    /// Take the place, then run `writer`, the copy-up or truncation of the
    /// open, without the table locked. `None`, and `writer` never runs, while
    /// passthrough opens of `ino` are live.
    pub(crate) fn take_then<R>(
        table: &'a Mutex<Passthrough<B>>,
        ino: u64,
        writer: impl FnOnce() -> R,
    ) -> Option<(Self, R)> {
        let slot = Self::take(table, ino)?;
        let done = writer();
        Some((slot, done))
    }

    /// `None` while passthrough opens of `ino` are live.
    fn take(table: &'a Mutex<Passthrough<B>>, ino: u64) -> Option<Self> {
        let taken = table.lock_recover().open_write(ino);
        taken.then_some(WriterSlot { table, ino })
    }

    /// The open succeeded and now holds the place until its release.
    pub(crate) fn keep(self) {
        std::mem::forget(self);
    }
}

impl<B> Drop for WriterSlot<'_, B> {
    fn drop(&mut self) {
        self.table.lock_recover().release(self.ino, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn passthrough(open: ReadOpen<u32>) -> Arc<u32> {
        match open {
            ReadOpen::New(b) | ReadOpen::Reused(b) => b,
            _ => panic!("expected a passthrough open"),
        }
    }

    #[test]
    fn three_reads_of_one_inode_register_one_backing_id() {
        let mut table = Passthrough::default();
        let registered = Cell::new(0);
        let register = || {
            registered.set(registered.get() + 1);
            Ok(7)
        };
        let first = table.open_read(1, true, register);
        assert!(matches!(first, ReadOpen::New(_)));
        let first = passthrough(first);
        for _ in 0..2 {
            let again = table.open_read(1, true, register);
            assert!(matches!(again, ReadOpen::Reused(_)));
            assert!(Arc::ptr_eq(&first, &passthrough(again)));
        }
        assert_eq!(registered.get(), 1);
    }

    #[test]
    fn a_writer_is_refused_while_passthrough_opens_are_live() {
        let table = Mutex::new(Passthrough::default());
        let _reader = passthrough(table.lock_recover().open_read(1, true, || Ok(7)));
        let refused =
            WriterSlot::take_then(&table, 1, || panic!("copied up beside a passthrough open"));
        assert!(refused.is_none());
        // The refusal took no place, so the reader's release empties the inode.
        table.lock_recover().release(1, true);
        assert!(table.lock_recover().inodes.is_empty());
    }

    #[test]
    fn a_writer_copies_up_only_once_it_counts_as_a_normal_open() {
        let table = Mutex::new(Passthrough::<u32>::default());
        let (slot, refs) = WriterSlot::take_then(&table, 1, || {
            // try_lock: the copy-up of a large file never holds the table lock.
            table.try_lock().unwrap().inodes[&1].normal_refs
        })
        .unwrap();
        assert_eq!(refs, 1);
        slot.keep();
        assert_eq!(table.lock_recover().inodes[&1].normal_refs, 1);
    }

    #[test]
    fn a_writer_opens_once_every_passthrough_open_is_released() {
        let table = Mutex::new(Passthrough::default());
        for _ in 0..2 {
            passthrough(table.lock_recover().open_read(1, true, || Ok(7)));
        }
        table.lock_recover().release(1, true);
        let refused = WriterSlot::take_then(&table, 1, || panic!("copied up beside a reader"));
        assert!(refused.is_none());
        table.lock_recover().release(1, true);
        WriterSlot::take(&table, 1).unwrap().keep();
        assert_eq!(table.lock_recover().inodes[&1].normal_refs, 1);
    }

    #[test]
    fn a_reader_goes_normal_while_a_normal_open_is_live() {
        let table = Mutex::new(Passthrough::<u32>::default());
        WriterSlot::take(&table, 1).unwrap().keep();
        let open = table
            .lock_recover()
            .open_read(1, true, || panic!("registered beside a normal open"));
        assert!(matches!(open, ReadOpen::ForcedNormal));
        assert_eq!(table.lock_recover().inodes[&1].normal_refs, 2);
    }

    #[test]
    fn a_failed_writer_open_gives_its_place_back() {
        let table = Mutex::new(Passthrough::default());
        drop(WriterSlot::take(&table, 1).unwrap());
        let open = table.lock_recover().open_read(1, true, || Ok(7));
        assert!(matches!(open, ReadOpen::New(_)));
    }

    #[test]
    fn an_ineligible_read_shares_a_live_backing_id() {
        let mut table = Passthrough::default();
        let first = passthrough(table.open_read(1, true, || Ok(7)));
        let other = table.open_read(1, false, || panic!("registered twice"));
        assert!(Arc::ptr_eq(&first, &passthrough(other)));
        assert!(matches!(
            table.open_read(2, false, || panic!("registered an ineligible file")),
            ReadOpen::Normal
        ));
    }

    #[test]
    fn the_last_passthrough_release_drops_the_backing_id() {
        let mut table = Passthrough::default();
        let held = passthrough(table.open_read(1, true, || Ok(7)));
        passthrough(table.open_read(1, true, || Ok(8)));
        table.release(1, true);
        assert_eq!(Arc::strong_count(&held), 2);
        table.release(1, true);
        assert_eq!(Arc::strong_count(&held), 1);
        assert!(table.inodes.is_empty());
        // The next reader registers a fresh id.
        let open = table.open_read(1, true, || Ok(9));
        assert_eq!(*passthrough(open), 9);
    }

    #[test]
    fn a_refused_registration_counts_as_a_normal_open() {
        let table = Mutex::new(Passthrough::<u32>::default());
        let open = table.lock_recover().open_read(1, true, || {
            Err(std::io::Error::from_raw_os_error(libc::EPERM))
        });
        assert!(matches!(open, ReadOpen::Unregistered(_)));
        WriterSlot::take(&table, 1).unwrap().keep();
        assert_eq!(table.lock_recover().inodes[&1].normal_refs, 2);
    }

    #[test]
    fn passthrough_replies_never_carry_keep_cache() {
        let normal = FopenFlags::FOPEN_KEEP_CACHE | FopenFlags::FOPEN_NOFLUSH;
        assert_eq!(passthrough_flags(normal), FopenFlags::FOPEN_NOFLUSH);
    }

    #[test]
    fn only_large_files_and_images_outside_the_overwrite_are_eligible() {
        let root = std::env::temp_dir().join(format!("eidos-pt-eligible-{}", std::process::id()));
        let (layer, overwrite) = (root.join("mod"), root.join("overwrite"));
        std::fs::create_dir_all(&layer).unwrap();
        std::fs::create_dir_all(&overwrite).unwrap();
        let file = |dir: &Path, name: &str, len: u64| {
            let path = dir.join(name);
            File::create(&path).unwrap().set_len(len).unwrap();
            let opened = File::open(&path).unwrap();
            passthrough_eligible(&path, &overwrite, &opened)
        };
        assert!(file(&layer, "big.bsa", MIN_SIZE));
        assert!(file(&layer, "plugin.DLL", 10));
        assert!(!file(&layer, "settings.ini", 10));
        assert!(!file(&overwrite, "big.bsa", MIN_SIZE));
        std::fs::remove_dir_all(root).unwrap();
    }
}
