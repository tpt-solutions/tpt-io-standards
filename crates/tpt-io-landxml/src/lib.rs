//! LandXML (`.xml`) civil engineering data: reader and writer for units, the project name, TIN
//! surfaces and alignment names.
//!
//! LandXML stores terrain surfaces as a triangulated irregular network (TIN): a list of points with
//! ids, and faces that name three point ids. This crate reads those into [`Surface`] values, keeps
//! the unit system and project name, and writes surfaces back out.
//!
//! ```
//! let xml = br#"<LandXML><Units><Metric linearUnit="meter"/></Units>
//!   <Surfaces><Surface name="Ground"><Definition surfType="TIN">
//!     <Pnts><P id="1">0 0 10</P><P id="2">1 0 11</P><P id="3">0 1 12</P></Pnts>
//!     <Faces><F>1 2 3</F></Faces>
//!   </Definition></Surface></Surfaces></LandXML>"#;
//! let land = tpt_io_landxml::parse(xml).unwrap();
//! assert_eq!(land.surfaces[0].faces[0], [1, 2, 3]);
//! land.validate().unwrap();
//! ```
//!
//! Alignments, cogo points, pipe networks and feature dictionaries are not decoded; alignment names are
//! kept and the rest are ignored.
//!
//! Spec access: LandXML is published by the LandXML consortium under an open licence. No schema text is
//! copied into this crate.

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

pub use error::LandXmlError;
pub use model::{LandXml, Surface};
pub use reader::parse;
pub use writer::write;
