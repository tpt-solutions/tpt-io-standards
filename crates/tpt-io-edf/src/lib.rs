//! EDF, EDF+ and BDF recordings: reader and writer for headers, signals and samples.
//!
//! An EDF file is a 256-byte header, then 256 bytes of header per signal, then data records of
//! 16-bit little-endian samples (BDF uses 24-bit samples, and its first byte is `0xFF`). Each signal has
//! digital limits and physical limits; samples scale linearly between them. This crate reads the header,
//! the signal definitions and the raw samples, converts samples to physical values on request, and writes
//! EDF (16-bit) back out.
//!
//! ```
//! let mut file = tpt_io_edf::Edf::default();
//! file.signals.push(tpt_io_edf::Signal {
//!     label: "EEG Fpz".into(), physical_dim: "uV".into(),
//!     physical_min: -100.0, physical_max: 100.0, digital_min: -32768, digital_max: 32767,
//!     samples_per_record: 2, ..Default::default()
//! });
//! file.records = 1;
//! file.samples = vec![vec![0, 16384]];
//! let bytes = tpt_io_edf::write(&file).unwrap();
//! let back = tpt_io_edf::parse(&bytes).unwrap();
//! assert_eq!(back.samples, file.samples);
//! assert!((back.physical(0)[1] - 50.0).abs() < 0.01);
//! ```
//!
//! BDF files are read with 24-bit samples; writing is EDF only. EDF+ annotations (TAL signals) are kept as
//! ordinary signals and are not decoded.
//!
//! Spec access: EDF is defined by Kemp et al. (1992) and BDF by BioSemi; both are published openly. No text
//! is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod edf;
pub mod error;

pub use edf::{parse, write, Edf, Signal};
pub use error::EdfError;
