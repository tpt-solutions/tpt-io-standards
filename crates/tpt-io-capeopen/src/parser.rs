//! Parsing of the canonical [`PropertyPackage`] text format produced by
//! [`crate::writer::encode_property_package`].
//!
//! The parser is a thin line/section scanner that reports byte offsets through
//! [`tpt_io_core::ParseError`]. [`PropertyPackageReader`] implements
//! [`tpt_io_core::Reader`] and applies the syntactic/semantic split described in
//! `spec.txt §4`.

use alloc::string::String;
use alloc::vec::Vec;

use tpt_io_core::{ParseError, Reader, ValidationSettings};

use crate::error::CapeOpenError;
use crate::model::{
    BaseProperty, Component, ComponentList, Phase, PropertyCalculationRequest, PropertyPackage,
};
use crate::writer::encode_property_package;

/// Split `s` on `delim` characters that are not escaped by a preceding `\`.
/// Backslash escapes the following character; the delimiter is only honored when
/// unescaped, so a field may itself contain the delimiter.
fn split_fields_escaped(s: &str, delim: char) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                cur.push(next);
            } else {
                cur.push('\\');
            }
        } else if c == delim {
            out.push(core::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out
}

fn unescape_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn opt_field(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Parse the canonical text format into a [`PropertyPackage`] without applying
/// semantic (`PropertyPackage::validate`) checks.
pub fn parse_property_package(input: &[u8]) -> Result<PropertyPackage, CapeOpenError> {
    let text = core::str::from_utf8(input)
        .map_err(|_| CapeOpenError::Core(ParseError::custom(0, "input is not valid UTF-8")))?;

    let mut name: Option<String> = None;
    let mut components: Vec<Component> = Vec::new();
    let mut phases: Vec<Phase> = Vec::new();
    let mut single: Vec<BaseProperty> = Vec::new();
    let mut two: Vec<BaseProperty> = Vec::new();
    let mut section = "";

    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let line_start = i;
        let mut j = i;
        while j < bytes.len() && bytes[j] != b'\n' {
            j += 1;
        }
        let mut line = &text[line_start..j];
        if line.ends_with('\r') {
            line = &line[..line.len() - 1];
        }
        let trimmed = line.trim();
        i = if j < bytes.len() { j + 1 } else { j };

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(sec) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = sec;
            continue;
        }

        let eq = match trimmed.find('=') {
            Some(p) => p,
            None => {
                return Err(CapeOpenError::Core(ParseError::token(line_start, Some("="), trimmed)))
            }
        };
        let key = trimmed[..eq].trim();
        let value = trimmed[eq + 1..].trim();

        match (section, key) {
            ("property-package", "name") => {
                name = Some(unescape_field(value));
            }
            ("components", "component") => {
                let f = split_fields_escaped(value, '|');
                let id = opt_field(f.first().map(|s| s.as_str()).unwrap_or(""));
                match id {
                    Some(id) => components.push(Component {
                        id,
                        description: f.get(1).and_then(|s| opt_field(s)),
                        cas_number: f.get(2).and_then(|s| opt_field(s)),
                        formula: f.get(3).and_then(|s| opt_field(s)),
                    }),
                    None => {
                        return Err(CapeOpenError::Core(ParseError::custom(
                            line_start,
                            "component line missing id",
                        )))
                    }
                }
            }
            ("phases", "phase") => phases.push(Phase::parse(value)),
            ("single-phase-properties", "property") => single.push(BaseProperty::parse(value)),
            ("two-phase-properties", "property") => two.push(BaseProperty::parse(value)),
            _ => { /* ignore unknown sections / keys */ }
        }
    }

    let name = name.ok_or_else(|| {
        CapeOpenError::PropertyPackage("property-package section missing 'name'".into())
    })?;

    Ok(PropertyPackage {
        name,
        components: ComponentList { components },
        supported_phases: phases,
        supported_single_phase: single,
        supported_two_phase: two,
    })
}

/// A [`tpt_io_core::Reader`] for [`PropertyPackage`].
///
/// Syntactic validation is the parse itself; semantic validation runs
/// [`PropertyPackage::validate`] when enabled via [`ValidationSettings`].
#[derive(Debug, Clone, Copy, Default)]
pub struct PropertyPackageReader {
    settings: ValidationSettings,
}

impl PropertyPackageReader {
    /// Create a reader with default (syntactic + semantic) validation.
    pub fn new() -> Self {
        PropertyPackageReader { settings: ValidationSettings::new() }
    }

    /// Builder: replace the validation settings.
    pub fn with_validation_settings(mut self, settings: ValidationSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Builder: toggle syntactic validation.
    pub fn with_syntactic_validation(mut self, enabled: bool) -> Self {
        self.settings = self.settings.with_syntactic_validation(enabled);
        self
    }

    /// Builder: toggle semantic validation.
    pub fn with_semantic_validation(mut self, enabled: bool) -> Self {
        self.settings = self.settings.with_semantic_validation(enabled);
        self
    }
}

impl Reader<PropertyPackage> for PropertyPackageReader {
    type Error = CapeOpenError;

    fn parse(&self, input: &[u8]) -> Result<PropertyPackage, CapeOpenError> {
        let pp = parse_property_package(input)?;
        if self.settings.is_semantic() {
            pp.validate()?;
        }
        Ok(pp)
    }
}

/// Convenience: round-trip a package through its canonical text form.
pub fn round_trip(pp: &PropertyPackage) -> Result<PropertyPackage, CapeOpenError> {
    parse_property_package(encode_property_package(pp).as_bytes())
}

/// Convenience: build a single-phase [`PropertyCalculationRequest`] from a
/// package's supported single-phase properties.
pub fn single_phase_request(pp: &PropertyPackage, phase: Phase) -> PropertyCalculationRequest {
    PropertyCalculationRequest::new(
        crate::model::CalculationKind::SinglePhase,
        alloc::vec![phase],
        pp.supported_single_phase.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ComponentList;

    #[test]
    fn escaped_delimiter_survives_round_trip() {
        let mut pp = PropertyPackage::new("PP", ComponentList::from_ids(&["WATER"]));
        pp.components.components[0].description = Some("a|b\\c".into());
        pp.supported_phases.push(Phase::Vapor);
        let back = parse_property_package(encode_property_package(&pp).as_bytes()).unwrap();
        assert_eq!(back.components.components[0].description, Some("a|b\\c".into()));
    }

    #[test]
    fn missing_name_errors() {
        assert!(parse_property_package(b"[property-package]\n").is_err());
    }

    #[test]
    fn unknown_section_is_ignored() {
        let src = b"[property-package]\nname = X\n[weird]\nfoo = bar\n[components]\ncomponent = A|||\n[phases]\nphase = Vapor\n";
        let pp = parse_property_package(src).unwrap();
        assert_eq!(pp.name, "X");
        assert_eq!(pp.components.len(), 1);
    }
}
