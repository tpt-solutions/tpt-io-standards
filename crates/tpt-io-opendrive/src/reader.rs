//! Reader for OpenDRIVE XML documents.

use alloc::string::ToString;
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::OpenDriveError;
use crate::model::{Geometry, GeometryKind, Header, Lane, LaneSection, OpenDrive, Road, Width};

/// Parse an OpenDRIVE (`<OpenDRIVE>`) document.
pub fn parse(input: &[u8]) -> Result<OpenDrive, OpenDriveError> {
    let root = xml::parse(input)?;
    if root.local_name() != "OpenDRIVE" {
        return Err(ParseError::custom(root.offset, "root element is not <OpenDRIVE>").into());
    }
    let header = match root.child("header") {
        Some(h) => Header {
            rev_major: num_u32(h, "revMajor")?.unwrap_or(0),
            rev_minor: num_u32(h, "revMinor")?.unwrap_or(0),
            name: h.attr("name").unwrap_or("").to_string(),
        },
        None => Header::default(),
    };
    let mut roads = Vec::new();
    for r in root.children_named("road") {
        roads.push(road(r)?);
    }
    Ok(OpenDrive { header, roads })
}

fn road(el: &Element) -> Result<Road, OpenDriveError> {
    let id = required(el, "id")?.to_string();
    let length = num(el, "length")?.unwrap_or(0.0);
    let junction = match el.attr("junction") {
        None | Some("-1") => None,
        Some(j) => Some(j.to_string()),
    };
    let mut geometry = Vec::new();
    if let Some(plan) = el.child("planView") {
        for g in plan.children_named("geometry") {
            geometry.push(segment(g)?);
        }
    }
    let mut lane_sections = Vec::new();
    if let Some(lanes) = el.child("lanes") {
        for ls in lanes.children_named("laneSection") {
            lane_sections.push(lane_section(ls)?);
        }
    }
    Ok(Road {
        id,
        name: el.attr("name").unwrap_or("").to_string(),
        length,
        junction,
        geometry,
        lane_sections,
    })
}

fn segment(el: &Element) -> Result<Geometry, OpenDriveError> {
    let kind_el = el
        .children
        .first()
        .ok_or_else(|| ParseError::custom(el.offset, "geometry has no shape element"))?;
    let kind = match kind_el.local_name() {
        "line" => GeometryKind::Line,
        "arc" => GeometryKind::Arc { curvature: num(kind_el, "curvature")?.unwrap_or(0.0) },
        "spiral" => GeometryKind::Spiral {
            curv_start: num(kind_el, "curvStart")?.unwrap_or(0.0),
            curv_end: num(kind_el, "curvEnd")?.unwrap_or(0.0),
        },
        "poly3" => GeometryKind::Poly3 {
            a: num(kind_el, "a")?.unwrap_or(0.0),
            b: num(kind_el, "b")?.unwrap_or(0.0),
            c: num(kind_el, "c")?.unwrap_or(0.0),
            d: num(kind_el, "d")?.unwrap_or(0.0),
        },
        "paramPoly3" => GeometryKind::ParamPoly3 {
            u: coefficients(kind_el, "U")?,
            v: coefficients(kind_el, "V")?,
        },
        _ => return Err(OpenDriveError::Core(ParseError::custom(kind_el.offset, "unsupported geometry shape"))),
    };
    Ok(Geometry {
        s: num(el, "s")?.unwrap_or(0.0),
        x: num(el, "x")?.unwrap_or(0.0),
        y: num(el, "y")?.unwrap_or(0.0),
        hdg: num(el, "hdg")?.unwrap_or(0.0),
        length: num(el, "length")?.unwrap_or(0.0),
        kind,
    })
}

/// The four coefficients of a parametric cubic, named `a<axis>`, `b<axis>`, `c<axis>`, `d<axis>`.
fn coefficients(el: &Element, axis: &str) -> Result<[f64; 4], OpenDriveError> {
    let mut out = [0.0; 4];
    for (i, prefix) in ["a", "b", "c", "d"].iter().enumerate() {
        out[i] = num(el, &alloc::format!("{prefix}{axis}"))?.unwrap_or(0.0);
    }
    Ok(out)
}

fn lane_section(el: &Element) -> Result<LaneSection, OpenDriveError> {
    let mut section = LaneSection {
        s: num(el, "s")?.unwrap_or(0.0),
        left: Vec::new(),
        center: Vec::new(),
        right: Vec::new(),
    };
    if let Some(side) = el.child("left") {
        for l in side.children_named("lane") {
            section.left.push(lane(l)?);
        }
    }
    if let Some(side) = el.child("center") {
        for l in side.children_named("lane") {
            section.center.push(lane(l)?);
        }
    }
    if let Some(side) = el.child("right") {
        for l in side.children_named("lane") {
            section.right.push(lane(l)?);
        }
    }
    Ok(section)
}

fn lane(el: &Element) -> Result<Lane, OpenDriveError> {
    let id = required(el, "id")?;
    let id: i32 = id
        .parse()
        .map_err(|_| ParseError::custom(el.offset, "lane id is not an integer"))?;
    let mut widths = Vec::new();
    for w in el.children_named("width") {
        widths.push(Width {
            s_offset: num(w, "sOffset")?.unwrap_or(0.0),
            a: num(w, "a")?.unwrap_or(0.0),
            b: num(w, "b")?.unwrap_or(0.0),
            c: num(w, "c")?.unwrap_or(0.0),
            d: num(w, "d")?.unwrap_or(0.0),
        });
    }
    Ok(Lane {
        id,
        kind: el.attr("type").unwrap_or("none").to_string(),
        level: matches!(el.attr("level"), Some("true" | "1")),
        widths,
    })
}

fn required<'a>(el: &'a Element, name: &str) -> Result<&'a str, OpenDriveError> {
    el.attr(name)
        .ok_or_else(|| ParseError::custom(el.offset, "required attribute is missing").into())
}

fn num(el: &Element, name: &str) -> Result<Option<f64>, OpenDriveError> {
    match el.attr(name) {
        None => Ok(None),
        Some(v) => v
            .trim()
            .parse::<f64>()
            .map(Some)
            .map_err(|_| ParseError::custom(el.offset, "expected a number").into()),
    }
}

fn num_u32(el: &Element, name: &str) -> Result<Option<u32>, OpenDriveError> {
    match el.attr(name) {
        None => Ok(None),
        Some(v) => v
            .trim()
            .parse::<u32>()
            .map(Some)
            .map_err(|_| ParseError::custom(el.offset, "expected an unsigned integer").into()),
    }
}
