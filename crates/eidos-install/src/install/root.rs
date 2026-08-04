//! Laying out a Root Builder split: the Data half becomes the mod root, the
//! rest lands under `Root/` at its own path.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};


use crate::RootSplit;


use super::*;

/// The mod-folder name Eidos projects onto the game install root at launch. Matched
/// case-insensitively by `Instance::root_layers`, so the casing here is cosmetic -
/// but it is MO2's Root Builder spelling, which is what a user coming from MO2 (or
/// reading a mod's install instructions) expects to see in the mod folder.
pub(crate) const ROOT_DIR_NAME: &str = "Root";

/// The two halves of a Root Builder archive, resolved to real paths inside the
/// extraction temp. Resolved BEFORE the destructive step, like every other install
/// path here: nothing is wiped until we know both halves are on disk.
pub(crate) struct RootSources {
    /// The subtree that becomes the mod root (its contents are `Data`-relative).
    /// `None` for a pure root mod, which has no Data half to place.
    pub(crate) data: Option<PathBuf>,
    /// Directories whose contents are overlaid onto the mod root ON TOP of `data`.
    /// This is where an archive's `Root/Data/` goes - see [`resolve_root_split`].
    pub(crate) data_extra: Vec<PathBuf>,
    /// What lands in the mod's `Root/`, as (path inside `Root/`, source) pairs.
    /// An archive's own `Root` directory contributes its CONTENTS; every other
    /// entry contributes itself, at the same relative path it had in the archive.
    ///
    /// The path is carried rather than derived from the source's file name because
    /// an entry is not always top-level: an archive addressing the install root
    /// contributes `SB/Binaries`, which has to land at `Root/SB/Binaries` and not
    /// at `Root/Binaries`.
    pub(crate) root: Vec<(String, PathBuf)>,
}

/// Resolve a [`RootSplit`] against the extraction temp.
///
/// Fallible on purpose, and every check here runs BEFORE the caller's destructive
/// step - the same contract as [`resolve_bain_sources`] and [`resolve_manual_root`].
/// An earlier version could not fail: it fell back to the whole extraction temp when
/// the Data half did not resolve, which made a Replace wipe the old mod and then die
/// half-way through with the mod gone and the message saying only `ENOENT`.
///
/// The Data half can genuinely fail to resolve even though the tree says it is a
/// directory: [`ArchiveTree::from_dir`] classifies with `Path::is_dir`, which follows
/// symlinks, while [`is_real_dir`] does not - so an archive shipping `Data` as a
/// symlink is a Dir in the tree and not a directory on disk.
pub(crate) fn resolve_root_split(tmp: &Path, split: &RootSplit) -> Result<RootSources, InstallError> {
    let refuse = |what: &str| {
        InstallError::BadSelection(format!("cannot lay out this archive: {what}"))
    };

    let data = match split.data_prefix.as_deref() {
        Some(p) => Some(
            resolve_ci(tmp, p.trim_matches(['/', '\\'].as_slice()))
                .filter(|p| is_real_dir(p))
                .ok_or_else(|| refuse(&format!("'{p}' is not a real directory")))?,
        ),
        None => None,
    };

    let mut root = Vec::new();
    let mut data_extra = Vec::new();
    if let Some(dir) = &split.root_dir {
        // The archive already used the convention: take what is INSIDE it, so the
        // result is `<mod>/Root/x.dll`, not `<mod>/Root/Root/x.dll`.
        let p = resolve_ci(tmp, dir)
            .filter(|p| is_real_dir(p))
            .ok_or_else(|| refuse(&format!("'{dir}' is not a real directory")))?;
        let rd = fs::read_dir(&p).map_err(|e| refuse(&format!("'{dir}': {e}")))?;
        for e in rd.flatten() {
            let path = e.path();
            // `Root/Data/` is a legitimate Root Builder layout - Root Builder maps
            // `Root/` onto the game folder, and the game folder's child IS `Data` -
            // and it is how a repackaged script extender ships. Left in `Root/` it
            // would be INVISIBLE here: the root union puts it at `<game>/Data`, and
            // the Data union is mounted over exactly that path, shadowing it. So it
            // joins the Data half, which is the layer that actually serves it.
            let is_data = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("data"));
            if is_data && is_real_dir(&path) {
                data_extra.push(path);
            } else {
                let name = e.file_name().to_string_lossy().into_owned();
                root.push((name, path));
            }
        }
    }
    for n in &split.root_entries {
        // Never skip: dropping an entry here is the exact bug this whole path was
        // written to fix, so a miss is a refusal, not a silent loss.
        let src = resolve_ci(tmp, n).ok_or_else(|| refuse(&format!("'{n}' is missing")))?;
        root.push((n.clone(), src));
    }

    // Two sources landing on the same name in `Root/` - a loose `notes` file beside
    // the archive's own `Root/notes/` - would have one clobber the other, or abort
    // the install mid-way on a type mismatch. Neither is ours to choose.
    let mut seen = std::collections::BTreeSet::new();
    for (rel, _) in &root {
        let key = rel.trim_matches('/').to_ascii_lowercase();
        if key.is_empty() {
            return Err(refuse("an entry has no name"));
        }
        if !seen.insert(key.clone()) {
            return Err(refuse(&format!("two entries would both become Root/{key}")));
        }
    }

    // An archive that resolves to nothing at all must not reach the wipe: a Replace
    // would delete the old mod and install an empty folder, reporting success.
    if data.is_none() && root.is_empty() && data_extra.is_empty() {
        return Err(refuse("it contains no installable content"));
    }
    Ok(RootSources { data, data_extra, root })
}

/// Lay a resolved Root Builder split into `dest`: the Data half becomes the mod
/// folder, the root half becomes `<mod>/Root/`.
pub(crate) fn place_root_split(src: &RootSources, dest: &Path, merging: bool) -> io::Result<()> {
    if let Some(data) = &src.data {
        if merging {
            copy_dir_all(data, dest)?;
        } else {
            move_dir_contents(data, dest)?;
        }
    }
    // On top of the Data half, never under it: an archive that ships both is saying
    // the `Root/Data` copy is the one to use.
    for extra in &src.data_extra {
        overlay_dir(extra, dest)?;
    }
    if src.root.is_empty() {
        return Ok(());
    }
    let root_dest = dest.join(ROOT_DIR_NAME);
    // The Data half may have just planted a FILE named `Root` here, and this runs
    // after the Replace wipe, so an EEXIST would take the old mod down with it.
    clear_non_dir(&root_dest)?;
    fs::create_dir_all(&root_dest)?;
    for (rel, from) in &src.root {
        let to = root_dest.join(rel.trim_matches('/'));
        // An entry can be nested (`SB/Binaries`), so its parent may not exist yet.
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        if is_real_dir(from) {
            clear_non_dir(&to)?;
            overlay_dir(from, &to)?;
            continue;
        }
        // Whatever occupies the name loses, whichever type it is - `remove_file` on
        // a directory is EISDIR, which used to abort the install after the wipe.
        // `symlink_metadata` does not follow, so a dangling link still counts as an
        // occupant and is removed rather than written through.
        match fs::symlink_metadata(&to).map(|m| m.file_type()) {
            Ok(t) if t.is_dir() => fs::remove_dir_all(&to)?,
            Ok(_) => fs::remove_file(&to)?,
            Err(_) => {}
        }
        if merging {
            fs::copy(from, &to)?;
        } else {
            fs::rename(from, &to)?;
        }
    }
    Ok(())
}
