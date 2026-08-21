# tpt-io-nifti

NIfTI-1 / NIfTI-2 neuroimaging parser for the `tpt-io` workspace — headers,
affine transforms, and multi-dimensional voxel arrays.

## Features

- **Header** (`header`): decodes the 348-byte NIfTI-1 header into a typed
  [`Nifti1Header`]: dimensions, voxel sizes, datatype, scale slope/intercept,
  `sform`/`qform` orientation, and the affine transform.
- **Coordinates** (`coord`): `voxel ↔ world` (RAS) mapping via the affine,
  plus `RAS ↔ LPS` conversion and its analytic inverse.
- **Model** (`model`): [`NiftiFile`] pairs the header with the raw voxel-data
  block and offers typed scalar access (`voxel`) honouring `scl_slope`/`scl_inter`.
- **Reader/Validator** (`parser`): [`NiftiReader`] parses `.nii` single files and
  `.hdr`/`.img` dual-file pairs; `NiftiValidator` checks syntactic and semantic
  invariants. A `gzip` feature adds `.nii.gz` decompression via `flate2`.
- **Round-trip** (`writer`): [`serialize_header`] rebuilds the 348-byte header.
- **Integration stub**: `sci-image-integration` feature provides a conversion
  boundary into the (not-yet-published) `tpt-sci-image` crate.

## Example

```rust
use tpt_io_core::Reader;
use tpt_io_nifti::header::parse_header;
use tpt_io_nifti::coord::{voxel_to_world, ras_to_lps};

// (build a 348-byte header as in the crate docs …)
let h = parse_header(&raw).unwrap();
let w = voxel_to_world(&h.affine(), 0.0, 0.0, 0.0);
let _lps = ras_to_lps(w);
```

## Status

Per the workspace `todo.md`, NIfTI is implemented for the core neuroimaging
subset (`.nii` / `.hdr`/`.img` header + data, metadata, affine, coordinate
conversions, `gzip` behind a feature, round-trip, fuzz, golden-file, and
property tests). NIfTI-2 is parsed only far enough to reject it with a clear
error until added. `cargo publish --dry-run` is blocked until `tpt-io-core` is
on crates.io (see §4 publish order).
