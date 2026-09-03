//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{ArchiveEntry, ArchiveTree, LayoutRules, BAIN_MIN_SUBPACKAGES, MAX_TREE_DEPTH};

mod extract;
mod fomod;
mod fsops;
mod meta;
mod picker;
mod root;
mod simple;
#[cfg(test)]
mod tests;

pub use extract::*;
pub use fomod::*;
use fsops::*;
pub use meta::*;
pub use picker::*;
use root::*;
pub use simple::*;

#[derive(Debug)]
pub enum InstallError {
    /// No 7-Zip binary found (`7z` / `7zz` / `7za`).
    No7z,
    /// Extraction failed (stderr).
    Extract(String),
    /// Not a "simple" archive: no Data-relative root found.
    NotSimple,
    /// A FOMOD scripted-installer archive (Tier 2, not yet supported).
    NeedsFomod,
    /// A FOMOD's `ModuleConfig.xml` could not be parsed.
    Fomod(String),
    /// The FOMOD's `<moduleDependencies>` are not satisfied; MO2 refuses the install.
    /// The string describes what the mod requires.
    UnmetDependency(String),
    /// The target `mods/<name>/` already exists and is not empty.
    Exists(PathBuf),
    /// A BAIN sub-package set or a manual data root the archive does not actually
    /// contain (the front end's pick went stale, or was never valid).
    BadSelection(String),
    Io(io::Error),
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallError::No7z => write!(f, "no 7-Zip binary found (install p7zip: 7z/7zz/7za)"),
            InstallError::Extract(e) => write!(f, "extraction failed: {e}"),
            InstallError::NotSimple => {
                write!(
                    f,
                    "not a simple archive (no recognised Data layout); manual install needed"
                )
            }
            InstallError::NeedsFomod => {
                write!(
                    f,
                    "this is a FOMOD scripted installer - not yet supported (Tier 2)"
                )
            }
            InstallError::Fomod(e) => write!(f, "FOMOD parse error: {e}"),
            InstallError::UnmetDependency(d) => {
                write!(f, "this mod's requirements are not met: {d}")
            }
            InstallError::Exists(p) => write!(f, "target already exists: {}", p.display()),
            InstallError::BadSelection(s) => write!(f, "invalid selection: {s}"),
            InstallError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for InstallError {
    fn from(e: io::Error) -> Self {
        InstallError::Io(e)
    }
}

/// What an install produced.
#[derive(Debug)]
pub struct InstallReport {
    pub name: String,
    /// The wrapper prefix that was stripped (empty if the archive was already
    /// Data-relative).
    pub stripped: String,
    /// Whether this was installed via the FOMOD scripted installer (default options).
    pub fomod: bool,
    /// FOMOD plan sources the archive did not actually contain (skipped, non-fatal).
    pub missing: Vec<String>,
    pub dest: PathBuf,
}

impl ArchiveTree {
    /// Build the tree by walking an extracted directory.
    ///
    /// Symlinked directories ARE followed - an archive may legitimately ship
    /// `Data` as a symlink, and refusing to descend reclassified such archives
    /// from Simple to Manual and dropped whole subtrees from BAIN and root
    /// detection (a first version of the loop guard did exactly that). What makes
    /// following safe is the [`MAX_TREE_DEPTH`] cap alone: a crafted `link -> ..`
    /// used to recurse until the stack gave out and took the GUI with it - a
    /// SIGSEGV from untrusted input - and the cap turns that into a bounded,
    /// harmless walk, the same defence `flatten` and `overlay_dir` already use.
    ///
    /// The kind comes from the dirent (no `stat(2)` per entry, which on a
    /// 30k-entry archive is 30k syscalls); only an actual symlink pays the extra
    /// stat to learn what it points at.
    pub fn from_dir(root: &Path) -> io::Result<ArchiveTree> {
        fn walk(
            base: &Path,
            dir: &Path,
            out: &mut Vec<ArchiveEntry>,
            depth: usize,
        ) -> io::Result<()> {
            if depth > MAX_TREE_DEPTH {
                return Ok(());
            }
            for e in fs::read_dir(dir)?.flatten() {
                let p = e.path();
                let Ok(t) = e.file_type() else { continue };
                let is_dir = if t.is_symlink() {
                    p.is_dir()
                } else {
                    t.is_dir()
                };
                if let Ok(rel) = p.strip_prefix(base) {
                    out.push(ArchiveEntry {
                        path: rel.to_string_lossy().replace('\\', "/"),
                        is_dir,
                    });
                }
                if is_dir {
                    walk(base, &p, out, depth + 1)?;
                }
            }
            Ok(())
        }
        let mut entries = Vec::new();
        walk(root, root, &mut entries, 0)?;
        Ok(ArchiveTree::from_entries(&entries))
    }
}

/// How to handle an existing `mods/<name>/` (MO2's overwrite prompt): fail, replace
/// (wipe + reinstall, keeping the user's meta.ini state), merge (install over the
/// existing files), or install under a different name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum OverwritePolicy {
    #[default]
    Fail,
    Replace,
    Merge,
    Rename(String),
}

/// What an archive turned out to be once extracted (see [`open_archive`]).
///
/// The order the classifier tries these in is MO2's installer priority: FOMOD (90)
/// beats Simple (50) beats BAIN (40) beats Manual (0). It matters - a combined
/// FOMOD/BAIN package must run its scripted installer, and an archive whose option
/// folders happen to look like sub-packages must not be hijacked from the wizard.
pub enum Opened {
    /// A FOMOD scripted installer: drive the wizard, then [`finish_fomod`].
    Fomod(Box<FomodSession>),
    /// A plain archive, already extracted: install it with [`install_extracted`].
    Simple(ExtractedTree),
    /// A Wrye Bash complex (BAIN) package: let the user tick sub-packages, then
    /// [`install_bain`].
    Bain {
        tree: ExtractedTree,
        /// Sub-package folder names in archive order, which is also the merge order
        /// (later wins). Always at least [`BAIN_MIN_SUBPACKAGES`] long.
        subpackages: Vec<String>,
        /// Top-level folders that were candidates but did not look like mod roots.
        /// Non-zero means "probably BAIN, but ask" - MO2 prompts here rather than
        /// classify, because `Data/` beside `Extras/` looks the same from outside.
        invalid: usize,
    },
    /// Nothing recognised the layout. The escape hatch: show the tree, let the user
    /// point at the data root, then [`install_manual`]. No archive is un-installable.
    Manual(ExtractedTree),
}

/// Extract `archive` once and classify it: a FOMOD (whose `config` drives the
/// wizard), a simple archive, a BAIN package (whose sub-packages the user ticks) or
/// an unrecognised layout the user must resolve by hand. The extracted tree rides
/// along in every case, so installing it costs no second extraction.
///
/// `game_id` is the Eidos game id (`skyrimse`), not MO2's short name: it selects
/// the [`LayoutRules`] the classification runs under, so passing the wrong spelling
/// silently falls back to the Gamebryo vocabulary instead of erroring.
pub fn open_archive(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
) -> Result<Opened, InstallError> {
    open_archive_with(archive, mods_dir, name, game_id, |_| {})
}

/// [`open_archive`], with 7-Zip's live percentage fed to `on_progress`.
///
/// Extraction dwarfs everything else this function does, so its percentage IS
/// the operation's percentage: a front end can drive a progress bar from this
/// callback alone and treat the classification after 100% as "finishing".
pub fn open_archive_with(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    on_progress: impl FnMut(u8),
) -> Result<Opened, InstallError> {
    let tree = extract_to_temp_with(archive, mods_dir, on_progress)?;
    if let Some(root) = find_fomod_root(&tree.tmp) {
        let config = parse_fomod_at(&root)?;
        return Ok(Opened::Fomod(Box::new(FomodSession {
            config,
            root,
            tree,
            name: name.to_string(),
            archive: archive.to_path_buf(),
        })));
    }
    // MO2's priority order, see `Opened`. Reading the extracted layout costs one
    // directory walk against an extraction that already paid for the whole archive.
    let rules = LayoutRules::for_game(game_id);
    let layout = ArchiveTree::from_dir(&tree.tmp)?;
    if layout.simple_archive_base(rules).is_some() {
        return Ok(Opened::Simple(tree));
    }
    let (subpackages, invalid) = layout.bain_subpackages(rules);
    if subpackages.len() >= BAIN_MIN_SUBPACKAGES {
        return Ok(Opened::Bain {
            tree,
            subpackages,
            invalid,
        });
    }
    // A Root Builder archive needs no question asked - the split is structural -
    // so it takes the Simple path, which `install_extracted` then lays out. Left to
    // fall through, it would reach the manual picker, whose contract is to drop
    // everything beside the chosen root: exactly the root half.
    if layout.root_builder_split(rules).is_some() {
        return Ok(Opened::Simple(tree));
    }
    Ok(Opened::Manual(tree))
}
