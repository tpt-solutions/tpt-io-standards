//! Integration tests for the GRIB2 reader and writer, against ecCodes-written samples (see `gen_grib.py`).

use tpt_io_grib::{parse, write, GribError, LatLonGrid, Message, ReferenceTime};

fn sample(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/samples/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// The values ecCodes decoded, with `nan` for missing points.
fn expected(name: &str) -> Vec<f64> {
    let text =
        std::fs::read_to_string(format!("{}/tests/samples/{name}", env!("CARGO_MANIFEST_DIR")))
            .unwrap();
    text.lines()
        .map(|l| if l.trim() == "nan" { f64::NAN } else { l.trim().parse().unwrap() })
        .collect()
}

fn message(values: Vec<f64>) -> Message {
    Message {
        discipline: 0,
        centre: 98,
        reference: ReferenceTime { year: 2026, month: 1, day: 2, hour: 12, minute: 0, second: 0 },
        category: 0,
        number: 0,
        forecast_time: 6,
        forecast_unit: 1,
        grid: LatLonGrid {
            ni: 4,
            nj: 3,
            lat_first: 60.0,
            lon_first: 0.0,
            lat_last: 50.0,
            lon_last: 6.0,
            di: 2.0,
            dj: 5.0,
            scanning_mode: 0,
        },
        values,
    }
}

#[test]
fn reads_an_ecodes_simple_packed_grid() {
    let messages = parse(&sample("simple_ll.grib2")).unwrap();
    assert_eq!(messages.len(), 1);
    let m = &messages[0];
    assert_eq!((m.grid.ni, m.grid.nj), (4, 3));
    assert_eq!((m.grid.lat_first, m.grid.lon_first), (60.0, 0.0));
    assert_eq!((m.grid.lat_last, m.grid.lon_last), (50.0, 6.0));
    assert_eq!((m.grid.di, m.grid.dj), (2.0, 5.0));
    assert_eq!(m.grid.scanning_mode, 0);
    let want = expected("simple_ll.expected.txt");
    assert_eq!(m.values.len(), want.len());
    for (got, want) in m.values.iter().zip(want) {
        assert!((got - want).abs() < 1e-9, "{got} != {want}");
    }
}

#[test]
fn bitmap_marks_missing_points_as_nan() {
    let m = &parse(&sample("bitmap_ll.grib2")).unwrap()[0];
    let want = expected("bitmap_ll.expected.txt");
    for (i, (got, want)) in m.values.iter().zip(want).enumerate() {
        if want.is_nan() {
            assert!(got.is_nan(), "point {i} should be missing");
        } else {
            assert!((got - want).abs() < 1e-9, "point {i}: {got} != {want}");
        }
    }
    assert!(m.values[5].is_nan() && m.values[9].is_nan());
}

#[test]
fn writer_output_reads_back_within_packing_resolution() {
    let values: Vec<f64> = (0..12).map(|k| 250.0 + 0.37 * k as f64).collect();
    let bytes = write(&[message(values.clone())]);
    let back = &parse(&bytes).unwrap()[0];
    assert_eq!(back.centre, 98);
    assert_eq!(
        back.reference,
        ReferenceTime { year: 2026, month: 1, day: 2, hour: 12, minute: 0, second: 0 }
    );
    assert_eq!((back.forecast_time, back.forecast_unit), (6, 1));
    assert_eq!(back.grid, message(values.clone()).grid);
    for (got, want) in back.values.iter().zip(&values) {
        assert!((got - want).abs() < 1e-3, "{got} != {want}");
    }
}

#[test]
fn writer_marks_nan_as_missing_through_the_bitmap() {
    let mut values: Vec<f64> = (0..12).map(|k| k as f64).collect();
    values[3] = f64::NAN;
    let back = &parse(&write(&[message(values)])).unwrap()[0];
    assert!(back.values[3].is_nan());
    assert!((back.values[4] - 4.0).abs() < 1e-3);
}

#[test]
fn several_messages_read_in_order() {
    let mut bytes = write(&[message(vec![1.0; 12])]);
    bytes.extend_from_slice(&write(&[message(vec![2.0; 12])]));
    let all = parse(&bytes).unwrap();
    assert_eq!(all.len(), 2);
    assert!((all[0].values[0] - 1.0).abs() < 1e-6);
    assert!((all[1].values[0] - 2.0).abs() < 1e-6);
}

#[test]
fn malformed_input_is_rejected_not_panicked() {
    assert!(matches!(parse(b"not grib at all"), Err(GribError::Invalid(_))));
    let mut edition1 = sample("simple_ll.grib2");
    edition1[7] = 1;
    assert!(matches!(parse(&edition1), Err(GribError::Unsupported(_))));
    let full = sample("simple_ll.grib2");
    assert!(parse(&full[..full.len() - 10]).is_err());
    let mut no_end = sample("simple_ll.grib2");
    let n = no_end.len();
    no_end[n - 4..].copy_from_slice(b"0000");
    assert!(matches!(parse(&no_end), Err(GribError::Invalid(_))));
}
