//! NetCDF-3 (classic / 64-bit) parser for the `tpt-io` workspace.
//!
//! [`parser::NetCdfReader`] parses a NetCDF-3 byte stream into a typed
//! [`model::NetCdf`]: dimensions, global and per-variable attributes, and variable
//! definitions with their data-section offsets. [`header`] decodes the tagged
//! header; [`writer::serialize_header`] round-trips it. Data values are read back
//! from the raw file bytes via the helper methods on [`NetCdf`].
//!
//! ```
//! use tpt_io_core::Reader;
//! use tpt_io_netcdf::header::parse;
//! use tpt_io_netcdf::model::NetCdf;
//!
//! // Build a minimal NetCDF-3 classic header: one dimension `x` of length 4,
//! // a variable `t` over `x` of type float, and one global attribute.
//! let mut out = Vec::new();
//! out.extend_from_slice(b"CDF"); // magic
//! out.push(1);                   // version 1 (classic)
//! out.extend_from_slice(&1u32.to_be_bytes()); // numrecs
//! out.extend_from_slice(&10i32.to_be_bytes()); // TAG_DIMENSION
//! out.extend_from_slice(&1u32.to_be_bytes());  // 1 dimension
//! out.extend_from_slice(&1u32.to_be_bytes());  // name length "x"
//! out.extend_from_slice(b"x\0\0\0");          // "x" + pad to 4
//! out.extend_from_slice(&4u32.to_be_bytes()); // dim length
//! out.extend_from_slice(&12i32.to_be_bytes()); // TAG_ATTRIBUTE
//! out.extend_from_slice(&0u32.to_be_bytes());  // 0 global attrs
//! out.extend_from_slice(&11i32.to_be_bytes()); // TAG_VARIABLE
//! out.extend_from_slice(&1u32.to_be_bytes());  // 1 variable
//! out.extend_from_slice(&1u32.to_be_bytes());  // name length "t"
//! out.extend_from_slice(b"t\0\0\0");          // "t" + pad to 4
//! out.extend_from_slice(&5i32.to_be_bytes());  // NC_FLOAT
//! out.extend_from_slice(&1u32.to_be_bytes());  // 1 dim
//! out.extend_from_slice(&0u32.to_be_bytes());  // dim id 0
//! out.extend_from_slice(&0u32.to_be_bytes());  // 0 attrs
//! out.extend_from_slice(&16u32.to_be_bytes()); // vlen
//! out.extend_from_slice(&0u32.to_be_bytes());  // begin
//!
//! let nc: NetCdf = parse(&out).unwrap();
//! assert_eq!(nc.dims[0].name, "x");
//! assert_eq!(nc.dims[0].len, 4);
//! assert_eq!(nc.vars[0].name, "t");
//! ```
//!
//! Downstream `sci-grid` integration is behind a feature flag as a stub until that
//! crate publishes (see [`integration`]).

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod error;
pub mod header;
pub mod integration;
pub mod model;
pub mod netcdf4;
pub mod parser;
pub mod writer;

pub use error::NetCdfError;
pub use header::{Attr, Dim, NcType, Var};
pub use model::NetCdf;
pub use netcdf4::{detect_format, parse_hdf5_superblock, FileKind, Hdf5Superblock};
pub use parser::{parse_netcdf, NetCdfReader, NetCdfValidator};
pub use writer::{serialize_header, NetCdfWriter};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
