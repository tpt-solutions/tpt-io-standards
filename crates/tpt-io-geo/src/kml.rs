//! KML 2.2 placemarks, and KMZ (zipped KML).
//!
//! [`parse`] reads every `Placemark` in a KML document as a feature. Its `name` and `description` become
//! properties, its `ExtendedData` values are added as properties, and its geometry (`Point`, `LineString`,
//! `Polygon`, or a `MultiGeometry` of these) is converted. [`write`] produces the same subset of KML.
//!
//! KMZ is a ZIP archive whose `doc.kml` part is the KML document. [`parse_kmz`] and [`write_kmz`] need the
//! `kmz` feature.
//!
//! A `MultiGeometry` reads as a [`GeometryEnum::GeometryCollection`], so a multi-point written to KML comes
//! back as a collection of points.
//!
//! Spec access: KML 2.2 is an OGC standard, published openly. No specification text is copied into this crate.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::xml::{self, Element};

use crate::error::GeoError;
use crate::geojson::{Feature, FeatureCollection, FeatureId, Geometry, GeometryEnum, Position};
use crate::json::Json;

/// Parse a KML document into a feature collection, one feature per `Placemark`.
pub fn parse(input: &[u8]) -> Result<FeatureCollection, GeoError> {
    let root = xml::parse(input)?;
    let mut features = Vec::new();
    for pm in root.descendants_named("Placemark") {
        features.push(placemark(pm)?);
    }
    Ok(FeatureCollection { features, bbox: None, foreign_members: Vec::new() })
}

/// Parse a KMZ archive: the `doc.kml` part of a ZIP container.
#[cfg(feature = "kmz")]
pub fn parse_kmz(input: &[u8]) -> Result<FeatureCollection, GeoError> {
    let doc = tpt_io_core::zip::entry(input, "doc.kml")?
        .ok_or_else(|| GeoError::custom(0, "KMZ archive has no doc.kml"))?;
    parse(&doc)
}

/// Write a feature collection as a KML document.
pub fn write(fc: &FeatureCollection) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\"><Document>\n",
    );
    for f in &fc.features {
        s.push_str("<Placemark>");
        if let Some(name) = text_property(f, "name") {
            let _ = write!(s, "<name>{}</name>", xml::escape_text(name));
        }
        if let Some(description) = text_property(f, "description") {
            let _ = write!(s, "<description>{}</description>", xml::escape_text(description));
        }
        if let Some(g) = &f.geometry {
            write_shape(&mut s, &g.geometry);
        }
        s.push_str("</Placemark>\n");
    }
    s.push_str("</Document></kml>\n");
    s
}

/// Write a feature collection as a KMZ archive holding `doc.kml`.
#[cfg(feature = "kmz")]
pub fn write_kmz(fc: &FeatureCollection) -> Vec<u8> {
    tpt_io_core::zip::write_stored(&[("doc.kml", write(fc).as_bytes())])
}

/// A string-valued property, or `None` when the feature has no such property.
pub(crate) fn text_property<'a>(f: &'a Feature, key: &str) -> Option<&'a str> {
    match &f.properties {
        Json::Object(entries) => {
            entries.iter().find(|(k, _)| k == key).and_then(|(_, v)| v.as_str())
        }
        _ => None,
    }
}

fn plain(geometry: GeometryEnum) -> Geometry {
    Geometry { geometry, bbox: None, foreign_members: Vec::new() }
}

fn placemark(pm: &Element) -> Result<Feature, GeoError> {
    let mut props: Vec<(String, Json)> = Vec::new();
    for key in ["name", "description"] {
        if let Some(t) = pm.child(key).map(|c| c.text_trimmed()).filter(|t| !t.is_empty()) {
            props.push((key.to_string(), Json::String(t.to_string())));
        }
    }
    for data in pm.descendants_named("Data") {
        if let (Some(name), Some(value)) = (data.attr("name"), data.child("value")) {
            props.push((name.to_string(), Json::String(value.text_trimmed().to_string())));
        }
    }
    let mut geometry = None;
    for c in &pm.children {
        if let Some(g) = shape(c)? {
            geometry = Some(plain(g));
            break;
        }
    }
    Ok(Feature {
        geometry,
        properties: if props.is_empty() { Json::Null } else { Json::Object(props) },
        id: pm.attr("id").map(|s| FeatureId::String(s.to_string())),
        bbox: None,
        foreign_members: Vec::new(),
    })
}

/// The geometry of one KML element, or `None` for an element that is not a geometry.
fn shape(el: &Element) -> Result<Option<GeometryEnum>, GeoError> {
    Ok(match el.local_name() {
        "Point" => match coords(el)?.into_iter().next() {
            Some(p) => Some(GeometryEnum::Point(p)),
            None => return Err(GeoError::custom(el.offset, "KML Point has no coordinates")),
        },
        "LineString" => Some(GeometryEnum::LineString(coords(el)?)),
        "Polygon" => {
            let mut rings = Vec::new();
            if let Some(outer) = el.child("outerBoundaryIs") {
                rings.push(ring(outer)?);
            }
            for inner in el.children_named("innerBoundaryIs") {
                rings.push(ring(inner)?);
            }
            Some(GeometryEnum::Polygon(rings))
        }
        "MultiGeometry" => {
            let mut parts = Vec::new();
            for c in &el.children {
                if let Some(g) = shape(c)? {
                    parts.push(plain(g));
                }
            }
            Some(GeometryEnum::GeometryCollection(parts))
        }
        _ => None,
    })
}

fn ring(boundary: &Element) -> Result<Vec<Position>, GeoError> {
    match boundary.child("LinearRing") {
        Some(r) => coords(r),
        None => Ok(Vec::new()),
    }
}

/// The positions in an element's `coordinates` child: whitespace-separated `lon,lat[,alt]` tuples.
fn coords(el: &Element) -> Result<Vec<Position>, GeoError> {
    let text = match el.child("coordinates") {
        Some(c) => c.text_trimmed(),
        None => return Ok(Vec::new()),
    };
    let mut out = Vec::new();
    for tuple in text.split_whitespace() {
        let mut parts = tuple.split(',');
        let mut values =
            alloc::vec![coord(parts.next(), el.offset)?, coord(parts.next(), el.offset)?];
        if let Some(alt) = parts.next() {
            values.push(coord(Some(alt), el.offset)?);
        }
        out.push(Position(values));
    }
    Ok(out)
}

fn coord(s: Option<&str>, offset: usize) -> Result<f64, GeoError> {
    s.and_then(|t| t.trim().parse::<f64>().ok())
        .ok_or_else(|| GeoError::custom(offset, "malformed KML coordinate"))
}

fn write_shape(s: &mut String, g: &GeometryEnum) {
    match g {
        GeometryEnum::Point(p) => point(s, p),
        GeometryEnum::LineString(ps) => line(s, ps),
        GeometryEnum::Polygon(rings) => polygon(s, rings),
        GeometryEnum::MultiPoint(ps) => {
            s.push_str("<MultiGeometry>");
            for p in ps {
                point(s, p);
            }
            s.push_str("</MultiGeometry>");
        }
        GeometryEnum::MultiLineString(lines) => {
            s.push_str("<MultiGeometry>");
            for l in lines {
                line(s, l);
            }
            s.push_str("</MultiGeometry>");
        }
        GeometryEnum::MultiPolygon(polys) => {
            s.push_str("<MultiGeometry>");
            for rings in polys {
                polygon(s, rings);
            }
            s.push_str("</MultiGeometry>");
        }
        GeometryEnum::GeometryCollection(gs) => {
            s.push_str("<MultiGeometry>");
            for g in gs {
                write_shape(s, &g.geometry);
            }
            s.push_str("</MultiGeometry>");
        }
    }
}

fn point(s: &mut String, p: &Position) {
    s.push_str("<Point><coordinates>");
    coord_tuple(s, p);
    s.push_str("</coordinates></Point>");
}

fn line(s: &mut String, ps: &[Position]) {
    s.push_str("<LineString><coordinates>");
    coord_list(s, ps);
    s.push_str("</coordinates></LineString>");
}

fn polygon(s: &mut String, rings: &[Vec<Position>]) {
    s.push_str("<Polygon>");
    for (i, ring) in rings.iter().enumerate() {
        let tag = if i == 0 { "outerBoundaryIs" } else { "innerBoundaryIs" };
        let _ = write!(s, "<{tag}><LinearRing><coordinates>");
        coord_list(s, ring);
        let _ = write!(s, "</coordinates></LinearRing></{tag}>");
    }
    s.push_str("</Polygon>");
}

fn coord_tuple(s: &mut String, p: &Position) {
    let _ = write!(s, "{},{}", p.longitude(), p.latitude());
    if let Some(alt) = p.elevation() {
        let _ = write!(s, ",{alt}");
    }
}

fn coord_list(s: &mut String, ps: &[Position]) {
    for (i, p) in ps.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        coord_tuple(s, p);
    }
}
