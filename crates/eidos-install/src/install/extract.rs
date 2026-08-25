//! The 7-Zip backend: listing, extraction into a unique temp dir, and the
//! [`ExtractedTree`] that owns it.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use std::fs;
use std::io;
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

/// Rebuild a real directory tree from an archive that used BACKSLASHES as its
/// path separator.
///
/// The ZIP spec is unambiguous (APPNOTE 4.4.17.1: the separator is `/`), but
/// Windows tools write `\` anyway and nobody notices, because on Windows 7-Zip
/// treats both as separators. On Linux a backslash is an ordinary filename
/// character, so p7zip extracts `Mod\Data\F4SE\Plugins\CBP.dll` as ONE file
/// whose name contains backslashes, at the top level. The result is a flat pile
/// with no `Data/` and no `fomod/`, so wrapper detection finds nothing, the
/// FOMOD is invisible, and the install fails in a way that looks like the mod is
/// broken. Observed on OpenCBP for Fallout 4, 15 entries out of 15.
///
/// Only fires when at least half the top-level entries carry a backslash. One
/// oddly-named file in an otherwise normal archive is left exactly as it is: a
/// backslash is legal here, and silently restructuring somebody's mod because of
/// one filename would be worse than the problem.
///
/// Returns how many entries were rebuilt.
fn unflatten_backslash_paths(root: &Path) -> io::Result<usize> {
    let entries: Vec<_> = fs::read_dir(root)?.filter_map(Result::ok).collect();
    let (mut flat, mut plain) = (Vec::new(), 0usize);
    for e in entries {
        match e.file_name().to_string_lossy().contains('\\') {
            true => flat.push(e),
            false => plain += 1,
        }
    }
    if flat.is_empty() || flat.len() < plain {
        return Ok(0);
    }

    // Directories last: the flat extraction leaves the archive's directory
    // entries behind as empty placeholders (`Mod\Data\`), and moving the files
    // first means those placeholders are then either redundant or already
    // created by `create_dir_all` below.
    flat.sort_by_key(|e| e.path().is_dir());

    let mut moved = 0;
    for e in flat {
        let raw = e.file_name().to_string_lossy().into_owned();
        let Some(rel) = safe_relative(&raw) else {
            // Refused rather than repaired. Splitting on backslashes is exactly
            // what turns `..\..\etc\passwd` from one harmlessly-named file
            // into a path that climbs out of the temp directory, so this guard
            // is not theoretical - it is a hole this function would otherwise
            // have opened.
            continue;
        };
        let dest = root.join(&rel);
        let src = e.path();
        if src.is_dir() {
            // A placeholder. Make sure the directory exists under its real name
            // and drop the placeholder; `remove_dir` refuses a non-empty one, so
            // an unexpected tree is left alone rather than lost.
            fs::create_dir_all(&dest)?;
            let _ = fs::remove_dir(&src);
            moved += 1;
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&src, &dest)?;
        moved += 1;
    }
    Ok(moved)
}

/// A backslash-separated archive name as a relative path, or `None` if it tries
/// to leave the tree. Every component must be an ordinary name: `..` climbs out,
/// an absolute component would escape entirely, and empty components come from
/// doubled separators and mean nothing.
fn safe_relative(raw: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    let mut any = false;
    for part in raw.split('\\') {
        let part = part.trim_end_matches('/');
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains('/') || part.starts_with('/') {
            return None;
        }
        out.push(part);
        any = true;
    }
    any.then_some(out)
}

/// Extract `archive` into a fresh temp directory beside `mods_dir`, rebuild any
/// backslash-flattened tree and heal NTFS-style case collisions, so everything
/// downstream (wrapper detection, FOMOD lookup, the move) reads a consistent
/// tree. The returned handle owns the
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
    // Before the case pass, not after: this one CREATES the directories that the
    // case pass then reconciles.
    match unflatten_backslash_paths(&tree.tmp) {
        Ok(0) => {}
        Ok(n) => eidos_log::info!(
            "eidos install: the archive used Windows path separators; rebuilt {n} entries into a real folder tree"
        ),
        Err(e) => eidos_log::warn!("eidos install: could not rebuild the archive's folder tree ({e})"),
    }
    normalize_case_collisions(&tree.tmp)?;
    Ok(tree)
}

#[cfg(test)]
mod backslash_tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join(format!("eidos-bs-{}-{tag}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_windows_separated_archive_becomes_a_real_tree() {
        // Exactly what p7zip leaves behind for OpenCBP_FO4-3.5.240.zip: every
        // entry at the top level, every name carrying its own path.
        let d = tmp("real");
        for f in [
            r"OpenCBP_FO4-3.5.240\Data\F4SE\Plugins\CBP.dll",
            r"OpenCBP_FO4-3.5.240\Data\Scripts\OCBP_API.pex",
            r"OpenCBP_FO4-3.5.240\FOMOD\info.xml",
            r"OpenCBP_FO4-3.5.240\OCBP_LICENSE",
        ] {
            fs::write(d.join(f), b"x").unwrap();
        }
        // The empty directory placeholders the archive also carried.
        for dir in [r"OpenCBP_FO4-3.5.240\Data\", r"OpenCBP_FO4-3.5.240\Data\F4SE\"] {
            fs::create_dir(d.join(dir)).unwrap();
        }

        let n = unflatten_backslash_paths(&d).unwrap();
        assert_eq!(n, 6);

        // The two things every later step looks for and could not see before.
        assert!(d.join("OpenCBP_FO4-3.5.240/Data/F4SE/Plugins/CBP.dll").is_file());
        assert!(d.join("OpenCBP_FO4-3.5.240/FOMOD/info.xml").is_file());
        assert!(d.join("OpenCBP_FO4-3.5.240/Data/Scripts/OCBP_API.pex").is_file());
        assert!(d.join("OpenCBP_FO4-3.5.240/OCBP_LICENSE").is_file());
        // And nothing backslash-named survives at the top.
        assert!(
            !fs::read_dir(&d).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().contains('\\')),
            "a flattened name was left behind"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn one_odd_filename_in_a_normal_archive_is_left_alone() {
        // A backslash is a legal character here. Restructuring somebody's mod
        // because ONE file is named strangely would be worse than the bug.
        let d = tmp("odd");
        fs::create_dir(d.join("Data")).unwrap();
        fs::write(d.join("readme.txt"), b"x").unwrap();
        fs::write(d.join("Data/a.esp"), b"x").unwrap();
        fs::write(d.join(r"weird\name.txt"), b"x").unwrap();

        assert_eq!(unflatten_backslash_paths(&d).unwrap(), 0, "must not fire on a minority");
        assert!(d.join(r"weird\name.txt").is_file(), "the odd name must survive untouched");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_traversal_attempt_is_refused_not_repaired() {
        // Splitting on backslashes is what would turn this from one harmlessly
        // named file into a write outside the temp directory.
        let d = tmp("evil");
        fs::write(d.join(r"..\..\escaped.txt"), b"x").unwrap();
        fs::write(d.join(r"Mod\Data\ok.esp"), b"x").unwrap();

        let n = unflatten_backslash_paths(&d).unwrap();
        assert_eq!(n, 1, "only the safe entry may be rebuilt");
        assert!(d.join("Mod/Data/ok.esp").is_file());
        assert!(d.join(r"..\..\escaped.txt").is_file(), "left in place, not moved");
        assert!(!d.parent().unwrap().parent().unwrap().join("escaped.txt").exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn safe_relative_accepts_ordinary_names_and_refuses_the_rest() {
        assert_eq!(safe_relative(r"Mod\Data\a.esp"), Some(PathBuf::from("Mod/Data/a.esp")));
        // Doubled separators and a trailing one are noise, not structure.
        assert_eq!(safe_relative(r"Mod\\Data\"), Some(PathBuf::from("Mod/Data")));
        assert_eq!(safe_relative(r"..\x"), None);
        assert_eq!(safe_relative(r"Mod\..\..\x"), None);
        // A forward slash inside a component means the name was never purely
        // backslash-separated; refuse rather than guess which one is structure.
        assert_eq!(safe_relative(r"Mod\a/b.esp"), None);
        assert_eq!(safe_relative(r"\"), None);
    }
}
