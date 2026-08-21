//! EDIF 2.0.0 / 4.0.0 parser for the `tpt-io` workspace.
//!
//! [`parser::EdifReader`] parses EDIF documents (an LISP-like,
//! balanced-parenthesis format) into a typed [`model::Edif`]: libraries, cells,
//! views, interface ports, instances, and nets. [`flatten::flatten`] produces a
//! single flat netlist from a hierarchical design via union-find port merging,
//! and [`writer::serialize`] round-trips a typed document back to EDIF text.
//!
//! ```
//! use tpt_io_core::Reader;
//! use tpt_io_edif::parser::EdifReader;
//! use tpt_io_edif::model::Edif;
//!
//! let src = b"(edif top (edifVersion 2 0 0) (edifLevel 0) \
//!   (library lib (cell c (view v (viewType NETLIST) \
//!     (interface (port a (direction IN)) (port z (direction OUT))) \
//!     (contents (net n (joined (portRef a) (portRef z))))))) \
//!   (design d (cellRef c (libraryRef lib)))";
//! let reader = EdifReader::new();
//! let edif: Edif = reader.parse(src).unwrap();
//! assert_eq!(edif.version, (2, 0, 0));
//! let flat = tpt_io_edif::flatten::flatten(&edif).unwrap();
//! assert_eq!(flat.nets.len(), 1);
//! ```
//!
//! Downstream `em-circuit` integration is behind a feature flag as a stub until
//! that crate publishes (see [`integration`]).

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod error;
pub mod flatten;
pub mod model;
pub mod parser;
pub mod token;
pub mod writer;

#[cfg(any(feature = "em-circuit-integration"))]
pub mod integration;

pub use error::EdifError;
pub use flatten::{flatten, flatten_from, FlatNet, FlatNetlist, FlatRef, LeafInstance};
pub use model::{
    Cell, Design, Direction, Edif, Instance, Library, Net, Port, PortRef, Property, View, ViewType,
};
pub use parser::{parse_edif, parse_sexp, EdifReader, EdifValidator, Sexp};
pub use writer::{serialize, EdifWriter};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
