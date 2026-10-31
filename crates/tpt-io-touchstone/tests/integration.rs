//! Integration tests for the Touchstone reader and writer.

use tpt_io_touchstone::{parse, write, Complex, Format, Param, TouchstoneError};

fn close(a: Complex, re: f64, im: f64) -> bool {
    (a.re - re).abs() < 1e-9 && (a.im - im).abs() < 1e-9
}

#[test]
fn ri_two_port_keeps_s11_s21_s12_s22_order() {
    let src = b"! comment line\n# GHz S RI R 50\n1.0  0.1 0.0  0.9 0.0  0.8 0.0  0.2 -0.1\n2.0 0.2 0.1 0.8 0.1 0.7 0.1 0.3 0.0\n";
    let f = parse(src, 2).unwrap();
    assert_eq!(f.param, Param::S);
    assert_eq!(f.reference_ohm, 50.0);
    assert_eq!(f.points.len(), 2);
    assert_eq!(f.points[0].freq_hz, 1.0e9);
    let m = &f.points[0].matrix;
    assert!(close(m[0][0], 0.1, 0.0));
    assert!(close(m[1][0], 0.9, 0.0), "S21 must sit at row 1, column 0");
    assert!(close(m[0][1], 0.8, 0.0));
    assert!(close(m[1][1], 0.2, -0.1));
}

#[test]
fn db_and_ma_formats_convert_to_real_and_imaginary() {
    // |S21| = -6.0206 dB → magnitude 0.5, at angle 90 degrees → (0, 0.5).
    let db = b"# MHz S DB R 50\n100 -20 0 -6.020599913279624 90 -20 0 -20 0\n";
    let f = parse(db, 2).unwrap();
    assert_eq!(f.points[0].freq_hz, 1.0e8);
    assert!(close(f.points[0].matrix[0][0], 0.1, 0.0));
    let s21 = f.points[0].matrix[1][0];
    assert!(s21.re.abs() < 1e-9 && (s21.im - 0.5).abs() < 1e-9, "got {s21:?}");

    let ma = b"# Hz S MA R 75\n5 0.5 0 0.5 0 0.5 0 0.5 0\n";
    let f = parse(ma, 2).unwrap();
    assert_eq!(f.reference_ohm, 75.0);
    assert!(close(f.points[0].matrix[0][0], 0.5, 0.0));
}

#[test]
fn one_port_and_wrapped_lines() {
    let one = b"# Hz Z RI R 50\n10 1.0 2.0\n20 3.0 4.0\n";
    let f = parse(one, 1).unwrap();
    assert_eq!(f.param, Param::Z);
    assert_eq!(f.points.len(), 2);
    assert!(close(f.points[1].matrix[0][0], 3.0, 4.0));

    let wrapped = b"# Hz S RI\n1 0 0 1 0\n0 0 1 0\n";
    let f = parse(wrapped, 2).unwrap();
    assert_eq!(f.points.len(), 1);
}

#[test]
fn write_then_parse_is_identity() {
    let src = b"# GHz S RI R 50\n1.5 0.1 0.2 0.3 0.4 0.5 0.6 0.7 0.8\n";
    let f = parse(src, 2).unwrap();
    let text = write(&f);
    assert!(text.starts_with(b"# Hz S RI R 50"));
    assert_eq!(parse(&text, 2).unwrap(), f);
    let c = Complex { re: 1.0, im: -1.0 };
    assert_eq!(c.re, 1.0);
    assert_eq!(Format::Ri, Format::Ri);
}

#[test]
fn malformed_files_are_errors() {
    assert!(parse(b"# GHz S RI\n1 0 0\n", 2).is_err());
    assert!(matches!(parse(b"# GHz Q RI\n", 1), Err(TouchstoneError::Invalid(_))));
    assert!(parse(b"1 0 0\n", 0).is_err());
    assert!(parse(b"# GHz S RI\n1 0 zero\n", 1).is_err());
    assert!(parse(b"# GHz S RI\n# GHz S RI\n", 1).is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 43;
    for _ in 0..300 {
        let len = (x % 160) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"0123456789.-+e #!GHzSRIMADB\n ";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes, 2);
    }
}
