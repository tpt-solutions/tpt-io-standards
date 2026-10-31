//! Integration tests for the DEXPI reader.

use tpt_io_dexpi::parse;

const PID: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<PlantInformation SchemaVersion="1.3">
  <Equipment ID="E-1" ComponentClass="CentrifugalPump" ComponentName="P-101">
    <Nozzle ID="N-1" ComponentClass="Nozzle"/>
    <Nozzle ID="N-2" ComponentClass="Nozzle"/>
  </Equipment>
  <PipingNetworkSegment ID="PS-1" ComponentClass="PipingNetworkSegment">
    <PipingComponent ID="V-1" ComponentClass="GateValve" ComponentName="XV-10"/>
  </PipingNetworkSegment>
  <ProcessInstrumentationFunction ID="PIC-1" ComponentClass="PressureIndicatingController" ComponentName="PIC-101"/>
  <Note>no id here</Note>
</PlantInformation>"#;

#[test]
fn collects_every_identified_component() {
    let p = parse(PID).unwrap();
    let ids: Vec<&str> = p.components.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["E-1", "PS-1", "V-1", "PIC-1"]);
}

#[test]
fn components_carry_kind_class_tag_and_nozzles() {
    let p = parse(PID).unwrap();
    let pump = &p.components[0];
    assert_eq!(pump.kind, "Equipment");
    assert_eq!(pump.class, "CentrifugalPump");
    assert_eq!(pump.tag.as_deref(), Some("P-101"));
    assert_eq!(pump.nozzles, ["N-1", "N-2"]);
    assert_eq!(p.components[2].kind, "PipingComponent");
    assert!(p.components[1].tag.is_none());
}

#[test]
fn malformed_xml_is_an_error() {
    assert!(parse(b"<PlantInformation><Equipment ID=\"x\"").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 107;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"<Equipment ID=\"E1\" ComponentClass=\"P\"/>Nozzle </>";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
