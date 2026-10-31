//! NRRD (Nearly Raw Raster Data) files: reader and writer for headers and inline data.
//!
//! An NRRD file is a magic line (`NRRD0004`), a header of `field: value` lines (and `key:=value` pairs),
//! a blank line, then the data. The header gives the type, dimension, sizes and encoding of the array.
//! This crate reads the header fields and the inline data for `raw` and `ascii` encodings, decodes
//! samples for the common scalar types, and writes raw files back out.
//!
//! ```
//! let bytes = b"NRRD0004\ntype: uchar\ndimension: 2\nsizes: 2 2\nencoding: raw\n\n\x01\x02\x03\x04";
//! let img = tpt_io_nrrd::parse(bytes).unwrap();
//! assert_eq!(img.sizes(), Some(vec![2, 2]));
//! assert_eq!(img.values().unwrap(), vec![1.0, 2.0, 3.0, 4.0]);
//! ```
//!
//! External data files (`data file:`) and the `gzip`/`bzip2` encodings are not read: they return
//! [`NrrdError::Unsupported`].
//!
//! Spec access: NRRD is defined by its format specification, published openly by Kindlmann and the
//! Teem project. No text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod nrrd;

pub use error::NrrdError;
pub use nrrd::{parse, write, Nrrd};
