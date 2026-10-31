//! PLY reading (ASCII and binary little-endian) and ASCII writing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::MeshError;
use crate::Mesh;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Ascii,
    BinaryLe,
}

fn scalar_size(ty: &str) -> Result<usize, MeshError> {
    match ty {
        "char" | "uchar" | "int8" | "uint8" => Ok(1),
        "short" | "ushort" | "int16" | "uint16" => Ok(2),
        "int" | "uint" | "float" | "int32" | "uint32" | "float32" => Ok(4),
        "double" | "float64" => Ok(8),
        _ => Err(MeshError::Invalid("unknown PLY property type".to_string())),
    }
}

fn read_binary(ty: &str, b: &[u8]) -> f64 {
    match ty {
        "char" | "int8" => b[0] as i8 as f64,
        "uchar" | "uint8" => b[0] as f64,
        "short" | "int16" => i16::from_le_bytes([b[0], b[1]]) as f64,
        "ushort" | "uint16" => u16::from_le_bytes([b[0], b[1]]) as f64,
        "int" | "int32" => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
        "uint" | "uint32" => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
        "float" | "float32" => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
        _ => {
            let mut a = [0u8; 8];
            a.copy_from_slice(&b[..8]);
            f64::from_le_bytes(a)
        }
    }
}

/// One element declaration: its name, count, and properties (name, type, list length type if a list).
struct Element {
    name: String,
    count: usize,
    props: Vec<(String, String, Option<String>)>,
}

/// Parse a PLY file (ASCII or binary little-endian) with `vertex` and `face` elements.
pub fn parse(input: &[u8]) -> Result<Mesh, MeshError> {
    let header_end = find(input, b"end_header").ok_or_else(|| ParseError::custom(0, "missing end_header"))?;
    let after = header_end + b"end_header".len();
    let body_start = match input.get(after) {
        Some(b'\r') if input.get(after + 1) == Some(&b'\n') => after + 2,
        Some(b'\n') => after + 1,
        _ => return Err(ParseError::custom(after, "end_header is not followed by a line break").into()),
    };
    let header = core::str::from_utf8(&input[..header_end])
        .map_err(|e| ParseError::custom(e.valid_up_to(), "PLY header is not text"))?;
    let mut kind = Kind::Ascii;
    let mut elements: Vec<Element> = Vec::new();
    for line in header.lines() {
        let t: Vec<&str> = line.split_whitespace().collect();
        match t.as_slice() {
            ["format", fmt, _] => {
                kind = match *fmt {
                    "ascii" => Kind::Ascii,
                    "binary_little_endian" => Kind::BinaryLe,
                    _ => return Err(MeshError::Invalid("only ascii and binary_little_endian PLY are read".to_string())),
                }
            }
            ["element", name, count] => elements.push(Element {
                name: name.to_string(),
                count: count.parse().map_err(|_| MeshError::Invalid("bad element count".to_string()))?,
                props: Vec::new(),
            }),
            ["property", "list", count_ty, item_ty, name] => {
                if let Some(e) = elements.last_mut() {
                    e.props.push((name.to_string(), item_ty.to_string(), Some(count_ty.to_string())));
                }
            }
            ["property", ty, name] => {
                if let Some(e) = elements.last_mut() {
                    e.props.push((name.to_string(), ty.to_string(), None));
                }
            }
            _ => {}
        }
    }
    let mut mesh = Mesh::default();
    let mut pos = body_start;
    let mut ascii_tokens: Vec<&str> = Vec::new();
    let text_body = if kind == Kind::Ascii {
        core::str::from_utf8(&input[body_start..])
            .map_err(|e| ParseError::custom(body_start + e.valid_up_to(), "PLY body is not text"))?
    } else {
        ""
    };
    if kind == Kind::Ascii {
        ascii_tokens = text_body.split_whitespace().collect();
    }
    let mut next_ascii = 0usize;
    for e in &elements {
        for _ in 0..e.count {
            let mut x = 0f32;
            let mut y = 0f32;
            let mut z = 0f32;
            let mut indices: Vec<u32> = Vec::new();
            for (name, ty, list) in &e.props {
                let values: Vec<f64> = match list {
                    Some(count_ty) => {
                        let n = read_value(kind, count_ty, input, &ascii_tokens, &mut next_ascii, &mut pos)? as usize;
                        let mut v = Vec::with_capacity(n);
                        for _ in 0..n {
                            v.push(read_value(kind, ty, input, &ascii_tokens, &mut next_ascii, &mut pos)?);
                        }
                        v
                    }
                    None => alloc::vec![read_value(kind, ty, input, &ascii_tokens, &mut next_ascii, &mut pos)?],
                };
                if e.name == "vertex" {
                    match (name.as_str(), values.first()) {
                        ("x", Some(v)) => x = *v as f32,
                        ("y", Some(v)) => y = *v as f32,
                        ("z", Some(v)) => z = *v as f32,
                        _ => {}
                    }
                } else if e.name == "face" && (name == "vertex_indices" || name == "vertex_index") {
                    indices = values.iter().map(|v| *v as u32).collect();
                }
            }
            if e.name == "vertex" {
                mesh.vertices.push([x, y, z]);
            } else if e.name == "face" {
                if indices.len() < 3 {
                    return Err(MeshError::Invalid("face has fewer than three corners".to_string()));
                }
                for k in 1..indices.len() - 1 {
                    mesh.triangles.push([indices[0], indices[k], indices[k + 1]]);
                }
            }
        }
    }
    Ok(mesh)
}

fn read_value(
    kind: Kind,
    ty: &str,
    input: &[u8],
    ascii: &[&str],
    next_ascii: &mut usize,
    pos: &mut usize,
) -> Result<f64, MeshError> {
    match kind {
        Kind::Ascii => {
            let tok = ascii
                .get(*next_ascii)
                .ok_or_else(|| MeshError::Invalid("PLY body ends early".to_string()))?;
            *next_ascii += 1;
            tok.parse::<f64>().map_err(|_| MeshError::Invalid("bad PLY number".to_string()))
        }
        Kind::BinaryLe => {
            let size = scalar_size(ty)?;
            let bytes = input
                .get(*pos..*pos + size)
                .ok_or_else(|| ParseError::eof(*pos))?;
            let v = read_binary(ty, bytes);
            *pos += size;
            Ok(v)
        }
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Write a PLY file in ASCII form with `vertex` and `face` elements.
pub fn write(mesh: &Mesh) -> Vec<u8> {
    let mut out = String::from("ply\nformat ascii 1.0\n");
    let _ = writeln!(out, "element vertex {}", mesh.vertices.len());
    out.push_str("property float x\nproperty float y\nproperty float z\n");
    let _ = writeln!(out, "element face {}", mesh.triangles.len());
    out.push_str("property list uchar int vertex_indices\nend_header\n");
    for v in &mesh.vertices {
        let _ = writeln!(out, "{} {} {}", v[0], v[1], v[2]);
    }
    for t in &mesh.triangles {
        let _ = writeln!(out, "3 {} {} {}", t[0], t[1], t[2]);
    }
    out.into_bytes()
}
