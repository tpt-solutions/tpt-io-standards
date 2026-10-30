//! Feature-gated integration stub for the future `tpt-medical-patient` crate.
//!
//! `tpt-medical-patient` is not yet published (per `todo.md`). This module
//! provides only the conversion boundary — a stable API plus a local placeholder
//! type — so the `medical-patient-integration` feature flag compiles and links
//! today.

#[cfg(feature = "medical-patient-integration")]
pub mod medical_patient {
    //! Stub conversion into a future `tpt_medical_patient` patient type.
    use crate::model::Patient;
    use crate::Resource;

    /// Placeholder for a future `tpt_medical_patient::Patient`.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct PatientStub {
        /// Family name (first name found).
        pub family_name: Option<String>,
        /// Given names.
        pub given_names: Vec<String>,
        /// NHI identifier value.
        pub nhi: Option<String>,
        /// Gender string.
        pub gender: Option<String>,
        /// Birth date string.
        pub birth_date: Option<String>,
    }

    /// Convert a FHIR [`Patient`] into a patient placeholder.
    pub fn to_patient(patient: &Patient) -> PatientStub {
        let (family, given) = patient
            .name
            .first()
            .map(|n| (n.family.clone(), n.given.clone()))
            .unwrap_or((None, Vec::new()));

        let nhi = patient
            .identifier
            .iter()
            .find(|id| id.system.as_deref().map(|s| s.contains("nhi")).unwrap_or(false))
            .and_then(|id| id.value.clone());

        PatientStub {
            family_name: family,
            given_names: given,
            nhi,
            gender: patient.gender.clone(),
            birth_date: patient.birth_date.clone(),
        }
    }

    /// Convert a FHIR [`Resource`] into a patient placeholder if it is a Patient.
    pub fn resource_to_patient(resource: &Resource) -> Option<PatientStub> {
        match resource {
            Resource::Patient(p) => Some(to_patient(p)),
            _ => None,
        }
    }
}
