//! Integration tests for the GLM reader and writer.

use tpt_io_gridlabd::{parse, write, GlmError, Item};

const FEEDER: &[u8] = b"// Two-house feeder for the reader tests
clock {
    timezone EST+5EDT;
    starttime '2001-01-01 00:00:00 EST';
}
#set verbose=1
module powerflow {
    solver_method NR;
}
object node {
    name feeder_head;
    phases ABCN;
    nominal_voltage 2400;
}
object transformer {
    name xfmr1;
    from feeder_head;
    to tn1;
    configuration xfmr_config;
}
object node {
    name tn1;
    parent feeder_head;
    phases AS;
    object triplex_meter {
        name tm1;
        parent tn1;
        object house {
            name h1;
            heating_setpoint 68;
        }
    }
}
";

#[test]
fn reads_the_model_structure() {
    let glm = parse(FEEDER).unwrap();
    let kinds: Vec<&str> = glm
        .items
        .iter()
        .map(|i| match i {
            Item::Directive(_) => "directive",
            Item::Module { .. } => "module",
            Item::Clock { .. } => "clock",
            Item::Class { .. } => "class",
            Item::Object(_) => "object",
        })
        .collect();
    assert_eq!(kinds, ["clock", "directive", "module", "object", "object", "object"]);
    assert_eq!(glm.objects_of_class("node").count(), 2);
}

#[test]
fn nested_children_are_reachable() {
    let glm = parse(FEEDER).unwrap();
    let node = glm.objects_of_class("node").nth(1).unwrap();
    let meter = &node.children[0];
    assert_eq!(meter.class, "triplex_meter");
    assert_eq!(meter.children[0].prop("heating_setpoint"), Some("68"));
}

#[test]
fn validation_accepts_the_feeder_and_rejects_bad_names() {
    let glm = parse(FEEDER).unwrap();
    assert!(glm.validate().is_ok());

    let dup = parse(b"object node { name a; }\nobject node { name a; }\n").unwrap();
    assert!(matches!(dup.validate(), Err(GlmError::Invalid(_))));

    let orphan = parse(b"object load { name l; parent nowhere; }\n").unwrap();
    assert!(matches!(orphan.validate(), Err(GlmError::Invalid(_))));
}

#[test]
fn write_then_parse_is_identity() {
    let glm = parse(FEEDER).unwrap();
    let text = write(&glm);
    assert_eq!(parse(&text).unwrap(), glm);
}

#[test]
fn malformed_input_is_an_error_not_a_panic() {
    assert!(parse(b"object node { name n1; ").is_err());
    assert!(parse(b"/* unterminated").is_err());
    let mut x: u64 = 9;
    for _ in 0..500 {
        let len = (x % 90) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                b"object {};/ #name\n*a".get((x % 20) as usize).copied().unwrap_or(b' ')
            })
            .collect();
        let _ = parse(&bytes);
    }
}
