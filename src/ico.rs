//! A small, dependency-free ICO encoder/reader.
//!
//! Why not rely on a generic encoder? ICO has quirks (256px is stored as `0`,
//! BMP entries use a doubled height and an AND mask, PNG entries are allowed
//! since Vista). Writing the container ourselves gives full control over each
//! entry's encoding and guarantees the requested sizes are really stored.

use crate::error::{Error, Result};
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, RgbaImage};

/// Largest dimension representable in an ICO directory entry.
pub const MAX_ICO_SIZE: u32 = 256;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// How each image is stored inside the ICO container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Encoding {
    /// PNG for 256px entries, classic 32-bit BMP/DIB for smaller ones.
    /// Matches what Windows itself ships; maximum compatibility. Default.
    #[default]
    Auto,
    /// PNG for every entry (smallest files, Vista+ only).
    Png,
    /// 32-bit BMP/DIB for every entry (legacy compatible, larger files).
    Bmp,
}

pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    PngEncoder::new_with_quality(&mut buf, CompressionType::Best, PngFilter::Adaptive).write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        ExtendedColorType::Rgba8,
    )?;
    Ok(buf)
}

/// Encode as an ICO "DIB" payload: BITMAPINFOHEADER + BGRA (bottom-up) + AND mask.
fn encode_bmp(img: &RgbaImage) -> Vec<u8> {
    let (w, h) = img.dimensions();
    let mask_stride = (w as usize).div_ceil(32) * 4;
    let xor_size = (w as usize) * (h as usize) * 4;
    let mask_size = mask_stride * h as usize;

    let mut out = Vec::with_capacity(40 + xor_size + mask_size);
    out.extend_from_slice(&40u32.to_le_bytes()); // biSize
    out.extend_from_slice(&(w as i32).to_le_bytes()); // biWidth
    out.extend_from_slice(&((h * 2) as i32).to_le_bytes()); // biHeight (XOR + AND)
    out.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    out.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    out.extend_from_slice(&0u32.to_le_bytes()); // biCompression = BI_RGB
    out.extend_from_slice(&((xor_size + mask_size) as u32).to_le_bytes()); // biSizeImage
    out.extend_from_slice(&0i32.to_le_bytes()); // biXPelsPerMeter
    out.extend_from_slice(&0i32.to_le_bytes()); // biYPelsPerMeter
    out.extend_from_slice(&0u32.to_le_bytes()); // biClrUsed
    out.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant

    for y in (0..h).rev() {
        for x in 0..w {
            let [r, g, b, a] = img.get_pixel(x, y).0;
            out.extend_from_slice(&[b, g, r, a]);
        }
    }

    // AND mask (1 = transparent). Redundant with alpha, but legacy readers use it.
    let mut mask = vec![0u8; mask_size];
    for y in 0..h {
        let row = (h - 1 - y) as usize;
        for x in 0..w {
            if img.get_pixel(x, y)[3] == 0 {
                mask[row * mask_stride + (x / 8) as usize] |= 0x80 >> (x % 8);
            }
        }
    }
    out.extend_from_slice(&mask);
    out
}

/// Serialize square RGBA images (`(size, image)`) into an ICO file.
///
/// Entries are written in ascending size order. Every image must be
/// `size` x `size` and `1 <= size <= 256`.
pub fn encode_ico(images: &[(u32, RgbaImage)], encoding: Encoding) -> Result<Vec<u8>> {
    if images.is_empty() {
        return Err(Error::NoSizes);
    }
    if images.len() > usize::from(u16::MAX) {
        return Err(Error::InvalidIco("too many images".into()));
    }

    let mut sorted: Vec<&(u32, RgbaImage)> = images.iter().collect();
    sorted.sort_by_key(|(s, _)| *s);

    let mut blobs = Vec::with_capacity(sorted.len());
    for (size, img) in &sorted {
        if *size == 0 || *size > MAX_ICO_SIZE {
            return Err(Error::InvalidSize(*size));
        }
        debug_assert_eq!(img.dimensions(), (*size, *size));
        let use_png = match encoding {
            Encoding::Png => true,
            Encoding::Bmp => false,
            Encoding::Auto => *size >= MAX_ICO_SIZE,
        };
        blobs.push(if use_png { encode_png(img)? } else { encode_bmp(img) });
    }

    let header_len = 6 + 16 * sorted.len();
    let total = header_len + blobs.iter().map(Vec::len).sum::<usize>();
    let mut out = Vec::with_capacity(total);

    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // type: icon
    out.extend_from_slice(&(sorted.len() as u16).to_le_bytes());

    let mut offset = header_len;
    for ((size, _), blob) in sorted.iter().zip(&blobs) {
        let dim = if *size >= MAX_ICO_SIZE { 0u8 } else { *size as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]); // w, h, palette, reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bit count
        out.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += blob.len();
    }
    for blob in &blobs {
        out.extend_from_slice(blob);
    }
    Ok(out)
}

/// Storage format of an ICO entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryFormat {
    Png,
    Bmp,
}

/// One parsed ICO directory entry.
#[derive(Debug, Clone)]
pub struct IcoEntry {
    pub width: u32,
    pub height: u32,
    pub bits_per_pixel: u16,
    pub bytes: u32,
    pub offset: u32,
    pub format: EntryFormat,
}

fn u16_at(d: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([d[i], d[i + 1]])
}
fn u32_at(d: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]])
}

/// Parse and validate the directory of an ICO file.
pub fn read_directory(data: &[u8]) -> Result<Vec<IcoEntry>> {
    let bad = |m: &str| Error::InvalidIco(m.to_string());
    if data.len() < 6 {
        return Err(bad("file too short"));
    }
    if u16_at(data, 0) != 0 {
        return Err(bad("reserved field is not zero"));
    }
    match u16_at(data, 2) {
        1 => {}
        2 => return Err(bad("this is a cursor (.cur), not an icon")),
        _ => return Err(bad("unknown resource type")),
    }
    let count = usize::from(u16_at(data, 4));
    if count == 0 {
        return Err(bad("contains no images"));
    }
    if data.len() < 6 + count * 16 {
        return Err(bad("truncated directory"));
    }

    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let b = 6 + i * 16;
        let bytes = u32_at(data, b + 8);
        let offset = u32_at(data, b + 12);
        let end = (offset as usize).checked_add(bytes as usize);
        if !end.is_some_and(|e| e <= data.len()) || bytes < 8 {
            return Err(bad("entry points outside the file"));
        }
        let payload = &data[offset as usize..offset as usize + bytes as usize];

        let (width, height, bpp, format) = if payload[..8] == PNG_SIGNATURE {
            if payload.len() < 26 {
                return Err(bad("truncated PNG entry"));
            }
            let w = u32::from_be_bytes([payload[16], payload[17], payload[18], payload[19]]);
            let h = u32::from_be_bytes([payload[20], payload[21], payload[22], payload[23]]);
            let channels = match payload[25] {
                0 => 1,
                2 => 3,
                3 => 1,
                4 => 2,
                6 => 4,
                _ => 0,
            };
            let bpp = u16::from(payload[24]) * channels;
            (w, h, bpp, EntryFormat::Png)
        } else {
            if payload.len() < 16 {
                return Err(bad("truncated BMP entry"));
            }
            let w = match data[b] {
                0 => 256,
                v => u32::from(v),
            };
            let h = match data[b + 1] {
                0 => 256,
                v => u32::from(v),
            };
            (w, h, u16_at(payload, 14), EntryFormat::Bmp)
        };
        entries.push(IcoEntry { width, height, bits_per_pixel: bpp, bytes, offset, format });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn sample(size: u32) -> (u32, RgbaImage) {
        let mut img = RgbaImage::from_pixel(size, size, Rgba([10, 200, 30, 255]));
        img.put_pixel(0, 0, Rgba([0, 0, 0, 0])); // one transparent pixel
        (size, img)
    }

    #[test]
    fn directory_roundtrip_all_encodings() {
        for enc in [Encoding::Auto, Encoding::Png, Encoding::Bmp] {
            let data = encode_ico(&[sample(32), sample(16), sample(256)], enc).unwrap();
            let dir = read_directory(&data).unwrap();
            let sizes: Vec<u32> = dir.iter().map(|e| e.width).collect();
            assert_eq!(sizes, vec![16, 32, 256], "{enc:?}");
            assert!(dir.iter().all(|e| e.height == e.width));
        }
    }

    #[test]
    fn auto_uses_bmp_small_png_large() {
        let data = encode_ico(&[sample(16), sample(256)], Encoding::Auto).unwrap();
        let dir = read_directory(&data).unwrap();
        assert_eq!(dir[0].format, EntryFormat::Bmp);
        assert_eq!(dir[1].format, EntryFormat::Png);
    }

    #[test]
    fn decodes_with_independent_decoder() {
        for enc in [Encoding::Png, Encoding::Bmp] {
            let data = encode_ico(&[sample(32)], enc).unwrap();
            let img = image::load_from_memory_with_format(&data, image::ImageFormat::Ico)
                .unwrap()
                .to_rgba8();
            assert_eq!(img.dimensions(), (32, 32));
            assert_eq!(img.get_pixel(16, 16).0, [10, 200, 30, 255], "{enc:?}");
            assert_eq!(img.get_pixel(0, 0)[3], 0, "{enc:?}");
        }
    }

    #[test]
    fn rejects_oversize_and_empty() {
        assert!(encode_ico(&[], Encoding::Auto).is_err());
        let big = (512, RgbaImage::new(512, 512));
        assert!(matches!(encode_ico(&[big], Encoding::Auto), Err(Error::InvalidSize(512))));
    }

    #[test]
    fn rejects_corrupt_input() {
        assert!(read_directory(&[]).is_err());
        assert!(read_directory(&[0, 0, 2, 0, 1, 0]).is_err()); // .cur
        let mut data = encode_ico(&[sample(16)], Encoding::Bmp).unwrap();
        data.truncate(data.len() - 10);
        assert!(read_directory(&data).is_err());
    }
}
