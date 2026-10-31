//! Integration tests for the WITSML trajectory reader.

use tpt_io_witsml::{parse, WitsmlError};

const TRAJ: &[u8] = br#"<?xml version="1.0"?>
<trajectorys version="1.4.1.1" xmlns="http://www.witsml.org/schemas/1series">
  <trajectory uidWell="W1" uidWellbore="B1" uid="T1">
    <name>Main trajectory</name>
    <trajectoryStation uid="s1">
      <md uom="m">0</md><incl uom="dega">0</incl><azi uom="dega">0</azi>
    </trajectoryStation>
    <trajectoryStation uid="s2">
      <md uom="m">100</md><incl uom="dega">12.5</incl><azi uom="dega">45</azi>
    </trajectoryStation>
  </trajectory>
  <trajectory uid="T2"><name>Sidetrack</name></trajectory>
</trajectorys>"#;

#[test]
fn reads_trajectories_and_stations() {
    let ts = parse(TRAJ).unwrap();
    assert_eq!(ts.len(), 2);
    let t = &ts[0];
    assert_eq!(t.uid.as_deref(), Some("T1"));
    assert_eq!(t.well.as_deref(), Some("W1"));
    assert_eq!(t.wellbore.as_deref(), Some("B1"));
    assert_eq!(t.name.as_deref(), Some("Main trajectory"));
    assert_eq!(t.stations.len(), 2);
    assert_eq!(t.stations[1].md, 100.0);
    assert_eq!(t.stations[1].incl, 12.5);
    assert_eq!(t.stations[1].azi, 45.0);
    assert_eq!(t.stations[1].md_uom.as_deref(), Some("m"));
    assert!(ts[1].stations.is_empty());
}

#[test]
fn a_station_missing_a_value_is_invalid() {
    let doc = br#"<trajectorys><trajectory uid="x"><trajectoryStation><md>1</md><incl>2</incl></trajectoryStation></trajectory></trajectorys>"#;
    assert!(matches!(parse(doc), Err(WitsmlError::Invalid(_))));
}

#[test]
fn bad_numbers_and_malformed_xml_are_errors() {
    let doc = br#"<trajectorys><trajectory><trajectoryStation><md>x</md><incl>2</incl><azi>3</azi></trajectoryStation></trajectory></trajectorys>"#;
    assert!(matches!(parse(doc), Err(WitsmlError::Core(_))));
    assert!(parse(b"<trajectorys><trajectory>").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 101;
    for _ in 0..200 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"<trajectory>md incl azi 0123456789.</ >";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
