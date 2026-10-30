//! Coordinate reference system handling for CityGML documents.
//!
//! CityGML uses GML envelopes with `srsName` attributes to declare the
//! coordinate reference system. This module provides utilities to parse,
//! validate, and transform CRS identifiers via [`tpt_io_geo`].
//!
//! The `geo-integration` feature gates the heavier transformation utilities
//! that depend on `tpt-io-geo`.

use alloc::string::String;

use crate::model::Crs;

/// Validate that a CRS identifier is well-formed.
///
/// Accepts URN-style (`urn:ogc:def:crs:EPSG::XXXX`), URL-style
/// (`http://www.opengis.net/def/crs/EPSG/0/XXXX`), and plain EPSG codes.
pub fn validate_crs(crs: &Crs) -> Result<(), &'static str> {
    if crs.identifier.is_empty() {
        return Err("CRS identifier is empty");
    }
    // URN style: urn:ogc:def:crs:EPSG::25832
    if crs.identifier.starts_with("urn:ogc:def:crs:") {
        return Ok(());
    }
    // URL style: http://www.opengis.net/def/crs/EPSG/0/25832
    if crs.identifier.contains("/def/crs/") {
        return Ok(());
    }
    // Plain EPSG code.
    if crs.identifier.parse::<u32>().is_ok() {
        return Ok(());
    }
    Err("unrecognised CRS identifier format")
}

/// Extract the EPSG code from a CRS identifier as a string.
///
/// Returns `None` if no numeric code can be extracted.
pub fn epsg_string(crs: &Crs) -> Option<String> {
    crs.epsg_code().map(|c| c.to_string())
}

/// Check if a CRS is a known geographic (lat/lon) system.
pub fn is_geographic(crs: &Crs) -> bool {
    match crs.epsg_code() {
        Some(4326) | Some(4258) | Some(4267) | Some(4269) => true,
        _ => crs.identifier.to_ascii_lowercase().contains("geographic"),
    }
}

/// Check if a CRS uses metric units (metres).
pub fn is_metric(crs: &Crs) -> bool {
    // Most projected CRS use metres. Geographic CRS use degrees.
    !is_geographic(crs)
}

#[cfg(feature = "geo-integration")]
pub mod transform {
    //! CRS transformation utilities via [`tpt_io_geo`].
    //!
    //! This module is only available when the `geo-integration` feature is
    //! enabled. It provides conversion between CityGML coordinate tuples
    //! and the geospatial primitives from `tpt-io-geo`.

    use crate::model::{Crs, Point3};

    /// Placeholder for a future `tpt_io_geo::GeoCrs`.
    ///
    /// `tpt-io-geo` is currently a scaffold (see `todo.md`); this stub
    /// provides the conversion boundary until that crate implements CRS
    /// handling.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GeoCrsStub {
        /// The CRS identifier string.
        pub identifier: String,
    }

    impl GeoCrsStub {
        /// Create a CRS stub from an identifier string.
        pub fn from_identifier(identifier: &str) -> Result<Self, &'static str> {
            if identifier.is_empty() {
                return Err("empty CRS identifier");
            }
            Ok(Self { identifier: identifier.to_string() })
        }
    }

    /// Convert a CityGML CRS into a [`GeoCrsStub`].
    pub fn to_geo_crs(crs: &Crs) -> Result<GeoCrsStub, &'static str> {
        GeoCrsStub::from_identifier(&crs.identifier)
    }

    /// Transform a single point from the source CRS to EPSG:4326 (WGS84).
    ///
    /// Placeholder: real implementation will use `tpt-io-geo`'s transform
    /// once that crate implements CRS handling.
    pub fn to_wgs84(pt: &Point3, src: &Crs) -> Result<Point3, &'static str> {
        let _ = (pt, src);
        Err("geo transformation not yet implemented in tpt-io-geo")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_urn_style_crs() {
        let crs = Crs { identifier: "urn:ogc:def:crs:EPSG::25832".to_string() };
        assert!(validate_crs(&crs).is_ok());
    }

    #[test]
    fn validate_url_style_crs() {
        let crs = Crs { identifier: "http://www.opengis.net/def/crs/EPSG/0/4326".to_string() };
        assert!(validate_crs(&crs).is_ok());
    }

    #[test]
    fn validate_plain_epsg() {
        let crs = Crs { identifier: "25832".to_string() };
        assert!(validate_crs(&crs).is_ok());
    }

    #[test]
    fn reject_empty_crs() {
        let crs = Crs { identifier: String::new() };
        assert!(validate_crs(&crs).is_err());
    }

    #[test]
    fn is_geographic_detection() {
        let wgs84 = Crs { identifier: "urn:ogc:def:crs:EPSG::4326".to_string() };
        assert!(is_geographic(&wgs84));
        let utm = Crs { identifier: "urn:ogc:def:crs:EPSG::25832".to_string() };
        assert!(!is_geographic(&utm));
    }

    #[test]
    fn is_metric_detection() {
        let utm = Crs { identifier: "urn:ogc:def:crs:EPSG::25832".to_string() };
        assert!(is_metric(&utm));
        let wgs84 = Crs { identifier: "urn:ogc:def:crs:EPSG::4326".to_string() };
        assert!(!is_metric(&wgs84));
    }
}
