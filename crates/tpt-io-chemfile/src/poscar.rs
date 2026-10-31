//! VASP POSCAR files: a comment, a scale, the lattice vectors, species, and atom positions.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::error::{lines, number, ChemError};

/// A POSCAR structure.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Poscar {
    /// The comment line.
    pub comment: String,
    /// The global scale factor.
    pub scale: f64,
    /// Lattice vectors as rows (scaled by `scale`).
    pub lattice: [[f64; 3]; 3],
    /// Species symbols, e.g. `["Si", "O"]`. Empty for the pre-VASP-5 layout without names.
    pub species: Vec<String>,
    /// Atom count per species.
    pub counts: Vec<usize>,
    /// `true` for `Direct` (fractional) coordinates, `false` for Cartesian.
    pub direct: bool,
    /// Positions in file order.
    pub positions: Vec<[f64; 3]>,
}

/// Parse a POSCAR file (VASP 5 layout with species names).
pub fn parse(input: &[u8]) -> Result<Poscar, ChemError> {
    let ls = lines(input)?;
    if ls.len() < 8 {
        return Err(ChemError::Invalid("POSCAR needs at least eight lines".into()));
    }
    let comment = ls[0].to_string();
    let scale = number(ls[1].trim(), 0)?;
    let mut lattice = [[0.0; 3]; 3];
    for (i, row) in lattice.iter_mut().enumerate() {
        let c: Vec<&str> = ls[2 + i].split_whitespace().collect();
        if c.len() < 3 {
            return Err(ChemError::Invalid("lattice vector needs three numbers".into()));
        }
        *row = [number(c[0], 0)?, number(c[1], 0)?, number(c[2], 0)?];
    }
    let species: Vec<String> = ls[5].split_whitespace().map(|s| s.to_string()).collect();
    let counts: Vec<usize> = ls[6]
        .split_whitespace()
        .map(|t| t.parse::<usize>().map_err(|_| ChemError::Invalid("bad species count".into())))
        .collect::<Result<_, _>>()?;
    if species.len() != counts.len() {
        return Err(ChemError::Invalid("species and counts differ in length".into()));
    }
    let total: usize = counts.iter().sum();
    let mut next = 7;
    let mode = ls[next].trim();
    let direct = match mode.chars().next() {
        Some('D' | 'd') => true,
        Some('C' | 'c' | 'K' | 'k') => false,
        _ => return Err(ChemError::Invalid("expected Direct or Cartesian".into())),
    };
    next += 1;
    let mut positions = Vec::with_capacity(total);
    for i in 0..total {
        let c: Vec<&str> = ls
            .get(next + i)
            .ok_or_else(|| ChemError::Invalid("fewer positions than the species counts".into()))?
            .split_whitespace()
            .collect();
        if c.len() < 3 {
            return Err(ChemError::Invalid("position needs three coordinates".into()));
        }
        positions.push([number(c[0], 0)?, number(c[1], 0)?, number(c[2], 0)?]);
    }
    Ok(Poscar { comment, scale, lattice, species, counts, direct, positions })
}

/// Serialise a POSCAR file in the VASP 5 layout.
pub fn write(p: &Poscar) -> Vec<u8> {
    let mut out = String::new();
    let _ = writeln!(out, "{}", p.comment);
    let _ = writeln!(out, "{}", p.scale);
    for row in &p.lattice {
        let _ = writeln!(out, "{} {} {}", row[0], row[1], row[2]);
    }
    let _ = writeln!(out, "{}", p.species.join(" "));
    let counts: Vec<String> = p.counts.iter().map(|c| c.to_string()).collect();
    let _ = writeln!(out, "{}", counts.join(" "));
    out.push_str(if p.direct { "Direct\n" } else { "Cartesian\n" });
    for q in &p.positions {
        let _ = writeln!(out, "{} {} {}", q[0], q[1], q[2]);
    }
    out.into_bytes()
}
