//! Integration tests for the gbXML reader.

use tpt_io_gbxml::{parse, GbXmlError};

const CAMPUS: &[u8] = br#"<?xml version="1.0"?>
<gbXML xmlns="http://www.gbxml.org/schema" version="6.01">
  <Campus id="c1">
    <Building id="b1" buildingType="Office">
      <Name>Head Office</Name>
      <Space id="s1"><Name>Lobby</Name></Space>
      <Space id="s2"/>
      <Surface id="w1" surfaceType="ExteriorWall">
        <PlanarGeometry>
          <PolyLoop>
            <CartesianPoint><Coordinate>0</Coordinate><Coordinate>0</Coordinate><Coordinate>0</Coordinate></CartesianPoint>
            <CartesianPoint><Coordinate>5</Coordinate><Coordinate>0</Coordinate><Coordinate>0</Coordinate></CartesianPoint>
            <CartesianPoint><Coordinate>5</Coordinate><Coordinate>0</Coordinate><Coordinate>3.5</Coordinate></CartesianPoint>
          </PolyLoop>
        </PlanarGeometry>
      </Surface>
    </Building>
    <Building id="b2"><Name>Annex</Name></Building>
  </Campus>
</gbXML>"#;

#[test]
fn reads_buildings_spaces_and_surfaces() {
    let m = parse(CAMPUS).unwrap();
    assert_eq!(m.buildings.len(), 2);
    let b1 = &m.buildings[0];
    assert_eq!(b1.id, "b1");
    assert_eq!(b1.building_type.as_deref(), Some("Office"));
    assert_eq!(b1.name.as_deref(), Some("Head Office"));
    assert_eq!(b1.spaces.len(), 2);
    assert_eq!(b1.spaces[0].name.as_deref(), Some("Lobby"));
    assert!(b1.spaces[1].name.is_none());
}

#[test]
fn surface_polyloop_points_are_read() {
    let m = parse(CAMPUS).unwrap();
    let w = &m.buildings[0].surfaces[0];
    assert_eq!(w.surface_type, "ExteriorWall");
    assert_eq!(w.points, vec![[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [5.0, 0.0, 3.5]]);
    assert!(m.buildings[1].surfaces.is_empty());
}

#[test]
fn missing_ids_and_bad_coordinates_are_errors() {
    assert!(matches!(parse(b"<gbXML><Building/></gbXML>"), Err(GbXmlError::Invalid(_))));
    let short = br#"<gbXML><Building id="b"><Surface id="s"><PlanarGeometry><PolyLoop><CartesianPoint><Coordinate>1</Coordinate></CartesianPoint></PolyLoop></PlanarGeometry></Surface></Building></gbXML>"#;
    assert!(matches!(parse(short), Err(GbXmlError::Invalid(_))));
    let bad = br#"<gbXML><Building id="b"><Surface id="s"><PlanarGeometry><PolyLoop><CartesianPoint><Coordinate>x</Coordinate><Coordinate>0</Coordinate><Coordinate>0</Coordinate></CartesianPoint></PolyLoop></PlanarGeometry></Surface></Building></gbXML>"#;
    assert!(parse(bad).is_err());
    assert!(parse(b"<Other/>").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 83;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"<gbXML>Building id=\"1\"Coordinate>0123456789.</ ";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
