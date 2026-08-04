//! A profile: one named set of enabled mods + their order (and, later, its own
//! `plugins.txt`, INIs and saves), all sharing the instance's single `mods/`
//! pool. This is what lets one mod collection serve several playthroughs.
//!
//! Mirrors Mod Organizer 2: a profile is just a directory under
//! `<instance>/profiles/<name>/`; its `modlist.txt` carries both the enabled set
//! and the priority order, while the mods themselves stay global.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};


mod inis;
mod modlist;
mod plugins;
mod saves;
#[cfg(test)]
mod tests;

pub use inis::*;
pub use modlist::*;
use plugins::*;
pub use saves::*;

/// One profile of an instance.
#[derive(Debug, Clone)]
pub struct Profile {
    pub instance_root: PathBuf,
    pub name: String,
}

/// One save file in a profile's `saves/` directory (MO2's savegame list).
#[derive(Debug, Clone)]
pub struct SaveEntry {
    /// The file's name (e.g. `Save1_quicksave.ess`).
    pub filename: String,
    /// The absolute path on disk.
    pub path: PathBuf,
    /// Size in bytes.
    pub size: u64,
    /// Last-modified time (used as the in-game date proxy).
    pub mtime: std::time::SystemTime,
}

impl Profile {
    /// `<instance>/profiles/<name>/`.
    pub fn dir(&self) -> PathBuf {
        self.instance_root.join("profiles").join(&self.name)
    }

    /// The shared mod pool (instance-wide, not per-profile).
    fn mods_dir(&self) -> PathBuf {
        self.instance_root.join("mods")
    }

    fn modlist_path(&self) -> PathBuf {
        self.dir().join("modlist.txt")
    }

}

/// Copy `src` to `dst` atomically: write a sibling `.tmp`, then rename over. A
/// plain `fs::copy` truncates the destination first, so a reader (or a crash)
/// mid-copy sees a torn file - and for the files this module moves around, a
/// torn `plugins.txt` is a wiped load order and a torn INI is a lost config.
pub(crate) fn copy_atomic(src: &Path, dst: &Path) -> io::Result<()> {
    let tmp = dst.with_extension("eidos-tmp");
    fs::copy(src, &tmp)?;
    match fs::rename(&tmp, dst) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Write `bytes` to `dst` through a temp file and a rename, so a reader never
/// sees a half-written file and a failure leaves the previous contents intact.
pub(crate) fn copy_atomic_bytes(dst: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = dst.with_extension("eidos-tmp");
    fs::write(&tmp, bytes)?;
    match fs::rename(&tmp, dst) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}
