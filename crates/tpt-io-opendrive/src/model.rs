//! The OpenDRIVE road-network model.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::OpenDriveError;

/// The `<header>` element.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    /// OpenDRIVE major revision, e.g. `1`.
    pub rev_major: u32,
    /// OpenDRIVE minor revision, e.g. `6`.
    pub rev_minor: u32,
    /// Map name.
    pub name: String,
}

/// A plan-view geometry segment.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// Start position along the road reference line, metres.
    pub s: f64,
    /// Start x coordinate, metres.
    pub x: f64,
    /// Start y coordinate, metres.
    pub y: f64,
    /// Start heading, radians.
    pub hdg: f64,
    /// Segment length, metres.
    pub length: f64,
    /// The segment shape.
    pub kind: GeometryKind,
}

/// The shape of a plan-view segment.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryKind {
    /// A straight line.
    Line,
    /// A circular arc with constant curvature.
    Arc { curvature: f64 },
    /// A clothoid (spiral) with curvature varying linearly from `curv_start` to `curv_end`.
    Spiral { curv_start: f64, curv_end: f64 },
    /// A cubic polynomial in local `u`, `v` (non-parametric form).
    Poly3 { a: f64, b: f64, c: f64, d: f64 },
    /// A parametric cubic polynomial with `u` and `v` coefficients (`[a, b, c, d]` each).
    ParamPoly3 { u: [f64; 4], v: [f64; 4] },
}

/// One width record for a lane: `width(s) = a + b·ds + c·ds² + d·ds³`, where `ds = s − sOffset`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Width {
    /// Offset of this record from the start of the lane section, metres.
    pub s_offset: f64,
    /// Constant term, metres.
    pub a: f64,
    /// Linear term.
    pub b: f64,
    /// Quadratic term.
    pub c: f64,
    /// Cubic term.
    pub d: f64,
}

/// A lane within a lane section.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    /// Lane id: positive on the left, negative on the right, zero for the centre lane.
    pub id: i32,
    /// Lane type, e.g. `driving`, `sidewalk`, `shoulder`, `none`.
    pub kind: String,
    /// Whether the lane is on the level (`level="true"`).
    pub level: bool,
    /// Width records in order along the section.
    pub widths: Vec<Width>,
}

/// A section of the road with a fixed set of lanes.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneSection {
    /// Start of the section along the reference line, metres.
    pub s: f64,
    /// Lanes on the left side (positive ids), in document order.
    pub left: Vec<Lane>,
    /// The centre lane (id 0).
    pub center: Vec<Lane>,
    /// Lanes on the right side (negative ids), in document order.
    pub right: Vec<Lane>,
}

/// A road: its reference line and lane sections.
#[derive(Debug, Clone, PartialEq)]
pub struct Road {
    /// Road id, unique within the map.
    pub id: String,
    /// Road name.
    pub name: String,
    /// Total length, metres.
    pub length: f64,
    /// Junction id, or `None` when the road is not part of a junction (`junction="-1"`).
    pub junction: Option<String>,
    /// Plan-view geometry segments in order.
    pub geometry: Vec<Geometry>,
    /// Lane sections in order.
    pub lane_sections: Vec<LaneSection>,
}

/// A whole OpenDRIVE map.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenDrive {
    /// The `<header>` element.
    pub header: Header,
    /// Roads in document order.
    pub roads: Vec<Road>,
}

impl OpenDrive {
    /// Check the rules a consumer depends on: unique road ids, geometry segments that join end to
    /// start (within 1 mm), lane sections that start at `s = 0` and increase, and unique lane ids
    /// within a section.
    pub fn validate(&self) -> Result<(), OpenDriveError> {
        let mut ids: Vec<&str> = self.roads.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(invalid("duplicate road id"));
        }
        for road in &self.roads {
            let mut expected_s = 0.0f64;
            for (i, g) in road.geometry.iter().enumerate() {
                if i == 0 && g.s != 0.0 {
                    return Err(invalid("first geometry segment must start at s = 0"));
                }
                if i > 0 && (g.s - expected_s).abs() > 1e-3 {
                    return Err(invalid("geometry segments leave a gap along the road"));
                }
                if !(g.length.is_finite() && g.length >= 0.0) {
                    return Err(invalid("geometry length must be finite and non-negative"));
                }
                expected_s = g.s + g.length;
            }
            let mut prev: Option<f64> = None;
            for ls in &road.lane_sections {
                if let Some(p) = prev {
                    if ls.s <= p {
                        return Err(invalid("lane sections must have increasing s"));
                    }
                } else if ls.s != 0.0 {
                    return Err(invalid("first lane section must start at s = 0"));
                }
                prev = Some(ls.s);
                let mut seen: Vec<i32> =
                    ls.left.iter().chain(&ls.center).chain(&ls.right).map(|l| l.id).collect();
                seen.sort_unstable();
                if seen.windows(2).any(|w| w[0] == w[1]) {
                    return Err(invalid("duplicate lane id in a lane section"));
                }
            }
        }
        Ok(())
    }
}

fn invalid(msg: &str) -> OpenDriveError {
    OpenDriveError::Invalid(msg.to_string())
}
