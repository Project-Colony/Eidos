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
//! game engines and mods rely on (see `docs/internals/architecture.md`).

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;

/// Prefix marking a "whiteout" in the overwrite layer: an empty file
/// `<dir>/.eidoswh.<name>` means `<name>` is deleted and any lower-layer copy
/// must stay hidden. This is how a union filesystem records deletions without
/// touching the read-only mod and game layers.
pub const WHITEOUT_PREFIX: &str = ".eidoswh.";

/// Marker file dropped INSIDE an overwrite directory that was deleted and then
/// re-created: it makes the directory opaque (its lower-layer contents stay
/// hidden), matching NTFS where a recreated directory is empty. Distinct from the
/// per-file `.eidoswh.<name>` whiteout so it is never mistaken for one.
pub const OPAQUE_MARKER: &str = ".eidoswh_opaque";

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
    /// What path resolution costs THIS mount. Per-stack, not global: Eidos
    /// mounts two unions at once - Data and, for Root Builder mods, the game
    /// install root - and a process-wide counter reported the same figure for
    /// both, which is worse than reporting none.
    ///
    /// `Arc`, for the same reason as the locks: a cloned stack describes the
    /// same mount and must add to the same tally.
    resolve: std::sync::Arc<ResolveStats>,
    /// What the read-only layers provide, walked once at construction.
    ///
    /// `None` means there is no index and every query walks the layers - the
    /// behaviour that existed before this field, kept reachable because it is
    /// the one that is never wrong.
    lower: Option<std::sync::Arc<LowerIndex>>,
    /// The overwrite membership index; `None` means dirty, rebuilt on next use.
    /// `Arc<Mutex<..>>` for the same reason as the locks and the stats above: a
    /// cloned stack describes the SAME mount, so a mutation noted through one
    /// clone must dirty the index every clone reads. See [`OwIndex`].
    ow: std::sync::Arc<std::sync::Mutex<Option<std::sync::Arc<OwIndex>>>>,
}

/// What the read-only layers provide, resolved once at construction.
///
/// The whole design rests on one property: **the layers never change while
/// mounted**. `LayerStack` has no method that writes into a layer - every
/// mutation lands in `overwrite`, which is deliberately NOT indexed and is read
/// live on every resolve. So there is no invalidation, not because it was
/// skipped, but because there is nothing to invalidate.
///
/// # Why absence is trustworthy
///
/// The index answers "no layer provides this" as confidently as it answers
/// "layer 7 does", and that is only sound if the build was COMPLETE. So the
/// build is all-or-nothing with no exemptions: any unreadable directory, any
/// surprise, and the whole index is discarded and every query walks the layers
/// as before. Slow is a cost; a mod file that silently is not there is a
/// corruption, and this filesystem exists to prevent exactly that.
///
/// `EIDOS_NO_INDEX=1` forces that fallback, for when the difference between the
/// two answers is the thing being debugged.
#[derive(Debug)]
struct LowerIndex {
    /// Folded virtual path -> the real path of the highest-priority layer that
    /// provides it. Folding matches `eq_ignore_case`: ASCII-lowercased.
    entries: HashMap<Box<[u8]>, Resolved>,
    /// Folded virtual directory -> its MERGED lower-layer listing, already in
    /// priority order and deduplicated by folded name.
    ///
    /// `entries` answers "who wins this path", which is all a resolve needs. A
    /// listing needs something else entirely: every layer's copy of one
    /// directory, merged. That is why `readdir` was the one handler the index did
    /// nothing for - it asked each of the layers separately, so a listing on a
    /// 50-layer stack cost 50 case-folding walks whatever the index knew.
    ///
    /// Merging at build time rather than per call is what makes it O(1): the
    /// layers cannot change while mounted, so their merge cannot either. Only the
    /// overwrite is read live, and the caller still applies whiteouts on top -
    /// those live in the overwrite and are none of this map's business.
    ///
    /// Names hidden with `.mohidden` are dropped HERE, before they can claim a
    /// slot, so a lower layer's copy of the file a hidden one shadowed becomes
    /// the winner - which is the point of hiding one mod's stray override.
    dirs: HashMap<Box<[u8]>, Box<[DirEntryInfo]>>,
}

/// One entry of a merged listing: the name as the winning layer spells it, where
/// it really is, and what `readdir` said it was.
type DirEntryInfo = (String, PathBuf, Option<fs::FileType>);

/// The two maps under construction, plus the per-directory bookkeeping that only
/// the build needs. Passed as one value so `walk_layer` keeps a single
/// accumulator argument rather than four.
#[derive(Default)]
struct IndexBuild {
    entries: HashMap<Box<[u8]>, Resolved>,
    dirs: HashMap<Box<[u8]>, Vec<DirEntryInfo>>,
    /// Folded child names already claimed in each directory, across all layers.
    /// Separate from `dirs` because it is dropped once the build finishes.
    claimed: HashMap<Box<[u8]>, HashSet<Vec<u8>>>,
}

#[derive(Debug, Clone)]
enum Resolved {
    /// Exactly one layer entry folds to this key at this priority.
    One(PathBuf),
    /// Two entries in ONE directory differ only by case, so which one wins
    /// depends on how the caller spelled it - something a folded map cannot
    /// represent. The walk can, so this key defers to it.
    Ambiguous,
}

/// Give up rather than index a tree this deep: the only way to reach it is a
/// symlink cycle, and a cycle would otherwise be walked until memory ran out.
const MAX_INDEX_DEPTH: usize = 64;
/// Give up rather than hold an index this large. A real setup measured 7,083
/// entries; this is three orders of magnitude of headroom before the fallback,
/// which is still correct, takes over.
const MAX_INDEX_ENTRIES: usize = 4_000_000;

impl LowerIndex {
    fn build(layers: &[PathBuf]) -> Option<std::sync::Arc<LowerIndex>> {
        if std::env::var("EIDOS_NO_INDEX").is_ok_and(|v| v != "0") {
            return None;
        }
        let mut build = IndexBuild::default();
        // Highest priority first, and `or_insert` keeps the first writer - the
        // same winner `layers.iter().find_map(..)` picks.
        for layer in layers {
            walk_layer(layer, &mut Vec::new(), 0, &mut build)?;
        }
        let dirs = build
            .dirs
            .into_iter()
            .map(|(k, v)| (k, v.into_boxed_slice()))
            .collect();
        Some(std::sync::Arc::new(LowerIndex {
            entries: build.entries,
            dirs,
        }))
    }

    /// The merged lower-layer listing of a directory.
    ///
    /// `None` means no layer provides that directory - which, from a complete
    /// index, is a real answer and not a shrug: the caller adds nothing rather
    /// than falling back to a walk that would find nothing either.
    fn children(&self, folded: &[u8]) -> Option<&[DirEntryInfo]> {
        self.dirs.get(folded).map(|v| &**v)
    }

    /// The real path for `vpath`, or `None` when no layer provides it.
    /// `Err(())` means the index cannot answer and the caller must walk.
    fn get(&self, folded: &[u8]) -> Result<Option<&PathBuf>, ()> {
        match self.entries.get(folded) {
            Some(Resolved::One(p)) => Ok(Some(p)),
            Some(Resolved::Ambiguous) => Err(()),
            None => Ok(None),
        }
    }
}

/// One layer's subtree, added to `entries` under folded virtual paths.
///
/// Returns `None` on ANY doubt - an unreadable directory, a name that is not
/// UTF-8, a tree too deep or too large. The caller discards the whole index.
fn walk_layer(dir: &Path, rel: &mut Vec<u8>, depth: usize, build: &mut IndexBuild) -> Option<()> {
    if depth > MAX_INDEX_DEPTH || build.entries.len() > MAX_INDEX_ENTRIES {
        return None;
    }
    // `rel` is this directory's own folded path until the loop appends a child
    // to it, so capture it once: it is the key every entry below files under.
    let parent: Box<[u8]> = rel.as_slice().into();
    // An unreadable directory is not "an empty directory": the walk cannot know
    // what is inside, so it cannot claim the paths under it do not exist.
    let read = fs::read_dir(dir).ok()?;
    // Names that fold to the same key inside THIS directory: the exact-case
    // preference in `ci_lookup` makes the winner depend on the query, so the
    // key is handed back to the walk instead of guessed at.
    let mut seen_here: HashSet<Vec<u8>> = HashSet::new();
    for entry in read {
        let entry = entry.ok()?;
        let name = entry.file_name();
        // Non-UTF-8 names compare by raw bytes in `eq_ignore_case` while UTF-8
        // ones ASCII-fold, so one folded key cannot represent both without
        // risking a collision between two genuinely different names. They do not
        // occur in practice; when they do, the walk handles them and this does
        // not have to.
        let text = name.to_str()?;
        let folded_name = text.to_ascii_lowercase().into_bytes();

        let mark = rel.len();
        if !rel.is_empty() {
            rel.push(b'/');
        }
        rel.extend_from_slice(&folded_name);

        let ambiguous = !seen_here.insert(folded_name.clone());
        let key: Box<[u8]> = rel.as_slice().into();
        let real = entry.path();
        if ambiguous {
            build.entries.insert(key, Resolved::Ambiguous);
        } else {
            build
                .entries
                .entry(key)
                .or_insert_with(|| Resolved::One(real.clone()));
        }

        // The merged listing. A hidden name is skipped BEFORE claiming its slot,
        // so the copy it was shadowing in a lower layer takes the name instead.
        if !is_hidden_name(text)
            && build
                .claimed
                .entry(parent.clone())
                .or_default()
                .insert(folded_name)
        {
            build.dirs.entry(parent.clone()).or_default().push((
                text.to_string(),
                real.clone(),
                entry.file_type().ok(),
            ));
        }

        // `is_dir` FOLLOWS symlinks, exactly as `ci_lookup`'s `exists` and
        // `read_dir` do, so the index sees what the walk sees. A symlink cycle
        // is caught by the depth cap, which abandons the whole index.
        //
        // Asked once and reused: it is a syscall, and this runs per entry over
        // every layer.
        if real.is_dir() {
            walk_layer(&real, rel, depth + 1, build)?;
        }
        rel.truncate(mark);
    }
    Some(())
}

/// Fold a virtual path the way `eq_ignore_case` compares its components.
fn fold_vpath(vpath: &str) -> Vec<u8> {
    let mut key: Vec<u8> = Vec::with_capacity(vpath.len());
    for comp in normalize(vpath).components() {
        let Some(text) = comp.as_os_str().to_str() else {
            return Vec::new();
        };
        if !key.is_empty() {
            key.push(b'/');
        }
        key.extend_from_slice(text.to_ascii_lowercase().as_bytes());
    }
    key
}

/// Number of mutation-lock shards. Small: contention only matters when two
/// threads touch the SAME path, which is rare, and false sharing between
/// unrelated paths costs only a brief wait.
const PATH_LOCK_SHARDS: usize = 64;

impl LayerStack {
    /// Build a stack from mod layers (highest priority first) and the writable
    /// overwrite layer.
    pub fn new(layers: Vec<PathBuf>, overwrite: PathBuf) -> Self {
        let path_locks: std::sync::Arc<[std::sync::Mutex<()>]> = (0..PATH_LOCK_SHARDS)
            .map(|_| std::sync::Mutex::new(()))
            .collect();
        // Built here, synchronously, so "is the index ready" is never a question
        // any caller can ask. `None` is a complete answer: it means every query
        // walks the layers exactly as it did before this existed.
        let lower = LowerIndex::build(&layers);
        Self {
            layers,
            overwrite,
            path_locks,
            resolve: Default::default(),
            lower,
            ow: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
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
        self.path_locks[shard]
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// The overwrite-layer path where a whiteout marker for `vpath` is written.
    /// The stored casing is whatever the caller used; lookups fold case via
    /// [`Self::find_whiteout`], so it does not matter.
    fn whiteout_path(&self, vpath: &str) -> PathBuf {
        let norm = normalize(vpath);
        let name = norm
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parent = norm
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        // Resolve the parent case-insensitively so the marker lands in the same
        // real directory the file resolves to (not a second case-variant dir).
        self.resolve_write(&parent)
            .join(format!("{WHITEOUT_PREFIX}{name}"))
    }

    /// Find an existing whiteout marker for `vpath`, matching the final path
    /// component case-insensitively so the check agrees with [`Self::list_dir`]
    /// (which also folds case). A case-sensitive check here was a real bug:
    /// deleting `FOO` then probing `foo` missed the marker and resurfaced the
    /// lower-layer file.
    fn find_whiteout(&self, vpath: &str) -> Option<PathBuf> {
        let norm = normalize(vpath);
        let name = norm.file_name()?.to_string_lossy().to_ascii_lowercase();
        let parent = norm
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parent_dir = self.ci_lookup(&self.overwrite, &parent)?;
        let want = format!("{WHITEOUT_PREFIX}{name}");
        fs::read_dir(&parent_dir).ok()?.flatten().find_map(|e| {
            (e.file_name().to_string_lossy().to_ascii_lowercase() == want).then(|| e.path())
        })
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
    ///
    /// ONE top-down descent answers both questions this used to ask separately.
    /// `hidden_by_whiteout` and `under_opaque_dir` each restarted from the
    /// overwrite root for every prefix of the path, so between them they walked
    /// it 2d-1 times to answer a question about d levels - quadratic work, and
    /// each of those walks paid `ci_lookup`'s enumeration whenever a component's
    /// spelling did not match. Descending once is linear and reads each level
    /// exactly once.
    fn overwrite_hides(&self, vpath: &str) -> bool {
        let norm = normalize(vpath);
        let comps: Vec<_> = norm.components().collect();
        let mut dir = self.overwrite.clone();
        for (i, comp) in comps.iter().enumerate() {
            let want = comp.as_os_str().to_string_lossy().to_ascii_lowercase();
            let marker = format!("{WHITEOUT_PREFIX}{want}");
            // ONE enumeration answers both questions at this level: is this
            // component whited out, and which real entry do we descend into.
            //
            // It has to be an enumeration rather than a probe, because a marker
            // is WRITTEN with the original case (`whiteout_path`) and MATCHED
            // without it (`find_whiteout`): `.eidoswh.Foo.esp` must answer a
            // query for `foo.esp`, so there is no single name to probe for.
            let Ok(entries) = fs::read_dir(&dir) else {
                // The overwrite does not go this deep, so it hides nothing here
                // and cannot hide anything below. Also the unreadable-directory
                // case, where both original checks likewise concluded "no".
                return false;
            };
            let mut child = None;
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_ascii_lowercase();
                if name == marker {
                    return true;
                }
                if name == want {
                    child = Some(e.path());
                }
            }
            let Some(next) = child else { return false };
            dir = next;
            // Opacity applies to proper ANCESTORS only: a directory deleted and
            // re-created hides what lies beneath it, but the leaf itself being
            // opaque says nothing about the leaf.
            if i + 1 < comps.len() && dir.join(OPAQUE_MARKER).exists() {
                return true;
            }
        }
        false
    }

    /// The overwrite index to read from, building it if it is dirty. `None`
    /// when the index is disabled (`EIDOS_NO_OW_INDEX`), which sends every
    /// query down the walk - the code path that is never wrong.
    fn ow_snapshot(&self) -> Option<std::sync::Arc<OwIndex>> {
        if !*OW_INDEX_ON {
            return None;
        }
        let mut slot = self.ow.lock().unwrap_or_else(|p| p.into_inner());
        if slot.is_none() {
            *slot = Some(std::sync::Arc::new(OwIndex::build(&self.overwrite)));
        }
        slot.clone()
    }

    /// Drop the overwrite index: something changed in a way not worth modelling
    /// precisely (a rename, a delete, a raw prepare). The next resolve rebuilds
    /// from disk, which is always right and cheap on a tree this small.
    fn ow_dirty(&self) {
        *self.ow.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }

    /// Note a CREATION in the overwrite: `vpath` now exists at `real`. The hot
    /// mutation by far - a shader cache compiling writes hundreds of these in a
    /// burst - so it updates the index in place instead of dropping it: the
    /// whole chain of new ancestors is inserted, and any whiteout on the path
    /// is forgotten (writing un-deletes).
    fn ow_note_created(&self, vpath: &str, real: &Path) {
        let mut slot = self.ow.lock().unwrap_or_else(|p| p.into_inner());
        let Some(arc) = slot.as_mut() else { return };
        let idx = std::sync::Arc::make_mut(arc);
        let norm = normalize(vpath);
        let comps: Vec<String> = norm
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        // Real ancestors, aligned with the vpath components: real is the leaf,
        // its parent the leaf's parent, and so on up to the overwrite root.
        let mut reals: Vec<PathBuf> = Vec::with_capacity(comps.len());
        let mut cur = real.to_path_buf();
        for _ in 0..comps.len() {
            reals.push(cur.clone());
            cur = match cur.parent() {
                Some(p) => p.to_path_buf(),
                None => break,
            };
        }
        reals.reverse();
        let mut prefix = String::new();
        for (i, comp) in comps.iter().enumerate() {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(comp);
            let key = fold_vpath(&prefix);
            if let Some(r) = reals.get(i) {
                if !idx.ambiguous.contains(&key) {
                    idx.entries.insert(key.clone(), r.clone());
                }
            }
            // Writing un-deletes, exactly as clear_whiteout did on disk - and the
            // marker file's own entry goes with it.
            //
            // THE LEAF ONLY. This used to run for every component, so creating
            // one file under a deleted directory lifted that directory's
            // whiteout and every sibling deleted with it came back: remove
            // "Data", create "Data/new.ini", and "Data/old.esp" was visible
            // again - with the index on, and only with it on. On disk the
            // directory is re-established opaque, which keeps the lower contents
            // hidden; the index modelled the un-delete but not the opacity, so it
            // disagreed with the walk in the one direction that matters. Pinned
            // by `a_create_must_not_resurrect_a_file_deleted_with_its_directory`.
            if i + 1 == comps.len() && idx.wh.remove(&key) {
                if let (Some(parent), Some(name)) =
                    (normalize(&prefix).parent(), normalize(&prefix).file_name())
                {
                    let marker = if parent.as_os_str().is_empty() {
                        format!("{WHITEOUT_PREFIX}{}", name.to_string_lossy())
                    } else {
                        format!(
                            "{}/{WHITEOUT_PREFIX}{}",
                            parent.to_string_lossy(),
                            name.to_string_lossy()
                        )
                    };
                    idx.entries.remove(&fold_vpath(&marker));
                }
            }
        }
    }

    /// Resolve a virtual path to the real file that should serve reads.
    ///
    /// The overwrite layer shadows everything (a file the game wrote or changed
    /// is what it should read back); a whiteout marker hides any lower copy;
    /// otherwise each mod layer is tried in priority order, falling through to
    /// the game data layer last. Returns `None` if nothing provides the path.
    /// What resolution has cost this mount so far.
    pub fn resolve_stats(&self) -> &ResolveStats {
        &self.resolve
    }

    /// Walk `vpath` under `root`, folding case, and return the real path.
    ///
    /// A method rather than a free function so the counters belong to the mount
    /// doing the work. The shape is the cost: one `exists` per component, and a
    /// FULL directory read whenever that misses - which is not the rare case. A
    /// layer that does not have the file misses its very first component and
    /// pays an enumeration to be sure the name is not merely spelled otherwise.
    fn ci_lookup(&self, root: &Path, vpath: &str) -> Option<PathBuf> {
        let comps: Vec<_> = normalize(vpath)
            .components()
            .map(|c| c.as_os_str().to_owned())
            .collect();
        self.ci_descend(root.to_path_buf(), &comps)
    }

    /// EVERY real directory in `root` that a virtual directory path folds onto.
    ///
    /// Normally one. But a layer holding both `meshes/` and `Meshes/` presents
    /// ONE directory in the merged view, whose contents are the union of the
    /// two, so a listing has to read both. [`LayerStack::ci_lookup`] answers
    /// "which path wins", which is the right question for a resolve and the
    /// wrong one here: it would silently drop everything in the variant it did
    /// not pick.
    fn ci_lookup_all(&self, root: &Path, vpath: &str) -> Vec<PathBuf> {
        let comps: Vec<_> = normalize(vpath)
            .components()
            .map(|c| c.as_os_str().to_owned())
            .collect();
        let mut out = Vec::new();
        self.ci_descend_all(root.to_path_buf(), &comps, &mut out);
        out
    }

    fn ci_descend_all(
        &self,
        current: PathBuf,
        rest: &[std::ffi::OsString],
        out: &mut Vec<PathBuf>,
    ) {
        use std::sync::atomic::Ordering::Relaxed;
        let Some((want, tail)) = rest.split_first() else {
            if current.is_dir() {
                out.push(current);
            }
            return;
        };
        self.resolve.scans.fetch_add(1, Relaxed);
        let Ok(entries) = fs::read_dir(&current) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            if eq_ignore_case(entry.file_name().as_os_str(), want) {
                self.ci_descend_all(entry.path(), tail, out);
            }
        }
    }

    /// The body of [`LayerStack::ci_lookup`], one component at a time, WITH
    /// BACKTRACKING.
    ///
    /// Taking the exact-case match and committing to it is wrong, and wrong in a
    /// way that hides files rather than reporting anything. A layer may hold two
    /// directories differing only in case - ext4 keeps them apart, the merged view
    /// does not - and real mods do: XP32 Maximum Skeleton ships both `meshes/`
    /// and `Meshes/`, with its animations and its FNIS behaviour file under the
    /// capitalised one. A greedy walk enters `meshes/`, fails to find the rest of
    /// the path there, and abandons the whole LAYER, so every file under
    /// `Meshes/` is invisible to the game. No error, no log: the mod is simply
    /// not applied.
    ///
    /// So a component that matches is a candidate, not a decision. Exact case is
    /// tried first because it is the common answer and costs one `exists`; only
    /// when the remainder fails underneath it does the scan look for siblings
    /// that fold to the same name.
    fn ci_descend(&self, current: PathBuf, rest: &[std::ffi::OsString]) -> Option<PathBuf> {
        use std::sync::atomic::Ordering::Relaxed;
        let Some((want, tail)) = rest.split_first() else {
            return Some(current);
        };

        let exact = current.join(want);
        self.resolve.probes.fetch_add(1, Relaxed);
        let exact_exists = exact.exists();
        if exact_exists {
            if let Some(found) = self.ci_descend(exact, tail) {
                return Some(found);
            }
        }

        // Either no exact match, or one whose subtree does not hold the rest.
        // Fold-equal siblings are the remaining candidates; the exact one is
        // skipped because it has already been tried.
        self.resolve.scans.fetch_add(1, Relaxed);
        for entry in fs::read_dir(&current).ok()?.filter_map(Result::ok) {
            let name = entry.file_name();
            if !eq_ignore_case(name.as_os_str(), want) {
                continue;
            }
            if exact_exists && name == *want {
                continue;
            }
            if let Some(found) = self.ci_descend(entry.path(), tail) {
                return Some(found);
            }
        }
        None
    }

    pub fn resolve_read(&self, vpath: &str) -> Option<PathBuf> {
        if !*TIMING_ON {
            return self.resolve_read_inner(vpath);
        }
        let t = std::time::Instant::now();
        let r = self.resolve_read_inner(vpath);
        self.resolve.ns.fetch_add(
            t.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
        r
    }

    fn resolve_read_inner(&self, vpath: &str) -> Option<PathBuf> {
        // A hidden file (or anything under a hidden directory) is not part of the
        // virtual view at all: `list_dir` never emits it, and refusing to resolve
        // it here means a path guessed or remembered by the game cannot reach it
        // either. The mod keeps the bytes; the game simply cannot see them.
        if under_hidden(vpath) {
            return None;
        }
        if let Some(ow) = self.ow_snapshot() {
            let folded = fold_vpath(vpath);
            // The root itself. The walk resolves it to the overwrite root - a
            // directory that always exists and is never an entry OF the index,
            // since the index maps the overwrite's children, not the overwrite.
            // Without this the mount's own root reads as a negative and readdir
            // of `/` returns ENOENT, which the real-mount suite catches in
            // seconds.
            if folded.is_empty() {
                return self.resolve_read_walk(vpath);
            }
            if ow.ambiguous.contains(&folded) {
                // Case-colliding overwrite entries: only the walk's backtracking
                // can pick correctly. Rare to the point of theoretical - our own
                // write path reuses existing casing - but refusing is free.
                return self.resolve_read_walk(vpath);
            }
            if let Some(real) = ow.entries.get(&folded) {
                // The belt: an entry the disk no longer backs means a mutation
                // slipped past the doors. Serve THIS query through the walk
                // (always right) and rebuild for the next one - a stale index
                // must cost one slow query, never a wrong answer.
                // `symlink_metadata`, not `exists()`: the walk returns dangling
                // symlinks too, and the index must not disagree with it.
                if fs::symlink_metadata(real).is_ok() {
                    return Some(real.clone());
                }
                self.ow_dirty();
                return self.resolve_read_walk(vpath);
            }
            // The overwrite provably has nothing at this path: go straight to
            // the layers, skipping the walk that used to cost every resolve its
            // probes and scans.
            let lower = match self.lower.as_ref().map(|i| i.get(&folded)) {
                Some(Ok(Some(path))) => {
                    self.resolve
                        .idx_hits
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    path.clone()
                }
                Some(Ok(None)) => {
                    self.resolve
                        .idx_negatives
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return None;
                }
                verdict @ (Some(Err(())) | None) => {
                    match verdict {
                        Some(Err(())) => self
                            .resolve
                            .idx_fallbacks
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        _ => self
                            .resolve
                            .idx_absent
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    };
                    self.layers
                        .iter()
                        .find_map(|layer| self.ci_lookup(layer, vpath))?
                }
            };
            if ow.hides(&folded) {
                return None;
            }
            return Some(lower);
        }
        self.resolve_read_walk(vpath)
    }

    /// The pre-index body of [`Self::resolve_read_inner`], kept whole: it is the
    /// path that is never wrong, the fallback for ambiguous keys and stale
    /// entries, and the reference the equivalence tests compare the index
    /// against.
    fn resolve_read_walk(&self, vpath: &str) -> Option<PathBuf> {
        use std::sync::atomic::Ordering::Relaxed;
        let (p0, s0) = (
            self.resolve.probes.load(Relaxed),
            self.resolve.scans.load(Relaxed),
        );
        let ow = self.ci_lookup(&self.overwrite, vpath);
        self.resolve
            .ow_probes
            .fetch_add(self.resolve.probes.load(Relaxed).wrapping_sub(p0), Relaxed);
        self.resolve
            .ow_scans
            .fetch_add(self.resolve.scans.load(Relaxed).wrapping_sub(s0), Relaxed);
        if let Some(p) = ow {
            return Some(p);
        }
        // Ask the layers BEFORE asking what the overwrite hides.
        //
        // Provably the same function: if nothing below provides the path the
        // answer is `None` whatever the whiteouts say, and if something does,
        // the hide check still gets the last word. What changes is the cost of
        // the commonest case by far - Wine probes vastly more paths than exist,
        // and each of those probes used to pay a full hide check before finding
        // out there was nothing to hide.
        // The index answers from memory when it can, and hands the question back
        // when it cannot - an ambiguous key, or no index at all. Both fall to the
        // walk, which is the code this replaces and is never wrong.
        let lower = match self.lower.as_ref().map(|i| i.get(&fold_vpath(vpath))) {
            Some(Ok(Some(path))) => {
                self.resolve.idx_hits.fetch_add(1, Relaxed);
                path.clone()
            }
            // A complete index saying "nothing has it" is as good as the walk
            // saying so, and this is the commonest answer by far: Wine probes
            // far more paths than exist.
            Some(Ok(None)) => {
                self.resolve.idx_negatives.fetch_add(1, Relaxed);
                return None;
            }
            verdict @ (Some(Err(())) | None) => {
                match verdict {
                    Some(Err(())) => self.resolve.idx_fallbacks.fetch_add(1, Relaxed),
                    _ => self.resolve.idx_absent.fetch_add(1, Relaxed),
                };
                let (p0, s0) = (
                    self.resolve.probes.load(Relaxed),
                    self.resolve.scans.load(Relaxed),
                );
                let found = self
                    .layers
                    .iter()
                    .find_map(|layer| self.ci_lookup(layer, vpath));
                self.resolve
                    .walk_probes
                    .fetch_add(self.resolve.probes.load(Relaxed).wrapping_sub(p0), Relaxed);
                self.resolve
                    .walk_scans
                    .fetch_add(self.resolve.scans.load(Relaxed).wrapping_sub(s0), Relaxed);
                found?
            }
        };
        if self.overwrite_hides(vpath) {
            return None;
        }
        Some(lower)
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
            let matched = fs::read_dir(&current).ok().and_then(|rd| {
                rd.flatten()
                    .find(|e| eq_ignore_case(e.file_name().as_os_str(), want))
            });
            current = matched.map(|e| e.path()).unwrap_or(exact);
        }
        current
    }

    /// Whether a write to `vpath` needs a copy-up first: it exists in a lower
    /// layer but not yet in the overwrite layer.
    pub fn needs_copy_up(&self, vpath: &str) -> bool {
        self.ci_lookup(&self.overwrite, vpath).is_none()
            && self
                .layers
                .iter()
                .any(|l| self.ci_lookup(l, vpath).is_some())
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
            if let Some(src) = self.layers.iter().find_map(|l| self.ci_lookup(l, vpath)) {
                // A DIRECTORY that lives only in a lower layer is materialised,
                // not skipped. It used to fall through both arms: nothing was
                // created, yet the path was recorded in the overwrite index as if
                // it had been. Two costs followed - the caller got a path that
                // does not exist, so `ops.rs`'s `set_permissions` right after this
                // returned ENOENT (a mode-only setattr is exactly how Wine clears
                // FILE_ATTRIBUTE_READONLY on a directory), and the index was told
                // about a path that was not there, which the next resolve had to
                // undo with a full rebuild.
                // Classified WITHOUT following links. `src.is_dir()` follows, so
                // a lower symlink-to-directory - which the install path itself
                // creates via `overlay_dir` - was materialised as a REAL empty
                // directory in the overwrite, and `getattr` switched from
                // reporting S_IFLNK to S_IFDIR: a type change visible to the
                // game, triggered by something as small as Wine clearing the
                // read-only attribute. A symlink is declined instead: recreating
                // the link here would be worse, because a RELATIVE target would
                // re-resolve against the overwrite's location and point somewhere
                // else entirely. (A symlink-to-FILE still copies up by content
                // below, exactly as it always did: `is_file` follows too, and a
                // content copy preserves what the vpath serves.)
                let real_dir = src.is_dir()
                    && fs::symlink_metadata(&src).is_ok_and(|m| !m.file_type().is_symlink());
                if real_dir {
                    fs::create_dir_all(&dest)?;
                } else if src.is_file() {
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
        } else if self
            .layers
            .iter()
            .any(|l| self.ci_lookup(l, vpath).is_some())
        {
            // A destination that already exists AND is shadowing a lower layer is
            // an orphaned copy-up from an earlier run, which may carry that run's
            // 0444. Heal it - but only in that case: a file living solely in the
            // Overwrite is the user's own, and a read-only mode they set on it
            // (the classic "stop the launcher rewriting my INI") must survive.
            ensure_owner_writable(&dest);
        }
        // Only what EXISTS is recorded. The declined branches above (a lower
        // symlink, a lower entry that vanished mid-call) leave nothing at `dest`,
        // and telling the index about a path that is not there was the original
        // half of this bug - every later resolve paid a full rebuild to undo it.
        if fs::symlink_metadata(&dest).is_ok() {
            self.ow_note_created(vpath, &dest);
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
        let needs_opacity = was_deleted
            && self
                .layers
                .iter()
                .any(|l| self.ci_lookup(l, vpath).is_some_and(|p| p.is_dir()));

        // ORDER IS THE WHOLE POINT: opacity goes down while the whiteout is still
        // standing, so a failure here leaves the delete intact.
        //
        // Doing it the other way round - clear, then write, then propagate the
        // error - looks like the careful version and is worse than swallowing the
        // error was. By the time the write is attempted, `clear_whiteout` has
        // already removed the only thing hiding the deleted lower contents, and
        // returning early also skips `ow_dirty()`, so an ENOSPC or EROFS on the
        // overwrite volume left every deleted file visible AND the index
        // disagreeing with the walk for the rest of the mount. This way the
        // failure path changes nothing at all: the directory exists, the whiteout
        // still hides what it hid, and the caller gets the errno.
        if needs_opacity {
            fs::write(dest.join(OPAQUE_MARKER), b"")?;
        }
        self.clear_whiteout(vpath);
        if needs_opacity {
            // Opacity changes what an ancestor HIDES, not just what exists:
            // model it by rebuild rather than by surgery.
            self.ow_dirty();
        } else {
            self.ow_note_created(vpath, &dest);
        }
        Ok(dest)
    }

    /// Prepare `vpath` for a fresh write in the overwrite layer: a symlink or
    /// other special entry the daemon creates directly. Materialises the parent
    /// directories and drops any whiteout, returning the real overwrite path.
    /// Unlike [`Self::open_for_write`] it does NOT copy a lower file up, because
    /// the caller is replacing the entry, not editing it; the new overwrite entry
    /// shadows any lower-layer copy.
    /// Create `vpath` in the overwrite as an empty file, and hand back a
    /// read-write handle on it.
    ///
    /// Exists so that `eidos-fuse` does not. Three handlers used to make a name
    /// appear in the overwrite with their own `OpenOptions`, which meant this
    /// layer did not know about every name it owns - and anything that has to be
    /// told about every change (a cache, an index, an audit) had three doors to
    /// watch instead of one, with no way to notice a fourth being added.
    ///
    /// Deliberately truncating and deliberately NOT a copy-up: the caller is
    /// creating or emptying a file, so resurrecting the bytes of a lower-layer
    /// file it is about to overwrite would be wrong.
    pub fn create_truncated(&self, vpath: &str) -> std::io::Result<(PathBuf, fs::File)> {
        let dest = self.prepare_overwrite_inner(vpath)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&dest)?;
        // In place, not a rebuild: this is the hot door - a shader cache
        // compiling writes hundreds of these in a burst.
        self.ow_note_created(vpath, &dest);
        Ok((dest, file))
    }

    /// Create `vpath` in the overwrite as a symlink to `target`, replacing any
    /// entry already there.
    pub fn create_symlink(&self, vpath: &str, target: &Path) -> std::io::Result<PathBuf> {
        let dest = self.prepare_overwrite_inner(vpath)?;
        // An existing entry is replaced rather than failing EEXIST: the overwrite
        // is where a re-created path lands, and `symlink` has already been told
        // this name is theirs.
        let _ = fs::remove_file(&dest);
        std::os::unix::fs::symlink(target, &dest)?;
        self.ow_note_created(vpath, &dest);
        Ok(dest)
    }

    /// Set `vpath`'s length, copying it up first if it lives only in a lower
    /// layer. Creates the file when the copy-up left nothing (a truncate of
    /// something that resolves but has no overwrite copy).
    pub fn set_len(&self, vpath: &str, size: u64) -> std::io::Result<PathBuf> {
        let dest = self.open_for_write(vpath)?;
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&dest)?;
        file.set_len(size)?;
        Ok(dest)
    }

    pub fn prepare_overwrite(&self, vpath: &str) -> std::io::Result<PathBuf> {
        let dest = self.prepare_overwrite_inner(vpath)?;
        // The caller creates the entry itself AFTER this returns, through a door
        // this layer cannot see - so the index cannot be updated in place, only
        // scheduled for a rebuild that will read whatever they made.
        self.ow_dirty();
        Ok(dest)
    }

    fn prepare_overwrite_inner(&self, vpath: &str) -> std::io::Result<PathBuf> {
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
        if dest.exists()
            && self
                .layers
                .iter()
                .any(|l| self.ci_lookup(l, vpath).is_some())
        {
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
        if self
            .layers
            .iter()
            .any(|l| self.ci_lookup(l, vpath).is_some())
        {
            let wh = self.whiteout_path(vpath);
            if let Some(parent) = wh.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(wh, b"")?;
        }
        self.ow_dirty();
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
        if self
            .layers
            .iter()
            .any(|l| self.ci_lookup(l, from).is_some())
        {
            let wh = self.whiteout_path(from);
            if let Some(parent) = wh.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(wh, b"")?;
        }
        self.ow_dirty();
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
        self.list_dir_typed(vpath)
            .into_iter()
            .map(|(name, real, _)| (name, real))
            .collect()
    }

    /// [`LayerStack::list_dir`] plus each entry's file type AS THE DIRECTORY
    /// ENTRY REPORTED IT.
    ///
    /// The type comes from `readdir`'s own `d_type` field, which costs nothing -
    /// it is already in the bytes the kernel handed back. `DirEntry::file_type`
    /// only falls back to an `lstat` on the filesystems that answer `DT_UNKNOWN`,
    /// and it does not follow symlinks, so the result is identical to calling
    /// `symlink_metadata` on every entry and enormously cheaper. `None` means
    /// even the fallback failed: the entry is being removed under us, and the
    /// caller must decide, because there is no honest type to report.
    pub fn list_dir_typed(&self, vpath: &str) -> Vec<(String, PathBuf, Option<fs::FileType>)> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut whiteouts: HashSet<String> = HashSet::new();
        let mut out: Vec<(String, PathBuf, Option<fs::FileType>)> = Vec::new();

        // Overwrite layer first: collect whiteouts (and hide the markers).
        let mut opaque = false;
        if let Some(dir) = self
            .ci_lookup(&self.overwrite, vpath)
            .filter(|d| d.is_dir())
        {
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
                        out.push((name, entry.path(), entry.file_type().ok()));
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
        //
        // The index has already merged them - in priority order, deduplicated,
        // hidden names dropped - so this is one hash lookup instead of one
        // case-folding walk per layer. Whiteouts stay here: they live in the
        // overwrite, which changes, and the index only ever knew the layers.
        match self.lower.as_ref().map(|i| i.children(&fold_vpath(vpath))) {
            Some(merged) => {
                // `None` from a complete index means no layer has this directory,
                // which is an answer: add nothing.
                for (name, real, kind) in merged.unwrap_or(&[]) {
                    let key = name.to_ascii_lowercase();
                    if whiteouts.contains(&key) {
                        continue;
                    }
                    if seen.insert(key) {
                        out.push((name.clone(), real.clone(), *kind));
                    }
                }
            }
            // No index: the walk this replaces, which is never wrong.
            None => {
                // Every fold-equal directory, not just the winner: a layer with
                // both `meshes/` and `Meshes/` shows ONE directory here, holding
                // the union of the two.
                for dir in self
                    .layers
                    .iter()
                    .flat_map(|l| self.ci_lookup_all(l, vpath))
                {
                    let Ok(entries) = fs::read_dir(&dir) else {
                        continue;
                    };
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        let key = name.to_ascii_lowercase();
                        if whiteouts.contains(&key) {
                            continue;
                        }
                        // A user-hidden entry is dropped without claiming its
                        // name, so a lower layer's copy of the file it shadowed
                        // becomes the winner - the point of hiding one mod's
                        // stray override.
                        if is_hidden_name(&name) {
                            continue;
                        }
                        if seen.insert(key) {
                            out.push((name, entry.path(), entry.file_type().ok()));
                        }
                    }
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
        out.sort_by_cached_key(|(name, _, _)| ntfs_order_key(name));
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
    vpath
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect()
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
/// How much filesystem work path resolution is doing, for the FUSE layer to
/// report. Free when nobody reads them: two relaxed increments on a path that
/// already performs a syscall.
///
/// They exist because the handler timings could say resolution was slow without
/// saying WHY, and the two candidates want opposite fixes: many cheap `exists`
/// calls means too many layers are being asked, while a few `read_dir` scans
/// means case folding is falling back to enumerating whole directories.
#[derive(Debug, Default)]
pub struct ResolveStats {
    /// `exists()` calls: one per path component per layer tried.
    pub probes: AtomicU64,
    /// Full directory reads, taken when a component's spelling did not match
    /// byte-for-byte. Orders of magnitude dearer than a probe.
    pub scans: AtomicU64,
    /// Nanoseconds spent inside `resolve_read`. Timed here rather than at the
    /// FUSE call sites because there are a dozen of those and one of them would
    /// eventually be added without its stopwatch.
    pub ns: AtomicU64,
    // ---- attribution: where the probes and scans above actually go ----------
    /// Probes/scans spent on the OVERWRITE pre-check. Measured as deltas around
    /// that phase, so under concurrency they are approximate (another thread's
    /// probes can land in the window); in a single-threaded bench they are
    /// exact, and the bench is what they exist for.
    pub ow_probes: AtomicU64,
    pub ow_scans: AtomicU64,
    /// Probes/scans spent walking layers because the index could not answer.
    /// Same delta caveat as above.
    pub walk_probes: AtomicU64,
    pub walk_scans: AtomicU64,
    /// Index verdicts, incremented directly: exact everywhere.
    pub idx_hits: AtomicU64,
    pub idx_negatives: AtomicU64,
    /// The index existed but could not answer (ambiguous key): fell to the walk.
    pub idx_fallbacks: AtomicU64,
    /// No index at all (build declined or disabled): every query walks.
    pub idx_absent: AtomicU64,
}

/// What the overwrite holds, as three folded-key sets, so the per-resolve
/// question "does the overwrite affect this path?" is a hashmap look instead of
/// a case-folding directory walk.
///
/// The overwrite is mutable - the game writes there - which is why the layer
/// index never covered it. But every mutation flows through this type's own
/// methods (`create_truncated`'s doc calls them the doors), so the index can be
/// kept honest: creations insert, everything else drops it for a lazy rebuild.
/// The rebuild walks a tree that is tiny in practice: a session writes INIs and
/// shader caches, not mod trees.
#[derive(Debug, Default, Clone)]
struct OwIndex {
    /// Folded vpath -> real path, files, dirs and symlinks alike. Byte keys,
    /// the same shape `fold_vpath` gives the layer index.
    entries: HashMap<Vec<u8>, PathBuf>,
    /// Folded keys where two overwrite entries collide case-insensitively. The
    /// index refuses these and the walk (with its backtracking) answers, the
    /// same contract as the layer index's ambiguous keys.
    ambiguous: HashSet<Vec<u8>>,
    /// Folded vpaths hidden by a whiteout marker: the name itself and below.
    wh: HashSet<Vec<u8>>,
    /// Folded vpaths of opaque directories: lower content STRICTLY below is
    /// hidden (the directory itself exists in the overwrite and resolves).
    opaque: HashSet<Vec<u8>>,
}

impl OwIndex {
    fn build(root: &Path) -> OwIndex {
        fn walk(dir: &Path, vprefix: &str, out: &mut OwIndex) {
            let Ok(rd) = fs::read_dir(dir) else { return };
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let vpath = if vprefix.is_empty() {
                    name.clone()
                } else {
                    format!("{vprefix}/{name}")
                };
                if name == OPAQUE_MARKER {
                    out.opaque.insert(fold_vpath(vprefix));
                    continue;
                }
                if let Some(target) = name.strip_prefix(WHITEOUT_PREFIX) {
                    let tv = if vprefix.is_empty() {
                        target.to_string()
                    } else {
                        format!("{vprefix}/{target}")
                    };
                    out.wh.insert(fold_vpath(&tv));
                    // The marker file itself is an overwrite entry too; recording
                    // it keeps the map a faithful mirror of the walk.
                }
                let key = fold_vpath(&vpath);
                let real = e.path();
                match out.entries.entry(key.clone()) {
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(real.clone());
                    }
                    std::collections::hash_map::Entry::Occupied(_) => {
                        out.ambiguous.insert(key);
                    }
                }
                if real.is_dir() {
                    walk(&real, &vpath, out);
                }
            }
        }
        let mut out = OwIndex::default();
        walk(root, "", &mut out);
        out
    }

    /// Whether a LOWER result at `vpath` is hidden - the indexed twin of
    /// [`LayerStack::overwrite_hides`]: a whiteout hides the name and below, an
    /// opaque directory hides proper descendants only.
    fn hides(&self, folded: &[u8]) -> bool {
        if self.wh.is_empty() && self.opaque.is_empty() {
            return false;
        }
        let mut from = 0usize;
        loop {
            let end = folded[from..]
                .iter()
                .position(|&b| b == b'/')
                .map(|i| from + i);
            let prefix = &folded[..end.unwrap_or(folded.len())];
            if self.wh.contains(prefix) {
                return true;
            }
            match end {
                // A proper ancestor: opacity applies.
                Some(_) if self.opaque.contains(prefix) => return true,
                Some(e) => from = e + 1,
                // The leaf: opacity on the leaf itself says nothing about it.
                None => return false,
            }
        }
    }
}

/// Whether the overwrite membership index is on. `EIDOS_NO_OW_INDEX=1` turns it
/// off, the same escape hatch shape as `EIDOS_NO_INDEX` for the layer index:
/// when a stale-view bug is suspected, disabling the suspect is one env var.
static OW_INDEX_ON: std::sync::LazyLock<bool> =
    std::sync::LazyLock::new(|| !std::env::var("EIDOS_NO_OW_INDEX").is_ok_and(|v| v != "0"));

/// Whether to time resolution. Same switch as the FUSE stats, read once: the
/// counters above are two relaxed adds on a path that already syscalls, but a
/// clock read twice per resolve is worth gating.
static TIMING_ON: std::sync::LazyLock<bool> =
    std::sync::LazyLock::new(|| std::env::var("EIDOS_FUSE_STATS").is_ok_and(|v| v != "0"));

/// Case-insensitive name comparison.
///
/// v1 folds ASCII case only; Windows folds a wider Unicode table. Most game and
/// mod paths are ASCII; broadening this is tracked in `docs/internals/architecture.md`.
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
    let Ok(meta) = fs::symlink_metadata(path) else {
        return;
    };
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
    libc::timespec {
        tv_sec: d.as_secs() as libc::time_t,
        tv_nsec: d.subsec_nanos() as libc::c_long,
    }
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
    let got = unsafe {
        libc::llistxattr(
            csrc.as_ptr(),
            names.as_mut_ptr() as *mut libc::c_char,
            names.len(),
        )
    };
    if got <= 0 {
        return;
    }
    names.truncate(got as usize);
    for name in names.split(|&b| b == 0).filter(|s| s.starts_with(b"user.")) {
        let Ok(cname) = std::ffi::CString::new(name) else {
            continue;
        };
        let vlen =
            unsafe { libc::lgetxattr(csrc.as_ptr(), cname.as_ptr(), std::ptr::null_mut(), 0) };
        if vlen < 0 {
            continue;
        }
        let mut val = vec![0u8; vlen as usize];
        let vgot = unsafe {
            libc::lgetxattr(
                csrc.as_ptr(),
                cname.as_ptr(),
                val.as_mut_ptr() as *mut libc::c_void,
                val.len(),
            )
        };
        if vgot < 0 {
            continue;
        }
        unsafe {
            libc::lsetxattr(
                cdst.as_ptr(),
                cname.as_ptr(),
                val.as_ptr() as *const libc::c_void,
                vgot as usize,
                0,
            )
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
        assert_eq!(
            secs, 1_400_000_000,
            "copy-up must keep the lower file's mtime, not stamp 'now'"
        );
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
        assert_eq!(
            fs::metadata(&dest).unwrap().permissions().mode() & 0o200,
            0o200
        );
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

        let mode = fs::symlink_metadata(game.join("pristine.esp"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o777,
            0o444,
            "the game file's mode was changed through a symlink"
        );
    }

    #[test]
    fn a_read_only_file_of_our_own_keeps_its_mode() {
        use std::os::unix::fs::PermissionsExt;
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        // Lives ONLY in the overwrite layer: the user's own file, which they set
        // read-only on purpose (the classic "stop the launcher rewriting my INI").
        put(&over, "SkyrimPrefs.ini", "mine");
        fs::set_permissions(
            over.join("SkyrimPrefs.ini"),
            fs::Permissions::from_mode(0o444),
        )
        .unwrap();
        let stack = LayerStack::new(vec![game], over.clone());

        let _ = stack.open_for_write("SkyrimPrefs.ini");
        let mode = fs::metadata(over.join("SkyrimPrefs.ini"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o200,
            0,
            "a user-set read-only mode must not be silently undone"
        );
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
        assert_eq!(
            read(&stack.resolve_read("meshes.dat").unwrap()),
            "vanilla mesh"
        );
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
        assert_eq!(
            read(&stack.resolve_read("textures/armor.dds").unwrap()),
            "tex"
        );
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
        assert!(normalize("../x/..")
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_))));
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
        put(
            &modd,
            "textures/rock.dds.mohidden",
            "the override we suppressed",
        );
        put(&modd, "textures/tree.dds", "still active");

        let stack = LayerStack::new(vec![modd, game], over);
        let mut names: Vec<String> = stack
            .list_dir("textures")
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        names.sort();
        // The suffixed name never appears, and rock.dds is the vanilla file again.
        assert_eq!(names, vec!["rock.dds", "tree.dds"]);
        assert_eq!(
            read(&stack.resolve_read("textures/rock.dds").unwrap()),
            "vanilla"
        );
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
        assert!(stack
            .resolve_read("meshes.MOHIDDEN/actors/body.nif")
            .is_none());
        assert!(stack.resolve_read("meshes/actors/head.nif").is_some());
    }

    #[test]
    fn a_multibyte_name_does_not_panic_the_resolver() {
        // Byte 9-from-the-end lands INSIDE the first CJK character here, which is
        // what a `&name[len - 9..]` slice panics on. Real mod folders look like
        // this: the pool this was found in had `至真女性皮肤4K-Zhizhen's female
        // skin 4K`. A panic here kills the mount in the middle of a read.
        for n in [
            "abcdefg日日日h",
            "至真女性皮肤4K.bsa",
            "Épées Légendaires.esp",
            "日本語",
        ] {
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
    fn list_dir_typed_reports_the_real_kind_of_every_entry() {
        // The FUSE readdir reply is built from these types. A directory reported
        // as a regular file is a lie the caller acts on: it will not descend into
        // it, and the Creation Engine answers that by restarting its enumeration
        // forever. So the type must come from the directory entry, per layer, and
        // survive the merge.
        let t = TempTree::new();
        let (game, modd, over) = (t.sub("game"), t.sub("mod"), t.sub("over"));
        put(&game, "Scripts/a.pex", "x"); // a DIRECTORY in the game layer
        put(&game, "plain.esm", "x");
        put(&modd, "meshes/b.nif", "y"); // a DIRECTORY in a mod layer
        std::os::unix::fs::symlink(game.join("plain.esm"), modd.join("link.esm")).unwrap();

        let stack = LayerStack::new(vec![modd, game], over);
        let got: Vec<(String, bool, bool)> = stack
            .list_dir_typed("")
            .into_iter()
            .map(|(n, _, ft)| {
                let ft = ft.expect("every entry here exists, so its type is knowable");
                (n, ft.is_dir(), ft.is_symlink())
            })
            .collect();

        let kind = |name: &str| {
            got.iter()
                .find(|(n, _, _)| n == name)
                .map(|(_, d, l)| (*d, *l))
        };
        assert_eq!(kind("Scripts"), Some((true, false)), "{got:?}");
        assert_eq!(kind("meshes"), Some((true, false)), "{got:?}");
        assert_eq!(kind("plain.esm"), Some((false, false)), "{got:?}");
        // Not followed: a symlink is reported as a symlink, matching the
        // `symlink_metadata` semantics this replaced.
        assert_eq!(kind("link.esm"), Some((false, true)), "{got:?}");
        // And the plain listing still agrees, entry for entry.
        let plain: Vec<String> = stack.list_dir("").into_iter().map(|(n, _)| n).collect();
        assert_eq!(
            plain,
            got.iter().map(|(n, _, _)| n.clone()).collect::<Vec<_>>()
        );
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
        assert_eq!(
            read(&stack.resolve_read("config/settings.ini").unwrap()),
            "tweaked"
        );
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
        assert_eq!(
            variants.len(),
            1,
            "exactly one overwrite copy, got {variants:?}"
        );

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
        assert_eq!(
            read(&stack.resolve_read("tools_bak/sub/b.txt").unwrap()),
            "b"
        );
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
        let mut names: Vec<String> = stack
            .list_dir("data2")
            .into_iter()
            .map(|(n, _)| n)
            .collect();
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
        assert!(
            stack.resolve_read("cache/a.txt").is_none(),
            "a.txt resurrected"
        );
        assert!(
            stack.resolve_read("cache/b.txt").is_none(),
            "b.txt resurrected"
        );
        let names: Vec<String> = stack
            .list_dir("cache")
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert!(
            names.is_empty(),
            "recreated dir should be empty, got {names:?}"
        );
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

    #[test]
    fn the_index_answers_without_walking_the_layers() {
        // What this whole change is for. Before the index a resolve cost one
        // `exists` per component per layer, plus a full directory read for every
        // one that missed - 95 probes and 37 enumerations on a real-sized stack.
        // Now the layers are not touched at all.
        let t = TempTree::new();
        let (over, a, b) = (t.sub("ow"), t.sub("a"), t.sub("b"));
        put(&b, "textures/actors/skin.dds", "x");
        fs::create_dir_all(a.join("textures/actors")).unwrap();
        fs::create_dir_all(&over).unwrap();
        let stack = LayerStack::new(vec![a, b.clone()], over);

        let st = stack.resolve_stats();
        let (p0, s0) = (
            st.probes.load(Ordering::Relaxed),
            st.scans.load(Ordering::Relaxed),
        );
        assert_eq!(
            stack.resolve_read("textures/actors/skin.dds"),
            Some(b.join("textures/actors/skin.dds"))
        );
        let (probes, scans) = (
            st.probes.load(Ordering::Relaxed) - p0,
            st.scans.load(Ordering::Relaxed) - s0,
        );
        // ZERO of both. The layers answer from the lower index, and the
        // overwrite - once "deliberately never indexed and still read live", at
        // one probe and one scan per resolve - now answers from its membership
        // index. This assertion is the whole point of that index: the cost of a
        // resolve no longer contains a single directory operation.
        assert_eq!(
            probes, 0,
            "nothing may be probed on a fully indexed resolve, got {probes}"
        );
        assert_eq!(scans, 0, "and nothing may enumerate, got {scans}");
    }

    #[test]
    fn the_mount_root_still_resolves_with_the_index_on() {
        // The regression the real-mount suite caught: the empty vpath is the
        // mount root, the walk answers it with the overwrite root, and the
        // index - which maps the overwrite's children, not the overwrite -
        // read it as a negative. readdir("/") then returned ENOENT.
        let t = TempTree::new();
        let (over, l) = (t.sub("ow"), t.sub("l"));
        put(&l, "a.dat", "x");
        fs::create_dir_all(&over).unwrap();
        let stack = LayerStack::new(vec![l], over.clone());
        assert_eq!(stack.resolve_read(""), Some(over));
        assert_eq!(stack.resolve_read(""), stack.resolve_read_walk(""));
    }

    #[test]
    fn the_overwrite_index_agrees_with_the_walk_on_the_whole_matrix() {
        // The walk is the reference implementation - the code that is never
        // wrong. The index must give the same answer on every shape the
        // overwrite can take, or it has no business existing.
        let t = TempTree::new();
        let (over, l) = (t.sub("ow"), t.sub("l"));
        put(&l, "Data/lower.esp", "L");
        put(&l, "Data/Deleted.esp", "D");
        put(&l, "Gone/inside.txt", "G");
        put(&l, "Recreated/old.txt", "O");
        put(&over, "Data/mine.ini", "M");
        put(&over, "Nested/Deep/file.txt", "N");
        fs::create_dir_all(&over).unwrap();
        let stack = LayerStack::new(vec![l], over);

        // Whiteout a file, delete a whole dir, and recreate another (opaque).
        stack.remove("Data/Deleted.esp").unwrap();
        stack.remove("Gone").unwrap();
        stack.remove("Recreated").unwrap();
        stack.make_dir("Recreated").unwrap();

        // And the shape that once slipped past this very matrix: create a file
        // UNDER a deleted directory, with the index warm. The un-whiteout used to
        // run for every ancestor instead of the leaf alone, so the walk hid
        // Gone/inside.txt while the index resurrected it - and this test was
        // green, because no query created anything between the deletes and the
        // comparisons. The oracle is only as strong as the operations it covers.
        let _ = stack.resolve_read("Gone/probe"); // warm
        let _ = stack.create_truncated("Gone/new.ini").unwrap();

        for q in [
            "Gone/new.ini",
            "Data/mine.ini",
            "DATA/MINE.INI",
            "Data/lower.esp",
            "data/LOWER.esp",
            "Data/Deleted.esp",
            "data/deleted.ESP",
            "Gone/inside.txt",
            "Recreated",
            "Recreated/old.txt",
            "Nested/Deep/file.txt",
            "nested/deep/FILE.txt",
            "Data/absent.esp",
            "nothing/at/all.bsa",
        ] {
            assert_eq!(
                stack.resolve_read(q),
                stack.resolve_read_walk(q),
                "index and walk disagree on {q}"
            );
        }
    }

    #[test]
    fn every_mutation_door_keeps_the_index_honest() {
        // Each door either updates the index in place or drops it; either way
        // the very next resolve must see the truth WITHOUT anyone rebuilding by
        // hand. A door that forgets its hook makes this fail.
        let t = TempTree::new();
        let (over, l) = (t.sub("ow"), t.sub("l"));
        put(&l, "Data/lower.esp", "L");
        fs::create_dir_all(&over).unwrap();
        let stack = LayerStack::new(vec![l], over);

        // Warm the index first, so a missing hook cannot hide behind a rebuild.
        assert!(stack.resolve_read("Data/lower.esp").is_some());

        let (_, mut f) = stack.create_truncated("Data/new.ini").unwrap();
        use std::io::Write as _;
        writeln!(f, "x").unwrap();
        assert!(
            stack.resolve_read("data/NEW.ini").is_some(),
            "create_truncated was invisible"
        );

        stack.make_dir("Fresh/Sub").unwrap();
        assert!(
            stack.resolve_read("fresh/sub").is_some(),
            "make_dir was invisible"
        );

        stack
            .create_symlink("Data/link.esp", Path::new("/nonexistent"))
            .unwrap();
        assert!(
            stack.resolve_read("data/link.ESP").is_some(),
            "create_symlink was invisible"
        );

        let dest = stack.open_for_write("Data/lower.esp").unwrap();
        assert_eq!(
            stack.resolve_read("DATA/lower.esp"),
            Some(dest),
            "copy-up was invisible"
        );

        stack.remove("Data/lower.esp").unwrap();
        assert_eq!(
            stack.resolve_read("Data/lower.esp"),
            None,
            "remove was invisible"
        );

        stack.rename("Data/new.ini", "Data/renamed.ini").unwrap();
        assert!(
            stack.resolve_read("Data/renamed.ini").is_some(),
            "rename target invisible"
        );
        assert_eq!(
            stack.resolve_read("Data/new.ini"),
            None,
            "rename source survived"
        );
    }

    #[test]
    fn a_creation_burst_updates_in_place_without_rebuilds() {
        // The Community Shaders case: hundreds of files created mid-session.
        // Each must be resolvable immediately, and the index must be updated
        // rather than dropped - asserted by the resolve cost staying at zero
        // directory operations right after each create.
        let t = TempTree::new();
        let (over, l) = (t.sub("ow"), t.sub("l"));
        put(&l, "Data/base.esp", "B");
        fs::create_dir_all(&over).unwrap();
        let stack = LayerStack::new(vec![l], over);
        assert!(stack.resolve_read("Data/base.esp").is_some()); // warm

        let st = stack.resolve_stats();
        for i in 0..50 {
            let v = format!("ShaderCache/entry{i}.bin");
            stack.create_truncated(&v).unwrap();
            let (p0, s0) = (
                st.probes.load(Ordering::Relaxed),
                st.scans.load(Ordering::Relaxed),
            );
            assert!(
                stack.resolve_read(&v.to_ascii_uppercase()).is_some(),
                "{v} not visible"
            );
            assert_eq!(
                st.probes.load(Ordering::Relaxed) - p0,
                0,
                "a resolve paid a probe after {v}"
            );
            assert_eq!(
                st.scans.load(Ordering::Relaxed) - s0,
                0,
                "a resolve paid a scan after {v}"
            );
        }
    }

    #[test]
    fn the_index_folds_case_exactly_as_the_walk_did() {
        // The reason the walk was slow was case folding, so an index that
        // dropped it would be fast and useless: Bethesda games ask for
        // `ccbgssse001-fish.bsa` while the file is `ccBGSSSE001-Fish.bsa`.
        let t = TempTree::new();
        let (over, l) = (t.sub("ow"), t.sub("l"));
        put(&l, "Textures/Skin.DDS", "x");
        fs::create_dir_all(&over).unwrap();
        let stack = LayerStack::new(vec![l.clone()], over);

        for spelling in [
            "textures/skin.dds",
            "TEXTURES/SKIN.DDS",
            "Textures/Skin.DDS",
        ] {
            assert_eq!(
                stack.resolve_read(spelling),
                Some(l.join("Textures/Skin.DDS")),
                "{spelling} must reach the same file"
            );
        }
        assert_eq!(stack.resolve_read("textures/absent.dds"), None);
    }

    #[test]
    fn two_names_differing_only_by_case_defer_to_the_walk() {
        // A folded map holds one key, but `ci_lookup` tries the exact spelling
        // first - so with `Skin.dds` and `SKIN.DDS` side by side the winner
        // depends on how the caller asked. The index cannot represent that, so
        // it declines and the walk answers, exactly as before.
        let t = TempTree::new();
        let (over, l) = (t.sub("ow"), t.sub("l"));
        put(&l, "t/Skin.dds", "one");
        put(&l, "t/SKIN.DDS", "two");
        fs::create_dir_all(&over).unwrap();
        let stack = LayerStack::new(vec![l.clone()], over);

        // Each exact spelling still finds its own file - the property the index
        // would have broken by picking a winner at build time.
        assert_eq!(
            fs::read_to_string(stack.resolve_read("t/Skin.dds").unwrap()).unwrap(),
            "one"
        );
        assert_eq!(
            fs::read_to_string(stack.resolve_read("t/SKIN.DDS").unwrap()).unwrap(),
            "two"
        );
        // And it cost a walk, which is the point: declining is the safe answer.
        let before = stack.resolve_stats().probes.load(Ordering::Relaxed);
        let _ = stack.resolve_read("t/skin.dds");
        assert!(
            stack.resolve_stats().probes.load(Ordering::Relaxed) - before > 1,
            "an ambiguous key must fall back to the layer walk"
        );
    }

    #[test]
    fn no_index_means_the_old_behaviour_and_the_same_answers() {
        // The escape hatch has to produce identical results, or it is not an
        // escape hatch, it is a second implementation.
        let t = TempTree::new();
        let (over, a, b) = (t.sub("ow"), t.sub("a"), t.sub("b"));
        put(&a, "shared.esp", "high");
        put(&b, "shared.esp", "low");
        put(&b, "only-low.esp", "x");
        fs::create_dir_all(&over).unwrap();

        let indexed = LayerStack::new(vec![a.clone(), b.clone()], over.clone());
        // SAFETY: single-threaded here, and the variable is read once per
        // LayerStack::new. Restored immediately below.
        unsafe { std::env::set_var("EIDOS_NO_INDEX", "1") };
        let walked = LayerStack::new(vec![a.clone(), b.clone()], over);
        unsafe { std::env::remove_var("EIDOS_NO_INDEX") };

        for q in [
            "shared.esp",
            "SHARED.ESP",
            "only-low.esp",
            "absent.esp",
            "a/b/c.esp",
        ] {
            assert_eq!(
                indexed.resolve_read(q),
                walked.resolve_read(q),
                "disagreed on {q}"
            );
        }
        // And priority is the layer order, not the disk order.
        assert_eq!(
            indexed.resolve_read("shared.esp"),
            Some(a.join("shared.esp"))
        );
    }

    // ---- a layer holding two spellings of one directory ----------------------
    //
    // ext4 keeps `meshes/` and `Meshes/` apart; the merged view must not. Real
    // mods ship both - XP32 Maximum Skeleton has its animations and its FNIS
    // behaviour file under the capitalised one - and the greedy walk entered the
    // lowercase one, failed to find the rest of the path, and abandoned the whole
    // LAYER. Every file under the other spelling was invisible, with no error
    // anywhere: the mod was simply not applied.

    fn case_variant_layer() -> (TempTree, LayerStack) {
        let dir = TempTree::new();
        let layer = dir.sub("layer");
        // The lowercase spelling exists and is what an exact-case probe finds
        // first - but the file lives under the capitalised one.
        fs::create_dir_all(layer.join("meshes/actors")).unwrap();
        fs::write(layer.join("meshes/actors/decoy.nif"), b"decoy").unwrap();
        fs::create_dir_all(layer.join("Meshes/actors")).unwrap();
        fs::write(layer.join("Meshes/actors/real.nif"), b"real").unwrap();
        let over = dir.sub("over");
        fs::create_dir_all(&over).unwrap();
        let stack = LayerStack::new(vec![layer], over);
        (dir, stack)
    }

    #[test]
    fn a_file_under_the_other_spelling_is_still_found() {
        let (_d, stack) = case_variant_layer();
        let got = stack.resolve_read("meshes/actors/real.nif");
        assert!(
            got.is_some(),
            "the greedy walk lost the whole Meshes/ subtree"
        );
        assert_eq!(fs::read(got.unwrap()).unwrap(), b"real");
        // And the one that IS under the exact-case spelling still resolves.
        assert!(stack.resolve_read("meshes/actors/decoy.nif").is_some());
    }

    #[test]
    fn both_spellings_merge_into_one_listing() {
        let (_d, stack) = case_variant_layer();
        let names: Vec<String> = stack
            .list_dir("meshes/actors")
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert!(
            names.iter().any(|n| n == "real.nif"),
            "missing from listing: {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "decoy.nif"),
            "missing from listing: {names:?}"
        );
        assert_eq!(names.len(), 2, "{names:?}");
    }

    #[test]
    fn the_walk_agrees_with_the_index_on_case_variants() {
        // The fallback is documented as the answer that is never wrong, so it has
        // to survive this too - it was the half that was wrong.
        let (_d, indexed) = case_variant_layer();
        let dir = TempTree::new();
        let layer = dir.sub("layer");
        fs::create_dir_all(layer.join("meshes/actors")).unwrap();
        fs::write(layer.join("meshes/actors/decoy.nif"), b"decoy").unwrap();
        fs::create_dir_all(layer.join("Meshes/actors")).unwrap();
        fs::write(layer.join("Meshes/actors/real.nif"), b"real").unwrap();
        let over = dir.sub("over");
        fs::create_dir_all(&over).unwrap();
        // SAFETY: single-threaded test, read once per LayerStack::new.
        unsafe { std::env::set_var("EIDOS_NO_INDEX", "1") };
        let walked = LayerStack::new(vec![layer], over);
        unsafe { std::env::remove_var("EIDOS_NO_INDEX") };

        for p in [
            "meshes/actors/real.nif",
            "meshes/actors/decoy.nif",
            "MESHES/ACTORS/REAL.NIF",
        ] {
            assert_eq!(
                indexed.resolve_read(p).is_some(),
                walked.resolve_read(p).is_some(),
                "index and walk disagree on {p}"
            );
        }
        let mut a: Vec<String> = indexed
            .list_dir("meshes/actors")
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        let mut b: Vec<String> = walked
            .list_dir("meshes/actors")
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        a.sort();
        b.sort();
        assert_eq!(a, b, "index and walk disagree on the listing");
    }

    /// A lower symlink-to-directory must never be materialised as a REAL
    /// directory: that is a type change the game can see (S_IFLNK becomes
    /// S_IFDIR), reachable from something as small as Wine clearing the
    /// read-only attribute - and the install path itself ships symlinks.
    #[test]
    fn open_for_write_never_turns_a_lower_symlink_into_a_real_directory() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "real/inner.txt", "content");
        std::os::unix::fs::symlink(game.join("real"), game.join("link")).unwrap();
        let stack = LayerStack::new(vec![game.clone()], over.clone());

        // A mode-only setattr arrives as open_for_write of the path itself.
        let dest = stack.open_for_write("link").unwrap();
        assert!(
            fs::symlink_metadata(&dest).is_err(),
            "a symlink must be declined, not replaced by {:?}",
            fs::symlink_metadata(&dest).map(|m| m.file_type())
        );
        // And the union still serves the symlink from the lower layer.
        let served = stack.resolve_read("link").expect("still resolvable");
        assert!(
            fs::symlink_metadata(&served)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the served entry must still BE the symlink"
        );
    }

    /// A mkdir that cannot establish opacity must change NOTHING.
    ///
    /// The failure path is the whole test: if the whiteout is cleared before the
    /// opaque marker is written, an ENOSPC or EROFS on the overwrite volume
    /// resurrects every file deleted under that directory - and skips the index
    /// invalidation on the way out, so the index and the walk then disagree for
    /// the rest of the mount.
    #[test]
    fn a_mkdir_that_cannot_establish_opacity_leaves_the_delete_standing() {
        use std::os::unix::fs::PermissionsExt;
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "Data/old.esp", "lower");
        let stack = LayerStack::new(vec![game.clone()], over.clone());

        stack.remove("Data").unwrap();
        assert!(
            stack.resolve_read("Data/old.esp").is_none(),
            "hidden by the delete"
        );
        let _ = stack.resolve_read("Data/probe"); // warm the index

        // Make the overwrite's Data directory unwritable so the marker cannot be
        // written, which is what a full or read-only overwrite volume looks like.
        let d = over.join("Data");
        fs::create_dir_all(&d).unwrap();
        fs::set_permissions(&d, fs::Permissions::from_mode(0o555)).unwrap();

        let failed = stack.make_dir("Data").is_err();
        fs::set_permissions(&d, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(failed, "the mkdir must report the failure it hit");

        assert!(
            stack.resolve_read("Data/old.esp").is_none(),
            "RESURRECTED by a failed mkdir: the delete must survive an error"
        );
        assert!(
            !stack.list_dir("Data").iter().any(|(n, _)| n == "old.esp"),
            "the listing must not show it either: {:?}",
            stack
                .list_dir("Data")
                .iter()
                .map(|(n, _)| n)
                .collect::<Vec<_>>()
        );
    }

    /// A directory that exists only in a lower layer must be MATERIALISED when
    /// it is opened for write, not merely recorded. It used to return a path
    /// that did not exist while telling the index it did, so the caller's very
    /// next `set_permissions` failed ENOENT - which is exactly how Wine clearing
    /// the read-only attribute on a mod directory arrives.
    #[test]
    fn open_for_write_on_a_lower_only_directory_creates_it() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "meshes/actors/keep.nif", "lower");
        let stack = LayerStack::new(vec![game.clone()], over);

        let dest = stack.open_for_write("meshes/actors").unwrap();
        assert!(
            dest.exists(),
            "open_for_write handed back a path that does not exist"
        );
        assert!(
            dest.is_dir(),
            "a lower directory must copy up as a directory"
        );
        // And the lower file underneath it is still readable through the union.
        assert!(stack.resolve_read("meshes/actors/keep.nif").is_some());
    }

    /// The audit's claim, run rather than argued: a delete, one resolve to WARM
    /// the overwrite index, then a create under the deleted directory - and the
    /// deleted lower file is claimed to come back.
    #[test]
    fn a_create_must_not_resurrect_a_file_deleted_with_its_directory() {
        let t = TempTree::new();
        let (game, over) = (t.sub("game"), t.sub("over"));
        put(&game, "Data/old.esp", "lower");
        let stack = LayerStack::new(vec![game.clone()], over);

        stack.remove("Data").unwrap();
        assert!(
            stack.resolve_read("Data/old.esp").is_none(),
            "hidden right after the delete"
        );

        // Warming matters: a live daemon resolves thousands of times a second, so
        // the index is always built by the time a write arrives. And the write
        // must be a REAL creation: `create_truncated` makes the file exist, which
        // is what routes it through `ow_note_created` - an `open_for_write` of a
        // path with no lower source creates nothing, skips the note entirely
        // since the existence gate, and left a first version of this test green
        // against the very bug it was written for.
        let _ = stack.resolve_read("Data/anything");
        let _ = stack.create_truncated("Data/new.ini").unwrap();

        assert!(
            stack.resolve_read("Data/old.esp").is_none(),
            "RESURRECTED: creating a sibling re-exposed a file deleted with its directory"
        );
    }
}
