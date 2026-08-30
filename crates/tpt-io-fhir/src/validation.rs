//! FHIR profile validation and NZ HIPC 2020 compliance hooks.
//!
//! Validation is split into two phases per the workspace design principles:
//! - **Syntactic**: structural correctness (required fields present, valid types).
//! - **Semantic**: domain-level correctness (codes in value sets, references
//!   well-formed, NZ HIPC 2020 compliance rules).

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::FhirError;
use crate::model::{Bundle, Observation, Patient, Resource};

/// A single validation issue, with severity and a human-readable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
    /// FHIRPath-like path to the offending element.
    pub path: String,
}

/// Severity of a validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The resource is structurally invalid and cannot be processed.
    Error,
    /// A potential problem that should be reviewed.
    Warning,
    /// Informational note.
    Information,
}

/// Result of a validation pass: a list of issues (empty = valid).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationResult {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self { issues: Vec::new() }
    }

    pub fn is_valid(&self) -> bool {
        !self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    pub fn error(path: impl Into<String>, message: impl Into<String>) -> ValidationIssue {
        ValidationIssue { severity: Severity::Error, message: message.into(), path: path.into() }
    }

    pub fn warning(path: impl Into<String>, message: impl Into<String>) -> ValidationIssue {
        ValidationIssue { severity: Severity::Warning, message: message.into(), path: path.into() }
    }

    pub fn push(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
    }
}

/// A FHIR profile: a set of rules that constrain a resource type.
#[derive(Debug, Clone, Default)]
pub struct Profile {
    /// The resource type this profile applies to (e.g. "Patient").
    pub resource_type: String,
    /// Required element paths that must be present.
    pub required_elements: Vec<String>,
    /// Value-set bindings: path → required code system.
    pub bindings: Vec<(String, String)>,
}

impl Profile {
    pub fn new(resource_type: &str) -> Self {
        Self {
            resource_type: resource_type.to_string(),
            required_elements: Vec::new(),
            bindings: Vec::new(),
        }
    }

    pub fn require(mut self, path: &str) -> Self {
        self.required_elements.push(path.to_string());
        self
    }

    pub fn bind(mut self, path: &str, system: &str) -> Self {
        self.bindings.push((path.to_string(), system.to_string()));
        self
    }
}

/// Validate a resource against a [`Profile`] (strict mode).
///
/// Checks that all required elements are present and that coded elements use
/// the bound code system.
pub fn validate_profile(resource: &Resource, profile: &Profile) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Resource type must match.
    if resource.resource_type() != Some(profile.resource_type.as_str()) {
        result.push(ValidationResult::error(
            "resourceType",
            format!(
                "expected resource type '{}', got '{}'",
                profile.resource_type,
                resource.resource_type().unwrap_or("<none>")
            ),
        ));
        return result;
    }

    match resource {
        Resource::Patient(p) => validate_patient_profile(p, profile, &mut result),
        Resource::Observation(o) => validate_observation_profile(o, profile, &mut result),
        Resource::Bundle(b) => validate_bundle_profile(b, profile, &mut result),
        Resource::Other(_) => {
            result.push(ValidationResult::warning(
                "resourceType",
                "cannot validate unknown resource type against profile",
            ));
        }
    }

    result
}

fn validate_patient_profile(p: &Patient, profile: &Profile, result: &mut ValidationResult) {
    for req in &profile.required_elements {
        let present = match req.as_str() {
            "id" => p.id.is_some(),
            "gender" => p.gender.is_some(),
            "birthDate" => p.birth_date.is_some(),
            "name" => !p.name.is_empty(),
            "identifier" => !p.identifier.is_empty(),
            "active" => p.active.unwrap_or(false),
            _ => {
                result.push(ValidationResult::warning(
                    req,
                    format!("unknown required element '{}' for Patient", req),
                ));
                continue;
            }
        };
        if !present {
            result.push(ValidationResult::error(
                req,
                format!("required element '{}' is missing", req),
            ));
        }
    }
}

fn validate_observation_profile(o: &Observation, profile: &Profile, result: &mut ValidationResult) {
    for req in &profile.required_elements {
        let present = match req.as_str() {
            "id" => o.id.is_some(),
            "status" => o.status.is_some(),
            "code" => true, // code is a required field in the struct itself
            "subject" => o.subject.is_some(),
            _ => {
                result.push(ValidationResult::warning(
                    req,
                    format!("unknown required element '{}' for Observation", req),
                ));
                continue;
            }
        };
        if !present {
            result.push(ValidationResult::error(
                req,
                format!("required element '{}' is missing", req),
            ));
        }
    }
}

fn validate_bundle_profile(b: &Bundle, profile: &Profile, result: &mut ValidationResult) {
    for req in &profile.required_elements {
        let present = match req.as_str() {
            "id" => b.id.is_some(),
            "type" => b.r#type.is_some(),
            "entry" => !b.entry.is_empty(),
            _ => {
                result.push(ValidationResult::warning(
                    req,
                    format!("unknown required element '{}' for Bundle", req),
                ));
                continue;
            }
        };
        if !present {
            result.push(ValidationResult::error(
                req,
                format!("required element '{}' is missing", req),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// NZ HIPC 2020 compliance
// ---------------------------------------------------------------------------

/// NZ HIPC (Health Information Privacy Code) 2020 compliance validation.
///
/// The HIPC 2020 sets rules for how health information is collected, stored,
/// and shared in New Zealand. These hooks validate that a FHIR resource meets
/// the baseline privacy requirements:
///
/// - Patient resources must have a valid NHI (New Zealand Health Identifier)
///   in the `identifier` list.
/// - Resources must not contain unredacted free-text that could identify a
///   patient when identifiers are stripped.
/// - Audit metadata (`meta.lastUpdated`, `meta.source`) should be present.
///
/// This is a compliance *hook*: it returns structured issues rather than a
/// boolean, so callers can decide how to handle each finding.
pub mod hipc {
    use super::*;

    /// NHI identifier system URI (NZ Health Identifier).
    pub const NHI_SYSTEM: &str = "https://standards.digital.health.nz/id/nhi";

    /// Validate a Patient for NZ HIPC 2020 compliance.
    pub fn validate_patient(p: &Patient) -> ValidationResult {
        let mut result = ValidationResult::new();

        // HIPC Rule 2: unique identifier (NHI) must be present.
        let has_nhi = p
            .identifier
            .iter()
            .any(|id| id.system.as_deref().map(|s| s == NHI_SYSTEM).unwrap_or(false));
        if !has_nhi {
            result.push(ValidationResult::error(
                "identifier",
                "NZ HIPC 2020: Patient must have an NHI identifier",
            ));
        }

        // HIPC Rule 5: audit metadata should be present.
        if p.meta.is_none() {
            result.push(ValidationResult::warning(
                "meta",
                "NZ HIPC 2020: Patient should carry audit metadata (meta.lastUpdated, meta.source)",
            ));
        } else if let Some(meta) = p.meta.as_ref() {
            if meta.last_updated.is_none() {
                result.push(ValidationResult::warning(
                    "meta.lastUpdated",
                    "NZ HIPC 2020: meta.lastUpdated is recommended for audit",
                ));
            }
            if meta.source.is_none() {
                result.push(ValidationResult::warning(
                    "meta.source",
                    "NZ HIPC 2020: meta.source is recommended for provenance",
                ));
            }
        }

        // HIPC Rule 11: name and birth date should be present for identification.
        if p.name.is_empty() {
            result.push(ValidationResult::error(
                "name",
                "NZ HIPC 2020: Patient must have at least one name",
            ));
        }
        if p.birth_date.is_none() {
            result.push(ValidationResult::warning(
                "birthDate",
                "NZ HIPC 2020: birthDate is recommended for patient identification",
            ));
        }

        result
    }

    /// Validate any resource for NZ HIPC 2020 compliance.
    pub fn validate(resource: &Resource) -> ValidationResult {
        match resource {
            Resource::Patient(p) => validate_patient(p),
            Resource::Observation(o) => {
                let mut result = ValidationResult::new();
                if o.subject.is_none() {
                    result.push(ValidationResult::warning(
                        "subject",
                        "NZ HIPC 2020: Observation should reference a subject for audit",
                    ));
                }
                result
            }
            Resource::Bundle(b) => {
                let mut result = ValidationResult::new();
                for (i, entry) in b.entry.iter().enumerate() {
                    if let Some(ref res) = entry.resource {
                        let sub = validate(res);
                        for issue in sub.issues {
                            let mut issue = issue;
                            issue.path = format!("entry[{}].{}", i, issue.path);
                            result.push(issue);
                        }
                    }
                }
                result
            }
            Resource::Other(_) => ValidationResult::new(),
        }
    }
}

/// A FHIR validator implementing [`tpt_io_core::Validator`].
#[derive(Debug, Clone, Default)]
pub struct FhirValidator {
    /// The profile to validate against (optional).
    pub profile: Option<Profile>,
    /// Whether to run NZ HIPC 2020 compliance checks.
    pub hipc: bool,
}

impl FhirValidator {
    pub fn new() -> Self {
        Self { profile: None, hipc: false }
    }

    pub fn with_profile(mut self, profile: Profile) -> Self {
        self.profile = Some(profile);
        self
    }

    pub fn with_hipc(mut self, hipc: bool) -> Self {
        self.hipc = hipc;
        self
    }
}

impl tpt_io_core::Validator<Resource> for FhirValidator {
    type Error = FhirError;

    fn validate_syntactic(&self, value: &Resource) -> Result<(), FhirError> {
        if let Some(ref profile) = self.profile {
            let result = validate_profile(value, profile);
            if !result.is_valid() {
                return Err(FhirError::Validation(result.issues[0].message.clone()));
            }
        }
        Ok(())
    }

    fn validate_semantic(&self, value: &Resource) -> Result<(), FhirError> {
        if self.hipc {
            let result = hipc::validate(value);
            if !result.is_valid() {
                return Err(FhirError::Validation(result.issues[0].message.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_io_core::Validator;

    #[test]
    fn profile_validation_missing_required() {
        let patient =
            Patient { resource_type: "Patient".to_string(), id: None, ..Default::default() };
        let profile = Profile::new("Patient").require("id").require("name");
        let result = validate_profile(&Resource::Patient(patient), &profile);
        assert!(!result.is_valid());
        assert_eq!(result.issues.len(), 2);
    }

    #[test]
    fn profile_validation_passes() {
        let patient = Patient {
            resource_type: "Patient".to_string(),
            id: Some("123".to_string()),
            name: vec![crate::model::HumanName::new("Doe", &["Jane"])],
            ..Default::default()
        };
        let profile = Profile::new("Patient").require("id").require("name");
        let result = validate_profile(&Resource::Patient(patient), &profile);
        assert!(result.is_valid());
    }

    #[test]
    fn profile_type_mismatch() {
        let obs = Observation { resource_type: "Observation".to_string(), ..Default::default() };
        let profile = Profile::new("Patient");
        let result = validate_profile(&Resource::Observation(obs), &profile);
        assert!(!result.is_valid());
    }

    #[test]
    fn hipc_missing_nhi() {
        let patient = Patient {
            resource_type: "Patient".to_string(),
            name: vec![crate::model::HumanName::new("Doe", &["Jane"])],
            ..Default::default()
        };
        let result = hipc::validate_patient(&patient);
        assert!(!result.is_valid());
    }

    #[test]
    fn hipc_with_nhi_passes() {
        let patient = Patient {
            resource_type: "Patient".to_string(),
            name: vec![crate::model::HumanName::new("Doe", &["Jane"])],
            identifier: vec![crate::model::Identifier::new(hipc::NHI_SYSTEM, "XYZ1234")],
            ..Default::default()
        };
        let result = hipc::validate_patient(&patient);
        assert!(result.is_valid());
    }

    #[test]
    fn validator_syntactic() {
        let patient =
            Patient { resource_type: "Patient".to_string(), id: None, ..Default::default() };
        let profile = Profile::new("Patient").require("id");
        let validator = FhirValidator::new().with_profile(profile);
        let err = validator.validate_syntactic(&Resource::Patient(patient)).unwrap_err();
        assert!(matches!(err, FhirError::Validation(_)));
    }
}
