# tpt-io-dicom

DICOM (PS3.5 / PS3.6 / PS3.7 / PS3.10) parser for the `tpt-io` workspace.

This crate is currently a scaffold: the error type and module layout follow the
`tpt-io-core` pattern, with format-specific parsers filled in incrementally per
`spec.txt` and the workspace `todo.md` checklist.

## Status

- `DicomError` enum built on `tpt_io_core::ParseError`
- Module layout ready for explicit/implicit VR transfer syntax parsing, tag
  dictionary, UID registry, pixel data extraction, anonymization, and
  `tpt-sci-image` integration

## Planned features

- Explicit VR transfer syntax parsing
- Implicit VR transfer syntax parsing
- Standard DICOM tag dictionary
- UID registry for SOP classes
- Pixel data extraction (raw + JPEG 2000 / RLE stubs)
- Study/series/instance hierarchy navigation
- Anonymization utilities
- Streaming pixel data for large volumetric datasets
- `tpt-sci-image` integration behind `sci-image-integration` feature flag
- Round-trip, fuzz, golden-file, and property-based tests

## Golden-file corpus

Licensed sample DICOM files for golden-file testing live in `tests/samples/`.
See that directory for licensing details.

## License

MIT OR Apache-2.0
