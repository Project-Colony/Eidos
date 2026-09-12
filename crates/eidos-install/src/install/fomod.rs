//! Driving `eidos-fomod`: find the wizard, build its context, apply the plan.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;

/// Build installer conditions from the same persisted plugin state used at launch.
pub fn fomod_context_for_instance(
    instance: &eidos_instance::Instance,
    game_data: &Path,
    game_id: &str,
    fallback_state: Option<&Path>,
) -> eidos_fomod::Context {
    let mods = instance.modlist();
    let enabled: Vec<_> = mods
        .iter()
        .filter(|m| m.is_active())
        .map(|m| m.path.clone())
        .collect();
    let disabled: Vec<_> = mods
        .iter()
        .filter(|m| !m.is_active() && !m.is_separator())
        .map(|m| m.path.clone())
        .collect();
    let plugins = instance.plugin_list(game_data, game_id, fallback_state);
    fomod_context_with_plugins(
        game_data,
        &enabled,
        &disabled,
        &instance.overwrite_dir(),
        plugins
            .iter()
            .flat_map(|list| list.plugins.iter().map(|p| (p.name.as_str(), p.enabled))),
    )
}

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
                            file_states
                                .entry(n)
                                .or_insert_with(|| "Inactive".to_string());
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
    eidos_fomod::Context {
        file_states,
        ..Default::default()
    }
}

/// Build conditions from discovered plugin activation after applying profile/prefix
/// state. Overwrite contributes installed presence just like the enabled mod roots.
/// Absent plugins remain Missing even if a stale activation entry names them.
pub fn fomod_context_with_plugins<'a>(
    game_data: &Path,
    enabled_mod_roots: &[PathBuf],
    disabled_mod_roots: &[PathBuf],
    overwrite: &Path,
    plugins: impl IntoIterator<Item = (&'a str, bool)>,
) -> eidos_fomod::Context {
    let mut roots = enabled_mod_roots.to_vec();
    roots.push(overwrite.to_path_buf());
    let mut ctx = fomod_context(game_data, &roots, disabled_mod_roots);
    let lower = enabled_mod_roots
        .iter()
        .rev()
        .cloned()
        .chain(std::iter::once(game_data.to_path_buf()))
        .collect();
    let stack = eidos_core::LayerStack::new(lower, overwrite.to_path_buf());
    let disabled: std::collections::HashSet<_> = disabled_mod_roots
        .iter()
        .flat_map(|root| fs::read_dir(root).into_iter().flatten())
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().to_ascii_lowercase())
        .collect();
    ctx.file_states.retain(|name, _| {
        stack.resolve_read(name).is_some_and(|p| p.is_file()) || disabled.contains(name)
    });
    for state in ctx.file_states.values_mut() {
        *state = "Inactive".into();
    }
    for (name, active) in plugins {
        if let Some(state) = ctx.file_states.get_mut(&name.to_ascii_lowercase()) {
            *state = if active { "Active" } else { "Inactive" }.into();
        }
    }
    ctx
}

/// Find the directory that contains a `fomod/ModuleConfig.xml` (case-insensitive),
/// descending through a wrapper folder or two.
pub(crate) fn find_fomod_root(tmp: &Path) -> Option<PathBuf> {
    fn walk(root: &Path, dir: &Path, depth: u32) -> Option<PathBuf> {
        if depth > 4 || !source_within(root, dir) {
            return None;
        }
        let entries: Vec<_> = fs::read_dir(dir).ok()?.flatten().collect();
        for e in &entries {
            if e.path().is_dir()
                && e.file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("fomod")
                && find_ci(&e.path(), "moduleconfig.xml").is_some()
            {
                return Some(dir.to_path_buf());
            }
        }
        for e in &entries {
            let is_fomod = e
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("fomod");
            if e.path().is_dir() && !is_fomod {
                if let Some(found) = walk(root, &e.path(), depth + 1) {
                    return Some(found);
                }
            }
        }
        None
    }
    walk(tmp, tmp, 0)
}

/// Parse the `fomod/ModuleConfig.xml` under `root`.
pub(crate) fn parse_fomod_at(root: &Path) -> Result<eidos_fomod::ModuleConfig, InstallError> {
    let fomod_dir = find_ci(root, "fomod")
        .ok_or_else(|| InstallError::Fomod("fomod/ not found".to_string()))?;
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
    Path::new(rel).components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

#[cfg(test)]
pub(crate) fn apply_plan(
    root: &Path,
    plan: &[eidos_fomod::FileItem],
    dest: &Path,
) -> Result<Vec<String>, InstallError> {
    apply_plan_for_game(root, plan, dest, LayoutRules::default())
}

pub(crate) fn apply_plan_for_game(
    root: &Path,
    plan: &[eidos_fomod::FileItem],
    dest: &Path,
    rules: LayoutRules,
) -> Result<Vec<String>, InstallError> {
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
        // An empty folder destination normally flattens its source. Loader mods
        // need the actual unit name; an explicit nonempty destination still wins.
        if rules.mod_unit == eidos_gamedef::ModUnit::Folder
            && item.is_folder
            && (destination.is_empty() || destination == ".")
            && ArchiveTree::from_dir(&src)?.has_mod_markers(rules)
        {
            destination = src
                .file_name()
                .ok_or_else(folder_layout_error)?
                .to_string_lossy()
                .into_owned();
        }
        let dst = dest.join(&destination);
        copy_plan_source(root, &src, dest, &dst, item.is_folder)?;
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
/// metadata and publishes only after the payload and metadata are ready, Rename installs under the
/// new name (failing if that also exists).
pub fn finish_fomod(
    session: FomodSession,
    selection: &eidos_fomod::Selection,
    mods_dir: &Path,
    game_id: &str,
    ctx: &eidos_fomod::Context,
    policy: OverwritePolicy,
) -> Result<InstallReport, InstallError> {
    finish_fomod_inner(
        session,
        selection,
        mods_dir,
        game_id,
        ctx,
        policy,
        None::<fn(&Path) -> Result<(), InstallError>>,
    )
}

/// Apply the exact FOMOD selection, then finish its payload in private staging
/// before metadata and publication. Merge policies return
/// [`InstallError::BadSelection`] before any installation write or callback;
/// use [`finish_fomod`] for an ordinary live Merge.
pub fn finish_fomod_with_finish(
    session: FomodSession,
    selection: &eidos_fomod::Selection,
    mods_dir: &Path,
    game_id: &str,
    ctx: &eidos_fomod::Context,
    policy: OverwritePolicy,
    finish: impl FnOnce(&Path) -> Result<(), InstallError>,
) -> Result<InstallReport, InstallError> {
    finish_fomod_inner(
        session,
        selection,
        mods_dir,
        game_id,
        ctx,
        policy,
        Some(finish),
    )
}

fn finish_fomod_inner(
    session: FomodSession,
    selection: &eidos_fomod::Selection,
    mods_dir: &Path,
    game_id: &str,
    ctx: &eidos_fomod::Context,
    policy: OverwritePolicy,
    finish: Option<impl FnOnce(&Path) -> Result<(), InstallError>>,
) -> Result<InstallReport, InstallError> {
    if let Some(req) = session.unmet_dependencies(ctx) {
        return Err(InstallError::UnmetDependency(req));
    }
    let plan = eidos_fomod::build_plan(&session.config, selection, ctx);
    install_destination_inner(
        &session.archive,
        mods_dir,
        &session.name,
        game_id,
        policy,
        finish.is_some(),
        |dest, _| {
            let missing =
                apply_plan_for_game(&session.root, &plan, dest, LayoutRules::for_game(game_id))?;
            if let Some(finish) = finish {
                finish(dest)?;
            }
            Ok((String::new(), true, missing))
        },
    )
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
