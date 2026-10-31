# Changelog

## Unreleased

- Initial reader for MCAP: header, schemas, channels, messages, and uncompressed
  chunks. `lz4` and `zstd` chunks are reported as unsupported.
- Writer for unchunked files that re-reads to the same `Mcap`.
