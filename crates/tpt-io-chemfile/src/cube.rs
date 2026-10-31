//! Gaussian cube files: a header, the atoms, and a volumetric grid of values.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::error::{lines, number, ChemError};

/// One axis of the voxel grid: point count and the step vector (Bohr).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Axis {
    /// Number of voxels along this axis.
    pub count: usize,
    /// The voxel step vector.
    pub step: [f64; 3],
}

/// A cube atom: its atomic number, charge, and position (Bohr).
#[derive(Debug, Clone, PartialEq)]
pub struct CubeAtom {
    /// Atomic number.
    pub z: u32,
    /// Charge (often the atomic number again).
    pub charge: f64,
    /// Position in Bohr.
    pub position: [f64; 3],
}

/// A Gaussian cube file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cube {
    /// The two comment lines, joined with a newline.
    pub comment: String,
    /// Grid origin (Bohr).
    pub origin: [f64; 3],
    /// The three grid axes, X then Y then Z.
    pub axes: [Axis; 3],
    /// The atoms.
    pub atoms: Vec<CubeAtom>,
    /// Voxel values in file order (X varies slowest).
    pub data: Vec<f64>,
}

/// Parse a Gaussian cube file.
pub fn parse(input: &[u8]) -> Result<Cube, ChemError> {
    let ls = lines(input)?;
    if ls.len() < 6 {
        return Err(ChemError::Invalid("cube header needs six lines".into()));
    }
    let comment = alloc::format!("{}\n{}", ls[0], ls[1]);
    let head: Vec<&str> = ls[2].split_whitespace().collect();
    if head.len() < 4 {
        return Err(ChemError::Invalid("cube origin line needs an atom count and origin".into()));
    }
    let natoms = head[0].parse::<i64>().map_err(|_| ChemError::Invalid("bad atom count".into()))?;
    // A negative count marks a file that carries orbital numbers; the atoms are still listed.
    let natoms_abs = natoms.unsigned_abs() as usize;
    let origin = [number(head[1], 0)?, number(head[2], 0)?, number(head[3], 0)?];
    let mut axes = Vec::new();
    for line in &ls[3..6] {
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.len() < 4 {
            return Err(ChemError::Invalid("cube axis line needs a count and a step".into()));
        }
        let count =
            c[0].parse::<usize>().map_err(|_| ChemError::Invalid("bad voxel count".into()))?;
        axes.push(Axis { count, step: [number(c[1], 0)?, number(c[2], 0)?, number(c[3], 0)?] });
    }
    let mut atoms = Vec::with_capacity(natoms_abs);
    for line in ls.iter().skip(6).take(natoms_abs) {
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.len() < 5 {
            return Err(ChemError::Invalid("cube atom line needs Z, charge and position".into()));
        }
        atoms.push(CubeAtom {
            z: c[0].parse().map_err(|_| ChemError::Invalid("bad atomic number".into()))?,
            charge: number(c[1], 0)?,
            position: [number(c[2], 0)?, number(c[3], 0)?, number(c[4], 0)?],
        });
    }
    let total: usize = axes.iter().map(|a| a.count).product();
    let values: Vec<f64> = ls
        .iter()
        .skip(6 + natoms_abs)
        .flat_map(|l| l.split_whitespace())
        .map(|t| number(t, 0))
        .collect::<Result<_, _>>()?;
    if values.len() != total {
        return Err(ChemError::Invalid(alloc::format!(
            "expected {total} voxel values, found {}",
            values.len()
        )));
    }
    let [a0, a1, a2]: [Axis; 3] =
        axes.try_into().map_err(|_| ChemError::Invalid("cube needs three axes".into()))?;
    Ok(Cube { comment, origin, axes: [a0, a1, a2], atoms, data: values })
}

/// Serialise a cube file. Values are written six per line, as Gaussian does.
pub fn write(c: &Cube) -> Vec<u8> {
    let mut out = String::new();
    let mut comments = c.comment.splitn(2, '\n');
    let _ = writeln!(out, "{}", comments.next().unwrap_or(""));
    let _ = writeln!(out, "{}", comments.next().unwrap_or(""));
    let _ = writeln!(out, "{} {} {} {}", c.atoms.len(), c.origin[0], c.origin[1], c.origin[2]);
    for a in &c.axes {
        let _ = writeln!(out, "{} {} {} {}", a.count, a.step[0], a.step[1], a.step[2]);
    }
    for a in &c.atoms {
        let p = a.position;
        let _ = writeln!(out, "{} {} {} {} {}", a.z, a.charge, p[0], p[1], p[2]);
    }
    for (i, v) in c.data.iter().enumerate() {
        let _ = write!(out, "{v}");
        out.push(if (i + 1) % 6 == 0 { '\n' } else { ' ' });
    }
    if c.data.len() % 6 != 0 {
        out.push('\n');
    }
    out.into_bytes()
}
