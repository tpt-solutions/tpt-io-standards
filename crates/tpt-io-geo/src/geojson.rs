//! GeoJSON (RFC 7946) parsing, serialisation, and validation.
//!
//! GeoJSON describes geographic features as JSON. Per RFC 7946 §3.1.1,
//! a position is an array of two or three numbers ordered
//! `[longitude, latitude, elevation]`. The default coordinate reference
//! system is WGS84 (EPSG:4326); a legacy `crs` member (RFC 7946 §4) is
//! accepted but, when present and non-`null`, only the `name` form is
//! recognised.
//!
//! Parsing is driven by the workspace's from-scratch [`crate::json`] parser.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{Reader, ValidationSettings, Validator};

use crate::error::GeoError;
use crate::json::{parse_str as json_parse_str, Json};

/// A GeoJSON position: `[lon, lat]` or `[lon, lat, z]`.
///
/// Coordinates follow RFC 7946 ordering (longitude first).
#[derive(Debug, Clone, PartialEq)]
pub struct Position(pub Vec<f64>);

impl Position {
    /// Longitude (first component), or 0.0 if absent.
    pub fn longitude(&self) -> f64 {
        self.0.first().copied().unwrap_or(0.0)
    }

    /// Latitude (second component), or 0.0 if absent.
    pub fn latitude(&self) -> f64 {
        self.0.get(1).copied().unwrap_or(0.0)
    }

    /// Elevation (third component), if present.
    pub fn elevation(&self) -> Option<f64> {
        self.0.get(2).copied()
    }

    /// Number of components (2 or 3).
    pub fn dim(&self) -> usize {
        self.0.len()
    }
}

/// An optional bounding box: `[west, south, east, north]` (2D) or with
/// additional `minz, maxz` (3D).
pub type Bbox = Vec<f64>;

/// The concrete geometry types of RFC 7946 §3.1.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryEnum {
    /// A single position.
    Point(Position),
    /// An array of positions.
    MultiPoint(Vec<Position>),
    /// A connected sequence of positions.
    LineString(Vec<Position>),
    /// An array of `LineString`s.
    MultiLineString(Vec<Vec<Position>>),
    /// A polygon: an exterior ring followed by zero or more interior rings.
    Polygon(Vec<Vec<Position>>),
    /// An array of `Polygon`s.
    MultiPolygon(Vec<Vec<Vec<Position>>>),
    /// A heterogeneous collection of geometries.
    GeometryCollection(Vec<Geometry>),
}

/// A GeoJSON geometry object.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// The geometry payload.
    pub geometry: GeometryEnum,
    /// Optional bounding box.
    pub bbox: Option<Bbox>,
    /// Preserved "foreign members" (members not defined by RFC 7946).
    pub foreign_members: Vec<(String, Json)>,
}

/// A GeoJSON Feature.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// The geometry, or `None` for a null geometry.
    pub geometry: Option<Geometry>,
    /// Feature properties (RFC 7946 §3.2).
    pub properties: Json,
    /// Optional feature identifier (string or number).
    pub id: Option<FeatureId>,
    /// Optional bounding box.
    pub bbox: Option<Bbox>,
    /// Preserved foreign members.
    pub foreign_members: Vec<(String, Json)>,
}

/// A feature identifier (RFC 7946 §3.2 allows string or number `id`).
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureId {
    /// A string identifier.
    String(String),
    /// A numeric identifier.
    Number(f64),
}

/// A GeoJSON FeatureCollection.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FeatureCollection {
    /// The features in document order.
    pub features: Vec<Feature>,
    /// Optional bounding box.
    pub bbox: Option<Bbox>,
    /// Preserved foreign members.
    pub foreign_members: Vec<(String, Json)>,
}

fn require_type(obj: &Json, expected: &str) -> Result<(), GeoError> {
    match obj.get("type").and_then(Json::as_str) {
        Some(t) if t == expected => Ok(()),
        Some(t) => {
            Err(GeoError::geojson(0, alloc::format!("expected type '{expected}', found '{t}'")))
        }
        None => Err(GeoError::geojson(0, "missing 'type' member")),
    }
}

fn parse_position(v: &Json) -> Result<Position, GeoError> {
    match v.as_array() {
        Some(arr) if arr.len() >= 2 => {
            let mut coords = Vec::with_capacity(arr.len());
            for (i, el) in arr.iter().enumerate() {
                match el.as_f64() {
                    Some(n) => coords.push(n),
                    None => return Err(GeoError::geojson(i, "position component is not a number")),
                }
            }
            Ok(Position(coords))
        }
        _ => Err(GeoError::geojson(0, "position must be an array of >= 2 numbers")),
    }
}

fn parse_position_array(v: &Json) -> Result<Vec<Position>, GeoError> {
    match v.as_array() {
        Some(arr) => arr.iter().map(parse_position).collect(),
        None => Err(GeoError::geojson(0, "expected an array of positions")),
    }
}

fn parse_line_array(v: &Json) -> Result<Vec<Vec<Position>>, GeoError> {
    match v.as_array() {
        Some(arr) => arr.iter().map(parse_position_array).collect(),
        None => Err(GeoError::geojson(0, "expected an array of linestrings")),
    }
}

fn parse_polygon_array(v: &Json) -> Result<Vec<Vec<Vec<Position>>>, GeoError> {
    match v.as_array() {
        Some(arr) => arr
            .iter()
            .map(|ring_arr| match ring_arr.as_array() {
                Some(rings) => rings.iter().map(parse_position_array).collect(),
                None => Err(GeoError::geojson(0, "polygon ring is not an array")),
            })
            .collect(),
        None => Err(GeoError::geojson(0, "expected an array of polygons")),
    }
}

fn parse_bbox(obj: &Json) -> Result<Option<Bbox>, GeoError> {
    match obj.get("bbox") {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Array(arr)) => {
            let mut b = Vec::with_capacity(arr.len());
            for el in arr {
                match el.as_f64() {
                    Some(n) => b.push(n),
                    None => return Err(GeoError::geojson(0, "bbox component is not a number")),
                }
            }
            Ok(Some(b))
        }
        Some(_) => Err(GeoError::geojson(0, "bbox must be an array")),
    }
}

fn parse_foreign_members(obj: &Json, known: &[&str]) -> Vec<(String, Json)> {
    let mut out = Vec::new();
    if let Some(entries) = obj.as_object() {
        for (k, v) in entries {
            if !known.contains(&k.as_str()) {
                out.push((k.clone(), v.clone()));
            }
        }
    }
    out
}

fn parse_geometry(obj: &Json) -> Result<Geometry, GeoError> {
    let known = ["type", "coordinates", "geometries", "bbox"];
    let geom = match obj.get("type").and_then(Json::as_str) {
        Some("Point") => GeometryEnum::Point(parse_position(
            obj.get("coordinates")
                .ok_or_else(|| GeoError::geojson(0, "Point missing coordinates"))?,
        )?),
        Some("MultiPoint") => GeometryEnum::MultiPoint(parse_position_array(
            obj.get("coordinates")
                .ok_or_else(|| GeoError::geojson(0, "MultiPoint missing coordinates"))?,
        )?),
        Some("LineString") => GeometryEnum::LineString(parse_position_array(
            obj.get("coordinates")
                .ok_or_else(|| GeoError::geojson(0, "LineString missing coordinates"))?,
        )?),
        Some("MultiLineString") => GeometryEnum::MultiLineString(parse_line_array(
            obj.get("coordinates")
                .ok_or_else(|| GeoError::geojson(0, "MultiLineString missing coordinates"))?,
        )?),
        Some("Polygon") => GeometryEnum::Polygon(parse_line_array(
            obj.get("coordinates")
                .ok_or_else(|| GeoError::geojson(0, "Polygon missing coordinates"))?,
        )?),
        Some("MultiPolygon") => GeometryEnum::MultiPolygon(parse_polygon_array(
            obj.get("coordinates")
                .ok_or_else(|| GeoError::geojson(0, "MultiPolygon missing coordinates"))?,
        )?),
        Some("GeometryCollection") => {
            let geoms = match obj.get("geometries").and_then(Json::as_array) {
                Some(arr) => arr.iter().map(parse_geometry).collect::<Result<_, _>>()?,
                None => return Err(GeoError::geojson(0, "GeometryCollection missing geometries")),
            };
            GeometryEnum::GeometryCollection(geoms)
        }
        Some(t) => return Err(GeoError::geojson(0, alloc::format!("unknown geometry type '{t}'"))),
        None => return Err(GeoError::geojson(0, "geometry missing 'type'")),
    };
    Ok(Geometry {
        geometry: geom,
        bbox: parse_bbox(obj)?,
        foreign_members: parse_foreign_members(obj, &known),
    })
}

fn parse_feature(obj: &Json) -> Result<Feature, GeoError> {
    require_type(obj, "Feature")?;
    let geom = match obj.get("geometry") {
        None | Some(Json::Null) => None,
        Some(g) => Some(parse_geometry(g)?),
    };
    let properties = obj.get("properties").cloned().unwrap_or(Json::Object(Vec::new()));
    let id = match obj.get("id") {
        Some(Json::String(s)) => Some(FeatureId::String(s.clone())),
        Some(Json::Number(n)) => Some(FeatureId::Number(*n)),
        Some(Json::Null) | None => None,
        Some(_) => return Err(GeoError::geojson(0, "feature 'id' must be string or number")),
    };
    let known = ["type", "geometry", "properties", "id", "bbox"];
    Ok(Feature {
        geometry: geom,
        properties,
        id,
        bbox: parse_bbox(obj)?,
        foreign_members: parse_foreign_members(obj, &known),
    })
}

/// Parse a single GeoJSON `Feature` object from a JSON value.
///
/// Used by the streaming reader, which hands each top-level feature object to
/// this function individually.
pub fn parse_feature_json(value: &Json) -> Result<Feature, GeoError> {
    match value.get("type").and_then(Json::as_str) {
        Some("Feature") => parse_feature(value),
        Some("FeatureCollection") => {
            // A nested collection: take its first feature for streaming continuity.
            match value.get("features").and_then(Json::as_array) {
                Some(arr) if !arr.is_empty() => parse_feature(&arr[0]),
                _ => Err(GeoError::geojson(0, "empty nested FeatureCollection")),
            }
        }
        Some(t) => Err(GeoError::geojson(0, alloc::format!("expected Feature, found '{t}'"))),
        None => Err(GeoError::geojson(0, "missing 'type' member")),
    }
}

/// Parse a GeoJSON document from a string slice.
///
/// Accepts a `FeatureCollection`, a single `Feature`, or any standalone
/// geometry object.
pub fn parse_str(input: &str) -> Result<FeatureCollection, GeoError> {
    let root = json_parse_str(input).map_err(GeoError::from)?;
    parse_json_value(&root)
}

/// Parse a GeoJSON document from a byte slice (interpreted as UTF-8).
pub fn parse(input: &[u8]) -> Result<FeatureCollection, GeoError> {
    let s = core::str::from_utf8(input)
        .map_err(|_| GeoError::geojson(0, "input is not valid UTF-8"))?;
    parse_str(s)
}

/// Parse any GeoJSON value into a [`FeatureCollection`].
///
/// Standalone geometry objects and Features are wrapped in a single-feature
/// collection so callers always receive a uniform type.
pub fn parse_json_value(root: &Json) -> Result<FeatureCollection, GeoError> {
    match root.get("type").and_then(Json::as_str) {
        Some("FeatureCollection") => {
            let known = ["type", "features", "bbox"];
            let features = match root.get("features").and_then(Json::as_array) {
                Some(arr) => arr.iter().map(parse_feature).collect::<Result<_, _>>()?,
                None => return Err(GeoError::geojson(0, "FeatureCollection missing features")),
            };
            Ok(FeatureCollection {
                features,
                bbox: parse_bbox(root)?,
                foreign_members: parse_foreign_members(root, &known),
            })
        }
        Some("Feature") => Ok(FeatureCollection {
            features: alloc::vec![parse_feature(root)?],
            bbox: None,
            foreign_members: Vec::new(),
        }),
        Some(_) => {
            // Standalone geometry.
            let geom = parse_geometry(root)?;
            let feature = Feature {
                geometry: Some(geom),
                properties: Json::Object(Vec::new()),
                id: None,
                bbox: None,
                foreign_members: Vec::new(),
            };
            Ok(FeatureCollection {
                features: alloc::vec![feature],
                bbox: None,
                foreign_members: Vec::new(),
            })
        }
        None => Err(GeoError::geojson(0, "missing 'type' member")),
    }
}

// ---- Serialisation -------------------------------------------------------

fn coords_to_json(positions: &[Position]) -> Json {
    Json::Array(
        positions
            .iter()
            .map(|p| Json::Array(p.0.iter().map(|n| Json::Number(*n)).collect()))
            .collect(),
    )
}

fn position_to_json(p: &Position) -> Json {
    Json::Array(p.0.iter().map(|n| Json::Number(*n)).collect())
}

fn geometry_to_json(g: &Geometry) -> Json {
    let mut entries: Vec<(String, Json)> = Vec::new();
    entries.push(("type".to_string(), Json::String(geometry_type_name(&g.geometry).to_string())));
    match &g.geometry {
        GeometryEnum::Point(p) => {
            entries.push(("coordinates".to_string(), position_to_json(p)));
        }
        GeometryEnum::MultiPoint(ps) => {
            entries.push(("coordinates".to_string(), coords_to_json(ps)))
        }
        GeometryEnum::LineString(ps) => {
            entries.push(("coordinates".to_string(), coords_to_json(ps)))
        }
        GeometryEnum::MultiLineString(ls) => {
            entries.push((
                "coordinates".to_string(),
                Json::Array(ls.iter().map(|line| coords_to_json(line)).collect()),
            ));
        }
        GeometryEnum::Polygon(rings) => {
            entries.push((
                "coordinates".to_string(),
                Json::Array(rings.iter().map(|ring| coords_to_json(ring)).collect()),
            ));
        }
        GeometryEnum::MultiPolygon(polys) => {
            entries.push((
                "coordinates".to_string(),
                Json::Array(
                    polys
                        .iter()
                        .map(|rings| {
                            Json::Array(rings.iter().map(|ring| coords_to_json(ring)).collect())
                        })
                        .collect(),
                ),
            ));
        }
        GeometryEnum::GeometryCollection(geoms) => {
            entries.push((
                "geometries".to_string(),
                Json::Array(geoms.iter().map(geometry_to_json).collect()),
            ));
        }
    }
    if let Some(bbox) = &g.bbox {
        entries.push(("bbox".to_string(), bbox_to_json(bbox)));
    }
    entries.extend(g.foreign_members.iter().cloned());
    Json::Object(entries)
}

fn geometry_type_name(g: &GeometryEnum) -> &'static str {
    match g {
        GeometryEnum::Point(_) => "Point",
        GeometryEnum::MultiPoint(_) => "MultiPoint",
        GeometryEnum::LineString(_) => "LineString",
        GeometryEnum::MultiLineString(_) => "MultiLineString",
        GeometryEnum::Polygon(_) => "Polygon",
        GeometryEnum::MultiPolygon(_) => "MultiPolygon",
        GeometryEnum::GeometryCollection(_) => "GeometryCollection",
    }
}

fn bbox_to_json(b: &Bbox) -> Json {
    Json::Array(b.iter().map(|n| Json::Number(*n)).collect())
}

fn feature_to_json(f: &Feature) -> Json {
    let mut entries: Vec<(String, Json)> = Vec::new();
    entries.push(("type".to_string(), Json::String("Feature".to_string())));
    entries.push((
        "geometry".to_string(),
        match &f.geometry {
            Some(g) => geometry_to_json(g),
            None => Json::Null,
        },
    ));
    entries.push(("properties".to_string(), f.properties.clone()));
    if let Some(id) = &f.id {
        entries.push((
            "id".to_string(),
            match id {
                FeatureId::String(s) => Json::String(s.clone()),
                FeatureId::Number(n) => Json::Number(*n),
            },
        ));
    }
    if let Some(bbox) = &f.bbox {
        entries.push(("bbox".to_string(), bbox_to_json(bbox)));
    }
    entries.extend(f.foreign_members.iter().cloned());
    Json::Object(entries)
}

/// Serialise a [`FeatureCollection`] to a compact JSON string.
pub fn to_string(fc: &FeatureCollection) -> String {
    let mut entries: Vec<(String, Json)> = Vec::new();
    entries.push(("type".to_string(), Json::String("FeatureCollection".to_string())));
    entries.push((
        "features".to_string(),
        Json::Array(fc.features.iter().map(feature_to_json).collect()),
    ));
    if let Some(bbox) = &fc.bbox {
        entries.push(("bbox".to_string(), bbox_to_json(bbox)));
    }
    entries.extend(fc.foreign_members.iter().cloned());
    crate::json::to_string(&Json::Object(entries))
}

/// A GeoJSON reader implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct GeoJsonReader {
    settings: ValidationSettings,
}

impl GeoJsonReader {
    /// Create a new reader with default validation settings.
    pub fn new() -> Self {
        Self { settings: ValidationSettings::new() }
    }

    /// Builder: configure validation settings.
    pub fn with_validation(mut self, settings: ValidationSettings) -> Self {
        self.settings = settings;
        self
    }
}

impl Reader<FeatureCollection> for GeoJsonReader {
    type Error = GeoError;

    fn parse(&self, input: &[u8]) -> Result<FeatureCollection, GeoError> {
        let fc = parse(input)?;
        self.settings.run(&GeoJsonValidator, &fc)?;
        Ok(fc)
    }
}

/// GeoJSON syntactic/semantic validator.
#[derive(Debug, Clone, Default)]
pub struct GeoJsonValidator;

impl Validator<FeatureCollection> for GeoJsonValidator {
    type Error = GeoError;

    fn validate_syntactic(&self, fc: &FeatureCollection) -> Result<(), GeoError> {
        for f in &fc.features {
            if let Some(g) = &f.geometry {
                validate_geometry_syntax(g)?;
            }
        }
        Ok(())
    }

    fn validate_semantic(&self, fc: &FeatureCollection) -> Result<(), GeoError> {
        // Semantic checks: positions within valid lon/lat ranges.
        for f in &fc.features {
            if let Some(g) = &f.geometry {
                validate_geometry_semantic(g)?;
            }
        }
        Ok(())
    }
}

fn validate_geometry_syntax(g: &Geometry) -> Result<(), GeoError> {
    match &g.geometry {
        GeometryEnum::Polygon(_) | GeometryEnum::MultiPolygon(_) => {
            // Every polygon ring must have >= 4 positions (closed) when non-empty.
            let polys: &[Vec<Vec<Position>>] = match &g.geometry {
                GeometryEnum::Polygon(rings) => core::slice::from_ref(rings),
                GeometryEnum::MultiPolygon(polys) => polys,
                _ => return Ok(()),
            };
            for poly in polys {
                for ring in poly {
                    if !ring.is_empty() && ring.len() < 4 {
                        return Err(GeoError::geojson(
                            0,
                            "polygon ring must have at least 4 positions",
                        ));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_geometry_semantic(g: &Geometry) -> Result<(), GeoError> {
    let visit = |p: &Position| -> Result<(), GeoError> {
        let lon = p.longitude();
        let lat = p.latitude();
        if !((-180.0..=180.0).contains(&lon)) {
            return Err(GeoError::invalid_coordinate(p.0.clone(), "longitude out of range"));
        }
        if !((-90.0..=90.0).contains(&lat)) {
            return Err(GeoError::invalid_coordinate(p.0.clone(), "latitude out of range"));
        }
        Ok(())
    };
    match &g.geometry {
        GeometryEnum::Point(p) => visit(p)?,
        GeometryEnum::MultiPoint(ps) | GeometryEnum::LineString(ps) => {
            for p in ps {
                visit(p)?;
            }
        }
        GeometryEnum::MultiLineString(ls) => {
            for line in ls {
                for p in line {
                    visit(p)?;
                }
            }
        }
        GeometryEnum::Polygon(rings) => {
            for ring in rings {
                for p in ring {
                    visit(p)?;
                }
            }
        }
        GeometryEnum::MultiPolygon(polys) => {
            for poly in polys {
                for ring in poly {
                    for p in ring {
                        visit(p)?;
                    }
                }
            }
        }
        GeometryEnum::GeometryCollection(geoms) => {
            for g in geoms {
                validate_geometry_semantic(g)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_point_feature() {
        let src = r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "geometry": {"type": "Point", "coordinates": [125.6, 10.1]},
                "properties": {"name": "Dinagat"}
            }]
        }"#;
        let fc = parse_str(src).unwrap();
        assert_eq!(fc.features.len(), 1);
        let g = fc.features[0].geometry.as_ref().unwrap();
        match &g.geometry {
            GeometryEnum::Point(p) => {
                assert_eq!(p.longitude(), 125.6);
                assert_eq!(p.latitude(), 10.1);
            }
            _ => panic!("expected point"),
        }
        assert_eq!(fc.features[0].properties.get("name").and_then(Json::as_str), Some("Dinagat"));
    }

    #[test]
    fn round_trip_feature_collection() {
        let src = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"Polygon","coordinates":[[[0,0],[4,0],[4,4],[0,4],[0,0]]]},"properties":{"a":1}}
        ]}"#;
        let fc = parse_str(src).unwrap();
        let out = to_string(&fc);
        let fc2 = parse_str(&out).unwrap();
        assert_eq!(fc, fc2);
    }

    #[test]
    fn semantic_validation_rejects_bad_coord() {
        let src = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"Point","coordinates":[200,10]},"properties":{}}
        ]}"#;
        let fc = parse_str(src).unwrap();
        assert!(GeoJsonValidator.validate_semantic(&fc).is_err());
    }

    #[test]
    fn parse_geometry_collection() {
        let src = r#"{"type":"GeometryCollection","geometries":[
            {"type":"Point","coordinates":[1,2]},
            {"type":"LineString","coordinates":[[1,2],[3,4]]}
        ]}"#;
        let fc = parse_str(src).unwrap();
        assert_eq!(fc.features.len(), 1);
    }

    #[test]
    fn foreign_members_preserved() {
        let src = r#"{"type":"Feature","geometry":{"type":"Point","coordinates":[1,2]},"properties":{},"extra":"kept"}"#;
        let fc = parse_str(src).unwrap();
        assert_eq!(fc.features[0].foreign_members.len(), 1);
    }
}
