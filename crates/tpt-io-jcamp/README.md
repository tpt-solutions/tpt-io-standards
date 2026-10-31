# tpt-io-jcamp

A reader and writer for JCAMP-DX spectroscopy files (infrared, Raman, NMR, mass spectra exported in
the JCAMP-DX format).

## Spec access and licence

JCAMP-DX is published by IUPAC. No specification text, spectrum or database entry is copied into
this crate; test files are written for these tests.

## What it reads

- `##LABEL=value` records, with labels upper-cased and continuation lines joined to the value.
- `##XYPOINTS=(XY..XY)` blocks: explicit `x y` pairs (commas, semicolons or spaces as separators).
- `##XYDATA=(X++(Y..Y))` blocks: each line starts with an X value followed by Y values at
  successive X steps.
- `##XFACTOR` and `##YFACTOR`, applied to every point. Values written with a `D` exponent are read.

## What it writes

The header records, then the points as one `XYPOINTS` block, then `##END=`. Parsing the output gives
back the same spectrum.

## What it does not do

- Compressed encodings (`SQZ`, `DIF`, `DUP`) are reported as `JcampError::Unsupported`; their
  decoding is not guessed.
- Multi-block files (`LINK`, `BLOCKS`) are read as one block of points.
- Peak tables, assignments and other labelled tables are kept as header records only.
