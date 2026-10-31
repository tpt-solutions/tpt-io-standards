//! Round-trip and error tests for the WKT, WKB, KML/KMZ and GPX readers and writers.

use tpt_io_geo::geojson::{
    Feature, FeatureCollection, FeatureId, Geometry, GeometryEnum, Position,
};
use tpt_io_geo::json::Json;
use tpt_io_geo::{gpx, kml, wkb, wkt, GeoError};

fn plain(geometry: GeometryEnum) -> Geometry {
    Geometry { geometry, bbox: None, foreign_members: Vec::new() }
}

fn pos(lon: f64, lat: f64) -> Position {
    Position(vec![lon, lat])
}

fn pos3(lon: f64, lat: f64, z: f64) -> Position {
    Position(vec![lon, lat, z])
}

fn feature(geometry: GeometryEnum, name: Option<&str>) -> Feature {
    Feature {
        geometry: Some(plain(geometry)),
        properties: match name {
            Some(n) => Json::Object(vec![("name".to_string(), Json::String(n.to_string()))]),
            None => Json::Null,
        },
        id: None,
        bbox: None,
        foreign_members: Vec::new(),
    }
}

fn collection(features: Vec<Feature>) -> FeatureCollection {
    FeatureCollection { features, bbox: None, foreign_members: Vec::new() }
}

// --- WKT ----------------------------------------------------------------------------------------

#[test]
fn wkt_reads_every_simple_type() {
    let point = wkt::parse_wkt("POINT (174 -41)").unwrap();
    assert_eq!(point.geometry, GeometryEnum::Point(pos(174.0, -41.0)));

    let line = wkt::parse_wkt("LINESTRING (0 0, 1 1, 2 0)").unwrap();
    assert_eq!(
        line.geometry,
        GeometryEnum::LineString(vec![pos(0.0, 0.0), pos(1.0, 1.0), pos(2.0, 0.0)])
    );

    let poly = wkt::parse_wkt("POLYGON ((0 0, 4 0, 4 4, 0 0), (1 1, 2 1, 2 2, 1 1))").unwrap();
    match poly.geometry {
        GeometryEnum::Polygon(rings) => assert_eq!(rings.len(), 2),
        other => panic!("expected a polygon, got {other:?}"),
    }
}

#[test]
fn wkt_accepts_both_multipoint_forms_and_z_tags() {
    let a = wkt::parse_wkt("MULTIPOINT (1 2, 3 4)").unwrap();
    let b = wkt::parse_wkt("MULTIPOINT ((1 2), (3 4))").unwrap();
    assert_eq!(a, b);

    let z = wkt::parse_wkt("POINT Z (1 2 3)").unwrap();
    assert_eq!(z.geometry, GeometryEnum::Point(pos3(1.0, 2.0, 3.0)));
}

#[test]
fn wkt_empty_bodies_and_collections() {
    assert_eq!(
        wkt::parse_wkt("LINESTRING EMPTY").unwrap().geometry,
        GeometryEnum::LineString(Vec::new())
    );
    assert!(matches!(wkt::parse_wkt("POINT EMPTY"), Err(GeoError::Unsupported(_))));

    let gc = wkt::parse_wkt("GEOMETRYCOLLECTION (POINT (1 2), LINESTRING (0 0, 1 1))").unwrap();
    match gc.geometry {
        GeometryEnum::GeometryCollection(gs) => assert_eq!(gs.len(), 2),
        other => panic!("expected a collection, got {other:?}"),
    }
}

#[test]
fn wkt_round_trips_with_and_without_elevation() {
    let flat = wkt::parse_wkt("MULTILINESTRING ((0 0, 1 1), (2 2, 3 3))").unwrap();
    assert_eq!(wkt::parse_wkt(&wkt::to_wkt(&flat)).unwrap(), flat);

    let tall = wkt::parse_wkt("POLYGON Z ((0 0 5, 4 0 5, 4 4 6, 0 0 5))").unwrap();
    assert_eq!(wkt::parse_wkt(&wkt::to_wkt(&tall)).unwrap(), tall);
}

#[test]
fn wkt_rejects_measures_trailing_text_and_unknown_types() {
    assert!(matches!(wkt::parse_wkt("POINT M (1 2 3)"), Err(GeoError::Unsupported(_))));
    assert!(matches!(wkt::parse_wkt("POINT (1 2) extra"), Err(GeoError::Core(_))));
    assert!(matches!(wkt::parse_wkt("CIRCLE (1 2)"), Err(GeoError::Core(_))));
    assert!(matches!(wkt::parse_wkt("POINT (1"), Err(GeoError::Core(_))));
}

// --- WKB ----------------------------------------------------------------------------------------

#[test]
fn wkb_reads_a_little_endian_point() {
    // POINT(1 2): byte order 1, type 1, then two little-endian doubles.
    let bytes = [
        0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x40,
    ];
    assert_eq!(wkb::parse_wkb(&bytes).unwrap().geometry, GeometryEnum::Point(pos(1.0, 2.0)));
}

#[test]
fn wkb_reads_a_big_endian_point() {
    // POINT(2 3) in big-endian order.
    let bytes =
        [0x00, 0x00, 0x00, 0x00, 0x01, 0x40, 0, 0, 0, 0, 0, 0, 0, 0x40, 0x08, 0, 0, 0, 0, 0, 0];
    assert_eq!(wkb::parse_wkb(&bytes).unwrap().geometry, GeometryEnum::Point(pos(2.0, 3.0)));
}

#[test]
fn wkb_round_trips_collections_and_elevations() {
    let g = plain(GeometryEnum::GeometryCollection(vec![
        plain(GeometryEnum::Point(pos3(1.0, 2.0, 3.0))),
        plain(GeometryEnum::MultiPolygon(vec![vec![vec![
            pos3(0.0, 0.0, 1.0),
            pos3(4.0, 0.0, 1.0),
            pos3(0.0, 0.0, 1.0),
        ]]])),
    ]));
    assert_eq!(wkb::parse_wkb(&wkb::to_wkb(&g)).unwrap(), g);
}

#[test]
fn wkb_reads_the_postgis_srid_flag() {
    // EWKB POINT with SRID 4326: type 0x20000001, the SRID, then the coordinates.
    let mut bytes = vec![0x01];
    bytes.extend_from_slice(&0x2000_0001u32.to_le_bytes());
    bytes.extend_from_slice(&4326u32.to_le_bytes());
    bytes.extend_from_slice(&1.5f64.to_le_bytes());
    bytes.extend_from_slice(&(-2.5f64).to_le_bytes());
    assert_eq!(wkb::parse_wkb(&bytes).unwrap().geometry, GeometryEnum::Point(pos(1.5, -2.5)));
}

#[test]
fn wkb_rejects_bad_headers_and_huge_counts() {
    assert!(matches!(wkb::parse_wkb(&[0x07, 1, 0, 0, 0]), Err(GeoError::Core(_))));
    assert!(matches!(wkb::parse_wkb(&[0x01, 0x42, 0, 0, 0]), Err(GeoError::Core(_))));
    // A LineString claiming four billion positions but holding none.
    let mut huge = vec![0x01];
    huge.extend_from_slice(&2u32.to_le_bytes());
    huge.extend_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(wkb::parse_wkb(&huge), Err(GeoError::Core(_))));
    assert!(matches!(wkb::parse_wkb(&[0x01, 0x01, 0, 0]), Err(GeoError::Core(_))));
}

// --- KML and KMZ --------------------------------------------------------------------------------

#[test]
fn kml_reads_placemarks_with_names_descriptions_and_extended_data() {
    let doc = br#"<?xml version="1.0"?>
<kml xmlns="http://www.opengis.net/kml/2.2"><Document>
  <Placemark id="p1">
    <name>Harbour</name>
    <description>Ferry terminal</description>
    <ExtendedData><Data name="depth"><value>12</value></Data></ExtendedData>
    <Point><coordinates>174.78,-41.28,0</coordinates></Point>
  </Placemark>
  <Placemark>
    <name>Coast</name>
    <LineString><coordinates>174.0,-41.0 174.5,-41.5</coordinates></LineString>
  </Placemark>
  <Placemark>
    <Polygon>
      <outerBoundaryIs><LinearRing><coordinates>0,0 4,0 4,4 0,0</coordinates></LinearRing></outerBoundaryIs>
      <innerBoundaryIs><LinearRing><coordinates>1,1 2,1 2,2 1,1</coordinates></LinearRing></innerBoundaryIs>
    </Polygon>
  </Placemark>
</Document></kml>"#;
    let fc = kml::parse(doc).unwrap();
    assert_eq!(fc.features.len(), 3);

    let harbour = &fc.features[0];
    assert_eq!(harbour.id, Some(FeatureId::String("p1".to_string())));
    assert_eq!(
        harbour.geometry.as_ref().unwrap().geometry,
        GeometryEnum::Point(pos3(174.78, -41.28, 0.0))
    );
    assert_eq!(
        harbour.properties.as_object().unwrap().iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
        vec!["name", "description", "depth"]
    );

    match &fc.features[2].geometry.as_ref().unwrap().geometry {
        GeometryEnum::Polygon(rings) => assert_eq!(rings.len(), 2),
        other => panic!("expected a polygon, got {other:?}"),
    }
}

#[test]
fn kml_write_then_read_keeps_points_lines_polygons_and_names() {
    let fc = collection(vec![
        feature(GeometryEnum::Point(pos3(1.0, 2.0, 3.0)), Some("A & B")),
        feature(GeometryEnum::LineString(vec![pos(0.0, 0.0), pos(1.0, 1.0)]), None),
        feature(
            GeometryEnum::Polygon(vec![vec![
                pos(0.0, 0.0),
                pos(4.0, 0.0),
                pos(4.0, 4.0),
                pos(0.0, 0.0),
            ]]),
            Some("Zone"),
        ),
    ]);
    let back = kml::parse(kml::write(&fc).as_bytes()).unwrap();
    assert_eq!(back.features[0].properties, fc.features[0].properties);
    assert_eq!(back.features[0].geometry, fc.features[0].geometry);
    assert_eq!(back.features[1].geometry, fc.features[1].geometry);
    assert_eq!(back.features[2].geometry, fc.features[2].geometry);
}

#[test]
fn kml_bad_coordinate_is_an_error() {
    let doc =
        br#"<kml><Placemark><Point><coordinates>abc,def</coordinates></Point></Placemark></kml>"#;
    assert!(matches!(kml::parse(doc), Err(GeoError::Core(_))));
}

#[cfg(feature = "kmz")]
#[test]
fn kmz_round_trip_and_missing_doc_kml() {
    let fc = collection(vec![feature(GeometryEnum::Point(pos(5.0, 6.0)), Some("Kmz point"))]);
    let archive = kml::write_kmz(&fc);
    assert_eq!(kml::parse_kmz(&archive).unwrap(), kml::parse(kml::write(&fc).as_bytes()).unwrap());

    let empty = tpt_io_core::zip::write_stored(&[("other.txt", b"x".as_slice())]);
    assert!(matches!(kml::parse_kmz(&empty), Err(GeoError::Core(_))));
}

// --- GPX ----------------------------------------------------------------------------------------

#[test]
fn gpx_reads_waypoints_routes_and_tracks() {
    let doc = br#"<?xml version="1.0"?>
<gpx version="1.1" creator="test" xmlns="http://www.topografix.com/GPX/1/1">
  <wpt lat="-41.28" lon="174.78"><ele>12.5</ele><name>Summit</name></wpt>
  <rte><name>Route</name><rtept lat="-41.0" lon="174.0"/><rtept lat="-41.5" lon="174.5"/></rte>
  <trk><name>Walk</name>
    <trkseg><trkpt lat="1" lon="2"/><trkpt lat="3" lon="4"/></trkseg>
    <trkseg><trkpt lat="5" lon="6"/></trkseg>
  </trk>
</gpx>"#;
    let fc = gpx::parse(doc).unwrap();
    assert_eq!(fc.features.len(), 3);
    assert_eq!(
        fc.features[0].geometry.as_ref().unwrap().geometry,
        GeometryEnum::Point(pos3(174.78, -41.28, 12.5))
    );
    match &fc.features[1].geometry.as_ref().unwrap().geometry {
        GeometryEnum::LineString(ps) => assert_eq!(ps.len(), 2),
        other => panic!("expected a route line, got {other:?}"),
    }
    match &fc.features[2].geometry.as_ref().unwrap().geometry {
        GeometryEnum::MultiLineString(segs) => assert_eq!(segs.len(), 2),
        other => panic!("expected a track, got {other:?}"),
    }
}

#[test]
fn gpx_write_then_read_keeps_points_and_tracks() {
    let fc = collection(vec![
        feature(GeometryEnum::Point(pos3(1.0, 2.0, 3.0)), Some("Peak")),
        feature(
            GeometryEnum::MultiLineString(vec![
                vec![pos(1.0, 2.0), pos(3.0, 4.0)],
                vec![pos(5.0, 6.0)],
            ]),
            Some("Walk"),
        ),
    ]);
    let back = gpx::parse(gpx::write(&fc).as_bytes()).unwrap();
    assert_eq!(back.features[0].geometry, fc.features[0].geometry);
    assert_eq!(back.features[1].geometry, fc.features[1].geometry);
    assert_eq!(back.features[0].properties, fc.features[0].properties);
}

#[test]
fn gpx_point_without_numeric_position_is_an_error() {
    assert!(matches!(
        gpx::parse(br#"<gpx><wpt lat="north" lon="1"/></gpx>"#),
        Err(GeoError::Core(_))
    ));
}
