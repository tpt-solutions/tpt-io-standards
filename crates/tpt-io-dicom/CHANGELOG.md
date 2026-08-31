# Changelog

## [0.1.0] - 2026-08-31

Initial release.

- Core parsing implementation
- Explicit/implicit VR transfer syntax parsing
- Standard DICOM tag dictionary
- UID registry for SOP classes
- Pixel data extraction (raw + JPEG 2000 / RLE stubs)
- Study/series/instance hierarchy navigation
- Anonymization utilities
- Streaming pixel data for large volumetric datasets
- Validation support
- Round-trip serialization
- Fuzz targets
- Golden-file corpus
- Property-based tests
- `no_std` + `alloc` + `std` support
