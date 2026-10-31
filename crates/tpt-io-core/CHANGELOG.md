# Changelog

## Unreleased

- `xml` module: an alloc-only XML element tree reader, with escaping helpers.
- `inflate` and `zip` (behind the `zip` feature, which also enables `checksums`): a DEFLATE decoder, a ZIP reader for
  stored and deflate members, and a stored-member ZIP writer with CRC-32 checksums.

## [0.1.0] - 2026-08-31

Initial release.

- Core parsing implementation
- Validation support
- Round-trip serialization
- Fuzz targets
- Golden-file corpus
- Property-based tests
- `no_std` + `alloc` + `std` support
