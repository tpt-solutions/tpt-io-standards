//! FHIR resource models: Patient, Observation, Bundle, and a generic
//! `Resource` enum that can hold any resource kind.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use super::common::{
    Address, Attachment, CodeableConcept, Contact, ContactPoint, HumanName, Identifier, Meta,
    Narrative, Period, Quantity, Reference,
};

/// The set of resource kinds this crate models directly. Unknown kinds are
/// captured by `Resource::Other`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceKind {
    Patient,
    Observation,
    Bundle,
    Composition,
    Encounter,
    Organization,
    Practitioner,
    Condition,
    MedicationRequest,
    DiagnosticReport,
    #[serde(other)]
    Other,
}

/// A FHIR Patient resource.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Patient {
    #[serde(rename = "resourceType", default, skip_serializing_if = "String::is_empty")]
    pub resource_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<Narrative>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identifier: Vec<Identifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub name: Vec<HumanName>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub telecom: Vec<ContactPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deceased_boolean: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deceased_date_time: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub address: Vec<Address>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marital_status: Option<CodeableConcept>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub photo: Vec<Attachment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contact: Vec<Contact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub communication: Vec<PatientCommunication>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub general_practitioner: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managing_organization: Option<Reference>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatientCommunication {
    pub language: CodeableConcept,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred: Option<bool>,
}

/// A FHIR Observation resource.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    #[serde(rename = "resourceType", default, skip_serializing_if = "String::is_empty")]
    pub resource_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<Narrative>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identifier: Vec<Identifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub category: Vec<CodeableConcept>,
    pub code: CodeableConcept,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_date_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_period: Option<Period>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub performer: Vec<Reference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_quantity: Option<Quantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_codeable_concept: Option<CodeableConcept>,
    #[serde(rename = "valueString", skip_serializing_if = "Option::is_none")]
    pub value_string: Option<String>,
    #[serde(rename = "valueBoolean", skip_serializing_if = "Option::is_none")]
    pub value_boolean: Option<bool>,
    #[serde(rename = "valueInteger", skip_serializing_if = "Option::is_none")]
    pub value_integer: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_range: Option<super::common::Range>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_ratio: Option<super::common::Ratio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpretation: Option<Vec<CodeableConcept>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub component: Vec<ObservationComponent>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationComponent {
    pub code: CodeableConcept,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_quantity: Option<Quantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_codeable_concept: Option<CodeableConcept>,
    #[serde(rename = "valueString", skip_serializing_if = "Option::is_none")]
    pub value_string: Option<String>,
    #[serde(rename = "valueBoolean", skip_serializing_if = "Option::is_none")]
    pub value_boolean: Option<bool>,
    #[serde(rename = "valueInteger", skip_serializing_if = "Option::is_none")]
    pub value_integer: Option<i32>,
}

/// The type of a FHIR Bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum BundleType {
    Document,
    Message,
    Transaction,
    TransactionResponse,
    #[default]
    Batch,
    BatchResponse,
    History,
    Searchset,
    Collection,
}

/// A FHIR Bundle resource — a collection of resources with metadata.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bundle {
    #[serde(rename = "resourceType", default, skip_serializing_if = "String::is_empty")]
    pub resource_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Identifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<BundleType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub link: Vec<BundleLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entry: Vec<BundleEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<super::common::Signature>,
}

/// A pagination or relationship link on a Bundle.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleLink {
    pub relation: String,
    pub url: String,
}

/// A single entry in a Bundle, holding an optional resource plus request/response metadata.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleEntry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub link: Vec<BundleLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<Resource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<BundleSearch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<BundleRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<BundleResponse>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleRequest {
    pub method: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_none_match: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_modified_since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_match: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_none_exist: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Resource>,
}

/// A generic FHIR resource. Strongly-typed variants for the kinds this crate
/// models; everything else is captured as `Other` with its raw JSON retained.
///
/// Custom deserialization dispatches on the `resourceType` field.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum Resource {
    Patient(Patient),
    Observation(Observation),
    Bundle(Bundle),
    Other(serde_json::Value),
}

impl<'de> Deserialize<'de> for Resource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
        let resource_type = value.get("resourceType").and_then(|v| v.as_str()).unwrap_or("");
        match resource_type {
            "Patient" => {
                let p: Patient = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(Resource::Patient(p))
            }
            "Observation" => {
                let o: Observation =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(Resource::Observation(o))
            }
            "Bundle" => {
                let b: Bundle = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(Resource::Bundle(b))
            }
            _ => Ok(Resource::Other(value)),
        }
    }
}

impl Resource {
    /// The `resourceType` string of this resource, if present.
    pub fn resource_type(&self) -> Option<&str> {
        match self {
            Resource::Patient(p) => {
                if p.resource_type.is_empty() {
                    None
                } else {
                    Some(&p.resource_type)
                }
            }
            Resource::Observation(o) => {
                if o.resource_type.is_empty() {
                    None
                } else {
                    Some(&o.resource_type)
                }
            }
            Resource::Bundle(b) => {
                if b.resource_type.is_empty() {
                    None
                } else {
                    Some(&b.resource_type)
                }
            }
            Resource::Other(v) => v.get("resourceType").and_then(|x| x.as_str()),
        }
    }

    /// The `id` of this resource, if present.
    pub fn id(&self) -> Option<&str> {
        match self {
            Resource::Patient(p) => p.id.as_deref(),
            Resource::Observation(o) => o.id.as_deref(),
            Resource::Bundle(b) => b.id.as_deref(),
            Resource::Other(v) => v.get("id").and_then(|x| x.as_str()),
        }
    }
}
