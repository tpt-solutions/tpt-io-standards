# tpt-io-pdb

A reader and writer for the Protein Data Bank's fixed-column format (`.pdb`).

## Spec access and licence

The PDB format is documented by the wwPDB. No documentation text, entry, or structure file is
copied into this crate; the tests use short records written for them.

## What it reads

- `HEADER`: classification, deposition date and PDB id.
- `TITLE`, with continuation lines joined by a space.
- `ATOM` and `HETATM`: serial, name, alternate location, residue name, chain, residue number,
  insertion code, coordinates, occupancy, B-factor and element.
- `CONECT`: an atom and the serials it is bonded to.

Columns are read by their positions in the standard, so records padded with spaces, or with
the element in the right-hand columns, are read correctly.

## Validation

Atom serial numbers must be unique.

## What it writes

The same records in their columns, with coordinates and factors to three and two decimals, as
the format specifies. Parsing the output gives back the same file for values written at that
precision.

## What it does not do

- `SEQRES`, `REMARK`, `HELIX`, `SHEET`, `SITE`, and the other annotation records are skipped.
- Anisotropic temperature factors (`ANISOU`) are skipped.
- mmCIF is not read here; use `tpt-io-cif`.
