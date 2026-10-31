//! OGC WaterML 2.0 measurement time series: reader and writer.
//!
//! WaterML 2.0 wraps observations in OGC Observations & Measurements (`om:`) and GML (`gml:`)
//! elements. A measurement time series holds a list of time-value pairs (`wml2:MeasurementTVP`),
//! a default unit, and the observed property it measures. This crate reads every
//! `MeasurementTimeseries` in a document and writes a single collection back out.
//!
//! ```
//! let series = tpt_io_waterml::TimeSeries {
//!     id: "ts1".into(),
//!     observed_property: "http://example.org/discharge".into(),
//!     uom: "m3/s".into(),
//!     points: vec![tpt_io_waterml::Point { time: "2024-01-01T00:00:00Z".into(), value: 12.5 }],
//! };
//! let xml = tpt_io_waterml::write(&[series.clone()]);
//! let back = tpt_io_waterml::parse(&xml).unwrap();
//! assert_eq!(back, vec![series]);
//! ```
//!
//! Quality flags, metadata beyond the unit, and the observation's feature of interest are not
//! modelled; they are dropped on read.
//!
//! Spec access: WaterML 2.0 is an OGC standard published under the OGC licence terms. No
//! specification text is copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod error;
pub mod series;

pub use error::WaterMlError;
pub use series::{parse, write, Point, TimeSeries};
