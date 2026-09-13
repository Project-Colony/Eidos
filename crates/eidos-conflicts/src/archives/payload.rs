//! Exact payload decoding; directory indexing never calls these routines.
use super::*;
use std::io::{BufRead, BufReader, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveMemberInfo {
    pub member: String,
    pub bytes_written: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Codec {
    Raw,
    Zlib,
    Lz4Frame,
    Lz4Block,
}
pub(super) struct Segment {
    pub offset: u64,
    pub stored: u64,
    pub unpacked: u64,
    pub codec: Codec,
}
impl Segment {
    pub(super) fn packed(offset: u64, packed: u64, unpacked: u64, codec: Codec) -> Self {
        Self {
            offset,
            stored: if packed == 0 { unpacked } else { packed },
            unpacked,
            codec: if packed == 0 { Codec::Raw } else { codec },
        }
    }
}
pub(super) enum Payload {
    General(Segment),
    Bsa { segment: Segment, prefixed: bool },
    Texture(Texture),
}
pub(super) struct Texture {
    pub width: u16,
    pub height: u16,
    pub mips: u8,
    pub format: u8,
    pub cube: u8,
    pub tile_mode: u8,
    pub chunks: Vec<TextureChunk>,
}
pub(super) struct TextureChunk {
    pub segment: Segment,
    pub first: u16,
    pub last: u16,
}
/// Decodes one case-insensitive member to a caller-owned sink. The sink may contain
/// partial bytes on error; use `export_archive_member` for transactional publication.
/// `max_bytes` bounds decoded output, including the reconstructed DDS header.
pub fn write_archive_member(
    file: &mut File,
    member: &str,
    max_bytes: u64,
    output: &mut impl Write,
) -> Result<ArchiveMemberInfo> {
    let identity = ArchiveIdentity::for_file(file)?;
    let (mut d, _) = parse_directory(file, Some(member))?;
    let (name, payload) = d
        .selected
        .take()
        .ok_or_else(|| corrupt("member is absent from archive"))?;
    let size = match payload {
        Payload::General(segment) => decode(&mut d.reader, &segment, max_bytes, output)?,
        Payload::Bsa {
            mut segment,
            prefixed,
        } => {
            d.reader.seek(SeekFrom::Start(segment.offset))?;
            if prefixed {
                let mut len = [0];
                if segment.stored == 0 {
                    return Err(corrupt("missing embedded filename"));
                }
                d.reader.read_exact(&mut len)?;
                let prefix = u64::from(len[0]) + 1;
                segment.stored = segment
                    .stored
                    .checked_sub(prefix)
                    .ok_or_else(|| corrupt("embedded filename exceeds member"))?;
                let mut embedded = vec![0; len[0] as usize];
                d.reader.read_exact(&mut embedded)?;
                // The embedded filename is metadata only and is never used as an output path.
                path(embedded.strip_suffix(&[0]).unwrap_or(&embedded))?;
                segment.offset += prefix;
            }
            segment.unpacked = segment.stored;
            if segment.codec != Codec::Raw {
                segment.stored = segment
                    .stored
                    .checked_sub(4)
                    .ok_or_else(|| corrupt("missing uncompressed size"))?;
                let mut size = [0; 4];
                d.reader.read_exact(&mut size)?;
                segment.unpacked = u32::from_le_bytes(size) as u64;
                segment.offset += 4;
            }
            decode(&mut d.reader, &segment, max_bytes, output)?
        }
        Payload::Texture(texture) => texture.write(&mut d.reader, max_bytes, output)?,
    };
    identity.verify_file(d.reader)?;
    Ok(ArchiveMemberInfo {
        member: name,
        bytes_written: size,
    })
}
fn limit(size: u64, max: u64) -> Result<()> {
    if size > max {
        Err(ArchiveError::Limit(format!(
            "member requires {size} bytes; limit is {max}"
        )))
    } else {
        Ok(())
    }
}
fn codec_error(e: impl std::fmt::Display) -> ArchiveError {
    corrupt(&format!("invalid compressed payload: {e}"))
}
// FrameDecoder treats UnexpectedEof as an end marker. A bounded strict reader
// reports InvalidData instead, so truncated frames cannot become successful files.
struct StrictReader<R> {
    inner: R,
    remaining: u64,
}
impl<R: Read> Read for StrictReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing compressed stream end marker",
            ));
        }
        let len = out
            .len()
            .min(usize::try_from(self.remaining).unwrap_or(usize::MAX));
        let n = self.inner.read(&mut out[..len])?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "truncated compressed stream",
            ));
        }
        self.remaining -= n as u64;
        Ok(n)
    }
}
fn copy_exact(reader: &mut impl Read, output: &mut impl Write, expected: u64) -> Result<u64> {
    let mut total = 0;
    let mut buf = [0; 64 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(codec_error)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > expected {
            return Err(corrupt("decoded payload exceeds declared size"));
        }
        output.write_all(&buf[..n])?;
    }
    if total != expected {
        return Err(corrupt("decoded payload size mismatch"));
    }
    Ok(total)
}
fn decode<R: Read + Seek>(
    reader: &mut R,
    s: &Segment,
    max: u64,
    output: &mut impl Write,
) -> Result<u64> {
    limit(s.unpacked, max)?;
    reader.seek(SeekFrom::Start(s.offset))?;
    match s.codec {
        Codec::Raw => copy_exact(&mut reader.take(s.stored), output, s.unpacked),
        Codec::Zlib => {
            let mut input = BufReader::new(reader.take(s.stored));
            if input.fill_buf()?.starts_with(&[0x1f, 0x8b]) {
                let mut decoder = flate2::bufread::GzDecoder::new(input);
                let size = copy_exact(&mut decoder, output, s.unpacked)?;
                let rest = decoder.into_inner();
                if rest.get_ref().limit() != 0 || !rest.buffer().is_empty() {
                    return Err(corrupt("trailing compressed payload data"));
                }
                return Ok(size);
            }
            let mut decoder = flate2::Decompress::new(true);
            let mut buf = [0; 64 * 1024];
            loop {
                let before_in = decoder.total_in();
                let before_out = decoder.total_out();
                let status = decoder
                    .decompress(input.fill_buf()?, &mut buf, flate2::FlushDecompress::None)
                    .map_err(codec_error)?;
                let consumed = (decoder.total_in() - before_in) as usize;
                let produced = (decoder.total_out() - before_out) as usize;
                input.consume(consumed);
                if decoder.total_out() > s.unpacked {
                    return Err(corrupt("decoded payload exceeds declared size"));
                }
                output.write_all(&buf[..produced])?;
                if status == flate2::Status::StreamEnd {
                    if decoder.total_in() != s.stored || decoder.total_out() != s.unpacked {
                        return Err(corrupt("compressed payload size mismatch or trailing data"));
                    }
                    return Ok(decoder.total_out());
                }
                if consumed == 0 && produced == 0 {
                    return Err(corrupt("missing zlib stream end marker"));
                }
            }
        }
        Codec::Lz4Frame => {
            // The frame decoder also returns zero for valid empty data blocks.
            // Inspect the block that produced zero before treating it as EOF.
            let mut header = [0; 7];
            if s.stored < header.len() as u64 {
                return Err(corrupt("truncated LZ4 frame header"));
            }
            reader.read_exact(&mut header)?;
            if header[..4] != [4, 34, 77, 24] {
                return Err(corrupt("invalid LZ4 frame signature"));
            }
            let header_size =
                7 + if header[4] & 8 != 0 { 8 } else { 0 } + if header[4] & 1 != 0 { 4 } else { 0 };
            reader.seek(SeekFrom::Start(s.offset))?;
            let mut decoder = lz4_flex::frame::FrameDecoder::new(StrictReader {
                inner: reader,
                remaining: s.stored,
            });
            let mut total = 0;
            let mut buf = [0; 64 * 1024];
            loop {
                let before = decoder.get_ref().remaining;
                let n = decoder.read(&mut buf).map_err(codec_error)?;
                if n == 0 {
                    let block = s.offset
                        + if before == s.stored {
                            header_size
                        } else {
                            s.stored - before
                        };
                    let input = decoder.get_mut();
                    let position = input.inner.stream_position()?;
                    input.inner.seek(SeekFrom::Start(block))?;
                    let mut marker = [0; 4];
                    input.inner.read_exact(&mut marker)?;
                    input.inner.seek(SeekFrom::Start(position))?;
                    if marker != [0; 4] {
                        continue;
                    }
                    if input.remaining != 0 || total != s.unpacked {
                        return Err(corrupt("LZ4 frame size mismatch or trailing data"));
                    }
                    return Ok(total);
                }
                total += n as u64;
                if total > s.unpacked {
                    return Err(corrupt("decoded payload exceeds declared size"));
                }
                output.write_all(&buf[..n])?;
            }
        }
        Codec::Lz4Block => {
            limit(s.stored, MAX_LZ4_BLOCK_BYTES)?;
            limit(s.unpacked, MAX_LZ4_BLOCK_BYTES)?;
            let mut packed = vec![0; s.stored as usize];
            reader.read_exact(&mut packed)?;
            let mut unpacked = vec![0; s.unpacked as usize];
            let size =
                lz4_flex::block::decompress_into(&packed, &mut unpacked).map_err(codec_error)?;
            if size as u64 != s.unpacked {
                return Err(corrupt("LZ4 block size mismatch"));
            }
            output.write_all(&unpacked)?;
            Ok(size as u64)
        }
    }
}

/// Raw LZ4 blocks need contiguous compressed and decoded buffers. Each is capped
/// at 128 MiB; raw/zlib/frame members stream under the caller's output bound.
pub const MAX_LZ4_BLOCK_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TEXTURE_DIMENSION: u16 = 16_384;
impl Texture {
    fn write(
        &self,
        reader: &mut (impl Read + Seek),
        max: u64,
        output: &mut impl Write,
    ) -> Result<u64> {
        if self.width == 0
            || self.height == 0
            || self.mips == 0
            || self.mips as u32 > u16::BITS - self.width.max(self.height).leading_zeros()
            || (self.cube == 1 && self.width != self.height)
        {
            return Err(corrupt("invalid texture dimensions or mip count"));
        }
        if self.width > MAX_TEXTURE_DIMENSION || self.height > MAX_TEXTURE_DIMENSION {
            return Err(ArchiveError::Limit(
                "texture dimensions exceed 16384".into(),
            ));
        }
        if self.cube > 1 || !matches!(self.tile_mode, 0 | 8) {
            return Err(ArchiveError::Unsupported(
                "texture flags or tile mode".into(),
            ));
        }
        // DXGI fixed-size typed formats. Unknown, typeless, planar and video
        // layouts are rejected instead of guessing their byte pitch.
        let (unit, block): (u64, bool) = match self.format {
            2..=4 => (16, false),
            6..=8 => (12, false),
            10..=14 | 16..=18 => (8, false),
            24..=26 | 28..=32 | 34..=38 | 41..=43 | 67 | 87..=89 | 91 | 93 => (4, false),
            49..=52 | 54..=59 | 85 | 86 | 115 => (2, false),
            61..=65 => (1, false),
            71 | 72 | 80 | 81 => (8, true),
            74 | 75 | 77 | 78 | 83 | 84 | 95 | 96 | 98 | 99 => (16, true),
            format => {
                return Err(ArchiveError::Unsupported(format!(
                    "DXGI texture format {format}"
                )))
            }
        };
        let sizes: Vec<u64> = (0..self.mips)
            .map(|mip| {
                let w = u64::from((self.width >> mip).max(1));
                let h = u64::from((self.height >> mip).max(1));
                if block {
                    w.div_ceil(4) * h.div_ceil(4) * unit
                } else {
                    w * h * unit
                }
            })
            .collect();
        let faces = if self.cube == 1 { 6 } else { 1 };
        let mut chunks: Vec<_> = self.chunks.iter().collect();
        chunks.sort_unstable_by_key(|c| c.first);
        let mut next = 0;
        let mut per_face = Vec::new();
        for chunk in &chunks {
            if chunk.first != next || chunk.last >= u16::from(self.mips) {
                return Err(corrupt("missing, overlapping or out-of-range texture mips"));
            }
            let size: u64 = sizes[chunk.first as usize..=chunk.last as usize]
                .iter()
                .sum();
            if size * faces != chunk.segment.unpacked {
                return Err(corrupt("texture chunk size disagrees with mip layout"));
            }
            per_face.push(size);
            next = chunk.last + 1;
        }
        if next != u16::from(self.mips) {
            return Err(corrupt("incomplete texture mip chain"));
        }
        let total = 148 + sizes.iter().sum::<u64>() * faces;
        limit(total, max)?;
        // Always emit DX10 to preserve sRGB, signed formats and cube metadata.
        let mut header = [0u8; 148];
        header[..4].copy_from_slice(b"DDS ");
        let pitch = if block {
            sizes[0]
        } else {
            u64::from(self.width) * unit
        };
        let mip_flags = if self.mips > 1 { 0x20000 } else { 0 };
        let caps = 0x1000
            | if self.mips > 1 {
                0x400008
            } else if faces == 6 {
                8
            } else {
                0
            };
        for (at, value) in [
            (4, 124),
            (8, 0x1007 | mip_flags | if block { 0x80000 } else { 8 }),
            (12, u32::from(self.height)),
            (16, u32::from(self.width)),
            (20, pitch as u32),
            (28, u32::from(self.mips)),
            (76, 32),
            (80, 4),
            (84, u32::from_le_bytes(*b"DX10")),
            (108, caps),
            (112, if faces == 6 { 0xfe00 } else { 0 }),
            (128, u32::from(self.format)),
            (132, 3),
            (136, if faces == 6 { 4 } else { 0 }),
            (140, 1),
        ] {
            header[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        output.write_all(&header)?;
        if faces == 1 || chunks.len() == 1 {
            for chunk in chunks {
                decode(reader, &chunk.segment, max, output)?;
            }
        } else {
            // DDS orders faces before mips. Spool multi-chunk cubes so each
            // chunk is decoded once, without retaining the whole cube in RAM.
            let mut scratch = tempfile::tempfile()?;
            let mut offsets = Vec::new();
            for chunk in &chunks {
                offsets.push(scratch.stream_position()?);
                decode(reader, &chunk.segment, max, &mut scratch)?;
            }
            for face in 0..faces {
                for (offset, size) in offsets.iter().zip(&per_face) {
                    scratch.seek(SeekFrom::Start(offset + face * size))?;
                    copy_exact(&mut (&mut scratch).take(*size), output, *size)?;
                }
            }
        }
        Ok(total)
    }
}
