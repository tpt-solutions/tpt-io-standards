//! gbXML building-energy models (XML): reader for buildings, spaces and surfaces (read only).
//!
//! gbXML describes a building for energy analysis as a campus of buildings, each made of spaces
//! bounded by surfaces. Each surface has a type (wall, roof, slab and so on) and a planar polyloop
//! of Cartesian coordinates. This crate reads those into [`Building`], [`Space`] and [`Surface`]
//! values.
//!
//! ```
//! let xml = br#"<gbXML><Campus><Building id="b1" buildingType="Office"><Name>HQ</Name>
//!   <Space id="s1"><Name>Lobby</Name></Space>
//!   <Surface id="w1" surfaceType="ExteriorWall"><PlanarGeometry><PolyLoop>
//!     <CartesianPoint><Coordinate>0</Coordinate><Coordinate>0</Coordinate><Coordinate>0</Coordinate></CartesianPoint>
//!     <CartesianPoint><Coordinate>5</Coordinate><Coordinate>0</Coordinate><Coordinate>0</Coordinate></CartesianPoint>
//!     <CartesianPoint><Coordinate>5</Coordinate><Coordinate>0</Coordinate><Coordinate>3</Coordinate></CartesianPoint>
//!   </PolyLoop></PlanarGeometry></Surface>
//! </Building></Campus></gbXML>"#;
//! let model = tpt_io_gbxml::parse(xml).unwrap();
//! assert_eq!(model.buildings[0].surfaces[0].points.len(), 3);
//! ```
//!
//! Read only. Construction layers, schedules, and HVAC systems are not decoded.
//!
//! Spec access: gbXML is published by the gbXML consortium under an open licence. No schema text is
//! copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;

pub use error::GbXmlError;
pub use model::{parse, Building, GbXml, Space, Surface};
