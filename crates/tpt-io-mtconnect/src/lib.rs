//! MTConnect XML documents: device models and observation streams (read only).
//!
//! MTConnect exposes factory equipment through two document kinds. A **devices** document
//! (`MTConnectDevices`) describes each device, its components (axes, controllers, and so on) and
//! their data items. A **streams** document (`MTConnectStreams`) carries the observations: samples,
//! events and conditions, each tied to a data item by `dataItemId`.
//!
//! ```
//! let devices = br#"<MTConnectDevices><Devices>
//!   <Device id="d1" uuid="M-1" name="Mill"><Components><Axes id="a1" name="Axes">
//!     <DataItems><DataItem id="x1" category="SAMPLE" type="POSITION" subType="ACTUAL"/></DataItems>
//!   </Axes></Components></Device></Devices></MTConnectDevices>"#;
//! let model = tpt_io_mtconnect::parse_devices(devices).unwrap();
//! assert_eq!(model.devices[0].name, "Mill");
//! assert!(model.data_item("x1").is_some());
//! ```
//!
//! This crate is read only. Observations keep their value text, and timestamps are not converted.
//!
//! Spec access: MTConnect is published by the MTConnect Institute under its open licence. No
//! standard text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod model;
pub mod reader;

pub use error::MtConnectError;
pub use model::{Component, DataItem, Device, Devices, Observation, Streams};
pub use reader::{parse_devices, parse_streams};
