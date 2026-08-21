//! Property-based tests (proptest) for EDIF invariants.

use proptest::prelude::*;
use tpt_io_core::Reader;
use tpt_io_edif::flatten::flatten;
use tpt_io_edif::parser::EdifReader;
use tpt_io_edif::writer::serialize;

type NetMembers = Vec<(Option<String>, String)>;

/// Build a single-flat-cell EDIF document with the given port names and nets.
fn build_flat(ports: &[String], nets: &[(String, NetMembers)]) -> Vec<u8> {
    let mut s = String::from(
        "(edif t (edifVersion 2 0 0) (edifLevel 0) (library lib (cell c (view v (viewType NETLIST) (interface ",
    );
    for p in ports {
        s.push_str(&format!("(port {p} (direction IN)) "));
    }
    s.push_str(") (contents ");
    for (nname, members) in nets {
        s.push_str(&format!("(net {nname} (joined "));
        for (inst, port) in members {
            match inst {
                Some(i) => s.push_str(&format!("(portRef {port} (instanceRef {i})) ")),
                None => s.push_str(&format!("(portRef {port} ")),
            }
        }
        s.push_str(")) ");
    }
    s.push_str("))))) (design d (cellRef c (libraryRef lib)))");
    s.into_bytes()
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
        let doc = build_flat(&ports, &nets);
        let reader = EdifReader::new();
        let edif = reader.parse(&doc).expect("parse");
        let flat = flatten(&edif).expect("flatten");
        prop_assert_eq!(flat.leaves.len(), 0, "flat design has no primitive instances");
        prop_assert_eq!(flat.nets.len(), 1, "all ports share one net");
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
        let doc = build_flat(&ports, &nets);
        let reader = EdifReader::new();
        let a = reader.parse(&doc).expect("parse");
        let b = reader.parse(&serialize(&a)).expect("reparse");
        prop_assert_eq!(a, b);
    }
}
