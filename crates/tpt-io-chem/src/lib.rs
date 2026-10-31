//! MDL molfiles (V2000) and SDF files: reader and writer.
//!
//! A molfile has a three-line header (name, program line, comment), a counts line, an atom block and a
//! bond block at fixed columns, and ends with `M  END`. An SDF file is a series of molfiles, each
//! followed by data items (`> <NAME>` with a value line) and a `$$$$` separator. This crate reads
//! and writes the V2000 form.
//!
//! ```
//! use tpt_io_chem::{Bond, MolAtom, Molecule};
//! let water = Molecule {
//!     name: "water".into(),
//!     atoms: vec![
//!         MolAtom { symbol: "O".into(), position: [0.0, 0.0, 0.1173] },
//!         MolAtom { symbol: "H".into(), position: [0.0, 0.7572, -0.4692] },
//!         MolAtom { symbol: "H".into(), position: [0.0, -0.7572, -0.4692] },
//!     ],
//!     bonds: vec![Bond { first: 1, second: 2, order: 1, stereo: 0 }, Bond { first: 1, second: 3, order: 1, stereo: 0 }],
//!     ..Molecule::default()
//! };
//! let sdf = tpt_io_chem::write_sdf(&[water]);
//! let records = tpt_io_chem::parse_sdf(&sdf).unwrap();
//! assert_eq!(records[0].atoms[0].symbol, "O");
//! assert_eq!(records[0].bonds.len(), 2);
//! ```
//!
//! Only the V2000 atom and bond blocks are read. Charges, isotopes, stereo flags beyond the bond stereo
//! column, and the V3000 format are not decoded.
//!
//! Spec access: the MDL molfile and SDF formats are published by Dassault Systèmes BIOVIA. No text is
//! copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod molfile;

pub use error::ChemError;
pub use molfile::{parse_mol, parse_sdf, write_mol, write_sdf, Bond, MolAtom, Molecule};
