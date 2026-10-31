//! Integration tests for the NRRD reader and writer.

use tpt_io_nrrd::{parse, write, NrrdError};

const RAW16: &[u8] = b"NRRD0004\n# a comment line\ntype: short\ndimension: 2\nsizes: 2 2\nendian: big\nencoding: raw\n\n\x00\x01\xff\xfe\x00\x10\x00\x20";

#[test]
fn reads_header_fields_and_sizes() {
    let n = parse(RAW16).unwrap();
    assert_eq!(n.magic, "NRRD0004");
    assert_eq!(n.field("type"), Some("short"));
    assert_eq!(n.field("dimension"), Some("2"));
    assert_eq!(n.sizes(), Some(vec![2, 2]));
    assert_eq!(n.sample_count(), Some(4));
}

#[test]
fn decodes_big_endian_signed_samples() {
    let n = parse(RAW16).unwrap();
    assert_eq!(n.values().unwrap(), vec![1.0, -2.0, 16.0, 32.0]);
}

#[test]
fn ascii_data_is_parsed_as_numbers() {
    let n =
        parse(b"NRRD0004\ntype: float\ndimension: 1\nsizes: 3\nencoding: ascii\n\n1.5 2.5\n-3\n")
            .unwrap();
    assert_eq!(n.values().unwrap(), vec![1.5, 2.5, -3.0]);
}

#[test]
fn write_then_parse_is_identity() {
    let n = parse(RAW16).unwrap();
    let bytes = write(&n);
    assert_eq!(parse(&bytes).unwrap(), n);
}

#[test]
fn key_value_pairs_and_external_data_are_handled() {
    let n = parse(b"NRRD0004\nsome:=value\ndimension: 1\nsizes: 1\n\nX").unwrap();
    assert_eq!(n.field("some:="), Some("value"));
    assert!(matches!(parse(b"NRRD0004\ndata file: x.raw\n\n"), Err(NrrdError::Unsupported(_))));
    assert!(matches!(parse(b"NRRD0004\nencoding: gzip\n\nxx"), Err(NrrdError::Unsupported(_))));
}

#[test]
fn malformed_files_are_errors() {
    assert!(parse(b"PNG\n\n").is_err());
    assert!(parse(b"NRRD0004\ntype uchar\n\n").is_err());
    assert!(parse(b"NRRD0004\ntype: uchar\n").is_err());
    let short =
        parse(b"NRRD0004\ntype: uchar\ndimension: 1\nsizes: 4\nencoding: raw\n\nab").unwrap();
    assert!(matches!(short.values(), Err(NrrdError::Invalid(_))));
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 113;
    for _ in 0..300 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"NRRD0004\ntype: uchar sizes dimension encoding: \n\x00\xff";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        if let Ok(n) = parse(&bytes) {
            let _ = n.values();
        }
    }
}
