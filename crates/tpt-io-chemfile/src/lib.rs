//! Molecular and crystal structure files: XYZ, Gaussian cube, VASP POSCAR and LAMMPS data.
//!
//! Each format is read by its own function and returns a typed structure:
//!
//! - [`xyz`]: an atom count, a comment line, and one `element x y z` row per atom (read and write).
//! - [`cube`]: a Gaussian cube file with its origin, voxel grid, atoms and volumetric data (read and write).
//! - [`poscar`]: a VASP POSCAR with its lattice, species and positions (read and write).
//! - [`lammps`]: a LAMMPS data file's `Atoms` section (read only).
//!
//! ```
//! let xyz = b"3\nwater\nO 0.0 0.0 0.0\nH 0.0 0.76 0.59\nH 0.0 -0.76 0.59\n";
//! let mol = tpt_io_chemfile::xyz::parse(xyz).unwrap();
//! assert_eq!(mol.atoms[1].element, "H");
//! ```
//!
//! Spec access: these formats are documented by their tool authors (the XYZ convention, Gaussian's
//! cube format, the VASP POSCAR format, and the LAMMPS data file format). No documentation text is
//! copied into this crate.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(any(feature = "alloc", feature = "std"))]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod cube;
pub mod error;
pub mod lammps;
pub mod poscar;
pub mod xyz;

pub use error::ChemError;

/// An atom with an element symbol and a Cartesian position in Ångström.
#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    /// The element symbol, e.g. `O`.
    pub element: alloc::string::String,
    /// Position `[x, y, z]`.
    pub position: [f64; 3],
}
