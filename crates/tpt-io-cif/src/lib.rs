//! CIF 1.1 and mmCIF (STAR-format) files: reader and writer.
//!
//! A CIF file is a sequence of `data_` blocks. Each block holds `_tag value` items and `loop_`
//! tables, whose values are tokens, quoted strings or semicolon-delimited text fields. mmCIF
//! (the macromolecular dialect used by the PDB) uses the same syntax, so this crate reads it too.
//!
//! ```
//! let cif = b"data_quartz
//! _cell_length_a 4.9134
//! loop_
//! _atom_site_label
//! _atom_site_fract_x
//! Si1 0.4697
//! O1  0.4135
//! ";
//! let file = tpt_io_cif::parse(cif).unwrap();
//! let block = &file.blocks[0];
//! assert_eq!(block.item("_cell_length_a"), Some("4.9134"));
//! assert_eq!(block.loops[0].rows[1][1], "0.4135");
//! ```
//!
//! Values keep their source form: quotes are kept on quoted strings, and text fields are stored
//! without their delimiters. Save frames, global blocks and the CIF 2.0 extensions are not read.
//!
//! Spec access: CIF 1.1 is published by the IUCr; the mmCIF dictionary by the wwPDB. No text from
//! either is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod star;

pub use error::CifError;
pub use star::{parse, write, CifFile, DataBlock, Item, Loop};
