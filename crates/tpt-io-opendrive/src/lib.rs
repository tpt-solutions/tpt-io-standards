//! ASAM OpenDRIVE road networks (`.xodr`): reader, writer and geometry validation.
//!
//! An OpenDRIVE file describes roads as a reference line (the plan view, a chain of geometry
//! segments) and lane sections laid out along it. This crate reads the header, roads, plan-view
//! geometry (line, arc, spiral and cubic polynomial segments) and lane sections with their lanes
//! and widths, and writes the same model back out.
//!
//! ```
//! let xodr = br#"<OpenDRIVE>
//!   <header revMajor="1" revMinor="6" name="tiny"/>
//!   <road name="R1" id="1" length="100" junction="-1">
//!     <planView><geometry s="0" x="0" y="0" hdg="0" length="100"><line/></geometry></planView>
//!     <lanes><laneSection s="0">
//!       <center><lane id="0" type="none" level="false"/></center>
//!       <right><lane id="-1" type="driving" level="false"><width sOffset="0" a="3.5" b="0" c="0" d="0"/></lane></right>
//!     </laneSection></lanes>
//!   </road>
//! </OpenDRIVE>"#;
//! let map = tpt_io_opendrive::parse(xodr).unwrap();
//! assert_eq!(map.roads[0].lane_sections[0].right[0].kind, "driving");
//! map.validate().unwrap();
//! ```
//!
//! Object, signal, junction-connection and elevation/superelevation data are not modelled and are
//! dropped. Geometry segments other than the five listed in [`GeometryKind`] are rejected.
//!
//! Spec access: ASAM OpenDRIVE is an ASAM standard. The licence terms apply to the published
//! document; no specification text is copied into this crate.

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

pub use error::OpenDriveError;
pub use model::{Geometry, GeometryKind, Header, Lane, LaneSection, OpenDrive, Road, Width};
pub use reader::parse;
pub use writer::write;
