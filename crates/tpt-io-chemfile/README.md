# tpt-io-chemfile

Readers and writers for four common structure file formats used in computational chemistry and
materials science.

| Format | Typical tool | Read | Write |
| --- | --- | --- | --- |
| XYZ | many (molecular viewers, quantum codes) | yes | yes |
| Gaussian cube | Gaussian and visualisation tools | yes | yes |
| POSCAR | VASP | yes | yes |
| LAMMPS data | LAMMPS | yes (`atomic` style) | no |

## Spec access and licence

Each format is documented by its tool's authors (the XYZ convention, Gaussian's cube
documentation, VASP's POSCAR documentation, and LAMMPS's `read_data` documentation). No
documentation text or example file is copied into this crate. Tests use structures written
for them.

## What each reader keeps

- **XYZ**: the atom count, the comment line, and each atom's element and position. Extra
  columns after the coordinates are ignored.
- **Cube**: the two comment lines, origin, the three axes (count and step), atoms (atomic number,
  charge, position in Bohr) and the voxel values in file order. A negative atom count is accepted.
- **POSCAR** (VASP 5 layout): comment, scale, lattice rows, species, counts, coordinate mode
  (`Direct` or `Cartesian`) and positions.
- **LAMMPS data**: title, the `N atoms` header, and the rows of the `Atoms` section.

## What it does not do

- Only the VASP 5 POSCAR layout (with species names) is read.
- The LAMMPS reader decodes the `atomic` style only; other styles are reported as malformed
  rows, not reinterpreted. There is no LAMMPS writer.
- Cube voxel data is read as one flat list; the grid is not reshaped into an array.
