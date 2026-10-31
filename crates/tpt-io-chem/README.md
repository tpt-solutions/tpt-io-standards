# tpt-io-chem

A reader and writer for MDL molfiles (V2000) and SDF files, the common formats for small-molecule
structures and their data.

## Spec access and licence

The molfile and SDF formats are published by Dassault Systèmes BIOVIA. No specification text,
database record or structure is copied into this crate; the test records are written for these
tests.

## What it reads

- The three header lines: name, program line, comment.
- The counts line (V2000 only).
- Atoms: symbol and position in Ångström.
- Bonds: the two 1-based atom indices, order and stereo (bonds that refer to an atom outside the
  atom block are rejected).
- SDF: records separated by `$$$$`, each with its data items (`> <NAME>` and a value running to
  the next blank line).

## What it writes

Molfiles (V2000 header, atoms, bonds, `M  END`) and SDF (each record followed by its data items
and `$$$$`). Parsing the output gives back the same records.

## What it does not do

- V3000 is rejected.
- Atom charge, mass difference, atom-to-atom mapping and the `M` property lines are not decoded.
- Data item values are kept as text, with no type conversion.
