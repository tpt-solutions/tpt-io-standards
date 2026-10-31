//! Integration tests for the molfile and SDF reader and writer.

use tpt_io_chem::{parse_mol, parse_sdf, write_sdf, ChemError, Molecule, MolAtom, Bond};

const SDF: &[u8] = b"ethanol
  test program
comment line
  3  2  0  0  0  0  0  0  0  0999 V2000
    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    1.5000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    2.0000    1.2000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  1  0  0  0  0
  2  3  1  0  0  0  0
M  END
> <CAS>
64-17-5

> <Formula>
C2H6O

$$$$
water
  test

  3  2  0  0  0  0  0  0  0  0999 V2000
    0.0000    0.0000    0.1173 O   0  0  0  0  0  0  0  0  0  0  0  0
    0.0000    0.7572   -0.4692 H   0  0  0  0  0  0  0  0  0  0  0  0
    0.0000   -0.7572   -0.4692 H   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  1  0  0  0  0
  1  3  1  0  0  0  0
M  END
$$$$
";

#[test]
fn reads_sdf_records_atoms_bonds_and_data_items() {
    let records = parse_sdf(SDF).unwrap();
    assert_eq!(records.len(), 2);
    let eth = &records[0];
    assert_eq!(eth.name, "ethanol");
    assert_eq!(eth.program_line, "  test program");
    assert_eq!(eth.atoms.len(), 3);
    assert_eq!(eth.atoms[2].symbol, "O");
    assert_eq!(eth.atoms[2].position, [2.0, 1.2, 0.0]);
    assert_eq!(eth.bonds[1], Bond { first: 2, second: 3, order: 1, stereo: 0 });
    assert_eq!(eth.properties[0], ("CAS".to_string(), "64-17-5".to_string()));
    assert_eq!(eth.properties[1].1, "C2H6O");
    assert_eq!(records[1].atoms[0].symbol, "O");
    assert!(records[1].properties.is_empty());
}

#[test]
fn write_then_parse_is_identity() {
    let records = parse_sdf(SDF).unwrap();
    let bytes = write_sdf(&records);
    assert_eq!(parse_sdf(&bytes).unwrap(), records);
}

#[test]
fn a_molecule_built_in_code_round_trips() {
    let m = Molecule {
        name: "H2".into(),
        program_line: "  x".into(),
        comment: String::new(),
        atoms: vec![
            MolAtom { symbol: "H".into(), position: [0.0, 0.0, 0.0] },
            MolAtom { symbol: "H".into(), position: [0.74, 0.0, 0.0] },
        ],
        bonds: vec![Bond { first: 1, second: 2, order: 1, stereo: 0 }],
        properties: vec![],
    };
    assert_eq!(parse_sdf(&write_sdf(std::slice::from_ref(&m))).unwrap(), vec![m]);
}

#[test]
fn a_bare_molfile_parses_without_properties() {
    let mol_text = b"x\n  y\n\n  1  0  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 N   0  0\nM  END\n";
    let m = parse_mol(mol_text).unwrap();
    assert_eq!(m.atoms[0].symbol, "N");
    assert!(m.properties.is_empty());
}

#[test]
fn malformed_records_are_errors() {
    assert!(matches!(parse_mol(b"a\nb\n"), Err(ChemError::Invalid(_))));
    let bad_bond = b"x\n\n\n  1  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0\n  1  5  1  0\nM  END\n";
    assert!(matches!(parse_mol(bad_bond), Err(ChemError::Invalid(_))));
    let v3000 = b"x\n\n\n  1  0  0  0  0  0  0  0  0  0999 V3000\nM  END\n";
    assert!(parse_mol(v3000).is_err());
    assert!(parse_sdf(b"x\n\n\n  0  0  0  0  0  0  0  0  0  0999 V2000\n").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 67;
    for _ in 0..300 {
        let len = (x % 240) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"V2000M  END$$$$> <>0123456789.- \n";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = parse_sdf(&bytes);
        let _ = parse_mol(&bytes);
    }
}
