//! Serialization of a [`PropertyPackage`] to the crate's canonical text format.
//!
//! CAPE-OPEN is a COM-based interoperability standard and has no on-disk file
//! format of its own. To give the pure-Rust model a stable, round-trippable
//! interchange representation, this module defines a small line-oriented text
//! codec. The output is canonical: re-parsing it yields a structurally identical
//! [`PropertyPackage`] (see the round-trip tests).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::error::CapeOpenError;
use crate::model::PropertyPackage;

/// Escape `|`, `\`, and newline so a field can never break the line/`|` framing.
fn escape_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

/// Reverse [`escape_field`].
#[cfg(test)]
fn unescape_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('|') => out.push('|'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn opt_field(o: &Option<String>) -> String {
    match o {
        Some(s) => escape_field(s),
        None => String::new(),
    }
}

/// Encode a [`PropertyPackage`] to the canonical text format.
pub fn encode_property_package(pp: &PropertyPackage) -> String {
    let mut out = String::new();
    out.push_str("[property-package]\n");
    out.push_str(&format!("name = {}\n", escape_field(&pp.name)));
    out.push('\n');

    out.push_str("[components]\n");
    for c in &pp.components.components {
        out.push_str(&format!(
            "component = {}|{}|{}|{}\n",
            escape_field(&c.id),
            opt_field(&c.description),
            opt_field(&c.cas_number),
            opt_field(&c.formula)
        ));
    }
    out.push('\n');

    out.push_str("[phases]\n");
    for p in &pp.supported_phases {
        out.push_str(&format!("phase = {}\n", p.as_str()));
    }
    out.push('\n');

    out.push_str("[single-phase-properties]\n");
    for p in &pp.supported_single_phase {
        out.push_str(&format!("property = {}\n", p.as_str()));
    }
    out.push('\n');

    out.push_str("[two-phase-properties]\n");
    for p in &pp.supported_two_phase {
        out.push_str(&format!("property = {}\n", p.as_str()));
    }
    out.push('\n');
    out
}

/// Serialize a [`PropertyPackage`] into owned bytes.
pub fn serialize(pp: &PropertyPackage) -> Result<Vec<u8>, CapeOpenError> {
    Ok(encode_property_package(pp).into_bytes())
}

#[cfg(any(feature = "alloc", feature = "std"))]
impl tpt_io_core::Writer<PropertyPackage> for PropertyPackageWriter {
    type Error = CapeOpenError;

    fn serialize(&self, value: &PropertyPackage) -> Result<Vec<u8>, CapeOpenError> {
        serialize(value)
    }
}

/// A [`tpt_io_core::Writer`] for [`PropertyPackage`].
#[derive(Debug, Clone, Copy, Default)]
pub struct PropertyPackageWriter;

impl PropertyPackageWriter {
    /// Create a new writer.
    pub fn new() -> Self {
        PropertyPackageWriter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ComponentList;

    #[test]
    fn field_escaping_round_trips() {
        let s = "a|b\\c\nd";
        assert_eq!(unescape_field(&escape_field(s)), s);
    }

    #[test]
    fn encode_and_serialize_agree() {
        let pp = PropertyPackage::new("PP", ComponentList::from_ids(&["WATER"]));
        assert_eq!(serialize(&pp).unwrap(), encode_property_package(&pp).into_bytes());
    }
}
