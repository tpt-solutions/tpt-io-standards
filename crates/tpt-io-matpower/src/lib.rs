//! MATPOWER case files (`case*.m`): reader, writer and structural validation.
//!
//! MATPOWER stores a power-flow case as a MATLAB function that fills a struct `mpc`
//! with numeric matrices: `bus`, `gen` and `branch`, plus optional data such as
//! `gencost`. This crate reads that script form into a typed [`Case`], keeps any other
//! fields in order so they survive a round trip, and writes the case back out.
//!
//! ```
//! let src = b"function mpc = tiny\n\
//! mpc.version = '2';\n\
//! mpc.baseMVA = 100;\n\
//! mpc.bus = [\n\t1\t3\t0\t0\t0\t0\t1\t1\t0\t230\t1\t1.1\t0.9;\n];\n\
//! mpc.gen = [\n\t1\t0\t0\t300\t-300\t1\t100\t1\t250\t10;\n];\n\
//! mpc.branch = [];\n";
//! let case = tpt_io_matpower::parse(src).unwrap();
//! assert_eq!(case.bus[0].id, 1);
//! assert_eq!(case.gen[0].pmax, 250.0);
//! ```
//!
//! Only the numeric and string value forms MATPOWER uses are read: scalars, quoted
//! strings, `[ … ]` matrices and `{ … }` cell arrays of strings. Other MATLAB syntax
//! (function calls, arithmetic, struct arrays) is skipped as an unknown statement and
//! is not preserved.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;
pub mod reader;
pub mod writer;

pub use error::MatpowerError;
pub use model::{Branch, Bus, Case, Gen, Value};
pub use reader::parse;
pub use writer::write;
