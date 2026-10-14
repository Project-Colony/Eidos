//! Bethesda archive directories and bounded, on-demand member extraction.
use crate::{ConflictMap, ConflictState, Layer, ModConflicts, OriginId, BASE_ORIGIN};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveArchive {
    pub name: String,
    pub plugin: Option<String>,
    pub order_uncertain: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetProvider {
    pub origin: OriginId,
    pub archive: Option<String>,
    pub plugin: Option<String>,
}
#[derive(Debug, Clone)]
pub struct AssetNode {
    pub winner: AssetProvider,
    pub alternatives: Vec<AssetProvider>,
    pub display_path: String,
    pub precedence_uncertain: bool,
}
impl AssetNode {
    pub fn is_conflicted(&self) -> bool {
        let origins: BTreeSet<_> = std::iter::once(&self.winner)
            .chain(&self.alternatives)
            .map(|p| p.origin)
            .filter(|&o| o != BASE_ORIGIN)
            .collect();
        origins.len() > 1
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveError {
    Io(String),
    Corrupt(String),
    Unsupported(String),
    Limit(String),
}
impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(s) => write!(f, "I/O: {s}"),
            Self::Corrupt(s) => write!(f, "Corrupt archive: {s}"),
            Self::Unsupported(s) => write!(f, "Unsupported archive: {s}"),
            Self::Limit(s) => write!(f, "Archive safety limit: {s}"),
        }
    }
}
impl std::error::Error for ArchiveError {}
impl From<io::Error> for ArchiveError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            Self::Corrupt("truncated archive data".into())
        } else {
            Self::Io(e.to_string())
        }
    }
}
#[derive(Debug, Clone)]
pub struct ArchiveDiagnostic {
    pub archive: String,
    pub origin: Option<OriginId>,
    pub error: ArchiveError,
}
type Result<T> = std::result::Result<T, ArchiveError>;
const MAX_ENTRIES: u64 = 2_000_000;
const MAX_DIRECTORY: u64 = 128 * 1024 * 1024;
const MAX_PATH: usize = 4096;
mod export;
mod payload;
pub use export::*;
pub use payload::*;
use payload::{Codec, Payload, Segment, Texture, TextureChunk};
fn corrupt(s: &str) -> ArchiveError {
    ArchiveError::Corrupt(s.into())
}
fn u32_at(b: &[u8], i: usize) -> u64 {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) as u64
}
fn u64_at(b: &[u8], i: usize) -> u64 {
    u64::from_le_bytes(b[i..i + 8].try_into().unwrap())
}
fn count(n: u64) -> Result<()> {
    if n > MAX_ENTRIES {
        Err(ArchiveError::Limit("too many directory entries".into()))
    } else {
        Ok(())
    }
}
struct Directory<R> {
    reader: R,
    len: u64,
    budget: u64,
    requested: Option<String>,
    selected: Option<(String, Payload)>,
}
impl<R: Read + Seek> Directory<R> {
    fn select(&mut self, name: &str, payload: Payload) {
        if self
            .requested
            .as_deref()
            .is_some_and(|s| s.eq_ignore_ascii_case(name))
        {
            self.selected = Some((name.to_owned(), payload));
        }
    }
    fn range(&self, offset: u64, size: u64) -> Result<()> {
        if offset > self.len || size > self.len - offset {
            Err(corrupt("offset or length outside archive"))
        } else {
            Ok(())
        }
    }
    fn read(&mut self, offset: u64, size: u64) -> Result<Vec<u8>> {
        self.range(offset, size)?;
        self.budget = self
            .budget
            .checked_sub(size)
            .ok_or_else(|| ArchiveError::Limit("directory exceeds 128 MiB".into()))?;
        let mut b = vec![0; size as usize];
        self.reader.seek(SeekFrom::Start(offset))?;
        self.reader.read_exact(&mut b)?;
        Ok(b)
    }
}
fn path(bytes: &[u8]) -> Result<String> {
    if bytes.is_empty() || bytes.len() > MAX_PATH {
        return Err(corrupt("empty or oversized member path"));
    }
    // Bethesda archive names use the Windows byte code page, not mandatory UTF-8.
    let s = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    };
    let s = s.replace('\\', "/");
    if s.starts_with('/')
        || s.split('/').any(|c| {
            c.is_empty()
                || c == "."
                || c == ".."
                || c.contains(':')
                || c.chars().any(char::is_control)
        })
    {
        return Err(corrupt("unsafe member path"));
    }
    Ok(s)
}
/// Reads only validated header, directory records and names. Supports TES3 BSA,
/// BSA 103/104/105, and BA2 1/2/3/7/8 GNRL/DX10. Returns no partial member list.
pub fn read_archive_members(path: &Path) -> Result<Vec<String>> {
    read_members(File::open(path)?)
}
fn read_members<R: Read + Seek>(reader: R) -> Result<Vec<String>> {
    Ok(parse_directory(reader, None)?.1)
}
fn parse_directory<R: Read + Seek>(
    mut reader: R,
    requested: Option<&str>,
) -> Result<(Directory<R>, Vec<String>)> {
    let len = reader.seek(SeekFrom::End(0))?;
    let mut d = Directory {
        reader,
        len,
        budget: MAX_DIRECTORY,
        requested: requested.map(|s| path(s.as_bytes())).transpose()?,
        selected: None,
    };
    let magic = d.read(0, 4)?;
    let members = match magic.as_slice() {
        b"BSA\0" => bsa(&mut d)?,
        b"BTDX" => ba2(&mut d)?,
        [0, 1, 0, 0] => tes3(&mut d)?,
        _ => return Err(ArchiveError::Unsupported("unknown signature".into())),
    };
    let mut seen = BTreeSet::new();
    for member in &members {
        if !seen.insert(member.to_ascii_lowercase()) {
            return Err(corrupt("duplicate case-insensitive member path"));
        }
    }
    Ok((d, members))
}
fn bsa<R: Read + Seek>(d: &mut Directory<R>) -> Result<Vec<String>> {
    let h = d.read(0, 36)?;
    let version = u32_at(&h, 4);
    if !matches!(version, 103..=105) {
        return Err(ArchiveError::Unsupported(format!("BSA version {version}")));
    }
    let flags = u32_at(&h, 12);
    if flags & 0x40 != 0 || flags & 3 != 3 {
        return Err(ArchiveError::Unsupported(
            "big-endian or omitted directory/file names".into(),
        ));
    }
    if u32_at(&h, 8) != 36 {
        return Err(corrupt("invalid folder table offset"));
    }
    let folders = u32_at(&h, 16);
    let files = u32_at(&h, 20);
    count(folders)?;
    count(files)?;
    let folder_names = u32_at(&h, 24);
    let file_names = u32_at(&h, 28);
    let record_size = if version == 105 { 24 } else { 16 };
    let table = d.read(36, folders * record_size)?;
    let blocks_start = 36 + folders * record_size;
    let blocks_len = folders + folder_names + files * 16;
    let blocks = d.read(blocks_start, blocks_len)?;
    let names_start = blocks_start + blocks_len;
    let names = d.read(names_start, file_names)?;
    let payload_start = names_start + file_names;
    let mut paths = Vec::new();
    let mut name_cursor = 0usize;
    let mut cursor = 0usize;
    for rec in table.chunks_exact(record_size as usize) {
        let n = u32_at(rec, 8);
        count(n)?;
        let stored_offset = if version == 105 {
            u64_at(rec, 16)
        } else {
            u32_at(rec, 12)
        };
        let offset = stored_offset
            .checked_sub(file_names)
            .ok_or_else(|| corrupt("folder offset underflow"))?;
        if offset != blocks_start + cursor as u64 {
            return Err(corrupt("overlapping or unordered folder directory"));
        }
        let length = *blocks
            .get(cursor)
            .ok_or_else(|| corrupt("missing folder name"))? as usize;
        cursor += 1;
        let raw = blocks
            .get(cursor..cursor + length)
            .ok_or_else(|| corrupt("folder name outside directory"))?;
        cursor += length;
        let raw = raw
            .strip_suffix(&[0])
            .ok_or_else(|| corrupt("unterminated folder name"))?;
        let folder = if raw.is_empty() {
            String::new()
        } else {
            path(raw)?
        };
        for _ in 0..n {
            let rec = blocks
                .get(cursor..cursor + 16)
                .ok_or_else(|| corrupt("file record outside directory"))?;
            cursor += 16;
            let size = u32_at(rec, 8) & 0x3fff_ffff;
            let offset = u32_at(rec, 12);
            d.range(offset, size)?;
            if offset < payload_start {
                return Err(corrupt("member overlaps directory"));
            }
            let tail = names
                .get(name_cursor..)
                .ok_or_else(|| corrupt("file name count mismatch"))?;
            let end = tail
                .iter()
                .position(|&b| b == 0)
                .ok_or_else(|| corrupt("unterminated file name"))?;
            let name = path(&tail[..end])?;
            name_cursor += end + 1;
            if name.contains('/') {
                return Err(corrupt("file name contains a directory"));
            }
            let name = if folder.is_empty() {
                name
            } else {
                format!("{folder}/{name}")
            };
            if name.len() > MAX_PATH {
                return Err(corrupt("oversized member path"));
            }
            d.select(
                &name,
                Payload::Bsa {
                    segment: Segment {
                        offset,
                        stored: size,
                        unpacked: size,
                        codec: if (flags & 4 != 0) != (u32_at(rec, 8) & (1 << 30) != 0) {
                            if version == 105 {
                                Codec::Lz4Frame
                            } else {
                                Codec::Zlib
                            }
                        } else {
                            Codec::Raw
                        },
                    },
                    prefixed: flags & 0x100 != 0,
                },
            );
            paths.push(name);
        }
    }
    if paths.len() as u64 != files
        || cursor != blocks.len()
        || names[name_cursor..].iter().any(|&b| b != 0)
    {
        return Err(corrupt("directory counts or name lengths disagree"));
    }
    Ok(paths)
}
fn ba2<R: Read + Seek>(d: &mut Directory<R>) -> Result<Vec<String>> {
    let h = d.read(0, 24)?;
    let version = u32_at(&h, 4);
    if !matches!(version, 1 | 2 | 3 | 7 | 8) {
        return Err(ArchiveError::Unsupported(format!("BA2 version {version}")));
    }
    let kind = &h[8..12];
    if kind != b"GNRL" && kind != b"DX10" {
        return Err(ArchiveError::Unsupported(format!(
            "BA2 type {:?}",
            String::from_utf8_lossy(kind)
        )));
    }
    let n = u32_at(&h, 12);
    count(n)?;
    let names_offset = u64_at(&h, 16);
    let mut cursor = match version {
        2 => 32,
        3 => 36,
        _ => 24,
    };
    d.range(0, cursor)?;
    let codec = if version == 3 {
        match u32_at(&d.read(32, 4)?, 0) {
            0 => Codec::Zlib,
            3 => Codec::Lz4Block,
            method if d.requested.is_some() => {
                return Err(ArchiveError::Unsupported(format!(
                    "BA2 compression method {method}"
                )))
            }
            _ => Codec::Zlib,
        }
    } else {
        Codec::Zlib
    };
    let mut name_cursor = names_offset;
    let mut paths = Vec::new();
    for _ in 0..n {
        let length = d.read(name_cursor, 2)?;
        name_cursor += 2;
        let length = u16::from_le_bytes(length.try_into().unwrap()) as u64;
        if length as usize > MAX_PATH {
            return Err(corrupt("oversized member path"));
        }
        paths.push(path(&d.read(name_cursor, length)?)?);
        name_cursor += length;
    }
    let mut ranges = Vec::new();
    for name in &paths {
        let payload = if kind == b"GNRL" {
            let r = d.read(cursor, 36)?;
            cursor += 36;
            let segment = Segment::packed(u64_at(&r, 16), u32_at(&r, 24), u32_at(&r, 28), codec);
            ranges.push((segment.offset, segment.stored));
            Payload::General(segment)
        } else {
            let r = d.read(cursor, 24)?;
            cursor += 24;
            let chunks = r[13] as u64;
            if chunks == 0 || u16::from_le_bytes([r[14], r[15]]) != 24 {
                return Err(corrupt("invalid texture chunk directory"));
            }
            let mut texture = Texture {
                width: u16::from_le_bytes([r[18], r[19]]),
                height: u16::from_le_bytes([r[16], r[17]]),
                mips: r[20],
                format: r[21],
                cube: r[22],
                tile_mode: r[23],
                chunks: Vec::new(),
            };
            for _ in 0..chunks {
                let c = d.read(cursor, 24)?;
                cursor += 24;
                let first = u16::from_le_bytes([c[16], c[17]]);
                let last = u16::from_le_bytes([c[18], c[19]]);
                if first > last {
                    return Err(corrupt("reversed texture mip range"));
                }
                let segment = Segment::packed(u64_at(&c, 0), u32_at(&c, 8), u32_at(&c, 12), codec);
                ranges.push((segment.offset, segment.stored));
                texture.chunks.push(TextureChunk {
                    segment,
                    first,
                    last,
                });
            }
            Payload::Texture(texture)
        };
        d.select(name, payload);
    }
    if names_offset < cursor {
        return Err(corrupt("name table overlaps records"));
    }
    let records_end = cursor;
    cursor = name_cursor;
    for (offset, size) in ranges {
        d.range(offset, size)?;
        if offset < records_end || (offset < cursor && offset + size > names_offset) {
            return Err(corrupt("member overlaps directory"));
        }
    }
    Ok(paths)
}
fn tes3<R: Read + Seek>(d: &mut Directory<R>) -> Result<Vec<String>> {
    let h = d.read(0, 12)?;
    let hash_offset = 12 + u32_at(&h, 4);
    let n = u32_at(&h, 8);
    count(n)?;
    let table = d.read(12, n * 12)?;
    let names_start = 12 + n * 12;
    let names_len = hash_offset
        .checked_sub(names_start)
        .ok_or_else(|| corrupt("TES3 name table underflow"))?;
    let names = d.read(names_start, names_len)?;
    let payload = hash_offset + n * 8;
    d.range(hash_offset, n * 8)?;
    let mut paths = Vec::new();
    for i in 0..n as usize {
        let size = u32_at(&table, i * 8);
        let offset = payload + u32_at(&table, i * 8 + 4);
        d.range(offset, size)?;
        let start = u32_at(&table, n as usize * 8 + i * 4) as usize;
        let tail = names
            .get(start..)
            .ok_or_else(|| corrupt("TES3 name offset outside table"))?;
        let end = tail
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| corrupt("unterminated TES3 name"))?;
        let name = path(&tail[..end])?;
        d.select(
            &name,
            Payload::General(Segment {
                offset,
                stored: size,
                unpacked: size,
                codec: Codec::Raw,
            }),
        );
        paths.push(name);
    }
    Ok(paths)
}

impl ConflictMap {
    /// Archive activation order is lowest first. Resolve each physical archive
    /// through the loose layer map before reading members; loose members win.
    pub fn build_with_archives_from(
        parts: &[(Layer, (Vec<String>, bool))],
        archives: &[ActiveArchive],
    ) -> Self {
        let mut map = Self::build_from(parts);
        for (key, n) in &map.files {
            let provider = |origin| AssetProvider {
                origin,
                archive: None,
                plugin: None,
            };
            map.asset_files.insert(
                key.clone(),
                AssetNode {
                    winner: provider(n.winner),
                    alternatives: n.alternatives.iter().map(|&o| provider(o)).collect(),
                    precedence_uncertain: false,
                    display_path: n.display_path.clone(),
                },
            );
        }
        let mut seen = BTreeSet::new();
        let mut unreadable_origins = BTreeSet::new();
        let mut unreadable_higher_archive = false;
        for a in archives.iter().rev() {
            let key = a.name.to_ascii_lowercase();
            if !seen.insert(key.clone()) {
                continue;
            }
            if key.contains('/') || key.contains('\\') {
                map.archive_diagnostics.push(ArchiveDiagnostic {
                    archive: a.name.clone(),
                    origin: None,
                    error: corrupt("archive name must be in Data root"),
                });
                continue;
            }
            let Some(n) = map.files.get(&key) else {
                map.archive_diagnostics.push(ArchiveDiagnostic {
                    archive: a.name.clone(),
                    origin: None,
                    error: ArchiveError::Io(
                        "active archive is absent from the winning layers".into(),
                    ),
                });
                continue;
            };
            let origin = n.winner;
            let Some((layer, _)) = parts.iter().find(|(l, _)| l.origin == origin) else {
                continue;
            };
            match read_archive_members(&layer.root.join(&n.display_path)) {
                Err(error) => {
                    unreadable_origins.insert(origin);
                    unreadable_higher_archive = true;
                    map.archive_diagnostics.push(ArchiveDiagnostic {
                        archive: a.name.clone(),
                        origin: Some(origin),
                        error,
                    });
                }
                Ok(paths) => {
                    for display in paths {
                        let provider = AssetProvider {
                            origin,
                            archive: Some(n.display_path.clone()),
                            plugin: a.plugin.clone(),
                        };
                        map.asset_files
                            .entry(display.to_ascii_lowercase())
                            .and_modify(|n| {
                                if n.winner.archive.is_some()
                                    && a.order_uncertain
                                    && n.winner.plugin == a.plugin
                                {
                                    n.precedence_uncertain = true;
                                }
                                n.alternatives.push(provider.clone());
                            })
                            .or_insert(AssetNode {
                                winner: provider,
                                alternatives: Vec::new(),
                                display_path: display,
                                // Missing directory metadata may conceal an override.
                                // Previously inserted higher archives and loose files
                                // remain known winners.
                                precedence_uncertain: unreadable_higher_archive,
                            });
                    }
                }
            }
        }
        map.asset_mods = asset_mods(&map.asset_files, parts, &unreadable_origins);
        for (key, node) in &map.asset_files {
            if node.alternatives.is_empty()
                || !std::iter::once(&node.winner)
                    .chain(&node.alternatives)
                    .any(|p| p.archive.is_some())
            {
                continue;
            }
            let origins: BTreeSet<_> = std::iter::once(&node.winner)
                .chain(&node.alternatives)
                .map(|p| p.origin)
                .collect();
            for origin in origins {
                map.asset_conflicts
                    .entry(origin)
                    .or_default()
                    .push(key.clone());
            }
        }
        map
    }
}
fn asset_mods(
    files: &BTreeMap<String, AssetNode>,
    parts: &[(Layer, (Vec<String>, bool))],
    unreadable_origins: &BTreeSet<OriginId>,
) -> HashMap<OriginId, ModConflicts> {
    let mut mods: HashMap<_, _> = parts
        .iter()
        .map(|(l, (_, hidden))| {
            (
                l.origin,
                ModConflicts {
                    has_hidden: *hidden,
                    ..Default::default()
                },
            )
        })
        .collect();
    let mut uncertain_origins = unreadable_origins.clone();
    for (key, n) in files {
        if n.precedence_uncertain {
            uncertain_origins.extend(
                std::iter::once(&n.winner)
                    .chain(&n.alternatives)
                    .map(|p| p.origin),
            );
            continue;
        }
        // Archive containers are transport files, not assets. Counting them as
        // wins would hide a mod whose every archived member is overridden.
        if n.alternatives.is_empty()
            && !key.contains('/')
            && (key.ends_with(".bsa") || key.ends_with(".ba2"))
        {
            continue;
        }
        let mut seen = BTreeSet::new();
        let providers: Vec<_> = std::iter::once(&n.winner)
            .chain(&n.alternatives)
            .map(|p| p.origin)
            .filter(|o| seen.insert(*o))
            .collect();
        if !n.precedence_uncertain {
            mods.entry(n.winner.origin).or_default().won += 1;
        }
        for (i, &higher) in providers.iter().enumerate() {
            mods.entry(higher).or_default().total += 1;
            for &lower in &providers[i + 1..] {
                if !n.precedence_uncertain && higher != BASE_ORIGIN && lower != BASE_ORIGIN {
                    mods.entry(higher).or_default().overwrites.insert(lower);
                    mods.entry(lower).or_default().overwritten_by.insert(higher);
                }
            }
        }
    }
    for (&origin, m) in &mut mods {
        m.state = if origin == BASE_ORIGIN {
            ConflictState::None
        } else if m.total > 0 && m.won == 0 && !uncertain_origins.contains(&origin) {
            ConflictState::Redundant
        } else {
            match (!m.overwrites.is_empty(), !m.overwritten_by.is_empty()) {
                (true, true) => ConflictState::Mixed,
                (true, false) => ConflictState::Overwrites,
                (false, true) => ConflictState::Overwritten,
                _ => ConflictState::None,
            }
        };
    }
    mods
}

#[cfg(test)]
mod tests;
