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
    assert_eq!(back.vertices[back.triangles[2][1] as usize], m.vertices[m.triangles[2][1] as usize]);
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
