//! Round-trip tests: parse → serialize → parse → equality, plus cross-format
//! consistency checks across the parsers.

use tpt_io_core::Reader;
use tpt_io_geo::crs::{project, reproject, Crs, CrsKind};
use tpt_io_geo::geojson::{parse_str, to_string, FeatureCollection, GeoJsonReader, GeometryEnum};
use tpt_io_geo::geotiff::{build_test_geotiff, parse_geotiff, Endian};
use tpt_io_geo::shapefile::{parse_dbf, parse_shp, ShapeGeometry, ShapeType};
use tpt_io_geo::stream::GeoJsonFeatureStream;

#[test]
fn geojson_round_trip() {
    let src = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature",
         "geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10],[0,0]]]},
         "properties":{"name":"square","id":7}},
        {"type":"Feature",
         "geometry":{"type":"Point","coordinates":[174.0,-41.0]},
         "properties":{"name":"pt"}}
    ]}"#;
    let fc = parse_str(src).unwrap();
    let out = to_string(&fc);
    let fc2 = parse_str(&out).unwrap();
    assert_eq!(fc, fc2);
    // Streaming reader yields the same number of features.
    let mut stream = GeoJsonFeatureStream::new(out.as_bytes()).unwrap();
    let streamed = stream.collect().unwrap();
    assert_eq!(streamed.len(), 2);
}

#[test]
fn geojson_reader_trait() {
    let src = br#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":{"type":"Point","coordinates":[1,2]},"properties":{}}
    ]}"#;
    let fc: FeatureCollection = GeoJsonReader::new().parse(src).unwrap();
    assert_eq!(fc.features.len(), 1);
    match &fc.features[0].geometry.as_ref().unwrap().geometry {
        GeometryEnum::Point(p) => assert_eq!(p.longitude(), 1.0),
        _ => panic!("expected point"),
    }
}

#[test]
fn geotiff_round_trip_metadata() {
    // Build a GeoTIFF from parameters, parse it, then rebuild from the parsed
    // metadata and confirm the extracted CRS/extent is preserved.
    let bytes =
        build_test_geotiff(Endian::Little, 64, 128, 300_000.0, 5_000_000.0, [5.0, 5.0, 0.0], 32759);
    let meta = parse_geotiff(&bytes).unwrap();
    assert_eq!(meta.epsg_code, Some(32759));
    assert!(meta.is_projected);
    assert_eq!(meta.width, 64);
    assert_eq!(meta.height, 128);
    let bbox = meta.bbox.unwrap();
    assert!((bbox[0] - 300_000.0).abs() < 1e-6);
    assert!((bbox[2] - 300_320.0).abs() < 1e-6);

    // Rebuild from the same parameters and confirm identical metadata.
    let bytes2 =
        build_test_geotiff(Endian::Big, 64, 128, 300_000.0, 5_000_000.0, [5.0, 5.0, 0.0], 32759);
    let meta2 = parse_geotiff(&bytes2).unwrap();
    assert_eq!(meta.epsg_code, meta2.epsg_code);
    assert_eq!(meta.bbox, meta2.bbox);
}

#[test]
fn reproject_utm_zone_round_trip() {
    // A point in WGS84 lon/lat, projected into its UTM zone and back, must
    // return to the original lon/lat.
    let lon = 174.0;
    let lat = -41.0;
    let zone = tpt_io_geo::crs::utm_zone(lon);
    let epsg = 32700u32 + zone; // southern hemisphere
    let crs = Crs::from_epsg(epsg).unwrap();
    let (x, y) = project(&crs, lon, lat).unwrap();
    let (lon2, lat2) = reproject(&crs, &Crs::wgs84(), x, y).unwrap();
    assert!((lon2 - lon).abs() < 1e-7, "lon {lon2} vs {lon}");
    assert!((lat2 - lat).abs() < 1e-7, "lat {lat2} vs {lat}");
    assert_eq!(crs.kind, CrsKind::Projected);
}

#[test]
fn shapefile_polygon_parse() {
    // Minimal polygon .shp built inline.
    let shp = build_polygon_shp();
    let sf = parse_shp(&shp).unwrap();
    assert_eq!(sf.records.len(), 1);
    assert_eq!(sf.records[0].shape_type, ShapeType::Polygon);
    match &sf.records[0].geometry {
        ShapeGeometry::Parts { parts, .. } => {
            assert_eq!(parts.len(), 1);
            assert_eq!(parts[0].len(), 5);
        }
        _ => panic!("expected parts"),
    }
}

#[test]
fn dbf_parse_basic() {
    let dbf = build_dbf();
    let table = parse_dbf(&dbf).unwrap();
    assert_eq!(table.num_records, 2);
    assert_eq!(table.fields[0].name, "ID");
    assert_eq!(table.records[0][0], "1");
    assert_eq!(table.records[1][0], "2");
}

/// Build a minimal polygon .shp (matches the layout documented in `parse_shp`).
fn build_polygon_shp() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&9994i32.to_be_bytes());
    for _ in 0..5 {
        b.extend_from_slice(&0i32.to_be_bytes());
    }
    let content_words = 64i32;
    let file_len = 50 + 4 + content_words;
    b.extend_from_slice(&(file_len as u32).to_be_bytes());
    b.extend_from_slice(&1000i32.to_le_bytes());
    b.extend_from_slice(&5i32.to_le_bytes());
    for v in [0.0f64, 0.0, 10.0, 10.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    for _ in 0..4 {
        b.extend_from_slice(&0.0f64.to_le_bytes());
    }
    b.extend_from_slice(&1i32.to_be_bytes());
    b.extend_from_slice(&content_words.to_be_bytes());
    b.extend_from_slice(&5i32.to_le_bytes());
    for v in [0.0f64, 0.0, 10.0, 10.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&1i32.to_le_bytes());
    b.extend_from_slice(&5i32.to_le_bytes());
    b.extend_from_slice(&0i32.to_le_bytes());
    for (x, y) in [
        (0.0f64, 0.0f64),
        (10.0f64, 0.0f64),
        (10.0f64, 10.0f64),
        (0.0f64, 10.0f64),
        (0.0f64, 0.0f64),
    ] {
        b.extend_from_slice(&x.to_le_bytes());
        b.extend_from_slice(&y.to_le_bytes());
    }
    b
}

/// Build a minimal dBASE table with one numeric field and two rows.
fn build_dbf() -> Vec<u8> {
    let mut b = Vec::new();
    b.push(0x03);
    b.extend_from_slice(&[0, 0, 0]);
    b.extend_from_slice(&2u32.to_le_bytes());
    b.extend_from_slice(&(32u16 + 32 + 1).to_le_bytes());
    b.extend_from_slice(&20u16.to_le_bytes());
    b.extend_from_slice(&[0u8; 20]);
    let mut name = [0u8; 11];
    name[..2].copy_from_slice(b"ID");
    b.extend_from_slice(&name);
    b.push(b'N');
    b.extend_from_slice(&[0u8; 4]);
    b.push(10);
    b.push(0);
    b.extend_from_slice(&[0u8; 14]);
    b.push(0x0d);
    for v in ["1", "2"] {
        b.push(b' ');
        let mut cell = [b' '; 10];
        let bytes = v.as_bytes();
        cell[..bytes.len()].copy_from_slice(bytes);
        b.extend_from_slice(&cell);
        b.extend_from_slice(&[b' '; 9]);
    }
    b
}
