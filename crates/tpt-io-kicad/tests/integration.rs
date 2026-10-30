//! Integration tests on the hand-written KiCad 6 fixtures in `tests/samples/`.

use tpt_io_kicad::{parse, DocKind, KicadError};

const PCB: &[u8] = include_bytes!("samples/divider.kicad_pcb");
const SCH: &[u8] = include_bytes!("samples/divider.kicad_sch");

#[test]
fn pcb_header_nets_and_footprints() {
    let doc = parse(PCB).unwrap();
    assert_eq!(doc.kind, DocKind::Pcb);
    assert_eq!(doc.version, Some(20211014));
    assert_eq!(doc.generator.as_deref(), Some("pcbnew"));

    let names: Vec<&str> = doc.nets.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["", "GND", "VOUT"]);
    assert_eq!(doc.nets[2].id, 2);

    assert_eq!(doc.footprints.len(), 2);
    let r1 = &doc.footprints[0];
    assert_eq!(r1.lib_id, "Resistor_SMD:R_0603_1608Metric");
    assert_eq!(r1.layer.as_deref(), Some("F.Cu"));
    assert_eq!((r1.x, r1.y, r1.rotation), (10.16, 20.32, 90.0));
    assert_eq!(r1.reference.as_deref(), Some("R1"));
    assert_eq!(r1.value.as_deref(), Some("10k"));

    let c1 = &doc.footprints[1];
    assert_eq!(c1.layer.as_deref(), Some("B.Cu"));
    assert_eq!(c1.rotation, 0.0, "missing rotation defaults to 0");
    assert!(doc.symbols.is_empty());
}

#[test]
fn schematic_symbols() {
    let doc = parse(SCH).unwrap();
    assert_eq!(doc.kind, DocKind::Schematic);
    assert_eq!(doc.version, Some(20211123));
    assert!(doc.nets.is_empty() && doc.footprints.is_empty());

    assert_eq!(doc.symbols.len(), 2);
    let r1 = &doc.symbols[0];
    assert_eq!(r1.lib_id, "Device:R");
    assert_eq!((r1.x, r1.y), (100.33, 50.8));
    assert_eq!(r1.reference.as_deref(), Some("R1"));
    assert_eq!(r1.value.as_deref(), Some("10k"));
    assert_eq!(doc.symbols[1].value.as_deref(), Some("100n"));
}

#[test]
fn other_kicad_files_are_unsupported_not_malformed() {
    let project = b"(kicad_pro (version 1))";
    assert!(matches!(parse(project), Err(KicadError::Unsupported(_))));
}

#[test]
fn truncated_file_reports_an_error_without_panicking() {
    for cut in [1usize, 40, PCB.len() / 2, PCB.len() - 2] {
        assert!(parse(&PCB[..cut]).is_err(), "cut at {cut} should fail");
    }
}
