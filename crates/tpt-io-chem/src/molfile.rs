//! MOL V2000 and SDF records: reading and writing.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use tpt_io_core::ParseError;

use crate::error::ChemError;

/// An atom from the atom block.
#[derive(Debug, Clone, PartialEq)]
pub struct MolAtom {
    /// Element symbol, e.g. `C` or `Cl`.
    pub symbol: String,
    /// Position `[x, y, z]` in Ångström.
    pub position: [f64; 3],
}

/// A bond from the bond block (atom indices are 1-based, as in the file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bond {
    /// First atom, 1-based.
    pub first: usize,
    /// Second atom, 1-based.
    pub second: usize,
    /// Bond order: 1 single, 2 double, 3 triple, 4 aromatic, and so on.
    pub order: u8,
    /// Bond stereo code (0 when not stereogenic).
    pub stereo: u8,
}

/// One molecule record: its header lines, atoms, bonds and SDF data items.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Molecule {
    /// Header line 1: the molecule name.
    pub name: String,
    /// Header line 2: program and timestamp line.
    pub program_line: String,
    /// Header line 3: comment.
    pub comment: String,
    /// Atoms in file order.
    pub atoms: Vec<MolAtom>,
    /// Bonds in file order.
    pub bonds: Vec<Bond>,
    /// SDF data items (`> <NAME>` and its value), in file order. Empty for a bare molfile.
    pub properties: Vec<(String, String)>,
}

fn field(line: &str, from: usize, to: usize) -> &str {
    line.get(from - 1..to.min(line.len())).unwrap_or("").trim()
}

fn int(s: &str, at: usize) -> Result<usize, ChemError> {
    s.parse::<usize>().map_err(|_| ParseError::custom(at, "expected an integer").into())
}

fn num(s: &str, at: usize) -> Result<f64, ChemError> {
    s.parse::<f64>().map_err(|_| ParseError::custom(at, "expected a number").into())
}

/// Parse one molfile record, starting at byte offset `base` of the source, from its lines.
fn record(lines: &[&str], base: usize) -> Result<Molecule, ChemError> {
    if lines.len() < 4 {
        return Err(ChemError::Invalid("a molfile needs a header and a counts line".to_string()));
    }
    let counts = lines[3];
    let natoms = int(field(counts, 1, 3), base)?;
    let nbonds = int(field(counts, 4, 6), base)?;
    if !counts.contains("V2000") {
        return Err(ChemError::Invalid("only the V2000 layout is supported".to_string()));
    }
    let mut mol = Molecule {
        name: lines[0].to_string(),
        program_line: lines[1].to_string(),
        comment: lines[2].to_string(),
        ..Molecule::default()
    };
    let atom_start = 4;
    let bond_start = atom_start + natoms;
    for i in 0..natoms {
        let line = lines.get(atom_start + i).ok_or_else(|| {
            ChemError::Invalid("fewer atom lines than the counts line".to_string())
        })?;
        mol.atoms.push(MolAtom {
            position: [
                num(field(line, 1, 10), base)?,
                num(field(line, 11, 20), base)?,
                num(field(line, 21, 30), base)?,
            ],
            symbol: field(line, 32, 34).to_string(),
        });
    }
    for i in 0..nbonds {
        let line = lines.get(bond_start + i).ok_or_else(|| {
            ChemError::Invalid("fewer bond lines than the counts line".to_string())
        })?;
        let first = int(field(line, 1, 3), base)?;
        let second = int(field(line, 4, 6), base)?;
        if first == 0 || second == 0 || first > natoms || second > natoms {
            return Err(ChemError::Invalid(
                "bond refers to an atom outside the atom block".to_string(),
            ));
        }
        mol.bonds.push(Bond {
            first,
            second,
            order: int(field(line, 7, 9), base)? as u8,
            stereo: int(field(line, 10, 12), base).unwrap_or(0) as u8,
        });
    }
    Ok(mol)
}

/// Parse a single molfile (the record ends at `M  END` or the end of input).
pub fn parse_mol(input: &[u8]) -> Result<Molecule, ChemError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8"))?;
    let lines: Vec<&str> = text.lines().take_while(|l| !l.starts_with("M  END")).collect();
    record(&lines, 0)
}

/// Parse an SDF file: molfile records separated by `$$$$`, each with its data items.
pub fn parse_sdf(input: &[u8]) -> Result<Vec<Molecule>, ChemError> {
    let text = core::str::from_utf8(input)
        .map_err(|e| ParseError::custom(e.valid_up_to(), "file is not valid UTF-8"))?;
    let mut out = Vec::new();
    let mut block: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.starts_with("$$$$") {
            if !block.is_empty() {
                out.push(entry(&block)?);
                block.clear();
            }
        } else {
            block.push(line);
        }
    }
    if block.iter().any(|l| !l.trim().is_empty()) {
        out.push(entry(&block)?);
    }
    Ok(out)
}

/// One SDF entry: the molfile up to `M  END`, then data items.
fn entry(block: &[&str]) -> Result<Molecule, ChemError> {
    let end = block
        .iter()
        .position(|l| l.starts_with("M  END"))
        .ok_or_else(|| ChemError::Invalid("record has no M  END line".to_string()))?;
    let mut mol = record(&block[..end], 0)?;
    let mut i = end + 1;
    while i < block.len() {
        let line = block[i];
        if let Some(rest) = line.strip_prefix('>') {
            let name = rest
                .split_once('<')
                .and_then(|(_, n)| n.split_once('>').map(|(n, _)| n.to_string()))
                .unwrap_or_default();
            let mut value = String::new();
            i += 1;
            while i < block.len() && !block[i].trim().is_empty() {
                if !value.is_empty() {
                    value.push('\n');
                }
                value.push_str(block[i]);
                i += 1;
            }
            mol.properties.push((name, value));
        }
        i += 1;
    }
    Ok(mol)
}

fn write_one(out: &mut String, m: &Molecule) {
    let _ = writeln!(out, "{}", m.name);
    let _ = writeln!(out, "{}", m.program_line);
    let _ = writeln!(out, "{}", m.comment);
    let _ =
        writeln!(out, "{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000", m.atoms.len(), m.bonds.len());
    for a in &m.atoms {
        let _ = writeln!(
            out,
            "{:>10.4}{:>10.4}{:>10.4} {:<3} 0  0  0  0  0  0  0  0  0  0  0  0",
            a.position[0], a.position[1], a.position[2], a.symbol
        );
    }
    for b in &m.bonds {
        let _ = writeln!(out, "{:>3}{:>3}{:>3}{:>3}", b.first, b.second, b.order, b.stereo);
    }
    out.push_str("M  END\n");
}

/// Serialise one molecule as a V2000 molfile.
pub fn write_mol(m: &Molecule) -> Vec<u8> {
    let mut out = String::new();
    write_one(&mut out, m);
    out.into_bytes()
}

/// Serialise molecules as an SDF file, with their data items after each record.
pub fn write_sdf(molecules: &[Molecule]) -> Vec<u8> {
    let mut out = String::new();
    for m in molecules {
        write_one(&mut out, m);
        for (name, value) in &m.properties {
            let _ = writeln!(out, "> <{name}>");
            let _ = writeln!(out, "{value}");
            out.push('\n');
        }
        out.push_str("$$$$\n");
    }
    out.into_bytes()
}
