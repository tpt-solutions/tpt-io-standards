//! SPICE netlist parser for the `tpt-io` workspace (HSPICE / LTSpice /
//! Ngspice / PSpice dialects).
//!
//! [`parser::parse_str`] and the [`parser::SpiceReader`] type parse element
//! cards (`R`/`L`/`C`/`V`/`I`/`D`/`Q`/`M`/`E`/`F`/`G`/`H`/`T`/`X`/`B`),
//! `.model` definitions, `.subckt`/`.ends` blocks, and analysis commands
//! (`.tran`/`.ac`/`.op`/`.dc`/`.noise`). Component values with SI suffixes
//! (`k`, `u`, `n`, `p`, `meg`, …) are scaled to their numeric form. Validation
//! lives in [`parser::SpiceValidator`].
//!
//! ```no_run
//! use tpt_io_core::Reader;
//! use tpt_io_spice::parser::SpiceReader;
//!
//! let netlist = b"RC filter\nR1 in out 1k\n.end\n";
//! let reader = SpiceReader::new();
//! // let net = reader.parse(netlist).unwrap();
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod error;
pub mod expand;
pub mod model;
pub mod parser;
pub mod writer;

#[cfg(feature = "em-circuit-integration")]
pub mod integration;

pub use error::SpiceError;
pub use expand::{expand_netlist, expand_subckt_instance};
pub use model::{Analysis, Element, ElementKind, Model, SpiceNetlist, Subckt, Value};
pub use parser::{parse_element, parse_str, parse_value, SpiceReader, SpiceValidator};
pub use writer::serialize;

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
