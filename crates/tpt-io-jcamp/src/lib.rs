//! JCAMP-DX spectroscopy files: reader and writer for labelled headers and XY data.
//!
//! JCAMP-DX stores spectra as `##LABEL=value` records. The data is in `##XYDATA=(X++(Y..Y))`
//! blocks, where each line starts with an X value followed by Y values at successive X steps, or in
//! `##XYPOINTS=(XY..XY)` blocks of explicit pairs. This crate reads both forms, applies the
//! `##XFACTOR` and `##YFACTOR` scale factors, and writes the explicit `XYPOINTS` form.
//!
//! ```
//! let text = b"##TITLE=demo\n##JCAMP-DX=5.00\n##XYPOINTS=(XY..XY)\n4000, 0.5\n3999, 0.25\n##END=\n";
//! let spectrum = tpt_io_jcamp::parse(text).unwrap();
//! assert_eq!(spectrum.title(), Some("demo"));
//! assert_eq!(spectrum.points[1], (3999.0, 0.25));
//! ```
//!
//! Compressed encodings (`SQZ`, `DIF`, `DUP`) are not decoded and are reported as unsupported.
//!
//! Spec access: JCAMP-DX is published by IUPAC. No text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod spectrum;

pub use error::JcampError;
pub use spectrum::{parse, write, Spectrum};
