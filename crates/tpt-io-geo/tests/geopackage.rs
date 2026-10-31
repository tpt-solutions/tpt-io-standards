//! Tests for the GeoPackage reader, against `tests/samples/cities.gpkg` (written by `gen_gpkg.py`).

use tpt_io_geo::geojson::{FeatureId, GeometryEnum, Position};
use tpt_io_geo::geopackage::{layers, read_features};
use tpt_io_geo::json::Json;
use tpt_io_geo::GeoError;

fn sample() -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/samples/cities.gpkg")).unwrap()
}

fn property<'a>(f: &'a tpt_io_geo::geojson::Feature, key: &str) -> Option<&'a Json> {
    f.properties.as_object()?.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

#[test]
fn lists_the_feature_table_and_its_geometry_column() {
    let found = layers(&sample()).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].table, "cities");
    assert_eq!(found[0].geometry_column, "geom");
    assert_eq!(found[0].geometry_type, "POINT");
    assert_eq!(found[0].srs_id, 4326);
}

#[test]
fn reads_every_row_as_a_feature_in_rowid_order() {
    let fc = read_features(&sample(), "cities").unwrap();
    assert_eq!(fc.features.len(), 2002);

    let first = &fc.features[0];
    assert_eq!(first.id, Some(FeatureId::Number(1.0)));
    assert_eq!(
        first.geometry.as_ref().unwrap().geometry,
        GeometryEnum::Point(Position(vec![174.78, -41.29]))
    );
    assert_eq!(property(first, "name"), Some(&Json::String("Wellington".to_string())));
    assert_eq!(property(first, "population"), Some(&Json::Number(215000.0)));
    assert_eq!(property(first, "fid"), Some(&Json::Number(1.0)));
}

#[test]
fn text_longer_than_a_page_is_read_through_overflow_pages() {
    let fc = read_features(&sample(), "cities").unwrap();
    let long = &fc.features[1];
    match property(long, "note") {
        Some(Json::String(s)) => assert_eq!(s.len(), 6000),
        other => panic!("expected the long note, got {other:?}"),
    }
}

#[test]
fn rows_deep_in_a_multi_level_b_tree_are_reached() {
    let fc = read_features(&sample(), "cities").unwrap();
    let last = fc.features.last().unwrap();
    assert_eq!(property(last, "name"), Some(&Json::String("town 2002".to_string())));
    assert_eq!(property(last, "population"), Some(&Json::Number(6006.0)));
    assert_eq!(
        last.geometry.as_ref().unwrap().geometry,
        GeometryEnum::Point(Position(vec![20.02, -20.02]))
    );
}

#[test]
fn missing_tables_and_non_databases_are_errors() {
    assert!(matches!(read_features(&sample(), "no_such_table"), Err(GeoError::Core(_))));
    assert!(matches!(layers(b"plain text, not SQLite"), Err(GeoError::Core(_))));
}
