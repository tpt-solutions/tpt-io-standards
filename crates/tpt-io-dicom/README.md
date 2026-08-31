# tpt-io-dicom

DICOM (PS3.5 / PS3.6 / PS3.7 / PS3.10) parser for the `tpt-io` workspace —
explicit and implicit VR transfer syntaxes, the standard DICOM tag dictionary,
UID registry, pixel data extraction, and anonymization utilities.

## Quick start

```rust
use tpt_io_core::Reader;
use tpt_io_dicom::parser::DicomReader;

let bytes = std::fs::read("sample.dcm").unwrap();
let dataset = DicomReader::new().parse(&bytes).unwrap();
```

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences |
| `alloc` | no | `alloc`-based types without `std` |
| `sci-image-integration` | no | Stub conversion into a future `tpt-sci-image` crate |

## Testing

```sh
cargo test -p tpt-io-dicom
cargo +nightly fuzz run dicom
```

## Status

Per the workspace `todo.md`, `tpt-io-dicom` implements explicit/implicit VR
transfer syntax parsing, the standard DICOM tag dictionary, UID registry,
pixel data extraction, anonymization utilities, and `tpt-sci-image` integration
stubs. Round-trip, fuzz, golden-file, and property-based tests are included.
`cargo publish --dry-run` is blocked until `tpt-io-core` is on crates.io
(see §4 publish order).

## License

Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
at your option.
