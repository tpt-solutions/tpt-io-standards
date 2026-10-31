//! Integration tests for the EDF and BDF reader and writer.

use tpt_io_edf::{parse, write, Edf, EdfError, Signal};

fn recording() -> Edf {
    Edf {
        bdf: false,
        patient: "X X X X".into(),
        recording: "Startdate 01-JAN-2024 X".into(),
        start_date: "01.01.24".into(),
        start_time: "12.00.00".into(),
        record_duration: 1.0,
        records: 2,
        signals: vec![
            Signal {
                label: "EEG Fpz-Cz".into(),
                transducer: "AgAgCl".into(),
                physical_dim: "uV".into(),
                physical_min: -100.0,
                physical_max: 100.0,
                digital_min: -32768,
                digital_max: 32767,
                prefiltering: "HP:0.1Hz".into(),
                samples_per_record: 2,
            },
            Signal {
                label: "ECG".into(),
                physical_dim: "mV".into(),
                physical_min: -5.0,
                physical_max: 5.0,
                digital_min: -2048,
                digital_max: 2047,
                samples_per_record: 1,
                ..Signal::default()
            },
        ],
        samples: vec![vec![0, 16384, -16384, 32767], vec![100, -100]],
    }
}

#[test]
fn write_then_parse_keeps_header_and_samples() {
    let e = recording();
    let bytes = write(&e).unwrap();
    assert_eq!(bytes.len(), 256 * 3 + 3 * 2 * 2);
    let back = parse(&bytes).unwrap();
    assert!(!back.bdf);
    assert_eq!(back.patient, "X X X X");
    assert_eq!(back.start_date, "01.01.24");
    assert_eq!(back.records, 2);
    assert_eq!(back.signals[0].label, "EEG Fpz-Cz");
    assert_eq!(back.signals[1].physical_dim, "mV");
    assert_eq!(back.samples, e.samples);
}

#[test]
fn physical_values_scale_between_the_limits() {
    let e = recording();
    let p = e.physical(0);
    assert!((p[0] - 0.0).abs() < 0.01);
    assert!((p[3] - 100.0).abs() < 0.01);
    assert!((p[2] + 50.0).abs() < 0.01);
    assert!(e.physical(9).is_empty());
}

#[test]
fn bdf_samples_are_read_as_24_bit_signed() {
    // A one-signal, one-record EDF header with two samples per record, then the same file
    // re-labelled as BDF: identifier byte 0xFF, and the two data samples stored as 24-bit values.
    let e = Edf {
        records: 1,
        record_duration: 1.0,
        signals: vec![Signal {
            label: "Ch1".into(),
            physical_min: -1.0,
            physical_max: 1.0,
            digital_min: -8388608,
            digital_max: 8388607,
            samples_per_record: 2,
            ..Signal::default()
        }],
        samples: vec![vec![0, 0]],
        ..Edf::default()
    };
    let edf = write(&e).unwrap();
    let mut bdf = edf[..256 * 2].to_vec();
    bdf[0] = 0xFF;
    bdf[1..8].copy_from_slice(b"BIOSEMI");
    // -1 as a 24-bit little-endian value, then 0x7FFFFF.
    bdf.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]);
    let back = parse(&bdf).unwrap();
    assert!(back.bdf);
    assert_eq!(back.samples, vec![vec![-1, 8_388_607]]);
    assert!((back.physical(0)[1] - 1.0).abs() < 1e-6);
}

#[test]
fn bdf_writing_is_unsupported_and_malformed_files_are_errors() {
    let mut e = recording();
    e.bdf = true;
    assert!(matches!(write(&e), Err(EdfError::Unsupported(_))));
    assert!(parse(b"short").is_err());
    let mut bytes = write(&recording()).unwrap();
    bytes.truncate(bytes.len() - 4);
    assert!(matches!(parse(&bytes), Err(EdfError::Invalid(_))));
}

#[test]
fn oversized_samples_are_rejected_on_write() {
    let mut e = recording();
    e.samples[0][0] = 70_000;
    assert!(matches!(write(&e), Err(EdfError::Invalid(_))));
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 131;
    for _ in 0..200 {
        let len = (x % 600) as usize;
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
