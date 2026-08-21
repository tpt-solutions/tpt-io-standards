//! Integration tests for `tpt-io-spice`: golden-file parsing, full round-trip,
//! temperature-dependent value resolution, subcircuit expansion, and
//! property-based (proptest) invariants.

use tpt_io_core::{Reader, Writer};
use tpt_io_spice::expand::expand_netlist;
use tpt_io_spice::parser::{parse_str, SpiceReader};
use tpt_io_spice::model::{ElementKind, SpiceNetlist};

const SAMPLE: &str = include_str!("samples/simple.sp");

#[test]
fn golden_file_parses() {
    let net = parse_str(SAMPLE).expect("golden sample must parse");
    assert_eq!(net.temperature, Some(27.0));
    assert_eq!(net.title.as_deref(), Some("Simple RC low-pass with an inverter subcircuit"));

    let r = net.elements.iter().find(|e| e.name == "R1").unwrap();
    assert_eq!(r.kind, ElementKind::R);
    assert_eq!(r.nodes, vec!["in", "out"]);
    assert_eq!(r.params, vec!["TC=0.001"]);

    let m = net.models.iter().find(|m| m.name == "nmos").unwrap();
    assert_eq!(m.device, "nmos");
    assert!(m.params.iter().any(|(k, _)| k == "VTO"));

    let sub = net.subckts.iter().find(|s| s.name == "inv").unwrap();
    assert_eq!(sub.params, vec!["w"]);
    assert_eq!(sub.ports, vec!["a", "y", "vdd"]);

    // The X instance resolved its subcircuit name and ports.
    let x = net.elements.iter().find(|e| e.kind == ElementKind::X).unwrap();
    assert_eq!(x.subckt.as_deref(), Some("inv"));
    assert_eq!(x.nodes, vec!["out", "vdd"]);
}

#[test]
fn reader_round_trips_golden_file() {
    let original = parse_str(SAMPLE).expect("parse");
    let serialized = SpiceReader::new().serialize(&original).expect("serialize");
    let reparsed = parse_str(core::str::from_utf8(&serialized).unwrap()).expect("reparse");
    assert_eq!(original, reparsed);
}

#[test]
fn temperature_scales_passive_value() {
    let net = parse_str(SAMPLE).expect("parse");
    let r = net.elements.iter().find(|e| e.name == "R1").unwrap();
    // TC=0.001 linear coefficient around 27 °C; at 127 °C: 1000 * (1 + 0.001*100).
    assert_eq!(r.value_at_temperature(27.0), Some(1000.0));
    assert!((r.value_at_temperature(127.0).unwrap() - 1100.0).abs() < 1e-9);
}

#[test]
fn expands_subckt_instance() {
    let net = parse_str(SAMPLE).expect("parse");
    let flat = expand_netlist(&net.elements, &net.subckts).expect("expand");
    // One X instance -> two elements (R1, C1) from the `inv` body.
    assert_eq!(flat.len(), net.elements.len() - 1 + 2);
    assert!(flat.iter().any(|e| e.name == "R1_X1"));
    assert!(flat.iter().any(|e| e.name == "C1_X1"));
}

#[test]
fn parse_rejects_passive_without_value() {
    // `R1 in out` parses but fails syntactic validation (no value).
    let bad = "R1 in out\n.end\n";
    let res = SpiceReader::new().parse(bad.as_bytes());
    eprintln!("DBG res = {res:?}");
    assert!(res.is_err());
}

#[cfg(test)]
mod proptest_roundtrip {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn no_panic_on_arbitrary_bytes(
            data in proptest::collection::vec(any::<u8>(), 0..512),
        ) {
            // Parsing arbitrary bytes must never panic; it returns Ok or Err.
            let s = core::str::from_utf8(&data).unwrap_or("\0invalid-utf8");
            let _ = parse_str(s);
        }

        #[test]
        fn roundtrip_is_idempotent(
            r in any::<f64>(),
            c in any::<f64>(),
        ) {
            // A tiny two-element netlist round-trips through serialization.
            let src = format!(
                "* rt\nR1 n1 n2 {r:?}\nC1 n2 0 {c:?}\n.end\n"
            );
            let net = parse_str(&src).expect("parse");
            let bytes = SpiceReader::new().serialize(&net).expect("serialize");
            let reparsed = parse_str(core::str::from_utf8(&bytes).unwrap()).expect("reparse");
            prop_assert_eq!(net, reparsed);
        }
    }
}
