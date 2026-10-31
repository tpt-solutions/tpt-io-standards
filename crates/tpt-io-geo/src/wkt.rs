//! Well-known text (WKT): the `POINT (x y)` form used by GIS tools and spatial databases.
//!
//! Reads and writes the seven OGC simple-feature types. An optional `Z` tag (`POINT Z (1 2 3)`) and `EMPTY`
//! bodies are accepted. M and ZM coordinates are rejected. Writing adds the `Z` tag, and a third ordinate, when
//! any position has an elevation.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::error::GeoError;
use crate::geojson::{Geometry, GeometryEnum, Position};

/// Parse one WKT geometry. Text after the geometry is an error.
pub fn parse_wkt(input: &str) -> Result<Geometry, GeoError> {
    let mut c = Cursor { s: input.as_bytes(), pos: 0 };
    let g = geometry(&mut c)?;
    c.skip_ws();
    if c.pos != c.s.len() {
        return Err(GeoError::custom(c.pos, "unexpected text after WKT geometry"));
    }
    Ok(g)
}

/// Write one geometry as WKT.
pub fn to_wkt(g: &Geometry) -> String {
    let mut s = String::new();
    write_geometry(&mut s, &g.geometry);
    s
}

fn plain(geometry: GeometryEnum) -> Geometry {
    Geometry { geometry, bbox: None, foreign_members: alloc::vec::Vec::new() }
}

fn unknown_kind(pos: usize) -> GeoError {
    GeoError::custom(pos, "malformed WKT: unknown geometry type")
}

struct Cursor<'a> {
    s: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn skip_ws(&mut self) {
        while self.pos < self.s.len() && self.s[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.s.get(self.pos).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), GeoError> {
        if self.eat(byte) {
            Ok(())
        } else {
            Err(GeoError::custom(self.pos, "malformed WKT: unexpected character"))
        }
    }

    /// The next run of ASCII letters, upper-cased. Empty when the next token is not a word.
    fn word(&mut self) -> String {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.s.len() && self.s[self.pos].is_ascii_alphabetic() {
            self.pos += 1;
        }
        core::str::from_utf8(&self.s[start..self.pos]).unwrap_or("").to_ascii_uppercase()
    }

    fn number(&mut self) -> Result<f64, GeoError> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.s.len()
            && matches!(self.s[self.pos], b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E')
        {
            self.pos += 1;
        }
        core::str::from_utf8(&self.s[start..self.pos])
            .ok()
            .and_then(|t| t.parse::<f64>().ok())
            .ok_or_else(|| GeoError::custom(start, "malformed WKT: expected a number"))
    }

    fn at_number(&mut self) -> bool {
        matches!(self.peek(), Some(b'0'..=b'9' | b'+' | b'-' | b'.'))
    }
}

fn geometry(c: &mut Cursor<'_>) -> Result<Geometry, GeoError> {
    Ok(plain(enum_geometry(c)?))
}

fn enum_geometry(c: &mut Cursor<'_>) -> Result<GeometryEnum, GeoError> {
    let kind = c.word();
    let tag = c.word();
    let (z, empty) = match tag.as_str() {
        "" => (false, false),
        "Z" => (true, false),
        "EMPTY" => (false, true),
        "M" | "ZM" => {
            return Err(GeoError::Unsupported("WKT M and ZM coordinates are not supported"))
        }
        _ => return Err(GeoError::custom(c.pos, "malformed WKT: unknown dimension tag")),
    };
    if empty {
        return match kind.as_str() {
            "POINT" => Err(GeoError::Unsupported("WKT POINT EMPTY has no position")),
            "LINESTRING" => Ok(GeometryEnum::LineString(Vec::new())),
            "POLYGON" => Ok(GeometryEnum::Polygon(Vec::new())),
            "MULTIPOINT" => Ok(GeometryEnum::MultiPoint(Vec::new())),
            "MULTILINESTRING" => Ok(GeometryEnum::MultiLineString(Vec::new())),
            "MULTIPOLYGON" => Ok(GeometryEnum::MultiPolygon(Vec::new())),
            "GEOMETRYCOLLECTION" => Ok(GeometryEnum::GeometryCollection(Vec::new())),
            _ => Err(unknown_kind(c.pos)),
        };
    }
    match kind.as_str() {
        "POINT" => {
            c.expect(b'(')?;
            let p = position(c, z)?;
            c.expect(b')')?;
            Ok(GeometryEnum::Point(p))
        }
        "LINESTRING" => Ok(GeometryEnum::LineString(positions(c, z)?)),
        "POLYGON" => Ok(GeometryEnum::Polygon(list(c, |c| positions(c, z))?)),
        "MULTIPOINT" => Ok(GeometryEnum::MultiPoint(list(c, |c| {
            // Both `MULTIPOINT (1 2, 3 4)` and `MULTIPOINT ((1 2), (3 4))` are valid WKT.
            if c.eat(b'(') {
                let p = position(c, z)?;
                c.expect(b')')?;
                Ok(p)
            } else {
                position(c, z)
            }
        })?)),
        "MULTILINESTRING" => Ok(GeometryEnum::MultiLineString(list(c, |c| positions(c, z))?)),
        "MULTIPOLYGON" => {
            Ok(GeometryEnum::MultiPolygon(list(c, |c| list(c, |c| positions(c, z)))?))
        }
        "GEOMETRYCOLLECTION" => Ok(GeometryEnum::GeometryCollection(list(c, geometry)?)),
        _ => Err(unknown_kind(c.pos)),
    }
}

/// A parenthesised, comma-separated list such as `(a, b, c)`.
fn list<'a, T>(
    c: &mut Cursor<'a>,
    mut item: impl FnMut(&mut Cursor<'a>) -> Result<T, GeoError>,
) -> Result<Vec<T>, GeoError> {
    c.expect(b'(')?;
    let mut out = Vec::new();
    loop {
        out.push(item(c)?);
        if !c.eat(b',') {
            break;
        }
    }
    c.expect(b')')?;
    Ok(out)
}

fn positions(c: &mut Cursor<'_>, z: bool) -> Result<Vec<Position>, GeoError> {
    list(c, |c| position(c, z))
}

fn position(c: &mut Cursor<'_>, z: bool) -> Result<Position, GeoError> {
    let lon = c.number()?;
    let lat = c.number()?;
    let mut coords = alloc::vec![lon, lat];
    if z || c.at_number() {
        coords.push(c.number()?);
    }
    Ok(Position(coords))
}

fn write_geometry(s: &mut String, g: &GeometryEnum) {
    let z = has_z(g);
    let tag = if z { " Z" } else { "" };
    match g {
        GeometryEnum::Point(p) => {
            let _ = write!(s, "POINT{tag} (");
            write_position(s, p, z);
            s.push(')');
        }
        GeometryEnum::MultiPoint(ps) => {
            let _ = write!(s, "MULTIPOINT{tag} ");
            list_or_empty(s, ps, |s, p| {
                s.push('(');
                write_position(s, p, z);
                s.push(')');
            });
        }
        GeometryEnum::LineString(ps) => {
            let _ = write!(s, "LINESTRING{tag} ");
            list_or_empty(s, ps, |s, p| write_position(s, p, z));
        }
        GeometryEnum::Polygon(rings) => {
            let _ = write!(s, "POLYGON{tag} ");
            list_or_empty(s, rings, |s, ring| join(s, ring, |s, p| write_position(s, p, z)));
        }
        GeometryEnum::MultiLineString(lines) => {
            let _ = write!(s, "MULTILINESTRING{tag} ");
            list_or_empty(s, lines, |s, line| join(s, line, |s, p| write_position(s, p, z)));
        }
        GeometryEnum::MultiPolygon(polys) => {
            let _ = write!(s, "MULTIPOLYGON{tag} ");
            list_or_empty(s, polys, |s, poly| {
                join(s, poly, |s, ring| join(s, ring, |s, p| write_position(s, p, z)))
            });
        }
        GeometryEnum::GeometryCollection(gs) => {
            let _ = write!(s, "GEOMETRYCOLLECTION{tag} ");
            list_or_empty(s, gs, |s, g| write_geometry(s, &g.geometry));
        }
    }
}

fn write_position(s: &mut String, p: &Position, z: bool) {
    let _ = write!(s, "{} {}", p.longitude(), p.latitude());
    if z {
        let _ = write!(s, " {}", p.elevation().unwrap_or(0.0));
    }
}

fn join<T>(s: &mut String, items: &[T], mut f: impl FnMut(&mut String, &T)) {
    s.push('(');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        f(s, item);
    }
    s.push(')');
}

fn list_or_empty<T>(s: &mut String, items: &[T], f: impl FnMut(&mut String, &T)) {
    if items.is_empty() {
        s.push_str("EMPTY");
    } else {
        join(s, items, f);
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
