//! JSON deserialization for FHIR resources.
//!
//! Uses `serde_json` to parse a FHIR JSON representation into the strongly-typed
//! [`model`](crate::model) structs. The `Resource` enum dispatches on the
//! `resourceType` field.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::FhirError;
use crate::model::{Bundle, Observation, Patient, Resource};

/// Parse a single FHIR JSON resource into the generic [`Resource`] enum.
///
/// Dispatches on the `resourceType` field. Known kinds map to strongly-typed
/// variants; unknown kinds fall through to `Resource::Other` holding the raw
/// JSON value.
pub fn parse_resource(json: &str) -> Result<Resource, FhirError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| FhirError::Json(e.to_string()))?;
    let resource_type = value.get("resourceType").and_then(|v| v.as_str()).unwrap_or("");

    match resource_type {
        "Patient" => {
            let p: Patient =
                serde_json::from_value(value).map_err(|e| FhirError::Json(e.to_string()))?;
            Ok(Resource::Patient(p))
        }
        "Observation" => {
            let o: Observation =
                serde_json::from_value(value).map_err(|e| FhirError::Json(e.to_string()))?;
            Ok(Resource::Observation(o))
        }
        "Bundle" => {
            let b: Bundle =
                serde_json::from_value(value).map_err(|e| FhirError::Json(e.to_string()))?;
            Ok(Resource::Bundle(b))
        }
        _ => Ok(Resource::Other(value)),
    }
}

/// Parse a FHIR Patient from JSON.
pub fn parse_patient(json: &str) -> Result<Patient, FhirError> {
    serde_json::from_str(json).map_err(|e| FhirError::Json(e.to_string()))
}

/// Parse a FHIR Observation from JSON.
pub fn parse_observation(json: &str) -> Result<Observation, FhirError> {
    serde_json::from_str(json).map_err(|e| FhirError::Json(e.to_string()))
}

/// Parse a FHIR Bundle from JSON.
pub fn parse_bundle(json: &str) -> Result<Bundle, FhirError> {
    serde_json::from_str(json).map_err(|e| FhirError::Json(e.to_string()))
}

/// Parse a JSON array of FHIR resources, each into a [`Resource`].
pub fn parse_resource_array(json: &str) -> Result<Vec<Resource>, FhirError> {
    serde_json::from_str(json).map_err(|e| FhirError::Json(e.to_string()))
}

/// Serialize a [`Resource`] back to a JSON string.
pub fn to_json(resource: &Resource) -> Result<String, FhirError> {
    serde_json::to_string(resource).map_err(|e| FhirError::Json(e.to_string()))
}

/// Serialize a [`Resource`] to a pretty-printed JSON string.
pub fn to_json_pretty(resource: &Resource) -> Result<String, FhirError> {
    serde_json::to_string_pretty(resource).map_err(|e| FhirError::Json(e.to_string()))
}

/// Extract the `resourceType` field from a JSON resource string without full parsing.
pub fn resource_type(json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("resourceType").and_then(|x| x.as_str().map(String::from)))
}

/// A JSON-backed FHIR reader implementing [`tpt_io_core::Reader`].
#[derive(Debug, Clone, Default)]
pub struct FhirJsonReader;

impl FhirJsonReader {
    pub fn new() -> Self {
        Self
    }
}

impl tpt_io_core::Reader<Resource> for FhirJsonReader {
    type Error = FhirError;

    fn parse(&self, input: &[u8]) -> Result<Resource, FhirError> {
        let s = core::str::from_utf8(input)
            .map_err(|_| FhirError::Json("input is not valid UTF-8".to_string()))?;
        parse_resource(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_patient_json() {
        let json = r#"{
            "resourceType": "Patient",
            "id": "example",
            "gender": "male",
            "name": [{"family": "Chalmers", "given": ["Peter", "James"]}]
        }"#;
        let r = parse_resource(json).unwrap();
        match r {
            Resource::Patient(p) => {
                assert_eq!(p.id.as_deref(), Some("example"));
                assert_eq!(p.gender.as_deref(), Some("male"));
                assert_eq!(p.name.len(), 1);
                assert_eq!(p.name[0].family.as_deref(), Some("Chalmers"));
                assert_eq!(p.name[0].given, vec!["Peter", "James"]);
            }
            _ => panic!("expected Patient variant"),
        }
    }

    #[test]
    fn parse_observation_json() {
        let json = r#"{
            "resourceType": "Observation",
            "id": "bp",
            "status": "final",
            "code": {"coding": [{"system": "http://loinc.org", "code": "85354-9"}]},
            "valueQuantity": {"value": 120.0, "unit": "mmHg"}
        }"#;
        let r = parse_resource(json).unwrap();
        match r {
            Resource::Observation(o) => {
                assert_eq!(o.id.as_deref(), Some("bp"));
                assert_eq!(o.status.as_deref(), Some("final"));
                assert_eq!(o.value_quantity.as_ref().unwrap().value, Some(120.0));
            }
            _ => panic!("expected Observation variant"),
        }
    }

    #[test]
    fn parse_unknown_resource_type() {
        let json = r#"{"resourceType": "ImagingStudy", "id": "x"}"#;
        let r = parse_resource(json).unwrap();
        assert!(matches!(r, Resource::Other(_)));
        assert_eq!(r.resource_type(), Some("ImagingStudy"));
        assert_eq!(r.id(), Some("x"));
    }

    #[test]
    fn resource_type_helper() {
        let json = r#"{"resourceType": "Patient"}"#;
        assert_eq!(resource_type(json), Some("Patient".to_string()));
    }

    #[test]
    fn round_trip_patient() {
        let json = r#"{"resourceType":"Patient","id":"abc","gender":"female","name":[{"family":"Doe","given":["Jane"]}]}"#;
        let p = parse_patient(json).unwrap();
        let out = serde_json::to_string(&p).unwrap();
        let p2 = parse_patient(&out).unwrap();
        assert_eq!(p, p2);
    }
}
