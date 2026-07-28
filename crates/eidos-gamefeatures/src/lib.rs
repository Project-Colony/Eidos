//! Per-game Bethesda features that Eidos must apply for mods to work, ported from
//! Mod Organizer 2's game plugins. These are Linux/Proton-native operations run
//! before the per-launch mount, writing into the game's Proton-prefix `Documents`.
//!
//! First feature: **archive (BSA) invalidation**. Bethesda engines prefer files
//! packed in the vanilla BSAs over loose files unless invalidation is on, so a mod
//! that ships loose overrides is silently ignored without it. MO2's
//! `GamebryoBSAInvalidation` writes `[Archive] bInvalidateOlderFiles=1` into the
//! game INI (plus a dummy BSA + `SInvalidationFile` dance for pre-SSE engines).
//!
//! The archive side is also where the ORPHAN diagnostic lives ([`orphan_archives`]):
//! a `.bsa`/`.ba2` is only loaded if an active plugin's base name owns it or the INI
//! registers it, so an archive matching neither is dead weight - the mod looks
//! installed and contributes nothing. That check needs file names and the plugin
//! list only. Eidos deliberately does NOT parse archive contents: five format
//! generations of reader would buy nothing, since the game opens its own archives
//! and the FUSE union never has to serve files from inside one.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod native_dll;
pub use native_dll::{
    community_shaders_in_roots, enb_cs_conflict, enb_in_game_root, ensure_d3dcompiler_47,
    ensure_native_dll, imported_dlls, is_tier1_dll, scan_imports_provisionable, NativeDllError,
};

mod prefix_registry;
pub use prefix_registry::{ensure_registry, registry_blob};

mod se_log;
pub use se_log::{parse_se_log, se_log_path, SePluginLoad};

mod prereqs;
pub use prereqs::{
    cabextract_available, find_winetricks, install_tier2_verb, is_tier2_verb, prefix_busy,
};

mod savegame;
pub use savegame::{
    missing_plugins, parse_sse_save, KnownPlugin, MissingPlugin, ModFolder, SaveCompression,
    SaveInfo, SaveParseError, SavePluginState,
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

/// The Bethesda engines truncate an INI value at 255 characters. That is why the
/// Creation games ship their archive list split across `sResourceArchiveList` and
/// `sResourceArchiveList2` (MO2 splits at the last comma before 256 in
/// `skyrimsedataarchives.cpp::writeArchiveList`).
const INI_VALUE_MAX: usize = 255;

/// The `...List2` continuation of an archive-list key, when the engine has one.
/// Only the Creation-engine key is split in two; the older `SArchiveList`
/// (Oblivion/FO3/FNV) has no continuation, so an overflow there cannot be helped.
fn continuation_key(key: &str) -> Option<&'static str> {
    key.eq_ignore_ascii_case("sResourceArchiveList").then_some("sResourceArchiveList2")
}

/// Prepend `bsa` to a comma-joined `[Archive]` list key (e.g. `SArchiveList`),
/// keeping the vanilla archives and skipping if it is already registered.
fn prepend_archive(ini: &Path, key: &str, bsa: &str) -> io::Result<()> {
    let (existing, _) = read_ini_text(ini)?;
    let listed = |v: Option<&str>| {
        v.unwrap_or_default().split(',').any(|a| a.trim().eq_ignore_ascii_case(bsa))
    };
    let current = eidos_ini::get_key(&existing, "Archive", key).unwrap_or_default();
    let cont = continuation_key(key);
    let cont_value = cont.and_then(|c| eidos_ini::get_key(&existing, "Archive", c));
    // Check the continuation too: a previous run may have pushed the dummy past the
    // 255-char cut into List2, and re-prepending it would register it twice.
    if listed(Some(current)) || listed(cont_value) {
        return Ok(());
    }
    let value = if current.trim().is_empty() {
        bsa.to_string()
    } else {
        format!("{bsa}, {}", current.trim())
    };

    // Overflow moves to the FRONT of the continuation key: the engine reads the
    // first list then the second, so the relative order - and with it archive
    // precedence - survives the split.
    if let (Some(c), true) = (cont, value.len() > INI_VALUE_MAX) {
        let (head, tail) = split_archive_value(&value);
        if !tail.is_empty() {
            let rest = cont_value.unwrap_or_default().trim();
            let merged = if rest.is_empty() { tail.to_string() } else { format!("{tail}, {rest}") };
            set_ini_key(ini, "Archive", key, head)?;
            return set_ini_key(ini, "Archive", c, &merged);
        }
    }
    set_ini_key(ini, "Archive", key, &value)
}

/// Split a comma-joined archive list at the last comma that still fits in
/// [`INI_VALUE_MAX`], returning `(head, tail)`. A single entry longer than the
/// limit has no usable split point and is returned whole (the engine will truncate
/// it, but silently dropping it here would be worse).
fn split_archive_value(value: &str) -> (&str, &str) {
    // A comma AT index INI_VALUE_MAX still leaves a head of exactly 255 chars, so
    // the search window is one past the limit. Commas are ASCII, so a byte position
    // is always a char boundary here.
    let limit = value.len().min(INI_VALUE_MAX + 1);
    match value.as_bytes()[..limit].iter().rposition(|&b| b == b',') {
        Some(i) => (value[..i].trim_end(), value[i + 1..].trim_start()),
        None => (value, ""),
    }
}

/// Register Morrowind mod BSAs in the numbered `[Archives]` list: keep the existing
/// (vanilla) entries and append any enabled-mod `.bsa` not already present, then
/// rewrite `Archive 0..N`. Morrowind only loads a BSA that is listed here, so a
/// BSA-shipping mod is otherwise silently ignored.
pub fn register_morrowind_archives(ini: &Path, mod_bsas: &[String]) -> io::Result<()> {
    let (mut text, _) = read_ini_text(ini)?;
    let mut archives = read_numbered_archives(&text);
    for b in mod_bsas {
        // The Morrowind engine only knows BSA. The shared [`mod_archives`] walk also
        // returns `.ba2` (FO4/Starfield), and a `.ba2` listed here would be a dead
        // entry the engine logs about, so drop anything that is not a BSA.
        if !b.to_ascii_lowercase().ends_with(".bsa") {
            continue;
        }
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

/// Whether `name` is a Bethesda archive: `.bsa` (Gamebryo through Skyrim SE) or
/// `.ba2` (Fallout 4, Fallout 76, Starfield).
pub fn is_archive_name(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    l.ends_with(".bsa") || l.ends_with(".ba2")
}

/// The archives each mod ships at the top of its folder, as `(mod name, archive
/// file name)`. Mods keep the caller's order (mod priority) and each mod's
/// archives are sorted by name, because `read_dir` order is arbitrary and a
/// diagnostic list that reshuffles between refreshes is unreadable.
///
/// `mods` is `(name, folder)` for the mods the caller counts as live - enabled and
/// not a separator - since only those reach the union. Only the top level of each
/// folder is scanned: a mod folder maps onto the game's `Data` root and the engine
/// loads archives from `Data` alone, so a `.bsa` sitting in a subfolder is never
/// read by the game. An unreadable mod folder contributes nothing rather than
/// failing the whole walk - this feeds a diagnostic, not a launch step.
pub fn mod_archives(mods: &[(String, PathBuf)]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (name, dir) in mods {
        let Ok(entries) = fs::read_dir(dir) else { continue };
        let mut found: Vec<String> = entries
            .flatten()
            // A directory named `*.bsa` is not an archive. `file_type` is a cheap
            // lstat that cannot fail for an entry we just listed; if it does, keep
            // the entry - a missed archive is worse than a spurious one.
            .filter(|e| !e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| is_archive_name(n))
            .collect();
        found.sort_by_key(|n| n.to_ascii_lowercase());
        out.extend(found.into_iter().map(|a| (name.clone(), a)));
    }
    out
}

/// The archives no active plugin can load and that the INI does not register: dead
/// weight, and a classic silent failure - a mod ships `MyMod - Textures.bsa`, its
/// `MyMod.esp` is disabled or was renamed by a patch, and the mod appears installed
/// while contributing nothing. Returns the offending `(mod name, archive)` pairs in
/// input order.
///
/// `archives` comes from [`mod_archives`], `active_plugin_names` is the ENABLED
/// plugins' file names (`Foo.esp`), and `ini_archives` from
/// [`registered_archives`] - an archive named there loads on its own and is never
/// an orphan.
pub fn orphan_archives(
    archives: &[(String, String)],
    active_plugin_names: &[String],
    ini_archives: &[String],
) -> Vec<(String, String)> {
    let bases: Vec<String> = active_plugin_names
        .iter()
        .map(|p| base_name(p).to_ascii_lowercase())
        .filter(|b| !b.is_empty())
        .collect();
    archives
        .iter()
        .filter(|(_, archive)| {
            if ini_archives.iter().any(|r| r.trim().eq_ignore_ascii_case(archive)) {
                return false;
            }
            let lower = archive.to_ascii_lowercase();
            let stem = base_name(&lower);
            // Stricter than MO2's `hasAssociatedPlugin` (`mainwindow.cpp:2081-2093`),
            // which only asks whether the archive name STARTS WITH a plugin's base
            // name: that ties `MyMod2 - Textures.bsa` to `MyMod.esp` and hides a real
            // orphan. The engine's own rule is `<base>.bsa` or `<base> - <suffix>.bsa`
            // (`- Textures`, `- Main`, `- Voices_en0`), so demanding equality or the
            // `" - "` separator is both stricter and closer to what the game loads.
            !bases.iter().any(|b| {
                stem == b || stem.strip_prefix(b.as_str()).is_some_and(|r| r.starts_with(" - "))
            })
        })
        .cloned()
        .collect()
}

/// A file name without its last extension (`MyMod - Textures.bsa` -> `MyMod -
/// Textures`, `Foo.esp` -> `Foo`). A name with no dot is its own base.
fn base_name(file: &str) -> &str {
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

/// The archives an INI registers explicitly. These load regardless of the plugin
/// list, so they are never orphans. Every list key across the engine generations is
/// read: `SArchiveList` (Oblivion/FO3/FNV), `sResourceArchiveList` plus its
/// `sResourceArchiveList2` continuation (Skyrim onwards - reading only the first
/// would report the whole tail of a split list as orphaned), and Morrowind's
/// numbered `[Archives]` block. Order is preserved and duplicates collapse.
pub fn registered_archives(ini_text: &str) -> Vec<String> {
    let mut out = read_numbered_archives(ini_text);
    for key in ["SArchiveList", "sResourceArchiveList", "sResourceArchiveList2"] {
        let Some(value) = eidos_ini::get_key(ini_text, "Archive", key) else { continue };
        out.extend(value.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned));
    }
    let mut seen: Vec<String> = Vec::with_capacity(out.len());
    out.retain(|a| {
        let l = a.to_ascii_lowercase();
        let fresh = !seen.contains(&l);
        if fresh {
            seen.push(l);
        }
        fresh
    });
    out
}

/// [`registered_archives`] unioned across a game's per-profile INIs in `ini_dir`
/// (for Morrowind that directory is the game install, where MO2 keeps its INI).
/// Missing or unreadable INIs contribute nothing: this only ever widens the
/// exemption set, so a read failure can at worst report an extra orphan.
pub fn registered_archives_in(ini_dir: &Path, game_id: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for file in ini_files_for(game_id) {
        let Ok((text, _)) = read_ini_text(&ini_dir.join(file)) else { continue };
        for a in registered_archives(&text) {
            if !out.iter().any(|e| e.eq_ignore_ascii_case(&a)) {
                out.push(a);
            }
        }
    }
    out
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

/// Read an INI file as text, returning `(text, was Latin-1)`. A missing file reads
/// as empty so callers can create it; any other error is returned, because an
/// EXISTING file must never be treated as empty - a caller would then truncate it.
///
/// Bethesda INIs localized in Windows-1252 (accented FR/DE text) are not valid
/// UTF-8: those decode as Latin-1, a byte-for-byte reversible mapping, so an edit
/// can encode back and preserve every original byte (see [`set_ini_key`]).
fn read_ini_text(path: &Path) -> io::Result<(String, bool)> {
    match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => Ok((s, false)),
            Err(e) => Ok((e.into_bytes().iter().map(|&b| b as char).collect(), true)),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok((String::new(), false)),
        Err(e) => Err(e),
    }
}

/// Set `[section] key=value` in an INI file on disk, preserving everything else
/// (and the file's CRLF/LF style). A thin file-I/O wrapper over the shared,
/// format-preserving [`eidos_ini::set_key`]; section and key match
/// case-insensitively.
pub fn set_ini_key(path: &Path, section: &str, key: &str, value: &str) -> io::Result<()> {
    let (existing, latin1) = read_ini_text(path)?;
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
    fn morrowind_registration_ignores_ba2() {
        let dir = tmp_dir();
        let ini = dir.join("Morrowind.ini");
        fs::write(&ini, "[Archives]\r\nArchive 0=Morrowind.bsa\r\n").unwrap();
        // The shared walk returns .ba2 too; the Morrowind engine cannot read one.
        register_morrowind_archives(&ini, &["ModX.bsa".into(), "ModY.ba2".into()]).unwrap();
        let s = fs::read_to_string(&ini).unwrap();
        assert!(s.contains("Archive 1=ModX.bsa"));
        assert!(!s.contains("ModY.ba2"));
        let _ = fs::remove_dir_all(&dir);
    }

    // --- the orphan-archive diagnostic -------------------------------------

    fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
        v.iter().map(|(m, a)| ((*m).to_string(), (*a).to_string())).collect()
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn archive_is_matched_to_its_own_plugin_only() {
        let archives = pairs(&[
            ("A", "MyMod.bsa"),             // exact base name
            ("A", "MyMod - Textures.bsa"),  // the engine's suffix form
            ("B", "MyMod2 - Textures.bsa"), // MO2's loose startsWith would miss this
            ("C", "Unrelated.bsa"),
        ]);
        let orphans = orphan_archives(&archives, &names(&["MyMod.esp"]), &[]);
        assert_eq!(orphans, pairs(&[("B", "MyMod2 - Textures.bsa"), ("C", "Unrelated.bsa")]));

        // Enabling MyMod2.esp rescues its archive and nothing else.
        let orphans = orphan_archives(&archives, &names(&["MyMod.esp", "MyMod2.esp"]), &[]);
        assert_eq!(orphans, pairs(&[("C", "Unrelated.bsa")]));
    }

    #[test]
    fn disabled_plugin_orphans_its_archive() {
        let archives = pairs(&[("A", "MyMod - Textures.bsa")]);
        // The caller passes ACTIVE plugins only: MyMod.esp exists but is unchecked,
        // so its archive is dead weight and must be reported.
        assert_eq!(orphan_archives(&archives, &names(&["Skyrim.esm"]), &[]), archives);
        // Same file with the plugin enabled: silent.
        assert!(orphan_archives(&archives, &names(&["Skyrim.esm", "MyMod.esp"]), &[]).is_empty());
    }

    #[test]
    fn ini_registered_archive_is_never_an_orphan() {
        let archives = pairs(&[("A", "Standalone.bsa"), ("A", "Other.bsa")]);
        // Registered in the INI (case-insensitively): it loads without any plugin.
        let ini = names(&["standalone.bsa"]);
        assert_eq!(orphan_archives(&archives, &[], &ini), pairs(&[("A", "Other.bsa")]));
    }

    #[test]
    fn ba2_is_treated_like_bsa() {
        let archives = pairs(&[
            ("A", "MyMod - Main.ba2"),
            ("A", "MyMod - Textures.ba2"),
            ("B", "Ghost - Main.ba2"),
            ("B", "Ghost.bsa"),
        ]);
        let orphans = orphan_archives(&archives, &names(&["MyMod.esp"]), &[]);
        assert_eq!(orphans, pairs(&[("B", "Ghost - Main.ba2"), ("B", "Ghost.bsa")]));
    }

    #[test]
    fn orphan_matching_is_case_insensitive_and_extension_agnostic() {
        let archives = pairs(&[("A", "MYMOD - TEXTURES.BSA"), ("A", "Lights.bsa")]);
        // Plugin extensions differ (.esm/.esl/.esp) and Windows names are case-blind.
        let orphans = orphan_archives(&archives, &names(&["mymod.esm", "Lights.esl"]), &[]);
        assert!(orphans.is_empty(), "{orphans:?}");
    }

    #[test]
    fn orphan_matching_tolerates_degenerate_names() {
        // A plugin that is only an extension, an archive with no stem, empty lists:
        // nothing here may panic or match by accident.
        let archives = pairs(&[("A", ".bsa"), ("A", "x.bsa"), ("A", "noext")]);
        assert_eq!(orphan_archives(&archives, &names(&[".esp", ""]), &[]).len(), 3);
        assert!(orphan_archives(&[], &names(&["A.esp"]), &[]).is_empty());
    }

    #[test]
    fn mod_archives_walks_only_the_top_level() {
        let a = tmp_dir();
        let b = tmp_dir();
        fs::write(a.join("Zeta.bsa"), b"").unwrap();
        fs::write(a.join("Alpha.ba2"), b"").unwrap();
        fs::write(a.join("readme.txt"), b"").unwrap();
        fs::create_dir_all(a.join("Bogus.bsa")).unwrap(); // a DIRECTORY named .bsa
        fs::create_dir_all(a.join("textures")).unwrap();
        fs::write(a.join("textures/Deep.bsa"), b"").unwrap(); // not in the Data root
        fs::write(b.join("B.bsa"), b"").unwrap();

        let mods =
            vec![("ModA".to_string(), a.clone()), ("ModB".to_string(), b.clone()),
                 ("Gone".to_string(), a.join("does-not-exist"))];
        // Mod order preserved, archives sorted inside a mod, a missing folder skipped.
        assert_eq!(
            mod_archives(&mods),
            pairs(&[("ModA", "Alpha.ba2"), ("ModA", "Zeta.bsa"), ("ModB", "B.bsa")])
        );
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
    }

    #[test]
    fn registered_archives_reads_every_list_key() {
        // Skyrim onwards splits the list in two because the engine truncates an INI
        // value at 255 chars; reading only the first key would orphan the tail.
        let text = "[Archive]\r\nsResourceArchiveList=Skyrim - Misc.bsa, Skyrim - Shaders.bsa\r\n\
                    sResourceArchiveList2= Skyrim - Voices.bsa ,\r\n";
        assert_eq!(
            registered_archives(text),
            names(&["Skyrim - Misc.bsa", "Skyrim - Shaders.bsa", "Skyrim - Voices.bsa"])
        );
        // Oblivion/FO3/FNV key, and Morrowind's numbered block, both understood.
        let old = registered_archives("[Archive]\nSArchiveList=Oblivion - Meshes.bsa\n");
        assert_eq!(old, names(&["Oblivion - Meshes.bsa"]));
        assert_eq!(
            registered_archives("[Archives]\nArchive 0=Morrowind.bsa\nArchive 1=Tribunal.bsa\n"),
            names(&["Morrowind.bsa", "Tribunal.bsa"])
        );
        // Duplicates across keys collapse; a truncated file yields nothing.
        let dup = registered_archives("[Archive]\nSArchiveList=a.bsa\nsResourceArchiveList=A.BSA\n");
        assert_eq!(dup, names(&["a.bsa"]));
        assert!(registered_archives("[Archive").is_empty());
    }

    #[test]
    fn registered_archives_in_unions_the_profile_inis() {
        let dir = tmp_dir();
        // Skyrim SE reads Skyrim.ini and SkyrimCustom.ini; a mod may be listed in
        // either, and the missing SkyrimPrefs.ini must not abort the union.
        fs::write(dir.join("Skyrim.ini"), "[Archive]\r\nsResourceArchiveList=Skyrim - Misc.bsa\r\n").unwrap();
        fs::write(dir.join("SkyrimCustom.ini"), "[Archive]\r\nsResourceArchiveList=Standalone.bsa\r\n").unwrap();
        let got = registered_archives_in(&dir, "skyrimse");
        assert_eq!(got, names(&["Skyrim - Misc.bsa", "Standalone.bsa"]));
        assert!(registered_archives_in(&dir, "nope").is_empty());
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
    fn prepend_keeps_a_windows_1252_vanilla_list() {
        let docs = tmp_dir();
        let data = tmp_dir();
        // A localized Oblivion.ini (0xE9 = e-acute) is not UTF-8: reading it as
        // UTF-8 and falling back to "" would silently drop the vanilla archives.
        let original = b"[Archive]\r\nSArchiveList=Oblivion - Voices Fran\xE7ais.bsa\r\n".to_vec();
        fs::write(docs.join("Oblivion.ini"), &original).unwrap();
        enable_bsa_invalidation(&docs, &data, "oblivion").unwrap();
        let after: String = fs::read(docs.join("Oblivion.ini")).unwrap().iter().map(|&b| b as char).collect();
        assert!(after.contains("Oblivion - Invalidation.bsa, Oblivion - Voices Fran\u{e7}ais.bsa"));
        let _ = fs::remove_dir_all(&docs);
        let _ = fs::remove_dir_all(&data);
    }

    #[test]
    fn long_skyrim_archive_list_overflows_into_list2() {
        let docs = tmp_dir();
        let data = tmp_dir();
        // A list already near the engine's 255-char INI value cut, plus the List2
        // continuation Skyrim LE ships. Prepending the dummy pushes past the cut.
        let vanilla: Vec<String> = (0..11).map(|i| format!("Skyrim - Filler{i:02}.bsa")).collect();
        let joined = vanilla.join(", ");
        assert!((200..=INI_VALUE_MAX).contains(&joined.len()), "fixture must start under the cut");
        fs::write(
            docs.join("Skyrim.ini"),
            format!("[Archive]\r\nsResourceArchiveList={joined}\r\nsResourceArchiveList2=Skyrim - Voices.bsa\r\n"),
        )
        .unwrap();
        enable_bsa_invalidation(&docs, &data, "skyrim").unwrap();

        let text = fs::read_to_string(docs.join("Skyrim.ini")).unwrap();
        let read = |k: &str| eidos_ini::get_key(&text, "Archive", k).unwrap().trim().to_string();
        let (l1, l2) = (read("sResourceArchiveList"), read("sResourceArchiveList2"));
        assert!(l1.len() <= INI_VALUE_MAX, "head must survive the engine's truncation: {}", l1.len());
        assert!(l1.len() > INI_VALUE_MAX - 30, "head must not be split earlier than it has to");
        assert!(!l1.ends_with(','));
        // Nothing lost, and the order across the two keys is unchanged (the engine
        // reads List then List2), so archive precedence is preserved.
        let mut expected = vec!["Skyrim - Invalidation.bsa".to_string()];
        expected.extend(vanilla.iter().cloned());
        expected.push("Skyrim - Voices.bsa".to_string());
        let got: Vec<String> =
            format!("{l1}, {l2}").split(',').map(|s| s.trim().to_string()).collect();
        assert_eq!(got, expected);

        // Idempotent even though the dummy now sits in a different key than it would
        // have without the split.
        enable_bsa_invalidation(&docs, &data, "skyrim").unwrap();
        let text2 = fs::read_to_string(docs.join("Skyrim.ini")).unwrap();
        assert_eq!(text2.matches("Skyrim - Invalidation.bsa").count(), 1);
        let _ = fs::remove_dir_all(&docs);
        let _ = fs::remove_dir_all(&data);
    }

    #[test]
    fn archive_value_split_has_no_split_point() {
        // One entry longer than the limit: returned whole rather than dropped.
        let huge = format!("{}.bsa", "x".repeat(300));
        assert_eq!(split_archive_value(&huge), (huge.as_str(), ""));
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

    #[test]
    fn an_unknown_load_order_is_not_evidence_of_orphans() {
        // Lin's eleven. Every one of these archives had an ACTIVE plugin; the
        // window reported all eleven as unloadable because the caller passed an
        // empty active list - it had simply never computed one. The rule the
        // caller now enforces is asserted here, at the layer that would be asked
        // the question: with nothing known, every archive looks orphaned, so an
        // empty active list must never reach this function.
        let archives = vec![
            ("SkyUI".to_string(), "SkyUI_SE.bsa".to_string()),
            ("USSEP FR".to_string(), "unofficial skyrim special edition patch.bsa".to_string()),
            (
                "USSEP FR".to_string(),
                "unofficial skyrim special edition patch - textures.bsa".to_string(),
            ),
        ];
        assert_eq!(
            orphan_archives(&archives, &[], &[]).len(),
            3,
            "with no active plugins EVERYTHING is an orphan - which is why the \
             caller must not ask when it does not know"
        );

        // With the real load order, in the real lowercase the mod ships, none of
        // them is an orphan - including the ` - Textures` sibling.
        let active = vec![
            "SkyUI_SE.esp".to_string(),
            "unofficial skyrim special edition patch.esp".to_string(),
        ];
        assert!(orphan_archives(&archives, &active, &[]).is_empty());

        // And case never decides it: the archive is lowercase, the plugin is not.
        let shouty = vec!["Unofficial Skyrim Special Edition Patch.ESP".to_string()];
        assert_eq!(orphan_archives(&archives[1..], &shouty, &[]).len(), 0);
    }

}
