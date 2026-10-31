//! GDSII stream files (`.gds`): reader and writer for libraries, structures and their elements.
//!
//! A GDSII file is a sequence of records. Each record has a two-byte big-endian length (counting the
//! header), a record type and a data type. Structures hold elements: boundaries (polygons), paths, text
//! and structure references (`SREF`). Real numbers use the base-16 excess-64 format, which this crate
//! converts to and from `f64`.
//!
//! ```
//! let lib = tpt_io_gds::Library {
//!     name: "TOP".into(),
//!     user_units: 1e-3,
//!     db_units: 1e-9,
//!     structures: vec![tpt_io_gds::Structure {
//!         name: "CELL".into(),
//!         elements: vec![tpt_io_gds::Element::Boundary {
//!             layer: 1, datatype: 0,
//!             xy: vec![(0, 0), (100, 0), (100, 50), (0, 0)],
//!         }],
//!     }],
//! };
//! let bytes = tpt_io_gds::write(&lib).unwrap();
//! let back = tpt_io_gds::parse(&bytes).unwrap();
//! assert_eq!(back.structures[0].elements.len(), 1);
//! ```
//!
//! Spec access: GDSII is the Calma stream format, documented publicly by its former owner and widely
//! implemented. No specification text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod stream;

pub use error::GdsError;
pub use stream::{decode_real8, encode_real8, parse, write, Element, Library, Structure};
