# tpt-io-mesh

Readers and writers for triangle meshes in three common formats: STL (for 3D printing and CAD), Wavefront OBJ,
and PLY.

## Spec access and licence

STL (3D Systems), OBJ (Wavefront) and PLY (Stanford) are published formats with no restrictive licence. No
specification text or model is copied into this crate; test meshes are built in code.

## What it reads

- **STL**: ASCII (`solid` … `endsolid`) and binary. Binary is detected from the length (84 + 50 × count
  bytes). Each triangle brings its own three vertices and its facet normal.
- **OBJ**: `v` vertices and `f` faces. Face corners may be `i`, `i/t`, `i//n` or `i/t/n`; negative indices count
  from the end; faces are fan-triangulated.
- **PLY**: ASCII and binary little-endian, with `vertex` x/y/z properties and `face` lists named
  `vertex_indices` or `vertex_index`.
- **GLB** (glTF 2.0 binary): the `POSITION` and `indices` accessors of every `TRIANGLES` primitive. Other
  primitive modes are skipped; buffers other than the binary chunk are rejected.

## What it writes

- STL: ASCII (`write_ascii`) and binary (`write_binary`), with facet normals.
- OBJ: vertices and triangular faces.
- PLY: ASCII, with `vertex` and `face` elements.
- GLB: one node, one `TRIANGLES` primitive, with float `POSITION` and `u32` indices.

Parsing the output gives back the same triangles and vertices.

## What it does not do

- Normals, texture coordinates, colours and materials are not read from OBJ or PLY.
- Binary big-endian PLY is not read.
- Normals, texture coordinates and materials in glTF are not read; glTF JSON files with external buffers are
  not supported (GLB only).
