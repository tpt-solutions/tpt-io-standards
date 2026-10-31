//! Composition reader for the openEHR RM JSON form.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::{parse_json, Json};

use crate::error::OpenEhrError;

/// A top-level entry of a composition's `content` list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Entry {
    /// The entry's `_type`, e.g. `OBSERVATION`, `EVALUATION`, `INSTRUCTION`, `ACTION`.
    pub kind: String,
    /// The entry name (`name.value`), if present.
    pub name: Option<String>,
    /// The archetype node id, if present.
    pub archetype_node_id: Option<String>,
}

/// The identity of a composition and its top-level entries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Composition {
    /// The composition name (`name.value`).
    pub name: Option<String>,
    /// The archetype id (`archetype_details.archetype_id.value`).
    pub archetype_id: Option<String>,
    /// The template id (`archetype_details.template_id.value`).
    pub template_id: Option<String>,
    /// The reference model version (`archetype_details.rm_version`).
    pub rm_version: Option<String>,
    /// The language code (`language.code_string`).
    pub language: Option<String>,
    /// The top-level entries in order.
    pub entries: Vec<Entry>,
}

/// Parse a composition in the openEHR RM JSON form.
pub fn parse(input: &[u8]) -> Result<Composition, OpenEhrError> {
    let root = parse_json(input)?;
    if root.get("_type").and_then(|t| t.as_str()) != Some("COMPOSITION") {
        return Err(OpenEhrError::Invalid("_type is not COMPOSITION".to_string()));
    }
    let text = |v: Option<&Json>| v.and_then(|x| x.as_str()).map(|s| s.to_string());
    let details = root.get("archetype_details");
    let mut c = Composition {
        name: text(root.get("name").and_then(|n| n.get("value"))),
        archetype_id: text(
            details.and_then(|d| d.get("archetype_id")).and_then(|a| a.get("value")),
        ),
        template_id: text(details.and_then(|d| d.get("template_id")).and_then(|t| t.get("value"))),
        rm_version: text(details.and_then(|d| d.get("rm_version"))),
        language: text(root.get("language").and_then(|l| l.get("code_string"))),
        entries: Vec::new(),
    };
    if let Some(content) = root.get("content").and_then(|c| c.as_array()) {
        for e in content {
            c.entries.push(Entry {
                kind: text(e.get("_type")).unwrap_or_default(),
                name: text(e.get("name").and_then(|n| n.get("value"))),
                archetype_node_id: text(e.get("archetype_node_id")),
            });
        }
    }
    Ok(c)
}
