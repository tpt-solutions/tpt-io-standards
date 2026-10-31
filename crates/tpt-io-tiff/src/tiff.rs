//! TIFF IFD reading, strip assembly, and writing of uncompressed single-strip images.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::decode::lzw;
use crate::error::TiffError;

const TAG_IMAGE_WIDTH: u16 = 256;
const TAG_IMAGE_LENGTH: u16 = 257;
const TAG_BITS_PER_SAMPLE: u16 = 258;
const TAG_COMPRESSION: u16 = 259;
const TAG_IMAGE_DESCRIPTION: u16 = 270;
const TAG_STRIP_OFFSETS: u16 = 273;
const TAG_SAMPLES_PER_PIXEL: u16 = 277;
const TAG_ROWS_PER_STRIP: u16 = 278;
const TAG_STRIP_BYTE_COUNTS: u16 = 279;
const TAG_SAMPLE_FORMAT: u16 = 339;

/// One decoded image (one IFD): its size, sample layout and little-endian pixel bytes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels (rows).
    pub height: u32,
    /// Bits per sample (8, 16 or 32 are decoded).
    pub bits_per_sample: u16,
    /// Samples per pixel (1 for greyscale, 3 for RGB).
    pub samples_per_pixel: u16,
    /// Sample format: 1 unsigned, 2 signed, 3 IEEE float.
    pub sample_format: u16,
    /// The `ImageDescription` text, if present.
    pub description: Option<String>,
    /// Pixel data, little-endian, interleaved as stored.
    pub pixels: Vec<u8>,
}

#[derive(Clone, Copy)]
struct Entry {
    tag: u16,
    kind: u16,
    count: u32,
    value: u32,
}

struct Reader<'a> {
    b: &'a [u8],
    big: bool,
}

impl Reader<'_> {
    fn u16(&self, at: usize) -> Result<u16, ParseError> {
        let s = self.b.get(at..at + 2).ok_or_else(|| ParseError::eof(at))?;
        Ok(if self.big {
            u16::from_be_bytes([s[0], s[1]])
        } else {
            u16::from_le_bytes([s[0], s[1]])
        })
    }

    fn u32(&self, at: usize) -> Result<u32, ParseError> {
        let s = self.b.get(at..at + 4).ok_or_else(|| ParseError::eof(at))?;
        let a = [s[0], s[1], s[2], s[3]];
        Ok(if self.big { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) })
    }

    fn entry(&self, at: usize) -> Result<Entry, ParseError> {
        Ok(Entry {
            tag: self.u16(at)?,
            kind: self.u16(at + 2)?,
            count: self.u32(at + 4)?,
            value: self.u32(at + 8)?,
        })
    }

    /// The four bytes of an entry's value field, in file order.
    fn inline(&self, e: Entry) -> [u8; 4] {
        if self.big {
            e.value.to_be_bytes()
        } else {
            e.value.to_le_bytes()
        }
    }

    fn short_at(&self, s: &[u8]) -> u32 {
        if self.big {
            u16::from_be_bytes([s[0], s[1]]) as u32
        } else {
            u16::from_le_bytes([s[0], s[1]]) as u32
        }
    }

    /// The values of an entry as unsigned integers (SHORT or LONG), in order.
    fn values(&self, e: Entry) -> Result<Vec<u32>, ParseError> {
        let size = match e.kind {
            3 => 2usize,
            4 => 4usize,
            _ => return Err(ParseError::custom(0, "expected SHORT or LONG values")),
        };
        let count = e.count as usize;
        if size * count <= 4 {
            // The values sit inside the entry itself.
            let raw = self.inline(e);
            return Ok((0..count)
                .map(|i| if size == 2 { self.short_at(&raw[2 * i..2 * i + 2]) } else { e.value })
                .collect());
        }
        let base = e.value as usize;
        (0..count)
            .map(|i| {
                if size == 2 {
                    self.u16(base + 2 * i).map(u32::from)
                } else {
                    self.u32(base + 4 * i)
                }
            })
            .collect()
    }
}

/// Parse a TIFF file: every IFD in the chain, decoded.
pub fn parse(input: &[u8]) -> Result<Vec<Image>, TiffError> {
    if input.len() < 8 {
        return Err(ParseError::eof(input.len()).into());
    }
    let big = match &input[..2] {
        b"II" => false,
        b"MM" => true,
        _ => return Err(ParseError::custom(0, "missing TIFF byte-order mark").into()),
    };
    let r = Reader { b: input, big };
    match r.u16(2)? {
        42 => {}
        43 => return Err(TiffError::Unsupported("BigTIFF".to_string())),
        _ => return Err(ParseError::custom(2, "bad TIFF magic number").into()),
    }
    let mut out = Vec::new();
    let mut offset = r.u32(4)? as usize;
    while offset != 0 {
        let n = r.u16(offset)? as usize;
        let mut entries = Vec::with_capacity(n);
        for i in 0..n {
            entries.push(r.entry(offset + 2 + 12 * i)?);
        }
        out.push(image(&r, &entries)?);
        offset = r.u32(offset + 2 + 12 * n)? as usize;
    }
    if out.is_empty() {
        return Err(ParseError::custom(4, "no image file directory").into());
    }
    Ok(out)
}

fn single(r: &Reader<'_>, entries: &[Entry], tag: u16) -> Result<Option<u32>, ParseError> {
    match entries.iter().find(|e| e.tag == tag) {
        Some(e) => Ok(r.values(*e)?.first().copied()),
        None => Ok(None),
    }
}

fn image(r: &Reader<'_>, entries: &[Entry]) -> Result<Image, TiffError> {
    let width = single(r, entries, TAG_IMAGE_WIDTH)?
        .ok_or_else(|| TiffError::Invalid("no ImageWidth".into()))?;
    let height = single(r, entries, TAG_IMAGE_LENGTH)?
        .ok_or_else(|| TiffError::Invalid("no ImageLength".into()))?;
    let spp = single(r, entries, TAG_SAMPLES_PER_PIXEL)?.unwrap_or(1) as u16;
    let bits = match entries.iter().find(|e| e.tag == TAG_BITS_PER_SAMPLE) {
        Some(e) => r.values(*e)?.first().copied().unwrap_or(1) as u16,
        None => 1,
    };
    let format = single(r, entries, TAG_SAMPLE_FORMAT)?.unwrap_or(1) as u16;
    let compression = single(r, entries, TAG_COMPRESSION)?.unwrap_or(1);
    let description = entries.iter().find(|e| e.tag == TAG_IMAGE_DESCRIPTION).map(|e| {
        let bytes = if e.count as usize <= 4 {
            r.inline(*e)[..e.count as usize].to_vec()
        } else {
            r.b.get(e.value as usize..e.value as usize + e.count as usize).unwrap_or(&[]).to_vec()
        };
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).to_string()
    });
    let offsets = match entries.iter().find(|e| e.tag == TAG_STRIP_OFFSETS) {
        Some(e) => r.values(*e)?,
        None => return Err(TiffError::Unsupported("tiled images".into())),
    };
    let counts = match entries.iter().find(|e| e.tag == TAG_STRIP_BYTE_COUNTS) {
        Some(e) => r.values(*e)?,
        None => return Err(TiffError::Invalid("no StripByteCounts".into())),
    };
    if offsets.len() != counts.len() {
        return Err(TiffError::Invalid("strip offsets and byte counts differ".into()));
    }
    let bytes_per_sample = match bits {
        8 => 1,
        16 => 2,
        32 => 4,
        other => return Err(TiffError::Unsupported(alloc::format!("{other} bits per sample"))),
    };
    let expected = width as usize * height as usize * spp as usize * bytes_per_sample;
    let mut raw = Vec::with_capacity(expected);
    for (off, cnt) in offsets.iter().zip(&counts) {
        let strip =
            r.b.get(*off as usize..*off as usize + *cnt as usize)
                .ok_or_else(|| ParseError::eof(*off as usize))?;
        match compression {
            1 => raw.extend_from_slice(strip),
            5 => raw.extend(lzw(strip)?),
            other => return Err(TiffError::Unsupported(alloc::format!("compression {other}"))),
        }
    }
    if raw.len() < expected {
        return Err(TiffError::Invalid("strips hold fewer bytes than the image size".into()));
    }
    raw.truncate(expected);
    // Samples stored big-endian in a big-endian file are converted to little-endian.
    if r.big && bytes_per_sample > 1 {
        for chunk in raw.chunks_exact_mut(bytes_per_sample) {
            chunk.reverse();
        }
    }
    Ok(Image {
        width,
        height,
        bits_per_sample: bits,
        samples_per_pixel: spp,
        sample_format: format,
        description,
        pixels: raw,
    })
}

/// Serialise images as a little-endian TIFF with one uncompressed strip per image, one IFD each.
pub fn write(images: &[Image]) -> Result<Vec<u8>, TiffError> {
    if images.is_empty() {
        return Err(TiffError::Invalid("no images to write".into()));
    }
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"II");
    out.extend_from_slice(&42u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    let mut prev_next_at: Option<usize> = Some(4);
    for img in images {
        let bytes_per_sample = match img.bits_per_sample {
            8 => 1,
            16 => 2,
            32 => 4,
            other => return Err(TiffError::Unsupported(alloc::format!("{other} bits per sample"))),
        };
        let expected = img.width as usize
            * img.height as usize
            * img.samples_per_pixel as usize
            * bytes_per_sample;
        if img.pixels.len() != expected {
            return Err(TiffError::Invalid("pixel count does not match the image size".into()));
        }
        // Pad to even so the IFD starts on a word boundary.
        if out.len() % 2 == 1 {
            out.push(0);
        }
        let desc_bytes: Vec<u8> = img
            .description
            .as_deref()
            .map(|d| {
                let mut v = d.as_bytes().to_vec();
                v.push(0);
                v
            })
            .unwrap_or_default();
        let desc_at = out.len();
        out.extend_from_slice(&desc_bytes);
        if out.len() % 2 == 1 {
            out.push(0);
        }
        let strip_at = out.len();
        out.extend_from_slice(&img.pixels);
        if out.len() % 2 == 1 {
            out.push(0);
        }
        let ifd_at = out.len();
        let mut entries: Vec<(u16, u16, u32, u32)> = alloc::vec![
            (TAG_IMAGE_WIDTH, 4, 1, img.width),
            (TAG_IMAGE_LENGTH, 4, 1, img.height),
            (TAG_BITS_PER_SAMPLE, 3, 1, img.bits_per_sample as u32),
            (TAG_COMPRESSION, 3, 1, 1),
            (262, 3, 1, 1),
            (TAG_STRIP_OFFSETS, 4, 1, strip_at as u32),
            (TAG_SAMPLES_PER_PIXEL, 3, 1, img.samples_per_pixel as u32),
            (TAG_ROWS_PER_STRIP, 4, 1, img.height),
            (TAG_STRIP_BYTE_COUNTS, 4, 1, img.pixels.len() as u32),
            (TAG_SAMPLE_FORMAT, 3, 1, img.sample_format as u32),
        ];
        if !desc_bytes.is_empty() {
            entries.push((TAG_IMAGE_DESCRIPTION, 2, desc_bytes.len() as u32, desc_at as u32));
        }
        entries.sort_by_key(|e| e.0);
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        for (tag, kind, count, value) in &entries {
            out.extend_from_slice(&tag.to_le_bytes());
            out.extend_from_slice(&kind.to_le_bytes());
            out.extend_from_slice(&count.to_le_bytes());
            if *kind == 3 && *count == 1 {
                out.extend_from_slice(&(*value as u16).to_le_bytes());
                out.extend_from_slice(&[0, 0]);
            } else {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        let next_link = out.len();
        out.extend_from_slice(&0u32.to_le_bytes());
        if let Some(at) = prev_next_at {
            out[at..at + 4].copy_from_slice(&(ifd_at as u32).to_le_bytes());
        }
        prev_next_at = Some(next_link);
    }
    Ok(out)
}
