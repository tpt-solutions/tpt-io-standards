//! WITSML trajectory and station reader.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::WitsmlError;

/// One survey station: measured depth, inclination and azimuth, with the unit each was written in.
#[derive(Debug, Clone, PartialEq)]
pub struct Station {
    /// The station `uid`, if present.
    pub uid: Option<String>,
    /// Measured depth.
    pub md: f64,
    /// The `uom` of `md`, e.g. `m` or `ft`.
    pub md_uom: Option<String>,
    /// Inclination from vertical.
    pub incl: f64,
    /// Azimuth from north.
    pub azi: f64,
}

/// A well trajectory: its identity and stations in document order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Trajectory {
    /// The trajectory `uid`.
    pub uid: Option<String>,
    /// The `uidWell` reference.
    pub well: Option<String>,
    /// The `uidWellbore` reference.
    pub wellbore: Option<String>,
    /// The trajectory `name`.
    pub name: Option<String>,
    /// Survey stations in document order.
    pub stations: Vec<Station>,
}

/// Parse a WITSML trajectory document (one or more `trajectory` elements).
pub fn parse(input: &[u8]) -> Result<Vec<Trajectory>, WitsmlError> {
    let root = xml::parse(input)?;
    let mut out = Vec::new();
    for t in root.descendants_named("trajectory") {
        out.push(trajectory(t)?);
    }
    Ok(out)
}

fn trajectory(el: &Element) -> Result<Trajectory, WitsmlError> {
    let mut t = Trajectory {
        uid: el.attr("uid").map(|s| s.to_string()),
        well: el.attr("uidWell").map(|s| s.to_string()),
        wellbore: el.attr("uidWellbore").map(|s| s.to_string()),
        name: el.child("name").map(|n| n.text_trimmed().to_string()),
        stations: Vec::new(),
    };
    for s in el.children_named("trajectoryStation") {
        t.stations.push(station(s)?);
    }
    Ok(t)
}

fn station(el: &Element) -> Result<Station, WitsmlError> {
    let value = |name: &str| -> Result<f64, WitsmlError> {
        let child = el
            .child(name)
            .ok_or_else(|| WitsmlError::Invalid(alloc::format!("station has no {name}")))?;
        child
            .text_trimmed()
            .parse::<f64>()
            .map_err(|_| WitsmlError::Core(ParseError::custom(child.offset, "expected a number")))
    };
    let md_el = el.child("md");
    Ok(Station {
        uid: el.attr("uid").map(|s| s.to_string()),
        md: value("md")?,
        md_uom: md_el.and_then(|m| m.attr("uom")).map(|s| s.to_string()),
        incl: value("incl")?,
        azi: value("azi")?,
    })
}
