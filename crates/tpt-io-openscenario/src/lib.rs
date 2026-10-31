//! ASAM OpenSCENARIO 1.x scenario files (`.xosc`): a structural reader and validator.
//!
//! An OpenSCENARIO file declares parameters, catalog locations and a road network, lists the
//! scenario entities, and describes the behaviour in a storyboard of stories, acts and maneuver
//! groups. This crate reads that structure into an [`OpenScenario`] value. It does not run the
//! scenario, evaluate parameters, or resolve catalog references.
//!
//! ```
//! let xosc = br#"<OpenSCENARIO>
//!   <FileHeader revMajor="1" revMinor="1" date="2024-01-01" description="demo" author="me"/>
//!   <Entities><ScenarioObject name="ego"><Vehicle name="car" vehicleCategory="car"/></ScenarioObject></Entities>
//!   <Storyboard><Story name="s1"><Act name="a1"><ManeuverGroup maximumExecutionCount="1"/></Act></Story></Storyboard>
//! </OpenSCENARIO>"#;
//! let s = tpt_io_openscenario::parse(xosc).unwrap();
//! assert_eq!(s.entities[0].name, "ego");
//! assert_eq!(s.stories[0].acts[0].maneuver_groups, 1);
//! s.validate().unwrap();
//! ```
//!
//! This reader is read-only: there is no writer, so the round-trip checks used by the other
//! crates do not apply here.
//!
//! Spec access: ASAM OpenSCENARIO is an ASAM standard; its licence terms apply to the published
//! document. No specification text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;
pub mod reader;

pub use error::OpenScenarioError;
pub use model::{Act, EntityKind, FileHeader, OpenScenario, Parameter, ScenarioObject, Story};
pub use reader::parse;
