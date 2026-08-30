//! Error type for the Geo parser, built on [`tpt_io_core::ParseError`].
//!
//! The `Core` variant wraps a [`tpt_io_core::ParseError`] carrying the byte
//! offset, expected-vs-actual token, spec reference, and surrounding bytes.
//! Format-specific failure modes (GeoJSON, Shapefile, GeoTIFF, CRS, spatial
//! indexing) each get a dedicated variant so callers can match precisely.

#[cfg(not(any(feature = "alloc", feature = "std")))]
compile_error!(
    "tpt-io-geo requires the `alloc` or `std` feature: its error type owns structured parse data."
);

use alloc::string::String;
use alloc::vec::Vec;

use tpt_io_core::ParseError;

use crate::json::JsonError;

/// Errors produced while parsing Geo data.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GeoError {
    /// A structural error reported by the shared core.
    Core(ParseError),
    /// A JSON-level failure while reading GeoJSON (RFC 7946).
    Json {
        /// Byte offset of the JSON failure.
        offset: usize,
        /// Human-readable reason.
        reason: String,
    },
    /// A GeoJSON semantic/structural violation (e.g. missing `type`).
    GeoJson {
        /// Byte offset where the problem was detected.
        offset: usize,
        /// Description of the GeoJSON problem.
        reason: String,
    },
    /// A Shapefile (.shp) format error.
    Shapefile {
        /// Byte offset (within the relevant file) of the failure.
        offset: usize,
        /// Description of the Shapefile problem.
        reason: String,
    },
    /// A dBASE (.dbf) table error.
    Dbf {
        /// Byte offset (within the .dbf) of the failure.
        offset: usize,
        /// Description of the dBASE problem.
        reason: String,
    },
    /// A GeoTIFF (TIFF) format error.
    GeoTiff {
        /// Byte offset (within the file) of the failure.
        offset: usize,
        /// Description of the GeoTIFF problem.
        reason: String,
    },
    /// A coordinate reference system error (PROJ/EPSG/datum).
    Crs {
        /// Description of the CRS problem.
        reason: String,
    },
    /// A spatial-index error (e.g. query against an inconsistent index).
    SpatialIndex {
        /// Description of the spatial-index problem.
        reason: String,
    },
    /// A coordinate failed validation (range or ordering).
    InvalidCoordinate {
        /// The offending coordinate (lon, lat, [z]).
        coord: Vec<f64>,
        /// Description of the validation failure.
        reason: String,
    },
    /// A Geo-specific feature or construct that is not yet supported.
    Unsupported(&'static str),
}

impl From<ParseError> for GeoError {
    fn from(e: ParseError) -> Self {
        GeoError::Core(e)
    }
}

impl From<JsonError> for GeoError {
    fn from(e: JsonError) -> Self {
        GeoError::Json { offset: e.offset, reason: e.reason }
    }
}

impl GeoError {
    /// Build a `Core` parse error from a custom reason at `offset`.
    pub fn custom(offset: usize, reason: impl Into<String>) -> Self {
        GeoError::Core(ParseError::custom(offset, reason.into()))
    }

    /// Build a `Core` unexpected-EOF error.
    pub fn eof(offset: usize) -> Self {
        GeoError::Core(ParseError::eof(offset))
    }

    /// Build a GeoJSON error.
    pub fn geojson(offset: usize, reason: impl Into<String>) -> Self {
        GeoError::GeoJson { offset, reason: reason.into() }
    }

    /// Build a Shapefile error.
    pub fn shapefile(offset: usize, reason: impl Into<String>) -> Self {
        GeoError::Shapefile { offset, reason: reason.into() }
    }

    /// Build a dBASE error.
    pub fn dbf(offset: usize, reason: impl Into<String>) -> Self {
        GeoError::Dbf { offset, reason: reason.into() }
    }

    /// Build a GeoTIFF error.
    pub fn geotiff(offset: usize, reason: impl Into<String>) -> Self {
        GeoError::GeoTiff { offset, reason: reason.into() }
    }

    /// Build a CRS error.
    pub fn crs(reason: impl Into<String>) -> Self {
        GeoError::Crs { reason: reason.into() }
    }

    /// Build a spatial-index error.
    pub fn spatial_index(reason: impl Into<String>) -> Self {
        GeoError::SpatialIndex { reason: reason.into() }
    }

    /// Build an invalid-coordinate error.
    pub fn invalid_coordinate(coord: Vec<f64>, reason: impl Into<String>) -> Self {
        GeoError::InvalidCoordinate { coord, reason: reason.into() }
    }
}

impl core::fmt::Display for GeoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GeoError::Core(e) => write!(f, "{e}"),
            GeoError::Json { offset, reason } => write!(f, "JSON error at {offset}: {reason}"),
            GeoError::GeoJson { offset, reason } => {
                write!(f, "GeoJSON error at {offset}: {reason}")
            }
            GeoError::Shapefile { offset, reason } => {
                write!(f, "Shapefile error at {offset}: {reason}")
            }
            GeoError::Dbf { offset, reason } => write!(f, "DBF error at {offset}: {reason}"),
            GeoError::GeoTiff { offset, reason } => {
                write!(f, "GeoTIFF error at {offset}: {reason}")
            }
            GeoError::Crs { reason } => write!(f, "CRS error: {reason}"),
            GeoError::SpatialIndex { reason } => write!(f, "spatial index error: {reason}"),
            GeoError::InvalidCoordinate { coord, reason } => {
                write!(f, "invalid coordinate {coord:?}: {reason}")
            }
            GeoError::Unsupported(feat) => write!(f, "Geo unsupported feature: {feat}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GeoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_error_converts() {
        let e = GeoError::from(ParseError::eof(0));
        assert!(matches!(e, GeoError::Core(_)));
    }

    #[test]
    fn json_error_converts() {
        let e = GeoError::from(JsonError::new(4, "bad"));
        assert!(matches!(e, GeoError::Json { offset: 4, .. }));
    }

    #[test]
    fn constructors() {
        assert!(matches!(GeoError::geojson(1, "x"), GeoError::GeoJson { .. }));
        assert!(matches!(GeoError::crs("y"), GeoError::Crs { .. }));
    }
}
