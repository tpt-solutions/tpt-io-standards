//! XYZ molecule files: an atom count, a comment line, and one `element x y z` line per atom.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::error::{lines, number, ChemError};
use crate::Atom;

/// A molecule read from an XYZ file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Molecule {
    /// The comment line (often the energy or the name).
    pub comment: String,
    /// The atoms in file order.
    pub atoms: Vec<Atom>,
}

/// Parse an XYZ file. Extra columns after the four required ones are ignored.
pub fn parse(input: &[u8]) -> Result<Molecule, ChemError> {
    let ls = lines(input)?;
    let count: usize = ls
        .first()
        .and_then(|l| l.trim().parse().ok())
        .ok_or_else(|| ChemError::Invalid("first line must be the atom count".to_string()))?;
    let comment = ls.get(1).map(|c| c.to_string()).unwrap_or_default();
    let mut atoms = Vec::with_capacity(count);
    for i in 0..count {
        let line = ls
            .get(i + 2)
            .ok_or_else(|| ChemError::Invalid("fewer atom lines than the count".to_string()))?;
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            return Err(ChemError::Invalid(
                "atom line needs an element and three coordinates".to_string(),
            ));
        }
        atoms.push(Atom {
            element: cols[0].to_string(),
            position: [number(cols[1], 0)?, number(cols[2], 0)?, number(cols[3], 0)?],
        });
    }
    Ok(Molecule { comment, atoms })
}

/// Serialise a molecule as XYZ.
pub fn write(m: &Molecule) -> Vec<u8> {
    let mut out = String::new();
    let _ = writeln!(out, "{}", m.atoms.len());
    let _ = writeln!(out, "{}", m.comment);
    for a in &m.atoms {
        let p = a.position;
        let _ = writeln!(out, "{} {} {} {}", a.element, p[0], p[1], p[2]);
    }
    out.into_bytes()
}
