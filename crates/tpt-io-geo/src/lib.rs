//! Geospatial parser for the `tpt-io` workspace: GeoJSON (RFC 7946), with
//! Shapefile / GeoTIFF / CRS handling filled in incrementally per `spec.txt`
//! and the workspace `todo.md` checklist.
//!
//! The shared, dependency-free JSON reader lives in `tpt-io-core` and is
//! reused by other JSON-based formats (e.g. FHIR).

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
extern crate alloc;

pub mod error;
pub mod geojson;
pub mod model;

pub use error::GeoError;
pub use geojson::{parse_geojson, GeoJsonReader, GeoJsonValidator};
pub use model::{Bbox, Feature, FeatureCollection, GeoJson, Geometry, Position};
pub use tpt_io_core::json::Json;

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
