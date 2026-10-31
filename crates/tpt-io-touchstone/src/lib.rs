//! Touchstone network-parameter files (`.s1p`, `.s2p`, …): reader and writer.
//!
//! A Touchstone file has an option line (`# GHz S MA R 50`) that sets the frequency unit, the
//! parameter type (`S`, `Y`, `Z`, `H` or `G`), the data format (`MA`, `DB` or `RI`) and the
//! reference impedance. Each data row is a frequency followed by an N×N matrix of complex values.
//! This crate converts every format to real and imaginary parts on read, so the matrices can be
//! compared directly, and writes the RI form back out.
//!
//! ```
//! let s2p = b"# GHz S DB R 50\n1.0 -3.0 -90.0 10.0 0.0 10.0 0.0 -3.0 -90.0\n";
//! let file = tpt_io_touchstone::parse(s2p, 2).unwrap();
//! assert_eq!(file.points[0].freq_hz, 1.0e9);
//! // S21 is the second value on the line: 10 dB at 0 degrees, i.e. a magnitude of 10^(10/20).
//! let s21 = file.points[0].matrix[1][0];
//! assert!((s21.re - 10f64.powf(0.5)).abs() < 1e-12);
//! ```
//!
//! The port count is not stored in the file, so it is supplied to [`parse`] (from the `.sNp`
//! extension). The two-port column order (S11 S21 S12 S22) is handled as the format specifies.
//!
//! Spec access: Touchstone is the IBIS Open Forum's file format, published openly. No text is copied
//! into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod touchstone;

pub use error::TouchstoneError;
pub use touchstone::{parse, write, Complex, Format, Param, Point, Touchstone};
