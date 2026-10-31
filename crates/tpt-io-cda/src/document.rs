//! CDA document identity, patient name, and section inventory.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::xml::{self, Element};
use tpt_io_core::ParseError;

use crate::error::CdaError;

/// A body section: its title and its LOINC-style code.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Section {
    /// The section title.
    pub title: Option<String>,
    /// The section `code` attribute (for example a LOINC code).
    pub code: Option<String>,
}

/// The identity and structure of a clinical document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClinicalDocument {
    /// The `id/@root` and `id/@extension`, joined as `root:extension` when both are present.
    pub id: Option<String>,
    /// The document title.
    pub title: Option<String>,
    /// The `effectiveTime/@value`, as written (usually `YYYYMMDDHHMMSS`).
    pub effective_time: Option<String>,
    /// The patient's given and family names joined by a space.
    pub patient: Option<String>,
    /// Body sections in document order.
    pub sections: Vec<Section>,
}

/// Parse a CDA `ClinicalDocument`.
pub fn parse(input: &[u8]) -> Result<ClinicalDocument, CdaError> {
    let root = xml::parse(input)?;
    if root.local_name() != "ClinicalDocument" {
        return Err(ParseError::custom(root.offset, "root element is not <ClinicalDocument>").into());
    }
    let mut doc = ClinicalDocument::default();
    if let Some(id) = root.child("id") {
        doc.id = match (id.attr("root"), id.attr("extension")) {
            (Some(r), Some(e)) => Some(alloc::format!("{r}:{e}")),
            (Some(r), None) => Some(r.to_string()),
            _ => None,
        };
    }
    doc.title = root.child("title").map(|t| t.text_trimmed().to_string());
    doc.effective_time = root.child("effectiveTime").and_then(|t| t.attr("value")).map(|v| v.to_string());
    doc.patient = root
        .path(&["recordTarget", "patientRole", "patient", "name"])
        .map(|n| {
            let given = n.child("given").map(|g| g.text_trimmed()).unwrap_or("");
            let family = n.child("family").map(|f| f.text_trimmed()).unwrap_or("");
            alloc::format!("{given} {family}").trim().to_string()
        })
        .filter(|s| !s.is_empty());
    for section in root.descendants_named("section") {
        doc.sections.push(section_of(section));
    }
    Ok(doc)
}

fn section_of(el: &Element) -> Section {
    Section {
        title: el.child("title").map(|t| t.text_trimmed().to_string()),
        code: el.child("code").and_then(|c| c.attr("code")).map(|c| c.to_string()),
    }
}
