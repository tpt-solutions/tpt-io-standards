//! Integration tests for EDIF parsing, flattening, and round-trip.
//!
//! Sample documents are generated from the typed model via [`serialize`], which
//! produces guaranteed-balanced EDIF, then re-parsed — this exercises both the
//! writer and the parser and avoids hand-counting parentheses.

use tpt_io_core::{Reader, Validator};
use tpt_io_edif::flatten::{flatten, FlatNetlist};
use tpt_io_edif::geometry::Point;
use tpt_io_edif::geometry::Shape;
use tpt_io_edif::model::{
    Cell, Direction, Edif, Instance, Library, Net, Port, PortRef, View, ViewType,
};
use tpt_io_edif::parser::{EdifReader, EdifValidator};
use tpt_io_edif::writer::serialize;

fn sample_edif() -> Edif {
    let prim = Cell {
        name: "prim".into(),
        cell_type: Some("GENERIC".into()),
        views: vec![View {
            name: "v".into(),
            view_type: ViewType::Netlist,
            interface: vec![
                Port { name: "p".into(), direction: Some(Direction::In) },
                Port { name: "q".into(), direction: Some(Direction::Out) },
            ],
            instances: vec![],
            nets: vec![],
            properties: vec![],
            geometry: vec![],
        }],
    };
    let sub = Cell {
        name: "sub".into(),
        cell_type: Some("GENERIC".into()),
        views: vec![View {
            name: "v".into(),
            view_type: ViewType::Netlist,
            interface: vec![
                Port { name: "a".into(), direction: Some(Direction::In) },
                Port { name: "z".into(), direction: Some(Direction::Out) },
            ],
            instances: vec![Instance {
                name: "u1".into(),
                cell_ref: "prim".into(),
                view_ref: "v".into(),
                library_ref: "lib".into(),
            }],
            nets: vec![
                Net {
                    name: "n1".into(),
                    members: vec![
                        PortRef { port: "a".into(), instance: None },
                        PortRef { port: "p".into(), instance: Some("u1".into()) },
                    ],
                },
                Net {
                    name: "n2".into(),
                    members: vec![
                        PortRef { port: "q".into(), instance: Some("u1".into()) },
                        PortRef { port: "z".into(), instance: None },
                    ],
                },
            ],
            properties: vec![],
            geometry: vec![],
        }],
    };
    let top = Cell {
        name: "top".into(),
        cell_type: Some("GENERIC".into()),
        views: vec![View {
            name: "v".into(),
            view_type: ViewType::Netlist,
            interface: vec![
                Port { name: "in".into(), direction: Some(Direction::In) },
                Port { name: "out".into(), direction: Some(Direction::Out) },
            ],
            instances: vec![Instance {
                name: "s1".into(),
                cell_ref: "sub".into(),
                view_ref: "v".into(),
                library_ref: "lib".into(),
            }],
            nets: vec![
                Net {
                    name: "g1".into(),
                    members: vec![
                        PortRef { port: "in".into(), instance: None },
                        PortRef { port: "a".into(), instance: Some("s1".into()) },
                    ],
                },
                Net {
                    name: "g2".into(),
                    members: vec![
                        PortRef { port: "z".into(), instance: Some("s1".into()) },
                        PortRef { port: "out".into(), instance: None },
                    ],
                },
            ],
            properties: vec![],
            geometry: vec![],
        }],
    };

    Edif {
        name: "top".into(),
        version: (2, 0, 0),
        libraries: vec![Library { name: "lib".into(), cells: vec![prim, sub, top] }],
        design: Some(tpt_io_edif::model::Design {
            name: "d".into(),
            cell_ref: "top".into(),
            library_ref: "lib".into(),
        }),
    }
}

#[test]
fn parse_generated_document() {
    let bytes = serialize(&sample_edif());
    let reader = EdifReader::new();
    let edif = reader.parse(&bytes).expect("parse succeeds");
    assert_eq!(edif.version, (2, 0, 0));
    assert_eq!(edif.libraries.len(), 1);
    assert_eq!(edif.libraries[0].cells.len(), 3);
}

#[test]
fn flatten_merges_instance_ports() {
    let bytes = serialize(&sample_edif());
    let reader = EdifReader::new();
    let edif = reader.parse(&bytes).expect("parse succeeds");
    let flat: FlatNetlist = flatten(&edif).expect("flatten succeeds");

    assert_eq!(flat.external_ports, vec!["in".to_string(), "out".to_string()]);
    // Primitive `prim` has no internal net, so after expansion only leaf `s1.u1` remains
    // and `in`/`out` end up on two distinct nets.
    assert_eq!(flat.leaves.len(), 1);
    assert_eq!(flat.leaves[0].path, "s1.u1");
    assert_eq!(flat.nets.len(), 2);

    let has = |name: &str| {
        flat.nets.iter().any(|n| n.members.iter().any(|m| m.path.is_empty() && m.port == name))
    };
    assert!(has("in"));
    assert!(has("out"));
}

#[test]
fn round_trip_preserves_structure() {
    let model = sample_edif();
    let bytes = serialize(&model);
    let reader = EdifReader::new();
    let edif2 = reader.parse(&bytes).expect("re-parse succeeds");
    assert_eq!(model, edif2);
}

#[test]
fn validator_accepts_well_formed() {
    let bytes = serialize(&sample_edif());
    let reader = EdifReader::new();
    let edif = reader.parse(&bytes).expect("parse succeeds");
    EdifValidator::new().validate_syntactic(&edif).expect("syntactic ok");
    EdifValidator::new().validate_semantic(&edif).expect("semantic ok");
}

#[test]
fn validator_rejects_undefined_design_cell() {
    let bad =
        b"(edif x (edifVersion 2 0 0) (edifLevel 0) (design d (cellRef nope (libraryRef lib))))";
    let reader = EdifReader::new();
    let edif = reader.parse(bad).expect("parse succeeds");
    assert!(EdifValidator::new().validate_semantic(&edif).is_err());
}

#[test]
fn golden_sample_parses() {
    let data = include_bytes!("samples/hierarchical.edif");
    let reader = EdifReader::new();
    let edif = reader.parse(data).expect("golden sample parses");
    assert!(!edif.libraries.is_empty());
    let _ = flatten(&edif);
}

#[test]
fn golden_geometry_extraction() {
    let data = include_bytes!("samples/schematic_layout.edif");
    let reader = EdifReader::new();
    let edif = reader.parse(data).expect("golden geometry sample parses");

    // Schematic figures: rectangle + label + path.
    let schematic = edif.geometry_for(ViewType::Schematic);
    assert_eq!(schematic.len(), 3);
    assert!(edif.geometry_for(ViewType::Layout).len() == 3);

    // The rect (layout) maps to a Rectangle shape.
    let layout = edif.find_cell("lib", "inv").unwrap().layout_view().unwrap();
    assert!(matches!(
        &layout.geometry[0].shape,
        Shape::Rectangle { a, b } if *a == Point { x: 0.0, y: 0.0 } && *b == Point { x: 200.0, y: 100.0 }
    ));
    // Layout carries a polygon and a circle too.
    assert!(layout.geometry.iter().any(|f| matches!(f.shape, Shape::Polygon(_))));
    assert!(layout
        .geometry
        .iter()
        .any(|f| matches!(f.shape, Shape::Circle { diameter, .. } if diameter == 60.0)));

    // Schematic carries a label with text.
    let schematic_view = edif.find_cell("lib", "inv").unwrap().schematic_view().unwrap();
    assert!(schematic_view
        .geometry
        .iter()
        .any(|f| matches!(&f.shape, Shape::Label { text, .. } if text == "INV")));

    // Round-trip preserves the geometry.
    let bytes = serialize(&edif);
    let reparsed = reader.parse(&bytes).expect("re-parse");
    assert_eq!(reparsed.geometry_for(ViewType::Schematic).len(), 3);
    assert_eq!(reparsed.geometry_for(ViewType::Layout).len(), 3);
}
