//! Integration tests for the STL, OBJ and PLY readers and writers.

use tpt_io_mesh::{obj, ply, stl, Mesh, MeshError};

fn tetra() -> Mesh {
    Mesh {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        triangles: vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
        normals: vec![],
    }
}

#[test]
fn obj_round_trip_and_fan_triangulation() {
    let m = tetra();
    assert_eq!(obj::parse(&obj::write(&m)).unwrap().triangles, m.triangles);
    let quad = obj::parse(b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1/1/1 2 3 4\n").unwrap();
    assert_eq!(quad.triangles, vec![[0, 1, 2], [0, 2, 3]]);
    let negative = obj::parse(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n").unwrap();
    assert_eq!(negative.triangles, vec![[0, 1, 2]]);
}

#[test]
fn obj_missing_vertices_are_errors() {
    assert!(matches!(obj::parse(b"v 0 0 0\nf 1 2 3\n"), Err(MeshError::Invalid(_))));
    assert!(matches!(obj::parse(b"v 0 0 0\nv 0 0 0\nf 1 2\n"), Err(MeshError::Invalid(_))));
}

#[test]
fn ply_ascii_round_trip() {
    let m = tetra();
    let back = ply::parse(&ply::write(&m)).unwrap();
    assert_eq!(back.vertices, m.vertices);
    assert_eq!(back.triangles, m.triangles);
}

#[test]
fn ply_binary_little_endian_is_read() {
    let mut bytes = b"ply\nformat binary_little_endian 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n".to_vec();
    for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        for c in v {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
    }
    bytes.push(3);
    for i in [0i32, 1, 2] {
        bytes.extend_from_slice(&i.to_le_bytes());
    }
    let m = ply::parse(&bytes).unwrap();
    assert_eq!(m.vertices[1], [1.0, 0.0, 0.0]);
    assert_eq!(m.triangles, vec![[0, 1, 2]]);
}

#[test]
fn stl_binary_and_ascii_round_trips() {
    let mut m = tetra();
    m.normals = vec![[0.0, 0.0, -1.0], [0.0, -1.0, 0.0], [-1.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
    let bin = stl::write_binary(&m);
    assert_eq!(bin.len(), 84 + 50 * 4);
    let back = stl::parse(&bin).unwrap();
    assert_eq!(back.triangles.len(), 4);
    assert_eq!(back.normals, m.normals);
    assert_eq!(
        back.vertices[back.triangles[2][1] as usize],
        m.vertices[m.triangles[2][1] as usize]
    );
    let ascii = stl::write_ascii(&m);
    let from_ascii = stl::parse(&ascii).unwrap();
    assert_eq!(from_ascii.triangles.len(), 4);
    assert_eq!(from_ascii.normals, m.normals);
}

#[test]
fn ply_ascii_face_out_of_range_is_not_checked_but_short_faces_are_errors() {
    assert!(ply::parse(b"ply\nformat ascii 1.0\nelement vertex 0\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n2 0 1\n").is_err());
    assert!(ply::parse(b"ply\nformat ascii 1.0\n").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 191;
    for _ in 0..300 {
        let len = (x % 300) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"v f solid facet vertex ply end_header 0123456789.- \n";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = stl::parse(&bytes);
        let _ = obj::parse(&bytes);
        let _ = ply::parse(&bytes);
    }
}

#[test]
fn glb_round_trip_keeps_triangles_and_vertices() {
    let m = tetra();
    let back = tpt_io_mesh::gltf::parse_glb(&tpt_io_mesh::gltf::write_glb(&m)).unwrap();
    assert_eq!(back.vertices, m.vertices);
    assert_eq!(back.triangles, m.triangles);
}

/// Build a GLB file from a JSON chunk and a binary chunk, padding both to four bytes.
fn glb(json: &str, bin: &[u8]) -> Vec<u8> {
    let mut json = json.as_bytes().to_vec();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let mut bin = bin.to_vec();
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = b"glTF".to_vec();
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4E4F_534Au32.to_le_bytes());
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004E_4942u32.to_le_bytes());
    out.extend_from_slice(&bin);
    out
}

#[test]
fn glb_reads_u16_indices_with_a_byte_stride_and_skips_lines() {
    let json = r#"{"asset":{"version":"2.0"},
        "meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1}]},
                  {"primitives":[{"attributes":{"POSITION":0},"mode":1}]}],
        "buffers":[{"byteLength":42}],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36,"byteStride":12},
                       {"buffer":0,"byteOffset":36,"byteLength":6}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},
                     {"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}]}"#;
    let mut bin = Vec::new();
    for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        for c in v {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    for i in [0u16, 1, 2] {
        bin.extend_from_slice(&i.to_le_bytes());
    }
    let m = tpt_io_mesh::gltf::parse_glb(&glb(json, &bin)).unwrap();
    assert_eq!(m.vertices.len(), 3);
    assert_eq!(m.triangles, vec![[0, 1, 2]]);
}

#[test]
fn glb_rejects_bad_magic_and_out_of_range_indices() {
    assert!(matches!(
        tpt_io_mesh::gltf::parse_glb(b"not a glb file at all"),
        Err(MeshError::Invalid(_))
    ));

    let json = r#"{"meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1}]}],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":12},{"buffer":0,"byteOffset":12,"byteLength":12}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":1,"type":"VEC3"},
                     {"bufferView":1,"componentType":5125,"count":3,"type":"SCALAR"}]}"#;
    let mut bin = vec![0u8; 12];
    for i in [0u32, 1, 2] {
        bin.extend_from_slice(&i.to_le_bytes());
    }
    assert!(matches!(tpt_io_mesh::gltf::parse_glb(&glb(json, &bin)), Err(MeshError::Invalid(_))));
}
