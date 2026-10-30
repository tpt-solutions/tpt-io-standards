//! XML deserialization for FHIR resources.
//!
//! FHIR XML uses a `value` attribute pattern for primitives (e.g.
//! `<id value="example"/>`) and a "shallow" nesting that does not map directly
//! onto the JSON structure. This module converts FHIR XML into a
//! `serde_json::Value` using the standard FHIR XML↔JSON mapping (including
//! wrapping known repeating elements in arrays), then reuses the JSON dispatch
//! path.

use alloc::string::String;
use alloc::vec::Vec;
use quick_xml::events::Event;

use crate::error::FhirError;
use crate::model::{Bundle, Observation, Patient, Resource};

/// Read the root element name from an XML string.
fn root_element_name(xml: &str) -> Result<String, FhirError> {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                return Ok(String::from_utf8_lossy(e.name().as_ref()).to_string());
            }
            Ok(Event::Empty(e)) => {
                return Ok(String::from_utf8_lossy(e.name().as_ref()).to_string());
            }
            Ok(Event::Eof) => return Err(FhirError::Xml("empty XML document".to_string())),
            Err(e) => return Err(FhirError::Xml(e.to_string())),
            _ => {}
        }
    }
}

/// Parse a single FHIR XML resource into the generic [`Resource`] enum.
pub fn parse_resource(xml: &str) -> Result<Resource, FhirError> {
    let root = root_element_name(xml)?;
    match root.as_str() {
        "Patient" => {
            let p = parse_patient(xml)?;
            Ok(Resource::Patient(p))
        }
        "Observation" => {
            let o = parse_observation(xml)?;
            Ok(Resource::Observation(o))
        }
        "Bundle" => {
            let b = parse_bundle(xml)?;
            Ok(Resource::Bundle(b))
        }
        _ => {
            let mut value = xml_to_json_value(xml)?;
            // Inject the resource type from the root element name.
            if let serde_json::Value::Object(ref mut map) = value {
                map.entry("resourceType".to_string())
                    .or_insert_with(|| serde_json::Value::String(root.clone()));
            }
            Ok(Resource::Other(value))
        }
    }
}

/// Insert `resourceType` from the root element name when the XML omits it,
/// so typed parses carry the same discriminator as the JSON form.
fn with_resource_type(mut value: serde_json::Value, kind: &str) -> serde_json::Value {
    if let serde_json::Value::Object(ref mut map) = value {
        map.entry("resourceType".to_string())
            .or_insert_with(|| serde_json::Value::String(kind.to_string()));
    }
    value
}

/// Parse a FHIR Patient from XML.
pub fn parse_patient(xml: &str) -> Result<Patient, FhirError> {
    let value = with_resource_type(xml_to_json_value(xml)?, "Patient");
    let value = wrap_all_repeating(&value);
    serde_json::from_value(value).map_err(|e| FhirError::Xml(e.to_string()))
}

/// Parse a FHIR Observation from XML.
pub fn parse_observation(xml: &str) -> Result<Observation, FhirError> {
    let value = with_resource_type(xml_to_json_value(xml)?, "Observation");
    let value = wrap_all_repeating(&value);
    serde_json::from_value(value).map_err(|e| FhirError::Xml(e.to_string()))
}

/// Parse a FHIR Bundle from XML.
pub fn parse_bundle(xml: &str) -> Result<Bundle, FhirError> {
    let value = with_resource_type(xml_to_json_value(xml)?, "Bundle");
    let value = wrap_all_repeating(&value);
    serde_json::from_value(value).map_err(|e| FhirError::Xml(e.to_string()))
}

/// Serialize a [`Resource`] to a FHIR XML string.
pub fn to_xml(resource: &Resource) -> Result<String, FhirError> {
    let mut value = match resource {
        Resource::Patient(p) => serde_json::to_value(p),
        Resource::Observation(o) => serde_json::to_value(o),
        Resource::Bundle(b) => serde_json::to_value(b),
        Resource::Other(v) => Ok(v.clone()),
    }
    .map_err(|e| FhirError::Xml(e.to_string()))?;

    // Ensure resourceType is present so the round-trip preserves the type.
    if let serde_json::Value::Object(ref mut map) = value {
        if !map.contains_key("resourceType") {
            let type_key = match resource {
                Resource::Patient(_) => "Patient",
                Resource::Observation(_) => "Observation",
                Resource::Bundle(_) => "Bundle",
                Resource::Other(_) => "Resource",
            };
            map.insert("resourceType".to_string(), serde_json::Value::String(type_key.to_string()));
        }
    }

    Ok(json_value_to_xml(&value))
}

/// A XML-backed FHIR reader implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct FhirXmlReader;

impl FhirXmlReader {
    pub fn new() -> Self {
        Self
    }
}

impl tpt_io_core::Reader<Resource> for FhirXmlReader {
    type Error = FhirError;

    fn parse(&self, input: &[u8]) -> Result<Resource, FhirError> {
        let s = core::str::from_utf8(input)
            .map_err(|_| FhirError::Xml("input is not valid UTF-8".to_string()))?;
        parse_resource(s)
    }
}

// ---------------------------------------------------------------------------
// FHIR repeating-element handling
// ---------------------------------------------------------------------------

/// The set of element names that are repeating (0..* or 1..*) in FHIR R4/R5.
/// These must always be arrays in the JSON representation, even when the XML
/// contains only a single instance.
const FHIR_REPEATING: &[&str] = &[
    // Patient
    "identifier",
    "name",
    "telecom",
    "address",
    "photo",
    "contact",
    "communication",
    "generalPractitioner",
    "link",
    // Observation
    "category",
    "performer",
    "interpretation",
    "note",
    "referenceRange",
    "component",
    "member",
    "hasMember",
    // Bundle
    "entry",
    // Shared data types
    "coding",
    "given",
    "prefix",
    "suffix",
    "line",
    "district",
    "period",
    "relationship",
    "qualifier",
    "tag",
    "profile",
    "designation",
    "property",
    "concept",
    "include",
    "exclude",
];

/// Wrap all known repeating fields in arrays recursively.
fn wrap_all_repeating(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if FHIR_REPEATING.contains(&k.as_str()) {
                    // Recurse first, then ensure the result is an array.
                    let inner = wrap_all_repeating(v);
                    let arr = match inner {
                        serde_json::Value::Array(a) => a,
                        serde_json::Value::Null => Vec::new(),
                        other => vec![other],
                    };
                    out.insert(k.clone(), serde_json::Value::Array(arr));
                } else {
                    out.insert(k.clone(), wrap_all_repeating(v));
                }
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(wrap_all_repeating).collect())
        }
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// FHIR XML → JSON value conversion
// ---------------------------------------------------------------------------

/// Convert a FHIR XML document into a `serde_json::Value`.
fn xml_to_json_value(xml: &str) -> Result<serde_json::Value, FhirError> {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut events = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(e) => events.push(e),
            Err(e) => return Err(FhirError::Xml(e.to_string())),
        }
    }

    let mut pos = 0;
    parse_element(&events, &mut pos)
}

/// Recursively parse XML events into a JSON value.
fn parse_element(events: &[Event], pos: &mut usize) -> Result<serde_json::Value, FhirError> {
    match events.get(*pos) {
        Some(Event::Start(e)) | Some(Event::Empty(e)) => {
            let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
            let is_empty = matches!(events.get(*pos), Some(Event::Empty(_)));

            let mut value_attr = None;
            let mut other_attrs = Vec::new();
            for attr in e.attributes() {
                let attr = attr.map_err(|e| FhirError::Xml(e.to_string()))?;
                let attr_name = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                let attr_val = String::from_utf8_lossy(&attr.value).to_string();
                if attr_name == "value" {
                    value_attr = Some(attr_val);
                } else if attr_name != "xmlns" {
                    other_attrs.push((attr_name, attr_val));
                }
            }

            *pos += 1;

            if is_empty {
                return build_value(&name, value_attr, &other_attrs, None);
            }

            // Parse children until the matching End event.
            let mut children: Vec<(String, serde_json::Value)> = Vec::new();
            let mut text_content = String::new();
            while *pos < events.len() {
                match &events[*pos] {
                    Event::End(_) => {
                        *pos += 1;
                        break;
                    }
                    Event::Text(t) => {
                        text_content.push_str(&t.unescape().unwrap_or_default());
                        *pos += 1;
                    }
                    Event::CData(t) => {
                        text_content.push_str(&String::from_utf8_lossy(t.as_ref()));
                        *pos += 1;
                    }
                    _ => {
                        let child_pos = *pos;
                        let child = parse_element(events, pos)?;
                        let child_name = events
                            .get(child_pos)
                            .and_then(|e| match e {
                                Event::Start(s) | Event::Empty(s) => {
                                    Some(String::from_utf8_lossy(s.name().as_ref()).to_string())
                                }
                                _ => None,
                            })
                            .unwrap_or_default();
                        children.push((child_name, child));
                    }
                }
            }

            let children_value = if children.is_empty() {
                if text_content.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::String(text_content))
                }
            } else {
                Some(group_children(children))
            };

            build_value(&name, value_attr, &other_attrs, children_value)
        }
        Some(Event::Text(t)) => {
            *pos += 1;
            Ok(serde_json::Value::String(t.unescape().unwrap_or_default().to_string()))
        }
        _ => {
            *pos += 1;
            Ok(serde_json::Value::Null)
        }
    }
}

/// Build a JSON value from collected parts of an XML element.
fn build_value(
    _name: &str,
    value_attr: Option<String>,
    other_attrs: &[(String, String)],
    children: Option<serde_json::Value>,
) -> Result<serde_json::Value, FhirError> {
    match (value_attr, children) {
        // Primitive with value attribute: <id value="example"/> → "example"
        (Some(v), None) => Ok(coerce_primitive(&v)),
        // Value attribute with children (extension-style).
        (Some(v), Some(c)) => {
            let mut obj = serde_json::Map::new();
            obj.insert("_value".to_string(), coerce_primitive(&v));
            if let serde_json::Value::Object(cmap) = c {
                for (k, v) in cmap {
                    obj.insert(k, v);
                }
            }
            Ok(serde_json::Value::Object(obj))
        }
        // No value attribute, has children: nested object.
        (None, Some(c)) => Ok(c),
        // No value, no children.
        (None, None) => {
            if other_attrs.is_empty() {
                Ok(serde_json::Value::Null)
            } else {
                let mut obj = serde_json::Map::new();
                for (k, v) in other_attrs {
                    obj.insert(k.clone(), coerce_primitive(v));
                }
                Ok(serde_json::Value::Object(obj))
            }
        }
    }
}

/// Coerce a string into a JSON value. Only booleans are coerced; numeric
/// strings are left as strings so that fields like `postalCode` (which are
/// string-typed in FHIR) are not incorrectly converted to numbers.
fn coerce_primitive(s: &str) -> serde_json::Value {
    if s == "true" {
        return serde_json::Value::Bool(true);
    }
    if s == "false" {
        return serde_json::Value::Bool(false);
    }
    serde_json::Value::String(s.to_string())
}

/// Group a flat list of (name, value) children into a JSON object, collecting
/// repeated names into arrays (per FHIR XML↔JSON rules).
fn group_children(children: Vec<(String, serde_json::Value)>) -> serde_json::Value {
    use alloc::collections::BTreeMap;

    let mut map: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for (name, value) in children {
        map.entry(name).or_default().push(value);
    }

    let mut obj = serde_json::Map::new();
    for (name, values) in map {
        if values.len() == 1 {
            obj.insert(name, values.into_iter().next().unwrap());
        } else {
            obj.insert(name, serde_json::Value::Array(values));
        }
    }
    serde_json::Value::Object(obj)
}

// ---------------------------------------------------------------------------
// JSON value → FHIR XML conversion (for serialization)
// ---------------------------------------------------------------------------

/// Convert a JSON value into a minimal FHIR XML string.
fn json_value_to_xml(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let resource_type =
                map.get("resourceType").and_then(|v| v.as_str()).unwrap_or("Resource");
            let mut out = format!("<{resource_type} xmlns=\"http://hl7.org/fhir\">");
            for (k, v) in map {
                if k == "resourceType" {
                    continue;
                }
                out.push_str(&json_field_to_xml(k, v));
            }
            out.push_str(&format!("</{resource_type}>"));
            out
        }
        _ => String::new(),
    }
}

fn json_field_to_xml(key: &str, value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => {
            format!("<{key} value=\"{s}\"/>")
        }
        serde_json::Value::Number(n) => {
            format!("<{key} value=\"{n}\"/>")
        }
        serde_json::Value::Bool(b) => {
            format!("<{key} value=\"{b}\"/>")
        }
        serde_json::Value::Null => String::new(),
        serde_json::Value::Array(arr) => arr.iter().map(|v| json_field_to_xml(key, v)).collect(),
        serde_json::Value::Object(obj) => {
            let mut out = format!("<{key}>");
            for (k, v) in obj {
                if k.starts_with('_') {
                    continue;
                }
                out.push_str(&json_field_to_xml(k, v));
            }
            out.push_str(&format!("</{key}>"));
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_patient_xml() {
        let xml = r#"<Patient xmlns="http://hl7.org/fhir">
            <id value="example"/>
            <gender value="male"/>
            <name>
                <family value="Chalmers"/>
                <given value="Peter"/>
                <given value="James"/>
            </name>
        </Patient>"#;
        let r = parse_resource(xml).unwrap();
        match r {
            Resource::Patient(p) => {
                assert_eq!(p.id.as_deref(), Some("example"));
                assert_eq!(p.gender.as_deref(), Some("male"));
                assert_eq!(p.name[0].family.as_deref(), Some("Chalmers"));
                assert_eq!(p.name[0].given, vec!["Peter", "James"]);
            }
            _ => panic!("expected Patient"),
        }
    }

    #[test]
    fn parse_observation_xml() {
        let xml = r#"<Observation xmlns="http://hl7.org/fhir">
            <id value="bp"/>
            <status value="final"/>
            <code>
                <coding>
                    <system value="http://loinc.org"/>
                    <code value="85354-9"/>
                </coding>
            </code>
            <valueQuantity>
                <value value="120"/>
                <unit value="mmHg"/>
            </valueQuantity>
        </Observation>"#;
        let r = parse_resource(xml).unwrap();
        match r {
            Resource::Observation(o) => {
                assert_eq!(o.id.as_deref(), Some("bp"));
                assert_eq!(o.status.as_deref(), Some("final"));
                assert_eq!(o.value_quantity.as_ref().unwrap().value, Some(120.0));
            }
            _ => panic!("expected Observation"),
        }
    }

    #[test]
    fn round_trip_patient_xml() {
        let xml = r#"<Patient xmlns="http://hl7.org/fhir"><id value="abc"/><gender value="female"/></Patient>"#;
        let p = parse_patient(xml).unwrap();
        let out = to_xml(&Resource::Patient(p.clone())).unwrap();
        let p2 = parse_patient(&out).unwrap();
        assert_eq!(p, p2);
    }

    #[test]
    fn unknown_root_element() {
        let xml = r#"<ImagingStudy xmlns="http://hl7.org/fhir"><id value="x"/></ImagingStudy>"#;
        let r = parse_resource(xml).unwrap();
        assert!(matches!(r, Resource::Other(_)));
        assert_eq!(r.resource_type(), Some("ImagingStudy"));
    }
}
