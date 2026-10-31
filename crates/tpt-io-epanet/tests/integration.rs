//! Integration tests for the EPANET `.inp` reader, writer and network checks.

use tpt_io_epanet::{parse, write, EpanetError};

const NETWORK: &[u8] = b"[TITLE]
Two-node test network
[JUNCTIONS]
;ID   Elev   Demand  Pattern
J1    100    10      1      ; first junction
J2    95     0
[RESERVOIRS]
R1    120    ; source
[TANKS]
T1    150    5   0   20   12   0
[PIPES]
P1    R1  J1  500  300  100  0  Open
P2    J1  J2  250  200  120  0.5
P3    J2  T1  300  150  110  0  CV
[COORDINATES]
J1    1000   2000
J2    1500   2000.5
R1    0      0
T1    2000   3000
[OPTIONS]
UNITS              GPM
[END]
";

#[test]
fn reads_sections_as_rows_with_comments() {
    let inp = parse(NETWORK).unwrap();
    let names: Vec<&str> = inp.sections.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["TITLE", "JUNCTIONS", "RESERVOIRS", "TANKS", "PIPES", "COORDINATES", "OPTIONS", "END"]);
    // Row 0 is the ";ID …" header comment; row 1 is the first junction.
    let junctions = inp.section("junctions").unwrap();
    assert!(junctions.rows[0].tokens.is_empty());
    assert_eq!(junctions.rows[1].comment.as_deref(), Some("first junction"));
    assert_eq!(junctions.rows[1].tokens, ["J1", "100", "10", "1"]);
}

#[test]
fn typed_views_read_the_network() {
    let inp = parse(NETWORK).unwrap();
    let j = inp.junctions().unwrap();
    assert_eq!(j.len(), 2);
    assert_eq!(j[0].pattern.as_deref(), Some("1"));
    assert_eq!(j[1].pattern, None);
    assert_eq!(inp.reservoirs().unwrap()[0].head, 120.0);
    let t = &inp.tanks().unwrap()[0];
    assert_eq!((t.max_level, t.diameter), (20.0, 12.0));
    let p = inp.pipes().unwrap();
    assert_eq!(p[1].minor_loss, 0.5);
    assert_eq!(p[0].status, "Open");
    assert_eq!(p[2].status, "CV");
    assert_eq!(inp.coordinates().unwrap()[1].y, 2000.5);
}

#[test]
fn the_network_validates() {
    parse(NETWORK).unwrap().validate().unwrap();
}

#[test]
fn validation_finds_unknown_nodes_and_bad_pipes() {
    let mut inp = parse(NETWORK).unwrap();
    inp.sections
        .iter_mut()
        .find(|s| s.name == "PIPES")
        .unwrap()
        .rows[0]
        .tokens[2] = "NOWHERE".into();
    assert!(matches!(inp.validate(), Err(EpanetError::Topology(_))));

    let bad = b"[JUNCTIONS]\nJ1 1\nJ1 2\n";
    assert!(matches!(parse(bad).unwrap().validate(), Err(EpanetError::Topology(_))));

    let zero = b"[JUNCTIONS]\nA 1\nB 1\n[RESERVOIRS]\nR 9\n[PIPES]\nP A B 0 100 100\n";
    assert!(matches!(parse(zero).unwrap().validate(), Err(EpanetError::Topology(_))));
}

#[test]
fn short_rows_in_typed_sections_are_invalid() {
    let inp = parse(b"[JUNCTIONS]\nJ1\n").unwrap();
    assert!(matches!(inp.junctions(), Err(EpanetError::Invalid(_))));
    let inp = parse(b"[JUNCTIONS]\nJ1 abc\n").unwrap();
    assert!(matches!(inp.junctions(), Err(EpanetError::Invalid(_))));
}

#[test]
fn row_outside_a_section_is_an_error_but_comments_are_kept() {
    assert!(parse(b"J1 100\n").is_err());
    let inp = parse(b"; preamble note\n[END]\n").unwrap();
    assert_eq!(inp.preamble[0].comment.as_deref(), Some("preamble note"));
    assert!(parse(b"[OPEN\n").is_err());
}

#[test]
fn write_then_parse_is_identity() {
    let inp = parse(NETWORK).unwrap();
    let text = write(&inp);
    assert_eq!(parse(&text).unwrap(), inp);
}

#[test]
fn swmm_style_sections_round_trip_generically() {
    let swmm = b"[CONDUITS]\nC1 J1 J2 100 0.013 0 0 0 0\n[SUBCATCHMENTS]\nS1 R1 J1 10 50 1000 0.5 0.7\n[END]\n";
    let inp = parse(swmm).unwrap();
    assert_eq!(inp.section("CONDUITS").unwrap().rows[0].tokens[3], "100");
    assert_eq!(parse(&write(&inp)).unwrap(), inp);
}
