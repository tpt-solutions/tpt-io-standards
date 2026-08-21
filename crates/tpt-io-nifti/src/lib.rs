//! NIfTI-1 / NIfTI-2 parser for the `tpt-io` workspace.
//!
//! [`parser::NiftiReader`] parses a NIfTI-1 `.nii` byte stream into a
//! [`model::NiftiFile`] (header + raw voxel data). The 348-byte header is
//! decoded in [`header`], exposing dimensions, voxel sizes, the datatype, and
//! the orientation affine (from `sform`/`qform`). [`coord`] provides
//! voxel↔world and RAS↔LPS conversions; [`writer::serialize_header`] round-trips
//! the header.
//!
//! ```
//! use tpt_io_core::Reader;
//! use tpt_io_nifti::header::parse_header;
//! use tpt_io_nifti::coord::{voxel_to_world, ras_to_lps};
//!
//! // Build a minimal NIfTI-1 header describing a 2x2x2 float volume.
//! let mut raw = [0u8; 348];
//! raw[0..4].copy_from_slice(&348i32.to_le_bytes());
//! raw[40..42].copy_from_slice(&3i16.to_le_bytes());
//! for (i, d) in [2u16, 2, 2].iter().enumerate() {
//!     raw[42 + i * 2..44 + i * 2].copy_from_slice(&(*d as i16).to_le_bytes());
//! }
//! raw[70..72].copy_from_slice(&16i16.to_le_bytes()); // float32
//! raw[72..74].copy_from_slice(&32i16.to_le_bytes()); // bitpix
//! raw[108..112].copy_from_slice(&352f32.to_le_bytes()); // vox_offset (past 348 + 4 ext)
//! for i in 1..4 {
//!     raw[76 + i * 4..80 + i * 4].copy_from_slice(&1.0f32.to_le_bytes()); // pixdim
//! }
//! raw[344..348].copy_from_slice(b"n+1\0");
//!
//! let h = parse_header(&raw).unwrap();
//! assert_eq!(h.ndim, 3);
//! assert_eq!(h.voxel_count(), 8);
//! let w = voxel_to_world(&h.affine(), 0.0, 0.0, 0.0);
//! let _lps = ras_to_lps(w);
//! ```
//!
//! Downstream `sci-image` integration is behind a feature flag as a stub until
//! that crate publishes (see [`integration`]).

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
// Multi-index loops over fixed-size header arrays are clearer with explicit indices.
#![allow(clippy::needless_range_loop)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod coord;
pub mod error;
pub mod header;
pub mod integration;
pub mod model;
pub mod parser;
pub mod writer;

pub use error::NiftiError;
pub use header::{DataType, Nifti1Header};
pub use model::NiftiFile;
pub use parser::{parse_hdr_img, parse_nii, NiftiReader, NiftiValidator};
pub use writer::{serialize_header, NiftiWriter};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
