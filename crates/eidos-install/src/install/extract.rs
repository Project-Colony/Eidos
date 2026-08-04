//! The 7-Zip backend: listing, extraction into a unique temp dir, and the
//! [`ExtractedTree`] that owns it.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};




use super::*;

pub(crate) static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// The first usable 7-Zip binary on `PATH`.
pub(crate) fn find_7z() -> Option<&'static str> {
    ["7z", "7zz", "7za"].into_iter().find(|b| Command::new(b).output().is_ok())
}

pub(crate) fn extract_all(bin: &str, archive: &Path, dest: &Path) -> Result<(), InstallError> {
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

/// An archive already extracted into a temp directory beside `mods/`, with NTFS
/// case collisions healed. Holding one means the expensive 7-Zip pass is already
/// paid for: [`install_extracted`] installs straight from it, so a simple archive
/// is never extracted twice. The temp is removed when this is dropped.
pub struct ExtractedTree {
    pub(crate) tmp: PathBuf,
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
