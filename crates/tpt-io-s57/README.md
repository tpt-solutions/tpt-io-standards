# tpt-io-s57

A reader for IHO S-57 electronic navigational chart (ENC) exchange files. S-57 stores charts
as ISO/IEC 8211 records; this crate reads that framing, the dataset parameters, the feature
objects and their attributes, and the geometry each feature resolves to.

## Spec access and licence

S-57 is published free of charge by the International Hydrographic Organization (IHO
Publication S-57, edition 3.1), and ISO/IEC 8211 is an open transfer standard. Only the field
layouts and structure rules were used to write this crate; no text from either document is
copied in. Chart data (ENC cells) carries its own licence terms, so no cells are included
here. The tests build their files from the ISO 8211 rules; they are not validated against
official NOAA or IHO sample cells.

## What it reads

- **ISO 8211 framing** (`iso8211`): records, the directory, and field areas. The first record
  is the data descriptive record (DDR); its field descriptions give each field's subfield
  labels and format controls (`A`, `A(n)`, `I`, `R`, `B(n)`, `b1n`, `b2n`).
- **Dataset parameters**: the dataset name and edition from `DSID`, and the coordinate
  (`COMF`) and sounding (`SOMF`) multiplying factors from `DSPM`. Both are required.
- **Features**: identifier, object class (`OBJL`), primitive (point, line, area), and the
  attributes in `ATTF` as attribute codes and text values.
- **Geometry**: point features resolve to their node positions. Line and area features resolve
  to one position chain per edge reference, from the edge's begin node through its interior
  points to its end node, reversed when the reference is reversed. Soundings from `SG3D`
  carry a depth in metres.

## Limits

- Read only. There is no writer, so the round-trip checks used for other crates do not apply.
- Files must be self-contained. A feature that refers to a vector record missing from the file
  is an error, not a partial read.
- Attribute values are text. Codes are not mapped to names; the S-57 object and attribute
  catalogues are not included.
- Area rings are not joined. Each edge chain is returned as it appears in the feature.
- Binary subfields are read little-endian, the ISO 8211 convention.

## Example

```rust
let dataset = tpt_io_s57::parse(&std::fs::read("US5MA22M.000")?)?;
for feature in &dataset.features {
    println!("{} class={} attrs={}", feature.id, feature.object_class, feature.attributes.len());
}
```

## Features

- `std` (default): `std::error::Error` for `S57Error`.
- `alloc`: builds without `std`, for `no_std` users with an allocator.

## Testing

`cargo test -p tpt-io-s57` builds synthetic ISO 8211 files and checks the dataset, the
attributes, the point, sounding and edge geometry, the rejection of dangling references, and
that every truncation of a file returns an error rather than panicking. The fuzz target is
`fuzz/fuzz_targets/s57.rs`.
