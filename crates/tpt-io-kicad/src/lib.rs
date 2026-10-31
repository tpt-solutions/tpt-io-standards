//! KiCad PCB and schematic reader for the `tpt-io` workspace.
//!
//! KiCad 6 and later store `.kicad_pcb` layouts and `.kicad_sch` schematics as
//! S-expressions. This crate tokenizes them with [`tpt_io_core::sexpr`], builds a
//! generic tree, and extracts the parts most tools need: the format version, the
//! generator, nets, footprints, and placed schematic symbols.
//!
//! ```
//! let src = br#"(kicad_pcb (version 20211014) (generator pcbnew)
//!   (net 0 "") (net 1 "GND")
//!   (footprint "Resistor_SMD:R_0603" (layer "F.Cu") (at 10 20 90)
//!     (property "Reference" "R1") (property "Value" "10k")))"#;
//! let doc = tpt_io_kicad::parse(src).unwrap();
//! assert_eq!(doc.nets[1].name, "GND");
//! assert_eq!(doc.footprints[0].reference.as_deref(), Some("R1"));
//! ```
//!
//! KiCad 5 PCB `module` footprints are read too, as are PCB zones (outline and filled areas) and
//! schematic labels and junctions. Not yet read: project files. See the crate README.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;
pub mod tree;

pub use error::KicadError;
pub use model::{parse, DocKind, Footprint, KicadDocument, Net, Symbol};
pub use tree::{parse_tree, Node};
