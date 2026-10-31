//! Reader for LandXML documents.

use alloc::string::ToString;
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::LandXmlError;
use crate::model::{LandXml, Surface};

/// Parse a LandXML document.
pub fn parse(input: &[u8]) -> Result<LandXml, LandXmlError> {
    let root = xml::parse(input)?;
    if root.local_name() != "LandXML" {
        return Err(ParseError::custom(root.offset, "root element is not <LandXML>").into());
    }
    let mut land = LandXml::default();
    if let Some(units) = root.child("Units") {
        if let Some(unit_el) = units.children.first() {
            land.linear_unit = unit_el.attr("linearUnit").map(|s| s.to_string());
        }
    }
    land.project = root.child("Project").and_then(|p| p.attr("name")).map(|s| s.to_string());
    if let Some(surfaces) = root.child("Surfaces") {
        for s in surfaces.children_named("Surface") {
            land.surfaces.push(surface(s)?);
        }
    }
    if let Some(alignments) = root.child("Alignments") {
        for a in alignments.children_named("Alignment") {
            land.alignments.push(a.attr("name").unwrap_or("").to_string());
        }
    }
    Ok(land)
}

fn surface(el: &Element) -> Result<Surface, LandXmlError> {
    let mut s = Surface { name: el.attr("name").unwrap_or("").to_string(), ..Surface::default() };
    let Some(def) = el.child("Definition") else {
        return Ok(s);
    };
    if let Some(pnts) = def.child("Pnts") {
        for p in pnts.children_named("P") {
            let id: u32 = p
                .attr("id")
                .and_then(|i| i.parse().ok())
                .ok_or_else(|| LandXmlError::Invalid("point without a numeric id".to_string()))?;
            let v = numbers(p.text_trimmed(), p.offset)?;
            match v.as_slice() {
                [x, y, z] => s.points.push((id, [*x, *y, *z])),
                _ => return Err(LandXmlError::Invalid("point needs three coordinates".to_string())),
            }
        }
    }
    if let Some(faces) = def.child("Faces") {
        for f in faces.children_named("F") {
            let ids: Vec<u32> = f
                .text_trimmed()
                .split_whitespace()
                .map(|t| t.parse::<u32>().map_err(|_| LandXmlError::Invalid("face id is not an integer".to_string())))
                .collect::<Result<_, _>>()?;
            match ids.as_slice() {
                [a, b, c] => s.faces.push([*a, *b, *c]),
                _ => return Err(LandXmlError::Invalid("face needs three point ids".to_string())),
            }
        }
    }
    Ok(s)
}

fn numbers(text: &str, at: usize) -> Result<Vec<f64>, LandXmlError> {
    text.split_whitespace()
        .map(|t| t.parse::<f64>().map_err(|_| ParseError::custom(at, "expected a number").into()))
        .collect()
}
