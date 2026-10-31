//! Integration tests for the AIXM reader.

use tpt_io_aixm::{parse, AixmError, FeatureKind};

const MESSAGE: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<AIXMBasicMessage xmlns="http://www.aixm.aero/schema/5.1"
    xmlns:aixm="http://www.aixm.aero/schema/5.1"
    xmlns:gml="http://www.opengis.net/gml/3.2">
  <hasMember>
    <aixm:AirportHeliport gml:id="AHP1">
      <aixm:timeSlice>
        <aixm:AirportHeliportTimeSlice>
          <aixm:designator>NZAA</aixm:designator>
          <aixm:name>Auckland</aixm:name>
          <aixm:ARP><aixm:ElevatedPoint><gml:pos>-37.008 174.792</gml:pos></aixm:ElevatedPoint></aixm:ARP>
        </aixm:AirportHeliportTimeSlice>
      </aixm:timeSlice>
    </aixm:AirportHeliport>
  </hasMember>
  <hasMember>
    <aixm:Airspace gml:id="AS1">
      <aixm:timeSlice>
        <aixm:AirspaceTimeSlice>
          <aixm:name>Zone A</aixm:name>
          <aixm:geometryComponent>
            <gml:Polygon><gml:posList>-36.0 174.0 -36.0 175.0 -37.0 175.0 -36.0 174.0</gml:posList></gml:Polygon>
          </aixm:geometryComponent>
        </aixm:AirspaceTimeSlice>
      </aixm:timeSlice>
    </aixm:Airspace>
  </hasMember>
  <hasMember>
    <aixm:Navaid gml:id="NAV1">
      <aixm:timeSlice><aixm:NavaidTimeSlice><aixm:designator>ABC</aixm:designator></aixm:NavaidTimeSlice></aixm:timeSlice>
    </aixm:Navaid>
  </hasMember>
</AIXMBasicMessage>"#;

#[test]
fn reads_each_feature_kind_with_identifiers() {
    let f = parse(MESSAGE).unwrap();
    assert_eq!(f.len(), 3);
    assert_eq!(f[0].kind, FeatureKind::AirportHeliport);
    assert_eq!(f[0].gml_id.as_deref(), Some("AHP1"));
    assert_eq!(f[0].designator.as_deref(), Some("NZAA"));
    assert_eq!(f[0].name.as_deref(), Some("Auckland"));
    assert_eq!(f[1].kind, FeatureKind::Airspace);
    assert_eq!(f[2].kind, FeatureKind::Navaid);
    assert!(f[2].name.is_none());
}

#[test]
fn positions_come_from_pos_and_pos_list() {
    let f = parse(MESSAGE).unwrap();
    assert_eq!(f[0].positions, vec![(-37.008, 174.792)]);
    assert_eq!(f[1].positions.len(), 4);
    assert_eq!(f[1].positions[0], (-36.0, 174.0));
}

#[test]
fn malformed_positions_are_errors() {
    let odd = br#"<M><hasMember><Route><gml:posList xmlns:gml="g">1 2 3</gml:posList></Route></hasMember></M>"#;
    assert!(matches!(parse(odd), Err(AixmError::Invalid(_))));
    let bad = br#"<M><hasMember><Route><pos>x y</pos></Route></hasMember></M>"#;
    assert!(parse(bad).is_err());
    assert!(parse(b"<M><hasMember>").is_err());
}

#[test]
fn empty_messages_have_no_features() {
    assert!(parse(b"<AIXMBasicMessage/>").unwrap().is_empty());
}
