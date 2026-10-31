//! Integration tests for the LandXML reader and writer.

use tpt_io_landxml::{parse, write, LandXmlError, LandXml, Surface};

const SURVEY: &[u8] = br#"<?xml version="1.0"?>
<LandXML xmlns="http://www.landxml.org/schema/LandXML-1.2">
  <Units><Imperial linearUnit="USSurveyFoot"/></Units>
  <Project name="Ridge Survey"/>
  <Surfaces>
    <Surface name="Existing Ground">
      <Definition surfType="TIN">
        <Pnts>
          <P id="1">1000.0 2000.0 150.5</P>
          <P id="2">1010.0 2000.0 151.0</P>
          <P id="3">1000.0 2010.0 152.25</P>
          <P id="4">1010.0 2010.0 153.0</P>
        </Pnts>
        <Faces>
          <F>1 2 3</F>
          <F>2 4 3</F>
        </Faces>
      </Definition>
    </Surface>
  </Surfaces>
  <Alignments><Alignment name="Road A"/></Alignments>
</LandXML>"#;

#[test]
fn reads_units_project_surface_and_alignments() {
    let land = parse(SURVEY).unwrap();
    assert_eq!(land.linear_unit.as_deref(), Some("USSurveyFoot"));
    assert_eq!(land.project.as_deref(), Some("Ridge Survey"));
    let s = &land.surfaces[0];
    assert_eq!(s.name, "Existing Ground");
    assert_eq!(s.points.len(), 4);
    assert_eq!(s.points[2], (3, [1000.0, 2010.0, 152.25]));
    assert_eq!(s.faces, vec![[1, 2, 3], [2, 4, 3]]);
    assert_eq!(land.alignments, ["Road A"]);
    land.validate().unwrap();
}

#[test]
fn write_then_parse_is_identity() {
    let land = parse(SURVEY).unwrap();
    let text = write(&land);
    assert_eq!(parse(&text).unwrap(), land);
}

#[test]
fn validation_rejects_unknown_points_and_degenerate_faces() {
    let mut land = parse(SURVEY).unwrap();
    land.surfaces[0].faces.push([1, 2, 99]);
    assert!(matches!(land.validate(), Err(LandXmlError::Invalid(_))));

    let mut land = LandXml::default();
    land.surfaces.push(Surface {
        name: "x".into(),
        points: vec![(1, [0.0; 3]), (2, [1.0; 3]), (3, [2.0; 3])],
        faces: vec![[1, 1, 2]],
    });
    assert!(matches!(land.validate(), Err(LandXmlError::Invalid(_))));
}

#[test]
fn malformed_input_is_an_error() {
    assert!(parse(b"<Other/>").is_err());
    assert!(parse(br#"<LandXML><Surfaces><Surface><Definition><Pnts><P id="1">1 2</P></Pnts></Definition></Surface></Surfaces></LandXML>"#).is_err());
    assert!(parse(br#"<LandXML><Surfaces><Surface><Definition><Faces><F>1 2</F></Faces></Definition></Surface></Surfaces></LandXML>"#).is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 79;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"<LandXML><P id=\"1\">0123456789. </P></F>";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
