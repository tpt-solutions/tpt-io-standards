//! IHO S-57 electronic navigational chart (ENC) exchange files: reader for the ISO 8211 framing,
//! the dataset parameters, the feature objects with their attributes, and their geometry.
//!
//! ```
//! // An empty file has no data descriptive record, so it is not a chart.
//! let err = tpt_io_s57::parse(b"").unwrap_err();
//! assert!(matches!(err, tpt_io_s57::S57Error::Invalid(_)));
//! ```
//!
//! Spec access: S-57 is published free of charge by the International Hydrographic Organization
//! (IHO Publication S-57, edition 3.1) and ISO/IEC 8211 is an open transfer standard. Only the
//! field layouts and structure rules were used; no text from either document is copied here.
//! Chart data (ENC cells) is distributed under separate licence terms; no cells are included in
//! this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod chart;
pub mod error;
pub mod iso8211;

pub use chart::{parse, Attribute, Dataset, Feature, Geometry, Position, Primitive};
pub use error::S57Error;
