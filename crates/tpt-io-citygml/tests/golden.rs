//! Golden-file tests: parse the checked-in CityGML samples under
//! `tests/samples/` and assert their known content.

use tpt_io_citygml::model::{CityGmlVersion, CityObjectKind, Lod, SurfaceClass, ThematicKey};
use tpt_io_citygml::parser::parse_str;

const WELLINGTON_20: &str = include_str!("samples/wellington_20.gml");
const BUILDING_30: &str = include_str!("samples/building_30.gml");

fn surface_count(geom: &tpt_io_citygml::model::Geometry) -> usize {
    geom.surfaces.len() + geom.solids.iter().map(|s| s.surfaces.len()).sum::<usize>()
}

#[test]
fn golden_citygml_20_model() {
    let model = parse_str(WELLINGTON_20).unwrap();

    assert_eq!(model.version, CityGmlVersion::V2_0);
    let crs = model.crs.as_ref().expect("envelope CRS");
    assert_eq!(crs.identifier, "urn:ogc:def:crs:EPSG::2193");
    assert_eq!(crs.epsg_code(), Some(2193));

    assert_eq!(model.objects.len(), 4);
    let kinds: Vec<_> = model.objects.iter().map(|o| o.kind).collect();
    assert_eq!(
        kinds,
        [
            CityObjectKind::Building,
            CityObjectKind::Vegetation,
            CityObjectKind::Water,
            CityObjectKind::Transportation,
        ]
    );
}

#[test]
fn golden_citygml_20_building_attributes_and_lod() {
    let model = parse_str(WELLINGTON_20).unwrap();
    let building = &model.objects[0];

    assert_eq!(building.id.as_deref(), Some("bldg-1"));
    assert_eq!(building.geometries.len(), 1);
    assert_eq!(building.geometries[0].lod, Lod::Lod1);
    // lod1Solid: one composite surface with a single polygon.
    assert_eq!(surface_count(&building.geometries[0]), 1);

    let function = building.attributes.iter().find(|a| a.key == ThematicKey::Function);
    assert_eq!(function.map(|a| a.value.as_str()), Some("1000"));
    let year = building.attributes.iter().find(|a| a.key == ThematicKey::YearBuilt);
    assert_eq!(year.map(|a| a.value.as_str()), Some("1998"));
}

#[test]
fn golden_citygml_20_lod_per_object() {
    let model = parse_str(WELLINGTON_20).unwrap();
    // Vegetation and transportation use lod1, water uses lod0.
    assert_eq!(model.objects[1].geometries[0].lod, Lod::Lod1);
    assert_eq!(model.objects[2].geometries[0].lod, Lod::Lod0);
    assert_eq!(model.objects[3].geometries[0].lod, Lod::Lod1);
}

#[test]
fn golden_citygml_30_model() {
    let model = parse_str(BUILDING_30).unwrap();

    assert_eq!(model.version, CityGmlVersion::V3_0);
    assert_eq!(model.objects.len(), 1);
    let building = &model.objects[0];
    assert_eq!(building.kind, CityObjectKind::Building);

    // One LoD2 geometry per boundary surface, classified by its element name.
    assert_eq!(building.geometries.len(), 2);
    assert!(building.geometries.iter().all(|g| g.lod == Lod::Lod2));
    assert_eq!(building.geometries[0].surfaces[0].class, SurfaceClass::Wall);
    assert_eq!(building.geometries[1].surfaces[0].class, SurfaceClass::Roof);
}

#[cfg(feature = "geo-integration")]
#[test]
fn golden_envelope_crs_reprojects_to_wgs84() {
    use tpt_io_citygml::geo::transform::to_wgs84;
    use tpt_io_citygml::model::Point3;

    // Wellington (NZTM, EPSG:2193): the sample's lower corner sits near the CBD.
    let model = parse_str(WELLINGTON_20).unwrap();
    let crs = model.crs.as_ref().unwrap();
    let pt = Point3 { x: 1748000.0, y: 5426000.0, z: 0.0 };
    let wgs = to_wgs84(&pt, crs).unwrap();

    assert!((wgs.x - 174.77).abs() < 0.05, "lon {}", wgs.x);
    assert!((wgs.y + 41.28).abs() < 0.05, "lat {}", wgs.y);
    assert_eq!(wgs.z, 0.0);
}

#[cfg(feature = "cityjson")]
#[test]
fn golden_citygml_20_cityjson_round_trip() {
    use tpt_io_citygml::cityjson::to_cityjson;

    let model = parse_str(WELLINGTON_20).unwrap();
    let bytes = to_cityjson(&model).unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&bytes).expect("valid JSON");

    assert_eq!(doc["type"], "CityJSON");
    let objects = doc["CityObjects"].as_object().expect("CityObjects object");
    assert_eq!(objects.len(), model.objects.len());
    for id in ["bldg-1", "veg-1", "wtr-1", "road-1"] {
        assert!(objects.contains_key(id), "missing CityObject {id}");
    }
    assert_eq!(objects["bldg-1"]["attributes"]["function"], "1000");
}
