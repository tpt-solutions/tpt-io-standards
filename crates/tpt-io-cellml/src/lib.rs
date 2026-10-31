//! CellML 1.x models (XML): reader for components, variables, unit definitions and connections.
//!
//! A CellML model is an XML `model` made of `component`s (each with `variable`s and a MathML `math`
//! block), `units` definitions, and `connection`s that map variables between components. This crate reads
//! the model name, its unit definitions, its components with their variables, and its connections. The
//! MathML equations are kept as the number of `math` blocks, not evaluated.
//!
//! ```
//! let xml = br#"<model xmlns="http://www.cellml.org/cellml/1.0#" name="decay">
//!   <units name="per_s"><unit units="second" exponent="-1"/></units>
//!   <component name="cell">
//!     <variable name="x" units="dimensionless" interface="public_interface"/>
//!     <math xmlns="http://www.w3.org/1998/Math/MathML"/>
//!   </component>
//! </model>"#;
//! let m = tpt_io_cellml::parse(xml).unwrap();
//! assert_eq!(m.components[0].variables[0].name, "x");
//! ```
//!
//! Read only. Equations are not evaluated, and CellML 2.0 namespaces are not read.
//!
//! Spec access: CellML is published by the CellML project (open standard). No specification text is copied
//! into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;

pub use error::CellMlError;
pub use model::{parse, CellModel, Component, Connection, UnitDef, Variable};
