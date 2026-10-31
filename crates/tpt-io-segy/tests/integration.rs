//! Integration tests for the SEG-Y reader and writer.

use tpt_io_segy::{parse, write, SegY, SegyError, Trace};

fn file_with(traces: Vec<Vec<f32>>) -> SegY {
    let mut f = SegY { text: b"C01 synthetic survey".to_vec(), ..SegY::default() };
    f.binary.sample_interval_us = 2000;
    f.binary.samples_per_trace = traces.first().map(|t| t.len()).unwrap_or(0) as u16;
    f.binary.raw = vec![0; 400];
    f.traces = traces
        .into_iter()
        .enumerate()
        .map(|(i, samples)| {
            let mut header = [0u8; 240];
            header[..4].copy_from_slice(&(i as u32 + 1).to_be_bytes());
            Trace { header, samples }
        })
        .collect();
    f
}

#[test]
fn write_then_parse_keeps_text_binary_and_traces() {
    let f = file_with(vec![vec![0.5, -1.25, 3.0], vec![10.0, 0.0, -10.0]]);
    let bytes = write(&f);
    assert_eq!(bytes.len(), 3600 + 2 * (240 + 3 * 4));
    let back = parse(&bytes).unwrap();
    assert_eq!(back.binary.sample_interval_us, 2000);
    assert_eq!(back.binary.samples_per_trace, 3);
    assert_eq!(back.binary.format, 5);
    assert_eq!(back.traces.len(), 2);
    assert_eq!(back.traces[1].samples, vec![10.0, 0.0, -10.0]);
    assert_eq!(&back.traces[0].header[..4], &1u32.to_be_bytes());
    assert_eq!(&back.text[..b"C01 synthetic survey".len()], b"C01 synthetic survey");
    assert_eq!(parse(&write(&back)).unwrap(), back);
}

#[test]
fn integer_formats_are_decoded() {
    let mut f = file_with(vec![vec![0.0, 0.0]]);
    f.binary.format = 3;
    let mut bytes = write(&f);
    // Rewrite the format field and the sample data as big-endian int16 values 7 and -3.
    bytes[3200 + 24..3200 + 26].copy_from_slice(&3u16.to_be_bytes());
    let data_at = 3600 + 240;
    bytes.truncate(data_at + 4);
    bytes[data_at..data_at + 2].copy_from_slice(&7i16.to_be_bytes());
    bytes[data_at + 2..data_at + 4].copy_from_slice(&(-3i16).to_be_bytes());
    let back = parse(&bytes).unwrap();
    assert_eq!(back.traces[0].samples, vec![7.0, -3.0]);
}

#[test]
fn unknown_format_codes_are_reported() {
    let mut bytes = write(&file_with(vec![vec![1.0]]));
    bytes[3200 + 24..3200 + 26].copy_from_slice(&7u16.to_be_bytes());
    assert!(matches!(parse(&bytes), Err(SegyError::Unsupported(_))));
}

#[test]
fn short_files_are_errors() {
    assert!(parse(&[0u8; 100]).is_err());
    let mut bytes = write(&file_with(vec![vec![1.0, 2.0]]));
    bytes.truncate(bytes.len() - 2);
    assert!(parse(&bytes).is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 149;
    for _ in 0..200 {
        let len = (x % 4000) as usize;
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
