//! Round-trip tests: parse → serialize → parse → equality, plus DLIS and
//! encoding-detection coverage.

use tpt_io_core::{Reader, Writer};
use tpt_io_welllog::binary;
use tpt_io_welllog::dlis;
use tpt_io_welllog::model::WellLog;
use tpt_io_welllog::parser::{parse_str, LasReader};

const SAMPLE: &str = "\
~Version Information
VERS. 2.0 :
WRAP. NO :
~Well Information
NULL. -999.25 :
WELL. TEST WELL 1 :
FLD. EXAMPLE FIELD :
~Curve Information
DEPT.M   : DEPTH
GR.API   : GAMMA RAY
ILD.OHMM   : DEEP RESISTIVITY
~ASCII Log Data
10.0 75.5 12.3
20.0 80.1 11.9
30.0 78.4 12.0
";

#[test]
fn ascii_round_trip() {
    let original = parse_str(SAMPLE).unwrap();
    let bytes = LasReader::new().serialize(&original).unwrap();
    let reparsed = parse_str(core::str::from_utf8(&bytes).unwrap()).unwrap();
    assert_eq!(original, reparsed);
}

#[test]
fn reader_trait_round_trip() {
    let original = parse_str(SAMPLE).unwrap();
    let reader = LasReader::new();
    let bytes = reader.serialize(&original).unwrap();
    let reparsed: WellLog = reader.parse(&bytes).unwrap();
    assert_eq!(original, reparsed);
}

#[test]
fn binary_las3_round_trip() {
    let header = "\
~Version Information
VERS. 3.0 :
WRAP. NO :
~Well Information
NULL. -999.25 :
WELL. BIN TEST :
~Curve Information
DEPT.M   F4 : DEPTH
GR.API   F4 : GAMMA RAY
~ASCII Log Data
BINARY
";
    let mut bytes = header.as_bytes().to_vec();
    for (d, g) in [(10.0f32, 75.5f32), (20.0, 80.1), (30.0, 78.4)] {
        bytes.extend_from_slice(&d.to_le_bytes());
        bytes.extend_from_slice(&g.to_le_bytes());
    }
    let log = binary::parse(&bytes).unwrap();
    assert_eq!(log.version, 3.0);
    let expected = [[10.0, 75.5], [20.0, 80.1], [30.0, 78.4]];
    for (r, exp) in log.data.iter().zip(expected.iter()) {
        for (v, e) in r.iter().zip(exp.iter()) {
            assert!((v - e).abs() < 1e-3, "got {v} expected {e}");
        }
    }
}

#[test]
fn golden_sample_parses() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/samples/well_sample.las"
    ))
    .expect("golden sample must exist");
    let log = parse_str(&src).expect("golden sample must parse");
    assert_eq!(log.well.well_name.as_deref(), Some("TAURANGA-1"));
    assert_eq!(log.well.api.as_deref(), Some("1234567890"));
    assert_eq!(log.curves.len(), 3);
    assert_eq!(log.curves[0].mnemonic, "DEPT");
    assert_eq!(log.curves[2].description, "DEEP RESISTIVITY");
    assert_eq!(log.data.len(), 5);
    assert_eq!(log.index_values().collect::<Vec<_>>(), vec![100.0, 110.0, 120.0, 130.0, 140.0]);
    assert_eq!(log.data[3], vec![130.0, 60.2, 2.05]);
}

#[test]
fn dlis_minimal_parses_without_panic() {
    // The DLIS unit tests already validate structure; here we just confirm the
    // public entry point is panic-free on a tiny synthetic stream.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u32.to_be_bytes());
    bytes.extend_from_slice(&[b' '; 60]);
    bytes.extend_from_slice(&80u32.to_be_bytes());
    bytes.push(b'S');
    bytes.push(b'V');
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&[0u8; 8]);
    let d = dlis::parse(&bytes).unwrap();
    assert_eq!(d.label.version, b'V');
}
