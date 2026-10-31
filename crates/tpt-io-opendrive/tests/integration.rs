//! Integration tests for the OpenDRIVE reader and writer.

use tpt_io_opendrive::{parse, write, GeometryKind, OpenDriveError};

const MAP: &[u8] = br#"<?xml version="1.0" standalone="yes"?>
<OpenDRIVE>
  <header revMajor="1" revMinor="6" name="two_roads"/>
  <road name="Main" id="1" length="150" junction="-1">
    <planView>
      <geometry s="0" x="0" y="0" hdg="0" length="100"><line/></geometry>
      <geometry s="100" x="100" y="0" hdg="0" length="50"><arc curvature="0.02"/></geometry>
    </planView>
    <lanes>
      <laneSection s="0">
        <left>
          <lane id="1" type="driving" level="false">
            <width sOffset="0" a="3.5" b="0" c="0" d="0"/>
          </lane>
        </left>
        <center><lane id="0" type="none" level="false"/></center>
        <right>
          <lane id="-1" type="driving" level="false">
            <width sOffset="0" a="3.25" b="0.01" c="0" d="0"/>
          </lane>
          <lane id="-2" type="sidewalk" level="true">
            <width sOffset="0" a="2" b="0" c="0" d="0"/>
          </lane>
        </right>
      </laneSection>
      <laneSection s="60">
        <right><lane id="-1" type="driving" level="false"><width sOffset="0" a="3.0" b="0" c="0" d="0"/></lane></right>
      </laneSection>
    </lanes>
  </road>
  <road name="Ramp" id="2" length="40" junction="7">
    <planView>
      <geometry s="0" x="5" y="5" hdg="1.5708" length="40"><spiral curvStart="0" curvEnd="0.05"/></geometry>
    </planView>
    <lanes><laneSection s="0"><center><lane id="0" type="none" level="false"/></center></laneSection></lanes>
  </road>
</OpenDRIVE>"#;

#[test]
fn reads_header_roads_geometry_and_lanes() {
    let map = parse(MAP).unwrap();
    assert_eq!((map.header.rev_major, map.header.rev_minor), (1, 6));
    assert_eq!(map.header.name, "two_roads");
    assert_eq!(map.roads.len(), 2);

    let main = &map.roads[0];
    assert!(main.junction.is_none());
    assert_eq!(main.geometry[1].kind, GeometryKind::Arc { curvature: 0.02 });
    assert_eq!(main.geometry[1].s, 100.0);

    let first = &main.lane_sections[0];
    assert_eq!(first.left[0].id, 1);
    assert_eq!(first.center[0].id, 0);
    assert_eq!(first.right.len(), 2);
    assert!(first.right[1].level);
    assert_eq!(first.right[0].widths[0].b, 0.01);
    assert_eq!(main.lane_sections[1].s, 60.0);

    let ramp = &map.roads[1];
    assert_eq!(ramp.junction.as_deref(), Some("7"));
    assert_eq!(ramp.geometry[0].kind, GeometryKind::Spiral { curv_start: 0.0, curv_end: 0.05 });
}

#[test]
fn map_is_valid() {
    parse(MAP).unwrap().validate().unwrap();
}

#[test]
fn write_then_parse_is_identity() {
    let map = parse(MAP).unwrap();
    let text = write(&map);
    assert_eq!(parse(&text).unwrap(), map);
}

#[test]
fn validation_finds_gaps_and_duplicates() {
    let mut map = parse(MAP).unwrap();
    map.roads[0].geometry[1].s = 120.0;
    assert!(matches!(map.validate(), Err(OpenDriveError::Invalid(_))));

    let mut map = parse(MAP).unwrap();
    map.roads[1].id = "1".into();
    assert!(matches!(map.validate(), Err(OpenDriveError::Invalid(_))));

    let mut map = parse(MAP).unwrap();
    map.roads[0].lane_sections[0].left[0].id = -1;
    assert!(matches!(map.validate(), Err(OpenDriveError::Invalid(_))));
}

#[test]
fn unsupported_shapes_and_bad_numbers_are_errors() {
    assert!(parse(b"<OpenDRIVE><road id=\"1\"><planView><geometry s=\"0\"><clothoid/></geometry></planView></road></OpenDRIVE>").is_err());
    assert!(parse(b"<OpenDRIVE><road id=\"1\" length=\"long\"/></OpenDRIVE>").is_err());
    assert!(parse(b"<robot/>").is_err());
}

#[test]
fn poly_and_param_poly_round_trip() {
    let src = br#"<OpenDRIVE><header revMajor="1" revMinor="6" name="p"/>
      <road id="9" length="10" junction="-1"><planView>
        <geometry s="0" x="0" y="0" hdg="0" length="10"><poly3 a="0" b="1" c="0.1" d="0.01"/></geometry>
        <geometry s="10" x="0" y="0" hdg="0" length="5"><paramPoly3 aU="0" bU="1" cU="0" dU="0" aV="0" bV="0" cV="0.2" dV="0"/></geometry>
      </planView><lanes/></road></OpenDRIVE>"#;
    let map = parse(src).unwrap();
    assert!(matches!(map.roads[0].geometry[0].kind, GeometryKind::Poly3 { .. }));
    assert_eq!(parse(&write(&map)).unwrap(), map);
}
