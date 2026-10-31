//! AIXM feature extraction.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};

use crate::error::AixmError;

/// The kind of an AIXM feature, from its element name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureKind {
    /// `AirportHeliport`.
    AirportHeliport,
    /// `Navaid`.
    Navaid,
    /// `Airspace`.
    Airspace,
    /// `Route`.
    Route,
    /// Any other feature element, kept by name.
    Other(String),
}

/// One feature from the message.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// The feature kind.
    pub kind: FeatureKind,
    /// The `gml:id` attribute, if present.
    pub gml_id: Option<String>,
    /// The `designator` of the first time slice, if present.
    pub designator: Option<String>,
    /// The `name` of the first time slice, if present.
    pub name: Option<String>,
    /// Positions as `(latitude, longitude)` pairs, from `gml:pos` and `gml:posList`.
    pub positions: Vec<(f64, f64)>,
}

/// Read every feature in an AIXM message. Each feature is taken from its `hasMember` element.
pub fn parse(input: &[u8]) -> Result<Vec<Feature>, AixmError> {
    let root = xml::parse(input)?;
    let mut out = Vec::new();
    for member in root.children_named("hasMember") {
        for el in &member.children {
            out.push(feature(el)?);
        }
    }
    Ok(out)
}

fn feature(el: &Element) -> Result<Feature, AixmError> {
    let kind = match el.local_name() {
        "AirportHeliport" => FeatureKind::AirportHeliport,
        "Navaid" => FeatureKind::Navaid,
        "Airspace" => FeatureKind::Airspace,
        "Route" => FeatureKind::Route,
        other => FeatureKind::Other(other.to_string()),
    };
    // Only the first time slice is read; later ones carry the history.
    let slice = el.descendants_named("timeSlice").first().copied();
    let text_of = |name: &str| -> Option<String> {
        slice
            .and_then(|s| s.descendants_named(name).first().map(|e| e.text_trimmed().to_string()))
            .filter(|t| !t.is_empty())
    };
    let mut positions = Vec::new();
    for pos in el.descendants_named("pos") {
        positions.push(pair(pos.text_trimmed(), pos.offset)?);
    }
    for list in el.descendants_named("posList") {
        let nums = numbers(list.text_trimmed(), list.offset)?;
        if nums.len() % 2 != 0 {
            return Err(AixmError::Invalid("posList has an odd number of values".into()));
        }
        positions.extend(nums.chunks_exact(2).map(|c| (c[0], c[1])));
    }
    Ok(Feature {
        kind,
        gml_id: el.attr("id").map(|s| s.to_string()),
        designator: text_of("designator"),
        name: text_of("name"),
        positions,
    })
}

fn numbers(text: &str, at: usize) -> Result<Vec<f64>, AixmError> {
    text.split_whitespace()
        .map(|t| {
            t.parse::<f64>().map_err(|_| {
                AixmError::Core(tpt_io_core::ParseError::custom(at, "position is not a number"))
            })
        })
        .collect()
}

fn pair(text: &str, at: usize) -> Result<(f64, f64), AixmError> {
    match numbers(text, at)?.as_slice() {
        [lat, lon] => Ok((*lat, *lon)),
        _ => Err(AixmError::Invalid("gml:pos needs exactly two values".into())),
    }
}
