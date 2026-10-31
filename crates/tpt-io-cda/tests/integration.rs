//! Integration tests for the CDA reader.

use tpt_io_cda::parse;

const DOC: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<ClinicalDocument xmlns="urn:hl7-org:v3">
  <id root="2.16.840.1.113883.19" extension="DOC-42"/>
  <code code="34133-9" codeSystem="2.16.840.1.113883.6.1"/>
  <title>Continuity of Care Document</title>
  <effectiveTime value="20240315093000"/>
  <recordTarget>
    <patientRole>
      <id root="2.16.840.1.113883.19.5" extension="12345"/>
      <patient>
        <name><given>Grace</given><family>Hopper</family></name>
      </patient>
    </patientRole>
  </recordTarget>
  <component>
    <structuredBody>
      <component>
        <section>
          <title>Problems</title>
          <code code="11450-4" codeSystem="2.16.840.1.113883.6.1"/>
          <text>Hypertension</text>
        </section>
      </component>
      <component>
        <section>
          <title>Medications</title>
          <code code="10160-0"/>
        </section>
      </component>
    </structuredBody>
  </component>
</ClinicalDocument>"#;

#[test]
fn reads_identity_patient_and_sections() {
    let d = parse(DOC).unwrap();
    assert_eq!(d.id.as_deref(), Some("2.16.840.1.113883.19:DOC-42"));
    assert_eq!(d.title.as_deref(), Some("Continuity of Care Document"));
    assert_eq!(d.effective_time.as_deref(), Some("20240315093000"));
    assert_eq!(d.patient.as_deref(), Some("Grace Hopper"));
    assert_eq!(d.sections.len(), 2);
    assert_eq!(d.sections[0].title.as_deref(), Some("Problems"));
    assert_eq!(d.sections[0].code.as_deref(), Some("11450-4"));
    assert_eq!(d.sections[1].code.as_deref(), Some("10160-0"));
}

#[test]
fn missing_parts_are_none_not_errors() {
    let d = parse(b"<ClinicalDocument/>").unwrap();
    assert!(d.id.is_none() && d.patient.is_none() && d.sections.is_empty());
}

#[test]
fn wrong_root_and_malformed_xml_are_errors() {
    assert!(parse(b"<Observation/>").is_err());
    assert!(parse(b"<ClinicalDocument><title></ClinicalDocument>").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 89;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"<ClinicalDocument>section title/> code=\"1\" ";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
