//! CAEX instance hierarchy and link reader.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element as XmlElement};
use tpt_io_core::ParseError;

use crate::error::AutomationMlError;

/// An attribute of an element, with its value text (from the `Value` child).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Attribute {
    /// Attribute name.
    pub name: String,
    /// The `AttributeDataType`, e.g. `xs:double`.
    pub data_type: Option<String>,
    /// The value text, if a `Value` child is present.
    pub value: Option<String>,
}

/// An `InternalElement`: a plant object with its attributes and nested objects.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Element {
    /// The element name.
    pub name: String,
    /// The element id.
    pub id: Option<String>,
    /// The `RefBaseSystemUnitPath`, the system-unit class this element was instantiated from.
    pub system_unit_class: Option<String>,
    /// Attributes in document order.
    pub attributes: Vec<Attribute>,
    /// Nested elements in document order.
    pub children: Vec<Element>,
}

/// An `InternalLink`: a connection between two element sides, written as `element:side`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Link {
    /// Link name.
    pub name: String,
    /// The first side reference.
    pub side_a: String,
    /// The second side reference.
    pub side_b: String,
}

/// A CAEX file: the top-level instance hierarchy elements and the links.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CaexFile {
    /// Top-level elements of every `InstanceHierarchy`, in document order.
    pub hierarchy: Vec<Element>,
    /// Internal links in document order.
    pub links: Vec<Link>,
}

/// Parse an AutomationML (CAEX) file.
pub fn parse(input: &[u8]) -> Result<CaexFile, AutomationMlError> {
    let root = xml::parse(input)?;
    if root.local_name() != "CAEXFile" {
        return Err(ParseError::custom(root.offset, "root element is not <CAEXFile>").into());
    }
    let mut file = CaexFile::default();
    for h in root.descendants_named("InstanceHierarchy") {
        for e in h.children_named("InternalElement") {
            file.hierarchy.push(element(e));
        }
    }
    for l in root.descendants_named("InternalLink") {
        file.links.push(Link {
            name: l.attr("Name").unwrap_or("").to_string(),
            side_a: l.attr("RefPartnerSideA").unwrap_or("").to_string(),
            side_b: l.attr("RefPartnerSideB").unwrap_or("").to_string(),
        });
    }
    Ok(file)
}

fn element(el: &XmlElement) -> Element {
    Element {
        name: el.attr("Name").unwrap_or("").to_string(),
        id: el.attr("ID").map(|s| s.to_string()),
        system_unit_class: el.attr("RefBaseSystemUnitPath").map(|s| s.to_string()),
        attributes: el
            .children_named("Attribute")
            .map(|a| Attribute {
                name: a.attr("Name").unwrap_or("").to_string(),
                data_type: a.attr("AttributeDataType").map(|s| s.to_string()),
                value: a.child("Value").map(|v| v.text_trimmed().to_string()),
            })
            .collect(),
        children: el.children_named("InternalElement").map(element).collect(),
    }
}
