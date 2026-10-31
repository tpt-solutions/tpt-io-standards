//! Integration tests for the AutomationML (CAEX) reader.

use tpt_io_automationml::{parse, AutomationMlError};

const PLANT: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<CAEXFile FileName="plant.aml" SchemaVersion="2.15">
  <InstanceHierarchy Name="Plant">
    <InternalElement Name="Line1" ID="l1">
      <InternalElement Name="Pump1" ID="p1" RefBaseSystemUnitPath="Lib/Pump">
        <Attribute Name="Power" AttributeDataType="xs:double">
          <Value>7.5</Value>
        </Attribute>
        <Attribute Name="Tag"><Value>P-101</Value></Attribute>
      </InternalElement>
      <InternalElement Name="Tank1" ID="t1"/>
    </InternalElement>
  </InstanceHierarchy>
  <InstanceHierarchy Name="Utilities"/>
  <InternalLink Name="pipe" RefPartnerSideA="p1:out" RefPartnerSideB="t1:in"/>
</CAEXFile>"#;

#[test]
fn reads_nested_elements_and_attributes() {
    let f = parse(PLANT).unwrap();
    assert_eq!(f.hierarchy.len(), 1);
    let line = &f.hierarchy[0];
    assert_eq!(line.name, "Line1");
    assert_eq!(line.children.len(), 2);
    let pump = &line.children[0];
    assert_eq!(pump.id.as_deref(), Some("p1"));
    assert_eq!(pump.system_unit_class.as_deref(), Some("Lib/Pump"));
    assert_eq!(pump.attributes[0].data_type.as_deref(), Some("xs:double"));
    assert_eq!(pump.attributes[0].value.as_deref(), Some("7.5"));
    assert_eq!(pump.attributes[1].data_type, None);
    assert!(line.children[1].attributes.is_empty());
}

#[test]
fn reads_internal_links() {
    let f = parse(PLANT).unwrap();
    assert_eq!(f.links.len(), 1);
    assert_eq!(f.links[0].side_a, "p1:out");
    assert_eq!(f.links[0].side_b, "t1:in");
}

#[test]
fn wrong_root_and_malformed_xml_are_errors() {
    assert!(matches!(parse(b"<Other/>"), Err(AutomationMlError::Core(_))));
    assert!(parse(b"<CAEXFile><InstanceHierarchy>").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 103;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"<CAEXFile>InternalElement Name=\"ID\"/>Value Attribute ";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
