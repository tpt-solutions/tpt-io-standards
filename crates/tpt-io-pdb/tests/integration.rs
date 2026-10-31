//! Integration tests for the PDB reader and writer.

use tpt_io_pdb::{parse, write, Atom, Header, Pdb, PdbError};

const SAMPLE: &[u8] = b"HEADER    TRANSFERASE                             01-JAN-24   1ABC
TITLE     TEST PROTEIN FRAGMENT
TITLE    2 SECOND LINE
ATOM      1  N   ALA A   1      11.104   6.134  -6.504  1.00 20.00           N
ATOM      2  CA  ALA A   1      11.639   6.071  -5.147  1.00 17.00           C
HETATM    3 ZN   ZN  A 101       0.000   0.000   0.000  0.50 30.25          ZN
CONECT    2    1    3
END
";

#[test]
fn reads_header_title_atoms_and_conect() {
    let p = parse(SAMPLE).unwrap();
    let h = p.header.as_ref().unwrap();
    assert_eq!(h.classification, "TRANSFERASE");
    assert_eq!(h.deposition_date, "01-JAN-24");
    assert_eq!(h.id_code, "1ABC");
    assert_eq!(p.title, "TEST PROTEIN FRAGMENT SECOND LINE");
    assert_eq!(p.atoms.len(), 3);
    let n = &p.atoms[0];
    assert_eq!((n.serial, n.name.as_str(), n.res_name.as_str()), (1, "N", "ALA"));
    assert_eq!((n.chain, n.res_seq), ('A', 1));
    assert_eq!(n.position, [11.104, 6.134, -6.504]);
    assert_eq!(n.element, "N");
    assert!(p.atoms[2].hetero);
    assert_eq!(p.atoms[2].element, "ZN");
    assert_eq!(p.atoms[2].b_factor, 30.25);
    assert_eq!(p.conect, vec![(2, vec![1, 3])]);
}

#[test]
fn write_then_parse_is_identity() {
    let p = parse(SAMPLE).unwrap();
    let text = write(&p);
    assert_eq!(parse(&text).unwrap(), p);
}

#[test]
fn records_have_fixed_widths_when_written() {
    let p = Pdb {
        header: Some(Header {
            classification: "X".into(),
            deposition_date: "01-JAN-24".into(),
            id_code: "9XYZ".into(),
        }),
        title: "HELLO".into(),
        atoms: vec![Atom {
            hetero: false,
            serial: 7,
            name: "CA".into(),
            alt_loc: ' ',
            res_name: "GLY".into(),
            chain: 'B',
            res_seq: 12,
            icode: ' ',
            position: [1.5, -2.25, 30.125],
            occupancy: 1.0,
            b_factor: 12.5,
            element: "C".into(),
        }],
        conect: vec![],
    };
    let text = String::from_utf8(write(&p)).unwrap();
    for line in text.lines() {
        assert!(line.len() <= 80, "record longer than 80 columns: {line:?}");
    }
    assert_eq!(parse(text.as_bytes()).unwrap(), p);
}

#[test]
fn duplicate_serials_and_bad_numbers_are_errors() {
    let dup = b"ATOM      1  N   ALA A   1      11.104   6.134  -6.504  1.00 20.00           N\n\
ATOM      1  C   ALA A   1      11.104   6.134  -6.504  1.00 20.00           C\n";
    assert!(matches!(parse(dup), Err(PdbError::Invalid(_))));
    let bad = b"ATOM      x  N   ALA A   1      11.104   6.134  -6.504  1.00 20.00           N\n";
    assert!(parse(bad).is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 61;
    for _ in 0..300 {
        let len = (x % 300) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"ATOMHETTRCNECEND 0123456789.-\n";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse(&bytes);
    }
}
