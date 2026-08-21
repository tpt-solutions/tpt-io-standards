//! Pixel data extraction (PS3.3 C.7.6.3 / PS3.5 Annex A + G).
//!
//! - Native (uncompressed) pixel data: direct byte slicing into frames.
//! - Encapsulated pixel data: fragment extraction from the item structure.
//! - RLE Lossless decompression implemented in pure Rust (PS3.5 Annex G).
//! - JPEG-family compressed data: fragments are extracted, but decoding
//!   requires an external image backend and returns
//!   [`DicomError::Unsupported`] by design.

use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use crate::dataset::Dataset;
use crate::error::{refs, DicomError, Result};
use crate::tag::Tag;
use crate::uid::TransferSyntax;

/// Tag `(7FE0,0010)` — Pixel Data.
pub const PIXEL_DATA: Tag = Tag::new(0x7FE0, 0x0010);
/// Tag `(0028,0008)` — Number of Frames.
const NUMBER_OF_FRAMES: Tag = Tag::new(0x0028, 0x0008);
/// Tag `(0028,0010)` — Rows.
const ROWS: Tag = Tag::new(0x0028, 0x0010);
/// Tag `(0028,0011)` — Columns.
const COLUMNS: Tag = Tag::new(0x0028, 0x0011);
/// Tag `(0028,0002)` — Samples per Pixel.
const SAMPLES_PER_PIXEL: Tag = Tag::new(0x0028, 0x0002);
/// Tag `(0028,0100)` — Bits Allocated.
const BITS_ALLOCATED: Tag = Tag::new(0x0028, 0x0100);

/// Geometry needed to slice native pixel data into frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameGeometry {
    /// Rows per frame.
    pub rows: u16,
    /// Columns per frame.
    pub columns: u16,
    /// Samples per pixel (1 grayscale, 3 RGB...).
    pub samples_per_pixel: u16,
    /// Bits allocated (8 or 16 typically).
    pub bits_allocated: u16,
    /// Number of frames (>= 1).
    pub number_of_frames: u32,
}

impl FrameGeometry {
    /// Bytes in one uncompressed frame.
    pub fn frame_bytes(&self) -> usize {
        self.rows as usize
            * self.columns as usize
            * self.samples_per_pixel as usize
            * self.bits_allocated.div_ceil(8) as usize
    }
}

/// Read the frame geometry from image-pixel module tags.
pub fn frame_geometry(ds: &Dataset) -> Result<FrameGeometry> {
    let get_u16 = |t: Tag| -> Result<u16> {
        ds.u16_at(t).ok_or_else(|| DicomError::MissingRequiredElement { tag: t.to_string() })
    };
    let rows = get_u16(ROWS)?;
    let columns = get_u16(COLUMNS)?;
    let samples = get_u16(SAMPLES_PER_PIXEL)?;
    let bits = get_u16(BITS_ALLOCATED)?;
    let frames = match ds.is_at(NUMBER_OF_FRAMES) {
        Some(n) if n > 0 => n as u32,
        _ => 1,
    };
    Ok(FrameGeometry {
        rows,
        columns,
        samples_per_pixel: samples,
        bits_allocated: bits,
        number_of_frames: frames,
    })
}

/// Extract raw pixel-data bytes plus the frame geometry for uncompressed
/// transfer syntaxes. Returns the verbatim value bytes.
pub fn native_pixel_data(ds: &Dataset) -> Result<(&[u8], FrameGeometry)> {
    let elem = ds
        .element(PIXEL_DATA)
        .ok_or_else(|| DicomError::MissingRequiredElement { tag: PIXEL_DATA.to_string() })?;
    let bytes = elem.value.as_raw().ok_or(DicomError::Unsupported(
        "pixel data is encapsulated; use `encapsulated_fragments` instead",
    ))?;
    Ok((bytes, frame_geometry(ds)?))
}

/// Extract the compressed byte-stream fragments of encapsulated pixel data
/// (PS3.5 Annex A.4): the basic offset table is returned first, then each
/// fragment's payload (item headers stripped).
pub fn encapsulated_fragments(ds: &Dataset) -> Result<(Vec<u32>, Vec<Vec<u8>>)> {
    let elem = ds
        .element(PIXEL_DATA)
        .ok_or_else(|| DicomError::MissingRequiredElement { tag: PIXEL_DATA.to_string() })?;
    let raw = elem
        .value
        .as_raw()
        .ok_or(DicomError::Unsupported("pixel data is native, not encapsulated"))?;
    parse_fragment_items(raw)
}

/// Parse `(FFFE,E000)` items out of an undefined-length OB value body.
fn parse_fragment_items(raw: &[u8]) -> Result<(Vec<u32>, Vec<Vec<u8>>)> {
    let mut offset_table = Vec::new();
    let mut fragments = Vec::new();
    let mut pos = 0usize;
    let mut first_item = true;
    while pos + 8 <= raw.len() {
        let group = u16::from_le_bytes([raw[pos], raw[pos + 1]]);
        let element = u16::from_le_bytes([raw[pos + 2], raw[pos + 3]]);
        if group != 0xFFFE || element != 0xE000 {
            return Err(DicomError::Core(
                tpt_io_core::ParseError::token(pos, Some("(fffe,e000) fragment item"), "other tag")
                    .with_spec(refs::PS3_5),
            ));
        }
        let len =
            u32::from_le_bytes([raw[pos + 4], raw[pos + 5], raw[pos + 6], raw[pos + 7]]) as usize;
        let payload_start = pos + 8;
        let payload_end = payload_start
            .checked_add(len)
            .ok_or_else(|| DicomError::invalid_value(payload_start, "fragment length overflows"))?;
        if payload_end > raw.len() {
            return Err(DicomError::Core(
                tpt_io_core::ParseError::eof(payload_end).with_spec(refs::PS3_5),
            ));
        }
        let payload = raw[payload_start..payload_end].to_vec();
        if first_item {
            // Basic Offset Table: u32 offsets (may be empty).
            offset_table = payload
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            first_item = false;
        } else {
            fragments.push(payload);
        }
        pos = payload_end;
    }
    Ok((offset_table, fragments))
}

/// Decode one RLE-compressed image fragment (PS3.5 Annex G).
///
/// The 64-byte header lists up to 15 segment byte offsets; each segment
/// encodes one byte-plane with PackBits-style RLE: a signed header byte
/// `n` means a literal run of `-n` following bytes when negative, or
/// `(n + 1)` repeats of the next byte when non-negative.
pub fn rle_decompress(fragment: &[u8], expected_size: usize) -> Result<Vec<u8>> {
    if fragment.len() < 64 {
        return Err(DicomError::invalid_value(0, "RLE fragment shorter than 64-byte header"));
    }
    let segment_count = u32::from_le_bytes([fragment[0], fragment[1], fragment[2], fragment[3]]);
    if segment_count == 0 || segment_count > 15 {
        return Err(DicomError::invalid_value(0, "RLE segment count out of range 1..=15"));
    }
    let mut out = Vec::with_capacity(expected_size);
    for s in 0..segment_count as usize {
        let le = |i: usize| {
            u32::from_le_bytes([fragment[i], fragment[i + 1], fragment[i + 2], fragment[i + 3]])
                as usize
        };
        // Header layout: u32 segment count, then one u32 offset per segment.
        let start = le((s + 1) * 4);
        let end = if s + 2 <= segment_count as usize { le((s + 2) * 4) } else { fragment.len() };
        if start < 64 || start > end || end > fragment.len() {
            return Err(DicomError::invalid_value(s * 4, "invalid RLE segment offsets"));
        }
        rle_decode_segment(&fragment[start..end], &mut out)?;
    }
    if out.len() != expected_size {
        return Err(DicomError::invalid_value(
            0,
            alloc::format!("RLE output size {} != expected {expected_size}", out.len()),
        ));
    }
    Ok(out)
}

/// Decode one RLE segment (one byte plane per Annex G).
fn rle_decode_segment(segment: &[u8], out: &mut Vec<u8>) -> Result<()> {
    let mut i = 0usize;
    while i < segment.len() {
        let control = segment[i] as i8;
        i += 1;
        if control >= 0 {
            // Repeat run: (c + 1) copies of the next byte.
            let count = control as usize + 1;
            if i + 1 > segment.len() {
                return Err(DicomError::invalid_value(i, "RLE repeat run exceeds segment"));
            }
            let byte = segment[i];
            i += 1;
            out.extend(core::iter::repeat(byte).take(count));
        } else if control != -128 {
            // Literal run of -c following bytes (PackBits convention).
            let count = -(control as i32) as usize;
            if i + count > segment.len() {
                return Err(DicomError::invalid_value(i, "RLE literal run exceeds segment"));
            }
            out.extend_from_slice(&segment[i..i + count]);
            i += count;
        } // -128 is a no-op per PackBits
    }
    Ok(())
}

/// Encode one byte-plane with the DICOM RLE scheme (tests + writing).
pub fn rle_encode_segment(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 2 + 16);
    let mut i = 0usize;
    while i < data.len() {
        let mut run = 1usize;
        while i + run < data.len() && data[i + run] == data[i] && run < 128 {
            run += 1;
        }
        if run >= 2 {
            // Repeat control: n - 1 encoded as u8, n in 2..=128.
            out.push((run as u8) - 1);
            out.push(data[i]);
            i += run;
        } else {
            let mut literal = 1usize;
            // Cap at 127 so the control byte stays in -127..-1 (DICOM RLE
            // has no -128 literal; that value is reserved/undefined).
            while i + literal < data.len()
                && literal < 127
                && data[i + literal] != data[i + literal - 1]
            {
                literal += 1;
            }
            // Literal control byte: -count (PackBits convention).
            out.push((literal as i32).wrapping_neg() as u8);
            out.extend_from_slice(&data[i..i + literal]);
            i += literal;
        }
    }
    out
}

/// Build a complete RLE fragment (header + one segment) from a buffer.
pub fn rle_encode_single_plane(data: &[u8]) -> Vec<u8> {
    let mut fragment = vec![0u8; 64];
    fragment[0..4].copy_from_slice(&1u32.to_le_bytes()); // one segment
    fragment[4..8].copy_from_slice(&64u32.to_le_bytes()); // segment starts at 64
    let mut segment = rle_encode_segment(data);
    fragment.append(&mut segment);
    fragment
}

/// Decompress all frames of an RLE Lossless image into native pixel bytes.
///
/// Assumes the common layout of one frame per fragment; each decoded
/// fragment must be exactly `frame_bytes` long.
pub fn decode_rle_frames(ds: &Dataset) -> Result<Vec<Vec<u8>>> {
    let geo = frame_geometry(ds)?;
    let frame_size = geo.frame_bytes();
    let (_, fragments) = encapsulated_fragments(ds)?;
    fragments.iter().map(|f| rle_decompress(f, frame_size)).collect()
}

/// Streaming frame access over native pixel data with a progress hook
/// (`tpt-io-core::Progress`) — for large volumetric CT/MRI datasets.
pub struct FrameStreamer<'a> {
    bytes: &'a [u8],
    geometry: FrameGeometry,
    next_frame: u32,
    total_bytes: u64,
    progress: &'a dyn tpt_io_core::Progress,
}

impl<'a> FrameStreamer<'a> {
    /// Build a streamer over the native pixel data of `ds`.
    pub fn new(ds: &'a Dataset, progress: &'a dyn tpt_io_core::Progress) -> Result<Self> {
        let (bytes, geometry) = native_pixel_data(ds)?;
        let total_bytes = geometry.frame_bytes() as u64 * geometry.number_of_frames as u64;
        Ok(Self { bytes, geometry, next_frame: 0, total_bytes, progress })
    }

    /// Total number of frames.
    pub fn total_frames(&self) -> u32 {
        self.geometry.number_of_frames
    }

    /// Bytes per frame.
    pub fn frame_size(&self) -> usize {
        self.geometry.frame_bytes()
    }

    /// Next frame's raw bytes, or `None` when exhausted.
    pub fn next_frame(&mut self) -> Option<&'a [u8]> {
        if self.next_frame >= self.geometry.number_of_frames {
            return None;
        }
        let size = self.geometry.frame_bytes();
        let start = self.next_frame as usize * size;
        if start + size > self.bytes.len() {
            self.next_frame = self.geometry.number_of_frames; // truncated input: stop
            return None;
        }
        self.next_frame += 1;
        self.progress.on_progress(self.next_frame as u64 * size as u64, Some(self.total_bytes));
        Some(&self.bytes[start..start + size])
    }
}

/// Decompress pixel data into per-frame native bytes for transfer syntaxes
/// this crate fully supports.
///
/// RLE Lossless is decoded in pure Rust. JPEG-family transfer syntaxes
/// return [`DicomError::UnsupportedTransferSyntax`] because they require an
/// external image codec backend (see `tpt-sci-image` integration plans).
pub fn decode_pixel_data(ds: &Dataset, ts: TransferSyntax) -> Result<Vec<Vec<u8>>> {
    match ts {
        TransferSyntax::ImplicitVrLittleEndian
        | TransferSyntax::ExplicitVrLittleEndian
        | TransferSyntax::ExplicitVrBigEndian => {
            let (bytes, geo) = native_pixel_data(ds)?;
            let size = geo.frame_bytes();
            if size == 0 {
                return Err(DicomError::invalid_value(0, "zero-sized frame"));
            }
            Ok(bytes.chunks(size).map(<[u8]>::to_vec).collect())
        }
        TransferSyntax::RleLossless => decode_rle_frames(ds),
        other => Err(DicomError::UnsupportedTransferSyntax(other.uid().into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::{Dataset, Encoding};
    use crate::element::{DataElement, Value};
    use crate::vr::Vr;

    fn ds_with_pixel(pixel: Vec<u8>, encapsulated: bool) -> Dataset {
        let mut ds = Dataset {
            elements: Vec::new(),
            encoding: Encoding { big_endian: false, explicit_vr: true },
        };
        ds.upsert(DataElement::us(ROWS, 2));
        ds.upsert(DataElement::us(COLUMNS, 2));
        ds.upsert(DataElement::us(SAMPLES_PER_PIXEL, 1));
        ds.upsert(DataElement::us(BITS_ALLOCATED, 8));
        ds.upsert(DataElement::text(NUMBER_OF_FRAMES, Vr::Is, "3"));
        let vr = Vr::Ob;
        ds.elements.push(DataElement {
            tag: PIXEL_DATA,
            vr,
            value: if encapsulated {
                Value::Raw(encapsulate(&[vec![0xAA], pixel]))
            } else {
                Value::Raw(pixel)
            },
        });
        ds
    }

    /// Wrap fragments in an undefined-length OB item structure.
    fn encapsulate(fragments: &[Vec<u8>]) -> Vec<u8> {
        let mut raw = Vec::new();
        for f in fragments {
            raw.extend_from_slice(&0xFFFEu16.to_le_bytes());
            raw.extend_from_slice(&0xE000u16.to_le_bytes());
            raw.extend_from_slice(&(f.len() as u32).to_le_bytes());
            raw.extend_from_slice(f);
        }
        raw
    }

    #[test]
    fn native_frames_streamed_with_progress() {
        // 3 frames of 4 bytes each.
        let ds = ds_with_pixel(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], false);
        let seen = core::cell::RefCell::new(alloc::vec::Vec::new());
        let progress = tpt_io_core::ProgressFn(|b, t| seen.borrow_mut().push((b, t)));
        let mut streamer = FrameStreamer::new(&ds, &progress).unwrap();
        assert_eq!(streamer.total_frames(), 3);
        assert_eq!(streamer.frame_size(), 4);
        assert_eq!(streamer.next_frame(), Some(&[1u8, 2, 3, 4][..]));
        assert_eq!(streamer.next_frame(), Some(&[5u8, 6, 7, 8][..]));
        assert_eq!(streamer.next_frame(), Some(&[9u8, 10, 11, 12][..]));
        assert_eq!(streamer.next_frame(), None);
        let seen = seen.into_inner();
        assert_eq!(seen.len(), 3); // progress fired per frame
        assert_eq!(seen[2], (12, Some(12)));
    }

    #[test]
    fn truncated_native_pixel_stops_early() {
        let ds = ds_with_pixel(vec![1, 2, 3], false); // less than one frame
        let mut streamer = FrameStreamer::new(&ds, &tpt_io_core::NoProgress).unwrap();
        assert_eq!(streamer.next_frame(), None);
    }

    #[test]
    fn rle_roundtrip_randomish_data() {
        // A pattern with runs and literals exercises both code paths.
        let mut data = Vec::new();
        data.extend(core::iter::repeat(7u8).take(130));
        data.push(9);
        data.extend(core::iter::repeat(3u8).take(5));
        data.extend(0u8..=200);
        let fragment = rle_encode_single_plane(&data);
        let decoded = rle_decompress(&fragment, data.len()).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn rle_rejects_bad_headers() {
        assert!(rle_decompress(&[], 4).is_err());
        assert!(rle_decompress(&[0u8; 64], 4).is_err(), "zero segment count");
        let mut bad = vec![0xFFu8; 64];
        bad[0..4].copy_from_slice(&16u32.to_le_bytes()); // >15 segments
        assert!(rle_decompress(&bad, 4).is_err());
    }

    #[test]
    fn encapsulated_fragments_extracted() {
        let frag = vec![1u8, 2, 3, 4];
        let ds = ds_with_pixel(frag.clone(), true);
        let (offsets, frags) = encapsulated_fragments(&ds).unwrap();
        assert!(offsets.is_empty(), "no basic offset table provided");
        assert_eq!(frags.len(), 1);
        assert_eq!(frags[0], frag);
    }

    #[test]
    fn decode_pixel_data_by_syntax() {
        let native = ds_with_pixel(vec![1, 2, 3, 4], false);
        for ts in [
            TransferSyntax::ImplicitVrLittleEndian,
            TransferSyntax::ExplicitVrLittleEndian,
            TransferSyntax::ExplicitVrBigEndian,
        ] {
            let frames = decode_pixel_data(&native, ts).unwrap();
            assert_eq!(frames.len(), 1);
            assert_eq!(frames[0], vec![1, 2, 3, 4]);
        }
        let err = decode_pixel_data(&native, TransferSyntax::Jpeg2000Lossless).unwrap_err();
        assert!(matches!(err, DicomError::UnsupportedTransferSyntax(_)));
    }
}
