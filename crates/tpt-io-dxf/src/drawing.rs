//! DXF group-code stream, drawing model, and writer.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::DxfError;

/// One drawing entity. Coordinates are in drawing units.
#[derive(Debug, Clone, PartialEq)]
pub enum Entity {
    /// A straight line from `start` to `end`.
    Line { layer: String, start: [f64; 3], end: [f64; 3] },
    /// A circle.
    Circle { layer: String, center: [f64; 3], radius: f64 },
    /// A lightweight polyline; `vertices` are `[x, y]` pairs.
    LwPolyline { layer: String, closed: bool, vertices: Vec<[f64; 2]> },
    /// A single-line text entity.
    Text { layer: String, insert: [f64; 3], height: f64, value: String },
    /// Any other entity type, kept by its name and layer only.
    Other { kind: String, layer: String },
}

/// A parsed drawing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Drawing {
    /// The `$ACADVER` header variable, e.g. `AC1027`.
    pub version: Option<String>,
    /// Layer names from the layer table.
    pub layers: Vec<String>,
    /// Entities in file order.
    pub entities: Vec<Entity>,
}

/// One (code, value) pair from the stream.
struct Pair<'a> {
    code: i32,
    value: &'a str,
    offset: usize,
}

fn pairs(input: &[u8]) -> Result<Vec<Pair<'_>>, DxfError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8"))?;
    let mut out = Vec::new();
    let mut lines = text.split('\n');
    let mut offset = 0usize;
    while let Some(code_line) = lines.next() {
        let code_offset = offset;
        offset += code_line.len() + 1;
        if code_line.trim().is_empty() {
            continue;
        }
        let Some(value_line) = lines.next() else {
            return Err(DxfError::Invalid("group code without a value".to_string()));
        };
        offset += value_line.len() + 1;
        let code = code_line
            .trim()
            .parse::<i32>()
            .map_err(|_| ParseError::custom(code_offset, "group code is not an integer"))?;
        out.push(Pair { code, value: value_line.trim(), offset: code_offset });
    }
    Ok(out)
}

fn num(p: &Pair<'_>) -> Result<f64, DxfError> {
    p.value
        .parse::<f64>()
        .map_err(|_| ParseError::custom(p.offset, "expected a number").into())
}

/// Parse a DXF (ASCII) file.
pub fn parse(input: &[u8]) -> Result<Drawing, DxfError> {
    let ps = pairs(input)?;
    let mut d = Drawing::default();
    let mut i = 0usize;
    let mut in_entities = false;
    let mut in_layers = false;
    let mut in_header = false;
    let mut expect_version = false;
    while i < ps.len() {
        let p = &ps[i];
        if p.code == 0 && p.value == "SECTION" {
            i += 1;
            if let Some(name) = ps.get(i).filter(|q| q.code == 2) {
                in_entities = name.value == "ENTITIES";
                in_layers = name.value == "TABLES";
                in_header = name.value == "HEADER";
            }
            i += 1;
            continue;
        }
        if p.code == 0 && p.value == "ENDSEC" {
            in_entities = false;
            in_layers = false;
            in_header = false;
            i += 1;
            continue;
        }
        if in_header {
            if p.code == 9 {
                expect_version = p.value == "$ACADVER";
            } else if p.code == 1 && expect_version {
                d.version = Some(p.value.to_string());
                expect_version = false;
            }
            i += 1;
            continue;
        }
        if in_layers && p.code == 0 && p.value == "LAYER" {
            i += 1;
            while i < ps.len() && !(ps[i].code == 0) {
                if ps[i].code == 2 {
                    d.layers.push(ps[i].value.to_string());
                }
                i += 1;
            }
            continue;
        }
        if in_entities && p.code == 0 {
            let kind = p.value;
            let start = i + 1;
            let mut end = start;
            while end < ps.len() && ps[end].code != 0 {
                end += 1;
            }
            d.entities.push(entity(kind, &ps[start..end])?);
            i = end;
            continue;
        }
        i += 1;
    }
    Ok(d)
}

fn entity(kind: &str, body: &[Pair<'_>]) -> Result<Entity, DxfError> {
    let layer = body
        .iter()
        .find(|p| p.code == 8)
        .map(|p| p.value.to_string())
        .unwrap_or_else(|| "0".to_string());
    let coord = |base: i32| -> Result<[f64; 3], DxfError> {
        let mut out = [0.0; 3];
        for p in body {
            if p.code == base {
                out[0] = num(p)?;
            } else if p.code == base + 10 {
                out[1] = num(p)?;
            } else if p.code == base + 20 {
                out[2] = num(p)?;
            }
        }
        Ok(out)
    };
    let first = |code: i32| body.iter().find(|p| p.code == code);
    Ok(match kind {
        "LINE" => Entity::Line { layer, start: coord(10)?, end: coord(11)? },
        "CIRCLE" => Entity::Circle {
            layer,
            center: coord(10)?,
            radius: match first(40) {
                Some(p) => num(p)?,
                None => 0.0,
            },
        },
        "LWPOLYLINE" => {
            let closed = match first(70) {
                Some(p) => (p.value.parse::<i32>().unwrap_or(0) & 1) == 1,
                None => false,
            };
            let mut vertices: Vec<[f64; 2]> = Vec::new();
            let mut pending_x: Option<f64> = None;
            for p in body {
                match p.code {
                    10 => {
                        if let Some(x) = pending_x.take() {
                            vertices.push([x, 0.0]);
                        }
                        pending_x = Some(num(p)?);
                    }
                    20 => {
                        let x = pending_x
                            .take()
                            .ok_or_else(|| DxfError::Invalid("LWPOLYLINE Y without X".to_string()))?;
                        vertices.push([x, num(p)?]);
                    }
                    _ => {}
                }
            }
            Entity::LwPolyline { layer, closed, vertices }
        }
        "TEXT" => Entity::Text {
            layer,
            insert: coord(10)?,
            height: match first(40) {
                Some(p) => num(p)?,
                None => 0.0,
            },
            value: first(1).map(|p| p.value.to_string()).unwrap_or_default(),
        },
        other => Entity::Other { kind: other.to_string(), layer },
    })
}

/// Serialise a drawing as DXF. Writes the version, layer table and the supported entities.
pub fn write(d: &Drawing) -> Vec<u8> {
    let mut out = String::new();
    let pair = |out: &mut String, code: i32, value: &str| {
        let _ = writeln!(out, "{code}\n{value}");
    };
    pair(&mut out, 0, "SECTION");
    pair(&mut out, 2, "HEADER");
    if let Some(v) = &d.version {
        pair(&mut out, 9, "$ACADVER");
        pair(&mut out, 1, v);
    }
    pair(&mut out, 0, "ENDSEC");
    pair(&mut out, 0, "SECTION");
    pair(&mut out, 2, "TABLES");
    pair(&mut out, 0, "TABLE");
    pair(&mut out, 2, "LAYER");
    for l in &d.layers {
        pair(&mut out, 0, "LAYER");
        pair(&mut out, 2, l);
    }
    pair(&mut out, 0, "ENDTAB");
    pair(&mut out, 0, "ENDSEC");
    pair(&mut out, 0, "SECTION");
    pair(&mut out, 2, "ENTITIES");
    for e in &d.entities {
        match e {
            Entity::Line { layer, start, end } => {
                pair(&mut out, 0, "LINE");
                pair(&mut out, 8, layer);
                point(&mut out, 10, start);
                point(&mut out, 11, end);
            }
            Entity::Circle { layer, center, radius } => {
                pair(&mut out, 0, "CIRCLE");
                pair(&mut out, 8, layer);
                point(&mut out, 10, center);
                pair(&mut out, 40, &alloc::format!("{radius}"));
            }
            Entity::LwPolyline { layer, closed, vertices } => {
                pair(&mut out, 0, "LWPOLYLINE");
                pair(&mut out, 8, layer);
                pair(&mut out, 90, &alloc::format!("{}", vertices.len()));
                pair(&mut out, 70, if *closed { "1" } else { "0" });
                for v in vertices {
                    pair(&mut out, 10, &alloc::format!("{}", v[0]));
                    pair(&mut out, 20, &alloc::format!("{}", v[1]));
                }
            }
            Entity::Text { layer, insert, height, value } => {
                pair(&mut out, 0, "TEXT");
                pair(&mut out, 8, layer);
                point(&mut out, 10, insert);
                pair(&mut out, 40, &alloc::format!("{height}"));
                pair(&mut out, 1, value);
            }
            Entity::Other { kind, layer } => {
                pair(&mut out, 0, kind);
                pair(&mut out, 8, layer);
            }
        }
    }
    pair(&mut out, 0, "ENDSEC");
    pair(&mut out, 0, "EOF");
    out.into_bytes()
}

fn point(out: &mut String, base: i32, p: &[f64; 3]) {
    let _ = writeln!(out, "{base}\n{}", p[0]);
    let _ = writeln!(out, "{}\n{}", base + 10, p[1]);
    let _ = writeln!(out, "{}\n{}", base + 20, p[2]);
}
