//! Integration tests for the JCAMP-DX reader and writer.

use tpt_io_jcamp::{parse, write, JcampError, Spectrum};

const IR: &[u8] = b"##TITLE=Water vapour
##JCAMP-DX=5.00
##DATA TYPE=INFRARED ABSORBANCE
##XUNITS=1/CM
##YUNITS=ABSORBANCE
##FIRSTX=4000
##LASTX=4000
##XFACTOR=1
##YFACTOR=0.5
##NPOINTS=3
##XYPOINTS=(XY..XY)
4000, 2
3999,4
3998 , 6
##END=
";

#[test]
fn reads_labels_and_applies_factors() {
    let s = parse(IR).unwrap();
    assert_eq!(s.title(), Some("Water vapour"));
    assert_eq!(s.label("DATA TYPE"), Some("INFRARED ABSORBANCE"));
    assert_eq!(s.label("XUNITS"), Some("1/CM"));
    assert_eq!(s.points, vec![(4000.0, 1.0), (3999.0, 2.0), (3998.0, 3.0)]);
}

#[test]
fn xydata_x_plus_plus_lines_are_expanded() {
    let src = b"##XYDATA=(X++(Y..Y))\n100 1 2 3\n103 4 5\n##END=\n";
    let s = parse(src).unwrap();
    assert_eq!(s.points.len(), 5);
    assert_eq!(s.points[0], (100.0, 1.0));
    assert_eq!(s.points[3], (103.0, 4.0));
}

#[test]
fn write_then_parse_is_identity() {
    let s = parse(IR).unwrap();
    let text = write(&s);
    assert_eq!(parse(&text).unwrap(), s);
}

#[test]
fn compressed_encodings_are_reported_not_guessed() {
    let src = b"##XYDATA=(X++(Y..Y))\n100 1@2@3\n##END=\n";
    assert!(matches!(parse(src), Err(JcampError::Unsupported(_))));
}

#[test]
fn files_without_data_or_with_text_before_labels_are_errors() {
    assert!(matches!(parse(b"##TITLE=x\n##END=\n"), Err(JcampError::Invalid(_))));
    assert!(parse(b"stray text\n##TITLE=x\n").is_err());
    assert!(parse(b"##XYPOINTS=(XY..XY)\n1 2 3\n").is_err());
}

#[test]
fn spectrum_can_be_built_and_round_trips() {
    let s = Spectrum {
        header: vec![("TITLE".into(), "built".into())],
        points: vec![(1.5, -0.25), (2.0, 0.0)],
    };
    assert_eq!(parse(&write(&s)).unwrap(), s);
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 71;
    for _ in 0..300 {
        let len = (x % 220) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"##XYPOINTSDATA=(),. 0123456789-\nEND";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
