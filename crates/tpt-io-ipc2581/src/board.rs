//! IPC-2581 board model and reader.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::Ipc2581Error;

/// A layer from the CAD data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Layer {
    /// Layer name.
    pub name: String,
    /// The `layerFunction`, e.g. `CONDUCTOR` or `SOLDERMASK`.
    pub function: Option<String>,
    /// The `side`, e.g. `TOP` or `BOTTOM`.
    pub side: Option<String>,
}

/// A placed component.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Component {
    /// Reference designator, e.g. `R1`.
    pub ref_des: String,
    /// The package it refers to, if given.
    pub package_ref: Option<String>,
}

/// A board: its revision, units, layers, step names, components and logical nets.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ipc2581 {
    /// The `revision` attribute of the root, e.g. `B` or `C`.
    pub revision: String,
    /// The `units` of the CAD header, e.g. `MILLIMETER`.
    pub units: Option<String>,
    /// Layers in document order.
    pub layers: Vec<Layer>,
    /// Step (board or panel) names in document order.
    pub steps: Vec<String>,
    /// Components in document order.
    pub components: Vec<Component>,
    /// Logical net names in document order.
    pub nets: Vec<String>,
}

/// Parse an IPC-2581 document.
pub fn parse(input: &[u8]) -> Result<Ipc2581, Ipc2581Error> {
    let root = xml::parse(input)?;
    if root.local_name() != "IPC-2581" {
        return Err(ParseError::custom(root.offset, "root element is not <IPC-2581>").into());
    }
    let revision = root
        .attr("revision")
        .ok_or_else(|| Ipc2581Error::Invalid("no revision attribute".to_string()))?
        .to_string();
    let mut board = Ipc2581 { revision, ..Ipc2581::default() };
    board.units = root
        .descendants_named("CadHeader")
        .first()
        .and_then(|h| h.attr("units"))
        .map(|u| u.to_string());
    for layer in root.descendants_named("Layer") {
        board.layers.push(layer_of(layer));
    }
    for step in root.descendants_named("Step") {
        board.steps.push(step.attr("name").unwrap_or("").to_string());
    }
    for c in root.descendants_named("Component") {
        board.components.push(Component {
            ref_des: c.attr("refDes").unwrap_or("").to_string(),
            package_ref: c.attr("packageRef").map(|s| s.to_string()),
        });
    }
    for n in root.descendants_named("LogicalNet") {
        board.nets.push(n.attr("name").unwrap_or("").to_string());
    }
    Ok(board)
}

fn layer_of(el: &Element) -> Layer {
    Layer {
        name: el.attr("name").unwrap_or("").to_string(),
        function: el.attr("layerFunction").map(|s| s.to_string()),
        side: el.attr("side").map(|s| s.to_string()),
    }
}
