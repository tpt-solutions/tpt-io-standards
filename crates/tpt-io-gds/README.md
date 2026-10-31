# tpt-io-gds

A reader and writer for GDSII stream files (`.gds`, `.gds2`), the binary IC layout format used
in semiconductor design and mask data.

## Spec access and licence

GDSII (the Calma stream format) is publicly documented and widely implemented. No specification
text, mask data or layout is copied into this crate; the test library is built in code.

## What it reads

- Records with their two-byte length, record type and data type, and the library name and units.
- `BGNSTR` and `STRNAME`: structures and their names.
- Elements: `BOUNDARY` (layer, datatype, points), `PATH` (layer, datatype, width, points), `TEXT`
  (layer, texttype, position, string) and `SREF` (structure name, position).
- Real numbers in the base-16 excess-64 format, converted by `decode_real8` and `encode_real8`.
- A library must end with `ENDLIB`; elements must sit inside a structure.

## What it writes

A library with its units, structures and elements, in the same record order. Parsing the output gives
back the same structures and units (to real-number precision). Timestamps are written as zeros.

## What it does not do

- `AREF`, `NODE`, `BOX`, `TEXTNODE`, and the transformation records (`STRANS`, `MAG`, `ANGLE`) are
  skipped when reading and not written. Structures that use them lose those elements.
- Property records are skipped.
- Layer and datatype ranges are not checked against the process.
