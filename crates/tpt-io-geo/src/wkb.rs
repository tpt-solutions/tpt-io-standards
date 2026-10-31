//! Well-known binary (WKB): the binary form of WKT used by spatial databases and GIS libraries.
//!
//! Reads ISO WKB (type codes 1 to 7, with 1000 added for Z) in either byte order, and the EWKB flags used by
//! PostGIS: the Z flag `0x8000_0000`, and the SRID flag `0x2000_0000` whose SRID is skipped. M and ZM are
//! rejected. Writing produces little-endian ISO WKB, with the Z type code when a position has an elevation.

use alloc::vec::Vec;

use crate::error::GeoError;
use crate::geojson::{Geometry, GeometryEnum, Position};

const EWKB_Z: u32 = 0x8000_0000;
const EWKB_M: u32 = 0x4000_0000;
const EWKB_SRID: u32 = 0x2000_0000;

/// Parse one WKB geometry. Trailing bytes after the geometry are an error.
pub fn parse_wkb(input: &[u8]) -> Result<Geometry, GeoError> {
    let mut r = Reader { b: input, pos: 0, le: true };
    let g = geometry(&mut r)?;
    if r.pos != input.len() {
        return Err(GeoError::custom(r.pos, "trailing bytes after WKB geometry"));
    }
    Ok(g)
}

/// Write one geometry as little-endian ISO WKB.
pub fn to_wkb(g: &Geometry) -> Vec<u8> {
    let mut out = Vec::new();
    write_geometry(&mut out, &g.geometry);
    out
}

fn plain(geometry: GeometryEnum) -> Geometry {
    Geometry { geometry, bbox: None, foreign_members: Vec::new() }
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
    le: bool,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], GeoError> {
        let s = self.b.get(self.pos..self.pos + N).ok_or_else(|| GeoError::eof(self.pos))?;
        let mut out = [0u8; N];
        out.copy_from_slice(s);
        self.pos += N;
        Ok(out)
    }

    fn u32(&mut self) -> Result<u32, GeoError> {
        let a = self.take::<4>()?;
        Ok(if self.le { u32::from_le_bytes(a) } else { u32::from_be_bytes(a) })
    }

    fn f64(&mut self) -> Result<f64, GeoError> {
        let a = self.take::<8>()?;
        Ok(if self.le { f64::from_le_bytes(a) } else { f64::from_be_bytes(a) })
    }

    /// A count of child items. Each item takes at least one byte, so a count larger than the remaining input
    /// is rejected before any loop runs.
    fn count(&mut self) -> Result<usize, GeoError> {
        let n = self.u32()? as usize;
        if n > self.b.len().saturating_sub(self.pos) {
            return Err(GeoError::eof(self.pos));
        }
        Ok(n)
    }
}

fn geometry(r: &mut Reader<'_>) -> Result<Geometry, GeoError> {
    Ok(plain(enum_geometry(r)?))
}

fn enum_geometry(r: &mut Reader<'_>) -> Result<GeometryEnum, GeoError> {
    let order = *r.b.get(r.pos).ok_or_else(|| GeoError::eof(r.pos))?;
    r.pos += 1;
    r.le = match order {
        0 => false,
        1 => true,
        _ => return Err(GeoError::custom(r.pos - 1, "WKB byte order must be 0 or 1")),
    };
    let raw = r.u32()?;
    if raw & EWKB_M != 0 {
        return Err(GeoError::Unsupported("WKB M and ZM coordinates are not supported"));
    }
    if raw & EWKB_SRID != 0 {
        r.u32()?;
    }
    let base = raw & !(EWKB_Z | EWKB_M | EWKB_SRID);
    let (kind, dim) = (base % 1000, base / 1000);
    if dim == 2 || dim == 3 {
        return Err(GeoError::Unsupported("WKB M and ZM coordinates are not supported"));
    }
    let z = raw & EWKB_Z != 0 || dim == 1;
    match kind {
        1 => Ok(GeometryEnum::Point(position(r, z)?)),
        2 => Ok(GeometryEnum::LineString(positions(r, z)?)),
        3 => Ok(GeometryEnum::Polygon(rings(r, z)?)),
        4 => {
            let mut out = Vec::new();
            for g in children(r)? {
                match g.geometry {
                    GeometryEnum::Point(p) => out.push(p),
                    _ => return Err(GeoError::custom(r.pos, "WKB MultiPoint holds a non-point")),
                }
            }
            Ok(GeometryEnum::MultiPoint(out))
        }
        5 => {
            let mut out = Vec::new();
            for g in children(r)? {
                match g.geometry {
                    GeometryEnum::LineString(ps) => out.push(ps),
                    _ => {
                        return Err(GeoError::custom(r.pos, "WKB MultiLineString holds a non-line"))
                    }
                }
            }
            Ok(GeometryEnum::MultiLineString(out))
        }
        6 => {
            let mut out = Vec::new();
            for g in children(r)? {
                match g.geometry {
                    GeometryEnum::Polygon(rs) => out.push(rs),
                    _ => {
                        return Err(GeoError::custom(r.pos, "WKB MultiPolygon holds a non-polygon"))
                    }
                }
            }
            Ok(GeometryEnum::MultiPolygon(out))
        }
        7 => Ok(GeometryEnum::GeometryCollection(children(r)?)),
        _ => Err(GeoError::custom(r.pos, "unknown WKB geometry type")),
    }
}

fn children(r: &mut Reader<'_>) -> Result<Vec<Geometry>, GeoError> {
    let n = r.count()?;
    let mut out = Vec::new();
    for _ in 0..n {
        out.push(geometry(r)?);
    }
    Ok(out)
}

fn position(r: &mut Reader<'_>, z: bool) -> Result<Position, GeoError> {
    let mut coords = alloc::vec![r.f64()?, r.f64()?];
    if z {
        coords.push(r.f64()?);
    }
    Ok(Position(coords))
}

fn positions(r: &mut Reader<'_>, z: bool) -> Result<Vec<Position>, GeoError> {
    let n = r.count()?;
    let mut out = Vec::new();
    for _ in 0..n {
        out.push(position(r, z)?);
    }
    Ok(out)
}

fn rings(r: &mut Reader<'_>, z: bool) -> Result<Vec<Vec<Position>>, GeoError> {
    let n = r.count()?;
    let mut out = Vec::new();
    for _ in 0..n {
        out.push(positions(r, z)?);
    }
    Ok(out)
}

fn header(out: &mut Vec<u8>, base: u32, z: bool) {
    out.push(1);
    let code = base + if z { 1000 } else { 0 };
    out.extend_from_slice(&code.to_le_bytes());
}

fn put_count(out: &mut Vec<u8>, n: usize) {
    out.extend_from_slice(&(n as u32).to_le_bytes());
}

fn put_position(out: &mut Vec<u8>, p: &Position, z: bool) {
    out.extend_from_slice(&p.longitude().to_le_bytes());
    out.extend_from_slice(&p.latitude().to_le_bytes());
    if z {
        out.extend_from_slice(&p.elevation().unwrap_or(0.0).to_le_bytes());
    }
}

fn put_positions(out: &mut Vec<u8>, ps: &[Position], z: bool) {
    put_count(out, ps.len());
    for p in ps {
        put_position(out, p, z);
    }
}

fn write_geometry(out: &mut Vec<u8>, g: &GeometryEnum) {
    let z = has_z(g);
    match g {
        GeometryEnum::Point(p) => {
            header(out, 1, z);
            put_position(out, p, z);
        }
        GeometryEnum::LineString(ps) => {
            header(out, 2, z);
            put_positions(out, ps, z);
        }
        GeometryEnum::Polygon(rings) => {
            header(out, 3, z);
            put_count(out, rings.len());
            for ring in rings {
                put_positions(out, ring, z);
            }
        }
        GeometryEnum::MultiPoint(ps) => {
            header(out, 4, z);
            put_count(out, ps.len());
            for p in ps {
                header(out, 1, z);
                put_position(out, p, z);
            }
        }
        GeometryEnum::MultiLineString(lines) => {
            header(out, 5, z);
            put_count(out, lines.len());
            for line in lines {
                header(out, 2, z);
                put_positions(out, line, z);
            }
        }
        GeometryEnum::MultiPolygon(polys) => {
            header(out, 6, z);
            put_count(out, polys.len());
            for rings in polys {
                header(out, 3, z);
                put_count(out, rings.len());
                for ring in rings {
                    put_positions(out, ring, z);
                }
            }
        }
        GeometryEnum::GeometryCollection(gs) => {
            header(out, 7, z);
            put_count(out, gs.len());
            for g in gs {
                write_geometry(out, &g.geometry);
            }
        }
    }
}

fn any_z(ps: &[Position]) -> bool {
    ps.iter().any(|p| p.dim() == 3)
}

fn has_z(g: &GeometryEnum) -> bool {
    match g {
        GeometryEnum::Point(p) => p.dim() == 3,
        GeometryEnum::MultiPoint(ps) | GeometryEnum::LineString(ps) => any_z(ps),
        GeometryEnum::MultiLineString(lines) | GeometryEnum::Polygon(lines) => {
            lines.iter().any(|l| any_z(l))
        }
        GeometryEnum::MultiPolygon(polys) => polys.iter().flatten().any(|l| any_z(l)),
        GeometryEnum::GeometryCollection(gs) => gs.iter().any(|g| has_z(&g.geometry)),
    }
}
