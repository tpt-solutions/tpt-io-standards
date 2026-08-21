//! Gerber parser for the `tpt-io` workspace.
//!
//! Scaffold status: the error type and module layout follow the `tpt-io-core`
//! pattern. Format-specific parsers are filled in incrementally per `spec.txt`
//! and the workspace `todo.md` checklist.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod error;

pub use error::GerberError;

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
