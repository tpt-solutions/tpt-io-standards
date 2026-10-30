//! Integration tests: round-trip, golden files, cross-format consistency.

use tpt_io_fhir::model::{HumanName, Patient, Resource};
use tpt_io_fhir::{json, xml, BundleCursor, BundleType, FhirError};

/// A canonical FHIR R4 Patient (JSON).
const PATIENT_JSON: &str = r#"{
    "resourceType": "Patient",
    "id": "example",
    "identifier": [
        {"system": "https://standards.digital.health.nz/id/nhi", "value": "XYZ1234"}
    ],
    "active": true,
    "name": [
        {"use": "official", "family": "Chalmers", "given": ["Peter", "James"]},
        {"use": "maiden", "family": "Windsor", "given": ["Peter", "James"]}
    ],
    "gender": "male",
    "birthDate": "1974-12-25",
    "address": [
        {"line": ["534 Erewhon St"], "city": "PleasantVille", "state": "Vic", "postalCode": "3999"}
    ]
}"#;

/// The same Patient in FHIR XML.
const PATIENT_XML: &str = r#"<Patient xmlns="http://hl7.org/fhir">
    <id value="example"/>
    <identifier>
        <system value="https://standards.digital.health.nz/id/nhi"/>
        <value value="XYZ1234"/>
    </identifier>
    <active value="true"/>
    <name>
        <use value="official"/>
        <family value="Chalmers"/>
        <given value="Peter"/>
        <given value="James"/>
    </name>
    <name>
        <use value="maiden"/>
        <family value="Windsor"/>
        <given value="Peter"/>
        <given value="James"/>
    </name>
    <gender value="male"/>
    <birthDate value="1974-12-25"/>
    <address>
        <line value="534 Erewhon St"/>
        <city value="PleasantVille"/>
        <state value="Vic"/>
        <postalCode value="3999"/>
    </address>
</Patient>"#;

#[test]
fn round_trip_json() {
    let patient: Patient = json::parse_patient(PATIENT_JSON).unwrap();
    let serialized = serde_json::to_string(&patient).unwrap();
    let patient2: Patient = json::parse_patient(&serialized).unwrap();
    assert_eq!(patient, patient2);
}

#[test]
fn round_trip_xml() {
    let patient: Patient = xml::parse_patient(PATIENT_XML).unwrap();
    let serialized = xml::to_xml(&Resource::Patient(patient.clone())).unwrap();
    let patient2: Patient = xml::parse_patient(&serialized).unwrap();
    assert_eq!(patient, patient2);
}

#[test]
fn cross_format_consistency() {
    // Parse the same patient from JSON and XML, compare the results.
    let from_json: Patient = json::parse_patient(PATIENT_JSON).unwrap();
    let from_xml: Patient = xml::parse_patient(PATIENT_XML).unwrap();
    assert_eq!(from_json.id, from_xml.id);
    assert_eq!(from_json.gender, from_xml.gender);
    assert_eq!(from_json.birth_date, from_xml.birth_date);
    assert_eq!(from_json.name.len(), from_xml.name.len());
    assert_eq!(from_json.name[0].family, from_xml.name[0].family);
    assert_eq!(from_json.name[0].given, from_xml.name[0].given);
    assert_eq!(from_json.identifier.len(), from_xml.identifier.len());
    assert_eq!(from_json.identifier[0].value, from_xml.identifier[0].value);
}

#[test]
fn parse_bundle_json() {
    let json_str = r#"{
        "resourceType": "Bundle",
        "id": "search-result",
        "type": "searchset",
        "total": 2,
        "link": [
            {"relation": "self", "url": "https://example.com/fhir/Patient?_page=1"},
            {"relation": "next", "url": "https://example.com/fhir/Patient?_page=2"}
        ],
        "entry": [
            {"fullUrl": "Patient/1", "resource": {"resourceType": "Patient", "id": "1"}},
            {"fullUrl": "Patient/2", "resource": {"resourceType": "Patient", "id": "2"}}
        ]
    }"#;
    let bundle = json::parse_bundle(json_str).unwrap();
    assert_eq!(bundle.r#type, Some(BundleType::Searchset));
    assert_eq!(bundle.total, Some(2));
    assert_eq!(bundle.entry.len(), 2);
    assert_eq!(bundle.link.len(), 2);
}

#[test]
fn bundle_cursor_pagination() {
    let json_str = r#"{
        "resourceType": "Bundle",
        "type": "searchset",
        "total": 100,
        "link": [
            {"relation": "self", "url": "Patient?_page=1"},
            {"relation": "next", "url": "Patient?_page=2"}
        ],
        "entry": []
    }"#;
    let cursor =
        BundleCursor::new(json::parse_bundle(json_str).unwrap(), "https://example.com/fhir");
    assert_eq!(cursor.total(), Some(100));
    assert!(cursor.has_next());
    assert_eq!(cursor.next_url(), Some("https://example.com/fhir/Patient?_page=2".to_string()));
}

#[test]
fn reference_resolution() {
    use tpt_io_fhir::reference::{resolve_bundle_references, ResourceIndex};

    let json_str = r#"{
        "resourceType": "Bundle",
        "type": "collection",
        "entry": [
            {"fullUrl": "Patient/1", "resource": {"resourceType": "Patient", "id": "1", "name": [{"family": "Doe"}]}},
            {"fullUrl": "Observation/100", "resource": {
                "resourceType": "Observation",
                "id": "100",
                "subject": {"reference": "Patient/1"},
                "code": {"text": "bp"}
            }}
        ]
    }"#;
    let bundle = json::parse_bundle(json_str).unwrap();
    let index = ResourceIndex::from_bundle(&bundle);
    assert_eq!(index.len(), 2);

    let result = resolve_bundle_references(&bundle);
    assert!(result.is_complete());
    assert_eq!(result.resolved.len(), 1);
}

#[test]
fn fhirpath_evaluation() {
    use tpt_io_fhir::fhirpath::evaluate_resource;

    let result = evaluate_resource("name.given", PATIENT_JSON).unwrap();
    assert!(result.is_collection());
}

#[test]
fn validation_profile() {
    use tpt_io_fhir::model::HumanName;
    use tpt_io_fhir::validation::{validate_profile, Profile};

    let patient = Patient {
        resource_type: "Patient".to_string(),
        id: Some("123".to_string()),
        name: vec![HumanName::new("Doe", &["Jane"])],
        ..Default::default()
    };
    let profile = Profile::new("Patient").require("id").require("name");
    let result = validate_profile(&Resource::Patient(patient), &profile);
    assert!(result.is_valid());
}

#[test]
fn validation_hipc() {
    use tpt_io_fhir::model::Identifier;
    use tpt_io_fhir::validation::hipc;

    let patient = tpt_io_fhir::model::Patient {
        resource_type: "Patient".to_string(),
        name: vec![HumanName::new("Doe", &["Jane"])],
        identifier: vec![Identifier::new(hipc::NHI_SYSTEM, "XYZ1234")],
        ..Default::default()
    };
    let result = hipc::validate_patient(&patient);
    assert!(result.is_valid());
}

#[test]
fn batch_build() {
    use tpt_io_fhir::batch::BatchBuilder;

    let patient = tpt_io_fhir::model::Patient {
        resource_type: "Patient".to_string(),
        id: Some("123".to_string()),
        ..Default::default()
    };
    let bundle = BatchBuilder::transaction().create(Resource::Patient(patient)).build();
    assert_eq!(bundle.r#type, Some(BundleType::Transaction));
    assert_eq!(bundle.entry.len(), 1);
}

#[test]
fn invalid_json_reports_error() {
    let bad = r#"{"resourceType": "Patient", "id": }"#;
    let err = json::parse_patient(bad).unwrap_err();
    assert!(matches!(err, FhirError::Json(_)));
}

#[test]
fn missing_resource_type() {
    let no_type = r#"{"id": "x"}"#;
    let r = json::parse_resource(no_type).unwrap();
    assert!(matches!(r, Resource::Other(_)));
}
