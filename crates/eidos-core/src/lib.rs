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
}

impl LayerStack {
    /// Build a stack from mod layers (highest priority first) and the writable
    /// overwrite layer.
    pub fn new(layers: Vec<PathBuf>, overwrite: PathBuf) -> Self {
        Self { layers, overwrite }
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
                }
            }
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
        let dest = self.resolve_write(vpath);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        self.clear_whiteout(vpath);
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

/// Strip a leading slash so a virtual path can be joined onto a real layer root.
fn normalize(vpath: &str) -> PathBuf {
    PathBuf::from(vpath.trim_start_matches('/'))
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
