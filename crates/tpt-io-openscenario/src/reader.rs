//! Reader for OpenSCENARIO XML documents.

use alloc::string::ToString;
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::OpenScenarioError;
use crate::model::{Act, EntityKind, FileHeader, OpenScenario, Parameter, ScenarioObject, Story};

/// Parse an OpenSCENARIO (`<OpenSCENARIO>`) document.
pub fn parse(input: &[u8]) -> Result<OpenScenario, OpenScenarioError> {
    let root = xml::parse(input)?;
    if root.local_name() != "OpenSCENARIO" {
        return Err(ParseError::custom(root.offset, "root element is not <OpenSCENARIO>").into());
    }
    let mut s = OpenScenario::default();

    if let Some(h) = root.child("FileHeader") {
        s.header = FileHeader {
            rev_major: uint(h, "revMajor")?,
            rev_minor: uint(h, "revMinor")?,
            date: h.attr("date").unwrap_or("").to_string(),
            description: h.attr("description").unwrap_or("").to_string(),
            author: h.attr("author").unwrap_or("").to_string(),
        };
    }
    if let Some(decls) = root.child("ParameterDeclarations") {
        for p in decls.children_named("ParameterDeclaration") {
            s.parameters.push(Parameter {
                name: required(p, "name")?.to_string(),
                kind: p.attr("parameterType").unwrap_or("").to_string(),
                value: p.attr("value").unwrap_or("").to_string(),
            });
        }
    }
    if let Some(cats) = root.child("CatalogLocations") {
        for cat in &cats.children {
            for dir in cat.children_named("Directory") {
                let path = dir.attr("path").unwrap_or("");
                s.catalog_locations.push(alloc::format!("{}:{path}", cat.local_name()));
            }
        }
    }
    s.road_network = root
        .path(&["RoadNetwork", "LogicFile"])
        .and_then(|f| f.attr("filepath"))
        .map(|p| p.to_string());
    if let Some(ents) = root.child("Entities") {
        for so in ents.children_named("ScenarioObject") {
            s.entities.push(scenario_object(so)?);
        }
    }
    if let Some(board) = root.child("Storyboard") {
        for st in board.children_named("Story") {
            let mut acts = Vec::new();
            for a in st.children_named("Act") {
                acts.push(Act {
                    name: a.attr("name").unwrap_or("").to_string(),
                    maneuver_groups: a.children_named("ManeuverGroup").count(),
                });
            }
            s.stories.push(Story { name: required(st, "name")?.to_string(), acts });
        }
    }
    Ok(s)
}

fn scenario_object(el: &Element) -> Result<ScenarioObject, OpenScenarioError> {
    let name = required(el, "name")?.to_string();
    // The entity is the first element child of ScenarioObject; its tag gives the kind.
    let entity = el
        .children
        .first()
        .ok_or_else(|| ParseError::custom(el.offset, "ScenarioObject declares no entity"))?;
    let kind = match entity.local_name() {
        "Vehicle" => EntityKind::Vehicle,
        "Pedestrian" => EntityKind::Pedestrian,
        "MiscObject" => EntityKind::MiscObject,
        "ExternalObjectReference" => EntityKind::ExternalReference,
        other => EntityKind::Other(other.to_string()),
    };
    let category = entity
        .attr("vehicleCategory")
        .or_else(|| entity.attr("pedestrianCategory"))
        .or_else(|| entity.attr("miscObjectCategory"))
        .map(|c| c.to_string());
    Ok(ScenarioObject { name, kind, category })
}

fn required<'a>(el: &'a Element, name: &str) -> Result<&'a str, OpenScenarioError> {
    el.attr(name)
        .ok_or_else(|| ParseError::custom(el.offset, "required attribute is missing").into())
}

fn uint(el: &Element, name: &str) -> Result<u32, OpenScenarioError> {
    match el.attr(name) {
        None => Ok(0),
        Some(v) => v
            .trim()
            .parse::<u32>()
            .map_err(|_| ParseError::custom(el.offset, "expected an unsigned integer").into()),
    }
}
