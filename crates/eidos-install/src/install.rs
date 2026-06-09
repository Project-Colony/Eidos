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

use crate::{ArchiveEntry, ArchiveTree};

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

    let dest = mods_dir.join(name);
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
                return Err(if tree.has_fomod() {
                    InstallError::NeedsFomod
                } else {
                    InstallError::NotSimple
                });
            }
        };
        let src = if base.is_empty() {
            tmp.clone()
        } else {
            tmp.join(base.trim_end_matches('/'))
        };
        fs::create_dir_all(&dest)?;
        move_dir_contents(&src, &dest)?;
        write_meta(archive, &dest, game_name)?;
        Ok(InstallReport { name: name.to_string(), stripped: base, dest: dest.clone() })
    })();

    let _ = fs::remove_dir_all(&tmp);
    result
}

/// Write a MO2-compatible `meta.ini`, seeded from the download's `<archive>.meta`
/// sidecar if MO2/Nexus left one next to the file.
fn write_meta(archive: &Path, dest: &Path, game_name: &str) -> io::Result<()> {
    // The sidecar is the full archive name + ".meta" (e.g. Mod-1234.7z.meta).
    let sidecar = PathBuf::from(format!("{}.meta", archive.to_string_lossy()));
    let from = ModMeta::read(&sidecar);

    let mut meta = ModMeta::default();
    meta.set("gameName", &from.game_name().unwrap_or_else(|| game_name.to_string()));
    if let Some(id) = from.mod_id() {
        meta.set("modid", &id.to_string());
    }
    if let Some(v) = from.version() {
        meta.set("version", &v);
    }
    if let Some(nv) = from.newest_version() {
        meta.set("newestVersion", &nv);
    }
    // The sidecar's category is a raw Nexus id we don't map yet; leave uncategorised.
    meta.set("category", "\"-1,\"");
    if let Some(file) = archive.file_name().and_then(|f| f.to_str()) {
        meta.set("installationFile", file);
    }
    meta.set("repository", &from.repository().unwrap_or_else(|| "Nexus".to_string()));
    meta.set("endorsed", "0");
    meta.set("tracked", "0");
    meta.write(&dest.join("meta.ini"))
}
