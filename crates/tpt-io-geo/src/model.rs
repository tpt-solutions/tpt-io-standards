//! GeoJSON (RFC 7946) data model.
//!
//! The model mirrors RFC 7946: a document is a `FeatureCollection`, a `Feature`,
//! or a bare `Geometry`. Positions are longitude/latitude (optionally with an
//! elevation), matching the RFC 7946 axis ordering.

use alloc::vec::Vec;

use tpt_io_core::json::Json;

/// A single position: `[longitude, latitude]` or `[longitude, latitude, elevation]`
/// per RFC 7946 §3.1.1.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// Longitude (degrees, east-positive).
    pub lon: f64,
    /// Latitude (degrees, north-positive).
    pub lat: f64,
    /// Elevation (metres), if present.
    pub elevation: Option<f64>,
}

/// A 2-D or 3-D bounding box: `[west, south]` / `[west, south, east, north]` /
/// with optional elevation per RFC 7946 §5.
#[derive(Debug, Clone, PartialEq)]
pub struct Bbox(pub Vec<f64>);

/// A GeoJSON geometry object.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// A single position.
    Point(Position),
    /// A set of positions.
    MultiPoint(Vec<Position>),
    /// A connected line of positions.
    LineString(Vec<Position>),
    /// A set of lines.
    MultiLineString(Vec<Vec<Position>>),
    /// A polygon: first element is the exterior ring, the rest are holes.
    Polygon(Vec<Vec<Position>>),
    /// A set of polygons.
    MultiPolygon(Vec<Vec<Vec<Position>>>),
    /// A heterogeneous collection of geometries.
    GeometryCollection(Vec<Geometry>),
}

impl Geometry {
    /// The GeoJSON `"type"` string for this geometry.
    pub fn type_name(&self) -> &'static str {
        match self {
            Geometry::Point(_) => "Point",
            Geometry::MultiPoint(_) => "MultiPoint",
            Geometry::LineString(_) => "LineString",
            Geometry::MultiLineString(_) => "MultiLineString",
            Geometry::Polygon(_) => "Polygon",
            Geometry::MultiPolygon(_) => "MultiPolygon",
            Geometry::GeometryCollection(_) => "GeometryCollection",
        }
    }
}

/// A GeoJSON `Feature`: a geometry plus arbitrary properties.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// The feature's geometry, or `None` for a null-geometry feature.
    pub geometry: Option<Geometry>,
    /// Application-defined properties (the `"properties"` object).
    pub properties: Option<Json>,
    /// Optional identifier (`"id"`), which may be a string or number.
    pub id: Option<Json>,
    /// Optional bounding box.
    pub bbox: Option<Bbox>,
}

/// A GeoJSON `FeatureCollection`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FeatureCollection {
    /// The features in document order.
    pub features: Vec<Feature>,
    /// Optional bounding box covering all features.
    pub bbox: Option<Bbox>,
}

/// A parsed GeoJSON document, which may be any of the three top-level forms.
#[derive(Debug, Clone, PartialEq)]
pub enum GeoJson {
    /// A feature collection.
    FeatureCollection(FeatureCollection),
    /// A single feature.
    Feature(Feature),
    /// A bare geometry.
    Geometry(Geometry),
}
