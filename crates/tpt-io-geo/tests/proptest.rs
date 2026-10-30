//! Property-based tests (proptest) for parser and CRS invariants.

use proptest::prelude::*;
use tpt_io_core::Validator;
use tpt_io_geo::crs::{project, reproject, Crs, Datum};
use tpt_io_geo::geojson::{
    to_string, Feature, FeatureCollection, GeoJsonValidator, Geometry, GeometryEnum, Position,
};
use tpt_io_geo::json::Json;
use tpt_io_geo::spatial_index::{point_in_ring, HasBounds, SpatialIndex};

/// Build a small FeatureCollection containing one Point feature at `(lon, lat)`.
fn point_fc(lon: f64, lat: f64) -> FeatureCollection {
    let feature = Feature {
        geometry: Some(Geometry {
            geometry: GeometryEnum::Point(Position(Vec::from([lon, lat]))),
            bbox: None,
            foreign_members: Vec::new(),
        }),
        properties: Json::Object(Vec::new()),
        id: None,
        bbox: None,
        foreign_members: Vec::new(),
    };
    FeatureCollection { features: alloc_vec(feature), bbox: None, foreign_members: Vec::new() }
}

fn alloc_vec<T>(item: T) -> Vec<T> {
    let mut v = Vec::new();
    v.push(item);
    v
}

proptest! {
    #[test]
    fn geojson_point_round_trip(lon in -180.0..180.0, lat in -90.0..90.0) {
        let fc = point_fc(lon, lat);
        let out = to_string(&fc);
        let fc2 = tpt_io_geo::geojson::parse_str(&out).unwrap();
        prop_assert_eq!(fc, fc2);
    }

    #[test]
    fn geojson_valid_coords_pass_validator(lon in -180.0..180.0, lat in -90.0..90.0) {
        let fc = point_fc(lon, lat);
        prop_assert!(GeoJsonValidator.validate_semantic(&fc).is_ok());
    }

    #[test]
    fn utm_reproject_round_trip(lon in -174.0..174.0, lat in -80.0..80.0) {
        let zone = tpt_io_geo::crs::utm_zone(lon);
        let crs = Crs::from_epsg(32700u32 + zone).unwrap();
        let (x, y) = project(&crs, lon, lat).unwrap();
        let (lon2, lat2) = reproject(&crs, &Crs::wgs84(), x, y).unwrap();
        prop_assert!((lon2 - lon).abs() < 1e-6, "lon {} vs {}", lon2, lon);
        prop_assert!((lat2 - lat).abs() < 1e-6, "lat {} vs {}", lat2, lat);
    }

    #[test]
    fn datum_transform_self_is_identity(lon in -180.0..180.0, lat in -90.0..90.0) {
        let (lon2, lat2, _) = tpt_io_geo::crs::transform_datum(lon, lat, 0.0, Datum::Wgs84, Datum::Wgs84);
        prop_assert!((lon2 - lon).abs() < 1e-9);
        prop_assert!((lat2 - lat).abs() < 1e-9);
    }

    #[test]
    fn spatial_index_range_subset(n in 1usize..40) {
        let items: Vec<Box2> = (0..n)
            .map(|i| Box2 { id: i, bbox: [i as f64, i as f64, i as f64 + 0.5, i as f64 + 0.5] })
            .collect();
        let idx = SpatialIndex::build(items);
        let q = [5.0, 5.0, 6.0, 6.0];
        let hits = idx.range(&q);
        prop_assert!(hits.iter().all(|b| b.id == 5 || b.id == 6));
    }

    #[test]
    fn point_in_ring_edges(_px in 0.0..10.0, _py in 0.0..10.0) {
        let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        prop_assert!(point_in_ring(&square, 5.0, 5.0));
        prop_assert!(!point_in_ring(&square, 50.0, 50.0));
    }
}

/// Helper item for spatial-index property tests.
struct Box2 {
    id: usize,
    bbox: [f64; 4],
}

impl HasBounds for Box2 {
    fn bounds(&self) -> [f64; 4] {
        self.bbox
    }
}
