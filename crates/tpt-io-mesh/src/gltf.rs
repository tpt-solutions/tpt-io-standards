//! glTF 2.0 binary (GLB): the triangle geometry of a file's meshes.
//!
//! [`parse_glb`] reads the positions and indices of every `TRIANGLES` primitive. Primitives in other modes are
//! skipped, and so are normals, texture coordinates, materials and node transforms, so the result is
//! object-space geometry only. Buffers other than the GLB binary chunk (external files and data URIs) are
//! rejected. [`write_glb`] writes one mesh as a single-node GLB with a position and an index accessor.
//!
//! Spec access: glTF 2.0 is a Khronos standard published under a royalty-free licence. No specification text is
//! copied into this crate.

use alloc::string::ToString;
use alloc::vec::Vec;

use tpt_io_core::json::{parse_json, Json};
use tpt_io_core::ParseError;

use crate::error::MeshError;
use crate::Mesh;

const MAGIC: &[u8; 4] = b"glTF";
const CHUNK_JSON: u32 = 0x4E4F_534A;
const CHUNK_BIN: u32 = 0x004E_4942;
const MODE_TRIANGLES: u64 = 4;
const CT_U8: u64 = 5121;
const CT_U16: u64 = 5123;
const CT_U32: u64 = 5125;
const CT_F32: u64 = 5126;

/// Parse a GLB file and return the triangles of its `TRIANGLES` primitives.
pub fn parse_glb(input: &[u8]) -> Result<Mesh, MeshError> {
    if input.get(0..4) != Some(MAGIC.as_slice()) {
        return Err(invalid("not a GLB file (missing glTF magic)"));
    }
    let version = le_u32(input, 4)?;
    if version != 2 {
        return Err(invalid("GLB version must be 2"));
    }
    let total = le_u32(input, 8)? as usize;
    if total > input.len() {
        return Err(ParseError::eof(input.len()).into());
    }
    let mut json = None;
    let mut bin: &[u8] = &[];
    let mut pos = 12;
    while pos + 8 <= total {
        let len = le_u32(input, pos)? as usize;
        let kind = le_u32(input, pos + 4)?;
        let data = input.get(pos + 8..pos + 8 + len).ok_or_else(|| ParseError::eof(pos))?;
        match kind {
            CHUNK_JSON => json = Some(data),
            CHUNK_BIN => bin = data,
            _ => {}
        }
        pos += 8 + len;
    }
    let json = parse_json(json.ok_or_else(|| invalid("GLB has no JSON chunk"))?)?;
    read_meshes(&json, bin)
}

/// Write one mesh as a GLB file: a single node holding one `TRIANGLES` primitive, with `POSITION` as a VEC3 float
/// accessor and the indices as a 32-bit scalar accessor.
pub fn write_glb(mesh: &Mesh) -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    for v in &mesh.vertices {
        for c in v {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    let index_offset = bin.len();
    for t in &mesh.triangles {
        for i in t {
            bin.extend_from_slice(&i.to_le_bytes());
        }
    }
    let data_len = bin.len();
    let index_bytes = data_len - index_offset;
    let vertex_count = mesh.vertices.len();
    let index_count = mesh.triangles.len() * 3;
    let (min, max) = bounds(&mesh.vertices);
    let json = alloc::format!(
        "{{\"asset\":{{\"version\":\"2.0\",\"generator\":\"tpt-io-mesh\"}},\
\"scene\":0,\"scenes\":[{{\"nodes\":[0]}}],\"nodes\":[{{\"mesh\":0}}],\
\"meshes\":[{{\"primitives\":[{{\"attributes\":{{\"POSITION\":0}},\"indices\":1,\"mode\":4}}]}}],\
\"buffers\":[{{\"byteLength\":{data_len}}}],\
\"bufferViews\":[{{\"buffer\":0,\"byteOffset\":0,\"byteLength\":{index_offset},\"target\":34962}},\
{{\"buffer\":0,\"byteOffset\":{index_offset},\"byteLength\":{index_bytes},\"target\":34963}}],\
\"accessors\":[{{\"bufferView\":0,\"componentType\":5126,\"count\":{vertex_count},\"type\":\"VEC3\",\
\"min\":[{},{},{}],\"max\":[{},{},{}]}},\
{{\"bufferView\":1,\"componentType\":5125,\"count\":{index_count},\"type\":\"SCALAR\"}}]}}",
        min[0], min[1], min[2], max[0], max[1], max[2]
    );
    let mut json_bytes = json.into_bytes();
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_JSON.to_le_bytes());
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
    out.extend_from_slice(&bin);
    out
}

fn invalid(what: &str) -> MeshError {
    MeshError::Invalid(what.to_string())
}

fn le_u32(b: &[u8], at: usize) -> Result<u32, MeshError> {
    let s = b.get(at..at + 4).ok_or_else(|| ParseError::eof(at))?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// A non-negative integer member, or `None` when absent or not an integer.
fn uint(v: &Json, key: &str) -> Option<u64> {
    let n = v.get(key)?.as_f64()?;
    if n >= 0.0 && n.fract() == 0.0 && n <= u32::MAX as f64 {
        Some(n as u64)
    } else {
        None
    }
}

/// The elements of an array member, or an empty slice when absent.
fn items<'a>(v: &'a Json, key: &str) -> &'a [Json] {
    match v.get(key).and_then(Json::as_array) {
        Some(a) => a,
        None => &[],
    }
}

fn component_size(ctype: u64) -> Result<usize, MeshError> {
    match ctype {
        CT_U8 => Ok(1),
        CT_U16 => Ok(2),
        CT_U32 | CT_F32 => Ok(4),
        _ => Err(invalid("unsupported accessor componentType")),
    }
}

/// An accessor resolved against the binary chunk: where its elements start, how far apart they are, and their type.
struct View<'a> {
    bin: &'a [u8],
    base: usize,
    count: usize,
    stride: usize,
    ctype: u64,
    comps: usize,
}

fn view<'a>(root: &Json, bin: &'a [u8], index: usize) -> Result<View<'a>, MeshError> {
    if items(root, "buffers").first().and_then(|b| b.get("uri")).is_some() {
        return Err(invalid("external buffers are not read; only the GLB binary chunk is"));
    }
    let acc = items(root, "accessors")
        .get(index)
        .ok_or_else(|| invalid("accessor index out of range"))?;
    let count = uint(acc, "count").ok_or_else(|| invalid("accessor has no count"))? as usize;
    let ctype =
        uint(acc, "componentType").ok_or_else(|| invalid("accessor has no componentType"))?;
    let comps = match acc.get("type").and_then(Json::as_str) {
        Some("SCALAR") => 1,
        Some("VEC3") => 3,
        _ => return Err(invalid("accessor type must be SCALAR or VEC3")),
    };
    let view_index =
        uint(acc, "bufferView").ok_or_else(|| invalid("accessor has no bufferView"))? as usize;
    let bv = items(root, "bufferViews")
        .get(view_index)
        .ok_or_else(|| invalid("bufferView index out of range"))?;
    if uint(bv, "buffer").unwrap_or(0) != 0 {
        return Err(invalid("only buffer 0 (the GLB binary chunk) is read"));
    }
    let size = component_size(ctype)? * comps;
    let base =
        (uint(bv, "byteOffset").unwrap_or(0) + uint(acc, "byteOffset").unwrap_or(0)) as usize;
    let stride = match uint(bv, "byteStride") {
        Some(s) if s > 0 => s as usize,
        _ => size,
    };
    if count > 0 && base + stride * (count - 1) + size > bin.len() {
        return Err(invalid("accessor reads past the binary chunk"));
    }
    Ok(View { bin, base, count, stride, ctype, comps })
}

fn read_positions(root: &Json, bin: &[u8], index: usize) -> Result<Vec<[f32; 3]>, MeshError> {
    let v = view(root, bin, index)?;
    if v.ctype != CT_F32 || v.comps != 3 {
        return Err(invalid("POSITION must be a VEC3 of 32-bit floats"));
    }
    let mut out = Vec::with_capacity(v.count);
    for i in 0..v.count {
        let at = v.base + i * v.stride;
        out.push([f32_at(v.bin, at), f32_at(v.bin, at + 4), f32_at(v.bin, at + 8)]);
    }
    Ok(out)
}

fn read_indices(root: &Json, bin: &[u8], index: usize) -> Result<Vec<u32>, MeshError> {
    let v = view(root, bin, index)?;
    if v.comps != 1 || !matches!(v.ctype, CT_U8 | CT_U16 | CT_U32) {
        return Err(invalid("indices must be SCALAR unsigned 8, 16 or 32-bit"));
    }
    let mut out = Vec::with_capacity(v.count);
    for i in 0..v.count {
        let at = v.base + i * v.stride;
        out.push(match v.ctype {
            CT_U8 => v.bin[at] as u32,
            CT_U16 => u16::from_le_bytes([v.bin[at], v.bin[at + 1]]) as u32,
            _ => u32_at(v.bin, at),
        });
    }
    Ok(out)
}

fn read_meshes(root: &Json, bin: &[u8]) -> Result<Mesh, MeshError> {
    let mut mesh = Mesh::default();
    for m in items(root, "meshes") {
        for prim in items(m, "primitives") {
            if uint(prim, "mode").unwrap_or(MODE_TRIANGLES) != MODE_TRIANGLES {
                continue;
            }
            let attributes =
                prim.get("attributes").ok_or_else(|| invalid("primitive has no attributes"))?;
            let Some(pos_acc) = uint(attributes, "POSITION") else { continue };
            let positions = read_positions(root, bin, pos_acc as usize)?;
            let n = positions.len();
            let base = mesh.vertices.len() as u32;
            mesh.vertices.extend_from_slice(&positions);
            let indices = match uint(prim, "indices") {
                Some(acc) => read_indices(root, bin, acc as usize)?,
                None => (0..n as u32).collect(),
            };
            if indices.len() % 3 != 0 {
                return Err(invalid("triangle index count is not a multiple of three"));
            }
            for tri in indices.chunks_exact(3) {
                if tri.iter().any(|&i| i as usize >= n) {
                    return Err(invalid("triangle index out of range"));
                }
                mesh.triangles.push([base + tri[0], base + tri[1], base + tri[2]]);
            }
        }
    }
    Ok(mesh)
}

fn bounds(vertices: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [0.0f32; 3];
    let mut max = [0.0f32; 3];
    if let Some(first) = vertices.first() {
        min = *first;
        max = *first;
        for v in vertices {
            for (k, c) in v.iter().enumerate() {
                min[k] = min[k].min(*c);
                max[k] = max[k].max(*c);
            }
        }
    }
    (min, max)
}
