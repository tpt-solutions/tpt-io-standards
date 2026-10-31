//! Two-line element sets (TLE): reader, writer and checksum verification.
//!
//! A TLE describes an Earth-orbiting object's mean orbital elements at an epoch, in two fixed-width
//! 69-character lines with a modulo-10 checksum in column 69. An optional name line may precede
//! them. Files usually hold many TLEs one after another; [`parse`] reads all of them.
//!
//! ```
//! let text = b"ISS (ZARYA)
//! 1 25544U 98067A   24001.50000000  .00016717  00000+0  10270-3 0  9008
//! 2 25544  51.6400 208.9163 0006317  69.9862  25.2906 15.49560000 10009
//! ";
//! let sets = tpt_io_tle::parse(text).unwrap();
//! assert_eq!(sets[0].name.as_deref(), Some("ISS (ZARYA)"));
//! assert_eq!(sets[0].satellite_number, 25544);
//! assert_eq!(sets[0].epoch_year, 2024);
//! ```
//!
//! The writer produces canonical column widths, so values that come from a canonical TLE round
//! trip exactly; other precisions are rounded to the TLE column precision on output.
//!
//! Spec access: the TLE format is documented by NORAD/Celestrak and in the public CelesTrak
//! format notes. No text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod tle;

pub use error::TleError;
pub use tle::{checksum, parse, write, Tle};
