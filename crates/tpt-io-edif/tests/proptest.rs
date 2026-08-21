//! Property-based tests (proptest) for EDIF invariants.

use proptest::prelude::*;
use tpt_io_core::Reader;
use tpt_io_edif::flatten::flatten;
use tpt_io_edif::model::{Cell, Direction, Edif, Library, Net, Port, PortRef, View, ViewType};
use tpt_io_edif::parser::EdifReader;
use tpt_io_edif::writer::serialize;

type NetMembers = Vec<(Option<String>, String)>;

/// Build a single-flat-cell EDIF model with the given port names and nets, then
/// serialize it to guaranteed-balanced bytes.
fn build_model(ports: &[String], nets: &[(String, NetMembers)]) -> Edif {
    let view = View {
        name: "v".into(),
        view_type: ViewType::Netlist,
        interface: ports
            .iter()
            .map(|p| Port { name: p.clone(), direction: Some(Direction::In) })
            .collect(),
        instances: vec![],
        nets: nets
            .iter()
            .map(|(nname, members)| Net {
                name: nname.clone(),
                members: members
                    .iter()
                    .map(|(inst, port)| PortRef { port: port.clone(), instance: inst.clone() })
                    .collect(),
            })
            .collect(),
        properties: vec![],
    };
    Edif {
        name: "t".into(),
        version: (2, 0, 0),
        libraries: vec![Library {
            name: "lib".into(),
            cells: vec![Cell {
                name: "c".into(),
                cell_type: Some("GENERIC".into()),
                views: vec![view],
            }],
        }],
        design: Some(tpt_io_edif::model::Design {
            name: "d".into(),
            cell_ref: "c".into(),
            library_ref: "lib".into(),
        }),
    }
}

fn ident() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_]{0,8}".prop_map(|s| s)
}

proptest! {
    #[test]
    fn flat_design_no_leaves(ports in proptest::collection::vec(ident(), 1..6)) {
        let nets: Vec<(String, NetMembers)> = vec![
            ("n1".to_string(), ports.iter().map(|p| (None, p.clone())).collect()),
        ];
        let model = build_model(&ports, &nets);
        let doc = serialize(&model);
        let reader = EdifReader::new();
        let edif = reader.parse(&doc).expect("parse");
        let flat = flatten(&edif).expect("flatten");
        prop_assert_eq!(flat.leaves.len(), 1, "flat design's single cell is the only primitive");
        prop_assert_eq!(flat.nets.len(), 1, "all ports share one net");
        prop_assert_eq!(&flat.leaves[0].path, "", "top cell is the primitive at root");
        prop_assert_eq!(&flat.leaves[0].cell, "c");
        let mut count = 0;
        for p in &ports {
            if flat.nets[0].members.iter().any(|m| m.path.is_empty() && &m.port == p) {
                count += 1;
            }
        }
        prop_assert_eq!(count, ports.len());
    }

    #[test]
    fn round_trip_stable(ports in proptest::collection::vec(ident(), 1..5)) {
        let nets: Vec<(String, NetMembers)> = vec![
            ("n1".to_string(), ports.iter().map(|p| (None, p.clone())).collect()),
        ];
        let model = build_model(&ports, &nets);
        let doc = serialize(&model);
        let reader = EdifReader::new();
        let a = reader.parse(&doc).expect("parse");
        let b = reader.parse(&serialize(&a)).expect("reparse");
        prop_assert_eq!(a, b);
    }
}
