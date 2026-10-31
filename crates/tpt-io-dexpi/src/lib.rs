//! DEXPI P&ID exchange files (XML): reader for the components of a process and instrumentation diagram.
//!
//! DEXPI describes a plant's P&ID as XML objects: equipment, piping components, process instrumentation
//! functions and nozzles, each with an `ID` and a `ComponentClass`. This crate reads every object that
//! carries an `ID` and a `ComponentClass` into a [`Component`] list, with its tag name and nozzle
//! references. Geometry and the full DEXPI class hierarchy are not decoded.
//!
//! ```
//! let xml = br#"<PlantInformation>
//!   <Equipment ID="E1" ComponentClass="CentrifugalPump" ComponentName="P-101"/>
//!   <PipingNetworkSegment ID="S1" ComponentClass="PipingNetworkSegment"/>
//! </PlantInformation>"#;
//! let plant = tpt_io_dexpi::parse(xml).unwrap();
//! assert_eq!(plant.components[0].class, "CentrifugalPump");
//! ```
//!
//! Spec access: DEXPI is published by the DEXPI consortium under its licence terms. The element and
//! attribute names used here follow the published DEXPI model; no schema text is copied into this crate.
//! Only the names exercised by the test documents are verified.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod plant;

pub use error::DexpiError;
pub use plant::{parse, Component, Plant};
