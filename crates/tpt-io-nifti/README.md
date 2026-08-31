# tpt-io-nifti

NIfTI-1 / NIfTI-2 neuroimaging parser for the `tpt-io` workspace — headers,
affine transforms, and multi-dimensional voxel arrays.

## Quick start

```rust
use tpt_io_core::Reader;
use tpt_io_nifti::header::parse_header;
use tpt_io_nifti::coord::{voxel_to_world, ras_to_lps};

let h = parse_header(&raw).unwrap();
let w = voxel_to_world(&h.affine(), 0.0, 0.0, 0.0);
let _lps = ras_to_lps(w);
```

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `gzip` | no | `.nii.gz` decompression via `flate2` (requires `std`). |
| `sci-image-integration` | no | Stub conversion into a future `tpt-sci-image` crate. |

## Testing

```sh
cargo test -p tpt-io-nifti
cargo test -p tpt-io-nifti --features gzip
cargo +nightly fuzz run nifti
```

## Status

Per the workspace `todo.md`, NIfTI is implemented for the core neuroimaging
subset (`.nii` / `.hdr`/`.img` header + data, metadata, affine, coordinate
conversions, `gzip` behind a feature, round-trip, fuzz, golden-file, and
property tests). NIfTI-2 is parsed only far enough to reject it with a clear
error until added. `cargo publish --dry-run` is blocked until `tpt-io-core` is
on crates.io (see §4 publish order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
