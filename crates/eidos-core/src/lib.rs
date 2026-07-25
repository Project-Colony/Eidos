//! Eidos core: the layer-resolution engine.
//!
//! This is the pure, filesystem-facing heart of the Eidos VFS, kept separate
//! from any FUSE binding so its behaviour can be pinned by unit tests. Given an
//! ordered stack of mod layers plus a writable "overwrite" layer, it answers the
//! two questions the FUSE daemon asks on every operation:
//!
//!   * read  -> which real file on disk serves this virtual path?
//!   * write -> where in the overwrite layer should this virtual path land?
//!
//! Path matching is case-insensitive, reproducing the Windows semantics that
//! game engines and mods rely on (see `docs/architecture.md`).

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// Prefix marking a "whiteout" in the overwrite layer: an empty file
/// `<dir>/.eidoswh.<name>` means `<name>` is deleted and any lower-layer copy
/// must stay hidden. This is how a union filesystem records deletions without
/// touching the read-only mod and game layers.
const WHITEOUT_PREFIX: &str = ".eidoswh.";

/// Marker file dropped INSIDE an overwrite directory that was deleted and then
/// re-created: it makes the directory opaque (its lower-layer contents stay
/// hidden), matching NTFS where a recreated directory is empty. Distinct from the
/// per-file `.eidoswh.<name>` whiteout so it is never mistaken for one.
const OPAQUE_MARKER: &str = ".eidoswh_opaque";

/// Suffix marking a file or directory the user hid from the virtual view, MO2's
/// convention (`filetree.cpp` renames through a `FileRenamer` with HIDE/UNHIDE).
/// Hiding is a rename inside the mod, never a delete, so it is undone by
/// stripping the suffix back off.
pub const HIDDEN_SUFFIX: &str = ".mohidden";

/// Whether a single path component names something the user hid. Case-insensitive:
/// the rest of the stack folds case, and a mod folder copied off a Windows box can
/// come back as `.MOHIDDEN`.
/// Compared as BYTES, not as a string slice. `&name[name.len() - 9..]` panics
/// whenever that offset lands inside a multi-byte character - which a Chinese or
/// accented mod file name does reach - and this runs on every single path
/// resolution, so the panic would take the mount down mid-read. Byte comparison
/// is also exactly right rather than merely safe: the suffix is pure ASCII, and a
/// UTF-8 continuation byte can never equal an ASCII one, so there is nothing to
/// mis-match.
pub fn is_hidden_name(name: &str) -> bool {
    let (name, suffix) = (name.as_bytes(), HIDDEN_SUFFIX.as_bytes());
    name.len() > suffix.len() && name[name.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// Whether any component of a virtual path was hidden - a hidden DIRECTORY takes
/// its whole subtree with it, which is how MO2 lets one click suppress a mod's
/// entire `meshes/` without touching the files.
fn under_hidden(vpath: &str) -> bool {
    vpath.split(['/', '\\']).any(is_hidden_name)
}

/// An ordered union of layers with a writable top layer.
///
/// `layers[0]` has the highest priority (the last-enabled mod wins on conflict);
/// the final entry is typically the pristine game data directory. `overwrite`
/// is the writable layer where the running game's new and modified files are
/// captured, leaving every lower layer untouched.
#[derive(Debug, Clone)]
pub struct LayerStack {
    layers: Vec<PathBuf>,
    overwrite: PathBuf,
    /// Per-path mutation locks, sharded. The FUSE daemon answers requests from
    /// several threads at once, so two of them can reach the same not-yet-copied
    /// -up file together; without this both would see it absent and both run the
    /// copy, interleaving their writes into one destination. A fixed shard array
    /// keeps this to one small allocation instead of a growing per-path map.
    /// `Arc`, so cloning a stack SHARES the locks: two clones describing the
    /// same overwrite layer must serialise against each other, not each hold
    /// their own set.
    path_locks: std::sync::Arc<[std::sync::Mutex<()>]>,
}

/// Number of mutation-lock shards. Small: contention only matters when two
/// threads touch the SAME path, which is rare, and false sharing between
/// unrelated paths costs only a brief wait.
const PATH_LOCK_SHARDS: usize = 64;

impl LayerStack {
    /// Build a stack from mod layers (highest priority first) and the writable
    /// overwrite layer.
    pub fn new(layers: Vec<PathBuf>, overwrite: PathBuf) -> Self {
        let path_locks: std::sync::Arc<[std::sync::Mutex<()>]> =
            (0..PATH_LOCK_SHARDS).map(|_| std::sync::Mutex::new(())).collect();
        Self { layers, overwrite, path_locks }
    }

    /// Take the mutation lock for `vpath`. Folded, so the two spellings of one
    /// file share a shard and cannot race each other.
    fn path_lock(&self, vpath: &str) -> std::sync::MutexGuard<'_, ()> {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        vpath.to_ascii_lowercase().hash(&mut h);
        let shard = (h.finish() as usize) % PATH_LOCK_SHARDS;
        // Recover from a poisoned lock: it guards no data, only ordering, so a
        // panicking holder must not take the whole mount down.
        self.path_locks[shard].lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The overwrite-layer path where a whiteout marker for `vpath` is written.
    /// The stored casing is whatever the caller used; lookups fold case via
    /// [`Self::find_whiteout`], so it does not matter.
    fn whiteout_path(&self, vpath: &str) -> PathBuf {
        let norm = normalize(vpath);
        let name = norm.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let parent = norm.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        // Resolve the parent case-insensitively so the marker lands in the same
        // real directory the file resolves to (not a second case-variant dir).
        self.resolve_write(&parent).join(format!("{WHITEOUT_PREFIX}{name}"))
    }

    /// Find an existing whiteout marker for `vpath`, matching the final path
    /// component case-insensitively so the check agrees with [`Self::list_dir`]
    /// (which also folds case). A case-sensitive check here was a real bug:
    /// deleting `FOO` then probing `foo` missed the marker and resurfaced the
    /// lower-layer file.
    fn find_whiteout(&self, vpath: &str) -> Option<PathBuf> {
        let norm = normalize(vpath);
        let name = norm.file_name()?.to_string_lossy().to_ascii_lowercase();
        let parent = norm.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let parent_dir = ci_lookup(&self.overwrite, &parent)?;
        let want = format!("{WHITEOUT_PREFIX}{name}");
        fs::read_dir(&parent_dir)
            .ok()?
            .flatten()
            .find_map(|e| (e.file_name().to_string_lossy().to_ascii_lowercase() == want).then(|| e.path()))
    }

    /// Drop any whiteout marker for `vpath` (case-insensitively): writing or
    /// re-creating a path un-deletes it.
    fn clear_whiteout(&self, vpath: &str) {
        if let Some(wh) = self.find_whiteout(vpath) {
            let _ = fs::remove_file(wh);
        }
    }

    /// Whether `vpath` is hidden by a whiteout on itself or any ancestor
    /// directory: deleting a directory hides everything beneath it (an opaque
    /// directory). `resolve_read` checks the overwrite layer first, so a path
    /// the game re-created still resolves; only un-recreated lower paths stay
    /// hidden.
    fn hidden_by_whiteout(&self, vpath: &str) -> bool {
        let norm = normalize(vpath);
        let mut prefix = PathBuf::new();
        for comp in norm.components() {
            prefix.push(comp);
            if self.find_whiteout(&prefix.to_string_lossy()).is_some() {
                return true;
            }
        }
        false
    }

    /// Resolve a virtual path to the real file that should serve reads.
    ///
    /// The overwrite layer shadows everything (a file the game wrote or changed
    /// is what it should read back); a whiteout marker hides any lower copy;
    /// otherwise each mod layer is tried in priority order, falling through to
    /// the game data layer last. Returns `None` if nothing provides the path.
    pub fn resolve_read(&self, vpath: &str) -> Option<PathBuf> {
        // A hidden file (or anything under a hidden directory) is not part of the
        // virtual view at all: `list_dir` never emits it, and refusing to resolve
        // it here means a path guessed or remembered by the game cannot reach it
        // either. The mod keeps the bytes; the game simply cannot see them.
        if under_hidden(vpath) {
            return None;
        }
        if let Some(p) = ci_lookup(&self.overwrite, vpath) {
            return Some(p);
        }
        if self.hidden_by_whiteout(vpath) || self.under_opaque_dir(vpath) {
            return None;
        }
        self.layers.iter().find_map(|layer| ci_lookup(layer, vpath))
    }

    /// Whether `vpath` lies under a directory that was deleted and then re-created
    /// in the overwrite layer (carrying an opaque marker). Such a directory hides
    /// its lower-layer contents, so a lower file beneath it must not resolve.
    fn under_opaque_dir(&self, vpath: &str) -> bool {
        let norm = normalize(vpath);
        let comps: Vec<_> = norm.components().collect();
        let mut prefix = PathBuf::new();
        // Proper ancestors only: each directory above the leaf component.
        for comp in comps.iter().take(comps.len().saturating_sub(1)) {
            prefix.push(comp);
            if let Some(dir) = ci_lookup(&self.overwrite, &prefix.to_string_lossy()) {
                if dir.join(OPAQUE_MARKER).exists() {
                    return true;
                }
            }
        }
        false
    }

    /// The overwrite layer's root directory (for statfs / free-space queries).
    pub fn overwrite_root(&self) -> &Path {
        &self.overwrite
    }

    /// The real path in the overwrite layer where a write to `vpath` lands,
    /// resolved case-insensitively against the existing overwrite tree: an entry
    /// that differs only in case is REUSED (so a write / delete / rename hits the
    /// same real file the reads fold to), and only genuinely-new trailing
    /// components take the requested casing.
    ///
    /// Writes never touch a lower layer; on first write the daemon copies the
    /// resolved lower file up to this path (copy-on-write) before applying the
    /// change. Without the case fold, writing `FOO.TXT` when the overwrite already
    /// holds `Foo.txt` would create a second case-variant (split-brain), and a
    /// delete of `FOO.TXT` would miss the `Foo.txt` copy and leave it visible.
    pub fn resolve_write(&self, vpath: &str) -> PathBuf {
        let mut current = self.overwrite.clone();
        for component in normalize(vpath).components() {
            let want = component.as_os_str();
            let exact = current.join(want);
            if exact.exists() {
                current = exact;
                continue;
            }
            let matched = fs::read_dir(&current)
                .ok()
                .and_then(|rd| rd.flatten().find(|e| eq_ignore_case(e.file_name().as_os_str(), want)));
            current = matched.map(|e| e.path()).unwrap_or(exact);
        }
        current
    }

    /// Whether a write to `vpath` needs a copy-up first: it exists in a lower
    /// layer but not yet in the overwrite layer.
    pub fn needs_copy_up(&self, vpath: &str) -> bool {
        ci_lookup(&self.overwrite, vpath).is_none()
            && self.layers.iter().any(|l| ci_lookup(l, vpath).is_some())
    }

    /// Ensure `vpath` is writable in the overwrite layer and return the real
    /// path to write to. If the file exists only in a lower layer, it is copied
    /// up first (copy-on-write); parent directories are created in the overwrite
    /// layer as needed. Lower layers are never touched, so the game install and
    /// every mod source stay pristine.
    pub fn open_for_write(&self, vpath: &str) -> std::io::Result<PathBuf> {
        // Serialise mutations of THIS path. The FUSE daemon serves requests from
        // several threads, and two of them arriving on the same not-yet-copied-up
        // file would both see `!dest.exists()` and both run `fs::copy` into the
        // same destination - interleaving the two writes. Sharded so unrelated
        // paths never contend.
        let _guard = self.path_lock(vpath);
        let dest = self.resolve_write(vpath);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        // Writing a path un-deletes it: drop any stale whiteout.
        self.clear_whiteout(vpath);
        if !dest.exists() {
            if let Some(src) = self.layers.iter().find_map(|l| ci_lookup(l, vpath)) {
                if src.is_file() {
                    fs::copy(&src, &dest)?;
                    // The point of a copy-up is that the result is WRITABLE, and
                    // `fs::copy` clones the source's mode - so a read-only lower
                    // file (a Steam depot restored 0444, a mod extracted from a
                    // Windows archive carrying the DOS read-only attribute) yields
                    // a read-only copy whose very next read-write open fails
                    // EACCES. Do it BEFORE clone_metadata: `lsetxattr` of a
                    // `user.*` attribute onto a 0444 file is refused even for the
                    // owner, so the xattrs would be silently dropped otherwise.
                    ensure_owner_writable(&dest);
                    // Re-apply the lower file's mtime/atime + user.* xattrs so the
                    // copied-up file looks unchanged to tools comparing mtimes
                    // (FileTime load order, xEdit, Wrye Bash) or reading DOS
                    // attributes - usvfs preserves these by writing through in place.
                    clone_metadata(&src, &dest);
                }
            }
        } else if self.layers.iter().any(|l| ci_lookup(l, vpath).is_some()) {
            // A destination that already exists AND is shadowing a lower layer is
            // an orphaned copy-up from an earlier run, which may carry that run's
            // 0444. Heal it - but only in that case: a file living solely in the
            // Overwrite is the user's own, and a read-only mode they set on it
            // (the classic "stop the launcher rewriting my INI") must survive.
            ensure_owner_writable(&dest);
        }
        Ok(dest)
    }

    /// Create a directory in the overwrite layer (and any missing parents),
    /// returning its real path. Idempotent.
    pub fn make_dir(&self, vpath: &str) -> std::io::Result<PathBuf> {
        let dest = self.resolve_write(vpath);
        // If this directory was previously deleted (a whiteout) and still exists in
        // a lower layer, re-creating it must NOT resurrect the deleted lower
        // contents (NTFS: a recreated directory is empty). Clearing the whiteout
        // alone would expose them, so drop an opaque marker that list_dir /
        // resolve_read honour to keep the lower files hidden.
        let was_deleted = self.find_whiteout(vpath).is_some();
        fs::create_dir_all(&dest)?;
        self.clear_whiteout(vpath);
        if was_deleted && self.layers.iter().any(|l| ci_lookup(l, vpath).is_some_and(|p| p.is_dir())) {
            let _ = fs::write(dest.join(OPAQUE_MARKER), b"");
        }
        Ok(dest)
    }

    /// Prepare `vpath` for a fresh write in the overwrite layer: a symlink or
    /// other special entry the daemon creates directly. Materialises the parent
    /// directories and drops any whiteout, returning the real overwrite path.
    /// Unlike [`Self::open_for_write`] it does NOT copy a lower file up, because
    /// the caller is replacing the entry, not editing it; the new overwrite entry
    /// shadows any lower-layer copy.
    pub fn prepare_overwrite(&self, vpath: &str) -> std::io::Result<PathBuf> {
        let _guard = self.path_lock(vpath);
        let dest = self.resolve_write(vpath);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        self.clear_whiteout(vpath);
        // A read-only copy left in the overwrite layer by an earlier run (copied up
        // from a 0444 lower file) would make the caller's truncating open fail
        // EACCES even though we are about to replace the contents wholesale. Only
        // for a path a lower layer also provides, so a user-set read-only mode on
        // their own Overwrite file is not silently undone.
        if dest.exists() && self.layers.iter().any(|l| ci_lookup(l, vpath).is_some()) {
            ensure_owner_writable(&dest);
        }
        Ok(dest)
    }

    /// Delete `vpath`: remove its overwrite copy if present, and write a whiteout
    /// so any copy in a lower (mod or game) layer stays hidden. Lower layers are
    /// never modified.
    pub fn remove(&self, vpath: &str) -> std::io::Result<()> {
        let dest = self.resolve_write(vpath);
        if dest.is_dir() {
            fs::remove_dir_all(&dest)?;
        } else if dest.exists() {
            fs::remove_file(&dest)?;
        }
        if self.layers.iter().any(|l| ci_lookup(l, vpath).is_some()) {
            let wh = self.whiteout_path(vpath);
            if let Some(parent) = wh.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(wh, b"")?;
        }
        Ok(())
    }

    /// Rename `from` to `to` within the overwrite layer (copying `from` up first
    /// if it only exists in a lower layer), then whiteout `from` so its lower
    /// copy is hidden. File-oriented; lower-only directory renames are not yet
    /// supported.
    pub fn rename(&self, from: &str, to: &str) -> std::io::Result<()> {
        // A directory rename must move the MERGED subtree (overwrite + every lower
        // layer), not just the overwrite half - otherwise a lower-only directory
        // errors and a mixed directory silently loses its lower children.
        if self.resolve_read(from).is_some_and(|p| p.is_dir()) {
            return self.rename_dir(from, to);
        }
        let src = self.open_for_write(from)?;
        let dst = self.resolve_write(to);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&src, &dst)?;
        self.clear_whiteout(to);
        if self.layers.iter().any(|l| ci_lookup(l, from).is_some()) {
            let wh = self.whiteout_path(from);
            if let Some(parent) = wh.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(wh, b"")?;
        }
        Ok(())
    }

    /// Rename a directory: materialise its merged contents (every layer) under `to`,
    /// then whiteout `from`. Recurses per child so nested directories and lower-only
    /// files are copied up and moved, mirroring usvfs's whole-subtree remap.
    fn rename_dir(&self, from: &str, to: &str) -> std::io::Result<()> {
        self.make_dir(to)?;
        for (name, _) in self.list_dir(from) {
            self.rename(&join_vpath(from, &name), &join_vpath(to, &name))?;
        }
        self.remove(from)?;
        Ok(())
    }

    /// The merged contents of a directory: every entry across the overwrite and
    /// mod layers, deduplicated by case-insensitive name with the highest layer
    /// winning. Each entry is `(name, real_path_on_disk)`, where the name keeps
    /// the casing of the layer that won.
    ///
    /// Used by the FUSE daemon to answer `readdir`.
    pub fn list_dir(&self, vpath: &str) -> Vec<(String, PathBuf)> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut whiteouts: HashSet<String> = HashSet::new();
        let mut out: Vec<(String, PathBuf)> = Vec::new();

        // Overwrite layer first: collect whiteouts (and hide the markers).
        let mut opaque = false;
        if let Some(dir) = ci_lookup(&self.overwrite, vpath).filter(|d| d.is_dir()) {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name == OPAQUE_MARKER {
                        opaque = true;
                        continue;
                    }
                    if let Some(hidden) = name.strip_prefix(WHITEOUT_PREFIX) {
                        whiteouts.insert(hidden.to_ascii_lowercase());
                        continue;
                    }
                    if is_hidden_name(&name) {
                        continue;
                    }
                    if seen.insert(name.to_ascii_lowercase()) {
                        out.push((name, entry.path()));
                    }
                }
            }
        }

        // If this directory was itself deleted and then re-created in the overwrite
        // layer it is opaque (a whiteout on it, or an opaque marker inside it): its
        // lower-layer contents stay hidden.
        if opaque || self.find_whiteout(vpath).is_some() {
            return out;
        }

        // Lower layers: skip whited-out names.
        for layer in &self.layers {
            let Some(dir) = ci_lookup(layer, vpath).filter(|d| d.is_dir()) else { continue };
            let Ok(entries) = fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let key = name.to_ascii_lowercase();
                if whiteouts.contains(&key) {
                    continue;
                }
                // A user-hidden entry is dropped without claiming its name, so a
                // lower layer's copy of the file it shadowed becomes the winner -
                // which is the whole point of hiding one mod's stray override.
                if is_hidden_name(&name) {
                    continue;
                }
                if seen.insert(key) {
                    out.push((name, entry.path()));
                }
            }
        }

        // Bethesda's Creation Engine indexes the Data tree at startup assuming
        // NTFS-style SORTED directory enumeration. Returning entries in raw order
        // (layer order + the backing filesystem's readdir order) makes its
        // loose-file indexer build a record it can't later resolve, then deref a
        // -1/null result -> a deterministic crash at the main menu under a FUSE
        // mount (works under MO2/usvfs, which hands Windows-friendly listings).
        // Emulate NTFS collation exactly: uppercase (the $UpCase table, which
        // usvfs reaches via LCMapStringW) then order by UTF-16 code unit. A plain
        // ASCII-lowercase sort orders names that mix letters with `_ { } ~` (all
        // between `Z` and `a` in ASCII) differently, which the indexer can notice.
        // Same fix CIOPFS / ntfs-emu apply so non-NTFS filesystems work with Wine.
        out.sort_by_cached_key(|(name, _)| ntfs_order_key(name));
        out
    }
}

/// NTFS-style collation key for ordering a directory listing: uppercase (the
/// `$UpCase` table, approximated here by Unicode uppercasing) then compare as
/// UTF-16 code units - what NTFS and usvfs (`LCMapStringW`) actually use. A plain
/// ASCII-lowercase ordering disagrees on names mixing letters with `_ { } ~`.
fn ntfs_order_key(name: &str) -> Vec<u16> {
    let upper: String = name.chars().flat_map(char::to_uppercase).collect();
    upper.encode_utf16().collect()
}

/// Turn a virtual path into a clean relative path that can be joined onto a real
/// layer root. Keeps only `Normal` segments: the leading slash, `.`, `..`, and
/// empty segments are dropped, so a vpath can never traverse out of its layer root
/// (defence in depth - the kernel never forwards `..` to FUSE lookups, but a vpath
/// constructed any other way stays contained).
fn normalize(vpath: &str) -> PathBuf {
    vpath.split('/').filter(|s| !s.is_empty() && *s != "." && *s != "..").collect()
}

/// Join a child entry name onto a virtual directory path.
fn join_vpath(parent: &str, name: &str) -> String {
    let p = parent.trim_matches('/');
    if p.is_empty() {
        name.to_string()
    } else {
        format!("{p}/{name}")
    }
}

/// Resolve `vpath` against `root` case-insensitively, component by component,
/// returning the real cased path on disk if every component matches.
///
/// The exact-case join is tried first as a fast path; only on a miss do we scan
/// the directory for a case-insensitive match.
fn ci_lookup(root: &Path, vpath: &str) -> Option<PathBuf> {
    let mut current = root.to_path_buf();
    for component in normalize(vpath).components() {
        let want = component.as_os_str();
        let exact = current.join(want);
        if exact.exists() {
            current = exact;
            continue;
        }
        let entry = fs::read_dir(&current)
            .ok()?
            .filter_map(Result::ok)
            .find(|e| eq_ignore_case(e.file_name().as_os_str(), want))?;
        current = entry.path();
    }
    Some(current)
}

/// Case-insensitive name comparison.
///
/// v1 folds ASCII case only; Windows folds a wider Unicode table. Most game and
/// mod paths are ASCII; broadening this is tracked in `docs/architecture.md`.
fn eq_ignore_case(a: &OsStr, b: &OsStr) -> bool {
    match (a.to_str(), b.to_str()) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => a == b, // non-UTF8 names: fall back to exact bytes
    }
}

/// Re-apply `src`'s modification/access times and `user.*` xattrs onto `dst` after
/// a copy-up. Best-effort: failures are ignored (the copy already succeeded).
/// Ensure the owner can write `path`, preserving every other permission bit.
/// Best-effort: a failure here is never worse than the read-only mode we started
/// from, and the caller's open will report the real error.
fn ensure_owner_writable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    // `symlink_metadata`, not `metadata`: both `metadata` and `set_permissions`
    // FOLLOW symlinks, so chmod-ing an overwrite entry that happens to be a
    // symlink would land on its TARGET - which can be a pristine game or mod
    // file, breaking the one guarantee this whole filesystem exists to make.
    // A symlink's own mode is meaningless on Linux, so there is nothing to do.
    let Ok(meta) = fs::symlink_metadata(path) else { return };
    if meta.file_type().is_symlink() {
        return;
    }
    let mode = meta.permissions().mode();
    if mode & 0o200 == 0 {
        let mut perms = meta.permissions();
        perms.set_mode(mode | 0o200);
        let _ = fs::set_permissions(path, perms);
    }
}

fn clone_metadata(src: &Path, dst: &Path) {
    if let Ok(meta) = fs::metadata(src) {
        if let (Ok(atime), Ok(mtime)) = (meta.accessed(), meta.modified()) {
            let times = [to_timespec(atime), to_timespec(mtime)];
            if let Ok(c) = cpath(dst) {
                unsafe { libc::utimensat(libc::AT_FDCWD, c.as_ptr(), times.as_ptr(), 0) };
            }
        }
    }
    copy_user_xattrs(src, dst);
}

fn to_timespec(t: std::time::SystemTime) -> libc::timespec {
    let d = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    libc::timespec { tv_sec: d.as_secs() as libc::time_t, tv_nsec: d.subsec_nanos() as libc::c_long }
}

fn cpath(p: &Path) -> std::io::Result<std::ffi::CString> {
    std::ffi::CString::new(p.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))
}

/// Copy every `user.*` extended attribute from `src` to `dst` (Linux `l*xattr`, so
/// a symlink itself is operated on, not its target). Best-effort.
fn copy_user_xattrs(src: &Path, dst: &Path) {
    let (Ok(csrc), Ok(cdst)) = (cpath(src), cpath(dst)) else {
        return;
    };
    let len = unsafe { libc::llistxattr(csrc.as_ptr(), std::ptr::null_mut(), 0) };
    if len <= 0 {
        return;
    }
    let mut names = vec![0u8; len as usize];
    let got =
        unsafe { libc::llistxattr(csrc.as_ptr(), names.as_mut_ptr() as *mut libc::c_char, names.len()) };
    if got <= 0 {
        return;
    }
    names.truncate(got as usize);
    for name in names.split(|&b| b == 0).filter(|s| s.starts_with(b"user.")) {
        let Ok(cname) = std::ffi::CString::new(name) else { continue };
        let vlen = unsafe { libc::lgetxattr(csrc.as_ptr(), cname.as_ptr(), std::ptr::null_mut(), 0) };
        if vlen < 0 {
            continue;
        }
        let mut val = vec![0u8; vlen as usize];
        let vgot = unsafe {
            libc::lgetxattr(csrc.as_ptr(), cname.as_ptr(), val.as_mut_ptr() as *mut libc::c_void, val.len())
        };
        if vgot < 0 {
            continue;
        }
        unsafe {
            libc::lsetxattr(cdst.as_ptr(), cname.as_ptr(), val.as_ptr() as *const libc::c_void, vgot as usize, 0)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// A throwaway directory tree that cleans itself up, no external deps.
    struct TempTree(PathBuf);

    impl TempTree {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("eidos-{}-{}", std::process::id(), n));
            fs::create_dir_all(&dir).unwrap();
            TempTree(dir)
        }

        fn sub(&self, name: &str) -> PathBuf {
            let p = self.0.join(name);
            fs::create_dir_all(&p).unwrap();
            p
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn put(dir: &Path, rel: &str, contents: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, contents).unwrap();
    }

    fn read(p: &Path) -> String {
        fs::read_to_string(p).unwrap()
    }

    fn set_mtime(p: &Path, t: std::time::SystemTime) {
        let ts = to_timespec(t);
        let times = [ts, ts];
        let c = std::ffi::CString::new(p.as_os_str().as_bytes()).unwrap();
        unsafe { libc::utimensat(libc::AT_FDCWD, c.as_ptr(), times.as_ptr(), 0) };
    }

    #[test]
    fn copy_up_preserves_mtime() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "mesh.nif", "data");
        // Backdate the lower file to a fixed old time (2014), then copy it up.
        let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_400_000_000);
        set_mtime(&game.join("mesh.nif"), old);
        let stack = LayerStack::new(vec![game], over);

        let dest = stack.open_for_write("mesh.nif").unwrap();
        let secs = fs::metadata(&dest)
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(secs, 1_400_000_000, "copy-up must keep the lower file's mtime, not stamp 'now'");
    }

    #[test]
    fn copy_up_of_a_read_only_file_is_writable() {
        use std::os::unix::fs::PermissionsExt;
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "readonly.ini", "vanilla");
        // A Steam depot restored 0444, or a mod extracted from a Windows archive
        // carrying the DOS read-only attribute.
        fs::set_permissions(game.join("readonly.ini"), fs::Permissions::from_mode(0o444)).unwrap();
        let stack = LayerStack::new(vec![game], over);

        let dest = stack.open_for_write("readonly.ini").unwrap();
        // The point of a copy-up is that the caller can now write to it.
        fs::OpenOptions::new()
            .write(true)
            .open(&dest)
            .expect("a copied-up file must be writable by its owner");
        // The lower layer is untouched, read-only mode included.
        assert_eq!(fs::metadata(&dest).unwrap().permissions().mode() & 0o200, 0o200);
    }

    #[test]
    fn copy_up_never_chmods_through_a_symlink_into_a_lower_layer() {
        use std::os::unix::fs::PermissionsExt;
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "pristine.esp", "vanilla");
        // The game file is read-only, as a restored Steam depot would be.
        fs::set_permissions(game.join("pristine.esp"), fs::Permissions::from_mode(0o444)).unwrap();
        // An overwrite entry that is a SYMLINK pointing back at it.
        std::os::unix::fs::symlink(game.join("pristine.esp"), over.join("link.esp")).unwrap();
        let stack = LayerStack::new(vec![game.clone()], over);

        // Any write-preparation on the link must not reach through it.
        let _ = stack.open_for_write("link.esp");
        let _ = stack.prepare_overwrite("link.esp");

        let mode = fs::symlink_metadata(game.join("pristine.esp")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o444, "the game file's mode was changed through a symlink");
    }

    #[test]
    fn a_read_only_file_of_our_own_keeps_its_mode() {
        use std::os::unix::fs::PermissionsExt;
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        // Lives ONLY in the overwrite layer: the user's own file, which they set
        // read-only on purpose (the classic "stop the launcher rewriting my INI").
        put(&over, "SkyrimPrefs.ini", "mine");
        fs::set_permissions(over.join("SkyrimPrefs.ini"), fs::Permissions::from_mode(0o444)).unwrap();
        let stack = LayerStack::new(vec![game], over.clone());

        let _ = stack.open_for_write("SkyrimPrefs.ini");
        let mode = fs::metadata(over.join("SkyrimPrefs.ini")).unwrap().permissions().mode();
        assert_eq!(mode & 0o200, 0, "a user-set read-only mode must not be silently undone");
    }

    #[test]
    fn prepare_overwrite_heals_a_read_only_copy_from_an_earlier_run() {
        use std::os::unix::fs::PermissionsExt;
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "gen.txt", "vanilla");
        // Simulate the overwrite copy an older Eidos left behind at 0444.
        put(&over, "gen.txt", "stale");
        fs::set_permissions(over.join("gen.txt"), fs::Permissions::from_mode(0o444)).unwrap();
        let stack = LayerStack::new(vec![game], over);

        let dest = stack.prepare_overwrite("gen.txt").unwrap();
        fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&dest)
            .expect("a truncating open must not fail on a stale read-only copy");
    }

    #[test]
    fn higher_priority_layer_wins() {
        let t = TempTree::new();
        let (game, a, b, over) = (t.sub("game"), t.sub("a"), t.sub("b"), t.sub("over"));
        put(&game, "textures.dat", "vanilla");
        put(&a, "textures.dat", "mod a");
        put(&b, "textures.dat", "mod b");

        let stack = LayerStack::new(vec![b, a, game], over);
        assert_eq!(read(&stack.resolve_read("textures.dat").unwrap()), "mod b");
    }

    #[test]
    fn falls_through_to_game_data() {
        let t = TempTree::new();
        let (game, a, over) = (t.sub("game"), t.sub("a"), t.sub("over"));
        put(&game, "meshes.dat", "vanilla mesh");

        let stack = LayerStack::new(vec![a, game], over);
        assert_eq!(read(&stack.resolve_read("meshes.dat").unwrap()), "vanilla mesh");
    }

    #[test]
    fn overwrite_layer_shadows_everything() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "save.dat", "old");
        put(&over, "save.dat", "new");

        let stack = LayerStack::new(vec![game], over);
        assert_eq!(read(&stack.resolve_read("save.dat").unwrap()), "new");
    }

    #[test]
    fn case_insensitive_lookup() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "Textures/Armor.DDS", "tex");

        let stack = LayerStack::new(vec![game], over);
        // The game engine asks with completely different casing.
        assert_eq!(read(&stack.resolve_read("textures/armor.dds").unwrap()), "tex");
    }

    #[test]
    fn normalize_strips_traversal_segments() {
        // `..`, `.`, leading slash, and empty segments are dropped, so a vpath can
        // never resolve outside its layer root (defence in depth).
        assert_eq!(normalize("/a/b"), PathBuf::from("a/b"));
        assert_eq!(normalize("a/../../b"), PathBuf::from("a/b"));
        assert_eq!(normalize("../../etc/passwd"), PathBuf::from("etc/passwd"));
        assert_eq!(normalize("a/./b//c"), PathBuf::from("a/b/c"));
        // No component is ever ParentDir / RootDir.
        assert!(normalize("../x/..").components().all(|c| matches!(c, std::path::Component::Normal(_))));
    }

    #[test]
    fn resolve_read_cannot_escape_the_layer_root() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "ok.txt", "in");
        // A secret one level above the game root must stay unreachable.
        std::fs::write(t.0.join("secret.txt"), "SECRET").unwrap();

        let stack = LayerStack::new(vec![game], over);
        assert_eq!(read(&stack.resolve_read("ok.txt").unwrap()), "in");
        // `..` is normalised away, so this resolves to <root>/secret.txt (absent), not
        // the parent's secret.
        assert!(stack.resolve_read("../secret.txt").is_none());
    }

    #[test]
    fn write_lands_in_overwrite_with_copy_up() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "config.ini", "vanilla");

        let stack = LayerStack::new(vec![game], over.clone());
        assert!(stack.needs_copy_up("config.ini"));
        assert_eq!(stack.resolve_write("config.ini"), over.join("config.ini"));
        // A brand-new file the game creates does not need a copy-up.
        assert!(!stack.needs_copy_up("brandnew.tmp"));
    }

    #[test]
    fn missing_path_resolves_to_none() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        let stack = LayerStack::new(vec![game], over);
        assert!(stack.resolve_read("does/not/exist").is_none());
    }

    #[test]
    fn list_dir_merges_layers_and_dedupes() {
        let t = TempTree::new();
        let (game, modd, over) = (t.sub("game"), t.sub("mod"), t.sub("over"));
        put(&game, "a.dat", "ga");
        put(&game, "shared.dat", "game version");
        put(&modd, "b.dat", "mb");
        put(&modd, "Shared.dat", "mod version"); // same name, different case + wins

        let stack = LayerStack::new(vec![modd, game], over);
        let mut names: Vec<String> = stack.list_dir("").into_iter().map(|(n, _)| n).collect();
        names.sort();
        // shared appears once (the mod's casing wins), plus the two uniques.
        assert_eq!(names, vec!["Shared.dat", "a.dat", "b.dat"]);

        // And the winning shared entry points at the mod's file.
        let shared = stack
            .list_dir("")
            .into_iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("shared.dat"))
            .unwrap();
        assert_eq!(read(&shared.1), "mod version");
    }

    #[test]
    fn a_hidden_file_leaves_the_view_and_lets_the_lower_layer_win() {
        let t = TempTree::new();
        let (game, modd, over) = (t.sub("game"), t.sub("mod"), t.sub("over"));
        put(&game, "textures/rock.dds", "vanilla");
        // What hiding actually produces on disk: the override renamed, not removed.
        put(&modd, "textures/rock.dds.mohidden", "the override we suppressed");
        put(&modd, "textures/tree.dds", "still active");

        let stack = LayerStack::new(vec![modd, game], over);
        let mut names: Vec<String> =
            stack.list_dir("textures").into_iter().map(|(n, _)| n).collect();
        names.sort();
        // The suffixed name never appears, and rock.dds is the vanilla file again.
        assert_eq!(names, vec!["rock.dds", "tree.dds"]);
        assert_eq!(read(&stack.resolve_read("textures/rock.dds").unwrap()), "vanilla");
        // Nor can it be reached by asking for the suffixed path directly.
        assert!(stack.resolve_read("textures/rock.dds.mohidden").is_none());
    }

    #[test]
    fn hiding_a_directory_hides_everything_under_it() {
        let t = TempTree::new();
        let (modd, over) = (t.sub("mod"), t.sub("over"));
        put(&modd, "meshes.MOHIDDEN/actors/body.nif", "suppressed");
        put(&modd, "meshes/actors/head.nif", "active");

        let stack = LayerStack::new(vec![modd], over);
        let names: Vec<String> = stack.list_dir("").into_iter().map(|(n, _)| n).collect();
        // Mixed case, because a mod folder round-tripped through Windows comes
        // back shouting.
        assert_eq!(names, vec!["meshes"]);
        assert!(stack.resolve_read("meshes.MOHIDDEN/actors/body.nif").is_none());
        assert!(stack.resolve_read("meshes/actors/head.nif").is_some());
    }

    #[test]
    fn a_multibyte_name_does_not_panic_the_resolver() {
        // Byte 9-from-the-end lands INSIDE the first CJK character here, which is
        // what a `&name[len - 9..]` slice panics on. Real mod folders look like
        // this: the pool this was found in had `至真女性皮肤4K-Zhizhen's female
        // skin 4K`. A panic here kills the mount in the middle of a read.
        for n in ["abcdefg日日日h", "至真女性皮肤4K.bsa", "Épées Légendaires.esp", "日本語"] {
            assert!(!is_hidden_name(n), "{n}");
            assert!(!under_hidden(&format!("meshes/{n}/x.nif")));
        }
        // And the suffix is still recognised when it is there, multi-byte stem or not.
        assert!(is_hidden_name("至真女性皮肤4K.bsa.mohidden"));
        assert!(under_hidden("meshes/至真.mohidden/body.nif"));
    }

    #[test]
    fn a_dotfile_is_not_mistaken_for_a_hidden_entry() {
        // `.mohidden` on its own is a legitimate (if odd) file name, not a marker:
        // the suffix has to be attached to something for the rename to be undoable.
        assert!(!is_hidden_name(".mohidden"));
        assert!(is_hidden_name("a.mohidden"));
        assert!(!is_hidden_name("mohidden"));
        assert!(!is_hidden_name("foo.mohidden.bak"));
    }

    #[test]
    fn list_dir_emits_ntfs_collated_order() {
        let t = TempTree::new();
        let (game, modd, over) = (t.sub("game"), t.sub("mod"), t.sub("over"));
        // Discriminates NTFS upcase-collation from a plain ASCII-lowercase sort:
        // `_` (0x5F) sits between `Z` (0x5A) and `a` (0x61), so upcasing orders
        // `_skse` AFTER the letters, while a lowercase sort would put it first.
        put(&game, "_skse.dat", "x");
        put(&game, "apple.dat", "x");
        put(&modd, "Zebra.dat", "x");

        let stack = LayerStack::new(vec![modd, game], over);
        let names: Vec<String> = stack.list_dir("").into_iter().map(|(n, _)| n).collect();
        // NTFS order: letters before `_`. A lowercase sort would wrongly yield
        // [_skse, apple, Zebra]; assert the emission order verbatim (no re-sort).
        assert_eq!(names, vec!["apple.dat", "Zebra.dat", "_skse.dat"]);
    }

    #[test]
    fn copy_up_clones_lower_file_then_diverges() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "config/settings.ini", "vanilla");
        let stack = LayerStack::new(vec![game.clone()], over.clone());

        // First write copies the lower file up, parents and all.
        let dest = stack.open_for_write("config/settings.ini").unwrap();
        assert_eq!(dest, over.join("config/settings.ini"));
        assert_eq!(read(&dest), "vanilla");

        // Diverging the overwrite copy leaves the game file pristine.
        fs::write(&dest, "tweaked").unwrap();
        assert_eq!(read(&dest), "tweaked");
        assert_eq!(read(&game.join("config/settings.ini")), "vanilla");

        // And the resolver now serves the overwrite version.
        assert_eq!(read(&stack.resolve_read("config/settings.ini").unwrap()), "tweaked");
    }

    #[test]
    fn open_for_write_creates_brand_new_file_path() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        let stack = LayerStack::new(vec![game], over.clone());

        let dest = stack.open_for_write("saves/save01.ess").unwrap();
        assert_eq!(dest, over.join("saves/save01.ess"));
        assert!(dest.parent().unwrap().is_dir()); // parents materialised
        assert!(!dest.exists()); // not created yet, just made writable
    }

    #[test]
    fn delete_hides_lower_file_via_whiteout() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "a.txt", "vanilla");
        put(&game, "b.txt", "keep");
        let stack = LayerStack::new(vec![game.clone()], over);

        stack.remove("a.txt").unwrap();
        assert!(stack.resolve_read("a.txt").is_none()); // hidden
        assert_eq!(read(&game.join("a.txt")), "vanilla"); // game pristine

        // list_dir excludes the deleted file and the marker; keeps the rest.
        let names: Vec<String> = stack.list_dir("").into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, vec!["b.txt"]);
    }

    #[test]
    fn recreate_after_delete_undeletes() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "a.txt", "vanilla");
        let stack = LayerStack::new(vec![game], over);

        stack.remove("a.txt").unwrap();
        assert!(stack.resolve_read("a.txt").is_none());

        // Writing it again clears the whiteout and serves the overwrite copy.
        let dest = stack.open_for_write("a.txt").unwrap();
        fs::write(&dest, "rewritten").unwrap();
        assert_eq!(read(&stack.resolve_read("a.txt").unwrap()), "rewritten");
    }

    #[test]
    fn rename_moves_and_whiteouts_source() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "save.tmp", "savedata");
        let stack = LayerStack::new(vec![game.clone()], over);

        stack.rename("save.tmp", "save.ess").unwrap();
        assert_eq!(read(&stack.resolve_read("save.ess").unwrap()), "savedata");
        assert!(stack.resolve_read("save.tmp").is_none()); // old name hidden
        assert_eq!(read(&game.join("save.tmp")), "savedata"); // game pristine
    }

    #[test]
    fn writes_and_deletes_fold_case_against_overwrite() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "Foo.txt", "vanilla");
        let stack = LayerStack::new(vec![game], over.clone());

        // Copy the lower file up under its real casing, then diverge it.
        let dest = stack.open_for_write("Foo.txt").unwrap();
        fs::write(&dest, "edited").unwrap();
        assert_eq!(dest, over.join("Foo.txt"));

        // Opening with DIFFERENT casing must reuse the same overwrite file, not
        // create a second case-variant (split-brain).
        let again = stack.open_for_write("FOO.TXT").unwrap();
        assert_eq!(again, over.join("Foo.txt"));
        let variants: Vec<_> = fs::read_dir(&over)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.eq_ignore_ascii_case("foo.txt"))
            .collect();
        assert_eq!(variants.len(), 1, "exactly one overwrite copy, got {variants:?}");

        // Deleting via yet another casing removes the real copy and hides the lower
        // file - a literal-case delete would have missed the `Foo.txt` copy.
        stack.remove("foo.TXT").unwrap();
        assert!(stack.resolve_read("Foo.txt").is_none());
        assert!(stack.resolve_read("FOO.TXT").is_none());
        assert!(!over.join("Foo.txt").exists());
    }

    #[test]
    fn rename_lower_only_directory_moves_the_whole_subtree() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "tools/a.txt", "a");
        put(&game, "tools/sub/b.txt", "b");
        let stack = LayerStack::new(vec![game.clone()], over);

        stack.rename("tools", "tools_bak").unwrap();
        // Every (lower-only) child arrived at the destination, nesting included...
        assert_eq!(read(&stack.resolve_read("tools_bak/a.txt").unwrap()), "a");
        assert_eq!(read(&stack.resolve_read("tools_bak/sub/b.txt").unwrap()), "b");
        // ...the source is gone (hidden), and the game install stays pristine.
        assert!(stack.resolve_read("tools/a.txt").is_none());
        assert!(stack.resolve_read("tools").is_none());
        assert_eq!(read(&game.join("tools/a.txt")), "a");
    }

    #[test]
    fn rename_mixed_directory_keeps_lower_children() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "data/lower.esp", "L"); // only in the game layer
        let stack = LayerStack::new(vec![game], over);
        // The game writes a new file into the same dir (overwrite layer).
        let w = stack.open_for_write("data/over.esp").unwrap();
        fs::write(&w, "O").unwrap();

        stack.rename("data", "data2").unwrap();
        // BOTH the lower and the overwrite child move - the lower one is not lost.
        assert_eq!(read(&stack.resolve_read("data2/lower.esp").unwrap()), "L");
        assert_eq!(read(&stack.resolve_read("data2/over.esp").unwrap()), "O");
        let mut names: Vec<String> = stack.list_dir("data2").into_iter().map(|(n, _)| n).collect();
        names.sort();
        assert_eq!(names, vec!["lower.esp", "over.esp"]);
        // Source hidden.
        assert!(stack.resolve_read("data/lower.esp").is_none());
        assert!(stack.list_dir("data").is_empty());
    }

    #[test]
    fn whiteout_is_case_insensitive() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "FOO.dat", "vanilla");
        let stack = LayerStack::new(vec![game], over);

        stack.remove("FOO.dat").unwrap();
        // Deleted with one casing, probed with another: must stay hidden, or a
        // case-insensitive game engine would resurface the "deleted" file.
        assert!(stack.resolve_read("foo.dat").is_none());
        assert!(stack.resolve_read("FOO.dat").is_none());
    }

    #[test]
    fn deleting_a_directory_hides_its_files_by_path() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "meshes/a.nif", "a");
        put(&game, "meshes/b.nif", "b");
        let stack = LayerStack::new(vec![game.clone()], over);

        stack.remove("meshes").unwrap();
        // The directory is opaque: children are hidden even when asked by full
        // path, not just absent from the listing.
        assert!(stack.resolve_read("meshes/a.nif").is_none());
        assert!(stack.resolve_read("meshes").is_none());
        // ...and the game install stays pristine.
        assert_eq!(read(&game.join("meshes/a.nif")), "a");
    }

    #[test]
    fn recreated_directory_is_opaque() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "data/old.esp", "lower");
        let stack = LayerStack::new(vec![game], over);

        stack.remove("data").unwrap();
        // Re-create the directory with a brand-new file.
        let dest = stack.open_for_write("data/new.esp").unwrap();
        fs::write(&dest, "fresh").unwrap();

        // Only the new file shows; the deleted lower file stays hidden.
        let names: Vec<String> = stack.list_dir("data").into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, vec!["new.esp"]);
        assert_eq!(read(&stack.resolve_read("data/new.esp").unwrap()), "fresh");
        assert!(stack.resolve_read("data/old.esp").is_none());
    }

    #[test]
    fn rmdir_then_mkdir_keeps_deleted_lower_files_hidden() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "cache/a.txt", "lower-a");
        put(&game, "cache/b.txt", "lower-b");
        let stack = LayerStack::new(vec![game.clone()], over);

        // The classic "clear the cache dir, then recreate it" pattern: deleting the
        // dir removes the per-file child markers, so make_dir must re-establish
        // opacity itself or the lower files come back.
        stack.remove("cache/a.txt").unwrap();
        stack.remove("cache/b.txt").unwrap();
        stack.remove("cache").unwrap(); // rmdir: destroys the child whiteouts
        stack.make_dir("cache").unwrap(); // mkdir: recreate

        // NTFS: a recreated directory is empty. The deleted lower files must NOT
        // resurface in lookups or the listing.
        assert!(stack.resolve_read("cache/a.txt").is_none(), "a.txt resurrected");
        assert!(stack.resolve_read("cache/b.txt").is_none(), "b.txt resurrected");
        let names: Vec<String> = stack.list_dir("cache").into_iter().map(|(n, _)| n).collect();
        assert!(names.is_empty(), "recreated dir should be empty, got {names:?}");
        // The game install stays pristine.
        assert_eq!(read(&game.join("cache/a.txt")), "lower-a");
    }

    #[test]
    fn prepare_overwrite_makes_parents_and_clears_whiteout() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "a/b.txt", "lower");
        let stack = LayerStack::new(vec![game], over.clone());

        stack.remove("a/b.txt").unwrap(); // whiteout it
        let dest = stack.prepare_overwrite("a/b.txt").unwrap();
        assert_eq!(dest, over.join("a/b.txt"));
        assert!(dest.parent().unwrap().is_dir()); // parents materialised
        assert!(!dest.exists()); // no lower file copied up, just made writable
    }
}
