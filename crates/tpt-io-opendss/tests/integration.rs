//! Integration tests for the OpenDSS reader and writer.

use tpt_io_opendss::{parse, write};

const FEEDER: &[u8] = b"! Small feeder for the reader tests
Clear
New Circuit.feeder basekv=12.47 bus1=sourcebus
~ pu=1.0 mvasc3=200
New Line.L1 bus1=sourcebus bus2=b2 length=0.5 units=km  ! main trunk
New Line.L2 bus1=b2 bus2=b3 length=0.2 // trailing note style
New Load.LD1 bus1=b3 kw=350 kvar=120 model=1
New XYcurve.c1 npts=3 xarray=[0 0.5 1.0] yarray=(1 0.9 0.8)
Set voltagebases=[12.47 0.48]
Solve
Redirect other.dss
";

#[test]
fn splits_commands_and_properties() {
    let s = parse(FEEDER).unwrap();
    let verbs: Vec<&str> = s.commands.iter().map(|c| c.verb.as_str()).collect();
    assert_eq!(verbs, ["Clear", "New", "New", "New", "New", "New", "Set", "Solve", "Redirect"]);

    let circuit = &s.commands[1];
    assert_eq!(circuit.object.as_deref(), Some("Circuit"));
    assert_eq!(circuit.name.as_deref(), Some("feeder"));
    assert_eq!(circuit.prop("basekv"), Some("12.47"));
    assert_eq!(circuit.prop("MVASC3"), Some("200"));
    assert_eq!(circuit.line, 3);
}

#[test]
fn element_queries_and_comments() {
    let s = parse(FEEDER).unwrap();
    let lines: Vec<&str> = s.elements("line").filter_map(|c| c.name.as_deref()).collect();
    assert_eq!(lines, ["L1", "L2"]);
    let l1 = s.elements("Line").next().unwrap();
    assert_eq!(l1.prop("units"), Some("km"));
}

#[test]
fn bracketed_values_stay_whole() {
    let s = parse(FEEDER).unwrap();
    let xy = s.elements("XYcurve").next().unwrap();
    assert_eq!(xy.prop("xarray"), Some("[0 0.5 1.0]"));
    assert_eq!(xy.prop("yarray"), Some("(1 0.9 0.8)"));
    let set = &s.commands[6];
    assert_eq!(set.prop("voltagebases"), Some("[12.47 0.48]"));
}

#[test]
fn positional_arguments_are_kept() {
    let s = parse(FEEDER).unwrap();
    assert_eq!(s.commands.last().unwrap().positional, ["other.dss"]);
}

#[test]
fn write_then_parse_is_identity() {
    let s = parse(FEEDER).unwrap();
    let text = write(&s);
    let again = parse(&text).unwrap();
    // Line numbers describe the source layout, so compare the command contents only.
    let contents = |s: &tpt_io_opendss::Script| -> Vec<_> {
        s.commands
            .iter()
            .map(|c| (c.verb.clone(), c.object.clone(), c.name.clone(), c.positional.clone(), c.props.clone()))
            .collect()
    };
    assert_eq!(contents(&again), contents(&s));
}

#[test]
fn unterminated_bracket_is_an_error() {
    assert!(parse(b"New Line.L1 xarray=[1 2\n").is_err());
    assert!(parse(b"New Line.L1 name=\"oops\n").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 7;
    for _ in 0..500 {
        let len = (x % 120) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                b" \n!~[](\"=abcNewLine.".get((x % 20) as usize).copied().unwrap_or(b'x')
            })
            .collect();
        let _ = parse(&bytes);
    }
}
