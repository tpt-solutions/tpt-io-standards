//! SEG-Y seismic data (`.sgy`): reader and writer for headers and traces.
//!
//! A SEG-Y file is a 3200-byte textual header, a 400-byte binary header, and then traces. Each trace has
//! a 240-byte header and a block of samples whose format is set in the binary header. This crate reads
//! all of them, converts the sample formats it knows (IBM 32-bit float, IEEE float, 32-, 16- and 8-bit
//! integers) to `f32`, and writes IEEE float traces back out with their original trace headers.
//!
//! ```
//! let mut file = tpt_io_segy::SegY::default();
//! file.binary.sample_interval_us = 4000;
//! file.binary.format = 5;
//! file.traces.push(tpt_io_segy::Trace { header: [0; 240], samples: vec![1.0, -0.5] });
//! file.binary.samples_per_trace = 2;
//! let bytes = tpt_io_segy::write(&file);
//! let back = tpt_io_segy::parse(&bytes).unwrap();
//! assert_eq!(back.traces[0].samples, vec![1.0, -0.5]);
//! ```
//!
//! Spec access: SEG-Y is published by the Society of Exploration Geophysicists. No standard text is copied
//! into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod segy;

pub use error::SegyError;
pub use segy::{ibm_to_f32, parse, write, BinaryHeader, SegY, Trace};
