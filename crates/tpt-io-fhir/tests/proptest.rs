//! Property-based tests (proptest) for FHIR resource invariants.

use proptest::prelude::*;
use tpt_io_fhir::model::Patient;
use tpt_io_fhir::{json, xml, Resource};

/// Generate a valid FHIR Patient as JSON.
fn patient_json(id: &str, family: &str, given: &[&str]) -> String {
    let given_json: Vec<String> = given.iter().map(|g| format!("\"{g}\"")).collect();
    format!(
        r#"{{"resourceType":"Patient","id":"{id}","name":[{{"family":"{family}","given":[{}]}}]}}"#,
        given_json.join(",")
    )
}

proptest! {
    #[test]
    fn patient_json_round_trip(
        id in "[a-zA-Z0-9]{1,10}",
        family in "[A-Za-z]{1,20}",
        g1 in "[A-Za-z]{1,15}",
    ) {
        let json_str = patient_json(&id, &family, &[&g1]);
        let patient: Patient = json::parse_patient(&json_str).prop_parse()?;
        prop_assert_eq!(patient.id.as_deref(), Some(id.as_str()));
        prop_assert_eq!(patient.name[0].family.as_deref(), Some(family.as_str()));
        prop_assert_eq!(&patient.name[0].given, &[g1.clone()]);

        // Round-trip: serialize and re-parse.
        let serialized = serde_json::to_string(&patient).prop_parse()?;
        let patient2: Patient = json::parse_patient(&serialized).prop_parse()?;
        prop_assert_eq!(patient, patient2);
    }

    #[test]
    fn patient_xml_round_trip(
        id in "[a-zA-Z0-9]{1,10}",
        family in "[A-Za-z]{1,20}",
    ) {
        let xml_str = format!(
            r#"<Patient xmlns="http://hl7.org/fhir"><id value="{id}"/><name><family value="{family}"/></name></Patient>"#
        );
        let patient: Patient = xml::parse_patient(&xml_str).prop_parse()?;
        prop_assert_eq!(patient.id.as_deref(), Some(id.as_str()));
        prop_assert_eq!(patient.name[0].family.as_deref(), Some(family.as_str()));

        // Round-trip through XML.
        let serialized = xml::to_xml(&Resource::Patient(patient.clone())).prop_parse()?;
        let patient2: Patient = xml::parse_patient(&serialized).prop_parse()?;
        prop_assert_eq!(patient, patient2);
    }

    #[test]
    fn resource_type_preserved(
        id in "[a-zA-Z0-9]{1,10}",
    ) {
        let json_str = format!(r#"{{"resourceType":"Patient","id":"{id}"}}"#);
        let r = json::parse_resource(&json_str).prop_parse()?;
        prop_assert_eq!(r.resource_type(), Some("Patient"));
        prop_assert_eq!(r.id(), Some(id.as_str()));
    }

    #[test]
    fn unknown_resource_type_falls_to_other(
        rt in "[A-Z][a-zA-Z]{2,20}",
        id in "[a-zA-Z0-9]{1,10}",
    ) {
        // Skip known types — we want to test the "other" path.
        prop_assume!(rt != "Patient" && rt != "Observation" && rt != "Bundle");
        let json_str = format!(r#"{{"resourceType":"{rt}","id":"{id}"}}"#);
        let r = json::parse_resource(&json_str).prop_parse()?;
        prop_assert!(matches!(r, Resource::Other(_)));
        prop_assert_eq!(r.resource_type(), Some(rt.as_str()));
    }
}

/// Helper trait to convert serde errors into proptest errors.
trait PropParse<T> {
    fn prop_parse(self) -> Result<T, TestCaseError>;
}

impl<T> PropParse<T> for Result<T, serde_json::Error> {
    fn prop_parse(self) -> Result<T, TestCaseError> {
        self.map_err(|e| TestCaseError::fail(format!("serde error: {e}")))
    }
}

impl<T> PropParse<T> for Result<T, tpt_io_fhir::FhirError> {
    fn prop_parse(self) -> Result<T, TestCaseError> {
        self.map_err(|e| TestCaseError::fail(format!("fhir error: {e}")))
    }
}
