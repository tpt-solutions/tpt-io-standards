//! Triangle meshes: readers and writers for STL, Wavefront OBJ and PLY.
//!
//! All three formats describe triangle surfaces. This crate reads them into a common [`Mesh`]: a list of
//! vertices and a list of triangles that index into it. STL has no shared vertices, so each STL triangle
//! brings its own three vertices. OBJ faces with more than three corners are fan-triangulated. PLY is read
//! in ASCII and binary little-endian form, with `vertex` x/y/z properties and `face` index lists.
//!
//! ```
//! let obj = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
//! let mesh = tpt_io_mesh::obj::parse(obj).unwrap();
//! assert_eq!(mesh.triangles, vec![[0, 1, 2]]);
//! ```
//!
//! Normals, colours, textures and materials are not read by this crate, apart from the STL facet normal
//! which is kept on [`Mesh::normals`] when present.
//!
//! Spec access: STL (3D Systems), OBJ (Wavefront), and PLY (Stanford) are published formats with no
//! restrictive licence. No specification text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod obj;
pub mod ply;
pub mod stl;

use alloc::vec::Vec;

pub use error::MeshError;

/// A triangle mesh: vertices, triangles as indices into them, and optional per-triangle normals.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    /// Vertex positions.
    pub vertices: Vec<[f32; 3]>,
    /// Triangles as three vertex indices each.
    pub triangles: Vec<[u32; 3]>,
    /// Per-triangle normals (STL facet normals), one per triangle, or empty when not stored.
    pub normals: Vec<[f32; 3]>,
}
