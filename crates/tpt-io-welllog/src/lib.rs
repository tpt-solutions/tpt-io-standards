//! WellLog parser for the `tpt-io` workspace — LAS 2.0/3.0 (ASCII **and**
//! binary) and DLIS (RP66 V1, frame-based).
//!
//! # LAS
//!
//! The LAS ASCII grammar (`~V`/`~W`/`~O`/`~C`/`~A` sections) is fully parsed by
//! [`parser::parse_str`] and the [`parser::LasReader`] type, which implements
//! [`tpt_io_core::Reader`] (and [`tpt_io_core::Writer`] for round-tripping).
//! Syntactic/semantic validation lives in [`parser::WellLogValidator`].
//!
//! LAS 3.0 also permits the `~A` section to carry raw binary records; these are
//! handled transparently by [`binary::parse`], which additionally performs
//! automatic text-encoding detection (transcoding UTF-16 inputs).
//!
//! ```
//! use tpt_io_core::Reader;
//! use tpt_io_welllog::parser::LasReader;
//!
//! let bytes = b"~Version Information\nVERS. 2.0 :\nWRAP. NO :\n~Well Information\nWELL. DEMO :\n~Curve Information\nDEPT. M :\nGR . API :\n~ASCII Log Data\n10.0 20.0\n20.0 21.0\n";
//! let log = LasReader::new().parse(bytes).unwrap();
//! assert_eq!(log.curves.len(), 2);
//! ```
//!
//! # DLIS
//!
//! [`dlis::parse`] reads the RP66 V1 container structure and surfaces the
//! frame-based organization ([`dlis::Frame`]/[`dlis::Channel`]) plus decoded
//! frame samples.
//!
//! # Multi-run correlation
//!
//! [`align`] provides depth-grid resampling and cross-correlation for lining up
//! multiple logging runs.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod align;
pub mod binary;
pub mod dlis;
pub mod error;
pub mod integration;
pub mod model;
pub mod parser;

pub use align::{align_runs, common_grid, cross_correlate, resample, AlignedLogs};
pub use error::WellLogError;
pub use model::{Curve, Parameter, WellInfo, WellLog};
pub use parser::{LasReader, WellLogValidator};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
