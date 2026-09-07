//! One instance, one file: `eidos pack` writes a whole modding setup into a
//! single `.eidos` archive, and `eidos unpack` puts it back on another machine.
//!
//! The problem this solves is a move. A modding instance is tens of thousands of
//! files across `mods/`, `profiles/`, `overwrite/` and `downloads/`, and copying
//! that tree to a new machine by hand loses the parts that are not files: which
//! profile was active, which tools were configured, and the fact that half of
//! those tools are named by an ABSOLUTE path that will not exist there. A
//! `.eidos` file carries the tree AND a manifest describing it, so unpacking can
//! repair what a plain copy cannot.
//!
//! What is deliberately NOT in the archive:
//!
//! * **The Proton prefix.** It lives in Steam's `compatdata`, not in the
//!   instance, it is machine-shaped (drive mappings, a `Z:` view of this
//!   filesystem, DLL overrides pointing at these paths) and Eidos rebuilds it in
//!   a minute with `eidos prereqs --install`. Carrying it would roughly double
//!   the archive to ship something the destination must regenerate anyway.
//! * **The game.** An instance mods a game it does not contain; the destination
//!   installs it from Steam.
//! * **External tools.** xEdit, DynDOLOD and friends usually live outside the
//!   instance. They are NAMED in the manifest - so unpacking can say exactly
//!   which ones are missing here - but not shipped: they are third-party
//!   binaries with their own licences and their own installers.
//!
//! The archive is a plain 7-Zip archive with a `.eidos` name, which is a
//! deliberate choice rather than a disguise: 7-Zip is already a hard requirement
//! for Eidos (no mod installs without it), so packing adds no dependency, and
//! anybody who ever loses this tool can still open their own backup with the
//! archiver they already have. It is NON-SOLID (`-ms=off`) so a single file can
//! be pulled out of a 70 GB backup without decompressing everything before it.

use std::fmt;
use std::path::{Path, PathBuf};

mod manifest;
mod plan;
mod relocate;
mod transfer;

pub use manifest::BackupManifest;
pub use plan::{plan, Left, Outside, Plan, Why};
pub use relocate::{relocate, Relocated};
pub use transfer::{PackReport, Transfer, UnpackReport};

/// The extension a backup gets. Not `.7z`: the point of a name of our own is
/// that a double-click can mean something, and that a user scanning a folder of
/// archives can tell which one is a whole instance.
pub const EXTENSION: &str = "eidos";

/// The manifest inside the archive, at its root.
pub const MANIFEST_NAME: &str = "eidos-backup.ini";

/// The on-disk format version of the ARCHIVE (not of the instance inside it,
/// which carries its own `schema_version` in `eidos-instance.ini`). Bump it when
/// an older Eidos could no longer put a backup back correctly.
pub const SCHEMA_VERSION: u32 = 1;

/// The compression levels 7-Zip accepts for `-mx`.
pub const LEVELS: [u8; 6] = [0, 1, 3, 5, 7, 9];

/// The default `-mx` level, and the reason it is 1 rather than 9.
///
/// Measured on 1199 MB of real Skyrim `.dds`/`.nif` content on a 16-thread
/// machine: LZMA2 at `-mx1` keeps 43.0% of the volume, and at `-mx9` it keeps
/// about 38% for something like ten times the time. On a 72 GB instance that is
/// the difference between twenty minutes and most of a day, to save a few GB on
/// a file that exists to be written once and read once. Textures and meshes are
/// already compressed formats; there is very little for a bigger dictionary to
/// find. `--level` is there for anyone who disagrees on their own corpus.
pub const DEFAULT_LEVEL: u8 = 1;

/// What packing or unpacking can refuse to do, in the user's terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferError {
    /// No 7-Zip on this machine.
    NoSevenZip,
    /// A precondition the user can fix: the destination exists, the archive is
    /// not a backup, the disk is too small. The string is a full sentence.
    Refused(String),
    /// The filesystem said no.
    Io(String),
    /// 7-Zip said no.
    Archive(String),
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransferError::NoSevenZip => write!(
                f,
                "7-Zip is not installed (looked for 7z, 7zz and 7za on PATH). \
                 Install the `7zip` package - Eidos needs it to install mods too."
            ),
            TransferError::Refused(why) | TransferError::Io(why) => write!(f, "{why}"),
            TransferError::Archive(why) => write!(f, "7-Zip failed: {why}"),
        }
    }
}

impl std::error::Error for TransferError {}

impl From<eidos_sevenzip::SevenZipError> for TransferError {
    fn from(e: eidos_sevenzip::SevenZipError) -> TransferError {
        match e {
            eidos_sevenzip::SevenZipError::NotFound => TransferError::NoSevenZip,
            eidos_sevenzip::SevenZipError::Failed(why) => TransferError::Archive(why),
        }
    }
}

/// How to pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Include `downloads/` - the archives mods were installed from.
    ///
    /// On by default because they are part of the instance and MO2 users expect
    /// to keep them (they carry the Nexus ids a re-download needs). Off is for
    /// somebody moving 40 GB of downloadable archives they would rather fetch
    /// again than carry.
    pub downloads: bool,
    /// The `-mx` level; see [`DEFAULT_LEVEL`].
    pub level: u8,
    /// Overwrite an existing destination.
    pub force: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            downloads: true,
            level: DEFAULT_LEVEL,
            force: false,
        }
    }
}

/// Free bytes on the filesystem holding `dir`, or `None` if it cannot be asked.
///
/// A pack of a 72 GB instance runs for twenty minutes; finding out at minute
/// nineteen that the target disk holds 4 GB is the failure this exists to
/// prevent. `None` is not an error - an unusual filesystem that refuses
/// `statvfs` is a reason to proceed unwarned, not a reason to refuse.
// statvfs fields are c_ulong (u64 on 64-bit, u32 on 32-bit); the `as u64` casts
// are no-ops here but keep the conversion correct on 32-bit targets.
#[allow(clippy::unnecessary_cast)]
pub fn free_bytes(dir: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(dir.as_os_str().as_bytes()).ok()?;
    // SAFETY: valid C path and a zeroed statvfs out-param; we check the return.
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    Some((st.f_bavail as u64).saturating_mul(st.f_frsize as u64))
}

/// A byte count as a person reads it: `72.2 GB`, `888 MB`, `908 B`.
pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1000.0 && u + 1 < UNITS.len() {
        v /= 1000.0;
        u += 1;
    }
    if u == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

/// The nearest existing ancestor of `path` - what to ask about free space when
/// the destination file, and possibly its folder, do not exist yet.
pub fn existing_ancestor(path: &Path) -> PathBuf {
    let mut p = path;
    loop {
        if p.is_dir() {
            return p.to_path_buf();
        }
        match p.parent() {
            // The parent of a bare `backup.eidos` is the EMPTY path, which is
            // not a directory and whose own parent is nothing - so walking on
            // would answer `/` for a relative name that means "here".
            Some(parent) if parent.as_os_str().is_empty() => return PathBuf::from("."),
            Some(parent) => p = parent,
            None => return PathBuf::from("/"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_read_the_way_a_person_says_them() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(908), "908 B");
        assert_eq!(human_bytes(1_000), "1.0 kB");
        assert_eq!(human_bytes(888_000_000), "888.0 MB");
        assert_eq!(human_bytes(77_538_508_832), "77.5 GB");
    }

    #[test]
    fn the_missing_binary_names_the_package_to_install() {
        // The message is the whole value of this variant: "7-Zip not found" sends
        // a user to a download page for a Windows installer.
        let m = TransferError::NoSevenZip.to_string();
        assert!(m.contains("7zip"), "{m}");
        assert!(m.contains("7zz"), "{m}");
    }

    #[test]
    fn a_relative_destination_asks_about_here_not_about_the_root() {
        assert_eq!(
            existing_ancestor(Path::new("backup.eidos")),
            PathBuf::from(".")
        );
        assert_eq!(
            existing_ancestor(Path::new("no-such-dir/backup.eidos")),
            PathBuf::from(".")
        );
    }

    #[test]
    fn free_space_is_asked_of_a_folder_that_exists() {
        let deep = std::env::temp_dir().join("eidos-transfer-no-such/a/b/c.eidos");
        let anc = existing_ancestor(&deep);
        assert!(anc.is_dir(), "{}", anc.display());
        assert!(free_bytes(&anc).is_some(), "temp dir should answer statvfs");
    }
}
