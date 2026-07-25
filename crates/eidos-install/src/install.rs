//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

use eidos_instance::ModMeta;

use crate::{
    bain_default_selection, fix_directory_name, guess_mod_name, guess_mod_name_and_id, ArchiveEntry,
    ArchiveTree, BAIN_MIN_SUBPACKAGES,
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

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
                write!(f, "not a simple archive (no recognised Data layout); manual install needed")
            }
            InstallError::NeedsFomod => {
                write!(f, "this is a FOMOD scripted installer - not yet supported (Tier 2)")
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

/// The first usable 7-Zip binary on `PATH`.
fn find_7z() -> Option<&'static str> {
    ["7z", "7zz", "7za"].into_iter().find(|b| Command::new(b).output().is_ok())
}

fn extract_all(bin: &str, archive: &Path, dest: &Path) -> Result<(), InstallError> {
    let out = Command::new(bin)
        .arg("x")
        .arg("-y")
        .arg(format!("-o{}", dest.display()))
        .arg(archive)
        .output()
        .map_err(|e| InstallError::Extract(e.to_string()))?;
    if !out.status.success() {
        return Err(InstallError::Extract(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(())
}

impl ArchiveTree {
    /// Build the tree by walking an extracted directory.
    pub fn from_dir(root: &Path) -> io::Result<ArchiveTree> {
        fn walk(base: &Path, dir: &Path, out: &mut Vec<ArchiveEntry>) -> io::Result<()> {
            for e in fs::read_dir(dir)?.flatten() {
                let p = e.path();
                let is_dir = p.is_dir();
                if let Ok(rel) = p.strip_prefix(base) {
                    out.push(ArchiveEntry {
                        path: rel.to_string_lossy().replace('\\', "/"),
                        is_dir,
                    });
                }
                if is_dir {
                    walk(base, &p, out)?;
                }
            }
            Ok(())
        }
        let mut entries = Vec::new();
        walk(root, root, &mut entries)?;
        Ok(ArchiveTree::from_entries(&entries))
    }
}

/// Move every top-level entry of `src` into `dest` (rename, so same-filesystem
/// installs are instant).
fn move_dir_contents(src: &Path, dest: &Path) -> io::Result<()> {
    for e in fs::read_dir(src)?.flatten() {
        fs::rename(e.path(), dest.join(e.file_name()))?;
    }
    Ok(())
}

fn is_nonempty_dir(p: &Path) -> bool {
    fs::read_dir(p).map(|mut rd| rd.next().is_some()).unwrap_or(false)
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

/// Install `archive` into `mods_dir/name`, MO2 Simple-installer style: extract,
/// strip the wrapper folder to the Data-relative root, move it in, and write a
/// MO2-compatible `meta.ini`. Fails if the destination already exists; use
/// [`install_archive_with_policy`] to replace/merge/rename instead.
pub fn install_archive(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_name: &str,
) -> Result<InstallReport, InstallError> {
    install_archive_with_policy(
        archive,
        mods_dir,
        name,
        game_name,
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
    game_name: &str,
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
    install_extracted(&tree, archive, mods_dir, name, game_name, policy, ctx)
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
    game_name: &str,
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

    let result = (|| {
        let layout = ArchiveTree::from_dir(tmp)?;
        let base = match layout.simple_archive_base() {
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
                    write_meta(archive, &dest, game_name, guessed_id)?;
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
                let (subpackages, _invalid) = layout.bain_subpackages();
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
                        write_meta(archive, &dest, game_name, guessed_id)?;
                        return Ok(InstallReport {
                            name: name.clone(),
                            stripped: String::new(),
                            fomod: false,
                            missing: Vec::new(),
                            dest: dest.clone(),
                        });
                    }
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
        // A merge installs over existing files (recursive copy); otherwise the dest
        // is empty/wiped, so a fast top-level rename suffices.
        if merging {
            copy_dir_all(&src, &dest)?;
        } else {
            move_dir_contents(&src, &dest)?;
        }
        write_meta(archive, &dest, game_name, guessed_id)?;
        Ok(InstallReport { name: name.clone(), stripped: base, fomod: false, missing: Vec::new(), dest: dest.clone() })
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

/// Re-apply the user-set fields (endorsement, tracked, category) from a previous
/// install's meta.ini onto a freshly written one, so a Replace doesn't lose them.
fn reapply_user_meta(old: &ModMeta, meta_path: &Path) {
    let mut m = ModMeta::read(meta_path);
    if old.endorsed() {
        m.set("endorsed", "1");
    }
    if old.tracked() {
        m.set("tracked", "1");
    }
    if let Some(c) = old.category() {
        m.set("category", &format!("\"{c}\""));
    }
    let _ = m.write(meta_path);
}

/// Write a MO2-compatible `meta.ini`, seeded from the download's `<archive>.meta`
/// sidecar if MO2/Nexus left one next to the file. `guessed_id` is the mod id
/// recovered from the filename, used when the sidecar carries none.
fn write_meta(archive: &Path, dest: &Path, game_name: &str, guessed_id: Option<u64>) -> io::Result<()> {
    // The sidecar is the full archive name + ".meta" (e.g. Mod-1234.7z.meta).
    let sidecar = PathBuf::from(format!("{}.meta", archive.to_string_lossy()));
    let from = ModMeta::read(&sidecar);

    let mut meta = ModMeta::default();
    meta.set("gameName", &from.game_name().unwrap_or_else(|| game_name.to_string()));
    // Mod id: the sidecar's, else the one guessed from the Nexus filename, so a
    // manually-downloaded archive with no sidecar can still be update-checked.
    if let Some(id) = from.mod_id().or(guessed_id) {
        meta.set("modid", &id.to_string());
    }
    // Version: the sidecar's, else a date stamp from the archive mtime (MO2's
    // dYYYY.M.D fallback) so update_available has a baseline to compare against.
    if let Some(v) = from.version().or_else(|| archive_date_version(archive)) {
        meta.set("version", &v);
    }
    if let Some(nv) = from.newest_version() {
        meta.set("newestVersion", &nv);
    }
    // The sidecar's category is a raw Nexus id we don't map yet; leave uncategorised.
    meta.set("category", "\"-1,\"");
    // nexusFileStatus mirrors the sidecar's fileCategory (1 = main file by default).
    meta.set("nexusFileStatus", &from.file_category().unwrap_or_else(|| "1".to_string()));
    // Record where the archive came from, absolute (MO2 stores the full path for a
    // file outside the downloads folder).
    let install_file = fs::canonicalize(archive)
        .unwrap_or_else(|_| archive.to_path_buf())
        .to_string_lossy()
        .into_owned();
    meta.set("installationFile", &install_file);
    meta.set("repository", &from.repository().unwrap_or_else(|| "Nexus".to_string()));
    meta.set("endorsed", "0");
    meta.set("tracked", "0");
    meta.write(&dest.join("meta.ini"))
}

/// MO2's date-stamp version fallback (`dYYYY.M.D`, no zero-padding) from the
/// archive's modification time, used when the download has no real version.
fn archive_date_version(archive: &Path) -> Option<String> {
    let secs = fs::metadata(archive)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let (y, m, d) = civil_from_unix(secs);
    Some(format!("d{y}.{m}.{d}"))
}

/// Year/month/day (UTC) from a Unix timestamp - Hinnant's civil-from-days, so no
/// calendar crate is needed.
fn civil_from_unix(secs: u64) -> (i64, u32, u32) {
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// The mod folder name for `archive`, with MO2's precedence: the download sidecar's
/// `modName`, else its `name`, else the filename guess - then sanitized with
/// [`fix_directory_name`] (real Nexus names contain `:`).
pub fn mod_name_for(archive: &Path) -> String {
    let sidecar = PathBuf::from(format!("{}.meta", archive.to_string_lossy()));
    let meta = ModMeta::read(&sidecar);
    let picked = meta
        .mod_name()
        .or_else(|| meta.name())
        .unwrap_or_else(|| guess_mod_name(&archive.to_string_lossy()));
    fix_directory_name(&picked)
        .or_else(|| fix_directory_name(&guess_mod_name(&archive.to_string_lossy())))
        .unwrap_or_else(|| "Mod".to_string())
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
fn find_fomod_root(tmp: &Path) -> Option<PathBuf> {
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

/// Find a directory entry by case-insensitive name.
fn find_ci(dir: &Path, name_lower: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name_lower))
        .map(|e| e.path())
}

/// Resolve a `/`-joined relative path under `root`, matching each component
/// case-insensitively (FOMOD sources are Windows-cased and may not match on disk).
fn resolve_ci(root: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = root.to_path_buf();
    for part in rel.split(['/', '\\']).filter(|s| !s.is_empty()) {
        // Security: refuse a `..` segment so an attacker-controlled FOMOD source
        // path can't read files outside the extracted archive root.
        if part == ".." {
            return None;
        }
        let exact = cur.join(part);
        if exact.exists() {
            cur = exact;
            continue;
        }
        cur = find_ci(&cur, &part.to_ascii_lowercase())?;
    }
    Some(cur)
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)?.flatten() {
        let from = e.path();
        let to = dst.join(e.file_name());
        if from.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Copy every entry of `src` over `dst`, later-wins. Unlike [`copy_dir_all`] this
/// tolerates a TYPE conflict between two overlays - a file landing where an earlier
/// sub-package left a directory, or the reverse - by dropping the loser, because
/// BAIN's contract is that the later sub-package wins outright. Without it,
/// `fs::copy` onto a directory fails EISDIR and aborts an otherwise fine install.
///
/// Symlinks are treated as opaque entries (never recursed into), so a symlink loop
/// inside a crafted archive cannot make this recurse forever.
fn overlay_dir(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)?.flatten() {
        let from = e.path();
        let to = dst.join(e.file_name());
        // Whatever occupies the name loses, unless both sides are directories (which
        // merge). `symlink_metadata` deliberately does not follow: a DANGLING symlink
        // still occupies the name, and - the reason this matters - removing the link
        // before writing is what stops the copy below from being redirected THROUGH a
        // symlink an earlier sub-package planted, which would write outside the mod.
        let occupant = fs::symlink_metadata(&to).map(|m| m.file_type()).ok();
        let both_dirs = is_real_dir(&from) && occupant.is_some_and(|t| t.is_dir());
        if !both_dirs {
            match occupant {
                Some(t) if t.is_dir() => fs::remove_dir_all(&to)?,
                Some(_) => fs::remove_file(&to)?,
                None => {}
            }
        }
        if is_real_dir(&from) {
            overlay_dir(&from, &to)?;
            continue;
        }
        // `read_link` succeeds only for a symlink (EINVAL otherwise), so this is the
        // discriminator. Recreate the link instead of copying through it: the link is
        // the content, and copying a DANGLING one would fail the whole install over an
        // entry the simple path (a plain rename) would have installed fine.
        match fs::read_link(&from) {
            Ok(target) => std::os::unix::fs::symlink(target, &to)?,
            Err(_) => {
                fs::copy(&from, &to)?;
            }
        }
    }
    Ok(())
}

/// Put `sources` (existing directories inside the extraction temp) into `dest`, in
/// order. One source into a destination we own is a top-level rename - instant on
/// the same filesystem, which matters for a multi-GB texture pack. Anything else has
/// to overlay so a later sub-package's files win over an earlier one's.
fn place_sources(sources: &[PathBuf], dest: &Path, merging: bool) -> io::Result<()> {
    if sources.len() == 1 && !merging {
        return move_dir_contents(&sources[0], dest);
    }
    for src in sources {
        overlay_dir(src, dest)?;
    }
    Ok(())
}

/// Resolve chosen BAIN sub-package names to directories inside the extraction temp,
/// in the order given. Every fallible check lives here so a caller can run it BEFORE
/// the destructive step and never wipe a mod over a stale pick.
fn resolve_bain_sources(tmp: &Path, chosen: &[String]) -> Result<Vec<PathBuf>, InstallError> {
    if chosen.is_empty() {
        return Err(InstallError::BadSelection("no sub-package selected".to_string()));
    }
    let mut out = Vec::with_capacity(chosen.len());
    for name in chosen {
        // A sub-package is one top-level folder name. Anything with a separator or a
        // relative segment is not one and must never be joined blindly: the list
        // comes from a front end, not necessarily from `bain_subpackages`.
        if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
            return Err(InstallError::BadSelection(format!("not a sub-package name: '{name}'")));
        }
        let exact = tmp.join(name);
        let dir = if is_real_dir(&exact) {
            exact
        } else {
            // The extraction may have case-folded a colliding name (see
            // `normalize_case_collisions`), so fall back to a case-insensitive match.
            find_ci(tmp, &name.to_ascii_lowercase())
                .filter(|p| is_real_dir(p))
                .ok_or_else(|| InstallError::BadSelection(format!("no such sub-package: '{name}'")))?
        };
        out.push(dir);
    }
    Ok(out)
}

/// Resolve a user-chosen data root (a `/`-joined prefix inside the archive, `""` for
/// the archive root) to a directory inside the extraction temp. `resolve_ci` already
/// refuses a `..` segment, so a hand-typed root cannot escape the temp.
fn resolve_manual_root(tmp: &Path, root: &str) -> Result<PathBuf, InstallError> {
    let trimmed = root.trim_matches(['/', '\\'].as_slice());
    if trimmed.is_empty() {
        return Ok(tmp.to_path_buf());
    }
    resolve_ci(tmp, trimmed)
        .filter(|p| is_real_dir(p))
        .ok_or_else(|| InstallError::BadSelection(format!("no such directory in the archive: '{root}'")))
}

/// The shared tail of the non-FOMOD install paths: resolve the destination per
/// `policy`, put the already-resolved `sources` into it, write the `meta.ini`.
///
/// `sources` must be resolved by the caller BEFORE this runs, because the Replace
/// wipe happens in here: like MO2 (and like [`install_extracted`]), the destructive
/// step comes last, so a stale selection can never cost the user their old mod. A
/// FRESH install that fails cleans its own debris, so the mod list never shows a
/// half-copied mod as installed.
fn install_sources(
    sources: &[PathBuf],
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_name: &str,
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
        write_meta(archive, &dest, game_name, guessed_id)?;
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
    game_name: &str,
    policy: OverwritePolicy,
) -> Result<InstallReport, InstallError> {
    let sources = resolve_bain_sources(tree.path(), subpackages)?;
    install_sources(&sources, archive, mods_dir, name, game_name, policy, String::new())
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
    game_name: &str,
    policy: OverwritePolicy,
) -> Result<InstallReport, InstallError> {
    let src = resolve_manual_root(tree.path(), data_root)?;
    let stripped = if data_root.is_empty() {
        String::new()
    } else {
        format!("{}/", data_root.trim_matches(['/', '\\'].as_slice()))
    };
    install_sources(&[src], archive, mods_dir, name, game_name, policy, stripped)
}

/// Parse the `fomod/ModuleConfig.xml` under `root`.
fn parse_fomod_at(root: &Path) -> Result<eidos_fomod::ModuleConfig, InstallError> {
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
fn escapes_root(rel: &str) -> bool {
    use std::path::Component;
    Path::new(rel)
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
}

fn apply_plan(root: &Path, plan: &[eidos_fomod::FileItem], dest: &Path) -> Result<Vec<String>, InstallError> {
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

/// An archive already extracted into a temp directory beside `mods/`, with NTFS
/// case collisions healed. Holding one means the expensive 7-Zip pass is already
/// paid for: [`install_extracted`] installs straight from it, so a simple archive
/// is never extracted twice. The temp is removed when this is dropped.
pub struct ExtractedTree {
    tmp: PathBuf,
}

impl ExtractedTree {
    /// The extracted tree's root on disk.
    pub fn path(&self) -> &Path {
        &self.tmp
    }
}

impl Drop for ExtractedTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.tmp);
    }
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

/// A FOMOD extracted and parsed, awaiting the user's choices (the GUI wizard). The
/// extraction temp is removed when the session is dropped (via [`ExtractedTree`]).
pub struct FomodSession {
    pub config: eidos_fomod::ModuleConfig,
    root: PathBuf,
    /// RAII guard only: `root` points inside this tree, so it must outlive the
    /// session, and its drop removes the extraction temp.
    #[allow(dead_code)]
    tree: ExtractedTree,
    name: String,
    archive: PathBuf,
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

/// Extract `archive` into a fresh temp directory beside `mods_dir` and heal
/// NTFS-style case collisions, so everything downstream (wrapper detection,
/// FOMOD lookup, the move) reads a consistent tree. The returned handle owns the
/// temp and removes it on drop, including on the `?` paths here.
pub fn extract_to_temp(archive: &Path, mods_dir: &Path) -> Result<ExtractedTree, InstallError> {
    let bin = find_7z().ok_or(InstallError::No7z)?;
    let tmp = mods_dir.join(format!(
        ".eidos-install-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&tmp)?;
    let tree = ExtractedTree { tmp };
    extract_all(bin, archive, &tree.tmp)?;
    normalize_case_collisions(&tree.tmp)?;
    Ok(tree)
}

/// Extract `archive` once and classify it: a FOMOD (whose `config` drives the
/// wizard), a simple archive, a BAIN package (whose sub-packages the user ticks) or
/// an unrecognised layout the user must resolve by hand. The extracted tree rides
/// along in every case, so installing it costs no second extraction.
pub fn open_archive(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
) -> Result<Opened, InstallError> {
    let tree = extract_to_temp(archive, mods_dir)?;
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
    let layout = ArchiveTree::from_dir(&tree.tmp)?;
    if layout.simple_archive_base().is_some() {
        return Ok(Opened::Simple(tree));
    }
    let (subpackages, invalid) = layout.bain_subpackages();
    if subpackages.len() >= BAIN_MIN_SUBPACKAGES {
        return Ok(Opened::Bain { tree, subpackages, invalid });
    }
    Ok(Opened::Manual(tree))
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

/// Apply the chosen selection and finish the FOMOD install, resolving a
/// destination collision per `policy` (like [`install_archive_with_policy`]):
/// Fail returns `Exists`, Merge installs over, Replace preserves the user
/// metadata and wipes only after the plan is built, Rename installs under the
/// new name (failing if that also exists).
pub fn finish_fomod(
    session: FomodSession,
    selection: &eidos_fomod::Selection,
    mods_dir: &Path,
    game_name: &str,
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
    write_meta(&session.archive, &dest, game_name, guessed_id)?;
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

/// Collapse every directory's case-colliding children into a single canonical,
/// lower-cased entry, recursively and in place. Best-effort and idempotent.
///
/// Resolution rules (NTFS-equivalent, deterministic):
/// - dir + dir: merge children into one lower-cased dir, recurse.
/// - file + file: keep the oldest `mtime` (the author's original; later same-name
///   entries are usually repack dupes), breaking ties by lexicographic name.
/// - file + dir: the file wins the canonical name; the dir is moved aside to
///   `<name>_dir` so its contents are never dropped.
/// - symlinks: treated as opaque, name-only entries; never followed (no loops).
/// - non-UTF8 names: logged and skipped; siblings are still normalised.
fn normalize_case_collisions(dir: &Path) -> io::Result<()> {
    resolve_dir_collisions(dir)?;
    // Recurse into the now-collision-free real subdirectories so nested collisions
    // (and collisions inside dirs merged above) settle too.
    for e in fs::read_dir(dir)?.flatten() {
        let p = e.path();
        if is_real_dir(&p) {
            normalize_case_collisions(&p)?;
        }
    }
    Ok(())
}

/// Resolve case-collisions among the immediate children of `dir` (one level).
fn resolve_dir_collisions(dir: &Path) -> io::Result<()> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for e in fs::read_dir(dir)?.flatten() {
        match e.file_name().to_str() {
            Some(s) => groups.entry(s.to_ascii_lowercase()).or_default().push(e.path()),
            None => eprintln!(
                "eidos install: skipping case-normalisation of non-UTF8 name {:?}",
                e.file_name()
            ),
        }
    }
    for (key, members) in groups {
        if members.len() > 1 {
            resolve_group(dir, &key, members)?;
        }
    }
    Ok(())
}

/// Collapse one collision group (>= 2 entries whose names lower-case to `key`,
/// `existing` ones living in `parent`) into a single canonical entry in `parent`.
fn resolve_group(parent: &Path, key: &str, members: Vec<PathBuf>) -> io::Result<()> {
    let (dirs, opaques): (Vec<PathBuf>, Vec<PathBuf>) =
        members.into_iter().partition(|p| is_real_dir(p));

    if opaques.is_empty() {
        // All directories: merge into one lower-cased canonical dir.
        merge_dirs_into(parent, key, &dirs)?;
        return Ok(());
    }

    // A file/symlink wins the canonical name. Any colliding directories move aside
    // to `<key>_dir` so their contents survive.
    if !dirs.is_empty() {
        merge_dirs_into(parent, &format!("{key}_dir"), &dirs)?;
    }
    let survivor = pick_oldest(&opaques);
    for o in &opaques {
        if *o != survivor {
            fs::remove_file(o)?; // removes the symlink/regular file, never a target
        }
    }
    rename_if_needed(&survivor, &parent.join(key))
}

/// Merge `dirs` (case-variants of one name in `parent`) into a single directory
/// named `target_name`. Staged under a fresh temp name first so an in-place rename
/// can never clobber a doomed sibling on case-sensitive ext4, then published.
fn merge_dirs_into(parent: &Path, target_name: &str, dirs: &[PathBuf]) -> io::Result<()> {
    let staging = parent.join(format!(".eidos-case-{}", COUNTER.fetch_add(1, Ordering::Relaxed)));
    fs::create_dir(&staging)?;
    for d in dirs {
        merge_into(d, &staging)?;
        // `d` should be empty now; remove it. If a skipped non-UTF8 collision left a
        // child behind, leave the residual dir (no data loss) and note it.
        if fs::remove_dir(d).is_err() {
            eprintln!("eidos install: left residual dir after case-merge: {}", d.display());
        }
    }
    // Belt-and-braces: settle any collision the staged union still holds.
    resolve_dir_collisions(&staging)?;
    publish_staging(parent, target_name, &staging)
}

/// Publish a finished `staging` directory under `parent/target_name`. If an entry
/// already holds that name: a directory is merged into (case folds together); a
/// FILE or symlink is left intact and the staged union lands at the first free
/// `target_name_<n>` instead - so neither the pre-existing entry nor the merged
/// contents are ever lost (the alternative `merge_into` on a file is ENOTDIR, which
/// would abort the whole install).
fn publish_staging(parent: &Path, target_name: &str, staging: &Path) -> io::Result<()> {
    let target = parent.join(target_name);
    if target.exists() {
        if is_real_dir(&target) {
            merge_into(staging, &target)?;
            let _ = fs::remove_dir_all(staging);
            return Ok(());
        }
        // Occupied by a file/symlink: find the first free suffixed name.
        let mut n = 1;
        let free = loop {
            let cand = parent.join(format!("{target_name}_{n}"));
            if !cand.exists() {
                break cand;
            }
            n += 1;
        };
        return fs::rename(staging, free);
    }
    fs::rename(staging, &target)
}

/// Move every child of `src` into `dst` (both existing dirs), resolving a
/// case-insensitive collision with an existing `dst` child by the same rules.
fn merge_into(src: &Path, dst: &Path) -> io::Result<()> {
    for e in fs::read_dir(src)?.flatten() {
        let name = e.file_name();
        let child = e.path();
        let key = match name.to_str() {
            Some(s) => s.to_ascii_lowercase(),
            None => {
                // Non-UTF8: best-effort move as-is; skip (don't clobber) if taken.
                let target = dst.join(&name);
                if !target.exists() {
                    fs::rename(&child, &target)?;
                } else {
                    eprintln!("eidos install: skipping non-UTF8 case-merge of {child:?}");
                }
                continue;
            }
        };
        match ci_find(dst, &key)? {
            None => fs::rename(&child, dst.join(&name))?, // no collision: preserve casing
            Some(existing) => resolve_group(dst, &key, vec![existing, child])?,
        }
    }
    Ok(())
}

/// The oldest-`mtime` member, breaking ties by the lexicographically-smallest
/// file name so the choice is deterministic across runs.
fn pick_oldest(paths: &[PathBuf]) -> PathBuf {
    paths
        .iter()
        .min_by(|a, b| mtime(a).cmp(&mtime(b)).then_with(|| a.file_name().cmp(&b.file_name())))
        .cloned()
        .expect("a collision group is never empty")
}

fn mtime(p: &Path) -> SystemTime {
    fs::symlink_metadata(p).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH)
}

/// A real directory, NOT a symlink to one (symlinks are treated as opaque).
fn is_real_dir(p: &Path) -> bool {
    fs::symlink_metadata(p).map(|m| m.file_type().is_dir()).unwrap_or(false)
}

/// The existing child of `dir` whose name lower-cases to `key`, if any.
fn ci_find(dir: &Path, key: &str) -> io::Result<Option<PathBuf>> {
    for e in fs::read_dir(dir)?.flatten() {
        if e.file_name().to_str().is_some_and(|s| s.eq_ignore_ascii_case(key)) {
            return Ok(Some(e.path()));
        }
    }
    Ok(None)
}

/// Rename `from` to `to` unless they are already the same path.
fn rename_if_needed(from: &Path, to: &Path) -> io::Result<()> {
    if from != to {
        fs::rename(from, to)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eidos_fomod::FileItem;

    /// A unique temp directory removed on drop (the crate has no `tempfile` dep).
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let d = std::env::temp_dir()
                .join(format!("eidos-install-test-{}-{}-{}", tag, std::process::id(), n));
            fs::create_dir_all(&d).unwrap();
            TempDir(d)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn file_item(source: &str, destination: &str) -> FileItem {
        FileItem {
            source: source.to_string(),
            destination: destination.to_string(),
            priority: 0,
            is_folder: false,
            always_install: false,
            install_if_usable: false,
            sequence: 0,
        }
    }

    // ---- case-collision normalisation -------------------------------------

    /// Write `content` to `root/rel`, creating parent dirs.
    fn write_at(root: &Path, rel: &str, content: &[u8]) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    /// Force a file's mtime so the "oldest wins" rule is testable deterministically
    /// (writing content stamps mtime to ~now, so set it afterwards). std-only.
    fn set_mtime(root: &Path, rel: &str, secs: u64) {
        let f = fs::OpenOptions::new().write(true).open(root.join(rel)).unwrap();
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs);
        f.set_times(std::fs::FileTimes::new().set_modified(t)).unwrap();
    }

    /// Sorted recursive listing of `root` (dirs end `/`, symlinks end `@`), for
    /// structural assertions and the idempotency snapshot.
    fn rel_paths(root: &Path) -> Vec<String> {
        fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
            let mut entries: Vec<_> = fs::read_dir(dir).unwrap().flatten().collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                let p = e.path();
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
                let ft = fs::symlink_metadata(&p).unwrap().file_type();
                if ft.is_symlink() {
                    out.push(format!("{rel}@"));
                } else if ft.is_dir() {
                    out.push(format!("{rel}/"));
                    walk(base, &p, out);
                } else {
                    out.push(rel);
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out
    }

    #[test]
    fn replace_with_bad_archive_keeps_the_old_mod() {
        // The Replace wipe must come AFTER extraction succeeds: a garbage archive
        // (or a missing 7z) must leave the existing mod untouched.
        let t = TempDir::new("replsafe");
        let mods = t.path().join("mods");
        write_at(&mods, "MyMod/textures/a.dds", b"precious");
        write_at(&mods, "MyMod/meta.ini", b"[General]\nendorsed=1\n");
        let bogus = t.path().join("not-an-archive.7z");
        fs::write(&bogus, b"this is not a 7z file").unwrap();

        let r = install_archive_with_policy(
            &bogus,
            &mods,
            "MyMod",
            "Skyrim Special Edition",
            OverwritePolicy::Replace,
            &eidos_fomod::Context::default(),
        );
        assert!(r.is_err(), "a garbage archive must not install");
        // Whatever the failure (No7z on a bare system, extraction failure with 7z
        // present), the old mod must still be fully intact.
        assert_eq!(fs::read(mods.join("MyMod/textures/a.dds")).unwrap(), b"precious");
        assert_eq!(fs::read(mods.join("MyMod/meta.ini")).unwrap(), b"[General]\nendorsed=1\n");
    }

    #[test]
    fn case_collision_file_same_dir() {
        let t = TempDir::new("ccfsd");
        write_at(t.path(), "meshes/armor.nif", b"a");
        write_at(t.path(), "Meshes/armor.nif", b"b");
        normalize_case_collisions(t.path()).unwrap();
        // One canonical lower-case dir, one file.
        assert_eq!(rel_paths(t.path()), vec!["meshes/".to_string(), "meshes/armor.nif".to_string()]);
    }

    #[test]
    fn case_collision_file_keeps_oldest_mtime() {
        let t = TempDir::new("ccmtime");
        write_at(t.path(), "meshes/armor.nif", b"OLD");
        write_at(t.path(), "Meshes/armor.nif", b"NEW");
        set_mtime(t.path(), "meshes/armor.nif", 100);
        set_mtime(t.path(), "Meshes/armor.nif", 200);
        normalize_case_collisions(t.path()).unwrap();
        assert_eq!(fs::read(t.path().join("meshes/armor.nif")).unwrap(), b"OLD");
    }

    #[test]
    fn case_collision_dir_merge() {
        let t = TempDir::new("ccdm");
        write_at(t.path(), "meshes/a.nif", b"a");
        write_at(t.path(), "Meshes/b.nif", b"b");
        normalize_case_collisions(t.path()).unwrap();
        assert_eq!(
            rel_paths(t.path()),
            vec!["meshes/".to_string(), "meshes/a.nif".to_string(), "meshes/b.nif".to_string()]
        );
    }

    #[test]
    fn case_collision_nested_dir() {
        let t = TempDir::new("ccnd");
        write_at(t.path(), "data/meshes/a.nif", b"a");
        write_at(t.path(), "Data/Meshes/b.nif", b"b");
        normalize_case_collisions(t.path()).unwrap();
        assert_eq!(
            rel_paths(t.path()),
            vec![
                "data/".to_string(),
                "data/meshes/".to_string(),
                "data/meshes/a.nif".to_string(),
                "data/meshes/b.nif".to_string(),
            ]
        );
    }

    #[test]
    fn case_collision_file_vs_dir() {
        let t = TempDir::new("ccfvd");
        fs::write(t.path().join("textures"), b"file").unwrap();
        write_at(t.path(), "Textures/inside.txt", b"in");
        normalize_case_collisions(t.path()).unwrap();
        assert!(t.path().join("textures").is_file(), "file wins the canonical name");
        assert!(
            t.path().join("textures_dir/inside.txt").is_file(),
            "dir is moved aside, contents preserved"
        );
    }

    #[test]
    fn case_collision_file_vs_dir_aside_name_taken() {
        // Regression: `textures` (file) + `Textures/` (dir) move the dir aside to
        // `textures_dir`, but a pre-existing FILE already holds that name. The merge
        // must NOT read_dir a file (ENOTDIR aborts the whole install); both the
        // pre-existing file and the rescued dir contents must survive.
        let t = TempDir::new("ccfvd_taken");
        fs::write(t.path().join("textures"), b"file").unwrap();
        write_at(t.path(), "Textures/inside.txt", b"in");
        fs::write(t.path().join("textures_dir"), b"unrelated").unwrap();
        normalize_case_collisions(t.path()).unwrap();
        assert!(t.path().join("textures").is_file(), "file wins the canonical name");
        assert_eq!(
            fs::read(t.path().join("textures_dir")).unwrap(),
            b"unrelated",
            "pre-existing file at the aside name is untouched"
        );
        assert!(
            t.path().join("textures_dir_1/inside.txt").is_file(),
            "moved-aside dir lands at the first free suffixed name, nothing lost"
        );
    }

    #[test]
    fn case_collision_symlink_preserved() {
        use std::os::unix::fs::symlink;
        let t = TempDir::new("ccsym");
        symlink("some_target", t.path().join("current")).unwrap();
        write_at(t.path(), "Current/f.txt", b"x");
        normalize_case_collisions(t.path()).unwrap();
        // The symlink (opaque) wins the canonical name and is NOT dereferenced.
        let meta = fs::symlink_metadata(t.path().join("current")).unwrap();
        assert!(meta.file_type().is_symlink(), "symlink preserved as-is");
        assert_eq!(fs::read_link(t.path().join("current")).unwrap().to_str(), Some("some_target"));
        assert!(t.path().join("current_dir/f.txt").is_file(), "colliding dir moved aside");
    }

    #[test]
    fn case_collision_non_utf8_skipped() {
        use std::os::unix::ffi::OsStrExt;
        let t = TempDir::new("ccnonutf8");
        // A genuine collision that MUST still resolve.
        write_at(t.path(), "Foo/a.txt", b"a");
        write_at(t.path(), "foo/b.txt", b"b");
        // A non-UTF8 sibling that must be left untouched (no panic).
        let bad = t.path().join(std::ffi::OsStr::from_bytes(b"weird\xff\xfename"));
        fs::write(&bad, b"keep").unwrap();
        normalize_case_collisions(t.path()).unwrap();
        assert!(t.path().join("foo/a.txt").is_file() && t.path().join("foo/b.txt").is_file());
        assert!(bad.exists(), "non-UTF8 entry is skipped, not lost");
    }

    #[test]
    fn case_collision_idempotent() {
        let t = TempDir::new("ccidem");
        write_at(t.path(), "meshes/a.nif", b"a");
        write_at(t.path(), "Meshes/b.nif", b"b");
        write_at(t.path(), "data/Meshes/c.nif", b"c");
        write_at(t.path(), "Data/meshes/d.nif", b"d");
        normalize_case_collisions(t.path()).unwrap();
        let after_first = rel_paths(t.path());
        normalize_case_collisions(t.path()).unwrap();
        assert_eq!(after_first, rel_paths(t.path()), "second pass is a no-op");
    }

    #[test]
    fn case_collision_empty_dir_preserved() {
        let t = TempDir::new("ccempty");
        write_at(t.path(), "Meshes/file.nif", b"x");
        fs::create_dir_all(t.path().join("meshes/empty_subdir")).unwrap();
        normalize_case_collisions(t.path()).unwrap();
        assert!(t.path().join("meshes/file.nif").is_file());
        assert!(t.path().join("meshes/empty_subdir").is_dir(), "empty dir survives the merge");
    }

    #[test]
    fn case_collision_deep_three_way() {
        let t = TempDir::new("ccdeep");
        write_at(t.path(), "A/B/C/file.txt", b"1");
        write_at(t.path(), "a/b/c/file.txt", b"2");
        write_at(t.path(), "A/b/C/file.txt", b"3");
        normalize_case_collisions(t.path()).unwrap();
        // Everything collapses to a single all-lower-case chain with one file.
        assert_eq!(
            rel_paths(t.path()),
            vec![
                "a/".to_string(),
                "a/b/".to_string(),
                "a/b/c/".to_string(),
                "a/b/c/file.txt".to_string(),
            ]
        );
    }

    #[test]
    fn case_collision_no_change_without_collision() {
        let t = TempDir::new("ccnone");
        // Mixed casing but NO sibling collides: every name must keep its casing.
        write_at(t.path(), "Meshes/Armor.nif", b"a");
        write_at(t.path(), "Meshes/Sub/Armor.nif", b"b");
        let before = rel_paths(t.path());
        normalize_case_collisions(t.path()).unwrap();
        assert_eq!(before, rel_paths(t.path()), "non-colliding names are never touched");
    }

    // MO2 parity (copyLeaf): a file with an empty destination lands at
    // <dest>/<source-filename>, not on the mod root dir (which fails EISDIR).
    #[test]
    fn empty_destination_uses_source_filename() {
        let root = TempDir::new("root");
        let dest = TempDir::new("dest");
        fs::write(root.path().join("real.esp"), b"data").unwrap();

        let plan = vec![file_item("real.esp", "")];
        apply_plan(root.path(), &plan, dest.path()).expect("apply_plan");

        let landed = dest.path().join("real.esp");
        assert!(landed.is_file(), "file should land at <dest>/real.esp");
        assert_eq!(fs::read(&landed).unwrap(), b"data");
    }

    // A trailing-slash destination means "into this directory": append the file name.
    #[test]
    fn trailing_slash_destination_uses_source_filename() {
        let root = TempDir::new("root");
        let dest = TempDir::new("dest");
        fs::create_dir_all(root.path().join("Core")).unwrap();
        fs::write(root.path().join("Core").join("real.esp"), b"data").unwrap();

        let plan = vec![file_item("Core/real.esp", "subdir/")];
        apply_plan(root.path(), &plan, dest.path()).expect("apply_plan");

        let landed = dest.path().join("subdir").join("real.esp");
        assert!(landed.is_file(), "file should land at <dest>/subdir/real.esp");
        assert_eq!(fs::read(&landed).unwrap(), b"data");
    }

    // A normal explicit destination is untouched (guards the new branch is gated).
    #[test]
    fn explicit_destination_is_preserved() {
        let root = TempDir::new("root");
        let dest = TempDir::new("dest");
        fs::write(root.path().join("real.esp"), b"data").unwrap();

        let plan = vec![file_item("real.esp", "renamed.esp")];
        apply_plan(root.path(), &plan, dest.path()).expect("apply_plan");

        assert!(dest.path().join("renamed.esp").is_file());
        assert!(!dest.path().join("real.esp").exists());
    }

    #[test]
    fn apply_plan_refuses_path_traversal_destination() {
        // A malicious FOMOD destination that escapes the mod folder must be refused,
        // not written outside it.
        let root = TempDir::new("root");
        let dest = TempDir::new("dest");
        fs::write(root.path().join("evil.esp"), b"data").unwrap();
        // A sentinel that must NOT be overwritten.
        let outside = dest.path().parent().unwrap().join("eidos-traversal-victim");
        let _ = fs::remove_file(&outside);

        let plan = vec![file_item("evil.esp", "../eidos-traversal-victim")];
        let r = apply_plan(root.path(), &plan, dest.path());
        assert!(matches!(r, Err(InstallError::Fomod(_))), "traversal must be refused");
        assert!(!outside.exists(), "nothing must be written outside the mod folder");

        // An absolute destination is refused too.
        let plan2 = vec![file_item("evil.esp", "/tmp/eidos-traversal-abs")];
        assert!(matches!(apply_plan(root.path(), &plan2, dest.path()), Err(InstallError::Fomod(_))));
    }

    #[test]
    fn resolve_ci_refuses_dotdot_source() {
        // A `..` in an attacker-controlled FOMOD source must not read outside root.
        let root = TempDir::new("root");
        fs::create_dir_all(root.path().join("sub")).unwrap();
        assert!(resolve_ci(root.path(), "sub").is_some());
        assert!(resolve_ci(root.path(), "../root").is_none());
        assert!(resolve_ci(root.path(), "sub/../../etc").is_none());
    }

    #[test]
    fn escapes_root_detects_traversal() {
        assert!(escapes_root("../x"));
        assert!(escapes_root("a/../../b"));
        assert!(escapes_root("/abs/path"));
        assert!(!escapes_root("a/b/c.esp"));
        assert!(!escapes_root("textures/foo.dds"));
    }

    #[test]
    fn fail_policy_errors_when_dest_exists() {
        let mods = TempDir::new("mods");
        fs::create_dir_all(mods.path().join("ExistingMod")).unwrap();
        fs::write(mods.path().join("ExistingMod/a.esp"), b"x").unwrap();
        // The archive is never read - Fail returns before extraction (no 7-Zip needed).
        let archive = mods.path().join("whatever.7z");
        let r = install_archive_with_policy(
            &archive,
            mods.path(),
            "ExistingMod",
            "skyrimse",
            OverwritePolicy::Fail,
            &eidos_fomod::Context::default(),
        );
        assert!(matches!(r, Err(InstallError::Exists(_))));
    }

    #[test]
    fn fomod_context_marks_present_plugins_active() {
        let game = TempDir::new("game");
        let modd = TempDir::new("mod");
        fs::write(game.path().join("Skyrim.esm"), b"").unwrap();
        fs::write(modd.path().join("SkyUI.esp"), b"").unwrap();
        let ctx = fomod_context(game.path(), &[modd.path().to_path_buf()], &[]);
        // A present plugin reads Active (so fileDependency state="Active" holds); an
        // absent one is left out, which eval treats as Missing.
        assert_eq!(ctx.file_states.get("skyrim.esm").map(String::as_str), Some("Active"));
        assert_eq!(ctx.file_states.get("skyui.esp").map(String::as_str), Some("Active"));
        assert_eq!(ctx.file_states.get("absent.esp"), None);
    }

    #[test]
    fn fomod_context_distinguishes_inactive_from_missing() {
        let game = TempDir::new("game");
        let en = TempDir::new("enabled");
        let dis = TempDir::new("disabled");
        fs::write(en.path().join("Active.esp"), b"").unwrap();
        fs::write(dis.path().join("Disabled.esp"), b"").unwrap();
        // A plugin shipped by BOTH an enabled and a disabled mod must read Active.
        fs::write(en.path().join("Shared.esp"), b"").unwrap();
        fs::write(dis.path().join("Shared.esp"), b"").unwrap();
        let ctx = fomod_context(
            game.path(),
            &[en.path().to_path_buf()],
            &[dis.path().to_path_buf()],
        );
        assert_eq!(ctx.file_states.get("active.esp").map(String::as_str), Some("Active"));
        assert_eq!(ctx.file_states.get("disabled.esp").map(String::as_str), Some("Inactive"));
        assert_eq!(ctx.file_states.get("shared.esp").map(String::as_str), Some("Active"));
        assert_eq!(ctx.file_states.get("absent.esp"), None); // -> Missing
    }

    #[test]
    fn reapply_user_meta_restores_endorsement_and_category() {
        let dir = TempDir::new("meta");
        let old_path = dir.path().join("old.ini");
        fs::write(&old_path, "[General]\nendorsed=1\ncategory=\"42,\"\ntracked=1\n").unwrap();
        let old = ModMeta::read(&old_path);

        let new_path = dir.path().join("new.ini");
        fs::write(&new_path, "[General]\nendorsed=0\ncategory=\"-1,\"\ntracked=0\n").unwrap();
        reapply_user_meta(&old, &new_path);

        let s = fs::read_to_string(&new_path).unwrap();
        assert!(s.contains("endorsed=1"));
        assert!(s.contains("tracked=1"));
        assert!(s.contains("category=\"42,\""));
    }

    #[test]
    fn apply_plan_reports_missing_sources() {
        let root = TempDir::new("root");
        let dest = TempDir::new("dest");
        fs::write(root.path().join("present.esp"), b"x").unwrap();
        let plan = vec![file_item("present.esp", "present.esp"), file_item("absent.esp", "absent.esp")];
        let missing = apply_plan(root.path(), &plan, dest.path()).unwrap();
        // The source the archive didn't contain is reported, the present one installed.
        assert_eq!(missing, vec!["absent.esp".to_string()]);
        assert!(dest.path().join("present.esp").is_file());
    }

    #[test]
    fn mod_name_for_prefers_sidecar_then_sanitizes() {
        let dir = TempDir::new("name");
        let archive = dir.path().join("Beyond Skyrim Bruma-1234-1-0.7z");
        // No sidecar -> the filename guess (clean here).
        assert_eq!(mod_name_for(&archive), "Beyond Skyrim Bruma");
        // A sidecar modName wins, and its ':' is sanitized out of the folder name.
        fs::write(
            PathBuf::from(format!("{}.meta", archive.display())),
            "[General]\nmodName=Beyond Skyrim: Bruma\nname=file name\n",
        )
        .unwrap();
        assert_eq!(mod_name_for(&archive), "Beyond Skyrim Bruma");
    }

    // ---- BAIN sub-packages + manual data root --------------------------------

    /// An [`ExtractedTree`] over a directory laid out by hand: the tests need the
    /// post-extraction state without paying for a real 7-Zip run. Dropping it removes
    /// the directory, exactly as a real extraction's would.
    fn extracted(dir: &Path) -> ExtractedTree {
        ExtractedTree { tmp: dir.to_path_buf() }
    }

    /// A `mods/` dir plus an extraction temp inside it (where a real install puts it).
    fn bain_layout(tag: &str) -> (TempDir, PathBuf, PathBuf) {
        let t = TempDir::new(tag);
        let mods = t.path().join("mods");
        let tmp = mods.join(".extract");
        fs::create_dir_all(&tmp).unwrap();
        (t, mods, tmp)
    }

    /// A three-sub-package BAIN pack whose `00 Core` and `01 Optional` both ship
    /// `textures/shared.dds`, so the merge order is observable.
    fn bain_pack(tag: &str) -> (TempDir, PathBuf, PathBuf, PathBuf) {
        let (t, mods, tmp) = bain_layout(tag);
        write_at(&tmp, "00 Core/MyMod.esp", b"core");
        write_at(&tmp, "00 Core/textures/shared.dds", b"CORE");
        write_at(&tmp, "01 Optional/textures/shared.dds", b"OPTIONAL");
        write_at(&tmp, "01 Optional/textures/extra.dds", b"extra");
        write_at(&tmp, "02 Unwanted/meshes/no.nif", b"no");
        let archive = t.path().join("Pack-1234-1-0.7z");
        fs::write(&archive, b"x").unwrap();
        (t, mods, tmp, archive)
    }

    #[test]
    fn bain_merges_chosen_subpackages_later_wins() {
        let (_t, mods, tmp, archive) = bain_pack("bainmerge");
        let tree = extracted(&tmp);
        let picks = vec!["00 Core".to_string(), "01 Optional".to_string()];
        let r = install_bain(&tree, &picks, &archive, &mods, "Pack", "skyrimse", OverwritePolicy::Fail)
            .expect("bain install");

        // Both chosen sub-packages are merged; the unticked one is not installed.
        assert!(r.dest.join("MyMod.esp").is_file());
        assert!(r.dest.join("textures/extra.dds").is_file());
        assert!(!r.dest.join("meshes").exists());
        // The BAIN contract: a later sub-package overwrites an earlier one's file.
        assert_eq!(fs::read(r.dest.join("textures/shared.dds")).unwrap(), b"OPTIONAL");
        assert!(r.dest.join("meta.ini").is_file(), "a BAIN install writes meta.ini like any other");
        assert!(!r.fomod);
    }

    #[test]
    fn bain_merge_order_is_the_callers_not_the_archives() {
        // Same pack, reversed ticks: now the core file survives. This is why the API
        // takes an ordered list instead of a set.
        let (_t, mods, tmp, archive) = bain_pack("bainorder");
        let tree = extracted(&tmp);
        let picks = vec!["01 Optional".to_string(), "00 Core".to_string()];
        let r = install_bain(&tree, &picks, &archive, &mods, "Pack", "skyrimse", OverwritePolicy::Fail)
            .expect("bain install (reversed)");
        assert_eq!(fs::read(r.dest.join("textures/shared.dds")).unwrap(), b"CORE");
    }

    #[test]
    fn bain_matches_subpackage_names_case_insensitively() {
        // The extraction may have case-folded a colliding folder name, so a pick that
        // no longer matches byte-for-byte must still resolve.
        let (t, mods, tmp) = bain_layout("bainci");
        write_at(&tmp, "00 core/MyMod.esp", b"x");
        let archive = t.path().join("Pack.7z");
        fs::write(&archive, b"x").unwrap();
        let tree = extracted(&tmp);
        let r = install_bain(
            &tree,
            &["00 Core".to_string()],
            &archive,
            &mods,
            "Pack",
            "skyrimse",
            OverwritePolicy::Fail,
        )
        .expect("bain install");
        assert!(r.dest.join("MyMod.esp").is_file());
    }

    #[test]
    fn bain_refuses_a_stale_or_malformed_selection() {
        let (t, mods, tmp) = bain_layout("bainbad");
        write_at(&tmp, "00 Core/MyMod.esp", b"x");
        let archive = t.path().join("Pack.7z");
        fs::write(&archive, b"x").unwrap();
        let tree = extracted(&tmp);

        for bad in [
            vec![],                                  // nothing ticked
            vec!["99 Nope".to_string()],             // not in the archive
            vec!["../outside".to_string()],          // traversal
            vec!["00 Core/textures".to_string()],    // not a top-level sub-package
        ] {
            let r = install_bain(&tree, &bad, &archive, &mods, "Pack", "skyrimse", OverwritePolicy::Fail);
            assert!(matches!(r, Err(InstallError::BadSelection(_))), "must refuse {bad:?}");
        }
        // A refused selection must not leave a half-made mod folder behind.
        assert!(!mods.join("Pack").exists());
    }

    #[test]
    fn bain_replace_keeps_the_old_mod_when_the_selection_is_stale() {
        // Destructive-step-last, the discipline the crate already keeps: a selection
        // that no longer resolves must be caught BEFORE the Replace wipe.
        let (t, mods, tmp) = bain_layout("bainrepl");
        write_at(&mods, "Pack/textures/a.dds", b"precious");
        write_at(&mods, "Pack/meta.ini", b"[General]\nendorsed=1\n");
        write_at(&tmp, "00 Core/MyMod.esp", b"x");
        write_at(&tmp, "01 Extras/meshes/a.nif", b"y");
        let archive = t.path().join("Pack.7z");
        fs::write(&archive, b"x").unwrap();
        let tree = extracted(&tmp);

        let r = install_bain(
            &tree,
            &["00 Core".to_string(), "99 Gone".to_string()],
            &archive,
            &mods,
            "Pack",
            "skyrimse",
            OverwritePolicy::Replace,
        );
        assert!(matches!(r, Err(InstallError::BadSelection(_))));
        assert_eq!(fs::read(mods.join("Pack/textures/a.dds")).unwrap(), b"precious");
        assert_eq!(fs::read(mods.join("Pack/meta.ini")).unwrap(), b"[General]\nendorsed=1\n");
    }

    #[test]
    fn failed_bain_install_cleans_up_its_fresh_destination() {
        // A late failure (here: a sub-package shipping a DIRECTORY named meta.ini, so
        // writing the real one fails) must not leave debris the mod list would show as
        // an installed mod.
        let (t, mods, tmp) = bain_layout("bainclean");
        write_at(&tmp, "00 Core/MyMod.esp", b"x");
        write_at(&tmp, "01 Extras/meta.ini/oops.txt", b"a directory where a file goes");
        write_at(&tmp, "01 Extras/meshes/a.nif", b"y");
        let archive = t.path().join("Pack.7z");
        fs::write(&archive, b"x").unwrap();
        let tree = extracted(&tmp);

        let r = install_bain(
            &tree,
            &["00 Core".to_string(), "01 Extras".to_string()],
            &archive,
            &mods,
            "Pack",
            "skyrimse",
            OverwritePolicy::Fail,
        );
        assert!(r.is_err(), "writing meta.ini over a directory must fail");
        assert!(!mods.join("Pack").exists(), "the fresh destination must be cleaned up");
    }

    #[test]
    fn manual_installs_from_the_chosen_root() {
        let (t, mods, tmp) = bain_layout("manualroot");
        write_at(&tmp, "Package/Data/meshes/a.nif", b"mesh");
        write_at(&tmp, "Package/Data/MyMod.esp", b"esp");
        write_at(&tmp, "Package/src/build.sh", b"tool");
        write_at(&tmp, "readme.txt", b"docs");
        let archive = t.path().join("Odd-1234-1-0.7z");
        fs::write(&archive, b"x").unwrap();
        let tree = extracted(&tmp);

        let r =
            install_manual(&tree, "Package/Data", &archive, &mods, "Odd", "skyrimse", OverwritePolicy::Fail)
                .expect("manual install");
        // Everything under the chosen root becomes the mod; everything beside it is
        // dropped, exactly like the wrapper strip on the simple path.
        assert!(r.dest.join("meshes/a.nif").is_file());
        assert!(r.dest.join("MyMod.esp").is_file());
        assert!(!r.dest.join("src").exists());
        assert!(!r.dest.join("readme.txt").exists());
        assert_eq!(r.stripped, "Package/Data/");
        assert!(r.dest.join("meta.ini").is_file());
    }

    #[test]
    fn manual_root_is_matched_case_insensitively() {
        let (t, mods, tmp) = bain_layout("manualci");
        write_at(&tmp, "Package/Data/MyMod.esp", b"esp");
        let archive = t.path().join("Odd.7z");
        fs::write(&archive, b"x").unwrap();
        let tree = extracted(&tmp);
        let r =
            install_manual(&tree, "package/data", &archive, &mods, "Odd", "skyrimse", OverwritePolicy::Fail)
                .expect("manual install");
        assert!(r.dest.join("MyMod.esp").is_file());
    }

    #[test]
    fn manual_empty_root_installs_the_archive_as_is() {
        // The user's "this IS already the data dir" - nothing is stripped.
        let (t, mods, tmp) = bain_layout("manualasis");
        write_at(&tmp, "Package/Data/MyMod.esp", b"esp");
        let archive = t.path().join("Odd.7z");
        fs::write(&archive, b"x").unwrap();
        let tree = extracted(&tmp);
        let r = install_manual(&tree, "", &archive, &mods, "Odd", "skyrimse", OverwritePolicy::Fail)
            .expect("manual install at root");
        assert!(r.dest.join("Package/Data/MyMod.esp").is_file());
        assert_eq!(r.stripped, "");
    }

    #[test]
    fn manual_refuses_a_root_outside_the_archive() {
        let (t, mods, tmp) = bain_layout("manualesc");
        write_at(&tmp, "Package/MyMod.esp", b"esp");
        let archive = t.path().join("Odd.7z");
        fs::write(&archive, b"x").unwrap();
        let tree = extracted(&tmp);
        for bad in ["../..", "Package/../../mods", "/etc", "nope"] {
            let r =
                install_manual(&tree, bad, &archive, &mods, "Odd", "skyrimse", OverwritePolicy::Fail);
            assert!(matches!(r, Err(InstallError::BadSelection(_))), "must refuse root '{bad}'");
        }
        assert!(!mods.join("Odd").exists());
    }

    #[test]
    fn overlay_replaces_a_conflicting_entry_type() {
        // Two sub-packages disagreeing on whether a name is a file or a directory: the
        // later one wins outright, rather than aborting the install with EISDIR.
        let t = TempDir::new("overlaytype");
        write_at(t.path(), "a/conflict/inner.txt", b"dir");
        write_at(t.path(), "b/conflict", b"file");
        write_at(t.path(), "b/other/x.txt", b"x");
        let dest = t.path().join("dest");
        overlay_dir(&t.path().join("a"), &dest).unwrap();
        overlay_dir(&t.path().join("b"), &dest).unwrap();
        assert_eq!(fs::read(dest.join("conflict")).unwrap(), b"file");
        assert!(dest.join("other/x.txt").is_file());
        // ...and the reverse, a directory landing on a file.
        let dest2 = t.path().join("dest2");
        overlay_dir(&t.path().join("b"), &dest2).unwrap();
        overlay_dir(&t.path().join("a"), &dest2).unwrap();
        assert!(dest2.join("conflict/inner.txt").is_file());
    }

    #[test]
    fn overlay_never_writes_through_a_planted_symlink() {
        // Security: an earlier sub-package ships `victim -> <outside>`, a later one
        // ships a real file of the same name. The copy must replace the LINK, never
        // follow it and write outside the mod folder.
        use std::os::unix::fs::symlink;
        let t = TempDir::new("overlayescape");
        let outside = t.path().join("outside.txt");
        fs::write(&outside, b"untouched").unwrap();
        fs::create_dir_all(t.path().join("a")).unwrap();
        symlink(&outside, t.path().join("a/victim")).unwrap();
        write_at(t.path(), "b/victim", b"payload");

        let dest = t.path().join("dest");
        overlay_dir(&t.path().join("a"), &dest).unwrap();
        overlay_dir(&t.path().join("b"), &dest).unwrap();
        assert_eq!(fs::read(&outside).unwrap(), b"untouched", "must not write through the link");
        assert_eq!(fs::read(dest.join("victim")).unwrap(), b"payload");
        assert!(!fs::symlink_metadata(dest.join("victim")).unwrap().file_type().is_symlink());
    }

    #[test]
    fn overlay_preserves_a_dangling_symlink() {
        // 7-Zip does extract symlinks; copying THROUGH a dangling one would fail the
        // whole install over an entry the simple (rename) path installs fine.
        use std::os::unix::fs::symlink;
        let t = TempDir::new("overlaysym");
        fs::create_dir_all(t.path().join("a")).unwrap();
        symlink("nowhere", t.path().join("a/link")).unwrap();
        let dest = t.path().join("dest");
        overlay_dir(&t.path().join("a"), &dest).unwrap();
        let meta = fs::symlink_metadata(dest.join("link")).unwrap();
        assert!(meta.file_type().is_symlink());
        assert_eq!(fs::read_link(dest.join("link")).unwrap().to_str(), Some("nowhere"));
    }

    #[test]
    fn from_dir_feeds_bain_detection_with_real_names() {
        // End-to-end over an actual directory: the disk layout an extraction leaves
        // must classify as BAIN, with the folder names as they exist on disk.
        let (_t, _mods, tmp) = bain_layout("bainfromdir");
        write_at(&tmp, "00 Core/MyMod.esp", b"x");
        write_at(&tmp, "01 Optional Textures/textures/a.dds", b"y");
        write_at(&tmp, "--03 Disabled/meshes/a.nif", b"z");
        write_at(&tmp, "Docs/readme.txt", b"d");
        let layout = ArchiveTree::from_dir(&tmp).unwrap();
        assert_eq!(layout.simple_archive_base(), None, "not a simple archive");
        let (subs, invalid) = layout.bain_subpackages();
        assert_eq!(subs, vec!["00 Core", "01 Optional Textures"]);
        assert_eq!(invalid, 0);
    }

    #[test]
    fn civil_from_unix_is_correct() {
        assert_eq!(civil_from_unix(1_700_000_000), (2023, 11, 14)); // 2023-11-14 UTC
        assert_eq!(civil_from_unix(0), (1970, 1, 1));
    }
}
