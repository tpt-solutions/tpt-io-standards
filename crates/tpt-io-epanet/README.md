# tpt-io-epanet

A reader, writer and network checker for EPANET input files (`.inp`), the hydraulic model
format for water distribution networks. The same section layout is used by SWMM, so SWMM
`.inp` files are read as well.

## Spec access and licence

The EPANET 2.2 input format is documented in the EPANET User Manual, which the US EPA
publishes in the public domain. No manual text or example network is copied into this
crate; test networks are written for these tests.

## What it reads

- Every `[SECTION]`, its rows, and `;` comments (trailing and whole-line).
- Comments that appear before the first section are kept.
- Typed views of the network sections:
  - `junctions()`: id, elevation, demand, optional pattern.
  - `reservoirs()`: id, head, optional pattern.
  - `tanks()`: id, elevation, initial, minimum and maximum level, diameter, minimum volume.
  - `pipes()`: id, end nodes, length, diameter, roughness, minor loss, status (default `Open`).
  - `coordinates()`: id and x, y.

Section names are matched case-insensitively. Other sections (`[OPTIONS]`, `[TIMES]`,
`[CURVES]`, SWMM `[CONDUITS]` and so on) are available as rows without typing.

## Validation

`Inp::validate` checks that node ids are unique across junctions, reservoirs and tanks,
that every pipe joins two known nodes, that pipe length and diameter are positive, and that
pipe status is `Open`, `Closed` or `CV`.

## Writing

`write` emits each section with its rows; tokens are joined with tabs and comments are
written after `;`. `parse(write(x))` returns the same rows.

## What it does not do

- It does not run a hydraulic simulation or check patterns and curves against their uses.
- Typed views cover the network sections only; other sections are rows.
- Token spacing inside a row is normalised to single whitespace on read.
