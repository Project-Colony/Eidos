//! Per-file conflict analysis for an Eidos load order: which mod wins each path,
//! which mods lose it, and each mod's overall conflict state. This is the
//! day-to-day reason MO2 exists - across dozens of mods many files come from
//! several mods, and which one wins decides whether the game looks right.
//!
//! Pure and game-agnostic (no FUSE, no I/O beyond walking the layer trees) so the
//! GUI and a future `eidos conflicts` CLI can both consume it. Mirrors MO2's
//! `DirectoryRefresher` (winner + ordered alternatives per file) and
//! `ModInfoWithConflictInfo` (per-mod overwrite/overwritten sets + state), but
//! collapsed into a single pass since Eidos has no BSA tiebreak yet.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Identifies a layer: `0` is conventionally the game's own data; mods get any
/// stable non-zero ids. Priority is the layer ORDER passed to [`ConflictMap::build`],
/// highest first - not the numeric value.
pub type OriginId = u32;

/// One input layer: a mod folder (or the game data) with a stable id and name.
#[derive(Debug, Clone)]
pub struct Layer {
    pub origin: OriginId,
    pub name: String,
    pub root: PathBuf,
}

/// Who provides one virtual path: the winner plus the ordered losers.
#[derive(Debug, Clone)]
pub struct FileNode {
    pub winner: OriginId,
    /// Lower-priority providers, in descending priority order.
    pub alternatives: Vec<OriginId>,
    /// The winning layer's casing of the path, for display.
    pub display_path: String,
}

impl FileNode {
    /// This path is contested (at least one mod is overwritten here).
    pub fn is_conflicted(&self) -> bool {
        !self.alternatives.is_empty()
    }
}

/// A mod's overall conflict state, matching MO2's `EConflictType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictState {
    /// No file of this mod is contested.
    #[default]
    None,
    /// Wins at least one contested file, loses none.
    Overwrites,
    /// Loses at least one file, but still wins some of its own.
    Overwritten,
    /// Both overwrites others and is overwritten by others.
    Mixed,
    /// Provides visible files but wins none of them (fully shadowed).
    Redundant,
}

/// One mod's conflicts.
#[derive(Debug, Clone, Default)]
pub struct ModConflicts {
    /// Mods this one overwrites (it wins a file they also provide).
    pub overwrites: BTreeSet<OriginId>,
    /// Mods that overwrite this one (they win a file it also provides).
    pub overwritten_by: BTreeSet<OriginId>,
    /// Files this mod wins.
    pub won: usize,
    /// Files this mod provides (won + lost).
    pub total: usize,
    pub state: ConflictState,
}

/// The full analysis: the merged file tree plus per-mod conflicts.
#[derive(Debug, Clone, Default)]
pub struct ConflictMap {
    /// Lowercased relative path -> who provides it.
    pub files: BTreeMap<String, FileNode>,
    pub mods: HashMap<OriginId, ModConflicts>,
    pub names: HashMap<OriginId, String>,
}

impl ConflictMap {
    /// Analyse `layers`, given **highest priority first** (e.g. the mod list with
    /// the game data appended as origin 0). Walks each layer once.
    pub fn build(layers: &[Layer]) -> ConflictMap {
        let mut files: BTreeMap<String, FileNode> = BTreeMap::new();

        // Highest priority first: the first layer to provide a path wins it; the
        // rest become alternatives, accumulating in descending-priority order.
        for layer in layers {
            for rel in collect_files(&layer.root) {
                let key = rel.to_ascii_lowercase();
                files
                    .entry(key)
                    .and_modify(|n| n.alternatives.push(layer.origin))
                    .or_insert_with(|| FileNode {
                        winner: layer.origin,
                        alternatives: Vec::new(),
                        display_path: rel.clone(),
                    });
            }
        }

        let mut mods: HashMap<OriginId, ModConflicts> = HashMap::new();
        for layer in layers {
            mods.entry(layer.origin).or_default();
        }

        for node in files.values() {
            if let Some(w) = mods.get_mut(&node.winner) {
                w.won += 1;
                w.total += 1;
                for &alt in &node.alternatives {
                    w.overwrites.insert(alt);
                }
            }
            for &alt in &node.alternatives {
                if let Some(a) = mods.get_mut(&alt) {
                    a.overwritten_by.insert(node.winner);
                    a.total += 1;
                }
            }
        }

        for mc in mods.values_mut() {
            mc.state = match (!mc.overwrites.is_empty(), !mc.overwritten_by.is_empty()) {
                (true, true) => ConflictState::Mixed,
                (true, false) => ConflictState::Overwrites,
                (false, true) => {
                    if mc.won == 0 {
                        ConflictState::Redundant
                    } else {
                        ConflictState::Overwritten
                    }
                }
                (false, false) => ConflictState::None,
            };
        }

        let names = layers.iter().map(|l| (l.origin, l.name.clone())).collect();
        ConflictMap { files, mods, names }
    }

    /// The name of an origin (or `?` if unknown).
    pub fn name(&self, origin: OriginId) -> &str {
        self.names.get(&origin).map(String::as_str).unwrap_or("?")
    }

    /// The conflict state of a mod (None if it has no files).
    pub fn state(&self, origin: OriginId) -> ConflictState {
        self.mods.get(&origin).map(|m| m.state).unwrap_or(ConflictState::None)
    }
}

/// Every file under `root`, as `/`-joined relative paths (recursive).
fn collect_files(root: &Path) -> Vec<String> {
    fn rec(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                rec(base, &p, out);
            } else if let Ok(rel) = p.strip_prefix(base) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    rec(root, root, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);

    struct Tmp(PathBuf);
    impl Tmp {
        fn new() -> Self {
            let n = N.fetch_add(1, Ordering::Relaxed);
            let d = std::env::temp_dir().join(format!("eidos-conf-{}-{}", std::process::id(), n));
            fs::create_dir_all(&d).unwrap();
            Tmp(d)
        }
        fn layer(&self, origin: OriginId, name: &str, files: &[&str]) -> Layer {
            let root = self.0.join(name);
            for f in files {
                let p = root.join(f);
                fs::create_dir_all(p.parent().unwrap()).unwrap();
                fs::write(p, b"x").unwrap();
            }
            Layer { origin, name: name.to_string(), root }
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn winner_is_highest_priority_rest_are_ordered_alternatives() {
        let t = Tmp::new();
        // Highest first: A wins shared.dat over B then C.
        let layers = [
            t.layer(1, "A", &["shared.dat"]),
            t.layer(2, "B", &["shared.dat"]),
            t.layer(3, "C", &["shared.dat"]),
        ];
        let map = ConflictMap::build(&layers);
        let node = &map.files["shared.dat"];
        assert_eq!(node.winner, 1);
        assert_eq!(node.alternatives, vec![2, 3]); // descending priority
        assert!(node.is_conflicted());
    }

    #[test]
    fn fully_shadowed_mod_is_redundant() {
        let t = Tmp::new();
        // B's only file is won by A -> B is redundant; A overwrites.
        let layers = [
            t.layer(1, "A", &["shared.dat"]),
            t.layer(2, "B", &["shared.dat"]),
        ];
        let map = ConflictMap::build(&layers);
        assert_eq!(map.state(1), ConflictState::Overwrites);
        assert_eq!(map.state(2), ConflictState::Redundant);
        assert!(map.mods[&1].overwrites.contains(&2));
        assert!(map.mods[&2].overwritten_by.contains(&1));
    }

    #[test]
    fn wins_one_loses_one_is_mixed() {
        let t = Tmp::new();
        // B wins low.dat (over C) but loses high.dat (to A) -> Mixed.
        let layers = [
            t.layer(1, "A", &["high.dat"]),
            t.layer(2, "B", &["high.dat", "low.dat"]),
            t.layer(3, "C", &["low.dat"]),
        ];
        let map = ConflictMap::build(&layers);
        assert_eq!(map.state(2), ConflictState::Mixed);
        assert!(map.mods[&2].overwrites.contains(&3)); // B beats C on low.dat
        assert!(map.mods[&2].overwritten_by.contains(&1)); // A beats B on high.dat
    }

    #[test]
    fn uncontested_mod_has_no_conflict() {
        let t = Tmp::new();
        let layers = [t.layer(1, "A", &["a.dat"]), t.layer(2, "B", &["b.dat"])];
        let map = ConflictMap::build(&layers);
        assert_eq!(map.state(1), ConflictState::None);
        assert_eq!(map.state(2), ConflictState::None);
        assert!(!map.files["a.dat"].is_conflicted());
    }

    #[test]
    fn case_insensitive_paths_collide() {
        let t = Tmp::new();
        let layers = [
            t.layer(1, "A", &["Meshes/Armor.nif"]),
            t.layer(2, "B", &["meshes/armor.nif"]),
        ];
        let map = ConflictMap::build(&layers);
        // One node, A wins, B is the alternative.
        assert_eq!(map.files.len(), 1);
        let node = map.files.values().next().unwrap();
        assert_eq!(node.winner, 1);
        assert_eq!(node.alternatives, vec![2]);
    }
}
