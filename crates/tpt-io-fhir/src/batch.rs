//! Batch import/export utilities for FHIR resources.
//!
//! Supports building and processing FHIR batch and transaction bundles,
//! which are the standard mechanism for bulk operations on a FHIR server.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::FhirError;
use crate::model::{Bundle, BundleEntry, BundleRequest, BundleType, Resource};

/// A batch operation to include in a batch bundle.
#[derive(Debug, Clone, PartialEq)]
pub enum BatchOperation {
    /// Create a resource (POST).
    Create { resource: Resource, full_url: Option<String> },
    /// Update a resource (PUT).
    Update { resource: Resource, full_url: String },
    /// Delete a resource.
    Delete { resource_type: String, id: String },
    /// Read a resource (GET).
    Read { resource_type: String, id: String },
    /// Search (GET with query).
    Search { resource_type: String, query: String },
}

/// Builder for a FHIR batch/transaction bundle.
#[derive(Debug, Clone, Default)]
pub struct BatchBuilder {
    bundle_type: BundleType,
    entries: Vec<BundleEntry>,
}

impl BatchBuilder {
    /// Create a new batch builder with the given bundle type.
    pub fn new(bundle_type: BundleType) -> Self {
        Self { bundle_type, entries: Vec::new() }
    }

    /// Create a batch builder for a `batch` bundle.
    pub fn batch() -> Self {
        Self::new(BundleType::Batch)
    }

    /// Create a batch builder for a `transaction` bundle.
    pub fn transaction() -> Self {
        Self::new(BundleType::Transaction)
    }

    /// Add an operation to the batch.
    pub fn push_operation(mut self, op: BatchOperation) -> Self {
        self.entries.push(operation_to_entry(op));
        self
    }

    /// Add a resource create operation.
    pub fn create(self, resource: Resource) -> Self {
        self.push_operation(BatchOperation::Create { resource, full_url: None })
    }

    /// Add a resource update operation.
    pub fn update(self, resource: Resource, full_url: &str) -> Self {
        self.push_operation(BatchOperation::Update { resource, full_url: full_url.to_string() })
    }

    /// Add a resource delete operation.
    pub fn delete(self, resource_type: &str, id: &str) -> Self {
        self.push_operation(BatchOperation::Delete {
            resource_type: resource_type.to_string(),
            id: id.to_string(),
        })
    }

    /// Build the [`Bundle`].
    pub fn build(self) -> Bundle {
        Bundle {
            resource_type: "Bundle".to_string(),
            r#type: Some(self.bundle_type),
            entry: self.entries,
            ..Default::default()
        }
    }

    /// Build and serialize to JSON.
    pub fn build_json(self) -> Result<String, FhirError> {
        let bundle = self.build();
        crate::json::to_json(&Resource::Bundle(bundle))
    }
}

fn operation_to_entry(op: BatchOperation) -> BundleEntry {
    match op {
        BatchOperation::Create { resource, full_url } => {
            let url = resource_type_of(&resource);
            BundleEntry {
                full_url,
                resource: Some(resource),
                request: Some(BundleRequest {
                    method: "POST".to_string(),
                    url,
                    ..Default::default()
                }),
                ..Default::default()
            }
        }
        BatchOperation::Update { resource, full_url } => BundleEntry {
            full_url: Some(full_url.clone()),
            resource: Some(resource),
            request: Some(BundleRequest {
                method: "PUT".to_string(),
                url: full_url,
                ..Default::default()
            }),
            ..Default::default()
        },
        BatchOperation::Delete { resource_type, id } => BundleEntry {
            request: Some(BundleRequest {
                method: "DELETE".to_string(),
                url: format!("{resource_type}/{id}"),
                ..Default::default()
            }),
            ..Default::default()
        },
        BatchOperation::Read { resource_type, id } => BundleEntry {
            request: Some(BundleRequest {
                method: "GET".to_string(),
                url: format!("{resource_type}/{id}"),
                ..Default::default()
            }),
            ..Default::default()
        },
        BatchOperation::Search { resource_type, query } => BundleEntry {
            request: Some(BundleRequest {
                method: "GET".to_string(),
                url: format!("{resource_type}?{query}"),
                ..Default::default()
            }),
            ..Default::default()
        },
    }
}

fn resource_type_of(resource: &Resource) -> String {
    resource.resource_type().unwrap_or("Resource").to_string()
}

/// A response entry parsed from a batch/transaction response bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchResponseEntry {
    /// HTTP status string (e.g. "200", "201").
    pub status: String,
    /// The location header value, if present.
    pub location: Option<String>,
    /// The response resource, if present.
    pub resource: Option<Resource>,
    /// The outcome (OperationOutcome resource), if present.
    pub outcome: Option<Resource>,
}

/// Parse a batch/transaction response bundle into individual response entries.
pub fn parse_batch_response(bundle: &Bundle) -> Result<Vec<BatchResponseEntry>, FhirError> {
    let mut entries = Vec::new();
    for entry in &bundle.entry {
        let response = entry
            .response
            .as_ref()
            .ok_or_else(|| FhirError::Batch("batch response entry missing response".to_string()))?;
        entries.push(BatchResponseEntry {
            status: response.status.clone(),
            location: response.location.clone(),
            resource: entry.resource.clone(),
            outcome: response.outcome.clone(),
        });
    }
    Ok(entries)
}

/// An import result: counts of succeeded and failed operations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportResult {
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<(usize, String)>,
}

impl ImportResult {
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }
}

/// Process a batch response and summarize the outcome.
pub fn summarize_batch_response(bundle: &Bundle) -> Result<ImportResult, FhirError> {
    let entries = parse_batch_response(bundle)?;
    let mut result = ImportResult::default();
    for (i, entry) in entries.iter().enumerate() {
        let status: u16 = entry.status.parse().unwrap_or(0);
        if (200..300).contains(&status) {
            result.succeeded += 1;
        } else {
            result.failed += 1;
            result.errors.push((i, format!("status {}", entry.status)));
        }
    }
    Ok(result)
}

/// Extract resources from a batch/transaction response (e.g. created resources
/// with their new IDs).
pub fn extract_response_resources(bundle: &Bundle) -> Vec<(String, Option<Resource>)> {
    bundle
        .entry
        .iter()
        .filter_map(|e| e.response.as_ref().map(|r| (r.status.clone(), e.resource.clone())))
        .collect()
}

/// A batch import entry for building request bundles from raw JSON resources.
#[derive(Debug, Clone)]
pub struct ImportEntry {
    pub method: String,
    pub url: String,
    pub json: String,
}

/// Build a batch bundle from a list of import entries.
pub fn build_import_bundle(entries: Vec<ImportEntry>) -> Result<Bundle, FhirError> {
    let mut bundle_entries = Vec::new();
    for entry in entries {
        let value: serde_json::Value =
            serde_json::from_str(&entry.json).map_err(|e| FhirError::Json(e.to_string()))?;
        let resource = crate::json::parse_resource(
            &serde_json::to_string(&value).map_err(|e| FhirError::Json(e.to_string()))?,
        )?;
        bundle_entries.push(BundleEntry {
            resource: Some(resource),
            request: Some(BundleRequest {
                method: entry.method,
                url: entry.url,
                ..Default::default()
            }),
            ..Default::default()
        });
    }
    Ok(Bundle {
        resource_type: "Bundle".to_string(),
        r#type: Some(BundleType::Batch),
        entry: bundle_entries,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BundleResponse;

    #[test]
    fn build_batch_bundle() {
        let patient = Resource::Patient(crate::model::Patient {
            resource_type: "Patient".to_string(),
            id: Some("123".to_string()),
            ..Default::default()
        });
        let bundle = BatchBuilder::batch()
            .create(patient.clone())
            .update(patient, "Patient/123")
            .delete("Patient", "456")
            .build();
        assert_eq!(bundle.r#type, Some(BundleType::Batch));
        assert_eq!(bundle.entry.len(), 3);
    }

    #[test]
    fn batch_request_methods() {
        let patient = Resource::Patient(crate::model::Patient {
            resource_type: "Patient".to_string(),
            ..Default::default()
        });
        let bundle = BatchBuilder::batch()
            .create(patient.clone())
            .update(patient, "Patient/123")
            .delete("Patient", "456")
            .build();
        assert_eq!(bundle.entry[0].request.as_ref().unwrap().method, "POST");
        assert_eq!(bundle.entry[1].request.as_ref().unwrap().method, "PUT");
        assert_eq!(bundle.entry[2].request.as_ref().unwrap().method, "DELETE");
    }

    #[test]
    fn parse_batch_response_test() {
        let response_bundle = Bundle {
            resource_type: "Bundle".to_string(),
            r#type: Some(BundleType::BatchResponse),
            entry: vec![
                BundleEntry {
                    response: Some(BundleResponse {
                        status: "201".to_string(),
                        location: Some("Patient/1".to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                BundleEntry {
                    response: Some(BundleResponse {
                        status: "404".to_string(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let entries = parse_batch_response(&response_bundle).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].status, "201");
        assert_eq!(entries[0].location.as_deref(), Some("Patient/1"));
    }

    #[test]
    fn summarize_response() {
        let response_bundle = Bundle {
            resource_type: "Bundle".to_string(),
            r#type: Some(BundleType::BatchResponse),
            entry: vec![
                BundleEntry {
                    response: Some(BundleResponse {
                        status: "201".to_string(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                BundleEntry {
                    response: Some(BundleResponse {
                        status: "404".to_string(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let summary = summarize_batch_response(&response_bundle).unwrap();
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 1);
        assert!(!summary.is_success());
    }

    #[test]
    fn build_import_bundle_test() {
        let entries = vec![ImportEntry {
            method: "POST".to_string(),
            url: "Patient".to_string(),
            json: r#"{"resourceType":"Patient","id":"abc"}"#.to_string(),
        }];
        let bundle = build_import_bundle(entries).unwrap();
        assert_eq!(bundle.entry.len(), 1);
        assert!(matches!(&bundle.entry[0].resource, Some(Resource::Patient(_))));
    }

    #[test]
    fn extract_response_resources_test() {
        let response_bundle = Bundle {
            resource_type: "Bundle".to_string(),
            entry: vec![BundleEntry {
                resource: Some(Resource::Patient(crate::model::Patient {
                    resource_type: "Patient".to_string(),
                    id: Some("1".to_string()),
                    ..Default::default()
                })),
                response: Some(BundleResponse { status: "201".to_string(), ..Default::default() }),
                ..Default::default()
            }],
            ..Default::default()
        };
        let resources = extract_response_resources(&response_bundle);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].0, "201");
    }
}
