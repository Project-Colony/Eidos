//! MO2's Simple installer: extract, strip the wrapper, place, write meta -
//! plus the shared placement the other flows call into.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use eidos_instance::ModMeta;

use crate::{
    bain_default_selection, fix_directory_name, guess_mod_name_and_id, ArchiveTree, LayoutRules,
    BAIN_MIN_SUBPACKAGES,
};

use super::*;

/// Install `archive` into `mods_dir/name`, MO2 Simple-installer style: extract,
/// strip the wrapper folder to the Data-relative root, move it in, and write a
/// MO2-compatible `meta.ini`. Fails if the destination already exists; use
/// [`install_archive_with_policy`] to replace/merge/rename instead.
pub fn install_archive(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
) -> Result<InstallReport, InstallError> {
    install_archive_with_policy(
        archive,
        mods_dir,
        name,
        game_id,
        OverwritePolicy::Fail,
        &eidos_fomod::Context::default(),
    )
}

/// Like [`install_archive`] but with an explicit [`OverwritePolicy`] for an existing
/// `mods/<name>/` (MO2's merge / replace / rename / cancel) and a FOMOD install
/// [`Context`](eidos_fomod::Context) (current plugin states) so a scripted installer's
/// fileDependency/gameDependency conditions evaluate correctly. See [`fomod_context`].
pub fn install_archive_with_policy(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    ctx: &eidos_fomod::Context,
) -> Result<InstallReport, InstallError> {
    // A Fail collision needs no 7-Zip at all - check before paying for extraction.
    if policy == OverwritePolicy::Fail {
        if let Some(n) = collision_name(mods_dir, name) {
            return Err(InstallError::Exists(mods_dir.join(n)));
        }
    }
    let tree = extract_to_temp(archive, mods_dir)?;
    install_extracted(&tree, archive, mods_dir, name, game_id, policy, ctx)
}

/// Install an already-extracted archive (see [`open_archive`]), resolving a
/// destination collision per `policy`. Splitting this from the extraction is what
/// lets the GUI classify an archive once and install it without a second pass.
#[allow(clippy::too_many_arguments)]
pub fn install_extracted(
    tree: &ExtractedTree,
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    ctx: &eidos_fomod::Context,
) -> Result<InstallReport, InstallError> {
    // Sanitize the folder name (a real Nexus modName can contain ':' etc.) and
    // recover the mod id from the filename for the meta.ini when there's no sidecar.
    let mut name = fix_directory_name(name).unwrap_or_else(|| "Mod".to_string());
    let (_, guessed_id) = guess_mod_name_and_id(&archive.to_string_lossy());

    // Resolve a collision with the existing mod folder per the policy. Done before
    // extraction, so a Fail or a Rename-onto-another-existing needs no 7-Zip. The
    // Replace WIPE itself is deferred until the archive has extracted and its
    // layout/FOMOD plan resolved (the destructive step comes last, like MO2), so a
    // missing 7z, a corrupt archive or a bad FOMOD can never destroy the old mod.
    let mut dest = mods_dir.join(&name);
    let mut preserved: Option<ModMeta> = None;
    let mut replacing = false;
    if dest.exists() && is_nonempty_dir(&dest) {
        match &policy {
            OverwritePolicy::Fail => return Err(InstallError::Exists(dest)),
            OverwritePolicy::Merge => {} // install over the existing files
            OverwritePolicy::Replace => {
                // Keep the user's metadata (endorsement / category / tracked) across
                // the wipe, like MO2's REPLACE.
                preserved = Some(ModMeta::read(&dest.join("meta.ini")));
                replacing = true;
            }
            OverwritePolicy::Rename(new) => {
                name = fix_directory_name(new).unwrap_or_else(|| "Mod".to_string());
                dest = mods_dir.join(&name);
                if dest.exists() && is_nonempty_dir(&dest) {
                    return Err(InstallError::Exists(dest));
                }
            }
        }
    }
    let merging = policy == OverwritePolicy::Merge;
    // Whether `dest` is ours to clean up on failure (a fresh install, not a
    // merge/replace over a pre-existing mod folder).
    let fresh = !dest.exists();

    // The archive is already extracted (same filesystem as mods/, so the final
    // move is a rename); `tree` owns the temp and removes it when the caller
    // drops it.
    let tmp = tree.path();
    let rules = LayoutRules::for_game(game_id);

    let result = (|| {
        let layout = ArchiveTree::from_dir(tmp)?;
        let base = match layout.simple_archive_base(rules) {
            Some(b) => b,
            None => {
                // A FOMOD scripted installer: run it with the default selections.
                if let Some(fomod_root) = find_fomod_root(tmp) {
                    // Parse the module, check its dependencies and build the plan
                    // BEFORE the destructive step: every fallible stage runs while
                    // the old mod is still intact.
                    let config = parse_fomod_at(&fomod_root)?;
                    if let Some(req) = eidos_fomod::unmet_module_dependencies(&config, ctx) {
                        return Err(InstallError::UnmetDependency(req));
                    }
                    let plan = eidos_fomod::build_default_plan(&config, ctx);
                    if replacing {
                        fs::remove_dir_all(&dest)?;
                    }
                    fs::create_dir_all(&dest)?;
                    let missing = apply_plan(&fomod_root, &plan, &dest)?;
                    write_meta(archive, &dest, game_id, guessed_id)?;
                    return Ok(InstallReport {
                        name: name.clone(),
                        stripped: String::new(),
                        fomod: true,
                        missing,
                        dest: dest.clone(),
                    });
                }
                // A Wrye Bash complex package. This entry point is non-interactive,
                // so there is no picker: install MO2's default tick set (the `00`
                // core sub-packages), exactly as the FOMOD branch above installs the
                // default selections. A front end that wants the checkbox list goes
                // through `open_archive` -> `Opened::Bain` -> `install_bain`.
                let (subpackages, _invalid) = layout.bain_subpackages(rules);
                if subpackages.len() >= BAIN_MIN_SUBPACKAGES {
                    let picks = bain_default_selection(&subpackages, &[]);
                    let chosen: Vec<String> = subpackages
                        .iter()
                        .zip(&picks)
                        .filter(|(_, on)| **on)
                        .map(|(s, _)| s.clone())
                        .collect();
                    // Nothing ticked by default means we would be guessing which
                    // sub-packages the mod needs: refuse instead, and let the
                    // interactive path ask.
                    if !chosen.is_empty() {
                        // Resolve first, wipe second (the whole point of the ordering).
                        let sources = resolve_bain_sources(tmp, &chosen)?;
                        if replacing {
                            fs::remove_dir_all(&dest)?;
                        }
                        fs::create_dir_all(&dest)?;
                        place_sources(&sources, &dest, merging)?;
                        write_meta(archive, &dest, game_id, guessed_id)?;
                        return Ok(InstallReport {
                            name: name.clone(),
                            stripped: String::new(),
                            fomod: false,
                            missing: Vec::new(),
                            dest: dest.clone(),
                        });
                    }
                }
                // A Root Builder archive: `Data/` for the mod plus content for the
                // game install root beside it. Unambiguous and non-interactive, so
                // it installs directly rather than falling to the manual picker -
                // which would drop the root half and leave a mod that looks
                // installed and does nothing.
                if let Some(split) = layout.root_builder_split(rules) {
                    // Resolve first, wipe second (the whole point of the ordering).
                    let sources = resolve_root_split(tmp, &split)?;
                    if replacing {
                        fs::remove_dir_all(&dest)?;
                    }
                    fs::create_dir_all(&dest)?;
                    place_root_split(&sources, &dest, merging)?;
                    write_meta(archive, &dest, game_id, guessed_id)?;
                    return Ok(InstallReport {
                        name: name.clone(),
                        stripped: split.data_prefix.unwrap_or_default(),
                        fomod: false,
                        missing: Vec::new(),
                        dest: dest.clone(),
                    });
                }
                return Err(InstallError::NotSimple);
            }
        };
        let src = if base.is_empty() {
            tmp.to_path_buf()
        } else {
            tmp.join(base.trim_end_matches('/'))
        };
        // Extraction + layout resolution succeeded: only now is the old mod wiped.
        if replacing {
            fs::remove_dir_all(&dest)?;
        }
        fs::create_dir_all(&dest)?;
        // A merge installs over existing files; otherwise the dest is empty/wiped,
        // so a fast top-level rename suffices. `overlay_dir`, not `copy_dir_all`:
        // the BAIN and root paths already merge through `place_sources` ->
        // `overlay_dir`, and the plain copy aborted with EISDIR the moment the
        // archive shipped a FILE where the existing mod had a DIRECTORY (or the
        // reverse, via `create_dir_all` onto a file) - half-merging the mod with
        // no cleanup, since a merge target is not `fresh`. Same policy, one
        // semantics: the incoming archive wins the name, both types included.
        if merging {
            overlay_dir(&src, &dest)?;
        } else {
            move_dir_contents(&src, &dest)?;
        }
        write_meta(archive, &dest, game_id, guessed_id)?;
        Ok(InstallReport {
            name: name.clone(),
            stripped: base,
            fomod: false,
            missing: Vec::new(),
            dest: dest.clone(),
        })
    })();

    // A failed FRESH install must not leave half-copied debris that the mod list
    // would show as an installed, enabled mod.
    if result.is_err() && fresh {
        let _ = fs::remove_dir_all(&dest);
    }

    let report = result?;
    // After a Replace, re-apply the preserved user metadata onto the fresh meta.ini.
    if let Some(old) = preserved {
        reapply_user_meta(&old, &report.dest.join("meta.ini"));
    }
    Ok(report)
}

/// Put `sources` (existing directories inside the extraction temp) into `dest`, in
/// order. One source into a destination we own is a top-level rename - instant on
/// the same filesystem, which matters for a multi-GB texture pack. Anything else has
/// to overlay so a later sub-package's files win over an earlier one's.
pub(crate) fn place_sources(sources: &[PathBuf], dest: &Path, merging: bool) -> io::Result<()> {
    if sources.len() == 1 && !merging {
        return move_dir_contents(&sources[0], dest);
    }
    for src in sources {
        overlay_dir(src, dest)?;
    }
    Ok(())
}

/// The shared tail of the non-FOMOD install paths: resolve the destination per
/// `policy`, put the already-resolved `sources` into it, write the `meta.ini`.
///
/// `sources` must be resolved by the caller BEFORE this runs, because the Replace
/// wipe happens in here: like MO2 (and like [`install_extracted`]), the destructive
/// step comes last, so a stale selection can never cost the user their old mod. A
/// FRESH install that fails cleans its own debris, so the mod list never shows a
/// half-copied mod as installed.
pub(crate) fn install_sources(
    sources: &[PathBuf],
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    stripped: String,
) -> Result<InstallReport, InstallError> {
    let mut name = fix_directory_name(name).unwrap_or_else(|| "Mod".to_string());
    let (_, guessed_id) = guess_mod_name_and_id(&archive.to_string_lossy());
    let mut dest = mods_dir.join(&name);
    let mut preserved: Option<ModMeta> = None;
    let mut replacing = false;
    if dest.exists() && is_nonempty_dir(&dest) {
        match &policy {
            OverwritePolicy::Fail => return Err(InstallError::Exists(dest)),
            OverwritePolicy::Merge => {} // install over the existing files
            OverwritePolicy::Replace => {
                preserved = Some(ModMeta::read(&dest.join("meta.ini")));
                replacing = true;
            }
            OverwritePolicy::Rename(new) => {
                name = fix_directory_name(new).unwrap_or_else(|| "Mod".to_string());
                dest = mods_dir.join(&name);
                if dest.exists() && is_nonempty_dir(&dest) {
                    return Err(InstallError::Exists(dest));
                }
            }
        }
    }
    let merging = policy == OverwritePolicy::Merge;
    // Whether `dest` is ours to clean up on failure (a fresh install, not a
    // merge/replace over a pre-existing mod folder).
    let fresh = !dest.exists();

    let result: Result<InstallReport, InstallError> = (|| {
        if replacing {
            fs::remove_dir_all(&dest)?;
        }
        fs::create_dir_all(&dest)?;
        place_sources(sources, &dest, merging)?;
        write_meta(archive, &dest, game_id, guessed_id)?;
        Ok(InstallReport {
            name: name.clone(),
            stripped,
            fomod: false,
            missing: Vec::new(),
            dest: dest.clone(),
        })
    })();

    if result.is_err() && fresh {
        let _ = fs::remove_dir_all(&dest);
    }
    let report = result?;
    if let Some(old) = preserved {
        reapply_user_meta(&old, &report.dest.join("meta.ini"));
    }
    Ok(report)
}

/// The sanitized destination folder name for `raw`, if installing it into
/// `mods_dir` would collide with an existing non-empty mod folder. Lets a front
/// end detect the collision BEFORE consuming a [`FomodSession`] (whose drop
/// removes the extraction temp, losing the user's wizard choices).
pub fn collision_name(mods_dir: &Path, raw: &str) -> Option<String> {
    let name = fix_directory_name(raw).unwrap_or_else(|| "Mod".to_string());
    let dest = mods_dir.join(&name);
    (dest.exists() && is_nonempty_dir(&dest)).then_some(name)
}
