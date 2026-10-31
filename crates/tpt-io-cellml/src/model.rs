//! CellML model reader.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::CellMlError;

/// A `variable` of a component.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Variable {
    /// Variable name.
    pub name: String,
    /// The `units` name it is measured in.
    pub units: Option<String>,
    /// The `interface` attribute (`public`, `private`, `public_and_private`, `none`).
    pub interface: Option<String>,
}

/// A `component`: its variables and the number of MathML blocks it holds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Component {
    /// Component name.
    pub name: String,
    /// Variables in document order.
    pub variables: Vec<Variable>,
    /// The number of `math` blocks in the component.
    pub math_blocks: usize,
}

/// A `units` definition and its base unit terms.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnitDef {
    /// The defined unit name.
    pub name: String,
    /// The `(units, exponent)` terms in order; the exponent defaults to 1.
    pub terms: Vec<(String, i32)>,
}

/// A `connection`: a component pair and the variable pairs mapped between them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Connection {
    /// The first component name.
    pub component_1: String,
    /// The second component name.
    pub component_2: String,
    /// The `(variable_1, variable_2)` pairs.
    pub variables: Vec<(String, String)>,
}

/// A CellML model.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CellModel {
    /// Model name.
    pub name: String,
    /// Unit definitions in document order.
    pub units: Vec<UnitDef>,
    /// Components in document order.
    pub components: Vec<Component>,
    /// Connections in document order.
    pub connections: Vec<Connection>,
}

/// Parse a CellML 1.x model.
pub fn parse(input: &[u8]) -> Result<CellModel, CellMlError> {
    let root = xml::parse(input)?;
    if root.local_name() != "model" {
        return Err(ParseError::custom(root.offset, "root element is not <model>").into());
    }
    let name = root
        .attr("name")
        .ok_or_else(|| CellMlError::Invalid("model has no name".to_string()))?
        .to_string();
    let mut m = CellModel { name, ..CellModel::default() };
    for u in root.children_named("units") {
        m.units.push(unit_def(u));
    }
    for c in root.children_named("component") {
        m.components.push(component(c));
    }
    for c in root.children_named("connection") {
        m.connections.push(connection(c));
    }
    Ok(m)
}

fn unit_def(el: &Element) -> UnitDef {
    UnitDef {
        name: el.attr("name").unwrap_or("").to_string(),
        terms: el
            .children_named("unit")
            .map(|u| {
                let exponent = u.attr("exponent").and_then(|e| e.parse::<i32>().ok()).unwrap_or(1);
                (u.attr("units").unwrap_or("").to_string(), exponent)
            })
            .collect(),
    }
}

fn component(el: &Element) -> Component {
    Component {
        name: el.attr("name").unwrap_or("").to_string(),
        variables: el
            .children_named("variable")
            .map(|v| Variable {
                name: v.attr("name").unwrap_or("").to_string(),
                units: v.attr("units").map(|s| s.to_string()),
                interface: v.attr("interface").map(|s| s.to_string()),
            })
            .collect(),
        math_blocks: el.children_named("math").count(),
    }
}

fn connection(el: &Element) -> Connection {
    let map_components = el.child("map_components");
    let (c1, c2) = match map_components {
        Some(m) => (
            m.attr("component_1").unwrap_or("").to_string(),
            m.attr("component_2").unwrap_or("").to_string(),
        ),
        None => (String::new(), String::new()),
    };
    Connection {
        component_1: c1,
        component_2: c2,
        variables: el
            .children_named("map_variables")
            .map(|m| {
                (
                    m.attr("variable_1").unwrap_or("").to_string(),
                    m.attr("variable_2").unwrap_or("").to_string(),
                )
            })
            .collect(),
    }
}
