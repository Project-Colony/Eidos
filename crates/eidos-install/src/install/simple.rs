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

/// Payloads cannot impersonate host-created replay or recovery state.
pub(crate) fn reject_installer_receipts(root: &Path) -> Result<(), InstallError> {
    for entry in fs::read_dir(root)? {
        let name = entry?.file_name();
        if [
            ".eidos-custom-installer.json",
            ".eidos-omod.mohidden",
            ".eidos-collection-installer.json",
            ".eidos-collection-recipe.json",
        ]
        .iter()
        .any(|reserved| name.to_string_lossy().eq_ignore_ascii_case(reserved))
        {
            return Err(InstallError::BadSelection(
                "Source payload collides with host installer receipt metadata".into(),
            ));
        }
    }
    Ok(())
}

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
    if let Some(session) = try_open_omod(archive, mods_dir, |_| {})? {
        return install_omod(&session, mods_dir, name, game_id, policy);
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
    install_extracted_inner(
        tree,
        archive,
        mods_dir,
        name,
        game_id,
        policy,
        ctx,
        None::<fn(&Path) -> Result<(), InstallError>>,
    )
}

/// Finish the placed payload in private staging before metadata and publication.
/// Merge policies return [`InstallError::BadSelection`] before any installation
/// write or callback; use [`install_extracted`] for an ordinary live Merge.
#[allow(clippy::too_many_arguments)]
pub fn install_extracted_with_finish(
    tree: &ExtractedTree,
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    ctx: &eidos_fomod::Context,
    finish: impl FnOnce(&Path) -> Result<(), InstallError>,
) -> Result<InstallReport, InstallError> {
    install_extracted_inner(
        tree,
        archive,
        mods_dir,
        name,
        game_id,
        policy,
        ctx,
        Some(finish),
    )
}

#[allow(clippy::too_many_arguments)]
fn install_extracted_inner(
    tree: &ExtractedTree,
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    ctx: &eidos_fomod::Context,
    finish: Option<impl FnOnce(&Path) -> Result<(), InstallError>>,
) -> Result<InstallReport, InstallError> {
    install_destination_inner(
        archive,
        mods_dir,
        name,
        game_id,
        policy,
        finish.is_some(),
        |dest, merging| {
            let result = (|| {
                let tmp = tree.path();
                // Match the interactive classifier: the scripted installer owns selection.
                if let Some(root) = find_fomod_root(tmp) {
                    let config = parse_fomod_at(&root)?;
                    if let Some(req) = eidos_fomod::unmet_module_dependencies(&config, ctx) {
                        return Err(InstallError::UnmetDependency(req));
                    }
                    let plan = eidos_fomod::build_default_plan(&config, ctx);
                    return Ok((
                        String::new(),
                        true,
                        apply_plan_for_game(&root, &plan, dest, LayoutRules::for_game(game_id))?,
                    ));
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
                    place_game_sources(&[src], dest, merging, rules)?;
                    return Ok((base, false, Vec::new()));
                }
                if rules.mod_unit == eidos_gamedef::ModUnit::Folder && layout.has_mod_markers(rules)
                {
                    return Err(folder_layout_error());
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
                        place_game_sources(
                            &resolve_bain_sources(tmp, &chosen)?,
                            dest,
                            merging,
                            rules,
                        )?;
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
            })()?;
            reject_installer_receipts(dest)?;
            if let Some(finish) = finish {
                finish(dest)?;
            }
            Ok(result)
        },
    )
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

/// Preserve an explicitly selected loader unit, including its original spelling.
/// Folder payloads are copied into validation staging so a rejected install can retry.
fn place_game_sources(
    sources: &[PathBuf],
    dest: &Path,
    merging: bool,
    rules: LayoutRules,
) -> Result<(), InstallError> {
    if rules.mod_unit != eidos_gamedef::ModUnit::Folder {
        return Ok(place_sources(sources, dest, merging)?);
    }
    for src in sources {
        let target = if ArchiveTree::from_dir(src)?.has_mod_markers(rules) {
            destination_child(dest, src.file_name().ok_or_else(folder_layout_error)?)?
        } else {
            dest.to_path_buf()
        };
        overlay_dir(src, &target)?;
    }
    Ok(())
}

pub(super) fn folder_layout_error() -> InstallError {
    InstallError::BadSelection(
        "This game requires named mod folders containing its marker file; select the parent of the mod folder, or package the bare files inside a named folder".into(),
    )
}

/// Check the owned output before a Merge checkpoint, backup, or live payload write.
fn validate_folder_payload(root: &Path, rules: LayoutRules) -> Result<(), InstallError> {
    fn check_entries(dir: &Path, depth: usize) -> io::Result<()> {
        if depth > crate::MAX_TREE_DEPTH {
            return Err(io::Error::other(
                "Mod folder nesting exceeds the installation limit",
            ));
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() || !(kind.is_dir() || kind.is_file()) {
                return Err(io::Error::other(format!(
                    "Named mod folders require regular files and directories: {}",
                    entry.path().display()
                )));
            }
            if kind.is_dir() {
                check_entries(&entry.path(), depth + 1)?;
            }
        }
        Ok(())
    }
    check_entries(root, 0)?;
    normalize_case_collisions(root)?;
    let tree = ArchiveTree::from_dir(root)?;
    if tree.data_looks_valid(rules) != crate::CheckReturn::Valid {
        return Err(folder_layout_error());
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
    install_destination_inner(
        archive,
        mods_dir,
        name,
        game_id,
        policy,
        false,
        |dest, merging| {
            place_game_sources(sources, dest, merging, LayoutRules::for_game(game_id))?;
            reject_installer_receipts(dest)?;
            Ok((stripped, false, Vec::new()))
        },
    )
}

/// Stage fresh installs and replacements on the destination filesystem.
/// The caller must hold the instance mutation lock. `place` receives a private
/// staging directory and `false` for merging. Merge policies return
/// [`InstallError::BadSelection`] before any installation write or callback.
/// It must finish every payload transformation before returning successfully.
pub fn install_destination(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    place: impl FnOnce(&Path, bool) -> Result<(String, bool, Vec<String>), InstallError>,
) -> Result<InstallReport, InstallError> {
    install_destination_inner(archive, mods_dir, name, game_id, policy, true, place)
}

/// Ordinary installers allow live overlays; exported transform callbacks require staging.
pub(super) fn install_destination_inner(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    staging_only: bool,
    place: impl FnOnce(&Path, bool) -> Result<(String, bool, Vec<String>), InstallError>,
) -> Result<InstallReport, InstallError> {
    if staging_only
        && matches!(
            policy,
            OverwritePolicy::Merge | OverwritePolicy::MergeWithBackup
        )
    {
        return Err(InstallError::BadSelection(
            "Payload transformations require private staging; Merge policies are not supported"
                .into(),
        ));
    }
    let rules = LayoutRules::for_game(game_id);
    // Validate native Merge payloads before touching the live mod or its metadata.
    if rules.mod_unit == eidos_gamedef::ModUnit::Folder
        || matches!(
            policy,
            OverwritePolicy::Merge | OverwritePolicy::MergeWithBackup
        )
    {
        fs::create_dir_all(mods_dir)?;
        let tmp = mods_dir.join(format!(
            ".eidos-install-stage-folder-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&tmp)?;
        let prepared = ExtractedTree::owned(tmp);
        let result = place(prepared.path(), false)?;
        if rules.mod_unit == eidos_gamedef::ModUnit::Folder {
            validate_folder_payload(prepared.path(), rules)?;
        }
        return install_destination_ready(
            archive,
            mods_dir,
            name,
            game_id,
            policy,
            &[],
            |dest, merging| {
                place_sources(&[prepared.path().to_path_buf()], dest, merging)?;
                Ok(result)
            },
        );
    }
    install_destination_ready(archive, mods_dir, name, game_id, policy, &[], place)
}

pub(super) fn install_destination_ready(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    archive_facts: &[(String, String)],
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
    if let OverwritePolicy::ReplaceOwned(owner) | OverwritePolicy::ReplaceOwnedWithBackup(owner) =
        &policy
    {
        if owner.is_empty()
            || owner.contains(['\r', '\n'])
            || preserved.as_ref().and_then(ModMeta::collection_owner) != Some(owner)
        {
            return Err(InstallError::BadSelection(
                "The collection reservation changed before publication".into(),
            ));
        }
    }

    let merging = matches!(
        policy,
        OverwritePolicy::Merge | OverwritePolicy::MergeWithBackup
    );
    let retain = matches!(
        policy,
        OverwritePolicy::ReplaceWithBackup
            | OverwritePolicy::MergeWithBackup
            | OverwritePolicy::ReplaceOwnedWithBackup(_)
    );
    // The checkpoint is a live write too, so the snapshot must precede it.
    let mut backup = if merging && retain && dest.exists() {
        Some(eidos_instance::backup_mod(&dest)?)
    } else {
        None
    };
    let result = (|| {
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
            Some(ExtractedTree::owned(tmp))
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
            for (key, value) in archive_facts {
                meta.set(key, value);
            }
            if let OverwritePolicy::ReplaceOwned(owner)
            | OverwritePolicy::ReplaceOwnedWithBackup(owner) = &policy
            {
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
            backup = publish_install_with(stage, &target, &dest, retain, |from, to| {
                fs::rename(from, to)
            })?;
        }
        Ok(InstallReport {
            name,
            stripped,
            fomod,
            missing,
            dest,
            backup: backup.clone(),
        })
    })();
    result.map_err(|error: InstallError| match backup {
        Some(path) => io::Error::other(format!(
            "{error}; previous mod preserved at {}",
            path.display()
        ))
        .into(),
        None => error,
    })
}

/// Keep the previous mod recoverable until its replacement is published. If a
/// rollback fails too, disarm cleanup and include the recovery path in the error.
#[cfg(test)]
pub(crate) fn publish_install(stage: ExtractedTree, payload: &Path, dest: &Path) -> io::Result<()> {
    publish_install_with(stage, payload, dest, false, |from, to| fs::rename(from, to)).map(|_| ())
}

fn publish_install_with(
    stage: ExtractedTree,
    payload: &Path,
    dest: &Path,
    retain: bool,
    mut rename: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> io::Result<Option<PathBuf>> {
    let had_previous = dest.exists();
    let retained = if had_previous && retain {
        Some(eidos_instance::reserve_mod_backup(dest)?)
    } else {
        None
    };
    let backup = retained
        .clone()
        .unwrap_or_else(|| stage.path().join("previous"));
    if had_previous {
        if let Err(error) = rename(dest, &backup) {
            // Only an empty reservation is ours to remove on this failure.
            if retained.is_some() {
                let _ = fs::remove_dir(&backup);
            }
            return Err(error);
        }
    }
    if let Err(error) = rename(payload, dest) {
        if had_previous {
            if let Err(rollback) = rename(&backup, dest) {
                if retained.is_none() {
                    std::mem::forget(stage);
                }
                return Err(io::Error::new(error.kind(), format!(
                    "cannot publish install: {error}; rollback failed: {rollback}; previous mod preserved at {}",
                    backup.display())));
            }
        }
        return Err(error);
    }
    Ok(retained)
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
    fn native_archive_cannot_publish_or_merge_host_receipts() {
        let tmp = std::env::temp_dir().join(format!(
            "eidos-native-receipt-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&tmp).unwrap();
        let root = ExtractedTree::owned(tmp);
        let source = root.path().join("source");
        fs::create_dir_all(source.join("Data/textures")).unwrap();
        fs::create_dir(source.join("Data/.EIDOS-OMOD.MOHIDDEN")).unwrap();
        fs::write(source.join("Data/textures/new.dds"), b"new").unwrap();
        fs::write(
            source.join("Data/.EIDOS-OMOD.MOHIDDEN/install.json"),
            b"forged receipt",
        )
        .unwrap();
        let archive = root.path().join("ordinary.zip");
        assert!(
            std::process::Command::new(eidos_sevenzip::find_7z().unwrap())
                .current_dir(&source)
                .args(["a", "-tzip"])
                .arg(&archive)
                .arg(".")
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap()
                .success()
        );
        let mods = root.path().join("mods");
        fs::create_dir(&mods).unwrap();
        for (name, policy) in [
            ("Fresh", OverwritePolicy::Fail),
            ("Replace", OverwritePolicy::Replace),
            ("Merge", OverwritePolicy::Merge),
        ] {
            let dest = mods.join(name);
            if policy != OverwritePolicy::Fail {
                fs::create_dir(&dest).unwrap();
                fs::write(dest.join("old.txt"), b"previous").unwrap();
                fs::write(dest.join("meta.ini"), b"[General]\nnotes=previous\n").unwrap();
            }
            let result = install_archive_with_policy(
                &archive,
                &mods,
                name,
                "skyrimse",
                policy,
                &eidos_fomod::Context::default(),
            );
            assert!(result.is_err(), "{result:?}");
            assert!(!dest.join("textures/new.dds").exists());
            assert!(!dest.join(".EIDOS-OMOD.MOHIDDEN").exists());
            if dest.exists() {
                assert_eq!(fs::read(dest.join("old.txt")).unwrap(), b"previous");
                assert_eq!(
                    fs::read(dest.join("meta.ini")).unwrap(),
                    b"[General]\nnotes=previous\n"
                );
            }
        }
    }

    #[test]
    fn installation_stages_and_recovery_trees_are_not_discovered_as_mods() {
        let temp = std::env::temp_dir().join(format!(
            "eidos-stage-discovery-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&temp).unwrap();
        let _cleanup = ExtractedTree::owned(temp.clone());
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
        let _cleanup = ExtractedTree::owned(temp.clone());
        let dest = temp.join("Old");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("precious.esp"), b"original").unwrap();
        fs::write(dest.join("meta.ini"), b"original metadata").unwrap();
        let stage = temp.join("stage");
        fs::create_dir(&stage).unwrap();
        let mut calls = 0;
        let error = publish_install_with(
            ExtractedTree::owned(stage.clone()),
            &stage.join("payload"),
            &dest,
            false,
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

    #[test]
    fn retained_publication_failures_preserve_original_or_explicit_recovery() {
        for failure in [1, 2, 3] {
            let temp = std::env::temp_dir().join(format!(
                "eidos-retained-rollback-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&temp).unwrap();
            let _cleanup = ExtractedTree::owned(temp.clone());
            let dest = temp.join("Old");
            fs::create_dir(&dest).unwrap();
            fs::write(dest.join("precious.esp"), b"original").unwrap();
            let stage = temp.join("stage");
            fs::create_dir_all(stage.join("payload")).unwrap();
            fs::write(stage.join("payload/new.esp"), b"replacement").unwrap();
            let mut calls = 0;
            let error = publish_install_with(
                ExtractedTree::owned(stage.clone()),
                &stage.join("payload"),
                &dest,
                true,
                |from, to| {
                    calls += 1;
                    if calls == failure || failure == 3 && calls == 2 {
                        Err(io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            "injected rename failure",
                        ))
                    } else {
                        fs::rename(from, to)
                    }
                },
            )
            .unwrap_err();
            let backup = temp.join("Old_backup");
            if failure == 3 {
                assert!(!dest.exists());
                assert_eq!(fs::read(backup.join("precious.esp")).unwrap(), b"original");
                assert!(error.to_string().contains(&backup.display().to_string()));
            } else {
                assert_eq!(fs::read(dest.join("precious.esp")).unwrap(), b"original");
                assert!(!backup.exists());
            }
            assert!(
                !stage.exists(),
                "a retained recovery tree is outside staging"
            );
        }
    }
}
