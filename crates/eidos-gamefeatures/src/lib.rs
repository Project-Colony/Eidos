//! Per-game Bethesda features that Eidos must apply for mods to work, ported from
//! Mod Organizer 2's game plugins. These are Linux/Proton-native operations run
//! before the per-launch mount, writing into the game's Proton-prefix `Documents`.
//!
//! First feature: **archive (BSA) invalidation**. Bethesda engines prefer files
//! packed in the vanilla BSAs over loose files unless invalidation is on, so a mod
//! that ships loose overrides is silently ignored without it. MO2's
//! `GamebryoBSAInvalidation` writes `[Archive] bInvalidateOlderFiles=1` into the
//! game INI (plus a dummy BSA + `SInvalidationFile` dance for pre-SSE engines).

use std::fs;
use std::io;
use std::path::Path;

/// The game INI that holds the `[Archive]` section: the first of the per-profile
/// INIs (see `ini_files_for`).
pub fn ini_file_for(game_id: &str) -> Option<&'static str> {
    ini_files_for(game_id).first().copied()
}

/// The per-profile user INIs for a game (the files MO2 keeps per profile), read
/// from the single `eidos-gamedef` descriptor. The first entry carries the
/// `[Archive]` section.
pub fn ini_files_for(game_id: &str) -> &'static [&'static str] {
    eidos_gamedef::GameDef::for_id(game_id).map_or(&[], |g| g.ini_files)
}

/// Enable archive (BSA) invalidation so loose mod files override the vanilla BSAs:
/// `[Archive] bInvalidateOlderFiles=1` in the game's Documents INI. `ini_dir` is the
/// prefix's `Documents/My Games/<game>` directory.
pub fn enable_bsa_invalidation(ini_dir: &Path, ini_file: &str) -> io::Result<()> {
    set_ini_key(&ini_dir.join(ini_file), "Archive", "bInvalidateOlderFiles", "1")
}

/// Set `[section] key=value` in an INI file on disk, preserving everything else
/// (and the file's CRLF/LF style). A thin file-I/O wrapper over the shared,
/// format-preserving [`eidos_ini::set_key`]; section and key match
/// case-insensitively.
pub fn set_ini_key(path: &Path, section: &str, key: &str, value: &str) -> io::Result<()> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    let out = eidos_ini::set_key(&existing, section, key, value);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);
    fn tmp() -> PathBuf {
        std::env::temp_dir()
            .join(format!("eidos-gf-{}-{}.ini", std::process::id(), N.fetch_add(1, Ordering::Relaxed)))
    }

    #[test]
    fn creates_section_and_key_in_a_missing_file() {
        let p = tmp();
        set_ini_key(&p, "Archive", "bInvalidateOlderFiles", "1").unwrap();
        let s = fs::read_to_string(&p).unwrap();
        assert!(s.contains("[Archive]"));
        assert!(s.contains("bInvalidateOlderFiles=1"));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn updates_existing_key_preserving_everything_else() {
        let p = tmp();
        fs::write(
            &p,
            "[Display]\r\niSize W=1920\r\n\r\n[Archive]\r\nbInvalidateOlderFiles=0\r\nsResourceArchiveList=x.bsa\r\n",
        )
        .unwrap();
        set_ini_key(&p, "Archive", "bInvalidateOlderFiles", "1").unwrap();
        let s = fs::read_to_string(&p).unwrap();
        assert!(s.contains("bInvalidateOlderFiles=1"));
        assert!(!s.contains("bInvalidateOlderFiles=0"));
        assert!(s.contains("iSize W=1920")); // other section preserved
        assert!(s.contains("sResourceArchiveList=x.bsa")); // sibling key preserved
        assert!(s.contains("\r\n")); // CRLF preserved
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn adds_key_to_an_existing_section() {
        let p = tmp();
        fs::write(&p, "[Archive]\nsResourceArchiveList=x.bsa\n").unwrap();
        set_ini_key(&p, "Archive", "bInvalidateOlderFiles", "1").unwrap();
        let s = fs::read_to_string(&p).unwrap();
        assert!(s.contains("bInvalidateOlderFiles=1"));
        assert!(s.contains("sResourceArchiveList=x.bsa"));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn ini_file_mapping() {
        assert_eq!(ini_file_for("skyrimse"), Some("Skyrim.ini"));
        assert_eq!(ini_file_for("fallout4"), Some("Fallout4.ini"));
        assert_eq!(ini_file_for("nope"), None);
    }

    #[test]
    fn ini_files_set() {
        assert_eq!(ini_files_for("skyrimse"), ["Skyrim.ini", "SkyrimPrefs.ini"].as_slice());
        assert!(ini_files_for("nope").is_empty());
        // The [Archive] INI is the first of the per-profile set.
        assert_eq!(ini_files_for("fallout4").first().copied(), ini_file_for("fallout4"));
    }
}
