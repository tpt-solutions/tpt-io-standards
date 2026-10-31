//! Reader for ARXML documents.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::ArxmlError;
use crate::model::{ArxmlElement, ArxmlFile, Package};

/// Parse an AUTOSAR XML document.
pub fn parse(input: &[u8]) -> Result<ArxmlFile, ArxmlError> {
    let root = xml::parse(input)?;
    if root.local_name() != "AUTOSAR" {
        return Err(ParseError::custom(root.offset, "root element is not <AUTOSAR>").into());
    }
    let schema_version = root
        .attributes
        .iter()
        .find(|(k, _)| k == "xmlns")
        .and_then(|(_, v)| v.rsplit('/').next())
        .map(|s| s.to_string());

    let mut packages = Vec::new();
    if let Some(ars) = root.child("AR-PACKAGES") {
        for p in ars.children_named("AR-PACKAGE") {
            packages.push(package(p, "")?);
        }
    }
    Ok(ArxmlFile { schema_version, packages })
}

fn package(el: &Element, parent: &str) -> Result<Package, ArxmlError> {
    let short_name = short_name_of(el)?;
    let path = alloc::format!("{parent}/{short_name}");
    let mut elements = Vec::new();
    if let Some(list) = el.child("ELEMENTS") {
        for e in &list.children {
            elements.push(ArxmlElement {
                kind: e.local_name().to_string(),
                short_name: short_name_of(e)?,
                uuid: e.attr("UUID").map(|u| u.to_string()),
            });
        }
    }
    let mut packages = Vec::new();
    if let Some(sub) = el.child("AR-PACKAGES") {
        for p in sub.children_named("AR-PACKAGE") {
            packages.push(package(p, &path)?);
        }
    }
    Ok(Package {
        short_name,
        path,
        uuid: el.attr("UUID").map(|u| u.to_string()),
        elements,
        packages,
    })
}

fn short_name_of(el: &Element) -> Result<String, ArxmlError> {
    el.child("SHORT-NAME")
        .map(|s| s.text_trimmed().to_string())
        .ok_or_else(|| ParseError::custom(el.offset, "element has no SHORT-NAME").into())
}
