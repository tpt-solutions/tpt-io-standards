//! GDSII records: real-number conversion, parsing into a library model, and writing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::error::GdsError;

const HEADER: u8 = 0x00;
const BGNLIB: u8 = 0x01;
const LIBNAME: u8 = 0x02;
const UNITS: u8 = 0x03;
const ENDLIB: u8 = 0x04;
const BGNSTR: u8 = 0x05;
const STRNAME: u8 = 0x06;
const ENDSTR: u8 = 0x07;
const BOUNDARY: u8 = 0x08;
const PATH: u8 = 0x09;
const SREF: u8 = 0x0A;
const TEXT: u8 = 0x0C;
const LAYER: u8 = 0x0D;
const DATATYPE: u8 = 0x0E;
const WIDTH: u8 = 0x0F;
const XY: u8 = 0x10;
const ENDEL: u8 = 0x11;
const SNAME: u8 = 0x12;
const TEXTTYPE: u8 = 0x16;
const STRING: u8 = 0x19;

/// One element of a structure.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    /// A polygon (`BOUNDARY`); the first point repeats at the end.
    Boundary { layer: i16, datatype: i16, xy: Vec<(i32, i32)> },
    /// A path (`PATH`) of the given width.
    Path { layer: i16, datatype: i16, width: i32, xy: Vec<(i32, i32)> },
    /// A text label (`TEXT`).
    Text { layer: i16, texttype: i16, at: (i32, i32), string: String },
    /// A reference to another structure (`SREF`).
    SRef { name: String, at: (i32, i32) },
}

/// A structure: a named cell and its elements.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Structure {
    /// The structure name.
    pub name: String,
    /// Elements in file order.
    pub elements: Vec<Element>,
}

/// A library: its name, units, and structures.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Library {
    /// The library name.
    pub name: String,
    /// User units per database unit (the first `UNITS` value).
    pub user_units: f64,
    /// Metres per database unit (the second `UNITS` value).
    pub db_units: f64,
    /// Structures in file order.
    pub structures: Vec<Structure>,
}

/// Convert a GDSII 8-byte real (base-16 excess-64) to `f64`.
pub fn decode_real8(b: [u8; 8]) -> f64 {
    let sign = if b[0] & 0x80 != 0 { -1.0 } else { 1.0 };
    let exp = (b[0] & 0x7F) as i32 - 64;
    let mut mant: u64 = 0;
    for &x in &b[1..] {
        mant = (mant << 8) | x as u64;
    }
    if mant == 0 {
        return 0.0;
    }
    sign * (mant as f64) / 72_057_594_037_927_936.0 * 16f64.powi(exp)
}

/// Convert an `f64` to a GDSII 8-byte real.
pub fn encode_real8(v: f64) -> [u8; 8] {
    if v == 0.0 || !v.is_finite() {
        return [0; 8];
    }
    let sign: u8 = if v < 0.0 { 0x80 } else { 0 };
    let mut m = v.abs();
    let mut e: i32 = 64;
    while m >= 1.0 {
        m /= 16.0;
        e += 1;
    }
    while m < 1.0 / 16.0 {
        m *= 16.0;
        e -= 1;
    }
    let mut mant = (m * 72_057_594_037_927_936.0).round() as u64;
    if mant >= 72_057_594_037_927_936 {
        mant >>= 4;
        e += 1;
    }
    let mut out = [0u8; 8];
    out[0] = sign | (e as u8 & 0x7F);
    for i in 0..7 {
        out[7 - i] = ((mant >> (8 * i)) & 0xFF) as u8;
    }
    out
}

struct Record<'a> {
    kind: u8,
    data: &'a [u8],
}

fn records(input: &[u8]) -> Result<Vec<Record<'_>>, GdsError> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < input.len() {
        if pos + 4 > input.len() {
            return Err(ParseError::eof(pos).into());
        }
        let len = u16::from_be_bytes([input[pos], input[pos + 1]]) as usize;
        if len < 4 || pos + len > input.len() {
            return Err(ParseError::custom(pos, "record length is out of range").into());
        }
        out.push(Record { kind: input[pos + 2], data: &input[pos + 4..pos + len] });
        pos += len;
    }
    Ok(out)
}

fn text(data: &[u8]) -> String {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).to_string()
}

fn i16s(data: &[u8]) -> Vec<i16> {
    data.chunks_exact(2).map(|c| i16::from_be_bytes([c[0], c[1]])).collect()
}

fn i32s(data: &[u8]) -> Vec<i32> {
    data.chunks_exact(4).map(|c| i32::from_be_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn pairs(data: &[u8]) -> Vec<(i32, i32)> {
    i32s(data).chunks_exact(2).map(|c| (c[0], c[1])).collect()
}

fn real8s(data: &[u8]) -> Vec<f64> {
    data.chunks_exact(8)
        .map(|c| {
            let mut b = [0u8; 8];
            b.copy_from_slice(c);
            decode_real8(b)
        })
        .collect()
}

/// Parse a GDSII stream into a library.
pub fn parse(input: &[u8]) -> Result<Library, GdsError> {
    let recs = records(input)?;
    let mut lib = Library::default();
    let mut current: Option<Structure> = None;
    let mut element: Option<Element> = None;
    let mut seen_end = false;
    for r in &recs {
        match r.kind {
            HEADER | BGNLIB => {}
            LIBNAME => lib.name = text(r.data),
            UNITS => {
                let u = real8s(r.data);
                if u.len() == 2 {
                    lib.user_units = u[0];
                    lib.db_units = u[1];
                }
            }
            ENDLIB => seen_end = true,
            BGNSTR => current = Some(Structure::default()),
            STRNAME => {
                if let Some(s) = current.as_mut() {
                    s.name = text(r.data);
                }
            }
            ENDSTR => {
                let s = current.take().ok_or_else(|| GdsError::Invalid("ENDSTR outside a structure".into()))?;
                lib.structures.push(s);
            }
            BOUNDARY => element = Some(Element::Boundary { layer: 0, datatype: 0, xy: Vec::new() }),
            PATH => element = Some(Element::Path { layer: 0, datatype: 0, width: 0, xy: Vec::new() }),
            TEXT => element = Some(Element::Text { layer: 0, texttype: 0, at: (0, 0), string: String::new() }),
            SREF => element = Some(Element::SRef { name: String::new(), at: (0, 0) }),
            LAYER => set_i16(&mut element, r.data, |e, v| match e {
                Element::Boundary { layer, .. } | Element::Path { layer, .. } | Element::Text { layer, .. } => *layer = v,
                _ => {}
            }),
            DATATYPE => set_i16(&mut element, r.data, |e, v| match e {
                Element::Boundary { datatype, .. } | Element::Path { datatype, .. } => *datatype = v,
                _ => {}
            }),
            TEXTTYPE => set_i16(&mut element, r.data, |e, v| {
                if let Element::Text { texttype, .. } = e {
                    *texttype = v;
                }
            }),
            WIDTH => {
                if let (Some(Element::Path { width, .. }), Some(v)) = (element.as_mut(), i32s(r.data).first()) {
                    *width = *v;
                }
            }
            XY => {
                let points = pairs(r.data);
                match element.as_mut() {
                    Some(Element::Boundary { xy, .. }) | Some(Element::Path { xy, .. }) => *xy = points,
                    Some(Element::Text { at, .. }) | Some(Element::SRef { at, .. }) => {
                        *at = *points.first().ok_or_else(|| GdsError::Invalid("XY without a point".into()))?;
                    }
                    None => return Err(GdsError::Invalid("XY outside an element".into())),
                }
            }
            SNAME => {
                if let Some(Element::SRef { name, .. }) = element.as_mut() {
                    *name = text(r.data);
                }
            }
            STRING => {
                if let Some(Element::Text { string, .. }) = element.as_mut() {
                    *string = text(r.data);
                }
            }
            ENDEL => {
                let e = element.take().ok_or_else(|| GdsError::Invalid("ENDEL without an element".into()))?;
                current
                    .as_mut()
                    .ok_or_else(|| GdsError::Invalid("element outside a structure".into()))?
                    .elements
                    .push(e);
            }
            _ => {}
        }
    }
    if !seen_end {
        return Err(GdsError::Invalid("stream has no ENDLIB".into()));
    }
    Ok(lib)
}

fn set_i16(element: &mut Option<Element>, data: &[u8], f: impl FnOnce(&mut Element, i16)) {
    if let (Some(e), Some(v)) = (element.as_mut(), i16s(data).first()) {
        f(e, *v);
    }
}

fn record(out: &mut Vec<u8>, kind: u8, dtype: u8, data: &[u8]) {
    let len = (data.len() + 4) as u16;
    out.extend_from_slice(&len.to_be_bytes());
    out.push(kind);
    out.push(dtype);
    out.extend_from_slice(data);
}

fn string_record(out: &mut Vec<u8>, kind: u8, s: &str) {
    let mut bytes = s.as_bytes().to_vec();
    if bytes.len() % 2 != 0 {
        bytes.push(0);
    }
    record(out, kind, 6, &bytes);
}

fn i16_record(out: &mut Vec<u8>, kind: u8, v: i16) {
    record(out, kind, 2, &v.to_be_bytes());
}

fn xy_record(out: &mut Vec<u8>, points: &[(i32, i32)]) {
    let mut data = Vec::with_capacity(points.len() * 8);
    for (x, y) in points {
        data.extend_from_slice(&x.to_be_bytes());
        data.extend_from_slice(&y.to_be_bytes());
    }
    record(out, XY, 3, &data);
}

/// Serialise a library as a GDSII stream. Dates in the `BGNLIB` and `BGNSTR` records are written as zeros.
pub fn write(lib: &Library) -> Result<Vec<u8>, GdsError> {
    let mut out = Vec::new();
    i16_record(&mut out, HEADER, 600);
    record(&mut out, BGNLIB, 2, &[0; 24]);
    string_record(&mut out, LIBNAME, &lib.name);
    let mut units = Vec::with_capacity(16);
    units.extend_from_slice(&encode_real8(lib.user_units));
    units.extend_from_slice(&encode_real8(lib.db_units));
    record(&mut out, UNITS, 5, &units);
    for s in &lib.structures {
        record(&mut out, BGNSTR, 2, &[0; 24]);
        string_record(&mut out, STRNAME, &s.name);
        for e in &s.elements {
            match e {
                Element::Boundary { layer, datatype, xy } => {
                    record(&mut out, BOUNDARY, 0, &[]);
                    i16_record(&mut out, LAYER, *layer);
                    i16_record(&mut out, DATATYPE, *datatype);
                    xy_record(&mut out, xy);
                }
                Element::Path { layer, datatype, width, xy } => {
                    record(&mut out, PATH, 0, &[]);
                    i16_record(&mut out, LAYER, *layer);
                    i16_record(&mut out, DATATYPE, *datatype);
                    record(&mut out, WIDTH, 3, &width.to_be_bytes());
                    xy_record(&mut out, xy);
                }
                Element::Text { layer, texttype, at, string } => {
                    record(&mut out, TEXT, 0, &[]);
                    i16_record(&mut out, LAYER, *layer);
                    i16_record(&mut out, TEXTTYPE, *texttype);
                    xy_record(&mut out, &[*at]);
                    string_record(&mut out, STRING, string);
                }
                Element::SRef { name, at } => {
                    record(&mut out, SREF, 0, &[]);
                    string_record(&mut out, SNAME, name);
                    xy_record(&mut out, &[*at]);
                }
            }
            record(&mut out, ENDEL, 0, &[]);
        }
        record(&mut out, ENDSTR, 0, &[]);
    }
    record(&mut out, ENDLIB, 0, &[]);
    Ok(out)
}
