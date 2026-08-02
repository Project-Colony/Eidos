//! Mod installer for Eidos, porting Mod Organizer 2's installer logic.
//!
//! The hard part of "install a downloaded archive" is not unzipping - it is
//! figuring out the `Data`-relative root inside an archive that may be wrapped in
//! one or more useless folders (`ModName-1234/...`). MO2 solves this with the
//! Simple installer (`InstallerQuick::getSimpleArchiveBase`) driven by a per-game
//! `ModDataChecker`. This module reproduces both as pure, testable logic over an
//! [`ArchiveTree`]; the archive backend and the extraction wiring live alongside.
//!
//! The `ModDataChecker` half is per-game, driven by [`LayoutRules`] rather than by
//! a hardcoded list: a mod root is whatever the game says a mod root looks like.
//! [`LayoutRules::default()`] is the Gamebryo vocabulary, which is what every game
//! that declares nothing still gets.
//!
//! Tier 1 (here): archive tree + per-game `ModDataChecker` + wrapper-strip.
//! Tier 2 (later): the FOMOD scripted installer.

use std::collections::BTreeMap;

/// One listed archive entry.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// `/`-joined path inside the archive.
    pub path: String,
    pub is_dir: bool,
}

/// A node in the archive tree.
#[derive(Debug, Clone)]
pub enum TreeNode {
    File { name: String },
    Dir { name: String, tree: ArchiveTree },
}

/// A case-insensitive tree of an archive's contents (mirrors MO2's
/// `ArchiveFileTree` / `IFileTree`). Keys are ASCII-lowercased, like the union.
#[derive(Debug, Clone, Default)]
pub struct ArchiveTree {
    /// Lowercased entry name -> node.
    pub entries: BTreeMap<String, TreeNode>,
}

/// How a Root Builder archive splits into the two places a mod can put files: the
/// game's `Data` directory, and the game install root. See
/// [`ArchiveTree::root_builder_split`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootSplit {
    /// `/`-joined prefix of the subtree that becomes the mod root (Data-relative),
    /// e.g. `Data/` - or `Data/inner/` when the Data half needed its own descent.
    /// `None` when the archive is root content only, with no `Data` half at all:
    /// a bare preloader or wrapper DLL, which is how the second half of SSE Engine
    /// Fixes ships on its own.
    pub data_prefix: Option<String>,
    /// The archive's own `Root` directory, when it shipped one. Its CONTENTS are
    /// placed into the mod's `Root/`.
    pub root_dir: Option<String>,
    /// Top-level entries placed into the mod's `Root/`, in tree order.
    pub root_entries: Vec<String>,
}

/// The verdict of a [`ModDataChecker`]-style check on an archive level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckReturn {
    /// This level is a valid mod root (loose files / a known Data folder).
    Valid,
    /// Not valid here, but a known fixup could make it so (reserved).
    Fixable,
    /// Not a mod root.
    Invalid,
}

impl ArchiveTree {
    /// Build the tree from a listing. Intermediate directories are created from
    /// file paths; explicit directory entries are honoured too.
    pub fn from_entries(entries: &[ArchiveEntry]) -> ArchiveTree {
        let mut root = ArchiveTree::default();
        for e in entries {
            let parts: Vec<&str> = e.path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
            root.insert(&parts, e.is_dir);
        }
        root
    }

    fn insert(&mut self, parts: &[&str], is_dir: bool) {
        match parts {
            [] => {}
            [last] => {
                let key = last.to_ascii_lowercase();
                if is_dir {
                    self.entries
                        .entry(key)
                        .or_insert_with(|| TreeNode::Dir { name: last.to_string(), tree: ArchiveTree::default() });
                } else {
                    self.entries
                        .entry(key)
                        .or_insert_with(|| TreeNode::File { name: last.to_string() });
                }
            }
            [dir, rest @ ..] => {
                let node = self
                    .entries
                    .entry(dir.to_ascii_lowercase())
                    .or_insert_with(|| TreeNode::Dir { name: dir.to_string(), tree: ArchiveTree::default() });
                if let TreeNode::Dir { tree, .. } = node {
                    tree.insert(rest, is_dir);
                }
            }
        }
    }

    /// Number of top-level entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// If this level is exactly one subdirectory (and nothing else), the wrapper
    /// to descend through.
    fn single_subdir(&self) -> Option<(&str, &ArchiveTree)> {
        if self.entries.len() != 1 {
            return None;
        }
        match self.entries.values().next() {
            Some(TreeNode::Dir { name, tree }) => Some((name.as_str(), tree)),
            _ => None,
        }
    }

    /// MO2's DataText layer: exactly one directory named `Data` (case-insensitive),
    /// every other top-level entry a loose documentation file. The `Data` dir is the
    /// real mod root to descend into (the sibling docs are dropped on the simple
    /// path). Without this, a `Data/ + readme.txt` archive is rejected as NotSimple.
    fn data_text_subdir(&self) -> Option<(&str, &ArchiveTree)> {
        let mut data: Option<(&str, &ArchiveTree)> = None;
        for node in self.entries.values() {
            match node {
                TreeNode::Dir { name, tree } if name.eq_ignore_ascii_case("data") => {
                    if data.is_some() {
                        return None; // a second directory: not this pattern
                    }
                    data = Some((name.as_str(), tree));
                }
                TreeNode::Dir { .. } => return None, // a non-Data directory beside it
                TreeNode::File { name } => {
                    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
                    if !DOC_EXTS.contains(&ext.as_str()) {
                        return None; // a non-doc file beside it
                    }
                }
            }
        }
        data
    }

    /// Every entry sitting BESIDE the `data_dir` path, as archive-relative paths.
    ///
    /// Walks down `data_dir` one component at a time and collects the other
    /// children of each level. For `SB/Content/Paks` in an archive holding
    /// `SB/Binaries/...` and `SB/Content/Paks/...` this is `["SB/Binaries"]`:
    /// everything that is NOT the data half, each at the path it must keep so it
    /// lands back where the archive meant it to.
    ///
    /// The first level is skipped deliberately: its siblings are the archive's own
    /// top level, which [`root_builder_split`](Self::root_builder_split) has
    /// already classified (and where it drops documentation). `None` when the path
    /// does not resolve.
    fn root_entries_beside(&self, data_dir: &str) -> Option<Vec<String>> {
        let parts: Vec<&str> = data_dir.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
        let mut out = Vec::new();
        let mut cur = self;
        let mut prefix = String::new();
        for (depth, part) in parts.iter().enumerate() {
            if depth > 0 {
                for node in cur.entries.values() {
                    let name = match node {
                        TreeNode::Dir { name, .. } | TreeNode::File { name } => name,
                    };
                    if !name.eq_ignore_ascii_case(part) {
                        out.push(format!("{prefix}{name}"));
                    }
                }
            }
            match cur.entries.get(&part.to_ascii_lowercase()) {
                Some(TreeNode::Dir { name, tree }) => {
                    prefix.push_str(name);
                    prefix.push('/');
                    cur = tree;
                }
                _ => return None,
            }
        }
        Some(out)
    }

    /// The Root Builder shape: ONE archive carrying both `Data`-relative content
    /// and content for the game INSTALL ROOT, next to the game executable.
    ///
    /// This is how a script-extender preloader, an ENB, a ReShade or an `.asi`
    /// loader ships. MO2 cannot install these at all without the third-party Root
    /// Builder plugin, because it maps every mod's contents into `Data` and
    /// discards what sits beside it. Eidos already mounts a mod's `Root/` over the
    /// game install root natively (`Instance::root_layers`), so the only missing
    /// piece is recognising the archive and laying it out.
    ///
    /// Matched when there is exactly one top-level directory named `Data`
    /// (case-insensitive) whose own contents resolve as a mod root, plus at least
    /// one sibling that is not a documentation file. The siblings become the mod's
    /// `Root/`. An explicit `Root/` directory is honoured as itself: its CONTENTS
    /// are what lands in `Root/`, not the folder nested inside itself.
    ///
    /// **Any other top-level DIRECTORY disqualifies the archive**, in both branches.
    /// Loose executables beside `Data/` are unambiguous - nothing else explains a
    /// `.dll` there - but a folder is not: `2K Textures/`, `Optional Textures/` and
    /// `Documentation/` are all ordinary archive structure, and filing them at the
    /// game root produces a mod that installs without error and does nothing, which
    /// is the worst outcome available here. Those go back to BAIN or the manual
    /// picker, where the user chooses.
    ///
    /// Docs are dropped, exactly as [`data_text_subdir`](Self::data_text_subdir)
    /// drops them. Anything else is kept: a stray file at the game root is inert,
    /// while a dropped `.dll` is a mod that silently does nothing - the failure
    /// this whole path exists to prevent.
    pub fn root_builder_split(&self, rules: LayoutRules) -> Option<RootSplit> {
        let mut data: Option<&str> = None;
        let mut root_dir: Option<String> = None;
        let mut game_dir: Option<&str> = None;
        let mut root_entries: Vec<String> = Vec::new();
        let mut has_image = false;
        for node in self.entries.values() {
            match node {
                TreeNode::Dir { name, .. } if name.eq_ignore_ascii_case("data") => {
                    if data.is_some() {
                        return None; // two Data dirs: not a shape we can reason about
                    }
                    data = Some(name.as_str());
                }
                TreeNode::Dir { name, .. } if name.eq_ignore_ascii_case("root") => {
                    if root_dir.is_some() {
                        return None;
                    }
                    root_dir = Some(name.clone());
                }
                // The game's own directory inside its install (`SB/`). An archive
                // leading with it is addressing the install ROOT, not the mod-merge
                // root: `SB/Binaries/Win64/ue4ss/Mods/...` is a UE4SS script mod,
                // which is Stellar Blade's equivalent of an SKSE plugin.
                //
                // Only reachable for a game whose mod root is nested (see
                // [`LayoutRules::game_dir`]), so this arm is unreachable for every
                // Bethesda game and cannot change what they do.
                TreeNode::Dir { name, .. }
                    if !rules.game_dir().is_empty()
                        && name.eq_ignore_ascii_case(rules.game_dir()) =>
                {
                    if game_dir.is_some() {
                        return None;
                    }
                    game_dir = Some(name.as_str());
                }
                // Structure we are not modelling: a BAIN sub-package, a texture
                // variant folder, a docs folder. Refuse rather than sweep it to the
                // game root.
                TreeNode::Dir { .. } => return None,
                TreeNode::File { name } => {
                    let ext =
                        name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
                    if IMAGE_EXTS.contains(&ext.as_str()) {
                        has_image = true;
                    }
                    if !DOC_EXTS.contains(&ext.as_str()) {
                        root_entries.push(name.clone());
                    }
                }
            }
        }
        if let Some(gd) = game_dir {
            // Two contradictory statements about where the content is anchored.
            // Whichever the author meant, guessing here files half the mod in the
            // wrong place, so it goes to the picker.
            if data.is_some() || root_dir.is_some() {
                return None;
            }
            // A subtree matching the game's OWN data dir cannot travel through
            // `Root/`: the root union would put it back at `<game>/<data_dir>`,
            // which is exactly the path the Data union is mounted over, so the
            // Data layer wins and the file is never served. `resolve_root_split`
            // handles the one-level version of this (`Root/Data`) by moving it to
            // the Data half; the nested version has to be cut out here.
            //
            // This is not a corner case. It is how a UE4SS mod that also ships a
            // pak arrives - the archive carries `SB/Binaries/.../Mods/X` and
            // `SB/Content/Paks/...` at once - and routing the whole thing to
            // `Root/` would install cleanly and serve half of it.
            let entries = match self.subtree(rules.data_dir) {
                None => {
                    // Pure install-root archive: the directory goes in whole.
                    let mut e = vec![gd.to_string()];
                    e.append(&mut root_entries);
                    e
                }
                Some(_) => {
                    let mut e = self.root_entries_beside(rules.data_dir)?;
                    e.append(&mut root_entries);
                    e
                }
            };
            let data_prefix = self
                .subtree(rules.data_dir)
                .map(|_| rules.data_dir.trim_matches('/').to_string());
            // Nothing outside the data dir: this is not the root shape at all, it
            // is an archive that simply wrapped its content in the game's own path.
            // `simple_archive_base` cannot see that far down, so it is claimed here
            // with no root half rather than sent to the picker.
            return Some(RootSplit { data_prefix, root_dir: None, root_entries: entries });
        }
        let Some(data) = data else {
            // No Data half at all. This is the pure root mod - a preloader or wrapper
            // DLL shipped on its own - and it is only safe to claim when nothing here
            // could have been Data content: the stray-directory rule above already
            // applies, so all that is left is to require either an explicit `Root/`
            // or an actual executable image to project. Otherwise leave it to the
            // manual picker rather than guess a mod into the game root, where it
            // would do nothing.
            if self.simple_archive_base(rules).is_some() || (root_dir.is_none() && !has_image) {
                return None;
            }
            return Some(RootSplit { data_prefix: None, root_dir, root_entries });
        };
        // No root half: a plain `Data/ + docs` archive, which is data_text_subdir's
        // job and already installs. Claiming it here would only add an empty Root/.
        if root_dir.is_none() && root_entries.is_empty() {
            return None;
        }
        // The Data half has to hold up on its own, or the folder is a coincidence
        // (a mod shipping a literal `data` folder of its own) and this is not the
        // Root Builder shape at all.
        let sub = match self.entries.get(&data.to_ascii_lowercase()) {
            Some(TreeNode::Dir { tree, .. }) => tree,
            _ => return None,
        };
        let inner = sub.simple_archive_base(rules)?;
        Some(RootSplit { data_prefix: Some(format!("{data}/{inner}")), root_dir, root_entries })
    }

    /// MO2's `ModDataChecker::dataLooksValid`: this level is a valid mod root if a
    /// top-level entry is one of the game's data folders, or a file carrying one of
    /// its data extensions.
    ///
    /// `rules` is the whole of what makes this per-game; see [`LayoutRules`], whose
    /// default is the Gamebryo list this used to read directly.
    pub fn data_looks_valid(&self, rules: LayoutRules) -> CheckReturn {
        for (key, node) in &self.entries {
            match node {
                TreeNode::Dir { .. } if rules.folder_matches(key) => return CheckReturn::Valid,
                TreeNode::File { .. } => {
                    if let Some(ext) = key.rsplit_once('.').map(|(_, e)| e) {
                        if rules.suffix_matches(ext) {
                            return CheckReturn::Valid;
                        }
                    }
                }
                _ => {}
            }
        }
        CheckReturn::Invalid
    }

    /// MO2's `getSimpleArchiveBase`: descend while there is exactly one wrapper
    /// subdirectory, until this level looks like a valid mod root. Returns the
    /// `/`-joined prefix to strip on extraction (empty if already valid), or
    /// `None` if it never resolves to a mod root (not a "simple" archive).
    pub fn simple_archive_base(&self, rules: LayoutRules) -> Option<String> {
        fn rec(tree: &ArchiveTree, prefix: &mut String, rules: LayoutRules) -> bool {
            if tree.data_looks_valid(rules) == CheckReturn::Valid {
                return true;
            }
            if let Some((name, sub)) = tree.single_subdir() {
                prefix.push_str(name);
                prefix.push('/');
                return rec(sub, prefix, rules);
            }
            // MO2's DataText layer: a sole `Data` dir beside loose docs - descend
            // into Data (the docs are not mod content and are dropped here).
            if let Some((name, sub)) = tree.data_text_subdir() {
                prefix.push_str(name);
                prefix.push('/');
                return rec(sub, prefix, rules);
            }
            false
        }
        let mut prefix = String::new();
        if rec(self, &mut prefix, rules) {
            Some(prefix)
        } else {
            None
        }
    }

    /// MO2's `InstallerBAIN::findSubpackages`: the top-level directories of a Wrye
    /// Bash complex package that each INDEPENDENTLY look like a mod root (`00 Core`,
    /// `01 Optional Textures`, ...), in the tree's case-insensitive order, plus how
    /// many candidate directories did not.
    ///
    /// Detection is structural, never a numeric-prefix guess: a folder qualifies only
    /// when [`data_looks_valid`](Self::data_looks_valid) accepts its own contents, so
    /// `00 Core` counts for exactly the same reason `Core` would and a numbered folder
    /// of screenshots does not. Skipped: files, MO2's IGNORED_FOLDERS (a combined
    /// FOMOD/BAIN package keeps its `fomod` dir), and any name starting with `--`,
    /// Wrye Bash's marker for a sub-package the author disabled.
    ///
    /// A non-zero invalid count is deliberately NOT a verdict. MO2 asks the user,
    /// because `Data/` beside `OptionalStuff/` is indistinguishable from a two-package
    /// BAIN by structure alone; the caller decides (see [`BAIN_MIN_SUBPACKAGES`]).
    pub fn bain_subpackages(&self, rules: LayoutRules) -> (Vec<String>, usize) {
        let mut valid = Vec::new();
        let mut invalid = 0usize;
        for (key, node) in &self.entries {
            // Only directories are candidates; a top-level `package.txt` or readme is
            // BAIN metadata, not a sub-package.
            let TreeNode::Dir { name, tree } = node else { continue };
            if BAIN_IGNORED_FOLDERS.contains(&key.as_str()) || key.starts_with("--") {
                continue;
            }
            if tree.data_looks_valid(rules) == CheckReturn::Valid {
                valid.push(name.clone());
            } else {
                invalid += 1;
            }
        }
        (valid, invalid)
    }

    /// The subtree at a `/`-joined path, each component matched case-insensitively
    /// (`""` is this tree). `None` if the path does not exist or names a file - which
    /// is what a manual-install picker needs to reject a bad "set as Data directory".
    pub fn subtree(&self, path: &str) -> Option<&ArchiveTree> {
        let mut cur = self;
        for part in path.split(['/', '\\']).filter(|s| !s.is_empty()) {
            match cur.entries.get(&part.to_ascii_lowercase()) {
                Some(TreeNode::Dir { tree, .. }) => cur = tree,
                _ => return None,
            }
        }
        Some(cur)
    }

    /// Whether the level at `path` looks like a valid mod root - MO2's live
    /// "The content of &lt;Data&gt; looks valid." / "does not look valid." feedback in
    /// the manual installer, so the user learns their pick is wrong BEFORE committing.
    /// A path that does not resolve is not valid.
    pub fn root_looks_valid(&self, path: &str, rules: LayoutRules) -> bool {
        self.subtree(path).is_some_and(|t| t.data_looks_valid(rules) == CheckReturn::Valid)
    }

    /// Flatten the tree to display rows for a picker UI, depth-first in
    /// case-insensitive order, directories and files interleaved as stored.
    ///
    /// Depth is capped (see `MAX_TREE_DEPTH`): an archive is untrusted input, and a
    /// front end walking a pathological nesting must degrade - rows below the cap are
    /// simply not listed - rather than blow the stack while the user watches.
    pub fn flatten(&self) -> Vec<TreeRow> {
        fn walk(tree: &ArchiveTree, prefix: &str, depth: usize, out: &mut Vec<TreeRow>) {
            if depth >= MAX_TREE_DEPTH {
                return;
            }
            for node in tree.entries.values() {
                let (name, sub) = match node {
                    TreeNode::Dir { name, tree } => (name, Some(tree)),
                    TreeNode::File { name } => (name, None),
                };
                let path =
                    if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
                out.push(TreeRow { depth, name: name.clone(), path: path.clone(), is_dir: sub.is_some() });
                if let Some(sub) = sub {
                    walk(sub, &path, depth + 1, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(self, "", 0, &mut out);
        out
    }

    /// Whether the archive contains a `fomod` directory anywhere - a scripted
    /// FOMOD installer (Tier 2, not handled by the Simple installer).
    pub fn has_fomod(&self) -> bool {
        self.entries.contains_key("fomod")
            || self
                .entries
                .values()
                .any(|n| matches!(n, TreeNode::Dir { tree, .. } if tree.has_fomod()))
    }
}

/// One row of a flattened [`ArchiveTree`], for a picker UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    /// Nesting level, 0 for a top-level entry.
    pub depth: usize,
    /// The entry's own name, in its original archive casing.
    pub name: String,
    /// The `/`-joined path from the archive root - what to hand back as the chosen
    /// data root (see [`install_manual`]).
    pub path: String,
    pub is_dir: bool,
}

/// How deep [`ArchiveTree::flatten`] will descend. Deliberately generous for real
/// mods and finite for hostile ones.
const MAX_TREE_DEPTH: usize = 64;

/// Top-level folders a Wrye Bash package may ship that are never sub-packages
/// (lowercased, to match [`ArchiveTree::entries`] keys) - MO2's
/// `InstallerBAIN::findSubpackages` IGNORED_FOLDERS.
const BAIN_IGNORED_FOLDERS: &[&str] =
    &["fomod", "omod conversion data", "images", "screenshots", "docs"];

/// How many independently-valid sub-packages a tree needs before it is offered as a
/// BAIN install. MO2's rule: with fewer than two there is nothing to choose between,
/// so the archive is better served by the simple or the manual path.
pub const BAIN_MIN_SUBPACKAGES: usize = 2;

/// MO2's default ticks for a BAIN package (`BainComplexInstallerDialog`): every
/// sub-package whose name starts with `00` - Wrye Bash's convention for the mandatory
/// core - plus anything the user picked last time, matched case-insensitively.
///
/// `previous` is the `option0..N` list a front end persisted on the mod after the
/// last install, so a reinstall comes back pre-selected. Pass `&[]` when there is
/// none. The returned vector is parallel to `subpackages`.
pub fn bain_default_selection(subpackages: &[String], previous: &[String]) -> Vec<bool> {
    subpackages
        .iter()
        .map(|s| {
            s.starts_with("00") || previous.iter().any(|p| p.eq_ignore_ascii_case(s))
        })
        .collect()
}

mod install;
pub use install::{
    collision_name, finish_fomod, fomod_context, install_archive, install_archive_with_policy,
    mod_name_for,
    extract_to_temp, install_bain, install_extracted, install_manual, open_archive, ExtractedTree,
    FomodSession, InstallError, InstallReport, Opened, OverwritePolicy,
};

/// What makes a directory level a valid mod root, for one game.
///
/// This is MO2's per-game `ModDataChecker` reduced to the two lists it actually
/// consults. It exists as a value rather than a pair of consts because the check
/// is the one piece of the installer that is not game-agnostic: a Stardew mod is a
/// folder holding `manifest.json`, a BepInEx mod is a `BepInEx/` tree, and neither
/// has ever resolved here because the Gamebryo vocabulary was the only vocabulary.
///
/// [`Default`] is that Gamebryo vocabulary, unchanged and still the answer for
/// every game that does not ask for another one - so a caller with no game in hand
/// classifies exactly as this module did when the lists were read from a const.
///
/// Matching is case-insensitive, so a game may spell its rules however its own
/// documentation does (`BepInEx`, not `bepinex`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRules {
    /// Top-level directory names that mark this level as a mod root.
    pub folders: &'static [&'static str],
    /// File extensions, without the dot, that mark this level as a mod root.
    pub suffixes: &'static [&'static str],
    /// The game's mod-merge root relative to its install directory, exactly as the
    /// game declares it (`Data`, `SB/Content/Paks`). Empty in the default rules,
    /// which belong to no game.
    ///
    /// Not part of the vocabulary: this is here so the checker can tell a
    /// `Data`-relative archive from one addressing the install ROOT. See
    /// [`Self::game_dir`].
    pub data_dir: &'static str,
}

impl Default for LayoutRules {
    fn default() -> Self {
        LayoutRules { folders: GAMEBRYO_FOLDERS, suffixes: GAMEBRYO_SUFFIXES, data_dir: "" }
    }
}

impl LayoutRules {
    /// The game's own directory inside its install, when the mod-merge root sits
    /// BELOW it - `SB` for Stellar Blade, whose data dir is `SB/Content/Paks`.
    ///
    /// Empty when the mod root is a direct child of the install root, which is
    /// every Bethesda game (`Data`, `Data Files`). That emptiness is what keeps
    /// [`ArchiveTree::root_builder_split`]'s install-root branch from ever firing
    /// for them.
    pub fn game_dir(&self) -> &'static str {
        match self.data_dir.trim_matches('/').split_once('/') {
            Some((first, rest)) if !first.is_empty() && !rest.trim_matches('/').is_empty() => first,
            _ => "",
        }
    }

    /// The rules for an Eidos game id (`skyrimse`, `stardew`, ...). An unknown id,
    /// or a game that declares no vocabulary of its own, gets [`Default`].
    pub fn for_game(game_id: &str) -> LayoutRules {
        eidos_gamedef::GameDef::for_id(game_id).map(LayoutRules::from).unwrap_or_default()
    }

    /// Whether `name` is one of this game's data folders.
    pub fn folder_matches(&self, name: &str) -> bool {
        self.folders.iter().any(|f| f.eq_ignore_ascii_case(name))
    }

    /// Whether `ext` (no dot) is one of this game's data file extensions.
    pub fn suffix_matches(&self, ext: &str) -> bool {
        self.suffixes.iter().any(|s| s.eq_ignore_ascii_case(ext))
    }
}

impl From<&eidos_gamedef::GameDef> for LayoutRules {
    /// An empty list on the descriptor means "the Gamebryo vocabulary", NOT "no
    /// vocabulary".
    ///
    /// This distinction is the single load-bearing line in the whole per-game
    /// checker. Taken literally, an empty list makes [`ArchiveTree::data_looks_valid`]
    /// return `Invalid` for every level of every archive, which makes
    /// `simple_archive_base` return `None`, which sends every install of every
    /// game - Skyrim included, since no built-in game declares these fields - to
    /// the manual picker. `every_builtin_game_keeps_the_default_vocabulary` exists
    /// to make that mistake impossible to merge.
    /// A game declares its vocabulary as a WHOLE, not list by list. Naming even one
    /// rule means the Gamebryo lists do not apply to that game at all.
    ///
    /// Falling back per-list would be worse than useless for the games this exists
    /// for: Stellar Blade's mods are `.pak`/`.ucas`/`.utoc` and nothing else, and a
    /// per-list fallback would leave it inheriting `textures/`, `meshes/` and
    /// `.esp`. An archive shipping a stray `textures` folder would then read as a
    /// valid mod root and install to the wrong place, silently.
    fn from(def: &eidos_gamedef::GameDef) -> Self {
        // `data_dir` is not part of the vocabulary and is always carried through:
        // where a game's mods deploy is a fact about the game, not a dialect it
        // opts into.
        let d = LayoutRules::default();
        let (folders, suffixes) = if def.valid_folders.is_empty() && def.valid_suffixes.is_empty() {
            (d.folders, d.suffixes)
        } else {
            (def.valid_folders, def.valid_suffixes)
        };
        LayoutRules { folders, suffixes, data_dir: def.data_dir }
    }
}

/// Known top-level Data folder names (lowercased), from MO2's
/// `GamebryoModDataChecker::possibleFolderNames`.
const GAMEBRYO_FOLDERS: &[&str] = &[
    "fonts",
    "interface",
    "menus",
    "meshes",
    "music",
    "scripts",
    "shaders",
    "sound",
    "strings",
    "textures",
    "trees",
    "video",
    "facegen",
    "materials",
    "skse",
    "obse",
    "mwse",
    "nvse",
    "fose",
    "f4se",
    "distantlod",
    "asi",
    "skyproc patchers",
    "tools",
    "mcm",
    "icons",
    "bookart",
    "distantland",
    "mits",
    "splash",
    "dllplugins",
    "calientetools",
    "netscriptframework",
    "shadersfx",
];

/// Known Data file extensions (lowercased), from
/// `GamebryoModDataChecker::possibleFileExtensions`.
const GAMEBRYO_SUFFIXES: &[&str] = &["esp", "esm", "esl", "bsa", "ba2", "modgroups", "ini"];

/// What kinds of content a mod ships - MO2's `ModDataContent` (the Content column
/// icons), shallow structural detection: top-level dirs by name + files by
/// extension, no record parsing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ContentFlags {
    pub plugins: bool,
    pub bsa: bool,
    pub textures: bool,
    pub meshes: bool,
    pub scripts: bool,
    pub skse: bool,
    pub interface: bool,
    pub sound: bool,
    pub facegen: bool,
}

impl ContentFlags {
    /// A compact letters string in MO2 icon order, e.g. `"P A T M"` (empty if none).
    /// P=plugin, A=archive (BSA), T=textures, M=meshes, S=scripts, K=SKSE,
    /// I=interface, U=sound, F=FaceGen.
    pub fn tags(&self) -> String {
        let mut s = String::new();
        for (on, ch) in [
            (self.plugins, 'P'),
            (self.bsa, 'A'),
            (self.textures, 'T'),
            (self.meshes, 'M'),
            (self.scripts, 'S'),
            (self.skse, 'K'),
            (self.interface, 'I'),
            (self.sound, 'U'),
            (self.facegen, 'F'),
        ] {
            if on {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push(ch);
            }
        }
        s
    }
}

/// Detect a mod's content kinds by a SHALLOW scan of its directory (MO2's
/// `GamebryoModDataContent` is shallow: top-level dirs by name, top-level files by
/// extension, plus FaceGen one level under meshes/textures). Cheap enough to run
/// per mod on a refresh.
pub fn classify_content_dir(root: &std::path::Path) -> ContentFlags {
    let mut c = ContentFlags::default();
    let Ok(rd) = std::fs::read_dir(root) else { return c };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            match name.as_str() {
                "textures" | "icons" | "bookart" => c.textures = true,
                "meshes" => c.meshes = true,
                "interface" | "menus" => c.interface = true,
                "music" | "sound" => c.sound = true,
                "scripts" => c.scripts = true,
                "skse" | "obse" | "f4se" | "nvse" | "fose" | "mwse" | "dllplugins" => c.skse = true,
                _ => {}
            }
            // FaceGen lives one level down (meshes/FaceGenData, textures/FaceGenData).
            if matches!(name.as_str(), "meshes" | "textures") && !c.facegen {
                if let Ok(sub) = std::fs::read_dir(e.path()) {
                    if sub
                        .flatten()
                        .any(|s| s.file_name().to_string_lossy().eq_ignore_ascii_case("facegendata"))
                    {
                        c.facegen = true;
                    }
                }
            }
        } else if let Some((_, ext)) = name.rsplit_once('.') {
            match ext {
                "esp" | "esm" | "esl" => c.plugins = true,
                "bsa" | "ba2" => c.bsa = true,
                _ => {}
            }
        }
    }
    c
}

/// Guess a clean mod name AND the Nexus mod id from a (possibly Nexus-suffixed)
/// archive filename: `Foo - Bar-19181-1-7-1575746557.7z` -> `("Foo - Bar", Some(19181))`.
/// Mirrors MO2's `interpretNexusFileName`: the trailing
/// `-<modid>-<version...>-<timestamp>` is stripped, tolerating lettered version
/// segments (`9b`, `2SE`) that the older all-digit strip left attached.
pub fn guess_mod_name_and_id(archive: &str) -> (String, Option<u64>) {
    let stem = std::path::Path::new(archive).file_stem().and_then(|s| s.to_str()).unwrap_or("Mod");
    let mut parts: Vec<&str> = stem.split('-').collect();
    let mut suffix: Vec<&str> = Vec::new();
    while parts.len() > 1 && parts.last().is_some_and(|p| is_version_like(p)) {
        suffix.push(parts.pop().unwrap());
    }
    // The mod id is the first version-like group of the suffix read left-to-right
    // (the last one popped) that is a pure number of >=2 digits.
    let mod_id = suffix.iter().rev().find_map(|p| {
        let t = p.trim();
        if t.len() >= 2 && t.bytes().all(|b| b.is_ascii_digit()) {
            t.parse::<u64>().ok()
        } else {
            None
        }
    });
    let name = parts.join("-");
    let name = name.trim().trim_end_matches('-').trim();
    let name = if name.is_empty() { stem.to_string() } else { name.to_string() };
    (name, mod_id)
}

/// Just the mod name (see [`guess_mod_name_and_id`]).
pub fn guess_mod_name(archive: &str) -> String {
    guess_mod_name_and_id(archive).0
}

/// A trailing archive-name group that looks like a Nexus version/id/timestamp:
/// pure digits, or digits followed by up to 3 letters (`9b`, `2SE`, `5a`).
fn is_version_like(s: &str) -> bool {
    let s = s.trim();
    let digits = s.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 {
        return false;
    }
    let tail = &s[digits..];
    tail.is_empty() || (tail.len() <= 3 && tail.bytes().all(|b| b.is_ascii_alphabetic()))
}

/// Documentation file extensions allowed beside a `Data` directory in MO2's
/// DataText layer (lowercased).
const DOC_EXTS: &[&str] = &["txt", "pdf", "md", "jpg", "jpeg", "png", "bmp"];

/// Executable images: what the Windows loader maps from the game install root, and
/// therefore what makes a Data-less archive a root mod rather than an unknown one.
const IMAGE_EXTS: &[&str] = &["dll", "exe", "asi"];

/// MO2's `fixDirectoryName`: make a mod folder name filesystem-safe - drop the
/// Windows-illegal set `<>:"/\|?*` and control chars, collapse internal whitespace,
/// strip trailing dots/spaces. Returns `None` if nothing usable remains. Needed
/// once real Nexus names (`Beyond Skyrim: Bruma`) are used as the folder name.
pub fn fix_directory_name(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') && !c.is_control())
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_end_matches(['.', ' ']).trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Gamebryo vocabulary, which is what every case below is written
    /// against: these tests predate the rules being a parameter and must keep
    /// asserting exactly what they asserted then.
    fn rules() -> LayoutRules {
        LayoutRules::default()
    }

    /// A game that declares nothing must classify with the Gamebryo vocabulary.
    ///
    /// This guards the one line that could break every existing install at once.
    /// No Bethesda game declares `valid_folders`/`valid_suffixes`, so
    /// `From<&GameDef>` has to read those empty lists as "use the default" rather
    /// than as "nothing is ever a mod root". Get that backwards and
    /// `data_looks_valid` returns `Invalid` everywhere, `simple_archive_base`
    /// returns `None` everywhere, and every Skyrim mod lands in the manual picker.
    ///
    /// The eleven Bethesda games are named outright rather than derived from
    /// `GAMES`, so that adding a twelfth game cannot quietly shrink what this
    /// covers - which is exactly what would have happened here when Stellar Blade
    /// arrived and the loop stopped meaning "every game".
    #[test]
    fn a_game_that_declares_nothing_keeps_the_gamebryo_vocabulary() {
        let default = LayoutRules::default();
        let vocabulary = |r: LayoutRules| (r.folders, r.suffixes);
        for id in [
            "skyrimse", "skyrim", "skyrimvr", "enderalse", "fallout4", "fallout4vr", "falloutnv",
            "fallout3", "oblivion", "morrowind", "starfield",
        ] {
            assert!(eidos_gamedef::GameDef::for_id(id).is_some(), "{id} vanished from the catalog");
            let r = LayoutRules::for_game(id);
            assert_eq!(vocabulary(r), vocabulary(default), "{id} lost the Gamebryo vocabulary");
            // Their mod root is the install root's own child, so they have no game
            // directory - which is what makes `root_builder_split`'s install-root
            // branch unreachable for them.
            assert!(r.game_dir().is_empty(), "{id} would now be read as install-root relative");
        }
        // An id nobody knows must not become "nothing is a mod root" either.
        assert_eq!(LayoutRules::for_game("no-such-game"), default);
        assert_eq!(LayoutRules::for_game(""), default);

        // And the general rule, over whatever the catalog holds: declaring nothing
        // yields the default, declaring anything yields something else. Both
        // branches must be exercised or the assertions above pass vacuously.
        let (mut declaring, mut inheriting) = (0, 0);
        for def in eidos_gamedef::GAMES {
            let rules = LayoutRules::from(def);
            assert_eq!(LayoutRules::for_game(def.id), rules, "{} disagrees via for_game", def.id);
            if def.valid_folders.is_empty() && def.valid_suffixes.is_empty() {
                inheriting += 1;
                assert_eq!(
                    vocabulary(rules),
                    vocabulary(default),
                    "{} declares nothing but lost the default",
                    def.id
                );
            } else {
                declaring += 1;
                assert_ne!(
                    vocabulary(rules),
                    vocabulary(default),
                    "{} declared rules that were ignored",
                    def.id
                );
            }
        }
        assert!(declaring > 0, "no game declares its own vocabulary");
        assert!(inheriting > 0, "no game relies on the default");
    }

    /// A game that declares any rule replaces the Gamebryo vocabulary entirely,
    /// including the list it did NOT name.
    ///
    /// A game whose mods are only ever `.pak` files has no folder vocabulary at
    /// all, and must not inherit one: `textures/` inside an Unreal archive is
    /// texture content, not a sign that the archive is a Bethesda mod root.
    #[test]
    fn declaring_any_rule_replaces_the_whole_vocabulary() {
        let def = eidos_gamedef::parse_game(
            "id = \"unreal-ish\"\nname = \"Unreal-ish\"\nsteam_app_id = 1\n\
             data_dir = \".\"\nvalid_suffixes = [\"pak\"]\n",
        )
        .unwrap();
        let r = LayoutRules::from(&def);
        assert!(r.suffix_matches("pak"), "the declared suffix applies");
        assert!(!r.suffix_matches("esp"), "and the Gamebryo suffixes are gone");
        assert!(r.folders.is_empty(), "an undeclared list stays empty, not Gamebryo");
        assert!(!r.folder_matches("textures"));
        // So a stray `textures/` no longer passes this game off as a mod root.
        assert_eq!(tree(&["textures/a.dds"]).data_looks_valid(r), CheckReturn::Invalid);
        assert_eq!(tree(&["mod_P.pak"]).data_looks_valid(r), CheckReturn::Valid);
    }

    /// Matching ignores case, so a descriptor may spell its rules the way the
    /// game's own documentation does (`BepInEx`, not `bepinex`).
    #[test]
    fn declared_vocabulary_matches_case_insensitively() {
        let def = eidos_gamedef::parse_game(
            "id = \"valheim\"\nname = \"Valheim\"\nsteam_app_id = 892970\n\
             data_dir = \".\"\nvalid_folders = [\"BepInEx\"]\nvalid_suffixes = [\"DLL\"]\n",
        )
        .unwrap();
        let r = LayoutRules::from(&def);
        assert!(r.folder_matches("bepinex") && r.folder_matches("BepInEx"));
        assert!(r.suffix_matches("dll"));
        // And the tree agrees, since its keys are lowercased.
        assert_eq!(tree(&["BepInEx/plugins/x.dll"]).data_looks_valid(r), CheckReturn::Valid);
    }

    fn tree(files: &[&str]) -> ArchiveTree {
        let entries: Vec<ArchiveEntry> =
            files.iter().map(|p| ArchiveEntry { path: p.to_string(), is_dir: p.ends_with('/') }).collect();
        ArchiveTree::from_entries(&entries)
    }

    #[test]
    fn valid_when_top_level_is_a_data_folder() {
        assert_eq!(tree(&["meshes/armor/a.nif"]).data_looks_valid(rules()), CheckReturn::Valid);
    }

    /// SSE Engine Fixes' All-In-One, the archive that motivated this: a `data/` half
    /// for the mod plus a loose preloader DLL for the game root. It is NOT a simple
    /// archive - which is precisely why the root half used to be discarded.
    #[test]
    fn engine_fixes_all_in_one_splits_into_both_halves() {
        let t = tree(&[
            "data/skse/plugins/EngineFixes.dll",
            "data/skse/plugins/EngineFixes.toml",
            "d3dx9_42.dll",
            "SSE Engine Fixes - Install Instructions.txt",
            "vortex_override_instructions.json",
        ]);
        assert_eq!(t.simple_archive_base(rules()), None, "must not already resolve as simple");
        let split = t.root_builder_split(rules()).expect("root builder split");
        assert_eq!(split.data_prefix.as_deref(), Some("data/"));
        assert_eq!(split.root_dir, None);
        // The DLL is kept, the .txt is dropped as documentation. The .json is not a
        // known doc extension, so it is kept rather than silently binned.
        assert!(split.root_entries.contains(&"d3dx9_42.dll".to_string()));
        assert!(split.root_entries.contains(&"vortex_override_instructions.json".to_string()));
        assert!(!split.root_entries.iter().any(|e| e.ends_with(".txt")));
    }

    /// An archive already using MO2's Root Builder convention. Its `Root/` is taken
    /// as itself, so the contents land one level deep, not two.
    #[test]
    fn an_explicit_root_folder_is_taken_as_itself() {
        let t = tree(&["Data/MyMod.esp", "Root/binkw64.dll", "Root/tools/patch.exe"]);
        let split = t.root_builder_split(rules()).expect("root builder split");
        assert_eq!(split.data_prefix.as_deref(), Some("Data/"));
        assert_eq!(split.root_dir.as_deref(), Some("Root"));
        assert!(split.root_entries.is_empty(), "Root/ contents are not also loose entries");
    }

    /// Engine Fixes' second half downloaded on its own: a bare preloader DLL, no
    /// `Data` anywhere. The purest root mod there is, and the shape MO2 cannot take.
    /// Stellar Blade's rules, whose data dir is nested two levels down.
    fn sb() -> LayoutRules {
        LayoutRules::for_game("stellarblade")
    }

    #[test]
    fn an_archive_leading_with_the_game_directory_is_install_root_relative() {
        // The real shape of a UE4SS script mod: everything is addressed from the
        // game INSTALL root, not from the mod-merge root. Before this it walked
        // down a single-subdir chain, recognised nothing, and reached the manual
        // picker - whose contract is to drop everything beside the chosen root,
        // which is the whole archive.
        let t = tree(&[
            "SB/Binaries/Win64/ue4ss/Mods/DekCNS/enabled.txt",
            "SB/Binaries/Win64/ue4ss/Mods/DekCNS/Scripts/main.lua",
        ]);
        assert_eq!(t.simple_archive_base(sb()), None, "no level of it is a mod root");
        let split = t.root_builder_split(sb()).expect("install-root relative");
        assert_eq!(split.data_prefix, None, "there is no Data half");
        assert_eq!(split.root_dir, None, "the archive used no Root/ convention");
        // The directory goes in whole, so it lands at Root/SB/Binaries/...
        assert_eq!(split.root_entries, vec!["SB".to_string()]);
    }

    #[test]
    fn the_game_directory_rule_cannot_fire_for_a_bethesda_game() {
        // Skyrim's mod root IS the install root's child, so it has no game
        // directory and this branch is unreachable - which is the whole reason
        // adding it could not disturb any existing game.
        assert_eq!(LayoutRules::for_game("skyrimse").game_dir(), "");
        assert_eq!(LayoutRules::for_game("morrowind").game_dir(), "", "'Data Files' is one component");
        assert_eq!(LayoutRules::default().game_dir(), "");
        assert_eq!(sb().game_dir(), "SB");
        // Under Skyrim's rules the same archive is just an unrecognised folder.
        let t = tree(&["SB/Binaries/Win64/ue4ss/Mods/X/enabled.txt"]);
        assert_eq!(t.root_builder_split(rules()), None);
    }

    #[test]
    fn a_ue4ss_mod_that_also_ships_a_pak_is_cut_along_the_data_dir() {
        // The real shape of CustomNanosuitSystem, and the reason the install-root
        // branch cannot simply route the whole directory to `Root/`: the root
        // union would put `SB/Content/Paks` back at exactly the path the Data
        // union is mounted over, so the pak would be shadowed and never served.
        // The archive would install cleanly and work by half.
        let t = tree(&[
            "SB/Content/Paks/~mods/a_P.pak",
            "SB/Binaries/Win64/ue4ss/Mods/X/enabled.txt",
        ]);
        assert_eq!(t.simple_archive_base(sb()), None, "not a wrapper chain");
        let split = t.root_builder_split(sb()).expect("cut into two halves");
        // The pak half becomes the mod root, so it deploys through the Data mount.
        assert_eq!(split.data_prefix.as_deref(), Some("SB/Content/Paks"));
        // The rest keeps the path it had, so it lands at Root/SB/Binaries/...
        assert_eq!(split.root_entries, vec!["SB/Binaries".to_string()]);
        assert_eq!(split.root_dir, None);
    }

    #[test]
    fn an_archive_wrapped_in_the_game_path_is_just_its_data_half() {
        // Nothing beside the data dir: the archive only wrapped its content in the
        // game's own path. There is no root half to make, and claiming one would
        // create an empty `Root/`.
        let t = tree(&["SB/Content/Paks/~mods/thing_P.pak"]);
        let split = t.root_builder_split(sb()).expect("a data half and nothing else");
        assert_eq!(split.data_prefix.as_deref(), Some("SB/Content/Paks"));
        assert!(split.root_entries.is_empty());
        // In practice the simple path claims this one first, which is the same
        // outcome by a shorter route - `~mods` is one of the game's data folders.
        assert_eq!(t.simple_archive_base(sb()).as_deref(), Some("SB/Content/Paks/"));
    }

    #[test]
    fn the_game_directory_does_not_mix_with_the_other_two_conventions() {
        // An archive claiming both anchors is saying two contradictory things.
        assert_eq!(tree(&["SB/Binaries/x.dll", "Data/textures/a.dds"]).root_builder_split(sb()), None);
        assert_eq!(tree(&["SB/Binaries/x.dll", "Root/dxgi.dll"]).root_builder_split(sb()), None);
        // And loose files beside it still travel with it, as they do everywhere
        // else on this path: a dropped .dll is a mod that silently does nothing.
        let split = tree(&["SB/Binaries/x.dll", "dxgi.dll", "readme.txt"])
            .root_builder_split(sb())
            .expect("game dir plus a loose image");
        assert_eq!(split.root_entries, vec!["SB".to_string(), "dxgi.dll".to_string()]);
    }

    #[test]
    fn a_bare_preloader_dll_is_a_pure_root_mod() {
        let t = tree(&["d3dx9_42.dll", "vortex_override_instructions.json"]);
        assert_eq!(t.simple_archive_base(rules()), None);
        let split = t.root_builder_split(rules()).expect("pure root mod");
        assert_eq!(split.data_prefix, None, "there is no Data half to place");
        assert!(split.root_entries.contains(&"d3dx9_42.dll".to_string()));
    }

    /// The guard on that: with no Data half AND no executable, we cannot tell root
    /// content from an archive whose layout we simply do not understand. Guessing
    /// would file it at the game root, where it would silently do nothing - so this
    /// goes to the manual picker instead.
    #[test]
    fn a_dataless_archive_without_an_executable_is_left_to_the_picker() {
        assert_eq!(tree(&["config.json", "notes.md"]).root_builder_split(rules()), None);
    }

    /// And a stray directory means the archive has structure we are not modelling,
    /// so the pure-root shortcut must not fire.
    #[test]
    fn a_dataless_archive_with_a_stray_directory_is_not_a_pure_root_mod() {
        assert_eq!(tree(&["loader.dll", "extras/thing.cfg"]).root_builder_split(rules()), None);
    }

    /// A plain `Data/ + readme` archive is the DataText case and already installs.
    /// Claiming it here would bolt an empty `Root/` onto every ordinary mod.
    #[test]
    fn a_data_folder_beside_only_docs_is_not_a_root_split() {
        let t = tree(&["Data/MyMod.esp", "readme.txt", "preview.png"]);
        assert!(t.simple_archive_base(rules()).is_some(), "still the simple/DataText path");
        assert_eq!(t.root_builder_split(rules()), None);
    }

    /// A mod shipping its own folder called `data` (config, not the game's Data) must
    /// not be torn in half: the Data candidate has to look like a mod root first.
    #[test]
    fn a_coincidental_data_folder_is_not_a_root_split() {
        assert_eq!(tree(&["data/settings.cfg", "loader.dll"]).root_builder_split(rules()), None);
    }

    /// A folder beside `Data/` is ordinary archive structure - a texture variant, a
    /// BAIN sub-package, a docs folder - and NOT game-root content. Sweeping it into
    /// `Root/` installs it next to the game exe, where nothing reads it: a mod that
    /// reports success and does nothing. These three all used to be claimed.
    #[test]
    fn a_directory_beside_data_disqualifies_the_split() {
        for files in [
            // A texture pack with resolution variants.
            &["Data/meshes/a.nif", "2K Textures/textures/a.dds"][..],
            // A BAIN-shaped pack whose sub-packages are not `00`-prefixed.
            &["Data/Core.esp", "Optional Textures/textures/a.dds"][..],
            // A documentation folder.
            &["Data/meshes/a.nif", "Documentation/manual.pdf"][..],
        ] {
            assert_eq!(
                tree(files).root_builder_split(rules()),
                None,
                "must not claim {files:?} - it belongs to BAIN or the manual picker"
            );
        }
    }

    /// The same rule on the Data-less side, which already had it: structure we do not
    /// model means we do not guess.
    #[test]
    fn a_directory_beside_a_loose_dll_disqualifies_the_split() {
        assert_eq!(tree(&["loader.dll", "extras/thing.cfg"]).root_builder_split(rules()), None);
    }

    #[test]
    fn valid_when_top_level_has_a_plugin() {
        assert_eq!(tree(&["MyMod.esp", "readme.txt"]).data_looks_valid(rules()), CheckReturn::Valid);
    }

    #[test]
    fn invalid_when_wrapped_in_a_useless_folder() {
        // The classic Nexus wrapper: nothing useful at the top level.
        assert_eq!(tree(&["MyMod-1234/meshes/a.nif"]).data_looks_valid(rules()), CheckReturn::Invalid);
    }

    #[test]
    fn base_strips_a_single_wrapper() {
        let t = tree(&["MyMod-1234/meshes/a.nif", "MyMod-1234/textures/b.dds"]);
        assert_eq!(t.simple_archive_base(rules()).as_deref(), Some("MyMod-1234/"));
    }

    #[test]
    fn base_strips_nested_wrappers() {
        let t = tree(&["a/b/scripts/x.pex", "a/b/MyMod.esp"]);
        assert_eq!(t.simple_archive_base(rules()).as_deref(), Some("a/b/"));
    }

    #[test]
    fn base_is_empty_when_already_valid() {
        let t = tree(&["meshes/a.nif", "MyMod.esp"]);
        assert_eq!(t.simple_archive_base(rules()).as_deref(), Some(""));
    }

    #[test]
    fn base_is_none_when_not_a_mod() {
        // A single wrapper of only docs is not a Bethesda mod root.
        let t = tree(&["MyMod/readme.txt", "MyMod/screenshot.png"]);
        assert_eq!(t.simple_archive_base(rules()), None);
    }

    #[test]
    fn case_insensitive_folder_match() {
        assert_eq!(tree(&["MESHES/a.nif"]).data_looks_valid(rules()), CheckReturn::Valid);
        assert_eq!(tree(&["x/SKSE/Plugins/y.dll"]).simple_archive_base(rules()).as_deref(), Some("x/"));
    }

    #[test]
    fn guess_mod_name_strips_nexus_suffix() {
        assert_eq!(
            guess_mod_name("/dl/Expressive Facial Animation - Female Edition-19181-1-7-1575746557.7z"),
            "Expressive Facial Animation - Female Edition"
        );
        assert_eq!(guess_mod_name("TrueHUD-62775-1-1-9-1703382929.7z"), "TrueHUD");
        assert_eq!(guess_mod_name("SkyUI_5_1.7z"), "SkyUI_5_1");

        // Lettered version segments (9b, 2SE) must also strip, and the mod id come out.
        assert_eq!(
            guess_mod_name_and_id("Unofficial Skyrim Special Edition Patch-266-4-2-9b-1700000000.7z"),
            ("Unofficial Skyrim Special Edition Patch".to_string(), Some(266))
        );
        assert_eq!(
            guess_mod_name_and_id("SkyUI_5_2_SE-12604-5-2SE.7z"),
            ("SkyUI_5_2_SE".to_string(), Some(12604))
        );
        // A non-Nexus name has no id.
        assert_eq!(guess_mod_name_and_id("SkyUI_5_1.7z"), ("SkyUI_5_1".to_string(), None));
    }

    #[test]
    fn data_text_archive_is_simple() {
        // Data/ + loose docs: MO2 installs it (DataText), descending into Data.
        let t = tree(&["Data/meshes/a.nif", "Data/MyMod.esp", "readme.txt", "preview.png"]);
        assert_eq!(t.simple_archive_base(rules()).as_deref(), Some("Data/"));
        // A non-doc, non-mod sibling beside Data is NOT the DataText pattern.
        let u = tree(&["Data/MyMod.esp", "loose.dll"]);
        assert_eq!(u.simple_archive_base(rules()), None);
    }

    #[test]
    fn bain_detects_numbered_subpackages() {
        // The canonical Wrye Bash complex package: numbered top-level folders, each
        // Data-relative on its own.
        let t = tree(&[
            "00 Core/meshes/a.nif",
            "00 Core/MyMod.esp",
            "01 Optional Textures/textures/b.dds",
            "10 Alternate/MyMod.esp",
        ]);
        let (subs, invalid) = t.bain_subpackages(rules());
        assert_eq!(subs, vec!["00 Core", "01 Optional Textures", "10 Alternate"]);
        assert_eq!(invalid, 0);
        // Original casing is preserved (the folder must be findable on disk), and the
        // order is the archive's, which is the merge order (later wins).
        assert!(subs.len() >= BAIN_MIN_SUBPACKAGES);
        // A BAIN package is NOT a simple archive - that is why the fallback exists.
        assert_eq!(t.simple_archive_base(rules()), None);
    }

    #[test]
    fn bain_skips_ignored_and_disabled_folders() {
        // fomod/images/screenshots/docs/omod conversion data are packaging cruft, and
        // `--` marks a sub-package the author disabled: none may be offered or counted
        // as invalid. `package.txt` is a file, so it is not a candidate at all.
        let t = tree(&[
            "00 Core/meshes/a.nif",
            "01 Extras/MyMod.esp",
            "--02 Disabled/textures/x.dds",
            "Docs/readme.txt",
            "Images/preview.png",
            "Screenshots/shot.png",
            "OMOD Conversion Data/script.txt",
            "fomod/info.xml",
            "package.txt",
        ]);
        let (subs, invalid) = t.bain_subpackages(rules());
        assert_eq!(subs, vec!["00 Core", "01 Extras"]);
        assert_eq!(invalid, 0, "skipped folders must not count as invalid either");
    }

    #[test]
    fn bain_counts_invalid_candidates_without_deciding() {
        // Two valid sub-packages beside a folder that is not one: MO2 does not
        // classify here, it asks. We report the count and let the caller prompt.
        let t = tree(&[
            "00 Core/MyMod.esp",
            "01 Optional/textures/b.dds",
            "Utilities/BuildScript/thing.exe",
        ]);
        let (subs, invalid) = t.bain_subpackages(rules());
        assert_eq!(subs, vec!["00 Core", "01 Optional"]);
        assert_eq!(invalid, 1);
    }

    #[test]
    fn bain_is_structural_not_a_numeric_prefix_guess() {
        // Numbered folders that hold no mod data are NOT sub-packages...
        let t = tree(&["00 Screens/a.png", "01 More Screens/b.png"]);
        let (subs, invalid) = t.bain_subpackages(rules());
        assert!(subs.is_empty());
        assert_eq!(invalid, 2);
        // ...and unnumbered folders that DO hold mod data are.
        let u = tree(&["Core/MyMod.esp", "Optional/meshes/a.nif"]);
        assert_eq!(u.bain_subpackages(rules()).0, vec!["Core", "Optional"]);
    }

    #[test]
    fn bain_does_not_claim_a_fomod() {
        // A scripted installer's own layout: the fomod dir plus Data-relative folders
        // that are not independently valid. Nothing here is a sub-package, so the
        // FOMOD keeps its (higher-priority) installer.
        let t = tree(&[
            "fomod/ModuleConfig.xml",
            "fomod/info.xml",
            "textures/x.dds",
            "meshes/y.nif",
            "MyMod.esp",
        ]);
        let (subs, _) = t.bain_subpackages(rules());
        assert!(subs.is_empty(), "a FOMOD must not be offered as BAIN");
        // Note a COMBINED fomod/bain package does have valid sub-packages; MO2 (and
        // `open_archive`) resolve that by priority - FOMOD is checked first.
    }

    #[test]
    fn bain_does_not_claim_a_simple_archive() {
        // A plain Data-relative mod: `meshes`/`textures` are content, not
        // sub-packages, and the archive is simple anyway.
        let t = tree(&["meshes/a.nif", "textures/b.dds", "MyMod.esp"]);
        assert_eq!(t.simple_archive_base(rules()).as_deref(), Some(""));
        let (subs, _) = t.bain_subpackages(rules());
        assert!(subs.len() < BAIN_MIN_SUBPACKAGES);
        // The classic wrapped archive is not BAIN either (one candidate at most).
        let u = tree(&["MyMod-1234/meshes/a.nif"]);
        assert_eq!(u.bain_subpackages(rules()).0.len(), 1);
    }

    #[test]
    fn bain_default_selection_ticks_00_and_previous() {
        let subs = vec!["00 Core".to_string(), "01 Extras".to_string(), "02 Alt".to_string()];
        assert_eq!(bain_default_selection(&subs, &[]), vec![true, false, false]);
        // A remembered choice comes back ticked, matched case-insensitively.
        assert_eq!(
            bain_default_selection(&subs, &["02 alt".to_string()]),
            vec![true, false, true]
        );
    }

    #[test]
    fn subtree_and_root_validity_drive_the_manual_picker() {
        // NB "Utilities", not "Tools": `tools` is itself a recognised Data folder, so a
        // top-level one would make the archive root look valid.
        let t = tree(&["Utilities/thing.exe", "Package/Data/meshes/a.nif", "Package/Data/MyMod.esp"]);
        // The archive root is not a mod root, nor is a random folder...
        assert!(!t.root_looks_valid("", rules()));
        assert!(!t.root_looks_valid("Utilities", rules()));
        // ...but the nested Data the user would point at is (case-insensitively).
        assert!(t.root_looks_valid("Package/Data", rules()));
        assert!(t.root_looks_valid("package/data", rules()));
        // A file or a missing path never resolves.
        assert!(t.subtree("Utilities/thing.exe").is_none());
        assert!(t.subtree("nope").is_none());
        assert!(!t.root_looks_valid("nope", rules()));
    }

    #[test]
    fn flatten_yields_depth_paths_for_a_tree_view() {
        let rows = tree(&["Data/meshes/a.nif", "readme.txt"]).flatten();
        let got: Vec<(usize, &str, &str, bool)> =
            rows.iter().map(|r| (r.depth, r.name.as_str(), r.path.as_str(), r.is_dir)).collect();
        assert_eq!(
            got,
            vec![
                (0, "Data", "Data", true),
                (1, "meshes", "Data/meshes", true),
                (2, "a.nif", "Data/meshes/a.nif", false),
                (0, "readme.txt", "readme.txt", false),
            ]
        );
        // Every listed directory path must round-trip through `subtree` - that is the
        // contract the picker relies on when the user right-clicks a row.
        for r in rows.iter().filter(|r| r.is_dir) {
            assert!(tree(&["Data/meshes/a.nif", "readme.txt"]).subtree(&r.path).is_some());
        }
    }

    #[test]
    fn flatten_is_bounded_on_a_pathological_nesting() {
        // A hostile archive must degrade (rows past the cap are dropped), never
        // recurse without end.
        let deep = (0..MAX_TREE_DEPTH + 20).map(|i| format!("d{i}")).collect::<Vec<_>>().join("/");
        let t = tree(&[&format!("{deep}/file.txt")]);
        assert_eq!(t.flatten().len(), MAX_TREE_DEPTH);
    }

    #[test]
    fn fix_directory_name_strips_illegal_chars() {
        assert_eq!(fix_directory_name("Beyond Skyrim: Bruma").as_deref(), Some("Beyond Skyrim Bruma"));
        assert_eq!(fix_directory_name("  A  B  ").as_deref(), Some("A B")); // collapse + trim
        assert_eq!(fix_directory_name("trailing... ").as_deref(), Some("trailing"));
        assert_eq!(fix_directory_name(r"a/b\c<d>e").as_deref(), Some("abcde"));
        assert_eq!(fix_directory_name("   "), None);
        assert_eq!(fix_directory_name(":::"), None);
    }

    #[test]
    fn classify_content_detects_kinds_shallow() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir()
            .join(format!("eidos-content-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(root.join("Textures")).unwrap();
        std::fs::create_dir_all(root.join("Meshes/FaceGenData")).unwrap();
        std::fs::create_dir_all(root.join("SKSE/Plugins")).unwrap();
        std::fs::write(root.join("MyMod.esp"), b"").unwrap();
        std::fs::write(root.join("MyMod.bsa"), b"").unwrap();
        std::fs::write(root.join("meta.ini"), b"").unwrap();

        let c = classify_content_dir(&root);
        assert!(c.plugins && c.bsa && c.textures && c.meshes && c.skse && c.facegen);
        assert!(!c.sound && !c.interface);
        // Tags string in MO2 icon order, space-separated.
        assert_eq!(c.tags(), "P A T M K F");
        // An empty (or separator) folder has no content.
        let empty = root.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert_eq!(classify_content_dir(&empty).tags(), "");
        let _ = std::fs::remove_dir_all(&root);
    }
}
