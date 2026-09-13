//! Bounded DDS inspection and selected-subresource decoding; no filesystem or GUI state.
use ddsfile::{
    AlphaMode, Caps2, D3D10ResourceDimension, D3DFormat, Dds, DxgiFormat, FourCC, Header, Header10,
    MiscFlag,
};
use std::io::Cursor;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Channel {
    #[default]
    Rgba,
    Rgb,
    Alpha,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Selection {
    pub mip: u32,
    pub layer: u32,
    /// Cube face order: +X, -X, +Y, -Y, +Z, -Z. Zero for ordinary textures.
    pub face: u32,
    pub channel: Channel,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdsInfo {
    pub width: u32,
    pub height: u32,
    pub mips: u32,
    /// Array elements; for cubemaps this is the number of cubes, not faces.
    pub layers: u32,
    pub faces: u32,
    pub format: String,
    pub hdr: bool,
}
#[derive(Debug)]
pub struct DecodedDds {
    pub info: DdsInfo,
    pub selection: Selection,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
pub const MAX_DDS_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_RGBA_BYTES: usize = 64 * 1024 * 1024;

/// Checks headers and complete subresource sizes without decoding or copying pixels.
pub fn inspect(bytes: &[u8]) -> Result<DdsInfo, String> {
    Ok(parse(bytes)?.info)
}
/// Decodes one mip/layer/face to straight RGBA8. BC6 uses fixed Reinhard/sRGB
/// display mapping; BC4/5 signed channels map [-1, 1] to [0, 255].
pub fn decode(bytes: &[u8], selection: Selection) -> Result<DecodedDds, String> {
    let parsed = parse(bytes)?;
    let info = parsed.info;
    if selection.mip >= info.mips || selection.layer >= info.layers || selection.face >= info.faces
    {
        return Err("DDS mip, array layer or cube face is out of range".into());
    }
    let width = (info.width >> selection.mip).max(1);
    let height = (info.height >> selection.mip).max(1);
    let output_size = u64::from(width) * u64::from(height) * 4;
    if output_size > MAX_RGBA_BYTES as u64 {
        return Err("DDS decoded image exceeds the 64 MiB limit; select a smaller mip".into());
    }
    let stride: usize = parsed.sizes.iter().sum();
    let offset = (selection.layer * info.faces + selection.face) as usize * stride
        + parsed.sizes[..selection.mip as usize].iter().sum::<usize>();
    let data = &parsed.data[offset..offset + parsed.sizes[selection.mip as usize]];
    let mut rgba = vec![0; output_size as usize];
    if parsed.block_size == 0 {
        rgba.copy_from_slice(data);
        if matches!(parsed.format as u32, 87 | 88 | 91 | 93) {
            for pixel in rgba.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
    } else {
        let columns = width.div_ceil(4) as usize;
        for (i, block) in data.chunks_exact(parsed.block_size).enumerate() {
            let pixels = decode_block(block, parsed.format)?;
            let x = (i % columns) * 4;
            let y = (i / columns) * 4;
            for row in 0..4.min(height as usize - y) {
                let count = 4.min(width as usize - x) * 4;
                let start = ((y + row) * width as usize + x) * 4;
                rgba[start..start + count].copy_from_slice(&pixels[row * 16..row * 16 + count]);
            }
        }
    }
    for pixel in rgba.chunks_exact_mut(4) {
        if parsed.alpha == AlphaMode::Opaque {
            pixel[3] = 255;
        } else if parsed.alpha == AlphaMode::PreMultiplied {
            let alpha = u32::from(pixel[3]);
            for value in &mut pixel[..3] {
                *value = if alpha == 0 {
                    0
                } else {
                    ((u32::from(*value) * 255 + alpha / 2) / alpha).min(255) as u8
                };
            }
        }
        match selection.channel {
            Channel::Rgba => {}
            Channel::Rgb => pixel[3] = 255,
            Channel::Alpha => {
                let alpha = pixel[3];
                pixel.copy_from_slice(&[alpha, alpha, alpha, 255]);
            }
        }
    }
    Ok(DecodedDds {
        info,
        selection,
        width,
        height,
        rgba,
    })
}

struct Parsed<'a> {
    info: DdsInfo,
    format: DxgiFormat,
    alpha: AlphaMode,
    block_size: usize,
    sizes: Vec<usize>,
    data: &'a [u8],
}
fn parse(bytes: &[u8]) -> Result<Parsed<'_>, String> {
    if bytes.len() > MAX_DDS_BYTES {
        return Err("DDS input exceeds the 128 MiB limit".into());
    }
    let rest = bytes.strip_prefix(b"DDS ").ok_or("Invalid DDS signature")?;
    let mut reader = Cursor::new(rest);
    let header = Header::read(&mut reader).map_err(|e| format!("Invalid DDS header: {e}"))?;
    let header10 = if header.spf.fourcc == Some(FourCC(FourCC::DX10)) {
        Some(Header10::read(&mut reader).map_err(|e| format!("Invalid DDS DX10 header: {e}"))?)
    } else {
        None
    };
    // Parse headers only: Dds::read would copy the entire caller-owned payload.
    let dds = Dds {
        header,
        header10,
        data: Vec::new(),
    };
    let h = &dds.header;
    if h.width == 0 || h.height == 0 {
        return Err("DDS dimensions must be nonzero".into());
    }
    if h.width > 16_384 || h.height > 16_384 {
        return Err("DDS dimensions exceed the 16384 limit".into());
    }
    if h.depth.unwrap_or(1) != 1 || h.caps2.contains(Caps2::VOLUME) {
        return Err("DDS volume textures are not supported".into());
    }
    let mips = h.mip_map_count.unwrap_or(1);
    if mips == 0 || mips > u32::BITS - h.width.max(h.height).leading_zeros() {
        return Err("Invalid DDS mip count".into());
    }
    let (layers, cube) = if let Some(h10) = &dds.header10 {
        match h10.resource_dimension {
            D3D10ResourceDimension::Texture2D => {}
            D3D10ResourceDimension::Texture1D
                if h.height == 1 && !h10.misc_flag.contains(MiscFlag::TEXTURECUBE) => {}
            _ => return Err("DDS resource dimension is not supported".into()),
        }
        (
            h10.array_size,
            h10.misc_flag.contains(MiscFlag::TEXTURECUBE),
        )
    } else {
        let cube = h.caps2.contains(Caps2::CUBEMAP);
        if cube && !h.caps2.contains(Caps2::CUBEMAP_ALLFACES)
            || !cube && h.caps2.intersects(Caps2::CUBEMAP_ALLFACES)
        {
            return Err("DDS partial cubemaps are not supported".into());
        }
        (1, cube)
    };
    if layers == 0 {
        return Err("DDS array is empty".into());
    }
    if layers > 2048 {
        return Err("DDS array exceeds the 2048 layer limit".into());
    }
    if cube && h.width != h.height {
        return Err("DDS cube faces must be square".into());
    }
    let (format, mut alpha) = format(&dds)?;
    let block_size = match format as u32 {
        28 | 29 | 87 | 88 | 91 | 93 => 0,
        71 | 72 | 80 | 81 => 8,
        74 | 75 | 77 | 78 | 83 | 84 | 95 | 96 | 98 | 99 => 16,
        _ => return Err(format!("DDS format {format:?} is not supported")),
    };
    if matches!(format as u32, 88 | 93) {
        alpha = AlphaMode::Opaque;
    }
    if alpha == AlphaMode::Custom {
        return Err("DDS custom alpha semantics are not supported".into());
    }
    if block_size == 0 && h.pitch.is_some_and(|p| p != h.width * 4) {
        return Err("DDS padded or invalid row pitch is not supported".into());
    }
    let sizes: Vec<usize> = (0..mips)
        .map(|mip| {
            let width = u64::from((h.width >> mip).max(1));
            let height = u64::from((h.height >> mip).max(1));
            if block_size == 0 {
                width * height * 4
            } else {
                width.div_ceil(4) * height.div_ceil(4) * block_size as u64
            }
        })
        .map(|size| usize::try_from(size).map_err(|_| "DDS size overflow".to_owned()))
        .collect::<Result<_, _>>()?;
    let faces = if cube { 6 } else { 1 };
    let expected = sizes
        .iter()
        .map(|&s| s as u64)
        .sum::<u64>()
        .checked_mul(u64::from(layers) * u64::from(faces))
        .ok_or("DDS size overflow")?;
    let data = &bytes[4 + reader.position() as usize..];
    if expected != data.len() as u64 {
        return Err("DDS payload size disagrees with its complete mip/array/face layout".into());
    }
    Ok(Parsed {
        info: DdsInfo {
            width: h.width,
            height: h.height,
            mips,
            layers,
            faces,
            format: format!("{format:?}"),
            hdr: matches!(format as u32, 95 | 96),
        },
        format,
        alpha,
        block_size,
        sizes,
        data,
    })
}
fn format(dds: &Dds) -> Result<(DxgiFormat, AlphaMode), String> {
    use DxgiFormat as F;
    if let Some(header) = &dds.header10 {
        return Ok((header.dxgi_format, header.alpha_mode));
    }
    let mut alpha = AlphaMode::Unknown;
    let format = if let Some(fourcc) = &dds.header.spf.fourcc {
        match &fourcc.0.to_le_bytes() {
            b"DXT1" => F::BC1_UNorm,
            b"DXT2" => {
                alpha = AlphaMode::PreMultiplied;
                F::BC2_UNorm
            }
            b"DXT3" => F::BC2_UNorm,
            b"DXT4" => {
                alpha = AlphaMode::PreMultiplied;
                F::BC3_UNorm
            }
            b"DXT5" => F::BC3_UNorm,
            b"ATI1" | b"BC4U" => F::BC4_UNorm,
            b"BC4S" => F::BC4_SNorm,
            b"ATI2" | b"BC5U" => F::BC5_UNorm,
            b"BC5S" => F::BC5_SNorm,
            _ => {
                return Err(format!(
                    "DDS FourCC {:?} is not supported",
                    String::from_utf8_lossy(&fourcc.0.to_le_bytes())
                ))
            }
        }
    } else {
        match dds.get_d3d_format() {
            Some(D3DFormat::A8B8G8R8) => F::R8G8B8A8_UNorm,
            Some(D3DFormat::A8R8G8B8) => F::B8G8R8A8_UNorm,
            Some(D3DFormat::X8B8G8R8) => {
                alpha = AlphaMode::Opaque;
                F::R8G8B8A8_UNorm
            }
            Some(D3DFormat::X8R8G8B8) => F::B8G8R8X8_UNorm,
            _ => return Err("DDS legacy pixel masks/format are not supported".into()),
        }
    };
    Ok((format, alpha))
}
fn decode_block(block: &[u8], format: DxgiFormat) -> Result<[u8; 64], String> {
    let mut rgba = [0; 64];
    match format as u32 {
        71 | 72 => bcdec_rs::bc1(block, &mut rgba, 16),
        74 | 75 => bcdec_rs::bc2(block, &mut rgba, 16),
        77 | 78 => bcdec_rs::bc3(block, &mut rgba, 16),
        80 | 81 | 83 | 84 => {
            let two = matches!(format as u32, 83 | 84);
            let signed = matches!(format as u32, 81 | 84);
            let mut rg = [0.0; 32];
            if two {
                bcdec_rs::bc5_float(block, &mut rg, 8, signed);
            } else {
                bcdec_rs::bc4_float(block, &mut rg, 4, signed);
            }
            for (i, pixel) in rgba.chunks_exact_mut(4).enumerate() {
                let display = |v: f32| unorm(if signed { v * 0.5 + 0.5 } else { v });
                let r = display(rg[i * if two { 2 } else { 1 }]);
                pixel.copy_from_slice(&if two {
                    [r, display(rg[i * 2 + 1]), 0, 255]
                } else {
                    [r, r, r, 255]
                });
            }
        }
        95 | 96 => {
            if matches!(block[0] & 31, 19 | 23 | 27 | 31) {
                return Err("DDS BC6 block uses a reserved mode".into());
            }
            let mut rgb = [0.0; 48];
            bcdec_rs::bc6h_float(block, &mut rgb, 12, format as u32 == 96);
            for (source, pixel) in rgb.chunks_exact(3).zip(rgba.chunks_exact_mut(4)) {
                pixel.copy_from_slice(&[
                    hdr_channel(source[0]),
                    hdr_channel(source[1]),
                    hdr_channel(source[2]),
                    255,
                ]);
            }
        }
        98 | 99 => {
            if block[0] == 0 {
                return Err("DDS BC7 block has no valid mode".into());
            }
            bcdec_rs::bc7(block, &mut rgba, 16);
        }
        _ => return Err("Unsupported DDS block format".into()),
    }
    Ok(rgba)
}
fn unorm(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
fn hdr_channel(value: f32) -> u8 {
    // BC6 is linear HDR: clip negative light, Reinhard-map, then encode sRGB.
    let positive = value.max(0.0);
    let linear = if positive.is_infinite() {
        1.0
    } else {
        positive / (1.0 + positive)
    };
    unorm(if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(b: &mut [u8], at: usize, n: u32) {
        b[at..at + 4].copy_from_slice(&n.to_le_bytes());
    }
    fn dx10(
        format: u32,
        width: u32,
        height: u32,
        mips: u32,
        layers: u32,
        cube: bool,
        data: &[u8],
    ) -> Vec<u8> {
        let mut b = vec![0; 148];
        b[..4].copy_from_slice(b"DDS ");
        for (at, n) in [
            (4, 124),
            (8, 0x21007),
            (12, height),
            (16, width),
            (28, mips),
            (76, 32),
            (80, 4),
            (84, u32::from_le_bytes(*b"DX10")),
            (108, 0x401008),
            (112, if cube { 0xfe00 } else { 0 }),
            (128, format),
            (132, 3),
            (136, if cube { 4 } else { 0 }),
            (140, layers),
        ] {
            put(&mut b, at, n);
        }
        b.extend_from_slice(data);
        b
    }
    fn pixels(format: u32, block: &[u8], expected: [u8; 4]) {
        let d = decode(
            &dx10(format, 4, 4, 1, 1, false, block),
            Selection::default(),
        )
        .unwrap();
        assert_eq!((d.width, d.height), (4, 4));
        assert_eq!(d.rgba, expected.repeat(16));
    }
    #[test]
    fn dds_golden_bc1_through_bc7_and_signed_channels() {
        let red = [0, 248, 0, 0, 0, 0, 0, 0];
        for format in [71, 72] {
            pixels(format, &red, [255, 0, 0, 255]);
        }
        pixels(71, &[0, 0, 0, 0, 255, 255, 255, 255], [0, 0, 0, 0]);
        for format in [74, 75] {
            pixels(
                format,
                &[[0x88; 8].as_slice(), &red].concat(),
                [255, 0, 0, 136],
            );
        }
        for format in [77, 78] {
            pixels(
                format,
                &[[128, 0, 0, 0, 0, 0, 0, 0].as_slice(), &red].concat(),
                [255, 0, 0, 128],
            );
        }
        pixels(80, &[128, 0, 0, 0, 0, 0, 0, 0], [128, 128, 128, 255]);
        pixels(81, &[129, 127, 0, 0, 0, 0, 0, 0], [0, 0, 0, 255]);
        pixels(81, &[0, 127, 0, 0, 0, 0, 0, 0], [128, 128, 128, 255]);
        let rg = [255, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0];
        pixels(83, &rg, [255, 64, 0, 255]);
        let signed_rg = [129, 127, 0, 0, 0, 0, 0, 0, 127, 129, 0, 0, 0, 0, 0, 0];
        pixels(84, &signed_rg, [0, 255, 0, 255]);
        // BC7 mode 6, all endpoint bits including P-bits set: opaque white.
        let mut white = [255; 16];
        white[0] = 0xc0;
        for format in [98, 99] {
            pixels(format, &white, [255, 255, 255, 255]);
        }
        let mut red_alpha = 64u128;
        for (i, endpoint) in [127u128, 127, 0, 0, 0, 0, 64, 64].into_iter().enumerate() {
            red_alpha |= endpoint << (7 + i * 7);
        }
        pixels(98, &red_alpha.to_le_bytes(), [254, 0, 0, 128]);
        // BC6 mode 11 has six explicit 10-bit endpoints and zero indices.
        let block = |r: u128| (3u128 | r << 5 | r << 35).to_le_bytes();
        // Endpoint 495 unquantizes to half 0x3c00 (1.0); Reinhard+sRGB => 188.
        pixels(95, &block(495), [188, 0, 0, 255]);
        pixels(95, &block(1023), [255, 0, 0, 255]);
        pixels(96, &block(1023), [0, 0, 0, 255]);
    }
    #[test]
    fn dds_rgba_bgra_alpha_and_premultiplication() {
        for (format, data) in [
            (28, [100, 50, 20, 128]),
            (29, [100, 50, 20, 128]),
            (87, [20, 50, 100, 128]),
            (91, [20, 50, 100, 128]),
        ] {
            let b = dx10(format, 1, 1, 1, 1, false, &data);
            assert_eq!(
                decode(&b, Selection::default()).unwrap().rgba,
                [100, 50, 20, 128]
            );
            assert_eq!(
                decode(
                    &b,
                    Selection {
                        channel: Channel::Rgb,
                        ..Selection::default()
                    }
                )
                .unwrap()
                .rgba,
                [100, 50, 20, 255]
            );
            assert_eq!(
                decode(
                    &b,
                    Selection {
                        channel: Channel::Alpha,
                        ..Selection::default()
                    }
                )
                .unwrap()
                .rgba,
                [128, 128, 128, 255]
            );
        }
        let mut b = dx10(28, 1, 1, 1, 1, false, &[32, 16, 8, 128]);
        put(&mut b, 144, 2);
        assert_eq!(
            decode(&b, Selection::default()).unwrap().rgba,
            [64, 32, 16, 128]
        );
        put(&mut b, 144, 3);
        assert_eq!(
            decode(&b, Selection::default()).unwrap().rgba,
            [32, 16, 8, 255]
        );
    }
    #[test]
    fn dds_cube_arrays_and_mips_use_face_before_mip_offsets() {
        let mut data = Vec::new();
        for layer in 0..2 {
            for face in 0..6 {
                data.extend_from_slice(&[layer, face, 10, 255].repeat(4));
                data.extend_from_slice(&[layer, face, 11, 255]);
            }
        }
        let b = dx10(28, 2, 2, 2, 2, true, &data);
        let info = inspect(&b).unwrap();
        assert_eq!((info.mips, info.layers, info.faces), (2, 2, 6));
        for layer in 0..2 {
            for face in 0..6 {
                for mip in 0..2 {
                    let selection = Selection {
                        layer,
                        face,
                        mip,
                        channel: Channel::Rgba,
                    };
                    let out = decode(&b, selection).unwrap();
                    assert_eq!(out.selection, selection);
                    assert_eq!(out.info, info);
                    assert_eq!(
                        out.rgba,
                        [layer as u8, face as u8, 10 + mip as u8, 255].repeat(if mip == 0 {
                            4
                        } else {
                            1
                        })
                    );
                }
            }
        }
        for selection in [
            Selection {
                layer: 2,
                ..Selection::default()
            },
            Selection {
                face: 6,
                ..Selection::default()
            },
            Selection {
                mip: 2,
                ..Selection::default()
            },
        ] {
            assert!(decode(&b, selection).is_err());
        }
        let ordinary = dx10(28, 1, 1, 1, 2, false, &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(
            decode(
                &ordinary,
                Selection {
                    layer: 1,
                    ..Selection::default()
                }
            )
            .unwrap()
            .rgba,
            [5, 6, 7, 8]
        );
        let mut one_dimensional = ordinary;
        put(&mut one_dimensional, 132, 2);
        assert_eq!(inspect(&one_dimensional).unwrap().layers, 2);
        put(&mut one_dimensional, 12, 2);
        assert!(inspect(&one_dimensional).is_err());
    }
    #[test]
    fn dds_non_multiple_block_dimensions_crop_each_row() {
        let red = [0, 248, 0, 0, 0, 0, 0, 0];
        let blue = [31, 0, 0, 0, 0, 0, 0, 0];
        let b = dx10(71, 5, 3, 1, 1, false, &[red, blue].concat());
        let output = decode(&b, Selection::default()).unwrap();
        let row = [[255, 0, 0, 255].repeat(4), vec![0, 0, 255, 255]].concat();
        assert_eq!(output.rgba, row.repeat(3));
    }
    #[test]
    fn dds_legacy_headers_and_full_cubes_decode() {
        for (fourcc, data, expected) in [
            (*b"DXT1", vec![0, 248, 0, 0, 0, 0, 0, 0], [255, 0, 0, 255]),
            (*b"ATI1", vec![64, 0, 0, 0, 0, 0, 0, 0], [64, 64, 64, 255]),
            (*b"BC4S", vec![129, 127, 0, 0, 0, 0, 0, 0], [0, 0, 0, 255]),
        ] {
            let mut b = dx10(71, 4, 4, 1, 1, false, &[]);
            b.truncate(128);
            put(&mut b, 84, u32::from_le_bytes(fourcc));
            b.extend_from_slice(&data);
            assert_eq!(
                decode(&b, Selection::default()).unwrap().rgba,
                expected.repeat(16)
            );
        }
        for (red_mask, blue_mask, data) in
            [(255, 0xff0000, [1, 2, 3, 4]), (0xff0000, 255, [3, 2, 1, 4])]
        {
            let mut b = dx10(28, 1, 1, 1, 1, true, &[]);
            b.truncate(128);
            for (at, n) in [
                (80, 0x41),
                (84, 0),
                (88, 32),
                (92, red_mask),
                (96, 0xff00),
                (100, blue_mask),
                (104, 0xff000000),
            ] {
                put(&mut b, at, n);
            }
            b.extend_from_slice(&data.repeat(6));
            assert_eq!(inspect(&b).unwrap().faces, 6);
            assert_eq!(
                decode(
                    &b,
                    Selection {
                        face: 5,
                        ..Selection::default()
                    }
                )
                .unwrap()
                .rgba,
                [1, 2, 3, 4]
            );
            put(&mut b, 112, 0x600);
            assert!(inspect(&b).is_err());
        }
    }
    #[test]
    fn dds_rejects_truncation_layout_overflow_unsupported_and_bad_blocks() {
        let good = dx10(28, 1, 1, 1, 1, false, &[1, 2, 3, 4]);
        for end in 0..good.len() {
            assert!(inspect(&good[..end]).is_err(), "truncated at {end}");
        }
        let mut trailing = good.clone();
        trailing.push(0);
        assert!(inspect(&trailing).is_err());
        for (at, value) in [
            (4, 0),
            (12, 0),
            (16, 0),
            (16, u32::MAX),
            (28, 0),
            (28, 32),
            (76, 0),
            (128, 70),
            (132, 4),
            (140, 0),
            (140, u32::MAX),
        ] {
            let mut b = good.clone();
            put(&mut b, at, value);
            assert!(inspect(&b).is_err(), "field {at}={value}");
        }
        let mut padded = good.clone();
        put(&mut padded, 8, 0x2100f);
        put(&mut padded, 20, 8);
        assert!(inspect(&padded).is_err());
        let mut reserved = [0; 16];
        reserved[0] = 0x13;
        assert!(decode(
            &dx10(95, 4, 4, 1, 1, false, &reserved),
            Selection::default()
        )
        .is_err());
        assert!(decode(&dx10(98, 4, 4, 1, 1, false, &[0; 16]), Selection::default()).is_err());
        // A small BC1 input must not permit a 128-MiB decoded allocation.
        let huge = dx10(71, 8192, 4096, 1, 1, false, &vec![0; 16 * 1024 * 1024]);
        assert!(inspect(&huge).is_ok());
        assert!(decode(&huge, Selection::default())
            .unwrap_err()
            .contains("limit"));
    }

    #[test]
    fn dds_archive_generated_multichunk_cube_decodes_selected_face_and_mip() {
        use std::io::Write;
        let mut archive = vec![0; 96];
        archive[..4].copy_from_slice(b"BTDX");
        archive[8..12].copy_from_slice(b"DX10");
        put(&mut archive, 4, 1);
        put(&mut archive, 12, 1);
        put(&mut archive, 16, 216);
        archive[37] = 2;
        archive[38] = 24;
        archive[40] = 2;
        archive[42] = 2;
        archive[44] = 2;
        archive[45] = 28;
        archive[46] = 1;
        archive[47] = 8;
        put(&mut archive, 48, 96);
        put(&mut archive, 60, 96);
        put(&mut archive, 72, 192);
        put(&mut archive, 84, 24);
        archive[88] = 1;
        archive[90] = 1;
        for face in 0..6 {
            archive.extend_from_slice(&[face, 40, 20, 255].repeat(4));
        }
        for face in 0..6 {
            archive.extend_from_slice(&[face, 41, 21, 255]);
        }
        archive.extend_from_slice(b"\x05\0a.dds");
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("eidos-dds-{}-{nonce}.ba2", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        std::fs::remove_file(path).unwrap();
        file.write_all(&archive).unwrap();
        let mut bytes = Vec::new();
        eidos_conflicts::write_archive_member(&mut file, "a.dds", 1024, &mut bytes).unwrap();
        let out = decode(
            &bytes,
            Selection {
                face: 5,
                mip: 1,
                ..Selection::default()
            },
        )
        .unwrap();
        assert_eq!((out.info.layers, out.info.faces, out.info.mips), (1, 6, 2));
        assert_eq!(out.rgba, [5, 41, 21, 255]);
        assert_eq!(
            decode(
                &bytes,
                Selection {
                    face: 4,
                    ..Selection::default()
                }
            )
            .unwrap()
            .rgba,
            [4, 40, 20, 255].repeat(4)
        );
    }

    #[test]
    fn dds_smaller_mip_can_preview_when_base_exceeds_allocation_bound() {
        let bytes = dx10(71, 8192, 4096, 2, 1, false, &vec![0; 20 * 1024 * 1024]);
        assert!(decode(&bytes, Selection::default()).is_err());
        let out = decode(
            &bytes,
            Selection {
                mip: 1,
                ..Selection::default()
            },
        )
        .unwrap();
        assert_eq!(
            (out.width, out.height, out.rgba.len()),
            (4096, 2048, 32 * 1024 * 1024)
        );
        assert!(out.rgba.chunks_exact(4).all(|p| p == [0, 0, 0, 255]));
        let oversized = vec![0; MAX_DDS_BYTES + 1];
        assert!(inspect(&oversized).unwrap_err().contains("128 MiB limit"));
    }
}
