//! gbXML model and reader.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::GbXmlError;

/// A surface: its id, surface type, and the vertices of its planar polyloop.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Surface {
    /// The surface id.
    pub id: String,
    /// The `surfaceType` attribute, e.g. `ExteriorWall`.
    pub surface_type: String,
    /// Polyloop vertices as `[x, y, z]`.
    pub points: Vec<[f64; 3]>,
}

/// A space: its id and name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Space {
    /// The space id.
    pub id: String,
    /// The space name, if given.
    pub name: Option<String>,
}

/// A building with its spaces and surfaces.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Building {
    /// The building id.
    pub id: String,
    /// The `buildingType` attribute, if given.
    pub building_type: Option<String>,
    /// The building name, if given.
    pub name: Option<String>,
    /// Spaces in document order.
    pub spaces: Vec<Space>,
    /// Surfaces in document order.
    pub surfaces: Vec<Surface>,
}

/// A gbXML document: every building of the campus.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GbXml {
    /// The buildings in document order.
    pub buildings: Vec<Building>,
}

/// Parse a gbXML document.
pub fn parse(input: &[u8]) -> Result<GbXml, GbXmlError> {
    let root = xml::parse(input)?;
    if root.local_name() != "gbXML" {
        return Err(ParseError::custom(root.offset, "root element is not <gbXML>").into());
    }
    let mut out = GbXml::default();
    // Buildings may sit directly under the root or inside a Campus.
    let mut buildings: Vec<&Element> = root.descendants_named("Building");
    buildings.sort_by_key(|b| b.offset);
    for b in buildings {
        out.buildings.push(building(b)?);
    }
    Ok(out)
}

fn building(el: &Element) -> Result<Building, GbXmlError> {
    let id = el
        .attr("id")
        .ok_or_else(|| GbXmlError::Invalid("building has no id".to_string()))?
        .to_string();
    let mut b = Building {
        id,
        building_type: el.attr("buildingType").map(|s| s.to_string()),
        name: el.child("Name").map(|n| n.text_trimmed().to_string()),
        ..Building::default()
    };
    for s in el.children_named("Space") {
        b.spaces.push(Space {
            id: s.attr("id").unwrap_or("").to_string(),
            name: s.child("Name").map(|n| n.text_trimmed().to_string()),
        });
    }
    for s in el.children_named("Surface") {
        b.surfaces.push(surface(s)?);
    }
    Ok(b)
}

fn surface(el: &Element) -> Result<Surface, GbXmlError> {
    let mut out = Surface {
        id: el
            .attr("id")
            .ok_or_else(|| GbXmlError::Invalid("surface has no id".to_string()))?
            .to_string(),
        surface_type: el.attr("surfaceType").unwrap_or("").to_string(),
        points: Vec::new(),
    };
    for pl in el.descendants_named("PolyLoop") {
        for p in pl.children_named("CartesianPoint") {
            let coords: Vec<f64> = p
                .children_named("Coordinate")
                .map(|c| {
                    c.text_trimmed()
                        .parse::<f64>()
                        .map_err(|_| GbXmlError::Core(ParseError::custom(c.offset, "expected a number")))
                })
                .collect::<Result<_, _>>()?;
            match coords.as_slice() {
                [x, y, z] => out.points.push([*x, *y, *z]),
                _ => return Err(GbXmlError::Invalid("CartesianPoint needs three coordinates".to_string())),
            }
        }
    }
    Ok(out)
}
