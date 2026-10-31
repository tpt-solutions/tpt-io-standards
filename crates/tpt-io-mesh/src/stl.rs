//! STL reading and writing, ASCII and binary.

use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::MeshError;
use crate::Mesh;

/// Parse an STL file. Binary is detected from its length (84 + 50 × count bytes); anything else is read as
/// ASCII.
pub fn parse(input: &[u8]) -> Result<Mesh, MeshError> {
    if input.len() >= 84 {
        let count = u32::from_le_bytes([input[80], input[81], input[82], input[83]]) as usize;
        if input.len() == 84 + 50 * count && !input.starts_with(b"solid") {
            return binary(input, count);
        }
        if input.len() == 84 + 50 * count && count > 0 {
            // Some binary files start with "solid" in their header; the length check decides.
            return binary(input, count);
        }
    }
    ascii(input)
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn binary(input: &[u8], count: usize) -> Result<Mesh, MeshError> {
    let mut mesh = Mesh::default();
    for i in 0..count {
        let at = 84 + 50 * i;
        let normal = [f32_at(input, at), f32_at(input, at + 4), f32_at(input, at + 8)];
        let base = mesh.vertices.len() as u32;
        for v in 0..3 {
            let p = at + 12 + 12 * v;
            mesh.vertices.push([f32_at(input, p), f32_at(input, p + 4), f32_at(input, p + 8)]);
        }
        mesh.triangles.push([base, base + 1, base + 2]);
        mesh.normals.push(normal);
    }
    Ok(mesh)
}

fn ascii(input: &[u8]) -> Result<Mesh, MeshError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "STL text is not UTF-8"))?;
    let mut mesh = Mesh::default();
    let mut pending: Vec<[f32; 3]> = Vec::new();
    let mut normal = [0.0f32; 3];
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("facet normal") {
            normal = parse3(rest)?;
        } else if let Some(rest) = t.strip_prefix("vertex") {
            pending.push(parse3(rest)?);
        } else if t == "endfacet" {
            if pending.len() != 3 {
                return Err(MeshError::Invalid("facet does not have three vertices".to_string()));
            }
            let base = mesh.vertices.len() as u32;
            mesh.vertices.append(&mut pending);
            mesh.triangles.push([base, base + 1, base + 2]);
            mesh.normals.push(normal);
        }
    }
    Ok(mesh)
}

fn parse3(s: &str) -> Result<[f32; 3], MeshError> {
    let v: Vec<f32> = s
        .split_whitespace()
        .map(|t| t.parse::<f32>().map_err(|_| MeshError::Invalid("bad number".to_string())))
        .collect::<Result<_, _>>()?;
    match v.as_slice() {
        [x, y, z] => Ok([*x, *y, *z]),
        _ => Err(MeshError::Invalid("expected three numbers".to_string())),
    }
}

/// Write an ASCII STL. Each triangle's normal is the stored normal, or zero when none is stored.
pub fn write_ascii(mesh: &Mesh) -> Vec<u8> {
    let mut out = alloc::string::String::from("solid mesh\n");
    for (i, t) in mesh.triangles.iter().enumerate() {
        let n = mesh.normals.get(i).copied().unwrap_or([0.0; 3]);
        let _ = writeln!(out, "facet normal {} {} {}", n[0], n[1], n[2]);
        out.push_str("outer loop\n");
        for &idx in t {
            let v = mesh.vertices[idx as usize];
            let _ = writeln!(out, "vertex {} {} {}", v[0], v[1], v[2]);
        }
        out.push_str("endloop\nendfacet\n");
    }
    out.push_str("endsolid mesh\n");
    out.into_bytes()
}

/// Write a binary STL.
pub fn write_binary(mesh: &Mesh) -> Vec<u8> {
    let mut out = alloc::vec![0u8; 80];
    out.extend_from_slice(&(mesh.triangles.len() as u32).to_le_bytes());
    for (i, t) in mesh.triangles.iter().enumerate() {
        let n = mesh.normals.get(i).copied().unwrap_or([0.0; 3]);
        for c in n {
            out.extend_from_slice(&c.to_le_bytes());
        }
        for &idx in t {
            for c in mesh.vertices[idx as usize] {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        out.extend_from_slice(&[0, 0]);
    }
    out
}
