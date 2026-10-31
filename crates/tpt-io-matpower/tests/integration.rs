//! Integration tests: a hand-written three-bus case, round trips, and a deterministic
//! property test over generated cases.

use tpt_io_matpower::{parse, write, Branch, Bus, Case, Gen, MatpowerError, Value};

const CASE3: &[u8] = b"function mpc = three_bus
% Hand-written three-bus example for the reader tests.
mpc.version = '2';
mpc.baseMVA = 100;

%% bus data
mpc.bus = [
\t1\t3\t0\t0\t0\t0\t1\t1.02\t0\t230\t1\t1.1\t0.9;
\t2\t1\t90\t30\t0\t0\t1\t1\t0\t230\t1\t1.1\t0.9;
\t3\t2\t0\t0\t0\t0\t1\t1\t0\t230\t1\t1.1\t0.9;
];

%% generator data
mpc.gen = [
\t1\t80\t0\t300\t-300\t1.02\t100\t1\t250\t10;
\t3\t50\t0\t300\t-300\t1\t100\t1\t150\t10;
];

%% branch data
mpc.branch = [
\t1\t2\t0.01\t0.085\t0.1\t250\t250\t250\t0\t0\t1\t-360\t360;
\t2\t3\t0.02\t0.12\t0.05\t150\t150\t150\t0\t0\t1\t-360\t360;
\t1\t3\t0.01\t0.1\t0\t120\t120\t120\t0.98\t-2\t0\t-360\t360;
];

mpc.gencost = [
\t2\t1500\t0\t3\t0.11\t5\t150;
\t2\t2000\t0\t3\t0.085\t1.2\t600;
];
mpc.bus_name = {'North'; 'Central'; 'South'};
";

#[test]
fn reads_the_three_bus_case() {
    let case = parse(CASE3).unwrap();
    assert_eq!(case.name.as_deref(), Some("three_bus"));
    assert_eq!(case.version.as_deref(), Some("2"));
    assert_eq!(case.base_mva, 100.0);
    assert_eq!(case.bus.len(), 3);
    assert_eq!(case.bus[0].kind, 3);
    assert_eq!(case.bus[1].pd, 90.0);
    assert_eq!(case.gen.len(), 2);
    assert_eq!(case.gen[1].bus, 3);
    assert_eq!(case.branch.len(), 3);
    assert_eq!(case.branch[2].ratio, 0.98);
    assert_eq!(case.branch[2].status, 0);
    assert_eq!(case.branch[0].angmin, -360.0);
    assert!(case.validate().is_ok());
}

#[test]
fn extra_fields_are_kept_in_order() {
    let case = parse(CASE3).unwrap();
    let names: Vec<&str> = case.extra.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names, ["gencost", "bus_name"]);
    match case.extra_field("gencost") {
        Some(Value::Matrix(rows)) => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].len(), 7);
        }
        other => panic!("unexpected gencost: {other:?}"),
    }
    assert_eq!(
        case.extra_field("bus_name"),
        Some(&Value::Cell(vec!["North".into(), "Central".into(), "South".into()]))
    );
}

#[test]
fn write_then_parse_is_identity() {
    let case = parse(CASE3).unwrap();
    let text = write(&case);
    let again = parse(&text).unwrap();
    assert_eq!(case, again);
}

#[test]
fn validation_catches_bad_references_and_missing_reference_bus() {
    let mut case = parse(CASE3).unwrap();
    case.gen[0].bus = 99;
    assert!(matches!(case.validate(), Err(MatpowerError::Invalid(_))));

    let mut case = parse(CASE3).unwrap();
    case.bus[0].kind = 2;
    assert!(matches!(case.validate(), Err(MatpowerError::Invalid(_))));

    let mut case = parse(CASE3).unwrap();
    case.base_mva = 0.0;
    assert!(case.validate().is_err());
}

#[test]
fn rejects_short_rows_and_bad_numbers() {
    let short = b"mpc.baseMVA = 100;\nmpc.bus = [\n1 3 0 0;\n];\n";
    assert!(matches!(parse(short), Err(MatpowerError::Invalid(_))));
    let bad = b"mpc.baseMVA = 1x;\n";
    assert!(matches!(parse(bad), Err(MatpowerError::Core(_))));
    let open = b"mpc.bus = [\n1 3 0";
    assert!(parse(open).is_err());
}

#[test]
fn unknown_statements_are_skipped() {
    let src = b"mpc.baseMVA = 10;\nmpc.userfcn = struct('x', 1);\nmpc.bus = [];\n";
    let case = parse(src).unwrap();
    assert_eq!(case.base_mva, 10.0);
    assert!(case.extra.is_empty());
}

/// Deterministic generator (xorshift64) so property failures reproduce exactly.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A value on a 1/64 grid, so it is exact in binary and prints shortest-round-trip.
    fn real(&mut self) -> f64 {
        (self.next() % 1_000_000) as f64 / 64.0 - 2000.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn random_case(rng: &mut Rng) -> Case {
    let nbus = 2 + rng.below(6) as u32;
    let bus = (1..=nbus)
        .map(|id| Bus {
            id,
            kind: if id == 1 { 3 } else { 1 + rng.below(2) as u32 },
            pd: rng.real(),
            qd: rng.real(),
            gs: rng.real(),
            bs: rng.real(),
            area: 1,
            vm: rng.real(),
            va: rng.real(),
            base_kv: rng.real(),
            zone: 1,
            vmax: rng.real(),
            vmin: rng.real(),
        })
        .collect();
    let gen = (0..rng.below(4))
        .map(|_| Gen {
            bus: 1 + rng.below(nbus as u64) as u32,
            pg: rng.real(),
            qg: rng.real(),
            qmax: rng.real(),
            qmin: rng.real(),
            vg: rng.real(),
            mbase: rng.real(),
            status: rng.below(2) as u32,
            pmax: rng.real(),
            pmin: rng.real(),
        })
        .collect();
    let branch = (0..rng.below(6))
        .map(|_| Branch {
            from: 1 + rng.below(nbus as u64) as u32,
            to: 1 + rng.below(nbus as u64) as u32,
            r: rng.real(),
            x: rng.real(),
            b: rng.real(),
            rate_a: rng.real(),
            rate_b: rng.real(),
            rate_c: rng.real(),
            ratio: rng.real(),
            angle: rng.real(),
            status: rng.below(2) as u32,
            angmin: rng.real(),
            angmax: rng.real(),
        })
        .collect();
    let mut extra = Vec::new();
    if rng.below(2) == 0 {
        extra.push((
            "gencost".to_string(),
            Value::Matrix(vec![vec![rng.real(), rng.real()], vec![rng.real()]]),
        ));
    }
    Case {
        name: Some("generated".into()),
        version: Some("2".into()),
        base_mva: 100.0,
        bus,
        gen,
        branch,
        extra,
    }
}

#[test]
fn generated_cases_round_trip() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..200 {
        let case = random_case(&mut rng);
        let text = write(&case);
        let back = parse(&text).unwrap_or_else(|e| {
            panic!("re-read failed: {e}\n{}", String::from_utf8_lossy(&text))
        });
        assert_eq!(case, back);
    }
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut rng = Rng(42);
    for _ in 0..500 {
        let len = rng.below(200) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        let _ = parse(&bytes);
    }
}
