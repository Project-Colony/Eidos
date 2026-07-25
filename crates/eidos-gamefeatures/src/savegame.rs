//! Skyrim SE save-game header parsing: who the save belongs to and - the part that
//! actually matters - the plugin list it was created with.
//!
//! Eidos keeps saves per profile, so a save routinely outlives the mod list that
//! produced it. A save that references a plugin the profile no longer loads is
//! precisely the setup that produces a mid-playthrough crash or a silently dead
//! quest, and the only way to warn about it is to read the save's own plugin table.
//! This is a port of MO2's `GamebryoSaveGame` / `GamebryoSaveGameInfo`
//! (`libs/game_bethesda`), narrowed to the one engine Eidos verifies against.
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

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// The 13-byte file id every Skyrim (LE and SE) save opens with.
const MAGIC: &[u8; 13] = b"TESV_SAVEGAME";

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

/// Six hours in 100 ns units. Skyrim writes a FILETIME that is offset by exactly
/// this much; MO2 subtracts it too (skyrimsesavegame.cpp:64-69) and we match, so the
/// timestamp Eidos shows agrees with the one the user sees in MO2 and in-game.
const FILETIME_SKEW_100NS: u64 = 216_000_000_000;

/// Seconds between the FILETIME epoch (1601-01-01) and the Unix epoch.
const FILETIME_TO_UNIX_SECS: i64 = 11_644_473_600;

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

/// Everything `parse_sse_save` extracts from a save header.
#[derive(Debug, Clone, Default)]
pub struct SaveInfo {
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
    /// Full-index plugins, in the save's own load order.
    pub plugins: Vec<String>,
    /// Light (ESL) plugins, present only from `formVersion` 78 on.
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
        self.plugins.iter().chain(self.light_plugins.iter()).map(String::as_str)
    }

    /// The in-game clock as `(days, hours, minutes)`.
    ///
    /// Skyrim writes `game_date` as three dot-separated integers. Anything else
    /// (a localised string from a different engine, a garbled field) yields `None`
    /// rather than a guess - the raw string is still available for display.
    pub fn playtime(&self) -> Option<(u32, u32, u32)> {
        let mut parts = self.game_date.split('.');
        let d = parts.next()?.trim().parse().ok()?;
        let h = parts.next()?.trim().parse().ok()?;
        let m = parts.next()?.trim().parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some((d, h, m))
    }
}

/// Why a save could not be read. Every variant is a normal outcome for a file that
/// the game may be writing right now, so none of them warrants more than degrading
/// the row back to name/date/size.
#[derive(Debug)]
pub enum SaveParseError {
    /// The file could not be opened, seeked or read.
    Io(io::Error),
    /// Not a Skyrim save at all: the `TESV_SAVEGAME` magic is missing or wrong.
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
            SaveParseError::NotASave => write!(f, "not a Skyrim save (bad TESV_SAVEGAME magic)"),
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

/// Parse the header of a Skyrim SE save, up to and including the plugin table.
///
/// Reads three bounded windows out of the file (fixed header, block sizes, first
/// slice of the payload) and stops - the object graph after the plugin table is
/// never touched, which is what makes this affordable to run over a whole saves
/// directory. Also handles LE-engine saves (`version` 9, uncompressed, 3-byte
/// screenshot pixels) and Enderal SE / Skyrim VR, which share this layout.
pub fn parse_sse_save(path: &Path) -> Result<SaveInfo, SaveParseError> {
    let mut file = fs::File::open(path)?;
    let file_len = file.metadata()?.len();

    let mut head = Vec::new();
    file.by_ref().take(HEADER_WINDOW).read_to_end(&mut head)?;
    let mut cur = Cur::new(&head);

    match cur.take(MAGIC.len(), "magic") {
        Ok(m) if m == MAGIC.as_slice() => {}
        // Covers the empty file and the "someone renamed a .txt" case alike: both
        // are "this is not a save", not "this save is broken".
        _ => return Err(SaveParseError::NotASave),
    }

    let mut info = SaveInfo::default();
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

    // SE is the only engine with a compressionType word, and the only one whose
    // screenshot carries an alpha channel. Both hang off the same version check.
    let (compression_raw, bytes_per_pixel) = if info.header_version == SE_HEADER_VERSION {
        (cur.u16("compressionType")?, 4u64)
    } else {
        (0, 3u64)
    };
    info.compression = match compression_raw {
        0 => SaveCompression::None,
        1 => SaveCompression::Zlib,
        2 => SaveCompression::Lz4,
        other => return Err(SaveParseError::UnknownCompression(other)),
    };

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
        file.by_ref().take(COMPRESSED_WINDOW).read_to_end(&mut raw)?;
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
    // MO2 reads this u32 as u8 + u16 + a 1-byte skip (skyrimsesavegame.cpp:145-149
    // plus readPlugins(1)); it is one field, the byte size of the plugin block, and
    // the count that follows is authoritative - so skip it rather than trust it.
    cur.skip(4, "pluginInfoSize")?;

    let count = cur.u8("pluginCount")?;
    info.plugins = read_names(&mut cur, usize::from(count), "plugin name")?;

    // The ESL block only exists from formVersion 78 on. Reading it unconditionally
    // on an older save would consume whatever record happens to follow the table.
    if info.form_version >= LIGHT_PLUGIN_FORM_VERSION {
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

/// Convert a Skyrim header FILETIME to a Unix timestamp, undoing the engine's
/// six-hour offset. `None` for a zero or pre-1970 value, so a blank field shows up
/// as "no timestamp" instead of a date in 1601.
fn filetime_to_unix(raw: u64) -> Option<i64> {
    let adjusted = raw.checked_sub(FILETIME_SKEW_100NS)?;
    let secs = i64::try_from(adjusted / 10_000_000).ok()?;
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
        let mut from = out.len() - offset;
        for _ in 0..match_len {
            if out.len() >= limit {
                return true;
            }
            let b = match out.get(from) {
                Some(&b) => b,
                None => return false,
            };
            out.push(b);
            from += 1;
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
        let end = self.pos.checked_add(n).ok_or(SaveParseError::Truncated(field))?;
        let slice = self.buf.get(self.pos..end).ok_or(SaveParseError::Truncated(field))?;
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
        self.take(N, field)?.try_into().map_err(|_| SaveParseError::Truncated(field))
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
        out.push(MissingPlugin { name: name.to_string(), state, providers });
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
        let Ok(entries) = fs::read_dir(&real) else { continue };
        for entry in entries.flatten() {
            // Symlinks count: Eidos mod folders are frequently linked into place.
            let is_file = entry.file_type().map(|t| t.is_file() || t.is_symlink()).unwrap_or(false);
            if !is_file {
                continue;
            }
            let Ok(file_name) = entry.file_name().into_string() else { continue };
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
    fn payload(form_version: u8, plugins: &[&str], light: &[&str]) -> Vec<u8> {
        let mut p = vec![form_version];
        p.extend_from_slice(&0u32.to_le_bytes()); // pluginInfoSize, unused by us
        p.push(plugins.len() as u8);
        for name in plugins {
            p.extend_from_slice(&ws(name));
        }
        if form_version >= LIGHT_PLUGIN_FORM_VERSION {
            p.extend_from_slice(&(light.len() as u16).to_le_bytes());
            for name in light {
                p.extend_from_slice(&ws(name));
            }
        }
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
                payload_override: None,
            }
        }

        fn bytes(&self) -> Vec<u8> {
            let plugins: Vec<&str> = self.plugins.iter().map(String::as_str).collect();
            let light: Vec<&str> = self.light.iter().map(String::as_str).collect();
            let inner = match &self.payload_override {
                Some(p) => p.clone(),
                None => payload(self.form_version, &plugins, &light),
            };

            let mut header = Vec::new();
            header.extend_from_slice(&self.version.to_le_bytes());
            header.extend_from_slice(&7u32.to_le_bytes()); // saveNumber
            header.extend_from_slice(&ws("Lyra"));
            header.extend_from_slice(&42u32.to_le_bytes()); // level
            header.extend_from_slice(&ws("Whiterun"));
            header.extend_from_slice(&ws("196.21.31"));
            header.extend_from_slice(&ws("BretonRace"));
            header.extend_from_slice(&0u16.to_le_bytes()); // sex
            header.extend_from_slice(&0f32.to_le_bytes());
            header.extend_from_slice(&0f32.to_le_bytes());
            // 2024-01-01T00:00:00Z as FILETIME, plus the 6h skew the engine adds.
            let ft = (1_704_067_200u64 + 11_644_473_600u64) * 10_000_000 + FILETIME_SKEW_100NS;
            header.extend_from_slice(&ft.to_le_bytes());
            header.extend_from_slice(&2u32.to_le_bytes()); // shot width
            header.extend_from_slice(&2u32.to_le_bytes()); // shot height
            if self.version == SE_HEADER_VERSION {
                header.extend_from_slice(&self.compression.to_le_bytes());
            }

            let mut save = self.magic.clone();
            save.extend_from_slice(&(header.len() as u32).to_le_bytes());
            save.extend_from_slice(&header);
            let bpp = if self.version == SE_HEADER_VERSION { 4 } else { 3 };
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
            let p = tmp("ess");
            fs::write(&p, self.bytes()).unwrap();
            p
        }
    }

    #[test]
    fn well_formed_lz4_save_yields_header_and_both_plugin_tables() {
        let p = Build::new().write();
        let info = parse_sse_save(&p).unwrap();
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
            let info = parse_sse_save(&p).unwrap();
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
        let info = parse_sse_save(&p).unwrap();
        assert_eq!(info.plugins, ["Skyrim.esm", "Update.esm", "MyMod.esp"]);
        assert!(info.light_plugins.is_empty());
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn le_engine_save_without_compression_word_still_parses() {
        let mut b = Build::new();
        b.version = 9; // LE: no compressionType, 3-byte screenshot pixels
        b.compression = 0;
        b.form_version = 74;
        let p = b.write();
        let info = parse_sse_save(&p).unwrap();
        assert_eq!(info.header_version, 9);
        assert_eq!(info.compression, SaveCompression::None);
        assert_eq!(info.plugins, ["Skyrim.esm", "Update.esm", "MyMod.esp"]);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn a_light_block_that_runs_off_the_end_keeps_the_plugins_and_flags_the_save() {
        // The save the game is halfway through writing: the normal table landed, the
        // ESL block claims three names and only one is there.
        let mut inner = payload(82, &["Skyrim.esm"], &[]);
        inner.truncate(inner.len() - 2); // drop the lightPluginCount payload() wrote
        inner.extend_from_slice(&3u16.to_le_bytes());
        inner.extend_from_slice(&ws("Tiny.esl"));

        let mut b = Build::new();
        b.compression = 0;
        b.payload_override = Some(inner);
        let p = b.write();
        let info = parse_sse_save(&p).unwrap();
        assert_eq!(info.plugins, ["Skyrim.esm"]);
        assert!(info.light_plugins.is_empty(), "a partial ESL block must not be half-trusted");
        assert!(info.truncated, "the caller has to know this list is short");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn a_complete_save_is_never_flagged_truncated() {
        for compression in [0u16, 1, 2] {
            let mut b = Build::new();
            b.compression = compression;
            let p = b.write();
            assert!(!parse_sse_save(&p).unwrap().truncated, "compression {compression}");
            let _ = fs::remove_file(&p);
        }
    }

    #[test]
    fn empty_file_is_rejected_as_not_a_save() {
        let p = tmp("ess");
        fs::write(&p, b"").unwrap();
        assert!(matches!(parse_sse_save(&p), Err(SaveParseError::NotASave)));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn bogus_magic_is_rejected_as_not_a_save() {
        let mut b = Build::new();
        b.magic = b"NOT_A_SAVEGAM".to_vec();
        let p = b.write();
        assert!(matches!(parse_sse_save(&p), Err(SaveParseError::NotASave)));
        let _ = fs::remove_file(&p);

        // Shorter than the magic itself: same answer, no panic on the short slice.
        let q = tmp("ess");
        fs::write(&q, b"TESV").unwrap();
        assert!(matches!(parse_sse_save(&q), Err(SaveParseError::NotASave)));
        let _ = fs::remove_file(&q);
    }

    #[test]
    fn every_truncation_of_a_valid_save_degrades_without_panicking() {
        // The exact case the audit calls out: the game is mid-write. Every prefix of
        // a good save must come back as Ok or Err, never a panic.
        for compression in [0u16, 1, 2] {
            let mut b = Build::new();
            b.compression = compression;
            let full = b.bytes();
            for cut in 0..full.len() {
                let p = tmp("ess");
                fs::write(&p, &full[..cut]).unwrap();
                if let Ok(info) = parse_sse_save(&p) {
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
        assert!(matches!(parse_sse_save(&p), Err(SaveParseError::Truncated("screenshot"))));
        let _ = fs::remove_file(&p);

        // And the pair whose product does not even fit in a u64.
        full[shot - 10..shot - 6].copy_from_slice(&u32::MAX.to_le_bytes());
        full[shot - 6..shot - 2].copy_from_slice(&u32::MAX.to_le_bytes());
        let q = tmp("ess");
        fs::write(&q, &full).unwrap();
        assert!(matches!(parse_sse_save(&q), Err(SaveParseError::Corrupt(_))));
        let _ = fs::remove_file(&q);
    }

    #[test]
    fn unknown_compression_type_is_reported_not_guessed() {
        let mut full = Build::new().bytes();
        let shot = shot_offset(&full);
        full[shot - 2..shot].copy_from_slice(&9u16.to_le_bytes());
        let p = tmp("ess");
        fs::write(&p, &full).unwrap();
        assert!(matches!(parse_sse_save(&p), Err(SaveParseError::UnknownCompression(9))));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn corrupting_any_single_byte_never_panics() {
        // Truncation is one failure mode; a save damaged in place (a bad sector, a
        // half-flushed write) is the other. Every byte of a good save, set to each of
        // the values most likely to break a length or count field.
        let full = Build::new().bytes();
        for at in 0..full.len() {
            for value in [0x00u8, 0x01, 0x7f, 0xff] {
                let mut bad = full.clone();
                bad[at] = value;
                let p = tmp("ess");
                fs::write(&p, &bad).unwrap();
                // The only requirement is that it returns. A corrupt length field may
                // legitimately still parse into nonsense - it must not abort.
                let _ = parse_sse_save(&p);
                let _ = fs::remove_file(&p);
            }
        }
    }

    #[test]
    fn lz4_decoder_rejects_malformed_blocks_without_panicking() {
        let mut out = Vec::new();
        // A zero back-offset would make the match copy from out.len(), i.e. itself.
        assert!(!lz4_block_decompress(&[0x01, b'x', 0x00, 0x00], 64, &mut out));
        out.clear();
        // Offset past everything produced so far.
        assert!(!lz4_block_decompress(&[0x11, b'x', 0xff, 0x7f], 64, &mut out));
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
        assert!(lz4_block_decompress(&[0x14, b'z', 0x01, 0x00], 64, &mut out));
        assert_eq!(out, b"zzzzzzzzz");

        // The cap stops production dead rather than growing the buffer.
        let mut capped = Vec::new();
        assert!(lz4_block_decompress(&[0x14, b'z', 0x01, 0x00], 3, &mut capped));
        assert_eq!(capped.len(), 3);
    }

    #[test]
    fn playtime_only_parses_the_shape_skyrim_writes() {
        let mut info = SaveInfo { game_date: "196.21.31".into(), ..SaveInfo::default() };
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
            KnownPlugin { name: "skyrim.esm", enabled: true, origin_mod: "" },
            KnownPlugin { name: "MYMOD.ESP", enabled: true, origin_mod: "My Mod" },
        ];
        assert!(missing_plugins(&info, &known, &[], None).is_empty());
    }

    #[test]
    fn inactive_plugin_is_seeded_with_its_owning_mod() {
        let info = info_with(&["MyMod.esp"], &[]);
        let known = [KnownPlugin { name: "MyMod.esp", enabled: false, origin_mod: "My Mod" }];
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
            ModFolder { name: "Unrelated", path: &empty },
            ModFolder { name: "Some Disabled Mod", path: &disabled },
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
        let known = [KnownPlugin { name: "Owned.esp", enabled: false, origin_mod: "Data" }];
        let mods = [ModFolder { name: "Data", path: &data }];
        let missing = missing_plugins(&info, &known, &mods, Some(&data));
        assert_eq!(missing.len(), 2);
        assert_eq!(missing[0].name, "Ghost.esp");
        assert!(missing[0].providers.is_empty(), "unmanaged Data plugin must not be offered");
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
        let big = ModFolder { name: "Big Mod", path: &m };
        let mods = [big, big];
        let missing = missing_plugins(&info, &[], &mods, None);
        assert_eq!(missing.len(), 2, "the duplicate save entry must collapse");
        assert_eq!(missing[0].providers, ["Big Mod"]);
        assert_eq!(missing[1].providers, ["Big Mod"]);
        let _ = fs::remove_dir_all(&root);
    }
}
