//! Reader for OPC UA NodeSet2 XML.

use alloc::string::ToString;
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::OpcUaError;
use crate::model::{Node, NodeKind, NodeSet, Reference};

/// Parse a `UANodeSet` document.
pub fn parse(input: &[u8]) -> Result<NodeSet, OpcUaError> {
    let root = xml::parse(input)?;
    if root.local_name() != "UANodeSet" {
        return Err(ParseError::custom(root.offset, "root element is not <UANodeSet>").into());
    }
    let mut set = NodeSet::default();
    if let Some(ns) = root.child("NamespaceUris") {
        for u in ns.children_named("Uri") {
            set.namespace_uris.push(u.text_trimmed().to_string());
        }
    }
    if let Some(aliases) = root.child("Aliases") {
        for a in aliases.children_named("Alias") {
            let name = a.attr("Alias").unwrap_or("").to_string();
            set.aliases.push((name, a.text_trimmed().to_string()));
        }
    }
    for el in &root.children {
        let kind = match el.local_name() {
            "UAObject" => NodeKind::Object,
            "UAVariable" => NodeKind::Variable,
            "UAMethod" => NodeKind::Method,
            "UAObjectType" => NodeKind::ObjectType,
            "UAVariableType" => NodeKind::VariableType,
            "UADataType" => NodeKind::DataType,
            "UAReferenceType" => NodeKind::ReferenceType,
            "UAView" => NodeKind::View,
            _ => continue,
        };
        set.nodes.push(node(el, kind)?);
    }
    Ok(set)
}

fn node(el: &Element, kind: NodeKind) -> Result<Node, OpcUaError> {
    let node_id = el
        .attr("NodeId")
        .ok_or_else(|| ParseError::custom(el.offset, "node has no NodeId"))?
        .to_string();
    let browse_name = el.attr("BrowseName").unwrap_or("").to_string();
    let mut references = Vec::new();
    if let Some(list) = el.child("References") {
        for r in list.children_named("Reference") {
            references.push(Reference {
                reference_type: r.attr("ReferenceType").unwrap_or("").to_string(),
                is_forward: !matches!(r.attr("IsForward"), Some("false" | "0")),
                target: r.text_trimmed().to_string(),
            });
        }
    }
    Ok(Node {
        kind,
        node_id,
        browse_name,
        display_name: el.child("DisplayName").map(|d| d.text_trimmed().to_string()),
        data_type: el.attr("DataType").map(|s| s.to_string()),
        references,
    })
}
