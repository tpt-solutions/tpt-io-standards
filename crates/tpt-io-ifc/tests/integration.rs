//! Integration tests for `tpt-io-ifc`: golden-file parsing, full round-trip,
//! relationship graph traversal, property set traversal, geometry extraction,
//! spatial navigation, schema detection, and property-based (proptest)
//! invariants.

use tpt_io_core::{Reader, Writer};

use tpt_io_ifc::geometry::GeometryExtractor;
use tpt_io_ifc::graph::RelGraph;
use tpt_io_ifc::model::IfcValue;
use tpt_io_ifc::parser::{parse_str, IfcReader};
use tpt_io_ifc::property::PropertyTraversal;
use tpt_io_ifc::schema::{detect_schema, IfcSchemaVersion};
use tpt_io_ifc::spatial::SpatialNavigator;
use tpt_io_ifc::writer::serialize;

const GOLDEN: &str = include_str!("samples/golden.ifc");

#[test]
fn golden_file_parses_without_error() {
    let file = parse_str(GOLDEN).expect("golden sample must parse");
    // The golden file declares a substantial spatial hierarchy + geometry.
    assert!(file.instance_count() > 50);
    assert_eq!(file.schema(), Some("IFC4"));
    assert_eq!(file.header.file_name.name, "golden.ifc");
}

#[test]
fn golden_round_trips() {
    let original = parse_str(GOLDEN).expect("parse");
    let bytes = serialize(&original).expect("serialize");
    let reparsed = parse_str(core::str::from_utf8(&bytes).unwrap()).expect("reparse");
    assert_eq!(original, reparsed);
}

#[test]
fn golden_schema_detection() {
    let file = parse_str(GOLDEN).expect("parse");
    assert_eq!(detect_schema(&file), IfcSchemaVersion::Ifc4);
}

#[test]
fn golden_relationship_graph() {
    let file = parse_str(GOLDEN).expect("parse");
    let graph = RelGraph::build(&file);

    // Project -> Site -> Building -> Storey aggregations.
    let agg = graph.aggregations().expect("aggregations");
    assert_eq!(agg.len(), 3);

    // One wall contained in one storey.
    let cont = graph.containment().expect("containment");
    assert_eq!(cont.len(), 1);

    // Material association exists.
    let mats = graph.material_associations().expect("materials");
    assert_eq!(mats.len(), 1);

    // Property set definition exists.
    let psets = graph.property_definitions().expect("psets");
    assert_eq!(psets.len(), 1);
}

#[test]
fn golden_property_traversal() {
    let file = parse_str(GOLDEN).expect("parse");
    let trav = PropertyTraversal::new(&file);

    // Find the wall (#32) and resolve its property sets.
    let wall_id = file
        .instances
        .iter()
        .find(|i| i.type_name == "IFCWALL")
        .map(|i| i.id)
        .expect("a wall exists");
    let psets = trav.property_sets(wall_id).expect("property sets");
    assert!(!psets.is_empty());
    assert_eq!(psets[0].name, "Pset_WallCommon");
    let prop_names: Vec<&str> = psets[0].properties.iter().map(|p| p.name.as_str()).collect();
    assert!(prop_names.contains(&"IsExternal"));
    assert!(prop_names.contains(&"LoadBearing"));
}

#[test]
fn golden_spatial_navigation() {
    let file = parse_str(GOLDEN).expect("parse");
    let nav = SpatialNavigator::new(&file);
    let trees = nav.project_tree().expect("project tree");
    assert_eq!(trees.len(), 1);

    // Project -> Site -> Building -> Storey.
    let site = &trees[0].children[0];
    assert_eq!(site.type_name, "IFCSITE");
    let bldg = &site.children[0];
    assert_eq!(bldg.type_name, "IFCBUILDING");
    let storey = &bldg.children[0];
    assert_eq!(storey.type_name, "IFCBUILDINGSTOREY");

    // The wall is contained in the storey.
    let wall_id = file
        .instances
        .iter()
        .find(|i| i.type_name == "IFCWALL")
        .map(|i| i.id)
        .expect("wall");
    assert_eq!(nav.storey_of(wall_id).unwrap(), Some(storey.id));
}

#[test]
fn golden_geometry_extraction() {
    let file = parse_str(GOLDEN).expect("parse");
    let extractor = GeometryExtractor::new(&file);

    // Find the IfcFacetedBrep (#49) and extract it.
    let brep = file
        .instances
        .iter()
        .find(|i| i.type_name == "IFCFACETEDBREP")
        .expect("a brep exists");
    let geom = extractor.extract_item(brep).expect("extract brep");
    match geom {
        tpt_io_ifc::geometry::Geometry::BRep(mesh) => {
            assert!(mesh.vertex_count() > 0);
            assert!(mesh.face_count() > 0);
        }
        other => panic!("expected BRep, got {other:?}"),
    }
}

#[test]
fn reader_round_trips_golden_file() {
    let original = IfcReader::new().parse(GOLDEN.as_bytes()).expect("reader parse");
    let bytes = IfcReader::new().serialize(&original).expect("serialize");
    let reparsed = parse_str(core::str::from_utf8(&bytes).unwrap()).expect("reparse");
    assert_eq!(original, reparsed);
}

#[test]
fn parse_rejects_invalid_header() {
    let bad = "NOT-A-HEADER;\n";
    assert!(parse_str(bad).is_err());
}

#[test]
fn parse_rejects_dangling_reference() {
    let src = "\
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition'),'2;1');
FILE_NAME('t','2024',(),(),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1 = IFCWALL('g',$,'w',$,#99,$);
ENDSEC;
END-ISO-10303-21;
";
    assert!(parse_str(src).is_err());
}

#[test]
fn no_panic_on_arbitrary_bytes() {
    // Parsing arbitrary bytes must never panic; it returns Ok or Err.
    let bytes: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
    let _ = IfcReader::new().parse(&bytes);
}

#[cfg(test)]
mod proptest_roundtrip {
    use super::*;
    use ::proptest::prelude::*;

    /// Build a structurally valid IFC file from a generated list of instance
    /// specs ((id, type, args)). Integer arguments round-trip losslessly.
    fn build_file(specs: Vec<(usize, String, Vec<IfcValue>)>) -> Vec<u8> {
        use tpt_io_ifc::model::{FileDescription, FileName, IfcFile, IfcHeader};

        let instances: Vec<_> = specs
            .into_iter()
            .map(|(id, ty, args)| tpt_io_ifc::model::EntityInstance::new(id, ty, args))
            .collect();
        let file = IfcFile {
            header: IfcHeader {
                description: FileDescription {
                    descriptions: vec!["ViewDefinition".to_string()],
                    implementation_level: "2;1".to_string(),
                },
                file_name: FileName {
                    name: "roundtrip.ifc".to_string(),
                    time_stamp: "2024".to_string(),
                    ..FileName::default()
                },
                schemas: vec!["IFC4".to_string()],
            },
            instances,
        };
        serialize(&file).expect("serialize")
    }

    proptest! {
        #[test]
        fn roundtrip_is_idempotent(
            specs in proptest::collection::vec(
                (1..1000usize, "IFCWALL|IFCCARTESIANPOINT|IFCAXIS2PLACEMENT3D",
                 proptest::collection::vec(
                     proptest::option::of(-9999..9999i64),
                     0..6,
                 )),
                1..12,
            ),
        ) {
            let concrete: Vec<(usize, String, Vec<IfcValue>)> = specs
                .into_iter()
                .map(|(id, ty, args)| {
                    let args: Vec<IfcValue> = args.into_iter().map(|a| match a {
                        Some(n) => IfcValue::Integer(n),
                        None => IfcValue::Derived,
                    }).collect();
                    (id, ty, args)
                })
                .collect();
            let bytes = build_file(concrete);
            let text = core::str::from_utf8(&bytes).expect("utf8");
            let reparsed = parse_str(text).expect("reparse must succeed");
            // Re-serialize and confirm structural equality.
            let bytes2 = serialize(&reparsed).expect("re-serialize");
            prop_assert_eq!(bytes, bytes2);
        }

        #[test]
        fn no_panic_on_arbitrary_ascii(
            data in "[ -~]{0,512}",
        ) {
            // Parsing arbitrary ASCII must never panic.
            let _ = parse_str(&data);
        }
    }
}

