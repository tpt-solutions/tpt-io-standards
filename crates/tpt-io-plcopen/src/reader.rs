//! Reader for PLCopen XML projects.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::PlcOpenError;
use crate::model::{Body, Language, Pou, PouKind, Project, Variable};

/// Parse a PLCopen `<project>` document.
pub fn parse(input: &[u8]) -> Result<Project, PlcOpenError> {
    let root = xml::parse(input)?;
    if root.local_name() != "project" {
        return Err(ParseError::custom(root.offset, "root element is not <project>").into());
    }
    let mut project = Project {
        name: root.path(&["contentHeader"]).and_then(|h| h.attr("name")).map(|s| s.to_string()),
        product: root
            .path(&["fileHeader"])
            .and_then(|h| h.attr("productName"))
            .map(|s| s.to_string()),
        ..Project::default()
    };
    if let Some(types) = root.child("types") {
        if let Some(dt) = types.child("dataTypes") {
            for d in dt.children_named("dataType") {
                if let Some(n) = d.attr("name") {
                    project.data_types.push(n.to_string());
                }
            }
        }
        if let Some(pous) = types.child("pous") {
            for p in pous.children_named("pou") {
                project.pous.push(pou(p)?);
            }
        }
    }
    Ok(project)
}

fn pou(el: &Element) -> Result<Pou, PlcOpenError> {
    let name = el
        .attr("name")
        .ok_or_else(|| ParseError::custom(el.offset, "POU has no name"))?
        .to_string();
    let kind = match el.attr("pouType") {
        Some("program") => PouKind::Program,
        Some("functionBlock") => PouKind::FunctionBlock,
        Some("function") => PouKind::Function,
        Some(other) => PouKind::Other(other.to_string()),
        None => PouKind::Other(String::new()),
    };
    let mut variables = Vec::new();
    if let Some(interface) = el.child("interface") {
        for section in &interface.children {
            for v in section.children_named("variable") {
                variables.push(variable(section.local_name(), v)?);
            }
        }
    }
    let body = el.child("body").and_then(|b| b.children.first()).map(body);
    Ok(Pou { name, kind, variables, body })
}

fn variable(class: &str, el: &Element) -> Result<Variable, PlcOpenError> {
    let name = el
        .attr("name")
        .ok_or_else(|| ParseError::custom(el.offset, "variable has no name"))?
        .to_string();
    let type_name = el
        .child("type")
        .and_then(|t| t.children.first())
        .map(|t| {
            if t.local_name() == "derived" {
                t.attr("name").unwrap_or("").to_string()
            } else {
                t.local_name().to_string()
            }
        })
        .unwrap_or_default();
    let initial = el
        .path(&["initialValue", "simpleValue"])
        .and_then(|v| v.attr("value"))
        .map(|s| s.to_string());
    Ok(Variable { class: class.to_string(), name, type_name, initial })
}

fn body(el: &Element) -> Body {
    let language = match el.local_name() {
        "ST" => Language::St,
        "IL" => Language::Il,
        "FBD" => Language::Fbd,
        "LD" => Language::Ld,
        "SFC" => Language::Sfc,
        other => Language::Other(other.to_string()),
    };
    let text = match language {
        Language::St | Language::Il => match el.child("xhtml") {
            Some(x) => x.text.clone(),
            None => el.text.clone(),
        },
        _ => String::new(),
    };
    Body { language, text }
}
