//! GeoTIFF metadata extraction.
//!
//! A GeoTIFF is a baseline TIFF (per TIFF 6.0) carrying a set of *geo keys*
//! in the `GeoKeyDirectoryTag` plus model-transformation tags. This module
//! parses the TIFF container (both `II` little-endian and `MM` big-endian
//! byte orders) and extracts the georeferencing metadata: the CRS (as an
//! EPSG code via the `ProjectedCSTypeGeoKey` / `GeographicTypeGeoKey`
//! keys), the model pixel scale / tie points / transformation matrix, and
//! the ground-space bounding box. Pixel sample values are **not** decoded;
//! only metadata is extracted, which is what most GIS pipelines need up
//! front.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{Reader, ValidationSettings, Validator};

use crate::error::GeoError;

/// TIFF byte order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// TIFF byte order.
pub enum Endian {
    Little,
    Big,
}

/// A single TIFF tag entry.
#[derive(Debug, Clone, PartialEq)]
struct TiffEntry {
    tag: u16,
    etype: u16,
    count: u32,
    /// The 4 raw bytes of the value/offset field.
    raw: [u8; 4],
}

/// A parsed IFD (image file directory).
#[derive(Debug, Clone, PartialEq, Default)]
struct Ifd {
    entries: Vec<TiffEntry>,
}

// TIFF field types.
const TYPE_BYTE: u16 = 1;
const TYPE_ASCII: u16 = 2;
const TYPE_SHORT: u16 = 3;
const TYPE_LONG: u16 = 4;
const TYPE_RATIONAL: u16 = 5;
const TYPE_DOUBLE: u16 = 12;

// Standard TIFF tags.
const TAG_IMAGE_WIDTH: u16 = 256;
const TAG_IMAGE_LENGTH: u16 = 257;

// GeoTIFF tags.
const TAG_MODEL_PIXEL_SCALE: u16 = 33550;
const TAG_MODEL_TIE_POINT: u16 = 33922;
const TAG_MODEL_TRANSFORMATION: u16 = 34264;
const TAG_GEO_KEY_DIRECTORY: u16 = 34735;
const TAG_GEO_DOUBLE_PARAMS: u16 = 34736;
const TAG_GEO_ASCII_PARAMS: u16 = 34737;

// GeoKey ids.
const KEY_GEOGRAPHIC_TYPE: u16 = 2048;
const KEY_GEOGRAPHIC_CITATION: u16 = 2049;
const KEY_PROJECTED_TYPE: u16 = 3072;
const KEY_PROJECTED_CITATION: u16 = 3073;

fn rd_u16(b: &[u8], e: Endian, off: usize) -> u16 {
    match e {
        Endian::Little => u16::from_le_bytes([b[off], b[off + 1]]),
        Endian::Big => u16::from_be_bytes([b[off], b[off + 1]]),
    }
}
fn rd_u32(b: &[u8], e: Endian, off: usize) -> u32 {
    match e {
        Endian::Little => u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]]),
        Endian::Big => u32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]]),
    }
}

fn rd_f64(b: &[u8], e: Endian, off: usize) -> f64 {
    let b = [
        b[off],
        b[off + 1],
        b[off + 2],
        b[off + 3],
        b[off + 4],
        b[off + 5],
        b[off + 6],
        b[off + 7],
    ];
    match e {
        Endian::Little => f64::from_le_bytes(b),
        Endian::Big => f64::from_be_bytes(b),
    }
}

fn type_size(t: u16) -> usize {
    match t {
        TYPE_BYTE | TYPE_ASCII => 1,
        TYPE_SHORT => 2,
        TYPE_LONG | TYPE_RATIONAL => 4,
        TYPE_DOUBLE => 8,
        _ => 4,
    }
}

impl TiffEntry {
    /// Whether the value fits inline in the 4-byte value/offset field.
    fn is_inline(&self) -> bool {
        (self.count as usize) * type_size(self.etype) <= 4
    }
}

// The `TiffDoc` keeps the original buffer alive so entries can read their
// out-of-line values by absolute offset.
struct TiffDoc {
    endian: Endian,
    ifds: Vec<Ifd>,
    bytes: Vec<u8>,
}

impl TiffDoc {
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

// Re-implement entry readers against `TiffDoc` to avoid aliasing the buffer.
fn entry_values_doc(doc: &TiffDoc, entry: &TiffEntry) -> Vec<f64> {
    let e = doc.endian;
    let size = type_size(entry.etype);
    let b = doc.bytes();
    let mut out = Vec::new();
    if entry.is_inline() || entry.count == 0 {
        let raw = entry.raw;
        let n = if entry.count == 0 { 0 } else { (4 / size).min(entry.count as usize) };
        for i in 0..n {
            let base = i * size;
            let v = match entry.etype {
                TYPE_SHORT => rd_u16(&raw, e, base) as f64,
                TYPE_LONG => rd_u32(&raw, e, base) as f64,
                TYPE_DOUBLE => {
                    let bytes =
                        [raw[base], raw[base + 1], raw[base + 2], raw[base + 3], 0, 0, 0, 0];
                    f64::from_le_bytes(bytes)
                }
                _ => 0.0,
            };
            out.push(v);
        }
    } else {
        let off = match e {
            Endian::Little => rd_u32(&entry.raw, Endian::Little, 0),
            Endian::Big => rd_u32(&entry.raw, Endian::Big, 0),
        };
        for i in 0..entry.count as usize {
            let base = off as usize + i * size;
            if base + size > b.len() {
                break;
            }
            let v = match entry.etype {
                TYPE_SHORT => rd_u16(b, e, base) as f64,
                TYPE_LONG => rd_u32(b, e, base) as f64,
                TYPE_DOUBLE => rd_f64(b, e, base),
                _ => 0.0,
            };
            out.push(v);
        }
    }
    out
}

fn entry_ascii_doc(doc: &TiffDoc, entry: &TiffEntry) -> String {
    let e = doc.endian;
    if entry.is_inline() {
        let mut s = String::new();
        for &c in &entry.raw {
            if c == 0 {
                break;
            }
            s.push(c as char);
        }
        return s;
    }
    let off = match e {
        Endian::Little => rd_u32(&entry.raw, Endian::Little, 0),
        Endian::Big => rd_u32(&entry.raw, Endian::Big, 0),
    } as usize;
    let b = doc.bytes();
    let mut s = String::new();
    for i in 0..entry.count as usize {
        if off + i >= b.len() {
            break;
        }
        let c = b[off + i];
        if c == 0 {
            break;
        }
        s.push(c as char);
    }
    s
}

// Helpers used by the first (buffer-less) impl; we route through TiffDoc now.

/// Parse a TIFF byte stream into a [`TiffDoc`].
fn parse_tiff(bytes: &[u8]) -> Result<TiffDoc, GeoError> {
    if bytes.len() < 8 {
        return Err(GeoError::geotiff(0, "file too short to be a TIFF"));
    }
    let endian = match (&bytes[0], &bytes[1]) {
        (b'I', b'I') => Endian::Little,
        (b'M', b'M') => Endian::Big,
        _ => return Err(GeoError::geotiff(0, "invalid TIFF byte-order marker")),
    };
    let magic = rd_u16(bytes, endian, 2);
    if magic != 42 {
        return Err(GeoError::geotiff(2, "not a baseline TIFF (magic != 42)"));
    }
    let mut doc = TiffDoc { endian, ifds: Vec::new(), bytes: bytes.to_vec() };
    let mut ifd_offset = rd_u32(bytes, endian, 4) as usize;
    let mut guard = 0;
    while ifd_offset != 0 && guard < 100 {
        guard += 1;
        if ifd_offset + 2 > bytes.len() {
            return Err(GeoError::geotiff(ifd_offset, "IFD offset out of range"));
        }
        let count = rd_u16(bytes, endian, ifd_offset) as usize;
        let mut entries = Vec::with_capacity(count);
        let mut pos = ifd_offset + 2;
        for _ in 0..count {
            if pos + 12 > bytes.len() {
                return Err(GeoError::geotiff(pos, "truncated IFD entry"));
            }
            let tag = rd_u16(bytes, endian, pos);
            let etype = rd_u16(bytes, endian, pos + 2);
            let ecount = rd_u32(bytes, endian, pos + 4);
            let mut raw = [0u8; 4];
            raw.copy_from_slice(&bytes[pos + 8..pos + 12]);
            entries.push(TiffEntry { tag, etype, count: ecount, raw });
            pos += 12;
        }
        doc.ifds.push(Ifd { entries });
        if pos + 4 > bytes.len() {
            break;
        }
        ifd_offset = rd_u32(bytes, endian, pos) as usize;
    }
    Ok(doc)
}

fn find_entry(ifd: &Ifd, tag: u16) -> Option<&TiffEntry> {
    ifd.entries.iter().find(|e| e.tag == tag)
}

/// A parsed GeoKey.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoKey {
    /// GeoKey id (e.g. 3072 = ProjectedCSTypeGeoKey).
    pub id: u16,
    /// TIFF tag location the value is stored in (34735/34736/34737/0).
    pub location: u16,
    /// Number of values.
    pub count: u16,
    /// Value offset / inline short value.
    pub value: i16,
}

/// Extracted GeoTIFF georeferencing metadata.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GeoTiffMetadata {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// EPSG code of the CRS, if a projected or geographic key is present.
    pub epsg_code: Option<u32>,
    /// Whether the CRS is projected (vs. geographic).
    pub is_projected: bool,
    /// Free-text citation for the CRS, if present.
    pub citation: Option<String>,
    /// Model pixel scale `[scaleX, scaleY, scaleZ]`, if present.
    pub pixel_scale: Option<[f64; 3]>,
    /// Model tie points (flattened 6-tuples `[i,j,k,X,Y,Z, ...]`), if present.
    pub tie_points: Vec<f64>,
    /// Full 16-element model transformation matrix, if present.
    pub model_transformation: Option<[f64; 16]>,
    /// Ground-space bounding box `[xmin, ymin, xmax, ymax]` in CRS units.
    pub bbox: Option<[f64; 4]>,
    /// Parsed geo keys.
    pub geo_keys: Vec<GeoKey>,
}

/// Parse GeoTIFF metadata from a byte stream.
pub fn parse_geotiff(bytes: &[u8]) -> Result<GeoTiffMetadata, GeoError> {
    let doc = parse_tiff(bytes)?;
    let ifd = doc.ifds.first().ok_or_else(|| GeoError::geotiff(4, "no IFD found"))?;

    let width = find_entry(ifd, TAG_IMAGE_WIDTH)
        .map(|e| entry_values_doc(&doc, e).first().copied().unwrap_or(0.0) as u32)
        .unwrap_or(0);
    let height = find_entry(ifd, TAG_IMAGE_LENGTH)
        .map(|e| entry_values_doc(&doc, e).first().copied().unwrap_or(0.0) as u32)
        .unwrap_or(0);

    let mut meta = GeoTiffMetadata { width, height, ..Default::default() };

    // GeoKey directory.
    if let Some(geo_dir) = find_entry(ifd, TAG_GEO_KEY_DIRECTORY) {
        let shorts = entry_values_doc(&doc, geo_dir);
        let shorts: Vec<i64> = shorts.into_iter().map(|v| v as i64).collect();
        if shorts.len() >= 4 {
            let num_keys = shorts[3] as usize;
            let mut keys = Vec::with_capacity(num_keys);
            let mut i = 4;
            while i + 4 <= shorts.len() && keys.len() < num_keys {
                let id = shorts[i] as u16;
                let location = shorts[i + 1] as u16;
                let count = shorts[i + 2] as u16;
                let value = shorts[i + 3] as i16;
                keys.push(GeoKey { id, location, count, value });
                i += 4;
            }
            meta.geo_keys = keys;
        }
    }

    // Resolve CRS from keys.
    let mut citation: Option<String> = None;
    for key in &meta.geo_keys {
        match key.id {
            KEY_PROJECTED_TYPE if key.value != 0 => {
                meta.epsg_code = Some(key.value as u32);
                meta.is_projected = true;
            }
            KEY_GEOGRAPHIC_TYPE if key.value != 0 && meta.epsg_code.is_none() => {
                meta.epsg_code = Some(key.value as u32);
                meta.is_projected = false;
            }
            KEY_PROJECTED_CITATION | KEY_GEOGRAPHIC_CITATION => {
                citation = Some(resolve_ascii_or_double(&doc, ifd, key));
            }
            _ => {}
        }
    }
    meta.citation = citation;

    // Model transformation / tie point + scale.
    if let Some(tr) = find_entry(ifd, TAG_MODEL_TRANSFORMATION) {
        let vals = entry_values_doc(&doc, tr);
        if vals.len() == 16 {
            let mut m = [0f64; 16];
            m.copy_from_slice(&vals);
            meta.model_transformation = Some(m);
        }
    }
    if let Some(ps) = find_entry(ifd, TAG_MODEL_PIXEL_SCALE) {
        let vals = entry_values_doc(&doc, ps);
        if vals.len() >= 3 {
            meta.pixel_scale = Some([vals[0], vals[1], vals[2]]);
        }
    }
    if let Some(tp) = find_entry(ifd, TAG_MODEL_TIE_POINT) {
        meta.tie_points = entry_values_doc(&doc, tp);
    }

    meta.bbox = compute_bbox(&meta);
    Ok(meta)
}

/// Resolve a key's value from the appropriate params tag.
fn resolve_ascii_or_double(doc: &TiffDoc, ifd: &Ifd, key: &GeoKey) -> String {
    match key.location {
        TAG_GEO_ASCII_PARAMS => {
            if let Some(e) = find_entry(ifd, TAG_GEO_ASCII_PARAMS) {
                entry_ascii_doc(doc, e)
            } else {
                String::new()
            }
        }
        TAG_GEO_DOUBLE_PARAMS => {
            if let Some(e) = find_entry(ifd, TAG_GEO_DOUBLE_PARAMS) {
                let vals = entry_values_doc(doc, e);
                let idx = key.value as usize;
                vals.get(idx).map(|v| v.to_string()).unwrap_or_default()
            } else {
                String::new()
            }
        }
        _ => key.value.to_string(),
    }
}

/// Compute the ground-space bounding box from the model transformation or
/// tie-point + pixel-scale, using the image dimensions.
fn compute_bbox(meta: &GeoTiffMetadata) -> Option<[f64; 4]> {
    let (w, h) = (meta.width as f64, meta.height as f64);
    if w == 0.0 || h == 0.0 {
        return None;
    }
    let corners: Vec<(f64, f64)> = if let Some(m) = meta.model_transformation {
        // Row-major 4x4: X = m0*i + m1*j + m3 ; Y = m4*i + m5*j + m7.
        let f = |i: f64, j: f64| (m[0] * i + m[1] * j + m[3], m[4] * i + m[5] * j + m[7]);
        alloc::vec![f(0.0, 0.0), f(w, 0.0), f(0.0, h), f(w, h)]
    } else if let (Some(scale), Some(tp)) = (meta.pixel_scale, meta_tiepoint(meta)) {
        let (ix, iy) = (tp.0, tp.1);
        let (ox, oy) = (tp.2, tp.3);
        let f = |i: f64, j: f64| (ox + (i - ix) * scale[0], oy - (j - iy) * scale[1]);
        alloc::vec![f(0.0, 0.0), f(w, 0.0), f(0.0, h), f(w, h)]
    } else {
        return None;
    };
    let mut xs: Vec<f64> = corners.iter().map(|c| c.0).collect();
    let mut ys: Vec<f64> = corners.iter().map(|c| c.1).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some([xs[0], ys[0], xs[xs.len() - 1], ys[ys.len() - 1]])
}

/// Extract the primary tie point `(i, j, X, Y)` (defaults to 0,0,0,0).
fn meta_tiepoint(meta: &GeoTiffMetadata) -> Option<(f64, f64, f64, f64)> {
    let tp = &meta.tie_points;
    if tp.len() >= 6 {
        Some((tp[0], tp[1], tp[3], tp[4]))
    } else {
        None
    }
}

/// A GeoTIFF reader implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct GeoTiffReader {
    settings: ValidationSettings,
}

impl GeoTiffReader {
    /// Create a new reader.
    pub fn new() -> Self {
        Self { settings: ValidationSettings::new() }
    }

    /// Builder: configure validation settings.
    pub fn with_validation(mut self, settings: ValidationSettings) -> Self {
        self.settings = settings;
        self
    }
}

impl Reader<GeoTiffMetadata> for GeoTiffReader {
    type Error = GeoError;

    fn parse(&self, input: &[u8]) -> Result<GeoTiffMetadata, GeoError> {
        let meta = parse_geotiff(input)?;
        self.settings.run(&GeoTiffValidator, &meta)?;
        Ok(meta)
    }
}

/// GeoTIFF syntactic/semantic validator.
#[derive(Debug, Clone, Default)]
pub struct GeoTiffValidator;

impl Validator<GeoTiffMetadata> for GeoTiffValidator {
    type Error = GeoError;

    fn validate_syntactic(&self, _m: &GeoTiffMetadata) -> Result<(), GeoError> {
        Ok(())
    }

    fn validate_semantic(&self, m: &GeoTiffMetadata) -> Result<(), GeoError> {
        if m.width == 0 || m.height == 0 {
            return Err(GeoError::geotiff(0, "image dimensions are zero"));
        }
        Ok(())
    }
}

/// Build a minimal in-memory GeoTIFF byte stream for round-trip / golden tests.
///
/// Produces a single-IFD TIFF with the geo tags needed to georeference a
/// `width x height` image whose origin is `(origin_x, origin_y)` at pixel
/// (0,0) with the given `pixel_scale`. The CRS is recorded via `epsg_code`.
pub fn build_test_geotiff(
    endian: Endian,
    width: u32,
    height: u32,
    origin_x: f64,
    origin_y: f64,
    pixel_scale: [f64; 3],
    epsg_code: u32,
) -> Vec<u8> {
    // Simplified builder: emits a valid little/big-endian TIFF with geo tags.
    let e = endian;
    let mut b = Vec::new();
    // header
    match e {
        Endian::Little => b.extend_from_slice(b"II"),
        Endian::Big => b.extend_from_slice(b"MM"),
    }
    b.extend_from_slice(&u16_to(e, 42));
    // IFD offset placeholder (filled later)
    let ifd_offset_pos = b.len();
    b.extend_from_slice(&u32_to(e, 0));

    // Entries: 256 width, 257 height, 33550 scale, 33922 tiepoint, 34735 geokeys.
    let entries: Vec<(u16, u16, u32)> = vec![
        (TAG_IMAGE_WIDTH, TYPE_LONG, 1),
        (TAG_IMAGE_LENGTH, TYPE_LONG, 1),
        (TAG_MODEL_PIXEL_SCALE, TYPE_DOUBLE, 3),
        (TAG_MODEL_TIE_POINT, TYPE_DOUBLE, 6),
        (TAG_GEO_KEY_DIRECTORY, TYPE_SHORT, 8),
    ];

    // Compute the layout: IFD starts at byte 8; entries then data regions.
    let ifd_pos = 8usize;
    let ifd_size = 2 + entries.len() * 12 + 4;
    let align = |x: usize| (x + 3) & !3;
    let scale_pos = align(ifd_pos + ifd_size);
    let tie_pos = align(scale_pos + 3 * 8);
    let keys_pos = align(tie_pos + 6 * 8);

    // Write the IFD now that all offsets are known.
    let mut ifd_bytes = Vec::new();
    ifd_bytes.extend_from_slice(&u16_to(e, entries.len() as u16));
    for (tag, ty, count) in &entries {
        let offset = match *tag {
            TAG_MODEL_PIXEL_SCALE => scale_pos as u32,
            TAG_MODEL_TIE_POINT => tie_pos as u32,
            TAG_GEO_KEY_DIRECTORY => keys_pos as u32,
            _ => match *tag {
                TAG_IMAGE_WIDTH => width,
                TAG_IMAGE_LENGTH => height,
                _ => 0,
            },
        };
        ifd_bytes.extend_from_slice(&u16_to(e, *tag));
        ifd_bytes.extend_from_slice(&u16_to(e, *ty));
        ifd_bytes.extend_from_slice(&u32_to(e, *count));
        ifd_bytes.extend_from_slice(&u32_to(e, offset));
    }
    ifd_bytes.extend_from_slice(&u32_to(e, 0)); // next IFD = 0

    b.truncate(ifd_pos);
    b.extend_from_slice(&ifd_bytes);

    // Now write data regions in order, aligning.
    while b.len() < scale_pos {
        b.push(0);
    }
    for v in pixel_scale.iter() {
        b.extend_from_slice(&f64_to_bytes(e, *v));
    }
    while b.len() < tie_pos {
        b.push(0);
    }
    for v in [0.0f64, 0.0, 0.0, origin_x, origin_y, 0.0].iter() {
        b.extend_from_slice(&f64_to_bytes(e, *v));
    }
    while b.len() < keys_pos {
        b.push(0);
    }
    // GeoKey directory: version 1, rev 1, minor 0, 1 key.
    let header = [1u16, 1, 0, 1];
    for h in header.iter() {
        b.extend_from_slice(&u16_to(e, *h));
    }
    // Key: choose the projected or geographic CRS key based on the EPSG code.
    let is_projected = (32601..=32660).contains(&epsg_code)
        || (32701..=32760).contains(&epsg_code)
        || epsg_code == 2193
        || epsg_code == 3857;
    let key_id = if is_projected { KEY_PROJECTED_TYPE } else { KEY_GEOGRAPHIC_TYPE };
    b.extend_from_slice(&u16_to(e, key_id));
    b.extend_from_slice(&u16_to(e, 0)); // location 0 (inline)
    b.extend_from_slice(&u16_to(e, 1));
    b.extend_from_slice(&u16_to(e, epsg_code as u16));

    // Fill IFD offset back at position 4.
    let ifd_off_bytes = u32_to(e, ifd_pos as u32);
    b[ifd_offset_pos..ifd_offset_pos + 4].copy_from_slice(&ifd_off_bytes);
    // pad to even length
    if b.len() % 2 == 1 {
        b.push(0);
    }
    b
}

fn u16_to(e: Endian, v: u16) -> [u8; 2] {
    match e {
        Endian::Little => v.to_le_bytes(),
        Endian::Big => v.to_be_bytes(),
    }
}
fn u32_to(e: Endian, v: u32) -> [u8; 4] {
    match e {
        Endian::Little => v.to_le_bytes(),
        Endian::Big => v.to_be_bytes(),
    }
}
fn f64_to_bytes(e: Endian, v: f64) -> [u8; 8] {
    match e {
        Endian::Little => v.to_le_bytes(),
        Endian::Big => v.to_be_bytes(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test_geotiff() {
        let bytes = build_test_geotiff(
            Endian::Little,
            100,
            200,
            500_000.0,
            4_000_000.0,
            [10.0, 10.0, 0.0],
            32755,
        );
        let meta = parse_geotiff(&bytes).unwrap();
        assert_eq!(meta.width, 100);
        assert_eq!(meta.height, 200);
        assert_eq!(meta.epsg_code, Some(32755));
        assert!(meta.is_projected);
        let bbox = meta.bbox.unwrap();
        assert_eq!(bbox[0], 500_000.0);
        assert_eq!(bbox[2], 501_000.0);
        assert_eq!(bbox[3], 4_000_000.0);
    }

    #[test]
    fn parse_big_endian() {
        let bytes = build_test_geotiff(Endian::Big, 10, 10, 0.0, 0.0, [1.0, 1.0, 0.0], 4326);
        let meta = parse_geotiff(&bytes).unwrap();
        assert_eq!(meta.epsg_code, Some(4326));
        assert!(!meta.is_projected);
    }

    #[test]
    fn reject_non_tiff() {
        assert!(parse_geotiff(b"not a tiff").is_err());
    }
}
