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
    install_destination(archive, mods_dir, name, game_id, policy, |dest, merging| {
        let tmp = tree.path();
        // Match the interactive classifier: the scripted installer owns selection.
        if let Some(root) = find_fomod_root(tmp) {
            let config = parse_fomod_at(&root)?;
            if let Some(req) = eidos_fomod::unmet_module_dependencies(&config, ctx) {
                return Err(InstallError::UnmetDependency(req));
            }
            let plan = eidos_fomod::build_default_plan(&config, ctx);
            return Ok((String::new(), true, apply_plan(&root, &plan, dest)?));
        }
        let rules = LayoutRules::for_game(game_id);
        let layout = ArchiveTree::from_dir(tmp)?;
        if let Some(base) = layout.simple_archive_base(rules) {
            let src = tmp.join(base.trim_end_matches('/'));
            if !source_within(tmp, &src) {
                return Err(InstallError::BadSelection(
                    "The selected data directory escapes the extracted archive".into(),
                ));
            }
            place_sources(&[src], dest, merging)?;
            return Ok((base, false, Vec::new()));
        }
        let (subpackages, _) = layout.bain_subpackages(rules);
        if subpackages.len() >= BAIN_MIN_SUBPACKAGES {
            let picks = bain_default_selection(&subpackages, &[]);
            let chosen: Vec<_> = subpackages
                .into_iter()
                .zip(picks)
                .filter_map(|(name, on)| on.then_some(name))
                .collect();
            if !chosen.is_empty() {
                place_sources(&resolve_bain_sources(tmp, &chosen)?, dest, merging)?;
                return Ok((String::new(), false, Vec::new()));
            }
        }
        if let Some(split) = layout.root_builder_split(rules) {
            place_root_split(&resolve_root_split(tmp, &split)?, dest, merging)?;
            return Ok((
                format!(
                    "{}{}",
                    split.wrapper_prefix,
                    split.data_prefix.unwrap_or_default()
                ),
                false,
                Vec::new(),
            ));
        }
        Err(InstallError::NotSimple)
    })
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

/// Install already-resolved BAIN/manual sources through the shared publication path.
pub(crate) fn install_sources(
    sources: &[PathBuf],
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    stripped: String,
) -> Result<InstallReport, InstallError> {
    install_destination(archive, mods_dir, name, game_id, policy, |dest, merging| {
        place_sources(sources, dest, merging)?;
        Ok((stripped, false, Vec::new()))
    })
}

/// Stage fresh installs and replacements on the destination filesystem. Merge
/// retains its existing overlay semantics; metadata always starts from the user copy.
pub(crate) fn install_destination(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    place: impl FnOnce(&Path, bool) -> Result<(String, bool, Vec<String>), InstallError>,
) -> Result<InstallReport, InstallError> {
    let mut name = resolved_mod_name(mods_dir, name)?;
    let mut dest = mods_dir.join(&name);
    if is_nonempty_dir(&dest) || fs::symlink_metadata(&dest).is_ok() && !is_real_dir(&dest) {
        match &policy {
            OverwritePolicy::Fail => return Err(InstallError::Exists(dest)),
            OverwritePolicy::Rename(new) => {
                name = resolved_mod_name(mods_dir, new)?;
                dest = mods_dir.join(&name);
                if is_nonempty_dir(&dest) {
                    return Err(InstallError::Exists(dest));
                }
            }
            _ => {}
        }
    }
    // Never treat a symlink or file at the mod name as an owned mod directory.
    if fs::symlink_metadata(&dest).is_ok() && !is_real_dir(&dest) {
        return Err(InstallError::Exists(dest));
    }
    let preserved = dest
        .exists()
        .then(|| ModMeta::read_checked(&dest.join("meta.ini")))
        .transpose()?;
    if let OverwritePolicy::ReplaceOwned(owner) = &policy {
        if owner.is_empty()
            || owner.contains(['\r', '\n'])
            || preserved.as_ref().and_then(ModMeta::collection_owner) != Some(owner)
        {
            return Err(InstallError::BadSelection(
                "The collection reservation changed before publication".into(),
            ));
        }
    }

    let merging = policy == OverwritePolicy::Merge;
    fs::create_dir_all(mods_dir)?;
    let stage = if merging {
        None
    } else {
        let tmp = mods_dir.join(format!(
            ".eidos-install-stage-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&tmp)?;
        Some(ExtractedTree { tmp })
    };
    let target = stage
        .as_ref()
        .map(|s| s.path().join("payload"))
        .unwrap_or_else(|| dest.clone());
    fs::create_dir_all(&target)?;
    // Merge changes the live directory. Persist its uncertain identity before the
    // first payload write, so a failed or interrupted overlay cannot authorize a
    // collection replacement or claim that the old exact source is still intact.
    let merge_checkpoint = if merging {
        let mut meta = preserved.clone().unwrap_or_default();
        meta.set("eidosCollectionOwner", "");
        meta.set_installed_files(&[]);
        meta.set("fileID", "0");
        meta.set_install_warning(
            format!(
                "{} Merge did not complete; installed content is unverified.",
                meta.install_warning().unwrap_or_default()
            )
            .trim(),
        );
        meta.write(&target.join("meta.ini"))?;
        Some(meta)
    } else {
        None
    };
    let installed = (|| -> Result<_, InstallError> {
        let (stripped, fomod, missing) = place(&target, merging)?;
        let (_, guessed_id) = guess_mod_name_and_id(&archive.to_string_lossy());
        write_meta_preserving(archive, &target, game_id, guessed_id, preserved, merging)?;
        let meta_path = target.join("meta.ini");
        let mut meta = ModMeta::read(&meta_path);
        if let OverwritePolicy::ReplaceOwned(owner) = &policy {
            meta.set("eidosCollectionOwner", owner);
        }

        let warning = if missing.is_empty() {
            if merging {
                meta.install_warning().unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            let previous = if merging {
                meta.install_warning().unwrap_or_default()
            } else {
                String::new()
            };
            format!("{previous} Missing archive sources: {}", missing.join(", "))
                .trim()
                .to_string()
        };
        meta.set_install_warning(&warning);
        meta.write(&meta_path)?;
        Ok((stripped, fomod, missing))
    })();
    let (stripped, fomod, missing) = match installed {
        Ok(result) => result,
        Err(error) => {
            // A payload can contain meta.ini too; restore the checkpoint on failure.
            if let Some(meta) = merge_checkpoint {
                if let Err(checkpoint_error) = meta.write(&target.join("meta.ini")) {
                    return Err(io::Error::other(format!(
                        "{error}; cannot persist incomplete Merge warning: {checkpoint_error}"
                    ))
                    .into());
                }
            }
            return Err(error);
        }
    };
    if let Some(stage) = stage {
        publish_install(stage, &target, &dest)?;
    }
    Ok(InstallReport {
        name,
        stripped,
        fomod,
        missing,
        dest,
    })
}

/// Keep the previous mod recoverable until its replacement is published. If a
/// rollback fails too, disarm cleanup and include the recovery path in the error.
pub(crate) fn publish_install(stage: ExtractedTree, payload: &Path, dest: &Path) -> io::Result<()> {
    publish_install_with(stage, payload, dest, |from, to| fs::rename(from, to))
}

fn publish_install_with(
    stage: ExtractedTree,
    payload: &Path,
    dest: &Path,
    mut rename: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let backup = stage.path().join("previous");
    let had_previous = dest.exists();
    if had_previous {
        rename(dest, &backup)?;
    }
    if let Err(error) = rename(payload, dest) {
        if had_previous {
            if let Err(rollback) = rename(&backup, dest) {
                std::mem::forget(stage);
                return Err(io::Error::new(error.kind(), format!(
                    "cannot publish install: {error}; rollback failed: {rollback}; previous mod preserved at {}",
                    backup.display())));
            }
        }
        return Err(error);
    }
    Ok(())
}

/// The sanitized destination folder name for `raw`, if installing it into
/// `mods_dir` would collide with an existing non-empty mod folder. Lets a front
/// end detect the collision BEFORE consuming a [`FomodSession`] (whose drop
/// removes the extraction temp, losing the user's wizard choices).
pub fn collision_name(mods_dir: &Path, raw: &str) -> Option<String> {
    let name = resolved_mod_name(mods_dir, raw)
        .unwrap_or_else(|_| fix_directory_name(raw).unwrap_or_else(|| "Mod".into()));
    let dest = mods_dir.join(&name);
    (is_nonempty_dir(&dest) || fs::symlink_metadata(&dest).is_ok() && !is_real_dir(&dest))
        .then_some(name)
}

/// Preserve the existing directory spelling; the game's namespace is case-insensitive.
fn resolved_mod_name(mods_dir: &Path, raw: &str) -> io::Result<String> {
    let wanted = fix_directory_name(raw).unwrap_or_else(|| "Mod".into());
    let entries = match fs::read_dir(mods_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(wanted),
        Err(error) => return Err(error),
    };
    let mut found = None;
    for entry in entries {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.eq_ignore_ascii_case(&wanted) {
            if found.is_some() {
                return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("Multiple mod folders differ only by case from '{wanted}'; resolve the ambiguity before installing")));
            }
            found = Some(name);
        }
    }
    Ok(found.unwrap_or(wanted))
}

#[cfg(test)]
mod publication_tests {
    use super::*;

    #[test]
    fn installation_stages_and_recovery_trees_are_not_discovered_as_mods() {
        let temp = std::env::temp_dir().join(format!(
            "eidos-stage-discovery-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&temp).unwrap();
        let _cleanup = ExtractedTree { tmp: temp.clone() };
        let instance = eidos_instance::Instance::portable(temp);
        instance.create().unwrap();
        instance.create_empty_mod("Old").unwrap();
        install_destination(
            Path::new("Update.7z"),
            &instance.mods_dir(),
            "Old",
            "skyrimse",
            OverwritePolicy::Replace,
            |target, _| {
                fs::write(target.join("new.esp"), b"replacement")?;
                // A retained rollback tree has the same parent as the staged payload.
                fs::create_dir(target.parent().unwrap().join("previous"))?;
                let names: Vec<_> = instance
                    .modlist()
                    .into_iter()
                    .map(|entry| entry.name)
                    .collect();
                assert_eq!(names, ["Old"], "staging/recovery must remain internal");
                fs::remove_dir(target.parent().unwrap().join("previous"))?;
                Ok((String::new(), false, Vec::new()))
            },
        )
        .unwrap();
    }

    #[test]
    fn failed_rollback_keeps_the_previous_mod_and_reports_its_recovery_path() {
        let temp = std::env::temp_dir().join(format!(
            "eidos-rollback-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&temp).unwrap();
        let _cleanup = ExtractedTree { tmp: temp.clone() };
        let dest = temp.join("Old");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("precious.esp"), b"original").unwrap();
        fs::write(dest.join("meta.ini"), b"original metadata").unwrap();
        let stage = temp.join("stage");
        fs::create_dir(&stage).unwrap();
        let mut calls = 0;
        let error = publish_install_with(
            ExtractedTree { tmp: stage.clone() },
            &stage.join("payload"),
            &dest,
            |from, to| {
                calls += 1;
                if calls == 1 {
                    fs::rename(from, to)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "injected rename failure",
                    ))
                }
            },
        )
        .unwrap_err();
        let backup = stage.join("previous");
        assert!(error.to_string().contains(&backup.display().to_string()));
        assert_eq!(fs::read(backup.join("precious.esp")).unwrap(), b"original");
        assert_eq!(
            fs::read(backup.join("meta.ini")).unwrap(),
            b"original metadata"
        );
    }
}
