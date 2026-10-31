//! Integration tests for the GDSII reader and writer.

use tpt_io_gds::{decode_real8, encode_real8, parse, write, Element, GdsError, Library, Structure};

fn sample() -> Library {
    Library {
        name: "TOP".into(),
        user_units: 0.001,
        db_units: 1e-9,
        structures: vec![
            Structure {
                name: "LEAF".into(),
                elements: vec![
                    Element::Boundary { layer: 1, datatype: 0, xy: vec![(0, 0), (100, 0), (100, 50), (0, 0)] },
                    Element::Path { layer: 2, datatype: 1, width: 10, xy: vec![(0, 0), (0, 500)] },
                    Element::Text { layer: 3, texttype: 0, at: (5, 5), string: "VDD".into() },
                ],
            },
            Structure {
                name: "TOP".into(),
                elements: vec![Element::SRef { name: "LEAF".into(), at: (1000, -2000) }],
            },
        ],
    }
}

#[test]
fn round_trip_keeps_every_element_kind() {
    let lib = sample();
    let bytes = write(&lib).unwrap();
    let back = parse(&bytes).unwrap();
    assert_eq!(back.name, "TOP");
    assert_eq!(back.structures, lib.structures);
}

#[test]
fn units_survive_within_real8_precision() {
    let bytes = write(&sample()).unwrap();
    let back = parse(&bytes).unwrap();
    assert!((back.user_units - 0.001).abs() < 1e-15);
    assert!((back.db_units - 1e-9).abs() < 1e-20);
}

#[test]
fn real8_known_values_and_round_trips() {
    // 1.0 is 0x41 0x10 0x00 ... in excess-64 base-16 form.
    assert_eq!(encode_real8(1.0), [0x41, 0x10, 0, 0, 0, 0, 0, 0]);
    assert_eq!(decode_real8([0x41, 0x10, 0, 0, 0, 0, 0, 0]), 1.0);
    assert_eq!(decode_real8(encode_real8(0.0)), 0.0);
    for v in [2.5, -7.25, 1e-6, 123456.0] {
        let back = decode_real8(encode_real8(v));
        assert!((back - v).abs() <= v.abs() * 1e-12, "{v} came back as {back}");
    }
}

#[test]
fn truncated_streams_and_missing_endlib_are_errors() {
    let bytes = write(&sample()).unwrap();
    assert!(parse(&bytes[..bytes.len() - 3]).is_err());
    let no_end = &bytes[..bytes.len() - 4];
    assert!(matches!(parse(no_end), Err(GdsError::Invalid(_))));
}

#[test]
fn an_element_outside_a_structure_is_an_error() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x00, 0x04, 0x08, 0x00]); // BOUNDARY with no structure
    bytes.extend_from_slice(&[0x00, 0x04, 0x11, 0x00]); // ENDEL
    bytes.extend_from_slice(&[0x00, 0x04, 0x04, 0x00]); // ENDLIB
    assert!(matches!(parse(&bytes), Err(GdsError::Invalid(_))));
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 137;
    for _ in 0..300 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect();
        let _ = parse(&bytes);
    }
}
