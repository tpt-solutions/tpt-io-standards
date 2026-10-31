//! Integration tests for the DAF and SPK reader, against a small kernel built in code.

use tpt_io_naif::spk::{self, State};
use tpt_io_naif::{Daf, Endian, NaifError};

fn put_f64(buf: &mut [u8], at: usize, v: f64) {
    buf[at..at + 8].copy_from_slice(&v.to_le_bytes());
}

fn put_i32(buf: &mut [u8], at: usize, v: i32) {
    buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// A little-endian SPK kernel with one segment: target 399, type 2, two records of three Chebyshev coefficients per
/// component. Record 0 covers [0, 100) with mid 50 and radius 50; record 1 covers [100, 200) with mid 150.
/// The x component is the constant 5, the y component is `s` (so its velocity is 1/50 km/s), and z is zero.
fn kernel() -> Vec<u8> {
    let mut f = vec![0u8; 4 * 1024];
    f[0..8].copy_from_slice(b"DAF/SPK ");
    put_i32(&mut f, 8, 2); // ND
    put_i32(&mut f, 12, 6); // NI
    put_i32(&mut f, 76, 2); // forward pointer to the first summary record
    put_i32(&mut f, 80, 2); // backward pointer
    f[88..96].copy_from_slice(b"LTL-IEEE");

    // Summary record (record 2): next, previous and count, then one summary of 2 doubles and 6 integers.
    put_f64(&mut f, 1024, 0.0);
    put_f64(&mut f, 1024 + 8, 0.0);
    put_f64(&mut f, 1024 + 16, 1.0);
    put_f64(&mut f, 1048, 0.0);
    put_f64(&mut f, 1056, 200.0);
    put_i32(&mut f, 1064, 399); // target
    put_i32(&mut f, 1068, 0); // centre
    put_i32(&mut f, 1072, 1); // frame
    put_i32(&mut f, 1076, 2); // data type
    put_i32(&mut f, 1080, 385); // first word
    let words = 22 + 4;
    put_i32(&mut f, 1084, 385 + words - 1); // last word

    // Name record (record 3).
    f[2048..2053].copy_from_slice(b"EARTH");

    // Segment data (record 4 onward, word 385). Each record: mid, radius, then x, y and z coefficients.
    let mut data: Vec<f64> = Vec::new();
    for mid in [50.0, 150.0] {
        data.extend_from_slice(&[mid, 50.0]);
        data.extend_from_slice(&[5.0, 0.0, 0.0]); // x
        data.extend_from_slice(&[0.0, 1.0, 0.0]); // y
        data.extend_from_slice(&[0.0, 0.0, 0.0]); // z
    }
    // Trailer: INIT, INTLEN, RSIZE and N.
    data.extend_from_slice(&[0.0, 100.0, 11.0, 2.0]);
    for (i, v) in data.iter().enumerate() {
        put_f64(&mut f, 3072 + 8 * i, *v);
    }
    f
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

#[test]
fn reads_the_file_record_and_summary() {
    let k = kernel();
    let daf = Daf::parse(&k).unwrap();
    assert_eq!(daf.id, "DAF/SPK");
    assert_eq!(daf.endian, Endian::Little);
    assert_eq!((daf.nd, daf.ni), (2, 6));
    assert_eq!(daf.summaries.len(), 1);
    let s = &daf.summaries[0];
    assert_eq!((s.target, s.center, s.frame, s.data_type), (399, 0, 1, 2));
    assert_eq!((s.begin, s.end), (385, 410));
    assert_eq!((s.start_et, s.end_et), (0.0, 200.0));
    assert_eq!(s.name, "EARTH");
    assert_eq!(daf.segment(s).unwrap().len(), 26);
}

#[test]
fn position_and_velocity_come_from_the_covering_record() {
    let k = kernel();
    let daf = Daf::parse(&k).unwrap();

    let at_mid = spk::state(&daf, 399, 50.0).unwrap();
    assert!(close(at_mid.position[0], 5.0));
    assert!(close(at_mid.position[1], 0.0));
    assert!(close(at_mid.velocity[1], 1.0 / 50.0));
    assert!(close(at_mid.velocity[0], 0.0));

    // Halfway between the midpoint and the start of record 0: s = -0.5, so y = -0.5.
    let before = spk::state(&daf, 399, 25.0).unwrap();
    assert!(close(before.position[1], -0.5));

    // Record 1 is selected for epochs from 100 on.
    let later = spk::state(&daf, 399, 125.0).unwrap();
    assert!(close(later.position[0], 5.0));
    assert!(close(later.position[1], -0.5));
}

#[test]
fn missing_bodies_and_epochs_have_no_segment() {
    let k = kernel();
    let daf = Daf::parse(&k).unwrap();
    assert_eq!(spk::state(&daf, 301, 50.0), Err(NaifError::NoSegment));
    assert_eq!(spk::state(&daf, 399, 500.0), Err(NaifError::NoSegment));
}

#[test]
fn unsupported_segment_type_is_reported() {
    let mut k = kernel();
    put_i32(&mut k, 1076, 1);
    let daf = Daf::parse(&k).unwrap();
    assert!(matches!(spk::state(&daf, 399, 50.0), Err(NaifError::Unsupported(_))));
}

#[test]
fn unknown_binary_formats_are_unsupported() {
    let mut k = kernel();
    k[88..96].copy_from_slice(b"VAX-GFLT");
    assert!(matches!(Daf::parse(&k), Err(NaifError::Unsupported(_))));
}

#[test]
fn truncated_data_is_an_error_when_read() {
    let k = kernel();
    let daf = Daf::parse(&k[..2000]).unwrap();
    assert!(matches!(spk::state(&daf, 399, 50.0), Err(NaifError::Core(_))));
}

#[test]
fn non_daf_input_is_rejected() {
    assert!(matches!(Daf::parse(&[0u8; 2048]), Err(NaifError::Invalid(_))));
    assert!(matches!(Daf::parse(b"short"), Err(NaifError::Core(_))));
}

#[test]
fn state_values_are_plain_data() {
    let s = State { position: [1.0, 2.0, 3.0], velocity: [0.0; 3] };
    assert_eq!(s, s.clone());
}
