//! The path<->inode table and per-handle state: reference counts against the
//! kernel's lookup count, the attribute cache, directory snapshots, open files.

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

use fuser::{BackingId, FileType};

pub(crate) const ROOT_INO: u64 = 1;

/// One kernel-side cache, individually switchable.
///
/// `EIDOS_FUSE_NO_CACHE=1` turns all four off together, which answers "is it the
/// caching?" but not "which one". `EIDOS_FUSE_NO_CACHE=attr,keep` names them, so
/// a stale-data bug can be bisected in four runs instead of by rebuilding with
/// lines commented out. The names are the kernel's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cache {
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
    pub(crate) fn name(self) -> &'static str {
        match self {
            Cache::Attr => "attr",
            Cache::Neg => "neg",
            Cache::Keep => "keep",
            Cache::Dir => "dir",
        }
    }

    /// Whether this particular cache is switched off.
    pub(crate) fn is_off(self) -> bool {
        let Ok(v) = std::env::var("EIDOS_FUSE_NO_CACHE") else {
            return false;
        };
        let v = v.trim();
        if v.is_empty() || v == "0" {
            return false;
        }
        // A bare truthy value means all of them; a list names the ones to drop.
        if !v.contains(',') && !["attr", "neg", "keep", "dir"].contains(&v) {
            return true;
        }
        v.split(',')
            .any(|part| part.trim().eq_ignore_ascii_case(self.name()))
    }
}

/// A directory entry in an `opendir` snapshot: `(inode, kind, name)`.
pub(crate) type DirEntry = (u64, FileType, String);

/// A merged directory listing without inodes: the layers collapsed and NTFS-collated.
/// Shared, because it is cached by path and handed to every enumeration of it.
pub(crate) type Listing = Arc<Vec<(String, FileType)>>;

/// One open directory handle. `entries` is filled by the first `readdir`.
pub(crate) struct OpenDir {
    pub(crate) ino: u64,
    pub(crate) vpath: String,
    pub(crate) entries: Option<Vec<DirEntry>>,
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
pub(crate) struct Inodes {
    pub(crate) by_ino: HashMap<u64, String>,
    pub(crate) by_path: HashMap<String, u64>,
    pub(crate) counts: HashMap<u64, u64>,
    pub(crate) next: u64,
}

impl Inodes {
    pub(crate) fn new() -> Self {
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
    pub(crate) fn intern(&mut self, vpath: &str) -> u64 {
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
    pub(crate) fn lookup(&mut self, vpath: &str) -> u64 {
        let ino = self.intern(vpath);
        *self.counts.entry(ino).or_insert(0) += 1;
        ino
    }

    pub(crate) fn path(&self, ino: u64) -> Option<String> {
        self.by_ino.get(&ino).cloned()
    }

    /// Drop `nlookup` kernel references from `ino`; free the mapping at zero.
    /// The root is pinned for the mount's lifetime and never freed.
    /// Returns whether the inode was actually FREED (its count reached zero), so
    /// the caller can drop the side tables keyed by it. Those entries are
    /// provably dead once the kernel forgets an inode - it holds no dentry an
    /// alias or negative invalidation could target - and they used to be kept
    /// for the life of the mount, growing with every distinct path ever touched
    /// in a daemon whose death takes the game with it.
    pub(crate) fn forget(&mut self, ino: u64, nlookup: u64) -> bool {
        if ino == ROOT_INO {
            return false;
        }
        let mut freed = false;
        if let Some(count) = self.counts.get_mut(&ino) {
            *count = count.saturating_sub(nlookup);
            if *count == 0 {
                freed = true;
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
        freed
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
    /// Returns the inode that was moved (if the kernel had one for `from`) plus
    /// every inode the rename CLOBBERED - the destination of an atomic replace.
    ///
    /// The clobbered list exists for the caller's side tables: `discard` removes
    /// a clobbered inode from `counts`, so the later FORGET for it finds no
    /// count, reports nothing freed, and the `aliases`/`negatives` entries keyed
    /// by it were retained for the life of the mount. Renaming over an existing
    /// file is the atomic-replace pattern every INI and save write uses, so that
    /// was the leak that grew fastest.
    pub(crate) fn rename(&mut self, from: &str, to: &str) -> (Option<u64>, Vec<u64>) {
        let (from_key, to_key) = (ikey(from), ikey(to));
        let mut clobbered_out: Vec<u64> = Vec::new();

        // Drop an inode that the rename has made unreachable, so a later `forget`
        // on it cannot unmap whoever owns its path now.
        fn discard(this: &mut Inodes, ino: u64, survivor: u64, out: &mut Vec<u64>) {
            if ino != survivor {
                this.by_ino.remove(&ino);
                this.counts.remove(&ino);
                out.push(ino);
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
                discard(self, clobbered, ino, &mut clobbered_out);
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
                discard(self, clobbered, child_ino, &mut clobbered_out);
            }
            // Keep the display path in step, preserving the child's own casing.
            if let Some(old_display) = self.by_ino.get(&child_ino).cloned() {
                if old_display.len() >= from.len() {
                    self.by_ino
                        .insert(child_ino, format!("{to}{}", &old_display[from.len()..]));
                }
            }
        }
        (moved_ino, clobbered_out)
    }
}

/// The case-folded key under which a virtual path is interned. Eidos resolves
/// paths case-insensitively (Windows games mix casing freely between the plugin
/// header, the loose-file indexer and BSA lookups), so one real file must map to
/// exactly one inode however the caller spelled it. ASCII-only, matching the
/// documented fold in `eidos-core`.
pub(crate) fn ikey(vpath: &str) -> String {
    vpath.to_ascii_lowercase()
}

/// An open file: its resolved backing path plus the real fd, kept open so
/// reads/writes use `pread`/`pwrite` without re-resolving. `backing` holds the
/// kernel passthrough registration when it could be set up (privileged only).
pub(crate) struct OpenFile {
    pub(crate) _real: PathBuf,
    pub(crate) file: Arc<File>,
    pub(crate) _backing: Option<BackingId>,
}
