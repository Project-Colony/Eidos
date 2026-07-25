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

mod native_dll;
pub use native_dll::{
    community_shaders_in_roots, enb_cs_conflict, enb_in_game_root, ensure_d3dcompiler_47,
    ensure_native_dll, imported_dlls, is_tier1_dll, scan_imports_provisionable, NativeDllError,
};

mod prereqs;
pub use prereqs::{
    cabextract_available, find_winetricks, install_tier2_verb, is_tier2_verb, prefix_busy,
};

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

/// Enable archive (BSA) invalidation so loose mod files override the vanilla BSAs.
/// `ini_dir` is the prefix's `Documents/My Games/<game>` directory.
///
/// FO4/FO4VR write into `Fallout4Custom.ini` and Starfield into `StarfieldCustom.ini`
/// (both override the base INI, which is where MO2 puts it); every other engine uses
/// its `[Archive]` INI. FO4/FO4VR/Starfield additionally need `sResourceDataDirsFinal=`
/// cleared - they ship it as `STRINGS\`, so the engine only scans `Data/Strings` for
/// loose files and ignores loose textures/meshes/scripts until it is emptied.
pub fn enable_bsa_invalidation(ini_dir: &Path, data_dir: &Path, game_id: &str) -> io::Result<()> {
    // Morrowind uses the numbered [Archives] list, not [Archive] bInvalidateOlderFiles
    // (see register_morrowind_archives); nothing to do in this path.
    if game_id == "morrowind" {
        return Ok(());
    }
    let target = match game_id {
        "fallout4" | "fallout4vr" => "Fallout4Custom.ini",
        "starfield" => "StarfieldCustom.ini",
        other => match ini_file_for(other) {
            Some(f) => f,
            None => return Ok(()),
        },
    };
    let path = ini_dir.join(target);
    set_ini_key(&path, "Archive", "bInvalidateOlderFiles", "1")?;
    if matches!(game_id, "fallout4" | "fallout4vr" | "starfield") {
        set_ini_key(&path, "Archive", "sResourceDataDirsFinal", "")?;
    }

    // Pre-SSE engines (Oblivion/FO3/FNV/Skyrim LE): bInvalidateOlderFiles is
    // timestamp-relative, so a loose file only wins if newer than the owning BSA -
    // mod files with preserved archive mtimes routinely lose. MO2 fixes this with
    // the "ArchiveInvalidation Invalidated" dummy BSA: a minimal archive registered
    // at the front of the [Archive] list, plus SInvalidationFile cleared so the
    // legacy ArchiveInvalidation.txt mechanism does not interfere.
    if let Some(inv) = pre_sse_invalidation(game_id) {
        // The dummy BSA lives in the Data tree; write it into the writable overwrite
        // layer (`data_dir`) so the real game install is never touched.
        let bsa = data_dir.join(inv.bsa_name);
        if !bsa.exists() {
            fs::create_dir_all(data_dir)?;
            fs::write(&bsa, dummy_bsa_bytes(inv.bsa_version))?;
        }
        prepend_archive(&path, inv.archive_key, inv.bsa_name)?;
        set_ini_key(&path, "Archive", "SInvalidationFile", "")?;
    }
    Ok(())
}

/// Per-game parameters for the pre-SSE dummy-BSA invalidation; `None` for the
/// Creation engines (SSE/FO4/Starfield), which use bInvalidateOlderFiles alone.
struct PreSse {
    bsa_name: &'static str,
    bsa_version: u32,
    archive_key: &'static str,
}

fn pre_sse_invalidation(game_id: &str) -> Option<PreSse> {
    match game_id {
        "oblivion" => {
            Some(PreSse { bsa_name: "Oblivion - Invalidation.bsa", bsa_version: 0x67, archive_key: "SArchiveList" })
        }
        "fallout3" | "falloutnv" => {
            Some(PreSse { bsa_name: "Fallout - Invalidation.bsa", bsa_version: 0x68, archive_key: "SArchiveList" })
        }
        "skyrim" => {
            Some(PreSse { bsa_name: "Skyrim - Invalidation.bsa", bsa_version: 0x68, archive_key: "sResourceArchiveList" })
        }
        _ => None,
    }
}

/// Prepend `bsa` to a comma-joined `[Archive]` list key (e.g. `SArchiveList`),
/// keeping the vanilla archives and skipping if it is already registered.
fn prepend_archive(ini: &Path, key: &str, bsa: &str) -> io::Result<()> {
    let existing = fs::read_to_string(ini).unwrap_or_default();
    let current = get_ini_key(&existing, "Archive", key).unwrap_or_default();
    if current.split(',').any(|a| a.trim().eq_ignore_ascii_case(bsa)) {
        return Ok(());
    }
    let value = if current.trim().is_empty() {
        bsa.to_string()
    } else {
        format!("{bsa}, {}", current.trim())
    };
    set_ini_key(ini, "Archive", key, &value)
}

/// Register Morrowind mod BSAs in the numbered `[Archives]` list: keep the existing
/// (vanilla) entries and append any enabled-mod `.bsa` not already present, then
/// rewrite `Archive 0..N`. Morrowind only loads a BSA that is listed here, so a
/// BSA-shipping mod is otherwise silently ignored.
pub fn register_morrowind_archives(ini: &Path, mod_bsas: &[String]) -> io::Result<()> {
    let mut text = fs::read_to_string(ini).unwrap_or_default();
    let mut archives = read_numbered_archives(&text);
    for b in mod_bsas {
        if !archives.iter().any(|a| a.eq_ignore_ascii_case(b)) {
            archives.push(b.clone());
        }
    }
    for (i, a) in archives.iter().enumerate() {
        text = eidos_ini::set_key(&text, "Archives", &format!("Archive {i}"), a);
    }
    if let Some(parent) = ini.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(ini, text)
}

/// The ordered `[Archives] Archive N=` values from INI text.
fn read_numbered_archives(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section = false;
    for line in text.lines() {
        let l = line.trim();
        if let Some(s) = eidos_ini::section_header(l) {
            in_section = s.eq_ignore_ascii_case("Archives");
            continue;
        }
        if in_section {
            if let Some((k, v)) = eidos_ini::key_value(l) {
                if k.to_ascii_lowercase().starts_with("archive ") && !v.trim().is_empty() {
                    out.push(v.trim().to_string());
                }
            }
        }
    }
    out
}

/// Read a `[section] key` value from INI text (the shared parser has a setter but
/// no getter); section and key match case-insensitively.
fn get_ini_key(text: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in text.lines() {
        let l = line.trim();
        if let Some(s) = eidos_ini::section_header(l) {
            in_section = s.eq_ignore_ascii_case(section);
            continue;
        }
        if in_section {
            if let Some((k, v)) = eidos_ini::key_value(l) {
                if k.eq_ignore_ascii_case(key) {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

/// MO2's "dummy" invalidation BSA (port of `dummybsa.cpp`): a minimal valid archive
/// with one empty folder and one zero-byte `dummy.dds`, used purely to be registered
/// in the archive list so the timestamp invalidation engages. `version` is 0x67
/// (Oblivion) or 0x68 (FO3/FNV/Skyrim LE). All multi-byte fields are little-endian.
fn dummy_bsa_bytes(version: u32) -> Vec<u8> {
    const FILE_NAME: &str = "dummy.dds";
    let total_file_name_len = (FILE_NAME.len() + 1) as u32; // 10

    let mut out: Vec<u8> = Vec::with_capacity(83);
    // Header (36 bytes).
    out.extend_from_slice(b"BSA\0");
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&0x24u32.to_le_bytes()); // offset to folder records (= header size)
    out.extend_from_slice(&(0x01u32 | 0x02).to_le_bytes()); // flags: has dirs + has files
    out.extend_from_slice(&1u32.to_le_bytes()); // folder count
    out.extend_from_slice(&1u32.to_le_bytes()); // file count
    out.extend_from_slice(&1u32.to_le_bytes()); // total folder-names length (empty + null)
    out.extend_from_slice(&total_file_name_len.to_le_bytes()); // total file-names length
    out.extend_from_slice(&2u32.to_le_bytes()); // file flags: has dds

    // Folder record (16 bytes): hash of "" (= 0), file count, offset to folder name.
    out.extend_from_slice(&gen_hash("").to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(0x34u32 + total_file_name_len).to_le_bytes());

    // File-record block: the folder name ("" + null) then the file record (16 bytes).
    out.push(0);
    out.extend_from_slice(&gen_hash(FILE_NAME).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // size
    out.extend_from_slice(&(0x44u32 + total_file_name_len + 4).to_le_bytes()); // offset to data

    // The file name + null, then 4 bytes of (zero) file size.
    out.extend_from_slice(FILE_NAME.as_bytes());
    out.push(0);
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

fn gen_hash_int(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0;
    for &b in bytes {
        hash = hash.wrapping_mul(0x1003f).wrapping_add(b as u32);
    }
    hash
}

/// Gamebryo BSA filename hash (port of `dummybsa.cpp::genHash`).
fn gen_hash(file_name: &str) -> u64 {
    let lower: Vec<u8> = file_name
        .bytes()
        .map(|b| {
            let c = b.to_ascii_lowercase();
            if c == b'\\' { b'/' } else { c }
        })
        .collect();
    let ext_pos = lower.iter().rposition(|&b| b == b'.').unwrap_or(lower.len());
    let length = ext_pos;
    let ext = &lower[ext_pos..];

    let mut hash: u64 = 0;
    if length > 0 {
        let last_before = lower[ext_pos - 1] as u64;
        let two_before = if length > 2 { lower[ext_pos - 2] as u64 } else { 0 };
        hash = last_before | (two_before << 8) | ((length as u64) << 16) | ((lower[0] as u64) << 24);
    }
    if !ext.is_empty() {
        match &ext[1..] {
            b"kf" => hash |= 0x80,
            b"nif" => hash |= 0x8000,
            b"dds" => hash |= 0x8080,
            b"wav" => hash |= 0x8000_0000,
            _ => {}
        }
        let part1_end = ext_pos.saturating_sub(2);
        let part1: &[u8] = if part1_end > 1 { &lower[1..part1_end] } else { &[] };
        let temp = (gen_hash_int(part1) as u64).wrapping_add(gen_hash_int(ext) as u64);
        hash |= (temp & 0xFFFF_FFFF) << 32;
    }
    hash
}

/// MO2 writes `[Launcher] bEnableFileSelection=1` before every run so the Bethesda
/// launcher/engine does not grey out (or reset) the plugin selection - it enforces
/// this for every Gamebryo/Creation game. Written into the `[Archive]` INI.
pub fn enable_file_selection(ini_dir: &Path, ini_file: &str) -> io::Result<()> {
    set_ini_key(&ini_dir.join(ini_file), "Launcher", "bEnableFileSelection", "1")
}

/// Set `[section] key=value` in an INI file on disk, preserving everything else
/// (and the file's CRLF/LF style). A thin file-I/O wrapper over the shared,
/// format-preserving [`eidos_ini::set_key`]; section and key match
/// case-insensitively.
pub fn set_ini_key(path: &Path, section: &str, key: &str, value: &str) -> io::Result<()> {
    // A missing file is created; an EXISTING file must never be truncated because
    // it cannot be read. Bethesda INIs localized in Windows-1252 (accented FR/DE
    // text) are not valid UTF-8: decode those as Latin-1 - a byte-for-byte
    // reversible mapping - edit, and encode back, preserving every original byte.
    let (existing, latin1) = match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => (s, false),
            Err(e) => {
                let s: String = e.into_bytes().iter().map(|&b| b as char).collect();
                (s, true)
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => (String::new(), false),
        Err(e) => return Err(e),
    };
    let out = eidos_ini::set_key(&existing, section, key, value);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if latin1 {
        // Chars above U+00FF cannot occur: the decoded input round-trips and the
        // inserted section/key/value text is ASCII. Guard anyway.
        let bytes: Vec<u8> =
            out.chars().map(|c| if (c as u32) <= 0xFF { c as u8 } else { b'?' }).collect();
        fs::write(path, bytes)
    } else {
        fs::write(path, out)
    }
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
    fn non_utf8_ini_is_edited_without_truncation() {
        // Windows-1252 INI ("Général" with an 0xE9 é): the update must preserve
        // every original byte instead of wiping the file as unreadable.
        let p = tmp();
        let original = b"[G\xE9n\xE9ral]\nsLanguage=FRENCH\n".to_vec();
        fs::write(&p, &original).unwrap();
        set_ini_key(&p, "Launcher", "bEnableFileSelection", "1").unwrap();
        let after = fs::read(&p).unwrap();
        let after_str: String = after.iter().map(|&b| b as char).collect();
        assert!(after.windows(original.len() - 1).any(|w| w == &original[..original.len() - 1]),
            "original bytes (incl. 0xE9) must be preserved: {after_str:?}");
        assert!(after_str.contains("[Launcher]") && after_str.contains("bEnableFileSelection=1"));
        let _ = fs::remove_file(&p);
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

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir()
            .join(format!("eidos-gfd-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn fallout4_invalidation_targets_custom_ini_with_both_keys() {
        let dir = tmp_dir();
        enable_bsa_invalidation(&dir, &dir, "fallout4").unwrap();
        let s = fs::read_to_string(dir.join("Fallout4Custom.ini")).unwrap();
        assert!(s.contains("bInvalidateOlderFiles=1"));
        assert!(s.contains("sResourceDataDirsFinal=")); // cleared so loose files load
        assert!(!dir.join("Fallout4.ini").exists()); // not the base INI
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn skyrim_invalidation_only_sets_invalidate_key() {
        let dir = tmp_dir();
        enable_bsa_invalidation(&dir, &dir, "skyrimse").unwrap();
        let s = fs::read_to_string(dir.join("Skyrim.ini")).unwrap();
        assert!(s.contains("bInvalidateOlderFiles=1"));
        assert!(!s.contains("sResourceDataDirsFinal")); // Creation/Gamebryo don't use it
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn morrowind_registers_mod_bsas_in_archives_list() {
        let dir = tmp_dir();
        let ini = dir.join("Morrowind.ini");
        // The existing (vanilla) numbered list.
        fs::write(&ini, "[Archives]\r\nArchive 0=Morrowind.bsa\r\nArchive 1=Tribunal.bsa\r\n").unwrap();
        register_morrowind_archives(&ini, &["ModX.bsa".to_string(), "Morrowind.bsa".to_string()]).unwrap();
        let s = fs::read_to_string(&ini).unwrap();
        assert!(s.contains("Archive 0=Morrowind.bsa"));
        assert!(s.contains("Archive 1=Tribunal.bsa"));
        assert!(s.contains("Archive 2=ModX.bsa")); // appended
        assert_eq!(s.matches("=Morrowind.bsa").count(), 1); // not duplicated
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dummy_bsa_has_a_valid_header() {
        let b = dummy_bsa_bytes(0x67);
        assert_eq!(b.len(), 83);
        assert_eq!(&b[0..4], b"BSA\0");
        assert_eq!(u32::from_le_bytes(b[4..8].try_into().unwrap()), 0x67); // version
        assert_eq!(u32::from_le_bytes(b[8..12].try_into().unwrap()), 0x24); // header size
        assert_eq!(u32::from_le_bytes(b[16..20].try_into().unwrap()), 1); // folder count
        assert_eq!(u32::from_le_bytes(b[20..24].try_into().unwrap()), 1); // file count
        assert!(b.ends_with(b"dummy.dds\0\0\0\0\0")); // name + null + 4-byte size
    }

    #[test]
    fn oblivion_invalidation_writes_dummy_bsa_and_registers_it() {
        let docs = tmp_dir();
        let data = tmp_dir();
        // The deployed INI already lists the vanilla archives.
        fs::write(docs.join("Oblivion.ini"), "[Archive]\r\nSArchiveList=Oblivion - Meshes.bsa\r\n").unwrap();
        enable_bsa_invalidation(&docs, &data, "oblivion").unwrap();

        // The dummy BSA is written into the overwrite (Data) layer, not the game dir.
        assert!(data.join("Oblivion - Invalidation.bsa").is_file());
        let ini = fs::read_to_string(docs.join("Oblivion.ini")).unwrap();
        assert!(ini.contains("bInvalidateOlderFiles=1"));
        assert!(ini.contains("Oblivion - Invalidation.bsa")); // registered at the front
        assert!(ini.contains("Oblivion - Meshes.bsa")); // vanilla list kept
        assert!(ini.contains("SInvalidationFile=")); // legacy mechanism disabled
        // Idempotent: a second run doesn't double-register.
        enable_bsa_invalidation(&docs, &data, "oblivion").unwrap();
        let ini2 = fs::read_to_string(docs.join("Oblivion.ini")).unwrap();
        assert_eq!(ini2.matches("Oblivion - Invalidation.bsa").count(), 1);

        let _ = fs::remove_dir_all(&docs);
        let _ = fs::remove_dir_all(&data);
    }

    #[test]
    fn file_selection_unlocks_the_launcher() {
        let dir = tmp_dir();
        enable_file_selection(&dir, "Skyrim.ini").unwrap();
        let s = fs::read_to_string(dir.join("Skyrim.ini")).unwrap();
        assert!(s.contains("[Launcher]"));
        assert!(s.contains("bEnableFileSelection=1"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ini_file_mapping() {
        assert_eq!(ini_file_for("skyrimse"), Some("Skyrim.ini"));
        assert_eq!(ini_file_for("fallout4"), Some("Fallout4.ini"));
        assert_eq!(ini_file_for("nope"), None);
    }

    #[test]
    fn ini_files_set() {
        assert_eq!(
            ini_files_for("skyrimse"),
            ["Skyrim.ini", "SkyrimPrefs.ini", "SkyrimCustom.ini"].as_slice()
        );
        assert!(ini_files_for("nope").is_empty());
        // The [Archive] INI is the first of the per-profile set.
        assert_eq!(ini_files_for("fallout4").first().copied(), ini_file_for("fallout4"));
    }
}
