//! AUTOSAR XML (`.arxml`) reader: the package hierarchy and the elements each package holds.
//!
//! An ARXML file is an `<AUTOSAR>` document whose `<AR-PACKAGES>` contain nested `<AR-PACKAGE>`
//! elements. Each package has a `<SHORT-NAME>`, and its `<ELEMENTS>` hold the model elements
//! (software components, interfaces, ECU configuration and so on), each with its own short name.
//! This crate reads that hierarchy into [`ArxmlFile`], giving every package its absolute path and
//! every element its kind (the XML tag) and short name. It does not decode the element contents.
//!
//! ```
//! let arxml = br#"<AUTOSAR xmlns="http://autosar.org/schema/r4.0">
//!   <AR-PACKAGES><AR-PACKAGE><SHORT-NAME>Components</SHORT-NAME>
//!     <ELEMENTS><APPLICATION-SW-COMPONENT-TYPE><SHORT-NAME>Sensor</SHORT-NAME></APPLICATION-SW-COMPONENT-TYPE></ELEMENTS>
//!   </AR-PACKAGE></AR-PACKAGES>
//! </AUTOSAR>"#;
//! let file = tpt_io_arxml::parse(arxml).unwrap();
//! assert_eq!(file.packages[0].path, "/Components");
//! assert_eq!(file.packages[0].elements[0].short_name, "Sensor");
//! file.validate().unwrap();
//! ```
//!
//! Read-only: there is no writer. Element contents, references and ECUC parameter values are not
//! interpreted.
//!
//! Spec access: AUTOSAR publishes the ARXML schema under its own licence terms. No schema text is
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

pub use error::ArxmlError;
pub use model::{ArxmlElement, ArxmlFile, Package};
pub use reader::parse;
