//! Integration tests for the LAS reader and writer.

use tpt_io_pointcloud::{parse, write, Cloud, LasError, Point};

fn cloud(format: u8) -> Cloud {
    let mut c = Cloud {
        version_major: 1,
        version_minor: 2,
        point_format: format,
        scale: [0.01, 0.01, 0.001],
        offset: [500000.0, 4000000.0, 100.0],
        points: Vec::new(),
    };
    c.points.push(Point {
        position: [500001.25, 4000002.5, 105.125],
        intensity: 1200,
        return_number: 1,
        number_of_returns: 2,
        classification: 2,
        scan_angle: -15,
        user_data: 7,
        point_source_id: 42,
        gps_time: if format == 1 { Some(1234.5) } else { None },
        rgb: None,
    });
    c.points.push(Point {
        position: [499999.99, 3999999.5, 99.875],
        intensity: 65535,
        return_number: 3,
        number_of_returns: 3,
        classification: 9,
        scan_angle: 0,
        ..Point::default()
    });
    c
}

#[test]
fn format_zero_round_trips_positions_and_fields() {
    let c = cloud(0);
    let back = parse(&write(&c).unwrap()).unwrap();
    assert_eq!(back.point_format, 0);
    assert_eq!(back.points.len(), 2);
    for (a, b) in back.points.iter().zip(&c.points) {
        for axis in 0..3 {
            assert!((a.position[axis] - b.position[axis]).abs() < 1e-9, "axis {axis}");
        }
        assert_eq!(a.intensity, b.intensity);
        assert_eq!(a.return_number, b.return_number);
        assert_eq!(a.number_of_returns, b.number_of_returns);
        assert_eq!(a.classification, b.classification);
        assert_eq!(a.scan_angle, b.scan_angle);
        assert_eq!(a.point_source_id, b.point_source_id);
    }
}

#[test]
fn format_one_carries_gps_time() {
    let c = cloud(1);
    let back = parse(&write(&c).unwrap()).unwrap();
    assert_eq!(back.points[0].gps_time, Some(1234.5));
    assert_eq!(back.points[1].gps_time, Some(0.0));
}

#[test]
fn coordinates_are_scaled_and_offset_on_read() {
    let mut bytes = write(&cloud(0)).unwrap();
    // Set the first point's raw X to 2 with scale 0.01 and offset 500000 → 500000.02.
    let first = 227;
    bytes[first..first + 4].copy_from_slice(&2i32.to_le_bytes());
    let back = parse(&bytes).unwrap();
    assert!((back.points[0].position[0] - 500000.02).abs() < 1e-9);
}

#[test]
fn unsupported_formats_and_bad_headers_are_errors() {
    let mut bytes = write(&cloud(0)).unwrap();
    bytes[104] = 6;
    assert!(matches!(parse(&bytes), Err(LasError::Unsupported(_))));
    assert!(matches!(write(&cloud(2)), Err(LasError::Unsupported(_))));
    assert!(parse(b"NOTLAS").is_err());
    let mut short = write(&cloud(0)).unwrap();
    short.truncate(short.len() - 1);
    assert!(parse(&short).is_err());
}

#[test]
fn values_outside_the_scale_are_rejected_on_write() {
    let mut c = cloud(0);
    c.points[0].position[0] = 500000.0 + 1e12;
    assert!(matches!(write(&c), Err(LasError::Invalid(_))));
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 157;
    for _ in 0..200 {
        let len = (x % 600) as usize;
        let mut bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect();
        if bytes.len() >= 4 {
            bytes[..4].copy_from_slice(b"LASF");
        }
        let _ = parse(&bytes);
    }
}
