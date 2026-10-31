//! Tests for the FlatGeobuf reader, against files written by GDAL through pyogrio (see `gen_fgb.py`).

use tpt_io_geo::flatgeobuf::read;
use tpt_io_geo::geojson::{GeometryEnum, Position};
use tpt_io_geo::json::Json;

fn sample(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/samples/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn property<'a>(f: &'a tpt_io_geo::geojson::Feature, key: &str) -> Option<&'a Json> {
    f.properties.as_object()?.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

#[test]
fn reads_points_with_typed_attributes() {
    let fc = read(&sample("fgb_points.fgb")).unwrap();
    assert_eq!(fc.features.len(), 3);
    let first = &fc.features[0];
    assert_eq!(
        first.geometry.as_ref().unwrap().geometry,
        GeometryEnum::Point(Position(vec![174.78, -41.29]))
    );
    assert_eq!(property(first, "name"), Some(&Json::String("Wellington".to_string())));
    assert_eq!(property(first, "count"), Some(&Json::Number(215000.0)));
    assert_eq!(property(first, "ratio"), Some(&Json::Number(1.5)));

    let last = &fc.features[2];
    assert_eq!(property(last, "name"), Some(&Json::String("Beta".to_string())));
    assert_eq!(property(last, "count"), Some(&Json::Number(-2.0)));
    assert_eq!(property(last, "ratio"), Some(&Json::Number(-0.125)));
}

#[test]
fn reads_lines_polygons_with_holes_and_multipolygons() {
    // With a spatial index GDAL writes features in Hilbert-curve order, so find each one by its label.
    let fc = read(&sample("fgb_mixed.fgb")).unwrap();
    assert_eq!(fc.features.len(), 3);
    let by_label = |label: &str| {
        fc.features
            .iter()
            .find(|f| property(f, "label") == Some(&Json::String(label.to_string())))
            .and_then(|f| f.geometry.as_ref())
            .map(|g| g.geometry.clone())
            .unwrap()
    };

    match by_label("road") {
        GeometryEnum::LineString(ps) => assert_eq!(ps.len(), 3),
        other => panic!("expected a line, got {other:?}"),
    }
    match by_label("park") {
        GeometryEnum::Polygon(rings) => {
            assert_eq!(rings.len(), 2, "exterior ring and one hole");
            assert_eq!(rings[0].len(), 5);
            assert_eq!(rings[1].len(), 5);
        }
        other => panic!("expected a polygon, got {other:?}"),
    }
    match by_label("islands") {
        GeometryEnum::MultiPolygon(polys) => assert_eq!(polys.len(), 2),
        other => panic!("expected a multipolygon, got {other:?}"),
    }
}

#[test]
fn rejects_files_that_are_not_flatgeobuf() {
    assert!(read(b"plain text, not FlatGeobuf").is_err());
    let full = sample("fgb_points.fgb");
    assert!(read(&full[..full.len() / 2]).is_err());
}
