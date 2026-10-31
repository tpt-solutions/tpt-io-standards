//! DEXPI component extraction.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};

use crate::error::DexpiError;

/// A P&ID object: its id, class, tag name and the ids of its nozzles.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Component {
    /// The element name, e.g. `Equipment`, `PipingNetworkSegment` or `ProcessInstrumentationFunction`.
    pub kind: String,
    /// The `ID` attribute.
    pub id: String,
    /// The `ComponentClass` attribute, e.g. `CentrifugalPump`.
    pub class: String,
    /// The `ComponentName` (tag) attribute, if given.
    pub tag: Option<String>,
    /// The `ID`s of the `Nozzle` children of this object.
    pub nozzles: Vec<String>,
}

/// A parsed P&ID: its components in document order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plant {
    /// Components in document order.
    pub components: Vec<Component>,
}

/// Parse a DEXPI document, collecting every element that carries an `ID` and a `ComponentClass`.
pub fn parse(input: &[u8]) -> Result<Plant, DexpiError> {
    let root = xml::parse(input)?;
    let mut plant = Plant::default();
    collect(&root, &mut plant);
    Ok(plant)
}

fn collect(el: &Element, plant: &mut Plant) {
    // Nozzles are references held by their owning object, so they are listed under `nozzles`.
    if el.local_name() != "Nozzle" {
        collect_component(el, plant);
    }
    for child in &el.children {
        collect(child, plant);
    }
}

fn collect_component(el: &Element, plant: &mut Plant) {
    if let (Some(id), Some(class)) = (el.attr("ID"), el.attr("ComponentClass")) {
        plant.components.push(Component {
            kind: el.local_name().to_string(),
            id: id.to_string(),
            class: class.to_string(),
            tag: el.attr("ComponentName").map(|s| s.to_string()),
            nozzles: el
                .descendants_named("Nozzle")
                .iter()
                .filter_map(|n| n.attr("ID").map(|s| s.to_string()))
                .collect(),
        });
    }
}
