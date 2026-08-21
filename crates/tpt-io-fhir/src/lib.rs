//! FHIR parser for the `tpt-io` workspace: JSON resource deserialization
//! (R4/R5), with XML, profile validation, reference resolution, and NZ HIPC
//! hooks filled in incrementally per `spec.txt` and the `todo.md` checklist.
//!
//! The shared, dependency-free JSON reader lives in `tpt-io-core`.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod error;
pub mod model;
pub mod parser;

pub use error::FhirError;
pub use model::{Bundle, BundleEntry, Fhir, Resource};
pub use parser::{parse_fhir, FhirReader, FhirValidator};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
