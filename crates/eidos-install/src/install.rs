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

use eidos_instance::ModMeta;

use crate::{fix_directory_name, guess_mod_name, guess_mod_name_and_id, ArchiveEntry, ArchiveTree};

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
    /// The target `mods/<name>/` already exists and is not empty.
    Exists(PathBuf),
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
            InstallError::Exists(p) => write!(f, "target already exists: {}", p.display()),
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

/// Install `archive` into `mods_dir/name`, MO2 Simple-installer style: extract,
/// strip the wrapper folder to the Data-relative root, move it in, and write a
/// MO2-compatible `meta.ini` (seeded from a `<archive>.meta` sidecar if present).
pub fn install_archive(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
    game_name: &str,
) -> Result<InstallReport, InstallError> {
    let bin = find_7z().ok_or(InstallError::No7z)?;

    // Sanitize the folder name (a real Nexus modName can contain ':' etc.) and
    // recover the mod id from the filename for the meta.ini when there's no sidecar.
    let name = fix_directory_name(name).unwrap_or_else(|| "Mod".to_string());
    let (_, guessed_id) = guess_mod_name_and_id(&archive.to_string_lossy());

    let dest = mods_dir.join(&name);
    if dest.exists() && is_nonempty_dir(&dest) {
        return Err(InstallError::Exists(dest));
    }

    // Extract into a same-filesystem temp so the final move is a rename.
    let tmp = mods_dir.join(format!(
        ".eidos-install-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&tmp)?;

    let result = (|| {
        extract_all(bin, archive, &tmp)?;
        let tree = ArchiveTree::from_dir(&tmp)?;
        let base = match tree.simple_archive_base() {
            Some(b) => b,
            None => {
                // A FOMOD scripted installer: run it with the default selections.
                if let Some(fomod_root) = find_fomod_root(&tmp) {
                    fs::create_dir_all(&dest)?;
                    apply_fomod_defaults(&fomod_root, &dest)?;
                    write_meta(archive, &dest, game_name, guessed_id)?;
                    return Ok(InstallReport {
                        name: name.clone(),
                        stripped: String::new(),
                        fomod: true,
                        dest: dest.clone(),
                    });
                }
                return Err(InstallError::NotSimple);
            }
        };
        let src = if base.is_empty() {
            tmp.clone()
        } else {
            tmp.join(base.trim_end_matches('/'))
        };
        fs::create_dir_all(&dest)?;
        move_dir_contents(&src, &dest)?;
        write_meta(archive, &dest, game_name, guessed_id)?;
        Ok(InstallReport { name: name.clone(), stripped: base, fomod: false, dest: dest.clone() })
    })();

    let _ = fs::remove_dir_all(&tmp);
    result
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
fn apply_plan(root: &Path, plan: &[eidos_fomod::FileItem], dest: &Path) -> Result<(), InstallError> {
    for item in plan {
        let Some(src) = resolve_ci(root, &item.source) else {
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
    Ok(())
}

/// Parse the FOMOD under `root` and install it with the default selections.
fn apply_fomod_defaults(root: &Path, dest: &Path) -> Result<(), InstallError> {
    let config = parse_fomod_at(root)?;
    let plan = eidos_fomod::build_default_plan(&config, &eidos_fomod::Context::default());
    apply_plan(root, &plan, dest)
}

/// A FOMOD extracted and parsed, awaiting the user's choices (the GUI wizard). The
/// extraction temp is removed when the session is dropped.
pub struct FomodSession {
    pub config: eidos_fomod::ModuleConfig,
    root: PathBuf,
    tmp: PathBuf,
    name: String,
    archive: PathBuf,
}

impl Drop for FomodSession {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.tmp);
    }
}

impl FomodSession {
    /// Resolve a FOMOD-relative path (e.g. a plugin or module image) to its
    /// extracted on-disk path, matching each component case-insensitively. Returns
    /// `None` if the archive did not ship it.
    pub fn resolve(&self, rel: &str) -> Option<PathBuf> {
        resolve_ci(&self.root, rel)
    }
}

/// Extract `archive`; if it is a FOMOD, return a session whose `config` drives a
/// wizard. Returns `Ok(None)` for a non-FOMOD archive (use [`install_archive`]).
pub fn open_fomod(
    archive: &Path,
    mods_dir: &Path,
    name: &str,
) -> Result<Option<FomodSession>, InstallError> {
    let bin = find_7z().ok_or(InstallError::No7z)?;
    let tmp = mods_dir.join(format!(
        ".eidos-install-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&tmp)?;
    if let Err(e) = extract_all(bin, archive, &tmp) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }
    let Some(root) = find_fomod_root(&tmp) else {
        let _ = fs::remove_dir_all(&tmp);
        return Ok(None);
    };
    let config = match parse_fomod_at(&root) {
        Ok(c) => c,
        Err(e) => {
            let _ = fs::remove_dir_all(&tmp);
            return Err(e);
        }
    };
    Ok(Some(FomodSession {
        config,
        root,
        tmp,
        name: name.to_string(),
        archive: archive.to_path_buf(),
    }))
}

/// Apply the chosen selection and finish the FOMOD install.
pub fn finish_fomod(
    session: FomodSession,
    selection: &eidos_fomod::Selection,
    mods_dir: &Path,
    game_name: &str,
) -> Result<InstallReport, InstallError> {
    let name = fix_directory_name(&session.name).unwrap_or_else(|| "Mod".to_string());
    let (_, guessed_id) = guess_mod_name_and_id(&session.archive.to_string_lossy());
    let dest = mods_dir.join(&name);
    if dest.exists() && is_nonempty_dir(&dest) {
        return Err(InstallError::Exists(dest));
    }
    fs::create_dir_all(&dest)?;
    let plan = eidos_fomod::build_plan(&session.config, selection, &eidos_fomod::Context::default());
    apply_plan(&session.root, &plan, &dest)?;
    write_meta(&session.archive, &dest, game_name, guessed_id)?;
    Ok(InstallReport { name, stripped: String::new(), fomod: true, dest })
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

    #[test]
    fn civil_from_unix_is_correct() {
        assert_eq!(civil_from_unix(1_700_000_000), (2023, 11, 14)); // 2023-11-14 UTC
        assert_eq!(civil_from_unix(0), (1970, 1, 1));
    }
}
