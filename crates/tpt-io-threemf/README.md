# tpt-io-threemf

Reader and writer for 3MF (3D Manufacturing Format) packages, the ZIP-based container used by 3D printers and
slicers.

## Spec access and licence

The 3MF Core Specification is published by the 3MF Consortium under a royalty-free licence. No specification text
is copied into this crate; test packages are built in code.

## What it reads

- The `3D/3dmodel.model` XML part of a ZIP package, with stored or deflate members.
- Objects in `resources`: id, optional name, `mesh` vertices (`x`, `y`, `z`) and triangles (`v1`, `v2`, `v3`).
- Build items: `objectid` and an optional 12-value `transform`.
- The `unit` attribute, defaulting to `millimeter`.

Each build item must refer to an object that exists, and each triangle must refer to vertices that exist; otherwise
the package is rejected.

## What it writes

- A package with `[Content_Types].xml`, `_rels/.rels` and `3D/3dmodel.model`, with stored members and CRC-32
  checksums.
- Objects, meshes, build items and transforms, as read above.

## What it does not do

- Materials, colour groups, texture coordinates, slices, beam lattices and production (component) extensions are
  skipped.
- Namespace prefixes on model elements are not handled; the default namespace used by 3MF writers is expected.
- Packages are not written with compression.
