//! AutoCAD DXF (ASCII): reader and writer for layers and the common 2D entities.
//!
//! A DXF file is a stream of group-code and value pairs, one per two lines: a code (an integer such as
//! `0` for an entity name, `10` for an X coordinate, `8` for a layer) and its value. Sections hold the
//! header variables, the layer table and the entities. This crate reads the `$ACADVER` header variable,
//! the layer names, and the entities `LINE`, `CIRCLE`, `LWPOLYLINE` and `TEXT`, and writes them back.
//!
//! ```
//! let dxf = b"0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nWalls\n10\n0\n20\n0\n11\n5\n21\n0\n0\nENDSEC\n0\nEOF\n";
//! let drawing = tpt_io_dxf::parse(dxf).unwrap();
//! assert_eq!(drawing.entities.len(), 1);
//! ```
//!
//! Binary DXF and other entity types are not read: they are reported as [`Entity::Other`] with their
//! layer, so no content is silently dropped in the count.
//!
//! Spec access: the DXF reference is published by Autodesk for interchange. No reference text is
//! copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod drawing;
pub mod error;

pub use drawing::{parse, write, Drawing, Entity};
pub use error::DxfError;
