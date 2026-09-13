//! The 7-Zip backend: listing, extraction into a unique temp dir, and the
//! [`ExtractedTree`] that owns it.

//! The archive backend + the Simple-install flow: extract, find the Data-relative
//! root (stripping the wrapper folder), move it into `mods/<name>/`, write a
//! MO2-compatible `meta.ini`. Like MO2, extraction is delegated to 7-Zip, which
//! handles `.7z`/`.zip`/`.rar` uniformly.

use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::*;

pub(crate) static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Keeps extraction and its receipt attached to one open regular file.
/// ponytail: a descriptor plus metadata avoids copying whole archives to another staging file.
#[derive(Debug)]
pub(crate) struct ArchiveOrigin {
    file: fs::File,
    path: PathBuf,
    stamp: [u64; 7],
    sha256: String,
}

fn archive_stamp(meta: &fs::Metadata) -> [u64; 7] {
    [
        meta.dev(),
        meta.ino(),
        meta.len(),
        meta.mtime() as u64,
        meta.mtime_nsec() as u64,
        meta.ctime() as u64,
        meta.ctime_nsec() as u64,
    ]
}

impl ArchiveOrigin {
    pub(crate) fn capture(path: &Path, cancel: &AtomicBool) -> Result<Self, InstallError> {
        Self::capture_bounded(path, cancel, u64::MAX)
    }

    pub(crate) fn capture_bounded(
        path: &Path,
        cancel: &AtomicBool,
        max_bytes: u64,
    ) -> Result<Self, InstallError> {
        if cancel.load(Ordering::Relaxed) {
            return Err(InstallError::BadSelection(
                "Archive operation cancelled".into(),
            ));
        }
        let path = path.canonicalize()?;
        let mut file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)?;
        let meta = file.metadata()?;
        if !meta.is_file() || meta.len() > max_bytes {
            return Err(InstallError::BadSelection(
                "Archive is not a regular file within the size limit".into(),
            ));
        }
        let mut origin = Self {
            file: file.try_clone()?,
            path,
            stamp: archive_stamp(&meta),
            sha256: String::new(),
        };
        origin.verify_current(cancel)?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut remaining = meta.len();
        while remaining != 0 {
            origin.verify_cancel(cancel)?;
            let count = file.read(&mut buffer[..remaining.min(64 * 1024) as usize])?;
            if count == 0 {
                return Err(InstallError::BadSelection(
                    "Archive changed while reading".into(),
                ));
            }
            hash.update(&buffer[..count]);
            remaining -= count as u64;
        }
        origin.verify_current(cancel)?;
        origin.sha256 = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
        Ok(origin)
    }

    fn verify_cancel(&self, cancel: &AtomicBool) -> Result<(), InstallError> {
        if cancel.load(Ordering::Relaxed) {
            Err(InstallError::BadSelection(
                "Archive operation cancelled".into(),
            ))
        } else {
            Ok(())
        }
    }

    /// The child opens the parent's descriptor; CLOEXEC does not affect this path.
    pub(crate) fn pinned_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "/proc/{}/fd/{}",
            std::process::id(),
            self.file.as_raw_fd()
        ))
    }
    pub(crate) fn verify_current(&self, cancel: &AtomicBool) -> Result<(), InstallError> {
        self.verify_cancel(cancel)?;
        let held = self.file.metadata()?;
        let named = fs::symlink_metadata(&self.path)?;
        if !held.is_file()
            || !named.is_file()
            || archive_stamp(&held) != self.stamp
            || archive_stamp(&named) != self.stamp
        {
            return Err(InstallError::BadSelection(
                "Archive changed; reopen before installing or replaying".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

/// Extract every entry of `archive` into `dest`, reporting 7-Zip's own progress.
///
/// A thin adapter over [`eidos_sevenzip`]: the process plumbing (piped stdout
/// read as it arrives, stderr drained on a side thread, the synthetic 100 that
/// closes 7-Zip's 99% gap) lives there, and this maps its two errors onto the
/// installer's own wording. The seam moved out because the workspace already
/// held two divergent copies of it and instance packing was about to add a
/// third.
pub(crate) fn extract_all_with(
    bin: &str,
    archive: &Path,
    dest: &Path,
    on_progress: impl FnMut(u8),
) -> Result<(), InstallError> {
    eidos_sevenzip::extract_all_with(bin, archive, dest, on_progress).map_err(sevenzip_err)
}

/// One place turns a 7-Zip failure into an install failure, so the two error
/// vocabularies meet exactly once.
pub(crate) fn sevenzip_err(e: eidos_sevenzip::SevenZipError) -> InstallError {
    match e {
        eidos_sevenzip::SevenZipError::NotFound => InstallError::No7z,
        eidos_sevenzip::SevenZipError::Failed(why) => InstallError::Extract(why),
    }
}

/// An archive already extracted into a temp directory beside `mods/`, with NTFS
/// case collisions healed. Holding one means the expensive 7-Zip pass is already
/// paid for: [`install_extracted`] installs straight from it, so a simple archive
/// is never extracted twice. The temp is removed when this is dropped.
pub struct ExtractedTree {
    pub(crate) tmp: PathBuf,
    pub(crate) origin: Option<ArchiveOrigin>,
}

impl ExtractedTree {
    pub(crate) fn owned(tmp: PathBuf) -> Self {
        Self { tmp, origin: None }
    }
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
/// Only fires when backslash-bearing names strictly OUTNUMBER the rest, so one
/// oddly-named file in an otherwise normal archive is left exactly as it is: a
/// backslash is legal here, and silently restructuring somebody's mod because of
/// one filename would be worse than the problem.
///
/// A name that is not valid UTF-8 never counts and is never touched, which is
/// not a detail. In every legacy CJK encoding the TRAIL byte of a two-byte
/// character can be 0x5C: CP932 `ソ` is 83 5C, `表` is 95 5C, and Big5 and GBK
/// have the same hazard - the dame-moji problem that has been biting Japanese
/// software since the 1980s. `to_string_lossy` turns the undecodable lead byte
/// into U+FFFD and leaves that 0x5C standing as a literal backslash, so a
/// perfectly ordinary Japanese mod looks like a flattened archive, and rebuilding
/// a destination from that same lossy string RENAMES the file, replacing its
/// bytes with EF BF BD. A `ソード.esp` sitting correctly at a mod's root ends up
/// buried in a garbage directory, and the install still reports success.
///
/// `to_str` is therefore the right tool and `OsStrExt` is the wrong one: the
/// bytes really do contain 0x5C, so splitting on them would stop the mangling
/// and still relocate the plugin into a phantom folder. An undecodable name is
/// simply an ordinary name in an encoding we do not know, and the crate already
/// treats it that way in `resolve_dir_collisions`.
///
/// Returns how many entries were rebuilt.
fn unflatten_backslash_paths(root: &Path) -> io::Result<usize> {
    let entries: Vec<_> = fs::read_dir(root)?.filter_map(Result::ok).collect();
    let (mut flat, mut plain) = (Vec::new(), 0usize);
    for e in entries {
        match e.file_name().to_str().is_some_and(|n| n.contains('\\')) {
            true => flat.push(e),
            false => plain += 1,
        }
    }
    if flat.len() <= plain {
        return Ok(0);
    }

    // Directories last: the flat extraction leaves the archive's directory
    // entries behind as empty placeholders (`Mod\Data\`), and moving the files
    // first means those placeholders are then either redundant or already
    // created by `create_dir_all` below.
    flat.sort_by_key(|e| e.path().is_dir());

    let mut moved = 0;
    for e in flat {
        // `to_str` again, for the same reason as the guard: a destination built
        // from a lossy string would rename the file it is meant to move.
        let Some(raw) = e.file_name().to_str().map(str::to_owned) else {
            continue;
        };
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
            // Remove FIRST, then mirror. A flattened extraction only ever leaves
            // EMPTY placeholders (`Mod\Data\`), so `remove_dir` succeeding is the
            // proof that this is one. Creating the destination first and then
            // swallowing a failed removal is what turns an archive that mixes
            // `\` and `/` into an empty mirror beside stranded real content -
            // and the install reports success, because `ArchiveTree::from_dir`
            // already normalises separators and so believes the files arrived.
            if fs::remove_dir(&src).is_err() {
                continue;
            }
            fs::create_dir_all(&dest)?;
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
    extract_to_temp_with(archive, mods_dir, |_| {})
}

/// [`extract_to_temp`], with 7-Zip's live percentage fed to `on_progress`.
pub fn extract_to_temp_with(
    archive: &Path,
    mods_dir: &Path,
    on_progress: impl FnMut(u8),
) -> Result<ExtractedTree, InstallError> {
    extract_to_temp_cancellable(archive, mods_dir, &AtomicBool::new(false), on_progress)
}

pub(crate) fn extract_to_temp_cancellable(
    archive: &Path,
    mods_dir: &Path,
    cancel: &AtomicBool,
    on_progress: impl FnMut(u8),
) -> Result<ExtractedTree, InstallError> {
    let bin = eidos_sevenzip::find_7z().ok_or(InstallError::No7z)?;
    let origin = ArchiveOrigin::capture(archive, cancel)?;
    let tmp = mods_dir.join(format!(
        ".eidos-install-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&tmp)?;
    let tree = ExtractedTree {
        tmp,
        origin: Some(origin),
    };
    let origin = tree.origin.as_ref().expect("captured archive");
    extract_all_with(bin, &origin.pinned_path(), &tree.tmp, on_progress)?;
    origin.verify_current(cancel)?;
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
    origin.verify_current(cancel)?;
    Ok(tree)
}

#[cfg(test)]
mod backslash_tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "eidos-bs-{}-{tag}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn pinned_archive_reads_original_bytes_and_rejects_path_replacement() {
        let root = ExtractedTree::owned(tmp("origin"));
        let archive = root.path().join("archive.zip");
        fs::write(&archive, b"original").unwrap();
        let cancel = AtomicBool::new(false);
        let origin = ArchiveOrigin::capture(&archive, &cancel).unwrap();
        assert_eq!(
            origin.sha256(),
            "0682c5f2076f099c34cfdd15a9e063849ed437a49677e6fcc5b4198c76575be5"
        );
        fs::rename(&archive, root.path().join("saved.zip")).unwrap();
        fs::write(&archive, b"replacement").unwrap();
        assert_eq!(fs::read(origin.pinned_path()).unwrap(), b"original");
        assert!(origin.verify_current(&cancel).is_err());
        fs::remove_file(&archive).unwrap();
        fs::rename(root.path().join("saved.zip"), &archive).unwrap();
        assert_eq!(fs::read(origin.pinned_path()).unwrap(), b"original");
        // Even an in-place rewrite restored to the original bytes cannot restore ctime.
        fs::write(&archive, b"original").unwrap();
        assert!(origin.verify_current(&cancel).is_err());
        assert!(ArchiveOrigin::capture_bounded(&archive, &cancel, 1).is_err());
        assert!(ArchiveOrigin::capture(&archive, &AtomicBool::new(true)).is_err());
        assert!(ArchiveOrigin::capture(root.path(), &cancel).is_err());
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
        for dir in [
            r"OpenCBP_FO4-3.5.240\Data\",
            r"OpenCBP_FO4-3.5.240\Data\F4SE\",
        ] {
            fs::create_dir(d.join(dir)).unwrap();
        }

        let n = unflatten_backslash_paths(&d).unwrap();
        assert_eq!(n, 6);

        // The two things every later step looks for and could not see before.
        assert!(d
            .join("OpenCBP_FO4-3.5.240/Data/F4SE/Plugins/CBP.dll")
            .is_file());
        assert!(d.join("OpenCBP_FO4-3.5.240/FOMOD/info.xml").is_file());
        assert!(d
            .join("OpenCBP_FO4-3.5.240/Data/Scripts/OCBP_API.pex")
            .is_file());
        assert!(d.join("OpenCBP_FO4-3.5.240/OCBP_LICENSE").is_file());
        // And nothing backslash-named survives at the top.
        assert!(
            !fs::read_dir(&d).unwrap().any(|e| e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains('\\')),
            "a flattened name was left behind"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_legacy_cjk_name_is_never_mistaken_for_a_separator() {
        use std::os::unix::ffi::OsStrExt;
        // The dame-moji problem. CP932 `ソ` is 83 5C and `表` is 95 5C; Big5 and
        // GBK have the same hazard. `to_string_lossy` turns the undecodable lead
        // byte into U+FFFD and leaves the 5C standing as a backslash, so a
        // perfectly ordinary Japanese mod looks flattened - and rebuilding a
        // destination from that lossy string RENAMES the file, replacing its
        // bytes with EF BF BD. A plugin sitting correctly at the mod root ends up
        // buried in a garbage directory, and the install still reports success.
        let d = tmp("cjk");
        let esp = std::ffi::OsStr::from_bytes(b"\x83\x5c\x81\x5b\x83\x68.esp"); // ソード.esp
        let txt = std::ffi::OsStr::from_bytes(b"\x95\x5c.txt"); // 表.txt
        fs::write(d.join(esp), b"plugin").unwrap();
        fs::write(d.join(txt), b"readme").unwrap();
        fs::create_dir(d.join("meshes")).unwrap();

        // Two undecodable names against one ordinary directory: under a lossy
        // reading this is 2 "flat" vs 1 plain and the pass would fire.
        assert_eq!(
            unflatten_backslash_paths(&d).unwrap(),
            0,
            "an unreadable name is not a path"
        );
        assert!(
            d.join(esp).is_file(),
            "the plugin must still be at the mod root, byte for byte"
        );
        assert_eq!(fs::read(d.join(esp)).unwrap(), b"plugin");
        assert!(d.join(txt).is_file());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_backslash_named_directory_that_is_not_empty_is_left_alone() {
        // Only an EMPTY placeholder may be consumed. Creating the destination
        // first and then swallowing a failed removal is what strands real
        // content beside an empty mirror - and the install reports success,
        // because the classifier normalises separators and believes the files
        // arrived.
        let d = tmp("nonempty");
        fs::create_dir(d.join(r"Mod\Data")).unwrap();
        fs::write(d.join(r"Mod\Data").join("real.esp"), b"x").unwrap();
        fs::write(d.join(r"Mod\readme.txt"), b"x").unwrap();

        let n = unflatten_backslash_paths(&d).unwrap();
        assert!(
            d.join(r"Mod\Data").join("real.esp").is_file(),
            "real content must not be stranded"
        );
        assert!(
            !d.join("Mod/Data").is_dir(),
            "no empty mirror may be created beside it"
        );
        assert_eq!(n, 1, "only the plain file is rebuilt");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_tie_does_not_fire() {
        // One odd name against one ordinary entry is not evidence of a
        // flattened archive, and the promise made to users is that a single
        // strange filename is left alone.
        let d = tmp("tie");
        fs::create_dir(d.join("Data")).unwrap();
        fs::write(d.join(r"weird\name.txt"), b"x").unwrap();
        assert_eq!(unflatten_backslash_paths(&d).unwrap(), 0);
        assert!(d.join(r"weird\name.txt").is_file());
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

        assert_eq!(
            unflatten_backslash_paths(&d).unwrap(),
            0,
            "must not fire on a minority"
        );
        assert!(
            d.join(r"weird\name.txt").is_file(),
            "the odd name must survive untouched"
        );
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
        assert!(
            d.join(r"..\..\escaped.txt").is_file(),
            "left in place, not moved"
        );
        assert!(!d
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("escaped.txt")
            .exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn safe_relative_accepts_ordinary_names_and_refuses_the_rest() {
        assert_eq!(
            safe_relative(r"Mod\Data\a.esp"),
            Some(PathBuf::from("Mod/Data/a.esp"))
        );
        // Doubled separators and a trailing one are noise, not structure.
        assert_eq!(
            safe_relative(r"Mod\\Data\"),
            Some(PathBuf::from("Mod/Data"))
        );
        assert_eq!(safe_relative(r"..\x"), None);
        assert_eq!(safe_relative(r"Mod\..\..\x"), None);
        // A forward slash inside a component means the name was never purely
        // backslash-separated; refuse rather than guess which one is structure.
        assert_eq!(safe_relative(r"Mod\a/b.esp"), None);
        assert_eq!(safe_relative(r"\"), None);
    }
}

// The 7-Zip progress tests moved with their subject to `eidos-sevenzip`.
// What stayed here is what is about the INSTALLER: the backslash-path repair
// above, and the error mapping proved below.

#[cfg(test)]
mod sevenzip_seam_tests {
    use super::*;

    #[test]
    fn a_seven_zip_failure_becomes_an_install_failure_with_its_reason() {
        // The two error vocabularies meet in exactly one place; if that
        // mapping ever drops the reason, a user gets a dialog with no cause.
        let mapped = sevenzip_err(eidos_sevenzip::SevenZipError::Failed("bad archive".into()));
        match mapped {
            InstallError::Extract(why) => assert_eq!(why, "bad archive"),
            other => panic!("expected Extract, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_binary_keeps_its_own_variant_and_its_advice() {
        // No7z has its own wording ("install p7zip") and must not be
        // flattened into a generic extraction failure.
        let mapped = sevenzip_err(eidos_sevenzip::SevenZipError::NotFound);
        assert!(matches!(mapped, InstallError::No7z), "{mapped:?}");
    }
}
