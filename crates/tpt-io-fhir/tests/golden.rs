//! Golden-file tests: parse the checked-in samples under `tests/samples/`
//! and assert their known content, so any change in parser behaviour shows
//! up as an explicit diff against these fixtures.

use tpt_io_fhir::model::{Patient, Resource};
use tpt_io_fhir::{json, xml, BundleCursor, BundleType};

const PATIENT_JSON: &str = include_str!("samples/patient_nhi.json");
const PATIENT_XML: &str = include_str!("samples/patient_nhi.xml");
const SEARCH_BUNDLE: &str = include_str!("samples/search_bundle.json");

fn assert_golden_patient(p: &Patient) {
    assert_eq!(p.id.as_deref(), Some("example"));
    assert_eq!(p.gender.as_deref(), Some("male"));
    assert_eq!(p.birth_date.as_deref(), Some("1974-12-25"));
    assert_eq!(p.identifier.len(), 1);
    assert_eq!(p.identifier[0].value.as_deref(), Some("XYZ1234"));
    assert_eq!(p.name.len(), 2);
    assert_eq!(p.name[0].family.as_deref(), Some("Chalmers"));
    assert_eq!(p.name[0].given, ["Peter", "James"]);
    assert_eq!(p.name[1].family.as_deref(), Some("Windsor"));
}

#[test]
fn golden_patient_json() {
    let p = json::parse_patient(PATIENT_JSON).unwrap();
    assert_golden_patient(&p);
}

#[test]
fn golden_patient_xml() {
    let p = xml::parse_patient(PATIENT_XML).unwrap();
    assert_golden_patient(&p);
}

#[test]
fn golden_patient_json_and_xml_agree() {
    let from_json = json::parse_patient(PATIENT_JSON).unwrap();
    let from_xml = xml::parse_patient(PATIENT_XML).unwrap();
    assert_eq!(from_json, from_xml);
}

#[test]
fn golden_patient_canonical_json_is_stable() {
    // Serialize once, parse the output, serialize again: the two canonical
    // forms must be byte-identical.
    let p = json::parse_patient(PATIENT_JSON).unwrap();
    let first = json::to_json(&Resource::Patient(p)).unwrap();
    let reparsed = json::parse_patient(&first).unwrap();
    let second = json::to_json(&Resource::Patient(reparsed)).unwrap();
    assert_eq!(first, second);
}

#[test]
fn golden_search_bundle_pagination() {
    let bundle = json::parse_bundle(SEARCH_BUNDLE).unwrap();
    assert_eq!(bundle.r#type, Some(BundleType::Searchset));
    assert_eq!(bundle.total, Some(2));
    assert_eq!(bundle.entry.len(), 2);

    let cursor = BundleCursor::new(bundle, "https://example.com/fhir");
    assert!(cursor.has_next());
    assert_eq!(cursor.next_url().as_deref(), Some("https://example.com/fhir/Patient?_page=2"));
}
