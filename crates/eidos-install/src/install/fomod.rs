//! Driving `eidos-fomod`: find the wizard, build its context, apply the plan.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fs;
use std::path::{Path, PathBuf};

use eidos_instance::ModMeta;

use crate::{
    fix_directory_name, guess_mod_name_and_id,
};


use super::*;

/// Build a FOMOD install [`Context`](eidos_fomod::Context) from the current setup:
/// a plugin (`.esp`/`.esm`/`.esl`) in the game's Data or an enabled mod is marked
/// Active; one present only in a DISABLED mod is marked Inactive; anything absent
/// reads Missing. This lets a scripted installer's `fileDependency` conditions
/// (which distinguish Active / Inactive / Missing) evaluate like MO2 instead of
/// collapsing Inactive into Missing. Eidos doesn't track the game version, so
/// gameDependency stays permissive.
pub fn fomod_context(
    game_data: &Path,
    enabled_mod_roots: &[PathBuf],
    disabled_mod_roots: &[PathBuf],
) -> eidos_fomod::Context {
    let mut file_states = std::collections::HashMap::new();
    let mut scan = |root: &Path, state: &str| {
        if let Ok(rd) = fs::read_dir(root) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_ascii_lowercase();
                if n.ends_with(".esp") || n.ends_with(".esm") || n.ends_with(".esl") {
                    // Active always wins (a plugin shipped by both an enabled and a
                    // disabled mod is Active); so only record Inactive when nothing
                    // already marked it Active.
                    match state {
                        "Active" => {
                            file_states.insert(n, "Active".to_string());
                        }
                        _ => {
                            file_states.entry(n).or_insert_with(|| "Inactive".to_string());
                        }
                    }
                }
            }
        }
    };
    // Disabled first (Inactive), then enabled + game Data (Active) so Active wins.
    for root in disabled_mod_roots {
        scan(root, "Inactive");
    }
    scan(game_data, "Active");
    for root in enabled_mod_roots {
        scan(root, "Active");
    }
    eidos_fomod::Context { file_states, ..Default::default() }
}

/// Find the directory that contains a `fomod/ModuleConfig.xml` (case-insensitive),
/// descending through a wrapper folder or two.
pub(crate) fn find_fomod_root(tmp: &Path) -> Option<PathBuf> {
    fn walk(dir: &Path, depth: u32) -> Option<PathBuf> {
        if depth > 4 {
            return None;
        }
        let entries: Vec<_> = fs::read_dir(dir).ok()?.flatten().collect();
        for e in &entries {
            if e.path().is_dir()
                && e.file_name().to_string_lossy().eq_ignore_ascii_case("fomod")
                && find_ci(&e.path(), "moduleconfig.xml").is_some()
            {
                return Some(dir.to_path_buf());
            }
        }
        for e in &entries {
            let is_fomod = e.file_name().to_string_lossy().eq_ignore_ascii_case("fomod");
            if e.path().is_dir() && !is_fomod {
                if let Some(found) = walk(&e.path(), depth + 1) {
                    return Some(found);
                }
            }
        }
        None
    }
    walk(tmp, 0)
}

/// Parse the `fomod/ModuleConfig.xml` under `root`.
pub(crate) fn parse_fomod_at(root: &Path) -> Result<eidos_fomod::ModuleConfig, InstallError> {
    let fomod_dir =
        find_ci(root, "fomod").ok_or_else(|| InstallError::Fomod("fomod/ not found".to_string()))?;
    let xml_path = find_ci(&fomod_dir, "moduleconfig.xml")
        .ok_or_else(|| InstallError::Fomod("ModuleConfig.xml not found".to_string()))?;
    let bytes = fs::read(&xml_path)?;
    let xml = eidos_fomod::decode_xml(&bytes);
    eidos_fomod::ModuleConfig::parse(&xml).map_err(InstallError::Fomod)
}

/// Copy a computed FOMOD plan from the extracted `root` into `dest`, resolving each
/// source case-insensitively; later (higher-priority) items overwrite earlier ones.
/// Sources the archive did not ship are skipped.
/// Whether a relative install path would escape its base directory: it contains a
/// `..` parent segment, or is absolute / rooted. A benign FOMOD destination is
/// always a plain relative path inside the mod folder, so any of these is a
/// path-traversal attempt (or corruption) that must be refused.
pub(crate) fn escapes_root(rel: &str) -> bool {
    use std::path::Component;
    Path::new(rel)
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
}

pub(crate) fn apply_plan(root: &Path, plan: &[eidos_fomod::FileItem], dest: &Path) -> Result<Vec<String>, InstallError> {
    let mut missing = Vec::new();
    for item in plan {
        let Some(src) = resolve_ci(root, &item.source) else {
            // MO2 logs each plan source the archive did not ship; collect them so the
            // caller can warn (the empty "no folder" source MO2 ignores by default).
            if !item.source.trim().is_empty() {
                missing.push(item.source.clone());
            }
            continue;
        };
        let mut destination = item.destination.replace('\\', "/");
        // MO2 (fomodinstallerdialog.cpp copyLeaf): for a file, an empty destination
        // or one ending in a separator means "into this directory" - append the
        // source's file name. Without this, `dest.join("")` is the mod root dir and
        // the copy fails with EISDIR, aborting the whole install.
        if !item.is_folder && (destination.is_empty() || destination.ends_with('/')) {
            if let Some(name) = item.source.rsplit(['/', '\\']).find(|s| !s.is_empty()) {
                destination.push_str(name);
            }
        }
        // Security: the destination comes from attacker-controlled FOMOD XML. Refuse
        // any path that escapes the mod folder (a `..` segment or an absolute path) -
        // otherwise a crafted `<file destination="../../...">` would write anywhere.
        if escapes_root(&destination) {
            return Err(InstallError::Fomod(format!(
                "refusing install path that escapes the mod folder: '{destination}'"
            )));
        }
        let dst = dest.join(&destination);
        if item.is_folder {
            copy_dir_all(&src, &dst)?;
        } else {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, &dst)?;
        }
    }
    Ok(missing)
}

/// A FOMOD extracted and parsed, awaiting the user's choices (the GUI wizard). The
/// extraction temp is removed when the session is dropped (via [`ExtractedTree`]).
pub struct FomodSession {
    pub config: eidos_fomod::ModuleConfig,
    pub(crate) root: PathBuf,
    /// RAII guard only: `root` points inside this tree, so it must outlive the
    /// session, and its drop removes the extraction temp.
    #[allow(dead_code)]
    pub(crate) tree: ExtractedTree,
    pub(crate) name: String,
    pub(crate) archive: PathBuf,
}

impl FomodSession {
    /// The (unsanitized) mod name this session will install under, for collision
    /// checks before the session is consumed.
    pub fn mod_name(&self) -> &str {
        &self.name
    }

    /// Resolve a FOMOD-relative path (e.g. a plugin or module image) to its
    /// extracted on-disk path, matching each component case-insensitively. Returns
    /// `None` if the archive did not ship it.
    pub fn resolve(&self, rel: &str) -> Option<PathBuf> {
        resolve_ci(&self.root, rel)
    }

    /// If this FOMOD's `<moduleDependencies>` are not met by `ctx`, a human
    /// description of what it requires (so a front end can refuse before showing the
    /// wizard, as MO2 does), else `None`.
    pub fn unmet_dependencies(&self, ctx: &eidos_fomod::Context) -> Option<String> {
        eidos_fomod::unmet_module_dependencies(&self.config, ctx)
    }
}

/// Apply the chosen selection and finish the FOMOD install, resolving a
/// destination collision per `policy` (like [`install_archive_with_policy`]):
/// Fail returns `Exists`, Merge installs over, Replace preserves the user
/// metadata and wipes only after the plan is built, Rename installs under the
/// new name (failing if that also exists).
pub fn finish_fomod(
    session: FomodSession,
    selection: &eidos_fomod::Selection,
    mods_dir: &Path,
    game_id: &str,
    ctx: &eidos_fomod::Context,
    policy: OverwritePolicy,
) -> Result<InstallReport, InstallError> {
    let mut name = fix_directory_name(&session.name).unwrap_or_else(|| "Mod".to_string());
    let (_, guessed_id) = guess_mod_name_and_id(&session.archive.to_string_lossy());
    let mut dest = mods_dir.join(&name);
    let mut preserved: Option<ModMeta> = None;
    let mut replacing = false;
    if dest.exists() && is_nonempty_dir(&dest) {
        match &policy {
            OverwritePolicy::Fail => return Err(InstallError::Exists(dest)),
            OverwritePolicy::Merge => {}
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
    // Belt-and-braces: never install a FOMOD whose module dependencies are unmet,
    // even if a caller skipped the upfront check (MO2 parity).
    if let Some(req) = eidos_fomod::unmet_module_dependencies(&session.config, ctx) {
        return Err(InstallError::UnmetDependency(req));
    }
    // Build the plan (pure) before the destructive step, so a bad selection can
    // never cost the existing mod.
    let plan = eidos_fomod::build_plan(&session.config, selection, ctx);
    if replacing {
        fs::remove_dir_all(&dest)?;
    }
    fs::create_dir_all(&dest)?;
    let missing = apply_plan(&session.root, &plan, &dest)?;
    write_meta(&session.archive, &dest, game_id, guessed_id)?;
    if let Some(old) = preserved {
        reapply_user_meta(&old, &dest.join("meta.ini"));
    }
    Ok(InstallReport { name, stripped: String::new(), fomod: true, missing, dest })
}

// ---- case-collision normalisation -------------------------------------------
//
// Windows (NTFS) is case-insensitive and case-preserving: an archive that holds
// two entries differing only in ASCII case (e.g. `textures/foo.dds` AND
// `Textures/foo.dds`) collapses to ONE file at extraction (last write wins). On
// case-sensitive ext4 a raw `7z x` leaves BOTH as distinct files, and Eidos's
// case-folding VFS would then resolve the virtual path to one nondeterministically
// while the other sits orphaned. `normalize_case_collisions` heals this once, on
// the freshly-extracted tree, before anything else reads it.
//
// It is deliberately NARROW: only genuine case-colliding siblings are touched.
// Non-colliding names keep their original casing - blanket lower-casing would be
// redundant (the VFS already folds case) and would diverge from MO2.
