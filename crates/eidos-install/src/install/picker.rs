//! The flows a human drives: BAIN sub-package selection, and the manual
//! choose-the-data-folder escape hatch.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::path::{Path, PathBuf};

use super::*;

/// Resolve chosen BAIN sub-package names to directories inside the extraction temp,
/// in the order given. Every fallible check lives here so a caller can run it BEFORE
/// the destructive step and never wipe a mod over a stale pick.
pub(crate) fn resolve_bain_sources(
    tmp: &Path,
    chosen: &[String],
) -> Result<Vec<PathBuf>, InstallError> {
    if chosen.is_empty() {
        return Err(InstallError::BadSelection(
            "no sub-package selected".to_string(),
        ));
    }
    let mut out = Vec::with_capacity(chosen.len());
    for name in chosen {
        // A sub-package is one top-level folder name. Anything with a separator or a
        // relative segment is not one and must never be joined blindly: the list
        // comes from a front end, not necessarily from `bain_subpackages`.
        if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
            return Err(InstallError::BadSelection(format!(
                "not a sub-package name: '{name}'"
            )));
        }
        let exact = tmp.join(name);
        let dir = if is_real_dir(&exact) {
            exact
        } else {
            // The extraction may have case-folded a colliding name (see
            // `normalize_case_collisions`), so fall back to a case-insensitive match.
            find_ci(tmp, &name.to_ascii_lowercase())
                .filter(|p| is_real_dir(p))
                .ok_or_else(|| {
                    InstallError::BadSelection(format!("no such sub-package: '{name}'"))
                })?
        };
        out.push(dir);
    }
    Ok(out)
}

/// Resolve a user-chosen data root (a `/`-joined prefix inside the archive, `""` for
/// the archive root) to a directory inside the extraction temp. `resolve_ci` already
/// refuses a `..` segment, so a hand-typed root cannot escape the temp.
pub(crate) fn resolve_manual_root(tmp: &Path, root: &str) -> Result<PathBuf, InstallError> {
    let trimmed = root.trim_matches(['/', '\\'].as_slice());
    if trimmed.is_empty() {
        return Ok(tmp.to_path_buf());
    }
    resolve_ci(tmp, trimmed)
        .filter(|p| is_real_dir(p))
        .ok_or_else(|| {
            InstallError::BadSelection(format!("no such directory in the archive: '{root}'"))
        })
}

/// Install the chosen sub-packages of a BAIN (Wrye Bash complex) package, merged
/// **in the order given**: a later sub-package overwrites an earlier one's files,
/// which is BAIN's contract (`10 Optional Textures` is meant to win over `00 Core`).
/// Pass the names in the order [`bain_subpackages`](crate::ArchiveTree::bain_subpackages)
/// listed them, minus whatever the user unticked.
///
/// `tree` is the extraction from [`open_archive`], so nothing is unpacked twice.
/// Unknown or malformed names are refused with [`InstallError::BadSelection`] before
/// anything is written. A SUCCESSFUL install may consume the extraction (a lone
/// source is moved, not copied - it matters for a multi-GB texture pack), so treat
/// `tree` as spent afterwards; a failed one leaves it usable for a retry under
/// another policy, which is what the overwrite prompt needs.
pub fn install_bain(
    tree: &ExtractedTree,
    subpackages: &[String],
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
) -> Result<InstallReport, InstallError> {
    let sources = resolve_bain_sources(tree.path(), subpackages)?;
    install_sources(
        &sources,
        archive,
        mods_dir,
        name,
        game_id,
        policy,
        String::new(),
    )
}

/// Install from an explicit, user-chosen data root inside the archive - MO2's manual
/// installer, the escape hatch for a layout no detector recognises. `data_root` is a
/// `/`-joined prefix (`""` = the archive root, as returned in
/// [`TreeRow::path`](crate::TreeRow)); everything under it becomes the mod, everything
/// beside it is dropped.
///
/// The root is NOT required to look valid: MO2 warns and installs anyway if the user
/// insists, so a front end should call
/// [`ArchiveTree::root_looks_valid`](crate::ArchiveTree::root_looks_valid) to show the
/// warning and leave the decision to the user. Like [`install_bain`], a successful
/// install may consume `tree`.
pub fn install_manual(
    tree: &ExtractedTree,
    data_root: &str,
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
) -> Result<InstallReport, InstallError> {
    let src = resolve_manual_root(tree.path(), data_root)?;
    let stripped = if data_root.is_empty() {
        String::new()
    } else {
        format!("{}/", data_root.trim_matches(['/', '\\'].as_slice()))
    };
    install_sources(&[src], archive, mods_dir, name, game_id, policy, stripped)
}
