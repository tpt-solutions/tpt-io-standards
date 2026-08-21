//! Integration tests for EDIF parsing, flattening, and round-trip.

use tpt_io_core::{Reader, Validator};
use tpt_io_edif::flatten::flatten;
use tpt_io_edif::parser::{EdifReader, EdifValidator};
use tpt_io_edif::writer::serialize;

const HIERARCHICAL: &[u8] = br#"
(edif top (edifVersion 2 0 0) (edifLevel 0)
  (library lib
    (cell prim (view v (viewType NETLIST)
      (interface (port p (direction IN)) (port q (direction OUT)))))
    (cell sub (view v (viewType NETLIST)
      (interface (port a (direction IN)) (port z (direction OUT)))
      (contents
        (instance u1 (viewRef v (cellRef prim (libraryRef lib)))
        (net n1 (joined (portRef a) (portRef p (instanceRef u1))))
        (net n2 (joined (portRef q (instanceRef u1)) (portRef z)))))
    (cell top (view v (viewType NETLIST)
      (interface (port in (direction IN)) (port out (direction OUT)))
      (contents
        (instance s1 (viewRef v (cellRef sub (libraryRef lib)))
        (net g1 (joined (portRef in) (portRef a (instanceRef s1))))
        (net g2 (joined (portRef z (instanceRef s1)) (portRef out)))))))
  (design d (cellRef top (libraryRef lib)))
"#;

#[test]
fn parse_hierarchical() {
    let reader = EdifReader::new();
    let edif = reader.parse(HIERARCHICAL).expect("parse succeeds");
    assert_eq!(edif.version, (2, 0, 0));
    assert_eq!(edif.libraries.len(), 1);
    let lib = &edif.libraries[0];
    assert_eq!(lib.cells.len(), 3);
}

#[test]
fn flatten_merges_instance_ports() {
    let reader = EdifReader::new();
    let edif = reader.parse(HIERARCHICAL).expect("parse succeeds");
    let flat = flatten(&edif).expect("flatten succeeds");

    // External ports.
    assert_eq!(flat.external_ports, vec!["in".to_string(), "out".to_string()]);
    // The primitive `prim` carries no internal net, so after full expansion only
    // the single leaf instance `s1.u1` remains and `in`/`out` are on two nets.
    assert_eq!(flat.leaves.len(), 1);
    assert_eq!(flat.leaves[0].path, "s1.u1");
    assert_eq!(flat.nets.len(), 2, "in and out are separate nets");

    let has = |name: &str| {
        flat.nets
            .iter()
            .any(|n| n.members.iter().any(|m| m.path.is_empty() && m.port == name))
    };
    assert!(has("in"));
    assert!(has("out"));
}

#[test]
fn round_trip_preserves_structure() {
    let reader = EdifReader::new();
    let edif = reader.parse(HIERARCHICAL).expect("parse succeeds");
    let bytes = serialize(&edif);
    let edif2 = reader.parse(&bytes).expect("re-parse succeeds");
    assert_eq!(edif, edif2);
}

#[test]
fn validator_accepts_well_formed() {
    let reader = EdifReader::new();
    let edif = reader.parse(HIERARCHICAL).expect("parse succeeds");
    EdifValidator::new()
        .validate_syntactic(&edif)
        .expect("syntactic ok");
    EdifValidator::new()
        .validate_semantic(&edif)
        .expect("semantic ok");
}

#[test]
fn validator_rejects_undefined_design_cell() {
    let bad = b"(edif x (edifVersion 2 0 0) (edifLevel 0) (design d (cellRef nope (libraryRef lib))))";
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
