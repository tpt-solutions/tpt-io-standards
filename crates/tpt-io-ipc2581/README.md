# tpt-io-ipc2581

A read-only reader for IPC-2581 PCB design data, the single-file XML format for exchanging a
printed-circuit design between CAD tools and manufacturers.

## Spec access and licence

IPC-2581 is published by IPC under its licence terms. No standard text, sample design or schema is
copied into this crate; the test document is written for these tests.

## What it reads

- The root `revision` (for example `B` or `C`).
- The `units` of the `CadHeader`.
- Every `Layer` with its name, `layerFunction` and `side`.
- Every `Step` name (boards and panels).
- Every `Component` with its reference designator and `packageRef`.
- Every `LogicalNet` name.

## What it does not do

- Read only.
- Geometry (profiles, padstacks, features, and polygons) and the bill of materials are not decoded.
- Element nesting is not checked against the schema; the reader searches the whole document, so
  components or nets in unexpected places are still found.
