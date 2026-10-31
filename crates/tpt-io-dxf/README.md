# tpt-io-dxf

A reader and writer for AutoCAD DXF drawings in the ASCII form.

## Spec access and licence

The DXF reference is published by Autodesk for interchange. No reference text, sample drawing or
template is copied into this crate; the test drawing is written for these tests.

## What it reads

- The `$ACADVER` header variable.
- Layer names from the `LAYER` table.
- Entities in the `ENTITIES` section:
  - `LINE` (start and end points);
  - `CIRCLE` (centre and radius);
  - `LWPOLYLINE` (vertices, closed flag from group 70);
  - `TEXT` (insertion point, height, value).
- Any other entity type is kept as `Entity::Other` with its name and layer, so it is counted but
  not decoded.

## What it writes

The version, the layer table, and the four supported entity types (and `Other` as a bare entity
with its layer). Parsing the output gives back the same drawing.

## What it does not do

- Binary DXF is not read; it is rejected as malformed group codes.
- Blocks, dimensions, hatches, splines, and extended entity data are not decoded.
- Units, extents, and the other header variables are ignored.
