//! Interpret parsed JSON into the FHIR model.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use tpt_io_core::json::{parse_json, Json};
use tpt_io_core::{ParseError, SpecReference};

use crate::error::FhirError;
use crate::model::{Bundle, BundleEntry, Fhir, Resource};

fn fhir_err(msg: impl ToString) -> FhirError {
    FhirError::Core(ParseError::custom(0, msg).with_spec(SpecReference::new("FHIR", "R4")))
}

fn require_object(j: &Json) -> Result<&JsonObject, FhirError> {
    match j {
        Json::Object(m) => Ok(m),
        _ => Err(fhir_err("FHIR document root must be a JSON object")),
    }
}

type JsonObject = alloc::collections::BTreeMap<String, Json>;

fn opt_string(obj: &JsonObject, key: &str) -> Result<Option<String>, FhirError> {
    match obj.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Str(s)) => Ok(Some(s.clone())),
        Some(other) => Err(fhir_err(alloc::format!(
            "member '{key}' must be a string, found {}",
            short_type(other)
        ))),
    }
}

fn short_type(j: &Json) -> &'static str {
    match j {
        Json::Null => "null",
        Json::Bool(_) => "bool",
        Json::Num(_) => "number",
        Json::Str(_) => "string",
        Json::Array(_) => "array",
        Json::Object(_) => "object",
    }
}

fn read_resource(obj: &JsonObject) -> Result<Resource, FhirError> {
    let resource_type = match obj.get("resourceType") {
        Some(Json::Str(s)) => s.clone(),
        _ => return Err(fhir_err("resource must have a string \"resourceType\"")),
    };
    let id = opt_string(obj, "id")?;
    let meta = match obj.get("meta") {
        Some(Json::Object(m)) => Some(Json::Object(m.clone())),
        Some(Json::Null) | None => None,
        Some(_) => return Err(fhir_err("\"meta\" must be an object")),
    };
    Ok(Resource { resource_type, id, meta, raw: Json::Object(obj.clone()) })
}

fn read_bundle(obj: &JsonObject) -> Result<Bundle, FhirError> {
    if obj.get("resourceType") != Some(&Json::Str("Bundle".to_string())) {
        return Err(fhir_err("bundle must have resourceType \"Bundle\""));
    }
    let id = opt_string(obj, "id")?;
    let entry = match obj.get("entry") {
        None | Some(Json::Null) => Vec::new(),
        Some(Json::Array(arr)) => {
            let mut entries = Vec::with_capacity(arr.len());
            for item in arr {
                let eobj = match item {
                    Json::Object(m) => m,
                    _ => return Err(fhir_err("bundle entries must be objects")),
                };
                let full_url = opt_string(eobj, "fullUrl")?;
                let resource = match eobj.get("resource") {
                    Some(Json::Object(m)) => read_resource(m)?,
                    _ => return Err(fhir_err("bundle entry missing \"resource\" object")),
                };
                let request = match eobj.get("request") {
                    Some(Json::Object(m)) => Some(Json::Object(m.clone())),
                    Some(Json::Null) | None => None,
                    Some(_) => return Err(fhir_err("\"request\" must be an object")),
                };
                let response = match eobj.get("response") {
                    Some(Json::Object(m)) => Some(Json::Object(m.clone())),
                    Some(Json::Null) | None => None,
                    Some(_) => return Err(fhir_err("\"response\" must be an object")),
                };
                entries.push(BundleEntry { full_url, resource, request, response });
            }
            entries
        }
        Some(_) => return Err(fhir_err("\"entry\" must be an array")),
    };
    Ok(Bundle { resource_type: "Bundle".to_string(), id, entry })
}

/// Parse a byte slice into a FHIR document.
///
/// The root `resourceType` dispatches: `"Bundle"` is parsed as a bundle; any
/// other type is treated as a single resource.
pub fn parse_fhir(input: &[u8]) -> Result<Fhir, FhirError> {
    let root = parse_json(input)?;
    let obj = require_object(&root)?;
    let type_name = match obj.get("resourceType") {
        Some(Json::Str(s)) => s.as_str(),
        _ => return Err(fhir_err("root must have a string \"resourceType\"")),
    };
    let doc = if type_name == "Bundle" {
        Fhir::Bundle(read_bundle(obj)?)
    } else {
        Fhir::Resource(read_resource(obj)?)
    };
    Ok(doc)
}

/// Reader for FHIR documents, implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct FhirReader {
    validation: tpt_io_core::ValidationSettings,
}

impl FhirReader {
    /// Create a reader with default validation.
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the validation configuration.
    pub fn with_validation(mut self, validation: tpt_io_core::ValidationSettings) -> Self {
        self.validation = validation;
        self
    }
}

impl tpt_io_core::Reader<Fhir> for FhirReader {
    type Error = FhirError;
    fn parse(&self, input: &[u8]) -> Result<Fhir, FhirError> {
        let doc = parse_fhir(input)?;
        self.validation.run(&FhirValidator, &doc)?;
        Ok(doc)
    }
}

/// Syntactic/semantic validation for a parsed [`Fhir`] document.
#[derive(Debug, Clone, Copy, Default)]
pub struct FhirValidator;

impl tpt_io_core::Validator<Fhir> for FhirValidator {
    type Error = FhirError;
    fn validate_syntactic(&self, value: &Fhir) -> Result<(), FhirError> {
        if let Fhir::Bundle(b) = value {
            for e in &b.entry {
                if e.resource.resource_type.is_empty() {
                    return Err(fhir_err("bundle entry resource missing resourceType"));
                }
            }
        }
        Ok(())
    }
    fn validate_semantic(&self, _value: &Fhir) -> Result<(), FhirError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_io_core::Reader;

    const BUNDLE: &str = r#"{
      "resourceType": "Bundle",
      "id": "example-bundle",
      "entry": [
        {
          "fullUrl": "urn:uuid:patient-1",
          "resource": {
            "resourceType": "Patient",
            "id": "patient-1",
            "name": [{ "family": "Smith", "given": ["John"] }]
          }
        },
        {
          "resource": {
            "resourceType": "Observation",
            "status": "final",
            "code": { "text": "Heart rate" }
          }
        }
      ]
    }"#;

    #[test]
    fn parses_bundle_with_resources() {
        let doc = parse_fhir(BUNDLE.as_bytes()).unwrap();
        let b = match doc {
            Fhir::Bundle(b) => b,
            _ => panic!("expected Bundle"),
        };
        assert_eq!(b.id.as_deref(), Some("example-bundle"));
        assert_eq!(b.len(), 2);
        assert_eq!(b.entry[0].resource.resource_type, "Patient");
        assert_eq!(b.entry[0].full_url.as_deref(), Some("urn:uuid:patient-1"));
        assert_eq!(b.entry[1].resource.resource_type, "Observation");
        // Resource-specific fields reachable through raw JSON.
        let name =
            b.entry[0].resource.field("name").and_then(|v| v.as_array()).and_then(|a| a.first());
        assert!(name.is_some());
        assert_eq!(
            b.entry[0]
                .resource
                .field("name")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|o| o.get("family"))
                .and_then(|v| v.as_str()),
            Some("Smith")
        );
    }

    #[test]
    fn parses_single_resource() {
        let doc = parse_fhir(br#"{"resourceType":"Patient","id":"x","active":true}"#).unwrap();
        match doc {
            Fhir::Resource(r) => {
                assert_eq!(r.resource_type, "Patient");
                assert_eq!(r.id.as_deref(), Some("x"));
                assert!(r.field("active").and_then(|v| v.as_bool()).unwrap_or(false));
            }
            _ => panic!("expected Resource"),
        }
    }

    #[test]
    fn rejects_non_object_root() {
        assert!(parse_fhir(b"[]").is_err());
        assert!(parse_fhir(br#"{"id":"x"}"#).is_err());
    }

    #[test]
    fn reader_runs_validation() {
        let r = FhirReader::new();
        let doc = r.parse(BUNDLE.as_bytes()).unwrap();
        assert!(matches!(doc, Fhir::Bundle(_)));
    }
}
