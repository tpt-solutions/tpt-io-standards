//! Geospatial parser for the `tpt-io` workspace.
//!
//! `tpt-io-geo` parses the three core Earth-science vector formats and
//! provides coordinate-system utilities on top:
//!
//! * **GeoJSON** (RFC 7946) — [`geojson`]
//! * **Esri Shapefile** (`.shp` / `.shx` / `.dbf`) — [`shapefile`]
//! * **GeoTIFF** metadata (georeferencing, not pixel samples) — [`geotiff`]
//! * **CRS handling** — PROJ strings, EPSG codes, datum transformation
//!   (WGS84 / NAD83 / NZGD2000), and reprojection — [`crs`]
//! * **Spatial indexing** — an R-tree for range queries, nearest-neighbour search,
//!   and point-in-polygon candidate selection — [`spatial_index`]
//! * **Streaming** parsers for large datasets — [`stream`]
//!
//! All parsers are implemented from scratch in pure Rust with no C/C++ FFI.
//!
//! ```
//! use tpt_io_core::Reader;
//! use tpt_io_geo::geojson::{GeoJsonReader, FeatureCollection};
//!
//! let src = br#"{"type":"FeatureCollection","features":[
//!   {"type":"Feature","geometry":{"type":"Point","coordinates":[174.0,-41.0]},"properties":{"name":"Wellington"}}
//! ]}"#;
//! let fc: FeatureCollection = GeoJsonReader::new().parse(src).unwrap();
//! assert_eq!(fc.features.len(), 1);
//! ```
//!
//! Downstream `tpt-eng-geometry` / `tpt-math-geometry` integrations are behind
//! feature flags as stubs until those crates publish (see [`integration`]).

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-geo requires the `alloc` or `std` feature: its data model owns structured parse data."
);

pub mod crs;
pub mod error;
pub mod geojson;
pub mod geotiff;
pub mod json;
pub mod shapefile;
pub mod spatial_index;
pub mod stream;
pub mod gpx;
pub mod kml;
pub mod wkb;
pub mod wkt;

#[cfg(any(feature = "eng-geometry-integration", feature = "math-geometry-integration"))]
pub mod integration;

pub use error::GeoError;

#[cfg(any(feature = "alloc", feature = "std"))]
pub use tpt_io_core::ParseError;
pub use tpt_io_core::{
    Encoding, FormatExtension, NoProgress, Progress, ProgressFn, Reader, Schema,
    ValidationSettings, Validator, Writer,
};
