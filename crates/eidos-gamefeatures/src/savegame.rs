//! Bethesda save-game header parsing: who the save belongs to and - the part that
//! actually matters - the plugin list it was created with. Covers the Skyrim
//! family (`.ess`) and Fallout 4 (`.fos`); see [`SaveEngine`] for how they differ.
//!
//! Eidos keeps saves per profile, so a save routinely outlives the mod list that
//! produced it. A save that references a plugin the profile no longer loads is
//! precisely the setup that produces a mid-playthrough crash or a silently dead
//! quest, and the only way to warn about it is to read the save's own plugin table.
//! This is a port of MO2's `GamebryoSaveGame` / `GamebryoSaveGameInfo`
//! (`libs/game_bethesda`), narrowed to the engines Eidos verifies against.
//!
//! Everything here is read-only and cheap: the parse stops at the end of the plugin
//! table and never touches the multi-megabyte object graph behind it. The screenshot
//! pixels are deliberately skipped, not decoded - only its dimensions are needed to
//! find the compressed block. A save that is truncated, corrupt, or still being
//! written by the running game returns an error and never panics, so the caller can
//! degrade to the filename/date/size row it already had.
//!
//! Wire layout (Skyrim SE, and identically Enderal SE / Skyrim VR):
//!
//! ```text
//!   13   magic "TESV_SAVEGAME"
//!   u32  headerSize
//!   u32  version                  12 on SE, 9 on the LE engine
//!   u32  saveNumber
//!   ws   playerName               ws = u16 byte length, then that many UTF-8 bytes
//!   u32  playerLevel
//!   ws   playerLocation
//!   ws   gameDate
//!   ws   playerRaceEditorId
//!   u16  playerSex
//!   f32  playerCurExp, f32 playerLvlUpExp
//!   u64  FILETIME
//!   u32  shotWidth, u32 shotHeight
//!   u16  compressionType          SE only, i.e. when version == 12
//!   ..   screenshot               width*height*4 on SE (RGBA), *3 on LE (RGB)
//!   u32  uncompressedSize, u32 compressedSize     only when compressionType != 0
//!   ..   payload                  raw / zlib stream / LZ4 block
//!   -- inside the payload --
//!   u8   formVersion
//!   u32  pluginInfoSize
//!   u8   pluginCount              then that many ws
//!   u16  lightPluginCount         only when formVersion >= 78, then that many ws
//! ```
//!
//! Wire layout (Fallout 4, and identically Fallout 4 VR). Same shape, three
//! differences, each of which shifts everything after it if missed:
//!
//! ```text
//!   12   magic "FO4_SAVEGAME"      one byte shorter than Skyrim's
//!   u32  headerSize
//!   u32  version                  15 on the builds seen in the wild
//!   u32  saveNumber
//!   ws   playerName
//!   u32  playerLevel
//!   ws   playerLocation
//!   ws   playtime                 localised, e.g. "0j.22h.20m.0 jours..."
//!   ws   playerRaceEditorId
//!   u16  playerSex
//!   f32  playerCurExp, f32 playerLvlUpExp
//!   u64  FILETIME
//!   u32  shotWidth, u32 shotHeight
//!   ..   screenshot               width*height*4 (RGBA); NO compressionType word,
//!                                 and the payload is never compressed
//!   u8   formVersion
//!   ws   gameVersion              Fallout only, e.g. "1.10.163.0"
//!   u32  pluginInfoSize           spans the WHOLE block, ESL table included
//!   u8   pluginCount              then that many ws
//!   u16  lightPluginCount         when pluginInfoSize says bytes remain
//! ```

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// The 13-byte file id every Skyrim (LE and SE) save opens with.
const MAGIC: &[u8; 13] = b"TESV_SAVEGAME";

/// The 12-byte file id of a Fallout 4 save (`.fos`). One byte shorter than
/// Skyrim's, which is why the magic is matched before anything else is read.
const FO4_MAGIC: &[u8; 12] = b"FO4_SAVEGAME";

/// Header `version` for the Special Edition engine. SE alone carries the
/// `compressionType` field and an alpha channel in the screenshot.
const SE_HEADER_VERSION: u32 = 12;

/// `formVersion` at which the engine started writing the light-plugin (ESL) block
/// after the normal plugin table (SSE 1.5.39+). Read from INSIDE the compressed
/// payload, not from the outer header version.
const LIGHT_PLUGIN_FORM_VERSION: u8 = 78;

/// How much of the file to pull in for the fixed header. The four strings are each
/// bounded by a u16 length, so 288 KiB covers even the pathological maximum; a real
/// header is well under a kilobyte.
const HEADER_WINDOW: u64 = 288 * 1024;

/// How much of the compressed block to read. The plugin table sits at the very
/// start of the payload, so the first megabyte always contains it - and refusing to
/// read further keeps a 40 MB save cheap.
const COMPRESSED_WINDOW: u64 = 1024 * 1024;

/// Hard ceiling on decompressed bytes. `uncompressedSize` comes straight out of the
/// file, so a corrupt u32 must never turn into a multi-gigabyte allocation; the
/// plugin table needs a few kilobytes at most.
const DECOMPRESS_CAP: usize = 4 * 1024 * 1024;

/// Seconds between the FILETIME epoch (1601-01-01) and the Unix epoch.
const FILETIME_TO_UNIX_SECS: i64 = 11_644_473_600;

/// Which engine wrote the save. The two layouts share their shape but differ in
/// three concrete places (magic length, the SE-only `compressionType`, and the
/// Fallout-only `gameVersion` string), so the parser carries this rather than
/// re-deriving it from the header version at each fork.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SaveEngine {
    /// Skyrim LE/SE, Enderal SE, Skyrim VR - `TESV_SAVEGAME`.
    #[default]
    Skyrim,
    /// Fallout 4 and Fallout 4 VR - `FO4_SAVEGAME`.
    Fallout4,
}

/// How the payload holding the plugin table is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SaveCompression {
    /// LE engine, and Fallout 4: the payload follows the screenshot verbatim.
    #[default]
    None,
    /// zlib stream (SSE builds before the LZ4 switch).
    Zlib,
    /// LZ4 block format - what retail SSE writes.
    Lz4,
}

/// Everything [`parse_save`] extracts from a save header.
#[derive(Debug, Clone, Default)]
pub struct SaveInfo {
    /// Which engine wrote it, from the file magic.
    pub engine: SaveEngine,
    /// The runtime version string the save was written by, e.g. `1.10.163.0`.
    /// Fallout 4 writes this next to `formVersion`; Skyrim has no such field and
    /// leaves it empty. Worth surfacing: a save from a different game build is
    /// exactly what a "why did my DLL plugins stop loading" session looks like.
    pub game_version: String,
    pub player_name: String,
    pub level: u32,
    pub location: String,
    /// The engine's in-game clock string, verbatim. Skyrim writes
    /// `"<days>.<hours>.<minutes>"`; see [`SaveInfo::playtime`].
    pub game_date: String,
    /// Editor id of the player race, e.g. `BretonRace`.
    pub race: String,
    /// The in-game save counter, the `#N` MO2 shows beside the character name.
    pub save_number: u32,
    /// Save creation time as a Unix timestamp in seconds, converted from the header
    /// FILETIME. `None` when the field is absent or implausible - callers keep using
    /// the file mtime in that case.
    pub created_unix: Option<i64>,
    /// Outer header version: 12 for SE, 9 for the LE engine.
    pub header_version: u32,
    /// `formVersion` from inside the payload. Gates the light-plugin block.
    pub form_version: u8,
    pub compression: SaveCompression,
    /// Screenshot dimensions. The pixels themselves are skipped on purpose.
    pub screenshot_width: u32,
    pub screenshot_height: u32,
    /// Byte offset of the first screenshot pixel, and how many bytes each takes
    /// (3 on the Skyrim LE engine, 4 on SE and Fallout 4). Recorded so
    /// [`read_screenshot`] can fetch the image on demand without parsing again.
    pub screenshot_offset: u64,
    pub screenshot_bytes_per_pixel: u8,
    /// Full-index plugins, in the save's own load order.
    pub plugins: Vec<String>,
    /// Light (ESL) plugins. Present from `formVersion` 78 on for Skyrim, and
    /// whenever `pluginInfoSize` says the block has bytes left for Fallout 4.
    pub light_plugins: Vec<String>,
    /// True when the plugin list is known to be INCOMPLETE: the save announced a
    /// light-plugin block that ran off the end of the data. A missing-plugin diff
    /// built from such a save is advisory, not authoritative - it can only
    /// under-report. False on a healthy save, including the usual case where we
    /// deliberately stopped reading just past the plugin table.
    pub truncated: bool,
}

impl SaveInfo {
    /// Every plugin the save references, normal ones first, then light ones.
    pub fn all_plugins(&self) -> impl Iterator<Item = &str> {
        self.plugins
            .iter()
            .chain(self.light_plugins.iter())
            .map(String::as_str)
    }

    /// The in-game clock as `(days, hours, minutes)`.
    ///
    /// Skyrim writes `game_date` as three dot-separated integers (`"5.3.42"`).
    /// Fallout 4 writes the same three numbers with localised unit suffixes and
    /// then repeats them spelled out - a French save reads
    /// `"0j.22h.20m.0 jours.22 heures.20 minutes"` - so the leading digits of the
    /// first three segments are the common denominator. A segment that does not
    /// begin with a digit yields `None` rather than a guess; the raw string stays
    /// available for display either way.
    pub fn playtime(&self) -> Option<(u32, u32, u32)> {
        // Digits up to the first non-digit: "5" -> 5, "22h" -> 22. Empty (or a
        // segment starting with a letter) is a parse failure, not a zero.
        fn leading_number(part: &str) -> Option<u32> {
            let digits: String = part
                .trim()
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse().ok()
        }
        // Skyrim's field is pure integers, and it stays parsed that way: loosening
        // it for both engines would turn a garbled Skyrim date like "12abc.5.3"
        // from an honest None into a confident wrong answer.
        let lenient = self.engine == SaveEngine::Fallout4;
        let number = |part: &str| {
            if lenient {
                leading_number(part)
            } else {
                part.trim().parse().ok()
            }
        };
        let mut parts = self.game_date.split('.');
        let d = number(parts.next()?)?;
        let h = number(parts.next()?)?;
        let m = number(parts.next()?)?;
        // Skyrim's field is exactly three segments; Fallout's spelled-out tail adds
        // more. Anything else with a 4th purely-numeric segment is a format this
        // parser does not know, so refuse it rather than report two thirds of it.
        match parts.next() {
            None => Some((d, h, m)),
            Some(_) if self.engine == SaveEngine::Fallout4 => Some((d, h, m)),
            Some(_) => None,
        }
    }
}

/// Why a save could not be read. Every variant is a normal outcome for a file that
/// the game may be writing right now, so none of them warrants more than degrading
/// the row back to name/date/size.
#[derive(Debug)]
pub enum SaveParseError {
    /// The file could not be opened, seeked or read.
    Io(io::Error),
    /// Not a save at all: neither the `TESV_SAVEGAME` nor the `FO4_SAVEGAME`
    /// magic is present.
    NotASave,
    /// The file ends inside the named field - truncated, or still being written.
    Truncated(&'static str),
    /// A field is present but impossible (a screenshot larger than the file, a zero
    /// LZ4 back-reference, ...).
    Corrupt(&'static str),
    /// A `compressionType` this parser does not know how to open.
    UnknownCompression(u16),
}

impl fmt::Display for SaveParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveParseError::Io(e) => write!(f, "{e}"),
            SaveParseError::NotASave => {
                write!(
                    f,
                    "not a save file (no TESV_SAVEGAME or FO4_SAVEGAME magic)"
                )
            }
            SaveParseError::Truncated(field) => write!(f, "save ends inside {field}"),
            SaveParseError::Corrupt(what) => write!(f, "save is corrupt: {what}"),
            SaveParseError::UnknownCompression(t) => write!(f, "unknown save compression type {t}"),
        }
    }
}

impl std::error::Error for SaveParseError {}

impl From<io::Error> for SaveParseError {
    fn from(e: io::Error) -> Self {
        SaveParseError::Io(e)
    }
}

/// Parse the header of a save, up to and including the plugin table.
///
/// Reads three bounded windows out of the file (fixed header, block sizes, first
/// slice of the payload) and stops - the object graph after the plugin table is
/// never touched, which is what makes this affordable to run over a whole saves
/// directory. Handles Skyrim SE, LE-engine saves (`version` 9, uncompressed,
/// 3-byte screenshot pixels), Enderal SE / Skyrim VR, and Fallout 4 / Fallout 4
/// VR - the engine is taken from the file magic, not from the caller.
pub fn parse_save(path: &Path) -> Result<SaveInfo, SaveParseError> {
    let mut file = fs::File::open(path)?;
    let file_len = file.metadata()?.len();

    let mut head = Vec::new();
    file.by_ref().take(HEADER_WINDOW).read_to_end(&mut head)?;
    let mut cur = Cur::new(&head);

    // The magic decides the engine AND how many bytes it occupied, so it is
    // matched before the cursor moves: Fallout's is 12 bytes, Skyrim's 13, and
    // consuming the wrong count shifts every field that follows by one.
    let engine = if head.starts_with(MAGIC.as_slice()) {
        SaveEngine::Skyrim
    } else if head.starts_with(FO4_MAGIC.as_slice()) {
        SaveEngine::Fallout4
    } else {
        // Covers the empty file and the "someone renamed a .txt" case alike: both
        // are "this is not a save", not "this save is broken".
        return Err(SaveParseError::NotASave);
    };
    let magic_len = match engine {
        SaveEngine::Skyrim => MAGIC.len(),
        SaveEngine::Fallout4 => FO4_MAGIC.len(),
    };
    cur.skip(magic_len, "magic")?;

    let mut info = SaveInfo {
        engine,
        ..SaveInfo::default()
    };
    // headerSize is read but not used to seek: the two published readings of it
    // disagree on whether the screenshot dimensions are inside the header, and both
    // leave the sequential cursor in the same place, so sequential reads (what MO2
    // does, and what is known to work on real saves) are the safer of the two.
    cur.skip(4, "headerSize")?;
    info.header_version = cur.u32("version")?;
    info.save_number = cur.u32("saveNumber")?;
    info.player_name = cur.wstring("playerName")?;
    info.level = cur.u32("playerLevel")?;
    info.location = cur.wstring("playerLocation")?;
    info.game_date = cur.wstring("gameDate")?;
    info.race = cur.wstring("playerRaceEditorId")?;
    cur.skip(2, "playerSex")?;
    cur.skip(8, "experience")?; // current xp + xp needed, two f32 we have no use for
    info.created_unix = filetime_to_unix(cur.u64("fileTime")?);
    info.screenshot_width = cur.u32("shotWidth")?;
    info.screenshot_height = cur.u32("shotHeight")?;

    // SE is the only engine with a compressionType word. Alpha in the screenshot
    // is a separate question: SE has it (gated on the header version), and so does
    // Fallout 4 - whose payload is nonetheless never compressed. Conflating the two
    // would either mis-size the screenshot or read a compression word that is not
    // there, and both shift the plugin table out of reach.
    let (compression_raw, bytes_per_pixel) = match info.engine {
        SaveEngine::Fallout4 => (0, 4u64),
        SaveEngine::Skyrim if info.header_version == SE_HEADER_VERSION => {
            (cur.u16("compressionType")?, 4u64)
        }
        SaveEngine::Skyrim => (0, 3u64),
    };
    info.compression = match compression_raw {
        0 => SaveCompression::None,
        1 => SaveCompression::Zlib,
        2 => SaveCompression::Lz4,
        other => return Err(SaveParseError::UnknownCompression(other)),
    };

    // Where the pixels start, kept so `read_screenshot` can come back for them
    // without re-deriving the header. Parsing NEVER reads them: the whole reason
    // this is affordable over a saves directory is that it touches three bounded
    // windows, and a screenshot is a megabyte per save.
    info.screenshot_offset = cur.pos as u64;
    info.screenshot_bytes_per_pixel = bytes_per_pixel as u8;

    // Skip the screenshot without reading it. u64 throughout with a checked multiply:
    // width and height are attacker-visible u32s straight out of the file.
    let pixels = u64::from(info.screenshot_width)
        .checked_mul(u64::from(info.screenshot_height))
        .and_then(|p| p.checked_mul(bytes_per_pixel))
        .ok_or(SaveParseError::Corrupt("screenshot dimensions overflow"))?;
    let payload_start = (cur.pos as u64)
        .checked_add(pixels)
        .ok_or(SaveParseError::Corrupt("screenshot dimensions overflow"))?;
    if payload_start > file_len {
        return Err(SaveParseError::Truncated("screenshot"));
    }
    file.seek(SeekFrom::Start(payload_start))?;

    let payload = read_payload(&mut file, info.compression)?;
    read_plugin_table(&payload, &mut info)?;
    Ok(info)
}

/// Read and, if needed, decompress the front of the block that holds the plugin
/// table. Deliberately a PREFIX: a real save's payload is tens of megabytes and the
/// plugin table is the first thing in it, so both the input read and the
/// decompressed output stop at a fixed cap. Everything downstream is written to
/// cope with a payload that ends early.
fn read_payload(
    file: &mut fs::File,
    compression: SaveCompression,
) -> Result<Vec<u8>, SaveParseError> {
    if compression == SaveCompression::None {
        let mut raw = Vec::new();
        file.by_ref()
            .take(COMPRESSED_WINDOW)
            .read_to_end(&mut raw)?;
        return Ok(raw);
    }

    let mut sizes = [0u8; 8];
    // uncompressedSize then compressedSize; only the second is load-bearing here,
    // and trusting the first would mean sizing a buffer from the file.
    let read = read_up_to(file, &mut sizes)?;
    if read < sizes.len() {
        return Err(SaveParseError::Truncated("compressed block sizes"));
    }
    let compressed_size = u32::from_le_bytes([sizes[4], sizes[5], sizes[6], sizes[7]]);
    if compressed_size == 0 {
        return Err(SaveParseError::Corrupt("compressed block is empty"));
    }
    let want = u64::from(compressed_size).min(COMPRESSED_WINDOW);
    let mut compressed = Vec::new();
    file.by_ref().take(want).read_to_end(&mut compressed)?;

    // Both decoders keep whatever they produced before hitting a fault or the end of
    // a short input, which is what lets a half-written save still yield its table.
    // Their success flag is deliberately ignored here: stopping early is the normal
    // case (we asked for a prefix), and the plugin-table reader below is what decides
    // whether what came out is usable.
    let mut out = Vec::new();
    let _fully_decoded = match compression {
        SaveCompression::Lz4 => lz4_block_decompress(&compressed, DECOMPRESS_CAP, &mut out),
        SaveCompression::Zlib => inflate_prefix(&compressed, DECOMPRESS_CAP, &mut out),
        SaveCompression::None => true,
    };
    if out.is_empty() {
        return Err(SaveParseError::Corrupt("compressed block did not decode"));
    }
    Ok(out)
}

/// Fill `buf` from `file`, tolerating short reads. Returns how many bytes landed.
/// `Read::read_exact` would turn "the game is still writing this save" into an
/// error with no partial data, which is exactly the case we want to survive.
fn read_up_to(file: &mut fs::File, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

/// Read `formVersion`, the plugin table and the light-plugin block out of the
/// decompressed payload.
fn read_plugin_table(payload: &[u8], info: &mut SaveInfo) -> Result<(), SaveParseError> {
    let mut cur = Cur::new(payload);
    info.form_version = cur.u8("formVersion")?;
    // Fallout 4 writes the runtime version here, between formVersion and the block
    // size; Skyrim does not. Miss it and every subsequent field is read out of the
    // middle of that string.
    if info.engine == SaveEngine::Fallout4 {
        info.game_version = cur.wstring("gameVersion")?;
    }
    // MO2 reads this u32 as u8 + u16 + a 1-byte skip (skyrimsesavegame.cpp:145-149
    // plus readPlugins(1)); it is one field, the byte size of the plugin block, and
    // the count that follows is authoritative - so the count wins for the normal
    // table. The SIZE still earns its keep on Fallout, below.
    let info_size = cur.u32("pluginInfoSize")?;
    let block_start = cur.pos;

    let count = cur.u8("pluginCount")?;
    info.plugins = read_names(&mut cur, usize::from(count), "plugin name")?;

    // Where the light (ESL) block is, and whether there is one at all, is decided
    // differently by the two engines:
    //
    // - Skyrim: by `formVersion` (78+, SSE 1.5.39). There is no size to lean on.
    // - Fallout 4: by `pluginInfoSize`, which measures the WHOLE block - normal
    //   table plus ESL table. Verified against 40 real saves: 3064 bytes announced,
    //   2527 consumed by the 105 normal plugins, and the remaining 537 are exactly
    //   the 22 ESLs. Using the size means never guessing a formVersion threshold
    //   for an engine whose ESL support arrived mid-life, and it self-corrects on a
    //   save that has no ESL block: nothing is left over, so nothing is read.
    let has_light = match info.engine {
        SaveEngine::Skyrim => info.form_version >= LIGHT_PLUGIN_FORM_VERSION,
        SaveEngine::Fallout4 => {
            // The size does more than say "there is an ESL block": it also says
            // where that block ENDS, so the buffer is clamped to it. An overstated
            // size would otherwise pull whatever follows the table in as plugin
            // names, and a missing-plugin diff built from those is worse than none.
            let consumed = cur.pos - block_start;
            match usize::try_from(info_size) {
                Ok(size) if consumed < size => {
                    let end = block_start.saturating_add(size).min(cur.buf.len());
                    cur.buf = &cur.buf[..end];
                    true
                }
                // A size too large for usize is a corrupt field, not an invitation
                // to read: the ESL list is simply not reported on such a save.
                _ => false,
            }
        }
    };
    if has_light {
        // A save cut short right after the normal table is normal, not fatal: keep
        // the plugins we already have and flag the result as partial.
        match cur.u16("lightPluginCount") {
            Ok(light) => match read_names(&mut cur, usize::from(light), "light plugin name") {
                Ok(names) => info.light_plugins = names,
                Err(_) => info.truncated = true,
            },
            Err(_) => info.truncated = true,
        }
    }
    Ok(())
}

/// Read `count` length-prefixed names. Grows the vector as it goes rather than
/// reserving `count` up front: `count` is file-controlled and a corrupt u16 would
/// otherwise reserve 64 K entries before the first byte is validated.
fn read_names(
    cur: &mut Cur<'_>,
    count: usize,
    field: &'static str,
) -> Result<Vec<String>, SaveParseError> {
    let mut names = Vec::new();
    for _ in 0..count {
        names.push(cur.wstring(field)?);
    }
    Ok(names)
}

/// Convert a header FILETIME to a timestamp. `None` for a zero or pre-1970 value,
/// so a blank field shows up as "no timestamp" instead of a date in 1601.
///
/// No correction is applied. An earlier version subtracted six hours, citing MO2 -
/// but measured against 53 real saves (13 Skyrim, 40 Fallout 4) the raw value
/// already equals the save's own timestamp, on the nose, in every single one; the
/// subtraction was wrong by exactly that much and only survived because the test
/// fixture added the same offset before parsing it back.
///
/// What the engines actually write is their LOCAL wall clock into a UTC-shaped
/// field - which is why the raw decode matches the local time baked into the file
/// name. Consumers must therefore format this as UTC to reproduce what the game
/// and the file name show; formatting it as local time re-introduces an offset.
fn filetime_to_unix(raw: u64) -> Option<i64> {
    let secs = i64::try_from(raw / 10_000_000).ok()?;
    let unix = secs.checked_sub(FILETIME_TO_UNIX_SECS)?;
    (unix > 0).then_some(unix)
}

/// Inflate a zlib stream into `out`, stopping at `limit` bytes. Returns whether the
/// stream ended cleanly; a truncated stream still leaves everything decoded so far
/// in `out`, which is what lets a half-written save yield its plugin table.
fn inflate_prefix(src: &[u8], limit: usize, out: &mut Vec<u8>) -> bool {
    let mut dec = flate2::read::ZlibDecoder::new(src).take(limit as u64);
    if dec.read_to_end(out).is_ok() {
        return true;
    }
    // Some SSE builds write a bare deflate stream with no zlib wrapper. Only worth
    // retrying when the zlib attempt produced nothing at all.
    if out.is_empty() {
        let mut raw = flate2::read::DeflateDecoder::new(src).take(limit as u64);
        return raw.read_to_end(out).is_ok();
    }
    false
}

/// Decode an LZ4 *block* (not a frame) into `out`, stopping once `limit` bytes have
/// been produced. Returns whether the block was consumed to its end.
///
/// Hand-written rather than pulled from a crate, deliberately. The workspace has no
/// LZ4 dependency, and the crates that exist all want the declared output size up
/// front - a size that comes straight out of the save file, so a corrupt u32 would
/// have us allocate gigabytes before a single byte is validated. Decoding
/// incrementally under a hard cap keeps a malformed save cheap, and it lets us stop
/// as soon as the plugin table (which sits in the first few kilobytes) is out. Every
/// index is checked; a malformed block returns false rather than panicking, and
/// whatever decoded before the fault is kept.
fn lz4_block_decompress(src: &[u8], limit: usize, out: &mut Vec<u8>) -> bool {
    let mut i = 0usize;
    while i < src.len() {
        let token = src[i];
        i += 1;

        // Literal run: high nibble, extended by a chain of 255s when it saturates.
        let mut lit = usize::from(token >> 4);
        if lit == 15 && !read_lsic(src, &mut i, &mut lit) {
            return false;
        }
        let end = match i.checked_add(lit) {
            Some(e) if e <= src.len() => e,
            _ => return false,
        };
        let room = limit.saturating_sub(out.len());
        out.extend_from_slice(&src[i..end.min(i.saturating_add(room))]);
        i = end;
        if out.len() >= limit {
            return true;
        }
        // The final sequence of a block is literals only, with no match after it.
        if i >= src.len() {
            return true;
        }

        // Match: a 2-byte little-endian back-offset, then the token's low nibble.
        let (lo, hi) = match (src.get(i), src.get(i + 1)) {
            (Some(&lo), Some(&hi)) => (lo, hi),
            _ => return false,
        };
        i += 2;
        let offset = usize::from(u16::from_le_bytes([lo, hi]));
        if offset == 0 || offset > out.len() {
            return false;
        }
        let mut match_len = usize::from(token & 0x0f);
        if match_len == 15 && !read_lsic(src, &mut i, &mut match_len) {
            return false;
        }
        match_len += 4; // LZ4 MINMATCH: the low nibble stores length - 4.

        // Byte at a time on purpose: an LZ4 match is allowed to overlap the bytes it
        // is producing (offset < match_len), so a bulk copy would be wrong.
        let start = out.len() - offset;
        for from in start..start + match_len {
            if out.len() >= limit {
                return true;
            }
            let b = match out.get(from) {
                Some(&b) => b,
                None => return false,
            };
            out.push(b);
        }
    }
    true
}

/// LZ4's linear small-integer code: add each byte to `acc` until one is not 255.
fn read_lsic(src: &[u8], i: &mut usize, acc: &mut usize) -> bool {
    loop {
        let b = match src.get(*i) {
            Some(&b) => b,
            None => return false,
        };
        *i += 1;
        *acc = match acc.checked_add(usize::from(b)) {
            Some(v) => v,
            None => return false,
        };
        if b != 255 {
            return true;
        }
    }
}

/// A bounds-checked cursor over an in-memory slice. Every accessor returns
/// `Truncated` instead of panicking, which is the entire point: a save the game is
/// still writing is a normal thing to be handed.
struct Cur<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Cur { buf, pos: 0 }
    }

    fn take(&mut self, n: usize, field: &'static str) -> Result<&'a [u8], SaveParseError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(SaveParseError::Truncated(field))?;
        let slice = self
            .buf
            .get(self.pos..end)
            .ok_or(SaveParseError::Truncated(field))?;
        self.pos = end;
        Ok(slice)
    }

    fn skip(&mut self, n: usize, field: &'static str) -> Result<(), SaveParseError> {
        self.take(n, field).map(|_| ())
    }

    fn u8(&mut self, field: &'static str) -> Result<u8, SaveParseError> {
        let b: [u8; 1] = self.fixed(field)?;
        Ok(b[0])
    }

    fn u16(&mut self, field: &'static str) -> Result<u16, SaveParseError> {
        Ok(u16::from_le_bytes(self.fixed(field)?))
    }

    fn u32(&mut self, field: &'static str) -> Result<u32, SaveParseError> {
        Ok(u32::from_le_bytes(self.fixed(field)?))
    }

    fn u64(&mut self, field: &'static str) -> Result<u64, SaveParseError> {
        Ok(u64::from_le_bytes(self.fixed(field)?))
    }

    /// `take` into a fixed-size array, so the integer readers have no unwrap and
    /// therefore no panic site at all.
    fn fixed<const N: usize>(&mut self, field: &'static str) -> Result<[u8; N], SaveParseError> {
        self.take(N, field)?
            .try_into()
            .map_err(|_| SaveParseError::Truncated(field))
    }

    /// A `wstring`: u16 byte length, then that many bytes. Skyrim writes UTF-8, but
    /// a mod-authored plugin name can be anything, so decode lossily rather than
    /// failing the whole save over one bad byte. Trailing NULs are dropped - some
    /// writers include the terminator in the length.
    fn wstring(&mut self, field: &'static str) -> Result<String, SaveParseError> {
        let len = usize::from(self.u16(field)?);
        let bytes = self.take(len, field)?;
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }
}

/// One plugin in the profile's current load order, as [`missing_plugins`] needs to
/// see it. Borrowed strings so the caller can build the slice straight off its own
/// plugin list without cloning; keeping it a plain type is what lets this crate stay
/// free of a dependency on the plugin list itself.
#[derive(Debug, Clone, Copy)]
pub struct KnownPlugin<'a> {
    /// File name, e.g. `Skyrim.esm`.
    pub name: &'a str,
    pub enabled: bool,
    /// The mod folder providing it; empty for the game's own Data directory.
    pub origin_mod: &'a str,
}

/// A mod folder to search for a plugin the load order has never heard of. The caller
/// passes EVERY mod - disabled ones included, since a plugin from a disabled mod is
/// absent from the load order entirely - plus the Overwrite folder, in priority
/// order (the order providers are reported in).
#[derive(Debug, Clone, Copy)]
pub struct ModFolder<'a> {
    pub name: &'a str,
    pub path: &'a Path,
}

/// Why a save's plugin will not load in the current profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavePluginState {
    /// Present in the load order but not active - a checkbox away from correct.
    Inactive,
    /// Not in the load order at all: the owning mod is disabled or gone.
    Absent,
}

/// A plugin the save was created with that the current profile will not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingPlugin {
    pub name: String,
    pub state: SavePluginState,
    /// Mods that hold a file of this name, i.e. what to enable to fix the save.
    /// Empty means nothing in the instance provides it and the user has to find the
    /// mod again.
    pub providers: Vec<String>,
}

/// Diff a save's plugin list against the profile's current one.
///
/// Port of `GamebryoSaveGameInfo::getMissingAssets`
/// (`libs/game_bethesda/src/gamebryo/gamebryosavegameinfo.cpp:19-93`): classify each
/// of the save's plugins as inactive or absent, then scan every mod folder for a
/// `.esp`/`.esl`/`.esm` of that name to name the mods that could supply it. An empty
/// result means the save is safe to load.
///
/// `data_dir` is the game's own Data directory, if the caller lists it among `mods`.
/// It is pruned exactly as MO2 prunes it (gamebryosavegameinfo.cpp:63-70): Data
/// holds every unmanaged plugin, so without the guard it would be offered as the
/// provider of everything.
///
/// Name matching is ASCII case-insensitive. Bethesda plugin names come from a
/// case-insensitive filesystem, and Eidos runs on one that is not, so `Skyrim.esm`
/// in a save and `skyrim.esm` on disk are the same plugin and must not be reported
/// as missing.
pub fn missing_plugins(
    info: &SaveInfo,
    known: &[KnownPlugin<'_>],
    mods: &[ModFolder<'_>],
    data_dir: Option<&Path>,
) -> Vec<MissingPlugin> {
    let mut by_name: HashMap<String, KnownPlugin<'_>> = HashMap::with_capacity(known.len());
    for k in known {
        by_name.insert(k.name.to_ascii_lowercase(), *k);
    }

    let mut out: Vec<MissingPlugin> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for name in info.all_plugins() {
        let key = name.to_ascii_lowercase();
        if index.contains_key(&key) {
            continue; // a save can list the same name in both tables
        }
        let (state, providers) = match by_name.get(&key) {
            Some(k) if k.enabled => continue, // active: nothing to fix
            // An inactive plugin already tells us which mod owns it, so seed the
            // provider list with that mod rather than making the scan below find it.
            Some(k) if !k.origin_mod.is_empty() => {
                (SavePluginState::Inactive, vec![k.origin_mod.to_string()])
            }
            Some(_) => (SavePluginState::Inactive, Vec::new()),
            None => (SavePluginState::Absent, Vec::new()),
        };
        index.insert(key, out.len());
        out.push(MissingPlugin {
            name: name.to_string(),
            state,
            providers,
        });
    }

    // Nothing to fix: do not stat a single mod folder. The common case by far, and
    // this runs once per selected save.
    if out.is_empty() {
        return out;
    }

    // Resolve once: a mod folder is routinely a symlink here, and the Data-dir guard
    // below is a path comparison that has to see through that.
    let data_real = data_dir.map(|d| fs::canonicalize(d).unwrap_or_else(|_| d.to_path_buf()));
    for m in mods {
        let real = fs::canonicalize(m.path).unwrap_or_else(|_| m.path.to_path_buf());
        let is_data_dir = data_real.as_deref() == Some(real.as_path());
        // A mod folder that vanished between the list and this scan is not an error.
        let Ok(entries) = fs::read_dir(&real) else {
            continue;
        };
        for entry in entries.flatten() {
            // Symlinks count: Eidos mod folders are frequently linked into place.
            let is_file = entry
                .file_type()
                .map(|t| t.is_file() || t.is_symlink())
                .unwrap_or(false);
            if !is_file {
                continue;
            }
            let Ok(file_name) = entry.file_name().into_string() else {
                continue;
            };
            if !is_plugin_file(&file_name) {
                continue;
            }
            let key = file_name.to_ascii_lowercase();
            let Some(&i) = index.get(&key) else { continue };
            if is_data_dir && by_name.get(&key).map(|k| k.origin_mod) != Some(m.name) {
                // MO2's guard: inside Data, only the mod that actually owns the
                // plugin may claim it, or every unmanaged mod becomes a candidate.
                continue;
            }
            let providers = &mut out[i].providers;
            if !providers.iter().any(|p| p == m.name) {
                providers.push(m.name.to_string());
            }
        }
    }
    out
}

/// Whether a file name is a Bethesda plugin, case-insensitively.
fn is_plugin_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".esp") || lower.ends_with(".esl") || lower.ends_with(".esm")
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_screenshot_is_expanded_to_rgba_whatever_the_engine_stored() {
        // The LE engine writes 3 bytes per pixel and SE/FO4 write 4. Callers get
        // one shape, so the expansion happens here rather than at every use.
        let dir = std::env::temp_dir().join(format!("eidos-shot-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);

        // A hand-built header is not worth it; instead check the arithmetic and
        // the guards directly through a synthesised SaveInfo over a raw file.
        let path = dir.join("fake.ess");
        // 2x1 pixels, 3 bytes each, at offset 4.
        fs::write(&path, [0u8, 0, 0, 0, 10, 20, 30, 40, 50, 60]).unwrap();
        let mut info = SaveInfo {
            engine: SaveEngine::Skyrim,
            ..Default::default()
        };
        info.screenshot_width = 2;
        info.screenshot_height = 1;
        info.screenshot_offset = 4;
        info.screenshot_bytes_per_pixel = 3;
        let shot = read_screenshot_with(&path, &info).unwrap();
        assert_eq!(shot.rgba, vec![10, 20, 30, 0xFF, 40, 50, 60, 0xFF]);

        // Four-byte pixels pass straight through.
        fs::write(&path, [0u8, 0, 0, 0, 1, 2, 3, 4]).unwrap();
        info.screenshot_width = 1;
        info.screenshot_bytes_per_pixel = 4;
        assert_eq!(
            read_screenshot_with(&path, &info).unwrap().rgba,
            vec![1, 2, 3, 4]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_hostile_screenshot_size_is_refused_rather_than_allocated() {
        let dir = std::env::temp_dir().join(format!("eidos-shot-bad-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("fake.ess");
        fs::write(&path, [0u8; 16]).unwrap();

        // The dimensions are two u32s straight out of an attacker-visible
        // header: multiplied out they would ask for an allocation the size of
        // the product.
        let mut info = SaveInfo {
            engine: SaveEngine::Skyrim,
            ..Default::default()
        };
        info.screenshot_offset = 0;
        info.screenshot_bytes_per_pixel = 4;
        info.screenshot_width = u32::MAX;
        info.screenshot_height = u32::MAX;
        assert!(read_screenshot_with(&path, &info).is_err());

        // Zero is not a screenshot either.
        info.screenshot_width = 0;
        info.screenshot_height = 0;
        assert!(read_screenshot_with(&path, &info).is_err());

        // And a plausible size the file cannot actually satisfy is Truncated,
        // not a panic or a short buffer.
        info.screenshot_width = 64;
        info.screenshot_height = 64;
        assert!(matches!(
            read_screenshot_with(&path, &info),
            Err(SaveParseError::Truncated("screenshot"))
        ));
        let _ = fs::remove_dir_all(&dir);
    }
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    /// Byte every synthetic screenshot is filled with. Distinctive so a test can
    /// find the screenshot in the built file and locate the header fields that sit
    /// immediately in front of it.
    const SHOT_FILL: u8 = 0xab;

    fn tmp(ext: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "eidos-save-{}-{}.{ext}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn tmp_dir() -> PathBuf {
        let d = tmp("d");
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn ws(s: &str) -> Vec<u8> {
        let b = s.as_bytes();
        let mut v = (b.len() as u16).to_le_bytes().to_vec();
        v.extend_from_slice(b);
        v
    }

    /// The payload that lives behind the (optional) compression: form version,
    /// plugin info size, then the two tables.
    ///
    /// `game_version` is `Some` for a Fallout 4 payload - the extra string the
    /// engine writes there - and `None` for Skyrim. On Fallout the plugin info
    /// size is emitted for real, because that is what the parser bounds the ESL
    /// table with; Skyrim ignores the field, so a zero keeps the old behaviour.
    fn payload(
        form_version: u8,
        game_version: Option<&str>,
        plugins: &[&str],
        light: &[&str],
    ) -> Vec<u8> {
        let mut p = vec![form_version];
        if let Some(gv) = game_version {
            p.extend_from_slice(&ws(gv));
        }
        let mut block = vec![plugins.len() as u8];
        for name in plugins {
            block.extend_from_slice(&ws(name));
        }
        let want_light = match game_version {
            // Fallout: the block simply ends after the normal table when there is
            // no ESL list, which is exactly what the size then says.
            Some(_) => !light.is_empty(),
            None => form_version >= LIGHT_PLUGIN_FORM_VERSION,
        };
        if want_light {
            block.extend_from_slice(&(light.len() as u16).to_le_bytes());
            for name in light {
                block.extend_from_slice(&ws(name));
            }
        }
        let size = if game_version.is_some() {
            block.len() as u32
        } else {
            0
        };
        p.extend_from_slice(&size.to_le_bytes());
        p.extend_from_slice(&block);
        p
    }

    /// An LZ4 block made entirely of literals: valid per the block format (the last
    /// sequence carries no match) and enough to exercise the decoder's literal path.
    fn lz4_literal_block(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if data.len() < 15 {
            out.push((data.len() as u8) << 4);
        } else {
            out.push(0xf0);
            let mut rest = data.len() - 15;
            while rest >= 255 {
                out.push(255);
                rest -= 255;
            }
            out.push(rest as u8);
        }
        out.extend_from_slice(data);
        out
    }

    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    struct Build {
        version: u32,
        compression: u16,
        form_version: u8,
        plugins: Vec<String>,
        light: Vec<String>,
        magic: Vec<u8>,
        /// The in-game clock string. Skyrim's terse form by default; Fallout's is
        /// localised, which is why the fixture can be given either.
        game_date: String,
        /// `Some` builds a Fallout 4 save: shorter magic, no compression word,
        /// 4-byte pixels, and the extra `gameVersion` string in the payload.
        game_version: Option<String>,
        /// Replaces the generated payload, for tests that need a malformed table.
        payload_override: Option<Vec<u8>>,
    }

    impl Build {
        fn new() -> Self {
            Build {
                version: SE_HEADER_VERSION,
                compression: 2,
                form_version: 82,
                plugins: ["Skyrim.esm", "Update.esm", "MyMod.esp"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                light: vec!["Tiny.esl".to_string()],
                magic: MAGIC.to_vec(),
                game_date: "196.21.31".to_string(),
                game_version: None,
                payload_override: None,
            }
        }

        /// The same save as written by Fallout 4: uncompressed, `FO4_SAVEGAME`,
        /// and a `gameVersion` the parser must consume before the block size.
        fn fallout4() -> Self {
            Build {
                version: 15,
                compression: 0,
                form_version: 68,
                plugins: ["Fallout4.esm", "DLCRobot.esm", "MyMod.esp"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                light: vec!["Tiny.esl".to_string()],
                magic: FO4_MAGIC.to_vec(),
                // What a French Fallout 4 actually writes: compact form, then the
                // same numbers spelled out.
                game_date: "0j.22h.20m.0 jours.22 heures.20 minutes".to_string(),
                game_version: Some("1.10.163.0".to_string()),
                payload_override: None,
            }
        }

        fn bytes(&self) -> Vec<u8> {
            let plugins: Vec<&str> = self.plugins.iter().map(String::as_str).collect();
            let light: Vec<&str> = self.light.iter().map(String::as_str).collect();
            let inner = match &self.payload_override {
                Some(p) => p.clone(),
                None => payload(
                    self.form_version,
                    self.game_version.as_deref(),
                    &plugins,
                    &light,
                ),
            };

            let mut header = Vec::new();
            header.extend_from_slice(&self.version.to_le_bytes());
            header.extend_from_slice(&7u32.to_le_bytes()); // saveNumber
            header.extend_from_slice(&ws("Lyra"));
            header.extend_from_slice(&42u32.to_le_bytes()); // level
            header.extend_from_slice(&ws("Whiterun"));
            header.extend_from_slice(&ws(&self.game_date));
            header.extend_from_slice(&ws("BretonRace"));
            header.extend_from_slice(&0u16.to_le_bytes()); // sex
            header.extend_from_slice(&0f32.to_le_bytes());
            header.extend_from_slice(&0f32.to_le_bytes());
            // 2024-01-01T00:00:00Z as a FILETIME, verbatim - the engines apply no
            // offset, so neither may the fixture (it used to add one and thereby
            // assert the parser's own error back at itself).
            let ft = (1_704_067_200u64 + 11_644_473_600u64) * 10_000_000;
            header.extend_from_slice(&ft.to_le_bytes());
            header.extend_from_slice(&2u32.to_le_bytes()); // shot width
            header.extend_from_slice(&2u32.to_le_bytes()); // shot height
                                                           // Fallout has no compressionType word at all; Skyrim SE alone does.
            if self.game_version.is_none() && self.version == SE_HEADER_VERSION {
                header.extend_from_slice(&self.compression.to_le_bytes());
            }

            let mut save = self.magic.clone();
            save.extend_from_slice(&(header.len() as u32).to_le_bytes());
            save.extend_from_slice(&header);
            let bpp = if self.game_version.is_some() || self.version == SE_HEADER_VERSION {
                4
            } else {
                3
            };
            save.extend_from_slice(&vec![SHOT_FILL; 2 * 2 * bpp]);

            match self.compression {
                0 => save.extend_from_slice(&inner),
                1 | 2 => {
                    let blob = if self.compression == 1 {
                        zlib(&inner)
                    } else {
                        lz4_literal_block(&inner)
                    };
                    save.extend_from_slice(&(inner.len() as u32).to_le_bytes());
                    save.extend_from_slice(&(blob.len() as u32).to_le_bytes());
                    save.extend_from_slice(&blob);
                }
                _ => unreachable!(),
            }
            save
        }

        fn write(&self) -> PathBuf {
            let p = tmp(if self.game_version.is_some() {
                "fos"
            } else {
                "ess"
            });
            fs::write(&p, self.bytes()).unwrap();
            p
        }
    }

    #[test]
    fn a_fallout4_save_parses_with_its_own_magic_and_game_version() {
        let p = Build::fallout4().write();
        let info = parse_save(&p).unwrap();
        assert_eq!(info.engine, SaveEngine::Fallout4);
        // The whole point: the 12-byte magic and the extra gameVersion string are
        // consumed exactly, so every later field lands where it should. A one-byte
        // slip here shows up as garbage names, not as an error.
        assert_eq!(info.game_version, "1.10.163.0");
        assert_eq!(info.player_name, "Lyra");
        assert_eq!(info.level, 42);
        assert_eq!(info.location, "Whiterun");
        assert_eq!(info.save_number, 7);
        assert_eq!(info.plugins, ["Fallout4.esm", "DLCRobot.esm", "MyMod.esp"]);
        assert_eq!(info.light_plugins, ["Tiny.esl"]);
        // Fallout never compresses and always has an alpha channel, unlike the
        // LE engine it shares "no compressionType word" with.
        assert_eq!(info.compression, SaveCompression::None);
        assert!(!info.truncated);
    }

    #[test]
    fn a_skyrim_save_reports_no_game_version() {
        // The field is Fallout-only: reading one on Skyrim would mean the parser
        // had consumed four bytes of the plugin block as a string length.
        let info = parse_save(&Build::new().write()).unwrap();
        assert_eq!(info.engine, SaveEngine::Skyrim);
        assert_eq!(info.game_version, "");
    }

    #[test]
    fn a_fallout4_save_without_esls_reads_no_light_table() {
        // formVersion 68 is well under Skyrim's 78 threshold, so the ESL block on
        // Fallout is decided by the block SIZE. With no ESLs the size stops right
        // after the normal table, and nothing further may be consumed - reading a
        // phantom u16 there would invent light plugins out of the next record.
        let mut b = Build::fallout4();
        b.light.clear();
        let info = parse_save(&b.write()).unwrap();
        assert_eq!(info.plugins, ["Fallout4.esm", "DLCRobot.esm", "MyMod.esp"]);
        assert!(info.light_plugins.is_empty());
        assert!(
            !info.truncated,
            "an absent ESL block is not a truncated one"
        );
    }

    #[test]
    fn a_fallout4_esl_block_is_read_even_at_a_form_version_skyrim_would_reject() {
        // Guards the actual defect this replaced: gating Fallout on Skyrim's
        // formVersion >= 78 silently dropped every ESL, because real saves report
        // 68. The 22 ESLs of a real load order would have vanished from the diff.
        let mut b = Build::fallout4();
        assert!(
            b.form_version < LIGHT_PLUGIN_FORM_VERSION,
            "the premise of this test"
        );
        b.light = ["A.esl", "B.esm", "C.esp"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let info = parse_save(&b.write()).unwrap();
        // Extensions are meaningless here - the ESL flag lives in the plugin
        // header, so an .esm and an .esp legitimately appear in the light table.
        assert_eq!(info.light_plugins, ["A.esl", "B.esm", "C.esp"]);
    }

    #[test]
    fn a_fallout4_playtime_survives_its_localised_suffixes() {
        // A French save writes "0j.22h.20m.0 jours.22 heures.20 minutes"; the
        // numeric parse must not choke on the units or on the spelled-out tail.
        let mut info = SaveInfo {
            engine: SaveEngine::Fallout4,
            ..SaveInfo::default()
        };
        info.game_date = "0j.22h.20m.0 jours.22 heures.20 minutes".to_string();
        assert_eq!(info.playtime(), Some((0, 22, 20)));
        info.game_date = "12d.7h.5m.12 days.7 hours.5 minutes".to_string();
        assert_eq!(info.playtime(), Some((12, 7, 5)));
        // Skyrim's terse form still works, on either engine.
        info.game_date = "196.21.31".to_string();
        assert_eq!(info.playtime(), Some((196, 21, 31)));
        // A segment that is not a number at all stays refused rather than zeroed.
        info.game_date = "many.hours.here".to_string();
        assert_eq!(info.playtime(), None);
    }

    #[test]
    fn a_skyrim_save_with_a_fourth_segment_is_still_refused() {
        // The old strictness must survive for Skyrim: an unexpected 4th segment
        // means a format this parser does not know, not two thirds of an answer.
        let mut info = SaveInfo::default();
        info.game_date = "1.2.3.4".to_string();
        assert_eq!(info.playtime(), None);
    }

    #[test]
    fn a_lying_light_count_cannot_read_past_the_announced_block() {
        // What `pluginInfoSize` buys, precisely: the ESL *count* is file-controlled
        // too, and a save that claims more ESLs than its block holds must not go
        // fishing in the records that follow. Everything after a Fallout plugin
        // table is a binary form graph, and decoding it as names yields garbage
        // that a missing-plugin diff would then present as real missing mods.
        //
        // (An OVERSTATED size is a different matter and deliberately not tested
        // here: if the file lies about where the block ends, bytes past the real
        // table are inside the announced one by definition. The guarantee is that
        // no read leaves the announced block - never that a lying file is repaired.)
        let mut b = Build::fallout4();
        let gv = "1.10.163.0";
        let mut inner = vec![68u8];
        inner.extend_from_slice(&ws(gv));
        let mut block = vec![1u8]; // one normal plugin
        block.extend_from_slice(&ws("Fallout4.esm"));
        // Claims four ESLs and holds exactly one. The three that follow live
        // OUTSIDE the block - enough to satisfy the count, which is what makes
        // this dangerous: without the clamp the read succeeds and the phantoms
        // are reported as real, with truncated=false to vouch for them.
        block.extend_from_slice(&4u16.to_le_bytes());
        block.extend_from_slice(&ws("Real.esl"));
        inner.extend_from_slice(&(block.len() as u32).to_le_bytes());
        inner.extend_from_slice(&block);
        // Bytes beyond the block that would parse as more names if anything read
        // them - the stand-in for the form graph a real save has here.
        for name in ["Ghost.esl", "Phantom.esl", "Wraith.esl"] {
            inner.extend_from_slice(&ws(name));
        }
        b.payload_override = Some(inner);
        let p = b.write();
        let info = parse_save(&p).unwrap();
        assert_eq!(info.plugins, ["Fallout4.esm"]);
        assert!(
            !info
                .light_plugins
                .iter()
                .any(|n| n.starts_with("Ghost") || n.starts_with("Phantom")),
            "read past the announced block: {:?}",
            info.light_plugins
        );
        assert!(
            info.truncated,
            "a count that outruns its block makes the list advisory"
        );
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn the_header_timestamp_is_taken_verbatim() {
        // Anchored on a value chosen OUTSIDE the parser, which is the whole point:
        // the previous fixture added the same six-hour offset the parser then
        // subtracted, so a wrong constant round-tripped cleanly and the error only
        // showed up against real saves (53 of 53 disagreed with their own
        // filenames). Both engines write the timestamp with no correction.
        for save in [Build::new().write(), Build::fallout4().write()] {
            let info = parse_save(&save).unwrap();
            assert_eq!(
                info.created_unix,
                Some(1_704_067_200),
                "2024-01-01T00:00:00Z must survive the round trip unshifted"
            );
            let _ = fs::remove_file(&save);
        }
    }

    #[test]
    fn a_fallout4_localised_playtime_survives_a_full_parse() {
        // The localised string has to make it through the real byte path, not just
        // through a hand-built SaveInfo: it is a header wstring like any other.
        let mut b = Build::fallout4();
        b.game_date = "3j.4h.5m.3 jours.4 heures.5 minutes".to_string();
        let p = b.write();
        let info = parse_save(&p).unwrap();
        assert_eq!(info.playtime(), Some((3, 4, 5)));
        assert_eq!(info.game_date, "3j.4h.5m.3 jours.4 heures.5 minutes");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn a_garbled_skyrim_playtime_is_still_refused() {
        // The Fallout leniency must not leak onto Skyrim: "12abc" is not 12 there.
        let mut info = SaveInfo::default();
        info.game_date = "12abc.5.3".to_string();
        assert_eq!(info.playtime(), None, "Skyrim's field is strict integers");
        info.engine = SaveEngine::Fallout4;
        assert_eq!(
            info.playtime(),
            Some((12, 5, 3)),
            "Fallout writes unit suffixes"
        );
    }

    #[test]
    fn a_fallout_save_is_recognised_by_magic_not_by_header_version() {
        // Header version 12 is Skyrim SE's marker. A Fallout save that happens to
        // carry it must still be read as Fallout - no compression word, 4-byte
        // pixels - because the magic is what decides.
        let mut b = Build::fallout4();
        b.version = SE_HEADER_VERSION;
        let p = b.write();
        let info = parse_save(&p).unwrap();
        assert_eq!(info.engine, SaveEngine::Fallout4);
        assert_eq!(info.compression, SaveCompression::None);
        assert_eq!(info.plugins, ["Fallout4.esm", "DLCRobot.esm", "MyMod.esp"]);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn a_file_with_neither_magic_is_not_a_save_of_either_engine() {
        let mut b = Build::new();
        b.magic = b"FO4_SAVEGAMX".to_vec(); // one byte off the Fallout magic
        assert!(matches!(
            parse_save(&b.write()),
            Err(SaveParseError::NotASave)
        ));
    }

    #[test]
    fn well_formed_lz4_save_yields_header_and_both_plugin_tables() {
        let p = Build::new().write();
        let info = parse_save(&p).unwrap();
        assert_eq!(info.player_name, "Lyra");
        assert_eq!(info.level, 42);
        assert_eq!(info.location, "Whiterun");
        assert_eq!(info.race, "BretonRace");
        assert_eq!(info.save_number, 7);
        assert_eq!(info.compression, SaveCompression::Lz4);
        assert_eq!(info.screenshot_width, 2);
        assert_eq!(info.plugins, ["Skyrim.esm", "Update.esm", "MyMod.esp"]);
        assert_eq!(info.light_plugins, ["Tiny.esl"]);
        assert_eq!(info.playtime(), Some((196, 21, 31)));
        assert_eq!(info.created_unix, Some(1_704_067_200));
        assert!(!info.truncated);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn zlib_and_uncompressed_payloads_parse_the_same_table() {
        for (compression, want) in [(1u16, SaveCompression::Zlib), (0, SaveCompression::None)] {
            let mut b = Build::new();
            b.compression = compression;
            let p = b.write();
            let info = parse_save(&p).unwrap();
            assert_eq!(info.compression, want);
            assert_eq!(info.plugins, ["Skyrim.esm", "Update.esm", "MyMod.esp"]);
            assert_eq!(info.light_plugins, ["Tiny.esl"]);
            let _ = fs::remove_file(&p);
        }
    }

    #[test]
    fn light_block_is_only_read_when_form_version_allows_it() {
        let mut b = Build::new();
        b.form_version = 74; // pre-1.5.39: no ESL block at all
        let p = b.write();
        let info = parse_save(&p).unwrap();
        assert_eq!(info.plugins, ["Skyrim.esm", "Update.esm", "MyMod.esp"]);
        assert!(info.light_plugins.is_empty());
        assert!(
            !info.truncated,
            "a pre-78 save has no ESL block to be short of"
        );
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn le_engine_save_without_compression_word_still_parses() {
        let mut b = Build::new();
        b.version = 9; // LE: no compressionType, 3-byte screenshot pixels
        b.compression = 0;
        b.form_version = 74;
        let p = b.write();
        let info = parse_save(&p).unwrap();
        assert_eq!(info.header_version, 9);
        assert_eq!(info.compression, SaveCompression::None);
        assert_eq!(info.plugins, ["Skyrim.esm", "Update.esm", "MyMod.esp"]);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn a_light_block_that_runs_off_the_end_keeps_the_plugins_and_flags_the_save() {
        // The save the game is halfway through writing: the normal table landed, the
        // ESL block claims three names and only one is there.
        let mut inner = payload(82, None, &["Skyrim.esm"], &[]);
        inner.truncate(inner.len() - 2); // drop the lightPluginCount payload() wrote
        inner.extend_from_slice(&3u16.to_le_bytes());
        inner.extend_from_slice(&ws("Tiny.esl"));

        let mut b = Build::new();
        b.compression = 0;
        b.payload_override = Some(inner);
        let p = b.write();
        let info = parse_save(&p).unwrap();
        assert_eq!(info.plugins, ["Skyrim.esm"]);
        assert!(
            info.light_plugins.is_empty(),
            "a partial ESL block must not be half-trusted"
        );
        assert!(info.truncated, "the caller has to know this list is short");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn a_complete_save_is_never_flagged_truncated() {
        for compression in [0u16, 1, 2] {
            let mut b = Build::new();
            b.compression = compression;
            let p = b.write();
            assert!(
                !parse_save(&p).unwrap().truncated,
                "compression {compression}"
            );
            let _ = fs::remove_file(&p);
        }
    }

    #[test]
    fn empty_file_is_rejected_as_not_a_save() {
        let p = tmp("ess");
        fs::write(&p, b"").unwrap();
        assert!(matches!(parse_save(&p), Err(SaveParseError::NotASave)));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn bogus_magic_is_rejected_as_not_a_save() {
        let mut b = Build::new();
        b.magic = b"NOT_A_SAVEGAM".to_vec();
        let p = b.write();
        assert!(matches!(parse_save(&p), Err(SaveParseError::NotASave)));
        let _ = fs::remove_file(&p);

        // Shorter than the magic itself: same answer, no panic on the short slice.
        let q = tmp("ess");
        fs::write(&q, b"TESV").unwrap();
        assert!(matches!(parse_save(&q), Err(SaveParseError::NotASave)));
        let _ = fs::remove_file(&q);
    }

    #[test]
    fn every_truncation_of_a_valid_save_degrades_without_panicking() {
        // The exact case the audit calls out: the game is mid-write. Every prefix of
        // a good save must come back as Ok or Err, never a panic.
        // Both engines: Fallout's uncompressed path, shorter magic, gameVersion
        // string and size-bounded ESL block are all new places to fall over.
        let mut cases: Vec<Build> = Vec::new();
        for compression in [0u16, 1, 2] {
            let mut b = Build::new();
            b.compression = compression;
            cases.push(b);
        }
        cases.push(Build::fallout4());
        for b in cases {
            let full = b.bytes();
            for cut in 0..full.len() {
                let p = tmp("ess");
                fs::write(&p, &full[..cut]).unwrap();
                if let Ok(info) = parse_save(&p) {
                    // Whatever it managed to read must be self-consistent: a short
                    // read may only ever produce a prefix of the real table.
                    assert!(info.plugins.len() <= 3, "cut {cut} invented plugins");
                }
                let _ = fs::remove_file(&p);
            }
        }
    }

    /// Offset of the (16-byte, SE) screenshot inside a built save. The header fields
    /// this test module pokes all sit immediately in front of it: compressionType at
    /// -2, shotHeight at -6, shotWidth at -10.
    fn shot_offset(save: &[u8]) -> usize {
        save.windows(16)
            .position(|w| w.iter().all(|&b| b == SHOT_FILL))
            .expect("the synthetic screenshot must be findable")
    }

    #[test]
    fn corrupt_screenshot_size_is_refused_instead_of_allocating() {
        let mut full = Build::new().bytes();
        let shot = shot_offset(&full);
        // A 256M x 256M screenshot: the skip must be rejected against the file
        // length, not attempted.
        full[shot - 10..shot - 6].copy_from_slice(&0x1000_0000u32.to_le_bytes());
        full[shot - 6..shot - 2].copy_from_slice(&0x1000_0000u32.to_le_bytes());
        let p = tmp("ess");
        fs::write(&p, &full).unwrap();
        assert!(matches!(
            parse_save(&p),
            Err(SaveParseError::Truncated("screenshot"))
        ));
        let _ = fs::remove_file(&p);

        // And the pair whose product does not even fit in a u64.
        full[shot - 10..shot - 6].copy_from_slice(&u32::MAX.to_le_bytes());
        full[shot - 6..shot - 2].copy_from_slice(&u32::MAX.to_le_bytes());
        let q = tmp("ess");
        fs::write(&q, &full).unwrap();
        assert!(matches!(parse_save(&q), Err(SaveParseError::Corrupt(_))));
        let _ = fs::remove_file(&q);
    }

    #[test]
    fn unknown_compression_type_is_reported_not_guessed() {
        let mut full = Build::new().bytes();
        let shot = shot_offset(&full);
        full[shot - 2..shot].copy_from_slice(&9u16.to_le_bytes());
        let p = tmp("ess");
        fs::write(&p, &full).unwrap();
        assert!(matches!(
            parse_save(&p),
            Err(SaveParseError::UnknownCompression(9))
        ));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn corrupting_any_single_byte_never_panics() {
        // Truncation is one failure mode; a save damaged in place (a bad sector, a
        // half-flushed write) is the other. Every byte of a good save, set to each of
        // the values most likely to break a length or count field.
        for full in [Build::new().bytes(), Build::fallout4().bytes()] {
            for at in 0..full.len() {
                for value in [0x00u8, 0x01, 0x7f, 0xff] {
                    let mut bad = full.clone();
                    bad[at] = value;
                    let p = tmp("ess");
                    fs::write(&p, &bad).unwrap();
                    // The only requirement is that it returns. A corrupt length field may
                    // legitimately still parse into nonsense - it must not abort.
                    let _ = parse_save(&p);
                    let _ = fs::remove_file(&p);
                }
            }
        }
    }

    #[test]
    fn lz4_decoder_rejects_malformed_blocks_without_panicking() {
        let mut out = Vec::new();
        // A zero back-offset would make the match copy from out.len(), i.e. itself.
        assert!(!lz4_block_decompress(
            &[0x01, b'x', 0x00, 0x00],
            64,
            &mut out
        ));
        out.clear();
        // Offset past everything produced so far.
        assert!(!lz4_block_decompress(
            &[0x11, b'x', 0xff, 0x7f],
            64,
            &mut out
        ));
        out.clear();
        // Literal length runs off the end of the block.
        assert!(!lz4_block_decompress(&[0xf0, 0xff], 64, &mut out));
        out.clear();
        // Truncated mid-literal.
        assert!(!lz4_block_decompress(&[0x50, b'a', b'b'], 64, &mut out));
    }

    #[test]
    fn lz4_decoder_handles_overlapping_matches_and_the_output_cap() {
        // token 0x14: 1 literal, match length 4+4=8; offset 1 means the match reads
        // the byte it is producing, the classic run-length case.
        let mut out = Vec::new();
        assert!(lz4_block_decompress(
            &[0x14, b'z', 0x01, 0x00],
            64,
            &mut out
        ));
        assert_eq!(out, b"zzzzzzzzz");

        // The cap stops production dead rather than growing the buffer.
        let mut capped = Vec::new();
        assert!(lz4_block_decompress(
            &[0x14, b'z', 0x01, 0x00],
            3,
            &mut capped
        ));
        assert_eq!(capped.len(), 3);
    }

    #[test]
    fn playtime_only_parses_the_shape_skyrim_writes() {
        let mut info = SaveInfo {
            game_date: "196.21.31".into(),
            ..SaveInfo::default()
        };
        assert_eq!(info.playtime(), Some((196, 21, 31)));
        info.game_date = "Day 22 at 1:53pm".into();
        assert_eq!(info.playtime(), None);
        info.game_date = "1.2.3.4".into();
        assert_eq!(info.playtime(), None);
        info.game_date = String::new();
        assert_eq!(info.playtime(), None);
    }

    fn info_with(plugins: &[&str], light: &[&str]) -> SaveInfo {
        SaveInfo {
            plugins: plugins.iter().map(|s| s.to_string()).collect(),
            light_plugins: light.iter().map(|s| s.to_string()).collect(),
            ..SaveInfo::default()
        }
    }

    #[test]
    fn active_plugins_are_not_reported_and_case_does_not_matter() {
        let info = info_with(&["Skyrim.esm", "MyMod.esp"], &[]);
        let known = [
            KnownPlugin {
                name: "skyrim.esm",
                enabled: true,
                origin_mod: "",
            },
            KnownPlugin {
                name: "MYMOD.ESP",
                enabled: true,
                origin_mod: "My Mod",
            },
        ];
        assert!(missing_plugins(&info, &known, &[], None).is_empty());
    }

    #[test]
    fn inactive_plugin_is_seeded_with_its_owning_mod() {
        let info = info_with(&["MyMod.esp"], &[]);
        let known = [KnownPlugin {
            name: "MyMod.esp",
            enabled: false,
            origin_mod: "My Mod",
        }];
        let missing = missing_plugins(&info, &known, &[], None);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].state, SavePluginState::Inactive);
        assert_eq!(missing[0].providers, ["My Mod"]);
    }

    #[test]
    fn absent_plugin_is_traced_to_a_disabled_mod_folder() {
        let root = tmp_dir();
        let disabled = root.join("Some Disabled Mod");
        fs::create_dir_all(&disabled).unwrap();
        fs::write(disabled.join("Ghost.esp"), b"").unwrap();
        let empty = root.join("Unrelated");
        fs::create_dir_all(&empty).unwrap();
        fs::write(empty.join("readme.txt"), b"").unwrap();

        let info = info_with(&["Ghost.esp"], &["Nowhere.esl"]);
        let mods = [
            ModFolder {
                name: "Unrelated",
                path: &empty,
            },
            ModFolder {
                name: "Some Disabled Mod",
                path: &disabled,
            },
        ];
        let missing = missing_plugins(&info, &[], &mods, None);
        assert_eq!(missing.len(), 2);
        assert_eq!(missing[0].name, "Ghost.esp");
        assert_eq!(missing[0].state, SavePluginState::Absent);
        assert_eq!(missing[0].providers, ["Some Disabled Mod"]);
        // Nothing on disk holds it: the user has to go find the mod again.
        assert_eq!(missing[1].name, "Nowhere.esl");
        assert!(missing[1].providers.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn game_data_dir_is_pruned_unless_it_actually_owns_the_plugin() {
        let root = tmp_dir();
        let data = root.join("Data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("Ghost.esp"), b"").unwrap();
        fs::write(data.join("Owned.esp"), b"").unwrap();

        // Ghost.esp is unmanaged and unknown -> Data must not be offered for it.
        // Owned.esp is known, inactive, and its origin IS the Data entry -> allowed.
        let info = info_with(&["Ghost.esp", "Owned.esp"], &[]);
        let known = [KnownPlugin {
            name: "Owned.esp",
            enabled: false,
            origin_mod: "Data",
        }];
        let mods = [ModFolder {
            name: "Data",
            path: &data,
        }];
        let missing = missing_plugins(&info, &known, &mods, Some(&data));
        assert_eq!(missing.len(), 2);
        assert_eq!(missing[0].name, "Ghost.esp");
        assert!(
            missing[0].providers.is_empty(),
            "unmanaged Data plugin must not be offered"
        );
        assert_eq!(missing[1].name, "Owned.esp");
        assert_eq!(missing[1].providers, ["Data"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_provider_is_listed_once_even_across_both_tables() {
        let root = tmp_dir();
        let m = root.join("Big Mod");
        fs::create_dir_all(&m).unwrap();
        fs::write(m.join("A.esp"), b"").unwrap();
        fs::write(m.join("B.esl"), b"").unwrap();

        let info = info_with(&["A.esp", "A.esp"], &["B.esl"]);
        // The same mod listed twice: a provider must still appear exactly once.
        let big = ModFolder {
            name: "Big Mod",
            path: &m,
        };
        let mods = [big, big];
        let missing = missing_plugins(&info, &[], &mods, None);
        assert_eq!(missing.len(), 2, "the duplicate save entry must collapse");
        assert_eq!(missing[0].providers, ["Big Mod"]);
        assert_eq!(missing[1].providers, ["Big Mod"]);
        let _ = fs::remove_dir_all(&root);
    }
}

/// A save's embedded screenshot, as RGBA8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    /// Always four bytes per pixel, whatever the file stored - the LE engine's
    /// three-byte pixels are expanded here so callers have one shape to handle.
    pub rgba: Vec<u8>,
}

/// The largest screenshot this will decode, in pixels.
///
/// A save header is attacker-visible: the width and height are two u32s straight
/// out of the file, and a corrupt or hostile pair would otherwise ask for an
/// allocation the size of the multiplication. Real ones are around
/// 800x450 on SE.
const MAX_SHOT_PIXELS: u64 = 8 * 1024 * 1024;

/// Read a save's embedded screenshot.
///
/// Deliberately separate from [`parse_save`], and never called by it: the list
/// parses every save in a directory, and reading every screenshot would turn a
/// cheap three-window read into hundreds of megabytes of I/O for images nobody
/// has looked at yet. This is for the ONE save that is selected.
pub fn read_screenshot(path: &Path) -> Result<Screenshot, SaveParseError> {
    let info = parse_save(path)?;
    read_screenshot_with(path, &info)
}

/// [`read_screenshot`] when the header has already been parsed - which it has,
/// everywhere the GUI needs this.
pub fn read_screenshot_with(path: &Path, info: &SaveInfo) -> Result<Screenshot, SaveParseError> {
    let (w, h) = (info.screenshot_width, info.screenshot_height);
    let bpp = u64::from(info.screenshot_bytes_per_pixel);
    let pixels = u64::from(w)
        .checked_mul(u64::from(h))
        .ok_or(SaveParseError::Corrupt("screenshot dimensions overflow"))?;
    if pixels == 0 {
        return Err(SaveParseError::Corrupt("no screenshot"));
    }
    if pixels > MAX_SHOT_PIXELS {
        return Err(SaveParseError::Corrupt("screenshot too large to be real"));
    }
    let len = pixels
        .checked_mul(bpp)
        .ok_or(SaveParseError::Corrupt("screenshot dimensions overflow"))?;

    let mut file = fs::File::open(path)?;
    file.seek(SeekFrom::Start(info.screenshot_offset))?;
    let mut raw = vec![0u8; len as usize];
    file.read_exact(&mut raw)
        .map_err(|_| SaveParseError::Truncated("screenshot"))?;

    let rgba = match info.screenshot_bytes_per_pixel {
        4 => raw,
        // The LE engine stores RGB. Expanded here rather than at the call site
        // so every consumer sees one format.
        3 => {
            let mut out = Vec::with_capacity(pixels as usize * 4);
            for px in raw.as_chunks::<3>().0 {
                out.extend_from_slice(px);
                out.push(0xFF);
            }
            out
        }
        other => {
            return Err(SaveParseError::Corrupt(match other {
                0 => "screenshot has no pixel size",
                _ => "unknown screenshot pixel size",
            }))
        }
    };
    Ok(Screenshot {
        width: w,
        height: h,
        rgba,
    })
}
