//! HEC-RAS text geometry files (`.g01`, `.g02`, …): reader for river reaches and cross sections (read only).
//!
//! A HEC-RAS geometry file is a text file of keyword lines. Each `River Reach=river,reach` line starts a
//! reach; `Reach XY=` gives its centreline points; each `Type RM … = 1 ,station,…` line starts a cross
//! section at a river station; and `#Sta/Elev=` gives the count followed by station and elevation pairs.
//! This crate reads those into a [`GeometryFile`].
//!
//! ```
//! let text = b"River Reach=Main,Upper\nReach XY= 2\n0 0\n100 0\nType RM Length L Ch R = 1 ,1000.5,50,50,50\n\
//! #Sta/Elev= 3\n0 100\n50 90\n100 101\n";
//! let g = tpt_io_hecras::parse(text).unwrap();
//! assert_eq!(g.reaches[0].cross_sections[0].river_station, 1000.5);
//! assert_eq!(g.reaches[0].cross_sections[0].points[1], (50.0, 90.0));
//! ```
//!
//! Read only. Bridges, culverts, inline structures, lateral structures and the unsteady-flow and plan files
//! are not read. The fixed-width layout is read as whitespace-separated numbers.
//!
//! Spec access: HEC-RAS text formats are documented by the US Army Corps of Engineers (public domain). No
//! text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod geometry;

pub use error::HecRasError;
pub use geometry::{parse, CrossSection, GeometryFile, Reach};
