//! Integration tests for the HL7 v2 reader and writer.

use tpt_io_hl7v2::{parse, write, Hl7Error};

const ADT: &[u8] = b"MSH|^~\\&|LAB|HOSP|EMR|HOSP|202401011200||ADT^A01|MSG001|P|2.5\r\
PID|1||12345^^^HOSP^MR||Doe^John^A||19800101|M\r\
PV1|1|I|WARD^101^A\r\
OBX|1|NM|GLU^Glucose||5.4|mmol/L\r\
OBX|2|NM|NA^Sodium||140|mmol/L\r";

#[test]
fn reads_separators_segments_and_fields() {
    let m = parse(ADT).unwrap();
    assert_eq!(m.separators.field, '|');
    assert_eq!(m.separators.component, '^');
    assert_eq!(m.separators.repetition, '~');
    assert_eq!(m.separators.escape, '\\');
    assert_eq!(m.segments.len(), 5);
    assert_eq!(m.message_type().as_deref(), Some("ADT^A01"));
    assert_eq!(m.control_id().as_deref(), Some("MSG001"));
    assert_eq!(m.segment("MSH").unwrap().field(1), Some("|"));
    assert_eq!(m.segment("MSH").unwrap().field(2), Some("^~\\&"));
    let pid = m.segment("PID").unwrap();
    assert_eq!(pid.field(3), Some("12345^^^HOSP^MR"));
    assert_eq!(pid.field(5), Some("Doe^John^A"));
    assert_eq!(pid.field(8), Some("M"));
    assert_eq!(pid.field(0), None);
    assert_eq!(pid.field(99), None);
}

#[test]
fn repeated_segments_are_all_returned() {
    let m = parse(ADT).unwrap();
    let values: Vec<&str> = m.segments_named("OBX").map(|s| s.field(5).unwrap()).collect();
    assert_eq!(values, ["5.4", "140"]);
}

#[test]
fn write_then_parse_is_identity() {
    let m = parse(ADT).unwrap();
    let text = write(&m);
    assert_eq!(parse(&text).unwrap(), m);
}

#[test]
fn custom_separators_are_honoured() {
    let m = parse(b"MSH#*\\~&#APP#FAC\rPID#1##X*Y\r").unwrap();
    assert_eq!(m.separators.field, '#');
    assert_eq!(m.separators.component, '*');
    assert_eq!(m.segment("PID").unwrap().field(3), Some("X*Y"));
}

#[test]
fn malformed_messages_are_errors() {
    assert!(matches!(parse(b"PID|1|2"), Err(Hl7Error::Invalid(_))));
    assert!(matches!(parse(b"MSH|^~"), Err(Hl7Error::Invalid(_))));
    assert!(matches!(parse(b"MSH|^~\\&|x\rpid|1"), Err(Hl7Error::Invalid(_))));
    assert!(parse(b"MSH|^~\\&|x\r\xff").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 53;
    for _ in 0..300 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"MSHPID|^~\\&\r\n1a ";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
