//! Shared foundation for every `tpt-io-*` format parser: the [`Reader`], [`Writer`],
//! [`Validator`], and [`Schema`] traits, a structured [`ParseError`] type with byte-offset
//! and spec-reference reporting, encoding detection, and optional checksum utilities.
//!
//! Core traits and [`ParseError`] are `no_std` compatible (with `alloc`); `std`-only
//! functionality (e.g. reading from [`std::io::Read`]) is gated behind the `std` feature,
//! which is enabled by default.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod checksum;
pub mod encoding;
#[cfg(feature = "alloc")]
pub mod error;
#[cfg(any(feature = "alloc", feature = "std"))]
pub mod json;
pub mod traits;

pub use encoding::Encoding;
#[cfg(feature = "alloc")]
pub use error::{ErrorContext, ErrorKind, ParseError, Result, SpecReference, TokenMismatch};
#[cfg(any(feature = "alloc", feature = "std"))]
pub use json::{parse_json, Json};
#[cfg(any(feature = "alloc", feature = "std"))]
pub use traits::Writer;
#[cfg(feature = "alloc")]
pub use traits::{FormatExtension, Schema};
pub use traits::{NoProgress, Progress, ProgressFn, Reader, ValidationSettings, Validator};
