//! Integration tests for the G-code reader, writer and motion summary.

use tpt_io_gcode::{parse, write, Comment, Dialect, GcodeError};

const SQUARE: &[u8] = b"%
(LinuxCNC style program)
O100 (subprogram header)
N10 G21 G90
N20 G0 X0 Y0 Z5
N30 G1 Z-1 F200
N40 G1 X10
N50 G1 Y10
N60 G1 X0
N70 G2 X0 Y0 I-5 J-5
N80 G91
N90 G1 X5
N100 G20 G90 G1 X0.5 Y0.5
%
";

const REPRAP: &[u8] = b"; printed part
G21 ; millimetres
G90
G1 X10 Y5 F600
G1 X10 Y15
M104 S210
";

#[test]
fn reads_line_numbers_words_and_comments() {
    let p = parse(SQUARE).unwrap();
    // The `%` delimiters are dropped; the comment-only and O-word lines are kept.
    assert_eq!(p.blocks.len(), 12);
    let first = &p.blocks[0];
    assert_eq!(first.line_number, None);
    assert_eq!(first.comment, Some(Comment::Paren("LinuxCNC style program".into())));
    assert_eq!(p.blocks[1].word('O').unwrap().value, 100.0);
    let n10 = &p.blocks[2];
    assert_eq!(n10.line_number, Some(10));
    assert_eq!(n10.word('G').unwrap().value, 21.0);
    assert_eq!(n10.word('G').unwrap().text, "G21");
}

#[test]
fn dialect_detection() {
    assert_eq!(parse(SQUARE).unwrap().dialect(), Dialect::LinuxCnc);
    assert_eq!(parse(REPRAP).unwrap().dialect(), Dialect::Reprap);
    assert_eq!(parse(b"G0 X1\n").unwrap().dialect(), Dialect::Generic);
}

#[test]
fn motion_summary_tracks_modal_state() {
    let p = parse(SQUARE).unwrap();
    let s = p.motion_summary();
    assert_eq!(s.rapid_moves, 1);
    assert_eq!(s.arc_moves, 1);
    assert_eq!(s.linear_moves, 6);
    // Path, starting at the origin:
    //   G0 Z5 -> (0,0,5): 5
    //   G1 Z-1 -> (0,0,-1): 6
    //   G1 X10, Y10, X0 -> (0,10,-1): 10 + 10 + 10
    //   G2 arc to (0,0,-1): not measured, position updates
    //   G91 G1 X5 -> (5,0,-1): 5
    //   G20 G90 G1 X0.5 Y0.5 -> inches, so (12.7,12.7,-1): sqrt(7.7^2 + 12.7^2)
    let inch = 25.4;
    let len =
        5.0 + 6.0 + 30.0 + 5.0 + ((0.5 * inch - 5.0_f64).powi(2) + (0.5 * inch).powi(2)).sqrt();
    assert!((s.path_length_mm - len).abs() < 1e-6, "got {} expected {}", s.path_length_mm, len);
    let b = s.xy_bounds.unwrap();
    assert_eq!(b[0], 0.0);
    assert!((b[1] - 0.5 * inch).abs() < 1e-9);
    assert_eq!(b[2], 0.0);
    assert!((b[3] - 0.5 * inch).abs() < 1e-9);
}

#[test]
fn reprap_program_summary_in_millimetres() {
    let s = parse(REPRAP).unwrap().motion_summary();
    assert_eq!(s.linear_moves, 2);
    // (0,0) -> (10,5): sqrt(125); then (10,5) -> (10,15): 10.
    assert!((s.path_length_mm - (125f64.sqrt() + 10.0)).abs() < 1e-9);
}

#[test]
fn write_then_parse_is_identity() {
    for src in [SQUARE, REPRAP] {
        let p = parse(src).unwrap();
        let text = write(&p);
        assert_eq!(parse(&text).unwrap(), p);
    }
}

#[test]
fn malformed_words_and_comments_are_errors() {
    assert!(matches!(parse(b"G1 X1.2.3\n"), Err(GcodeError::BadNumber { .. })));
    assert!(parse(b"G1 X1 (unterminated\n").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 29;
    for _ in 0..400 {
        let len = (x % 160) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"GXYZNO0123456789.-(); \n%";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        if let Ok(p) = parse(&bytes) {
            let _ = p.motion_summary();
            let _ = write(&p);
        }
    }
}
