//! Wavefront OBJ reading and writing (vertices and faces).

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::error::MeshError;
use crate::Mesh;

/// Parse an OBJ file: `v` vertices and `f` faces. Faces may use `i`, `i/t`, `i//n` or `i/t/n` references;
/// only the vertex index is used. Negative indices count back from the end. Faces are fan-triangulated.
pub fn parse(input: &[u8]) -> Result<Mesh, MeshError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| tpt_io_core::ParseError::custom(e.valid_up_to(), "OBJ is not UTF-8"))?;
    let mut mesh = Mesh::default();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if let Some(rest) = line.strip_prefix("v ") {
            let c: Vec<f32> = rest
                .split_whitespace()
                .take(3)
                .map(|t| t.parse::<f32>().map_err(|_| MeshError::Invalid("bad vertex".to_string())))
                .collect::<Result<_, _>>()?;
            if c.len() != 3 {
                return Err(MeshError::Invalid("vertex needs three coordinates".to_string()));
            }
            mesh.vertices.push([c[0], c[1], c[2]]);
        } else if let Some(rest) = line.strip_prefix("f ") {
            let count = mesh.vertices.len() as i64;
            let mut corners: Vec<u32> = Vec::new();
            for tok in rest.split_whitespace() {
                let raw = tok
                    .split('/')
                    .next()
                    .and_then(|s| s.parse::<i64>().ok())
                    .ok_or_else(|| MeshError::Invalid("bad face index".to_string()))?;
                let idx = if raw < 0 { count + raw } else { raw - 1 };
                if idx < 0 || idx >= count {
                    return Err(MeshError::Invalid("face refers to a missing vertex".to_string()));
                }
                corners.push(idx as u32);
            }
            if corners.len() < 3 {
                return Err(MeshError::Invalid("face has fewer than three corners".to_string()));
            }
            for k in 1..corners.len() - 1 {
                mesh.triangles.push([corners[0], corners[k], corners[k + 1]]);
            }
        }
    }
    Ok(mesh)
}

/// Write an OBJ file with vertices and triangular faces.
pub fn write(mesh: &Mesh) -> Vec<u8> {
    let mut out = String::new();
    for v in &mesh.vertices {
        let _ = writeln!(out, "v {} {} {}", v[0], v[1], v[2]);
    }
    for t in &mesh.triangles {
        let _ = writeln!(out, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1);
    }
    out.into_bytes()
}
