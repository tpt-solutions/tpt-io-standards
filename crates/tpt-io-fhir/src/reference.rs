//! Reference resolution for cross-resource links.
//!
//! FHIR `Reference` elements point to other resources by relative URL, absolute
//! URL, or bare logical id. This module resolves references within a collection
//! of resources (typically a Bundle) and reports unresolvable ones.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::error::FhirError;
use crate::model::{Bundle, Patient, Reference, Resource};

/// An index of resources keyed by their relative FHIR URL (e.g.
/// `"Patient/123"`), built from a set of bundle entries or a flat list.
#[derive(Debug, Default)]
pub struct ResourceIndex {
    by_url: BTreeMap<String, Resource>,
}

impl ResourceIndex {
    /// Build an index from a list of `(url, resource)` pairs.
    pub fn from_pairs(pairs: Vec<(String, Resource)>) -> Self {
        let mut by_url = BTreeMap::new();
        for (url, res) in pairs {
            by_url.insert(url, res);
        }
        Self { by_url }
    }

    /// Build an index from a [`Bundle`], using each entry's `fullUrl` (or a
    /// generated `resourceType/id` fallback) as the key.
    pub fn from_bundle(bundle: &Bundle) -> Self {
        let mut by_url = BTreeMap::new();
        for entry in &bundle.entry {
            if let Some(ref res) = entry.resource {
                let key = entry.full_url.clone().or_else(|| {
                    Some(format!(
                        "{}/{}",
                        res.resource_type().unwrap_or(""),
                        res.id().unwrap_or("")
                    ))
                });
                if let Some(k) = key {
                    if !k.is_empty() {
                        by_url.insert(k, res.clone());
                    }
                }

                // Also index by resourceType/id.
                if let (Some(rt), Some(id)) = (res.resource_type(), res.id()) {
                    by_url.insert(format!("{rt}/{id}"), res.clone());
                }
            }
        }
        Self { by_url }
    }

    /// Look up a resource by its exact URL.
    pub fn get(&self, url: &str) -> Option<&Resource> {
        self.by_url.get(url)
    }

    /// All indexed URLs.
    pub fn urls(&self) -> Vec<&str> {
        self.by_url.keys().map(String::as_str).collect()
    }

    /// The number of indexed resources.
    pub fn len(&self) -> usize {
        self.by_url.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_url.is_empty()
    }
}

/// Resolve a single [`Reference`] against a [`ResourceIndex`].
///
/// Returns the referenced resource, or `None` if the reference is unresolvable.
pub fn resolve_reference(reference: &Reference, index: &ResourceIndex) -> Option<Resource> {
    let ref_str = reference.reference.as_ref()?;

    // 1. Direct URL lookup.
    if let Some(res) = index.get(ref_str) {
        return Some(res.clone());
    }

    // 2. Strip common prefixes.
    for prefix in &["urn:uuid:", "urn:oid:"] {
        if let Some(stripped) = ref_str.strip_prefix(prefix) {
            if let Some(res) = index.get(stripped) {
                return Some(res.clone());
            }
        }
    }

    None
}

/// Outcome of resolving all references in a resource tree.
#[derive(Debug, Default)]
pub struct ResolutionResult {
    /// Successfully resolved references: (from_path, target_resource).
    pub resolved: Vec<(String, Resource)>,
    /// References that could not be resolved: (from_path, reference_string).
    pub unresolved: Vec<(String, String)>,
}

impl ResolutionResult {
    pub fn is_complete(&self) -> bool {
        self.unresolved.is_empty()
    }
}

/// Resolve all references in a [`Bundle`], returning which were resolved and
/// which were not.
pub fn resolve_bundle_references(bundle: &Bundle) -> ResolutionResult {
    let index = ResourceIndex::from_bundle(bundle);
    let mut result = ResolutionResult::default();

    for (i, entry) in bundle.entry.iter().enumerate() {
        if let Some(ref res) = entry.resource {
            resolve_resource_refs(res, &index, &format!("entry[{}]", i), &mut result);
        }
    }

    result
}

fn resolve_resource_refs(
    resource: &Resource,
    index: &ResourceIndex,
    base_path: &str,
    result: &mut ResolutionResult,
) {
    match resource {
        Resource::Patient(p) => {
            for (gi, gp) in p.general_practitioner.iter().enumerate() {
                if let Some(resolved) = resolve_reference(gp, index) {
                    result
                        .resolved
                        .push((format!("{}.generalPractitioner[{}]", base_path, gi), resolved));
                } else if gp.reference.is_some() {
                    result.unresolved.push((
                        format!("{}.generalPractitioner[{}]", base_path, gi),
                        gp.reference.clone().unwrap(),
                    ));
                }
            }
            if let Some(ref mo) = p.managing_organization {
                if let Some(resolved) = resolve_reference(mo, index) {
                    result.resolved.push((format!("{}.managingOrganization", base_path), resolved));
                } else if mo.reference.is_some() {
                    result.unresolved.push((
                        format!("{}.managingOrganization", base_path),
                        mo.reference.clone().unwrap(),
                    ));
                }
            }
        }
        Resource::Observation(o) => {
            if let Some(ref s) = o.subject {
                if let Some(resolved) = resolve_reference(s, index) {
                    result.resolved.push((format!("{}.subject", base_path), resolved));
                } else if s.reference.is_some() {
                    result
                        .unresolved
                        .push((format!("{}.subject", base_path), s.reference.clone().unwrap()));
                }
            }
            for (pi, perf) in o.performer.iter().enumerate() {
                if let Some(resolved) = resolve_reference(perf, index) {
                    result.resolved.push((format!("{}.performer[{}]", base_path, pi), resolved));
                } else if perf.reference.is_some() {
                    result.unresolved.push((
                        format!("{}.performer[{}]", base_path, pi),
                        perf.reference.clone().unwrap(),
                    ));
                }
            }
        }
        Resource::Bundle(b) => {
            for (ei, entry) in b.entry.iter().enumerate() {
                if let Some(ref res) = entry.resource {
                    resolve_resource_refs(
                        res,
                        index,
                        &format!("{}.entry[{}]", base_path, ei),
                        result,
                    );
                }
            }
        }
        Resource::Other(_) => {}
    }
}

/// A patient-resolution helper: given a reference string, resolve it to a
/// [`Patient`] if possible.
pub fn resolve_patient(reference: &Reference, index: &ResourceIndex) -> Result<Patient, FhirError> {
    let res = resolve_reference(reference, index).ok_or_else(|| {
        FhirError::Reference(format!("unresolvable reference: {:?}", reference.reference))
    })?;
    match res {
        Resource::Patient(p) => Ok(p),
        _ => Err(FhirError::Reference("reference does not point to a Patient".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BundleEntry, Observation, Patient};

    fn make_bundle() -> Bundle {
        Bundle {
            resource_type: "Bundle".to_string(),
            r#type: Some(crate::model::BundleType::Collection),
            entry: vec![
                BundleEntry {
                    full_url: Some("Patient/1".to_string()),
                    resource: Some(Resource::Patient(Patient {
                        resource_type: "Patient".to_string(),
                        id: Some("1".to_string()),
                        ..Default::default()
                    })),
                    ..Default::default()
                },
                BundleEntry {
                    full_url: Some("Observation/100".to_string()),
                    resource: Some(Resource::Observation(Observation {
                        resource_type: "Observation".to_string(),
                        id: Some("100".to_string()),
                        subject: Some(Reference::new("Patient/1")),
                        code: crate::model::CodeableConcept::with_text("bp"),
                        ..Default::default()
                    })),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn index_from_bundle() {
        let bundle = make_bundle();
        let index = ResourceIndex::from_bundle(&bundle);
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn resolve_bundle() {
        let bundle = make_bundle();
        let result = resolve_bundle_references(&bundle);
        assert!(result.is_complete());
        assert_eq!(result.resolved.len(), 1);
    }

    #[test]
    fn unresolved_reference() {
        let bundle = Bundle {
            resource_type: "Bundle".to_string(),
            entry: vec![BundleEntry {
                resource: Some(Resource::Observation(Observation {
                    resource_type: "Observation".to_string(),
                    subject: Some(Reference::new("Patient/999")),
                    code: crate::model::CodeableConcept::with_text("bp"),
                    ..Default::default()
                })),
                ..Default::default()
            }],
            ..Default::default()
        };
        let result = resolve_bundle_references(&bundle);
        assert!(!result.is_complete());
        assert_eq!(result.unresolved.len(), 1);
    }

    #[test]
    fn resolve_patient_helper() {
        let bundle = make_bundle();
        let index = ResourceIndex::from_bundle(&bundle);
        let patient = resolve_patient(&Reference::new("Patient/1"), &index).unwrap();
        assert_eq!(patient.id.as_deref(), Some("1"));
    }

    #[test]
    fn resolve_patient_wrong_type() {
        let bundle = make_bundle();
        let index = ResourceIndex::from_bundle(&bundle);
        let err = resolve_patient(&Reference::new("Observation/100"), &index).unwrap_err();
        assert!(matches!(err, FhirError::Reference(_)));
    }
}
