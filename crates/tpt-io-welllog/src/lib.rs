//! WellLog parser for the `tpt-io` workspace — LAS 2.0/3.0 ASCII (plus DLIS
//! stubs pending).
//!
//! The LAS ASCII grammar (`~V`/`~W`/`~O`/`~C`/`~A` sections) is fully parsed by
//! [`parser::parse_str`] and the [`parser::LasReader`] type, which implements
//! [`tpt_io_core::Reader`] (and [`tpt_io_core::Writer`] for round-tripping).
//! Syntactic/semantic validation lives in [`parser::WellLogValidator`].
//!
//! ```no_run
//! use tpt_io_core::Reader;
//! use tpt_io_welllog::parser::LasReader;
//!
//! let bytes = b"~Version Information\nVERS. 2.0:\nWRAP. NO:\n";
//! let reader = LasReader::new();
//! // let log = reader.parse(bytes).unwrap();
//! ```
//!
//! DLIS (binary, frame-based) support is tracked in `todo.md`.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod error;
pub mod model;
pub mod parser;

pub use error::WellLogError;
pub use model::{Curve, Parameter, WellInfo, WellLog};
pub use parser::{LasReader, WellLogValidator};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
