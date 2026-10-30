//! Integration tests for `tpt-io-gerber`: golden-file parsing, full round-trip,
//! design-rule extraction, and property-based (proptest) invariants.

use tpt_io_core::{Reader, Writer};
use tpt_io_gerber::aperture::Aperture;
use tpt_io_gerber::attributes::Attributes;
use tpt_io_gerber::drc::extract_drc;
use tpt_io_gerber::excellon;
use tpt_io_gerber::model::{GerberFile, GraphicObject, ObjectKind};
use tpt_io_gerber::parser::{parse_str, GerberReader};

const SAMPLE: &str = include_str!("samples/simple.gbr");

#[test]
fn golden_file_parses() {
    let file = parse_str(SAMPLE).expect("golden sample must parse");
    assert_eq!(file.unit, tpt_io_gerber::model::Unit::Mm);
    assert_eq!(file.apertures.len(), 3);
    assert_eq!(file.aperture_macros.len(), 1);
    assert_eq!(file.aperture_macros[0].0, "RECT");

    // Aperture 11 is a circle with a hole.
    assert_eq!(
        file.apertures[1],
        (11, Aperture::Circle { diameter: 1.0, hole_diameter: Some(0.3) })
    );

    // Aperture attribute captured on aperture 10.
    let ap10_attr = file
        .aperture_attributes
        .iter()
        .find(|(id, _)| *id == 10)
        .expect("aperture 10 attribute present");
    assert_eq!(ap10_attr.1.get("AperFunction"), Some(&["Pad".to_string()][..]));

    // The flash carries the X3 net attribute.
    let flash = file
        .objects
        .iter()
        .find(|o| matches!(o.kind, ObjectKind::Flash { .. }))
        .expect("a flash exists");
    assert_eq!(flash.attributes.net_names(), vec!["GND"]);

    // Exactly one region was defined.
    assert!(file.objects.iter().any(|o| matches!(o.kind, ObjectKind::Region { .. })));
}

#[test]
fn reader_round_trips_golden_file() {
    let original = parse_str(SAMPLE).expect("parse");
    let serialized = GerberReader::new().serialize(&original).expect("serialize");
    let reparsed = parse_str(core::str::from_utf8(&serialized).unwrap()).expect("reparse");
    assert_eq!(original, reparsed);
}

#[test]
fn excellon_round_trips() {
    let src = "M48\nMETRIC,LZ\nT01C0.5\nT02C0.8\nM95\nG90\nT01\nX500Y500\nX1000Y1000\nT02\nX2000Y2000\nM30\n";
    let file = excellon::parse_str(src).expect("parse excellon");
    let bytes = excellon::serialize(&file).expect("serialize excellon");
    let reparsed = excellon::parse_str(core::str::from_utf8(&bytes).unwrap()).expect("reparse");
    assert_eq!(file, reparsed);
}

#[test]
fn drc_extracts_widths_and_rings() {
    let file = parse_str(SAMPLE).expect("parse");
    let drc = extract_drc(&file);
    // Aperture 11 (1.0 dia, 0.3 hole) -> annular ring (1.0 - 0.3)/2 = 0.35.
    assert_eq!(drc.min_annular_ring, Some(0.35));
    // Aperture 10 (0.5) and 11 (1.0) -> smallest width 0.5.
    assert_eq!(drc.min_trace_width, Some(0.5));
}

#[test]
fn parse_rejects_undefined_aperture_in_validation() {
    let bad = "%FSLAX36Y36*%\n%MOMM*%\nD99*\nX0Y0D03*\nM02*\n";
    let res = GerberReader::new().parse(bad.as_bytes());
    assert!(res.is_err());
}

#[cfg(test)]
mod proptest_roundtrip {
    use super::*;
    use proptest::prelude::*;

    /// Build a structurally valid Gerber file from generated integer coordinates
    /// (multiples of 1e-6 so round-tripping through the fixed-point format is
    /// lossless).
    fn build_file(coords: Vec<(i64, i64)>) -> GerberFile {
        let mut file = GerberFile::default();
        file.apertures.push((10, Aperture::Circle { diameter: 0.5, hole_diameter: None }));
        let mut prev: Option<(f64, f64)> = None;
        for (x, y) in coords {
            let fx = x as f64 / 1e6;
            let fy = y as f64 / 1e6;
            let kind = match prev {
                None => ObjectKind::Flash { x: fx, y: fy },
                Some((px, py)) => ObjectKind::Line { x0: px, y0: py, x1: fx, y1: fy },
            };
            file.objects.push(GraphicObject {
                kind,
                aperture: Some(10),
                attributes: Attributes::new(),
            });
            prev = Some((fx, fy));
        }
        file.end_of_file = true;
        file
    }

    proptest! {
        #[test]
        fn roundtrip_is_idempotent(
            // The file's 3.6 coordinate format covers |value| < 1000 mm, so
            // generate only micrometre counts inside that range.
            coords in proptest::collection::vec(
                (-999_999_999i64..=999_999_999, -999_999_999i64..=999_999_999),
                1..12,
            ),
        ) {
            let file = build_file(coords);
            let bytes = GerberReader::new().serialize(&file).expect("serialize");
            let reparsed = parse_str(core::str::from_utf8(&bytes).unwrap()).expect("reparse");
            prop_assert_eq!(file, reparsed);
        }

        #[test]
        fn no_panic_on_arbitrary_bytes(
            data in proptest::collection::vec(any::<u8>(), 0..512),
        ) {
            // Parsing arbitrary bytes must never panic; it returns Ok or Err.
            let _ = parse_str(core::str::from_utf8(&data).unwrap_or("\0invalid-utf8"));
        }
    }
}
