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

/// The game's own data layer. Like MO2 (`doConflictCheck` treats a file whose
/// only alternative is the `data` origin as unconflicted), overriding base-game
/// files is NOT a conflict - conflict flags are mod-versus-mod only.
pub const BASE_ORIGIN: OriginId = 0;

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
    /// This path is contested between MODS (the base game does not count: an
    /// override of a vanilla file is normal modding, not a conflict - MO2
    /// behaves the same). The game still appears in `alternatives` so the Data
    /// view can show where a file ultimately comes from.
    pub fn is_conflicted(&self) -> bool {
        self.alternatives.iter().any(|&a| a != BASE_ORIGIN)
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
    /// Mods this one ranks above on a shared file (pairwise among all providers,
    /// not only the file's outright winner) - it overwrites them where they overlap.
    pub overwrites: BTreeSet<OriginId>,
    /// Mods that rank above this one on a shared file (pairwise) - they overwrite
    /// it where they overlap.
    pub overwritten_by: BTreeSet<OriginId>,
    /// Files this mod wins.
    pub won: usize,
    /// Files this mod provides (won + lost).
    pub total: usize,
    /// The mod has `*.mohidden` files (MO2's `hasHiddenFiles`): some content has
    /// been hidden from the view, surfaced as a distinct flag.
    pub has_hidden: bool,
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
        let mut layer_hidden: HashMap<OriginId, bool> = HashMap::new();
        for layer in layers {
            let (rels, hidden) = collect_files(&layer.root);
            if hidden {
                layer_hidden.insert(layer.origin, true);
            }
            for rel in rels {
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

        // Derive per-mod conflicts. MO2's `ModInfoWithConflictInfo` compares every
        // provider of a path against every *other* provider PAIRWISE, not just
        // against the winner, so a file provided by A>B>C records the full chain
        // (B overwrites C *and* is overwritten by A). Pairs involving the base
        // game are skipped (MO2 parity - beating vanilla files is not a conflict,
        // so a pure replacer mod stays flag-free).
        for node in files.values() {
            // Providers highest priority first: the winner then its ordered
            // (descending-priority) alternatives.
            let providers: Vec<OriginId> = std::iter::once(node.winner)
                .chain(node.alternatives.iter().copied())
                .collect();

            // The winner wins this file; every provider provides it (counted once).
            if let Some(w) = mods.get_mut(&node.winner) {
                w.won += 1;
            }
            for &p in &providers {
                if let Some(m) = mods.get_mut(&p) {
                    m.total += 1;
                }
            }

            // Each higher-priority provider overwrites every lower one, skipping
            // any pair that involves the base game.
            for (i, &higher) in providers.iter().enumerate() {
                for &lower in &providers[i + 1..] {
                    if higher == BASE_ORIGIN || lower == BASE_ORIGIN {
                        continue;
                    }
                    if let Some(h) = mods.get_mut(&higher) {
                        h.overwrites.insert(lower);
                    }
                    if let Some(l) = mods.get_mut(&lower) {
                        l.overwritten_by.insert(higher);
                    }
                }
            }
        }

        // State, in MO2's precedence (`doConflictCheck`): a mod that provides
        // visible files but wins NONE of them is Redundant *first* - even when it
        // pairwise outranks a lower-priority loser - so beating another loser never
        // promotes it to Mixed/Overwrites. The base game itself is never flagged.
        for (&origin, mc) in mods.iter_mut() {
            mc.has_hidden = layer_hidden.get(&origin).copied().unwrap_or(false);
            if origin == BASE_ORIGIN {
                mc.state = ConflictState::None;
                continue;
            }
            mc.state = if mc.total > 0 && mc.won == 0 {
                ConflictState::Redundant
            } else {
                match (!mc.overwrites.is_empty(), !mc.overwritten_by.is_empty()) {
                    (true, true) => ConflictState::Mixed,
                    (true, false) => ConflictState::Overwrites,
                    (false, true) => ConflictState::Overwritten,
                    (false, false) => ConflictState::None,
                }
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

/// Every file under `root`, as `/`-joined relative paths (recursive), with
/// MO2's structure exclusions (`DirectoryRefresher::cleanStructure` +
/// `s_HiddenExt`): root-level `meta.ini`/`readme.txt` and the root `fomod/`
/// dir are manager metadata, not mod content; `*.mohidden` files and
/// directories are hidden from the view and the conflict check alike.
/// Returns `(files, has_hidden)`: `has_hidden` is set when any `*.mohidden` entry
/// was skipped, mirroring MO2's `hasHiddenFiles` (a mod with hidden files gets its
/// own flag). The Overwrite layer's `.eidoswh*` whiteout/opacity markers are
/// manager artifacts and are skipped too, so they never appear as conflicting files.
fn collect_files(root: &Path) -> (Vec<String>, bool) {
    fn rec(base: &Path, dir: &Path, depth: usize, out: &mut Vec<String>, hidden: &mut bool) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            if name.starts_with(".eidoswh") {
                continue;
            }
            if name.ends_with(".mohidden") {
                *hidden = true;
                continue;
            }
            if p.is_dir() {
                if depth == 0 && name == "fomod" {
                    continue;
                }
                rec(base, &p, depth + 1, out, hidden);
            } else {
                if depth == 0 && (name == "meta.ini" || name == "readme.txt") {
                    continue;
                }
                if let Ok(rel) = p.strip_prefix(base) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut hidden = false;
    rec(root, root, 0, &mut out, &mut hidden);
    (out, hidden)
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
    fn three_providers_record_pairwise_relations() {
        let t = Tmp::new();
        // tri.dat is provided by A>B>C. MO2 records the FULL pairwise chain, not
        // just winner-vs-loser: B (which also wins bwin.dat, so it isn't fully
        // shadowed) loses tri.dat to A yet beats C on it; C loses to BOTH A and B.
        let layers = [
            t.layer(1, "A", &["tri.dat"]),
            t.layer(2, "B", &["tri.dat", "bwin.dat"]),
            t.layer(3, "C", &["tri.dat"]),
        ];
        let map = ConflictMap::build(&layers);

        let node = &map.files["tri.dat"];
        assert_eq!(node.winner, 1);
        assert_eq!(node.alternatives, vec![2, 3]);

        // B is Mixed: overwrites exactly {C}, overwritten_by exactly {A}.
        assert_eq!(map.state(2), ConflictState::Mixed);
        assert_eq!(map.mods[&2].overwrites, BTreeSet::from([3]));
        assert_eq!(map.mods[&2].overwritten_by, BTreeSet::from([1]));

        // C is overwritten by BOTH higher providers - the B>C relation is exactly
        // what a winner-centric pass would miss.
        assert_eq!(map.mods[&3].overwritten_by, BTreeSet::from([1, 2]));
        assert!(map.mods[&3].overwrites.is_empty());

        // A wins outright over both.
        assert_eq!(map.mods[&1].overwrites, BTreeSet::from([2, 3]));
        assert_eq!(map.state(1), ConflictState::Overwrites);
    }

    #[test]
    fn won_none_but_outranks_a_loser_stays_redundant() {
        let t = Tmp::new();
        // A>B>C all provide only the same file. B wins nothing (A is always above
        // it) yet pairwise outranks C. Redundant must take precedence over the
        // non-empty `overwrites` set - B must NOT become Mixed/Overwrites.
        let layers = [
            t.layer(1, "A", &["f.dat"]),
            t.layer(2, "B", &["f.dat"]),
            t.layer(3, "C", &["f.dat"]),
        ];
        let map = ConflictMap::build(&layers);

        assert_eq!(map.mods[&2].won, 0);
        assert!(map.mods[&2].overwrites.contains(&3)); // pairwise-beats the lower loser
        assert!(map.mods[&2].overwritten_by.contains(&1));
        assert_eq!(map.state(2), ConflictState::Redundant); // not Mixed

        // C wins nothing either -> also Redundant.
        assert_eq!(map.state(3), ConflictState::Redundant);
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
    fn base_game_overrides_are_not_conflicts() {
        let t = Tmp::new();
        // A only overrides a vanilla file: no flag (MO2 parity), but the game
        // stays visible as the alternative for the Data view.
        let layers = [
            t.layer(1, "A", &["textures/face.dds"]),
            t.layer(BASE_ORIGIN, "[game]", &["textures/face.dds", "skyrim.esm"]),
        ];
        let map = ConflictMap::build(&layers);
        assert_eq!(map.state(1), ConflictState::None);
        let node = &map.files["textures/face.dds"];
        assert!(!node.is_conflicted());
        assert_eq!(node.alternatives, vec![BASE_ORIGIN]);
    }

    #[test]
    fn manager_metadata_and_hidden_files_are_excluded() {
        let t = Tmp::new();
        // Every MO2 mod folder carries meta.ini; fomod/ and *.mohidden are
        // manager artifacts too. None of them may produce conflicts.
        let layers = [
            t.layer(1, "A", &[
                "meta.ini",
                "readme.txt",
                "fomod/ModuleConfig.xml",
                "textures/old.dds.mohidden",
                "meshes.mohidden/body.nif",
                "textures/real.dds",
            ]),
            t.layer(2, "B", &["meta.ini", "textures/real.dds"]),
        ];
        let map = ConflictMap::build(&layers);
        // Only the real content file is in the tree.
        assert_eq!(map.files.keys().collect::<Vec<_>>(), vec!["textures/real.dds"]);
        assert_eq!(map.state(1), ConflictState::Overwrites);
        assert_eq!(map.state(2), ConflictState::Redundant);
    }

    #[test]
    fn mohidden_sets_has_hidden_and_whiteouts_are_skipped() {
        let t = Tmp::new();
        let layers = [
            t.layer(1, "A", &[
                "textures/real.dds",
                "textures/old.dds.mohidden",  // hidden -> has_hidden, out of the tree
                ".eidoswh.deleted.esp",       // overwrite whiteout marker -> skipped
            ]),
            t.layer(2, "B", &["textures/real.dds"]),
        ];
        let map = ConflictMap::build(&layers);
        // Neither the whiteout marker nor the .mohidden file is a conflicting file.
        assert_eq!(map.files.keys().collect::<Vec<_>>(), vec!["textures/real.dds"]);
        // A carries hidden files (MO2's hasHiddenFiles flag); B does not.
        assert!(map.mods[&1].has_hidden);
        assert!(!map.mods[&2].has_hidden);
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
