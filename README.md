# tpt-io-standards

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A pure-Rust, feature-gated workspace for parsing, validating, and serializing
industry-standard data formats. Each format lives in its own crate under
`crates/`, sharing a common foundation ([`tpt-io-core`]) so that every parser
offers the same error-reporting, encoding-detection, and validation ergonomics.

Repository: <https://github.com/tpt-solutions/tpt-io-standards>

All parsers are implemented from scratch in pure Rust (no C/C++ FFI such as
libgdal, dcmtk, or netcdf-c), keeping them cross-platform, memory-safe, and
WASM-compatible. Parsers aim for zero-copy / low-allocation reading where
possible, with strict schema validation and structured, byte-offset-aware
errors.

## Workspace layout

| Crate | Format(s) | Domain |
| --- | --- | --- |
| `tpt-io-core` | shared traits, errors, encoding & checksum utils | foundation |
| `tpt-io-dicom` | DICOM (PS3.5/6/7/10) | Medical |
| `tpt-io-fhir` | HL7 FHIR R4/R5 (JSON + XML) | Medical |
| `tpt-io-nifti` | NIfTI-1 / NIfTI-2 | Medical |
| `tpt-io-ifc` | IFC 4.0 / 4.3 (STEP-PF) | Construction |
| `tpt-io-citygml` | CityGML 2.0 / 3.0 | Construction |
| `tpt-io-spice` | SPICE (HSPICE/LTSpice/Ngspice/PSpice dialects) | Electronics |
| `tpt-io-gerber` | Gerber X2/X3 + Excellon | Electronics |
| `tpt-io-edif` | EDIF 2.0.0 / 4.0.0 | Electronics |
| `tpt-io-geo` | GeoJSON / Shapefile / GeoTIFF | Earth & GIS |
| `tpt-io-netcdf` | NetCDF-3 / NetCDF-4 (HDF5) | Earth & GIS |
| `tpt-io-welllog` | LAS 2.0/3.0 + DLIS | Earth & GIS |
| `tpt-io-capeopen` | CAPE-OPEN 1.1 / 2.0 | Process |
| `tpt-io-sbml` | SBML Level 2 / 3 | Process |
| `tpt-io` | umbrella re-export of all formats | umbrella |

## Using the umbrella crate

Downstream Tier-2 repositories enable only the formats they need:

```toml
[dependencies]
tpt-io = { version = "0.1", features = ["dicom", "ifc"] }
```

Compiling `tpt-io` with **no** features is a true zero-cost crate that only
re-exports [`tpt-io-core`]. The feature tree is intentionally flat (no
transitive feature dependencies) to avoid Cargo feature-unification surprises.

Individual format crates can also be depended on directly:

```toml
[dependencies]
tpt-io-dicom = "0.1"
```

## Feature flags on `tpt-io-core`

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | `std`-only conveniences (e.g. `std::io` adapters) |
| `alloc` | no | `alloc`-based types without `std` |
| `checksums` | no | CRC32 / MD5 / SHA256 helpers |

## Design principles

- **Structured errors** — every parse error carries the byte offset, the
  expected-vs-actual token, a reference into the format specification, and a
  slice of surrounding bytes (see `tpt_io_core::ParseError`).
- **Two-phase validation** — syntactic (is it structurally valid?) and
  semantic (does it make sense?) checks, split via `Validator` and toggleable
  with `with_syntactic_validation` / `with_semantic_validation`.
- **Zero-copy where possible** — parsers prefer `&str` / `&[u8]` slices into
  the input buffer and support streaming for very large files.
- **Extensible** — the `FormatExtension` trait lets users add custom tags or
  dialects without forking a crate.

## MSRV policy

The minimum supported Rust version (MSRV) is **1.75**, declared in
`[workspace.package]` (`rust-version`). Crates must therefore target edition
2021. We will only raise the MSRV in a minor (or major) release, and the bump
will be called out in the changelog. CI always builds against the pinned
`rust-version` in addition to stable.

## Building & testing

```sh
cargo build --workspace
cargo test  --workspace
cargo test  --workspace --all-features
cargo clippy --workspace --all-targets --all-features
cargo fmt   --all -- --check
```

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at
your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
