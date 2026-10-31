# tpt-io-cif

A reader and writer for CIF 1.1 crystallographic files and for mmCIF, the
macromolecular dialect that uses the same STAR syntax.

## Spec access and licence

CIF 1.1 is published by the International Union of Crystallography; mmCIF dictionaries
are published by the wwPDB. No specification text, dictionary or sample structure is copied
into this crate; the tests use a small file written for them.

## What it reads

- `data_` blocks, each with its `_tag value` items and `loop_` tables.
- Values as tokens; quoted strings keep their quotes; text fields (`;` … `;`) are stored
  without the delimiters.
- Case-insensitive lookup by tag (`DataBlock::item`, `DataBlock::loop_with`).
- `#` comments.

## What it writes

Each block, its items (text fields with delimiters), and its loops. Parsing the output
gives back the same file.

## Validation

Loops must have values that fill whole rows, and every tag must have a value. Content before
the first `data_` block is rejected.

## What it does not do

- Save frames, global blocks and the CIF 2.0 extensions are not read.
- Values are not interpreted as numbers or checked against a dictionary.
- Multi-line values are read but their internal layout is not reformatted on write.
