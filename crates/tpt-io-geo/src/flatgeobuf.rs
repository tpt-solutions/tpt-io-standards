//! FlatGeobuf: a binary vector format with a FlatBuffers header, feature records and an optional packed index.
//!
//! [`read`] reads a FlatGeobuf file into a [`FeatureCollection`]. It reads the header (the column list and the feature
//! count), skips the spatial index, and decodes each feature's geometry and attributes. Writing is not implemented.
//!
//! The FlatBuffers tables are read directly through their vtables, so the reader needs no generated code. Attribute
//! values are typed by the header's column list. Binary attributes are read as `null`.
//!
//! Spec access: FlatGeobuf is an open format published by its authors (flatgeobuf.org) under a BSD licence. No
//! specification text is copied into this crate.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::GeoError;
use crate::geojson::{Feature, FeatureCollection, Geometry, GeometryEnum, Position};
use crate::json::Json;

/// The first seven bytes of a FlatGeobuf file: `fgb`, the format version, then `fgb`. The eighth byte is the
/// format's minor version and is not checked.
const MAGIC: &[u8] = b"fgb\x03fgb";
/// The size of one packed index node: four doubles for the bounding box and one 8-byte offset.
const NODE_ITEM_LEN: usize = 40;
/// The index node size assumed when the header leaves it out.
const DEFAULT_NODE_SIZE: usize = 16;

/// Read a FlatGeobuf file into a feature collection.
pub fn read(input: &[u8]) -> Result<FeatureCollection, GeoError> {
    if !input.starts_with(MAGIC) {
        return Err(GeoError::custom(0, "not a FlatGeobuf file"));
    }
    let header_len = u32_at(input, 8)? as usize;
    let header = Table::root(slice(input, 12, header_len)?)?;
    let geometry_type = header.u8(2)?.unwrap_or(0);
    let columns = header.tables(7)?.iter().map(column).collect::<Result<Vec<_>, _>>()?;
    let count = header.u64(8)?.unwrap_or(0) as usize;
    let node_size = header.u16(9)?.map_or(DEFAULT_NODE_SIZE, |n| n as usize);

    let mut at = 12 + header_len;
    at += index_size(count, node_size);
    let mut features = Vec::new();
    for _ in 0..count {
        let len = u32_at(input, at)? as usize;
        let buf = slice(input, at + 4, len)?;
        features.push(feature(buf, geometry_type, &columns)?);
        at += 4 + len;
    }
    Ok(FeatureCollection { features, bbox: None, foreign_members: Vec::new() })
}

fn slice(input: &[u8], at: usize, len: usize) -> Result<&[u8], GeoError> {
    input.get(at..at + len).ok_or_else(|| GeoError::eof(at))
}

fn u32_at(input: &[u8], at: usize) -> Result<u32, GeoError> {
    let s = slice(input, at, 4)?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// The bytes of the packed index that follows the features' header: the bottom level holds one node per item, and
/// each level above holds one node per `node_size` nodes below, up to a single root.
fn index_size(items: usize, node_size: usize) -> usize {
    if items == 0 || node_size == 0 {
        return 0;
    }
    let mut nodes = items;
    let mut level = items;
    loop {
        level = level.div_ceil(node_size);
        nodes += level;
        if level == 1 {
            break;
        }
    }
    nodes * NODE_ITEM_LEN
}

/// A FlatBuffers table: the buffer it lives in, and the table's position in that buffer.
#[derive(Clone, Copy)]
struct Table<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Table<'a> {
    /// The root table of a FlatBuffer: the buffer starts with the offset of the root table.
    fn root(buf: &'a [u8]) -> Result<Self, GeoError> {
        let pos = u32_at(buf, 0)? as usize;
        Ok(Table { buf, pos })
    }

    /// The position of field `index`, or `None` when the table does not store it (its default applies).
    fn field(&self, index: usize) -> Result<Option<usize>, GeoError> {
        let soffset = i32_at(self.buf, self.pos)?;
        let vtable = (self.pos as i64 - soffset as i64) as usize;
        let vsize = u16_at(self.buf, vtable)? as usize;
        let slot = 4 + 2 * index;
        if slot + 2 > vsize {
            return Ok(None);
        }
        let offset = u16_at(self.buf, vtable + slot)? as usize;
        Ok(if offset == 0 { None } else { Some(self.pos + offset) })
    }

    fn u8(&self, index: usize) -> Result<Option<u8>, GeoError> {
        match self.field(index)? {
            Some(at) => Ok(Some(slice(self.buf, at, 1)?[0])),
            None => Ok(None),
        }
    }

    fn u16(&self, index: usize) -> Result<Option<u16>, GeoError> {
        match self.field(index)? {
            Some(at) => Ok(Some(u16_at(self.buf, at)?)),
            None => Ok(None),
        }
    }

    fn u64(&self, index: usize) -> Result<Option<u64>, GeoError> {
        match self.field(index)? {
            Some(at) => Ok(Some(le_u64(self.buf, at)?)),
            None => Ok(None),
        }
    }

    fn string(&self, index: usize) -> Result<Option<String>, GeoError> {
        match self.vector(index)? {
            Some((start, len)) => {
                Ok(Some(String::from_utf8_lossy(slice(self.buf, start, len)?).into_owned()))
            }
            None => Ok(None),
        }
    }

    /// A nested table stored by offset.
    fn table(&self, index: usize) -> Result<Option<Table<'a>>, GeoError> {
        match self.field(index)? {
            Some(at) => Ok(Some(Table { buf: self.buf, pos: indirect(self.buf, at)? })),
            None => Ok(None),
        }
    }

    /// A vector stored by offset, as (first element position, element count).
    fn vector(&self, index: usize) -> Result<Option<(usize, usize)>, GeoError> {
        match self.field(index)? {
            Some(at) => {
                let start = indirect(self.buf, at)?;
                Ok(Some((start + 4, u32_at(self.buf, start)? as usize)))
            }
            None => Ok(None),
        }
    }

    fn f64s(&self, index: usize) -> Result<Vec<f64>, GeoError> {
        let mut out = Vec::new();
        if let Some((start, len)) = self.vector(index)? {
            for k in 0..len {
                out.push(f64::from_le_bytes(array8(self.buf, start + 8 * k)?));
            }
        }
        Ok(out)
    }

    fn u32s(&self, index: usize) -> Result<Vec<usize>, GeoError> {
        let mut out = Vec::new();
        if let Some((start, len)) = self.vector(index)? {
            for k in 0..len {
                out.push(u32_at(self.buf, start + 4 * k)? as usize);
            }
        }
        Ok(out)
    }

    fn bytes(&self, index: usize) -> Result<Vec<u8>, GeoError> {
        match self.vector(index)? {
            Some((start, len)) => Ok(slice(self.buf, start, len)?.to_vec()),
            None => Ok(Vec::new()),
        }
    }

    fn tables(&self, index: usize) -> Result<Vec<Table<'a>>, GeoError> {
        let mut out = Vec::new();
        if let Some((start, len)) = self.vector(index)? {
            for k in 0..len {
                let elem = start + 4 * k;
                out.push(Table { buf: self.buf, pos: indirect(self.buf, elem)? });
            }
        }
        Ok(out)
    }
}

/// Follow an offset stored at `at`: the target is `at` plus the unsigned 32-bit value stored there.
fn indirect(buf: &[u8], at: usize) -> Result<usize, GeoError> {
    Ok(at + u32_at(buf, at)? as usize)
}

fn i32_at(buf: &[u8], at: usize) -> Result<i32, GeoError> {
    Ok(u32_at(buf, at)? as i32)
}

fn u16_at(buf: &[u8], at: usize) -> Result<u16, GeoError> {
    let s = slice(buf, at, 2)?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}

fn le_u64(buf: &[u8], at: usize) -> Result<u64, GeoError> {
    Ok(u64::from_le_bytes(array8(buf, at)?))
}

fn array8(buf: &[u8], at: usize) -> Result<[u8; 8], GeoError> {
    let s = slice(buf, at, 8)?;
    let mut out = [0u8; 8];
    out.copy_from_slice(s);
    Ok(out)
}

/// A column from the header: its name and its type code.
fn column(t: &Table<'_>) -> Result<(String, u8), GeoError> {
    Ok((t.string(0)?.unwrap_or_default(), t.u8(1)?.unwrap_or(0)))
}

fn feature(buf: &[u8], default_type: u8, columns: &[(String, u8)]) -> Result<Feature, GeoError> {
    let root = Table::root(buf)?;
    let geometry = match root.table(0)? {
        Some(g) => Some(Geometry {
            geometry: geometry(&g, default_type)?,
            bbox: None,
            foreign_members: Vec::new(),
        }),
        None => None,
    };
    let properties = properties(&root.bytes(1)?, columns)?;
    Ok(Feature {
        geometry,
        properties: Json::Object(properties),
        id: None,
        bbox: None,
        foreign_members: Vec::new(),
    })
}

fn geometry(g: &Table<'_>, default_type: u8) -> Result<GeometryEnum, GeoError> {
    let kind = g.u8(6)?.filter(|&k| k != 0).unwrap_or(default_type);
    let positions = positions(&g.f64s(1)?, &g.f64s(2)?)?;
    let ends = g.u32s(0)?;
    let parts = g.tables(7)?;
    Ok(match kind {
        1 => GeometryEnum::Point(
            positions
                .into_iter()
                .next()
                .ok_or_else(|| GeoError::custom(0, "point feature has no coordinates"))?,
        ),
        2 => GeometryEnum::LineString(positions),
        3 => GeometryEnum::Polygon(split(positions, &ends)),
        4 => GeometryEnum::MultiPoint(positions),
        5 => GeometryEnum::MultiLineString(split(positions, &ends)),
        6 if parts.is_empty() => GeometryEnum::MultiPolygon(alloc::vec![split(positions, &ends)]),
        6 => {
            let mut polygons = Vec::new();
            for part in &parts {
                match geometry(part, 3)? {
                    GeometryEnum::Polygon(rings) => polygons.push(rings),
                    _ => return Err(GeoError::custom(0, "MultiPolygon part is not a polygon")),
                }
            }
            GeometryEnum::MultiPolygon(polygons)
        }
        7 => {
            let mut members = Vec::new();
            for part in &parts {
                members.push(Geometry {
                    geometry: geometry(part, 0)?,
                    bbox: None,
                    foreign_members: Vec::new(),
                });
            }
            GeometryEnum::GeometryCollection(members)
        }
        _ => return Err(GeoError::Unsupported("FlatGeobuf geometry type not read")),
    })
}

/// Pair up the `xy` doubles, with the `z` doubles appended when present.
fn positions(xy: &[f64], z: &[f64]) -> Result<Vec<Position>, GeoError> {
    if xy.len() % 2 != 0 {
        return Err(GeoError::custom(0, "FlatGeobuf coordinates come in x, y pairs"));
    }
    let mut out = Vec::with_capacity(xy.len() / 2);
    for (i, pair) in xy.chunks_exact(2).enumerate() {
        let mut coords = alloc::vec![pair[0], pair[1]];
        if let Some(&zi) = z.get(i) {
            coords.push(zi);
        }
        out.push(Position(coords));
    }
    Ok(out)
}

/// Split positions into parts at the given end indices (in coordinates). With no ends, all positions form one part.
fn split(positions: Vec<Position>, ends: &[usize]) -> Vec<Vec<Position>> {
    if ends.is_empty() {
        return alloc::vec![positions];
    }
    let mut out = Vec::with_capacity(ends.len());
    let mut start = 0;
    for &end in ends {
        let end = end.min(positions.len());
        out.push(positions[start.min(end)..end].to_vec());
        start = end;
    }
    out
}

/// Decode a feature's attributes: each entry is a column index (2 bytes) followed by that column's value.
fn properties(bytes: &[u8], columns: &[(String, u8)]) -> Result<Vec<(String, Json)>, GeoError> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let index = u16_at(bytes, at)? as usize;
        at += 2;
        let (name, ctype) = columns
            .get(index)
            .ok_or_else(|| GeoError::custom(at, "property refers to an unknown column"))?;
        let (value, size) = value(bytes, at, *ctype)?;
        at += size;
        out.push((name.clone(), value));
    }
    Ok(out)
}

/// One attribute value of the given column type, and its size in bytes.
fn value(b: &[u8], at: usize, ctype: u8) -> Result<(Json, usize), GeoError> {
    let fixed = |n: usize| slice(b, at, n);
    Ok(match ctype {
        0 => (Json::Number(fixed(1)?[0] as i8 as f64), 1),
        1 => (Json::Number(fixed(1)?[0] as f64), 1),
        2 => (Json::Bool(fixed(1)?[0] != 0), 1),
        3 => (Json::Number(i16::from_le_bytes([fixed(2)?[0], fixed(2)?[1]]) as f64), 2),
        4 => (Json::Number(u16::from_le_bytes([fixed(2)?[0], fixed(2)?[1]]) as f64), 2),
        5 => (Json::Number(i32::from_le_bytes(four(b, at)?) as f64), 4),
        6 => (Json::Number(u32::from_le_bytes(four(b, at)?) as f64), 4),
        7 => (Json::Number(i64::from_le_bytes(array8(b, at)?) as f64), 8),
        8 => (Json::Number(u64::from_le_bytes(array8(b, at)?) as f64), 8),
        9 => (Json::Number(f32::from_le_bytes(four(b, at)?) as f64), 4),
        10 => (Json::Number(f64::from_le_bytes(array8(b, at)?)), 8),
        11..=13 => {
            let len = u32::from_le_bytes(four(b, at)?) as usize;
            let text = String::from_utf8_lossy(slice(b, at + 4, len)?).into_owned();
            (Json::String(text), 4 + len)
        }
        14 => {
            let len = u32::from_le_bytes(four(b, at)?) as usize;
            slice(b, at + 4, len)?;
            (Json::Null, 4 + len)
        }
        _ => return Err(GeoError::Unsupported("FlatGeobuf column type not read")),
    })
}

fn four(b: &[u8], at: usize) -> Result<[u8; 4], GeoError> {
    let s = slice(b, at, 4)?;
    Ok([s[0], s[1], s[2], s[3]])
}
