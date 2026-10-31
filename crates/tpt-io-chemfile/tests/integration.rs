//! Integration tests for the XYZ, cube, POSCAR and LAMMPS readers.

use tpt_io_chemfile::{cube, lammps, poscar, xyz, ChemError};

#[test]
fn xyz_round_trip_and_extra_columns() {
    let src = b"3\nwater energy=-76.4\nO 0.0 0.0 0.1173\nH 0.0 0.7572 -0.4692 0.5\nH 0.0 -0.7572 -0.4692\n";
    let m = xyz::parse(src).unwrap();
    assert_eq!(m.comment, "water energy=-76.4");
    assert_eq!(m.atoms.len(), 3);
    assert_eq!(m.atoms[1].position, [0.0, 0.7572, -0.4692]);
    assert_eq!(xyz::parse(&xyz::write(&m)).unwrap(), m);
}

#[test]
fn xyz_count_mismatch_is_an_error() {
    assert!(matches!(xyz::parse(b"2\nc\nO 0 0 0\n"), Err(ChemError::Invalid(_))));
    assert!(matches!(xyz::parse(b"x\nc\n"), Err(ChemError::Invalid(_))));
    assert!(xyz::parse(b"1\nc\nO 0 0\n").is_err());
}

fn small_cube() -> cube::Cube {
    cube::Cube {
        comment: "density\nsynthetic".into(),
        origin: [0.0, 0.0, 0.0],
        axes: [
            cube::Axis { count: 2, step: [0.5, 0.0, 0.0] },
            cube::Axis { count: 2, step: [0.0, 0.5, 0.0] },
            cube::Axis { count: 2, step: [0.0, 0.0, 0.5] },
        ],
        atoms: vec![cube::CubeAtom { z: 1, charge: 1.0, position: [0.0, 0.0, 0.0] }],
        data: (0..8).map(|i| i as f64 * 0.25).collect(),
    }
}

#[test]
fn cube_round_trip() {
    let c = small_cube();
    let bytes = cube::write(&c);
    assert_eq!(cube::parse(&bytes).unwrap(), c);
}

#[test]
fn cube_value_count_is_checked() {
    let mut c = small_cube();
    c.data.pop();
    let bytes = cube::write(&c);
    assert!(matches!(cube::parse(&bytes), Err(ChemError::Invalid(_))));
}

const POSCAR: &[u8] = b"Si diamond
5.43
0.0 0.5 0.5
0.5 0.0 0.5
0.5 0.5 0.0
Si O
2 1
Direct
0.00 0.00 0.00
0.25 0.25 0.25
0.75 0.75 0.75
";

#[test]
fn poscar_reads_lattice_species_and_positions() {
    let p = poscar::parse(POSCAR).unwrap();
    assert_eq!(p.species, ["Si", "O"]);
    assert_eq!(p.counts, [2, 1]);
    assert!(p.direct);
    assert_eq!(p.positions.len(), 3);
    assert_eq!(p.lattice[1], [0.5, 0.0, 0.5]);
    assert_eq!(poscar::parse(&poscar::write(&p)).unwrap(), p);
}

#[test]
fn poscar_needs_a_coordinate_mode_and_enough_positions() {
    let bad_mode = b"t\n1\n1 0 0\n0 1 0\n0 0 1\nSi\n1\nRelative\n0 0 0\n";
    assert!(poscar::parse(bad_mode).is_err());
    let short = b"t\n1\n1 0 0\n0 1 0\n0 0 1\nSi\n2\nCartesian\n0 0 0\n";
    assert!(matches!(poscar::parse(short), Err(ChemError::Invalid(_))));
}

const LAMMPS: &[u8] = b"LAMMPS data file
2 atoms
1 atom types

0 10 xlo xhi
0 10 ylo yhi
0 10 zlo zhi

Atoms # atomic

1 1 0.0 0.0 0.0
2 1 1.5 0.0 0.0
";

#[test]
fn lammps_atoms_section_is_read() {
    let d = lammps::parse(LAMMPS).unwrap();
    assert_eq!(d.title, "LAMMPS data file");
    assert_eq!(d.atom_count, 2);
    assert_eq!(d.atoms[1].position, [1.5, 0.0, 0.0]);
    assert_eq!(d.atoms[1].kind, 1);
}

#[test]
fn lammps_missing_section_is_an_error() {
    assert!(lammps::parse(b"title\n2 atoms\n").is_err());
}

#[test]
fn arbitrary_bytes_never_panic() {
    let mut x: u64 = 41;
    for _ in 0..300 {
        let len = (x % 200) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let alphabet = b"0123456789 .-\nOHSi Direct";
                alphabet[(x % alphabet.len() as u64) as usize]
            })
            .collect();
        let _ = xyz::parse(&bytes);
        let _ = cube::parse(&bytes);
        let _ = poscar::parse(&bytes);
        let _ = lammps::parse(&bytes);
    }
}
