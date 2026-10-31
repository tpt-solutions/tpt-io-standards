//! LAMMPS data files (read only): the header counts and the `Atoms` section.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::{lines, number, ChemError};

/// One row of the `Atoms` section (atom style `atomic`: `id type x y z`).
#[derive(Debug, Clone, PartialEq)]
pub struct LammpsAtom {
    /// Atom id.
    pub id: u64,
    /// Atom type (1-based).
    pub kind: u32,
    /// Position `[x, y, z]`.
    pub position: [f64; 3],
}

/// A LAMMPS data file: the title line, the declared atom count, and the atoms.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LammpsData {
    /// The first line, which LAMMPS treats as a comment.
    pub title: String,
    /// The `N atoms` header value.
    pub atom_count: usize,
    /// The atoms in the `Atoms` section.
    pub atoms: Vec<LammpsAtom>,
}

/// Read a LAMMPS data file. Only the `atomic` atom style is decoded; the header keyword
/// `atoms` gives the count, and `Atoms` (with an optional `# style` comment) gives the rows.
pub fn parse(input: &[u8]) -> Result<LammpsData, ChemError> {
    let ls = lines(input)?;
    let title = ls.first().map(|s| s.to_string()).unwrap_or_default();
    let mut atom_count = None;
    let mut i = 1;
    while i < ls.len() {
        let t = ls[i].trim();
        if t.ends_with("atoms") && !t.starts_with('#') {
            atom_count = t.split_whitespace().next().and_then(|n| n.parse::<usize>().ok());
        }
        if t == "Atoms" || t.starts_with("Atoms #") {
            break;
        }
        i += 1;
    }
    let count = atom_count.ok_or_else(|| ChemError::Invalid("missing 'N atoms' header".into()))?;
    // Skip the blank line after the heading.
    i += 1;
    while i < ls.len() && ls[i].trim().is_empty() {
        i += 1;
    }
    let mut atoms = Vec::with_capacity(count);
    for line in ls.iter().skip(i).take(count) {
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.len() < 5 {
            return Err(ChemError::Invalid("Atoms row needs id, type, x, y, z".into()));
        }
        atoms.push(LammpsAtom {
            id: c[0].parse().map_err(|_| ChemError::Invalid("bad atom id".into()))?,
            kind: c[1].parse().map_err(|_| ChemError::Invalid("bad atom type".into()))?,
            position: [number(c[2], 0)?, number(c[3], 0)?, number(c[4], 0)?],
        });
    }
    if atoms.len() != count {
        return Err(ChemError::Invalid("fewer Atoms rows than the header count".into()));
    }
    Ok(LammpsData { title, atom_count: count, atoms })
}
