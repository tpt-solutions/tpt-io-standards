//! Integration tests for the IPC-2581 reader.

use tpt_io_ipc2581::{parse, Ipc2581Error};

const BOARD: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<IPC-2581 revision="C" xmlns="http://webstds.ipc.org/2581">
  <Content roleRef="Owner">
    <CadHeader units="MILLIMETER" specification="SPEC-1"/>
  </Content>
  <Ecad name="demo">
    <CadData>
      <Layer name="TOP" layerFunction="CONDUCTOR" side="TOP"/>
      <Layer name="SM_TOP" layerFunction="SOLDERMASK" side="TOP"/>
      <Layer name="BOTTOM" layerFunction="CONDUCTOR" side="BOTTOM"/>
      <Step name="board">
        <Component refDes="R1" packageRef="R0603"/>
        <Component refDes="C7" packageRef="C0805"/>
        <LogicalNet name="VCC"/>
        <LogicalNet name="GND"/>
      </Step>
    </CadData>
  </Ecad>
</IPC-2581>"#;

#[test]
fn reads_revision_units_layers_steps_components_and_nets() {
    let b = parse(BOARD).unwrap();
    assert_eq!(b.revision, "C");
    assert_eq!(b.units.as_deref(), Some("MILLIMETER"));
    assert_eq!(b.layers.len(), 3);
    assert_eq!(b.layers[1].function.as_deref(), Some("SOLDERMASK"));
    assert_eq!(b.layers[2].side.as_deref(), Some("BOTTOM"));
    assert_eq!(b.steps, ["board"]);
    assert_eq!(b.components[1].ref_des, "C7");
    assert_eq!(b.components[0].package_ref.as_deref(), Some("R0603"));
    assert_eq!(b.nets, ["VCC", "GND"]);
}

#[test]
fn a_file_without_optional_parts_still_reads() {
    let b = parse(br#"<IPC-2581 revision="B"/>"#).unwrap();
    assert_eq!(b.revision, "B");
    assert!(b.units.is_none() && b.layers.is_empty() && b.components.is_empty());
}

#[test]
fn wrong_root_or_missing_revision_are_errors() {
    assert!(parse(b"<Other revision=\"B\"/>").is_err());
    assert!(matches!(parse(b"<IPC-2581/>"), Err(Ipc2581Error::Invalid(_))));
    assert!(parse(b"<IPC-2581 revision=\"B\"><Ecad>").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 97;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"<IPC-2581 revision=\"C\"/>Layer name Step ";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
