//! Gerber (RS-274X / X2 / X3) and Excellon drill-file parser for the `tpt-io`
//! workspace.
//!
//! The crate targets PCB fabrication data: it parses Gerber layer definitions
//! (apertures, macros, graphics operations, X2/X3 attributes, polarity) into a
//! structured [`GerberFile`], and Excellon drill files into an
//! [`ExcellonFile`]. Both expose a [`tpt_io_core::Reader`] implementation,
//! syntax/semantic [`tpt_io_core::Validator`]s, and a [`tpt_io_core::Writer`]
//! for round-tripping Gerber back to text.
//!
//! ```
//! use tpt_io_core::Reader;
//! use tpt_io_gerber::parser::GerberReader;
//! use tpt_io_gerber::model::GerberFile;
//!
//! let src = b"%FSLAX36Y36*%\n%MOIN*%\n%ADD10C,0.5*%\nD10*\nX0Y0D03*\nX100000Y0D01*\nM02*\n";
//! let reader = GerberReader::new();
//! let file: GerberFile = reader.parse(src).unwrap();
//! assert_eq!(file.objects.len(), 2);
//! ```
//!
//! DRC primitives (trace widths, annular rings, design-rule metadata) are
//! available via [`drc::extract_drc`]. Downstream `eng-*` integrations are
//! behind feature flags as stubs until those crates publish (see
//! [`integration`]).

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod aperture;
pub mod attributes;
pub mod coord;
pub mod drc;
pub mod error;
pub mod excellon;
pub mod model;
pub mod parser;
pub mod writer;

#[cfg(any(feature = "eng-pcb-integration", feature = "eng-geometry-integration"))]
pub mod integration;

pub use aperture::{Aperture, ApertureMacro, MacroPrimitive};
pub use attributes::Attributes;
pub use coord::{Axis, CoordinateFormat, Notation, ZeroSuppression};
pub use drc::{extract_drc, DrcReport};
pub use error::GerberError;
pub use excellon::{ExcellonFile, ExcellonReader, ExcellonValidator, Hole, Slot};
pub use model::{GerberFile, GraphicObject, Interpolation, ObjectKind, Polarity, Quadrant, Unit};
pub use parser::{parse_str, GerberReader, GerberValidator};

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
