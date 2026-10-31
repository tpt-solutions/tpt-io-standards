//! EPANET input files (`.inp`): reader, writer and network validation.
//!
//! An `.inp` file is a set of `[SECTION]` blocks of whitespace-separated rows, with `;` comments.
//! This crate keeps every section as ordered rows, so any section round-trips, including the
//! SWMM sections that share the same layout. It also gives typed views of the sections that
//! describe a hydraulic network: junctions, reservoirs, tanks, pipes and coordinates.
//!
//! ```
//! let inp = b"[JUNCTIONS]\n;ID  Elev  Demand\nJ1  100  10\n[RESERVOIRS]\nR1  120\n\
//! [PIPES]\nP1  R1  J1  500  300  100  0  Open\n[COORDINATES]\nJ1 1.0 2.0\nR1 0.0 0.0\n[END]\n";
//! let net = tpt_io_epanet::parse(inp).unwrap();
//! assert_eq!(net.pipes().unwrap()[0].length, 500.0);
//! net.validate().unwrap();
//! ```
//!
//! Spec access: the EPANET 2.2 input file format is documented in the EPANET User Manual, which
//! the US EPA publishes in the public domain. No manual text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod inp;

pub use error::EpanetError;
pub use inp::{parse, write, Coordinate, Inp, Junction, Pipe, Reservoir, Row, Section, Tank};
