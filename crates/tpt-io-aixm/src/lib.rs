//! AIXM 5.1 aeronautical information (XML): reader for airports, navaids, airspaces and routes.
//!
//! AIXM messages list features (airports and heliports, navigation aids, airspaces, routes) as
//! members of a `hasMember` collection. Each feature has a time slice with its name and designator,
//! and geometry given as GML positions. This crate reads the features it finds into [`Feature`]
//! values with their kind, identifier, name, designator and positions.
//!
//! ```
//! let xml = br#"<AIXMBasicMessage xmlns:aixm="http://www.aixm.aero/schema/5.1">
//!   <hasMember><aixm:Navaid gml:id="NAV1" xmlns:gml="http://www.opengis.net/gml/3.2">
//!     <aixm:timeSlice><aixm:NavaidTimeSlice>
//!       <aixm:designator>ABC</aixm:designator><aixm:name>Alpha</aixm:name>
//!       <aixm:location><aixm:ElevatedPoint><gml:pos>-41.3 174.8</gml:pos></aixm:ElevatedPoint></aixm:location>
//!     </aixm:NavaidTimeSlice></aixm:timeSlice>
//!   </aixm:Navaid></hasMember>
//! </AIXMBasicMessage>"#;
//! let features = tpt_io_aixm::parse(xml).unwrap();
//! assert_eq!(features[0].designator.as_deref(), Some("ABC"));
//! assert_eq!(features[0].positions, vec![(-41.3, 174.8)]);
//! ```
//!
//! Read only. Only the first time slice of a feature is read; service, restriction and
//! timetable data are not decoded.
//!
//! Spec access: AIXM is published by EUROCONTROL and the FAA under their open terms. No schema text
//! is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod feature;

pub use error::AixmError;
pub use feature::{parse, Feature, FeatureKind};
