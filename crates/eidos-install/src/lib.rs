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

mod install;
pub use install::{
    finish_fomod, install_archive, mod_name_for, open_fomod, FomodSession, InstallError,
    InstallReport,
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
    fn fix_directory_name_strips_illegal_chars() {
        assert_eq!(fix_directory_name("Beyond Skyrim: Bruma").as_deref(), Some("Beyond Skyrim Bruma"));
        assert_eq!(fix_directory_name("  A  B  ").as_deref(), Some("A B")); // collapse + trim
        assert_eq!(fix_directory_name("trailing... ").as_deref(), Some("trailing"));
        assert_eq!(fix_directory_name(r"a/b\c<d>e").as_deref(), Some("abcde"));
        assert_eq!(fix_directory_name("   "), None);
        assert_eq!(fix_directory_name(":::"), None);
    }
}
