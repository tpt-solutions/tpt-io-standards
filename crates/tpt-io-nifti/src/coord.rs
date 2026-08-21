//! Coordinate-system helpers: voxel ↔ world, RAS ↔ LPS.
//!
//! NIfTI affine matrices map voxel indices `(i, j, k)` to world coordinates in
//! the RAS convention (Right, Anterior, Superior). Many neuroimaging/geometry
//! toolkits use LPS (Left, Posterior, Superior); conversion is a simple sign
//! flip on the first two world axes.

/// A 3D point.
pub type Point3 = [f32; 3];

/// Apply a 4x4 affine (row-major) to a voxel index, returning world RAS coords.
pub fn voxel_to_world(affine: &[[f32; 4]; 4], i: f32, j: f32, k: f32) -> Point3 {
    [
        affine[0][0] * i + affine[0][1] * j + affine[0][2] * k + affine[0][3],
        affine[1][0] * i + affine[1][1] * j + affine[1][2] * k + affine[1][3],
        affine[2][0] * i + affine[2][1] * j + affine[2][2] * k + affine[2][3],
    ]
}

/// Invert [`voxel_to_world`] (ignoring the homogeneous 4th row, assumed
/// `[0,0,0,1]`). Returns `None` if the 3x3 linear part is singular.
pub fn world_to_voxel(affine: &[[f32; 4]; 4], x: f32, y: f32, z: f32) -> Option<Point3> {
    let m = [
        [affine[0][0], affine[0][1], affine[0][2]],
        [affine[1][0], affine[1][1], affine[1][2]],
        [affine[2][0], affine[2][1], affine[2][2]],
    ];
    let (inv, det) = mat3_inv(m);
    if det.abs() < 1e-12 {
        return None;
    }
    let dx = x - affine[0][3];
    let dy = y - affine[1][3];
    let dz = z - affine[2][3];
    Some([
        inv[0][0] * dx + inv[0][1] * dy + inv[0][2] * dz,
        inv[1][0] * dx + inv[1][1] * dy + inv[1][2] * dz,
        inv[2][0] * dx + inv[2][1] * dy + inv[2][2] * dz,
    ])
}

/// Convert a world point from RAS to LPS convention.
pub fn ras_to_lps(p: Point3) -> Point3 {
    [-p[0], -p[1], p[2]]
}

/// Convert a world point from LPS to RAS convention.
pub fn lps_to_ras(p: Point3) -> Point3 {
    [-p[0], -p[1], p[2]]
}

/// Invert a 3x3 matrix, returning `(inverse, determinant)`.
fn mat3_inv(m: [[f32; 3]; 3]) -> ([[f32; 3]; 3], f32) {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-12 {
        return ([[0.0; 3]; 3], det);
    }
    let inv_det = 1.0 / det;
    let inv = [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
        ],
    ];
    (inv, det)
}
