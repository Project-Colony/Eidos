//! Mod installer for Eidos, porting Mod Organizer 2's installer logic.
//!
//! The hard part of "install a downloaded archive" is not unzipping - it is
//! figuring out the `Data`-relative root inside an archive that may be wrapped in
//! one or more useless folders (`ModName-1234/...`). MO2 solves this with the
//! Simple installer (`InstallerQuick::getSimpleArchiveBase`) driven by a per-game
//! `ModDataChecker`. This module reproduces both as pure, testable logic over an
//! [`ArchiveTree`]; the archive backend and the extraction wiring live alongside.
//!
//! Tier 1 (here): archive tree + Gamebryo `ModDataChecker` + wrapper-strip.
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

    /// MO2's `ModDataChecker::dataLooksValid` for Gamebryo games: this level is a
    /// valid mod root if a top-level entry is a known Data folder or a known
    /// plugin/archive file. (Per-game checkers can specialise this later.)
    pub fn data_looks_valid(&self) -> CheckReturn {
        for (key, node) in &self.entries {
            match node {
                TreeNode::Dir { .. } if GAMEBRYO_FOLDERS.contains(&key.as_str()) => {
                    return CheckReturn::Valid;
                }
                TreeNode::File { .. } => {
                    if let Some(ext) = key.rsplit_once('.').map(|(_, e)| e) {
                        if GAMEBRYO_SUFFIXES.contains(&ext) {
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
    pub fn simple_archive_base(&self) -> Option<String> {
        fn rec(tree: &ArchiveTree, prefix: &mut String) -> bool {
            if tree.data_looks_valid() == CheckReturn::Valid {
                return true;
            }
            if let Some((name, sub)) = tree.single_subdir() {
                prefix.push_str(name);
                prefix.push('/');
                return rec(sub, prefix);
            }
            // MO2's DataText layer: a sole `Data` dir beside loose docs - descend
            // into Data (the docs are not mod content and are dropped here).
            if let Some((name, sub)) = tree.data_text_subdir() {
                prefix.push_str(name);
                prefix.push('/');
                return rec(sub, prefix);
            }
            false
        }
        let mut prefix = String::new();
        if rec(self, &mut prefix) {
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
    pub fn bain_subpackages(&self) -> (Vec<String>, usize) {
        let mut valid = Vec::new();
        let mut invalid = 0usize;
        for (key, node) in &self.entries {
            // Only directories are candidates; a top-level `package.txt` or readme is
            // BAIN metadata, not a sub-package.
            let TreeNode::Dir { name, tree } = node else { continue };
            if BAIN_IGNORED_FOLDERS.contains(&key.as_str()) || key.starts_with("--") {
                continue;
            }
            if tree.data_looks_valid() == CheckReturn::Valid {
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
    pub fn root_looks_valid(&self, path: &str) -> bool {
        self.subtree(path).is_some_and(|t| t.data_looks_valid() == CheckReturn::Valid)
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

    fn tree(files: &[&str]) -> ArchiveTree {
        let entries: Vec<ArchiveEntry> =
            files.iter().map(|p| ArchiveEntry { path: p.to_string(), is_dir: p.ends_with('/') }).collect();
        ArchiveTree::from_entries(&entries)
    }

    #[test]
    fn valid_when_top_level_is_a_data_folder() {
        assert_eq!(tree(&["meshes/armor/a.nif"]).data_looks_valid(), CheckReturn::Valid);
    }

    #[test]
    fn valid_when_top_level_has_a_plugin() {
        assert_eq!(tree(&["MyMod.esp", "readme.txt"]).data_looks_valid(), CheckReturn::Valid);
    }

    #[test]
    fn invalid_when_wrapped_in_a_useless_folder() {
        // The classic Nexus wrapper: nothing useful at the top level.
        assert_eq!(tree(&["MyMod-1234/meshes/a.nif"]).data_looks_valid(), CheckReturn::Invalid);
    }

    #[test]
    fn base_strips_a_single_wrapper() {
        let t = tree(&["MyMod-1234/meshes/a.nif", "MyMod-1234/textures/b.dds"]);
        assert_eq!(t.simple_archive_base().as_deref(), Some("MyMod-1234/"));
    }

    #[test]
    fn base_strips_nested_wrappers() {
        let t = tree(&["a/b/scripts/x.pex", "a/b/MyMod.esp"]);
        assert_eq!(t.simple_archive_base().as_deref(), Some("a/b/"));
    }

    #[test]
    fn base_is_empty_when_already_valid() {
        let t = tree(&["meshes/a.nif", "MyMod.esp"]);
        assert_eq!(t.simple_archive_base().as_deref(), Some(""));
    }

    #[test]
    fn base_is_none_when_not_a_mod() {
        // A single wrapper of only docs is not a Bethesda mod root.
        let t = tree(&["MyMod/readme.txt", "MyMod/screenshot.png"]);
        assert_eq!(t.simple_archive_base(), None);
    }

    #[test]
    fn case_insensitive_folder_match() {
        assert_eq!(tree(&["MESHES/a.nif"]).data_looks_valid(), CheckReturn::Valid);
        assert_eq!(tree(&["x/SKSE/Plugins/y.dll"]).simple_archive_base().as_deref(), Some("x/"));
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
        assert_eq!(t.simple_archive_base().as_deref(), Some("Data/"));
        // A non-doc, non-mod sibling beside Data is NOT the DataText pattern.
        let u = tree(&["Data/MyMod.esp", "loose.dll"]);
        assert_eq!(u.simple_archive_base(), None);
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
        let (subs, invalid) = t.bain_subpackages();
        assert_eq!(subs, vec!["00 Core", "01 Optional Textures", "10 Alternate"]);
        assert_eq!(invalid, 0);
        // Original casing is preserved (the folder must be findable on disk), and the
        // order is the archive's, which is the merge order (later wins).
        assert!(subs.len() >= BAIN_MIN_SUBPACKAGES);
        // A BAIN package is NOT a simple archive - that is why the fallback exists.
        assert_eq!(t.simple_archive_base(), None);
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
        let (subs, invalid) = t.bain_subpackages();
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
        let (subs, invalid) = t.bain_subpackages();
        assert_eq!(subs, vec!["00 Core", "01 Optional"]);
        assert_eq!(invalid, 1);
    }

    #[test]
    fn bain_is_structural_not_a_numeric_prefix_guess() {
        // Numbered folders that hold no mod data are NOT sub-packages...
        let t = tree(&["00 Screens/a.png", "01 More Screens/b.png"]);
        let (subs, invalid) = t.bain_subpackages();
        assert!(subs.is_empty());
        assert_eq!(invalid, 2);
        // ...and unnumbered folders that DO hold mod data are.
        let u = tree(&["Core/MyMod.esp", "Optional/meshes/a.nif"]);
        assert_eq!(u.bain_subpackages().0, vec!["Core", "Optional"]);
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
        let (subs, _) = t.bain_subpackages();
        assert!(subs.is_empty(), "a FOMOD must not be offered as BAIN");
        // Note a COMBINED fomod/bain package does have valid sub-packages; MO2 (and
        // `open_archive`) resolve that by priority - FOMOD is checked first.
    }

    #[test]
    fn bain_does_not_claim_a_simple_archive() {
        // A plain Data-relative mod: `meshes`/`textures` are content, not
        // sub-packages, and the archive is simple anyway.
        let t = tree(&["meshes/a.nif", "textures/b.dds", "MyMod.esp"]);
        assert_eq!(t.simple_archive_base().as_deref(), Some(""));
        let (subs, _) = t.bain_subpackages();
        assert!(subs.len() < BAIN_MIN_SUBPACKAGES);
        // The classic wrapped archive is not BAIN either (one candidate at most).
        let u = tree(&["MyMod-1234/meshes/a.nif"]);
        assert_eq!(u.bain_subpackages().0.len(), 1);
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
        assert!(!t.root_looks_valid(""));
        assert!(!t.root_looks_valid("Utilities"));
        // ...but the nested Data the user would point at is (case-insensitively).
        assert!(t.root_looks_valid("Package/Data"));
        assert!(t.root_looks_valid("package/data"));
        // A file or a missing path never resolves.
        assert!(t.subtree("Utilities/thing.exe").is_none());
        assert!(t.subtree("nope").is_none());
        assert!(!t.root_looks_valid("nope"));
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
