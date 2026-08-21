//! IFC parser for the `tpt-io` workspace: STEP-Physical File (SPF, ISO 10303-21)
//! parsing, with entity-instance resolution, relationship graphs, property sets,
//! geometry extraction, and schema-version detection filled in incrementally per
//! `spec.txt` and the `todo.md` checklist.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod error;
pub mod model;
pub mod parser;

pub use error::IfcError;
pub use model::{EntityInstance, IfcDocument, Value};
pub use parser::{parse_ifc, IfcReader, IfcValidator};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
