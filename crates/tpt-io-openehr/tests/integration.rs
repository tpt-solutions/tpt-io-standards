//! Integration tests for the openEHR composition reader.

use tpt_io_openehr::{parse, OpenEhrError};

const COMP: &[u8] = br#"{
  "_type": "COMPOSITION",
  "name": { "_type": "DV_TEXT", "value": "Vital signs" },
  "archetype_details": {
    "archetype_id": { "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
    "template_id": { "value": "Vitals_Template" },
    "rm_version": "1.0.4"
  },
  "language": { "code_string": "en" },
  "content": [
    {
      "_type": "OBSERVATION",
      "archetype_node_id": "openEHR-EHR-OBSERVATION.blood_pressure.v2",
      "name": { "value": "Blood pressure" }
    },
    {
      "_type": "EVALUATION",
      "archetype_node_id": "openEHR-EHR-EVALUATION.problem.v1"
    }
  ]
}"#;

#[test]
fn reads_identity_and_top_level_entries() {
    let c = parse(COMP).unwrap();
    assert_eq!(c.name.as_deref(), Some("Vital signs"));
    assert_eq!(c.archetype_id.as_deref(), Some("openEHR-EHR-COMPOSITION.encounter.v1"));
    assert_eq!(c.template_id.as_deref(), Some("Vitals_Template"));
    assert_eq!(c.rm_version.as_deref(), Some("1.0.4"));
    assert_eq!(c.language.as_deref(), Some("en"));
    assert_eq!(c.entries.len(), 2);
    assert_eq!(c.entries[0].kind, "OBSERVATION");
    assert_eq!(c.entries[0].name.as_deref(), Some("Blood pressure"));
    assert_eq!(c.entries[1].kind, "EVALUATION");
    assert!(c.entries[1].name.is_none());
}

#[test]
fn other_types_and_bad_json_are_errors() {
    assert!(matches!(parse(br#"{"_type":"PARTY"}"#), Err(OpenEhrError::Invalid(_))));
    assert!(matches!(parse(b"[]"), Err(OpenEhrError::Invalid(_))));
    assert!(matches!(parse(b"{\"_type\":\"COMPOSITION\""), Err(OpenEhrError::Core(_))));
}

#[test]
fn a_minimal_composition_is_accepted() {
    let c = parse(br#"{"_type":"COMPOSITION"}"#).unwrap();
    assert!(c.entries.is_empty() && c.name.is_none());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 127;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"{}[]\":,_typeCOMPOSITION value 0123";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
