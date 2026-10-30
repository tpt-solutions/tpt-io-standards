# Changelog

## [0.1.0] - 2026-08-31

Initial release.

- Core parsing implementation
- NetCDF-3 classic and 64-bit offset header parsing
- Dimension and variable metadata extraction
- Attribute extraction (global and per-variable)
- Coordinate-variable detection
- Chunked hyperslab reading
- NetCDF-4 (HDF5) detection and dataset reading
- Deflate/shuffle/chunking filter support
- Validation support
- Round-trip serialization
- Fuzz targets
- Golden-file corpus
- Property-based tests
- `no_std` + `alloc` + `std` support
