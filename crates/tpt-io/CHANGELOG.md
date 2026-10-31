# Changelog

## Unreleased

- Added the `s57` feature: re-exports `tpt-io-s57` (IHO S-57 ENC reader) and adds
  `IoError::S57`.

## [0.1.0] - 2026-08-31

Initial release.

- Core parsing implementation
- Feature-gated re-exports for all format crates
- Unified `IoError` type wrapping domain errors
- Format detection by file extension and magic bytes
- Validation support
- Fuzz targets
- Golden-file corpus
- Property-based tests
- `no_std` + `alloc` + `std` support
