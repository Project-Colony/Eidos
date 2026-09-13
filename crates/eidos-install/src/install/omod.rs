//! OMOD container decoding. Format references: erri120/OMODFramework v2.2.2
//! (config/CRC streams) and 05b3d562 (the version-4 writer and script type prefix).
//! All decoding finishes in owned staging; script bodies are never executed here.

pub mod known_handlers;
pub mod obmm;
pub mod scripted;

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::*;

const MAX_ARCHIVE_BYTES: u64 = u32::MAX as u64 - 1;
const MAX_OUTPUT_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 100_000;
const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CRC_BYTES: u64 = 32 * 1024 * 1024;
const MAX_STRING_BYTES: usize = 4 * 1024 * 1024;
const MAX_DICTIONARY_BYTES: u32 = 64 * 1024 * 1024;
const DECODER_TIMEOUT: Duration = Duration::from_secs(600);

fn invalid(message: impl Into<String>) -> InstallError {
    InstallError::Extract(format!("OMOD: {}", message.into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmodCompression {
    Lzma,
    Zip,
}

/// Keep both historical representations without guessing the creator's timezone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmodCreationTime {
    Legacy(String),
    DotNetBinary(i64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmodMetadata {
    pub format_version: u8,
    pub name: String,
    pub major: i32,
    pub minor: i32,
    pub build: i32,
    pub author: String,
    pub email: String,
    pub website: String,
    pub description: String,
    pub created: OmodCreationTime,
    pub compression: OmodCompression,
}

impl OmodMetadata {
    pub fn version(&self) -> String {
        let mut version = self.major.to_string();
        if self.minor >= 0 {
            version.push_str(&format!(".{}", self.minor));
            if self.build >= 0 {
                version.push_str(&format!(".{}", self.build));
            }
        }
        version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmodScriptKind {
    Obmm,
    Python,
    CSharp,
    VisualBasic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmodScript {
    pub kind: OmodScriptKind,
    /// Exact UTF-8 bytes of the .NET string, including its original type prefix.
    pub bytes: Vec<u8>,
    body_start: usize,
}

impl OmodScript {
    pub fn source(&self) -> &str {
        // Constructed only after the entire .NET string has passed UTF-8 validation.
        std::str::from_utf8(&self.bytes[self.body_start..]).expect("validated OMOD script")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OmodFileKind {
    Data,
    Plugin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmodMember {
    pub path: String,
    pub crc32: u32,
    pub size: u64,
    pub kind: OmodFileKind,
}

/// A fully decoded OMOD, retaining separate source roots for script selection.
/// Dropping the session cleans only its owned extraction directory.
pub struct OmodSession {
    pub metadata: OmodMetadata,
    pub script: Option<OmodScript>,
    pub readme: Option<String>,
    pub image: Option<PathBuf>,
    pub members: Vec<OmodMember>,
    pub archive: PathBuf,
    origin: ArchiveOrigin,
    tree: ExtractedTree,
}

impl OmodSession {
    fn verify_archive(&self, cancelled: &AtomicBool) -> Result<(), InstallError> {
        if self.archive != self.origin.path() {
            return Err(invalid("archive path changed after decoding"));
        }
        self.origin.verify_current(cancelled)
    }

    /// Grouped decoded sources; collection hash matching can search both roots.
    pub fn payload_root(&self) -> PathBuf {
        self.tree.path().join("payload")
    }
    pub fn data_root(&self) -> PathBuf {
        self.tree.path().join("payload/data")
    }
    pub fn plugins_root(&self) -> PathBuf {
        self.tree.path().join("payload/plugins")
    }
}

struct Binary<'a> {
    rest: &'a [u8],
}
impl<'a> Binary<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], InstallError> {
        if n > self.rest.len() {
            return Err(invalid("truncated binary record"));
        }
        let (out, rest) = self.rest.split_at(n);
        self.rest = rest;
        Ok(out)
    }
    fn u8(&mut self) -> Result<u8, InstallError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, InstallError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, InstallError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, InstallError> {
        Ok(self.u32()? as i32)
    }
    fn u64(&mut self) -> Result<u64, InstallError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn string(&mut self) -> Result<String, InstallError> {
        let mut size = 0usize;
        for shift in (0..35).step_by(7) {
            let byte = self.u8()?;
            if shift == 28 && byte > 7 {
                return Err(invalid("invalid .NET seven-bit string length"));
            }
            size |= usize::from(byte & 127) << shift;
            if byte & 128 == 0 {
                if size > MAX_STRING_BYTES {
                    return Err(invalid("metadata string exceeds 4 MiB"));
                }
                return String::from_utf8(self.take(size)?.to_vec())
                    .map_err(|_| invalid("metadata string is not UTF-8"));
            }
        }
        Err(invalid("invalid .NET seven-bit string length"))
    }
    fn finish(self) -> Result<(), InstallError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(invalid("trailing bytes after binary record"))
        }
    }
}

fn parse_config(bytes: &[u8]) -> Result<OmodMetadata, InstallError> {
    let mut r = Binary { rest: bytes };
    let format_version = r.u8()?;
    if format_version > 4 {
        return Err(invalid(format!(
            "unsupported config version {format_version}"
        )));
    }
    let name = r.string()?;
    let major = r.i32()?;
    let minor = r.i32()?;
    let author = r.string()?;
    let email = r.string()?;
    let website = r.string()?;
    let description = r.string()?;
    let created = if format_version >= 2 {
        OmodCreationTime::DotNetBinary(r.u64()? as i64)
    } else {
        OmodCreationTime::Legacy(r.string()?)
    };
    let compression = match r.u8()? {
        0 => OmodCompression::Lzma,
        1 => OmodCompression::Zip,
        other => return Err(invalid(format!("unsupported compression method {other}"))),
    };
    let build = if format_version >= 1 { r.i32()? } else { -1 };
    r.finish()?;
    if name.trim().is_empty() || major < 0 || minor < -1 || build < -1 {
        return Err(invalid("invalid name or version in config"));
    }
    Ok(OmodMetadata {
        format_version,
        name,
        major,
        minor,
        build,
        author,
        email,
        website,
        description,
        created,
        compression,
    })
}

fn parse_string(bytes: &[u8]) -> Result<String, InstallError> {
    let mut r = Binary { rest: bytes };
    let text = r.string()?;
    r.finish()?;
    Ok(text)
}

fn parse_script(bytes: &[u8]) -> Result<OmodScript, InstallError> {
    let text = parse_string(bytes)?;
    let (kind, body_start) = match text.as_bytes().first() {
        Some(0) => (OmodScriptKind::Obmm, 1),
        Some(1) => (OmodScriptKind::Python, 1),
        Some(2) => (OmodScriptKind::CSharp, 1),
        Some(3) => (OmodScriptKind::VisualBasic, 1),
        _ => (OmodScriptKind::Obmm, 0),
    };
    Ok(OmodScript {
        kind,
        bytes: text.into_bytes(),
        body_start,
    })
}

fn member_path(raw: &str) -> Result<String, InstallError> {
    let path = raw.replace('\\', "/");
    let parts: Vec<_> = path.split('/').collect();
    if path.len() > 4096
        || parts.len() > MAX_TREE_DEPTH
        || parts.iter().any(|part| {
            part.is_empty()
                || *part == "."
                || *part == ".."
                || part.ends_with(['.', ' '])
                || part
                    .chars()
                    .any(|c| c.is_control() || ":*?\"<>|".contains(c))
                || part.to_ascii_lowercase().starts_with(".eidos")
        })
        || parts.first().is_some_and(|p| {
            ["meta.ini", "root", "omod-readme.txt"]
                .iter()
                .any(|reserved| p.eq_ignore_ascii_case(reserved))
        })
    {
        return Err(invalid(format!("unsafe or reserved payload path: {raw:?}")));
    }
    Ok(path)
}

fn parse_crc(
    bytes: &[u8],
    kind: OmodFileKind,
    members: &mut Vec<OmodMember>,
    paths: &mut BTreeSet<String>,
    total: &mut u64,
) -> Result<(), InstallError> {
    let mut r = Binary { rest: bytes };
    while !r.rest.is_empty() {
        if members.len() >= MAX_FILES {
            return Err(invalid("payload exceeds 100000 files"));
        }
        let path = member_path(&r.string()?)?;
        let crc32 = r.u32()?;
        let size = r.u64()?;
        if size > MAX_FILE_BYTES {
            return Err(invalid("member length is negative or exceeds 2 GiB"));
        }
        *total = total
            .checked_add(size)
            .filter(|n| *n <= MAX_OUTPUT_BYTES)
            .ok_or_else(|| invalid("payload exceeds 8 GiB"))?;
        let key = path.to_ascii_lowercase();
        let mut parent = key.as_str();
        while let Some((prefix, _)) = parent.rsplit_once('/') {
            if paths.contains(prefix) {
                return Err(invalid(format!("file/directory collision: {path}")));
            }
            parent = prefix;
        }
        let prefix = format!("{key}/");
        if paths
            .range(prefix.clone()..)
            .next()
            .is_some_and(|p| p.starts_with(&prefix))
            || !paths.insert(key)
        {
            return Err(invalid(format!(
                "duplicate or case-colliding payload path: {path}"
            )));
        }
        members.push(OmodMember {
            path,
            crc32,
            size,
            kind,
        });
    }
    Ok(())
}

#[derive(Debug)]
struct ZipEntry {
    name: String,
    size: u64,
    crc32: u32,
}

/// Read the real ZIP directory, not 7-Zip's line-oriented filename listing.
/// OMOD ZIP32 containers use store/deflate; encrypted, split and ZIP64 inputs
/// are refused explicitly before starting a decoder.
fn zip_directory(path: &Path) -> Result<Vec<ZipEntry>, InstallError> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if !(22..=MAX_ARCHIVE_BYTES).contains(&length) {
        return Err(invalid("ZIP size is invalid or exceeds the ZIP32 limit"));
    }
    let tail_size = length.min(65557) as usize;
    file.seek(SeekFrom::End(-(tail_size as i64)))?;
    let mut tail = vec![0; tail_size];
    file.read_exact(&mut tail)?;
    let pos = (0..=tail.len() - 22)
        .rev()
        .find(|p| {
            tail[*p..].starts_with(b"PK\x05\x06")
                && *p + 22 + u16::from_le_bytes([tail[*p + 20], tail[*p + 21]]) as usize
                    == tail.len()
        })
        .ok_or_else(|| invalid("missing ZIP end-of-directory"))?;
    let mut end = Binary {
        rest: &tail[pos + 4..],
    };
    if end.u16()? != 0 || end.u16()? != 0 {
        return Err(invalid("split ZIP is unsupported"));
    }
    let count = end.u16()?;
    if count == u16::MAX {
        return Err(invalid("ZIP64 is unsupported"));
    }
    if count != end.u16()? || count == 0 || count > 8 {
        return Err(invalid("invalid OMOD ZIP entry count (maximum 8)"));
    }
    let directory_size = u64::from(end.u32()?);
    let directory_offset = u64::from(end.u32()?);
    let end_offset = length - tail_size as u64 + pos as u64;
    if directory_offset + directory_size != end_offset || directory_size > 1024 * 1024 {
        return Err(invalid("invalid ZIP directory bounds or ZIP64 archive"));
    }
    file.seek(SeekFrom::Start(directory_offset))?;
    let mut directory = vec![0; directory_size as usize];
    file.read_exact(&mut directory)?;
    let mut r = Binary { rest: &directory };
    let mut entries = Vec::new();
    let mut names = BTreeSet::new();
    let mut ranges = Vec::new();
    for _ in 0..count {
        if r.u32()? != 0x02014b50 {
            return Err(invalid("invalid ZIP central header"));
        }
        r.u16()?; // Creator version; permissions are checked independently below.
        if r.u16()? >= 45 {
            return Err(invalid("ZIP64 or newer ZIP features are unsupported"));
        }
        let flags = r.u16()?;
        let method = r.u16()?;
        if flags & (1 | 64 | 8192) != 0 || !matches!(method, 0 | 8) {
            return Err(invalid("encrypted or unsupported ZIP compression"));
        }
        r.take(4)?;
        let crc32 = r.u32()?;
        let packed = r.u32()?;
        let size = r.u32()?;
        let name_len = r.u16()? as usize;
        let extra_len = r.u16()? as usize;
        let comment_len = r.u16()? as usize;
        if r.u16()? != 0 {
            return Err(invalid("split ZIP entry is unsupported"));
        }
        r.u16()?;
        let attributes = r.u32()?;
        let offset = u64::from(r.u32()?);
        let mode = (attributes >> 16) & 0o170000;
        if !matches!(mode, 0 | 0o100000) || attributes & 16 != 0 {
            return Err(invalid("ZIP entries must be regular files"));
        }
        let name = std::str::from_utf8(r.take(name_len)?)
            .map_err(|_| invalid("ZIP entry name is not UTF-8"))?
            .to_string();
        r.take(extra_len + comment_len)?;
        if name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_lowercase() || b == b'.')
            || !names.insert(name.clone())
        {
            return Err(invalid(
                "invalid, duplicate or case-colliding ZIP entry name",
            ));
        }
        if packed == u32::MAX || size == u32::MAX || offset >= directory_offset {
            return Err(invalid("ZIP64 or invalid ZIP local entry bounds"));
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut local = [0; 30];
        file.read_exact(&mut local)?;
        let mut h = Binary { rest: &local };
        if h.u32()? != 0x04034b50 {
            return Err(invalid("invalid ZIP local header"));
        }
        h.u16()?;
        if h.u16()? != flags || h.u16()? != method {
            return Err(invalid("inconsistent ZIP local header"));
        }
        h.take(4)?;
        let local_crc = h.u32()?;
        let local_packed = h.u32()?;
        let local_size = h.u32()?;
        let local_name_len = h.u16()? as usize;
        let local_extra_len = h.u16()? as u64;
        if local_name_len != name_len
            || (flags & 8 == 0
                && (local_crc != crc32 || local_packed != packed || local_size != size))
        {
            return Err(invalid("inconsistent ZIP local sizes or checksum"));
        }
        let mut local_name = vec![0; name_len];
        file.read_exact(&mut local_name)?;
        if local_name != name.as_bytes() {
            return Err(invalid("inconsistent ZIP local name"));
        }
        let data_end = offset + 30 + name_len as u64 + local_extra_len + u64::from(packed);
        if data_end > directory_offset || ranges.iter().any(|(a, b)| offset < *b && data_end > *a) {
            return Err(invalid("overlapping or out-of-bounds ZIP entries"));
        }
        ranges.push((offset, data_end));
        entries.push(ZipEntry {
            name,
            size: u64::from(size),
            crc32,
        });
    }
    r.finish()?;
    Ok(entries)
}

fn validate_container(entries: &[ZipEntry]) -> Result<(), InstallError> {
    let names: BTreeSet<_> = entries.iter().map(|e| e.name.as_str()).collect();
    if !names.contains("config") || !(names.contains("data") || names.contains("plugins")) {
        return Err(invalid(
            "config and a paired data/plugins payload are required",
        ));
    }
    let mut total = 0u64;
    for e in entries {
        let limit = match e.name.as_str() {
            "data" | "plugins" => MAX_ARCHIVE_BYTES,
            "data.crc" | "plugins.crc" => MAX_CRC_BYTES,
            "config" | "script" | "readme" | "image" => MAX_METADATA_BYTES,
            _ => return Err(invalid(format!("unknown container entry: {}", e.name))),
        };
        total = total
            .checked_add(e.size)
            .ok_or_else(|| invalid("container size overflow"))?;
        if e.size > limit || total > MAX_ARCHIVE_BYTES {
            return Err(invalid("container entry exceeds its output limit"));
        }
    }
    for (payload, crc) in [("data", "data.crc"), ("plugins", "plugins.crc")] {
        if names.contains(payload) != names.contains(crc) {
            return Err(invalid(format!("{payload} and {crc} must both be present")));
        }
    }
    Ok(())
}

/// Decode with a bounded channel so stdout cannot fill memory or outgrow disk
/// limits. Cancellation/timeout/error kills and reaps the child before joining.
fn decode_stream(
    bin: &str,
    archive: &Path,
    member: Option<&str>,
    expected: u64,
    cancelled: &AtomicBool,
    mut consume: impl FnMut(&[u8]) -> Result<(), InstallError>,
) -> Result<u32, InstallError> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(invalid("cancelled"));
    }
    let mut command = Command::new(bin);
    command
        .args([
            "x",
            "-so",
            "-y",
            "-bso0",
            "-bsp0",
            if member.is_some() { "-tzip" } else { "-tlzma" },
            "--",
        ])
        .arg(archive)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(member) = member {
        command.arg(member);
    }
    let mut child = command.spawn()?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let (send, recv) = mpsc::sync_channel(2);
    let output = std::thread::spawn(move || loop {
        let mut buffer = vec![0; 65536];
        match stdout.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                buffer.truncate(n);
                if send.send(Ok(buffer)).is_err() {
                    break;
                }
            }
            Err(e) => {
                let _ = send.send(Err(e));
                break;
            }
        }
    });
    let errors = std::thread::spawn(move || {
        let mut text = Vec::new();
        let mut buffer = [0; 4096];
        while let Ok(n) = stderr.read(&mut buffer) {
            if n == 0 {
                break;
            }
            let keep = n.min(16384usize.saturating_sub(text.len()));
            text.extend_from_slice(&buffer[..keep]);
        }
        text
    });
    let started = Instant::now();
    let mut count = 0u64;
    let mut hash = crc32fast::Hasher::new();
    let result = (|| {
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(invalid("cancelled"));
            }
            if started.elapsed() > DECODER_TIMEOUT {
                return Err(invalid("decoder timed out"));
            }
            match recv.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(bytes)) => {
                    count = count
                        .checked_add(bytes.len() as u64)
                        .filter(|n| *n <= expected)
                        .ok_or_else(|| invalid("decoder output exceeds the declared length"))?;
                    hash.update(&bytes);
                    consume(&bytes)?;
                }
                Ok(Err(e)) => return Err(e.into()),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        if count != expected {
            return Err(invalid(format!(
                "decoded {count} bytes, expected {expected}"
            )));
        }
        // A decoder can close stdout before exiting; cancellation stays responsive.
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            if cancelled.load(Ordering::Relaxed) || started.elapsed() > DECODER_TIMEOUT {
                return Err(invalid("decoder cancelled or timed out"));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    })();
    drop(recv);
    if result.is_err() {
        let _ = child.kill();
    }
    let waited = child.wait();
    let _ = output.join();
    let stderr = String::from_utf8_lossy(&errors.join().unwrap_or_default())
        .trim()
        .to_string();
    let status = result.map_err(|error| {
        if stderr.is_empty() {
            error
        } else {
            invalid(format!("{error}; 7-Zip: {stderr}"))
        }
    })?;
    waited?;
    if !status.success() {
        return Err(invalid(format!("7-Zip {status}: {stderr}")));
    }
    Ok(hash.finalize())
}

fn extract_entry(
    bin: &str,
    archive: &Path,
    entry: &ZipEntry,
    target: &Path,
    cancelled: &AtomicBool,
) -> Result<(), InstallError> {
    let mut file = File::create(target)?;
    let crc = decode_stream(
        bin,
        archive,
        Some(&entry.name),
        entry.size,
        cancelled,
        |bytes| {
            file.write_all(bytes)?;
            Ok(())
        },
    )?;
    if crc != entry.crc32 {
        return Err(invalid(format!("outer ZIP CRC mismatch: {}", entry.name)));
    }
    Ok(())
}

fn unpack_group(
    bin: &str,
    encoded: &Path,
    destination: &Path,
    members: &[&OmodMember],
    compression: OmodCompression,
    cancelled: &AtomicBool,
    mut on_progress: impl FnMut(u8),
) -> Result<(), InstallError> {
    let expected: u64 = members.iter().map(|m| m.size).sum();
    let adapted = encoded.with_extension("lzma");
    let (input, member) = match compression {
        OmodCompression::Zip => {
            let zip = zip_directory(encoded)?;
            if zip.len() != 1 || zip[0].name != "a" || zip[0].size != expected {
                return Err(invalid(
                    "nested ZIP must contain only 'a' at the declared payload length",
                ));
            }
            (encoded, Some("a"))
        }
        OmodCompression::Lzma => {
            let mut raw = File::open(encoded)?;
            let mut properties = [0; 5];
            raw.read_exact(&mut properties)?;
            let dictionary = u32::from_le_bytes(properties[1..].try_into().unwrap());
            if properties[0] >= 225 || dictionary > MAX_DICTIONARY_BYTES {
                return Err(invalid(
                    "invalid LZMA properties or dictionary exceeds 64 MiB",
                ));
            }
            let mut output = File::create(&adapted)?;
            output.write_all(&properties)?;
            output.write_all(&expected.to_le_bytes())?;
            io::copy(&mut raw, &mut output)?;
            drop(output);
            (adapted.as_path(), None)
        }
    };
    fs::create_dir_all(destination)?;
    let mut index = 0usize;
    let mut written = 0u64;
    let mut total = 0u64;
    let mut current: Option<File> = None;
    let mut hash = crc32fast::Hasher::new();
    let mut advance = |bytes: &[u8]| -> Result<(), InstallError> {
        let mut rest = bytes;
        while index < members.len() {
            let spec = members[index];
            if current.is_none() {
                let target = checked_destination(destination, &destination.join(&spec.path))?;
                fs::create_dir_all(target.parent().unwrap())?;
                current = Some(File::options().write(true).create_new(true).open(target)?);
            }
            let take = rest.len().min((spec.size - written) as usize);
            if take > 0 {
                current.as_mut().unwrap().write_all(&rest[..take])?;
                hash.update(&rest[..take]);
                written += take as u64;
                total += take as u64;
                rest = &rest[take..];
            }
            if written == spec.size {
                if std::mem::replace(&mut hash, crc32fast::Hasher::new()).finalize() != spec.crc32 {
                    return Err(invalid(format!("payload CRC mismatch: {}", spec.path)));
                }
                current = None;
                written = 0;
                index += 1;
            } else {
                break;
            }
        }
        if !rest.is_empty() {
            return Err(invalid("trailing bytes after final payload member"));
        }
        on_progress(if expected == 0 {
            100
        } else {
            ((total * 100) / expected) as u8
        });
        Ok(())
    };
    decode_stream(bin, input, member, expected, cancelled, &mut advance)?;
    advance(&[])?; // Materialize trailing zero-byte entries too.
    if index != members.len() {
        return Err(invalid("truncated member stream"));
    }
    Ok(())
}

fn decode_container(
    origin: ArchiveOrigin,
    work_dir: &Path,
    entries: Vec<ZipEntry>,
    cancelled: &AtomicBool,
    mut on_progress: impl FnMut(u8),
) -> Result<OmodSession, InstallError> {
    validate_container(&entries)?;
    origin.verify_current(cancelled)?;
    let pinned = origin.pinned_path();
    let archive = pinned.as_path();
    if cancelled.load(Ordering::Relaxed) {
        return Err(invalid("cancelled"));
    }
    let bin = eidos_sevenzip::find_7z().ok_or(InstallError::No7z)?;
    fs::create_dir_all(work_dir)?;
    let tmp = work_dir.join(format!(
        ".eidos-install-omod-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&tmp)?;
    let tree = ExtractedTree::owned(tmp);
    let controls = tree.path().join("container");
    fs::create_dir(&controls)?;
    // Validate the small config and inventory before decoding the large groups.
    on_progress(0);
    for entry in entries
        .iter()
        .filter(|e| !matches!(e.name.as_str(), "data" | "plugins"))
    {
        extract_entry(bin, archive, entry, &controls.join(&entry.name), cancelled)?;
    }
    let metadata = parse_config(&fs::read(controls.join("config"))?)?;
    let optional = |name: &str| -> Result<Option<Vec<u8>>, InstallError> {
        if entries.iter().any(|e| e.name == name) {
            Ok(Some(fs::read(controls.join(name))?))
        } else {
            Ok(None)
        }
    };
    let script = optional("script")?.map(|b| parse_script(&b)).transpose()?;
    let readme = optional("readme")?.map(|b| parse_string(&b)).transpose()?;
    let image = entries
        .iter()
        .any(|e| e.name == "image")
        .then(|| controls.join("image"));
    let mut members = Vec::new();
    let mut paths = BTreeSet::new();
    let mut total = 0;
    for (name, kind) in [
        ("data", OmodFileKind::Data),
        ("plugins", OmodFileKind::Plugin),
    ] {
        if let Some(bytes) = optional(&format!("{name}.crc"))? {
            parse_crc(&bytes, kind, &mut members, &mut paths, &mut total)?;
        }
    }
    if members.is_empty() {
        return Err(invalid("container declares no payload files"));
    }
    let mut completed = 0;
    for (name, kind) in [
        ("data", OmodFileKind::Data),
        ("plugins", OmodFileKind::Plugin),
    ] {
        let destination = tree.path().join("payload").join(name);
        fs::create_dir_all(&destination)?;
        if let Some(entry) = entries.iter().find(|e| e.name == name) {
            extract_entry(bin, archive, entry, &controls.join(name), cancelled)?;
            let group: Vec<_> = members.iter().filter(|m| m.kind == kind).collect();
            let group_bytes: u64 = group.iter().map(|m| m.size).sum();
            unpack_group(
                bin,
                &controls.join(name),
                &destination,
                &group,
                metadata.compression,
                cancelled,
                |percent| {
                    on_progress(if total == 0 {
                        100
                    } else {
                        ((completed + group_bytes * u64::from(percent) / 100) * 100 / total) as u8
                    })
                },
            )?;
            completed += group_bytes;
        }
    }
    on_progress(100);
    origin.verify_current(cancelled)?;
    Ok(OmodSession {
        metadata,
        script,
        readme,
        image,
        members,
        archive: origin.path().to_path_buf(),
        origin,
        tree,
    })
}

/// Inspect/decode an OMOD without publishing it or interpreting a script. The
/// cancellation flag is checked during every decoder and before staging creation.
pub fn open_omod_with(
    archive: &Path,
    work_dir: &Path,
    cancelled: &AtomicBool,
    on_progress: impl FnMut(u8),
) -> Result<OmodSession, InstallError> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(invalid("OMOD decoding cancelled"));
    }
    let origin = ArchiveOrigin::capture_bounded(archive, cancelled, MAX_ARCHIVE_BYTES)?;
    let entries = zip_directory(&origin.pinned_path())?;
    decode_container(origin, work_dir, entries, cancelled, on_progress)
}

/// Cancellable container probe used by background installer classification.
pub fn try_open_omod_with(
    archive: &Path,
    work_dir: &Path,
    cancelled: &AtomicBool,
    on_progress: impl FnMut(u8),
) -> Result<Option<OmodSession>, InstallError> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(invalid("OMOD decoding cancelled"));
    }

    let required = archive
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("omod"));
    let entries = match zip_directory(archive) {
        Ok(entries) => entries,
        Err(error) if required => return Err(error),
        Err(_) => return Ok(None),
    };
    let candidate = required
        || entries.iter().any(|e| e.name == "config")
            && entries.iter().any(|e| {
                matches!(
                    e.name.as_str(),
                    "data" | "plugins" | "data.crc" | "plugins.crc"
                )
            });
    if !candidate {
        return Ok(None);
    }
    // The cheap directory probe above avoids hashing ordinary archives twice.
    // Reparse the candidate through the held source used by every decoder.
    let origin = ArchiveOrigin::capture_bounded(archive, cancelled, MAX_ARCHIVE_BYTES)?;
    let entries = zip_directory(&origin.pinned_path())?;
    decode_container(origin, work_dir, entries, cancelled, on_progress).map(Some)
}

/// Probe the OMOD container contract and fully decode a recognized archive.
/// Non-OMOD archives return `None`; malformed `.omod` files return an error.
pub fn try_open_omod(
    archive: &Path,
    work_dir: &Path,
    on_progress: impl FnMut(u8),
) -> Result<Option<OmodSession>, InstallError> {
    try_open_omod_with(archive, work_dir, &AtomicBool::new(false), on_progress)
}

/// Publish only an unscripted OMOD. Scripted sessions require the explicit
/// interpreter/selection path and are never silently installed with all files.
pub fn install_omod(
    session: &OmodSession,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
) -> Result<InstallReport, InstallError> {
    install_omod_inner(
        session,
        mods_dir,
        name,
        game_id,
        policy,
        None::<fn(&Path) -> Result<(), InstallError>>,
    )
}

/// Complete a collection recipe in private staging before publishing an OMOD.
/// Merge policies fail before any write or callback, matching the other
/// transform entry points. Ordinary `install_omod` retains live Merge behavior.
pub fn install_omod_with_finish(
    session: &OmodSession,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    finish: impl FnOnce(&Path) -> Result<(), InstallError>,
) -> Result<InstallReport, InstallError> {
    install_omod_inner(session, mods_dir, name, game_id, policy, Some(finish))
}

fn install_omod_inner(
    session: &OmodSession,
    mods_dir: &Path,
    name: &str,
    game_id: &str,
    policy: OverwritePolicy,
    finish: Option<impl FnOnce(&Path) -> Result<(), InstallError>>,
) -> Result<InstallReport, InstallError> {
    if finish.is_some()
        && matches!(
            policy,
            OverwritePolicy::Merge | OverwritePolicy::MergeWithBackup
        )
    {
        return Err(InstallError::BadSelection(
            "OMOD transforms require a staged replacement; Merge is not supported".into(),
        ));
    }
    if session.script.is_some() {
        return Err(InstallError::BadSelection("This OMOD has an installer script; explicit script evaluation is required before installation".into()));
    }
    let cancelled = AtomicBool::new(false);
    session.verify_archive(&cancelled)?;
    let quote = |text: &str| {
        format!(
            "\"{}\"",
            text.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\r', "\\r")
                .replace('\n', "\\n")
        )
    };
    let facts = vec![
        ("author".into(), quote(&session.metadata.author)),
        ("version".into(), session.metadata.version()),
        ("eidosOmodName".into(), quote(&session.metadata.name)),
        (
            "eidosOmodDescription".into(),
            quote(&session.metadata.description),
        ),
        ("eidosOmodEmail".into(), quote(&session.metadata.email)),
        ("eidosOmodWebsite".into(), quote(&session.metadata.website)),
        (
            "eidosOmodCreated".into(),
            quote(&format!("{:?}", session.metadata.created)),
        ),
    ];
    // Recheck source bytes in private staging before any ordinary Merge checkpoint.
    let tmp = mods_dir.join(format!(
        ".eidos-install-omod-publish-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(mods_dir)?;
    fs::create_dir(&tmp)?;
    let prepared = ExtractedTree::owned(tmp);
    for spec in &session.members {
        session.verify_archive(&cancelled)?;
        let base = match spec.kind {
            OmodFileKind::Data => session.data_root(),
            OmodFileKind::Plugin => session.plugins_root(),
        };
        let src = checked_destination(session.tree.path(), &base.join(&spec.path))?;
        if !source_within(session.tree.path(), &src) || !fs::symlink_metadata(&src)?.is_file() {
            return Err(invalid("decoded source changed or is not a regular file"));
        }
        let dst = checked_destination(prepared.path(), &prepared.path().join(&spec.path))?;
        fs::create_dir_all(dst.parent().unwrap())?;
        let mut input = File::open(&src)?;
        let mut output = File::options().write(true).create_new(true).open(&dst)?;
        let mut count = 0u64;
        let mut hash = crc32fast::Hasher::new();
        let mut buffer = [0; 65536];
        loop {
            let n = input.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > spec.size {
                return Err(invalid("decoded member grew before publication"));
            }
            hash.update(&buffer[..n]);
            output.write_all(&buffer[..n])?;
        }
        if count != spec.size || hash.finalize() != spec.crc32 {
            return Err(invalid("decoded member changed before publication"));
        }
        session.verify_archive(&cancelled)?;
    }
    if let Some(readme) = &session.readme {
        fs::write(prepared.path().join("omod-readme.txt"), readme)?;
    }
    // Source inventory is checked, but a target game can still require a folder unit.
    if LayoutRules::for_game(game_id).mod_unit == eidos_gamedef::ModUnit::Folder {
        return Err(invalid(
            "OMOD's Data-relative payload requires a file-unit game",
        ));
    }
    if let Some(finish) = finish {
        finish(prepared.path())?;
    }
    session.verify_archive(&cancelled)?;
    super::simple::install_destination_ready(
        &session.archive,
        mods_dir,
        name,
        game_id,
        policy,
        &facts,
        |dest, merging| {
            place_sources(&[prepared.path().to_path_buf()], dest, merging)?;
            Ok((String::new(), false, Vec::new()))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::super::tests::{omod_config, omod_crc, omod_string, omod_zip};
    use super::*;

    #[test]
    fn dotnet_strings_are_bounded_exact_and_utf8() {
        let expected = "É".repeat(100);
        let mut bytes = Vec::new();
        omod_string(&mut bytes, &expected);
        assert_eq!(parse_string(&bytes).unwrap(), expected);
        for bad in [
            &[128][..],
            &[255, 255, 255, 255, 8],
            &[1, 255],
            &[1, b'a', 0],
            &[128, 128, 128, 128, 128],
        ] {
            assert!(parse_string(bad).is_err(), "accepted {bad:?}");
        }
        let mut oversized = Vec::new();
        let mut n = MAX_STRING_BYTES + 1;
        while n >= 128 {
            oversized.push((n as u8) | 128);
            n >>= 7;
        }
        oversized.push(n as u8);
        assert!(parse_string(&oversized)
            .unwrap_err()
            .to_string()
            .contains("4 MiB"));
    }

    #[test]
    fn config_preserves_creation_representation_and_rejects_trailing_data() {
        for version in 0..=4 {
            let bytes = omod_config(version, 1);
            let parsed = parse_config(&bytes).unwrap();
            assert_eq!(
                parsed.created,
                if version < 2 {
                    OmodCreationTime::Legacy("12/09/2006 12:30".into())
                } else {
                    OmodCreationTime::DotNetBinary(638000000000000000)
                }
            );
            let mut bad = bytes.clone();
            bad.push(0);
            assert!(parse_config(&bad).is_err());
            assert!(parse_config(&bytes[..bytes.len() - 1]).is_err());
        }
        assert!(parse_config(&omod_config(5, 1)).is_err());
        assert!(parse_config(&omod_config(4, 2)).is_err());
    }

    #[test]
    fn crc_file_count_and_combined_output_limits_are_enforced() {
        let bytes = omod_crc(&[("Demo.esp", b"abc")]);
        let mut paths = BTreeSet::new();
        let mut total = MAX_OUTPUT_BYTES - 2;
        assert!(parse_crc(
            &bytes,
            OmodFileKind::Data,
            &mut Vec::new(),
            &mut paths,
            &mut total
        )
        .unwrap_err()
        .to_string()
        .contains("8 GiB"));
        let mut members = vec![
            OmodMember {
                path: "empty".into(),
                size: 0,
                crc32: 0,
                kind: OmodFileKind::Data
            };
            MAX_FILES
        ];
        assert!(parse_crc(
            &bytes,
            OmodFileKind::Data,
            &mut members,
            &mut BTreeSet::new(),
            &mut 0
        )
        .unwrap_err()
        .to_string()
        .contains("100000"));
        let mut long = bytes.clone();
        let n = long.len();
        long[n - 8..].copy_from_slice(&(MAX_FILE_BYTES + 1).to_le_bytes());
        assert!(parse_crc(
            &long,
            OmodFileKind::Data,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut 0
        )
        .unwrap_err()
        .to_string()
        .contains("2 GiB"));
    }

    #[test]
    fn archive_origin_binds_decode_completion_and_unscripted_publication() {
        let base = std::env::temp_dir().join(format!(
            "eidos-omod-origin-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&base).unwrap();
        let owned = ExtractedTree::owned(base);
        let archive = owned.path().join("pack.omod");
        let original = omod_zip(&[
            ("config", omod_config(4, 1)),
            ("data.crc", omod_crc(&[("Demo.esp", b"abc")])),
            ("data", omod_zip(&[("a", b"abc".to_vec())])),
        ]);
        fs::write(&archive, &original).unwrap();
        let work = owned.path().join("work");
        let cancel = AtomicBool::new(false);
        assert!(open_omod_with(&archive, &work, &cancel, |percent| {
            if percent == 100 {
                fs::write(&archive, b"replacement").unwrap();
            }
        })
        .is_err());
        assert_eq!(fs::read_dir(&work).unwrap().count(), 0);

        fs::write(&archive, &original).unwrap();
        let session = open_omod_with(&archive, &work, &cancel, |_| {}).unwrap();
        let mods = owned.path().join("mods");
        assert!(install_omod_with_finish(
            &session,
            &mods,
            "Pack",
            "oblivion",
            OverwritePolicy::Replace,
            |_| {
                fs::write(&archive, b"replacement")?;
                Ok(())
            }
        )
        .is_err());
        assert!(!mods.join("Pack").exists());
    }

    #[test]
    fn cancellation_creates_no_staging_and_decoder_output_is_bounded() {
        let base = std::env::temp_dir().join(format!(
            "eidos-omod-cancel-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&base).unwrap();
        let owned = ExtractedTree::owned(base);
        let archive = owned.path().join("pack.omod");
        fs::write(
            &archive,
            omod_zip(&[
                ("config", omod_config(4, 1)),
                ("data.crc", omod_crc(&[("Demo.esp", b"abc")])),
                ("data", omod_zip(&[("a", b"abc".to_vec())])),
            ]),
        )
        .unwrap();
        let work = owned.path().join("work");
        assert!(
            open_omod_with(&archive, &work, &AtomicBool::new(true), |_| {})
                .err()
                .unwrap()
                .to_string()
                .contains("cancelled")
        );
        assert!(!work.exists());
        let bin = eidos_sevenzip::find_7z().expect("7-Zip is required for installer tests");
        let mut written = 0;
        assert!(decode_stream(
            bin,
            &archive,
            Some("config"),
            1,
            &AtomicBool::new(false),
            |bytes| {
                written += bytes.len();
                Ok(())
            }
        )
        .unwrap_err()
        .to_string()
        .contains("declared length"));
        assert_eq!(written, 0);
        let cancel = AtomicBool::new(false);
        assert!(decode_stream(
            bin,
            &archive,
            Some("config"),
            omod_config(4, 1).len() as u64,
            &cancel,
            |_| {
                cancel.store(true, Ordering::Relaxed);
                Ok(())
            }
        )
        .unwrap_err()
        .to_string()
        .contains("cancelled"));
    }

    #[test]
    fn zip_header_symlinks_encryption_duplicates_and_bounds_are_rejected() {
        let base = std::env::temp_dir().join(format!(
            "eidos-omod-zip-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&base).unwrap();
        let owned = ExtractedTree::owned(base);
        let archive = owned.path().join("pack.omod");
        let original = omod_zip(&[
            ("config", omod_config(4, 1)),
            ("data.crc", omod_crc(&[("Demo.esp", b"abc")])),
            ("data", omod_zip(&[("a", b"abc".to_vec())])),
        ]);
        let central = u32::from_le_bytes(
            original[original.len() - 6..original.len() - 2]
                .try_into()
                .unwrap(),
        ) as usize;
        let mut cases = vec![];
        let mut linked = original.clone();
        linked[central + 38..central + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
        cases.push(linked);
        let mut encrypted = original.clone();
        encrypted[central + 8] |= 1;
        cases.push(encrypted);
        let mut zip64 = original.clone();
        zip64[central + 6] = 45;
        cases.push(zip64);
        let mut mismatch = original.clone();
        mismatch[30] = b'x';
        cases.push(mismatch);
        let mut out_of_bounds = original.clone();
        out_of_bounds[central + 42..central + 46].copy_from_slice(&u32::MAX.to_le_bytes());
        cases.push(out_of_bounds);
        let mut trailing = original.clone();
        trailing.push(0);
        cases.push(trailing);
        cases.push(omod_zip(&[("config", vec![]), ("config", vec![])]));
        cases.push(omod_zip(&[("config", vec![]), ("Config", vec![])]));
        cases.push(omod_zip(&[("../escape", vec![])]));
        for (index, bytes) in cases.into_iter().enumerate() {
            fs::write(&archive, bytes).unwrap();
            assert!(
                zip_directory(&archive).is_err(),
                "accepted ZIP mutation {index}"
            );
        }
        let mut corrupted = original;
        corrupted[36] ^= 1;
        fs::write(&archive, corrupted).unwrap();
        let error = open_omod_with(
            &archive,
            &owned.path().join("work"),
            &AtomicBool::new(false),
            |_| {},
        )
        .err()
        .unwrap()
        .to_string();
        assert!(
            error.contains("7-Zip") && (error.contains("CRC") || error.contains("Data Error")),
            "{error}"
        );
    }
}
