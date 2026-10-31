//! IPC-2581 PCB design data (XML): reader for the revision, units, layers, steps, components and nets.
//!
//! IPC-2581 is a single XML file that carries a printed-circuit design: its revision and content
//! header, the CAD header units, the layers, the steps (boards or panels), the placed components and the
//! logical nets. This crate reads those into an [`Ipc2581`] value. Geometry (profiles, padstacks,
//! features) and the bill of materials are not decoded.
//!
//! ```
//! let xml = br#"<IPC-2581 revision="C"><Content><CadHeader units="MILLIMETER"/></Content>
//!   <Ecad><CadData>
//!     <Layer name="TOP" layerFunction="CONDUCTOR" side="TOP"/>
//!     <Step name="board">
//!       <Component refDes="R1" packageRef="R0603"/>
//!       <LogicalNet name="VCC"/>
//!     </Step>
//!   </CadData></Ecad></IPC-2581>"#;
//! let board = tpt_io_ipc2581::parse(xml).unwrap();
//! assert_eq!(board.revision, "C");
//! assert_eq!(board.components[0].ref_des, "R1");
//! ```
//!
//! Spec access: IPC-2581 is published by IPC under its licence terms. No standard text is copied into this
//! crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod board;

pub use board::{parse, Component, Ipc2581, Layer};
pub use error::Ipc2581Error;
