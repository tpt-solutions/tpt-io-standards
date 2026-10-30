//! Common FHIR data types shared across resources.
//!
//! These are the building blocks referenced by every domain resource
//! (Identifier, HumanName, CodeableConcept, Reference, …). They follow the
//! FHIR R4/R5 base data-type definitions.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// A FHIR Reference(…) — a pointer to another resource, possibly absolute,
/// relative, or a bare identifier.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    #[serde(rename = "reference", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(default)]
    pub _serde_base: bool,
}

impl Reference {
    pub fn new(reference: &str) -> Self {
        Self {
            reference: Some(reference.to_string()),
            r#type: None,
            display: None,
            _serde_base: false,
        }
    }

    pub fn with_display(mut self, display: &str) -> Self {
        self.display = Some(display.to_string());
        self
    }
}

/// A human name with parts and a use context.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanName {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub given: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefix: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suffix: Vec<String>,
}

impl HumanName {
    pub fn new(family: &str, given: &[&str]) -> Self {
        Self {
            use_field: None,
            text: None,
            family: Some(family.to_string()),
            given: given.iter().map(|s| s.to_string()).collect(),
            prefix: Vec::new(),
            suffix: Vec::new(),
        }
    }
}

/// A contact point (phone, email, fax, …).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactPoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<i32>,
}

/// A postal address.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// A coded concept referencing a terminology system.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_selected: Option<bool>,
}

/// A concept that may include one or more `Coding`s plus free text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeableConcept {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coding: Vec<Coding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl CodeableConcept {
    pub fn with_text(text: &str) -> Self {
        Self { coding: Vec::new(), text: Some(text.to_string()) }
    }
}

/// An identifier with a system (namespace) and a value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<CodeableConcept>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl Identifier {
    pub fn new(system: &str, value: &str) -> Self {
        Self {
            use_field: None,
            r#type: None,
            system: Some(system.to_string()),
            value: Some(value.to_string()),
        }
    }
}

/// A measured or measurable amount with comparator and unit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quantity {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "deserialize_optional_number"
    )]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// A range of low/high `Quantity` values.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Range {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<Quantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<Quantity>,
}

/// A numerator/denominator pair of `Quantity` values.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ratio {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numerator: Option<Quantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denominator: Option<Quantity>,
}

/// A time range defined by optional start/end string (FHIR date-time).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Period {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

/// Metadata about a resource (version, last updated, source, profiles, tags).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profile: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tag: Vec<Coding>,
}

/// XHTML narrative, kept as a raw div string for round-trip fidelity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Narrative {
    pub status: String,
    #[serde(rename = "div")]
    pub div: String,
}

/// An extension with a URL and an optional typed value.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
    pub url: String,
    #[serde(rename = "valueString", skip_serializing_if = "Option::is_none")]
    pub value_string: Option<String>,
    #[serde(rename = "valueBoolean", skip_serializing_if = "Option::is_none")]
    pub value_boolean: Option<bool>,
    #[serde(rename = "valueInteger", skip_serializing_if = "Option::is_none")]
    pub value_integer: Option<i32>,
    #[serde(rename = "valueCode", skip_serializing_if = "Option::is_none")]
    pub value_code: Option<String>,
    #[serde(rename = "valueCodeableConcept", skip_serializing_if = "Option::is_none")]
    pub value_codeable_concept: Option<CodeableConcept>,
}

/// A patient's relationship to another person (contact).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationship: Vec<CodeableConcept>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<HumanName>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub telecom: Vec<ContactPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,
}

/// A digital signature on a FHIR resource.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Signature {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub r#type: Vec<CodeableConcept>,
    pub when: String,
    pub who: Reference,
    #[serde(rename = "data", skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// An attachment (base64 blob with metadata).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(rename = "data", skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

// ---------------------------------------------------------------------------
// Custom deserializers
// ---------------------------------------------------------------------------

/// Deserialize an optional `f64` that may be either a JSON number or a JSON
/// string containing a number (needed for FHIR XML→JSON conversion, where
/// all primitives arrive as strings).
pub fn deserialize_optional_number<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumberOrString {
        Number(f64),
        String(String),
    }

    match Option::<NumberOrString>::deserialize(deserializer)? {
        None => Ok(None),
        Some(NumberOrString::Number(n)) => Ok(Some(n)),
        Some(NumberOrString::String(s)) => {
            if s.is_empty() {
                Ok(None)
            } else {
                s.parse::<f64>()
                    .map(Some)
                    .map_err(|_| serde::de::Error::custom(format!("invalid number: {s}")))
            }
        }
    }
}
