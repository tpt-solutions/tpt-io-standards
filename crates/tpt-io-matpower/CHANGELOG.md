# Changelog

## Unreleased

- Initial reader for MATPOWER case scripts: `bus`, `gen` and `branch` matrices,
  scalar, string, matrix and cell-array fields kept in file order.
- Writer that re-reads to an equal `Case`.
- `Case::validate` for bus numbering, reference bus, status flags and references.
