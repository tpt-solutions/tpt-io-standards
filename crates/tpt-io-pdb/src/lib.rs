//! Protein Data Bank (PDB) files: reader and writer for the fixed-column records.
//!
//! A PDB file is a sequence of 80-column records. This crate reads the `HEADER`, `TITLE`, `ATOM`,
//! `HETATM` and `CONECT` records, and writes them back in the same columns. Coordinates are in
//! Ångström, stored with three decimals as the format specifies.
//!
//! ```
//! let text = b"ATOM      1  N   ALA A   1      11.104   6.134  -6.504  1.00 20.00           N\n\
//! END\n";
//! let pdb = tpt_io_pdb::parse(text).unwrap();
//! assert_eq!(pdb.atoms[0].name, "N");
//! assert_eq!(pdb.atoms[0].position, [11.104, 6.134, -6.504]);
//! ```
//!
//! For mmCIF files, use `tpt-io-cif`, which reads the same STAR syntax used by modern PDB entries.
//!
//! Spec access: the PDB format is documented by the wwPDB. No documentation text is copied into this
//! crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod record;

pub use error::PdbError;
pub use record::{parse, write, Atom, Header, Pdb};
