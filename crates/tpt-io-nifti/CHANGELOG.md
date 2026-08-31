# Changelog

## [0.1.0] - 2026-08-31

Initial release.

- Core parsing implementation
- NIfTI-1 header decoding (dimensions, voxel sizes, datatype, affine, sform/qform)
- Affine transform and coordinate conversions (voxel ↔ world, RAS ↔ LPS)
- Multi-dimensional voxel-array access with scl_slope/scl_inter scaling
- `.nii` single-file and `.hdr`/`.img` dual-file pair parsing
- NIfTI-2 detection with clear rejection error
- gzip decompression for `.nii.gz` files
- Validation support
- Round-trip serialization
- Fuzz targets
- Golden-file corpus
- Property-based tests
- `no_std` + `alloc` + `std` support
