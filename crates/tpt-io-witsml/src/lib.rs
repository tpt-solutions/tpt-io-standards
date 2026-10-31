//! WITSML well trajectories (XML): reader for trajectory identity and survey stations (read only).
//!
//! WITSML is the oil-and-gas data-exchange standard. A trajectory document describes a wellbore's
//! survey: each `trajectoryStation` gives a measured depth (`md`), an inclination (`incl`) and an azimuth
//! (`azi`), with units. This crate reads the trajectory's name, its well and wellbore references, and its
//! stations.
//!
//! ```
//! let xml = br#"<trajectorys version="1.4.1.1" xmlns="http://www.witsml.org/schemas/1series">
//!   <trajectory uidWell="W1" uidWellbore="B1" uid="T1">
//!     <name>Main trajectory</name>
//!     <trajectoryStation uid="s1">
//!       <md uom="m">0</md><incl uom="dega">0</incl><azi uom="dega">0</azi>
//!     </trajectoryStation>
//!     <trajectoryStation uid="s2">
//!       <md uom="m">100</md><incl uom="dega">12.5</incl><azi uom="dega">45</azi>
//!     </trajectoryStation>
//!   </trajectory>
//! </trajectorys>"#;
//! let t = tpt_io_witsml::parse(xml).unwrap();
//! assert_eq!(t[0].stations[1].incl, 12.5);
//! ```
//!
//! Read only. Corrections, error models, and the WITSML 2.0 and RESQML formats are not read.
//!
//! Spec access: WITSML is published by Energistics under its licence terms. No schema text is copied into
//! this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod trajectory;

pub use error::WitsmlError;
pub use trajectory::{parse, Station, Trajectory};
