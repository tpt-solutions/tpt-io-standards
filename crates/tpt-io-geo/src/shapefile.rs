//! Esri Shapefile (.shp / .shx / .dbf) parsing.
//!
//! A Shapefile is a trio of files:
//!
//! * `.shp` — the main geometry file (a 100-byte header followed by
//!   record headers + content, all multi-byte integers big-endian *except*
//!   the shape type and geometry bodies which are little-endian).
//! * `.shx` — the index file, sharing the same header layout with one
//!   8-byte record per `.shp` record (offset/length in 16-bit words).
//! * `.dbf` — the dBASE III+ attribute table, parsed separately below.
//!
//! Coordinate values are stored in the dataset's own CRS (declared by an
//! optional side-car `.prj`; not parsed here). Per-record geometry is
//! decoded into strongly-typed [`ShapeRecord`]s.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{Reader, ValidationSettings, Validator};

use crate::error::GeoError;

/// The integer shape-type codes of the Shapefile specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeType {
    /// Null shape (no geometry).
    Null,
    /// 2D point.
    Point,
    /// 2D polyline.
    PolyLine,
    /// 2D polygon.
    Polygon,
    /// 2D multipoint.
    MultiPoint,
    /// 3D point (with Z).
    PointZ,
    /// 3D polyline.
    PolyLineZ,
    /// 3D polygon.
    PolygonZ,
    /// 3D multipoint.
    MultiPointZ,
    /// Point with M (measure).
    PointM,
    /// Polyline with M.
    PolyLineM,
    /// Polygon with M.
    PolygonM,
    /// Multipoint with M.
    MultiPointM,
    /// Multipatch (3D surface).
    MultiPatch,
    /// Any other (unsupported) shape code.
    Unknown(i32),
}

impl ShapeType {
    fn from_code(c: i32) -> ShapeType {
        match c {
            0 => ShapeType::Null,
            1 => ShapeType::Point,
            3 => ShapeType::PolyLine,
            5 => ShapeType::Polygon,
            8 => ShapeType::MultiPoint,
            11 => ShapeType::PointZ,
            13 => ShapeType::PolyLineZ,
            15 => ShapeType::PolygonZ,
            18 => ShapeType::MultiPointZ,
            21 => ShapeType::PointM,
            23 => ShapeType::PolyLineM,
            25 => ShapeType::PolygonM,
            28 => ShapeType::MultiPointM,
            31 => ShapeType::MultiPatch,
            n => ShapeType::Unknown(n),
        }
    }

    fn has_z(&self) -> bool {
        matches!(
            self,
            ShapeType::PointZ | ShapeType::PolyLineZ | ShapeType::PolygonZ | ShapeType::MultiPointZ
        )
    }

    fn has_m(&self) -> bool {
        matches!(
            self,
            ShapeType::PointM
                | ShapeType::PolyLineM
                | ShapeType::PolygonM
                | ShapeType::MultiPointM
                | ShapeType::PointZ
                | ShapeType::PolyLineZ
                | ShapeType::PolygonZ
                | ShapeType::MultiPointZ
        )
    }
}

/// The 100-byte Shapefile header.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeHeader {
    /// The shape type of every record.
    pub shape_type: ShapeType,
    /// Bounding box `[xmin, ymin, xmax, ymax]`.
    pub bbox: [f64; 4],
    /// Z range `[zmin, zmax]`.
    pub z_range: [f64; 2],
    /// Measure range `[mmin, mmax]`.
    pub m_range: [f64; 2],
    /// File length in 16-bit words (header + all records).
    pub file_length_words: u32,
}

/// A single coordinate, optionally carrying Z and M.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X (easting).
    pub x: f64,
    /// Y (northing).
    pub y: f64,
    /// Optional Z (elevation).
    pub z: Option<f64>,
    /// Optional M (measure).
    pub m: Option<f64>,
}

/// The decoded geometry payload of a record.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeGeometry {
    /// No geometry.
    Null,
    /// A single point.
    Point(Point),
    /// A multipoint (shared bbox + points).
    MultiPoint {
        /// Bounding box `[xmin, ymin, xmax, ymax]`.
        bbox: [f64; 4],
        /// The points.
        points: Vec<Point>,
    },
    /// A polyline or polygon: a set of parts (each a ring/line).
    Parts {
        /// Bounding box `[xmin, ymin, xmax, ymax]`.
        bbox: [f64; 4],
        /// Each part is an ordered list of points.
        parts: Vec<Vec<Point>>,
    },
}

/// A single `.shp` record.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeRecord {
    /// The 1-based record number from the record header.
    pub record_number: u32,
    /// The shape type stored in this record.
    pub shape_type: ShapeType,
    /// The decoded geometry.
    pub geometry: ShapeGeometry,
}

/// A parsed `.shp` file: header plus records.
#[derive(Debug, Clone, PartialEq)]
pub struct Shapefile {
    /// File header.
    pub header: ShapeHeader,
    /// Records in file order.
    pub records: Vec<ShapeRecord>,
}

/// One entry of the `.shx` index file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexEntry {
    /// Record offset from file start, in 16-bit words.
    pub offset_words: u32,
    /// Record content length, in 16-bit words.
    pub content_length_words: u32,
}

/// A parsed `.shx` index file.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeIndex {
    /// Header (mirrors the `.shp` header).
    pub header: ShapeHeader,
    /// One entry per `.shp` record.
    pub entries: Vec<IndexEntry>,
}

// ---- byte readers --------------------------------------------------------

fn rd_i32_be(b: &[u8], off: usize) -> i32 {
    i32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}
fn rd_u32_be(b: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}
fn rd_i32_le(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}
fn rd_f64_le(b: &[u8], off: usize) -> f64 {
    f64::from_le_bytes([
        b[off],
        b[off + 1],
        b[off + 2],
        b[off + 3],
        b[off + 4],
        b[off + 5],
        b[off + 6],
        b[off + 7],
    ])
}

fn parse_header(b: &[u8], base: usize) -> Result<ShapeHeader, GeoError> {
    if b.len() < base + 100 {
        return Err(GeoError::shapefile(base, "header shorter than 100 bytes"));
    }
    let file_code = rd_i32_be(b, base);
    if file_code != 9994 {
        return Err(GeoError::shapefile(base, "not a Shapefile (file code != 9994)"));
    }
    let file_length_words = rd_u32_be(b, base + 24);
    let version = rd_i32_le(b, base + 28);
    if version != 1000 {
        return Err(GeoError::shapefile(base + 28, "unsupported Shapefile version"));
    }
    let shape_type = ShapeType::from_code(rd_i32_le(b, base + 32));
    let bbox = [
        rd_f64_le(b, base + 36),
        rd_f64_le(b, base + 44),
        rd_f64_le(b, base + 52),
        rd_f64_le(b, base + 60),
    ];
    let z_range = [rd_f64_le(b, base + 68), rd_f64_le(b, base + 76)];
    let m_range = [rd_f64_le(b, base + 84), rd_f64_le(b, base + 92)];
    Ok(ShapeHeader { shape_type, bbox, z_range, m_range, file_length_words })
}

fn read_points(
    b: &[u8],
    base: usize,
    n: usize,
    z: Option<&[f64]>,
    m: Option<&[f64]>,
) -> Vec<Point> {
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let off = base + i * 16;
        let x = rd_f64_le(b, off);
        let y = rd_f64_le(b, off + 8);
        let zv = z.map(|z| z[i]);
        let mv = m.map(|m| m[i]);
        pts.push(Point { x, y, z: zv, m: mv });
    }
    pts
}

/// Read the trailing Z and M arrays for Z/M shape types.
///
/// `points_base` is the offset of the point block; `n` is the point count.
/// Z types carry a Z array followed by an M array; M-only types carry just an
/// M array. Arrays are only read when they fit within the record slice.
fn read_zm_arrays(
    b: &[u8],
    points_base: usize,
    n: usize,
    st: ShapeType,
) -> (Option<Vec<f64>>, Option<Vec<f64>>) {
    let has_z = st.has_z();
    let has_m = st.has_m();
    let z_base = points_base + n * 16;
    let m_base = if has_z { z_base + n * 8 } else { z_base };
    let mut z = None;
    let mut m = None;
    if has_z && z_base + n * 8 <= b.len() {
        let mut zv = Vec::with_capacity(n);
        for i in 0..n {
            zv.push(rd_f64_le(b, z_base + i * 8));
        }
        z = Some(zv);
    }
    if has_m && m_base + n * 8 <= b.len() {
        let mut mv = Vec::with_capacity(n);
        for i in 0..n {
            mv.push(rd_f64_le(b, m_base + i * 8));
        }
        m = Some(mv);
    }
    (z, m)
}

fn parse_geometry(
    b: &[u8],
    base: usize,
    len_bytes: usize,
    st: ShapeType,
) -> Result<ShapeGeometry, GeoError> {
    match st {
        ShapeType::Null => Ok(ShapeGeometry::Null),
        ShapeType::Point | ShapeType::PointZ | ShapeType::PointM => {
            if len_bytes < 16 {
                return Err(GeoError::shapefile(base, "point record too short"));
            }
            let x = rd_f64_le(b, base);
            let y = rd_f64_le(b, base + 8);
            let (z, m) = read_zm_arrays(b, base + 16, 1, st);
            Ok(ShapeGeometry::Point(Point {
                x,
                y,
                z: z.and_then(|v| v.into_iter().next()),
                m: m.and_then(|v| v.into_iter().next()),
            }))
        }
        ShapeType::MultiPoint | ShapeType::MultiPointZ | ShapeType::MultiPointM => {
            if len_bytes < 32 {
                return Err(GeoError::shapefile(base, "multipoint record too short"));
            }
            let bbox = [
                rd_f64_le(b, base),
                rd_f64_le(b, base + 8),
                rd_f64_le(b, base + 16),
                rd_f64_le(b, base + 24),
            ];
            let n = rd_i32_le(b, base + 32) as usize;
            let points_base = base + 36;
            let (z, m) = read_zm_arrays(b, points_base, n, st);
            let pts = read_points(b, points_base, n, z.as_deref(), m.as_deref());
            Ok(ShapeGeometry::MultiPoint { bbox, points: pts })
        }
        ShapeType::PolyLine
        | ShapeType::Polygon
        | ShapeType::PolyLineZ
        | ShapeType::PolygonZ
        | ShapeType::PolyLineM
        | ShapeType::PolygonM => {
            if len_bytes < 36 {
                return Err(GeoError::shapefile(base, "poly record too short"));
            }
            let bbox = [
                rd_f64_le(b, base),
                rd_f64_le(b, base + 8),
                rd_f64_le(b, base + 16),
                rd_f64_le(b, base + 24),
            ];
            let num_parts = rd_i32_le(b, base + 32) as usize;
            let num_points = rd_i32_le(b, base + 36) as usize;
            let parts_base = base + 40;
            let mut part_starts = Vec::with_capacity(num_parts);
            for i in 0..num_parts {
                part_starts.push(rd_i32_le(b, parts_base + i * 4) as usize);
            }
            let points_base = parts_base + num_parts * 4;
            let (z, m) = read_zm_arrays(b, points_base, num_points, st);
            let all_pts = read_points(b, points_base, num_points, z.as_deref(), m.as_deref());
            let mut parts = Vec::with_capacity(num_parts);
            for i in 0..num_parts {
                let start = part_starts[i];
                let end = if i + 1 < num_parts { part_starts[i + 1] } else { num_points };
                parts.push(all_pts[start..end].to_vec());
            }
            Ok(ShapeGeometry::Parts { bbox, parts })
        }
        _ => Err(GeoError::shapefile(base, "unsupported shape type")),
    }
}

/// Parse a single `.shp` record beginning at byte offset `pos`.
///
/// Returns the parsed record and the offset just past it, so callers (e.g. the
/// streaming reader) can resume from `next_pos`.
pub fn parse_shp_record_at(b: &[u8], pos: usize) -> Result<(ShapeRecord, usize), GeoError> {
    if pos + 8 > b.len() {
        return Err(GeoError::shapefile(pos, "record header out of range"));
    }
    let record_number = rd_i32_be(b, pos) as u32;
    let content_length_words = rd_i32_be(b, pos + 4) as usize;
    let content_length_bytes = content_length_words * 2;
    let content_base = pos + 8;
    if content_base + content_length_bytes > b.len() {
        return Err(GeoError::shapefile(content_base, "record runs past end of file"));
    }
    let shape_type = ShapeType::from_code(rd_i32_le(b, content_base));
    let geometry =
        parse_geometry(b, content_base + 4, content_length_bytes.saturating_sub(4), shape_type)?;
    let next_pos = content_base + content_length_bytes;
    Ok((ShapeRecord { record_number, shape_type, geometry }, next_pos))
}

/// Parse a `.shp` byte stream into a [`Shapefile`].
pub fn parse_shp(b: &[u8]) -> Result<Shapefile, GeoError> {
    let header = parse_header(b, 0)?;
    let mut records = Vec::new();
    let mut pos = 100;
    while pos + 8 <= b.len() {
        let record_number = rd_i32_be(b, pos) as u32;
        let content_length_words = rd_i32_be(b, pos + 4) as usize;
        let content_length_bytes = content_length_words * 2;
        let content_base = pos + 8;
        if content_base + content_length_bytes > b.len() {
            return Err(GeoError::shapefile(content_base, "record runs past end of file"));
        }
        let shape_type = ShapeType::from_code(rd_i32_le(b, content_base));
        let geometry = parse_geometry(
            b,
            content_base + 4,
            content_length_bytes.saturating_sub(4),
            shape_type,
        )?;
        records.push(ShapeRecord { record_number, shape_type, geometry });
        pos = content_base + content_length_bytes;
    }
    Ok(Shapefile { header, records })
}

/// Parse a `.shx` index byte stream into a [`ShapeIndex`].
pub fn parse_shx(b: &[u8]) -> Result<ShapeIndex, GeoError> {
    let header = parse_header(b, 0)?;
    let mut entries = Vec::new();
    let mut pos = 100;
    while pos + 8 <= b.len() {
        let offset_words = rd_i32_be(b, pos) as u32;
        let content_length_words = rd_i32_be(b, pos + 4) as u32;
        entries.push(IndexEntry { offset_words, content_length_words });
        pos += 8;
    }
    Ok(ShapeIndex { header, entries })
}

/// A parsed dBASE III+ table.
#[derive(Debug, Clone, PartialEq)]
pub struct DbfTable {
    /// Column descriptors in file order.
    pub fields: Vec<DbfField>,
    /// Number of records.
    pub num_records: u32,
    /// Records: each is a list of string cell values, one per field.
    pub records: Vec<Vec<String>>,
}

/// A dBASE field descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbfField {
    /// Field name (trimmed).
    pub name: String,
    /// Field type code (`C`, `N`, `F`, `L`, `D`, `M`, …).
    pub field_type: char,
    /// Display/field length in bytes.
    pub length: u8,
    /// Decimal places (numeric/float fields).
    pub decimal: u8,
}

fn rd_u16_le(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}
fn rd_u32_le(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Parse a `.dbf` byte stream into a [`DbfTable`].
pub fn parse_dbf(b: &[u8]) -> Result<DbfTable, GeoError> {
    if b.len() < 32 {
        return Err(GeoError::dbf(0, "dBASE file too short"));
    }
    let num_records = rd_u32_le(b, 4);
    let header_len = rd_u16_le(b, 8) as usize;
    let record_len = rd_u16_le(b, 10) as usize;
    if header_len == 0 || record_len == 0 {
        return Err(GeoError::dbf(8, "invalid dBASE header/record length"));
    }
    let mut fields = Vec::new();
    let mut pos = 32;
    while pos < header_len && b[pos] != 0x0d {
        if pos + 32 > b.len() {
            return Err(GeoError::dbf(pos, "truncated field descriptor"));
        }
        let raw_name = &b[pos..pos + 11];
        let mut name = String::new();
        for &c in raw_name {
            if c == 0x00 {
                break;
            }
            name.push(c as char);
        }
        let field_type = b[pos + 11] as char;
        let length = b[pos + 16];
        let decimal = b[pos + 17];
        fields.push(DbfField { name, field_type, length, decimal });
        pos += 32;
    }
    let mut records = Vec::with_capacity(num_records as usize);
    let data_start = header_len;
    for r in 0..num_records as usize {
        let start = data_start + r * record_len;
        let end = start + record_len;
        if end > b.len() {
            return Err(GeoError::dbf(start, "record runs past end of file"));
        }
        let row = &b[start..end];
        let mut cells = Vec::with_capacity(fields.len());
        let mut off = 1; // skip deletion flag byte
        for f in &fields {
            let len = f.length as usize;
            let slice = &row[off..off + len];
            let s = alloc::string::String::from_utf8_lossy(slice).trim().to_string();
            cells.push(s);
            off += len;
        }
        records.push(cells);
    }
    Ok(DbfTable { fields, num_records, records })
}

/// A `Shapefile` joined with its optional `.shx` index and `.dbf` attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapefileDataset {
    /// The geometry file.
    pub shapefile: Shapefile,
    /// The index file, if supplied.
    pub index: Option<ShapeIndex>,
    /// The attribute table, if supplied.
    pub attributes: Option<DbfTable>,
}

impl Default for ShapefileDataset {
    fn default() -> Self {
        Self {
            shapefile: Shapefile {
                header: ShapeHeader {
                    shape_type: ShapeType::Null,
                    bbox: [0.0; 4],
                    z_range: [0.0; 2],
                    m_range: [0.0; 2],
                    file_length_words: 0,
                },
                records: Vec::new(),
            },
            index: None,
            attributes: None,
        }
    }
}

impl ShapefileDataset {
    /// Build a dataset from the three component byte streams.
    ///
    /// `shx` and `dbf` may be `None`; the geometry is always required.
    pub fn from_parts(
        shp: &[u8],
        shx: Option<&[u8]>,
        dbf: Option<&[u8]>,
    ) -> Result<ShapefileDataset, GeoError> {
        let shapefile = parse_shp(shp)?;
        let index = match shx {
            Some(b) => Some(parse_shx(b)?),
            None => None,
        };
        let attributes = match dbf {
            Some(b) => Some(parse_dbf(b)?),
            None => None,
        };
        Ok(ShapefileDataset { shapefile, index, attributes })
    }

    /// Number of geometry records.
    pub fn len(&self) -> usize {
        self.shapefile.records.len()
    }

    /// Whether the dataset has no records.
    pub fn is_empty(&self) -> bool {
        self.shapefile.records.is_empty()
    }

    /// Join each geometry record with its attribute row, if a `.dbf` is present.
    ///
    /// Returns `None` for a record with no corresponding attribute row.
    pub fn record_attributes(&self, record_index: usize) -> Option<&[String]> {
        self.attributes.as_ref().and_then(|d| d.records.get(record_index)).map(|v| v.as_slice())
    }
}

/// A Shapefile reader implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct ShapefileReader {
    settings: ValidationSettings,
}

impl ShapefileReader {
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

impl Reader<ShapefileDataset> for ShapefileReader {
    type Error = GeoError;

    fn parse(&self, input: &[u8]) -> Result<ShapefileDataset, GeoError> {
        // The reader parses a `.shp` only; use `from_parts` for the full trio.
        let shapefile = parse_shp(input)?;
        self.settings.run(&ShapefileValidator, &shapefile)?;
        Ok(ShapefileDataset { shapefile, index: None, attributes: None })
    }
}

/// Shapefile syntactic/semantic validator.
#[derive(Debug, Clone, Default)]
pub struct ShapefileValidator;

impl Validator<Shapefile> for ShapefileValidator {
    type Error = GeoError;

    fn validate_syntactic(&self, _sf: &Shapefile) -> Result<(), GeoError> {
        Ok(())
    }

    fn validate_semantic(&self, sf: &Shapefile) -> Result<(), GeoError> {
        if sf.header.file_length_words == 0 {
            return Err(GeoError::shapefile(0, "empty Shapefile"));
        }
        Ok(())
    }
}

/// Convenience: extract all 2D points from a geometry (for quick inspection).
pub fn geometry_points(g: &ShapeGeometry) -> Vec<Point> {
    match g {
        ShapeGeometry::Point(p) => alloc::vec![*p],
        ShapeGeometry::MultiPoint { points, .. } => points.clone(),
        ShapeGeometry::Parts { parts, .. } => parts.iter().flatten().copied().collect(),
        ShapeGeometry::Null => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal `.shp` with a single polygon record.
    fn build_shp() -> Vec<u8> {
        let mut b = Vec::new();
        // header
        b.extend_from_slice(&9994i32.to_be_bytes());
        for _ in 0..5 {
            b.extend_from_slice(&0i32.to_be_bytes());
        }
        // file length words: header(50) + record header(4) + content(64)
        let content_words = 64i32; // 128 bytes
        let file_len = 50 + 4 + content_words;
        b.extend_from_slice(&(file_len as u32).to_be_bytes());
        b.extend_from_slice(&1000i32.to_le_bytes()); // version
        b.extend_from_slice(&5i32.to_le_bytes()); // shape type Polygon
        for v in [0.0f64, 0.0, 10.0, 10.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for _ in 0..4 {
            b.extend_from_slice(&0.0f64.to_le_bytes()); // zmin, zmax, mmin, mmax
        }
        // record header
        b.extend_from_slice(&1i32.to_be_bytes()); // record number
        b.extend_from_slice(&content_words.to_be_bytes()); // content length words
                                                           // record content: shape type + bbox + numparts + numpoints + parts + points
        b.extend_from_slice(&5i32.to_le_bytes());
        for v in [0.0f64, 0.0, 10.0, 10.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&1i32.to_le_bytes()); // 1 part
        b.extend_from_slice(&5i32.to_le_bytes()); // 5 points
        b.extend_from_slice(&0i32.to_le_bytes()); // part 0 start
        for (x, y) in [
            (0.0f64, 0.0f64),
            (10.0f64, 0.0f64),
            (10.0f64, 10.0f64),
            (0.0f64, 10.0f64),
            (0.0f64, 0.0f64),
        ] {
            b.extend_from_slice(&x.to_le_bytes());
            b.extend_from_slice(&y.to_le_bytes());
        }
        b
    }

    #[test]
    fn parse_polygon_shp() {
        let shp = build_shp();
        let sf = parse_shp(&shp).unwrap();
        assert_eq!(sf.records.len(), 1);
        match &sf.records[0].geometry {
            ShapeGeometry::Parts { parts, .. } => {
                assert_eq!(parts.len(), 1);
                assert_eq!(parts[0].len(), 5);
                assert_eq!(parts[0][0].x, 0.0);
            }
            _ => panic!("expected parts"),
        }
    }

    #[test]
    fn parse_point_shp() {
        let mut b = Vec::new();
        b.extend_from_slice(&9994i32.to_be_bytes());
        for _ in 0..5 {
            b.extend_from_slice(&0i32.to_be_bytes());
        }
        let content_words = 10i32; // 20 bytes
        let file_len = 50 + 4 + content_words;
        b.extend_from_slice(&(file_len as u32).to_be_bytes());
        b.extend_from_slice(&1000i32.to_le_bytes());
        b.extend_from_slice(&1i32.to_le_bytes()); // Point
        for v in [0.0f64, 0.0, 1.0, 1.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for _ in 0..4 {
            b.extend_from_slice(&0.0f64.to_le_bytes());
        }
        b.extend_from_slice(&1i32.to_be_bytes());
        b.extend_from_slice(&content_words.to_be_bytes());
        b.extend_from_slice(&1i32.to_le_bytes()); // shape type
        b.extend_from_slice(&3.0f64.to_le_bytes());
        b.extend_from_slice(&4.0f64.to_le_bytes());
        let sf = parse_shp(&b).unwrap();
        assert_eq!(sf.records.len(), 1);
        match &sf.records[0].geometry {
            ShapeGeometry::Point(p) => {
                assert_eq!(p.x, 3.0);
                assert_eq!(p.y, 4.0);
            }
            _ => panic!("expected point"),
        }
    }

    #[test]
    fn rejects_non_shapefile() {
        let b = [0u8; 120];
        assert!(parse_shp(&b).is_err());
    }

    #[test]
    fn parse_dbf_basic() {
        let mut b = Vec::new();
        b.push(0x03); // version
        b.extend_from_slice(&[0, 0, 0]); // ymd
        b.extend_from_slice(&1u32.to_le_bytes()); // 1 record
        b.extend_from_slice(&(32u16 + 32 + 1).to_le_bytes()); // header len = 65
        b.extend_from_slice(&20u16.to_le_bytes()); // record len = 20
        b.extend_from_slice(&[0u8; 20]); // reserved
                                         // one field: name "ID", type 'N', len 10, decimal 0
        let mut name = [0u8; 11];
        name[..2].copy_from_slice(b"ID");
        b.extend_from_slice(&name);
        b.push(b'N');
        b.extend_from_slice(&[0u8; 4]); // displacement
        b.push(10); // length
        b.push(0); // decimal
        b.extend_from_slice(&[0u8; 14]); // rest
        b.push(0x0d); // terminator
                      // record: deletion flag + "42        "
        b.push(b' ');
        let mut cell = [b' '; 10];
        cell[..2].copy_from_slice(b"42");
        b.extend_from_slice(&cell);
        // pad to record length 20
        b.extend_from_slice(&[b' '; 9]);
        let dbf = parse_dbf(&b).unwrap();
        assert_eq!(dbf.fields.len(), 1);
        assert_eq!(dbf.fields[0].name, "ID");
        assert_eq!(dbf.records[0][0], "42");
    }
}
