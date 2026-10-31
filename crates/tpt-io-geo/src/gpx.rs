//! GPX 1.1 waypoints, routes and tracks.
//!
//! [`parse`] reads each `wpt` as a point feature, each `rte` as a line string of its `rtept` points, and each
//! `trk` as a multi-line string with one line per `trkseg`. `name` becomes a property and `ele` the elevation.
//! [`write`] writes point features as `wpt`, and line strings and multi-line strings as `trk`; other geometry
//! types are skipped, since GPX has no equivalent.
//!
//! Spec access: GPX 1.1 is published by Topografix under an open licence. No specification text is copied into
//! this crate.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::xml::{self, Element};

use crate::error::GeoError;
use crate::geojson::{Feature, FeatureCollection, Geometry, GeometryEnum, Position};
use crate::json::Json;
use crate::kml::text_property;

/// Parse a GPX document into a feature collection.
pub fn parse(input: &[u8]) -> Result<FeatureCollection, GeoError> {
    let root = xml::parse(input)?;
    let mut features = Vec::new();
    for el in root.descendants_named("wpt") {
        features.push(feature(GeometryEnum::Point(point_of(el)?), el));
    }
    for el in root.descendants_named("rte") {
        let ps = el.children_named("rtept").map(point_of).collect::<Result<Vec<_>, _>>()?;
        features.push(feature(GeometryEnum::LineString(ps), el));
    }
    for el in root.descendants_named("trk") {
        let mut lines = Vec::new();
        for seg in el.children_named("trkseg") {
            lines.push(seg.children_named("trkpt").map(point_of).collect::<Result<Vec<_>, _>>()?);
        }
        features.push(feature(GeometryEnum::MultiLineString(lines), el));
    }
    Ok(FeatureCollection { features, bbox: None, foreign_members: Vec::new() })
}

/// Write a feature collection as a GPX 1.1 document.
pub fn write(fc: &FeatureCollection) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<gpx version=\"1.1\" creator=\"tpt-io-geo\" \
         xmlns=\"http://www.topografix.com/GPX/1/1\">\n",
    );
    for f in &fc.features {
        let Some(g) = &f.geometry else { continue };
        match &g.geometry {
            GeometryEnum::Point(p) => {
                let _ = write!(s, "<wpt lat=\"{}\" lon=\"{}\">", p.latitude(), p.longitude());
                if let Some(e) = p.elevation() {
                    let _ = write!(s, "<ele>{e}</ele>");
                }
                name(&mut s, f);
                s.push_str("</wpt>\n");
            }
            GeometryEnum::LineString(ps) => track(&mut s, f, &[ps.as_slice()]),
            GeometryEnum::MultiLineString(lines) => {
                let segs: Vec<&[Position]> = lines.iter().map(|l| l.as_slice()).collect();
                track(&mut s, f, &segs);
            }
            _ => {}
        }
    }
    s.push_str("</gpx>\n");
    s
}

fn point_of(el: &Element) -> Result<Position, GeoError> {
    let lat = coord_attr(el, "lat")?;
    let lon = coord_attr(el, "lon")?;
    let mut values = alloc::vec![lon, lat];
    if let Some(e) = el.child("ele") {
        let ele = e
            .text_trimmed()
            .parse::<f64>()
            .map_err(|_| GeoError::custom(e.offset, "malformed GPX elevation"))?;
        values.push(ele);
    }
    Ok(Position(values))
}

fn coord_attr(el: &Element, name: &str) -> Result<f64, GeoError> {
    el.attr(name)
        .and_then(|v| v.parse::<f64>().ok())
        .ok_or_else(|| GeoError::custom(el.offset, "GPX point needs numeric lat and lon"))
}

fn feature(geometry: GeometryEnum, el: &Element) -> Feature {
    let mut props: Vec<(String, Json)> = Vec::new();
    if let Some(n) = el.child("name").map(|c| c.text_trimmed()).filter(|t| !t.is_empty()) {
        props.push(("name".to_string(), Json::String(n.to_string())));
    }
    Feature {
        geometry: Some(Geometry { geometry, bbox: None, foreign_members: Vec::new() }),
        properties: if props.is_empty() { Json::Null } else { Json::Object(props) },
        id: None,
        bbox: None,
        foreign_members: Vec::new(),
    }
}

fn name(s: &mut String, f: &Feature) {
    if let Some(n) = text_property(f, "name") {
        let _ = write!(s, "<name>{}</name>", xml::escape_text(n));
    }
}

fn track(s: &mut String, f: &Feature, segments: &[&[Position]]) {
    s.push_str("<trk>");
    name(s, f);
    for seg in segments {
        s.push_str("<trkseg>\n");
        for p in *seg {
            let _ = write!(s, "<trkpt lat=\"{}\" lon=\"{}\"", p.latitude(), p.longitude());
            match p.elevation() {
                Some(e) => {
                    let _ = writeln!(s, "><ele>{e}</ele></trkpt>");
                }
                None => s.push_str("/>\n"),
            }
        }
        s.push_str("</trkseg>\n");
    }
    s.push_str("</trk>\n");
}
