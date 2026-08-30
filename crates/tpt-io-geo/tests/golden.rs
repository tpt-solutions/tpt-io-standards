//! Golden-file tests: parse a checked-in sample and assert stable, expected output.
//!
//! The corpus lives in `tests/samples/`. These tests guard against accidental
//! parser regressions on real-world-shaped documents.

use std::path::PathBuf;

use tpt_io_geo::geojson::{parse_str, GeometryEnum};

fn sample_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("samples");
    p.push(name);
    p
}

#[test]
fn golden_wellington_geojson() {
    let src = std::fs::read_to_string(sample_path("wellington.geojson"))
        .expect("golden sample must exist");
    let fc = parse_str(&src).expect("golden sample must parse");

    assert_eq!(fc.features.len(), 3);

    // Feature 0: a named Polygon.
    let f0 = &fc.features[0];
    assert_eq!(
        f0.id.as_ref().map(|id| match id {
            tpt_io_geo::geojson::FeatureId::String(s) => s.clone(),
            tpt_io_geo::geojson::FeatureId::Number(n) => n.to_string(),
        }),
        Some("bldg-1".to_string())
    );
    match &f0.geometry.as_ref().unwrap().geometry {
        GeometryEnum::Polygon(parts) => {
            assert_eq!(parts.len(), 1);
            assert_eq!(parts[0].len(), 5);
        }
        other => panic!("expected polygon, got {other:?}"),
    }
    assert_eq!(f0.properties.get("name").and_then(|v| v.as_str()), Some("Beehive"));

    // Feature 1: a Point with id "bldg-2".
    let f1 = &fc.features[1];
    assert!(matches!(&f1.geometry.as_ref().unwrap().geometry, GeometryEnum::Point(_)));

    // Feature 2: a LineString.
    let f2 = &fc.features[2];
    assert!(matches!(&f2.geometry.as_ref().unwrap().geometry, GeometryEnum::LineString(_)));

    // Foreign members (the top-level "name") must be preserved.
    assert!(fc.foreign_members.iter().any(|(k, _)| k == "name"));
}

#[test]
fn golden_file_round_trips() {
    let src = std::fs::read_to_string(sample_path("wellington.geojson")).unwrap();
    let fc = parse_str(&src).unwrap();
    let out = tpt_io_geo::geojson::to_string(&fc);
    let fc2 = parse_str(&out).unwrap();
    assert_eq!(fc, fc2);
}
